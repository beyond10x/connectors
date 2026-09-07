#![forbid(unsafe_code)]

//! Pure Connectors use cases.

mod admin;
mod audio;
pub mod authority;
mod browser;
mod connect_session;
mod dispatch;
mod egress;
mod endpoint;
pub use endpoint::{
    constrain_endpoint_description, normalize_endpoint_operation, NormalizedEndpointOperation,
};
mod rate_limit;
pub use rate_limit::operation_rate_advice;
mod factory;
mod git_fetch;
mod planning;
mod remediation;
pub use remediation::{
    CredentialReadiness, RemediationAdmission, RemediationAuthority, RemediationBinding,
    RemediationError, RemediationMetadata, RemediationRequest, RemediationResult, RemediationRoute,
    RemediationTarget,
};
mod runtime;
mod sip;
mod voice;

pub use admin::{
    AdminConfigurationField, AdminCredentialInput, AdminCredentialRequirement,
    AdminCredentialStatus, AdminError, AdminIntegration, AdminIntegrationStatus, AdminRegistry,
    AdminStatus, CredentialState,
};
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
pub use egress::{
    retry_after_seconds, EgressByteStream, EgressHttpRequest, EgressHttpResponse,
    EgressStreamingHttpRequest, EgressStreamingHttpResponse, EgressTransport, EgressTransportError,
    EgressTransportFailure, EgressWebSocket, EgressWebSocketFrame,
};
pub use factory::{
    ConnectorServiceFactory, DeploymentApproval, DeploymentRisk, OperationDeployment,
    OperationEffect, ProviderIdentity, ServiceDeployment, ServiceDispatch, ServiceFactoryBindError,
    ServiceManifest, ServiceOperation, ServiceProviderMetadata,
};
pub use git_fetch::{
    GitFetchAccess, GitFetchBroker, GitFetchControlError, GitFetchDataError, GitFetchExchange,
    GitFetchExchangeResponse, GitFetchGrant, GitFetchService,
};
pub use planning::{plan_operation, PlanError, PlanningEnvironment};
pub use runtime::{
    BackendCapabilities, BackendReadinessError, ConnectSessionAccess, ConnectorBackend,
    DelegatedExecution, HostedCompletionError, HostedCompletionPage, HostedCompletionSubmission,
    PrincipalContext, PrincipalContextError, PrincipalIdentity,
};
pub use sip::{
    admit_sip_dial, admit_sip_plan, validate_sip_deployment_route, AdmittedSipPlan,
    FixedHostResolution, NoHostResolution, SipAdmissionError, SipDeploymentRoute,
    SipDialRouteTable, SipHostResolver, SipNetworkMode, SipSignalingTarget, SipSignalingTransport,
    SocketAperture,
};
pub use voice::{
    admit_voice_dial, admit_voice_plan, validate_voice_application_route, AdmittedVoicePlan,
    VoiceAdmissionError, VoiceApplicationRoute, VOICE_APPLICATION_PROFILE,
};
