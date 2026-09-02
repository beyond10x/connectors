//! Identity-authenticated hosted transport for the Connector operation contract.

use std::collections::BTreeSet;
use std::sync::Arc;

use async_trait::async_trait;
use axum::body::Body;
use axum::extract::{Path as AxumPath, Query, State};
use axum::http::{header, HeaderMap, StatusCode};
use axum::response::{Html, IntoResponse, Json, Response};
use axum::Router;
use futures_util::StreamExt as _;
use protocol::connection::MAX_FRAME_BYTES as CONNECTION_MAX_FRAME_BYTES;
use protocol::datasource::{
    DatasourceError, DatasourceErrorCode, RequestEnvelope as DatasourceRequestEnvelope,
    ResponseEnvelope as DatasourceResponseEnvelope, MAX_FRAME_BYTES as DATASOURCE_MAX_FRAME_BYTES,
};
use protocol::event::{
    EventError, EventErrorCode, RequestEnvelope as EventRequestEnvelope,
    ResponseEnvelope as EventResponseEnvelope, MAX_FRAME_BYTES as EVENT_MAX_FRAME_BYTES,
};
use protocol::operation::{
    OperationError, OperationErrorCode, OperationRequest, RequestEnvelope, ResponseEnvelope,
    MAX_FRAME_BYTES as OPERATION_MAX_FRAME_BYTES,
};
use serde::{Deserialize, Serialize};
use service::{AdminRegistry, ConnectorBackend, HostedCompletionError, HostedCompletionSubmission};
use subscription_custody::SubscriptionCustody;

mod admin;
mod admission;
mod approval;
mod catalog_route;
mod connect;
mod connection_route;
mod docs;
mod enforcement;
mod health;
mod mcp;
mod principal;
mod subscription;

pub use admin::router_with_admin;

use admission::{
    dispatch_admitted, dispatch_admitted_signal, enforcement_refusal_response, redescribe,
    session_for_signal,
};
use enforcement::InvokeAdmission;
pub use enforcement::{
    canonical_input_digest, issue_approval, HostedAuthority, RecoveryError, SIGNAL_AUDIT_STATE_KEY,
};
pub use principal::HostedPrincipal;

pub const CONNECTORS_AUDIENCE: &str = "urn:b10x:connectors";
const MAX_COMPLETION_BYTES: usize = 8 * 1024;

/// Receiver-owned admission policy for effect-bearing hosted Connector request families.
#[derive(Debug, Clone, Default)]
pub struct HostedAdmissionPolicy {
    operator_groups: BTreeSet<String>,
    kubernetes_read_groups: BTreeSet<String>,
    kubernetes_restart_groups: BTreeSet<String>,
    monitoring_read_groups: BTreeSet<String>,
    generated_service_operations: BTreeSet<String>,
}

impl HostedAdmissionPolicy {
    #[must_use]
    pub fn new(operator_groups: impl IntoIterator<Item = String>) -> Self {
        Self {
            operator_groups: operator_groups.into_iter().collect(),
            kubernetes_read_groups: BTreeSet::new(),
            kubernetes_restart_groups: BTreeSet::new(),
            monitoring_read_groups: BTreeSet::new(),
            generated_service_operations: BTreeSet::new(),
        }
    }

    /// Add the broad receiver gate for configured Kubernetes groups. The Integration performs the
    /// exact namespace check again before every read or mutation.
    #[must_use]
    pub fn with_kubernetes_groups(
        mut self,
        read_groups: impl IntoIterator<Item = String>,
        restart_groups: impl IntoIterator<Item = String>,
    ) -> Self {
        self.kubernetes_read_groups = read_groups.into_iter().collect();
        self.kubernetes_restart_groups = restart_groups.into_iter().collect();
        self
    }

    /// Add the broad receiver gate for the deployment-managed monitoring integration. The
    /// Integration repeats the exact tenant, group, Connection, and provider checks itself.
    #[must_use]
    pub fn with_monitoring_groups(mut self, read_groups: impl IntoIterator<Item = String>) -> Self {
        self.monitoring_read_groups = read_groups.into_iter().collect();
        self
    }

    /// Admit explicitly deployed generated-service operations to the ordinary Grant and approval
    /// decision path. This is receiver reach, not authority: effectful calls still require their
    /// durable exact-operation Grant and, when described, one-time approval.
    #[must_use]
    pub fn with_generated_service_operations(
        mut self,
        operations: impl IntoIterator<Item = String>,
    ) -> Self {
        self.generated_service_operations = operations.into_iter().collect();
        self
    }

    fn admits_operator(&self, principal: &HostedPrincipal) -> bool {
        !self.operator_groups.is_empty() && !self.operator_groups.is_disjoint(&principal.groups)
    }

    fn admits_operation(
        &self,
        principal: &HostedPrincipal,
        request: &protocol::operation::OperationRequest,
    ) -> bool {
        self.admits_operator(principal)
            || matches!(request, protocol::operation::OperationRequest::Invoke(invoke)
                if self.generated_service_operations.contains(&invoke.operation_ref))
            || tenant_member_module_read(request)
            || self.admits_kubernetes_operation(principal, request)
            || self.admits_monitoring_operation(principal, request)
    }

    fn admits_kubernetes_operation(
        &self,
        principal: &HostedPrincipal,
        request: &protocol::operation::OperationRequest,
    ) -> bool {
        let protocol::operation::OperationRequest::Invoke(invoke) = request else {
            return false;
        };
        let groups = match invoke.operation_ref.as_str() {
            "kubernetes.deployment.status" => &self.kubernetes_read_groups,
            "kubernetes.pod.logs" => &self.kubernetes_read_groups,
            "kubernetes.deployment.rollout-restart" => &self.kubernetes_restart_groups,
            _ => return false,
        };
        !groups.is_empty() && !groups.is_disjoint(&principal.groups)
    }

    fn admits_monitoring_operation(
        &self,
        principal: &HostedPrincipal,
        request: &protocol::operation::OperationRequest,
    ) -> bool {
        let protocol::operation::OperationRequest::Invoke(invoke) = request else {
            return false;
        };
        matches!(
            invoke.operation_ref.as_str(),
            "grafana-dashboards-list"
                | "grafana-dashboard-get"
                | "grafana-datasources-list"
                | "prometheus-query-range"
                | "loki-query-range"
                | "alertmanager-alerts-list"
        ) && !self.monitoring_read_groups.is_empty()
            && !self.monitoring_read_groups.is_disjoint(&principal.groups)
    }
}

