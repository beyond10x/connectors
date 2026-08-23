#![forbid(unsafe_code)]

//! Value-free Connector deployment configuration.

mod file;
mod hosted;
mod personal;

pub use hosted::{
    HostedAuthorityConfig, HostedGitlabConfig, HostedGrafanaConfig, HostedGrafanaTargetConfig,
    HostedIdentityConfig, HostedJiraConfig, HostedKubernetesConfig, HostedListenerConfig,
    HostedServerConfig, HostedServerConfigError, HostedSipConfig, HostedSipCredentialConfig,
    HostedSlackConfig, HostedStorageConfig, HostedVaultConfig, JiraSharedAuth,
    KubernetesNamespaceAccessConfig,
};
pub use personal::{
    AudioIntegrationConfig, AuthorityConfig, BrowserIntegrationConfig, CatalogIntegrationConfig,
    ConfigError, ConnectionConfig, B10xConnectionConfig, B10xIntegrationConfig,
    GrafanaIntegrationConfig, InitiationConfig, KubernetesIntegrationConfig, NetworkScopeConfig,
    OwnerConfig, PersonalConfig, PersonalVoiceConfig, SlackInstanceConfig, SlackInstanceProfile,
    SlackIntegrationConfig,
};
