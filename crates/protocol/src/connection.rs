//! Credential-free Connection and Connect Session protocol.

// The frozen JSON Schema is the normative field-level contract. This module is its strict Rust
// projection and deliberately has no type capable of carrying credential material.
#![allow(missing_docs)]

use serde::{Deserialize, Serialize};

use crate::operation::OwnerContext;

pub const CONTRACT: &str = "b10x.connector-connection.v0alpha1";
pub const MAX_FRAME_BYTES: usize = 64 * 1024;
pub const MAX_RESPONSE_BYTES: usize = 128 * 1024;
pub const MAX_SEARCH_RESULTS: u16 = 64;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RequestEnvelope {
    pub protocol: String,
    pub request_id: String,
    pub context: OwnerContext,
    pub request: ConnectionRequest,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "method",
    content = "params",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum ConnectionRequest {
    Search(SearchRequest),
    Describe(DescribeRequest),
    /// Read stored observations only. An active provider refresh remains an admitted operation.
    ObservationSearch(ObservationSearchRequest),
    /// Explicitly turn one recognized stored observation into a mediated Connection.
    Materialize(MaterializeRequest),
    ConnectSessionCreate(ConnectSessionCreateRequest),
    ConnectSessionStatus(ConnectSessionStatusRequest),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SearchRequest {
    pub query: String,
    pub limit: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DescribeRequest {
    pub connection_ref: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ObservationSearchRequest {
    pub source_connection_ref: String,
    pub query: String,
    pub limit: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MaterializeRequest {
    /// Opaque observation identity. Resource bindings and route selection remain Connector-owned.
    pub observation_ref: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConnectSessionCreateRequest {
    pub integration_ref: String,
    pub label: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConnectSessionStatusRequest {
    pub connect_session_ref: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConnectionState {
    Created,
    Authorized,
    Callable,
    Degraded,
    Revoked,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConnectionInitiator {
    B10x,
    Provider,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChannelState {
    Starting,
    Connected,
    Reconnecting,
    Stopped,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RouteAdapter {
    GrafanaDatasourceProxyV1,
}

/// Value-free projection of a Connection's immutable route.
///
/// The Connector-owned resource binding is deliberately absent: callers need to know that a route
/// is mediated, not the Grafana data-source UID or proxy path used to implement it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ConnectionRoute {
    Direct,
    ViaConnection {
        parent_connection_ref: String,
        route_adapter: RouteAdapter,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ChannelSummary {
    pub channel_ref: String,
    pub binding_ref: String,
    pub state: ChannelState,
    pub events: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConnectionSummary {
    pub connection_ref: String,
    pub integration_ref: String,
    pub label: String,
    pub state: ConnectionState,
    pub initiation: Vec<ConnectionInitiator>,
    pub route: ConnectionRoute,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConnectionDescription {
    #[serde(flatten)]
    pub summary: ConnectionSummary,
    pub channels: Vec<ChannelSummary>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DiscoveryObservationState {
    Observed,
    Unsupported,
    Materialized,
    Withdrawn,
}

/// Value-free projection of one already-reconciled discovery observation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DiscoveryObservationSummary {
    pub observation_ref: String,
    pub discovery_ref: String,
    pub source_connection_ref: String,
    pub observed_type: String,
    pub title: String,
    pub state: DiscoveryObservationState,
    pub evidence_generation: u64,
    pub evidence_sha256: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target_provider_ref: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub connection_ref: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConnectSessionState {
    Pending,
    Completed,
    Expired,
    Failed,
}

/// Value-free status for one single-purpose acquisition session.
///
/// `completion_endpoint` is a short-lived Connector-owned endpoint, not an Agent Endpoint and not
/// the durable Connection. It is present only while pending and accepts one completion attempt.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConnectSessionStatus {
    pub connect_session_ref: String,
    pub integration_ref: String,
    pub state: ConnectSessionState,
    pub expires_at_unix_ms: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub completion_endpoint: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub connection_ref: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "result",
    content = "value",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum ConnectionResult {
    Search {
        connections: Vec<ConnectionSummary>,
    },
    Describe(ConnectionDescription),
    ObservationSearch {
        observations: Vec<DiscoveryObservationSummary>,
    },
    Materialize(ConnectionDescription),
    ConnectSessionCreate(ConnectSessionStatus),
    ConnectSessionStatus(ConnectSessionStatus),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConnectionErrorCode {
    Unavailable,
    NotFound,
    NotGranted,
    InvalidInput,
    StaleAuthority,
    Conflict,
    Protocol,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, thiserror::Error)]
#[error("{code:?}: {message}")]
#[serde(deny_unknown_fields)]
pub struct ConnectionError {
    pub code: ConnectionErrorCode,
    pub message: String,
    pub retriable: bool,
}

impl ConnectionError {
    #[must_use]
    pub fn new(code: ConnectionErrorCode, message: impl Into<String>, retriable: bool) -> Self {
        Self {
            code,
            message: message.into(),
            retriable,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResponseStatus {
    Ok,
    Error,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ResponseEnvelope {
    pub protocol: String,
    pub request_id: String,
    pub status: ResponseStatus,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub response: Option<ConnectionResult>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<ConnectionError>,
}

impl RequestEnvelope {
    pub fn validate(&self) -> Result<(), ConnectionError> {
        if self.protocol != CONTRACT || !valid_ref(&self.request_id, 128) {
            return Err(protocol_refusal());
        }
        validate_context(&self.context)?;
        match &self.request {
            ConnectionRequest::Search(request) => {
                if request.query.len() > 512
                    || request.limit == 0
                    || request.limit > MAX_SEARCH_RESULTS
                {
                    return Err(invalid_input("connection search bounds are invalid"));
                }
            }
            ConnectionRequest::Describe(request) => require_ref(&request.connection_ref)?,
            ConnectionRequest::ObservationSearch(request) => {
                require_ref(&request.source_connection_ref)?;
                if request.query.len() > 512
                    || request.limit == 0
                    || request.limit > MAX_SEARCH_RESULTS
                {
                    return Err(invalid_input("observation search bounds are invalid"));
                }
            }
            ConnectionRequest::Materialize(request) => require_ref(&request.observation_ref)?,
            ConnectionRequest::ConnectSessionCreate(request) => {
                require_ref(&request.integration_ref)?;
                if request.label.trim().is_empty() || request.label.len() > 256 {
                    return Err(invalid_input("connection label is invalid"));
                }
            }
            ConnectionRequest::ConnectSessionStatus(request) => {
                require_ref(&request.connect_session_ref)?;
            }
        }
        Ok(())
    }
}

impl ResponseEnvelope {
    #[must_use]
    pub fn success(request_id: impl Into<String>, response: ConnectionResult) -> Self {
        Self {
            protocol: CONTRACT.to_owned(),
            request_id: request_id.into(),
            status: ResponseStatus::Ok,
            response: Some(response),
            error: None,
        }
    }

    #[must_use]
    pub fn failure(request_id: impl Into<String>, error: ConnectionError) -> Self {
        Self {
            protocol: CONTRACT.to_owned(),
            request_id: request_id.into(),
            status: ResponseStatus::Error,
            response: None,
            error: Some(error),
        }
    }

    pub fn validate(&self) -> Result<(), ConnectionError> {
        if self.protocol != CONTRACT || !valid_ref(&self.request_id, 128) {
            return Err(protocol_refusal());
        }
        match (self.status, &self.response, &self.error) {
            (ResponseStatus::Ok, Some(result), None) => validate_result(result)?,
            (ResponseStatus::Error, None, Some(error))
                if !error.message.is_empty() && error.message.len() <= 4096 => {}
            _ => return Err(protocol_refusal()),
        }
        if serde_json::to_vec(self).map_or(true, |bytes| bytes.len() > MAX_RESPONSE_BYTES) {
            return Err(protocol_refusal());
        }
        Ok(())
    }
}

fn validate_result(result: &ConnectionResult) -> Result<(), ConnectionError> {
    match result {
        ConnectionResult::Search { connections } => {
            if connections.len() > usize::from(MAX_SEARCH_RESULTS) {
                return Err(protocol_refusal());
            }
            for connection in connections {
                validate_summary(connection)?;
            }
        }
        ConnectionResult::Describe(description) | ConnectionResult::Materialize(description) => {
            validate_summary(&description.summary)?;
            if description.channels.len() > 64 {
                return Err(protocol_refusal());
            }
            for channel in &description.channels {
                require_ref(&channel.channel_ref)?;
                require_ref(&channel.binding_ref)?;
                if channel.events.len() > 64
                    || channel.events.iter().any(|event| !valid_ref(event, 512))
                {
                    return Err(protocol_refusal());
                }
            }
        }
        ConnectionResult::ObservationSearch { observations } => {
            if observations.len() > usize::from(MAX_SEARCH_RESULTS) {
                return Err(protocol_refusal());
            }
            for observation in observations {
                validate_observation(observation)?;
            }
        }
        ConnectionResult::ConnectSessionCreate(session)
        | ConnectionResult::ConnectSessionStatus(session) => {
            require_ref(&session.connect_session_ref)?;
            require_ref(&session.integration_ref)?;
            if session
                .completion_endpoint
                .as_deref()
                .is_some_and(|value| value.is_empty() || value.len() > 4096 || value.contains('\n'))
                || session
                    .connection_ref
                    .as_deref()
                    .is_some_and(|value| !valid_ref(value, 512))
            {
                return Err(protocol_refusal());
            }
            match session.state {
                ConnectSessionState::Pending
                    if session.completion_endpoint.is_some()
                        && session.connection_ref.is_none() => {}
                ConnectSessionState::Completed
                    if session.completion_endpoint.is_none()
                        && session.connection_ref.is_some() => {}
                ConnectSessionState::Expired | ConnectSessionState::Failed
                    if session.completion_endpoint.is_none()
                        && session.connection_ref.is_none() => {}
                _ => return Err(protocol_refusal()),
            }
        }
    }
    Ok(())
}

fn validate_observation(observation: &DiscoveryObservationSummary) -> Result<(), ConnectionError> {
    if !valid_ref(&observation.observation_ref, 512)
        || !valid_ref(&observation.discovery_ref, 512)
        || !valid_ref(&observation.source_connection_ref, 512)
        || observation.observed_type.is_empty()
        || observation.observed_type.len() > 128
        || observation
            .observed_type
            .bytes()
            .any(|byte| byte.is_ascii_control())
        || observation.title.trim().is_empty()
        || observation.title.len() > 256
        || observation.evidence_generation == 0
        || observation.evidence_sha256.len() != 64
        || !observation
            .evidence_sha256
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit())
        || observation
            .target_provider_ref
            .as_deref()
            .is_some_and(|value| !valid_ref(value, 512))
        || observation
            .connection_ref
            .as_deref()
            .is_some_and(|value| !valid_ref(value, 512))
    {
        return Err(protocol_refusal());
    }
    match observation.state {
        DiscoveryObservationState::Observed
            if observation.target_provider_ref.is_some()
                && observation.connection_ref.is_none() => {}
        DiscoveryObservationState::Unsupported
            if observation.target_provider_ref.is_none()
                && observation.connection_ref.is_none() => {}
        DiscoveryObservationState::Materialized
            if observation.target_provider_ref.is_some()
                && observation.connection_ref.is_some() => {}
        DiscoveryObservationState::Withdrawn if observation.connection_ref.is_none() => {}
        _ => return Err(protocol_refusal()),
    }
    Ok(())
}

fn validate_summary(summary: &ConnectionSummary) -> Result<(), ConnectionError> {
    if !valid_ref(&summary.connection_ref, 512)
        || !valid_ref(&summary.integration_ref, 512)
        || summary.label.trim().is_empty()
        || summary.label.len() > 256
        || summary.initiation.is_empty()
        || summary.initiation.len() > 2
    {
        return Err(protocol_refusal());
    }
    match &summary.route {
        ConnectionRoute::Direct => {}
        ConnectionRoute::ViaConnection {
            parent_connection_ref,
            ..
        } if valid_ref(parent_connection_ref, 512)
            && parent_connection_ref != &summary.connection_ref => {}
        ConnectionRoute::ViaConnection { .. } => return Err(protocol_refusal()),
    }
    Ok(())
}

fn validate_context(context: &OwnerContext) -> Result<(), ConnectionError> {
    if !valid_ref(&context.tenant_id, 512)
        || !valid_ref(&context.agent_id, 512)
        || context.agent_revision == 0
        || !valid_ref(&context.authority_snapshot_id, 512)
        || context.authority_snapshot_sha256.len() != 64
        || !context
            .authority_snapshot_sha256
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit())
    {
        return Err(protocol_refusal());
    }
    Ok(())
}

fn require_ref(value: &str) -> Result<(), ConnectionError> {
    if valid_ref(value, 512) {
        Ok(())
    } else {
        Err(invalid_input("reference is invalid"))
    }
}

fn valid_ref(value: &str, max: usize) -> bool {
    !value.is_empty()
        && value.len() <= max
        && value
            .bytes()
            .all(|byte| !byte.is_ascii_control() && byte != b' ')
}

fn invalid_input(message: &'static str) -> ConnectionError {
    ConnectionError::new(ConnectionErrorCode::InvalidInput, message, false)
}

fn protocol_refusal() -> ConnectionError {
    ConnectionError::new(
        ConnectionErrorCode::Protocol,
        "connection protocol frame was refused",
        false,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn context() -> OwnerContext {
        OwnerContext {
            tenant_id: "tenant-local".to_owned(),
            agent_id: "agent-dev".to_owned(),
            agent_revision: 1,
            authority_snapshot_id: "authority-1".to_owned(),
            authority_snapshot_sha256: "a".repeat(64),
        }
    }

    #[test]
    fn pending_and_completed_sessions_cannot_mix_endpoint_and_connection() {
        let pending = ConnectSessionStatus {
            connect_session_ref: "connect-session:1".to_owned(),
            integration_ref: "slack".to_owned(),
            state: ConnectSessionState::Pending,
            expires_at_unix_ms: 1,
            completion_endpoint: Some("unix:/state/connect.sock".to_owned()),
            connection_ref: None,
        };
        ResponseEnvelope::success(
            "request-1",
            ConnectionResult::ConnectSessionCreate(pending.clone()),
        )
        .validate()
        .unwrap();
        let mut invalid = pending;
        invalid.connection_ref = Some("connection:1".to_owned());
        assert!(ResponseEnvelope::success(
            "request-1",
            ConnectionResult::ConnectSessionCreate(invalid)
        )
        .validate()
        .is_err());
    }

    #[test]
    fn secret_shaped_unknown_fields_are_refused() {
        let json = serde_json::json!({
            "protocol": CONTRACT,
            "request_id": "request-1",
            "context": context(),
            "request": {
                "method": "connect_session_create",
                "params": {
                    "integration_ref": "slack",
                    "label": "Development Slack",
                    "token": "SENTINEL-NOT-A-REAL-SECRET"
                }
            }
        });
        assert!(serde_json::from_value::<RequestEnvelope>(json).is_err());
    }

    fn summary(route: ConnectionRoute) -> ConnectionSummary {
        ConnectionSummary {
            connection_ref: "connection:prometheus".to_owned(),
            integration_ref: "integration:prometheus".to_owned(),
            label: "Prometheus via Grafana".to_owned(),
            state: ConnectionState::Callable,
            initiation: vec![ConnectionInitiator::B10x],
            route,
        }
    }

    #[test]
    fn mediated_route_is_value_free_closed_and_cannot_self_reference() {
        let response = |route| {
            ResponseEnvelope::success(
                "request-1",
                ConnectionResult::Search {
                    connections: vec![summary(route)],
                },
            )
        };
        response(ConnectionRoute::ViaConnection {
            parent_connection_ref: "connection:grafana".to_owned(),
            route_adapter: RouteAdapter::GrafanaDatasourceProxyV1,
        })
        .validate()
        .expect("closed mediated route");

        assert!(response(ConnectionRoute::ViaConnection {
            parent_connection_ref: "connection:prometheus".to_owned(),
            route_adapter: RouteAdapter::GrafanaDatasourceProxyV1,
        })
        .validate()
        .is_err());

        let unknown = serde_json::json!({
            "kind": "via_connection",
            "parent_connection_ref": "connection:grafana",
            "route_adapter": "caller_selected_proxy"
        });
        assert!(serde_json::from_value::<ConnectionRoute>(unknown).is_err());
    }

    #[test]
    fn observations_are_value_free_and_lifecycle_consistent() {
        let observation = DiscoveryObservationSummary {
            observation_ref: "observation:grafana:prometheus".to_owned(),
            discovery_ref: "grafana-data-sources".to_owned(),
            source_connection_ref: "connection:grafana".to_owned(),
            observed_type: "prometheus".to_owned(),
            title: "Infrastructure Prometheus".to_owned(),
            state: DiscoveryObservationState::Observed,
            evidence_generation: 3,
            evidence_sha256: "b".repeat(64),
            target_provider_ref: Some("prometheus".to_owned()),
            connection_ref: None,
        };
        ResponseEnvelope::success(
            "request-1",
            ConnectionResult::ObservationSearch {
                observations: vec![observation.clone()],
            },
        )
        .validate()
        .expect("recognized observation");

        let invalid = DiscoveryObservationSummary {
            state: DiscoveryObservationState::Materialized,
            ..observation
        };
        assert!(ResponseEnvelope::success(
            "request-1",
            ConnectionResult::ObservationSearch {
                observations: vec![invalid]
            }
        )
        .validate()
        .is_err());
    }

    #[test]
    fn materialization_accepts_only_an_opaque_observation_reference() {
        let request = RequestEnvelope {
            protocol: CONTRACT.to_owned(),
            request_id: "request-materialize-1".to_owned(),
            context: context(),
            request: ConnectionRequest::Materialize(MaterializeRequest {
                observation_ref: "observation:grafana:prometheus".to_owned(),
            }),
        };
        request.validate().unwrap();

        ResponseEnvelope::success(
            "request-materialize-1",
            ConnectionResult::Materialize(ConnectionDescription {
                summary: summary(ConnectionRoute::ViaConnection {
                    parent_connection_ref: "connection:grafana".to_owned(),
                    route_adapter: RouteAdapter::GrafanaDatasourceProxyV1,
                }),
                channels: Vec::new(),
            }),
        )
        .validate()
        .unwrap();
    }
}
