//! `POST /v1alpha2/invoke` (contracts/service/compatibility.md § 2.1). Every
//! admitted invocation, read or write, is anchored and acknowledged before the
//! adapter is called, and its one final observation is recorded after
//! (audit.md § 1-3). An admitted write also records its attempt before
//! dispatch and settles it after, and its Response carries the `mutation`
//! naming that attempt (ess/domains/mutations.yaml, "The HTTP host's attempt
//! ledger").
//!
//! A host whose service configuration has no `standing_grants` reads the
//! request with [`InvokeRequest::decode`], which refuses `idempotency_key`
//! `unsupported`, and records, dispatches and settles a write unkeyed.
//!
//! A host whose configuration sets `standing_grants`
//! ([`ServiceState::keyed`]) reads it with [`KeyedInvokeRequest::decode`],
//! and every write there is keyed and admitted only under a standing grant
//! (ess/domains/standing_approvals.yaml). Before the anchor, in this order,
//! each refusal answered with no record, no attempt and zero adapter calls:
//!
//! | Check | Refusal |
//! |---|---|
//! | `idempotency_key` on an operation that is not a write | `unsupported` |
//! | a write without `idempotency_key` | `invalid_input` |
//! | the key is live for another request (other input, operation or revision) | `idempotency_conflict` |
//! | the key reservation store cannot be read | `unavailable` |
//! | the grant file cannot be read safely | `unavailable` |
//! | no grant covers instance, operation, every target member, `not_after`, revocation | `approval_required` |
//! | every covering grant has `max_per_hour` uses in the last hour | `rate_limited` |
//! | the use store fails | `unavailable` |
//!
//! A live key for this exact request is a replay: the original attempt's
//! outcome, `replayed: true`, with no grant check, no count and no adapter
//! call. Before `approval_required` or `rate_limited` the key is looked up
//! again, so a concurrent winner is replayed instead (operations § 4 step 5).
use super::{Service, admitted};
use crate::local::{
    audit, mutations,
    mutations::AttemptRef,
    service::{Admitted, GrantCheck, KeyLookup, KeyedPreparation, ServiceState, Write},
    standing,
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
        EffectKnowledge, Error, ErrorCode, InvokeRequest, InvokeResponse, KeyedInvokeRequest,
        MutationObservation, Refusal, ResponseStatus, Version,
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
pub(super) const MUTATION_PROFILE: &str = "mutation";
const DEADLINE: Duration = Duration::from_secs(20);
/// How long a replay waits for a pending original before it answers
/// `outcome_unknown`: the original's own deadline and its settlement.
const REPLAY_WAIT: Duration = Duration::from_secs(25);
const REPLAY_POLL: Duration = Duration::from_millis(50);

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
    // Refusal before the anchor: `unauthorized` or `unavailable`, no record,
    // zero adapter calls (compatibility.md § 2.1, Admission).
    let principal = match admitted(&service, &headers).await {
        Ok(principal) => principal,
        Err(error) => return refused(None, code(error.code), &error.message),
    };
    let Ok(bytes) = body else {
        return refused(None, ErrorCode::Capacity, "request exceeds byte limit");
    };
    // Without standing grants the reader is the unchanged one: a key is
    // refused `unsupported` with no request ID (module docs).
    let decoded = if state.keyed() {
        KeyedInvokeRequest::decode(&bytes)
    } else {
        InvokeRequest::decode(&bytes).map(KeyedInvokeRequest::from)
    };
    let request = match decoded {
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
    // Refusal (keyed host only; the unchanged reader refused the key above):
    // a key promises a replay only a write's attempt can give.
    if operation.profile != MUTATION_PROFILE && request.idempotency_key.is_some() {
        return refuse(
            ErrorCode::Unsupported,
            "idempotency_key is accepted only for a write",
        );
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
    let execution = tokio::spawn(execute(
        service.clone(),
        state,
        principal,
        request,
        operation,
        permit,
    ));
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
    /// A concurrent request with the same key and input holds the attempt.
    Replay(mutations::Observation),
}

/// What the keyed admission of a write found before the anchor.
enum Keyed {
    /// A new attempt under this grant use.
    New { key: String, taken: standing::Use },
    /// The key names an attempt for this exact request.
    Replay(mutations::Observation),
}

/// A refusal before the anchor: no record, no attempt, zero adapter calls.
struct Refused {
    code: ErrorCode,
    message: &'static str,
    retry_after_seconds: Option<u64>,
}

impl Refused {
    fn new(code: ErrorCode, message: &'static str) -> Self {
        Self {
            code,
            message,
            retry_after_seconds: None,
        }
    }
}

/// Operations § 4 steps 4 and 5 under a standing grant: the key, then the
/// grant, each refusal named (module docs).
fn admit_write(
    state: &ServiceState,
    principal: &str,
    request: &KeyedInvokeRequest,
    operation: &connectors_core::Operation,
) -> std::result::Result<Keyed, Refused> {
    let write = Write {
        request_id: &request.request_id,
        principal,
        operation: &request.operation,
        contract: &operation.contract,
        profile: &operation.profile,
        descriptor_revision: &request.revision,
        input: &request.input,
    };
    // Refusal: a write is keyed.
    let Some(key) = request.idempotency_key.clone() else {
        return Err(Refused::new(
            ErrorCode::InvalidInput,
            "a write requires idempotency_key",
        ));
    };
    let lookup = |write: &Write<'_>| match state.lookup_key(write, &key) {
        Ok(found) => Ok(found),
        // Refusal: the key is live for another request; neither the stored
        // request nor the differing axis is disclosed.
        Err(mutations::Failure::Conflict) => Err(Refused::new(
            ErrorCode::IdempotencyConflict,
            "idempotency_key is in use for another request",
        )),
        // Refusal: the reservation cannot be read, so no replay or new
        // attempt can be decided.
        Err(_) => Err(Refused::new(
            ErrorCode::Unavailable,
            "the key reservation store is unavailable",
        )),
    };
    if let KeyLookup::Replay(original) = lookup(&write)? {
        return Ok(Keyed::Replay(original));
    }
    // A refusal after a miss first looks for a concurrent winner.
    let unless_won = |refused: Refused| match lookup(&write)? {
        KeyLookup::Replay(original) => Ok(Keyed::Replay(original)),
        KeyLookup::Miss => Err(refused),
    };
    match state.check_grant(&write) {
        GrantCheck::Admitted(taken) => Ok(Keyed::New { key, taken }),
        // Refusal: the grant file is a symbolic link, writable by others,
        // oversized or malformed.
        GrantCheck::Unreadable(reason) => {
            tracing::warn!(?reason, "the standing grant file was refused");
            Err(Refused::new(
                ErrorCode::Unavailable,
                "standing grants are unavailable",
            ))
        }
        // Refusal: no grant covers this write; the axis is not named.
        GrantCheck::Uncovered(reason) => {
            tracing::info!(request_id = %request.request_id, ?reason, "no standing grant covers the write");
            unless_won(Refused::new(
                ErrorCode::ApprovalRequired,
                "no standing grant covers this write",
            ))
        }
        // Refusal: the covering grants used their hour.
        GrantCheck::AtCap {
            retry_after_seconds,
        } => unless_won(Refused {
            code: ErrorCode::RateLimited,
            message: "the standing grant's hourly writes are used",
            retry_after_seconds: Some(retry_after_seconds),
        }),
        // Refusal: the hourly count cannot be kept.
        GrantCheck::UsesUnavailable => Err(Refused::new(
            ErrorCode::Unavailable,
            "the standing grant count is unavailable",
        )),
    }
}

/// The anchor, then for a write the attempt (prepare, link, open), the one
/// dispatch, the attempt settlement, then the final observation.
async fn execute(
    service: Service,
    state: Arc<ServiceState>,
    principal: String,
    request: KeyedInvokeRequest,
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

    // Keyed admission runs only on a host with standing grants; without them
    // a write goes straight to its unkeyed attempt (module docs).
    let keyed = if write && state.keyed() {
        let admitted = {
            let state = state.clone();
            let principal = principal.clone();
            let request = request.clone();
            let operation = operation.clone();
            tokio::task::spawn_blocking(move || {
                admit_write(&state, &principal, &request, &operation)
            })
            .await
            .unwrap_or_else(|_| Err(Refused::new(ErrorCode::Internal, "internal error")))
        };
        match admitted {
            Ok(keyed) => Some(keyed),
            Err(refused) => {
                return Refusal {
                    request_id: Some(request_id),
                    error: Error {
                        code: refused.code,
                        message: refused.message.to_owned(),
                        retry_after_seconds: refused.retry_after_seconds,
                    },
                }
                .into_response();
            }
        }
    } else {
        None
    };

    // The anchor is acknowledged before any adapter call; without it nothing
    // is dispatched and no record is referenced (audit.md § 2).
    let anchored = {
        let state = state.clone();
        let request_id = request_id.clone();
        let principal = principal.clone();
        let operation = request.operation.clone();
        let revision = request.revision.clone();
        tokio::task::spawn_blocking(move || {
            state.anchor(&Admitted {
                request_id: &request_id,
                principal: &principal,
                operation: &operation,
                descriptor_revision: &revision,
                access,
            })
        })
        .await
    };
    let Ok(Ok(reference)) = anchored else {
        if let Some(Keyed::New { taken, .. }) = &keyed {
            state.release_grant(taken);
        }
        return Refusal {
            request_id: Some(request_id),
            error: Error::new(ErrorCode::Unavailable, "execution audit is unavailable"),
        }
        .into_response();
    };

    let mut attempt = None;
    if write {
        let grant = match keyed {
            Some(Keyed::Replay(original)) => {
                return replay(&state, &reference, request_id, original).await;
            }
            Some(Keyed::New { key, taken }) => Some((key, taken)),
            None => None,
        };
        let staged = {
            let state = state.clone();
            let reference = reference.clone();
            let request = request.clone();
            let operation = operation.clone();
            let principal = principal.clone();
            tokio::task::spawn_blocking(move || {
                let grant = grant.as_ref().map(|(key, taken)| (key.as_str(), taken));
                stage(&state, &reference, &principal, &request, &operation, grant)
            })
            .await
            .unwrap_or(Staged::NotRecorded(ErrorCode::Internal))
        };
        // Nothing was dispatched: the final observation records the refusal
        // (audit.md § 1 rule 6; execution_audit.yaml, Failed link).
        match staged {
            Staged::Open(opened) => attempt = Some(opened),
            Staged::Replay(original) => {
                return replay(&state, &reference, request_id, original).await;
            }
            Staged::NotRecorded(code) => {
                let complete = finish(
                    &state,
                    &reference,
                    audit::Outcome::Refused,
                    Some(text(code)),
                )
                .await;
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
                    audit::Outcome::Refused,
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
        let result = answer.as_ref().ok().cloned();
        let cause_text = cause_text.clone();
        tokio::task::spawn_blocking(move || {
            if state.keyed() {
                state.settle_keyed(attempt, result.as_ref(), cause_text.as_deref())
            } else {
                state.settle_attempt(attempt, result.is_some(), cause_text.as_deref())
            }
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
/// Without a grant (a host with no standing grants) the attempt is unkeyed,
/// as at the wave base. With one, `PrepareAttempt` reserves the key in the
/// same transaction and records the grant, and a concurrent winner holding
/// the key is replayed instead.
fn stage(
    state: &ServiceState,
    reference: &audit::Reference,
    principal: &str,
    request: &KeyedInvokeRequest,
    operation: &connectors_core::Operation,
    grant: Option<(&str, &standing::Use)>,
) -> Staged {
    let write = Write {
        request_id: &request.request_id,
        principal,
        operation: &request.operation,
        contract: &operation.contract,
        profile: &operation.profile,
        descriptor_revision: &request.revision,
        input: &request.input,
    };
    let Some((key, taken)) = grant else {
        let prepared = match state.prepare_attempt(&write) {
            Ok(prepared) => prepared,
            Err(mutations::Failure::Capacity) => return Staged::NotRecorded(ErrorCode::Capacity),
            Err(_) => return Staged::NotRecorded(ErrorCode::Unavailable),
        };
        return link_and_open(state, reference, prepared);
    };
    let prepared = match state.prepare_keyed(&write, key, taken) {
        Ok(KeyedPreparation::Prepared(prepared)) => prepared,
        Ok(KeyedPreparation::Existing(original)) => return Staged::Replay(original),
        // Refusal: the attempt was prepared and aborted because its grant
        // could not be recorded with it.
        Ok(KeyedPreparation::GrantUnrecorded(attempt)) => {
            return Staged::NotAttempted(attempt, CauseStage::AttemptStore);
        }
        Err(mutations::Failure::Capacity) => return Staged::NotRecorded(ErrorCode::Capacity),
        // Refusal: a concurrent request reserved the key for other input.
        Err(mutations::Failure::Conflict) => {
            return Staged::NotRecorded(ErrorCode::IdempotencyConflict);
        }
        Err(_) => return Staged::NotRecorded(ErrorCode::Unavailable),
    };
    link_and_open(state, reference, prepared)
}

/// `LinkAnchorAttempt`, then `OpenDispatch`; either failing aborts the
/// prepared attempt as `not_attempted`.
fn link_and_open(
    state: &ServiceState,
    reference: &audit::Reference,
    prepared: mutations::Prepared,
) -> Staged {
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

/// A keyed replay (operations § 5.1): the original attempt's outcome,
/// `replayed: true`, naming the original attempt and its request. Nothing is
/// dispatched, no grant is read and nothing is counted. A pending original is
/// awaited up to [`REPLAY_WAIT`] and then answered `outcome_unknown`. This
/// request's own anchor records what it answered.
async fn replay(
    state: &Arc<ServiceState>,
    reference: &audit::Reference,
    request_id: String,
    original: mutations::Observation,
) -> InvokeResponse {
    let attempt = original.reference;
    let mut current = original;
    let waited = Instant::now();
    while matches!(
        current.state,
        mutations::State::Prepared | mutations::State::Dispatching
    ) && waited.elapsed() < REPLAY_WAIT
    {
        tokio::time::sleep(REPLAY_POLL).await;
        let observed = {
            let state = state.clone();
            tokio::task::spawn_blocking(move || state.observe_attempt(attempt)).await
        };
        match observed {
            Ok(Ok(observation)) => current = observation,
            _ => break,
        }
    }
    let replayed = |classification, cause| MutationObservation {
        classification,
        attempt: AttemptId::parse(&attempt.attempt_id.to_string()).map(|id| AttemptReference {
            instance: state.instance().to_owned(),
            id,
        }),
        original_request_id: Some(current.request_id.clone()),
        replayed: true,
        cause,
    };
    let settled = current.result.as_ref();
    let (outcome, answer, mutation) = match current.state {
        mutations::State::Completed => {
            match settled.and_then(|settlement| settlement.get("result")) {
                Some(result) => (
                    audit::Outcome::Success,
                    Ok(result.clone()),
                    replayed(EffectKnowledge::Applied, None),
                ),
                // The write applied and its result was too large to retain.
                None => (
                    audit::Outcome::Error,
                    Err(Error::new(
                        ErrorCode::Capacity,
                        "the write applied; its result was not retained for replay",
                    )),
                    replayed(EffectKnowledge::Applied, None),
                ),
            }
        }
        mutations::State::Aborted | mutations::State::Failed => (
            audit::Outcome::Refused,
            Err(Error::new(
                ErrorCode::Unavailable,
                "the original write was not attempted",
            )),
            replayed(EffectKnowledge::NotAttempted, None),
        ),
        mutations::State::Indeterminate => (
            audit::Outcome::Unknown,
            Err(Error::new(
                ErrorCode::OutcomeUnknown,
                "the original write was dispatched and its outcome is unknown",
            )),
            replayed(EffectKnowledge::Unknown, None),
        ),
        // Still pending after the wait: never dispatched again from here.
        mutations::State::Prepared | mutations::State::Dispatching => (
            audit::Outcome::Unknown,
            Err(Error::new(
                ErrorCode::OutcomeUnknown,
                "the original write has not settled",
            )),
            replayed(
                EffectKnowledge::Unknown,
                Some(DiagnosticCause {
                    code: BaseErrorCode::Timeout,
                    stage: CauseStage::Observation,
                }),
            ),
        ),
    };
    let code = answer.as_ref().err().map(|error| text(error.code));
    let complete = finish(state, reference, outcome, code).await;
    respond(request_id, reference, complete, answer, Some(mutation))
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
    request: &KeyedInvokeRequest,
    operation: &connectors_core::Operation,
) -> std::result::Result<Value, (Error, DiagnosticCause)> {
    let failed = |code: ErrorCode, base: BaseErrorCode, stage, message: String| {
        (
            Error::new(code, message),
            DiagnosticCause { code: base, stage },
        )
    };
    let call =
        service
            .adapter
            .invoke_at(&request.revision, &request.operation, request.input.clone());
    let result = match tokio::time::timeout(DEADLINE, Unwind(call)).await {
        // The adapter panicked after dispatch: its answer is lost, which a
        // write observes as `unknown` (mutations.yaml) and a read as an
        // `internal` error, each with its anchor's final observation.
        Ok(Err(Panicked)) => {
            return Err(failed(
                ErrorCode::Internal,
                BaseErrorCode::Internal,
                CauseStage::Dispatch,
                "internal error".to_owned(),
            ));
        }
        Err(_) => {
            return Err(failed(
                ErrorCode::Timeout,
                BaseErrorCode::Timeout,
                CauseStage::Dispatch,
                "operation deadline exceeded".to_owned(),
            ));
        }
        Ok(Ok(Err(error))) => {
            return Err(failed(
                code(error.code.clone()),
                base(error.code),
                CauseStage::Dispatch,
                error.message,
            ));
        }
        Ok(Ok(Ok(result))) => result,
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
        Base::InvalidInput | Base::Conflict => BaseErrorCode::InvalidInput,
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
        Base::InvalidInput | Base::Conflict => ErrorCode::InvalidInput,
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
        | StaleAuthority | ConnectionNotReady | SessionNotReady | OfferRejected | Conflict => {
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

/// The adapter call panicked; its answer is lost.
struct Panicked;

/// Polls the adapter call and turns a panic into [`Panicked`], so an answer
/// lost to a panic after dispatch is observed like any other lost answer
/// instead of unwinding the host task past its settlement and final
/// observation.
struct Unwind<F>(F);

impl<F: std::future::Future + Unpin> std::future::Future for Unwind<F> {
    type Output = std::result::Result<F::Output, Panicked>;

    fn poll(
        mut self: std::pin::Pin<&mut Self>,
        context: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        let call = &mut self.0;
        match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            std::pin::Pin::new(call).poll(context)
        })) {
            Ok(poll) => poll.map(Ok),
            Err(_) => std::task::Poll::Ready(Err(Panicked)),
        }
    }
}
