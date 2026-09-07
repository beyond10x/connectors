//! Identity and management-policy boundary for Connection requests.

use axum::body::{Body, Bytes};
use axum::extract::State;
use axum::http::{header, HeaderMap, StatusCode};
use axum::response::{IntoResponse as _, Json, Response};
use protocol::connection::{
    ConnectionError, ConnectionErrorCode, ConnectionRequest, RequestEnvelope, ResponseEnvelope,
};
use protocol::connection_v2 as v2;
use service::{ConnectSessionAccess, PrincipalContext};

use super::{
    bearer, error, HostedPrincipal, HostedState, IdentityVerificationError, CONNECTORS_AUDIENCE,
};

pub(super) async fn handle(
    State(state): State<HostedState>,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    let json = headers
        .get(header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok());
    if !json.is_some_and(|value| {
        let mime = value.split(';').next().unwrap_or("").trim();
        mime == "application/json" || (mime.starts_with("application/") && mime.ends_with("+json"))
    }) {
        return StatusCode::UNSUPPORTED_MEDIA_TYPE.into_response();
    }
    // The probe selects a response identity only; the strict reader consumes ORIGINAL bytes.
    let probe: serde_json::Value = match serde_json::from_slice(&body) {
        Ok(value) => value,
        Err(_) => return StatusCode::BAD_REQUEST.into_response(),
    };
    let version = match probe.get("protocol").and_then(serde_json::Value::as_str) {
        Some(protocol::connection::CONTRACT) => v2::Version::V0Alpha1,
        Some(v2::CONTRACT) => v2::Version::V0Alpha2,
        _ => return StatusCode::BAD_REQUEST.into_response(),
    };
    let request = match v2::decode_request(&body) {
        Ok((_, request)) => request,
        Err(refusal) => {
            let request_id = probe
                .get("request_id")
                .and_then(serde_json::Value::as_str)
                .filter(|value| {
                    ResponseEnvelope::failure(*value, refusal.clone())
                        .validate()
                        .is_ok()
                })
                .unwrap_or("invalid-request");
            return project(
                version,
                failure(request_id, refusal, StatusCode::BAD_REQUEST),
            )
            .await;
        }
    };
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
    let owner = match principal.principal_context(&request.request_id) {
        Ok(owner) => owner,
        Err(_) => return error(StatusCode::UNAUTHORIZED, "identity-access-token-refused"),
    };
    let response = if let Ok(ordinary_request) = request.clone().into_v1() {
        ordinary(&state, &principal, &owner, ordinary_request).await
    } else {
        let refusal = if principal.tenant_id != request.context.tenant_id
            || !principal.allows("connectors.invoke")
        {
            service::RemediationError::Refused
        } else {
            super::remediation::hosted_bound_refusal(&state, &principal, &owner, &request.request)
        };
        let (code, status) = match refusal {
            service::RemediationError::Refused => {
                (ConnectionErrorCode::NotGranted, StatusCode::FORBIDDEN)
            }
            service::RemediationError::InvalidInput => {
                (ConnectionErrorCode::InvalidInput, StatusCode::BAD_REQUEST)
            }
            service::RemediationError::Conflict => {
                (ConnectionErrorCode::Conflict, StatusCode::CONFLICT)
            }
            _ => (
                ConnectionErrorCode::Unavailable,
                StatusCode::SERVICE_UNAVAILABLE,
            ),
        };
        failure(
            &request.request_id,
            ConnectionError::new(code, refusal.to_string(), false),
            status,
        )
    };
    project(version, response).await
}

async fn ordinary(
    state: &HostedState,
    principal: &HostedPrincipal,
    owner: &PrincipalContext,
    request: RequestEnvelope,
) -> Response {
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
        && !state.policy.admits_operator(principal)
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
    if let Err(refusal) = crate::legacy_discovery::admit(&request.request) {
        return failure(&request.request_id, refusal, StatusCode::GONE);
    }
    let response = match state
        .backend
        .handle_connection(owner, request.request)
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

async fn project(version: v2::Version, response: Response) -> Response {
    let (mut parts, body) = response.into_parts();
    let bytes = match axum::body::to_bytes(body, v2::MAX_RESPONSE_BYTES).await {
        Ok(bytes) => bytes,
        Err(_) => return StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    };
    let (_, envelope) = match v2::decode_response(&bytes) {
        Ok(response) => response,
        Err(_) => return StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    };
    match version.encode_response(envelope) {
        Ok(bytes) => {
            parts.headers.remove(header::CONTENT_LENGTH);
            Response::from_parts(parts, Body::from(bytes))
        }
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    }
}
