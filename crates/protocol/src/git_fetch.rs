//! Short-lived, read-only Git repository acquisition.

use serde::{Deserialize, Serialize};

use crate::operation::OwnerContext;

/// Exact control-plane contract identity.
pub const CONTRACT: &str = "b10x.connector-git-fetch.v1";
/// Maximum control request frame accepted by the hosted route.
pub const MAX_FRAME_BYTES: usize = 16 * 1024;
/// Maximum control response frame returned by the hosted route.
pub const MAX_RESPONSE_BYTES: usize = 16 * 1024;
/// Largest shallow history admitted by a fetch session.
pub const MAX_DEPTH: u8 = 50;

/// Correlated request to acquire one exact repository snapshot.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RequestEnvelope {
    /// Exact protocol identity.
    pub protocol: String,
    /// Caller-generated correlation identity.
    pub request_id: String,
    /// Non-authoritative caller context, checked against receiver-derived Identity.
    pub context: OwnerContext,
    /// Exact repository selection.
    pub request: CreateRequest,
}

/// Provider repository and revision requested by the caller.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CreateRequest {
    /// Caller-stable retry identity. Replays keep the locator stable and rotate authority.
    pub idempotency_key: String,
    /// Principal-owned Connector Connection.
    pub connection_ref: String,
    /// GitLab numeric project identity.
    pub project_id: u64,
    /// Provider-selected branch, normally the project's current default branch.
    pub reference: String,
    /// Exact lowercase hexadecimal commit expected at the branch tip.
    pub expected_commit: String,
    /// Requested shallow history, from one through [`MAX_DEPTH`].
    pub depth: u8,
}

/// Successful creation response. `source_authorization` is emitted exactly once.
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CreatedSession {
    /// Exact protocol identity.
    pub protocol: String,
    /// Echo of the request correlation identity.
    pub request_id: String,
    /// Opaque, non-secret fetch-session identity.
    pub session_ref: String,
    /// Configured Substrate source name.
    pub source: String,
    /// Stable internal HTTPS Smart Git locator containing no credential.
    pub locator: String,
    /// Provider-verified branch.
    pub reference: String,
    /// Provider-verified exact commit.
    pub expected_commit: String,
    /// Admitted shallow history.
    pub depth: u8,
    /// Receiver-clock expiry as Unix milliseconds.
    pub expires_at_unix_ms: u64,
    /// One-use source capability, presented only to the internal byte plane.
    pub source_authorization: String,
}

impl RequestEnvelope {
    /// Validate input shape before Identity, Connection, or provider lookup.
    #[must_use]
    pub fn is_valid(&self) -> bool {
        self.protocol == CONTRACT
            && valid_ref(&self.request_id, 128)
            && valid_ref(&self.context.tenant_id, 512)
            && valid_ref(&self.context.agent_id, 512)
            && self.context.agent_revision > 0
            && valid_ref(&self.context.authority_snapshot_id, 512)
            && is_sha256(&self.context.authority_snapshot_sha256)
            && self.request.is_valid()
    }
}

impl CreateRequest {
    /// Validate exact repository coordinate shape independently of a transport envelope.
    #[must_use]
    pub fn is_valid(&self) -> bool {
        valid_ref(&self.connection_ref, 512)
            && valid_ref(&self.idempotency_key, 128)
            && self.project_id > 0
            && valid_git_reference(&self.reference)
            && is_commit(&self.expected_commit)
            && (1..=MAX_DEPTH).contains(&self.depth)
    }
}

impl CreatedSession {
    /// Validate correlation and the credential-free source coordinates at a client boundary.
    #[must_use]
    pub fn is_valid(&self, request_id: &str) -> bool {
        let locator = url::Url::parse(&self.locator);
        self.protocol == CONTRACT
            && self.request_id == request_id
            && valid_ref(&self.session_ref, 512)
            && valid_source_name(&self.source)
            && locator.is_ok_and(|locator| {
                locator.scheme() == "https"
                    && locator.host_str().is_some()
                    && locator.username().is_empty()
                    && locator.password().is_none()
                    && locator.query().is_none()
                    && locator.fragment().is_none()
                    && locator.path().ends_with(".git")
            })
            && valid_git_reference(&self.reference)
            && is_commit(&self.expected_commit)
            && (1..=MAX_DEPTH).contains(&self.depth)
            && self.expires_at_unix_ms > 0
            && valid_authority(&self.source_authorization)
    }
}

fn valid_ref(value: &str, maximum: usize) -> bool {
    !value.is_empty() && value.len() <= maximum && value.bytes().all(|byte| byte.is_ascii_graphic())
}

fn is_commit(value: &str) -> bool {
    value.len() == 40
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn is_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn valid_source_name(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
}

fn valid_authority(value: &str) -> bool {
    (32..=256).contains(&value.len())
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
}

fn valid_git_reference(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 1_024
        && value.is_ascii()
        && !value.starts_with('-')
        && !value.starts_with('/')
        && !value.ends_with('/')
        && !value.ends_with('.')
        && !value.contains("..")
        && !value.contains("@{")
        && !value.contains("//")
        && !value
            .bytes()
            .any(|byte| byte <= b' ' || byte == 0x7f || b"~^:?*[\\".contains(&byte))
        && value
            .split('/')
            .all(|part| !part.is_empty() && part != "." && !part.ends_with(".lock"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn envelope() -> RequestEnvelope {
        RequestEnvelope {
            protocol: CONTRACT.to_owned(),
            request_id: "request-one".to_owned(),
            context: OwnerContext {
                tenant_id: "tenant-one".to_owned(),
                agent_id: "workspace".to_owned(),
                agent_revision: 1,
                authority_snapshot_id: "snapshot-one".to_owned(),
                authority_snapshot_sha256: "a".repeat(64),
            },
            request: CreateRequest {
                idempotency_key: "coding-session-one".to_owned(),
                connection_ref: "connection:gitlab:one".to_owned(),
                project_id: 42,
                reference: "trunk/release".to_owned(),
                expected_commit: "b".repeat(40),
                depth: 50,
            },
        }
    }

    #[test]
    fn request_refuses_unbounded_or_ambiguous_revisions() {
        assert!(envelope().is_valid());
        for reference in ["", "-branch", "branch..other", "branch@{1}", "a//b"] {
            let mut request = envelope();
            request.request.reference = reference.to_owned();
            assert!(!request.is_valid(), "{reference}");
        }
        let mut request = envelope();
        request.request.depth = MAX_DEPTH + 1;
        assert!(!request.is_valid());
    }

    #[test]
    fn response_refuses_secret_or_non_tls_locators() {
        let mut response = CreatedSession {
            protocol: CONTRACT.to_owned(),
            request_id: "request-one".to_owned(),
            session_ref: "git-fetch:one".to_owned(),
            source: "gitlab".to_owned(),
            locator: "https://connectors.internal/internal/git-fetch/one/repository.git".to_owned(),
            reference: "trunk".to_owned(),
            expected_commit: "c".repeat(40),
            depth: 1,
            expires_at_unix_ms: 1,
            source_authorization: "x".repeat(43),
        };
        assert!(response.is_valid("request-one"));
        response.locator = "https://secret@connectors.internal/repository.git".to_owned();
        assert!(!response.is_valid("request-one"));
    }
}
