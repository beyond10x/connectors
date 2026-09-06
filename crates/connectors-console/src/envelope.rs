//! Reducing a transport envelope to the payload a caller asked for, or to a refusal.
//!
//! # Two defects this fixed
//!
//! Every result used to be printed as its envelope — `protocol`, `request_id` and `status` wrapped
//! around the payload — so `-o compact` rendered transport metadata instead of records, and `-o
//! text` buried the answer three levels down.
//!
//! Worse: a `status: error` envelope was printed as though it were a result **and the process
//! exited `0`**. `connectors connection list | jq` reported success for a request the daemon had
//! refused by name. Both are one fix, because the step that drops the metadata is the step that
//! notices the refusal.
//!
//! # Why a macro over three types
//!
//! The three protocols carry structurally identical envelopes with distinct types. Matching them
//! generically over `serde_json::Value` would work today and would stop compiling nothing on the
//! day a contract changes shape, which is the day it matters.

/// Reduce one protocol envelope to its payload, or to the Connector's own refusal.
///
/// Expands to `Result<serde_json::Value, ReducedError>`. The caller maps `ReducedError` into
/// whatever its own error surface is; the point is that a refusal cannot be mistaken for a result.
#[macro_export]
macro_rules! reduce_envelope {
    ($envelope:expr, connection) => {{
        let envelope = $crate::envelope::without_instruction_endpoints($envelope);
        $crate::reduce_envelope!(envelope)
    }};
    ($envelope:expr, operation) => {{
        $crate::envelope::OperationEnvelope::reduce($envelope)
    }};
    ($envelope:expr) => {{
        let envelope = $envelope;
        match (envelope.status, envelope.response, envelope.error) {
            (_, _, Some(error)) => Err($crate::envelope::ReducedError {
                code: ::serde_json::to_value(error.code)
                    .ok()
                    .and_then(|value| value.as_str().map(::std::borrow::ToOwned::to_owned))
                    .unwrap_or_else(|| "refused".to_owned()),
                message: error.message,
                retriable: error.retriable,
                retry_after_seconds: None,
                authentication: None,
            }),
            (_, Some(result), None) => ::serde_json::to_value(result)
                .map($crate::output::payload)
                .map_err(|error| $crate::envelope::ReducedError {
                    code: "malformed-response".to_owned(),
                    message: error.to_string(),
                    retriable: false,
                    retry_after_seconds: None,
                    authentication: None,
                }),
            (_, None, None) => Err($crate::envelope::ReducedError {
                code: "malformed-response".to_owned(),
                message: "the Connector returned neither a result nor an error".to_owned(),
                retriable: false,
                retry_after_seconds: None,
                authentication: None,
            }),
        }
    }};
}

/// Ordinary result renderers never receive one-use instruction capabilities or completion paths.
/// Trusted acquisition consumes the original typed response through its private client workflow.
#[must_use]
pub fn without_instruction_endpoints(
    mut envelope: protocol::connection::ResponseEnvelope,
) -> protocol::connection::ResponseEnvelope {
    if let Some(
        protocol::connection::ConnectionResult::ConnectSessionCreate(status)
        | protocol::connection::ConnectionResult::ConnectSessionStatus(status),
    ) = envelope.response.as_mut()
    {
        status.browser_completion_url = None;
        status.completion_endpoint = None;
    }
    envelope
}

/// The Connector answered, and its answer was a refusal.
///
/// Distinct from being unable to reach the Connector at all. The `code` is the contract's own
/// vocabulary, forwarded rather than reinterpreted: it is more precise than anything this layer
/// could invent, and a script branching on it should see what the daemon said.
#[derive(Debug, Clone, thiserror::Error)]
#[error("{message}")]
pub struct ReducedError {
    pub code: String,
    pub message: String,
    pub retriable: bool,
    pub retry_after_seconds: Option<u64>,
    pub authentication: Option<AuthenticationFacts>,
}

/// Reference-free model facts. No daemon string, private endpoint or caller input can fit here.
#[derive(Debug, Clone, serde::Serialize)]
pub struct AuthenticationFacts {
    need: protocol::operation::v3::AuthenticationNeed,
    attempt: protocol::operation::v3::AuthenticationAttemptState,
    next_action: protocol::operation::v3::AuthenticationNextAction,
}

/// Typed reduction keeps predecessor rate semantics separate from the v3 public projection.
pub trait OperationEnvelope {
    fn reduce(self) -> Result<serde_json::Value, ReducedError>;
}

impl OperationEnvelope for protocol::operation::ResponseEnvelope {
    fn reduce(self) -> Result<serde_json::Value, ReducedError> {
        let delay = self
            .error
            .as_ref()
            .and_then(|error| error.retry_after_seconds);
        crate::reduce_envelope!(self).map_err(|mut error| {
            error.retry_after_seconds = delay;
            error
        })
    }
}

