//! Server-owned admission for one SIP-to-application voice session.

use std::time::Duration;

use domain::ZeroIoPlan;
use protocol::sip::SipDialInput;

use crate::authority::{validate_endpoint, MAX_AUTHORITY_LIFETIME_SECONDS};
use crate::{
    admit_sip_dial, admit_sip_plan, AdmittedSipPlan, SipAdmissionError, SipDeploymentRoute,
    SipDialRouteTable,
};

/// Exact RTVBP binding selected by the server-owned voice composition path.
pub const VOICE_APPLICATION_PROFILE: &str = "b10x.voice.v1";

/// Deployment-selected application route. None of these values comes from a call or model.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VoiceApplicationRoute {
    pub actor: String,
    pub audience: String,
    pub deployment: String,
    pub resource: String,
    pub endpoint: String,
    pub authority_lifetime: Duration,
    pub session_lease: Duration,
}

/// Failure before either socket-capable adapter receives a plan.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum VoiceAdmissionError {
    #[error(transparent)]
    Sip(#[from] SipAdmissionError),
    #[error("voice application route contains an empty server identity")]
    EmptyIdentity,
    #[error("voice application endpoint is not an exact absolute wss URI")]
    InvalidEndpoint,
    #[error("voice authority lifetime must be an integral 1..=60 seconds")]
    InvalidAuthorityLifetime,
    #[error("voice session lease must be integral and outlive establishment authority")]
    InvalidSessionLease,
}

/// Non-serializable evidence for the one supervised voice runtime composition point.
pub struct AdmittedVoicePlan {
    sip: AdmittedSipPlan,
    application: VoiceApplicationRoute,
    _proof: VoiceAdmissionProof,
}

struct VoiceAdmissionProof;

impl AdmittedVoicePlan {
    #[must_use]
    pub fn sip(&self) -> &AdmittedSipPlan {
        &self.sip
    }

    #[must_use]
    pub fn application(&self) -> &VoiceApplicationRoute {
        &self.application
    }
}

impl std::fmt::Debug for AdmittedVoicePlan {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("AdmittedVoicePlan")
            .field("sip", &self.sip)
            .field("application", &self.application)
            .finish_non_exhaustive()
    }
}

/// Join grant admission with the exact SIP and application routes before any network I/O.
pub fn admit_voice_plan(
    plan: &ZeroIoPlan,
    sip_route: SipDeploymentRoute,
    application: VoiceApplicationRoute,
) -> Result<AdmittedVoicePlan, VoiceAdmissionError> {
    validate_application(&application)?;
    let sip = admit_sip_plan(plan, sip_route)?;
    Ok(AdmittedVoicePlan {
        sip,
        application,
        _proof: VoiceAdmissionProof,
    })
}

/// Resolve a `sip.dial` alias and join it to the exact application route before network I/O.
pub fn admit_voice_dial(
    plan: &ZeroIoPlan,
    input: &SipDialInput,
    sip_routes: &SipDialRouteTable,
    application: VoiceApplicationRoute,
) -> Result<AdmittedVoicePlan, VoiceAdmissionError> {
    validate_application(&application)?;
    let sip = admit_sip_dial(plan, input, sip_routes)?;
    Ok(AdmittedVoicePlan {
        sip,
        application,
        _proof: VoiceAdmissionProof,
    })
}

fn validate_application(application: &VoiceApplicationRoute) -> Result<(), VoiceAdmissionError> {
    if [
        application.actor.as_str(),
        application.audience.as_str(),
        application.deployment.as_str(),
        application.resource.as_str(),
    ]
    .into_iter()
    .any(str::is_empty)
    {
        return Err(VoiceAdmissionError::EmptyIdentity);
    }
    validate_endpoint(&application.endpoint).map_err(|_| VoiceAdmissionError::InvalidEndpoint)?;
    if application.authority_lifetime.is_zero()
        || application.authority_lifetime.as_secs() > MAX_AUTHORITY_LIFETIME_SECONDS
        || application.authority_lifetime.subsec_nanos() != 0
    {
        return Err(VoiceAdmissionError::InvalidAuthorityLifetime);
    }
    if application.session_lease < application.authority_lifetime
        || application.session_lease.subsec_nanos() != 0
    {
        return Err(VoiceAdmissionError::InvalidSessionLease);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;
    use std::net::{IpAddr, Ipv4Addr, SocketAddr};

    use domain::{
        AdmittedOperation, Capability, ConnectionAuthority, Implementation, InitiationPolicy,
        Interaction, OperationFacts, Placement, ProtocolPlan, SipPlan,
    };

    use super::*;
    use crate::SocketAperture;

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
                "org",
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

    fn sip_route() -> SipDeploymentRoute {
        let loopback = IpAddr::V4(Ipv4Addr::LOCALHOST);
        let aperture = SocketAperture::new(loopback, 1..=u16::MAX).unwrap();
        SipDeploymentRoute {
            connection: "connection".to_owned(),
            signaling_bind: SocketAddr::new(loopback, 0),
            sent_by: "127.0.0.1".to_owned(),
            target: SocketAddr::new(loopback, 5_060),
            signaling_transport: crate::SipSignalingTransport::Udp,
            to_uri: "sip:callee@127.0.0.1:5060".to_owned(),
            from_uri: "sip:caller@127.0.0.1".to_owned(),
            media_advertised: loopback,
            media_bind: loopback,
            signaling_apertures: vec![aperture.clone()],
            media_apertures: vec![aperture],
            dial_timeout: Duration::from_secs(5),
            network_mode: crate::SipNetworkMode::Loopback,
        }
    }

    fn application_route() -> VoiceApplicationRoute {
        VoiceApplicationRoute {
            actor: "connectors".to_owned(),
            audience: "application".to_owned(),
            deployment: "application-1".to_owned(),
            resource: "channel-1".to_owned(),
            endpoint: "wss://application.example/voice".to_owned(),
            authority_lifetime: Duration::from_secs(30),
            session_lease: Duration::from_secs(60),
        }
    }

    #[test]
    fn one_proof_joins_exact_sip_and_application_routes() {
        let admitted = admit_voice_plan(&plan(), sip_route(), application_route()).unwrap();
        assert_eq!(admitted.sip().route().connection, "connection");
        assert_eq!(admitted.sip().organization(), "org");
        assert_eq!(
            admitted.application().endpoint,
            "wss://application.example/voice"
        );
    }

    #[test]
    fn invalid_endpoint_and_authority_windows_refuse_before_io() {
        let mut route = application_route();
        route.endpoint = "ws://application.example/voice".to_owned();
        assert!(matches!(
            admit_voice_plan(&plan(), sip_route(), route),
            Err(VoiceAdmissionError::InvalidEndpoint)
        ));

        let mut route = application_route();
        route.authority_lifetime = Duration::from_secs(61);
        assert!(matches!(
            admit_voice_plan(&plan(), sip_route(), route),
            Err(VoiceAdmissionError::InvalidAuthorityLifetime)
        ));
    }
}
