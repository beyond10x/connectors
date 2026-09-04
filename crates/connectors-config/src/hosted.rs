//! Strict, value-free hosted server configuration.

use std::net::SocketAddr;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use connector_address::credential::CredentialRef;

use crate::hosted_git_fetch::{HostedGitlabConfig, HostedTlsListenerConfig};
use crate::personal::{InitiationConfig, PlatformIntegrationConfig, SlackIntegrationConfig};

pub(crate) const MAX_CONFIG_BYTES: u64 = 256 * 1024;
const NATIVE_SIP_AUTHORITY: &str = "io.b10x";
const NATIVE_SIP_SERVICE: &str = "default";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HostedServerConfig {
    pub tenant_id: String,
    /// Tenants admitted to the partitioned module backend. Empty means only `tenant_id`.
    #[serde(default)]
    pub module_tenant_ids: Vec<String>,
    pub server: HostedListenerConfig,
    pub identity: HostedIdentityConfig,
    #[serde(default)]
    pub authority: HostedAuthorityConfig,
    pub storage: HostedStorageConfig,
    #[serde(default)]
    pub egress: HostedEgressConfig,
    pub kubernetes: HostedKubernetesConfig,
    #[serde(default)]
    pub grafana: HostedGrafanaConfig,
    #[serde(default)]
    pub vault: HostedVaultConfig,
    #[serde(default)]
    pub secrets: HostedSecretsConfig,
    #[serde(default)]
    pub claude_code: HostedClaudeCodeConfig,
    #[serde(default)]
    pub catalog: HostedCatalogConfig,
    pub sip: HostedSipConfig,
    #[serde(default)]
    pub slack: Option<HostedSlackConfig>,
    #[serde(default)]
    pub gitlab: Option<HostedGitlabConfig>,
    #[serde(default)]
    pub jira: Option<HostedJiraConfig>,
    #[serde(default, alias = "b10x")]
    pub platform: Option<PlatformIntegrationConfig>,
}

/// Deployment acknowledgement for the transport invariant enforced by credential-bearing
/// Integrations. Unknown or omitted policies remain disabled so older configuration fails closed.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HostedEgressConfig {
    #[serde(default)]
    pub policy: HostedEgressPolicy,
}

/// Enables Connector-owned custody for a person's Claude Code subscription credential. The
/// credential value is never configuration; enabling this requires the hosted Vault store.
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HostedClaudeCodeConfig {
    #[serde(default)]
    pub enabled: bool,
}

/// Generic, catalog-driven self-service connections.
///
/// The provider list is deployment policy, not a second catalog: every credential name, address,
/// request template and destination still comes from the compiled Connector catalog. An empty
/// list admits every catalog provider whose credential explicitly declares Connect Session entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HostedCatalogConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub public_origin: Option<String>,
    #[serde(default)]
    pub grant_ref: Option<String>,
    #[serde(default)]
    pub providers: Vec<String>,
    #[serde(default = "default_connect_session_ttl_seconds")]
    pub connect_session_ttl_seconds: u64,
}

impl Default for HostedCatalogConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            public_origin: None,
            grant_ref: None,
            providers: Vec::new(),
            connect_session_ttl_seconds: default_connect_session_ttl_seconds(),
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HostedEgressPolicy {
    #[default]
    Disabled,
    ConnectionBoundPostDnsV1,
}

/// Value-free hosted Jira Cloud policy. Organization credentials are deployment-owned and may
/// serve only the bounded issue datasource; delegated credentials are principal-owned and may
/// additionally invoke the curated operation surface.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HostedJiraConfig {
    pub cloud_id: String,
    pub site_origin: String,
    pub public_origin: String,
    pub allowed_project_keys: Vec<String>,
    pub shared_auth: JiraSharedAuth,
    #[serde(default)]
    pub service_oauth_client_id: Option<String>,
    pub user_oauth_client_id: String,
    pub oauth_redirect_uri: String,
    pub organization_read_grant_ref: String,
    pub user_grant_ref: String,
    pub initiation: InitiationConfig,
    #[serde(default = "default_connect_session_ttl_seconds")]
    pub connect_session_ttl_seconds: u64,
    #[serde(default = "default_jira_refresh_skew_seconds")]
    pub refresh_skew_seconds: u64,
}

/// Deployment-selected organization credential kind. Values remain in the Secret Store.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum JiraSharedAuth {
    ServiceOauth,
    ServiceApiToken,
}

/// Value-free hosted Slack policy. Operator registration credentials arrive through the admin API;
/// principal-owned credentials arrive through Connect Sessions. All use the configured SecretStore.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HostedSlackConfig {
    pub public_origin: String,
    pub team_id: String,
    #[serde(default)]
    pub oauth_client_id: Option<String>,
    #[serde(default)]
    pub oauth_redirect_uri: Option<String>,
    pub org_read_grant_ref: String,
    pub user_grant_ref: String,
    pub companion_grant_ref: String,
    pub initiation: InitiationConfig,
    pub allowed_events: Vec<String>,
    #[serde(default = "default_connect_session_ttl_seconds")]
    pub connect_session_ttl_seconds: u64,
}

impl HostedSlackConfig {
    #[must_use]
    pub fn policy(&self) -> SlackIntegrationConfig {
        SlackIntegrationConfig {
            grant_ref: self.companion_grant_ref.clone(),
            org_read_grant_ref: Some(self.org_read_grant_ref.clone()),
            user_grant_ref: Some(self.user_grant_ref.clone()),
            companion_grant_ref: Some(self.companion_grant_ref.clone()),
            expected_team_id: Some(self.team_id.clone()),
            oauth_client_id: self.oauth_client_id.clone(),
            oauth_redirect_uri: self.oauth_redirect_uri.clone(),
            initiation: self.initiation,
            allowed_events: self.allowed_events.clone(),
            connect_session_ttl_seconds: self.connect_session_ttl_seconds,
            // A hosted receiver acquires every identity through OAuth or an org install; it never
            // reads a credential off a local path, so it declares no instances.
            instances: Vec::new(),
        }
    }
}

