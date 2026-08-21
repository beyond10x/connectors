//! Application-layer admission of the exact network facts a SIP driver may consume.

use std::collections::BTreeMap;
use std::net::{IpAddr, SocketAddr};
use std::ops::RangeInclusive;
use std::time::Duration;

use domain::{DriverId, Interaction, ProtocolPlan, ZeroIoPlan};
use protocol::sip::{SipDialInput, SIP_DIAL_OPERATION};

/// Maximum time an admitted outbound invitation may remain unanswered.
pub const MAX_SIP_DIAL_TIMEOUT: Duration = Duration::from_secs(30);

/// Clear signaling transports supported by the first native SIP driver profile.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SipSignalingTransport {
    /// SIP over UDP.
    Udp,
    /// SIP over TCP with Content-Length framing.
    Tcp,
}

/// Deployment-selected maturity boundary for SIP routes outside loopback.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SipNetworkMode {
    /// Every signaling and media address must be loopback.
    Loopback,
    /// An operator explicitly admitted exact non-loopback apertures for development characterization.
    OperatorAuthorizedDevelopment,
}

/// One exact IP and bounded port interval admitted for a socket role.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SocketAperture {
    address: IpAddr,
    /// Bits of `address` that must match. Full width is one exact address, which is the default.
    prefix: u8,
    ports: RangeInclusive<u16>,
}

impl SocketAperture {
    /// One exact address and port range.
    ///
    /// # Errors
    ///
    /// [`SipAdmissionError::InvalidAperture`] for an empty port range or one containing zero.
    pub fn new(address: IpAddr, ports: RangeInclusive<u16>) -> Result<Self, SipAdmissionError> {
        Self::network(address, Self::full_width(address), ports)
    }

    /// A network prefix and port range, admitting every address inside the prefix.
    ///
    /// # Why a prefix exists at all
    ///
    /// An exact address is the right aperture for a trunk that has one. A trunk reached by name
    /// inside a cluster may not: a headless Service answers with a different pod address per
    /// lookup, so pinning one would refuse the call the moment the trunk was rescheduled. Without
    /// a prefix an operator's only options are to pin an address that changes or to abandon the
    /// aperture, and the second is what actually happens.
    ///
    /// This stays an aperture, not an escape hatch: the prefix is declared by the deployment, and
    /// a resolved address outside it is refused exactly as an exact mismatch is.
    ///
    /// # Errors
    ///
    /// [`SipAdmissionError::InvalidAperture`] for an empty port range, one containing zero, a
    /// prefix wider than the address family, or a prefix whose host bits are set — `10.0.0.7/24`
    /// is refused rather than silently read as `10.0.0.0/24`, because the two say different things
    /// and the operator meant one of them.
    pub fn network(
        address: IpAddr,
        prefix: u8,
        ports: RangeInclusive<u16>,
    ) -> Result<Self, SipAdmissionError> {
        if ports.is_empty() || ports.contains(&0) || prefix > Self::full_width(address) {
            return Err(SipAdmissionError::InvalidAperture);
        }
        // Checked against the address itself, not through `covers`: every address trivially
        // matches its own prefix, so asking `covers` here would always say yes and let
        // `10.0.0.7/24` through as if it were `10.0.0.0/24`.
        let declared_cleanly = match address {
            IpAddr::V4(address) => host_bits_clear(&address.octets(), prefix),
            IpAddr::V6(address) => host_bits_clear(&address.octets(), prefix),
        };
        if !declared_cleanly {
            return Err(SipAdmissionError::InvalidAperture);
        }
        Ok(Self {
            address,
            prefix,
            ports,
        })
    }

    const fn full_width(address: IpAddr) -> u8 {
        match address {
            IpAddr::V4(_) => 32,
            IpAddr::V6(_) => 128,
        }
    }

    /// Whether `target` falls inside this aperture's prefix, ignoring ports.
    fn covers(&self, target: IpAddr) -> bool {
        // Mixing families is never a match: an IPv4-mapped IPv6 address is a different address,
        // and treating it as equal is how a v6 aperture silently admits a v4 destination.
        match (self.address, target) {
            (IpAddr::V4(network), IpAddr::V4(target)) => {
                masked(&network.octets(), &target.octets(), self.prefix)
            }
            (IpAddr::V6(network), IpAddr::V6(target)) => {
                masked(&network.octets(), &target.octets(), self.prefix)
            }
            _ => false,
        }
    }

    #[must_use]
    pub fn contains(&self, target: SocketAddr) -> bool {
        self.covers(target.ip()) && self.ports.contains(&target.port())
    }

    #[must_use]
    pub fn contains_ip(&self, target: IpAddr) -> bool {
        self.covers(target)
    }
}

/// Whether every bit below `prefix` is zero, so the address names a network rather than a host.
fn host_bits_clear(octets: &[u8], prefix: u8) -> bool {
    let whole = usize::from(prefix / 8);
    let remainder = prefix % 8;
    if remainder != 0 {
        // The bits of the partial byte that fall outside the prefix.
        if octets[whole] & (0xFF_u8 >> remainder) != 0 {
            return false;
        }
        return octets[whole + 1..].iter().all(|octet| *octet == 0);
    }
    octets[whole..].iter().all(|octet| *octet == 0)
}

