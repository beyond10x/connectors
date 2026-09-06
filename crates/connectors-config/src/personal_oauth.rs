//! Explicit personal OAuth deployment admission and binding validation.

use crate::personal::{config_ref, CatalogIntegrationConfig, ConfigError, InitiationConfig};
use serde::{Deserialize, Serialize};

/// Explicit personal acquisition selection, independent of legacy catalog OAuth defaults.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PersonalOAuthFlow {
    AuthorizationCodePkce,
    DeviceAuthorization,
}

/// Per-registration client authentication. Only Public is initially implemented.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OAuthClientAuthentication {
    Public,
    ClientSecretPost,
}

/// Where the human's browser runs relative to the persistent local daemon.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OAuthBrowserPlacement {
    SameMachine,
    OtherMachine,
}

/// Deployment selection; provider permission and storage protection are checked independently.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OAuthRegistrationUse {
    DevelopmentOnly,
    ProductionAllowed,
}

/// Dedicated prepared credential custody. This option is durable and unsealed at rest.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PersonalOAuthCustody {
    DevelopmentFile,
}

/// Deployment registration and grant ceiling; never a compiled provider registration value.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PersonalOAuthRegistration {
    pub auth_profile: String,
    pub flow: PersonalOAuthFlow,
    pub client_authentication: OAuthClientAuthentication,
    pub client_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub client_secret_ref: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub redirect_uri: Option<String>,
    pub browser_placement: OAuthBrowserPlacement,
    pub registration_use: OAuthRegistrationUse,
    pub custody: PersonalOAuthCustody,
    #[serde(default = "personal_oauth_ttl")]
    pub session_ttl_seconds: u64,
    pub allowed_scopes: Vec<String>,
}

const fn personal_oauth_ttl() -> u64 {
    300
}

impl PersonalOAuthRegistration {
    /// Refuse unsupported posture before session allocation, listener creation or egress.
    pub fn validate(&self) -> Result<(), ConfigError> {
        if !config_ref(&self.auth_profile, 256)
            || !config_ref(&self.client_id, 4096)
            || self.client_authentication != OAuthClientAuthentication::Public
            || self.client_secret_ref.is_some()
            || self.registration_use != OAuthRegistrationUse::DevelopmentOnly
            || !(30..=600).contains(&self.session_ttl_seconds)
            || self.allowed_scopes.is_empty()
            || self.allowed_scopes.len() > 64
            || self.allowed_scopes.iter().any(|s| {
                s.is_empty()
                    || s.len() > 256
                    || !s
                        .bytes()
                        .all(|b| b.is_ascii_graphic() && !matches!(b, b'"' | b'\\' | b','))
            })
            || self
                .allowed_scopes
                .iter()
                .collect::<std::collections::BTreeSet<_>>()
                .len()
                != self.allowed_scopes.len()
        {
            return Err(ConfigError::Invalid);
        }
        match self.flow {
            PersonalOAuthFlow::AuthorizationCodePkce => {
                if self.browser_placement != OAuthBrowserPlacement::SameMachine {
                    return Err(ConfigError::Invalid);
                }
                let raw = self.redirect_uri.as_deref().ok_or(ConfigError::Invalid)?;
                let uri = url::Url::parse(raw).map_err(|_| ConfigError::Invalid)?;
                if raw.len() > 1024
                    || uri.as_str() != raw
                    || uri.scheme() != "http"
                    || uri.host_str() != Some("127.0.0.1")
                    || uri.port().is_none_or(|port| port == 0)
                    || !uri.username().is_empty()
                    || uri.password().is_some()
                    || uri.query().is_some()
                    || uri.fragment().is_some()
                    || uri.path().len() <= 1
                    || uri.path().contains(['%', '\\'])
                    || !uri.path().bytes().all(|b| b.is_ascii_graphic())
                    || uri.path()[1..]
                        .split('/')
                        .any(|p| p.is_empty() || p == "." || p == "..")
                {
                    return Err(ConfigError::Invalid);
                }
            }
            PersonalOAuthFlow::DeviceAuthorization if self.redirect_uri.is_some() => {
                return Err(ConfigError::Invalid)
            }
            PersonalOAuthFlow::DeviceAuthorization => {}
        }
        Ok(())
    }
}

impl CatalogIntegrationConfig {
    pub(crate) fn validate(&self) -> Result<(), ConfigError> {
        if self.provider.is_empty()
            || !config_ref(&self.grant_ref, 512)
            || matches!(self.initiation, InitiationConfig::Provider)
            || self
                .credential_file
                .as_deref()
                .is_some_and(|path| !path.is_absolute())
        {
            return Err(ConfigError::Invalid);
        }
        if let Some(oauth) = &self.oauth {
            oauth.validate()?;
            if self.credential.as_deref() != Some(oauth.auth_profile.as_str())
                || self.credential_file.is_some()
                || !self.usernames.is_empty()
            {
                return Err(ConfigError::Invalid);
            }
        }
        // A user half is a credential *name* and a printable account identifier, both bounded.
        // Refused here rather than at assembly time so a configuration that cannot work is a
        // configuration the daemon never starts on.
        for (credential, user) in &self.usernames {
            if !config_ref(credential, 256) || !config_ref(user, 512) {
                return Err(ConfigError::Invalid);
            }
        }
        Ok(())
    }
}
