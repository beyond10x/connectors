//! Identity-authenticated hosted transport for the Connector operation contract.

use std::collections::BTreeSet;
use std::sync::Arc;

use async_trait::async_trait;
use axum::body::Body;
use axum::extract::{DefaultBodyLimit, Path as AxumPath, State};
use axum::http::{header, HeaderMap, StatusCode};
use axum::response::{Html, IntoResponse, Json, Response};
use axum::routing::{get, post};
use axum::Router;
use futures_util::StreamExt as _;
use protocol::catalog::{
    RequestEnvelope as CatalogRequestEnvelope, ResponseEnvelope as CatalogResponseEnvelope,
};
use protocol::connection::{
    ConnectionError, ConnectionErrorCode, RequestEnvelope as ConnectionRequestEnvelope,
    ResponseEnvelope as ConnectionResponseEnvelope, MAX_FRAME_BYTES as CONNECTION_MAX_FRAME_BYTES,
};
use protocol::event::{
    EventError, EventErrorCode, RequestEnvelope as EventRequestEnvelope,
    ResponseEnvelope as EventResponseEnvelope, MAX_FRAME_BYTES as EVENT_MAX_FRAME_BYTES,
};
use protocol::operation::{
    OperationError, OperationErrorCode, RequestEnvelope, ResponseEnvelope,
    MAX_FRAME_BYTES as OPERATION_MAX_FRAME_BYTES,
};
use serde::Serialize;
use service::{
    ConnectorBackend, HostedCompletionError, HostedCompletionSubmission, PrincipalContext,
};

pub const CONNECTORS_AUDIENCE: &str = "urn:b10x:connectors";
const MAX_COMPLETION_BYTES: usize = 8 * 1024;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HostedPrincipal {
    pub issuer: String,
    pub tenant_id: String,
    pub subject: String,
    pub actor_subject: String,
    pub token_id: String,
    pub scopes: BTreeSet<String>,
    pub groups: BTreeSet<String>,
    pub authority_snapshot_sha256: String,
    pub deployment_id: Option<String>,
}

impl HostedPrincipal {
    fn allows(&self, scope: &str) -> bool {
        self.scopes.contains(scope)
    }

    fn principal_context(&self) -> Result<PrincipalContext, service::PrincipalContextError> {
        PrincipalContext::hosted(
            self.tenant_id.clone(),
            self.subject.clone(),
            self.actor_subject.clone(),
            self.token_id.clone(),
            self.authority_snapshot_sha256.clone(),
        )
    }
}

/// Receiver-owned admission policy for effect-bearing hosted Connector request families.
#[derive(Debug, Clone, Default)]
pub struct HostedAdmissionPolicy {
    operator_groups: BTreeSet<String>,
}

impl HostedAdmissionPolicy {
    #[must_use]
    pub fn new(operator_groups: impl IntoIterator<Item = String>) -> Self {
        Self {
            operator_groups: operator_groups.into_iter().collect(),
        }
    }

    fn admits_operator(&self, principal: &HostedPrincipal) -> bool {
        !self.operator_groups.is_empty() && !self.operator_groups.is_disjoint(&principal.groups)
    }