/// Whether `target` matches `network` in its first `prefix` bits.
fn masked(network: &[u8], target: &[u8], prefix: u8) -> bool {
    let whole = usize::from(prefix / 8);
    let remainder = prefix % 8;
    if network[..whole] != target[..whole] {
        return false;
    }
    if remainder == 0 {
        return true;
    }
    // The partial byte: keep the high `remainder` bits, discard the rest.
    let mask = 0xFF_u8 << (8 - remainder);
    network[whole] & mask == target[whole] & mask
}

/// Where a trunk's signalling goes, before it is an address.
///
/// # Why a name is not resolved here
///
/// A trunk inside a cluster is reached by service name, and that name may answer on a different
/// address per call — that is what DNS load balancing *is*. Resolving once at configuration load
/// would pin the first answer and go stale the moment the trunk moved.
///
/// Resolution therefore happens per dial, in the adapter, because this crate plans and proves and
/// opens nothing. What matters for safety is the order: **the resolved address is then checked
/// against the trunk's aperture like any other**, so a name cannot reach anywhere its configured
/// aperture does not already admit. Were it resolved after admission, DNS would be an aperture
/// bypass — anyone who could answer for that name could redirect the call.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SipSignalingTarget {
    /// A fixed address, admitted exactly as written.
    Address(SocketAddr),
    /// A name resolved at dial time, then aperture-checked.
    Host {
        /// The name to resolve. Never caller-supplied.
        host: String,
        /// The port to reach it on.
        port: u16,
    },
}

impl From<SocketAddr> for SipSignalingTarget {
    fn from(address: SocketAddr) -> Self {
        Self::Address(address)
    }
}

impl SipSignalingTarget {
    /// The address, when this target has already been resolved.
    ///
    /// A route that reached admission or a driver is always resolved; this returns `None` only for
    /// a configured-but-unresolved target, which is a programming error rather than a refusal a
    /// caller can cause.
    #[must_use]
    pub const fn address(&self) -> Option<SocketAddr> {
        match self {
            Self::Address(address) => Some(*address),
            Self::Host { .. } => None,
        }
    }
}

/// Deployment-selected route. No request or model field can construct this from wire input.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SipDeploymentRoute {
    pub connection: String,
    pub signaling_bind: SocketAddr,
    pub sent_by: String,
    pub target: SipSignalingTarget,
    pub signaling_transport: SipSignalingTransport,
    pub to_uri: String,
    pub from_uri: String,
    pub media_advertised: IpAddr,
    pub media_bind: IpAddr,
    pub signaling_apertures: Vec<SocketAperture>,
    pub media_apertures: Vec<SocketAperture>,
    pub dial_timeout: Duration,
    pub network_mode: SipNetworkMode,
    /// Whether a caller-supplied number may be dialled through this trunk.
    ///
    /// Off by default, and deliberately per trunk: a fixed endpoint reached by alias must not
    /// silently become an open dialler because a caller passed a number.
    pub accepts_dialed_number: bool,
}

/// Failure before the socket-capable crate receives a plan.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum SipAdmissionError {
    #[error("operation is not an admitted SIP session establishment")]
    WrongOperation,
    #[error("admitted SIP identity is incomplete")]
    InvalidIdentity,
    #[error("deployment SIP route belongs to another Connection")]
    ConnectionMismatch,
    #[error("SIP route has an invalid socket aperture")]
    InvalidAperture,
    #[error("SIP signaling target is outside its admitted aperture")]
    SignalingTargetRefused,
    #[error("SIP listener is outside its admitted aperture")]
    SignalingBindRefused,
    #[error("SIP media listener is outside its admitted aperture")]
    MediaBindRefused,
    #[error("SIP advertised media address is outside its admitted aperture")]
    MediaAdvertisedRefused,
    #[error("SIP route addresses do not satisfy their declared network mode")]
    NetworkModeMismatch,
    #[error("SIP route has an invalid finite deadline")]
    InvalidDeadline,
    #[error("sip.dial target is not a valid Connection-owned alias")]
    InvalidTargetAlias,
    #[error("sip.dial target alias is not configured on this Connection")]
    UnknownTargetAlias,
    #[error("SIP target table contains a duplicate alias")]
    DuplicateTargetAlias,
    #[error("sip.dial named no target and this Connection declares no default trunk")]
    NoDefaultTarget,
    #[error("sip.dial number is not a valid dialled number")]
    InvalidNumber,
    #[error("this SIP trunk does not admit a dialled number")]
    NumberNotAdmitted,
    #[error("the SIP trunk name did not resolve to any address")]
    SignalingTargetUnresolved,
}

/// Turns a trunk's configured name into the addresses it currently answers on.
///
/// Supplied by the adapter, because this crate opens nothing. An empty answer is a resolution
/// failure; the first address is taken, which is how a DNS load balancer's own ordering is
/// honoured rather than second-guessed.
pub trait SipHostResolver {
    /// Addresses `host` answers on at `port`, in the order the resolver returned them.
    fn resolve(&self, host: &str, port: u16) -> Vec<SocketAddr>;
}

