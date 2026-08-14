#![forbid(unsafe_code)]

//! Connectors server composition and direct-session authority.

pub mod authority;
mod dispatch;
mod sip;
mod voice;

pub use dispatch::{
    AuditSink, BuiltInDriver, CredentialSet, DispatchError, DispatchPolicy, DispatchResult,
    Dispatcher, SensitiveValue,
};
pub use sip::{
    admit_sip_dial, admit_sip_plan, AdmittedSipPlan, SipAdmissionError, SipDeploymentRoute,
    SipDialRouteTable, SipNetworkMode, SipSignalingTransport, SocketAperture,
};
pub use voice::{
    admit_voice_dial, admit_voice_plan, AdmittedVoicePlan, VoiceAdmissionError,
    VoiceApplicationRoute, VOICE_APPLICATION_PROFILE,
};
