//! Exact-input human approval issuance for hosted Connector invocations.

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::operation::{OperationError, OwnerContext, MAX_ARGUMENT_BYTES};

/// Exact approval issuance contract identity.
pub const CONTRACT: &str = "b10x.connector-approval.v1";
/// Maximum request frame accepted by the hosted approval route.
pub const MAX_FRAME_BYTES: usize = 128 * 1024;
/// Maximum response frame returned by the hosted approval route.
pub const MAX_RESPONSE_BYTES: usize = 8 * 1024;
/// Longest approval lifetime a human may issue.
pub const MAX_TTL_SECONDS: u64 = 300;

/// Correlated request to issue one exact-input approval.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RequestEnvelope {
    /// Exact protocol identity.
    pub protocol: String,
    /// Caller-generated correlation identity.
    pub request_id: String,
    /// Tenant and agent provenance; receiver authentication remains authoritative.
    pub context: OwnerContext,
    /// Exact invocation a human approved.
    pub request: IssueRequest,
}

/// Invocation coordinates sealed into one approval record.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct IssueRequest {
    /// Connector-owned operation identity.
    pub operation_ref: String,
    /// Connector-owned credential-free Connection identity.
    pub connection_ref: String,
    /// Current description lease observed by the human-facing caller.
    pub description_ref: String,
    /// Exact input whose canonical digest is approved.
    pub input: Value,
    /// Requested finite validity period.
    pub ttl_seconds: u64,
}

/// Successful one-time approval issuance.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct IssuedApproval {
    /// Exact protocol identity.
    pub protocol: String,
    /// Echo of the request correlation identity.
    pub request_id: String,
    /// Opaque proof presented exactly once beside the invocation.
    pub approval_evidence_ref: String,
    /// Receiver-clock expiry as Unix seconds.
    pub expires_at_seconds: u64,
}

impl RequestEnvelope {
    /// Validate the closed request before authority lookup or backend work.
    pub fn validate(&self) -> Result<(), OperationError> {
        if self.protocol != CONTRACT
            || !valid_ref(&self.request_id, 128)
            || !valid_ref(&self.context.tenant_id, 512)
            || !valid_ref(&self.context.agent_id, 512)
            || self.context.agent_revision == 0
            || !valid_ref(&self.context.authority_snapshot_id, 512)
            || !is_digest(&self.context.authority_snapshot_sha256)
            || !valid_ref(&self.request.operation_ref, 512)
            || !valid_ref(&self.request.connection_ref, 512)
            || !valid_ref(&self.request.description_ref, 512)
            || !(1..=MAX_TTL_SECONDS).contains(&self.request.ttl_seconds)
            || serde_json::to_vec(&self.request.input)
                .map_or(true, |input| input.len() > MAX_ARGUMENT_BYTES)
        {
            return Err(OperationError::new(
                crate::operation::OperationErrorCode::InvalidInput,
                "approval issuance request is invalid",
                false,
            ));
        }
        Ok(())
    }
}

impl IssuedApproval {
    /// Validate correlation and the opaque proof shape at a client boundary.
    pub fn validate(&self, request_id: &str) -> bool {
        self.protocol == CONTRACT
            && self.request_id == request_id
            && valid_ref(&self.approval_evidence_ref, 512)
            && self.expires_at_seconds > 0
    }
}

fn valid_ref(value: &str, maximum: usize) -> bool {
    !value.is_empty() && value.len() <= maximum && value.bytes().all(|byte| byte.is_ascii_graphic())
}

fn is_digest(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request() -> RequestEnvelope {
        RequestEnvelope {
            protocol: CONTRACT.to_owned(),
            request_id: "request-1".to_owned(),
            context: OwnerContext {
                tenant_id: "tenant-1".to_owned(),
                agent_id: "agent-1".to_owned(),
                agent_revision: 1,
                authority_snapshot_id: "snapshot-1".to_owned(),
                authority_snapshot_sha256: "a".repeat(64),
            },
            request: IssueRequest {
                operation_ref: "todo.create_list".to_owned(),
                connection_ref: "connection:todo".to_owned(),
                description_ref: "description:todo".to_owned(),
                input: serde_json::json!({"title": "ship it"}),
                ttl_seconds: 120,
            },
        }
    }

    #[test]
    fn realm_is_not_an_approval_or_route_coordinate() {
        let encoded = serde_json::to_value(request()).unwrap();
        assert!(request().validate().is_ok());
        assert!(encoded.pointer("/context/realm").is_none());
        assert!(encoded.pointer("/request/realm").is_none());
    }

    #[test]
    fn approval_lifetime_is_bounded() {
        let mut request = request();
        request.request.ttl_seconds = MAX_TTL_SECONDS + 1;
        assert!(request.validate().is_err());
    }
}
