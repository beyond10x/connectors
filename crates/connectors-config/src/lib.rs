#![forbid(unsafe_code)]

//! Value-free Connector deployment configuration.

mod file;
mod hosted;
mod personal;

pub use hosted::{
    HostedAuthorityConfig, HostedGitlabConfig, HostedGrafanaConfig, HostedGrafanaTargetConfig,
    HostedIdentityConfig, HostedKubernetesConfig, HostedListenerConfig, HostedServerConfig,
    HostedServerConfigError, HostedSipConfig, HostedSipCredentialConfig, HostedSlackConfig,
    HostedStorageConfig, HostedVaultConfig, KubernetesNamespaceAccessConfig,
};
pub use personal::{
    AudioIntegrationConfig, AuthorityConfig, BrowserIntegrationConfig, ConfigError,
    ConnectionConfig, B10xConnectionConfig, B10xIntegrationConfig,
    GrafanaIntegrationConfig, InitiationConfig, KubernetesIntegrationConfig, PersonalConfig,
    PersonalVoiceConfig, SlackIntegrationConfig,
};