fn tenant_member_module_read(request: &protocol::operation::OperationRequest) -> bool {
    let protocol::operation::OperationRequest::Invoke(invoke) = request else {
        return false;
    };
    matches!(
        invoke.operation_ref.as_str(),
        "work/request.get"
            | "work/request.list"
            | "work/task.get"
            | "work/task.list"
            | "work.requests.get"
            | "work.requests.list"
            | "work.tasks.get"
            | "work.tasks.list"
            | "work-request-get"
            | "work-request-list"
            | "work-task-get"
            | "work-task-list"
            | "ontology/branch.list"
            | "ontology/claim.explain"
            | "ontology/claim.query"
            | "ontology.branches.list"
            | "ontology.claims.explain"
            | "ontology.claims.query"
            | "ontology-branch-list"
            | "knowledge-explain"
            | "knowledge-query"
            | "planner/project.list"
            | "planner/board.get"
            | "planner/search.project"
            | "planner/entity.get"
            | "planner/entity.list"
            | "planner/story.transition.explain"
            | "planner/sync.preview"
            | "planner/sync.conflict.list"
            | "planner/search.all"
            | "planner/sync.session.preview"
            | "planner.projects.list"
            | "planner.board.get"
            | "planner.entities.get"
            | "planner.entities.list"
            | "planner.stories.transitions.explain"
            | "planner.sync.conflicts.list"
            | "planner.search.all"
            | "planner.sync.sessions.preview"
            | "planner-project-list"
            | "planner-board-get"
            | "planner-search"
            | "planner-entity-get"
            | "planner-entity-list"
            | "planner-story-transition-explain"
            | "planner-sync-preview"
            | "planner-sync-conflict-list"
            | "planner-search-all"
            | "planner-sync-session-preview"
    )
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum IdentityVerificationError {
    #[error("Identity authority is unavailable")]
    Unavailable,
    #[error("Identity authority refused the presented access token")]
    Refused,
}

#[async_trait]
pub trait IdentityVerifier: Send + Sync + 'static {
    async fn ready(&self) -> Result<(), IdentityVerificationError>;

    /// Public origin whose CLI login discovery issues credentials accepted by this verifier.
    fn login_origin(&self) -> Option<&str> {
        None
    }

    async fn verify(
        &self,
        credential: &str,
        audience: &str,
    ) -> Result<HostedPrincipal, IdentityVerificationError>;
}

#[derive(Clone)]
struct HostedState {
    verifier: Arc<dyn IdentityVerifier>,
    backend: Arc<dyn ConnectorBackend>,
    policy: HostedAdmissionPolicy,
    authority: HostedAuthority,
    subscription_custody: Option<Arc<SubscriptionCustody>>,
    admin: Option<Arc<AdminRegistry>>,
}

#[derive(Debug, Serialize)]
#[serde(deny_unknown_fields)]
struct ErrorBody {
    error: &'static str,
}

pub fn router(
    verifier: Arc<dyn IdentityVerifier>,
    backend: Arc<dyn ConnectorBackend>,
    policy: HostedAdmissionPolicy,
    authority: HostedAuthority,
) -> Router {
    router_with_subscription_custody(verifier, backend, policy, authority, None)
}

/// Builds the hosted transport with optional Connector-owned subscription custody. Absence keeps
/// every custody and lease route unreachable.
pub fn router_with_subscription_custody(
    verifier: Arc<dyn IdentityVerifier>,
    backend: Arc<dyn ConnectorBackend>,
    policy: HostedAdmissionPolicy,
    authority: HostedAuthority,
    subscription_custody: Option<Arc<SubscriptionCustody>>,
) -> Router {
    router_with_admin(
        verifier,
        backend,
        policy,
        authority,
        subscription_custody,
        None,
    )
}

async fn operation(
    State(state): State<HostedState>,
    headers: HeaderMap,
    Json(request): Json<RequestEnvelope>,
) -> Response {
    if let Err(error) = request.validate() {
        return operation_failure(&request.request_id, error, StatusCode::BAD_REQUEST);
    }
    let Some(credential) = bearer(&headers) else {
        return error(StatusCode::UNAUTHORIZED, "identity-access-token-required");
    };
    let principal = match state.verifier.verify(credential, CONNECTORS_AUDIENCE).await {
        Ok(principal) => principal,
        Err(IdentityVerificationError::Refused) => {
            return error(StatusCode::UNAUTHORIZED, "identity-access-token-refused");
        }
        Err(IdentityVerificationError::Unavailable) => {
            return error(StatusCode::SERVICE_UNAVAILABLE, "identity-unavailable");
        }
    };
    operation_decided(&state, &principal, request).await
}

