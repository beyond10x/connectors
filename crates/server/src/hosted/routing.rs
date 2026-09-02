//! Route assembly and native-client bootstrap for the hosted transport.

use std::sync::Arc;

use axum::extract::{DefaultBodyLimit, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse as _, Json, Response};
use axum::routing::{get, post};
use axum::Router;
use serde::Serialize;
use service::ConnectorBackend;
use subscription_custody::SubscriptionCustody;

use super::{
    approval, catalog_route, connect, connection_route, datasource, docs, error, event, health,
    mcp, operation, HostedAdmissionPolicy, HostedAuthority, HostedState, IdentityVerifier,
    CONNECTORS_AUDIENCE,
};

pub(super) const MAX_COMPLETION_BYTES: usize = 8 * 1024;

/// Public facts a native Connectors client needs before it can authenticate.
///
/// The hosted Connector publishes its Identity dependency. Identity remains relying-party neutral
/// and never publishes a Connector URL in the opposite direction.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ClientDiscovery {
    pub protocol: &'static str,
    pub identity_origin: String,
    pub identity_audience: &'static str,
}

impl ClientDiscovery {
    /// Bind discovery to one already validated Identity origin.
    #[must_use]
    pub fn new(identity_origin: &url::Url) -> Self {
        Self {
            protocol: "b10x.connectors-client-discovery.v1",
            identity_origin: identity_origin.as_str().trim_end_matches('/').to_owned(),
            identity_audience: CONNECTORS_AUDIENCE,
        }
    }
}

pub fn router(
    verifier: Arc<dyn IdentityVerifier>,
    backend: Arc<dyn ConnectorBackend>,
    policy: HostedAdmissionPolicy,
    authority: HostedAuthority,
) -> Router {
    router_with_subscription_custody(verifier, backend, policy, authority, None)
}

/// Builds the hosted transport with optional Connector-owned subscription custody. Absence keeps
/// every custody and lease route unreachable.
pub fn router_with_subscription_custody(
    verifier: Arc<dyn IdentityVerifier>,
    backend: Arc<dyn ConnectorBackend>,
    policy: HostedAdmissionPolicy,
    authority: HostedAuthority,
    subscription_custody: Option<Arc<SubscriptionCustody>>,
) -> Router {
    router_inner(
        verifier,
        backend,
        policy,
        authority,
        subscription_custody,
        None,
    )
}

/// Builds the production hosted transport with native-client discovery at the same API base.
pub fn router_with_client_discovery(
    verifier: Arc<dyn IdentityVerifier>,
    backend: Arc<dyn ConnectorBackend>,
    policy: HostedAdmissionPolicy,
    authority: HostedAuthority,
    subscription_custody: Option<Arc<SubscriptionCustody>>,
    client_discovery: ClientDiscovery,
) -> Router {
    router_inner(
        verifier,
        backend,
        policy,
        authority,
        subscription_custody,
        Some(client_discovery),
    )
}

fn router_inner(
    verifier: Arc<dyn IdentityVerifier>,
    backend: Arc<dyn ConnectorBackend>,
    policy: HostedAdmissionPolicy,
    authority: HostedAuthority,
    subscription_custody: Option<Arc<SubscriptionCustody>>,
    client_discovery: Option<ClientDiscovery>,
) -> Router {
    Router::new()
        .route("/livez", get(health::liveness))
        .route("/readyz", get(health::readiness))
        .route("/healthz", get(health::readiness))
        .route(
            "/.well-known/connectors-client",
            get(client_discovery_route),
        )
        .route("/openapi.json", get(docs::openapi))
        .route("/docs", get(docs::page))
        .route(
            "/operations",
            post(operation).layer(DefaultBodyLimit::max(protocol::operation::MAX_FRAME_BYTES)),
        )
        .route(
            "/approvals",
            post(approval::issue).layer(DefaultBodyLimit::max(protocol::approval::MAX_FRAME_BYTES)),
        )
        .route(
            "/connections",
            post(connection_route::handle)
                .layer(DefaultBodyLimit::max(protocol::connection::MAX_FRAME_BYTES)),
        )
        .route(
            "/catalog",
            post(catalog_route::handle)
                .layer(DefaultBodyLimit::max(protocol::catalog::MAX_FRAME_BYTES)),
        )
        .route(
            "/events",
            post(event).layer(DefaultBodyLimit::max(protocol::event::MAX_FRAME_BYTES)),
        )
        .route(
            "/datasources",
            post(datasource).layer(DefaultBodyLimit::max(protocol::datasource::MAX_FRAME_BYTES)),
        )
        .route(
            "/mcp",
            post(mcp::handle).layer(DefaultBodyLimit::max(protocol::operation::MAX_FRAME_BYTES)),
        )
        .route(
            "/connect-sessions/{session_ref}",
            get(connect::completion_page)
                .post(connect::complete_session)
                .layer(DefaultBodyLimit::max(MAX_COMPLETION_BYTES)),
        )
        .route(
            "/oauth/{integration_ref}/callback",
            get(connect::oauth_callback),
        )
        .merge(super::subscription::routes())
        .with_state(HostedState {
            verifier,
            backend,
            policy,
            authority,
            subscription_custody,
            client_discovery,
        })
}

async fn client_discovery_route(State(state): State<HostedState>) -> Response {
    state.client_discovery.map_or_else(
        || {
            error(
                StatusCode::SERVICE_UNAVAILABLE,
                "client-discovery-unavailable",
            )
        },
        |discovery| Json(discovery).into_response(),
    )
}
