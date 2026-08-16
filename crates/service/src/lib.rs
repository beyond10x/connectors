#![forbid(unsafe_code)]

//! Pure Connectors use cases.

mod audio;
pub mod authority;
mod browser;
mod connect_session;
mod dispatch;
mod planning;
mod runtime;
mod sip;
mod voice;

pub use audio::{
    admit_audio_plan, admit_speech_speak, validate_audio_deployment_route, AdmittedAudioPlan,
    AudioAdmissionError, AudioDeploymentRoute, MAX_UTTERANCE, MAX_UTTERANCES_PER_CONNECTION,
};
pub use browser::{
    admit_browser_address, admit_browser_plan, validate_browser_deployment_route,
    AdmittedBrowserPlan, BrowserAdmissionError, BrowserDeploymentRoute, MAX_NAVIGATION,
    MAX_NAVIGATIONS_PER_CONNECTION, MAX_SCREENSHOTS_PER_CONNECTION,
};
pub use connect_session::{
    ConnectSessionLifecycle, ConnectSessionLifecycleError, ConnectSessionTerminal,
};
pub use dispatch::{
    AuditSink, BuiltInDriver, CredentialSet, DispatchError, DispatchPolicy, DispatchResult,
    Dispatcher, SensitiveValue,
};
pub use planning::{plan_operation, PlanError, PlanningEnvironment};
pub use runtime::{
    BackendCapabilities, ConnectorBackend, HostedCompletionError, HostedCompletionPage,
    HostedCompletionSubmission, PrincipalContext, PrincipalContextError, PrincipalIdentity,
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
