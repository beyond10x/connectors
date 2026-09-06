//! Additive safe authentication outcomes for ConnectorOperation v0alpha3.
// Field-level documentation is generated into the immutable contract bundle.
#![allow(missing_docs)]

use super::wire;
use serde::{Deserialize, Serialize};

pub use wire::{OperationRequest, OperationResult, OwnerContext, ResponseStatus};
pub const CONTRACT: &str = "b10x.connector-operation.v0alpha3";
/// Fixed text used when a requested predecessor cannot express authentication remediation.
pub const DOWNGRADE_MESSAGE: &str = "The operation is unavailable.";

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
        self.clone().into_v2().validate().map_err(Into::into)
    }

    /// Lossless request projection to the unchanged internal v2 API.
    #[must_use]
    pub fn into_v2(self) -> wire::RequestEnvelope {
        wire::RequestEnvelope {
            protocol: wire::CONTRACT.into(),
            request_id: self.request_id,
            context: self.context,
            request: self.request,
        }
    }
}

impl From<wire::RequestEnvelope> for RequestEnvelope {
    fn from(value: wire::RequestEnvelope) -> Self {
        Self {
            protocol: CONTRACT.into(),
            request_id: value.request_id,
            context: value.context,
            request: value.request,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum AuthenticationNeed {
    AuthorizeConfigured,
    ReauthorizeExisting,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum AuthenticationAttemptState {
    NotAttempted,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum AuthenticationNextAction {
    StartTrustedRemediation,
}

/// Safe facts only; this value cannot carry a session URL or execution authority.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AuthenticationRequired {
    pub operation_ref: String,
    pub connection_ref: String,
    pub integration_ref: String,
    pub auth_profile: String,
    pub need: AuthenticationNeed,
    pub attempt: AuthenticationAttemptState,
    pub next_action: AuthenticationNextAction,
}

impl AuthenticationRequired {
    pub fn validate(&self) -> Result<(), OperationError> {
        if [
            &self.operation_ref,
            &self.connection_ref,
            &self.integration_ref,
        ]
        .iter()
        .any(|value| {
            value.is_empty() || value.len() > 512 || !value.bytes().all(|b| b.is_ascii_graphic())
        }) || self.auth_profile.is_empty()
            || self.auth_profile.len() > 128
            || !self.auth_profile.bytes().all(|b| {
                b.is_ascii_lowercase() || b.is_ascii_digit() || matches!(b, b'.' | b'_' | b'-')
            })
        {
            return Err(protocol_refusal());
        }
        Ok(())
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
    AuthenticationRequired,
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
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "authentication_if_present"
    )]
    #[schemars(with = "AuthenticationRequired")]
    pub authentication: Option<Box<AuthenticationRequired>>,
}

fn delay_if_present<'de, D: serde::Deserializer<'de>>(de: D) -> Result<Option<u64>, D::Error> {
    u64::deserialize(de).map(Some)
}
fn authentication_if_present<'de, D: serde::Deserializer<'de>>(
    de: D,
) -> Result<Option<Box<AuthenticationRequired>>, D::Error> {
    AuthenticationRequired::deserialize(de).map(|value| Some(Box::new(value)))
}

impl OperationError {
    #[must_use]
    pub fn new(code: OperationErrorCode, message: impl Into<String>, retriable: bool) -> Self {
        Self {
            code,
            message: message.into(),
            retriable,
            retry_after_seconds: None,
            authentication: None,
        }
    }

    #[must_use]
    pub fn authentication_required(authentication: AuthenticationRequired) -> Self {
        Self {
            authentication: Some(Box::new(authentication)),
            ..Self::new(
                OperationErrorCode::AuthenticationRequired,
                "Authentication is required.",
                false,
            )
        }
    }

    pub fn validate(&self) -> Result<(), OperationError> {
        if self.message.is_empty()
            || self.message.len() > 4096
            || (self.retry_after_seconds.is_some() && self.code != OperationErrorCode::RateLimited)
        {
            return Err(protocol_refusal());
        }
        match (self.code, &self.authentication) {
            (OperationErrorCode::AuthenticationRequired, Some(value)) if !self.retriable => {
                value.validate()
            }
            (OperationErrorCode::AuthenticationRequired, _) | (_, Some(_)) => {
                Err(protocol_refusal())
            }
            (_, None) => Ok(()),
        }
    }

    /// Authentication loses all extension facts and its message; all other v2 fields are retained.
    #[must_use]
    pub fn into_v2(self) -> wire::OperationError {
        use wire::OperationErrorCode as Old;
        let code = match self.code {
            OperationErrorCode::AuthenticationRequired => {
                return wire::OperationError::new(Old::Unavailable, DOWNGRADE_MESSAGE, false)
            }
            OperationErrorCode::Unavailable => Old::Unavailable,
            OperationErrorCode::NotFound => Old::NotFound,
            OperationErrorCode::NotGranted => Old::NotGranted,
            OperationErrorCode::InvalidInput => Old::InvalidInput,
            OperationErrorCode::StaleAuthority => Old::StaleAuthority,
            OperationErrorCode::ApprovalRequired => Old::ApprovalRequired,
            OperationErrorCode::ApprovalDenied => Old::ApprovalDenied,
            OperationErrorCode::ResultTooLarge => Old::ResultTooLarge,
            OperationErrorCode::Protocol => Old::Protocol,
            OperationErrorCode::OutcomeUnknown => Old::OutcomeUnknown,
            OperationErrorCode::RateLimited => Old::RateLimited,
        };
        wire::OperationError {
            code,
            message: self.message,
            retriable: self.retriable,
            retry_after_seconds: self.retry_after_seconds,
        }
    }
}

impl From<wire::OperationError> for OperationError {
    fn from(value: wire::OperationError) -> Self {
        use wire::OperationErrorCode as Old;
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
            Old::RateLimited => OperationErrorCode::RateLimited,
        };
        Self {
            code,
            message: value.message,
            retriable: value.retriable,
            retry_after_seconds: value.retry_after_seconds,
            authentication: None,
        }
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
            protocol: CONTRACT.into(),
            request_id: request_id.into(),
            status: ResponseStatus::Ok,
            response: Some(response),
            error: None,
        }
    }
    #[must_use]
    pub fn failure(request_id: impl Into<String>, error: OperationError) -> Self {
        Self {
            protocol: CONTRACT.into(),
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
            error.validate()?;
        }
        self.clone()
            .into_v2()
            .validate()
            .map_err(OperationError::from)?;
        if serde_json::to_vec(self).map_or(true, |bytes| bytes.len() > wire::MAX_RESULT_BYTES) {
            return Err(OperationError::new(
                OperationErrorCode::ResultTooLarge,
                "operation response exceeds the admitted bound",
                false,
            ));
        }
        Ok(())
    }
    #[must_use]
    pub fn into_v2(self) -> wire::ResponseEnvelope {
        wire::ResponseEnvelope {
            protocol: wire::CONTRACT.into(),
            request_id: self.request_id,
            status: self.status,
            response: self.response,
            error: self.error.map(OperationError::into_v2),
        }
    }
}

impl From<wire::ResponseEnvelope> for ResponseEnvelope {
    fn from(value: wire::ResponseEnvelope) -> Self {
        Self {
            protocol: CONTRACT.into(),
            request_id: value.request_id,
            status: value.status,
            response: value.response,
            error: value.error.map(Into::into),
        }
    }
}

pub(super) fn protocol_refusal() -> OperationError {
    OperationError::new(
        OperationErrorCode::Protocol,
        "operation protocol identity or framing is invalid",
        false,
    )
}
