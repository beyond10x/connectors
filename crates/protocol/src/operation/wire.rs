//! ConnectorOperation v0alpha2 and explicit loss-aware v0alpha1 transport adapters.
#![allow(missing_docs)]

use super::legacy;
pub use legacy::{
    ApprovalPosture, ChannelSignal, ConnectionSummary, DescribeRequest, EffectClass,
    InvocationResult, InvokeRequest, OperationRequest, OperationSummary, OwnerContext,
    RequestedSessionTermination, ResponseStatus, SearchRequest, SessionRequest,
    SessionSignalRequest, SessionState, SessionStatus, SessionTerminateRequest, SessionTermination,
    MAX_ARGUMENT_BYTES, MAX_CONNECTION_AUDIENCES, MAX_FRAME_BYTES, MAX_RESULT_BYTES,
    MAX_SEARCH_RESULTS,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;

pub const CONTRACT: &str = "b10x.connector-operation.v0alpha2";
pub const MAX_RATE_ALTERNATIVES: usize = 16;
pub const MAX_APPLICABILITY_CHARS: usize = 4096;
pub const MAX_SOURCE_URL_CHARS: usize = 2048;
pub const MAX_RATE_BUCKET_CHARS: usize = 512;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum RateLimitBasis {
    MinimumAllowance,
    Ceiling,
}

/// The existing fixed declaration shape, without extending its fields.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct FixedRateLimit {
    pub requests: u32,
    pub per_seconds: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bucket: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PublishedRate {
    pub requests: u32,
    pub per_seconds: u32,
    pub basis: RateLimitBasis,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ConditionalRateLimit {
    pub applies_when: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rate: Option<PublishedRate>,
    pub source_url: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ConditionalRateAdvice {
    pub declaration: ConditionalRateLimit,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub suggested_interval_ms: Option<u64>,
}

impl ConditionalRateAdvice {
    #[must_use]
    pub fn new(declaration: ConditionalRateLimit) -> Self {
        let suggested_interval_ms = declaration
            .rate
            .as_ref()
            .and_then(|rate| suggested_interval_ms(rate.requests, rate.per_seconds));
        Self {
            declaration,
            suggested_interval_ms,
        }
    }
}

/// Advisory spacing only: absence establishes no numeric rate and never means unlimited.
#[must_use]
pub fn suggested_interval_ms(requests: u32, per_seconds: u32) -> Option<u64> {
    if requests == 0 || per_seconds == 0 {
        return None;
    }
    Some((u64::from(per_seconds) * 1000).div_ceil(u64::from(requests)))
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct OperationRateAdvice {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fixed: Option<FixedRateLimit>,
    pub alternatives: Vec<ConditionalRateAdvice>,
}

impl OperationRateAdvice {
    pub fn validate(&self) -> Result<(), OperationError> {
        if self.alternatives.len() > MAX_RATE_ALTERNATIVES
            || (self.fixed.is_none() && self.alternatives.is_empty())
        {
            return Err(protocol_refusal());
        }
        if let Some(fixed) = &self.fixed {
            if suggested_interval_ms(fixed.requests, fixed.per_seconds).is_none()
                || fixed
                    .bucket
                    .as_deref()
                    .is_some_and(|bucket| !bounded_text(bucket, MAX_RATE_BUCKET_CHARS))
            {
                return Err(protocol_refusal());
            }
        }
        for alternative in &self.alternatives {
            let declaration = &alternative.declaration;
            if !bounded_text(&declaration.applies_when, MAX_APPLICABILITY_CHARS)
                || !bounded_text(&declaration.source_url, MAX_SOURCE_URL_CHARS)
                || !valid_source_url(&declaration.source_url)
            {
                return Err(protocol_refusal());
            }
            let expected = match &declaration.rate {
                Some(rate) => Some(
                    suggested_interval_ms(rate.requests, rate.per_seconds)
                        .ok_or_else(protocol_refusal)?,
                ),
                None => None,
            };
            if alternative.suggested_interval_ms != expected {
                return Err(protocol_refusal());
            }
        }
        Ok(())
    }
}

fn bounded_text(value: &str, maximum: usize) -> bool {
    !value.is_empty() && value.chars().count() <= maximum && !value.chars().any(char::is_control)
}
fn valid_source_url(value: &str) -> bool {
    fluent_uri::Uri::parse(value).is_ok_and(|uri| {
        uri.scheme().as_str() == "https"
            && uri.fragment().is_none()
            && uri.authority().is_some_and(|authority| {
                !authority.host().is_empty()
                    && authority.userinfo().is_none()
                    && authority.port_to_u16().is_ok()
            })
    })
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RequestEnvelope {
    pub protocol: String,
    pub request_id: String,
    pub context: OwnerContext,
    pub request: OperationRequest,
}
impl RequestEnvelope {
    pub fn validate(&self) -> Result<(), OperationError> {
        if self.protocol != CONTRACT {
            return Err(protocol_refusal());
        }
        self.clone().into_legacy().validate().map_err(Into::into)
    }
    /// Requests lose no semantics; only the explicitly selected contract identity changes.
    #[must_use]
    pub fn into_legacy(self) -> legacy::RequestEnvelope {
        legacy::RequestEnvelope {
            protocol: legacy::CONTRACT.to_owned(),
            request_id: self.request_id,
            context: self.context,
            request: self.request,
        }
    }
}
impl From<legacy::RequestEnvelope> for RequestEnvelope {
    fn from(value: legacy::RequestEnvelope) -> Self {
        Self {
            protocol: CONTRACT.to_owned(),
            request_id: value.request_id,
            context: value.context,
            request: value.request,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct OperationDescription {
    pub operation_ref: String,
    pub title: String,
    pub description: String,
    pub input_schema: Value,
    pub output_schema: Value,
    pub effect: EffectClass,
    pub approval: ApprovalPosture,
    pub connections: Vec<ConnectionSummary>,
    pub description_ref: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rate_advice: Option<OperationRateAdvice>,
}
impl OperationDescription {
    /// V1 loses rate advice, preserving vendor schemas and deployed description fields.
    #[must_use]
    pub fn into_legacy(self) -> legacy::OperationDescription {
        legacy::OperationDescription {
            operation_ref: self.operation_ref,
            title: self.title,
            description: self.description,
            input_schema: self.input_schema,
            output_schema: self.output_schema,
            effect: self.effect,
            approval: self.approval,
            connections: self.connections,
            description_ref: self.description_ref,
        }
    }
}
impl From<legacy::OperationDescription> for OperationDescription {
    fn from(value: legacy::OperationDescription) -> Self {
        Self {
            operation_ref: value.operation_ref,
            title: value.title,
            description: value.description,
            input_schema: value.input_schema,
            output_schema: value.output_schema,
            effect: value.effect,
            approval: value.approval,
            connections: value.connections,
            description_ref: value.description_ref,
            rate_advice: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(
    tag = "result",
    content = "value",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum OperationResult {
    Search { operations: Vec<OperationSummary> },
    Describe(OperationDescription),
    Invoke(InvocationResult),
    SessionStatus(SessionStatus),
    SessionTerminate(SessionStatus),
    SessionReconcile(SessionStatus),
    SessionSignal(SessionStatus),
}
impl OperationResult {
    #[must_use]
    pub fn into_legacy(self) -> legacy::OperationResult {
        match self {
            Self::Search { operations } => legacy::OperationResult::Search { operations },
            Self::Describe(value) => legacy::OperationResult::Describe(value.into_legacy()),
            Self::Invoke(value) => legacy::OperationResult::Invoke(value),
            Self::SessionStatus(value) => legacy::OperationResult::SessionStatus(value),
            Self::SessionTerminate(value) => legacy::OperationResult::SessionTerminate(value),
            Self::SessionReconcile(value) => legacy::OperationResult::SessionReconcile(value),
            Self::SessionSignal(value) => legacy::OperationResult::SessionSignal(value),
        }
    }
}
impl From<legacy::OperationResult> for OperationResult {
    fn from(value: legacy::OperationResult) -> Self {
        match value {
            legacy::OperationResult::Search { operations } => Self::Search { operations },
            legacy::OperationResult::Describe(value) => Self::Describe(value.into()),
            legacy::OperationResult::Invoke(value) => Self::Invoke(value),
            legacy::OperationResult::SessionStatus(value) => Self::SessionStatus(value),
            legacy::OperationResult::SessionTerminate(value) => Self::SessionTerminate(value),
            legacy::OperationResult::SessionReconcile(value) => Self::SessionReconcile(value),
            legacy::OperationResult::SessionSignal(value) => Self::SessionSignal(value),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum OperationErrorCode {
    Unavailable,
    NotFound,
    NotGranted,
    InvalidInput,
    StaleAuthority,
    ApprovalRequired,
    ApprovalDenied,
    ResultTooLarge,
    Protocol,
    OutcomeUnknown,
    RateLimited,
}
#[derive(
    Debug, Clone, PartialEq, Eq, Serialize, Deserialize, thiserror::Error, schemars::JsonSchema,
)]
#[error("{code:?}: {message}")]
#[serde(deny_unknown_fields)]
pub struct OperationError {
    pub code: OperationErrorCode,
    pub message: String,
    pub retriable: bool,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "delay_if_present"
    )]
    #[schemars(with = "u64")]
    pub retry_after_seconds: Option<u64>,
}
fn delay_if_present<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<Option<u64>, D::Error> {
    u64::deserialize(deserializer).map(Some)
}
impl OperationError {
    #[must_use]
    pub fn new(code: OperationErrorCode, message: impl Into<String>, retriable: bool) -> Self {
        Self {
            code,
            message: message.into(),
            retriable,
            retry_after_seconds: None,
        }
    }
    #[must_use]
    pub fn rate_limited(message: impl Into<String>, retry_after_seconds: Option<u64>) -> Self {
        Self {
            code: OperationErrorCode::RateLimited,
            message: message.into(),
            retriable: true,
            retry_after_seconds,
        }
    }
    /// A definite v2 refusal becomes legacy Unavailable with no delay extension.
    #[must_use]
    pub fn into_legacy(self) -> legacy::OperationError {
        use legacy::OperationErrorCode as Old;
        let code = match self.code {
            OperationErrorCode::Unavailable | OperationErrorCode::RateLimited => Old::Unavailable,
            OperationErrorCode::NotFound => Old::NotFound,
            OperationErrorCode::NotGranted => Old::NotGranted,
            OperationErrorCode::InvalidInput => Old::InvalidInput,
            OperationErrorCode::StaleAuthority => Old::StaleAuthority,
            OperationErrorCode::ApprovalRequired => Old::ApprovalRequired,
            OperationErrorCode::ApprovalDenied => Old::ApprovalDenied,
            OperationErrorCode::ResultTooLarge => Old::ResultTooLarge,
            OperationErrorCode::Protocol => Old::Protocol,
            OperationErrorCode::OutcomeUnknown => Old::OutcomeUnknown,
        };
        legacy::OperationError::new(code, self.message, self.retriable)
    }
}
impl From<legacy::OperationError> for OperationError {
    fn from(value: legacy::OperationError) -> Self {
        use legacy::OperationErrorCode as Old;
        let code = match value.code {
            Old::Unavailable => OperationErrorCode::Unavailable,
            Old::NotFound => OperationErrorCode::NotFound,
            Old::NotGranted => OperationErrorCode::NotGranted,
            Old::InvalidInput => OperationErrorCode::InvalidInput,
            Old::StaleAuthority => OperationErrorCode::StaleAuthority,
            Old::ApprovalRequired => OperationErrorCode::ApprovalRequired,
            Old::ApprovalDenied => OperationErrorCode::ApprovalDenied,
            Old::ResultTooLarge => OperationErrorCode::ResultTooLarge,
            Old::Protocol => OperationErrorCode::Protocol,
            Old::OutcomeUnknown => OperationErrorCode::OutcomeUnknown,
        };
        Self::new(code, value.message, value.retriable)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ResponseEnvelope {
    pub protocol: String,
    pub request_id: String,
    pub status: ResponseStatus,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub response: Option<OperationResult>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<OperationError>,
}
impl ResponseEnvelope {
    #[must_use]
    pub fn success(request_id: impl Into<String>, response: OperationResult) -> Self {
        Self {
            protocol: CONTRACT.to_owned(),
            request_id: request_id.into(),
            status: ResponseStatus::Ok,
            response: Some(response),
            error: None,
        }
    }
    #[must_use]
    pub fn failure(request_id: impl Into<String>, error: OperationError) -> Self {
        Self {
            protocol: CONTRACT.to_owned(),
            request_id: request_id.into(),
            status: ResponseStatus::Error,
            response: None,
            error: Some(error),
        }
    }
    pub fn validate(&self) -> Result<(), OperationError> {
        if self.protocol != CONTRACT {
            return Err(protocol_refusal());
        }
        if let Some(error) = &self.error {
            if error.retry_after_seconds.is_some() && error.code != OperationErrorCode::RateLimited
            {
                return Err(protocol_refusal());
            }
        }
        if let Some(OperationResult::Describe(description)) = &self.response {
            if let Some(advice) = &description.rate_advice {
                advice.validate()?;
            }
        }
        self.clone()
            .into_legacy()
            .validate()
            .map_err(OperationError::from)?;
        if serde_json::to_vec(self).map_or(true, |bytes| bytes.len() > MAX_RESULT_BYTES) {
            return Err(OperationError::new(
                OperationErrorCode::ResultTooLarge,
                "operation response exceeds the admitted bound",
                false,
            ));
        }
        Ok(())
    }
    #[must_use]
    pub fn into_legacy(self) -> legacy::ResponseEnvelope {
        legacy::ResponseEnvelope {
            protocol: legacy::CONTRACT.to_owned(),
            request_id: self.request_id,
            status: self.status,
            response: self.response.map(OperationResult::into_legacy),
            error: self.error.map(OperationError::into_legacy),
        }
    }
}
impl From<legacy::ResponseEnvelope> for ResponseEnvelope {
    fn from(value: legacy::ResponseEnvelope) -> Self {
        Self {
            protocol: CONTRACT.to_owned(),
            request_id: value.request_id,
            status: value.status,
            response: value.response.map(Into::into),
            error: value.error.map(Into::into),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Version {
    V0Alpha1,
    V0Alpha2,
}
impl Version {
    pub fn encode_response(self, response: ResponseEnvelope) -> Result<Vec<u8>, OperationError> {
        response.validate()?;
        match self {
            Self::V0Alpha1 => serde_json::to_vec(&response.into_legacy()),
            Self::V0Alpha2 => serde_json::to_vec(&response),
        }
        .map_err(|_| protocol_refusal())
    }
}
/// Validate the selected identity before returning one request for the shared backend.
/// This adapter performs no dispatch and never retries or falls back to another wire version.
pub fn decode_request(bytes: &[u8]) -> Result<(Version, RequestEnvelope), OperationError> {
    if bytes.len() > MAX_FRAME_BYTES {
        return Err(protocol_refusal());
    }
    let frame: legacy::RequestEnvelope =
        serde_json::from_slice(bytes).map_err(|_| protocol_refusal())?;
    match frame.protocol.as_str() {
        legacy::CONTRACT => {
            frame.validate().map_err(OperationError::from)?;
            Ok((Version::V0Alpha1, frame.into()))
        }
        CONTRACT => {
            let current: RequestEnvelope =
                serde_json::from_slice(bytes).map_err(|_| protocol_refusal())?;
            current.validate()?;
            Ok((Version::V0Alpha2, current))
        }
        _ => Err(protocol_refusal()),
    }
}
fn protocol_refusal() -> OperationError {
    OperationError::new(
        OperationErrorCode::Protocol,
        "operation protocol identity or framing is invalid",
        false,
    )
}
