//! Local daemon process control, authenticated by the owner-only Unix socket.
//!
//! This contract never reaches a provider and carries no provider authority or credentials.

use serde::{Deserialize, Serialize};

/// Exact local lifecycle contract identity.
pub const CONTRACT: &str = "b10x.connector-lifecycle.v0alpha1";
/// Maximum request or response frame, excluding its newline.
pub const MAX_FRAME_BYTES: usize = 8192;

/// One owner-authenticated lifecycle request.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RequestEnvelope {
    /// Exact contract identity.
    pub protocol: String,
    /// Bounded caller correlation identifier.
    pub request_id: String,
    /// Process action, independent of provider grants.
    pub request: LifecycleRequest,
}

/// Actions supported only by the local daemon control socket.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(tag = "method", rename_all = "snake_case", deny_unknown_fields)]
pub enum LifecycleRequest {
    /// Inspect the running process without contacting any provider.
    Status {},
    /// Acknowledge and gracefully stop this exact daemon.
    Stop {},
}

/// Non-secret facts about the process that answered the request.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DaemonStatus {
    /// Process identity for display; clients must never use this to signal a process.
    pub process_id: u32,
    /// Running Connectors version.
    pub version: String,
    /// Explicit configuration path, if the runtime supplied one.
    pub configuration: Option<String>,
    /// Whether this reply acknowledged graceful shutdown.
    pub stopping: bool,
}

/// Correlated lifecycle reply, written before shutdown is requested.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ResponseEnvelope {
    /// Exact contract identity.
    pub protocol: String,
    /// Identifier copied from the request.
    pub request_id: String,
    /// Process metadata.
    pub response: DaemonStatus,
}

impl RequestEnvelope {
    /// Reject foreign contracts and unbounded or non-printable correlation identifiers.
    #[must_use]
    pub fn valid(&self) -> bool {
        self.protocol == CONTRACT && valid_id(&self.request_id)
    }
}

impl ResponseEnvelope {
    /// Validate identity, correlation and bounded process metadata.
    #[must_use]
    pub fn valid_for(&self, request_id: &str) -> bool {
        self.protocol == CONTRACT
            && self.request_id == request_id
            && valid_id(&self.request_id)
            && self.response.process_id != 0
            && !self.response.version.is_empty()
            && self.response.version.len() <= 128
            && self
                .response
                .configuration
                .as_ref()
                .is_none_or(|path| path.len() <= 4096 && !path.chars().any(char::is_control))
    }
}

fn valid_id(value: &str) -> bool {
    !value.is_empty() && value.len() <= 128 && value.bytes().all(|byte| byte.is_ascii_graphic())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lifecycle_rejects_extra_fields_and_foreign_contracts() {
        assert!(serde_json::from_str::<RequestEnvelope>(
            r#"{"protocol":"b10x.connector-lifecycle.v0alpha1","request_id":"r1","request":{"method":"stop","pid":1}}"#
        ).is_err());
        let request = RequestEnvelope {
            protocol: "other".into(),
            request_id: "r1".into(),
            request: LifecycleRequest::Status {},
        };
        assert!(!request.valid());
    }
}
