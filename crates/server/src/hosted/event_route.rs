//! Exact event-version boundary and the existing operator/self event authority.
use super::*;
use axum::body::Bytes;
use protocol::event::v2;

pub(super) async fn event(
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
    #[derive(Deserialize)]
    struct Identity {
        protocol: String,
    }
    let identity: Identity = match serde_json::from_slice(&body) {
        Ok(value) => value,
        Err(_) => return StatusCode::BAD_REQUEST.into_response(),
    };
    if identity.protocol != v2::CONTRACT {
        let legacy = match serde_json::from_slice::<EventRequestEnvelope>(&body) {
            Ok(value) => value,
            Err(_) => return StatusCode::UNPROCESSABLE_ENTITY.into_response(),
        };
        return super::legacy_event(State(state), headers, Json(legacy)).await;
    }
    let request: v2::RequestEnvelope = match serde_json::from_slice(&body) {
        Ok(value) => value,
        Err(_) => return StatusCode::BAD_REQUEST.into_response(),
    };
    if body.len() > v2::MAX_FRAME_BYTES {
        return StatusCode::PAYLOAD_TOO_LARGE.into_response();
    }
    if let Err(error) = request.validate() {
        return (
            StatusCode::BAD_REQUEST,
            Json(v2::ResponseEnvelope::failure(request.request_id, error)),
        )
            .into_response();
    }
    let Some(credential) = bearer(&headers) else {
        return error(StatusCode::UNAUTHORIZED, "identity-access-token-required");
    };
    let principal = match state.verifier.verify(credential, CONNECTORS_AUDIENCE).await {
        Ok(value) => value,
        Err(IdentityVerificationError::Refused) => {
            return error(StatusCode::UNAUTHORIZED, "identity-access-token-refused")
        }
        Err(IdentityVerificationError::Unavailable) => {
            return error(StatusCode::SERVICE_UNAVAILABLE, "identity-unavailable")
        }
    };
    let operator =
        principal.allows("connectors.events.read") && state.policy.admits_operator(&principal);
    let self_read = principal.allows("connectors.events.self")
        && request
            .request
            .legacy_read()
            .is_some_and(|request| super::self_service_slack_event(&request));
    if principal.tenant_id != request.context.tenant_id || !(operator || self_read) {
        return (
            StatusCode::FORBIDDEN,
            Json(v2::ResponseEnvelope::failure(
                request.request_id,
                EventError::new(
                    EventErrorCode::NotGranted,
                    "the verified authority does not admit this Connector event request",
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
    let response = match state
        .backend
        .handle_event_v2(&owner, request.request.clone())
        .await
    {
        Ok(result) if result.matches(&request.request) => {
            v2::ResponseEnvelope::success(&request.request_id, result)
        }
        Ok(_) => v2::ResponseEnvelope::failure(
            &request.request_id,
            EventError::new(
                EventErrorCode::Protocol,
                "subscription backend returned an unrelated result",
                false,
            ),
        ),
        Err(error) => v2::ResponseEnvelope::failure(&request.request_id, error),
    };
    let response = match response.validate() {
        Ok(()) => response,
        Err(error) => v2::ResponseEnvelope::failure(request.request_id, error),
    };
    Json(response).into_response()
}
