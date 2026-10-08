//! `POST /v1alpha2/invoke` (contracts/service/compatibility.md § 2.1). Every
//! admitted invocation, read or write, is anchored and acknowledged before the
//! adapter is called, and its one final observation is recorded after
//! (audit.md § 1-3). An admitted write also records its attempt before
//! dispatch and settles it after, and its Response carries the `mutation`
//! naming that attempt (ess/domains/mutations.yaml, "The HTTP host's attempt
//! ledger").
use super::{Service, admitted};
use crate::local::{
    audit, mutations,
    mutations::AttemptRef,
    service::{Admitted, ServiceState, Write},
};
use axum::{
    Json,
    body::Bytes,
    extract::State,
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response as HttpResponse},
};
use connectors_core::{
    RESPONSE_LIMIT,
    v1alpha2::{
        AttemptId, AttemptReference, AuditStatus, BaseErrorCode, CauseStage, DiagnosticCause,
        EffectKnowledge, Error, ErrorCode, InvokeRequest, InvokeResponse, MutationObservation,
        Refusal, ResponseStatus, Version,
    },
};
use connectors_sdk::validate;
use serde_json::Value;
use std::{
    sync::Arc,
    time::{Duration, Instant},
};

/// An `external_write` operation declares `profile: mutation`, and a read
/// profile cannot carry it (contracts/operations/v1alpha1/semantics.md § 2,
/// Effect declaration rule); the legacy Descriptor carries no `effects`.
const MUTATION_PROFILE: &str = "mutation";
const DEADLINE: Duration = Duration::from_secs(20);

pub(super) async fn invoke(
    State(service): State<Service>,
    headers: HeaderMap,
    body: std::result::Result<Bytes, axum::extract::rejection::BytesRejection>,
) -> HttpResponse {
    // A host without `state` answers before decoding (compatibility.md § 2.1).
    let Some(state) = service.state.clone() else {
        return refused(
            None,
            ErrorCode::Unavailable,
            "service state is not configured",
        );
    };
    if let Err(error) = admitted(&service, &headers).await {
        return refused(None, code(error.code), &error.message);
    }
    let Ok(bytes) = body else {
        return refused(None, ErrorCode::Capacity, "request exceeds byte limit");
    };
    let request = match InvokeRequest::decode(&bytes) {
        Ok(request) => request,
        Err(refusal) => return answer(refusal.into_response()),
    };
    if !connectors_core::valid_id(&request.request_id)
        || !connectors_core::valid_id(&request.operation)
    {
        return refused(None, ErrorCode::InvalidInput, "invalid request identifier");
    }
    let request_id = request.request_id.clone();
    let refuse = |code, message: &str| refused(Some(request_id.clone()), code, message);
    let descriptor = service.adapter.descriptor();
    if request.revision != descriptor.revision {
        return refuse(
            ErrorCode::StaleDescription,
            "refresh the descriptor before resubmitting",
        );
    }
    let operation = match descriptor.operation(&request.operation) {
        Ok(operation) => operation.clone(),
        Err(error) => return refuse(code(error.code), &error.message),
    };
    if let Err(error) = validate(&operation.input_schema, &request.input) {
        return refuse(code(error.code), &error.message);
    }
    let Ok(permit) = service.slots.clone().try_acquire_owned() else {
        return refuse(ErrorCode::Capacity, "service concurrency limit reached");
    };
    // A full set of retained final observations admits nothing new
    // (execution_audit.yaml, Final-observation recovery).
    if !state.accepting() {
        return refuse(ErrorCode::Unavailable, "execution audit recovery is full");
    }

    // Everything from the anchor on runs in a host task the request awaits: a
    // caller that hangs up drops only this await, never an anchor without its
    // final observation, a dispatched write or its settlement (mutations.yaml).
    let execution = tokio::spawn(execute(service.clone(), state, request, operation, permit));
    match execution.await {
        Ok(response) => answer(response),
        Err(_) => refuse(ErrorCode::Internal, "internal error"),
    }
}

/// How an admitted write's attempt stands before dispatch.
enum Staged {
    /// Dispatch is open for this attempt.
    Open(AttemptRef),
    /// No attempt was recorded (`capacity` or an unavailable store).
    NotRecorded(ErrorCode),
    /// The attempt was recorded and not dispatched (`not_attempted`).
    NotAttempted(AttemptRef, CauseStage),
}

