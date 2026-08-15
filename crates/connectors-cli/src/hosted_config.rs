//! Strict, value-free hosted server configuration.

use std::fs;
use std::net::SocketAddr;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

const MAX_CONFIG_BYTES: u64 = 256 * 1024;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HostedServerConfig {
    pub tenant_id: String,
    pub server: HostedListenerConfig,
    pub identity: HostedIdentityConfig,
    pub storage: HostedStorageConfig,
    pub kubernetes: HostedKubernetesConfig,
    pub sip: HostedSipConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HostedListenerConfig {
    pub listen: SocketAddr,
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
        let metadata = fs::metadata(path).map_err(|_| HostedServerConfigError::Read)?;
        if !metadata.is_file() || metadata.len() > MAX_CONFIG_BYTES {
            return Err(HostedServerConfigError::Invalid);
        }
        let text = fs::read_to_string(path).map_err(|_| HostedServerConfigError::Read)?;
        if text.len() as u64 > MAX_CONFIG_BYTES {
            return Err(HostedServerConfigError::Invalid);
        }
        let config: Self = toml::from_str(&text).map_err(|_| HostedServerConfigError::Parse)?;
        config.validate()?;
        Ok(config)
    }

    fn validate(&self) -> Result<(), HostedServerConfigError> {
        if !valid_ref(&self.tenant_id, 256)
            || self.identity.origin.len() > 2_048
            || !self.storage.state_root.is_absolute()
            || (self.kubernetes.enabled != !self.kubernetes.namespaces.is_empty())
            || self
                .kubernetes
                .namespaces
                .iter()
                .any(|namespace| !valid_dns_label(namespace, 63))
            || (self.sip.enabled
                != (self.sip.listen.is_some() && self.sip.deployment_config.is_some()))
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

fn default_kubernetes_token_file() -> PathBuf {
    PathBuf::from("/var/run/secrets/kubernetes.io/serviceaccount/token")
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
    use super::*;

    #[test]
    fn hosted_integrations_are_explicit_and_fail_closed() {
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
}
