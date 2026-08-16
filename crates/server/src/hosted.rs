//! Identity-authenticated hosted transport for the Connector operation contract.

use std::collections::BTreeSet;
use std::sync::Arc;

use async_trait::async_trait;
use axum::extract::{DefaultBodyLimit, State};
use axum::http::{header, HeaderMap, StatusCode};
use axum::response::{IntoResponse, Json, Response};
use axum::routing::{get, post};
use axum::Router;
use protocol::catalog::{
    RequestEnvelope as CatalogRequestEnvelope, ResponseEnvelope as CatalogResponseEnvelope,
};
use protocol::connection::{
    ConnectionError, ConnectionErrorCode, RequestEnvelope as ConnectionRequestEnvelope,
    ResponseEnvelope as ConnectionResponseEnvelope, MAX_FRAME_BYTES as CONNECTION_MAX_FRAME_BYTES,
};
use protocol::operation::{
    OperationError, OperationErrorCode, RequestEnvelope, ResponseEnvelope,
    MAX_FRAME_BYTES as OPERATION_MAX_FRAME_BYTES,
};
use serde::Serialize;
use service::{ConnectorBackend, PrincipalContext};

pub const CONNECTORS_AUDIENCE: &str = "urn:b10x:connectors";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HostedPrincipal {
    pub issuer: String,
    pub tenant_id: String,
    pub subject: String,
    pub actor_subject: String,
    pub token_id: String,
    pub scopes: BTreeSet<String>,
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
}

#[derive(Debug, Serialize)]
#[serde(deny_unknown_fields)]
struct ErrorBody {
    error: &'static str,
}

pub fn router(verifier: Arc<dyn IdentityVerifier>, backend: Arc<dyn ConnectorBackend>) -> Router {
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
        .with_state(HostedState { verifier, backend })
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
    ) {
        return connection_failure(
            &request.request_id,
            ConnectionError::new(
                ConnectionErrorCode::NotGranted,
                "hosted connection mutation is disabled until Connector-owned management policy is configured",
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
    match state.verifier.ready().await {
        Ok(()) => (StatusCode::OK, "ok\n").into_response(),
        Err(_) => (StatusCode::SERVICE_UNAVAILABLE, "identity-unavailable\n").into_response(),
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
    ) {
        return operation_failure(
            &request.request_id,
            OperationError::new(
                OperationErrorCode::NotGranted,
                "hosted invocation is disabled until Connector-owned Grant admission is configured",
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

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::Request;
    use protocol::connection::{ConnectionRequest, ConnectionResult};
    use protocol::operation::{OperationRequest, OperationResult, OwnerContext, SearchRequest};
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
                ]),
                authority_snapshot_sha256: "b".repeat(64),
                deployment_id: None,
            })
        }
    }

    struct Backend;

    #[async_trait]
    impl ConnectorBackend for Backend {
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
    async fn hosted_route_requires_identity_and_exact_tenant_binding() {
        let app = router(Arc::new(Verifier), Arc::new(Backend));
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
        let app = router(Arc::new(Verifier), Arc::new(Backend));
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
        let app = router(Arc::new(Verifier), Arc::new(Backend));
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
}
