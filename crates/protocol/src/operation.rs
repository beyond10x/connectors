//! Credential-free operation protocol for local Agent and other clean-room consumers.

// The normative field-level documentation is the released JSON Schema beside this reader. Keeping
// the Rust projection mechanically aligned with those exact wire names is more valuable than a
// second prose copy that can drift independently.
#![allow(missing_docs)]

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Exact protocol identity. A different value is a different contract.
pub const CONTRACT: &str = "b10x.connector-operation.v0alpha1";
pub const MAX_FRAME_BYTES: usize = 512 * 1024;
pub const MAX_ARGUMENT_BYTES: usize = 64 * 1024;
pub const MAX_RESULT_BYTES: usize = 256 * 1024;
pub const MAX_SEARCH_RESULTS: u16 = 25;
pub const MAX_CONNECTION_AUDIENCES: usize = 16;
const MAX_REFERENCE_BYTES: usize = 512;

/// Owner facts presented by a client and re-evaluated by Connectors.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OwnerContext {
    pub tenant_id: String,
    pub agent_id: String,
    pub agent_revision: u64,
    pub authority_snapshot_id: String,
    pub authority_snapshot_sha256: String,
}

/// One strict request frame. Local transport identity is additional evidence, never replaced by
/// the caller-written owner context.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RequestEnvelope {
    pub protocol: String,
    pub request_id: String,
    pub context: OwnerContext,
    pub request: OperationRequest,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(
    tag = "method",
    content = "params",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum OperationRequest {
    Search(SearchRequest),
    Describe(DescribeRequest),
    Invoke(InvokeRequest),
    SessionStatus(SessionRequest),
    SessionTerminate(SessionTerminateRequest),
    SessionReconcile(SessionRequest),
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
    pub operation_ref: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InvokeRequest {
    pub operation_ref: String,
    pub connection_ref: String,
    /// Opaque description lease returned by `describe`; stale catalog/authority leases refuse.
    pub description_ref: String,
    pub input: Value,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub approval_evidence_ref: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SessionRequest {
    pub execution_ref: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SessionTerminateRequest {
    pub execution_ref: String,
    pub reason: RequestedSessionTermination,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RequestedSessionTermination {
    Completed,
    Cancelled,
    Revoked,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EffectClass {
    ReadOnly,
    Mutating,
    Destructive,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ApprovalPosture {
    NotRequired,
    Required,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConnectionSummary {
    pub connection_ref: String,
    pub label: String,
    /// Target Provider whose API semantics this Connection exposes. This does not select a route.
    pub provider: String,
    /// Curated Explorer filters from the Provider catalog. Never authorization or visibility.
    pub audiences: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OperationSummary {
    pub operation_ref: String,
    pub title: String,
    pub effect: EffectClass,
    pub approval: ApprovalPosture,
    pub connections: Vec<ConnectionSummary>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InvocationResult {
    pub operation_ref: String,
    pub output: Value,
    pub connector_audit_ref: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub execution_ref: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionState {
    Establishing,
    Established,
    Terminating,
    Terminated,
    OutcomeUnknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionTermination {
    Completed,
    Cancelled,
    Revoked,
    LeaseExpired,
    RemoteEnded,
    Failed,
    OutcomeUnknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SessionStatus {
    pub execution_ref: String,
    pub operation_ref: String,
    pub connection_ref: String,
    pub state: SessionState,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub termination: Option<SessionTermination>,
    pub connector_audit_ref: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
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
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, thiserror::Error)]
#[error("{code:?}: {message}")]
#[serde(deny_unknown_fields)]
pub struct OperationError {
    pub code: OperationErrorCode,
    pub message: String,
    pub retriable: bool,
}

impl OperationError {
    #[must_use]
    pub fn new(code: OperationErrorCode, message: impl Into<String>, retriable: bool) -> Self {
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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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

impl RequestEnvelope {
    /// Validate bounds and exact protocol identity before authority lookup or backend work.
    pub fn validate(&self) -> Result<(), OperationError> {
        if self.protocol != CONTRACT || !valid_ref(&self.request_id, 128) {
            return Err(protocol_refusal());
        }
        validate_context(&self.context)?;
        match &self.request {
            OperationRequest::Search(request) => {
                if request.query.len() > 512
                    || request.limit == 0
                    || request.limit > MAX_SEARCH_RESULTS
                {
                    return Err(invalid_input("search bounds are invalid"));
                }
            }
            OperationRequest::Describe(request) => {
                require_ref(&request.operation_ref)?;
            }
            OperationRequest::Invoke(request) => {
                require_ref(&request.operation_ref)?;
                require_ref(&request.connection_ref)?;
                require_ref(&request.description_ref)?;
                if request
                    .approval_evidence_ref
                    .as_deref()
                    .is_some_and(|value| !valid_ref(value, MAX_REFERENCE_BYTES))
                    || serde_json::to_vec(&request.input)
                        .map_or(true, |value| value.len() > MAX_ARGUMENT_BYTES)
                {
                    return Err(invalid_input(
                        "invoke input or approval evidence is invalid",
                    ));
                }
            }
            OperationRequest::SessionStatus(request)
            | OperationRequest::SessionReconcile(request) => {
                require_ref(&request.execution_ref)?;
            }
            OperationRequest::SessionTerminate(request) => {
                require_ref(&request.execution_ref)?;
            }
        }
        Ok(())
    }
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
        if self.protocol != CONTRACT || !valid_ref(&self.request_id, 128) {
            return Err(protocol_refusal());
        }
        match (self.status, &self.response, &self.error) {
            (ResponseStatus::Ok, Some(response), None) => validate_result(response)?,
            (ResponseStatus::Error, None, Some(error)) => {
                if error.message.is_empty() || error.message.len() > 4096 {
                    return Err(protocol_refusal());
                }
            }
            _ => return Err(protocol_refusal()),
        }
        let bytes = serde_json::to_vec(self).map_err(|_| protocol_refusal())?;
        if bytes.len() > MAX_RESULT_BYTES {
            return Err(OperationError::new(
                OperationErrorCode::ResultTooLarge,
                "operation response exceeds the admitted bound",
                false,
            ));
        }
        Ok(())
    }
}

fn validate_result(result: &OperationResult) -> Result<(), OperationError> {
    match result {
        OperationResult::Search { operations } => {
            if operations.len() > usize::from(MAX_SEARCH_RESULTS) {
                return Err(protocol_refusal());
            }
            for operation in operations {
                validate_summary(operation)?;
            }
        }
        OperationResult::Describe(description) => {
            validate_summary(&OperationSummary {
                operation_ref: description.operation_ref.clone(),
                title: description.title.clone(),
                effect: description.effect,
                approval: description.approval,
                connections: description.connections.clone(),
            })?;
            if description.description.len() > 16_384
                || !valid_ref(&description.description_ref, MAX_REFERENCE_BYTES)
            {
                return Err(protocol_refusal());
            }
        }
        OperationResult::Invoke(invocation) => {
            if !valid_ref(&invocation.operation_ref, MAX_REFERENCE_BYTES)
                || !valid_ref(&invocation.connector_audit_ref, MAX_REFERENCE_BYTES)
                || invocation
                    .execution_ref
                    .as_deref()
                    .is_some_and(|value| !valid_ref(value, MAX_REFERENCE_BYTES))
            {
                return Err(protocol_refusal());
            }
        }
        OperationResult::SessionStatus(status)
        | OperationResult::SessionTerminate(status)
        | OperationResult::SessionReconcile(status) => validate_status(status)?,
    }
    Ok(())
}

fn validate_summary(summary: &OperationSummary) -> Result<(), OperationError> {
    if !valid_ref(&summary.operation_ref, MAX_REFERENCE_BYTES)
        || summary.title.is_empty()
        || summary.title.len() > 1024
        || (matches!(
            summary.effect,
            EffectClass::Mutating | EffectClass::Destructive
        ) && summary.approval != ApprovalPosture::Required)
        || summary.connections.len() > 64
        || summary.connections.iter().any(|connection| {
            !valid_ref(&connection.connection_ref, MAX_REFERENCE_BYTES)
                || connection.label.is_empty()
                || connection.label.len() > 1024
                || !valid_ref(&connection.provider, 128)
                || connection.audiences.len() > MAX_CONNECTION_AUDIENCES
                || connection
                    .audiences
                    .iter()
                    .enumerate()
                    .any(|(index, audience)| {
                        !valid_ref(audience, 64) || connection.audiences[..index].contains(audience)
                    })
        })
    {
        return Err(protocol_refusal());
    }
    Ok(())
}

fn validate_status(status: &SessionStatus) -> Result<(), OperationError> {
    if !valid_ref(&status.execution_ref, MAX_REFERENCE_BYTES)
        || !valid_ref(&status.operation_ref, MAX_REFERENCE_BYTES)
        || !valid_ref(&status.connection_ref, MAX_REFERENCE_BYTES)
        || !valid_ref(&status.connector_audit_ref, MAX_REFERENCE_BYTES)
    {
        return Err(protocol_refusal());
    }
    let coherent = match status.state {
        SessionState::Establishing | SessionState::Established | SessionState::Terminating => {
            status.termination.is_none()
        }
        SessionState::Terminated => status
            .termination
            .is_some_and(|reason| reason != SessionTermination::OutcomeUnknown),
        SessionState::OutcomeUnknown => {
            status.termination == Some(SessionTermination::OutcomeUnknown)
        }
    };
    if !coherent {
        return Err(protocol_refusal());
    }
    Ok(())
}

fn validate_context(context: &OwnerContext) -> Result<(), OperationError> {
    if !valid_ref(&context.tenant_id, 256)
        || !valid_ref(&context.agent_id, 256)
        || context.agent_revision == 0
        || !valid_ref(&context.authority_snapshot_id, 256)
        || context.authority_snapshot_sha256.len() != 64
        || !context
            .authority_snapshot_sha256
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err(OperationError::new(
            OperationErrorCode::StaleAuthority,
            "owner authority context is invalid",
            false,
        ));
    }
    Ok(())
}

fn require_ref(value: &str) -> Result<(), OperationError> {
    if valid_ref(value, MAX_REFERENCE_BYTES) {
        Ok(())
    } else {
        Err(invalid_input("operation reference is invalid"))
    }
}

fn valid_ref(value: &str, maximum: usize) -> bool {
    !value.is_empty() && value.len() <= maximum && value.bytes().all(|byte| byte.is_ascii_graphic())
}

fn invalid_input(message: &'static str) -> OperationError {
    OperationError::new(OperationErrorCode::InvalidInput, message, false)
}

fn protocol_refusal() -> OperationError {
    OperationError::new(
        OperationErrorCode::Protocol,
        "operation protocol identity or framing is invalid",
        false,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn context() -> OwnerContext {
        OwnerContext {
            tenant_id: "tenant-1".to_owned(),
            agent_id: "agent-1".to_owned(),
            agent_revision: 1,
            authority_snapshot_id: "authority-1".to_owned(),
            authority_snapshot_sha256: "a".repeat(64),
        }
    }

    #[test]
    fn invoke_requires_a_description_lease_and_bounded_structured_input() {
        let mut request = RequestEnvelope {
            protocol: CONTRACT.to_owned(),
            request_id: "request-1".to_owned(),
            context: context(),
            request: OperationRequest::Invoke(InvokeRequest {
                operation_ref: "sip.dial".to_owned(),
                connection_ref: "connection-asterisk-dev".to_owned(),
                description_ref: "description-1".to_owned(),
                input: serde_json::json!({"target": "asterisk-dev"}),
                approval_evidence_ref: Some("approval-1".to_owned()),
            }),
        };
        request.validate().unwrap();
        if let OperationRequest::Invoke(invoke) = &mut request.request {
            invoke.description_ref.clear();
        }
        assert_eq!(
            request.validate().unwrap_err().code,
            OperationErrorCode::InvalidInput
        );
    }

    #[test]
    fn owner_context_is_not_defaultable() {
        let request = RequestEnvelope {
            protocol: CONTRACT.to_owned(),
            request_id: "request-1".to_owned(),
            context: OwnerContext {
                authority_snapshot_sha256: String::new(),
                ..context()
            },
            request: OperationRequest::Search(SearchRequest {
                query: "sip".to_owned(),
                limit: 10,
            }),
        };
        assert_eq!(
            request.validate().unwrap_err().code,
            OperationErrorCode::StaleAuthority
        );
    }

    #[test]
    fn a_terminal_status_cannot_omit_its_observed_reason() {
        let response = ResponseEnvelope::success(
            "request-1",
            OperationResult::SessionStatus(SessionStatus {
                execution_ref: "execution-1".to_owned(),
                operation_ref: "sip.dial".to_owned(),
                connection_ref: "connection-1".to_owned(),
                state: SessionState::Terminated,
                termination: None,
                connector_audit_ref: "audit-1".to_owned(),
            }),
        );
        assert_eq!(
            response.validate().unwrap_err().code,
            OperationErrorCode::Protocol
        );
    }

    #[test]
    fn response_envelope_round_trips_and_refuses_unknown_fields() {
        let response = ResponseEnvelope::success(
            "request-1",
            OperationResult::Search {
                operations: Vec::new(),
            },
        );
        let mut value = serde_json::to_value(response).unwrap();
        serde_json::from_value::<ResponseEnvelope>(value.clone())
            .unwrap()
            .validate()
            .unwrap();
        value["ambient_secret"] = serde_json::json!("must-refuse");
        assert!(serde_json::from_value::<ResponseEnvelope>(value).is_err());
    }

    #[test]
    fn connection_audiences_are_bounded_discovery_metadata() {
        let mut response = ResponseEnvelope::success(
            "request-search-1",
            OperationResult::Search {
                operations: vec![OperationSummary {
                    operation_ref: "prometheus-query-range".to_owned(),
                    title: "Query Prometheus range data".to_owned(),
                    effect: EffectClass::ReadOnly,
                    approval: ApprovalPosture::NotRequired,
                    connections: vec![ConnectionSummary {
                        connection_ref: "connection:prometheus:infra".to_owned(),
                        label: "Infrastructure metrics".to_owned(),
                        provider: "prometheus".to_owned(),
                        audiences: vec!["sre".to_owned(), "developer".to_owned()],
                    }],
                }],
            },
        );
        response.validate().unwrap();
        let OperationResult::Search { operations } = response.response.as_mut().unwrap() else {
            unreachable!();
        };
        operations[0].connections[0]
            .audiences
            .push("sre".to_owned());
        assert_eq!(
            response.validate().unwrap_err().code,
            OperationErrorCode::Protocol
        );
    }

    #[test]
    fn effect_bearing_operations_require_approval() {
        for effect in [EffectClass::Mutating, EffectClass::Destructive] {
            let response = ResponseEnvelope::success(
                "request-search-1",
                OperationResult::Search {
                    operations: vec![OperationSummary {
                        operation_ref: "colab.rooms.create".to_owned(),
                        title: "Create a conversation room".to_owned(),
                        effect,
                        approval: ApprovalPosture::NotRequired,
                        connections: Vec::new(),
                    }],
                },
            );
            assert_eq!(
                response.validate().unwrap_err().code,
                OperationErrorCode::Protocol
            );
        }
    }
}
