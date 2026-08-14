//! Server-owned admission of the exact network facts a SIP driver may consume.

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
    ports: RangeInclusive<u16>,
}

impl SocketAperture {
    pub fn new(address: IpAddr, ports: RangeInclusive<u16>) -> Result<Self, SipAdmissionError> {
        if ports.is_empty() || ports.contains(&0) {
            return Err(SipAdmissionError::InvalidAperture);
        }
        Ok(Self { address, ports })
    }

    #[must_use]
    pub fn contains(&self, target: SocketAddr) -> bool {
        target.ip() == self.address && self.ports.contains(&target.port())
    }

    #[must_use]
    pub fn contains_ip(&self, target: IpAddr) -> bool {
        target == self.address
    }
}

/// Deployment-selected route. No request or model field can construct this from wire input.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SipDeploymentRoute {
    pub connection: String,
    pub signaling_bind: SocketAddr,
    pub sent_by: String,
    pub target: SocketAddr,
    pub signaling_transport: SipSignalingTransport,
    pub to_uri: String,
    pub from_uri: String,
    pub media_advertised: IpAddr,
    pub media_bind: IpAddr,
    pub signaling_apertures: Vec<SocketAperture>,
    pub media_apertures: Vec<SocketAperture>,
    pub dial_timeout: Duration,
    pub network_mode: SipNetworkMode,
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
}

/// Deployment-owned alias table. Callers can select a name but cannot construct any route field.
#[derive(Debug, Clone)]
pub struct SipDialRouteTable {
    connection: String,
    routes: BTreeMap<String, SipDeploymentRoute>,
}

impl SipDialRouteTable {
    pub fn new<I>(connection: impl Into<String>, routes: I) -> Result<Self, SipAdmissionError>
    where
        I: IntoIterator<Item = (String, SipDeploymentRoute)>,
    {
        let connection = connection.into();
        let mut admitted = BTreeMap::new();
        for (alias, route) in routes {
            SipDialInput {
                target: alias.clone(),
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
        Ok(Self {
            connection,
            routes: admitted,
        })
    }

    #[must_use]
    pub fn connection(&self) -> &str {
        &self.connection
    }

    fn resolve(&self, input: &SipDialInput) -> Result<SipDeploymentRoute, SipAdmissionError> {
        input
            .validate()
            .map_err(|_| SipAdmissionError::InvalidTargetAlias)?;
        self.routes
            .get(&input.target)
            .cloned()
            .ok_or(SipAdmissionError::UnknownTargetAlias)
    }
}

/// Resolve the caller's opaque alias and produce socket-opening evidence for `sip.dial` only.
pub fn admit_sip_dial(
    plan: &ZeroIoPlan,
    input: &SipDialInput,
    routes: &SipDialRouteTable,
) -> Result<AdmittedSipPlan, SipAdmissionError> {
    if plan.facts().operation != SIP_DIAL_OPERATION
        || plan.admission().connection() != routes.connection()
    {
        return Err(SipAdmissionError::WrongOperation);
    }
    admit_sip_plan(plan, routes.resolve(input)?)
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
        && (!route.target.ip().is_loopback()
            || !route.signaling_bind.ip().is_loopback()
            || !route.media_advertised.is_loopback()
            || !route.media_bind.is_loopback())
    {
        return Err(SipAdmissionError::NetworkModeMismatch);
    }
    if !route
        .signaling_apertures
        .iter()
        .any(|aperture| aperture.contains(route.target))
    {
        return Err(SipAdmissionError::SignalingTargetRefused);
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
            target: SocketAddrV4::new(Ipv4Addr::LOCALHOST, 5_060).into(),
            signaling_transport: SipSignalingTransport::Udp,
            to_uri: "sip:callee@127.0.0.1:5060".to_owned(),
            from_uri: "sip:caller@127.0.0.1".to_owned(),
            media_advertised: loopback,
            media_bind: loopback,
            signaling_apertures: vec![SocketAperture::new(loopback, 1..=u16::MAX).unwrap()],
            media_apertures: vec![SocketAperture::new(loopback, 1..=u16::MAX).unwrap()],
            dial_timeout: Duration::from_secs(5),
            network_mode: SipNetworkMode::Loopback,
        }
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
                target: "asterisk-dev".to_owned(),
            },
            &routes,
        )
        .unwrap();
        assert_eq!(admitted.route().target.port(), 5_060);

        for target in ["missing", "sip:echo@127.0.0.1:5062", "127.0.0.1:5062"] {
            assert!(admit_sip_dial(
                &plan(),
                &SipDialInput {
                    target: target.to_owned(),
                },
                &routes,
            )
            .is_err());
        }
    }

    #[test]
    fn stable_network_and_aperture_widening_refuse_before_the_driver() {
        let mut stable = route();
        stable.network_mode = SipNetworkMode::Loopback;
        stable.target = SocketAddrV4::new(Ipv4Addr::new(10, 0, 0, 1), 5_060).into();
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
        characterized.target = SocketAddrV4::new(Ipv4Addr::new(10, 0, 0, 1), 5_060).into();
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