    fn admits_operation(
        &self,
        principal: &HostedPrincipal,
        request: &protocol::operation::OperationRequest,
    ) -> bool {
        self.admits_operator(principal) || tenant_member_module_read(request)
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
) -> Router {
    Router::new()
        .route("/livez", get(liveness))
        .route("/readyz", get(readiness))
        .route("/healthz", get(readiness))
        .route(
            "/operations",
            post(operation).layer(DefaultBodyLimit::max(OPERATION_MAX_FRAME_BYTES)),
        )
        .route(
            "/connections",
            post(connection).layer(DefaultBodyLimit::max(CONNECTION_MAX_FRAME_BYTES)),
        )
        .route(
            "/catalog",
            post(catalog).layer(DefaultBodyLimit::max(protocol::catalog::MAX_FRAME_BYTES)),
        )
        .route(
            "/events",
            post(event).layer(DefaultBodyLimit::max(EVENT_MAX_FRAME_BYTES)),
        )
        .route(
            "/connect-sessions/{session_ref}",
            get(completion_page)
                .post(complete_session)
                .layer(DefaultBodyLimit::max(MAX_COMPLETION_BYTES)),
        )
        .with_state(HostedState {
            verifier,
            backend,
            policy,
        })
}

async fn completion_page(
    State(state): State<HostedState>,
    AxumPath(session_ref): AxumPath<String>,
) -> Response {
    let response = match state.backend.hosted_completion_page(&session_ref) {
        Ok(page) => Html(page.html).into_response(),
        Err(HostedCompletionError::NotFound | HostedCompletionError::Refused) => {
            error(StatusCode::NOT_FOUND, "connect-session-not-found")
        }
        Err(HostedCompletionError::Invalid) => {
            error(StatusCode::BAD_REQUEST, "connect-session-invalid")
        }
        Err(HostedCompletionError::Unavailable) => error(
            StatusCode::SERVICE_UNAVAILABLE,
            "connect-session-unavailable",
        ),
    };
    secure_completion_response(response)
}

async fn complete_session(
    State(state): State<HostedState>,
    AxumPath(session_ref): AxumPath<String>,
    headers: HeaderMap,
    body: Body,
) -> Response {
    let capability = headers
        .get("x-connect-session")
        .and_then(|value| value.to_str().ok())
        .filter(|value| {
            !value.is_empty()
                && value.len() <= 256
                && value
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
        });
    if headers
        .get(header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        != Some("application/octet-stream")
        || capability.is_none()
    {
        return secure_completion_response(error(
            StatusCode::BAD_REQUEST,
            "connect-session-invalid",
        ));
    }
    let submission = match read_completion_submission(body).await {
        Ok(submission) if !submission.is_empty() => submission,
        Ok(_) | Err(()) => {
            return secure_completion_response(error(
                StatusCode::BAD_REQUEST,
                "connect-session-invalid",
            ));
        }
    };
    let result = state
        .backend
        .complete_hosted_session(
            &session_ref,
            capability.expect("checked capability"),
            submission,
        )
        .await;
    let response = match result {
        Ok(()) => Json(serde_json::json!({"accepted": true})).into_response(),
        Err(HostedCompletionError::NotFound | HostedCompletionError::Refused) => {
            error(StatusCode::FORBIDDEN, "connect-session-refused")
        }
        Err(HostedCompletionError::Invalid) => {
            error(StatusCode::BAD_REQUEST, "connect-session-invalid")
        }
        Err(HostedCompletionError::Unavailable) => error(
            StatusCode::SERVICE_UNAVAILABLE,
            "connect-session-unavailable",
        ),
    };
    secure_completion_response(response)
}

async fn read_completion_submission(body: Body) -> Result<HostedCompletionSubmission, ()> {
    // Reserving the full admitted bound prevents Vec growth from leaving an earlier copy of
    // credential bytes in freed heap storage. HTTP frame buffers remain transport-owned and are
    // copied exactly once, directly into the zeroizing-owned allocation.
    let mut submission = HostedCompletionSubmission::with_capacity(MAX_COMPLETION_BYTES);
    let mut stream = body.into_data_stream();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|_| ())?;
        if submission.len().saturating_add(chunk.len()) > MAX_COMPLETION_BYTES {
            return Err(());
        }
        if !submission.extend_from_slice(&chunk) {
            return Err(());
        }
    }
    Ok(submission)
}

fn secure_completion_response(mut response: Response) -> Response {
    let headers = response.headers_mut();
    headers.insert(
        header::CACHE_CONTROL,
        "no-store".parse().expect("static header"),
    );
    headers.insert("pragma", "no-cache".parse().expect("static header"));
    headers.insert(
        "referrer-policy",
        "no-referrer".parse().expect("static header"),
    );
    headers.insert(
        "x-content-type-options",
        "nosniff".parse().expect("static header"),
    );
    headers.insert(
        "content-security-policy",
        "default-src 'none'; script-src 'unsafe-inline'; style-src 'unsafe-inline'; connect-src 'self'; form-action 'self'; base-uri 'none'"
            .parse()
            .expect("static header"),
    );
    response
}