/// The decide half of the `/operations` route: the scope map, the receiver policy, the S-047
/// re-describe and Grant/approval admission, and dispatch — every admission decision, behind a
/// principal the caller of this function has already verified. The MCP transport
/// (`hosted/mcp.rs`) funnels its synthesized envelopes through exactly this seam and
/// `datasource_decided`; it never reaches a backend itself (design 14, S-053).
async fn operation_decided(
    state: &HostedState,
    principal: &HostedPrincipal,
    request: RequestEnvelope,
) -> Response {
    let required_scope = match &request.request {
        protocol::operation::OperationRequest::Search(_)
        | protocol::operation::OperationRequest::Describe(_) => "connectors.catalog.read",
        protocol::operation::OperationRequest::Invoke(_)
        | protocol::operation::OperationRequest::SessionStatus(_)
        | protocol::operation::OperationRequest::SessionTerminate(_)
        | protocol::operation::OperationRequest::SessionReconcile(_)
        // Acting on a live call, so it sits with the acting family rather than the reading one:
        // a keypress reaches someone else's system and cannot be undone.
        | protocol::operation::OperationRequest::SessionSignal(_) => "connectors.invoke",
    };
    if principal.tenant_id != request.context.tenant_id || !principal.allows(required_scope) {
        return operation_failure(
            &request.request_id,
            OperationError::new(
                OperationErrorCode::NotGranted,
                "the verified authority does not admit this Connector operation family",
                false,
            ),
            StatusCode::FORBIDDEN,
        );
    }
    if matches!(
        &request.request,
        protocol::operation::OperationRequest::Invoke(_)
            | protocol::operation::OperationRequest::SessionStatus(_)
            | protocol::operation::OperationRequest::SessionTerminate(_)
            | protocol::operation::OperationRequest::SessionReconcile(_)
            // Listed explicitly, because this is a `matches!` and not a `match`: a new acting
            // variant that is forgotten here compiles cleanly and skips the management policy
            // entirely, which is the quietest way to lose an authorization check.
            | protocol::operation::OperationRequest::SessionSignal(_)
    ) && !state.policy.admits_operation(principal, &request.request)
    {
        return operation_failure(
            &request.request_id,
            OperationError::new(
                OperationErrorCode::NotGranted,
                "the Connector-owned management policy does not admit this principal",
                false,
            ),
            StatusCode::FORBIDDEN,
        );
    }
    let owner = match principal.principal_context(&request.request_id) {
        Ok(owner) => owner,
        Err(_) => return error(StatusCode::UNAUTHORIZED, "identity-access-token-refused"),
    };
    let mut admitted = None;
    let mut redeemed = None;
    if let OperationRequest::Invoke(invoke) = &request.request {
        let description =
            match redescribe(state, &owner, &request.request_id, &invoke.operation_ref).await {
                Ok(description) => description,
                Err(response) => return *response,
            };
        match state
            .authority
            .admit_invoke(principal, invoke, &description)
        {
            // Described read-only and demanding nothing: the pre-existing read path, unchanged
            // for callers whose Grants cover no effects.
            Ok(InvokeAdmission::ReadPath) => {}
            // Admission proven: the AdmittedOperation was built through `from_decision`
            // against wall-clock time inside the authority, and the dispatch seam below
            // consumes it by value — the Integrations' own approval-presence checks are
            // deleted in its favour (S-047).
            Ok(InvokeAdmission::Proven {
                operation,
                redemption,
            }) => {
                admitted = Some(operation);
                redeemed = redemption;
            }
            Err(refusal) => {
                return enforcement_refusal_response(&request.request_id, refusal);
            }
        }
    }
    // S-049: a session signal carries the session's own authority. The signal names only an
    // `execution_ref`, so the Connector's own session record supplies what a GrantRequest can
    // name — the operation and Connection that created the session — and the Grant admitting
    // that operation admits signalling into it. No proof, no dispatch.
    let mut admitted_signal = None;
    if let OperationRequest::SessionSignal(signal) = &request.request {
        let session =
            match session_for_signal(state, principal, &owner, &request.request_id, signal).await {
                Ok(session) => session,
                Err(response) => return *response,
            };
        let description =
            match redescribe(state, &owner, &request.request_id, &session.operation_ref).await {
                Ok(description) => description,
                Err(response) => return *response,
            };
        match state
            .authority
            .admit_signal(principal, &session, signal, &description)
        {
            Ok(operation) => admitted_signal = Some((session, operation)),
            Err(refusal) => {
                return enforcement_refusal_response(&request.request_id, refusal);
            }
        }
    }
    // `SessionSignal` is deliberately not in this fence: it is enabled behind the S-049
    // admission above, while the session-lifecycle reads and writes stay disabled until the
    // durable session decisions exist.
    if matches!(
        &request.request,
        OperationRequest::SessionStatus(_)
            | OperationRequest::SessionTerminate(_)
            | OperationRequest::SessionReconcile(_)
    ) {
        return operation_failure(
            &request.request_id,
            OperationError::new(
                OperationErrorCode::Unavailable,
                "hosted Connector sessions are disabled until Connector Grant, approval, and durable session decisions are enforced",
                false,
            ),
            StatusCode::SERVICE_UNAVAILABLE,
        );
    }
    let request_id = request.request_id;
    let outcome = match (request.request, admitted) {
        // The proven seam: an invocation the authority admitted as effect-capable reaches the
        // backend only here, with the AdmittedOperation consumed by value.
        (OperationRequest::Invoke(invoke), Some(operation)) => {
            dispatch_admitted(&*state.backend, &owner, *operation, invoke).await
        }
        // The signal twin of the proven seam (S-049): a keypress reaches the backend only with
        // the proof covering the exact session it enters. The admission block above either
        // produced the proof or returned a refusal already; `dispatch_admitted_signal` refuses
        // a `None` rather than dispatching, so a future edit that breaks that construction
        // fails closed instead of reopening the ungated path.
        (OperationRequest::SessionSignal(signal), _) => {
            dispatch_admitted_signal(&*state.backend, &owner, admitted_signal, signal).await
        }
        (request, _) => state.backend.handle(&owner, request).await,
    };
    if let Some(redemption) = redeemed {
        // The terminal outcome row follows dispatch; the attempted row was durable before it.
        state.authority.conclude(redemption, outcome.is_ok());
    }
    let response = match outcome {
        Ok(result) => ResponseEnvelope::success(&request_id, result),
        Err(error) => ResponseEnvelope::failure(&request_id, error),
    };
    let response = match response.validate() {
        Ok(()) => response,
        Err(error) => ResponseEnvelope::failure(request_id, error),
    };
    Json(response).into_response()
}

async fn event(
    State(state): State<HostedState>,
    headers: HeaderMap,
    Json(request): Json<EventRequestEnvelope>,
) -> Response {
    if let Err(error) = request.validate() {
        return event_failure(&request.request_id, error, StatusCode::BAD_REQUEST);
    }
    let Some(credential) = bearer(&headers) else {
        return error(StatusCode::UNAUTHORIZED, "identity-access-token-required");
    };
    let principal = match state.verifier.verify(credential, CONNECTORS_AUDIENCE).await {
        Ok(principal) => principal,
        Err(IdentityVerificationError::Refused) => {
            return error(StatusCode::UNAUTHORIZED, "identity-access-token-refused");
        }
        Err(IdentityVerificationError::Unavailable) => {
            return error(StatusCode::SERVICE_UNAVAILABLE, "identity-unavailable");
        }
    };
    let operator_read =
        principal.allows("connectors.events.read") && state.policy.admits_operator(&principal);
    let self_read =
        principal.allows("connectors.events.self") && self_service_slack_event(&request.request);
    if principal.tenant_id != request.context.tenant_id || !(operator_read || self_read) {
        return event_failure(
            &request.request_id,
            EventError::new(
                EventErrorCode::NotGranted,
                "the verified authority does not admit Connector event reads",
                false,
            ),
            StatusCode::FORBIDDEN,
        );
    }
    let owner = match principal.principal_context(&request.request_id) {
        Ok(owner) => owner,
        Err(_) => return error(StatusCode::UNAUTHORIZED, "identity-access-token-refused"),
    };
    let response = match state.backend.handle_event(&owner, request.request).await {
        Ok(result) => EventResponseEnvelope::success(&request.request_id, result),
        Err(error) => EventResponseEnvelope::failure(&request.request_id, error),
    };
    Json(response).into_response()
}