impl OperationEnvelope for protocol::operation::v3::ResponseEnvelope {
    fn reduce(self) -> Result<serde_json::Value, ReducedError> {
        use protocol::operation::v3::OperationErrorCode as Code;
        if self.validate().is_err() {
            return Err(ReducedError {
                code: "malformed-response".into(),
                message: "the Connector returned an invalid operation response".into(),
                retriable: false,
                retry_after_seconds: None,
                authentication: None,
            });
        }
        if let Some(error) = self.error {
            let message = match error.code {
                Code::AuthenticationRequired => {
                    "authentication is required; the operation was not attempted"
                }
                Code::OutcomeUnknown => {
                    "the operation outcome is unknown; do not automatically repeat it"
                }
                Code::RateLimited => "the provider refused this operation because of a rate limit",
                Code::ApprovalRequired => "the operation requires approval",
                Code::ApprovalDenied => "operation approval was refused",
                Code::NotGranted => "the operation was not granted",
                Code::InvalidInput => "the operation input was invalid",
                Code::StaleAuthority => "the operation description or authority is stale",
                Code::NotFound => "the operation was not found",
                Code::Unavailable => "the operation is unavailable",
                Code::ResultTooLarge => "the operation result exceeded its bound",
                Code::Protocol => "the operation protocol request was refused",
            };
            return Err(ReducedError {
                code: serde_json::to_value(error.code)
                    .ok()
                    .and_then(|value| value.as_str().map(str::to_owned))
                    .unwrap_or_else(|| "refused".into()),
                message: message.into(),
                retriable: error.retriable,
                retry_after_seconds: error.retry_after_seconds,
                authentication: error.authentication.map(|value| AuthenticationFacts {
                    need: value.need,
                    attempt: value.attempt,
                    next_action: value.next_action,
                }),
            });
        }
        // Ordinary successful operation results retain their existing contract.
        crate::reduce_envelope!(self)
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    /// A stand-in with the shape every protocol envelope has, so the macro's behaviour is asserted
    /// without pulling a protocol crate into this test.
    struct Envelope {
        status: &'static str,
        response: Option<serde_json::Value>,
        error: Option<Error>,
    }
    struct Error {
        code: &'static str,
        message: String,
        retriable: bool,
    }

    #[test]
    fn a_refusal_becomes_an_error_rather_than_a_result() {
        let envelope = Envelope {
            status: "error",
            response: None,
            error: Some(Error {
                code: "not_found",
                message: "no Integration owns this Connection request".to_owned(),
                retriable: false,
            }),
        };
        let reduced = reduce_envelope!(envelope);
        let error = reduced.expect_err("a refusal must not reduce to a result");
        assert_eq!(
            error.code, "not_found",
            "the Connector's own code is forwarded"
        );
    }

    #[test]
    fn a_result_loses_its_envelope_and_its_discriminant() {
        let envelope = Envelope {
            status: "ok",
            response: Some(json!({"result": "search", "value": {"connections": []}})),
            error: None,
        };
        let reduced = reduce_envelope!(envelope).expect("a result reduces");
        assert_eq!(
            reduced,
            json!({"connections": []}),
            "neither the transport envelope nor the enum tag reaches the caller"
        );
    }

    #[test]
    fn an_envelope_carrying_neither_is_a_named_failure_not_an_empty_success() {
        let envelope = Envelope {
            status: "ok",
            response: None,
            error: None,
        };
        let error = reduce_envelope!(envelope).expect_err("must not succeed");
        assert_eq!(error.code, "malformed-response");
    }
}

#[cfg(test)]
mod personal_oauth_tests {
    use protocol::connection::*;

    #[test]
    fn ordinary_connection_result_payload_has_no_private_instruction_endpoint() {
        for create in [true, false] {
            let status = ConnectSessionStatus {
                connect_session_ref: "connect-session:fixture".into(),
                integration_ref: "gitlab".into(),
                state: ConnectSessionState::Pending,
                expires_at_unix_ms: 1_000,
                completion_endpoint: Some("/private/one-use.sock".into()),
                browser_completion_url: Some(
                    "http://127.0.0.1:18423/#token=PRIVATE-SENTINEL".into(),
                ),
                connection_ref: None,
            };
            let result = if create {
                ConnectionResult::ConnectSessionCreate(status)
            } else {
                ConnectionResult::ConnectSessionStatus(status)
            };
            let reduced = crate::reduce_envelope!(
                ResponseEnvelope::success("request:fixture", result),
                connection
            )
            .unwrap();
            let serialized = serde_json::to_string(&reduced).unwrap();
            assert!(!serialized.contains("PRIVATE-SENTINEL"));
            assert!(!serialized.contains("one-use.sock"));
            assert!(!serialized.contains("browser_completion_url"));
            assert!(!serialized.contains("completion_endpoint"));
            assert!(serialized.contains("connect-session:fixture"));
        }
    }
}
