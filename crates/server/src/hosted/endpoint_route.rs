//! Endpoint inventory management and endpoint-aware operation normalization.

use super::{
    bearer, error, HostedPrincipal, HostedState, IdentityVerificationError, CONNECTORS_AUDIENCE,
};
use axum::body::{Body, Bytes};
use axum::extract::State;
use axum::http::{header, HeaderMap, StatusCode};
use axum::response::{IntoResponse, Json, Response};
use protocol::{
    endpoint,
    operation::{self, v4},
};
use service::{constrain_endpoint_description, normalize_endpoint_operation};

async fn principal(state: &HostedState, headers: &HeaderMap) -> Result<HostedPrincipal, Response> {
    let Some(credential) = bearer(headers) else {
        return Err(error(
            StatusCode::UNAUTHORIZED,
            "identity-access-token-required",
        ));
    };
    state
        .verifier
        .verify(credential, CONNECTORS_AUDIENCE)
        .await
        .map_err(|refusal| match refusal {
            IdentityVerificationError::Refused => {
                error(StatusCode::UNAUTHORIZED, "identity-access-token-refused")
            }
            IdentityVerificationError::Unavailable => {
                error(StatusCode::SERVICE_UNAVAILABLE, "identity-unavailable")
            }
        })
}

pub(super) async fn handle(
    State(state): State<HostedState>,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    if !headers
        .get(header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .is_some_and(|value| {
            let mime = value.split(';').next().unwrap_or("").trim();
            mime == "application/json"
                || (mime.starts_with("application/") && mime.ends_with("+json"))
        })
    {
        return StatusCode::UNSUPPORTED_MEDIA_TYPE.into_response();
    }
    let request: endpoint::RequestEnvelope = match serde_json::from_slice(&body) {
        Ok(request) => request,
        Err(_) => return StatusCode::BAD_REQUEST.into_response(),
    };
    if request.validate().is_err() {
        return StatusCode::BAD_REQUEST.into_response();
    }
    let principal = match principal(&state, &headers).await {
        Ok(value) => value,
        Err(response) => return response,
    };
    let management = matches!(
        request.request,
        endpoint::EndpointRequest::Bind(_) | endpoint::EndpointRequest::Refresh(_)
    );
    let scope = if management {
        "connectors.connections.manage"
    } else {
        "connectors.catalog.read"
    };
    if principal.tenant_id != request.context.tenant_id
        || !principal.allows(scope)
        || (management && !state.policy.admits_operator(&principal))
    {
        return (
            StatusCode::FORBIDDEN,
            Json(endpoint::ResponseEnvelope::failure(
                request.request_id,
                endpoint::EndpointError::new(
                    endpoint::EndpointErrorCode::NotGranted,
                    "endpoint management or inventory is not admitted",
                    false,
                ),
            )),
        )
            .into_response();
    }
    let owner = match principal.principal_context(&request.request_id) {
        Ok(owner) => owner,
        Err(_) => return error(StatusCode::UNAUTHORIZED, "identity-access-token-refused"),
    };
    let response = match state.backend.handle_endpoint(&owner, request.request).await {
        Ok(result) => endpoint::ResponseEnvelope::success(&request.request_id, result),
        Err(error) => endpoint::ResponseEnvelope::failure(&request.request_id, error),
    };
    if let Err(error) = response.validate() {
        return Json(endpoint::ResponseEnvelope::failure(
            request.request_id,
            error,
        ))
        .into_response();
    }
    Json(response).into_response()
}

pub(super) async fn operation_v4(
    state: &HostedState,
    headers: &HeaderMap,
    body: &[u8],
) -> Response {
    let request: v4::RequestEnvelope = match serde_json::from_slice(body) {
        Ok(request) => request,
        Err(_) => return StatusCode::BAD_REQUEST.into_response(),
    };
    if let Err(error) = request.validate() {
        return (
            StatusCode::BAD_REQUEST,
            Json(v4::ResponseEnvelope::failure(request.request_id, error)),
        )
            .into_response();
    }
    let principal = match principal(state, headers).await {
        Ok(value) => value,
        Err(response) => return response,
    };
    let reading = matches!(
        request.request,
        v4::OperationRequest::Search(_) | v4::OperationRequest::Describe(_)
    );
    let scope = if reading {
        "connectors.catalog.read"
    } else {
        "connectors.invoke"
    };
    // Check identity and the existing operation-family policy before source validation performs I/O.
    // This placeholder is used solely for the operation-id-based family policy, never dispatched.
    let policy_request = match request.request.clone().into_internal(Some("policy-only")) {
        Ok(value) => value,
        Err(error) => {
            return Json(v4::ResponseEnvelope::failure(request.request_id, error)).into_response()
        }
    };
    if principal.tenant_id != request.context.tenant_id
        || !principal.allows(scope)
        || (!reading && !state.policy.admits_operation(&principal, &policy_request))
    {
        return (
            StatusCode::FORBIDDEN,
            Json(v4::ResponseEnvelope::failure(
                request.request_id,
                v4::OperationError::new(
                    v4::OperationErrorCode::NotGranted,
                    "the operation is not admitted",
                    false,
                ),
            )),
        )
            .into_response();
    }
    let owner = match principal.principal_context(&request.request_id) {
        Ok(value) => value,
        Err(_) => return error(StatusCode::UNAUTHORIZED, "identity-access-token-refused"),
    };
    let normalized = match normalize_endpoint_operation(&*state.backend, &owner, request.request)
        .await
    {
        Ok(value) => value,
        Err(error) => {
            return Json(v4::ResponseEnvelope::failure(request.request_id, error)).into_response()
        }
    };
    let response = super::operation_decided(
        state,
        &principal,
        operation::RequestEnvelope {
            protocol: operation::CONTRACT.into(),
            request_id: request.request_id.clone(),
            context: request.context,
            request: normalized.request,
        },
    )
    .await;
    let (mut parts, body) = response.into_parts();
    let bytes = match axum::body::to_bytes(body, operation::MAX_RESULT_BYTES).await {
        Ok(value) => value,
        Err(_) => return StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    };
    let mut response = match operation::versions::decode_response(&bytes) {
        Ok((_, response)) => response,
        Err(_) => return Response::from_parts(parts, Body::from(bytes)),
    };
    if let Some(result) = response.response.take() {
        response = match constrain_endpoint_description(
            result,
            normalized.description_connection.as_deref(),
        ) {
            Ok(result) => operation::v3::ResponseEnvelope::success(&request.request_id, result),
            Err(error) => {
                operation::v3::ResponseEnvelope::failure(&request.request_id, error.into())
            }
        };
    }
    let response = v4::ResponseEnvelope::from(response);
    let bytes = match serde_json::to_vec(&response) {
        Ok(value) if response.validate().is_ok() => value,
        _ => return StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    };
    parts.headers.remove(header::CONTENT_LENGTH);
    Response::from_parts(parts, Body::from(bytes))
}