impl<F: Fn(&str, u16) -> Vec<SocketAddr>> SipHostResolver for F {
    fn resolve(&self, host: &str, port: u16) -> Vec<SocketAddr> {
        self(host, port)
    }
}

/// A resolver for trunks that are all fixed addresses, which never needs to resolve anything.
pub struct NoHostResolution;

impl SipHostResolver for NoHostResolution {
    fn resolve(&self, _host: &str, _port: u16) -> Vec<SocketAddr> {
        Vec::new()
    }
}

/// An answer already obtained, so admission stays synchronous and does no lookup.
///
/// Paired with [`SipDialRouteTable::pending_host`]: the adapter resolves off the reactor and hands
/// the result in. The addresses are still aperture-checked, exactly as a live lookup's would be.
pub struct FixedHostResolution(pub Vec<SocketAddr>);

impl SipHostResolver for FixedHostResolution {
    fn resolve(&self, _host: &str, _port: u16) -> Vec<SocketAddr> {
        self.0.clone()
    }
}

/// Deployment-owned alias table. Callers can select a name but cannot construct any route field.
#[derive(Debug, Clone)]
pub struct SipDialRouteTable {
    connection: String,
    routes: BTreeMap<String, SipDeploymentRoute>,
    default_alias: Option<String>,
}

impl SipDialRouteTable {
    /// Build the table with no default trunk, so every dial must name its alias.
    ///
    /// # Errors
    ///
    /// As [`SipDialRouteTable::with_default`].
    pub fn new<I>(connection: impl Into<String>, routes: I) -> Result<Self, SipAdmissionError>
    where
        I: IntoIterator<Item = (String, SipDeploymentRoute)>,
    {
        Self::with_default(connection, routes, None)
    }

    /// Build the table, optionally naming the trunk a dial without an alias selects.
    ///
    /// # Errors
    ///
    /// [`SipAdmissionError::InvalidTargetAlias`] for an alias outside the closed grammar,
    /// [`SipAdmissionError::ConnectionMismatch`] for a route belonging elsewhere,
    /// [`SipAdmissionError::DuplicateTargetAlias`] for a repeated alias, and
    /// [`SipAdmissionError::UnknownTargetAlias`] when the named default is not in the table.
    pub fn with_default<I>(
        connection: impl Into<String>,
        routes: I,
        default_alias: Option<String>,
    ) -> Result<Self, SipAdmissionError>
    where
        I: IntoIterator<Item = (String, SipDeploymentRoute)>,
    {
        let connection = connection.into();
        let mut admitted = BTreeMap::new();
        for (alias, route) in routes {
            SipDialInput {
                target: Some(alias.clone()),
                number: None,
            }
            .validate()
            .map_err(|_| SipAdmissionError::InvalidTargetAlias)?;
            if route.connection != connection {
                return Err(SipAdmissionError::ConnectionMismatch);
            }
            if admitted.insert(alias, route).is_some() {
                return Err(SipAdmissionError::DuplicateTargetAlias);
            }
        }
        // A default naming a trunk that is not configured is refused at construction rather than
        // at the first dial, so the fault surfaces when the deployment is composed.
        if let Some(alias) = &default_alias {
            if !admitted.contains_key(alias) {
                return Err(SipAdmissionError::UnknownTargetAlias);
            }
        }
        Ok(Self {
            connection,
            routes: admitted,
            default_alias,
        })
    }

    #[must_use]
    pub fn connection(&self) -> &str {
        &self.connection
    }

    /// The alias a dial without a target selects, if one is configured.
    #[must_use]
    pub fn default_alias(&self) -> Option<&str> {
        self.default_alias.as_deref()
    }

    /// The trunk this dial selects, applying the default when no alias was named.
    fn select(&self, input: &SipDialInput) -> Result<&SipDeploymentRoute, SipAdmissionError> {
        input.validate().map_err(|error| match error {
            protocol::sip::SipDialInputError::InvalidNumber => SipAdmissionError::InvalidNumber,
            protocol::sip::SipDialInputError::InvalidTargetAlias => {
                SipAdmissionError::InvalidTargetAlias
            }
        })?;
        let alias = match (&input.target, &self.default_alias) {
            (Some(alias), _) => alias.as_str(),
            (None, Some(default)) => default.as_str(),
            // Refused rather than guessed. A Connection with two trunks and no declared default
            // has no defensible answer, and picking one would place a call somewhere nobody chose.
            (None, None) => return Err(SipAdmissionError::NoDefaultTarget),
        };
        self.routes
            .get(alias)
            .ok_or(SipAdmissionError::UnknownTargetAlias)
    }

