#![forbid(unsafe_code)]

//! Personal-local daemon composition and the first catalog-backed operation backend.

mod composite_backend;
mod config;
mod hosted;
mod hosted_config;
mod hosted_vault;
mod kubernetes_backend;
mod kubernetes_local_backend;
mod monitoring_backend;
mod runtime;
mod sip_backend;
mod slack_backend;

pub use composite_backend::{CompositeBackend, CredentialStoreBackend};
pub use config::{
    AuthorityConfig, ConfigError, GrafanaIntegrationConfig, InitiationConfig,
    KubernetesIntegrationConfig, PersonalConfig, PersonalVoiceConfig, SlackIntegrationConfig,
};
pub use hosted::{IdentityHttpVerifier, IdentityVerifierConfigError};
pub use hosted_config::{HostedServerConfig, HostedServerConfigError};
pub use hosted_vault::{HostedVaultError, HostedVaultStore};
pub use kubernetes_backend::{KubernetesBackendError, KubernetesStatusBackend};
pub use kubernetes_local_backend::{KubernetesLocalBackend, KubernetesLocalError};
pub use monitoring_backend::{MonitoringBackend, MonitoringError};
pub use runtime::{load_authority_issuer, RuntimeLauncher};
pub use sip_backend::{
    LaunchError, LaunchedSession, RefusingBackend, SessionLauncher, SipOperationBackend,
};
pub use slack_backend::{SlackBackend, SlackError};
