//! Strict, value-free hosted server configuration.

use std::net::SocketAddr;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use connector_address::credential::CredentialRef;

use crate::file::{read_trusted_config, TrustedConfigReadError, TrustedOwner};
use crate::personal::{B10xIntegrationConfig, InitiationConfig, SlackIntegrationConfig};

const MAX_CONFIG_BYTES: u64 = 256 * 1024;
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
    pub kubernetes: HostedKubernetesConfig,
    #[serde(default)]
    pub vault: HostedVaultConfig,
    pub sip: HostedSipConfig,
    #[serde(default)]
    pub slack: Option<HostedSlackConfig>,
    #[serde(default)]
    pub b10x: Option<B10xIntegrationConfig>,
}

/// Value-free hosted Slack policy. Every credential still arrives through a Connect Session and
/// is committed only by the configured SecretStore.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HostedSlackConfig {
    pub public_origin: String,
    pub team_id: String,
    pub oauth_client_id: String,
    pub oauth_redirect_uri: String,
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
            oauth_client_id: Some(self.oauth_client_id.clone()),
            oauth_redirect_uri: Some(self.oauth_redirect_uri.clone()),
            initiation: self.initiation,
            allowed_events: self.allowed_events.clone(),
            connect_session_ttl_seconds: self.connect_session_ttl_seconds,
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

#[derive(Debug, thiserror::Error)]
pub enum HostedServerConfigError {
    #[error("hosted Connector configuration could not be read")]
    Read,
    #[error("hosted Connector configuration is malformed")]
    Parse,
    #[error("hosted Connector configuration is incomplete or inconsistent")]
    Invalid,
}

impl HostedServerConfig {
    pub fn read(path: &Path) -> Result<Self, HostedServerConfigError> {
        let text = read_trusted_config(path, MAX_CONFIG_BYTES, TrustedOwner::CurrentUserOrRoot)
            .map_err(|error| match error {
                TrustedConfigReadError::Io(_) => HostedServerConfigError::Read,
                TrustedConfigReadError::Unsafe => HostedServerConfigError::Invalid,
            })?;
        let config: Self = toml::from_str(&text).map_err(|_| HostedServerConfigError::Parse)?;
        config.validate()?;
        Ok(config)
    }

    fn validate(&self) -> Result<(), HostedServerConfigError> {
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
        let vault_required = self.sip.credentials.is_some() || self.slack.is_some();
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
            || (self.sip.enabled != sip_complete)
            || !sip_credentials_valid
            || (self.vault.enabled && !vault_complete)
            || (!self.vault.enabled && !vault_empty)
            || (self.vault.enabled != vault_required)
            || !valid_dns_label(&self.vault.mount, 63)
            || self
                .b10x
                .as_ref()
                .is_some_and(|b10x| b10x.validate().is_err())
            || !valid_groups(&self.authority.operator_groups)
            || !slack_valid
        {
            return Err(HostedServerConfigError::Invalid);
        }
        if !self.kubernetes.enabled
            && (self.kubernetes.token_file != default_kubernetes_token_file()
                || self.kubernetes.ca_file != default_kubernetes_ca_file())
        {
            return Err(HostedServerConfigError::Invalid);
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

fn default_kubernetes_ca_file() -> PathBuf {
    PathBuf::from("/var/run/secrets/kubernetes.io/serviceaccount/ca.crt")
}

fn valid_ref(value: &str, maximum: usize) -> bool {
    !value.is_empty() && value.len() <= maximum && value.bytes().all(|byte| byte.is_ascii_graphic())
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
[b10x]
tenant_member_modules = ["ontology", "planner", "work"]
module_signing_key_file = "/var/run/b10x-module-auth/private.pem"
module_signing_key_id = "developer-1"
module_signing_issuer = "urn:b10x:connectors:b10x:b10x"
work_origin = "http://b10x-work:8080"
ontology_origin = "http://b10x-ontology:8080"
planner_origin = "http://b10x-planner:8080"
[b10x.connection]
connection_ref = "connection:b10x:b10x"
label = "B10x private services"
grant_ref = "grant:deployment:b10x:b10x"
initiation = "b10x"
"#,
        )
        .unwrap();

        config.validate().unwrap();
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
        enabled.validate().unwrap();

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
        config.validate().unwrap();

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
