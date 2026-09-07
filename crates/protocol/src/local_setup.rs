//! Confidential enrollment over the owner-only local socket. Never available over hosted HTTP.
#![allow(missing_docs)]

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::PathBuf;
use zeroize::Zeroize as _;

pub const CONTRACT: &str = "b10x.connector-local-setup.v0alpha1";
pub const MAX_FRAME_BYTES: usize = 64 * 1024;

#[derive(Serialize, Deserialize)]
#[serde(transparent)]
pub struct SecretValue(String);

impl SecretValue {
    pub fn new(value: String) -> Self {
        Self(value)
    }
    pub fn expose(&self) -> &str {
        &self.0
    }
}
impl std::fmt::Debug for SecretValue {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("SecretValue([REDACTED])")
    }
}
impl Drop for SecretValue {
    fn drop(&mut self) {
        self.0.zeroize();
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RequestEnvelope {
    pub protocol: String,
    pub request_id: String,
    pub request: LocalSetupRequest,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(
    tag = "method",
    content = "params",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum LocalSetupRequest {
    Enroll(EnrollRequest),
    AuthStatus {},
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EnrollRequest {
    pub provider: String,
    pub instance: Option<String>,
    pub credential: String,
    pub endpoints: BTreeMap<String, String>,
    pub usernames: BTreeMap<String, String>,
    pub allow_writes: bool,
    pub operator_network: bool,
    pub source: CredentialSource,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum CredentialSource {
    Pasted {
        value: SecretValue,
    },
    File {
        path: PathBuf,
    },
    Argocd {
        username: String,
        password: SecretValue,
        project: String,
        role: String,
        expires_in_seconds: u64,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, thiserror::Error)]
#[serde(rename_all = "snake_case")]
pub enum SetupError {
    #[error("the enrollment input was refused")]
    InvalidInput,
    #[error("the daemon enrollment configuration was refused")]
    Configuration,
    #[error("credential enrollment is unavailable")]
    Unavailable,
    #[error("credential acquisition outcome is unknown; inspect the provider before retrying")]
    OutcomeUnknown,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ResponseEnvelope {
    pub protocol: String,
    pub request_id: String,
    pub result: Option<serde_json::Value>,
    pub error: Option<SetupError>,
}

impl RequestEnvelope {
    pub fn valid(&self) -> bool {
        self.protocol == CONTRACT
            && valid_text(&self.request_id, 128)
            && match &self.request {
                LocalSetupRequest::Enroll(request) => request.valid(),
                LocalSetupRequest::AuthStatus {} => true,
            }
    }
}

impl EnrollRequest {
    pub fn valid(&self) -> bool {
        valid_text(&self.provider, 128)
            && valid_text(&self.credential, 256)
            && self
                .instance
                .as_ref()
                .is_none_or(|value| valid_text(value, 256))
            && [&self.endpoints, &self.usernames]
                .into_iter()
                .all(|values| {
                    values.len() <= 64
                        && values
                            .iter()
                            .all(|(key, value)| valid_text(key, 256) && valid_text(value, 4096))
                })
            && match &self.source {
                CredentialSource::Pasted { value } => {
                    !value.expose().trim().is_empty() && value.expose().len() <= 32 * 1024
                }
                CredentialSource::File { path } => {
                    path.is_absolute() && path.as_os_str().len() <= 4096
                }
                CredentialSource::Argocd {
                    username,
                    password,
                    project,
                    role,
                    expires_in_seconds,
                } => {
                    self.provider == "argocd"
                        && valid_text(username, 256)
                        && valid_text(project, 256)
                        && valid_text(role, 256)
                        && !password.expose().is_empty()
                        && password.expose().len() <= 8192
                        && (1..=366 * 24 * 3600).contains(expires_in_seconds)
                }
            }
    }
}

fn valid_text(value: &str, bound: usize) -> bool {
    !value.is_empty() && value.len() <= bound && !value.chars().any(char::is_control)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn credential_debug_never_contains_its_value() {
        let value = CredentialSource::Pasted {
            value: SecretValue::new("private-sentinel".into()),
        };
        let debug = format!("{value:?}");
        assert!(!debug.contains("private-sentinel"));
        assert!(debug.contains("REDACTED"));
    }
}