    /// The alias this dial selects, after the default is applied.
    ///
    /// The permission subject is looked up from this rather than from the caller's field, because
    /// a dial that named no alias still reaches exactly one trunk and must be authorized against
    /// that trunk rather than against nothing.
    ///
    /// # Errors
    ///
    /// As the selection step of [`admit_sip_dial`].
    pub fn selected_alias(&self, input: &SipDialInput) -> Result<String, SipAdmissionError> {
        // Selection is run first so an alias is only reported for a dial that would be admitted,
        // and the two can never disagree about which trunk was chosen.
        self.select(input)?;
        match (&input.target, &self.default_alias) {
            (Some(alias), _) => Ok(alias.clone()),
            (None, Some(default)) => Ok(default.clone()),
            (None, None) => Err(SipAdmissionError::NoDefaultTarget),
        }
    }

    /// The name this dial must resolve before it can be admitted, if its trunk is named.
    ///
    /// Exists so an async adapter can do the lookup on a blocking task and hand the answer back
    /// through [`FixedHostResolution`]. `getaddrinfo` blocks, and a telephony path that blocks the
    /// reactor stalls every other call on that thread.
    ///
    /// # Errors
    ///
    /// As the selection step of [`admit_sip_dial`]: an invalid alias or number, an unknown alias,
    /// or no alias and no declared default.
    pub fn pending_host(
        &self,
        input: &SipDialInput,
    ) -> Result<Option<(String, u16)>, SipAdmissionError> {
        Ok(match &self.select(input)?.target {
            SipSignalingTarget::Host { host, port } => Some((host.clone(), *port)),
            SipSignalingTarget::Address(_) => None,
        })
    }

    fn resolve(
        &self,
        input: &SipDialInput,
        resolver: &dyn SipHostResolver,
    ) -> Result<SipDeploymentRoute, SipAdmissionError> {
        let mut route = self.select(input)?.clone();

        // Resolve before admission, never after: the address that comes back is aperture-checked
        // by `admit_sip_plan` exactly like a configured one.
        if let SipSignalingTarget::Host { host, port } = &route.target {
            let address = *resolver
                .resolve(host, *port)
                .first()
                .ok_or(SipAdmissionError::SignalingTargetUnresolved)?;
            route.target = SipSignalingTarget::Address(address);
        }

        if let Some(number) = &input.number {
            if !route.accepts_dialed_number {
                return Err(SipAdmissionError::NumberNotAdmitted);
            }
            route.to_uri = dialed_uri(&route.to_uri, number)
                .ok_or(SipAdmissionError::SignalingTargetRefused)?;
        }
        Ok(route)
    }
}

/// Rebuild a trunk's request URI with `number` as its user part.
///
/// **The host is taken from the trunk's own URI and never from the caller.** That is the whole
/// safety argument for accepting a number at all: the caller supplies digits, which have already
/// been validated as digits, and everything that decides *where* the call goes comes from the
/// Connection.
fn dialed_uri(to_uri: &str, number: &str) -> Option<String> {
    let (scheme, rest) = to_uri
        .strip_prefix("sips:")
        .map(|rest| ("sips", rest))
        .or_else(|| to_uri.strip_prefix("sip:").map(|rest| ("sip", rest)))?;
    // `rsplit_once` so the last `@` wins: the host is the authority, and anything before it is the
    // user part being replaced.
    let host = rest.rsplit_once('@').map_or(rest, |(_, host)| host);
    if host.is_empty() {
        return None;
    }
    Some(format!("{scheme}:{number}@{host}"))
}

/// Resolve the caller's opaque alias and produce socket-opening evidence for `sip.dial` only.
pub fn admit_sip_dial(
    plan: &ZeroIoPlan,
    input: &SipDialInput,
    routes: &SipDialRouteTable,
    resolver: &dyn SipHostResolver,
) -> Result<AdmittedSipPlan, SipAdmissionError> {
    if plan.facts().operation != SIP_DIAL_OPERATION
        || plan.admission().connection() != routes.connection()
    {
        return Err(SipAdmissionError::WrongOperation);
    }
    admit_sip_plan(plan, routes.resolve(input, resolver)?)
}

/// Non-serializable evidence handed only to the socket-capable `driver-sip` crate.
pub struct AdmittedSipPlan {
    provider: String,
    operation: String,
    organization: String,
    principal: String,
    grant: String,
    route: SipDeploymentRoute,
    _proof: AdmissionProof,
}

struct AdmissionProof;

impl AdmittedSipPlan {
    #[must_use]
    pub fn provider(&self) -> &str {
        &self.provider
    }

    #[must_use]
    pub fn operation(&self) -> &str {
        &self.operation
    }

    #[must_use]
    pub fn organization(&self) -> &str {
        &self.organization
    }

    #[must_use]
    pub fn principal(&self) -> &str {
        &self.principal
    }

    #[must_use]
    pub fn grant(&self) -> &str {
        &self.grant
    }

    #[must_use]
    pub fn route(&self) -> &SipDeploymentRoute {
        &self.route
    }

    #[must_use]
    pub fn admits_signaling(&self, target: SocketAddr) -> bool {
        self.route
            .signaling_apertures
            .iter()
            .any(|aperture| aperture.contains(target))
    }

    #[must_use]
    pub fn admits_media(&self, target: SocketAddr) -> bool {
        self.route
            .media_apertures
            .iter()
            .any(|aperture| aperture.contains(target))
    }
}

