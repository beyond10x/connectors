//! Private configured adapter composition. No database or credential resolver is
//! exported to business adapters. See contracts/cli/v1alpha1/private-adapter.md.
pub(crate) mod artifact;
pub(crate) mod channel;
pub mod launch;
mod process;
mod server;
pub mod state;
mod writes;
pub use process::{Child, PreparedInvocation};
pub use server::{Adapter, serve};
pub use writes::{PreparedWrite, WriteEffect, WriteResult};

use super::registry;
use connectors_core::{Descriptor, ErrorCode};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

pub const VERSION: &str = "connectors-private/1";
pub const WRITE_VERSION: &str = "connectors-private/2";
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum PrivateProtocol {
    #[serde(rename = "connectors-private/1")]
    V1,
    #[serde(rename = "connectors-private/2")]
    V2,
}
impl PrivateProtocol {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::V1 => VERSION,
            Self::V2 => WRITE_VERSION,
        }
    }
}
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
    /// The upstream answered 403. Additive to the private protocol: a host
    /// that predates it refuses the reply as unreadable, and a child that
    /// predates it still sends `forbidden`, which reads as admission.
    ProviderForbidden,
    /// The upstream's own timeout or capacity answer to a sent request (a
    /// transport deadline that passed after sending, a database statement
    /// timeout, a marked capacity refusal). Additive like `provider_forbidden`:
    /// a host that predates them refuses the reply as unreadable.
    ProviderTimeout,
    ProviderCapacity,
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
            let failure = Failure::from_provider(
                connectors_core::Error::new(code.clone(), "private-provider-message").answered(),
            );
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

    /// story:service-failure-carries-upstream-reason.
    #[test]
    fn a_providers_reason_crosses_the_private_boundary_only_beside_its_own_answer() {
        use crate::local::owner::{Code, Error};
        let scope = "Unauthorized; scope does not match";
        let answered = |code| {
            connectors_core::Error::new(code, "private-provider-message")
                .answered()
                .with_upstream_reason(Some(scope.into()))
        };
        let refusal = Refusal::from_provider(answered(ErrorCode::Unauthorized));
        assert_eq!(refusal.failure, Failure::InvalidCredential);
        assert_eq!(refusal.reason.as_deref(), Some(scope));
        let error: Error = refusal.into();
        assert_eq!(error.code, Code::ServiceFailure);
        assert_eq!(error.service_code, Some(ErrorCode::Unauthorized));
        assert_eq!(error.service_reason.as_deref(), Some(scope));
        let bytes = serde_json::to_string(&error).unwrap();
        assert_eq!(
            bytes,
            r#"{"code":"service_failure","service_code":"unauthorized","service_reason":"Unauthorized; scope does not match"}"#
        );
        let back: Error = serde_json::from_str(&bytes).unwrap();
        assert_eq!(back.service_reason.as_deref(), Some(scope));
        // A provider 403 keeps its reason beside its provider origin.
        let refused: Error = Refusal::from_provider(answered(ErrorCode::Forbidden)).into();
        assert_eq!(refused.code, Code::Forbidden);
        assert_eq!(refused.service_reason.as_deref(), Some(scope));
        // A failure the host reports as its own carries none: a 5xx reads as
        // unavailability, and an adapter's own refusal is not the provider's.
        let unavailable: Error = Refusal::from_provider(answered(ErrorCode::Unavailable)).into();
        assert_eq!(unavailable.code, Code::Unavailable);
        assert_eq!(unavailable.service_reason, None);
        let raised = Refusal::from_provider(
            connectors_core::Error::new(ErrorCode::Unauthorized, "configured")
                .with_upstream_reason(Some(scope.into())),
        );
        assert_eq!(raised.reason, None);
        assert_eq!(
            Error::from(Refusal::from(Failure::Protocol)).service_reason,
            None
        );
        // The private reply carries the reason only when there is one, so a
        // failure without one keeps its earlier bytes.
        let without = Reply::Failed {
            request_id: "r".into(),
            code: Failure::InvalidCredential,
            reason: None,
        };
        assert_eq!(
            serde_json::to_string(&without).unwrap(),
            r#"{"kind":"failed","request_id":"r","code":"invalid_credential"}"#
        );
        let with: Reply = serde_json::from_str(
            r#"{"kind":"failed","request_id":"r","code":"invalid_credential","reason":"Unauthorized; scope does not match"}"#,
        )
        .unwrap();
        assert!(matches!(with, Reply::Failed { reason: Some(r), .. } if r == scope));
    }

    /// The host admits a child's reason again, against the credential document
    /// it sent: the child is not trusted to have redacted it.
    #[test]
    fn the_host_drops_a_childs_reason_that_carries_the_credential_or_is_unbounded() {
        let secret = connectors_sdk::Secret(br#"{"token":"fixture-pat-one"}"#.to_vec());
        let scope = "Unauthorized; scope does not match";
        assert_eq!(
            admitted(Some(scope.into()), &secret).as_deref(),
            Some(scope)
        );
        for withheld in [
            "Unauthorized for fixture-pat-one".to_owned(),
            "Bearer abc".to_owned(),
            format!("{}a", "abc ".repeat(64)),
            "two\nlines".to_owned(),
            String::new(),
        ] {
            assert_eq!(
                admitted(Some(withheld.clone()), &secret),
                None,
                "{withheld:?}"
            );
        }
        assert_eq!(admitted(None, &secret), None);
    }

    #[test]
    fn a_provider_refusal_keeps_its_code_and_names_the_provider_as_origin() {
        use crate::local::owner::{Code, Error, Origin};
        let answered = |code| {
            Failure::from_provider(
                connectors_core::Error::new(code, "private-provider-message").answered(),
            )
        };
        let raised =
            |code| Failure::from_provider(connectors_core::Error::new(code, "configured scope"));
        let refused: Error = answered(ErrorCode::Forbidden).into();
        assert_eq!(refused.code, Code::Forbidden);
        assert_eq!(refused.origin, Origin::Provider);
        let missing: Error = answered(ErrorCode::NotFound).into();
        assert_eq!(missing.code, Code::ServiceFailure);
        assert_eq!(missing.service_code, Some(ErrorCode::NotFound));
        assert_eq!(missing.origin, Origin::Provider);
        // An adapter's own refusal before any request is not the provider's.
        let scope: Error = raised(ErrorCode::Forbidden).into();
        assert_eq!(scope.code, Code::Forbidden);
        assert_eq!(scope.origin, Origin::Host);
        let absent: Error = raised(ErrorCode::NotFound).into();
        assert_eq!(absent.code, Code::NotFound);
        assert_eq!(absent.origin, Origin::Host);
        // Private protocol: a child that predates the variant sends
        // `forbidden`, which still reads as the host's; the new variant is
        // its own code.
        let old: Failure = serde_json::from_str(r#""forbidden""#).unwrap();
        assert_eq!(Error::from(old).origin, Origin::Host);
        let new: Failure = serde_json::from_str(r#""provider_forbidden""#).unwrap();
        assert_eq!(new, Failure::ProviderForbidden);
        assert_eq!(
            serde_json::to_string(&Failure::ProviderForbidden).unwrap(),
            r#""provider_forbidden""#
        );
        let admission: Error = Code::Forbidden.into();
        assert_eq!(admission.origin, Origin::Host);
        // The host origin is the owner/1 default and is not written.
        let bytes = serde_json::to_string(&admission).unwrap();
        assert_eq!(bytes, r#"{"code":"forbidden"}"#);
        let bytes = serde_json::to_string(&refused).unwrap();
        assert_eq!(bytes, r#"{"code":"forbidden","origin":"provider"}"#);
        let back: Error = serde_json::from_str(&bytes).unwrap();
        assert_eq!(back.origin, Origin::Provider);
        let host: Error = Failure::NotFound.into();
        assert_eq!(host.origin, Origin::Host);
    }

    #[test]
    fn a_provider_timeout_or_capacity_answer_is_the_providers_and_the_hosts_own_is_not() {
        use crate::local::owner::{Code, Error, Origin};
        for (code, wire, owner) in [
            (ErrorCode::Timeout, r#""provider_timeout""#, Code::Timeout),
            (
                ErrorCode::Capacity,
                r#""provider_capacity""#,
                Code::Capacity,
            ),
        ] {
            let answered = Failure::from_provider(
                connectors_core::Error::new(code.clone(), "private-provider-message").answered(),
            );
            let bytes = serde_json::to_string(&answered).unwrap();
            assert_eq!(bytes, wire);
            let back: Failure = serde_json::from_str(&bytes).unwrap();
            let error: Error = back.into();
            assert_eq!(error.code, owner);
            assert_eq!(error.origin, Origin::Provider);
            assert_eq!(error.service_code, None);
            // Raised by the adapter or the host before or without an answer.
            let raised: Error =
                Failure::from_provider(connectors_core::Error::new(code, "own limit")).into();
            assert_eq!(raised.code, owner);
            assert_eq!(raised.origin, Origin::Host);
        }
        for host in [Failure::Timeout, Failure::Capacity] {
            assert_eq!(Error::from(host).origin, Origin::Host);
        }
        // Unsupported is raised before dispatch whatever its source.
        let unsupported: Error = Failure::from_provider(
            connectors_core::Error::new(ErrorCode::Unsupported, "x").answered(),
        )
        .into();
        assert_eq!(unsupported.origin, Origin::Host);
    }
}

/// A dispatched read's failure and, when it is the upstream's own answer, the
/// bounded reason the upstream gave (`connectors_core::reason`). The failure
/// stays the closed private code; the reason never decides anything.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Refusal {
    pub failure: Failure,
    pub reason: Option<String>,
}
impl From<Failure> for Refusal {
    fn from(failure: Failure) -> Self {
        Self {
            failure,
            reason: None,
        }
    }
}
impl Refusal {
    /// [`Failure::from_provider`], keeping the reason only of an upstream's
    /// own answer.
    pub fn from_provider(error: connectors_core::Error) -> Self {
        let reason = error
            .upstream_reason
            .as_deref()
            .filter(|_| error.upstream_answer)
            .and_then(connectors_core::reason::admit);
        Self {
            failure: Failure::from_provider(error),
            reason,
        }
    }
}
/// A child's reason as the host accepts it: already in admitted form, and
/// holding no piece of the credential document the host sent for the call.
/// Anything else is dropped; the failure itself still stands.
pub(crate) fn admitted(reason: Option<String>, secret: &connectors_sdk::Secret) -> Option<String> {
    reason.filter(|reason| {
        connectors_core::reason::admit(reason).as_deref() == Some(reason.as_str())
            && !connectors_core::reason::carries_credential(reason, &secret.0)
    })
}

impl Failure {
    pub fn from_provider(error: connectors_core::Error) -> Self {
        // Only the upstream's own answer is the provider refusing. A refusal the
        // adapter raises from its configuration before any request keeps the
        // host-side code and is reported at admission.
        match error.code {
            ErrorCode::Forbidden if error.upstream_answer => Self::ProviderForbidden,
            ErrorCode::NotFound if error.upstream_answer => Self::ProviderNotFound,
            ErrorCode::Timeout if error.upstream_answer => Self::ProviderTimeout,
            ErrorCode::Capacity if error.upstream_answer => Self::ProviderCapacity,
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
    /// Present only for a profile whose material is acquired by OAuth consent;
    /// omitted otherwise, so every other profile keeps its bytes and revision.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub acquisition: Option<Acquisition>,
}
/// Where a CLI obtains an OAuth profile's protected entry: the consent and
/// token endpoints and the scopes it requests. Nonsecret metadata only.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Acquisition {
    pub authorize_url: String,
    pub token_url: String,
    pub scopes: BTreeSet<String>,
}
impl Acquisition {
    /// Both endpoints https with a host and at most 2048 bytes; 1 to 64 scopes,
    /// each 1 to 256 printable ASCII bytes without whitespace.
    fn bounded(&self) -> bool {
        let endpoint = |value: &str| {
            value.len() <= 2048
                && url::Url::parse(value).is_ok_and(|url| {
                    url.scheme() == "https" && url.host_str().is_some_and(|host| !host.is_empty())
                })
        };
        endpoint(&self.authorize_url)
            && endpoint(&self.token_url)
            && (1..=64).contains(&self.scopes.len())
            && self.scopes.iter().all(|scope| {
                (1..=256).contains(&scope.len()) && scope.bytes().all(|b| b.is_ascii_graphic())
            })
    }
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
    pub fn validate_for(&self, protocol: PrivateProtocol) -> Result<()> {
        self.validate()?;
        if self.requirements.iter().any(|requirement| {
            requirement.effect == Effect::Unknown
                || (protocol == PrivateProtocol::V1 && requirement.effect != Effect::Read)
        }) {
            return Err(Failure::Unsupported);
        }
        Ok(())
    }
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
                || profile
                    .acquisition
                    .as_ref()
                    .is_some_and(|acquisition| !acquisition.bounded())
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
        /// The upstream's admitted reason for a failed `invoke`; omitted when
        /// there is none. Additive: a host that predates it refuses the reply.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        reason: Option<String>,
    },
}