async fn connection(
    State(state): State<HostedState>,
    headers: HeaderMap,
    Json(request): Json<ConnectionRequestEnvelope>,
) -> Response {
    if let Err(error) = request.validate() {
        return connection_failure(&request.request_id, error, StatusCode::BAD_REQUEST);
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
    let required_scope = match &request.request {
        protocol::connection::ConnectionRequest::CandidateSearch(_)
        | protocol::connection::ConnectionRequest::Search(_)
        | protocol::connection::ConnectionRequest::Describe(_)
        | protocol::connection::ConnectionRequest::ObservationSearch(_)
        | protocol::connection::ConnectionRequest::ConnectSessionStatus(_) => {
            "connectors.catalog.read"
        }
        protocol::connection::ConnectionRequest::CandidateActivate(_)
        | protocol::connection::ConnectionRequest::Materialize(_)
        | protocol::connection::ConnectionRequest::ConnectSessionCreate(_) => {
            "connectors.connections.manage"
        }
    };
    if principal.tenant_id != request.context.tenant_id || !principal.allows(required_scope) {
        return connection_failure(
            &request.request_id,
            ConnectionError::new(
                ConnectionErrorCode::NotGranted,
                "the verified authority does not admit this Connector connection request family",
                false,
            ),
            StatusCode::FORBIDDEN,
        );
    }
    if matches!(
        &request.request,
        protocol::connection::ConnectionRequest::CandidateActivate(_)
            | protocol::connection::ConnectionRequest::Materialize(_)
            | protocol::connection::ConnectionRequest::ConnectSessionCreate(_)
    ) && !state.policy.admits_operator(&principal)
    {
        return connection_failure(
            &request.request_id,
            ConnectionError::new(
                ConnectionErrorCode::NotGranted,
                "the Connector-owned management policy does not admit this principal",
                false,
            ),
            StatusCode::FORBIDDEN,
        );
    }
    let owner = match principal.principal_context() {
        Ok(owner) => owner,
        Err(_) => return error(StatusCode::UNAUTHORIZED, "identity-access-token-refused"),
    };
    let response = match state
        .backend
        .handle_connection(&owner, request.request)
        .await
    {
        Ok(result) => ConnectionResponseEnvelope::success(&request.request_id, result),
        Err(error) => ConnectionResponseEnvelope::failure(&request.request_id, error),
    };
    Json(response).into_response()
}

async fn liveness() -> &'static str {
    "ok\n"
}

async fn readiness(State(state): State<HostedState>) -> Response {
    if state.verifier.ready().await.is_err() {
        return (StatusCode::SERVICE_UNAVAILABLE, "identity-unavailable\n").into_response();
    }
    match state.backend.ready().await {
        Ok(()) => (StatusCode::OK, "ok\n").into_response(),
        Err(_) => (StatusCode::SERVICE_UNAVAILABLE, "backend-unavailable\n").into_response(),
    }
}