fn self_service_slack_event(request: &protocol::event::EventRequest) -> bool {
    match request {
        protocol::event::EventRequest::Search(request) => request.query == "slack",
        protocol::event::EventRequest::Receive(request) => {
            request.channel_ref.starts_with("event-channel:slack:")
        }
        protocol::event::EventRequest::Replay(request) => {
            request.event_ref.starts_with("event:slack:")
        }
    }
}

async fn datasource(
    State(state): State<HostedState>,
    headers: HeaderMap,
    Json(request): Json<DatasourceRequestEnvelope>,
) -> Response {
    if let Err(error) = request.validate() {
        return datasource_failure(&request.request_id, error, StatusCode::BAD_REQUEST);
    }
    let Some(credential) = bearer(&headers) else {
        return error(StatusCode::UNAUTHORIZED, "identity-access-token-required");
    };
    let principal = match state.verifier.verify(credential, CONNECTORS_AUDIENCE).await {
        Ok(principal) => principal,
        Err(IdentityVerificationError::Refused) => {
            return error(StatusCode::UNAUTHORIZED, "identity-access-token-refused");
        }
        Err(IdentityVerificationError::Unavailable) => {
            return error(StatusCode::SERVICE_UNAVAILABLE, "identity-unavailable");
        }
    };
    datasource_decided(&state, &principal, request).await
}

/// The decide half of the `/datasources` route: tenant binding, the catalog-read scope, and
/// dispatch, behind a principal the caller of this function has already verified. Like
/// `operation_decided`, this seam is the only way the MCP transport reaches datasource state
/// (design 14, S-053).
async fn datasource_decided(
    state: &HostedState,
    principal: &HostedPrincipal,
    request: DatasourceRequestEnvelope,
) -> Response {
    if principal.tenant_id != request.context.tenant_id
        || !principal.allows("connectors.catalog.read")
    {
        return datasource_failure(
            &request.request_id,
            DatasourceError::new(
                DatasourceErrorCode::NotGranted,
                "the verified authority does not admit Connector datasource reads",
                false,
            ),
            StatusCode::FORBIDDEN,
        );
    }
    let owner = match principal.principal_context(&request.request_id) {
        Ok(owner) => owner,
        Err(_) => return error(StatusCode::UNAUTHORIZED, "identity-access-token-refused"),
    };
    let response = match state
        .backend
        .handle_datasource(&owner, request.request)
        .await
    {
        Ok(result) => DatasourceResponseEnvelope::success(&request.request_id, result),
        Err(error) => DatasourceResponseEnvelope::failure(&request.request_id, error),
    };
    let response = match response.validate() {
        Ok(()) => response,
        Err(error) => DatasourceResponseEnvelope::failure(&request.request_id, error),
    };
    Json(response).into_response()
}

fn bearer(headers: &HeaderMap) -> Option<&str> {
    headers
        .get(header::AUTHORIZATION)?
        .to_str()
        .ok()?
        .strip_prefix("Bearer ")
        .filter(|value| !value.is_empty() && value.len() <= 512 && value.is_ascii())
}

fn error(status: StatusCode, code: &'static str) -> Response {
    (status, Json(ErrorBody { error: code })).into_response()
}

fn operation_failure(request_id: &str, failure: OperationError, status: StatusCode) -> Response {
    (status, Json(ResponseEnvelope::failure(request_id, failure))).into_response()
}

fn event_failure(request_id: &str, failure: EventError, status: StatusCode) -> Response {
    (
        status,
        Json(EventResponseEnvelope::failure(request_id, failure)),
    )
        .into_response()
}

fn datasource_failure(request_id: &str, failure: DatasourceError, status: StatusCode) -> Response {
    (
        status,
        Json(DatasourceResponseEnvelope::failure(request_id, failure)),
    )
        .into_response()
}

#[cfg(test)]
mod tests {
    use std::convert::Infallible;

    use super::*;
    use axum::body::{to_bytes, Body, Bytes};
    use axum::http::Request;
    use protocol::approval::{IssuedApproval, RequestEnvelope as ApprovalRequestEnvelope};
    use protocol::catalog::RequestEnvelope as CatalogRequestEnvelope;
    use protocol::connection::{
        ConnectionError, ConnectionRequest, ConnectionResult,
        RequestEnvelope as ConnectionRequestEnvelope,
    };
    use protocol::datasource::{
        DatasourceRequest, DatasourceResult, RequestEnvelope as DatasourceRequestEnvelope,
        SearchRequest as DatasourceSearchRequest,
    };
    use protocol::operation::{
        ApprovalPosture, EffectClass, InvocationResult, InvokeRequest, OperationDescription,
        OperationRequest, OperationResult, OwnerContext, SearchRequest,
    };
    use service::PrincipalContext;
    use tower::ServiceExt as _;

    mod admin_routes;
    mod contract_validation;
    mod docs;
    mod enforcement;
    mod mcp;
    mod mcp_monitoring;
    mod signal;

    struct Verifier;

    #[async_trait]
    impl IdentityVerifier for Verifier {
        async fn ready(&self) -> Result<(), IdentityVerificationError> {
            Ok(())
        }

        fn login_origin(&self) -> Option<&str> {
            Some("https://identity.example.test")
        }

        async fn verify(
            &self,
            credential: &str,
            audience: &str,
        ) -> Result<HostedPrincipal, IdentityVerificationError> {
            if credential != "access" || audience != CONNECTORS_AUDIENCE {
                return Err(IdentityVerificationError::Refused);
            }
            Ok(HostedPrincipal {
                issuer: "https://identity.example.test".to_owned(),
                tenant_id: "tenant-dev".to_owned(),
                subject: "person:test".to_owned(),
                email: Some("test@example.com".to_owned()),
                actor_subject: "person:test".to_owned(),
                token_id: "token-test".to_owned(),
                scopes: BTreeSet::from([
                    "connectors.approvals.issue".to_owned(),
                    "connectors.catalog.read".to_owned(),
                    "connectors.connections.manage".to_owned(),
                    "connectors.connections.self".to_owned(),
                    "connectors.credentials.lease".to_owned(),
                    "connectors.invoke".to_owned(),
                    "connectors.events.read".to_owned(),
                    "connectors.integrations.manage".to_owned(),
                ]),
                groups: BTreeSet::from(["operator".to_owned()]),
                authority_snapshot_sha256: "b".repeat(64),
                deployment_id: None,
            })
        }
    }

    struct Backend;

    #[async_trait]
    impl ConnectorBackend for Backend {
        async fn ready(&self) -> Result<(), service::BackendReadinessError> {
            // This transport test backend has no configured dependency.
            Ok(())
        }

