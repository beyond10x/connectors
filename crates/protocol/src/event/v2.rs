//! Explicit endpoint subscription lifecycle, preserving the frozen v1 durable event reads.
#![allow(missing_docs)]

pub use super::{
    ChannelSummary, DataEvent, EventError, EventErrorCode, ReceiveRequest, ReplayRequest,
    ResponseStatus, SearchRequest, MAX_FRAME_BYTES, MAX_RESPONSE_BYTES,
};
use crate::operation::OwnerContext;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;

pub const CONTRACT: &str = "b10x.connector-event.v0alpha2";
pub const MAX_PARAMETER_BYTES: usize = 16 * 1024;
pub const MAX_PARAMETERS: usize = 32;

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
    Subscribe(SubscribeRequest),
    Unsubscribe(UnsubscribeRequest),
}

/// Parameters are provider-declared channel configuration, never credentials or route overrides.
/// The runtime validates each name and value against its pinned channel declaration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SubscribeRequest {
    pub endpoint_ref: String,
    pub channel_binding: String,
    pub parameters: BTreeMap<String, Value>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct UnsubscribeRequest {
    pub subscription_ref: String,
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
    Subscribe {
        subscription_ref: String,
        channel: ChannelSummary,
    },
    Unsubscribe {
        subscription_ref: String,
    },
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

impl EventRequest {
    /// Existing reads retain exactly their old authority and backend dispatch.
    #[must_use]
    pub fn legacy_read(&self) -> Option<super::EventRequest> {
        match self {
            Self::Search(value) => Some(super::EventRequest::Search(value.clone())),
            Self::Receive(value) => Some(super::EventRequest::Receive(value.clone())),
            Self::Replay(value) => Some(super::EventRequest::Replay(value.clone())),
            Self::Subscribe(_) | Self::Unsubscribe(_) => None,
        }
    }
}

impl From<super::EventResult> for EventResult {
    fn from(value: super::EventResult) -> Self {
        match value {
            super::EventResult::Search { channels } => Self::Search { channels },
            super::EventResult::Receive { events, next } => Self::Receive { events, next },
            super::EventResult::Replay(event) => Self::Replay(event),
        }
    }
}

impl RequestEnvelope {
    pub fn validate(&self) -> Result<(), EventError> {
        if self.protocol != CONTRACT
            || !super::valid_ref(&self.request_id, 128)
            || serde_json::to_vec(self).map_or(true, |value| value.len() > MAX_FRAME_BYTES)
        {
            return Err(super::protocol_refusal());
        }
        super::validate_context(&self.context)?;
        if let Some(request) = self.request.legacy_read() {
            return super::RequestEnvelope {
                protocol: super::CONTRACT.into(),
                request_id: self.request_id.clone(),
                context: self.context.clone(),
                request,
            }
            .validate();
        }
        match &self.request {
            EventRequest::Subscribe(request) => {
                super::require_ref(&request.endpoint_ref)?;
                super::require_ref(&request.channel_binding)?;
                if request.parameters.len() > MAX_PARAMETERS
                    || request
                        .parameters
                        .keys()
                        .any(|key| !super::valid_ref(key, 128))
                    || serde_json::to_vec(&request.parameters)
                        .map_or(true, |value| value.len() > MAX_PARAMETER_BYTES)
                {
                    return Err(super::invalid_input(
                        "subscription parameters exceed their bounds",
                    ));
                }
            }
            EventRequest::Unsubscribe(request) => super::require_ref(&request.subscription_ref)?,
            _ => unreachable!("legacy reads returned above"),
        }
        Ok(())
    }
}

impl ResponseEnvelope {
    #[must_use]
    pub fn success(request_id: impl Into<String>, response: EventResult) -> Self {
        Self {
            protocol: CONTRACT.into(),
            request_id: request_id.into(),
            status: ResponseStatus::Ok,
            response: Some(response),
            error: None,
        }
    }
    #[must_use]
    pub fn failure(request_id: impl Into<String>, error: EventError) -> Self {
        Self {
            protocol: CONTRACT.into(),
            request_id: request_id.into(),
            status: ResponseStatus::Error,
            response: None,
            error: Some(error),
        }
    }
    pub fn validate(&self) -> Result<(), EventError> {
        if self.protocol != CONTRACT
            || !super::valid_ref(&self.request_id, 128)
            || serde_json::to_vec(self).map_or(true, |value| value.len() > MAX_RESPONSE_BYTES)
        {
            return Err(super::protocol_refusal());
        }
        match (self.status, &self.response, &self.error) {
            (ResponseStatus::Ok, Some(result), None) => result.validate(),
            (ResponseStatus::Error, None, Some(error))
                if !error.message.is_empty() && error.message.len() <= 4096 =>
            {
                Ok(())
            }
            _ => Err(super::protocol_refusal()),
        }
    }
}

impl EventResult {
    fn validate(&self) -> Result<(), EventError> {
        match self {
            Self::Search { channels } => super::validate_result(&super::EventResult::Search {
                channels: channels.clone(),
            }),
            Self::Receive { events, next } => {
                super::validate_result(&super::EventResult::Receive {
                    events: events.clone(),
                    next: next.clone(),
                })
            }
            Self::Replay(event) => super::validate_event(event),
            Self::Subscribe {
                subscription_ref,
                channel,
            } => {
                super::require_ref(subscription_ref)?;
                super::validate_result(&super::EventResult::Search {
                    channels: vec![channel.clone()],
                })
            }
            Self::Unsubscribe { subscription_ref } => super::require_ref(subscription_ref),
        }
    }
    #[must_use]
    pub fn matches(&self, request: &EventRequest) -> bool {
        match (self, request) {
            (Self::Search { .. }, EventRequest::Search(_)) => true,
            (Self::Receive { events, .. }, EventRequest::Receive(request)) => {
                events.len() <= usize::from(request.limit)
                    && events
                        .iter()
                        .all(|event| event.channel_ref == request.channel_ref)
            }
            (Self::Replay(event), EventRequest::Replay(request)) => {
                event.event_ref == request.event_ref
            }
            (Self::Subscribe { channel, .. }, EventRequest::Subscribe(request)) => {
                channel.binding_ref == request.channel_binding
            }
            (Self::Unsubscribe { subscription_ref }, EventRequest::Unsubscribe(request)) => {
                *subscription_ref == request.subscription_ref
            }
            _ => false,
        }
    }
}