/// Receiver-owned mapping from Identity-verified group facts to hosted Connector operators.
/// An empty mapping admits no effect-bearing hosted request.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HostedAuthorityConfig {
    #[serde(default)]
    pub operator_groups: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HostedListenerConfig {
    pub listen: SocketAddr,
    #[serde(default = "default_base_path")]
    pub base_path: String,
    /// Dedicated TLS-only listener for the internal Git byte plane.
    ///
    /// This is intentionally a second socket: the public hosted listener never mounts the
    /// `/internal/git-fetch` routes, even when an ingress is accidentally broadened.
    #[serde(default)]
    pub git_fetch_tls: Option<HostedTlsListenerConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HostedIdentityConfig {
    pub origin: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HostedStorageConfig {
    pub state_root: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HostedKubernetesConfig {
    pub enabled: bool,
    /// Deprecated operator-only compatibility surface. Use `namespace_access` for group grants.
    #[serde(default)]
    pub namespaces: Vec<String>,
    /// Exact namespace and Identity-group grants. Wildcards are deliberately unsupported.
    #[serde(default)]
    pub namespace_access: Vec<KubernetesNamespaceAccessConfig>,
    #[serde(default = "default_kubernetes_token_file")]
    pub token_file: PathBuf,
    #[serde(default = "default_kubernetes_ca_file")]
    pub ca_file: PathBuf,
}

/// Receiver-owned Kubernetes grant for one exact namespace.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct KubernetesNamespaceAccessConfig {
    pub namespace: String,
    #[serde(default)]
    pub read_groups: Vec<String>,
    #[serde(default)]
    pub restart_groups: Vec<String>,
}

/// Deployment-managed, read-only Grafana federation policy. Datasource UIDs remain sealed behind
/// their digests; the Integration resolves the current UID only after reading the exact origin.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HostedGrafanaConfig {
    pub enabled: bool,
    #[serde(default)]
    pub origin: Option<String>,
    #[serde(default)]
    pub connection_ref: Option<String>,
    #[serde(default)]
    pub label: Option<String>,
    #[serde(default)]
    pub grant_ref: Option<String>,
    #[serde(default)]
    pub read_groups: Vec<String>,
    #[serde(default)]
    pub targets: Vec<HostedGrafanaTargetConfig>,
    #[serde(default = "default_grafana_reconcile_interval_seconds")]
    pub reconcile_interval_seconds: u64,
}

impl Default for HostedGrafanaConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            origin: None,
            connection_ref: None,
            label: None,
            grant_ref: None,
            read_groups: Vec::new(),
            targets: Vec::new(),
            reconcile_interval_seconds: default_grafana_reconcile_interval_seconds(),
        }
    }
}

/// One exact Grafana datasource allowed to become a mediated read-only Connection.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HostedGrafanaTargetConfig {
    pub provider: String,
    pub uid_sha256: String,
    pub connection_ref: String,
    pub label: String,
    pub grant_ref: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HostedSipConfig {
    pub enabled: bool,
    #[serde(default)]
    pub listen: Option<SocketAddr>,
    #[serde(default)]
    pub deployment_config: Option<PathBuf>,
    #[serde(default)]
    pub credentials: Option<HostedSipCredentialConfig>,
}

/// Value-free Vault addresses for the optional SIP digest challenge response.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HostedSipCredentialConfig {
    pub authority: String,
    pub service: String,
    pub username_credential: String,
    pub password_credential: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HostedVaultConfig {
    pub enabled: bool,
    #[serde(default)]
    pub address: Option<String>,
    #[serde(default = "default_vault_mount")]
    pub mount: String,
    #[serde(default)]
    pub role: Option<String>,
    #[serde(default)]
    pub token_file: Option<PathBuf>,
    #[serde(default)]
    pub ca_file: Option<PathBuf>,
}

impl Default for HostedVaultConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            address: None,
            mount: default_vault_mount(),
            role: None,
            token_file: None,
            ca_file: None,
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HostedSecretsConfig {
    pub enabled: bool,
    #[serde(default)]
    pub origin: Option<String>,
    #[serde(default)]
    pub token_file: Option<PathBuf>,
    #[serde(default)]
    pub ca_file: Option<PathBuf>,
}

#[derive(Debug, thiserror::Error)]
pub enum HostedServerConfigError {
    #[error("hosted Connector configuration could not be read")]
    Read,
    #[error("hosted Connector configuration is malformed")]
    Parse,
    #[error("hosted Connector configuration is incomplete or inconsistent")]
    Invalid,
    #[error(
        "credential-bearing hosted provider has no enforced Connection-bound post-DNS transport"
    )]
    CredentialEgressUnconfined,
}

impl HostedServerConfig {
    pub fn read(path: &Path) -> Result<Self, HostedServerConfigError> {
        Self::read_with_git_fetch_override(path, None)
    }

