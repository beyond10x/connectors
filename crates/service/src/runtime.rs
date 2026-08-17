//! Transport-independent runtime application port and admitted caller context.

use std::collections::BTreeSet;
use std::num::NonZeroU64;

use async_trait::async_trait;
use protocol::connection::{
    ConnectionError, ConnectionErrorCode, ConnectionRequest, ConnectionResult,
};
use protocol::datasource::{
    DatasourceError, DatasourceErrorCode, DatasourceRequest, DatasourceResult,
};
use protocol::event::{EventError, EventErrorCode, EventRequest, EventResult};
use protocol::operation::{OperationError, OperationRequest, OperationResult, OwnerContext};
use serde::Serialize;
use zeroize::Zeroizing;

/// The identity which actually reached the Connector application boundary.
///
/// Personal-local requests carry a revisioned Agent identity. Hosted Identity envelopes carry a
/// principal and actor but no invented Agent revision; the absence is represented explicitly.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PrincipalIdentity {
    subject: String,
    actor_subject: String,
    email: Option<String>,
    agent_revision: Option<NonZeroU64>,
}

/// Receiver-admitted authority facts passed to Connector use cases.
///
/// This is deliberately distinct from the caller-written wire [`OwnerContext`]. Transports must
/// construct it from authenticated evidence, and hosted transports never fabricate local Agent
/// generations.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PrincipalContext {
    tenant_id: String,
    principal: PrincipalIdentity,
    authority_snapshot_id: String,
    authority_snapshot_sha256: String,
    verified_groups: BTreeSet<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("admitted Connector principal context is invalid")]
pub struct PrincipalContextError;

impl PrincipalContext {
    /// Admit a validated personal-local wire owner as a revisioned Agent principal.
    pub fn local(owner: &OwnerContext) -> Result<Self, PrincipalContextError> {
        let revision = NonZeroU64::new(owner.agent_revision).ok_or(PrincipalContextError)?;
        Self::new(
            owner.tenant_id.clone(),
            PrincipalIdentity {
                subject: owner.agent_id.clone(),
                actor_subject: owner.agent_id.clone(),
                email: None,
                agent_revision: Some(revision),
            },
            owner.authority_snapshot_id.clone(),
            owner.authority_snapshot_sha256.clone(),
            BTreeSet::new(),
        )
    }

    /// Admit one Identity-verified hosted principal without manufacturing an Agent revision.
    pub fn hosted(
        tenant_id: String,
        subject: String,
        actor_subject: String,
        email: Option<String>,
        authority_snapshot_id: String,
        authority_snapshot_sha256: String,
    ) -> Result<Self, PrincipalContextError> {
        Self::new(
            tenant_id,
            PrincipalIdentity {
                subject,
                actor_subject,
                email,
                agent_revision: None,
            },
            authority_snapshot_id,
            authority_snapshot_sha256,
            BTreeSet::new(),
        )
    }

    /// Admit one Identity-verified hosted principal and its receiver-visible group facts.
    pub fn hosted_with_groups(
        tenant_id: String,
        subject: String,
        actor_subject: String,
        email: Option<String>,
        authority_snapshot_id: String,
        authority_snapshot_sha256: String,
        verified_groups: BTreeSet<String>,
    ) -> Result<Self, PrincipalContextError> {
        Self::new(
            tenant_id,
            PrincipalIdentity {
                subject,
                actor_subject,
                email,
                agent_revision: None,
            },
            authority_snapshot_id,
            authority_snapshot_sha256,
            verified_groups,
        )
    }

    fn new(
        tenant_id: String,
        principal: PrincipalIdentity,
        authority_snapshot_id: String,
        authority_snapshot_sha256: String,
        verified_groups: BTreeSet<String>,
    ) -> Result<Self, PrincipalContextError> {
        if !valid_ref(&tenant_id, 512)
            || !valid_ref(&principal.subject, 512)
            || !valid_ref(&principal.actor_subject, 512)
            || principal.email.as_deref().is_some_and(|email| {
                email.len() > 254
                    || !email.is_ascii()
                    || email.chars().any(char::is_whitespace)
                    || email.split('@').count() != 2
            })
            || !valid_ref(&authority_snapshot_id, 512)
            || authority_snapshot_sha256.len() != 64
            || !authority_snapshot_sha256
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
            || verified_groups.iter().any(|group| {
                group.is_empty()
                    || group.len() > 64
                    || !group.bytes().all(|byte| {
                        byte.is_ascii_lowercase()
                            || byte.is_ascii_digit()
                            || matches!(byte, b'_' | b'-')
                    })
            })
        {
            return Err(PrincipalContextError);
        }
        Ok(Self {
            tenant_id,
            principal,
            authority_snapshot_id,
            authority_snapshot_sha256,
            verified_groups,
        })
    }