async fn catalog(
    State(state): State<HostedState>,
    headers: HeaderMap,
    Json(request): Json<CatalogRequestEnvelope>,
) -> Response {
    if let Err(error) = request.validate() {
        return (
            StatusCode::BAD_REQUEST,
            Json(CatalogResponseEnvelope::failure(&request.request_id, error)),
        )
            .into_response();
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
    if principal.tenant_id != request.context.tenant_id
        || !principal.allows("connectors.catalog.read")
    {
        return (
            StatusCode::FORBIDDEN,
            Json(CatalogResponseEnvelope::failure(
                &request.request_id,
                protocol::catalog::CatalogError {
                    code: "not_granted".to_owned(),
                    message: "the verified authority does not admit catalog reads".to_owned(),
                },
            )),
        )
            .into_response();
    }
    let request_id = request.request_id;
    let response = match crate::catalog_projection::handle(request.request) {
        Ok(result) => CatalogResponseEnvelope::success(&request_id, result),
        Err(error) => CatalogResponseEnvelope::failure(&request_id, error),
    };
    Json(response).into_response()
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
    let required_scope = match &request.request {
        protocol::operation::OperationRequest::Search(_)
        | protocol::operation::OperationRequest::Describe(_) => "connectors.catalog.read",
        protocol::operation::OperationRequest::Invoke(_)
        | protocol::operation::OperationRequest::SessionStatus(_)
        | protocol::operation::OperationRequest::SessionTerminate(_)
        | protocol::operation::OperationRequest::SessionReconcile(_) => "connectors.invoke",
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
    ) && !state.policy.admits_operation(&principal, &request.request)
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
    let owner = match principal.principal_context() {
        Ok(owner) => owner,
        Err(_) => return error(StatusCode::UNAUTHORIZED, "identity-access-token-refused"),
    };
    let response = match state.backend.handle(&owner, request.request).await {
        Ok(result) => ResponseEnvelope::success(&request.request_id, result),
        Err(error) => ResponseEnvelope::failure(&request.request_id, error),
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
    if principal.tenant_id != request.context.tenant_id
        || !principal.allows("connectors.events.read")
        || !state.policy.admits_operator(&principal)
    {
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
    let owner = match principal.principal_context() {
        Ok(owner) => owner,
        Err(_) => return error(StatusCode::UNAUTHORIZED, "identity-access-token-refused"),
    };
    let response = match state.backend.handle_event(&owner, request.request).await {
        Ok(result) => EventResponseEnvelope::success(&request.request_id, result),
        Err(error) => EventResponseEnvelope::failure(&request.request_id, error),
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

fn connection_failure(request_id: &str, failure: ConnectionError, status: StatusCode) -> Response {
    (
        status,
        Json(ConnectionResponseEnvelope::failure(request_id, failure)),
    )
        .into_response()
}

fn event_failure(request_id: &str, failure: EventError, status: StatusCode) -> Response {
    (
        status,
        Json(EventResponseEnvelope::failure(request_id, failure)),
    )
        .into_response()
}

#[cfg(test)]
mod tests {
    use std::convert::Infallible;

    use super::*;
    use axum::body::{Body, Bytes};
    use axum::http::Request;
    use protocol::connection::{ConnectionRequest, ConnectionResult};
    use protocol::operation::{
        InvokeRequest, OperationRequest, OperationResult, OwnerContext, SearchRequest,
    };
    use tower::ServiceExt as _;

    struct Verifier;

    #[async_trait]
    impl IdentityVerifier for Verifier {
        async fn ready(&self) -> Result<(), IdentityVerificationError> {
            Ok(())
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
                actor_subject: "person:test".to_owned(),
                token_id: "token-test".to_owned(),
                scopes: BTreeSet::from([
                    "connectors.catalog.read".to_owned(),
                    "connectors.connections.manage".to_owned(),
                    "connectors.invoke".to_owned(),
                    "connectors.events.read".to_owned(),
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
    }

    struct UnavailableBackend;

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

    #[tokio::test]
    async fn hosted_completion_streams_fragments_into_a_redacted_bounded_submission() {
        const SENTINEL: &str = "SENTINEL-NOT-A-REAL-SECRET";
        let chunks = [
            Ok::<_, Infallible>(Bytes::from_static(b"xapp-")),
            Ok(Bytes::from_static(SENTINEL.as_bytes())),
            Ok(Bytes::from_static(b"\nxoxb-value\nxoxp-value")),
        ];
        let submission =
            read_completion_submission(Body::from_stream(futures_util::stream::iter(chunks)))
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
        assert!(read_completion_submission(oversized).await.is_err());
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

    #[tokio::test]
    async fn hosted_route_requires_identity_and_exact_tenant_binding() {
        let app = router(
            Arc::new(Verifier),
            Arc::new(Backend),
            HostedAdmissionPolicy::new(["operator".to_owned()]),
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
    async fn hosted_liveness_and_identity_backed_readiness_are_distinct() {
        let app = router(
            Arc::new(Verifier),
            Arc::new(Backend),
            HostedAdmissionPolicy::new(["operator".to_owned()]),
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
}