    pub(crate) fn validate(&self) -> Result<(), HostedServerConfigError> {
        let mut module_tenants = self.module_tenant_ids.clone();
        module_tenants.sort();
        module_tenants.dedup();
        let vault_complete = self.vault.address.is_some()
            && self.vault.role.is_some()
            && self.vault.token_file.is_some()
            && self.vault.ca_file.is_some();
        let vault_empty = self.vault.address.is_none()
            && self.vault.role.is_none()
            && self.vault.token_file.is_none()
            && self.vault.ca_file.is_none();
        let secrets_complete = self.secrets.origin.is_some() && self.secrets.token_file.is_some();
        let secrets_empty = self.secrets.origin.is_none()
            && self.secrets.token_file.is_none()
            && self.secrets.ca_file.is_none();
        let slack_valid = self.slack.as_ref().is_none_or(|slack| {
            let origin = url::Url::parse(&slack.public_origin);
            origin.is_ok_and(|origin| {
                origin.scheme() == "https"
                    && origin.host_str().is_some()
                    && origin.username().is_empty()
                    && origin.password().is_none()
                    && origin.query().is_none()
                    && origin.fragment().is_none()
                    && origin.path() == self.server.base_path
            }) && slack.policy().validate().is_ok()
        });
        let gitlab_valid = self.gitlab.as_ref().is_none_or(|gitlab| {
            let origin = url::Url::parse(&gitlab.origin);
            let public_origin = url::Url::parse(&gitlab.public_origin);
            let git_fetch_origin = gitlab
                .git_fetch_origin
                .as_deref()
                .map(url::Url::parse)
                .transpose();
            let redirect = url::Url::parse(&gitlab.oauth_redirect_uri);
            let callback_valid = public_origin
                .as_ref()
                .ok()
                .zip(redirect.as_ref().ok())
                .is_some_and(|(public_origin, redirect)| {
                    public_origin.scheme() == "https"
                        && public_origin.host_str().is_some()
                        && public_origin.username().is_empty()
                        && public_origin.password().is_none()
                        && public_origin.query().is_none()
                        && public_origin.fragment().is_none()
                        && public_origin.path() == self.server.base_path
                        && redirect.scheme() == public_origin.scheme()
                        && redirect.host_str() == public_origin.host_str()
                        && redirect.port_or_known_default() == public_origin.port_or_known_default()
                        && redirect.username().is_empty()
                        && redirect.password().is_none()
                        && redirect.query().is_none()
                        && redirect.fragment().is_none()
                        && redirect.path()
                            == format!("{}/oauth/gitlab/callback", self.server.base_path)
                                .replace("//", "/")
                });
            origin.is_ok_and(|origin| valid_https_origin(&origin))
                && callback_valid
                && git_fetch_origin
                    .is_ok_and(|origin| origin.as_ref().is_none_or(valid_https_origin))
                && valid_ref(&gitlab.oauth_client_id, 256)
                && valid_ref(&gitlab.user_grant_ref, 512)
                && (60..=900).contains(&gitlab.connect_session_ttl_seconds)
                && (60..=900).contains(&gitlab.refresh_skew_seconds)
        });
        let git_fetch_tls_valid = match (
            self.gitlab
                .as_ref()
                .and_then(|gitlab| gitlab.git_fetch_origin.as_ref()),
            self.server.git_fetch_tls.as_ref(),
        ) {
            (None, None) => true,
            (Some(_), Some(tls)) => {
                tls.listen != self.server.listen
                    && tls.certificate_file.is_absolute()
                    && tls.private_key_file.is_absolute()
                    && tls.certificate_file != tls.private_key_file
            }
            _ => false,
        };
        let jira_valid = self.jira.as_ref().is_none_or(|jira| {
            let site = url::Url::parse(&jira.site_origin);
            let public_origin = url::Url::parse(&jira.public_origin);
            let redirect = url::Url::parse(&jira.oauth_redirect_uri);
            let callback_valid = public_origin
                .as_ref()
                .ok()
                .zip(redirect.as_ref().ok())
                .is_some_and(|(public_origin, redirect)| {
                    public_origin.scheme() == "https"
                        && public_origin.host_str().is_some()
                        && public_origin.username().is_empty()
                        && public_origin.password().is_none()
                        && public_origin.query().is_none()
                        && public_origin.fragment().is_none()
                        && public_origin.path() == self.server.base_path
                        && redirect.scheme() == public_origin.scheme()
                        && redirect.host_str() == public_origin.host_str()
                        && redirect.port_or_known_default() == public_origin.port_or_known_default()
                        && redirect.username().is_empty()
                        && redirect.password().is_none()
                        && redirect.query().is_none()
                        && redirect.fragment().is_none()
                        && redirect.path()
                            == format!("{}/oauth/jira/callback", self.server.base_path)
                                .replace("//", "/")
                });
            let projects = &jira.allowed_project_keys;
            let mut canonical_projects = projects.clone();
            canonical_projects.sort();
            canonical_projects.dedup();
            site.is_ok_and(|site| {
                valid_https_origin(&site)
                    && site
                        .host_str()
                        .is_some_and(|host| host.ends_with(".atlassian.net"))
            }) && callback_valid
                && valid_ref(&jira.cloud_id, 128)
                && !projects.is_empty()
                && projects == &canonical_projects
                && projects
                    .iter()
                    .all(|project| valid_jira_project_key(project))
                && match jira.shared_auth {
                    JiraSharedAuth::ServiceOauth => jira
                        .service_oauth_client_id
                        .as_deref()
                        .is_some_and(|value| valid_ref(value, 256)),
                    JiraSharedAuth::ServiceApiToken => jira.service_oauth_client_id.is_none(),
                }
                && valid_ref(&jira.user_oauth_client_id, 256)
                && valid_ref(&jira.organization_read_grant_ref, 512)
                && valid_ref(&jira.user_grant_ref, 512)
                && jira.organization_read_grant_ref != jira.user_grant_ref
                && (60..=900).contains(&jira.connect_session_ttl_seconds)
                && (60..=900).contains(&jira.refresh_skew_seconds)
        });
        let catalog_valid = if self.catalog.enabled {
            let mut providers = self.catalog.providers.clone();
            providers.sort();
            providers.dedup();
            self.catalog
                .public_origin
                .as_deref()
                .and_then(|origin| url::Url::parse(origin).ok())
                .is_some_and(|origin| {
                    origin.scheme() == "https"
                        && origin.host_str().is_some()
                        && origin.username().is_empty()
                        && origin.password().is_none()
                        && origin.query().is_none()
                        && origin.fragment().is_none()
                        && origin.path() == self.server.base_path
                })
                && self
                    .catalog
                    .grant_ref
                    .as_deref()
                    .is_some_and(|value| valid_ref(value, 512))
                && providers == self.catalog.providers
                && providers.iter().all(|provider| valid_ref(provider, 128))
                && (60..=900).contains(&self.catalog.connect_session_ttl_seconds)
        } else {
            self.catalog.public_origin.is_none()
                && self.catalog.grant_ref.is_none()
                && self.catalog.providers.is_empty()
                && self.catalog.connect_session_ttl_seconds == default_connect_session_ttl_seconds()
        };
        let vault_required = self.claude_code.enabled
            || self.catalog.enabled
            || self.sip.credentials.is_some()
            || self.slack.is_some()
            || self.gitlab.is_some()
            || self.jira.is_some()
            || self.grafana.enabled;
        let sip_complete = self.sip.listen.is_some() && self.sip.deployment_config.is_some();
        let sip_credentials_valid = self.sip.credentials.as_ref().is_none_or(|credentials| {
            credentials.authority == NATIVE_SIP_AUTHORITY
                && credentials.service == NATIVE_SIP_SERVICE
                && CredentialRef::new(
                    &self.tenant_id,
                    &credentials.authority,
                    &credentials.service,
                    &credentials.username_credential,
                )
                .is_ok()
                && CredentialRef::new(
                    &self.tenant_id,
                    &credentials.authority,
                    &credentials.service,
                    &credentials.password_credential,
                )
                .is_ok()
                && credentials.username_credential != credentials.password_credential
        });
        if !valid_ref(&self.tenant_id, 256)
            || module_tenants != self.module_tenant_ids
            || (!self.module_tenant_ids.is_empty()
                && self.module_tenant_ids.as_slice() != std::slice::from_ref(&self.tenant_id))
            || self
                .module_tenant_ids
                .iter()
                .any(|tenant| !valid_ref(tenant, 256))
            || self.identity.origin.len() > 2_048
            || !valid_base_path(&self.server.base_path)
            || !self.storage.state_root.is_absolute()
            || (self.kubernetes.enabled
                != (!self.kubernetes.namespaces.is_empty()
                    || !self.kubernetes.namespace_access.is_empty()))
            || (!self.kubernetes.namespaces.is_empty()
                && !self.kubernetes.namespace_access.is_empty())
            || self
                .kubernetes
                .namespaces
                .iter()
                .any(|namespace| !valid_dns_label(namespace, 63))
            || !valid_namespace_access(&self.kubernetes.namespace_access)
            || !valid_grafana(&self.grafana)
            || (self.sip.enabled != sip_complete)
            || !sip_credentials_valid
            || (self.vault.enabled && !vault_complete)
            || (!self.vault.enabled && !vault_empty)
            || (self.secrets.enabled && !secrets_complete)
            || (!self.secrets.enabled && !secrets_empty)
            || (self.vault.enabled && self.secrets.enabled)
            || ((self.vault.enabled || self.secrets.enabled) != vault_required)
            || !valid_dns_label(&self.vault.mount, 63)
            || self
                .platform
                .as_ref()
                .is_some_and(|platform| platform.validate().is_err())
            || !valid_groups(&self.authority.operator_groups)
            || !slack_valid
            || !gitlab_valid
            || !git_fetch_tls_valid
            || !jira_valid
            || !catalog_valid
        {
            return Err(HostedServerConfigError::Invalid);
        }
        if !self.kubernetes.enabled
            && (self.kubernetes.token_file != default_kubernetes_token_file()
                || self.kubernetes.ca_file != default_kubernetes_ca_file())
        {
            return Err(HostedServerConfigError::Invalid);
        }
        let supported_credential_egress = self.slack.is_some()
            || self.gitlab.is_some()
            || self.grafana.enabled
            || self.catalog.enabled;
        let unsupported_credential_egress = self.sip.credentials.is_some() || self.jira.is_some();
        if unsupported_credential_egress
            || (supported_credential_egress
                && self.egress.policy != HostedEgressPolicy::ConnectionBoundPostDnsV1)
        {
            return Err(HostedServerConfigError::CredentialEgressUnconfined);
        }
        Ok(())
    }