        async fn handle(
            &self,
            context: &PrincipalContext,
            _request: OperationRequest,
        ) -> Result<OperationResult, OperationError> {
            assert_eq!(context.subject(), "person:test");
            assert_eq!(context.actor_subject(), "person:test");
            assert_eq!(context.agent_revision(), None);
            Ok(OperationResult::Search { operations: vec![] })
        }

        async fn handle_connection(
            &self,
            context: &PrincipalContext,
            _request: ConnectionRequest,
        ) -> Result<ConnectionResult, ConnectionError> {
            assert_eq!(context.agent_revision(), None);
            Ok(ConnectionResult::Search {
                connections: Vec::new(),
            })
        }

        async fn handle_datasource(
            &self,
            context: &PrincipalContext,
            _request: DatasourceRequest,
        ) -> Result<DatasourceResult, DatasourceError> {
            assert_eq!(
                context.verified_groups(),
                &BTreeSet::from(["operator".to_owned()])
            );
            Ok(DatasourceResult::Search {
                definitions: Vec::new(),
            })
        }
    }

    struct UnavailableBackend;

    const APPROVAL_OPERATION: &str = "todo.create_list";

    struct ApprovalBackend;

    #[async_trait]
    impl ConnectorBackend for ApprovalBackend {
        async fn ready(&self) -> Result<(), service::BackendReadinessError> {
            Ok(())
        }

        fn owns_operation(&self, request: &OperationRequest) -> bool {
            match request {
                OperationRequest::Describe(request) => request.operation_ref == APPROVAL_OPERATION,
                OperationRequest::Invoke(request) => request.operation_ref == APPROVAL_OPERATION,
                _ => false,
            }
        }

        async fn handle(
            &self,
            _context: &PrincipalContext,
            request: OperationRequest,
        ) -> Result<OperationResult, OperationError> {
            match request {
                OperationRequest::Describe(request)
                    if request.operation_ref == APPROVAL_OPERATION =>
                {
                    Ok(OperationResult::Describe(OperationDescription {
                        operation_ref: request.operation_ref,
                        title: "Create todo list".to_owned(),
                        description: "Create one todo list".to_owned(),
                        input_schema: serde_json::json!({"type": "object"}),
                        output_schema: serde_json::json!({"type": "object"}),
                        effect: EffectClass::Mutating,
                        approval: ApprovalPosture::Required,
                        connections: vec![protocol::operation::ConnectionSummary {
                            connection_ref: "connection:todo".to_owned(),
                            label: "Todo".to_owned(),
                            provider: "provider:todo".to_owned(),
                            audiences: Vec::new(),
                            purpose: Some("Generated Todo service".to_owned()),
                        }],
                        description_ref: "description:todo-create-list".to_owned(),
                    }))
                }
                OperationRequest::Invoke(request)
                    if request.operation_ref == APPROVAL_OPERATION =>
                {
                    Ok(OperationResult::Invoke(InvocationResult {
                        operation_ref: request.operation_ref,
                        output: serde_json::json!({"accepted": true}),
                        connector_audit_ref: "audit:todo-create-list".to_owned(),
                        execution_ref: None,
                    }))
                }
                _ => Err(OperationError::new(
                    OperationErrorCode::NotFound,
                    "synthetic approval operation not found",
                    false,
                )),
            }
        }
    }

    #[async_trait]
    impl ConnectorBackend for UnavailableBackend {
        async fn ready(&self) -> Result<(), service::BackendReadinessError> {
            Err(service::BackendReadinessError)
        }

        async fn handle(
            &self,
            _context: &PrincipalContext,
            _request: OperationRequest,
        ) -> Result<OperationResult, OperationError> {
            unreachable!("readiness never dispatches an operation")
        }
    }

    fn envelope(tenant_id: &str) -> RequestEnvelope {
        RequestEnvelope {
            protocol: protocol::operation::CONTRACT.to_owned(),
            request_id: "request-1".to_owned(),
            context: OwnerContext {
                tenant_id: tenant_id.to_owned(),
                agent_id: "agent-test".to_owned(),
                agent_revision: 1,
                authority_snapshot_id: "snapshot-test".to_owned(),
                authority_snapshot_sha256: "a".repeat(64),
            },
            request: OperationRequest::Search(SearchRequest {
                query: String::new(),
                limit: 10,
            }),
        }
    }

    fn invocation_envelope(operation_ref: &str, approval: Option<&str>) -> RequestEnvelope {
        let mut request = envelope("tenant-dev");
        request.request = OperationRequest::Invoke(InvokeRequest {
            operation_ref: operation_ref.to_owned(),
            connection_ref: "connection:test".to_owned(),
            description_ref: "description:test".to_owned(),
            input: serde_json::json!({}),
            approval_evidence_ref: approval.map(str::to_owned),
        });
        request
    }

    fn operation_http_request(envelope: &RequestEnvelope) -> Request<Body> {
        Request::post("/operations")
            .header(header::CONTENT_TYPE, "application/json")
            .header(header::AUTHORIZATION, "Bearer access")
            .body(Body::from(serde_json::to_vec(envelope).unwrap()))
            .unwrap()
    }

