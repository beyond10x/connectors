//! Hosted GitLab and internal Git byte-plane placement.

use std::net::SocketAddr;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::file::{read_trusted_config, TrustedConfigReadError, TrustedOwner};
use crate::hosted::{HostedServerConfig, HostedServerConfigError, MAX_CONFIG_BYTES};
use crate::personal::InitiationConfig;

/// Value-free policy for delegated GitLab user Connections.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HostedGitlabConfig {
    /// Exact self-managed GitLab origin; the Integration appends `/api/v4` itself.
    pub origin: String,
    /// Public Connectors origin used to construct one-use setup pages.
    pub public_origin: String,
    /// Internal TLS origin serving the Git byte plane. Absence disables fetch-session creation.
    #[serde(default)]
    pub git_fetch_origin: Option<String>,
    pub oauth_client_id: String,
    pub oauth_redirect_uri: String,
    pub user_grant_ref: String,
    pub initiation: InitiationConfig,
    #[serde(default = "default_connect_session_ttl_seconds")]
    pub connect_session_ttl_seconds: u64,
    #[serde(default = "default_gitlab_refresh_skew_seconds")]
    pub refresh_skew_seconds: u64,
}

/// Value-free TLS listener configuration for the internal Git byte plane.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HostedTlsListenerConfig {
    pub listen: SocketAddr,
    pub certificate_file: PathBuf,
    pub private_key_file: PathBuf,
}

/// Secret-free operational placement for the internal Git byte plane.
///
/// Deployers may supply this alongside an otherwise immutable hosted configuration. Applying it
/// replaces both halves of the Git fetch placement atomically and happens before validation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HostedGitFetchOverride {
    pub origin: String,
    pub listen: SocketAddr,
    pub certificate_file: PathBuf,
    pub private_key_file: PathBuf,
}

impl HostedServerConfig {
    /// Read a trusted hosted configuration and atomically apply an operational Git fetch
    /// placement before validating the composed result.
    pub fn read_with_git_fetch_override(
        path: &Path,
        git_fetch: Option<&HostedGitFetchOverride>,
    ) -> Result<Self, HostedServerConfigError> {
        let text = read_trusted_config(path, MAX_CONFIG_BYTES, TrustedOwner::CurrentUserOrRoot)
            .map_err(|error| match error {
                TrustedConfigReadError::Io(_) => HostedServerConfigError::Read,
                TrustedConfigReadError::Unsafe => HostedServerConfigError::Invalid,
            })?;
        let mut config: Self = toml::from_str(&text).map_err(|_| HostedServerConfigError::Parse)?;
        if let Some(git_fetch) = git_fetch {
            let gitlab = config
                .gitlab
                .as_mut()
                .ok_or(HostedServerConfigError::Invalid)?;
            gitlab.git_fetch_origin = Some(git_fetch.origin.clone());
            config.server.git_fetch_tls = Some(HostedTlsListenerConfig {
                listen: git_fetch.listen,
                certificate_file: git_fetch.certificate_file.clone(),
                private_key_file: git_fetch.private_key_file.clone(),
            });
        }
        config.validate()?;
        Ok(config)
    }
}

const fn default_connect_session_ttl_seconds() -> u64 {
    300
}

const fn default_gitlab_refresh_skew_seconds() -> u64 {
    300
}