    #[must_use]
    pub fn actor_subject(&self) -> &str {
        &self.principal.actor_subject
    }

    #[must_use]
    pub fn email(&self) -> Option<&str> {
        self.principal.email.as_deref()
    }

    #[must_use]
    pub fn tenant_id(&self) -> &str {
        &self.tenant_id
    }

    #[must_use]
    pub fn authority_snapshot_id(&self) -> &str {
        &self.authority_snapshot_id
    }

    #[must_use]
    pub fn authority_snapshot_sha256(&self) -> &str {
        &self.authority_snapshot_sha256
    }

    #[must_use]
    pub fn subject(&self) -> &str {
        &self.principal.subject
    }

    #[must_use]
    pub const fn agent_revision(&self) -> Option<NonZeroU64> {
        self.principal.agent_revision
    }

    /// Exact Identity-verified group facts visible only inside the Connector receiver.
    #[must_use]
    pub fn verified_groups(&self) -> &BTreeSet<String> {
        &self.verified_groups
    }
}

fn valid_ref(value: &str, maximum: usize) -> bool {
    !value.is_empty() && value.len() <= maximum && value.bytes().all(|byte| byte.is_ascii_graphic())
}

/// Coarse route families implemented by a backend. Search is aggregated over every backend which
/// advertises the corresponding family; non-search dispatch additionally uses the ownership
/// predicates on [`ConnectorBackend`].
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct BackendCapabilities {
    pub operations: bool,
    pub connections: bool,
    pub events: bool,
    pub datasources: bool,
}

/// Value-free readiness failure for one configured Integration dependency.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("a configured Connector backend dependency is unavailable")]
pub struct BackendReadinessError;

/// Secret-bearing, bounded completion payload. Diagnostics never reveal its contents and its
/// allocation is cleared on drop.
pub struct HostedCompletionSubmission(Zeroizing<Vec<u8>>);

impl HostedCompletionSubmission {
    #[must_use]
    pub fn new(value: Vec<u8>) -> Self {
        Self(Zeroizing::new(value))
    }

    /// Allocate the secret-owned receive buffer before transport bytes arrive. Callers which
    /// enforce an upper bound should reserve that complete bound so extending the buffer cannot
    /// reallocate previously received secret material into ordinary freed heap storage.
    #[must_use]
    pub fn with_capacity(capacity: usize) -> Self {
        Self(Zeroizing::new(Vec::with_capacity(capacity)))
    }

    /// Append one transport chunk without ever reallocating the zeroizing-owned receive buffer.
    /// Returns `false` when the reserved capacity cannot admit the complete chunk.
    #[must_use]
    pub fn extend_from_slice(&mut self, value: &[u8]) -> bool {
        if value.len() > self.0.capacity().saturating_sub(self.0.len()) {
            return false;
        }
        self.0.extend_from_slice(value);
        true
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.0.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    #[must_use]
    pub fn expose_secret(&self) -> &[u8] {
        self.0.as_slice()
    }
}

impl std::fmt::Debug for HostedCompletionSubmission {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("HostedCompletionSubmission(<redacted>)")
    }
}

/// Value-free hosted Connect Session page supplied by the owning Integration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HostedCompletionPage {
    pub title: String,
    pub html: String,
}

/// Closed refusal vocabulary for capability-authenticated hosted credential completion.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum HostedCompletionError {
    #[error("hosted Connect Session was not found")]
    NotFound,
    #[error("hosted Connect Session capability was refused")]
    Refused,
    #[error("hosted Connect Session submission was invalid")]
    Invalid,
    #[error("hosted Connect Session completion is unavailable")]
    Unavailable,
}

impl BackendCapabilities {
    pub const OPERATIONS: Self = Self {
        operations: true,
        connections: false,
        events: false,
        datasources: false,
    };
}