    #[must_use]
    pub fn admitted_module_tenants(&self) -> Vec<String> {
        if self.module_tenant_ids.is_empty() {
            vec![self.tenant_id.clone()]
        } else {
            self.module_tenant_ids.clone()
        }
    }

    /// Resolve the compatibility namespace list into the same exact policy shape.
    #[must_use]
    pub fn kubernetes_namespace_access(&self) -> Vec<KubernetesNamespaceAccessConfig> {
        if self.kubernetes.namespace_access.is_empty() {
            self.kubernetes
                .namespaces
                .iter()
                .map(|namespace| KubernetesNamespaceAccessConfig {
                    namespace: namespace.clone(),
                    read_groups: Vec::new(),
                    restart_groups: Vec::new(),
                })
                .collect()
        } else {
            self.kubernetes.namespace_access.clone()
        }
    }
}

fn valid_namespace_access(access: &[KubernetesNamespaceAccessConfig]) -> bool {
    let namespaces = access
        .iter()
        .map(|entry| entry.namespace.as_str())
        .collect::<Vec<_>>();
    let mut canonical = namespaces.clone();
    canonical.sort_unstable();
    canonical.dedup();
    namespaces == canonical
        && access.iter().all(|entry| {
            valid_dns_label(&entry.namespace, 63)
                && valid_groups(&entry.read_groups)
                && valid_groups(&entry.restart_groups)
                && entry
                    .restart_groups
                    .iter()
                    .all(|group| entry.read_groups.contains(group))
        })
}

fn valid_grafana(config: &HostedGrafanaConfig) -> bool {
    let empty = config.origin.is_none()
        && config.connection_ref.is_none()
        && config.label.is_none()
        && config.grant_ref.is_none()
        && config.read_groups.is_empty()
        && config.targets.is_empty();
    if !config.enabled {
        return empty
            && config.reconcile_interval_seconds == default_grafana_reconcile_interval_seconds();
    }
    let Some(origin) = config
        .origin
        .as_deref()
        .and_then(|origin| url::Url::parse(origin).ok())
    else {
        return false;
    };
    let origin_valid = origin.scheme() == "https"
        && origin.host_str().is_some()
        && origin.username().is_empty()
        && origin.password().is_none()
        && origin.query().is_none()
        && origin.fragment().is_none()
        && matches!(origin.path(), "" | "/");
    let target_keys = config
        .targets
        .iter()
        .map(|target| {
            (
                target.provider.as_str(),
                target.connection_ref.as_str(),
                target.uid_sha256.as_str(),
            )
        })
        .collect::<Vec<_>>();
    let mut canonical = target_keys.clone();
    canonical.sort_unstable();
    canonical.dedup();
    origin_valid
        && config
            .connection_ref
            .as_deref()
            .is_some_and(|value| valid_ref(value, 512))
        && config
            .label
            .as_deref()
            .is_some_and(|value| valid_display(value, 256))
        && config
            .grant_ref
            .as_deref()
            .is_some_and(|value| valid_ref(value, 512))
        && valid_groups(&config.read_groups)
        && !config.read_groups.is_empty()
        && !config.targets.is_empty()
        && target_keys == canonical
        && (60..=3600).contains(&config.reconcile_interval_seconds)
        && config.targets.iter().all(|target| {
            matches!(
                target.provider.as_str(),
                "prometheus" | "loki" | "alertmanager"
            ) && target.uid_sha256.len() == 64
                && target
                    .uid_sha256
                    .bytes()
                    .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
                && valid_ref(&target.connection_ref, 512)
                && valid_display(&target.label, 256)
                && valid_ref(&target.grant_ref, 512)
        })
}