impl std::fmt::Debug for AdmittedSipPlan {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("AdmittedSipPlan")
            .field("provider", &self.provider)
            .field("operation", &self.operation)
            .field("organization", &self.organization)
            .field("principal", &self.principal)
            .field("grant", &self.grant)
            .field("route", &self.route)
            .finish_non_exhaustive()
    }
}

/// Join grant admission and deployment-only routing into one socket-opening proof.
pub fn admit_sip_plan(
    plan: &ZeroIoPlan,
    route: SipDeploymentRoute,
) -> Result<AdmittedSipPlan, SipAdmissionError> {
    let ProtocolPlan::SipV1(sip) = plan.protocol() else {
        return Err(SipAdmissionError::WrongOperation);
    };
    if plan.protocol().driver() != DriverId::SipV1
        || plan.facts().interaction != Interaction::SessionEstablishment
    {
        return Err(SipAdmissionError::WrongOperation);
    }
    if plan.admission().organization().is_empty()
        || plan.admission().principal().is_empty()
        || plan.admission().grant().is_empty()
    {
        return Err(SipAdmissionError::InvalidIdentity);
    }
    if sip.connection != route.connection || plan.admission().connection() != route.connection {
        return Err(SipAdmissionError::ConnectionMismatch);
    }
    // Nothing unresolved reaches a driver. `validate_sip_deployment_route` tolerates a name so a
    // deployment can declare one; admission is where that tolerance ends.
    if route.target.address().is_none() {
        return Err(SipAdmissionError::SignalingTargetUnresolved);
    }
    validate_sip_deployment_route(&route)?;
    Ok(AdmittedSipPlan {
        provider: plan.facts().provider.clone(),
        operation: plan.facts().operation.clone(),
        organization: plan.admission().organization().to_owned(),
        principal: plan.admission().principal().to_owned(),
        grant: plan.admission().grant().to_owned(),
        route,
        _proof: AdmissionProof,
    })
}