    #[tokio::test]
    async fn a_human_issues_one_exact_input_approval_which_is_spent_once() {
        let store = Arc::new(connector_state::MemoryState::new());
        domain::GrantSet {
            revision: 1,
            grants: vec![domain::Grant {
                grant: "grant:todo".to_owned(),
                provider: "provider:todo".to_owned(),
                connection: "connection:todo".to_owned(),
                selector: None,
                allow: BTreeSet::from([APPROVAL_OPERATION.to_owned()]),
                deny: BTreeSet::new(),
                inbound_events: BTreeSet::new(),
            }],
        }
        .write(&*store, "tenant-dev")
        .unwrap();
        let authority = HostedAuthority::bound(
            Arc::<connector_state::MemoryState>::clone(&store),
            "https://identity.example.test",
        );
        let app = router(
            Arc::new(Verifier),
            Arc::new(ApprovalBackend),
            HostedAdmissionPolicy::new(["operator".to_owned()])
                .with_generated_service_operations([APPROVAL_OPERATION.to_owned()]),
            authority,
        );
        let input = serde_json::json!({"title": "Ship it"});
        let issue = ApprovalRequestEnvelope {
            protocol: protocol::approval::CONTRACT.to_owned(),
            request_id: "request-approval-1".to_owned(),
            context: envelope("tenant-dev").context,
            request: protocol::approval::IssueRequest {
                operation_ref: APPROVAL_OPERATION.to_owned(),
                connection_ref: "connection:todo".to_owned(),
                description_ref: "description:todo-create-list".to_owned(),
                input: input.clone(),
                ttl_seconds: 120,
            },
        };
        let response = app
            .clone()
            .oneshot(
                Request::post("/approvals")
                    .header(header::CONTENT_TYPE, "application/json")
                    .header(header::AUTHORIZATION, "Bearer access")
                    .body(Body::from(serde_json::to_vec(&issue).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::CREATED);
        assert_eq!(response.headers()[header::CACHE_CONTROL], "no-store");
        let proof: IssuedApproval = serde_json::from_slice(
            &to_bytes(response.into_body(), protocol::approval::MAX_RESPONSE_BYTES)
                .await
                .unwrap(),
        )
        .unwrap();

        let invoke = || {
            let mut envelope = envelope("tenant-dev");
            envelope.request = OperationRequest::Invoke(InvokeRequest {
                operation_ref: APPROVAL_OPERATION.to_owned(),
                connection_ref: "connection:todo".to_owned(),
                description_ref: "description:todo-create-list".to_owned(),
                input: input.clone(),
                approval_evidence_ref: Some(proof.approval_evidence_ref.clone()),
            });
            operation_http_request(&envelope)
        };
        assert_eq!(
            app.clone().oneshot(invoke()).await.unwrap().status(),
            StatusCode::OK
        );
        assert_eq!(
            app.oneshot(invoke()).await.unwrap().status(),
            StatusCode::FORBIDDEN
        );
    }

    fn connection_envelope(tenant_id: &str) -> ConnectionRequestEnvelope {
        ConnectionRequestEnvelope {
            protocol: protocol::connection::CONTRACT.to_owned(),
            request_id: "request-connection-1".to_owned(),
            context: OwnerContext {
                tenant_id: tenant_id.to_owned(),
                agent_id: "agent-test".to_owned(),
                agent_revision: 1,
                authority_snapshot_id: "snapshot-test".to_owned(),
                authority_snapshot_sha256: "a".repeat(64),
            },
            request: ConnectionRequest::Search(protocol::connection::SearchRequest {
                query: String::new(),
                limit: 10,
            }),
        }
    }

    fn datasource_envelope(tenant_id: &str) -> DatasourceRequestEnvelope {
        DatasourceRequestEnvelope {
            protocol: protocol::datasource::CONTRACT.to_owned(),
            request_id: "request-datasource-1".to_owned(),
            context: OwnerContext {
                tenant_id: tenant_id.to_owned(),
                agent_id: "agent-test".to_owned(),
                agent_revision: 1,
                authority_snapshot_id: "snapshot-test".to_owned(),
                authority_snapshot_sha256: "a".repeat(64),
            },
            request: DatasourceRequest::Search(DatasourceSearchRequest {
                query: String::new(),
                limit: 10,
            }),
        }
    }

    #[tokio::test]
    async fn hosted_completion_streams_fragments_into_a_redacted_bounded_submission() {
        const SENTINEL: &str = "SENTINEL-NOT-A-REAL-SECRET";
        let chunks = [
            Ok::<_, Infallible>(Bytes::from_static(b"xapp-")),
            Ok(Bytes::from_static(SENTINEL.as_bytes())),
            Ok(Bytes::from_static(b"\nxoxb-value\nxoxp-value")),
        ];
        let submission = connect::read_completion_submission(Body::from_stream(
            futures_util::stream::iter(chunks),
        ))
        .await
        .unwrap();
        assert_eq!(
            submission.expose_secret(),
            format!("xapp-{SENTINEL}\nxoxb-value\nxoxp-value").as_bytes()
        );
        assert!(!format!("{submission:?}").contains(SENTINEL));

        let oversized = Body::from_stream(futures_util::stream::iter([
            Ok::<_, Infallible>(Bytes::from(vec![b'x'; MAX_COMPLETION_BYTES])),
            Ok(Bytes::from_static(b"x")),
        ]));
        assert!(connect::read_completion_submission(oversized)
            .await
            .is_err());
    }

    #[test]
    fn tenant_members_receive_only_read_only_module_invocation() {
        let read = OperationRequest::Invoke(InvokeRequest {
            operation_ref: "work/task.list".to_owned(),
            connection_ref: "connection:b10x".to_owned(),
            description_ref: "description:test".to_owned(),
            input: serde_json::json!({}),
            approval_evidence_ref: None,
        });
        let write = OperationRequest::Invoke(InvokeRequest {
            operation_ref: "work/task.create".to_owned(),
            connection_ref: "connection:b10x".to_owned(),
            description_ref: "description:test".to_owned(),
            input: serde_json::json!({}),
            approval_evidence_ref: None,
        });
        let external = OperationRequest::Invoke(InvokeRequest {
            operation_ref: "slack.chat.post-message".to_owned(),
            connection_ref: "connection:slack".to_owned(),
            description_ref: "description:test".to_owned(),
            input: serde_json::json!({}),
            approval_evidence_ref: None,
        });
        assert!(tenant_member_module_read(&read));
        assert!(!tenant_member_module_read(&write));
        assert!(!tenant_member_module_read(&external));
    }

    #[test]
    fn self_event_scope_is_closed_to_slack_specific_requests() {
        assert!(self_service_slack_event(
            &protocol::event::EventRequest::Search(protocol::event::SearchRequest {
                query: "slack".to_owned(),
                limit: 10,
            })
        ));
        assert!(self_service_slack_event(
            &protocol::event::EventRequest::Receive(protocol::event::ReceiveRequest {
                channel_ref: "event-channel:slack:companion".to_owned(),
                after: None,
                limit: 10,
                wait_ms: 0,
            })
        ));
        assert!(!self_service_slack_event(
            &protocol::event::EventRequest::Search(protocol::event::SearchRequest {
                query: String::new(),
                limit: 10,
            })
        ));
        assert!(!self_service_slack_event(
            &protocol::event::EventRequest::Receive(protocol::event::ReceiveRequest {
                channel_ref: "event-channel:b10x:work".to_owned(),
                after: None,
                limit: 10,
                wait_ms: 0,
            })
        ));
    }

    #[tokio::test]
    async fn hosted_route_requires_identity_and_exact_tenant_binding() {
        let app = router(
            Arc::new(Verifier),
            Arc::new(Backend),
            HostedAdmissionPolicy::new(["operator".to_owned()]),
            HostedAuthority::unbound(),
        );
        let request = Request::post("/operations")
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(
                serde_json::to_vec(&envelope("tenant-dev")).unwrap(),
            ))
            .unwrap();
        assert_eq!(
            app.clone().oneshot(request).await.unwrap().status(),
            StatusCode::UNAUTHORIZED
        );

        let request = Request::post("/operations")
            .header(header::CONTENT_TYPE, "application/json")
            .header(header::AUTHORIZATION, "Bearer access")
            .body(Body::from(serde_json::to_vec(&envelope("other")).unwrap()))
            .unwrap();
        assert_eq!(
            app.clone().oneshot(request).await.unwrap().status(),
            StatusCode::FORBIDDEN
        );

        let request = Request::post("/operations")
            .header(header::CONTENT_TYPE, "application/json")
            .header(header::AUTHORIZATION, "Bearer access")
            .body(Body::from(
                serde_json::to_vec(&envelope("tenant-dev")).unwrap(),
            ))
            .unwrap();
        assert_eq!(app.oneshot(request).await.unwrap().status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn hosted_connection_route_uses_the_same_identity_boundary() {
        let app = router(
            Arc::new(Verifier),
            Arc::new(Backend),
            HostedAdmissionPolicy::new(["operator".to_owned()]),
            HostedAuthority::unbound(),
        );
        let request = Request::post("/connections")
            .header(header::CONTENT_TYPE, "application/json")
            .header(header::AUTHORIZATION, "Bearer access")
            .body(Body::from(
                serde_json::to_vec(&connection_envelope("tenant-dev")).unwrap(),
            ))
            .unwrap();
        assert_eq!(app.oneshot(request).await.unwrap().status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn subscription_credential_stays_in_custody_and_only_an_exact_lease_redeems() {
        const PROVIDER_CREDENTIAL: &str = "synthetic-provider-credential-never-log";
        let custody = Arc::new(SubscriptionCustody::new(Arc::new(
            connector_secrets::MemoryStore::new(),
        )));
        let app = router_with_subscription_custody(
            Arc::new(Verifier),
            Arc::new(Backend),
            HostedAdmissionPolicy::new(["operator".to_owned()]),
            HostedAuthority::unbound(),
            Some(custody),
        );

        let connected = app
            .clone()
            .oneshot(
                Request::put("/subscription-credentials/claude-code")
                    .header(header::CONTENT_TYPE, "application/json")
                    .header(header::AUTHORIZATION, "Bearer access")
                    .body(Body::from(
                        serde_json::json!({"credential": PROVIDER_CREDENTIAL}).to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(connected.status(), StatusCode::OK);
        assert_eq!(connected.headers()[header::CACHE_CONTROL], "no-store");
        let connected_body = to_bytes(connected.into_body(), 1024).await.unwrap();
        assert!(!connected_body
            .windows(PROVIDER_CREDENTIAL.len())
            .any(|window| { window == PROVIDER_CREDENTIAL.as_bytes() }));

        let leased = app
            .clone()
            .oneshot(
                Request::post("/subscription-credentials/claude-code/leases")
                    .header(header::CONTENT_TYPE, "application/json")
                    .header(header::AUTHORIZATION, "Bearer access")
                    .body(Body::from(
                        serde_json::json!({
                            "attempt_id": "attempt-one",
                            "ttl_seconds": 60,
                            "maximum_uses": 1
                        })
                        .to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(leased.status(), StatusCode::OK);
        assert_eq!(leased.headers()[header::CACHE_CONTROL], "no-store");
        let lease: serde_json::Value =
            serde_json::from_slice(&to_bytes(leased.into_body(), 4096).await.unwrap()).unwrap();
        let lease_id = lease["lease_id"].as_str().unwrap();
        let lease_token = lease["lease_token"].as_str().unwrap();
        assert_ne!(lease_token, PROVIDER_CREDENTIAL);

        let wrong_attempt = app
            .clone()
            .oneshot(
                Request::post(format!("/subscription-leases/{lease_id}/redeem"))
                    .header(header::CONTENT_TYPE, "application/json")
                    .header(header::AUTHORIZATION, format!("Bearer {lease_token}"))
                    .body(Body::from(r#"{"attempt_id":"attempt-two"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(wrong_attempt.status(), StatusCode::UNAUTHORIZED);

        let redeemed = app
            .clone()
            .oneshot(
                Request::post(format!("/subscription-leases/{lease_id}/redeem"))
                    .header(header::CONTENT_TYPE, "application/json")
                    .header(header::AUTHORIZATION, format!("Bearer {lease_token}"))
                    .body(Body::from(r#"{"attempt_id":"attempt-one"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(redeemed.status(), StatusCode::OK);
        assert_eq!(redeemed.headers()[header::CACHE_CONTROL], "no-store");
        let redeemed: serde_json::Value =
            serde_json::from_slice(&to_bytes(redeemed.into_body(), 20 * 1024).await.unwrap())
                .unwrap();
        assert_eq!(redeemed["credential"], PROVIDER_CREDENTIAL);
        assert_eq!(redeemed["kind"], "oauth");

        let spent = app
            .oneshot(
                Request::post(format!("/subscription-leases/{lease_id}/redeem"))
                    .header(header::CONTENT_TYPE, "application/json")
                    .header(header::AUTHORIZATION, format!("Bearer {lease_token}"))
                    .body(Body::from(r#"{"attempt_id":"attempt-one"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(spent.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn subscription_oauth_start_is_identity_scoped_bounded_and_non_cacheable() {
        let custody = Arc::new(
            SubscriptionCustody::with_claude_oauth(
                Arc::new(connector_secrets::MemoryStore::new()),
                subscription_custody::ClaudeOAuthConfig::official().unwrap(),
            )
            .unwrap(),
        );
        let app = router_with_subscription_custody(
            Arc::new(Verifier),
            Arc::new(Backend),
            HostedAdmissionPolicy::new(["operator".to_owned()]),
            HostedAuthority::unbound(),
            Some(custody),
        );
        let response = app
            .oneshot(
                Request::post("/subscription-credentials/claude-code/oauth/start")
                    .header(header::AUTHORIZATION, "Bearer access")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(response.headers()[header::CACHE_CONTROL], "no-store");
        assert_eq!(response.headers()[header::PRAGMA], "no-cache");
        let body: serde_json::Value =
            serde_json::from_slice(&to_bytes(response.into_body(), 16 * 1024).await.unwrap())
                .unwrap();
        let authorization = body["authorization_url"].as_str().unwrap();
        assert!(authorization.starts_with("https://claude.com/cai/oauth/authorize?"));
        assert!(authorization.contains("code_challenge_method=S256"));
        assert!(!authorization.contains("code_verifier"));
        assert!(body["flow_id"].as_str().unwrap().len() >= 43);
    }

    #[tokio::test]
    async fn hosted_datasource_route_passes_verified_groups_and_exact_tenant() {
        let app = router(
            Arc::new(Verifier),
            Arc::new(Backend),
            HostedAdmissionPolicy::new(["operator".to_owned()]),
            HostedAuthority::unbound(),
        );
        let refused = Request::post("/datasources")
            .header(header::CONTENT_TYPE, "application/json")
            .header(header::AUTHORIZATION, "Bearer access")
            .body(Body::from(
                serde_json::to_vec(&datasource_envelope("other")).unwrap(),
            ))
            .unwrap();
        assert_eq!(
            app.clone().oneshot(refused).await.unwrap().status(),
            StatusCode::FORBIDDEN
        );

        let admitted = Request::post("/datasources")
            .header(header::CONTENT_TYPE, "application/json")
            .header(header::AUTHORIZATION, "Bearer access")
            .body(Body::from(
                serde_json::to_vec(&datasource_envelope("tenant-dev")).unwrap(),
            ))
            .unwrap();
        assert_eq!(
            app.oneshot(admitted).await.unwrap().status(),
            StatusCode::OK
        );
    }

    #[tokio::test]
    async fn hosted_liveness_and_identity_backed_readiness_are_distinct() {
        let app = router(
            Arc::new(Verifier),
            Arc::new(Backend),
            HostedAdmissionPolicy::new(["operator".to_owned()]),
            HostedAuthority::unbound(),
        );
        assert_eq!(
            app.clone()
                .oneshot(Request::get("/livez").body(Body::empty()).unwrap())
                .await
                .unwrap()
                .status(),
            StatusCode::OK
        );
        assert_eq!(
            app.oneshot(Request::get("/readyz").body(Body::empty()).unwrap())
                .await
                .unwrap()
                .status(),
            StatusCode::OK
        );
    }

    #[tokio::test]
    async fn hosted_liveness_stays_local_when_a_backend_dependency_is_unready() {
        let app = router(
            Arc::new(Verifier),
            Arc::new(UnavailableBackend),
            HostedAdmissionPolicy::new(["operator".to_owned()]),
            HostedAuthority::unbound(),
        );
        assert_eq!(
            app.clone()
                .oneshot(Request::get("/livez").body(Body::empty()).unwrap())
                .await
                .unwrap()
                .status(),
            StatusCode::OK
        );
        assert_eq!(
            app.oneshot(Request::get("/readyz").body(Body::empty()).unwrap())
                .await
                .unwrap()
                .status(),
            StatusCode::SERVICE_UNAVAILABLE
        );
    }

    #[tokio::test]
    async fn hosted_completion_failures_are_non_cacheable_and_browser_hardened() {
        let app = router(
            Arc::new(Verifier),
            Arc::new(Backend),
            HostedAdmissionPolicy::new(["operator".to_owned()]),
            HostedAuthority::unbound(),
        );
        for request in [
            Request::get("/connect-sessions/connect-session:unknown")
                .body(Body::empty())
                .unwrap(),
            Request::post("/connect-sessions/connect-session:unknown")
                .header(header::CONTENT_TYPE, "text/plain")
                .body(Body::from("refused"))
                .unwrap(),
        ] {
            let response = app.clone().oneshot(request).await.unwrap();
            assert_eq!(
                response.headers().get(header::CACHE_CONTROL).unwrap(),
                "no-store"
            );
            assert_eq!(
                response.headers().get("referrer-policy").unwrap(),
                "no-referrer"
            );
            assert_eq!(
                response
                    .headers()
                    .get("content-security-policy")
                    .unwrap(),
                "default-src 'none'; script-src 'unsafe-inline'; style-src 'unsafe-inline'; connect-src 'self'; form-action 'self'; base-uri 'none'"
            );
        }
    }

    #[test]
    fn pod_log_transport_gate_admits_only_kubernetes_read_groups_or_operator() {
        let policy = HostedAdmissionPolicy::new(["operator".to_owned()])
            .with_kubernetes_groups(["dev".to_owned(), "sre".to_owned()], ["sre".to_owned()]);
        let request = OperationRequest::Invoke(InvokeRequest {
            operation_ref: "kubernetes.pod.logs".to_owned(),
            connection_ref: "connection:kubernetes:in-cluster".to_owned(),
            description_ref: "description:test".to_owned(),
            input: serde_json::json!({}),
            approval_evidence_ref: None,
        });
        let principal = |group: &str| HostedPrincipal {
            issuer: "https://identity.example.test".to_owned(),
            tenant_id: "tenant-dev".to_owned(),
            subject: "person:test".to_owned(),
            actor_subject: "person:test".to_owned(),
            email: None,
            token_id: "token-test".to_owned(),
            scopes: BTreeSet::new(),
            groups: BTreeSet::from([group.to_owned()]),
            authority_snapshot_sha256: "b".repeat(64),
            deployment_id: None,
        };
        assert!(policy.admits_operation(&principal("dev"), &request));
        assert!(policy.admits_operation(&principal("sre"), &request));
        assert!(policy.admits_operation(&principal("operator"), &request));
        assert!(!policy.admits_operation(&principal("viewer"), &request));
    }

    #[test]
    fn monitoring_transport_gate_admits_only_configured_groups_or_operator() {
        let policy = HostedAdmissionPolicy::new(["operator".to_owned()])
            .with_monitoring_groups(["dev".to_owned(), "sre".to_owned()]);
        let request = OperationRequest::Invoke(InvokeRequest {
            operation_ref: "prometheus-query-range".to_owned(),
            connection_ref: "connection:prometheus:dev".to_owned(),
            description_ref: "description:test".to_owned(),
            input: serde_json::json!({}),
            approval_evidence_ref: None,
        });
        let principal = |group: &str| HostedPrincipal {
            issuer: "https://identity.example.test".to_owned(),
            tenant_id: "tenant-dev".to_owned(),
            subject: "person:test".to_owned(),
            actor_subject: "person:test".to_owned(),
            email: None,
            token_id: "token-test".to_owned(),
            scopes: BTreeSet::new(),
            groups: BTreeSet::from([group.to_owned()]),
            authority_snapshot_sha256: "b".repeat(64),
            deployment_id: None,
        };
        assert!(policy.admits_operation(&principal("dev"), &request));
        assert!(policy.admits_operation(&principal("sre"), &request));
        assert!(policy.admits_operation(&principal("operator"), &request));
        assert!(!policy.admits_operation(&principal("viewer"), &request));
    }
}
