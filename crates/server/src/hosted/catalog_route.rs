//! Identity boundary for the generated catalog projection.

use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse as _, Json, Response};
use protocol::catalog::{CatalogError, RequestEnvelope, ResponseEnvelope};

use super::{bearer, error, HostedState, IdentityVerificationError, CONNECTORS_AUDIENCE};

pub(super) async fn handle(
    State(state): State<HostedState>,
    headers: HeaderMap,
    Json(request): Json<RequestEnvelope>,
) -> Response {
    if let Err(error) = request.validate() {
        return (
            StatusCode::BAD_REQUEST,
            Json(ResponseEnvelope::failure(&request.request_id, error)),
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
            Json(ResponseEnvelope::failure(
                &request.request_id,
                CatalogError {
                    code: "not_granted".to_owned(),
                    message: "the verified authority does not admit catalog reads".to_owned(),
                },
            )),
        )
            .into_response();
    }
    let request_id = request.request_id;
    let response = match crate::catalog_projection::handle(request.request, &*state.backend) {
        Ok(result) => ResponseEnvelope::success(&request_id, result),
        Err(error) => ResponseEnvelope::failure(&request_id, error),
    };
    let response = match response.validate() {
        Ok(()) => response,
        Err(error) => ResponseEnvelope::failure(request_id, error),
    };
    Json(response).into_response()
}
