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

/// Receiver-verified attempt and grant provenance for delegated agent execution.
///
/// There is no request deserializer or public field constructor. A trusted Connector authority
/// adapter attaches this only after validating the delegated capability and current Grant set.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DelegatedExecution {
    agent_id: String,
    attempt_id: String,
    delegation_id: String,
    grant_id: String,
    grant_revision: u64,
}

impl DelegatedExecution {
    /// Admit already-verified delegated execution coordinates.
    pub fn after_verification(
        agent_id: String,
        attempt_id: String,
        delegation_id: String,
        grant_id: String,
        grant_revision: u64,
    ) -> Result<Self, PrincipalContextError> {
        if !valid_ref(&agent_id, 512)
            || !valid_ref(&attempt_id, 512)
            || !valid_ref(&delegation_id, 512)
            || !valid_ref(&grant_id, 512)
            || grant_revision == 0
        {
            return Err(PrincipalContextError);
        }
        Ok(Self {
            agent_id,
            attempt_id,
            delegation_id,
            grant_id,
            grant_revision,
        })
    }

    #[must_use]
    pub fn agent_id(&self) -> &str {
        &self.agent_id
    }

    #[must_use]
    pub fn attempt_id(&self) -> &str {
        &self.attempt_id
    }

    #[must_use]
    pub fn delegation_id(&self) -> &str {
        &self.delegation_id
    }

    #[must_use]
    pub fn grant_id(&self) -> &str {
        &self.grant_id
    }

    #[must_use]
    pub const fn grant_revision(&self) -> u64 {
        self.grant_revision
    }
}

