#![forbid(unsafe_code)]

//! Value-free Connector deployment configuration.

mod file;
mod hosted;
mod personal;

pub use hosted::{
    HostedIdentityConfig, HostedKubernetesConfig, HostedListenerConfig, HostedServerConfig,
    HostedServerConfigError, HostedSipConfig, HostedSipCredentialConfig, HostedStorageConfig,
    HostedVaultConfig,
};
pub use personal::{
    AuthorityConfig, ConfigError, GrafanaIntegrationConfig, InitiationConfig,
    KubernetesIntegrationConfig, PersonalConfig, PersonalVoiceConfig, SlackIntegrationConfig,
};