fn valid_groups(groups: &[String]) -> bool {
    let mut canonical = groups.to_vec();
    canonical.sort();
    canonical.dedup();
    canonical == groups
        && groups.iter().all(|group| {
            (1..=64).contains(&group.len())
                && group.bytes().enumerate().all(|(index, byte)| {
                    byte.is_ascii_lowercase()
                        || byte.is_ascii_digit() && index > 0
                        || matches!(byte, b'-' | b'_') && index > 0
                })
        })
}

fn default_vault_mount() -> String {
    "secret".to_owned()
}

fn default_connect_session_ttl_seconds() -> u64 {
    300
}

fn default_grafana_reconcile_interval_seconds() -> u64 {
    300
}

fn default_kubernetes_token_file() -> PathBuf {
    PathBuf::from("/var/run/secrets/kubernetes.io/serviceaccount/token")
}

fn default_base_path() -> String {
    "/".to_owned()
}

fn valid_base_path(value: &str) -> bool {
    value == "/"
        || (value.starts_with('/')
            && !value.ends_with('/')
            && value.len() <= 128
            && value.split('/').skip(1).all(|segment| {
                !segment.is_empty()
                    && segment.bytes().all(|byte| {
                        byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-'
                    })
            }))
}

fn valid_https_origin(origin: &url::Url) -> bool {
    origin.scheme() == "https"
        && origin.host_str().is_some()
        && origin.port_or_known_default() == Some(443)
        && origin.username().is_empty()
        && origin.password().is_none()
        && origin.query().is_none()
        && origin.fragment().is_none()
        && matches!(origin.path(), "" | "/")
}

fn default_jira_refresh_skew_seconds() -> u64 {
    300
}

fn valid_jira_project_key(value: &str) -> bool {
    (1..=32).contains(&value.len())
        && value.as_bytes().first().is_some_and(u8::is_ascii_uppercase)
        && value
            .bytes()
            .all(|byte| byte.is_ascii_uppercase() || byte.is_ascii_digit() || byte == b'_')
}

fn default_kubernetes_ca_file() -> PathBuf {
    PathBuf::from("/var/run/secrets/kubernetes.io/serviceaccount/ca.crt")
}

fn valid_ref(value: &str, maximum: usize) -> bool {
    !value.is_empty() && value.len() <= maximum && value.bytes().all(|byte| byte.is_ascii_graphic())
}

fn valid_display(value: &str, maximum: usize) -> bool {
    !value.trim().is_empty() && value.len() <= maximum && !value.chars().any(char::is_control)
}

