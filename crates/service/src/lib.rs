#![forbid(unsafe_code)]

//! Pure Connectors use cases.

pub mod authority;
mod connect_session;
mod dispatch;
mod planning;
mod runtime;
mod sip;
mod voice;

pub use connect_session::{
    ConnectSessionLifecycle, ConnectSessionLifecycleError, ConnectSessionTerminal,
};
pub use dispatch::{
    AuditSink, BuiltInDriver, CredentialSet, DispatchError, DispatchPolicy, DispatchResult,
    Dispatcher, SensitiveValue,
};
pub use planning::{plan_operation, PlanError, PlanningEnvironment};
pub use runtime::{
    BackendCapabilities, ConnectorBackend, PrincipalContext, PrincipalContextError,
    PrincipalIdentity,
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
