//! Private configured adapter composition. No database or credential resolver is
//! exported to business adapters. See contracts/cli/v1alpha1/private-adapter.md.
pub(crate) mod artifact;
pub(crate) mod channel;
mod process;
mod server;
pub mod state;
pub use process::Child;
pub use server::{Adapter, serve};

use super::registry;
use connectors_core::{Descriptor, ErrorCode};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

pub const VERSION: &str = "connectors-private/1";
pub const SECRET_LIMIT: usize = 65536;
pub const INPUT_LIMIT: usize = 1024 * 1024;
pub const RESULT_LIMIT: usize = 8 * 1024 * 1024;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Failure {
    InvalidInput,
    InvalidCredential,
    InsufficientScope,
    IdentityMismatch,
    Forbidden,
    Unavailable,
    Timeout,
    Interrupted,
    Protocol,
    ReadinessMismatch,
    IncarnationMismatch,
    InvalidConfiguration,
    Capacity,
    Unsupported,
    NotFound,
    StaleDescription,
    StaleCursor,
    /// Native invocation failures distinct from host admission; no raw data.
    ProviderNotFound,
    ProviderRateLimited,
    ProviderInternal,
}
pub type Result<T> = std::result::Result<T, Failure>;

#[cfg(test)]
mod failure_tests {
    use super::*;