fn valid_dns_label(value: &str, maximum: usize) -> bool {
    !value.is_empty()
        && value.len() <= maximum
        && value
            .as_bytes()
            .first()
            .is_some_and(u8::is_ascii_alphanumeric)
        && value
            .as_bytes()
            .last()
            .is_some_and(u8::is_ascii_alphanumeric)
        && value
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::os::unix::fs::{symlink, PermissionsExt as _};

    use super::*;

    #[test]
    fn an_existing_hosted_config_with_the_old_b10x_section_parses_unchanged() {
        // D5 (S-052): a hosted deployment written before the rename carries `[b10x]`
        // and `initiation = "b10x"` and must keep parsing into the platform field.
        let config: HostedServerConfig = toml::from_str(
            r#"
tenant_id = "babelforce"
module_tenant_ids = ["babelforce"]
[server]
listen = "0.0.0.0:8080"
base_path = "/api/connectors/v1"
[identity]
origin = "https://identity.code.dev.babelforce.com"
[authority]
operator_groups = ["operator"]
[storage]
state_root = "/var/lib/b10x-connectors"
[egress]
policy = "connection_bound_post_dns_v1"
[kubernetes]
enabled = false
namespaces = []
[vault]
enabled = false
mount = "b10x-connectors"
[sip]
enabled = false
listen = "0.0.0.0:5060"
[b10x]
work_origin = "http://b10x-work:8080"
[b10x.connection]
connection_ref = "connection:b10x:b10x"
label = "platform private services"
grant_ref = "grant:deployment:b10x:b10x"
initiation = "b10x"
"#,
        )
        .unwrap();
        let platform = config
            .platform
            .as_ref()
            .expect("the old [b10x] section lands in the platform field");
        assert!(matches!(
            platform.connection.initiation,
            InitiationConfig::Platform
        ));
    }

    #[test]
    fn hosted_deployment_accepts_planner_integration() {
        let config: HostedServerConfig = toml::from_str(
            r#"
tenant_id = "babelforce"
module_tenant_ids = ["babelforce"]
[server]
listen = "0.0.0.0:8080"
base_path = "/api/connectors/v1"
[identity]
origin = "https://identity.code.dev.babelforce.com"
[authority]
operator_groups = ["operator"]
[storage]
state_root = "/var/lib/b10x-connectors"
[egress]
policy = "connection_bound_post_dns_v1"
[kubernetes]
enabled = true
namespaces = ["b10x"]
token_file = "/var/run/secrets/kubernetes.io/serviceaccount/token"
ca_file = "/var/run/secrets/kubernetes.io/serviceaccount/ca.crt"
[vault]
enabled = true
mount = "b10x-connectors"
address = "https://b10x-vault.b10x.svc:8200"
role = "b10x-connectors"
token_file = "/var/run/secrets/kubernetes.io/serviceaccount/token"
ca_file = "/etc/b10x-vault-ca/ca.crt"
[sip]
enabled = false
listen = "0.0.0.0:5060"
[slack]
public_origin = "https://code.dev.babelforce.com/api/connectors/v1"
team_id = "T01234567"
oauth_client_id = "123456789.987654321"
oauth_redirect_uri = "https://code.dev.babelforce.com/api/connectors/v1/oauth/slack/callback"
org_read_grant_ref = "grant:slack:org-read"
user_grant_ref = "grant:slack:org-user"
companion_grant_ref = "grant:slack:workspace-companion"
initiation = "provider"
allowed_events = ["app_mention"]
connect_session_ttl_seconds = 300
[platform]
tenant_member_modules = ["ontology", "planner", "work"]
module_signing_key_file = "/var/run/b10x-module-auth/private.pem"
module_signing_key_id = "developer-1"
module_signing_issuer = "urn:b10x:connectors:b10x:b10x"
work_origin = "http://b10x-work:8080"
ontology_origin = "http://b10x-ontology:8080"
planner_origin = "http://b10x-planner:8080"
[platform.connection]
connection_ref = "connection:b10x:b10x"
label = "platform private services"
grant_ref = "grant:deployment:b10x:b10x"
initiation = "platform"
"#,
        )
        .unwrap();

        config.validate().unwrap();
        let mut bot_only = config.clone();
        let slack = bot_only.slack.as_mut().unwrap();
        slack.oauth_client_id = None;
        slack.oauth_redirect_uri = None;
        bot_only.validate().unwrap();
        bot_only.slack.as_mut().unwrap().oauth_client_id = Some("123456789.987654321".to_owned());
        assert!(matches!(
            bot_only.validate(),
            Err(HostedServerConfigError::Invalid)
        ));

        let mut missing_policy = config;
        missing_policy.egress.policy = HostedEgressPolicy::Disabled;
        assert!(matches!(
            missing_policy.validate(),
            Err(HostedServerConfigError::CredentialEgressUnconfined)
        ));
    }

    #[test]
    fn hosted_integrations_are_explicit_and_fail_closed() {
        let config: HostedServerConfig = toml::from_str(
            r#"
tenant_id = "tenant-dev"

[server]
listen = "0.0.0.0:8080"
base_path = "/api/connectors/v1"

[identity]
origin = "https://identity.example.test"

[storage]
state_root = "/var/lib/b10x-connectors"

[kubernetes]
enabled = true
namespaces = ["b10x"]

[sip]
enabled = false
"#,
        )
        .unwrap();
        config.validate().unwrap();

        let invalid: HostedServerConfig = toml::from_str(
            r#"
tenant_id = "tenant-dev"
[server]
listen = "0.0.0.0:8080"
[identity]
origin = "https://identity.example.test"
[storage]
state_root = "/var/lib/b10x-connectors"
[egress]
policy = "connection_bound_post_dns_v1"
[kubernetes]
enabled = false
namespaces = ["b10x"]
[sip]
enabled = true
listen = "0.0.0.0:5060"
"#,
        )
        .unwrap();
        assert!(invalid.validate().is_err());
    }

    #[test]
    fn kubernetes_namespace_groups_are_exact_sorted_and_restart_is_a_read_subset() {
        let config: HostedServerConfig = toml::from_str(
            r#"
tenant_id = "tenant-dev"
[server]
listen = "0.0.0.0:8080"
[identity]
origin = "https://identity.example.test"
[storage]
state_root = "/var/lib/b10x-connectors"
[kubernetes]
enabled = true
[[kubernetes.namespace_access]]
namespace = "latest"
read_groups = ["dev", "sre"]
restart_groups = ["sre"]
[sip]
enabled = false
"#,
        )
        .unwrap();
        config.validate().unwrap();
        assert_eq!(config.kubernetes_namespace_access()[0].namespace, "latest");

        let mut invalid = config.clone();
        invalid.kubernetes.namespace_access[0].restart_groups = vec!["operator".to_owned()];
        assert!(invalid.validate().is_err());
        invalid.kubernetes.namespace_access[0].restart_groups = vec!["sre".to_owned()];
        invalid.kubernetes.namespaces = vec!["legacy".to_owned()];
        assert!(invalid.validate().is_err());
    }

    #[test]
    fn hosted_vault_is_all_or_nothing() {
        let enabled: HostedServerConfig = toml::from_str(
            r#"
tenant_id = "tenant-dev"
[server]
listen = "0.0.0.0:8080"
[identity]
origin = "https://identity.example.test"
[storage]
state_root = "/var/lib/b10x-connectors"
[kubernetes]
enabled = false
namespaces = []
[vault]
enabled = true
address = "https://b10x-vault.b10x.svc:8200"
mount = "b10x-connectors"
role = "b10x-connectors"
token_file = "/var/run/secrets/kubernetes.io/serviceaccount/token"
ca_file = "/etc/b10x-vault-ca/ca.crt"
[sip]
enabled = true
listen = "0.0.0.0:5060"
deployment_config = "/etc/b10x-connectors-sip/deployment.toml"
[sip.credentials]
authority = "io.b10x"
service = "default"
username_credential = "sip_username"
password_credential = "sip_password"
"#,
        )
        .unwrap();
        assert!(matches!(
            enabled.validate(),
            Err(HostedServerConfigError::CredentialEgressUnconfined)
        ));

        let mut inconsistent = enabled;
        inconsistent.vault.enabled = false;
        assert!(inconsistent.validate().is_err());

        inconsistent.vault.address = None;
        inconsistent.vault.role = None;
        inconsistent.vault.ca_file = None;
        assert!(
            inconsistent.validate().is_err(),
            "a disabled Vault cannot retain even one runtime identity field"
        );
        inconsistent.vault.token_file = None;
        inconsistent.sip.credentials = None;
        inconsistent.validate().unwrap();
    }

    #[test]
    fn claude_code_custody_is_explicit_and_requires_the_hosted_secret_store() {
        let enabled: HostedServerConfig = toml::from_str(
            r#"
tenant_id = "tenant-dev"
[server]
listen = "0.0.0.0:8080"
[identity]
origin = "https://identity.example.test"
[storage]
state_root = "/var/lib/b10x-connectors"
[kubernetes]
enabled = false
[vault]
enabled = true
address = "https://vault.example.test"
mount = "b10x-connectors"
role = "connectors"
token_file = "/var/run/secrets/kubernetes.io/serviceaccount/token"
ca_file = "/etc/vault/ca.crt"
[claude_code]
enabled = true
[sip]
enabled = false
"#,
        )
        .unwrap();
        enabled.validate().unwrap();

        let mut without_custody = enabled.clone();
        without_custody.vault.enabled = false;
        assert!(without_custody.validate().is_err());

        let mut disabled = enabled;
        disabled.claude_code.enabled = false;
        assert!(
            disabled.validate().is_err(),
            "unused Vault configuration is refused"
        );
    }

    #[test]
    fn hosted_vault_requires_a_valid_distinct_sip_digest_pair() {
        let mut config: HostedServerConfig = toml::from_str(
            r#"
tenant_id = "tenant-dev"
[server]
listen = "0.0.0.0:8080"
[identity]
origin = "https://identity.example.test"
[storage]
state_root = "/var/lib/b10x-connectors"
[kubernetes]
enabled = false
namespaces = []
[vault]
enabled = true
address = "https://b10x-vault.b10x.svc:8200"
mount = "b10x-connectors"
role = "b10x-connectors"
token_file = "/var/run/secrets/kubernetes.io/serviceaccount/token"
ca_file = "/etc/b10x-vault-ca/ca.crt"
[sip]
enabled = true
listen = "0.0.0.0:5060"
deployment_config = "/etc/b10x-connectors-sip/deployment.toml"
[sip.credentials]
authority = "io.b10x"
service = "default"
username_credential = "sip_username"
password_credential = "sip_password"
"#,
        )
        .unwrap();
        assert!(matches!(
            config.validate(),
            Err(HostedServerConfigError::CredentialEgressUnconfined)
        ));

        config.sip.credentials.as_mut().unwrap().password_credential = "sip_username".to_owned();
        assert!(config.validate().is_err());
        config.sip.credentials.as_mut().unwrap().password_credential = "sip.password".to_owned();
        assert!(config.validate().is_err());
        config.sip.credentials.as_mut().unwrap().password_credential = "sip_password".to_owned();
        config.sip.credentials.as_mut().unwrap().authority = "org.asterisk.ari".to_owned();
        assert!(config.validate().is_err());
        config.sip.credentials = None;
        assert!(config.validate().is_err());
    }

    #[test]
    fn hosted_grafana_requires_vault_exact_groups_and_digest_bound_targets() {
        let mut config: HostedServerConfig = toml::from_str(
            r#"
tenant_id = "tenant-dev"
[server]
listen = "0.0.0.0:8080"
[identity]
origin = "https://identity.example.test"
[storage]
state_root = "/var/lib/b10x-connectors"
[egress]
policy = "connection_bound_post_dns_v1"
[kubernetes]
enabled = false
namespaces = []
[grafana]
enabled = true
origin = "https://grafana.example.test"
connection_ref = "connection:grafana:global"
label = "Global infrastructure Grafana"
grant_ref = "grant:grafana:global-read"
read_groups = ["dev", "sre"]
reconcile_interval_seconds = 300
[[grafana.targets]]
provider = "prometheus"
uid_sha256 = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
connection_ref = "connection:prometheus:dev"
label = "Prometheus · dev"
grant_ref = "grant:prometheus:dev-read"
[vault]
enabled = true
address = "https://b10x-vault.b10x.svc:8200"
mount = "b10x-connectors"
role = "b10x-connectors"
token_file = "/var/run/secrets/kubernetes.io/serviceaccount/token"
ca_file = "/etc/b10x-vault-ca/ca.crt"
[sip]
enabled = false
"#,
        )
        .unwrap();
        config.validate().unwrap();

        config.grafana.read_groups.reverse();
        assert!(config.validate().is_err());
        config.grafana.read_groups.reverse();
        config.grafana.targets[0].uid_sha256 = "not-a-digest".to_owned();
        assert!(config.validate().is_err());
        config.grafana.targets[0].uid_sha256 = "a".repeat(64);
        config.vault.enabled = false;
        assert!(config.validate().is_err());
    }

    #[test]
    fn hosted_gitlab_requires_vault_and_a_same_origin_callback() {
        let mut config: HostedServerConfig = toml::from_str(
            r#"
tenant_id = "tenant-dev"
[server]
listen = "0.0.0.0:8080"
base_path = "/api/connectors/v1"
[identity]
origin = "https://identity.example.test"
[storage]
state_root = "/var/lib/b10x-connectors"
[egress]
policy = "connection_bound_post_dns_v1"
[kubernetes]
enabled = false
namespaces = []
[vault]
enabled = true
address = "https://b10x-vault.b10x.svc:8200"
mount = "b10x-connectors"
role = "b10x-connectors"
token_file = "/var/run/secrets/kubernetes.io/serviceaccount/token"
ca_file = "/etc/b10x-vault-ca/ca.crt"
[sip]
enabled = false
[gitlab]
origin = "https://gitlab.example.test"
public_origin = "https://code.example.test/api/connectors/v1"
oauth_client_id = "gitlab-oauth-application"
oauth_redirect_uri = "https://code.example.test/api/connectors/v1/oauth/gitlab/callback"
user_grant_ref = "grant:gitlab:delegated-user"
initiation = "provider"
connect_session_ttl_seconds = 300
refresh_skew_seconds = 300
"#,
        )
        .unwrap();
        assert!(config.validate().is_ok());

        config.egress.policy = HostedEgressPolicy::Disabled;
        assert!(matches!(
            config.validate(),
            Err(HostedServerConfigError::CredentialEgressUnconfined)
        ));
        config.egress.policy = HostedEgressPolicy::ConnectionBoundPostDnsV1;

        config.gitlab.as_mut().unwrap().oauth_redirect_uri =
            "https://attacker.example/api/connectors/v1/oauth/gitlab/callback".to_owned();
        assert!(config.validate().is_err());
        config.gitlab.as_mut().unwrap().oauth_redirect_uri =
            "https://code.example.test/api/connectors/v1/oauth/gitlab/callback".to_owned();

        config.gitlab.as_mut().unwrap().git_fetch_origin =
            Some("https://connectors-git-fetch.example.test".to_owned());
        assert!(config.validate().is_err());
        config.server.git_fetch_tls = Some(HostedTlsListenerConfig {
            listen: "0.0.0.0:8443".parse().unwrap(),
            certificate_file: PathBuf::from("/etc/connectors-git-fetch/tls.crt"),
            private_key_file: PathBuf::from("/etc/connectors-git-fetch/tls.key"),
        });
        assert!(config.validate().is_ok());
        config.server.git_fetch_tls.as_mut().unwrap().listen = config.server.listen;
        assert!(config.validate().is_err());
        config.server.git_fetch_tls.as_mut().unwrap().listen = "0.0.0.0:8443".parse().unwrap();

        config.vault.enabled = false;
        assert!(config.validate().is_err());
    }

    #[test]
    fn hosted_jira_separates_organization_and_user_authority() {
        let mut config: HostedServerConfig = toml::from_str(
            r#"
tenant_id = "tenant-dev"
[server]
listen = "0.0.0.0:8080"
base_path = "/api/connectors/v1"
[identity]
origin = "https://identity.example.test"
[storage]
state_root = "/var/lib/b10x-connectors"
[kubernetes]
enabled = false
namespaces = []
[vault]
enabled = true
address = "https://b10x-vault.b10x.svc:8200"
mount = "b10x-connectors"
role = "b10x-connectors"
token_file = "/var/run/secrets/kubernetes.io/serviceaccount/token"
ca_file = "/etc/b10x-vault-ca/ca.crt"
[sip]
enabled = false
[jira]
cloud_id = "11111111-2222-3333-4444-555555555555"
site_origin = "https://example.atlassian.net"
public_origin = "https://code.example.test/api/connectors/v1"
allowed_project_keys = ["OPS", "SUPPORT"]
shared_auth = "service_oauth"
service_oauth_client_id = "jira-service-client"
user_oauth_client_id = "jira-user-client"
oauth_redirect_uri = "https://code.example.test/api/connectors/v1/oauth/jira/callback"
organization_read_grant_ref = "grant:jira:organization-read"
user_grant_ref = "grant:jira:delegated-user"
initiation = "provider"
connect_session_ttl_seconds = 300
refresh_skew_seconds = 300
"#,
        )
        .unwrap();
        assert!(matches!(
            config.validate(),
            Err(HostedServerConfigError::CredentialEgressUnconfined)
        ));

        config.jira.as_mut().unwrap().allowed_project_keys.reverse();
        assert!(config.validate().is_err());
        config.jira.as_mut().unwrap().allowed_project_keys.reverse();
        config.jira.as_mut().unwrap().user_grant_ref = "grant:jira:organization-read".to_owned();
        assert!(config.validate().is_err());
        config.jira.as_mut().unwrap().user_grant_ref = "grant:jira:delegated-user".to_owned();
        config.jira.as_mut().unwrap().oauth_redirect_uri =
            "https://attacker.example/api/connectors/v1/oauth/jira/callback".to_owned();
        assert!(config.validate().is_err());
        config.jira.as_mut().unwrap().oauth_redirect_uri =
            "https://code.example.test/api/connectors/v1/oauth/jira/callback".to_owned();
        config.vault.enabled = false;
        assert!(config.validate().is_err());
    }

    #[test]
    fn hosted_jira_service_api_token_excludes_a_service_oauth_client_id() {
        let mut config: HostedServerConfig = toml::from_str(
            r#"
tenant_id = "tenant-dev"
[server]
listen = "0.0.0.0:8080"
base_path = "/api/connectors/v1"
[identity]
origin = "https://identity.example.test"
[storage]
state_root = "/var/lib/b10x-connectors"
[kubernetes]
enabled = false
namespaces = []
[vault]
enabled = true
address = "https://b10x-vault.b10x.svc:8200"
mount = "b10x-connectors"
role = "b10x-connectors"
token_file = "/var/run/secrets/kubernetes.io/serviceaccount/token"
ca_file = "/etc/b10x-vault-ca/ca.crt"
[sip]
enabled = false
[jira]
cloud_id = "11111111-2222-3333-4444-555555555555"
site_origin = "https://example.atlassian.net"
public_origin = "https://code.example.test/api/connectors/v1"
allowed_project_keys = ["OPS"]
shared_auth = "service_api_token"
user_oauth_client_id = "jira-user-client"
oauth_redirect_uri = "https://code.example.test/api/connectors/v1/oauth/jira/callback"
organization_read_grant_ref = "grant:jira:organization-read"
user_grant_ref = "grant:jira:delegated-user"
initiation = "platform"
"#,
        )
        .unwrap();
        assert!(matches!(
            config.validate(),
            Err(HostedServerConfigError::CredentialEgressUnconfined)
        ));

        config.jira.as_mut().unwrap().service_oauth_client_id = Some("not-admitted".to_owned());
        assert!(config.validate().is_err());
    }

    #[test]
    fn hosted_configuration_uses_same_handle_and_refuses_mutable_or_symlinked_files() {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("hosted.toml");
        let text = r#"
tenant_id = "tenant-dev"
[server]
listen = "127.0.0.1:8080"
[identity]
origin = "https://identity.example.test"
[storage]
state_root = "/var/lib/b10x-connectors"
[kubernetes]
enabled = false
namespaces = []
[sip]
enabled = false
"#;
        fs::write(&path, text).unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).unwrap();
        HostedServerConfig::read(&path).unwrap();

        fs::set_permissions(&path, fs::Permissions::from_mode(0o620)).unwrap();
        assert!(matches!(
            HostedServerConfig::read(&path),
            Err(HostedServerConfigError::Invalid)
        ));
        fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).unwrap();
        let link = root.path().join("hosted-link.toml");
        symlink(&path, &link).unwrap();
        assert!(HostedServerConfig::read(&link).is_err());
    }
}
