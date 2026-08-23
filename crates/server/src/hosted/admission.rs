//! The hosted route's admission seams: proven dispatch and its rechecks (S-047, S-049).
//!
//! Everything here serves the `/operations` route's admission flow. The two dispatch functions
//! are the only seams through which an admitted request reaches a backend, each consuming its
//! [`AdmittedOperation`] proof by value; the rechecks (`redescribe`, `session_for_signal`)
//! resolve the Connector's own account of what is being decided, never the caller's claims; and
//! [`enforcement_refusal_response`] renders every enforcement refusal as one axis-free body.
//! The decisions themselves live in [`super::enforcement`] — this module holds only the route
//! side of the seam.

use axum::http::StatusCode;
use axum::response::Response;
use domain::AdmittedOperation;
use protocol::operation::{
    DescribeRequest, InvokeRequest, OperationDescription, OperationError, OperationErrorCode,
    OperationRequest, OperationResult, SessionRequest, SessionSignalRequest, SessionStatus,
};
use service::{ConnectorBackend, PrincipalContext};

use super::enforcement::EnforcementRefusal;
use super::{operation_failure, HostedState};

/// Dispatch one proven invocation, consuming the [`AdmittedOperation`] by value.
///
/// This is the only seam through which an invocation the authority admitted as effect-capable
/// reaches a backend, and it holds the proof for exactly the length of dispatch (S-047). The
/// proof's own operation and Connection must be the ones dispatched — the description lease
/// already binds them, and repeating the identity here keeps the proof load-bearing rather than
/// ceremonial.
pub(super) async fn dispatch_admitted(
    backend: &dyn ConnectorBackend,
    owner: &PrincipalContext,
    operation: AdmittedOperation,
    invoke: InvokeRequest,
) -> Result<OperationResult, OperationError> {
    if operation.operation() != invoke.operation_ref
        || operation.connection() != invoke.connection_ref
    {
        return Err(OperationError::new(
            OperationErrorCode::NotGranted,
            "the admission proof does not cover this invocation",
            false,
        ));
    }
    backend
        .handle(owner, OperationRequest::Invoke(invoke))
        .await
}

/// Dispatch one proven session signal, consuming the [`AdmittedOperation`] by value.
///
/// The signal twin of [`dispatch_admitted`] (S-049): the proof's operation and Connection must
/// be the session's own, and the session must be the one the signal addresses — repeating the
/// identity here keeps the proof load-bearing rather than ceremonial.
pub(super) async fn dispatch_admitted_signal(
    backend: &dyn ConnectorBackend,
    owner: &PrincipalContext,
    admission: Option<(SessionStatus, Box<AdmittedOperation>)>,
    signal: SessionSignalRequest,
) -> Result<OperationResult, OperationError> {
    // A missing proof is unreachable by construction — the admission block refused already —
    // and refuses rather than dispatches if a future edit breaks that construction.
    let Some((session, operation)) = admission else {
        return Err(signal_proof_gap());
    };
    if operation.operation() != session.operation_ref
        || operation.connection() != session.connection_ref
        || session.execution_ref != signal.execution_ref
    {
        return Err(signal_proof_gap());
    }
    backend
        .handle(owner, OperationRequest::SessionSignal(signal))
        .await
}

fn signal_proof_gap() -> OperationError {
    OperationError::new(
        OperationErrorCode::NotGranted,
        "the admission proof does not cover this session signal",
        false,
    )
}

/// Resolve the live session a signal addresses, through the Connector backend's own record.
///
/// A session nobody holds answers the same refusal a session no Grant admits: hosted callers
/// cannot read session state at all today, and the signal seam must not become the way to
/// probe which execution refs exist.
pub(super) async fn session_for_signal(
    state: &HostedState,
    owner: &PrincipalContext,
    request_id: &str,
    signal: &SessionSignalRequest,
) -> Result<SessionStatus, Box<Response>> {
    let status = state
        .backend
        .handle(
            owner,
            OperationRequest::SessionStatus(SessionRequest {
                execution_ref: signal.execution_ref.clone(),
            }),
        )
        .await;
    match status {
        Ok(OperationResult::SessionStatus(session)) => Ok(session),
        Ok(_) => Err(Box::new(operation_failure(
            request_id,
            wrong_recheck_result(),
            StatusCode::INTERNAL_SERVER_ERROR,
        ))),
        Err(error) if error.code == OperationErrorCode::NotFound => Err(Box::new(
            enforcement_refusal_response(request_id, EnforcementRefusal::NotAdmitted),
        )),
        Err(error) => Err(Box::new(operation_failure(
            request_id,
            error,
            StatusCode::SERVICE_UNAVAILABLE,
        ))),
    }
}

/// Re-describe one operation through the Connector backend for an admission decision. The
/// description is the Connector's own — never the caller's claims about the operation.
pub(super) async fn redescribe(
    state: &HostedState,
    owner: &PrincipalContext,
    request_id: &str,
    operation_ref: &str,
) -> Result<OperationDescription, Box<Response>> {
    let description = state
        .backend
        .handle(
            owner,
            OperationRequest::Describe(DescribeRequest {
                operation_ref: operation_ref.to_owned(),
            }),
        )
        .await;
    match description {
        Ok(OperationResult::Describe(description)) => Ok(description),
        Ok(_) => Err(Box::new(operation_failure(
            request_id,
            wrong_recheck_result(),
            StatusCode::INTERNAL_SERVER_ERROR,
        ))),
        Err(error) => Err(Box::new(operation_failure(
            request_id,
            error,
            StatusCode::SERVICE_UNAVAILABLE,
        ))),
    }
}

fn wrong_recheck_result() -> OperationError {
    OperationError::new(
        OperationErrorCode::Protocol,
        "the Connector backend returned the wrong result while rechecking invocation authority",
        false,
    )
}

/// One axis-free body for every enforcement 403, and one honest outage body for every 503:
/// grant, approval, replay and session-signal refusals render byte-identically, and replay is
/// distinct only in the gate's own journal.
pub(super) fn enforcement_refusal_response(
    request_id: &str,
    refusal: EnforcementRefusal,
) -> Response {
    match refusal {
        EnforcementRefusal::NotAdmitted => operation_failure(
            request_id,
            OperationError::new(
                OperationErrorCode::NotGranted,
                "the Connector authority does not admit this invocation",
                false,
            ),
            StatusCode::FORBIDDEN,
        ),
        EnforcementRefusal::Unavailable => operation_failure(
            request_id,
            OperationError::new(
                OperationErrorCode::Unavailable,
                "the Connector Grant authority is unavailable",
                true,
            ),
            StatusCode::SERVICE_UNAVAILABLE,
        ),
    }
}
