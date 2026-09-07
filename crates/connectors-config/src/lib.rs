#![forbid(unsafe_code)]

//! Value-free Connector deployment configuration.

mod file;
mod hosted;
mod hosted_catalog;
mod hosted_git_fetch;
mod personal;
mod personal_oauth;
#[cfg(test)]
mod personal_oauth_tests;

pub use hosted::{
    HostedAuthorityConfig, HostedGrafanaConfig, HostedGrafanaTargetConfig, HostedIdentityConfig,
    HostedJiraConfig, HostedKubernetesConfig, HostedListenerConfig, HostedSecretsConfig,
    HostedServerConfig, HostedServerConfigError, HostedSipConfig, HostedSipCredentialConfig,
    HostedSlackConfig, HostedStorageConfig, HostedVaultConfig, JiraSharedAuth,
    KubernetesNamespaceAccessConfig,
};
pub use hosted_catalog::{HostedCatalogBinding, HostedCatalogConfig};
pub use hosted_git_fetch::{HostedGitFetchOverride, HostedGitlabConfig, HostedTlsListenerConfig};
pub use personal::{
    AudioIntegrationConfig, AuthorityConfig, BrowserIntegrationConfig, CatalogIntegrationConfig,
    ConfigError, ConnectionConfig, GrafanaIntegrationConfig, InitiationConfig,
    KubernetesIntegrationConfig, NetworkScopeConfig, OwnerConfig, PersonalConfig,
    PersonalVoiceConfig, PlatformConnectionConfig, PlatformIntegrationConfig, SlackInstanceConfig,
    SlackInstanceProfile, SlackIntegrationConfig,
};
pub use personal_oauth::{
    OAuthBrowserPlacement, OAuthClientAuthentication, OAuthRegistrationUse, PersonalOAuthCustody,
    PersonalOAuthFlow, PersonalOAuthRegistration,
};
