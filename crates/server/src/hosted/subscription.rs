use axum::extract::{DefaultBodyLimit, Path, State};
use axum::http::{header, HeaderMap, StatusCode};
use axum::response::{IntoResponse, Json, Response};
use axum::routing::{get, post};
use axum::Router;
use serde::{Deserialize, Serialize};
use subscription_custody::CustodyError;
use zeroize::Zeroizing;

use super::{
    bearer, error, HostedPrincipal, HostedState, IdentityVerificationError, CONNECTORS_AUDIENCE,
};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct ConnectRequest {
    credential: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct CreateLeaseRequest {
    attempt_id: String,
    #[serde(default = "default_lease_ttl")]
    ttl_seconds: u64,
    #[serde(default = "default_lease_uses")]
    maximum_uses: u16,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct RedeemLeaseRequest {
    attempt_id: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct CompleteOAuthRequest {
    flow_id: String,
    code: String,
}

#[derive(Serialize)]
#[serde(deny_unknown_fields)]
struct StatusResponse {
    provider: &'static str,
    connected: bool,
}

#[derive(Serialize)]
#[serde(deny_unknown_fields)]
struct LeaseResponse {
    lease_id: String,
    lease_token: String,
    expires_at: u64,
}

#[derive(Serialize)]
#[serde(deny_unknown_fields)]
struct RedeemedResponse {
    credential: String,
    kind: &'static str,
}

#[derive(Serialize)]
#[serde(deny_unknown_fields)]
struct OAuthStartResponse {
    authorization_url: String,
    flow_id: String,
    expires_at: u64,
}

const fn default_lease_ttl() -> u64 {
    15 * 60
}

const fn default_lease_uses() -> u16 {
    32
}

pub(super) fn routes() -> Router<HostedState> {
    Router::new()
        .route(
            "/subscription-credentials/claude-code",
            get(status)
                .put(connect)
                .delete(disconnect)
                .layer(DefaultBodyLimit::max(20 * 1024)),
        )
        .route(
            "/subscription-credentials/claude-code/leases",
            post(create_lease).layer(DefaultBodyLimit::max(4 * 1024)),
        )
        .route(
            "/subscription-credentials/claude-code/oauth/start",
            post(start_oauth).layer(DefaultBodyLimit::max(1024)),
        )
        .route(
            "/subscription-credentials/claude-code/oauth/complete",
            post(complete_oauth).layer(DefaultBodyLimit::max(12 * 1024)),
        )
        .route(
            "/subscription-leases/{lease_id}/redeem",
            post(redeem_lease).layer(DefaultBodyLimit::max(4 * 1024)),
        )
}

pub(super) async fn status(State(state): State<HostedState>, headers: HeaderMap) -> Response {
    let principal = match principal(&state, &headers, "connectors.connections.self").await {
        Ok(principal) => principal,
        Err(response) => return response,
    };
    let Some(custody) = state.subscription_custody.as_ref() else {
        return error(StatusCode::NOT_FOUND, "subscription-custody-disabled");
    };
    match custody
        .connected(&principal.tenant_id, &principal.subject)
        .await
    {
        Ok(connected) => confidential_json(StatusResponse {
            provider: "claude-code",
            connected,
        }),
        Err(error) => custody_error(error),
    }
}

pub(super) async fn connect(
    State(state): State<HostedState>,
    headers: HeaderMap,
    Json(request): Json<ConnectRequest>,
) -> Response {
    let principal = match principal(&state, &headers, "connectors.connections.self").await {
        Ok(principal) => principal,
        Err(response) => return response,
    };
    let Some(custody) = state.subscription_custody.as_ref() else {
        return error(StatusCode::NOT_FOUND, "subscription-custody-disabled");
    };
    match custody
        .connect(
            &principal.tenant_id,
            &principal.subject,
            Zeroizing::new(request.credential),
        )
        .await
    {
        Ok(()) => confidential_json(StatusResponse {
            provider: "claude-code",
            connected: true,
        }),
        Err(error) => custody_error(error),
    }
}

pub(super) async fn disconnect(State(state): State<HostedState>, headers: HeaderMap) -> Response {
    let principal = match principal(&state, &headers, "connectors.connections.self").await {
        Ok(principal) => principal,
        Err(response) => return response,
    };
    let Some(custody) = state.subscription_custody.as_ref() else {
        return error(StatusCode::NOT_FOUND, "subscription-custody-disabled");
    };
    match custody
        .disconnect(&principal.tenant_id, &principal.subject)
        .await
    {
        Ok(()) => confidential_json(StatusResponse {
            provider: "claude-code",
            connected: false,
        }),
        Err(error) => custody_error(error),
    }
}

pub(super) async fn start_oauth(State(state): State<HostedState>, headers: HeaderMap) -> Response {
    let principal = match principal(&state, &headers, "connectors.connections.self").await {
        Ok(principal) => principal,
        Err(response) => return response,
    };
    let Some(custody) = state.subscription_custody.as_ref() else {
        return error(StatusCode::NOT_FOUND, "subscription-custody-disabled");
    };
    match custody
        .start_oauth(&principal.tenant_id, &principal.subject)
        .await
    {
        Ok(start) => confidential_json(OAuthStartResponse {
            authorization_url: start.authorization_url,
            flow_id: start.flow_id,
            expires_at: start.expires_at,
        }),
        Err(error) => custody_error(error),
    }
}

pub(super) async fn complete_oauth(
    State(state): State<HostedState>,
    headers: HeaderMap,
    Json(request): Json<CompleteOAuthRequest>,
) -> Response {
    let principal = match principal(&state, &headers, "connectors.connections.self").await {
        Ok(principal) => principal,
        Err(response) => return response,
    };
    let Some(custody) = state.subscription_custody.as_ref() else {
        return error(StatusCode::NOT_FOUND, "subscription-custody-disabled");
    };
    match custody
        .complete_oauth(
            &principal.tenant_id,
            &principal.subject,
            &request.flow_id,
            &request.code,
        )
        .await
    {
        Ok(()) => confidential_json(StatusResponse {
            provider: "claude-code",
            connected: true,
        }),
        Err(error) => custody_error(error),
    }
}

pub(super) async fn create_lease(
    State(state): State<HostedState>,
    headers: HeaderMap,
    Json(request): Json<CreateLeaseRequest>,
) -> Response {
    let principal = match principal(&state, &headers, "connectors.credentials.lease").await {
        Ok(principal) => principal,
        Err(response) => return response,
    };
    let Some(custody) = state.subscription_custody.as_ref() else {
        return error(StatusCode::NOT_FOUND, "subscription-custody-disabled");
    };
    match custody
        .lease(
            &principal.tenant_id,
            &principal.subject,
            &request.attempt_id,
            std::time::Duration::from_secs(request.ttl_seconds),
            request.maximum_uses,
        )
        .await
    {
        Ok(capability) => confidential_json(LeaseResponse {
            lease_token: capability.expose_at_transport_boundary().to_owned(),
            lease_id: capability.lease_id,
            expires_at: capability.expires_at,
        }),
        Err(error) => custody_error(error),
    }
}

pub(super) async fn redeem_lease(
    State(state): State<HostedState>,
    Path(lease_id): Path<String>,
    headers: HeaderMap,
    Json(request): Json<RedeemLeaseRequest>,
) -> Response {
    let Some(custody) = state.subscription_custody.as_ref() else {
        return error(StatusCode::NOT_FOUND, "subscription-custody-disabled");
    };
    let Some(lease_token) = bearer(&headers) else {
        return error(StatusCode::UNAUTHORIZED, "subscription-lease-required");
    };
    match custody
        .redeem(&lease_id, lease_token, &request.attempt_id)
        .await
    {
        Ok(credential) => confidential_json(RedeemedResponse {
            credential: credential.expose_secret().to_owned(),
            kind: "oauth",
        }),
        Err(error) => custody_error(error),
    }
}

#[allow(clippy::result_large_err)]
async fn principal(
    state: &HostedState,
    headers: &HeaderMap,
    scope: &str,
) -> Result<HostedPrincipal, Response> {
    let credential = bearer(headers)
        .ok_or_else(|| error(StatusCode::UNAUTHORIZED, "identity-access-token-required"))?;
    let principal = state
        .verifier
        .verify(credential, CONNECTORS_AUDIENCE)
        .await
        .map_err(|failure| match failure {
            IdentityVerificationError::Refused => {
                error(StatusCode::UNAUTHORIZED, "identity-access-token-refused")
            }
            IdentityVerificationError::Unavailable => {
                error(StatusCode::SERVICE_UNAVAILABLE, "identity-unavailable")
            }
        })?;
    if principal.scopes.contains(scope) {
        Ok(principal)
    } else {
        Err(error(StatusCode::FORBIDDEN, "identity-scope-refused"))
    }
}

fn confidential_json<T: Serialize>(value: T) -> Response {
    let mut response = Json(value).into_response();
    response.headers_mut().insert(
        header::CACHE_CONTROL,
        header::HeaderValue::from_static("no-store"),
    );
    response
        .headers_mut()
        .insert(header::PRAGMA, header::HeaderValue::from_static("no-cache"));
    response
}

fn custody_error(error: CustodyError) -> Response {
    match error {
        CustodyError::InvalidCredential | CustodyError::InvalidAttempt => {
            super::error(StatusCode::BAD_REQUEST, "subscription-request-invalid")
        }
        CustodyError::NotConnected => {
            super::error(StatusCode::CONFLICT, "subscription-not-connected")
        }
        CustodyError::LeaseRefused => {
            super::error(StatusCode::UNAUTHORIZED, "subscription-lease-refused")
        }
        CustodyError::OauthRefused => {
            super::error(StatusCode::BAD_REQUEST, "subscription-oauth-refused")
        }
        CustodyError::Unavailable => super::error(
            StatusCode::SERVICE_UNAVAILABLE,
            "subscription-custody-unavailable",
        ),
    }
}