/// The anchor, then for a write the attempt (prepare, link, open), the one
/// dispatch, the attempt settlement, then the final observation.
async fn execute(
    service: Service,
    state: Arc<ServiceState>,
    request: InvokeRequest,
    operation: connectors_core::Operation,
    _permit: tokio::sync::OwnedSemaphorePermit,
) -> InvokeResponse {
    let request_id = request.request_id.clone();
    let write = operation.profile == MUTATION_PROFILE;
    let access = if write {
        audit::Access::Write
    } else {
        audit::Access::Read
    };

    // The anchor is acknowledged before any adapter call; without it nothing
    // is dispatched and no record is referenced (audit.md § 2).
    let anchored = {
        let state = state.clone();
        let request_id = request_id.clone();
        let operation = request.operation.clone();
        let revision = request.revision.clone();
        tokio::task::spawn_blocking(move || {
            state.anchor(&Admitted {
                request_id: &request_id,
                operation: &operation,
                descriptor_revision: &revision,
                access,
            })
        })
        .await
    };
    let Ok(Ok(reference)) = anchored else {
        return Refusal {
            request_id: Some(request_id),
            error: Error::new(ErrorCode::Unavailable, "execution audit is unavailable"),
        }
        .into_response();
    };

    let mut attempt = None;
    if write {
        let staged = {
            let state = state.clone();
            let reference = reference.clone();
            let request = request.clone();
            let operation = operation.clone();
            tokio::task::spawn_blocking(move || stage(&state, &reference, &request, &operation))
                .await
                .unwrap_or(Staged::NotRecorded(ErrorCode::Internal))
        };
        match staged {
            Staged::Open(opened) => attempt = Some(opened),
            Staged::NotRecorded(code) => {
                let complete =
                    finish(&state, &reference, audit::Outcome::Error, Some(text(code))).await;
                return respond(
                    request_id,
                    &reference,
                    complete,
                    Err(Error::new(
                        code,
                        "the write's attempt could not be recorded",
                    )),
                    None,
                );
            }
            Staged::NotAttempted(attempt, stage) => {
                let complete = finish(
                    &state,
                    &reference,
                    audit::Outcome::Error,
                    Some(text(ErrorCode::Unavailable)),
                )
                .await;
                let mutation = observation(
                    &state,
                    &request_id,
                    EffectKnowledge::NotAttempted,
                    Some(attempt),
                    Some(DiagnosticCause {
                        code: BaseErrorCode::Unavailable,
                        stage,
                    }),
                );
                return respond(
                    request_id,
                    &reference,
                    complete,
                    Err(Error::new(
                        ErrorCode::Unavailable,
                        "the write was not attempted",
                    )),
                    Some(mutation),
                );
            }
        }
    }

    let started = Instant::now();
    let answer = dispatch(&service, &request, &operation).await;
    tracing::info!(request_id = ?request.request_id, operation = ?request.operation, elapsed_ms = started.elapsed().as_millis(), success = answer.is_ok(), "operation completed");

    let Some(attempt) = attempt else {
        // A read: its known result or error, observed as it is.
        let (outcome, code) = match &answer {
            Ok(_) => (audit::Outcome::Success, None),
            Err((error, _)) => (audit::Outcome::Error, Some(text(error.code))),
        };
        let complete = finish(&state, &reference, outcome, code).await;
        return respond(
            request_id,
            &reference,
            complete,
            answer.map_err(|(error, _)| error),
            None,
        );
    };

    // A write: only a deliverable value is definitive; anything else after
    // dispatch is `unknown` and is never redispatched (mutations.yaml).
    let cause = answer.as_ref().err().map(|(_, cause)| cause.clone());
    let cause_text = cause.as_ref().map(|cause| text(cause.code));
    let settled = {
        let state = state.clone();
        let applied = answer.is_ok();
        let cause_text = cause_text.clone();
        tokio::task::spawn_blocking(move || {
            state.settle_attempt(attempt, applied, cause_text.as_deref())
        })
        .await
        .unwrap_or(false)
    };
    match answer {
        Ok(value) => {
            let complete = finish(&state, &reference, audit::Outcome::Success, None).await;
            // A known effect stays known when its settlement was not stored.
            let secondary = (!settled).then_some(DiagnosticCause {
                code: BaseErrorCode::Unavailable,
                stage: CauseStage::AttemptStore,
            });
            let mutation = observation(
                &state,
                &request_id,
                EffectKnowledge::Applied,
                Some(attempt),
                secondary,
            );
            respond(request_id, &reference, complete, Ok(value), Some(mutation))
        }
        Err(_) => {
            let complete = finish(
                &state,
                &reference,
                audit::Outcome::Unknown,
                cause_text.clone(),
            )
            .await;
            let mutation = observation(
                &state,
                &request_id,
                EffectKnowledge::Unknown,
                Some(attempt),
                cause,
            );
            respond(
                request_id,
                &reference,
                complete,
                Err(Error::new(
                    ErrorCode::OutcomeUnknown,
                    "the write was dispatched and its outcome is unknown",
                )),
                Some(mutation),
            )
        }
    }
}

