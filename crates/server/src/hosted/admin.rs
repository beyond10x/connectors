//! Explicitly operator-only hosted Integration administration.

use axum::extract::{DefaultBodyLimit, Path, State};
use axum::http::{header, HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, put};
use axum::{Json, Router};
use serde::{Deserialize, Serialize};
use service::{AdminCredentialInput, AdminError};

use super::{bearer, ErrorBody, HostedState, IdentityVerificationError, CONNECTORS_AUDIENCE};

const MAX_ADMIN_CREDENTIAL_REQUEST_BYTES: usize = 12 * 1024;
const ADMIN_SCOPE: &str = "connectors.integrations.manage";

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct CredentialRequest {
    request_id: String,
    value: String,
    #[serde(default)]
    replace: bool,
}

#[derive(Serialize)]
#[serde(deny_unknown_fields)]
struct CredentialResponse<'a> {
    request_id: &'a str,
    integration_ref: &'a str,
    credential: &'a str,
    state: &'static str,
    replaced: bool,
}

pub(super) fn routes() -> Router<HostedState> {
    Router::new()
        .route("/admin/auth-metadata", get(auth_metadata))
        .route("/admin/integrations", get(status))
        .route(
            "/admin/integrations/{integration_ref}/credentials/{credential}",
            put(set_credential).layer(DefaultBodyLimit::max(MAX_ADMIN_CREDENTIAL_REQUEST_BYTES)),
        )
}

#[derive(Serialize)]
#[serde(deny_unknown_fields)]
struct AuthMetadata<'a> {
    identity_origin: &'a str,
    audience: &'static str,
    scope: &'static str,
}

async fn auth_metadata(State(state): State<HostedState>) -> Response {
    let Some(identity_origin) = state.verifier.login_origin() else {
        return confidential_error(StatusCode::SERVICE_UNAVAILABLE, "identity-unavailable");
    };
    confidential_json(AuthMetadata {
        identity_origin,
        audience: CONNECTORS_AUDIENCE,
        scope: ADMIN_SCOPE,
    })
}

async fn status(State(state): State<HostedState>, headers: HeaderMap) -> Response {
    if let Err(response) = authorize(&state, &headers).await {
        return *response;
    }
    let Some(admin) = state.admin.as_ref() else {
        return confidential_error(StatusCode::SERVICE_UNAVAILABLE, "admin-unavailable");
    };
    confidential_json(admin.status().await)
}

async fn set_credential(
    State(state): State<HostedState>,
    headers: HeaderMap,
    Path((integration_ref, credential)): Path<(String, String)>,
    Json(request): Json<CredentialRequest>,
) -> Response {
    let principal = match authorize(&state, &headers).await {
        Ok(principal) => principal,
        Err(response) => return *response,
    };
    if request.request_id.is_empty()
        || request.request_id.len() > 512
        || !request
            .request_id
            .bytes()
            .all(|byte| byte.is_ascii_graphic())
    {
        return confidential_error(StatusCode::UNPROCESSABLE_ENTITY, "admin-request-invalid");
    }
    let Some(admin) = state.admin.as_ref() else {
        return confidential_error(StatusCode::SERVICE_UNAVAILABLE, "admin-unavailable");
    };
    let result = admin
        .put(
            &principal.actor_subject,
            &request.request_id,
            &integration_ref,
            &credential,
            AdminCredentialInput::new(request.value),
            request.replace,
        )
        .await;
    match result {
        Ok(replaced) => confidential_json(CredentialResponse {
            request_id: &request.request_id,
            integration_ref: &integration_ref,
            credential: &credential,
            state: "present",
            replaced,
        }),
        Err(AdminError::Invalid) => {
            confidential_error(StatusCode::UNPROCESSABLE_ENTITY, "admin-credential-invalid")
        }
        Err(AdminError::NotFound) => {
            confidential_error(StatusCode::NOT_FOUND, "admin-requirement-not-found")
        }
        Err(AdminError::Conflict) => {
            confidential_error(StatusCode::CONFLICT, "admin-credential-exists")
        }
        Err(AdminError::Unavailable | AdminError::AuditUnavailable) => {
            confidential_error(StatusCode::SERVICE_UNAVAILABLE, "admin-unavailable")
        }
    }
}

async fn authorize(
    state: &HostedState,
    headers: &HeaderMap,
) -> Result<super::HostedPrincipal, Box<Response>> {
    let credential = bearer(headers).ok_or_else(|| {
        Box::new(confidential_error(
            StatusCode::UNAUTHORIZED,
            "identity-access-token-required",
        ))
    })?;
    let principal = match state.verifier.verify(credential, CONNECTORS_AUDIENCE).await {
        Ok(principal) => principal,
        Err(IdentityVerificationError::Refused) => {
            return Err(Box::new(confidential_error(
                StatusCode::UNAUTHORIZED,
                "identity-access-token-refused",
            )));
        }
        Err(IdentityVerificationError::Unavailable) => {
            return Err(Box::new(confidential_error(
                StatusCode::SERVICE_UNAVAILABLE,
                "identity-unavailable",
            )));
        }
    };
    if !principal.allows(ADMIN_SCOPE) || !state.policy.admits_operator(&principal) {
        return Err(Box::new(confidential_error(
            StatusCode::FORBIDDEN,
            "admin-not-granted",
        )));
    }
    Ok(principal)
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

fn confidential_error(status: StatusCode, code: &'static str) -> Response {
    let mut response = (status, Json(ErrorBody { error: code })).into_response();
    response.headers_mut().insert(
        header::CACHE_CONTROL,
        header::HeaderValue::from_static("no-store"),
    );
    response
        .headers_mut()
        .insert(header::PRAGMA, header::HeaderValue::from_static("no-cache"));
    response
}