    #[test]
    fn provider_codes_survive_the_private_boundary_without_raw_error_data() {
        for code in [
            ErrorCode::NotFound,
            ErrorCode::RateLimited,
            ErrorCode::Internal,
        ] {
            let failure = Failure::from_provider(connectors_core::Error::new(
                code.clone(),
                "private-provider-message",
            ));
            let bytes = serde_json::to_vec(&failure).unwrap();
            assert!(!String::from_utf8_lossy(&bytes).contains("private-provider-message"));
            let failure: Failure = serde_json::from_slice(&bytes).unwrap();
            let error: crate::local::owner::Error = failure.into();
            assert_eq!(error.code, crate::local::owner::Code::ServiceFailure);
            assert_eq!(error.service_code, Some(code));
        }
        let local = Failure::from_service(connectors_core::Error::new(
            ErrorCode::NotFound,
            "missing selection",
        ));
        assert_eq!(local, Failure::NotFound);
        assert!(serde_json::from_str::<Failure>(r#"{"provider":"unreviewed_code"}"#).is_err());
    }
}

impl Failure {
    pub fn from_provider(error: connectors_core::Error) -> Self {
        match error.code {
            ErrorCode::NotFound => Self::ProviderNotFound,
            ErrorCode::RateLimited => Self::ProviderRateLimited,
            ErrorCode::Internal => Self::ProviderInternal,
            _ => Self::from_service(error),
        }
    }
    pub fn from_service(error: connectors_core::Error) -> Self {
        match error.code {
            ErrorCode::InvalidInput => Self::InvalidInput,
            ErrorCode::Unsupported => Self::Unsupported,
            ErrorCode::Unauthorized => Self::InvalidCredential,
            ErrorCode::Forbidden => Self::Forbidden,
            ErrorCode::NotFound => Self::NotFound,
            ErrorCode::StaleDescription => Self::StaleDescription,
            ErrorCode::StaleCursor => Self::StaleCursor,
            ErrorCode::Timeout => Self::Timeout,
            ErrorCode::Capacity => Self::Capacity,
            ErrorCode::UpstreamProtocol => Self::Protocol,
            ErrorCode::Internal | ErrorCode::Unavailable | ErrorCode::RateLimited => {
                Self::Unavailable
            }
        }
    }
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EntryField {
    pub name: String,
    pub label: String,
    pub max_bytes: u32,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Profile {
    pub id: String,
    pub revision: String,
    pub purpose: registry::Purpose,
    pub subject: registry::Subject,
    pub scheme: String,
    pub capability: String,
    pub minimum_scopes: BTreeSet<String>,
    pub evidence_lifetime_ms: u64,
    pub fields: Vec<EntryField>,
}
impl Profile {
    pub fn registry_profile(&self) -> registry::StaticProfile {
        registry::StaticProfile {
            id: self.id.clone(),
            revision: self.revision.clone(),
            purpose: self.purpose,
            subject: self.subject,
            minimum_scopes: self.minimum_scopes.clone(),
            evidence_lifetime_ms: self.evidence_lifetime_ms,
        }
    }
}
#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Effect {
    Read,
    Write,
    Unknown,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Requirement {
    pub operation: String,
    pub profile: String,
    pub scopes: BTreeSet<String>,
    pub effect: Effect,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Bootstrap {
    pub instance: String,
    pub adapter: String,
    pub protocol: String,
    pub configuration_revision: String,
    pub provider_authority: String,
    pub descriptor: String,
    pub profiles: Vec<Profile>,
    pub requirements: Vec<Requirement>,
}
impl Bootstrap {
    pub fn descriptor(&self) -> Result<Descriptor> {
        connectors_core::read_json(self.descriptor.as_bytes()).map_err(|_| Failure::Protocol)
    }
    pub fn profile(&self, id: &str) -> Result<&Profile> {
        self.profiles
            .iter()
            .find(|p| p.id == id)
            .ok_or(Failure::Unsupported)
    }
    pub fn binding(&self, id: &str) -> Result<registry::Binding> {
        Ok(registry::Binding {
            instance_id: self.instance.clone(),
            adapter_id: self.adapter.clone(),
            configuration_revision: self.configuration_revision.clone(),
            provider_authority: self.provider_authority.clone(),
            profile: self.profile(id)?.registry_profile(),
        })
    }
    pub fn validate(&self) -> Result<()> {
        if ![&self.instance, &self.adapter, &self.configuration_revision]
            .iter()
            .all(|v| connectors_core::valid_id(v))
            || self.protocol != connectors_core::WIRE_VERSION
            || !bounded(&self.provider_authority, 4096)
            || self.descriptor.len() > INPUT_LIMIT
            || self.profiles.is_empty()
            || self.profiles.len() > 64
            || self.requirements.len() > 256
        {
            return Err(Failure::Protocol);
        }
        let descriptor = self.descriptor()?;
        if descriptor.instance != self.instance
            || descriptor.adapter != self.adapter
            || descriptor.version != self.protocol
            || !connectors_core::valid_id(&descriptor.revision)
            || descriptor.operations.len() != self.requirements.len()
        {
            return Err(Failure::Protocol);
        }
        let mut profiles = BTreeSet::new();
        for profile in &self.profiles {
            if !profiles.insert(&profile.id)
                || !connectors_core::valid_id(&profile.id)
                || !connectors_core::valid_id(&profile.revision)
                || !scopes(&profile.minimum_scopes)
                || profile.evidence_lifetime_ms == 0
                || profile.evidence_lifetime_ms > 300_000
                || (profile.purpose == registry::Purpose::DelegatedUser)
                    != (profile.subject == registry::Subject::User)
                || !matches!(
                    (profile.scheme.as_str(), profile.capability.as_str()),
                    ("http_bearer", "http-bearer")
                        | ("http_basic", "http-basic")
                        | ("mtls", "mtls-client-identity")
                        | ("session_authority", "session-authority")
                )
                || profile.fields.is_empty()
                || profile.fields.len() > 16
            {
                return Err(Failure::Protocol);
            }
            let mut fields = BTreeSet::new();
            for field in &profile.fields {
                if !fields.insert(&field.name)
                    || !connectors_core::valid_id(&field.name)
                    || !bounded(&field.label, 128)
                    || !(1..=65536).contains(&field.max_bytes)
                {
                    return Err(Failure::Protocol);
                }
            }
        }
        let mut operations = BTreeSet::new();
        for operation in &descriptor.operations {
            if !operations.insert(&operation.id) || !connectors_core::valid_id(&operation.id) {
                return Err(Failure::Protocol);
            }
            // Compilation of the schemas catches unusable reviewed declarations
            // before the owner can capture credentials for this artifact.
            for schema in [&operation.input_schema, &operation.output_schema] {
                if let Err(error) = connectors_sdk::validate(schema, &serde_json::Value::Null) {
                    // Rejection of null is ordinary validation. Compilation of an
                    // invalid schema is the SDK's distinct Internal failure.
                    if error.code != ErrorCode::InvalidInput {
                        return Err(Failure::Protocol);
                    }
                }
            }
        }
        let mut required = BTreeSet::new();
        for requirement in &self.requirements {
            if !required.insert(&requirement.operation)
                || !operations.contains(&requirement.operation)
                || !profiles.contains(&requirement.profile)
                || !scopes(&requirement.scopes)
            {
                return Err(Failure::Protocol);
            }
        }
        Ok(())
    }
}
fn bounded(value: &str, max: usize) -> bool {
    !value.is_empty() && value.len() <= max && !value.chars().any(char::is_control)
}
fn scopes(value: &BTreeSet<String>) -> bool {
    value.len() <= 64 && value.iter().all(|v| bounded(v, 256))
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Baseline {
    pub identity: registry::ExternalIdentity,
    pub granted_scopes: Option<BTreeSet<String>>,
    pub credential_expires_at_ms: Option<u64>,
    pub collected_at_ms: u64,
    pub valid_until_ms: u64,
}
impl Baseline {
    pub fn into_registry(self) -> registry::ValidatedBaseline {
        registry::ValidatedBaseline {
            identity: self.identity,
            granted_scopes: self.granted_scopes,
            credential_expires_at_ms: self.credential_expires_at_ms,
            collected_at_ms: self.collected_at_ms,
            valid_until_ms: self.valid_until_ms,
        }
    }
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
enum Request {
    Hello {
        version: String,
        nonce: String,
        child_incarnation: String,
    },
    Validate {
        request_id: String,
        profile: String,
        deadline_ms: u64,
    },
    Invoke {
        request_id: String,
        operation: String,
        revision: String,
        partition: String,
        deadline_ms: u64,
    },
    Stop {
        request_id: String,
    },
}
#[derive(Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
enum Reply {
    Ready {
        version: String,
        nonce: String,
        child_incarnation: String,
        bootstrap: Bootstrap,
    },
    Validated {
        request_id: String,
        baseline: Baseline,
    },
    Success {
        request_id: String,
    },
    Stopped {
        request_id: String,
    },
    Failed {
        request_id: String,
        code: Failure,
    },
}
