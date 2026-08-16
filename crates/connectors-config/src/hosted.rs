//! Strict, value-free hosted server configuration.

use std::net::SocketAddr;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use connector_address::credential::CredentialRef;

use crate::file::{read_trusted_config, TrustedConfigReadError, TrustedOwner};
use crate::personal::B10xIntegrationConfig;

const MAX_CONFIG_BYTES: u64 = 256 * 1024;
const NATIVE_SIP_AUTHORITY: &str = "io.b10x";
const NATIVE_SIP_SERVICE: &str = "default";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HostedServerConfig {
    pub tenant_id: String,
    pub server: HostedListenerConfig,
    pub identity: HostedIdentityConfig,
    pub storage: HostedStorageConfig,
    pub kubernetes: HostedKubernetesConfig,
    #[serde(default)]
    pub vault: HostedVaultConfig,
    pub sip: HostedSipConfig,
    #[serde(default)]
    pub b10x: Option<B10xIntegrationConfig>,
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
    #[serde(default)]
    pub namespaces: Vec<String>,
    #[serde(default = "default_kubernetes_token_file")]
    pub token_file: PathBuf,
    #[serde(default = "default_kubernetes_ca_file")]
    pub ca_file: PathBuf,
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
        let vault_complete = self.vault.address.is_some()
            && self.vault.role.is_some()
            && self.vault.token_file.is_some()
            && self.vault.ca_file.is_some();
        let vault_empty = self.vault.address.is_none()
            && self.vault.role.is_none()
            && self.vault.token_file.is_none()
            && self.vault.ca_file.is_none();
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
            || self.identity.origin.len() > 2_048
            || !valid_base_path(&self.server.base_path)
            || !self.storage.state_root.is_absolute()
            || (self.kubernetes.enabled != !self.kubernetes.namespaces.is_empty())
            || self
                .kubernetes
                .namespaces
                .iter()
                .any(|namespace| !valid_dns_label(namespace, 63))
            || (self.sip.enabled != sip_complete)
            || !sip_credentials_valid
            || (self.vault.enabled && !vault_complete)
            || (!self.vault.enabled && !vault_empty)
            || (self.vault.enabled != self.sip.credentials.is_some())
            || (self.vault.enabled && !self.sip.enabled)
            || !valid_dns_label(&self.vault.mount, 63)
            || self
                .b10x
                .as_ref()
                .is_some_and(|b10x| b10x.validate().is_err())
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
}

fn default_vault_mount() -> String {
    "secret".to_owned()
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
