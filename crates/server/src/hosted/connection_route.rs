//! Identity and management-policy boundary for Connection requests.

use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse as _, Json, Response};
use protocol::connection::{
    ConnectionError, ConnectionErrorCode, ConnectionRequest, RequestEnvelope, ResponseEnvelope,
};
use service::ConnectSessionAccess;

use super::{bearer, error, HostedState, IdentityVerificationError, CONNECTORS_AUDIENCE};

pub(super) async fn handle(
    State(state): State<HostedState>,
    headers: HeaderMap,
    Json(request): Json<RequestEnvelope>,
) -> Response {
    if let Err(error) = request.validate() {
        return failure(&request.request_id, error, StatusCode::BAD_REQUEST);
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
    let self_service = matches!(
        &request.request,
        ConnectionRequest::ConnectSessionCreate(request)
            if state.backend.connect_session_access(request) == ConnectSessionAccess::SelfService
    );
    let required_scope = match &request.request {
        ConnectionRequest::CandidateSearch(_)
        | ConnectionRequest::Search(_)
        | ConnectionRequest::Describe(_)
        | ConnectionRequest::ObservationSearch(_)
        | ConnectionRequest::ConnectSessionStatus(_) => "connectors.catalog.read",
        ConnectionRequest::ConnectSessionCreate(_) if self_service => "connectors.connections.self",
        ConnectionRequest::CandidateActivate(_)
        | ConnectionRequest::Materialize(_)
        | ConnectionRequest::ConnectSessionCreate(_) => "connectors.connections.manage",
    };
    if principal.tenant_id != request.context.tenant_id || !principal.allows(required_scope) {
        return failure(
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
        ConnectionRequest::CandidateActivate(_)
            | ConnectionRequest::Materialize(_)
            | ConnectionRequest::ConnectSessionCreate(_)
    ) && !self_service
        && !state.policy.admits_operator(&principal)
    {
        return failure(
            &request.request_id,
            ConnectionError::new(
                ConnectionErrorCode::NotGranted,
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
    let response = match state
        .backend
        .handle_connection(&owner, request.request)
        .await
    {
        Ok(result) => ResponseEnvelope::success(&request.request_id, result),
        Err(error) => ResponseEnvelope::failure(&request.request_id, error),
    };
    Json(response).into_response()
}

fn failure(request_id: &str, error: ConnectionError, status: StatusCode) -> Response {
    (status, Json(ResponseEnvelope::failure(request_id, error))).into_response()
}
