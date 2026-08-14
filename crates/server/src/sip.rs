//! Server-owned admission of the exact network facts a SIP driver may consume.

use std::net::{IpAddr, SocketAddr};
use std::ops::RangeInclusive;
use std::time::Duration;

use domain::{DriverId, Interaction, ProtocolPlan, ZeroIoPlan};

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
    pub to_uri: String,
    pub from_uri: String,
    pub media_advertised: IpAddr,
    pub media_bind: IpAddr,
    pub signaling_apertures: Vec<SocketAperture>,
    pub media_apertures: Vec<SocketAperture>,
    pub dial_timeout: Duration,
    pub development_loopback_only: bool,
}

/// Failure before the socket-capable crate receives a plan.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum SipAdmissionError {
    #[error("operation is not an admitted SIP session establishment")]
    WrongOperation,
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
    #[error("the first sipx integration is restricted to explicit loopback development")]
    StableNetworkNotYetSupported,
    #[error("SIP route has an invalid finite deadline")]
    InvalidDeadline,
}

/// Non-serializable evidence handed only to the socket-capable `driver-sip` crate.
pub struct AdmittedSipPlan {
    provider: String,
    operation: String,
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
    if sip.connection != route.connection || plan.admission().connection() != route.connection {
        return Err(SipAdmissionError::ConnectionMismatch);
    }
    if route.dial_timeout.is_zero() {
        return Err(SipAdmissionError::InvalidDeadline);
    }
    if !route.development_loopback_only
        || !route.target.ip().is_loopback()
        || !route.signaling_bind.ip().is_loopback()
        || !route.media_advertised.is_loopback()
        || !route.media_bind.is_loopback()
    {
        return Err(SipAdmissionError::StableNetworkNotYetSupported);
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
    Ok(AdmittedSipPlan {
        provider: plan.facts().provider.clone(),
        operation: plan.facts().operation.clone(),
        principal: plan.admission().principal().to_owned(),
        grant: plan.admission().grant().to_owned(),
        route,
        _proof: AdmissionProof,
    })
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;
    use std::net::{Ipv4Addr, SocketAddrV4};

    use domain::{
        AdmittedOperation, Capability, Implementation, OperationFacts, Placement, SipPlan,
    };

    use super::*;

    fn plan() -> ZeroIoPlan {
        ZeroIoPlan::new(
            OperationFacts {
                provider: "loopback-pbx".to_owned(),
                operation: "call-establish".to_owned(),
                service: "voice".to_owned(),
                interaction: Interaction::SessionEstablishment,
                placement: Placement::ConnectorsDeployment,
                implementation: Implementation::BuiltIn,
                required_capabilities: BTreeSet::from([Capability::PrivateNetwork]),
                permission_subjects: vec!["loopback:127.0.0.1".to_owned()],
            },
            AdmittedOperation::from_grant_decision(
                "loopback-pbx",
                "call-establish",
                "principal",
                "grant",
                "connection",
            ),
            ProtocolPlan::SipV1(SipPlan {
                connection: "connection".to_owned(),
            }),
        )
    }

    fn route() -> SipDeploymentRoute {
        let loopback = IpAddr::V4(Ipv4Addr::LOCALHOST);
        SipDeploymentRoute {
            connection: "connection".to_owned(),
            signaling_bind: SocketAddrV4::new(Ipv4Addr::LOCALHOST, 0).into(),
            sent_by: "127.0.0.1".to_owned(),
            target: SocketAddrV4::new(Ipv4Addr::LOCALHOST, 5_060).into(),
            to_uri: "sip:callee@127.0.0.1:5060".to_owned(),
            from_uri: "sip:caller@127.0.0.1".to_owned(),
            media_advertised: loopback,
            media_bind: loopback,
            signaling_apertures: vec![SocketAperture::new(loopback, 1..=u16::MAX).unwrap()],
            media_apertures: vec![SocketAperture::new(loopback, 1..=u16::MAX).unwrap()],
            dial_timeout: Duration::from_secs(5),
            development_loopback_only: true,
        }
    }

    #[test]
    fn exact_loopback_route_produces_non_serializable_driver_evidence() {
        let admitted = admit_sip_plan(&plan(), route()).unwrap();
        assert_eq!(admitted.provider(), "loopback-pbx");
        assert_eq!(admitted.operation(), "call-establish");
        assert!(admitted.admits_signaling(([127, 0, 0, 1], 5_060).into()));
        assert!(admitted.admits_media(([127, 0, 0, 1], 16_384).into()));
    }

    #[test]
    fn stable_network_and_aperture_widening_refuse_before_the_driver() {
        let mut stable = route();
        stable.development_loopback_only = false;
        assert!(matches!(
            admit_sip_plan(&plan(), stable),
            Err(SipAdmissionError::StableNetworkNotYetSupported)
        ));

        let mut outside = route();
        outside.signaling_apertures =
            vec![SocketAperture::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 5_061..=5_061).unwrap()];
        assert!(matches!(
            admit_sip_plan(&plan(), outside),
            Err(SipAdmissionError::SignalingTargetRefused)
        ));
    }
}
