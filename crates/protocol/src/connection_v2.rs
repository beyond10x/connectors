//! Additive trusted bound remediation contracts and pure v1/v2 adapters.
// Field-level documentation is generated into the new bundle; predecessor artifacts stay frozen.
#![allow(missing_docs)]
use crate::{
    connection as old,
    operation::{v3::AuthenticationNeed, OwnerContext},
};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use serde_json::Value;

pub use old::{
    CandidateActivateRequest, CandidateSearchRequest, ConnectSessionCreateRequest,
    ConnectSessionState, ConnectSessionStatus, ConnectSessionStatusRequest,
    ConnectionCandidateSummary, ConnectionDescription, ConnectionError, ConnectionErrorCode,
    ConnectionSummary, DescribeRequest, DiscoveryObservationSummary, MaterializeRequest,
    ObservationSearchRequest, ResponseStatus, SearchRequest, MAX_RESPONSE_BYTES,
    MAX_SEARCH_RESULTS,
};
pub const CONTRACT: &str = "b10x.connector-connection.v0alpha2";
pub const MAX_FRAME_BYTES: usize = 128 * 1024;
pub const MAX_INPUT_BYTES: usize = crate::operation::MAX_ARGUMENT_BYTES;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RequestEnvelope {
    pub protocol: String,
    pub request_id: String,
    pub context: OwnerContext,
    pub request: ConnectionRequest,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(
    tag = "method",
    content = "params",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum ConnectionRequest {
    CandidateSearch(CandidateSearchRequest),
    CandidateActivate(CandidateActivateRequest),
    Search(SearchRequest),
    Describe(DescribeRequest),
    ObservationSearch(ObservationSearchRequest),
    Materialize(MaterializeRequest),
    ConnectSessionCreate(ConnectSessionCreateRequest),
    ConnectSessionStatus(ConnectSessionStatusRequest),
    RemediationStart(RemediationStartRequest),
    RemediationStatus(RemediationStatusRequest),
    RemediationAcknowledge(RemediationAcknowledgeRequest),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RemediationStartRequest {
    pub operation_ref: String,
    pub connection_ref: String,
    pub input: Value,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RemediationStatusRequest {
    pub connect_session_ref: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RemediationAcknowledgeRequest {
    pub connect_session_ref: String,
    pub operation_ref: String,
    pub connection_ref: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum RemediationResumeState {
    Pending,
    Ready,
    Consumed,
    Expired,
    Failed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum RemediationNextAction {
    FreshDescriptionThenExplicitInvoke,
}

/// Trusted session projection. Never serialize it directly into a model/tool result.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct BoundRemediationStatus {
    pub connect_session_ref: String,
    pub operation_ref: String,
    pub connection_ref: String,
    pub integration_ref: String,
    pub auth_profile: String,
    pub need: AuthenticationNeed,
    pub session_state: ConnectSessionState,
    pub expires_at_unix_ms: u64,
    pub resume_state: RemediationResumeState,
    pub session: ConnectSessionStatus,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RemediationAcknowledgement {
    pub connect_session_ref: String,
    pub operation_ref: String,
    pub connection_ref: String,
    pub next_action: RemediationNextAction,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(
    tag = "result",
    content = "value",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum ConnectionResult {
    CandidateSearch {
        candidates: Vec<ConnectionCandidateSummary>,
    },
    CandidateActivate(ConnectionDescription),
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
    RemediationStart(BoundRemediationStatus),
    RemediationStatus(BoundRemediationStatus),
    RemediationAcknowledge(RemediationAcknowledgement),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
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
        if self.protocol != CONTRACT {
            return Err(protocol_refusal());
        }
        // The inherited reader owns all common context and ordinary command semantics.
        old::RequestEnvelope {
            protocol: old::CONTRACT.into(),
            request_id: self.request_id.clone(),
            context: self.context.clone(),
            request: old::ConnectionRequest::Search(SearchRequest {
                query: String::new(),
                limit: 1,
            }),
        }
        .validate()?;
        match &self.request {
            ConnectionRequest::RemediationStart(value) => {
                require_operation_ref(&value.operation_ref)?;
                require_ref(&value.connection_ref)?;
                if serde_json::to_vec(&value.input)
                    .map_or(true, |bytes| bytes.len() > MAX_INPUT_BYTES)
                {
                    return Err(protocol_refusal());
                }
            }
            ConnectionRequest::RemediationStatus(value) => require_ref(&value.connect_session_ref)?,
            ConnectionRequest::RemediationAcknowledge(value) => {
                require_ref(&value.connect_session_ref)?;
                require_operation_ref(&value.operation_ref)?;
                require_ref(&value.connection_ref)?;
            }
            _ => self.clone().into_v1()?.validate()?,
        }
        if serde_json::to_vec(self).map_or(true, |bytes| bytes.len() > MAX_FRAME_BYTES) {
            return Err(protocol_refusal());
        }
        Ok(())
    }

    /// Bound commands cannot be expressed by unbound v1 session creation.
    pub fn into_v1(self) -> Result<old::RequestEnvelope, ConnectionError> {
        Ok(old::RequestEnvelope {
            protocol: old::CONTRACT.into(),
            request_id: self.request_id,
            context: self.context,
            request: self.request.into_v1()?,
        })
    }
}

impl ConnectionRequest {
    fn into_v1(self) -> Result<old::ConnectionRequest, ConnectionError> {
        Ok(match self {
            Self::CandidateSearch(v) => old::ConnectionRequest::CandidateSearch(v),
            Self::CandidateActivate(v) => old::ConnectionRequest::CandidateActivate(v),
            Self::Search(v) => old::ConnectionRequest::Search(v),
            Self::Describe(v) => old::ConnectionRequest::Describe(v),
            Self::ObservationSearch(v) => old::ConnectionRequest::ObservationSearch(v),
            Self::Materialize(v) => old::ConnectionRequest::Materialize(v),
            Self::ConnectSessionCreate(v) => old::ConnectionRequest::ConnectSessionCreate(v),
            Self::ConnectSessionStatus(v) => old::ConnectionRequest::ConnectSessionStatus(v),
            Self::RemediationStart(_)
            | Self::RemediationStatus(_)
            | Self::RemediationAcknowledge(_) => return Err(downgrade_refusal()),
        })
    }
}
impl From<old::ConnectionRequest> for ConnectionRequest {
    fn from(value: old::ConnectionRequest) -> Self {
        match value {
            old::ConnectionRequest::CandidateSearch(v) => Self::CandidateSearch(v),
            old::ConnectionRequest::CandidateActivate(v) => Self::CandidateActivate(v),
            old::ConnectionRequest::Search(v) => Self::Search(v),
            old::ConnectionRequest::Describe(v) => Self::Describe(v),
            old::ConnectionRequest::ObservationSearch(v) => Self::ObservationSearch(v),
            old::ConnectionRequest::Materialize(v) => Self::Materialize(v),
            old::ConnectionRequest::ConnectSessionCreate(v) => Self::ConnectSessionCreate(v),
            old::ConnectionRequest::ConnectSessionStatus(v) => Self::ConnectSessionStatus(v),
        }
    }
}
impl From<old::RequestEnvelope> for RequestEnvelope {
    fn from(value: old::RequestEnvelope) -> Self {
        Self {
            protocol: CONTRACT.into(),
            request_id: value.request_id,
            context: value.context,
            request: value.request.into(),
        }
    }
}

impl BoundRemediationStatus {
    /// Validate projection structure and equality only, not current time, admission or readiness.
    pub fn validate(&self) -> Result<(), ConnectionError> {
        for value in [
            &self.connect_session_ref,
            &self.connection_ref,
            &self.integration_ref,
        ] {
            require_ref(value)?;
        }
        require_operation_ref(&self.operation_ref)?;
        if !valid_profile(&self.auth_profile)
            || self.connect_session_ref != self.session.connect_session_ref
            || self.integration_ref != self.session.integration_ref
            || self.session_state != self.session.state
            || self.expires_at_unix_ms != self.session.expires_at_unix_ms
            || self
                .session
                .connection_ref
                .as_ref()
                .is_some_and(|value| value != &self.connection_ref)
        {
            return Err(protocol_refusal());
        }
        old::ResponseEnvelope::success(
            "bound-status-validation",
            old::ConnectionResult::ConnectSessionStatus(self.session.clone()),
        )
        .validate()?;
        let coherent = match self.resume_state {
            RemediationResumeState::Pending => self.session_state == ConnectSessionState::Pending,
            RemediationResumeState::Ready | RemediationResumeState::Consumed => {
                self.session_state == ConnectSessionState::Completed
            }
            // Publication may complete before the resume deadline expires.
            RemediationResumeState::Expired => matches!(
                self.session_state,
                ConnectSessionState::Completed | ConnectSessionState::Expired
            ),
            RemediationResumeState::Failed => self.session_state == ConnectSessionState::Failed,
        };
        if !coherent {
            return Err(protocol_refusal());
        }
        Ok(())
    }
}
impl RemediationAcknowledgement {
    pub fn validate(&self) -> Result<(), ConnectionError> {
        require_ref(&self.connect_session_ref)?;
        require_operation_ref(&self.operation_ref)?;
        require_ref(&self.connection_ref)
    }
}

impl ResponseEnvelope {
    #[must_use]
    pub fn success(request_id: impl Into<String>, response: ConnectionResult) -> Self {
        Self {
            protocol: CONTRACT.into(),
            request_id: request_id.into(),
            status: ResponseStatus::Ok,
            response: Some(response),
            error: None,
        }
    }
    #[must_use]
    pub fn failure(request_id: impl Into<String>, error: ConnectionError) -> Self {
        Self {
            protocol: CONTRACT.into(),
            request_id: request_id.into(),
            status: ResponseStatus::Error,
            response: None,
            error: Some(error),
        }
    }
    pub fn validate(&self) -> Result<(), ConnectionError> {
        if self.protocol != CONTRACT {
            return Err(protocol_refusal());
        }
        match (self.status, &self.response, &self.error) {
            (
                ResponseStatus::Ok,
                Some(
                    ConnectionResult::RemediationStart(value)
                    | ConnectionResult::RemediationStatus(value),
                ),
                None,
            ) => {
                value.validate()?;
                if matches!(&self.response, Some(ConnectionResult::RemediationStart(_))) {
                    // Starting acquisition must deliver a usable completion route. Later status
                    // polls can omit the one-use capability without invalidating Pending.
                    old::ResponseEnvelope::success(
                        self.request_id.clone(),
                        old::ConnectionResult::ConnectSessionCreate(value.session.clone()),
                    )
                    .validate()?;
                }
                old::ResponseEnvelope::success(
                    self.request_id.clone(),
                    old::ConnectionResult::Search {
                        connections: Vec::new(),
                    },
                )
                .validate()?;
            }
            (ResponseStatus::Ok, Some(ConnectionResult::RemediationAcknowledge(value)), None) => {
                value.validate()?;
                old::ResponseEnvelope::success(
                    self.request_id.clone(),
                    old::ConnectionResult::Search {
                        connections: Vec::new(),
                    },
                )
                .validate()?;
            }
            _ => self.clone().into_v1()?.validate()?,
        }
        if serde_json::to_vec(self).map_or(true, |bytes| bytes.len() > MAX_RESPONSE_BYTES) {
            return Err(protocol_refusal());
        }
        Ok(())
    }
    pub fn into_v1(self) -> Result<old::ResponseEnvelope, ConnectionError> {
        Ok(old::ResponseEnvelope {
            protocol: old::CONTRACT.into(),
            request_id: self.request_id,
            status: self.status,
            response: self.response.map(ConnectionResult::into_v1).transpose()?,
            error: self.error,
        })
    }
}

impl ConnectionResult {
    fn into_v1(self) -> Result<old::ConnectionResult, ConnectionError> {
        Ok(match self {
            Self::CandidateSearch { candidates } => {
                old::ConnectionResult::CandidateSearch { candidates }
            }
            Self::CandidateActivate(v) => old::ConnectionResult::CandidateActivate(v),
            Self::Search { connections } => old::ConnectionResult::Search { connections },
            Self::Describe(v) => old::ConnectionResult::Describe(v),
            Self::ObservationSearch { observations } => {
                old::ConnectionResult::ObservationSearch { observations }
            }
            Self::Materialize(v) => old::ConnectionResult::Materialize(v),
            Self::ConnectSessionCreate(v) => old::ConnectionResult::ConnectSessionCreate(v),
            Self::ConnectSessionStatus(v) => old::ConnectionResult::ConnectSessionStatus(v),
            Self::RemediationStart(_)
            | Self::RemediationStatus(_)
            | Self::RemediationAcknowledge(_) => return Err(downgrade_refusal()),
        })
    }
}
impl From<old::ConnectionResult> for ConnectionResult {
    fn from(value: old::ConnectionResult) -> Self {
        match value {
            old::ConnectionResult::CandidateSearch { candidates } => {
                Self::CandidateSearch { candidates }
            }
            old::ConnectionResult::CandidateActivate(v) => Self::CandidateActivate(v),
            old::ConnectionResult::Search { connections } => Self::Search { connections },
            old::ConnectionResult::Describe(v) => Self::Describe(v),
            old::ConnectionResult::ObservationSearch { observations } => {
                Self::ObservationSearch { observations }
            }
            old::ConnectionResult::Materialize(v) => Self::Materialize(v),
            old::ConnectionResult::ConnectSessionCreate(v) => Self::ConnectSessionCreate(v),
            old::ConnectionResult::ConnectSessionStatus(v) => Self::ConnectSessionStatus(v),
        }
    }
}
impl From<old::ResponseEnvelope> for ResponseEnvelope {
    fn from(value: old::ResponseEnvelope) -> Self {
        Self {
            protocol: CONTRACT.into(),
            request_id: value.request_id,
            status: value.status,
            response: value.response.map(Into::into),
            error: value.error,
        }
    }
}

fn require_ref(value: &str) -> Result<(), ConnectionError> {
    if value.is_empty()
        || value.len() > 512
        || value.bytes().any(|b| b.is_ascii_control() || b == b' ')
    {
        Err(protocol_refusal())
    } else {
        Ok(())
    }
}
fn require_operation_ref(value: &str) -> Result<(), ConnectionError> {
    if value.is_empty() || value.len() > 512 || !value.bytes().all(|b| b.is_ascii_graphic()) {
        Err(protocol_refusal())
    } else {
        Ok(())
    }
}
fn valid_profile(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value.bytes().all(|b| {
            b.is_ascii_lowercase() || b.is_ascii_digit() || matches!(b, b'.' | b'_' | b'-')
        })
}
fn protocol_refusal() -> ConnectionError {
    ConnectionError::new(
        ConnectionErrorCode::Protocol,
        "connection protocol identity or framing is invalid",
        false,
    )
}
fn downgrade_refusal() -> ConnectionError {
    ConnectionError::new(
        ConnectionErrorCode::Protocol,
        "bound remediation requires ConnectorConnection v0alpha2",
        false,
    )
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Version {
    V0Alpha1,
    V0Alpha2,
}
#[derive(Deserialize)]
struct Identity {
    protocol: String,
}
fn read<T: DeserializeOwned>(bytes: &[u8]) -> Result<T, ConnectionError> {
    serde_json::from_slice(bytes).map_err(|_| protocol_refusal())
}
fn selected(bytes: &[u8], maximum: usize) -> Result<Version, ConnectionError> {
    if bytes.len() > maximum {
        return Err(protocol_refusal());
    }
    let identity: Identity = read(bytes)?;
    match identity.protocol.as_str() {
        old::CONTRACT => Ok(Version::V0Alpha1),
        CONTRACT => Ok(Version::V0Alpha2),
        _ => Err(protocol_refusal()),
    }
}
fn write<T: Serialize>(value: &T, maximum: usize) -> Result<Vec<u8>, ConnectionError> {
    let bytes = serde_json::to_vec(value).map_err(|_| protocol_refusal())?;
    if bytes.len() > maximum {
        return Err(protocol_refusal());
    }
    Ok(bytes)
}
pub fn decode_request(bytes: &[u8]) -> Result<(Version, RequestEnvelope), ConnectionError> {
    let version = selected(bytes, MAX_FRAME_BYTES)?;
    let request = match version {
        Version::V0Alpha1 => {
            if bytes.len() > old::MAX_FRAME_BYTES {
                return Err(protocol_refusal());
            }
            let request: old::RequestEnvelope = read(bytes)?;
            request.validate()?;
            request.into()
        }
        Version::V0Alpha2 => {
            let request: RequestEnvelope = read(bytes)?;
            request.validate()?;
            request
        }
    };
    Ok((version, request))
}
pub fn decode_response(bytes: &[u8]) -> Result<(Version, ResponseEnvelope), ConnectionError> {
    let version = selected(bytes, MAX_RESPONSE_BYTES)?;
    let response = match version {
        Version::V0Alpha1 => {
            let response: old::ResponseEnvelope = read(bytes)?;
            response.validate()?;
            response.into()
        }
        Version::V0Alpha2 => {
            let response: ResponseEnvelope = read(bytes)?;
            response.validate()?;
            response
        }
    };
    Ok((version, response))
}
impl Version {
    pub fn encode_request(self, request: RequestEnvelope) -> Result<Vec<u8>, ConnectionError> {
        request.validate()?;
        match self {
            Self::V0Alpha1 => write(&request.into_v1()?, old::MAX_FRAME_BYTES),
            Self::V0Alpha2 => write(&request, MAX_FRAME_BYTES),
        }
    }
    pub fn encode_response(self, response: ResponseEnvelope) -> Result<Vec<u8>, ConnectionError> {
        response.validate()?;
        match self {
            Self::V0Alpha1 => write(&response.into_v1()?, MAX_RESPONSE_BYTES),
            Self::V0Alpha2 => write(&response, MAX_RESPONSE_BYTES),
        }
    }
}