/// `PrepareAttempt`, `LinkAnchorAttempt` and `OpenDispatch`, in that order.
fn stage(
    state: &ServiceState,
    reference: &audit::Reference,
    request: &InvokeRequest,
    operation: &connectors_core::Operation,
) -> Staged {
    let prepared = match state.prepare_attempt(&Write {
        request_id: &request.request_id,
        operation: &request.operation,
        contract: &operation.contract,
        profile: &operation.profile,
        descriptor_revision: &request.revision,
        input: &request.input,
    }) {
        Ok(prepared) => prepared,
        Err(mutations::Failure::Capacity) => return Staged::NotRecorded(ErrorCode::Capacity),
        Err(_) => return Staged::NotRecorded(ErrorCode::Unavailable),
    };
    let attempt = prepared.reference();
    if !state.link_attempt(reference, attempt) {
        state.abort_attempt(attempt, "unavailable");
        return Staged::NotAttempted(attempt, CauseStage::Audit);
    }
    match state.open_attempt(prepared) {
        Ok(opened) => Staged::Open(opened),
        Err(_) => {
            // Not dispatched: fenced to Aborted where the store still answers.
            state.abort_attempt(attempt, "unavailable");
            Staged::NotAttempted(attempt, CauseStage::AttemptStore)
        }
    }
}

/// The § 5 observation naming this host's attempt.
fn observation(
    state: &ServiceState,
    request_id: &str,
    classification: EffectKnowledge,
    attempt: Option<AttemptRef>,
    cause: Option<DiagnosticCause>,
) -> MutationObservation {
    let attempt = attempt
        .and_then(|attempt| AttemptId::parse(&attempt.attempt_id.to_string()))
        .map(|id| AttemptReference {
            instance: state.instance().to_owned(),
            id,
        });
    MutationObservation {
        classification,
        original_request_id: attempt.as_ref().map(|_| request_id.to_owned()),
        attempt,
        replayed: false,
        cause,
    }
}

async fn finish(
    state: &Arc<ServiceState>,
    reference: &audit::Reference,
    outcome: audit::Outcome,
    code: Option<String>,
) -> bool {
    let state = state.clone();
    let reference = reference.clone();
    tokio::task::spawn_blocking(move || state.finish(&reference, outcome, code))
        .await
        .unwrap_or(false)
}

fn respond(
    request_id: String,
    reference: &audit::Reference,
    complete: bool,
    result: std::result::Result<Value, Error>,
    mutation: Option<MutationObservation>,
) -> InvokeResponse {
    let (status, result, error) = match result {
        Ok(value) => (ResponseStatus::Success, Some(value), None),
        Err(error) => (ResponseStatus::Error, None, Some(error)),
    };
    InvokeResponse {
        version: Version::V1alpha2,
        request_id: Some(request_id),
        status,
        result,
        error,
        audit_ref: Some(reference.audit_ref.clone()),
        audit_status: if complete {
            AuditStatus::Complete
        } else {
            AuditStatus::Incomplete
        },
        source_audit: None,
        mutation,
    }
}

/// The one adapter call. An error carries the safe error a read answers and
/// the cause a write's `unknown` observation names.
async fn dispatch(
    service: &Service,
    request: &InvokeRequest,
    operation: &connectors_core::Operation,
) -> std::result::Result<Value, (Error, DiagnosticCause)> {
    let failed = |code: ErrorCode, base: BaseErrorCode, stage, message: String| {
        (
            Error::new(code, message),
            DiagnosticCause { code: base, stage },
        )
    };
    let result = match tokio::time::timeout(
        DEADLINE,
        service
            .adapter
            .invoke_at(&request.revision, &request.operation, request.input.clone()),
    )
    .await
    {
        Err(_) => {
            return Err(failed(
                ErrorCode::Timeout,
                BaseErrorCode::Timeout,
                CauseStage::Dispatch,
                "operation deadline exceeded".to_owned(),
            ));
        }
        Ok(Err(error)) => {
            return Err(failed(
                code(error.code.clone()),
                base(error.code),
                CauseStage::Dispatch,
                error.message,
            ));
        }
        Ok(Ok(result)) => result,
    };
    if validate(&operation.output_schema, &result).is_err() {
        return Err(failed(
            ErrorCode::UpstreamProtocol,
            BaseErrorCode::UpstreamProtocol,
            CauseStage::Response,
            "result does not match the declared output schema".to_owned(),
        ));
    }
    let Ok(bytes) = serde_json::to_vec(&result) else {
        return Err(failed(
            ErrorCode::Internal,
            BaseErrorCode::Internal,
            CauseStage::Response,
            "internal error".to_owned(),
        ));
    };
    if bytes.len() > RESPONSE_LIMIT - 1024 {
        return Err(failed(
            ErrorCode::Capacity,
            BaseErrorCode::Capacity,
            CauseStage::Response,
            "result exceeds byte limit".to_owned(),
        ));
    }
    Ok(result)
}

