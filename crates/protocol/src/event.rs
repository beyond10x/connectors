//! Generic durable data-event pull protocol.

// The protocol carries normalized provider payloads, but never provider transport envelopes or
// credential material. Operational events are a different family and are intentionally absent.
#![allow(missing_docs)]

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::operation::OwnerContext;

pub const CONTRACT: &str = "b10x.connector-event.v0alpha1";
pub const MAX_FRAME_BYTES: usize = 64 * 1024;
pub const MAX_RESPONSE_BYTES: usize = 1024 * 1024;
pub const MAX_EVENT_BYTES: usize = 512 * 1024;
pub const MAX_SEARCH_RESULTS: u16 = 64;
pub const MAX_RECEIVE_RESULTS: u16 = 100;
pub const MAX_WAIT_MS: u32 = 30_000;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RequestEnvelope {
    pub protocol: String,
    pub request_id: String,
    pub context: OwnerContext,
    pub request: EventRequest,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(
    tag = "method",
    content = "params",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum EventRequest {
    Search(SearchRequest),
    Receive(ReceiveRequest),
    Replay(ReplayRequest),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SearchRequest {
    pub query: String,
    pub limit: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ReceiveRequest {
    pub channel_ref: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub after: Option<String>,
    pub limit: u16,
    pub wait_ms: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ReplayRequest {
    pub event_ref: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum EventProvenance {
    Native,
    Polled,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ChannelSummary {
    pub channel_ref: String,
    pub connection_ref: String,
    pub integration_ref: String,
    pub binding_ref: String,
    pub events: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DataEvent {
    pub event_ref: String,
    pub channel_ref: String,
    pub connection_ref: String,
    pub integration_ref: String,
    pub event_type: String,
    pub provenance: EventProvenance,
    pub received_at_unix_ms: u64,
    pub payload: Value,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(
    tag = "result",
    content = "value",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum EventResult {
    Search {
        channels: Vec<ChannelSummary>,
    },
    Receive {
        events: Vec<DataEvent>,
        next: String,
    },
    Replay(DataEvent),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum EventErrorCode {
    Unavailable,
    NotFound,
    NotGranted,
    InvalidInput,
    StaleAuthority,
    Protocol,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, thiserror::Error, schemars::JsonSchema)]
#[error("{code:?}: {message}")]
#[serde(deny_unknown_fields)]
pub struct EventError {
    pub code: EventErrorCode,
    pub message: String,
    pub retriable: bool,
}

impl EventError {
    #[must_use]
    pub fn new(code: EventErrorCode, message: impl Into<String>, retriable: bool) -> Self {
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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ResponseEnvelope {
    pub protocol: String,
    pub request_id: String,
    pub status: ResponseStatus,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub response: Option<EventResult>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<EventError>,
}

impl RequestEnvelope {
    pub fn validate(&self) -> Result<(), EventError> {
        if self.protocol != CONTRACT || !valid_ref(&self.request_id, 128) {
            return Err(protocol_refusal());
        }
        validate_context(&self.context)?;
        match &self.request {
            EventRequest::Search(request) => {
                if request.query.len() > 512
                    || request.limit == 0
                    || request.limit > MAX_SEARCH_RESULTS
                {
                    return Err(invalid_input("event search bounds are invalid"));
                }
            }
            EventRequest::Receive(request) => {
                require_ref(&request.channel_ref)?;
                if request.limit == 0
                    || request.limit > MAX_RECEIVE_RESULTS
                    || request.wait_ms > MAX_WAIT_MS
                    || request.after.as_deref().is_some_and(|value| {
                        value.is_empty()
                            || value.len() > 128
                            || !value.bytes().all(|byte| byte.is_ascii_digit())
                    })
                {
                    return Err(invalid_input("event receive bounds are invalid"));
                }
            }
            EventRequest::Replay(request) => require_ref(&request.event_ref)?,
        }
        Ok(())
    }
}

impl ResponseEnvelope {
    #[must_use]
    pub fn success(request_id: impl Into<String>, response: EventResult) -> Self {
        Self {
            protocol: CONTRACT.to_owned(),
            request_id: request_id.into(),
            status: ResponseStatus::Ok,
            response: Some(response),
            error: None,
        }
    }

    #[must_use]
    pub fn failure(request_id: impl Into<String>, error: EventError) -> Self {
        Self {
            protocol: CONTRACT.to_owned(),
            request_id: request_id.into(),
            status: ResponseStatus::Error,
            response: None,
            error: Some(error),
        }
    }

    pub fn validate(&self) -> Result<(), EventError> {
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

fn validate_result(result: &EventResult) -> Result<(), EventError> {
    match result {
        EventResult::Search { channels } => {
            if channels.len() > usize::from(MAX_SEARCH_RESULTS) {
                return Err(protocol_refusal());
            }
            for channel in channels {
                require_ref(&channel.channel_ref)?;
                require_ref(&channel.connection_ref)?;
                require_ref(&channel.integration_ref)?;
                require_ref(&channel.binding_ref)?;
                if channel.events.is_empty()
                    || channel.events.len() > 64
                    || channel.events.iter().any(|event| !valid_ref(event, 512))
                {
                    return Err(protocol_refusal());
                }
            }
        }
        EventResult::Receive { events, next } => {
            if events.len() > usize::from(MAX_RECEIVE_RESULTS)
                || next.is_empty()
                || !next.bytes().all(|byte| byte.is_ascii_digit())
            {
                return Err(protocol_refusal());
            }
            for event in events {
                validate_event(event)?;
            }
        }
        EventResult::Replay(event) => validate_event(event)?,
    }
    Ok(())
}

fn validate_event(event: &DataEvent) -> Result<(), EventError> {
    if !valid_ref(&event.event_ref, 512)
        || !valid_ref(&event.channel_ref, 512)
        || !valid_ref(&event.connection_ref, 512)
        || !valid_ref(&event.integration_ref, 512)
        || !valid_ref(&event.event_type, 512)
        || serde_json::to_vec(&event.payload).map_or(true, |bytes| bytes.len() > MAX_EVENT_BYTES)
    {
        return Err(protocol_refusal());
    }
    Ok(())
}

fn validate_context(context: &OwnerContext) -> Result<(), EventError> {
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

fn require_ref(value: &str) -> Result<(), EventError> {
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

fn invalid_input(message: &'static str) -> EventError {
    EventError::new(EventErrorCode::InvalidInput, message, false)
}

fn protocol_refusal() -> EventError {
    EventError::new(
        EventErrorCode::Protocol,
        "event protocol frame was refused",
        false,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cursor_and_wait_are_bounded() {
        let request = ReceiveRequest {
            channel_ref: "channel:slack:one".to_owned(),
            after: Some("not-a-cursor".to_owned()),
            limit: 1,
            wait_ms: 0,
        };
        let context = OwnerContext {
            tenant_id: "tenant-local".to_owned(),
            agent_id: "agent-dev".to_owned(),
            agent_revision: 1,
            authority_snapshot_id: "authority-1".to_owned(),
            authority_snapshot_sha256: "a".repeat(64),
        };
        let envelope = RequestEnvelope {
            protocol: CONTRACT.to_owned(),
            request_id: "request-1".to_owned(),
            context,
            request: EventRequest::Receive(request),
        };
        assert!(envelope.validate().is_err());
    }
}
