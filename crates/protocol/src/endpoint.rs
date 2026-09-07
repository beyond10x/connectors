//! Versioned, credential-free service inventory and operator bindings.
#![allow(missing_docs)]

use crate::operation::{self, OwnerContext};
pub use domain::endpoint::{
    Endpoint, EndpointBinding, EndpointCredentialReference, EndpointState, EndpointTls,
    EndpointTransport,
};
use serde::{Deserialize, Serialize};

pub const CONTRACT: &str = "b10x.connector-endpoint.v0alpha1";
pub const MAX_FRAME_BYTES: usize = 64 * 1024;
pub const MAX_RESULT_BYTES: usize = 512 * 1024;
pub const MAX_RESULTS: u16 = 100;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
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
    List(ListRequest),
    Show(ShowRequest),
    Refresh(RefreshRequest),
    Bind(BindRequest),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ListRequest {
    pub source_ref: Option<String>,
    pub query: String,
    pub limit: u16,
    pub cursor: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ShowRequest {
    pub endpoint_ref: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RefreshRequest {
    pub source_ref: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct BindRequest {
    pub endpoint_ref: String,
    pub binding: EndpointBinding,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(
    tag = "result",
    content = "value",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum EndpointResult {
    List {
        endpoints: Vec<Endpoint>,
        next_cursor: Option<String>,
        warnings: Vec<String>,
    },
    Show {
        endpoint: Endpoint,
    },
    Refresh {
        endpoints: usize,
        warnings: Vec<String>,
    },
    Bind {
        endpoint: Endpoint,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum EndpointErrorCode {
    NotFound,
    NotGranted,
    InvalidInput,
    Conflict,
    Unavailable,
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
    pub fn new(code: EndpointErrorCode, message: impl Into<String>, retriable: bool) -> Self {
        Self {
            code,
            message: message.into(),
            retriable,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ResponseEnvelope {
    pub protocol: String,
    pub request_id: String,
    pub status: operation::ResponseStatus,
    pub response: Option<EndpointResult>,
    pub error: Option<EndpointError>,
}

impl RequestEnvelope {
    pub fn validate(&self) -> Result<(), EndpointError> {
        if self.protocol != CONTRACT || !reference(&self.request_id, 128) {
            return Err(refusal());
        }
        operation::RequestEnvelope {
            protocol: operation::CONTRACT.into(),
            request_id: self.request_id.clone(),
            context: self.context.clone(),
            request: operation::OperationRequest::Search(operation::SearchRequest {
                query: String::new(),
                limit: 1,
            }),
        }
        .validate()
        .map_err(|_| refusal())?;
        let valid = match &self.request {
            EndpointRequest::List(value) => {
                value
                    .source_ref
                    .as_deref()
                    .is_none_or(|v| reference(v, 512))
                    && value.query.len() <= 512
                    && value.limit > 0
                    && value.limit <= MAX_RESULTS
                    && value.cursor.as_deref().is_none_or(|v| reference(v, 4096))
            }
            EndpointRequest::Show(value) => reference(&value.endpoint_ref, 512),
            EndpointRequest::Refresh(value) => value
                .source_ref
                .as_deref()
                .is_none_or(|v| reference(v, 512)),
            EndpointRequest::Bind(value) => {
                reference(&value.endpoint_ref, 512) && value.binding.validate()
            }
        };
        if !valid || serde_json::to_vec(self).map_or(true, |v| v.len() > MAX_FRAME_BYTES) {
            return Err(EndpointError::new(
                EndpointErrorCode::InvalidInput,
                "endpoint request bounds are invalid",
                false,
            ));
        }
        Ok(())
    }
}

impl ResponseEnvelope {
    pub fn success(request_id: impl Into<String>, response: EndpointResult) -> Self {
        Self {
            protocol: CONTRACT.into(),
            request_id: request_id.into(),
            status: operation::ResponseStatus::Ok,
            response: Some(response),
            error: None,
        }
    }

    pub fn failure(request_id: impl Into<String>, error: EndpointError) -> Self {
        Self {
            protocol: CONTRACT.into(),
            request_id: request_id.into(),
            status: operation::ResponseStatus::Error,
            response: None,
            error: Some(error),
        }
    }

    pub fn validate(&self) -> Result<(), EndpointError> {
        if self.protocol != CONTRACT
            || !reference(&self.request_id, 128)
            || serde_json::to_vec(self).map_or(true, |v| v.len() > MAX_RESULT_BYTES)
        {
            return Err(refusal());
        }
        match (&self.status, &self.response, &self.error) {
            (operation::ResponseStatus::Ok, Some(result), None) => {
                let valid = match result {
                    EndpointResult::List {
                        endpoints,
                        next_cursor,
                        warnings,
                    } => {
                        endpoints.len() <= usize::from(MAX_RESULTS)
                            && endpoints.iter().all(Endpoint::validate)
                            && next_cursor.as_deref().is_none_or(|v| reference(v, 4096))
                            && valid_warnings(warnings)
                    }
                    EndpointResult::Show { endpoint } | EndpointResult::Bind { endpoint } => {
                        endpoint.validate()
                    }
                    EndpointResult::Refresh { warnings, .. } => valid_warnings(warnings),
                };
                if !valid {
                    return Err(refusal());
                }
            }
            (operation::ResponseStatus::Error, None, Some(error))
                if !error.message.is_empty()
                    && error.message.len() <= 4096
                    && !error.message.chars().any(char::is_control) => {}
            _ => return Err(refusal()),
        }
        Ok(())
    }

    /// Bind a successful response to the requested verb and exact endpoint reference.
    pub fn matches_request(&self, request: &EndpointRequest) -> bool {
        match (&self.response, request) {
            (None, _) => self.error.is_some(),
            (Some(EndpointResult::List { endpoints, .. }), EndpointRequest::List(request)) => {
                endpoints.len() <= usize::from(request.limit)
                    && request.source_ref.as_ref().is_none_or(|source| {
                        endpoints
                            .iter()
                            .all(|endpoint| &endpoint.source_ref == source)
                    })
            }
            (Some(EndpointResult::Show { endpoint }), EndpointRequest::Show(request)) => {
                endpoint.endpoint_ref == request.endpoint_ref
            }
            (Some(EndpointResult::Bind { endpoint }), EndpointRequest::Bind(request)) => {
                endpoint.endpoint_ref == request.endpoint_ref
                    && endpoint.binding.as_ref() == Some(&request.binding)
            }
            (Some(EndpointResult::Refresh { .. }), EndpointRequest::Refresh(_)) => true,
            _ => false,
        }
    }
}

fn valid_warnings(values: &[String]) -> bool {
    values.len() <= 100
        && values
            .iter()
            .all(|v| !v.is_empty() && v.len() <= 1024 && !v.chars().any(char::is_control))
}
fn reference(value: &str, maximum: usize) -> bool {
    !value.is_empty() && value.len() <= maximum && value.bytes().all(|v| v.is_ascii_graphic())
}
fn refusal() -> EndpointError {
    EndpointError::new(
        EndpointErrorCode::Protocol,
        "endpoint protocol frame is invalid",
        false,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn envelope(request: EndpointRequest) -> RequestEnvelope {
        RequestEnvelope {
            protocol: CONTRACT.into(),
            request_id: "endpoint:test".into(),
            context: OwnerContext {
                tenant_id: "tenant".into(),
                agent_id: "agent".into(),
                agent_revision: 1,
                authority_snapshot_id: "snapshot".into(),
                authority_snapshot_sha256: "a".repeat(64),
            },
            request,
        }
    }

    #[test]
    fn endpoint_page_and_source_reference_bounds_are_enforced() {
        let mut request = envelope(EndpointRequest::List(ListRequest {
            source_ref: None,
            query: String::new(),
            limit: MAX_RESULTS,
            cursor: None,
        }));
        assert!(request.validate().is_ok());
        let EndpointRequest::List(list) = &mut request.request else {
            panic!()
        };
        list.limit += 1;
        assert!(request.validate().is_err());
        assert!(envelope(EndpointRequest::Show(ShowRequest {
            endpoint_ref: String::new()
        }))
        .validate()
        .is_err());
    }

    #[test]
    fn endpoint_binding_cannot_embed_url_credentials_or_redirect_paths() {
        for address in [
            "https://user:password@service.example",
            "https://service.example/?token=secret",
            "https://service.example/#secret",
        ] {
            let binding = EndpointBinding {
                provider: "loki".into(),
                base_path: None,
                credential: None,
                direct_address: Some(address.into()),
                database: None,
                tls: None,
            };
            assert!(!binding.validate());
        }
        let binding = EndpointBinding {
            provider: "loki".into(),
            base_path: Some("//another.example".into()),
            credential: None,
            direct_address: None,
            database: None,
            tls: None,
        };
        assert!(!binding.validate());
    }

    #[test]
    fn endpoint_reader_rejects_unknown_credential_values() {
        let value = serde_json::json!({"kind":"kubernetes_secret","namespace":"default","name":"database","keys":{"password":"password"},"password":"not-accepted"});
        assert!(serde_json::from_value::<EndpointCredentialReference>(value).is_err());
    }
}
