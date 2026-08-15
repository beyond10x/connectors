#![forbid(unsafe_code)]

//! Connectors server composition and direct-session authority.

pub mod authority;
mod dispatch;
pub mod hosted;
#[cfg(unix)]
pub mod local;
mod sip;
mod voice;

pub use dispatch::{
    AuditSink, BuiltInDriver, CredentialSet, DispatchError, DispatchPolicy, DispatchResult,
    Dispatcher, SensitiveValue,
};
pub use sip::{
    admit_sip_dial, admit_sip_plan, validate_sip_deployment_route, AdmittedSipPlan,
    SipAdmissionError, SipDeploymentRoute, SipDialRouteTable, SipNetworkMode,
    SipSignalingTransport, SocketAperture,
};
pub use voice::{
    admit_voice_dial, admit_voice_plan, validate_voice_application_route, AdmittedVoicePlan,
    VoiceAdmissionError, VoiceApplicationRoute, VOICE_APPLICATION_PROFILE,
};