/// Validate deployment-owned SIP network facts without constructing driver evidence.
pub fn validate_sip_deployment_route(route: &SipDeploymentRoute) -> Result<(), SipAdmissionError> {
    if route.connection.is_empty()
        || route.sent_by.is_empty()
        || route.to_uri.is_empty()
        || route.from_uri.is_empty()
    {
        return Err(SipAdmissionError::InvalidIdentity);
    }
    if route.dial_timeout.is_zero() || route.dial_timeout > MAX_SIP_DIAL_TIMEOUT {
        return Err(SipAdmissionError::InvalidDeadline);
    }
    if route.network_mode == SipNetworkMode::Loopback
        && (!route.signaling_bind.ip().is_loopback()
            || !route.media_advertised.is_loopback()
            || !route.media_bind.is_loopback())
    {
        return Err(SipAdmissionError::NetworkModeMismatch);
    }
    // **Target checks need an address, and a configured trunk may still be a name.** This function
    // runs twice for a named trunk: once at configuration load, where everything above is checked
    // and the target cannot be, and again inside `admit_sip_plan` after resolution, where it is.
    // `admit_sip_plan` separately refuses an unresolved target, so skipping here widens nothing --
    // it is what lets a deployment declare a trunk by name at all.
    if let Some(target) = route.target.address() {
        if route.network_mode == SipNetworkMode::Loopback && !target.ip().is_loopback() {
            return Err(SipAdmissionError::NetworkModeMismatch);
        }
        if !route
            .signaling_apertures
            .iter()
            .any(|aperture| aperture.contains(target))
        {
            return Err(SipAdmissionError::SignalingTargetRefused);
        }
    }
    if route.signaling_bind.port() != 0
        && !route
            .signaling_apertures
            .iter()
            .any(|aperture| aperture.contains(route.signaling_bind))
    {
        return Err(SipAdmissionError::SignalingBindRefused);
    }
    if !route
        .media_apertures
        .iter()
        .any(|aperture| aperture.contains_ip(route.media_bind))
    {
        return Err(SipAdmissionError::MediaBindRefused);
    }
    if !route
        .media_apertures
        .iter()
        .any(|aperture| aperture.contains_ip(route.media_advertised))
    {
        return Err(SipAdmissionError::MediaAdvertisedRefused);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;
    use std::net::{Ipv4Addr, SocketAddrV4};

    use domain::{
        AdmittedOperation, Capability, ConnectionAuthority, Implementation, InitiationPolicy,
        OperationFacts, Placement, SipPlan,
    };

    use super::*;

    fn plan_for_organization(organization: &str) -> ZeroIoPlan {
        ZeroIoPlan::new(
            OperationFacts {
                provider: "loopback-pbx".to_owned(),
                operation: SIP_DIAL_OPERATION.to_owned(),
                service: "voice".to_owned(),
                interaction: Interaction::SessionEstablishment,
                placement: Placement::ConnectorsDeployment,
                implementation: Implementation::BuiltIn,
                required_capabilities: BTreeSet::from([Capability::PrivateNetwork]),
                permission_subjects: vec!["loopback:127.0.0.1".to_owned()],
            },
            AdmittedOperation::from_grant_decision(
                "loopback-pbx",
                SIP_DIAL_OPERATION,
                organization,
                "principal",
                "grant",
                ConnectionAuthority::new("connection", InitiationPolicy::b10x_only())
                    .unwrap(),
            ),
            ProtocolPlan::SipV1(SipPlan {
                connection: "connection".to_owned(),
            }),
        )
    }

    fn plan() -> ZeroIoPlan {
        plan_for_organization("org")
    }

    fn route() -> SipDeploymentRoute {
        let loopback = IpAddr::V4(Ipv4Addr::LOCALHOST);
        SipDeploymentRoute {
            connection: "connection".to_owned(),
            signaling_bind: SocketAddrV4::new(Ipv4Addr::LOCALHOST, 0).into(),
            sent_by: "127.0.0.1".to_owned(),
            target: SipSignalingTarget::Address(SocketAddrV4::new(Ipv4Addr::LOCALHOST, 5_060).into()),
            signaling_transport: SipSignalingTransport::Udp,
            to_uri: "sip:callee@127.0.0.1:5060".to_owned(),
            from_uri: "sip:caller@127.0.0.1".to_owned(),
            media_advertised: loopback,
            media_bind: loopback,
            signaling_apertures: vec![SocketAperture::new(loopback, 1..=u16::MAX).unwrap()],
            media_apertures: vec![SocketAperture::new(loopback, 1..=u16::MAX).unwrap()],
            dial_timeout: Duration::from_secs(5),
            network_mode: SipNetworkMode::Loopback,
            accepts_dialed_number: false,
        }
    }

    fn dial(target: Option<&str>, number: Option<&str>) -> SipDialInput {
        SipDialInput {
            target: target.map(ToOwned::to_owned),
            number: number.map(ToOwned::to_owned),
        }
    }

    /// A trunk reached by name, as a cluster service is.
    fn named_trunk() -> SipDeploymentRoute {
        let mut route = route();
        route.target = SipSignalingTarget::Host {
            host: "ivr.latest.cluster.svc.local".to_owned(),
            port: 5_060,
        };
        route.to_uri = "sip:ivr@ivr.latest.cluster.svc.local".to_owned();
        route.accepts_dialed_number = true;
        route
    }

    #[test]
    fn a_dial_with_only_a_number_takes_the_declared_default_trunk() {
        // `sip.dial(12341234)`: no alias, because the Connection already says which trunk.
        let mut trunk = route();
        trunk.accepts_dialed_number = true;
        let routes = SipDialRouteTable::with_default(
            "connection",
            [("ivr".to_owned(), trunk)],
            Some("ivr".to_owned()),
        )
        .unwrap();
        let admitted =
            admit_sip_dial(&plan(), &dial(None, Some("12341234")), &routes, &NoHostResolution)
                .unwrap();
        assert_eq!(admitted.route().to_uri, "sip:12341234@127.0.0.1:5060");
    }

    #[test]
    fn a_dial_with_no_alias_and_no_default_is_refused_rather_than_guessed() {
        // Two trunks and no declared default has no defensible answer, and picking one would place
        // a call somewhere nobody chose.
        let routes = SipDialRouteTable::new("connection", [("ivr".to_owned(), route())]).unwrap();
        assert_eq!(
            admit_sip_dial(&plan(), &dial(None, None), &routes, &NoHostResolution).unwrap_err(),
            SipAdmissionError::NoDefaultTarget
        );
    }

    #[test]
    fn a_default_naming_an_absent_trunk_is_refused_when_the_table_is_built() {
        // At composition, not at the first call: a deployment fault should not wait for someone to
        // dial before it surfaces.
        assert_eq!(
            SipDialRouteTable::with_default(
                "connection",
                [("ivr".to_owned(), route())],
                Some("missing".to_owned()),
            )
            .unwrap_err(),
            SipAdmissionError::UnknownTargetAlias
        );
    }

    #[test]
    fn a_trunk_that_does_not_admit_numbers_refuses_one() {
        // A fixed endpoint reached by alias must not silently become an open dialler.
        let routes = SipDialRouteTable::with_default(
            "connection",
            [("fixed".to_owned(), route())],
            Some("fixed".to_owned()),
        )
        .unwrap();
        assert_eq!(
            admit_sip_dial(&plan(), &dial(None, Some("100")), &routes, &NoHostResolution)
                .unwrap_err(),
            SipAdmissionError::NumberNotAdmitted
        );
    }

    #[test]
    fn a_named_trunk_is_resolved_before_admission_so_the_answer_is_aperture_checked() {
        // **The security property.** DNS decides an address; the aperture decides whether that
        // address is allowed. Resolving after admission would invert that and let anyone who can
        // answer for the name redirect the call.
        let mut trunk = named_trunk();
        trunk.network_mode = SipNetworkMode::OperatorAuthorizedDevelopment;
        trunk.signaling_apertures =
            vec![SocketAperture::new(IpAddr::V4(Ipv4Addr::new(10, 0, 0, 7)), 5_060..=5_060).unwrap()];
        let routes = SipDialRouteTable::with_default(
            "connection",
            [("ivr".to_owned(), trunk)],
            Some("ivr".to_owned()),
        )
        .unwrap();

        let inside = |_: &str, port: u16| vec![SocketAddr::from(([10, 0, 0, 7], port))];
        let admitted =
            admit_sip_dial(&plan(), &dial(None, Some("12341234")), &routes, &inside).unwrap();
        assert_eq!(
            admitted.route().target.address(),
            Some(SocketAddr::from(([10, 0, 0, 7], 5_060)))
        );

        // The same name, answered with an address the aperture does not admit.
        let outside = |_: &str, port: u16| vec![SocketAddr::from(([10, 0, 0, 8], port))];
        assert_eq!(
            admit_sip_dial(&plan(), &dial(None, Some("12341234")), &routes, &outside).unwrap_err(),
            SipAdmissionError::SignalingTargetRefused
        );

        // A name nothing answers for is a named refusal, not a panic or a fallback.
        let nothing = |_: &str, _: u16| Vec::new();
        assert_eq!(
            admit_sip_dial(&plan(), &dial(None, Some("12341234")), &routes, &nothing).unwrap_err(),
            SipAdmissionError::SignalingTargetUnresolved
        );
    }

    #[test]
    fn the_dialled_host_comes_from_the_trunk_and_never_from_the_caller() {
        // The whole safety argument for accepting a number: digits in, and everything that decides
        // where the call goes comes from the Connection's own URI.
        assert_eq!(
            dialed_uri("sip:ivr@ivr.latest.cluster.svc.local", "12341234").as_deref(),
            Some("sip:12341234@ivr.latest.cluster.svc.local")
        );
        // A trunk URI with no user part still yields the trunk's host.
        assert_eq!(
            dialed_uri("sip:pbx.example:5060", "100").as_deref(),
            Some("sip:100@pbx.example:5060")
        );
        // The scheme is preserved rather than downgraded.
        assert_eq!(
            dialed_uri("sips:ivr@secure.example", "100").as_deref(),
            Some("sips:100@secure.example")
        );
        // Not a SIP URI at all, and no host to borrow.
        assert_eq!(dialed_uri("https://pbx.example", "100"), None);
        assert_eq!(dialed_uri("sip:", "100"), None);
    }

    #[test]
    fn an_exact_aperture_still_admits_exactly_one_address() {
        // The default, and the property every existing deployment relies on: adding prefixes must
        // not widen an aperture that did not ask for one.
        let exact = SocketAperture::new(IpAddr::V4(Ipv4Addr::new(10, 0, 0, 7)), 5_060..=5_060)
            .expect("an exact aperture");
        assert!(exact.contains(SocketAddr::from(([10, 0, 0, 7], 5_060))));
        assert!(!exact.contains(SocketAddr::from(([10, 0, 0, 8], 5_060))));
        assert!(!exact.contains(SocketAddr::from(([10, 0, 0, 7], 5_061))));
    }

    #[test]
    fn a_prefix_aperture_admits_its_network_and_refuses_outside_it() {
        let network = SocketAperture::network(
            IpAddr::V4(Ipv4Addr::new(10, 42, 0, 0)),
            16,
            5_060..=5_060,
        )
        .expect("a /16 aperture");
        assert!(network.contains(SocketAddr::from(([10, 42, 0, 7], 5_060))));
        assert!(network.contains(SocketAddr::from(([10, 42, 255, 254], 5_060))));
        assert!(!network.contains(SocketAddr::from(([10, 43, 0, 1], 5_060))));
        assert!(!network.contains(SocketAddr::from(([10, 42, 0, 7], 5_061))));
    }

    #[test]
    fn a_prefix_that_is_not_on_its_own_boundary_is_refused_rather_than_rounded() {
        // `10.0.0.7/24` and `10.0.0.0/24` say different things, and the operator meant one of them.
        // Silently rounding would widen an aperture without anyone writing that down.
        assert_eq!(
            SocketAperture::network(IpAddr::V4(Ipv4Addr::new(10, 0, 0, 7)), 24, 5_060..=5_060)
                .unwrap_err(),
            SipAdmissionError::InvalidAperture
        );
        assert_eq!(
            SocketAperture::network(IpAddr::V4(Ipv4Addr::LOCALHOST), 33, 5_060..=5_060).unwrap_err(),
            SipAdmissionError::InvalidAperture
        );
    }

    #[test]
    fn a_partial_byte_prefix_masks_only_the_bits_it_declares() {
        // /12 is the classic private-range boundary and the case an all-whole-bytes implementation
        // gets wrong: 172.16.0.0/12 covers 172.16 through 172.31 and must stop there.
        let network =
            SocketAperture::network(IpAddr::V4(Ipv4Addr::new(172, 16, 0, 0)), 12, 5_060..=5_060)
                .expect("a /12 aperture");
        assert!(network.contains(SocketAddr::from(([172, 16, 0, 1], 5_060))));
        assert!(network.contains(SocketAddr::from(([172, 31, 255, 254], 5_060))));
        assert!(!network.contains(SocketAddr::from(([172, 32, 0, 1], 5_060))));
        assert!(!network.contains(SocketAddr::from(([172, 15, 255, 254], 5_060))));
    }

    #[test]
    fn an_aperture_never_matches_across_address_families() {
        // An IPv4-mapped IPv6 address is a different address. Treating it as equal is how a v6
        // aperture silently admits a v4 destination.
        let v6 = SocketAperture::network(
            IpAddr::V6(std::net::Ipv6Addr::UNSPECIFIED),
            0,
            5_060..=5_060,
        )
        .expect("a v6 aperture");
        assert!(!v6.contains(SocketAddr::from(([10, 0, 0, 7], 5_060))));
        assert!(v6.contains(SocketAddr::new(
            IpAddr::V6(std::net::Ipv6Addr::LOCALHOST),
            5_060
        )));
    }

    #[test]
    fn exact_loopback_route_produces_non_serializable_driver_evidence() {
        let admitted = admit_sip_plan(&plan(), route()).unwrap();
        assert_eq!(admitted.provider(), "loopback-pbx");
        assert_eq!(admitted.operation(), SIP_DIAL_OPERATION);
        assert!(admitted.admits_signaling(([127, 0, 0, 1], 5_060).into()));
        assert!(admitted.admits_media(([127, 0, 0, 1], 16_384).into()));
    }

    #[test]
    fn sip_dial_resolves_only_an_exact_connection_owned_alias() {
        let routes =
            SipDialRouteTable::new("connection", [("asterisk-dev".to_owned(), route())]).unwrap();
        let admitted = admit_sip_dial(
            &plan(),
            &SipDialInput {
                target: Some("asterisk-dev".to_owned()),
                number: None,
            },
            &routes,
            &NoHostResolution,
        )
        .unwrap();
        assert_eq!(
            admitted.route().target.address().map(|target| target.port()),
            Some(5_060)
        );

        for target in ["missing", "sip:echo@127.0.0.1:5062", "127.0.0.1:5062"] {
            assert!(admit_sip_dial(
                &plan(),
                &SipDialInput {
                    target: Some(target.to_owned()),
                    number: None,
                },
                &routes,
                &NoHostResolution,
            )
            .is_err());
        }
    }

    #[test]
    fn stable_network_and_aperture_widening_refuse_before_the_driver() {
        let mut stable = route();
        stable.network_mode = SipNetworkMode::Loopback;
        stable.target =
            SipSignalingTarget::Address(SocketAddrV4::new(Ipv4Addr::new(10, 0, 0, 1), 5_060).into());
        stable.signaling_apertures =
            vec![
                SocketAperture::new(IpAddr::V4(Ipv4Addr::new(10, 0, 0, 1)), 5_060..=5_060).unwrap(),
            ];
        assert!(matches!(
            admit_sip_plan(&plan(), stable),
            Err(SipAdmissionError::NetworkModeMismatch)
        ));

        let mut characterized = route();
        characterized.network_mode = SipNetworkMode::OperatorAuthorizedDevelopment;
        characterized.target = SipSignalingTarget::Address(
            SocketAddrV4::new(Ipv4Addr::new(10, 0, 0, 1), 5_060).into(),
        );
        characterized.signaling_apertures =
            vec![
                SocketAperture::new(IpAddr::V4(Ipv4Addr::new(10, 0, 0, 1)), 5_060..=5_060).unwrap(),
            ];
        assert!(admit_sip_plan(&plan(), characterized).is_ok());

        let mut outside = route();
        outside.signaling_apertures =
            vec![SocketAperture::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 5_061..=5_061).unwrap()];
        assert!(matches!(
            admit_sip_plan(&plan(), outside),
            Err(SipAdmissionError::SignalingTargetRefused)
        ));

        let mut advertised_outside = route();
        advertised_outside.media_advertised = IpAddr::V4(Ipv4Addr::new(127, 0, 0, 2));
        assert!(matches!(
            admit_sip_plan(&plan(), advertised_outside),
            Err(SipAdmissionError::MediaAdvertisedRefused)
        ));
    }

    #[test]
    fn zero_or_excessive_dial_deadlines_refuse_before_the_driver() {
        for dial_timeout in [
            Duration::ZERO,
            MAX_SIP_DIAL_TIMEOUT + Duration::from_secs(1),
        ] {
            let mut invalid = route();
            invalid.dial_timeout = dial_timeout;
            assert!(matches!(
                admit_sip_plan(&plan(), invalid),
                Err(SipAdmissionError::InvalidDeadline)
            ));
        }

        let mut exact_maximum = route();
        exact_maximum.dial_timeout = MAX_SIP_DIAL_TIMEOUT;
        assert!(admit_sip_plan(&plan(), exact_maximum).is_ok());
    }

    #[test]
    fn missing_organization_in_grant_evidence_refuses_before_the_driver() {
        assert!(matches!(
            admit_sip_plan(&plan_for_organization(""), route()),
            Err(SipAdmissionError::InvalidIdentity)
        ));
    }
}
