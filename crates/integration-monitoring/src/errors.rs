pub(crate) fn connection_unavailable() -> ConnectionError {
    ConnectionError::new(
        ConnectionErrorCode::Unavailable,
        "connection management is temporarily unavailable",
        true,
    )
}

pub(crate) fn connect_session_error(error: ConnectSessionLifecycleError) -> ConnectionError {
    match error {
        ConnectSessionLifecycleError::Capacity => ConnectionError::new(
            ConnectionErrorCode::Conflict,
            "too many Connect Sessions are pending",
            true,
        ),
        _ => connection_unavailable(),
    }
}

pub(crate) fn connection_not_found() -> ConnectionError {
    ConnectionError::new(
        ConnectionErrorCode::NotFound,
        "connection, observation, or Connect Session was not found",
        false,
    )
}

pub(crate) fn connection_protocol() -> ConnectionError {
    ConnectionError::new(
        ConnectionErrorCode::Protocol,
        "connection backend returned an incompatible response",
        false,
    )
}

pub(crate) fn operation_unavailable() -> OperationError {
    OperationError::new(
        OperationErrorCode::Unavailable,
        "monitoring connector runtime is unavailable",
        true,
    )
}

pub(crate) fn operation_not_found() -> OperationError {
    OperationError::new(
        OperationErrorCode::NotFound,
        "monitoring operation was not found",
        false,
    )
}

pub(crate) fn operation_not_granted() -> OperationError {
    OperationError::new(
        OperationErrorCode::NotGranted,
        "operation is not granted for this Connection",
        false,
    )
}

pub(crate) fn operation_invalid() -> OperationError {
    OperationError::new(
        OperationErrorCode::InvalidInput,
        "operation input is outside the catalog contract",
        false,
    )
}

/// Why an upstream HTTP exchange failed, reduced to redaction-safe facts. The status number and
/// the transport class are the only upstream details that may leave this seam: the response body
/// can carry provider data or a credential echo and the transport error's Display string can
/// embed the URL, so neither travels past it (S-065, S-066).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum UpstreamFailure {
    /// The egress transport failed before any upstream status arrived; the class (timeout,
    /// connect, tls, body-read, other) is the last discriminator of a live diagnosis (S-066).
    Transport(EgressTransportFailure),
    /// The upstream answered, but its body exceeded the executor's admitted result bound. Not a
    /// transport failure: the upstream was reachable and talkative — too talkative (S-066).
    ResponseTooLarge,
    /// The upstream answered outside 2xx; the body is dropped, only the status travels.
    Status(u16),
    /// The upstream answered 2xx with a body that does not parse as JSON.
    Body,
}

/// Why a monitoring dispatch was refused after validation and planning succeeded.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RefusalCause {
    Upstream(UpstreamFailure),
    /// The credential store failed for a reason other than an absent credential.
    CredentialCustody,
}

impl From<UpstreamFailure> for RefusalCause {
    fn from(failure: UpstreamFailure) -> Self {
        Self::Upstream(failure)
    }
}

/// One structured refusal line for the pod log, in the readiness line's JSON-object shape.
/// It names the operation, the route, and the redaction-safe failure classification; the exact
/// status makes a 401 distinguishable from a 403 or 404 in `kubectl logs`, and the transport
/// class makes a timeout distinguishable from a refused connect or a broken handshake (S-066).
/// Never the upstream body — it may carry provider data — never the transport error's Display
/// string — it can embed the URL — and never credential material.
pub(crate) fn refusal_log_line(operation_ref: &str, route: &str, cause: RefusalCause) -> Value {
    let mut line = serde_json::json!({
        "event": "monitoring_dispatch_refused",
        "operation_ref": operation_ref,
        "route": route,
        "cause": match cause {
            RefusalCause::Upstream(UpstreamFailure::Transport(_)) => "upstream-transport",
            RefusalCause::Upstream(UpstreamFailure::ResponseTooLarge) => {
                "upstream-response-too-large"
            }
            RefusalCause::Upstream(UpstreamFailure::Status(_)) => "upstream-status",
            RefusalCause::Upstream(UpstreamFailure::Body) => "upstream-body",
            RefusalCause::CredentialCustody => "credential-custody",
        },
    });
    match cause {
        RefusalCause::Upstream(UpstreamFailure::Status(status)) => {
            line["upstream_status"] = Value::from(status);
        }
        RefusalCause::Upstream(UpstreamFailure::Transport(class)) => {
            line["transport_class"] = Value::from(class.as_str());
        }
        _ => {}
    }
    line
}

/// Log one refusal line and produce the protocol answer. The code stays `unavailable` and
/// retriable — the published contract — while the message names the failure class: upstream
/// refused (status class), upstream unreachable, malformed upstream body, or credential
/// custody failure. The client-facing message carries only the status class; the exact status
/// and the transport class stay in the server-side log line. The one exception is a result-bound
/// breach (S-066): a retry of the identical request can only breach again, so it answers
/// `result_too_large` and not-retriable, the same contract the Kubernetes integration publishes.
pub(crate) fn refuse_dispatch(
    operation_ref: &str,
    route: &str,
    cause: RefusalCause,
) -> OperationError {
    eprintln!("{}", refusal_log_line(operation_ref, route, cause));
    let message = match cause {
        RefusalCause::Upstream(UpstreamFailure::Transport(_)) => {
            "monitoring upstream is unreachable".to_owned()
        }
        RefusalCause::Upstream(UpstreamFailure::ResponseTooLarge) => {
            return OperationError::new(
                OperationErrorCode::ResultTooLarge,
                "monitoring upstream response exceeds the result bound",
                false,
            );
        }
        RefusalCause::Upstream(UpstreamFailure::Status(status)) => format!(
            "monitoring upstream refused the dispatch (upstream_status {}xx)",
            status / 100
        ),
        RefusalCause::Upstream(UpstreamFailure::Body) => {
            "monitoring upstream returned a non-JSON response".to_owned()
        }
        RefusalCause::CredentialCustody => "monitoring credential custody failed".to_owned(),
    };
    OperationError::new(OperationErrorCode::Unavailable, message, true)
}

pub(crate) fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(|poisoned| {
        mutex.clear_poison();
        poisoned.into_inner()
    })
}
use std::sync::{Mutex, MutexGuard};

use protocol::connection::{ConnectionError, ConnectionErrorCode};
use protocol::operation::{OperationError, OperationErrorCode};
use serde_json::Value;
use service::{ConnectSessionLifecycleError, EgressTransportFailure};