/// Receiver-admitted authority facts passed to Connector use cases.
///
/// This is deliberately distinct from the caller-written wire [`OwnerContext`]. Transports must
/// construct it from authenticated evidence, and hosted transports never fabricate local Agent
/// generations.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PrincipalContext {
    tenant_id: String,
    realm: Option<String>,
    principal: PrincipalIdentity,
    authority_snapshot_id: String,
    authority_snapshot_sha256: String,
    verified_groups: BTreeSet<String>,
    issuer: Option<String>,
    token_id: Option<String>,
    deployment_id: Option<String>,
    request_id: Option<String>,
    trace_id: Option<String>,
    execution: Option<DelegatedExecution>,
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
            realm: None,
            principal,
            authority_snapshot_id,
            authority_snapshot_sha256,
            verified_groups,
            issuer: None,
            token_id: None,
            deployment_id: None,
            request_id: None,
            trace_id: None,
            execution: None,
        })
    }

    /// Attach delegation facts validated by a Connector-owned capability receiver.
    pub fn with_verified_execution(
        mut self,
        execution: DelegatedExecution,
    ) -> Result<Self, PrincipalContextError> {
        if self.principal.actor_subject == self.principal.subject
            || self.principal.actor_subject != execution.agent_id
        {
            return Err(PrincipalContextError);
        }
        self.execution = Some(execution);
        Ok(self)
    }

    /// Attach the exact optional realm supplied by the authentication adapter.
    ///
    /// This is deliberately not accepted from an operation envelope. An absent realm remains
    /// distinct from a realm whose literal value is `default`.
    pub fn with_verified_realm(
        mut self,
        realm: Option<String>,
    ) -> Result<Self, PrincipalContextError> {
        if realm.as_deref().is_some_and(|value| !valid_ref(value, 512)) {
            return Err(PrincipalContextError);
        }
        self.realm = realm;
        Ok(self)
    }

    /// Retain the authenticated request join carried by a hosted receiver. Local adapters do not
    /// invent these Identity facts; hosted adapters must populate them before dispatch.
    pub fn with_hosted_provenance(
        mut self,
        issuer: String,
        token_id: String,
        deployment_id: Option<String>,
        request_id: String,
        trace_id: String,
    ) -> Result<Self, PrincipalContextError> {
        if !valid_ref(&issuer, 512)
            || !valid_ref(&token_id, 512)
            || deployment_id
                .as_deref()
                .is_some_and(|value| !valid_ref(value, 512))
            || !valid_ref(&request_id, 512)
            || !valid_ref(&trace_id, 512)
        {
            return Err(PrincipalContextError);
        }
        self.issuer = Some(issuer);
        self.token_id = Some(token_id);
        self.deployment_id = deployment_id;
        self.request_id = Some(request_id);
        self.trace_id = Some(trace_id);
        Ok(self)
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

    /// Exact realm claim admitted by authentication, or absence.
    #[must_use]
    pub fn realm(&self) -> Option<&str> {
        self.realm.as_deref()
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

    #[must_use]
    pub fn issuer(&self) -> Option<&str> {
        self.issuer.as_deref()
    }

    #[must_use]
    pub fn token_id(&self) -> Option<&str> {
        self.token_id.as_deref()
    }

    #[must_use]
    pub fn deployment_id(&self) -> Option<&str> {
        self.deployment_id.as_deref()
    }

    #[must_use]
    pub fn request_id(&self) -> Option<&str> {
        self.request_id.as_deref()
    }

    #[must_use]
    pub fn trace_id(&self) -> Option<&str> {
        self.trace_id.as_deref()
    }

    /// Receiver-verified delegated execution provenance, when this call represents an agent.
    #[must_use]
    pub const fn execution(&self) -> Option<&DelegatedExecution> {
        self.execution.as_ref()
    }

    /// The provenance-free authority identity for lease and cursor derivation.
    ///
    /// Serializing the whole context into a lease digest made every lease stale by
    /// construction: `request_id`, `trace_id`, and `token_id` differ on the next
    /// authenticated request, and access tokens rotate every few minutes. The authority
    /// snapshot fields are deliberately excluded too: the hosted receiver derives both from
    /// the verified token (the id IS the token id, the sha hashes the introspection
    /// envelope), so they rotate with every token and even differ between the
    /// catalog-scoped describe and the invoke-scoped call that follows it. What remains is
    /// the authority that actually gates behavior: tenant, exact optional realm, principal identity, and verified
    /// groups — a lease therefore survives token rotation and dies on a real authority
    /// change. Fields are length-framed so adjacent values cannot alias.
    #[must_use]
    pub fn stable_authority_seed(&self) -> Vec<u8> {
        let mut seed = Vec::new();
        let mut push = |part: &str| {
            seed.extend_from_slice(&(part.len() as u64).to_be_bytes());
            seed.extend_from_slice(part.as_bytes());
        };
        push(&self.tenant_id);
        push(self.realm.as_deref().unwrap_or(""));
        push(&self.principal.subject);
        push(&self.principal.actor_subject);
        push(self.principal.email.as_deref().unwrap_or(""));
        push(
            &self
                .principal
                .agent_revision
                .map(|revision| revision.to_string())
                .unwrap_or_default(),
        );
        for group in &self.verified_groups {
            push(group);
        }
        if let Some(execution) = &self.execution {
            push(&execution.agent_id);
            push(&execution.attempt_id);
            push(&execution.delegation_id);
            push(&execution.grant_id);
            push(&execution.grant_revision.to_string());
        }
        seed
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

/// Receiver-declared admission posture for starting one Connect Session profile.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectSessionAccess {
    /// Any authenticated principal carrying the self-service connection scope may start the flow.
    SelfService,
    /// Only a Connector operator may start the flow.
    Operator,
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

    /// Whether this invocation finishes without a process-owned session, stream or subscription.
    /// This is lifetime metadata only: all ordinary ownership, Grant and approval gates still run.
    /// An embedding or new adapter that has not established this property requires a daemon.
    fn supports_ephemeral_invocation(&self, _request: &protocol::operation::InvokeRequest) -> bool {
        false
    }

    fn owns_connection(&self, _request: &ConnectionRequest) -> bool {
        false
    }

    /// Exact inventory ownership, without resource, credential, or provider I/O.
    fn owns_endpoint(&self, _request: &protocol::endpoint::EndpointRequest) -> bool {
        false
    }

    /// Credential-free inventory and operator bindings; transport authority is independently checked.
    async fn handle_endpoint(
        &self,
        _context: &PrincipalContext,
        _request: protocol::endpoint::EndpointRequest,
    ) -> Result<protocol::endpoint::EndpointResult, protocol::endpoint::EndpointError> {
        Err(protocol::endpoint::EndpointError::new(
            protocol::endpoint::EndpointErrorCode::Unavailable,
            "endpoint discovery is not configured",
            false,
        ))
    }

    /// Validate endpoint policy and current immutable resource identity, then publish or reuse
    /// non-secret Connection metadata. Source authentication may be used to validate inventory.
    /// This is not invocation admission: never resolve target credentials, open a tunnel, or
    /// dispatch the requested operation here; those happen behind the ordinary Grant seam.
    async fn resolve_endpoint(
        &self,
        _context: &PrincipalContext,
        _endpoint_ref: &str,
        _operation_ref: &str,
    ) -> Result<String, OperationError> {
        Err(OperationError::new(
            protocol::operation::OperationErrorCode::NotFound,
            "the endpoint is not available to this operation",
            false,
        ))
    }

    /// Describe exactly one admitted Connection. Providers may have separately versioned target
    /// projections; a target-aware caller must never receive another backend's schema or lease.
    async fn describe_target(
        &self,
        context: &PrincipalContext,
        operation_ref: &str,
        connection_ref: &str,
    ) -> Result<protocol::operation::OperationDescription, OperationError> {
        let result = self
            .handle(
                context,
                OperationRequest::Describe(protocol::operation::DescribeRequest {
                    operation_ref: operation_ref.into(),
                }),
            )
            .await?;
        match crate::constrain_endpoint_description(result, Some(connection_ref))? {
            OperationResult::Describe(description)
                if description.operation_ref == operation_ref =>
            {
                Ok(description)
            }
            _ => Err(OperationError::new(
                protocol::operation::OperationErrorCode::Protocol,
                "target owner returned an unrelated operation description",
                false,
            )),
        }
    }

    /// Exact ownership only; performs no credential read, session work or provider access.
    fn owns_remediation(&self, _route: crate::RemediationRoute<'_>) -> bool {
        false
    }

    /// Static configured metadata, including Created bindings, without callable discovery.
    ///
    /// Returning metadata declares an implemented bound acquisition path for this exact
    /// profile; otherwise return Unsupported. A one-shot caller still cannot start a session.
    /// Neither this lookup nor its output is a grant/management decision or output-safe data.
    fn remediation_metadata<'a>(
        &'a self,
        _context: &PrincipalContext,
        _target: crate::RemediationTarget<'_>,
    ) -> Result<crate::RemediationMetadata<'a>, crate::RemediationError> {
        Err(crate::RemediationError::Unsupported)
    }

    /// Supply current personal operation/self-management admission from the actual policy owner.
    /// Does not validate raw input, observe a credential, allocate a session or dispatch work.
    /// Hosted receivers use their real Grant store and never fall back to this personal policy.
    fn personal_remediation_admission(
        &self,
        _context: &PrincipalContext,
        _target: crate::RemediationTarget<'_>,
    ) -> Result<crate::RemediationAdmission, crate::RemediationError> {
        Err(crate::RemediationError::Unsupported)
    }

    /// Observe the exact generation only after non-consuming current grant/input admission.
    /// Never spend approval/event evidence, start acquisition, refresh, or dispatch here.
    /// Unsupported preserves the ordinary v2 path and does not manufacture an auth outcome.
    async fn credential_readiness(
        &self,
        _context: &PrincipalContext,
        _target: crate::RemediationTarget<'_>,
    ) -> crate::CredentialReadiness {
        crate::CredentialReadiness::Unsupported
    }

    /// Trusted bound start/status/ack; distinct from the unchanged Connection-v1 handler.
    ///
    /// The owner rechecks live authority on the stored binding for every request and at its
    /// publication decision, preserving its existing lock order, capacity and lifecycle.
    /// Neither Start facts nor a session reference substitute for that recheck. This method
    /// never calls ordinary operation handling or returns a reusable invocation authority.
    async fn handle_remediation(
        &self,
        _context: &PrincipalContext,
        _request: crate::RemediationRequest,
        _authority: std::sync::Arc<dyn crate::RemediationAuthority>,
    ) -> Result<crate::RemediationResult, crate::RemediationError> {
        Err(crate::RemediationError::Unsupported)
    }

    /// Declare who may start a backend-owned Connect Session profile.
    fn connect_session_access(
        &self,
        _request: &protocol::connection::ConnectSessionCreateRequest,
    ) -> ConnectSessionAccess {
        ConnectSessionAccess::Operator
    }

    /// The self-service setup flows this backend can complete for the named provider right now.
    ///
    /// This is a runtime answer, not a declaration: a backend whose deployment lacks the OAuth
    /// configuration a flow needs returns nothing for it, so a product never shows a person a
    /// control that would refuse. The default is nothing, because a backend that has not thought
    /// about self-service does not offer it.
    fn setup_profiles(&self, _provider_ref: &str) -> Vec<protocol::catalog::SetupProfileSummary> {
        Vec::new()
    }

    fn owns_event(&self, _request: &EventRequest) -> bool {
        false
    }

    /// Exact subscription ownership; never opens a stream during routing.
    fn owns_event_v2(&self, request: &protocol::event::v2::EventRequest) -> bool {
        request
            .legacy_read()
            .is_some_and(|request| self.owns_event(&request))
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

    fn owns_hosted_oauth_state(&self, _integration_ref: &str, _state: &str) -> bool {
        false
    }

    async fn complete_hosted_oauth(
        &self,
        _integration_ref: &str,
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

    /// Explicit stream lifecycle. Implementations must bind subscriptions to admitted principal,
    /// endpoint identity and current channel grant, and release routes on shutdown or revocation.
    async fn handle_event_v2(
        &self,
        context: &PrincipalContext,
        request: protocol::event::v2::EventRequest,
    ) -> Result<protocol::event::v2::EventResult, EventError> {
        if let Some(request) = request.legacy_read() {
            return self.handle_event(context, request).await.map(Into::into);
        }
        Err(EventError::new(
            EventErrorCode::NotFound,
            "no Integration owns this subscription",
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
    fn the_stable_authority_seed_ignores_request_scoped_provenance() {
        let plain = PrincipalContext::local(&owner(7)).unwrap();
        let provenanced = PrincipalContext::local(&owner(7))
            .unwrap()
            .with_hosted_provenance(
                "https://identity.example.test".to_owned(),
                "token-after-rotation".to_owned(),
                None,
                "request-2".to_owned(),
                "trace-2".to_owned(),
            )
            .unwrap();
        // A lease derived while serving one request must admit the next authenticated
        // request of the same principal: fresh request/trace ids and a rotated access
        // token are not an authority change.
        assert_eq!(
            plain.stable_authority_seed(),
            provenanced.stable_authority_seed()
        );
        let other_principal = PrincipalContext::hosted(
            "tenant-test".to_owned(),
            "person:other".to_owned(),
            "person:other".to_owned(),
            None,
            "snapshot-test".to_owned(),
            "a".repeat(64),
        )
        .unwrap();
        assert_ne!(
            plain.stable_authority_seed(),
            other_principal.stable_authority_seed()
        );
    }

    #[test]
    fn the_stable_authority_seed_survives_token_scoped_snapshot_fields() {
        // The hosted receiver fills the snapshot id with the token id and the snapshot
        // sha with a hash of the introspection envelope, so both rotate with every access
        // token. The same admitted principal must keep the same seed across them.
        let first = PrincipalContext::hosted(
            "tenant-test".to_owned(),
            "person:owner".to_owned(),
            "person:owner".to_owned(),
            None,
            "token-catalog-scope".to_owned(),
            "c".repeat(64),
        )
        .unwrap();
        let second = PrincipalContext::hosted(
            "tenant-test".to_owned(),
            "person:owner".to_owned(),
            "person:owner".to_owned(),
            None,
            "token-invoke-scope".to_owned(),
            "d".repeat(64),
        )
        .unwrap();
        assert_eq!(
            first.stable_authority_seed(),
            second.stable_authority_seed()
        );
    }

    #[test]
    fn the_stable_authority_seed_distinguishes_absent_and_literal_default_realms() {
        let hosted = || {
            PrincipalContext::hosted(
                "tenant-test".to_owned(),
                "person:owner".to_owned(),
                "person:owner".to_owned(),
                None,
                "snapshot-test".to_owned(),
                "a".repeat(64),
            )
            .unwrap()
        };
        let absent = hosted().stable_authority_seed();
        let default = hosted()
            .with_verified_realm(Some("default".to_owned()))
            .unwrap()
            .stable_authority_seed();
        assert_ne!(absent, default);
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
        assert_eq!(admitted.execution(), None);
    }

    #[test]
    fn delegated_execution_must_match_the_authenticated_actor() {
        let hosted = PrincipalContext::hosted(
            "tenant-test".to_owned(),
            "person:owner".to_owned(),
            "agent:worker".to_owned(),
            None,
            "token-test".to_owned(),
            "b".repeat(64),
        )
        .unwrap();
        let execution = DelegatedExecution::after_verification(
            "agent:worker".to_owned(),
            "attempt:one".to_owned(),
            "delegation:one".to_owned(),
            "grant:one".to_owned(),
            7,
        )
        .unwrap();
        let admitted = hosted.clone().with_verified_execution(execution).unwrap();
        assert_eq!(admitted.execution().unwrap().attempt_id(), "attempt:one");
        assert_ne!(
            hosted.stable_authority_seed(),
            admitted.stable_authority_seed()
        );

        let wrong = DelegatedExecution::after_verification(
            "agent:other".to_owned(),
            "attempt:one".to_owned(),
            "delegation:one".to_owned(),
            "grant:one".to_owned(),
            7,
        )
        .unwrap();
        assert!(hosted.with_verified_execution(wrong).is_err());
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

// Contract tests for the non-consuming remediation ports.
#[cfg(test)]
mod remediation_contract_tests {
    use std::sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    };

    use async_trait::async_trait;
    use protocol::connection_v2::{
        RemediationAcknowledgeRequest, RemediationAcknowledgement, RemediationNextAction,
        RemediationStatusRequest,
    };
    use protocol::operation::{
        v3::{AuthenticationAttemptState, AuthenticationNeed, AuthenticationNextAction},
        OperationError, OperationErrorCode, OperationRequest, OperationResult,
    };

    use crate::remediation::*;
    use crate::{BackendReadinessError, ConnectorBackend, PrincipalContext};

    const PRIVATE: &str = "https://private.example.test/SYNTHETIC_PRIVATE_INSTRUCTION";

    struct OrdinaryBackend(Arc<AtomicUsize>);

    #[async_trait]
    impl ConnectorBackend for OrdinaryBackend {
        async fn ready(&self) -> Result<(), BackendReadinessError> {
            self.0.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }

        // The signature intentionally remains the existing ordinary wire-v2 port.
        async fn handle(
            &self,
            _context: &PrincipalContext,
            _request: OperationRequest,
        ) -> Result<OperationResult, OperationError> {
            self.0.fetch_add(1, Ordering::SeqCst);
            Err(OperationError::new(
                OperationErrorCode::Unavailable,
                "ordinary fixture refusal",
                false,
            ))
        }
    }

    struct RefusingAuthority(Arc<AtomicUsize>);

    impl RemediationAuthority for RefusingAuthority {
        fn recheck(
            &self,
            _context: &PrincipalContext,
            _binding: &RemediationBinding,
            _now_unix_ms: u64,
        ) -> Result<(), RemediationError> {
            self.0.fetch_add(1, Ordering::SeqCst);
            Err(RemediationError::Refused)
        }
    }

    fn context() -> PrincipalContext {
        PrincipalContext::hosted(
            "fixture-tenant".into(),
            "fixture-person".into(),
            "fixture-person".into(),
            None,
            "fixture-snapshot".into(),
            "a".repeat(64),
        )
        .unwrap()
    }

    fn binding() -> RemediationBinding {
        // Synthetic test data, never asserted to name production authority or real digests.
        RemediationBinding {
            operation_ref: PRIVATE.into(),
            connection_ref: PRIVATE.into(),
            integration_ref: PRIVATE.into(),
            auth_profile: "oauth".into(),
            need: AuthenticationNeed::AuthorizeConfigured,
            canonical_input_sha256: "a".repeat(64),
            stable_authority_sha256: "b".repeat(64),
            grant_ref: "fixture-only-policy".into(),
            grant_revision: None,
            admission_policy_sha256: "c".repeat(64),
            expires_at_unix_ms: 10_000,
        }
    }

    #[test]
    fn personal_remediation_factory_defaults_to_refusal_without_backend_work() {
        let work = Arc::new(AtomicUsize::new(0));
        let backend: Arc<dyn ConnectorBackend> = Arc::new(OrdinaryBackend(work.clone()));
        for (operation_ref, connection_ref) in [("unknown", "unknown"), (PRIVATE, PRIVATE)] {
            let result = backend.personal_remediation_admission(
                &context(),
                RemediationTarget {
                    operation_ref,
                    connection_ref,
                },
            );
            assert_eq!(result.unwrap_err(), RemediationError::Unsupported);
        }
        assert_eq!(work.load(Ordering::SeqCst), 0);
    }

    #[tokio::test]
    async fn remediation_defaults_do_not_run_existing_backend_work() {
        let work = Arc::new(AtomicUsize::new(0));
        let backend: Arc<dyn ConnectorBackend> = Arc::new(OrdinaryBackend(work.clone()));
        let context = context();
        for (operation_ref, connection_ref) in [
            ("unknown-operation", "unknown-connection"),
            (PRIVATE, PRIVATE),
        ] {
            let target = RemediationTarget {
                operation_ref,
                connection_ref,
            };
            assert!(!backend.owns_remediation(RemediationRoute::Target(target)));
            assert_eq!(
                backend.remediation_metadata(&context, target).unwrap_err(),
                RemediationError::Unsupported,
            );
            assert_eq!(
                backend.credential_readiness(&context, target).await,
                CredentialReadiness::Unsupported,
            );
        }
        assert!(!backend.owns_remediation(RemediationRoute::Session(PRIVATE)));
        assert_eq!(work.load(Ordering::SeqCst), 0);
    }

    #[tokio::test]
    async fn remediation_default_bound_methods_refuse_without_authority_or_operation_work() {
        let work = Arc::new(AtomicUsize::new(0));
        let checks = Arc::new(AtomicUsize::new(0));
        let backend: Arc<dyn ConnectorBackend> = Arc::new(OrdinaryBackend(work.clone()));
        let authority: Arc<dyn RemediationAuthority> = Arc::new(RefusingAuthority(checks.clone()));
        let context = context();
        let requests = [
            RemediationRequest::Start(Box::new(binding())),
            RemediationRequest::Status(RemediationStatusRequest {
                connect_session_ref: PRIVATE.into(),
            }),
            RemediationRequest::Acknowledge(RemediationAcknowledgeRequest {
                connect_session_ref: PRIVATE.into(),
                operation_ref: PRIVATE.into(),
                connection_ref: PRIVATE.into(),
            }),
        ];
        for request in requests {
            let error = backend
                .handle_remediation(&context, request, authority.clone())
                .await
                .unwrap_err();
            assert_eq!(error, RemediationError::Unsupported);
            assert_eq!(
                error.to_string(),
                "authentication remediation is unsupported"
            );
            assert!(!format!("{error:?}").contains(PRIVATE));
        }
        assert_eq!(work.load(Ordering::SeqCst), 0);
        assert_eq!(checks.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn remediation_internal_diagnostics_do_not_trust_printable_reference_fields() {
        let facts = protocol::operation::v3::AuthenticationRequired {
            operation_ref: PRIVATE.into(),
            connection_ref: PRIVATE.into(),
            integration_ref: PRIVATE.into(),
            auth_profile: "oauth".into(),
            need: AuthenticationNeed::AuthorizeConfigured,
            attempt: AuthenticationAttemptState::NotAttempted,
            next_action: AuthenticationNextAction::StartTrustedRemediation,
        };
        // This is deliberately valid under the unchanged DTO contract. Validity is not secrecy.
        facts.validate().unwrap();
        let mut error = protocol::operation::v3::OperationError::authentication_required(facts);
        error.message = PRIVATE.into();
        error.validate().unwrap();
        let envelope =
            protocol::operation::v3::ResponseEnvelope::failure("fixture-response", error);
        envelope.validate().unwrap();
        let bytes = serde_json::to_vec(&envelope).unwrap();
        let (version, decoded) = protocol::operation::versions::decode_response(&bytes).unwrap();
        assert_eq!(version, protocol::operation::versions::Version::V0Alpha3);
        assert_eq!(decoded.error.unwrap().message, PRIVATE);

        let bound = binding();
        assert_eq!(bound.operation_ref, PRIVATE);
        assert_eq!(format!("{bound:?}"), "RemediationBinding(<redacted>)");
        let request = RemediationRequest::Start(Box::new(bound));
        assert_eq!(format!("{request:?}"), "RemediationRequest(<redacted>)");
        let result = RemediationResult::Acknowledged(RemediationAcknowledgement {
            connect_session_ref: PRIVATE.into(),
            operation_ref: PRIVATE.into(),
            connection_ref: PRIVATE.into(),
            next_action: RemediationNextAction::FreshDescriptionThenExplicitInvoke,
        });
        assert_eq!(format!("{result:?}"), "RemediationResult(<redacted>)");
        // Real client/console/MCP output tests for the valid envelope remain separately required.
    }
}
