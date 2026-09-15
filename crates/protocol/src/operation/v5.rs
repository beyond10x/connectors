//! One endpoint selector for every configured or discovered operation target.
#![allow(missing_docs)]

use super::{v3, wire};
use serde::{Deserialize, Serialize};
pub use v3::{OperationError, OperationErrorCode};
pub use wire::{OperationResult, OwnerContext, ResponseStatus};

pub const CONTRACT: &str = "b10x.connector-operation.v0alpha5";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RequestEnvelope {
    pub protocol: String,
    pub request_id: String,
    pub context: OwnerContext,
    pub request: OperationRequest,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(tag = "method", content = "params", rename_all = "snake_case", deny_unknown_fields)]
pub enum OperationRequest {
    Search(wire::SearchRequest),
    Describe(DescribeRequest),
    Invoke(InvokeRequest),
    SessionStatus(wire::SessionRequest),
    SessionTerminate(wire::SessionTerminateRequest),
    SessionReconcile(wire::SessionRequest),
    SessionSignal(wire::SessionSignalRequest),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DescribeRequest {
    pub operation_ref: String,
    #[serde(default, skip_serializing_if = "Option::is_none", deserialize_with = "target_if_present")]
    pub endpoint_ref: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct InvokeRequest {
    pub operation_ref: String,
    pub endpoint_ref: String,
    pub description_ref: String,
    pub input: serde_json::Value,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub approval_evidence_ref: Option<String>,
}

fn target_if_present<'de, D: serde::Deserializer<'de>>(de: D) -> Result<Option<String>, D::Error> {
    String::deserialize(de).map(Some)
}

impl RequestEnvelope {
    pub fn validate(&self) -> Result<(), OperationError> {
        if self.protocol != CONTRACT { return Err(v3::protocol_refusal()); }
        self.request.validate_target()?;
        // This validates bounds only; no placeholder reaches an execution backend.
        wire::RequestEnvelope {
            protocol: wire::CONTRACT.into(),
            request_id: self.request_id.clone(),
            context: self.context.clone(),
            request: self.request.clone().into_internal(Some("validation-only"))?,
        }.validate()?;
        if serde_json::to_vec(self).map_or(true, |bytes| bytes.len() > wire::MAX_FRAME_BYTES) {
            return Err(v3::protocol_refusal());
        }
        Ok(())
    }
}

impl OperationRequest {
    pub fn endpoint_ref(&self) -> Option<&str> {
        match self {
            Self::Describe(value) => value.endpoint_ref.as_deref(),
            Self::Invoke(value) => Some(&value.endpoint_ref),
            _ => None,
        }
    }

    /// Resolution is completed by the admitted endpoint owner, never by a wire reader.
    pub fn into_internal(self, resolved_endpoint: Option<&str>) -> Result<wire::OperationRequest, OperationError> {
        self.validate_target()?;
        Ok(match self {
            Self::Search(value) => wire::OperationRequest::Search(value),
            Self::Describe(value) => wire::OperationRequest::Describe(wire::DescribeRequest {
                operation_ref: value.operation_ref,
            }),
            Self::Invoke(value) => wire::OperationRequest::Invoke(wire::InvokeRequest {
                operation_ref: value.operation_ref,
                connection_ref: resolved_endpoint.ok_or_else(v3::protocol_refusal)?.into(),
                description_ref: value.description_ref,
                input: value.input,
                approval_evidence_ref: value.approval_evidence_ref,
            }),
            Self::SessionStatus(value) => wire::OperationRequest::SessionStatus(value),
            Self::SessionTerminate(value) => wire::OperationRequest::SessionTerminate(value),
            Self::SessionReconcile(value) => wire::OperationRequest::SessionReconcile(value),
            Self::SessionSignal(value) => wire::OperationRequest::SessionSignal(value),
        })
    }

    pub fn validate_target(&self) -> Result<(), OperationError> {
        if self.endpoint_ref().is_some_and(|value| value.is_empty() || value.len() > 512
            || !value.bytes().all(|byte| byte.is_ascii_graphic())) {
            return Err(OperationError::new(OperationErrorCode::InvalidInput, "endpoint_ref must be a nonempty endpoint reference", false));
        }
        Ok(())
    }
}

impl From<wire::OperationRequest> for OperationRequest {
    fn from(value: wire::OperationRequest) -> Self {
        match value {
            wire::OperationRequest::Search(value) => Self::Search(value),
            wire::OperationRequest::Describe(value) => Self::Describe(DescribeRequest {
                operation_ref: value.operation_ref, endpoint_ref: None,
            }),
            wire::OperationRequest::Invoke(value) => Self::Invoke(InvokeRequest {
                operation_ref: value.operation_ref, endpoint_ref: value.connection_ref,
                description_ref: value.description_ref, input: value.input,
                approval_evidence_ref: value.approval_evidence_ref,
            }),
            wire::OperationRequest::SessionStatus(value) => Self::SessionStatus(value),
            wire::OperationRequest::SessionTerminate(value) => Self::SessionTerminate(value),
            wire::OperationRequest::SessionReconcile(value) => Self::SessionReconcile(value),
            wire::OperationRequest::SessionSignal(value) => Self::SessionSignal(value),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ResponseEnvelope {
    pub protocol: String,
    pub request_id: String,
    pub status: ResponseStatus,
    pub response: Option<OperationResult>,
    pub error: Option<OperationError>,
}

impl ResponseEnvelope {
    pub fn success(request_id: impl Into<String>, response: OperationResult) -> Self {
        v3::ResponseEnvelope::success(request_id, response).into()
    }
    pub fn failure(request_id: impl Into<String>, error: OperationError) -> Self {
        v3::ResponseEnvelope::failure(request_id, error).into()
    }
    pub fn validate(&self) -> Result<(), OperationError> {
        if self.protocol != CONTRACT { return Err(v3::protocol_refusal()); }
        self.clone().into_v3().validate()
    }
    pub fn into_v3(self) -> v3::ResponseEnvelope {
        v3::ResponseEnvelope { protocol: v3::CONTRACT.into(), request_id: self.request_id,
            status: self.status, response: self.response, error: self.error }
    }
}

impl From<v3::ResponseEnvelope> for ResponseEnvelope {
    fn from(value: v3::ResponseEnvelope) -> Self {
        Self { protocol: CONTRACT.into(), request_id: value.request_id, status: value.status,
            response: value.response, error: value.error }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn input() -> serde_json::Value {
        json!({"method":"invoke","params":{"operation_ref":"slack-chat-post-message",
            "endpoint_ref":"endpoint:slack:bot-one","description_ref":"description:one","input":{}}})
    }

    #[test]
    fn target_is_required_and_old_selectors_are_refused() {
        let request: OperationRequest = serde_json::from_value(input()).unwrap();
        assert!(request.clone().into_internal(None).is_err());
        let wire::OperationRequest::Invoke(resolved) = request.into_internal(Some("endpoint:slack:bot-one")).unwrap() else { panic!() };
        assert_eq!(resolved.connection_ref, "endpoint:slack:bot-one");
        for field in ["connection_ref", "instance_id"] {
            let mut value = input();
            value["params"][field] = json!("legacy-target");
            assert!(serde_json::from_value::<OperationRequest>(value).is_err());
        }
        for replacement in [serde_json::Value::Null, json!(7)] {
            let mut value = input();
            value["params"]["endpoint_ref"] = replacement;
            assert!(serde_json::from_value::<OperationRequest>(value).is_err());
        }
        let mut value = input();
        value["params"].as_object_mut().unwrap().remove("endpoint_ref");
        assert!(serde_json::from_value::<OperationRequest>(value).is_err());
    }
}
