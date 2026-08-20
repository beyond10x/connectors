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
            }),
            (_, Some(result), None) => ::serde_json::to_value(result)
                .map($crate::output::payload)
                .map_err(|error| $crate::envelope::ReducedError {
                    code: "malformed-response".to_owned(),
                    message: error.to_string(),
                    retriable: false,
                }),
            (_, None, None) => Err($crate::envelope::ReducedError {
                code: "malformed-response".to_owned(),
                message: "the Connector returned neither a result nor an error".to_owned(),
                retriable: false,
            }),
        }
    }};
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
}

#[cfg(test)]
mod tests {
    use super::*;
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
        assert_eq!(error.code, "not_found", "the Connector's own code is forwarded");
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