fn base(code: connectors_core::ErrorCode) -> BaseErrorCode {
    use connectors_core::ErrorCode as Base;
    match code {
        Base::InvalidInput => BaseErrorCode::InvalidInput,
        Base::Unsupported => BaseErrorCode::Unsupported,
        Base::Unauthorized => BaseErrorCode::Unauthorized,
        Base::Forbidden => BaseErrorCode::Forbidden,
        Base::NotFound => BaseErrorCode::NotFound,
        Base::StaleDescription => BaseErrorCode::StaleDescription,
        Base::StaleCursor => BaseErrorCode::StaleCursor,
        Base::RateLimited => BaseErrorCode::RateLimited,
        Base::Unavailable => BaseErrorCode::Unavailable,
        Base::Capacity => BaseErrorCode::Capacity,
        Base::Timeout => BaseErrorCode::Timeout,
        Base::UpstreamProtocol => BaseErrorCode::UpstreamProtocol,
        Base::Internal => BaseErrorCode::Internal,
    }
}

/// The 13 base codes keep their names on this binding.
fn code(code: connectors_core::ErrorCode) -> ErrorCode {
    use connectors_core::ErrorCode as Base;
    match code {
        Base::InvalidInput => ErrorCode::InvalidInput,
        Base::Unsupported => ErrorCode::Unsupported,
        Base::Unauthorized => ErrorCode::Unauthorized,
        Base::Forbidden => ErrorCode::Forbidden,
        Base::NotFound => ErrorCode::NotFound,
        Base::StaleDescription => ErrorCode::StaleDescription,
        Base::StaleCursor => ErrorCode::StaleCursor,
        Base::RateLimited => ErrorCode::RateLimited,
        Base::Unavailable => ErrorCode::Unavailable,
        Base::Capacity => ErrorCode::Capacity,
        Base::Timeout => ErrorCode::Timeout,
        Base::UpstreamProtocol => ErrorCode::UpstreamProtocol,
        Base::Internal => ErrorCode::Internal,
    }
}
fn text(code: impl serde::Serialize) -> String {
    serde_json::to_value(code)
        .ok()
        .and_then(|value| value.as_str().map(str::to_owned))
        .unwrap_or_else(|| "internal".to_owned())
}

/// The extended HTTP mapping (compatibility.md § 5).
fn http_status(code: ErrorCode) -> StatusCode {
    use ErrorCode::*;
    match code {
        InvalidInput | Unsupported => StatusCode::BAD_REQUEST,
        Unauthorized => StatusCode::UNAUTHORIZED,
        Forbidden | ApprovalRequired | ApprovalRefused | NotGranted | RouteRefused
        | InsufficientScope => StatusCode::FORBIDDEN,
        NotFound => StatusCode::NOT_FOUND,
        StaleDescription | StaleCursor | ApprovalReplayed | IdempotencyConflict
        | StaleAuthority | ConnectionNotReady | SessionNotReady | OfferRejected => {
            StatusCode::CONFLICT
        }
        SessionLost | OfferExpired | LeaseExpired | Revoked => StatusCode::GONE,
        Capacity => StatusCode::PAYLOAD_TOO_LARGE,
        RateLimited => StatusCode::TOO_MANY_REQUESTS,
        Internal => StatusCode::INTERNAL_SERVER_ERROR,
        UpstreamProtocol | OutcomeUnknown => StatusCode::BAD_GATEWAY,
        Unavailable | RouteUnavailable => StatusCode::SERVICE_UNAVAILABLE,
        Timeout => StatusCode::GATEWAY_TIMEOUT,
    }
}

/// A refusal before the anchor: no record exists, so `audit_ref` is null and
/// `audit_status` is `unavailable`.
fn refused(request_id: Option<String>, code: ErrorCode, message: &str) -> HttpResponse {
    let mut message = message.to_owned();
    if message.len() > connectors_core::v1alpha2::MESSAGE_LIMIT {
        message = "request refused".to_owned();
    }
    answer(
        Refusal {
            request_id,
            error: Error::new(code, message),
        }
        .into_response(),
    )
}

fn answer(response: InvokeResponse) -> HttpResponse {
    let status = response
        .error
        .as_ref()
        .map(|error| http_status(error.code))
        .unwrap_or(StatusCode::OK);
    (status, Json(response)).into_response()
}