/// Connectors-owned application boundary.
///
/// Ownership predicates must be value-free and must cover dynamic runtime references (including
/// execution, Connection, Connect Session, observation, candidate, channel, and event refs). A
/// router selects exactly one owner before calling a non-search handler; `NotFound` is therefore a
/// domain result, never a routing signal. Search remains an aggregate operation selected through
/// [`BackendCapabilities`].
#[async_trait]
pub trait ConnectorBackend: Send + Sync + 'static {
    /// Check mandatory backend dependencies without reading a credential or widening authority.
    /// Every backend must state its posture explicitly; there is no fail-open default.
    async fn ready(&self) -> Result<(), BackendReadinessError>;

    fn capabilities(&self) -> BackendCapabilities {
        BackendCapabilities::OPERATIONS
    }

    fn owns_operation(&self, _request: &OperationRequest) -> bool {
        false
    }

    fn owns_connection(&self, _request: &ConnectionRequest) -> bool {
        false
    }

    fn owns_event(&self, _request: &EventRequest) -> bool {
        false
    }

    fn owns_datasource(&self, _request: &DatasourceRequest) -> bool {
        false
    }

    fn owns_hosted_completion(&self, _connect_session_ref: &str) -> bool {
        false
    }

    fn hosted_completion_page(
        &self,
        _connect_session_ref: &str,
    ) -> Result<HostedCompletionPage, HostedCompletionError> {
        Err(HostedCompletionError::NotFound)
    }

    async fn complete_hosted_session(
        &self,
        _connect_session_ref: &str,
        _capability: &str,
        _submission: HostedCompletionSubmission,
    ) -> Result<(), HostedCompletionError> {
        Err(HostedCompletionError::NotFound)
    }

    fn owns_hosted_oauth_state(&self, _state: &str) -> bool {
        false
    }

    async fn complete_hosted_oauth(
        &self,
        _state: &str,
        _code: Option<&str>,
        _error: Option<&str>,
    ) -> Result<(), HostedCompletionError> {
        Err(HostedCompletionError::NotFound)
    }

    async fn handle(
        &self,
        context: &PrincipalContext,
        request: OperationRequest,
    ) -> Result<OperationResult, OperationError>;

    async fn handle_connection(
        &self,
        _context: &PrincipalContext,
        _request: ConnectionRequest,
    ) -> Result<ConnectionResult, ConnectionError> {
        Err(ConnectionError::new(
            ConnectionErrorCode::Unavailable,
            "connection management is not configured",
            false,
        ))
    }

    async fn handle_event(
        &self,
        _context: &PrincipalContext,
        _request: EventRequest,
    ) -> Result<EventResult, EventError> {
        Err(EventError::new(
            EventErrorCode::Unavailable,
            "event delivery is not configured",
            false,
        ))
    }

    async fn handle_datasource(
        &self,
        _context: &PrincipalContext,
        _request: DatasourceRequest,
    ) -> Result<DatasourceResult, DatasourceError> {
        Err(DatasourceError::new(
            DatasourceErrorCode::Unavailable,
            "datasources are not configured",
            false,
        ))
    }

    /// Terminate and join backend-owned work before its transport endpoint disappears.
    async fn shutdown(&self) {}
}

#[cfg(test)]
mod tests {
    use super::*;

    fn owner(revision: u64) -> OwnerContext {
        OwnerContext {
            tenant_id: "tenant-test".to_owned(),
            agent_id: "agent-test".to_owned(),
            agent_revision: revision,
            authority_snapshot_id: "snapshot-test".to_owned(),
            authority_snapshot_sha256: "a".repeat(64),
        }
    }

    #[test]
    fn local_principals_require_a_real_agent_revision() {
        assert!(PrincipalContext::local(&owner(0)).is_err());
        let admitted = PrincipalContext::local(&owner(7)).unwrap();
        assert_eq!(admitted.subject(), "agent-test");
        assert_eq!(admitted.actor_subject(), "agent-test");
        assert_eq!(admitted.agent_revision().unwrap().get(), 7);
    }

    #[test]
    fn hosted_principals_do_not_fabricate_agent_revisions() {
        let admitted = PrincipalContext::hosted(
            "tenant-test".to_owned(),
            "person:owner".to_owned(),
            "service:caller".to_owned(),
            Some("owner@example.test".to_owned()),
            "token-test".to_owned(),
            "b".repeat(64),
        )
        .unwrap();
        assert_eq!(admitted.subject(), "person:owner");
        assert_eq!(admitted.actor_subject(), "service:caller");
        assert_eq!(admitted.email(), Some("owner@example.test"));
        assert_eq!(admitted.agent_revision(), None);
    }

    #[test]
    fn hosted_completion_submission_never_reallocates_received_secret_material() {
        let mut submission = HostedCompletionSubmission::with_capacity(8);
        assert!(submission.extend_from_slice(b"secret"));
        let allocation = submission.expose_secret().as_ptr();
        assert!(!submission.extend_from_slice(b"overflow"));
        assert_eq!(submission.expose_secret(), b"secret");
        assert_eq!(submission.expose_secret().as_ptr(), allocation);
    }
}
