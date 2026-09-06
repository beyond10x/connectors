//! The HTTP version boundary around the shared operation admission owner.

use super::*;
use axum::body::Bytes;
use protocol::operation::{
    legacy, v3,
    versions::{decode_request, decode_response, Version},
    MAX_FRAME_BYTES,
};

pub(super) async fn operation(
    State(state): State<HostedState>,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    let content_type = headers
        .get(header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok());
    if !content_type.is_some_and(|value| {
        let mime = value.split(';').next().unwrap_or("").trim();
        mime == "application/json" || (mime.starts_with("application/") && mime.ends_with("+json"))
    }) {
        return StatusCode::UNSUPPORTED_MEDIA_TYPE.into_response();
    }
    // Select only a supported identity. This probe never admits or dispatches a request.
    let probe: serde_json::Value = match serde_json::from_slice(&body) {
        Ok(value) => value,
        Err(_) => return StatusCode::BAD_REQUEST.into_response(),
    };
    let version = match probe.get("protocol").and_then(serde_json::Value::as_str) {
        Some(legacy::CONTRACT) => Version::V0Alpha1,
        Some(protocol::operation::wire::CONTRACT) => Version::V0Alpha2,
        Some(v3::CONTRACT) => Version::V0Alpha3,
        // Retain the existing v2 protocol-refusal envelope for unknown identities. The
        // original-byte reader below still rejects them before authentication or dispatch.
        _ => Version::V0Alpha2,
    };
    let request = match decode_request(&body) {
        Ok((_, request)) => request,
        Err(refusal) => {
            let request_id = probe
                .get("request_id")
                .and_then(serde_json::Value::as_str)
                .filter(|request_id| {
                    v3::ResponseEnvelope::failure(*request_id, refusal.clone())
                        .validate()
                        .is_ok()
                })
                .unwrap_or("invalid-request");
            return project(
                version,
                (
                    StatusCode::BAD_REQUEST,
                    Json(v3::ResponseEnvelope::failure(request_id, refusal)),
                )
                    .into_response(),
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
    project(
        version,
        operation_decided(&state, &principal, request).await,
    )
    .await
}

async fn project(version: Version, response: Response) -> Response {
    let (mut parts, body) = response.into_parts();
    let bytes = match axum::body::to_bytes(body, MAX_FRAME_BYTES).await {
        Ok(bytes) => bytes,
        Err(_) => return StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    };
    let envelope = match decode_response(&bytes) {
        Ok((_, envelope)) => envelope,
        Err(_) => return Response::from_parts(parts, Body::from(bytes)),
    };
    // Predecessors project authentication needs to neutral Unavailable bodies;
    // keep their HTTP status aligned with that defined loss.
    if version != Version::V0Alpha3
        && parts.status == StatusCode::CONFLICT
        && envelope
            .error
            .as_ref()
            .is_some_and(|error| error.code == v3::OperationErrorCode::AuthenticationRequired)
    {
        parts.status = StatusCode::SERVICE_UNAVAILABLE;
    }
    match version.encode_response(envelope) {
        Ok(bytes) => {
            parts.headers.remove(header::CONTENT_LENGTH);
            Response::from_parts(parts, Body::from(bytes))
        }
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    }
}
