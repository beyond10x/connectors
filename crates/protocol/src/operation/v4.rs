//! Endpoint-aware operation requests; predecessor readers remain byte-for-byte strict.
#![allow(missing_docs)]

use super::{v3, wire};
use serde::{Deserialize, Serialize};
pub use v3::{OperationError, OperationErrorCode};
pub use wire::{OperationResult, OwnerContext, ResponseStatus};

pub const CONTRACT: &str = "b10x.connector-operation.v0alpha4";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RequestEnvelope {
    pub protocol: String,
    pub request_id: String,
    pub context: OwnerContext,
    pub request: OperationRequest,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(
    tag = "method",
    content = "params",
    rename_all = "snake_case",
    deny_unknown_fields
)]
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
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "target_if_present"
    )]
    pub connection_ref: Option<String>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "target_if_present"
    )]
    pub endpoint_ref: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct InvokeRequest {
    pub operation_ref: String,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "target_if_present"
    )]
    pub connection_ref: Option<String>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "target_if_present"
    )]
    pub endpoint_ref: Option<String>,
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
        if self.protocol != CONTRACT {
            return Err(v3::protocol_refusal());
        }
        self.request.validate_target()?;
        // The placeholder validates the old request bounds only. It is never routed or admitted.
        wire::RequestEnvelope {
            protocol: wire::CONTRACT.into(),
            request_id: self.request_id.clone(),
            context: self.context.clone(),
            request: self
                .request
                .clone()
                .into_internal(Some("validation-only"))?,
        }
        .validate()?;
        if serde_json::to_vec(self).map_or(true, |bytes| bytes.len() > wire::MAX_FRAME_BYTES) {
            return Err(v3::protocol_refusal());
        }
        Ok(())
    }
}

impl OperationRequest {
    /// Resolve endpoint identity outside this wire type, through the admitted backend owner.
    pub fn into_internal(
        self,
        resolved_connection: Option<&str>,
    ) -> Result<wire::OperationRequest, OperationError> {
        self.validate_target()?;
        Ok(match self {
            Self::Search(v) => wire::OperationRequest::Search(v),
            Self::Describe(v) => wire::OperationRequest::Describe(wire::DescribeRequest {
                operation_ref: v.operation_ref,
            }),
            Self::Invoke(v) => wire::OperationRequest::Invoke(wire::InvokeRequest {
                operation_ref: v.operation_ref,
                connection_ref: v
                    .connection_ref
                    .or_else(|| resolved_connection.map(str::to_owned))
                    .ok_or_else(v3::protocol_refusal)?,
                description_ref: v.description_ref,
                input: v.input,
                approval_evidence_ref: v.approval_evidence_ref,
            }),
            Self::SessionStatus(v) => wire::OperationRequest::SessionStatus(v),
            Self::SessionTerminate(v) => wire::OperationRequest::SessionTerminate(v),
            Self::SessionReconcile(v) => wire::OperationRequest::SessionReconcile(v),
            Self::SessionSignal(v) => wire::OperationRequest::SessionSignal(v),
        })
    }

    pub fn validate_target(&self) -> Result<(), OperationError> {
        let (connection, endpoint, required) = match self {
            Self::Describe(v) => (&v.connection_ref, &v.endpoint_ref, false),
            Self::Invoke(v) => (&v.connection_ref, &v.endpoint_ref, true),
            _ => return Ok(()),
        };
        if (connection.is_some() && endpoint.is_some())
            || (required && connection.is_none() && endpoint.is_none())
            || [connection, endpoint].into_iter().flatten().any(|value| {
                value.is_empty()
                    || value.len() > 512
                    || !value.bytes().all(|byte| byte.is_ascii_graphic())
            })
        {
            return Err(OperationError::new(
                OperationErrorCode::InvalidInput,
                "select exactly one operation target",
                false,
            ));
        }
        Ok(())
    }
}

impl From<wire::OperationRequest> for OperationRequest {
    fn from(value: wire::OperationRequest) -> Self {
        match value {
            wire::OperationRequest::Search(v) => Self::Search(v),
            wire::OperationRequest::Describe(v) => Self::Describe(DescribeRequest {
                operation_ref: v.operation_ref,
                connection_ref: None,
                endpoint_ref: None,
            }),
            wire::OperationRequest::Invoke(v) => Self::Invoke(InvokeRequest {
                operation_ref: v.operation_ref,
                connection_ref: Some(v.connection_ref),
                endpoint_ref: None,
                description_ref: v.description_ref,
                input: v.input,
                approval_evidence_ref: v.approval_evidence_ref,
            }),
            wire::OperationRequest::SessionStatus(v) => Self::SessionStatus(v),
            wire::OperationRequest::SessionTerminate(v) => Self::SessionTerminate(v),
            wire::OperationRequest::SessionReconcile(v) => Self::SessionReconcile(v),
            wire::OperationRequest::SessionSignal(v) => Self::SessionSignal(v),
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
        if self.protocol != CONTRACT {
            return Err(v3::protocol_refusal());
        }
        self.clone().into_v3().validate()
    }
    pub fn into_v3(self) -> v3::ResponseEnvelope {
        v3::ResponseEnvelope {
            protocol: v3::CONTRACT.into(),
            request_id: self.request_id,
            status: self.status,
            response: self.response,
            error: self.error,
        }
    }
}

impl From<v3::ResponseEnvelope> for ResponseEnvelope {
    fn from(value: v3::ResponseEnvelope) -> Self {
        Self {
            protocol: CONTRACT.into(),
            request_id: value.request_id,
            status: value.status,
            response: value.response,
            error: value.error,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn request(connection: Option<&str>, endpoint: Option<&str>) -> RequestEnvelope {
        RequestEnvelope {
            protocol: CONTRACT.into(),
            request_id: "test".into(),
            context: OwnerContext {
                tenant_id: "tenant".into(),
                agent_id: "agent".into(),
                agent_revision: 1,
                authority_snapshot_id: "authority".into(),
                authority_snapshot_sha256: "a".repeat(64),
            },
            request: OperationRequest::Invoke(InvokeRequest {
                operation_ref: "loki.query".into(),
                connection_ref: connection.map(str::to_owned),
                endpoint_ref: endpoint.map(str::to_owned),
                description_ref: "description:one".into(),
                input: json!({}),
                approval_evidence_ref: None,
            }),
        }
    }

    #[test]
    fn target_is_exclusive_and_never_projected_without_resolution() {
        assert!(request(None, None).validate().is_err());
        assert!(request(Some("connection:one"), Some("endpoint:one"))
            .validate()
            .is_err());
        let value = request(None, Some("endpoint:one"));
        assert!(value.validate().is_ok());
        assert!(value.request.clone().into_internal(None).is_err());
        let wire::OperationRequest::Invoke(value) = value
            .request
            .into_internal(Some("connection:resolved"))
            .unwrap()
        else {
            panic!()
        };
        assert_eq!(value.connection_ref, "connection:resolved");
    }

    #[test]
    fn predecessors_reject_endpoint_fields() {
        let mut value = serde_json::to_value(request(None, Some("endpoint:one"))).unwrap();
        value["protocol"] = json!(v3::CONTRACT);
        assert!(
            super::super::versions::decode_request(&serde_json::to_vec(&value).unwrap()).is_err()
        );
    }
}
