//! Credential-free Connection and Connect Session protocol.

// The frozen JSON Schema is the normative field-level contract. This module is its strict Rust
// projection and deliberately has no type capable of carrying credential material.
#![allow(missing_docs)]

use serde::{Deserialize, Serialize};
use url::Url;

use crate::operation::OwnerContext;

pub const CONTRACT: &str = "b10x.connector-connection.v0alpha1";
pub const MAX_FRAME_BYTES: usize = 64 * 1024;
pub const MAX_RESPONSE_BYTES: usize = 128 * 1024;
pub const MAX_SEARCH_RESULTS: u16 = 64;

const MAX_BROWSER_CAPABILITY_TOKEN_BYTES: usize = 256;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RequestEnvelope {
    pub protocol: String,
    pub request_id: String,
    pub context: OwnerContext,
    pub request: EndpointRequest,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(
    tag = "method",
    content = "params",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum EndpointRequest {
    /// Passively enumerate local, value-free Integration candidates before any provider contact.
    CandidateSearch(CandidateSearchRequest),
    /// Explicitly activate one opaque candidate and create its direct Connection.
    CandidateActivate(CandidateActivateRequest),
    Search(SearchRequest),
    Describe(DescribeRequest),
    /// Read stored observations only. An active provider refresh remains an admitted operation.
    ObservationSearch(ObservationSearchRequest),
    /// Explicitly turn one recognized stored observation into a mediated Connection.
    Materialize(MaterializeRequest),
    ConnectSessionCreate(ConnectSessionCreateRequest),
    ConnectSessionStatus(ConnectSessionStatusRequest),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CandidateSearchRequest {
    pub integration_ref: String,
    pub query: String,
    pub limit: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CandidateActivateRequest {
    /// Opaque candidate identity. Credential-source and provider routes remain Connector-owned.
    pub candidate_ref: String,
    pub label: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SearchRequest {
    pub query: String,
    pub limit: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DescribeRequest {
    pub endpoint_ref: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ObservationSearchRequest {
    /// Wire name is the published v0alpha1/v0alpha2 field; only the Rust name migrated.
    #[serde(rename = "source_connection_ref")]
    pub source_endpoint_ref: String,
    pub query: String,
    pub limit: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct MaterializeRequest {
    /// Opaque observation identity. Resource bindings and route selection remain Connector-owned.
    pub observation_ref: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ConnectSessionCreateRequest {
    pub integration_ref: String,
    pub label: String,
    /// Provider-declared acquisition profile. It selects credential purpose, never a credential.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub auth_profile: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ConnectSessionStatusRequest {
    pub connect_session_ref: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum EndpointState {
    Created,
    Authorized,
    Callable,
    Degraded,
    Revoked,
}

/// Durable authority owner for a Connection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum EndpointScope {
    Tenant,
    Principal,
}

/// External actor whose identity the provider observes for calls through a Connection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum EndpointActor {
    App,
    User,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum EndpointInitiator {
    /// Wire id `b10x`: pinned by the connector-connection contract vectors (D5).
    #[serde(rename = "b10x")]
    Platform,
    Provider,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ChannelState {
    Starting,
    Connected,
    Reconnecting,
    Stopped,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum RouteAdapter {
    GrafanaDatasourceProxyV1,
    KubernetesServiceProxyV1,
}

/// Value-free projection of a Connection's immutable route.
///
/// The Connector-owned resource binding is deliberately absent: callers need to know that a route
/// is mediated, not the Grafana data-source UID or proxy path used to implement it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum EndpointRoute {
    Direct,
    ViaEndpoint {
        parent_endpoint_ref: String,
        route_adapter: RouteAdapter,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ChannelSummary {
    pub channel_ref: String,
    pub binding_ref: String,
    pub state: ChannelState,
    pub events: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct EndpointSummary {
    pub endpoint_ref: String,
    pub integration_ref: String,
    pub label: String,
    pub state: EndpointState,
    pub initiation: Vec<EndpointInitiator>,
    pub route: EndpointRoute,
    /// Ownership and actor metadata are value-free and must not be inferred from token prefixes.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scope: Option<EndpointScope>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub actor: Option<EndpointActor>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub auth_profile: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct EndpointDescription {
    #[serde(flatten)]
    pub summary: EndpointSummary,
    pub channels: Vec<ChannelSummary>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum DiscoveryObservationState {
    Observed,
    Unsupported,
    Materialized,
    Withdrawn,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum EndpointCandidateState {
    Detected,
    Activated,
}

/// Value-free projection of a potential direct Connection found in trusted local configuration.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct EndpointCandidateSummary {
    pub candidate_ref: String,
    pub integration_ref: String,
    pub title: String,
    pub state: EndpointCandidateState,
    pub evidence_sha256: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub endpoint_ref: Option<String>,
}

/// Value-free projection of one already-reconciled discovery observation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DiscoveryObservationSummary {
    pub observation_ref: String,
    pub discovery_ref: String,
    /// Wire name is the published v0alpha1/v0alpha2 field; only the Rust name migrated.
    #[serde(rename = "source_connection_ref")]
    pub source_endpoint_ref: String,
    pub observed_type: String,
    pub title: String,
    pub state: DiscoveryObservationState,
    pub evidence_generation: u64,
    pub evidence_sha256: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target_provider_ref: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub endpoint_ref: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ConnectSessionState {
    Pending,
    Completed,
    Expired,
    Failed,
}

/// Value-free status for one single-purpose acquisition session.
///
/// Completion endpoints are short-lived Connector-owned endpoints, not Agent Endpoints and not
/// the durable Connection. They are present only while pending and accept one completion attempt.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ConnectSessionStatus {
    pub connect_session_ref: String,
    pub integration_ref: String,
    pub state: ConnectSessionState,
    pub expires_at_unix_ms: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub completion_endpoint: Option<String>,
    /// Optional one-use browser setup page. Provider credentials post directly to Connectors.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub browser_completion_url: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub endpoint_ref: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(
    tag = "result",
    content = "value",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum EndpointResult {
    CandidateSearch {
        candidates: Vec<EndpointCandidateSummary>,
    },
    CandidateActivate(EndpointDescription),
    Search {
        endpoints: Vec<EndpointSummary>,
    },
    Describe(EndpointDescription),
    ObservationSearch {
        observations: Vec<DiscoveryObservationSummary>,
    },
    Materialize(EndpointDescription),
    ConnectSessionCreate(ConnectSessionStatus),
    ConnectSessionStatus(ConnectSessionStatus),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum EndpointErrorCode {
    Unavailable,
    NotFound,
    NotGranted,
    InvalidInput,
    StaleAuthority,
    Conflict,
    Protocol,
}

#[derive(
    Debug, Clone, PartialEq, Eq, Serialize, Deserialize, thiserror::Error, schemars::JsonSchema,
)]
#[error("{code:?}: {message}")]
#[serde(deny_unknown_fields)]
pub struct EndpointError {
    pub code: EndpointErrorCode,
    pub message: String,
    pub retriable: bool,
}

impl EndpointError {
    #[must_use]
    pub fn new(code: EndpointErrorCode, message: impl Into<String>, retriable: bool) -> Self {
        Self {
            code,
            message: message.into(),
            retriable,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ResponseStatus {
    Ok,
    Error,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ResponseEnvelope {
    pub protocol: String,
    pub request_id: String,
    pub status: ResponseStatus,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub response: Option<EndpointResult>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<EndpointError>,
}

impl RequestEnvelope {
    pub fn validate(&self) -> Result<(), EndpointError> {
        if self.protocol != CONTRACT || !valid_ref(&self.request_id, 128) {
            return Err(protocol_refusal());
        }
        validate_context(&self.context)?;
        match &self.request {
            EndpointRequest::CandidateSearch(request) => {
                require_ref(&request.integration_ref)?;
                if request.query.len() > 512
                    || request.limit == 0
                    || request.limit > MAX_SEARCH_RESULTS
                {
                    return Err(invalid_input(
                        "connection candidate search bounds are invalid",
                    ));
                }
            }
            EndpointRequest::CandidateActivate(request) => {
                require_ref(&request.candidate_ref)?;
                if request.label.trim().is_empty() || request.label.len() > 256 {
                    return Err(invalid_input("connection label is invalid"));
                }
            }
            EndpointRequest::Search(request) => {
                if request.query.len() > 512
                    || request.limit == 0
                    || request.limit > MAX_SEARCH_RESULTS
                {
                    return Err(invalid_input("connection search bounds are invalid"));
                }
            }
            EndpointRequest::Describe(request) => require_ref(&request.endpoint_ref)?,
            EndpointRequest::ObservationSearch(request) => {
                require_ref(&request.source_endpoint_ref)?;
                if request.query.len() > 512
                    || request.limit == 0
                    || request.limit > MAX_SEARCH_RESULTS
                {
                    return Err(invalid_input("observation search bounds are invalid"));
                }
            }
            EndpointRequest::Materialize(request) => require_ref(&request.observation_ref)?,
            EndpointRequest::ConnectSessionCreate(request) => {
                require_ref(&request.integration_ref)?;
                if request.label.trim().is_empty() || request.label.len() > 256 {
                    return Err(invalid_input("connection label is invalid"));
                }
                if request.auth_profile.as_deref().is_some_and(|profile| {
                    profile.is_empty()
                        || profile.len() > 128
                        || !profile.bytes().all(|byte| {
                            byte.is_ascii_lowercase()
                                || byte.is_ascii_digit()
                                || matches!(byte, b'.' | b'-' | b'_')
                        })
                }) {
                    return Err(invalid_input("connection auth profile is invalid"));
                }
            }
            EndpointRequest::ConnectSessionStatus(request) => {
                require_ref(&request.connect_session_ref)?;
            }
        }
        Ok(())
    }
}

impl ResponseEnvelope {
    #[must_use]
    pub fn success(request_id: impl Into<String>, response: EndpointResult) -> Self {
        Self {
            protocol: CONTRACT.to_owned(),
            request_id: request_id.into(),
            status: ResponseStatus::Ok,
            response: Some(response),
            error: None,
        }
    }

    #[must_use]
    pub fn failure(request_id: impl Into<String>, error: EndpointError) -> Self {
        Self {
            protocol: CONTRACT.to_owned(),
            request_id: request_id.into(),
            status: ResponseStatus::Error,
            response: None,
            error: Some(error),
        }
    }

    pub fn validate(&self) -> Result<(), EndpointError> {
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

fn validate_result(result: &EndpointResult) -> Result<(), EndpointError> {
    match result {
        EndpointResult::CandidateSearch { candidates } => {
            if candidates.len() > usize::from(MAX_SEARCH_RESULTS) {
                return Err(protocol_refusal());
            }
            for candidate in candidates {
                if !valid_ref(&candidate.candidate_ref, 512)
                    || !valid_ref(&candidate.integration_ref, 512)
                    || candidate.title.trim().is_empty()
                    || candidate.title.len() > 256
                    || candidate.evidence_sha256.len() != 64
                    || !candidate
                        .evidence_sha256
                        .bytes()
                        .all(|byte| byte.is_ascii_hexdigit())
                    || candidate
                        .endpoint_ref
                        .as_deref()
                        .is_some_and(|value| !valid_ref(value, 512))
                {
                    return Err(protocol_refusal());
                }
                match candidate.state {
                    EndpointCandidateState::Detected if candidate.endpoint_ref.is_none() => {}
                    EndpointCandidateState::Activated if candidate.endpoint_ref.is_some() => {}
                    _ => return Err(protocol_refusal()),
                }
            }
        }
        EndpointResult::CandidateActivate(description) => {
            validate_description(description)?;
        }
        EndpointResult::Search { endpoints } => {
            if endpoints.len() > usize::from(MAX_SEARCH_RESULTS) {
                return Err(protocol_refusal());
            }
            for connection in endpoints {
                validate_summary(connection)?;
            }
        }
        EndpointResult::Describe(description) | EndpointResult::Materialize(description) => {
            validate_description(description)?;
        }
        EndpointResult::ObservationSearch { observations } => {
            if observations.len() > usize::from(MAX_SEARCH_RESULTS) {
                return Err(protocol_refusal());
            }
            for observation in observations {
                validate_observation(observation)?;
            }
        }
        EndpointResult::ConnectSessionCreate(session)
        | EndpointResult::ConnectSessionStatus(session) => {
            require_ref(&session.connect_session_ref)?;
            require_ref(&session.integration_ref)?;
            if session
                .completion_endpoint
                .as_deref()
                .is_some_and(|value| value.is_empty() || value.len() > 4096 || value.contains('\n'))
                || session
                    .browser_completion_url
                    .as_deref()
                    .is_some_and(|value| !valid_browser_completion_url(value))
                || session
                    .endpoint_ref
                    .as_deref()
                    .is_some_and(|value| !valid_ref(value, 512))
            {
                return Err(protocol_refusal());
            }
            match session.state {
                ConnectSessionState::Pending
                    if (session.completion_endpoint.is_some()
                        || session.browser_completion_url.is_some())
                        && session.endpoint_ref.is_none() => {}
                ConnectSessionState::Completed
                    if session.completion_endpoint.is_none()
                        && session.browser_completion_url.is_none()
                        && session.endpoint_ref.is_some() => {}
                ConnectSessionState::Expired | ConnectSessionState::Failed
                    if session.completion_endpoint.is_none()
                        && session.browser_completion_url.is_none()
                        && session.endpoint_ref.is_none() => {}
                _ => return Err(protocol_refusal()),
            }
        }
    }
    Ok(())
}

fn validate_description(description: &EndpointDescription) -> Result<(), EndpointError> {
    validate_summary(&description.summary)?;
    if description.channels.len() > 64 {
        return Err(protocol_refusal());
    }
    for channel in &description.channels {
        require_ref(&channel.channel_ref)?;
        require_ref(&channel.binding_ref)?;
        if channel.events.len() > 64 || channel.events.iter().any(|event| !valid_ref(event, 512)) {
            return Err(protocol_refusal());
        }
    }
    Ok(())
}

fn validate_observation(observation: &DiscoveryObservationSummary) -> Result<(), EndpointError> {
    if !valid_ref(&observation.observation_ref, 512)
        || !valid_ref(&observation.discovery_ref, 512)
        || !valid_ref(&observation.source_endpoint_ref, 512)
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
            .endpoint_ref
            .as_deref()
            .is_some_and(|value| !valid_ref(value, 512))
    {
        return Err(protocol_refusal());
    }
    match observation.state {
        DiscoveryObservationState::Observed
            if observation.target_provider_ref.is_some()
                && observation.endpoint_ref.is_none() => {}
        DiscoveryObservationState::Unsupported
            if observation.target_provider_ref.is_none()
                && observation.endpoint_ref.is_none() => {}
        DiscoveryObservationState::Materialized
            if observation.target_provider_ref.is_some()
                && observation.endpoint_ref.is_some() => {}
        DiscoveryObservationState::Withdrawn if observation.endpoint_ref.is_none() => {}
        _ => return Err(protocol_refusal()),
    }
    Ok(())
}

fn validate_summary(summary: &EndpointSummary) -> Result<(), EndpointError> {
    if !valid_ref(&summary.endpoint_ref, 512)
        || !valid_ref(&summary.integration_ref, 512)
        || summary.label.trim().is_empty()
        || summary.label.len() > 256
        || summary.initiation.is_empty()
        || summary.initiation.len() > 2
        || summary.auth_profile.as_deref().is_some_and(|profile| {
            profile.is_empty()
                || profile.len() > 128
                || !profile.bytes().all(|byte| {
                    byte.is_ascii_lowercase()
                        || byte.is_ascii_digit()
                        || matches!(byte, b'.' | b'-' | b'_')
                })
        })
        || summary.scope.is_some() != summary.actor.is_some()
    {
        return Err(protocol_refusal());
    }
    match &summary.route {
        EndpointRoute::Direct => {}
        EndpointRoute::ViaEndpoint {
            parent_endpoint_ref,
            ..
        } if valid_ref(parent_endpoint_ref, 512)
            && parent_endpoint_ref != &summary.endpoint_ref => {}
        EndpointRoute::ViaEndpoint { .. } => return Err(protocol_refusal()),
    }
    Ok(())
}

fn validate_context(context: &OwnerContext) -> Result<(), EndpointError> {
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

fn require_ref(value: &str) -> Result<(), EndpointError> {
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

fn valid_browser_completion_url(value: &str) -> bool {
    if value.is_empty() || value.len() > 4096 || value.contains(['\r', '\n']) {
        return false;
    }
    let Ok(url) = Url::parse(value) else {
        return false;
    };
    let Some(token) = url
        .fragment()
        .and_then(|fragment| fragment.strip_prefix("token="))
    else {
        return false;
    };
    let local = url.scheme() == "http"
        && url.host_str() == Some("127.0.0.1")
        && url.port().is_some_and(|port| port != 0)
        && url.path() == "/";
    let hosted = url.scheme() == "https"
        && url.host_str().is_some()
        && url.path().contains("/connect-sessions/")
        && !url.path().ends_with('/');
    (local || hosted)
        && url.username().is_empty()
        && url.password().is_none()
        && url.query().is_none()
        && !token.is_empty()
        && token.len() <= MAX_BROWSER_CAPABILITY_TOKEN_BYTES
        && token
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
}

fn invalid_input(message: &'static str) -> EndpointError {
    EndpointError::new(EndpointErrorCode::InvalidInput, message, false)
}

fn protocol_refusal() -> EndpointError {
    EndpointError::new(
        EndpointErrorCode::Protocol,
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
            browser_completion_url: Some(
                "http://127.0.0.1:41000/#token=opaque-capability".to_owned(),
            ),
            endpoint_ref: None,
        };
        ResponseEnvelope::success(
            "request-1",
            EndpointResult::ConnectSessionCreate(pending.clone()),
        )
        .validate()
        .unwrap();
        let mut invalid = pending;
        invalid.endpoint_ref = Some("connection:1".to_owned());
        assert!(ResponseEnvelope::success(
            "request-1",
            EndpointResult::ConnectSessionCreate(invalid)
        )
        .validate()
        .is_err());
    }

    #[test]
    fn browser_completion_url_is_an_exact_loopback_capability() {
        assert!(valid_browser_completion_url(
            "http://127.0.0.1:41000/#token=opaque-capability_1"
        ));
        for invalid in [
            "https://127.0.0.1:41000/#token=opaque-capability",
            "http://localhost:41000/#token=opaque-capability",
            "http://127.0.0.1/#token=opaque-capability",
            "http://127.0.0.1:0/#token=opaque-capability",
            "http://127.0.0.1:80@evil.example/#token=opaque-capability",
            "http://127.0.0.1:41000/complete#token=opaque-capability",
            "http://127.0.0.1:41000/?next=evil#token=opaque-capability",
            "http://127.0.0.1:41000/#token=",
            "http://127.0.0.1:41000/#token=not+padded=",
            "http://127.0.0.1:41000/#token=opaque&next=evil",
            "http://127.0.0.1:41000/#other=opaque-capability",
        ] {
            assert!(!valid_browser_completion_url(invalid), "accepted {invalid}");
        }
        let oversized = format!(
            "http://127.0.0.1:41000/#token={}",
            "a".repeat(MAX_BROWSER_CAPABILITY_TOKEN_BYTES + 1)
        );
        assert!(!valid_browser_completion_url(&oversized));
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

    fn summary(route: EndpointRoute) -> EndpointSummary {
        EndpointSummary {
            endpoint_ref: "connection:prometheus".to_owned(),
            integration_ref: "integration:prometheus".to_owned(),
            label: "Prometheus via Grafana".to_owned(),
            state: EndpointState::Callable,
            initiation: vec![EndpointInitiator::Platform],
            route,
            scope: None,
            actor: None,
            auth_profile: None,
        }
    }

    #[test]
    fn mediated_route_is_value_free_closed_and_cannot_self_reference() {
        let response = |route| {
            ResponseEnvelope::success(
                "request-1",
                EndpointResult::Search {
                    endpoints: vec![summary(route)],
                },
            )
        };
        response(EndpointRoute::ViaEndpoint {
            parent_endpoint_ref: "connection:grafana".to_owned(),
            route_adapter: RouteAdapter::GrafanaDatasourceProxyV1,
        })
        .validate()
        .expect("closed mediated route");

        assert!(response(EndpointRoute::ViaEndpoint {
            parent_endpoint_ref: "connection:prometheus".to_owned(),
            route_adapter: RouteAdapter::GrafanaDatasourceProxyV1,
        })
        .validate()
        .is_err());

        let unknown = serde_json::json!({
            "kind": "via_connection",
            "parent_endpoint_ref": "connection:grafana",
            "route_adapter": "caller_selected_proxy"
        });
        assert!(serde_json::from_value::<EndpointRoute>(unknown).is_err());
    }

    #[test]
    fn observations_are_value_free_and_lifecycle_consistent() {
        let observation = DiscoveryObservationSummary {
            observation_ref: "observation:grafana:prometheus".to_owned(),
            discovery_ref: "grafana-data-sources".to_owned(),
            source_endpoint_ref: "connection:grafana".to_owned(),
            observed_type: "prometheus".to_owned(),
            title: "Infrastructure Prometheus".to_owned(),
            state: DiscoveryObservationState::Observed,
            evidence_generation: 3,
            evidence_sha256: "b".repeat(64),
            target_provider_ref: Some("prometheus".to_owned()),
            endpoint_ref: None,
        };
        ResponseEnvelope::success(
            "request-1",
            EndpointResult::ObservationSearch {
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
            EndpointResult::ObservationSearch {
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
            request: EndpointRequest::Materialize(MaterializeRequest {
                observation_ref: "observation:grafana:prometheus".to_owned(),
            }),
        };
        request.validate().unwrap();

        ResponseEnvelope::success(
            "request-materialize-1",
            EndpointResult::Materialize(EndpointDescription {
                summary: summary(EndpointRoute::ViaEndpoint {
                    parent_endpoint_ref: "connection:grafana".to_owned(),
                    route_adapter: RouteAdapter::GrafanaDatasourceProxyV1,
                }),
                channels: Vec::new(),
            }),
        )
        .validate()
        .unwrap();
    }

    #[test]
    fn candidates_are_value_free_and_activation_selects_no_route() {
        let search = RequestEnvelope {
            protocol: CONTRACT.to_owned(),
            request_id: "candidate-search-1".to_owned(),
            context: context(),
            request: EndpointRequest::CandidateSearch(CandidateSearchRequest {
                integration_ref: "kubernetes".to_owned(),
                query: "dev".to_owned(),
                limit: 32,
            }),
        };
        search.validate().unwrap();
        let response = ResponseEnvelope::success(
            "candidate-search-1",
            EndpointResult::CandidateSearch {
                candidates: vec![EndpointCandidateSummary {
                    candidate_ref: "candidate:kubernetes:opaque".to_owned(),
                    integration_ref: "kubernetes".to_owned(),
                    title: "development".to_owned(),
                    state: EndpointCandidateState::Detected,
                    evidence_sha256: "a".repeat(64),
                    endpoint_ref: None,
                }],
            },
        );
        response.validate().unwrap();
        let encoded = serde_json::to_string(&response).unwrap();
        assert!(!encoded.contains("kubeconfig"));
        assert!(!encoded.contains("server_url"));
        assert!(!encoded.contains("credential"));

        let activation: Result<RequestEnvelope, _> = serde_json::from_value(serde_json::json!({
            "protocol": CONTRACT,
            "request_id": "candidate-activate-bad",
            "context": context(),
            "request": {
                "method": "candidate_activate",
                "params": {
                    "candidate_ref": "candidate:kubernetes:opaque",
                    "label": "development",
                    "server_url": "https://cluster.example"
                }
            }
        }));
        assert!(activation.is_err());
    }
}
