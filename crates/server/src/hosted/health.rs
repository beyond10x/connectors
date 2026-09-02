//! Local liveness and dependency-backed readiness probes.

use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{IntoResponse as _, Response};

use super::HostedState;

pub(super) async fn liveness() -> &'static str {
    "ok\n"
}

pub(super) async fn readiness(State(state): State<HostedState>) -> Response {
    if crate::catalog_projection::ready().is_err() {
        return (StatusCode::SERVICE_UNAVAILABLE, "catalog-invalid\n").into_response();
    }
    if state.verifier.ready().await.is_err() {
        return (StatusCode::SERVICE_UNAVAILABLE, "identity-unavailable\n").into_response();
    }
    match state.backend.ready().await {
        Ok(()) => (StatusCode::OK, "ok\n").into_response(),
        Err(_) => (StatusCode::SERVICE_UNAVAILABLE, "backend-unavailable\n").into_response(),
    }
}
