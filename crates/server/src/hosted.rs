//! Identity-authenticated hosted transport for the Connector operation contract.

use std::sync::Arc;

use async_trait::async_trait;
use axum::extract::{DefaultBodyLimit, State};
use axum::http::{header, HeaderMap, StatusCode};
use axum::response::{IntoResponse, Json, Response};
use axum::routing::{get, post};
use axum::Router;
use protocol::operation::{
    OperationError, OperationErrorCode, RequestEnvelope, ResponseEnvelope, MAX_FRAME_BYTES,
};
use serde::Serialize;

use crate::local::OperationBackend;

pub const CONNECTORS_AUDIENCE: &str = "b10x.connectors";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HostedPrincipal {
    pub tenant_id: String,
    pub subject: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum IdentityVerificationError {
    #[error("Identity authority is unavailable")]
    Unavailable,
    #[error("Identity authority refused the presented session")]
    Refused,
}

#[async_trait]
pub trait IdentityVerifier: Send + Sync + 'static {
    async fn verify(
        &self,
        credential: &str,
        audience: &str,
    ) -> Result<HostedPrincipal, IdentityVerificationError>;
}

#[derive(Clone)]
struct HostedState {
    verifier: Arc<dyn IdentityVerifier>,
    backend: Arc<dyn OperationBackend>,
}

#[derive(Debug, Serialize)]
#[serde(deny_unknown_fields)]
struct ErrorBody {
    error: &'static str,
}

pub fn router(verifier: Arc<dyn IdentityVerifier>, backend: Arc<dyn OperationBackend>) -> Router {
    Router::new()
        .route("/healthz", get(health))
        .route("/v0alpha1/operations", post(operation))
        .layer(DefaultBodyLimit::max(MAX_FRAME_BYTES))
        .with_state(HostedState { verifier, backend })
}

async fn health() -> &'static str {
    "ok\n"
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
        return error(StatusCode::UNAUTHORIZED, "identity-session-required");
    };
    let principal = match state.verifier.verify(credential, CONNECTORS_AUDIENCE).await {
        Ok(principal) => principal,
        Err(IdentityVerificationError::Refused) => {
            return error(StatusCode::UNAUTHORIZED, "identity-session-refused");
        }
        Err(IdentityVerificationError::Unavailable) => {
            return error(StatusCode::SERVICE_UNAVAILABLE, "identity-unavailable");
        }
    };
    if principal.tenant_id != request.context.tenant_id {
        return operation_failure(
            &request.request_id,
            OperationError::new(
                OperationErrorCode::NotGranted,
                "authenticated tenant does not match the Connector owner context",
                false,
            ),
            StatusCode::FORBIDDEN,
        );
    }
    let response = match state
        .backend
        .handle(&request.context, request.request)
        .await
    {
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

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::Request;
    use protocol::operation::{OperationRequest, OperationResult, OwnerContext, SearchRequest};
    use tower::ServiceExt as _;

    struct Verifier;

    #[async_trait]
    impl IdentityVerifier for Verifier {
        async fn verify(
            &self,
            credential: &str,
            audience: &str,
        ) -> Result<HostedPrincipal, IdentityVerificationError> {
            if credential != "session" || audience != CONNECTORS_AUDIENCE {
                return Err(IdentityVerificationError::Refused);
            }
            Ok(HostedPrincipal {
                tenant_id: "tenant-dev".to_owned(),
                subject: "person:test".to_owned(),
            })
        }
    }

    struct Backend;

    #[async_trait]
    impl OperationBackend for Backend {
        async fn handle(
            &self,
            _context: &OwnerContext,
            _request: OperationRequest,
        ) -> Result<OperationResult, OperationError> {
            Ok(OperationResult::Search { operations: vec![] })
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

    #[tokio::test]
    async fn hosted_route_requires_identity_and_exact_tenant_binding() {
        let app = router(Arc::new(Verifier), Arc::new(Backend));
        let request = Request::post("/v0alpha1/operations")
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(
                serde_json::to_vec(&envelope("tenant-dev")).unwrap(),
            ))
            .unwrap();
        assert_eq!(
            app.clone().oneshot(request).await.unwrap().status(),
            StatusCode::UNAUTHORIZED
        );

        let request = Request::post("/v0alpha1/operations")
            .header(header::CONTENT_TYPE, "application/json")
            .header(header::AUTHORIZATION, "Bearer session")
            .body(Body::from(serde_json::to_vec(&envelope("other")).unwrap()))
            .unwrap();
        assert_eq!(
            app.clone().oneshot(request).await.unwrap().status(),
            StatusCode::FORBIDDEN
        );

        let request = Request::post("/v0alpha1/operations")
            .header(header::CONTENT_TYPE, "application/json")
            .header(header::AUTHORIZATION, "Bearer session")
            .body(Body::from(
                serde_json::to_vec(&envelope("tenant-dev")).unwrap(),
            ))
            .unwrap();
        assert_eq!(app.oneshot(request).await.unwrap().status(), StatusCode::OK);
    }
}
