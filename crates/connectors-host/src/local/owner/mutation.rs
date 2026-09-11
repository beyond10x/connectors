//! Local write coordination. Selectors are reconstructed under current owner
//! policy; observations never grant a native preparation or a dispatch receipt.
use super::{Code, Error, approval_issuance as issuance};
use crate::local::{audit, config::Paths, mutations as ledger};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::time::Instant;
use uuid::Uuid;
mod execution;
mod recovery;
pub(super) use execution::{Control, Invocation, execute};
pub(in crate::local::owner) use recovery::recover_observation;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ApprovalCode {
    ApprovalRequired,
    ApprovalRefused,
    ApprovalReplayed,
    IdempotencyConflict,
}
/// The private write binding extends its error vocabulary independently of the
/// unchanged owner/1 codec. Neither branch contains native error messages.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum FailureCode {
    Owner(Code),
    Approval(ApprovalCode),
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Failure {
    pub code: FailureCode,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub service_code: Option<connectors_core::ErrorCode>,
}
impl From<Error> for Failure {
    fn from(error: Error) -> Self {
        Self {
            code: FailureCode::Owner(error.code),
            service_code: error.service_code,
        }
    }
}
impl From<Code> for Failure {
    fn from(code: Code) -> Self {
        Error::from(code).into()
    }
}
impl From<ApprovalCode> for Failure {
    fn from(code: ApprovalCode) -> Self {
        Self {
            code: FailureCode::Approval(code),
            service_code: None,
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Classification {
    NotAttempted,
    Refused,
    Applied,
    Unknown,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Stage {
    Preflight,
    AttemptStore,
    Approval,
    Dispatch,
    Response,
    Observation,
    Audit,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Cause {
    pub code: connectors_core::ErrorCode,
    pub stage: Stage,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Attempt {
    pub instance: String,
    pub id: Uuid,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MutationObservation {
    pub classification: Classification,
    pub attempt: Option<Attempt>,
    pub original_request_id: Option<String>,
    pub replayed: bool,
    pub cause: Option<Cause>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AuditStatus {
    Complete,
    Incomplete,
    Unavailable,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourceAudit {
    pub instance: String,
    pub audit_ref: Option<String>,
    pub audit_status: AuditStatus,
}
/// Safe host result. Ordinary admission errors use the outer Result and reveal
/// no original attempt. Native payloads must pass their selected output schema.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Delivery {
    pub request_id: String,
    pub result: Option<Value>,
    pub error: Option<Failure>,
    pub mutation: MutationObservation,
    pub source_audit: SourceAudit,
}
impl Delivery {
    pub(super) fn validate(&self) -> Result<(), Error> {
        let m = &self.mutation;
        if Uuid::parse_str(&self.request_id)
            .ok()
            .is_none_or(|id| id.is_nil() || id.to_string() != self.request_id)
            || self.result.is_some() == self.error.is_some()
            || (self.result.is_some() && m.classification != Classification::Applied)
            || (m.classification == Classification::Unknown
                && !self
                    .error
                    .as_ref()
                    .is_some_and(|e| e.code == FailureCode::Owner(Code::OutcomeUnknown)))
            || m.attempt.is_some() != m.original_request_id.is_some()
            || (m.replayed && m.attempt.is_none())
            || m.attempt
                .as_ref()
                .is_some_and(|a| a.id.is_nil() || a.instance != self.source_audit.instance)
            || m.original_request_id.as_ref().is_some_and(|id| {
                id.is_empty() || id.len() > 256 || id.chars().any(char::is_control)
            })
            || !connectors_core::valid_id(&self.source_audit.instance)
            || self.source_audit.instance.len() > 128
            || ((self.source_audit.audit_status == AuditStatus::Unavailable)
                != self.source_audit.audit_ref.is_none())
            || self
                .source_audit
                .audit_ref
                .as_ref()
                .is_some_and(|id| id.is_empty() || id.len() > 128)
        {
            return Err(Code::OutcomeUnknown.into());
        }
        Ok(())
    }
}
/// Original Linux monotonic deadline, transferable only within this boot's
/// owner-checked local channel. It is a budget, never wall-clock authority.
#[derive(Clone, Copy)]
pub struct Deadline(u64);
impl Deadline {
    pub fn start() -> Result<Self, Error> {
        Ok(Self(
            Self::now()?
                .checked_add(20_000_000_000)
                .ok_or(Code::Unavailable)?,
        ))
    }
    pub(super) fn from_ticks(ticks: u64) -> Result<Self, Error> {
        let value = Self(ticks);
        value.until()?;
        Ok(value)
    }
    pub(super) fn ticks(self) -> u64 {
        self.0
    }
    pub fn until(self) -> Result<Instant, Error> {
        let start = Instant::now();
        let remaining = self
            .0
            .checked_sub(Self::now()?)
            .filter(|v| (1..=20_000_000_000).contains(v))
            .ok_or(Code::Timeout)?;
        start
            .checked_add(std::time::Duration::from_nanos(remaining))
            .ok_or_else(|| Code::Timeout.into())
    }
    fn now() -> Result<u64, Error> {
        let mut stamp = libc::timespec {
            tv_sec: 0,
            tv_nsec: 0,
        };
        // SAFETY: a live timespec of the syscall's declared size is supplied.
        if unsafe { libc::clock_gettime(libc::CLOCK_MONOTONIC, &mut stamp) } != 0
            || stamp.tv_sec < 0
            || !(0..1_000_000_000).contains(&stamp.tv_nsec)
        {
            return Err(Code::Unavailable.into());
        }
        (stamp.tv_sec as u64)
            .checked_mul(1_000_000_000)
            .and_then(|v| v.checked_add(stamp.tv_nsec as u64))
            .ok_or_else(|| Code::Unavailable.into())
    }
}
/// Stored separately from each delivery's current audit and correlation. This
/// contains only a validated native payload or a closed safe error.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
enum StoredOutcome {
    Success { value: Value },
    Failure { error: Failure },
}

struct NoClock;
impl ledger::Clock for NoClock {
    fn now(&self) -> ledger::Result<ledger::ClockInterval> {
        Err(ledger::Failure::ClockUnavailable)
    }
}
fn store<C: ledger::Clock>(paths: &Paths, clock: C) -> Result<ledger::Store<C>, Error> {
    ledger::Store::new(&paths.state, clock, ledger::Limits::default())
        .map_err(|_| Code::MetadataUnavailable.into())
}
fn candidate(
    current: &issuance::Resolved,
    key: Option<&str>,
    request_id: &str,
) -> Result<ledger::Candidate, Error> {
    if key.is_some_and(|key| key.is_empty() || key.len() > 256) {
        return Err(Code::InvalidInput.into());
    }
    let s = &current.preparation.subject;
    let t = &s.target;
    Ok(ledger::Candidate {
        namespace: ledger::Namespace {
            receiver_instance: t.instance.clone(),
            tenant: None,
            realm: None,
            caller: s.authority.scope.caller.clone(),
            executor: None,
            origin: ledger::Origin::Direct,
        },
        fingerprint: ledger::Fingerprint {
            operation: ledger::OperationRef {
                instance: t.instance.clone(),
                adapter: current.adapter.adapter_id.clone(),
                operation: t.operation.clone(),
            },
            connection_ref: t.connection.clone(),
            connection_revision: t.connection_revision.clone(),
            contract_ref: t.contract.clone(),
            profile: t.profile.clone(),
            descriptor_revision: t.descriptor_revision.clone(),
            configuration_revision: t.configuration_revision.clone(),
            canonicalization_version: s.canonicalization.clone(),
            input_digest: s.input_sha256.clone(),
            route: None,
        },
        caller_key: key.map(str::to_owned),
        request_id: request_id.into(),
        // Approval is deliberately absent from key equality. Only a later
        // verified new candidate may replace this lookup-only placeholder.
        approval: ledger::Approval::NotRequired,
    })
}
fn unchanged(first: &issuance::Resolved, current: &issuance::Resolved) -> Result<(), Error> {
    if first.preparation.subject != current.preparation.subject
        || first.policy != current.policy
        || first.adapter.selection() != current.adapter.selection()
        || first.config.secret_service_socket != current.config.secret_service_socket
    {
        return Err(Code::Forbidden.into());
    }
    Ok(())
}
fn audit_store(paths: &Paths) -> Result<audit::Store, Error> {
    audit::Store::new(&paths.state, 100_000).map_err(|_| Code::MetadataUnavailable.into())
}
fn anchor(current: &issuance::Resolved, request_id: &str) -> audit::Anchor {
    let s = &current.preparation.subject;
    audit::Anchor {
        instance_id: s.target.instance.clone(),
        kind: audit::Kind::AdmittedExecution,
        activity: Some(audit::Activity::Invoke),
        hop: audit::Hop::Execution,
        stage: audit::Stage::Admission,
        request_id: Some(request_id.into()),
        principal_ref: Some(s.authority.scope.caller.clone()),
        operation_id: Some(s.target.operation.clone()),
        connection_ref: Some(s.target.connection.clone()),
        descriptor_revision: Some(s.target.descriptor_revision.clone()),
        recorded_at_ms: connectors_sdk::now_ms() as i64,
        attempt_id: None,
    }
}
fn admit_audit(store: &audit::Store, facts: &audit::Anchor) -> Result<audit::Admission, Error> {
    match store.anchor(facts) {
        Ok(audit::Acknowledgement::Execution(receipt)) => Ok(receipt),
        Err(audit::Failure::OutcomeUnknown) => Err(Code::OutcomeUnknown.into()),
        _ => Err(Code::MetadataUnavailable.into()),
    }
}
fn finish_audit(store: &audit::Store, reference: audit::Reference, delivery: &mut Delivery) {
    let observation = audit::FinalObservation {
        observation_id: Uuid::new_v4(),
        outcome: if delivery.mutation.classification == Classification::Unknown {
            audit::Outcome::Unknown
        } else if delivery.error.is_none() {
            audit::Outcome::Success
        } else if matches!(
            delivery.mutation.classification,
            Classification::NotAttempted | Classification::Refused
        ) {
            audit::Outcome::Refused
        } else {
            audit::Outcome::Error
        },
        code: delivery
            .error
            .as_ref()
            .and_then(|e| serde_json::to_value(e.code).ok())
            .and_then(|v| v.as_str().map(str::to_owned)),
        recorded_at_ms: connectors_sdk::now_ms() as i64,
    };
    delivery.source_audit.audit_ref = Some(reference.audit_ref.clone());
    delivery.source_audit.audit_status = if store.append(&reference, &observation).is_ok() {
        AuditStatus::Complete
    } else {
        AuditStatus::Incomplete
    };
}
fn delivery(
    current: &issuance::Resolved,
    request_id: &str,
    classification: Classification,
    outcome: StoredOutcome,
) -> Delivery {
    let (result, error) = match outcome {
        StoredOutcome::Success { value } => (Some(value), None),
        StoredOutcome::Failure { error } => (None, Some(error)),
    };
    Delivery {
        request_id: request_id.into(),
        result,
        error,
        mutation: MutationObservation {
            classification,
            attempt: None,
            original_request_id: None,
            replayed: false,
            cause: None,
        },
        source_audit: SourceAudit {
            instance: current.adapter.instance_id.clone(),
            audit_ref: None,
            audit_status: AuditStatus::Unavailable,
        },
    }
}
fn failure(
    current: &issuance::Resolved,
    request_id: &str,
    classification: Classification,
    error: impl Into<Failure>,
) -> Delivery {
    delivery(
        current,
        request_id,
        classification,
        StoredOutcome::Failure {
            error: error.into(),
        },
    )
}
fn lookup_error(
    current: &issuance::Resolved,
    request_id: &str,
    error: ledger::Failure,
) -> Delivery {
    if error == ledger::Failure::Conflict {
        failure(
            current,
            request_id,
            Classification::NotAttempted,
            ApprovalCode::IdempotencyConflict,
        )
    } else {
        let mut value = failure(
            current,
            request_id,
            Classification::Unknown,
            Code::OutcomeUnknown,
        );
        value.mutation.cause = Some(Cause {
            code: connectors_core::ErrorCode::Unavailable,
            stage: Stage::Observation,
        });
        value
    }
}
fn project(
    current: &issuance::Resolved,
    request_id: &str,
    original: ledger::Observation,
) -> Result<Delivery, Error> {
    let classification = match original.state {
        ledger::State::Aborted => Classification::NotAttempted,
        ledger::State::Completed => Classification::Applied,
        ledger::State::Failed => Classification::Refused,
        ledger::State::Prepared | ledger::State::Dispatching | ledger::State::Indeterminate => {
            Classification::Unknown
        }
    };
    let pending = matches!(
        original.state,
        ledger::State::Prepared | ledger::State::Dispatching
    );
    let outcome = if pending {
        StoredOutcome::Failure {
            error: Code::OutcomeUnknown.into(),
        }
    } else {
        // A malformed retained safe result establishes no disclosure authority.
        let raw = original.result.ok_or(Code::MetadataUnavailable)?;
        let outcome: StoredOutcome = if original.state == ledger::State::Aborted
            && raw == serde_json::json!({"cause":"recovery_before_dispatch"})
        {
            StoredOutcome::Failure {
                error: Code::Interrupted.into(),
            }
        } else if original.state == ledger::State::Indeterminate
            && raw == serde_json::json!({"cause":"recovery_without_outcome"})
        {
            StoredOutcome::Failure {
                error: Code::OutcomeUnknown.into(),
            }
        } else {
            serde_json::from_value(raw).map_err(|_| Code::MetadataUnavailable)?
        };
        if matches!(outcome, StoredOutcome::Success { .. })
            && classification != Classification::Applied
        {
            return Err(Code::MetadataUnavailable.into());
        }
        if classification == Classification::Unknown
            && !matches!(&outcome, StoredOutcome::Failure { error } if error.code == FailureCode::Owner(Code::OutcomeUnknown))
        {
            return Err(Code::MetadataUnavailable.into());
        }
        outcome
    };
    let mut value = delivery(current, request_id, classification, outcome);
    value.mutation.attempt = Some(Attempt {
        instance: current.adapter.instance_id.clone(),
        id: original.reference.attempt_id,
    });
    value.mutation.original_request_id = Some(original.request_id);
    value.mutation.replayed = !pending;
    Ok(value)
}

/// Current disclosure admission precedes exact lookup. This performs no secret,
/// key, clock-network or provider access and starts no process. Pending originals
/// stay pending; an observer is never allowed to abort/recover another dispatcher.
pub fn observe(
    paths: &Paths,
    alias: &str,
    request: &issuance::Request<'_>,
    key: Option<&str>,
    until: Instant,
) -> Result<Option<Delivery>, Error> {
    observe_original(paths, alias, request, key, until)
        .map(|value| value.map(|value| value.delivery))
}

/// An admitted lookup result, distinct from a live execution reply. Only this
/// lookup can identify an unsettled original; replayed=false on a live reply
/// does not by itself mean that the durable attempt is still pending.
pub struct OriginalObservation {
    pub delivery: Delivery,
    pub pending: bool,
}

pub fn observe_original(
    paths: &Paths,
    alias: &str,
    request: &issuance::Request<'_>,
    key: Option<&str>,
    until: Instant,
) -> Result<Option<OriginalObservation>, Error> {
    observe_as_original(
        paths,
        alias,
        request,
        key,
        until,
        &Uuid::new_v4().to_string(),
        None,
    )
}
fn observe_as(
    paths: &Paths,
    alias: &str,
    request: &issuance::Request<'_>,
    key: Option<&str>,
    until: Instant,
    request_id: &str,
    existing_audit: Option<&audit::Reference>,
) -> Result<Option<Delivery>, Error> {
    observe_as_original(
        paths,
        alias,
        request,
        key,
        until,
        request_id,
        existing_audit,
    )
    .map(|value| value.map(|value| value.delivery))
}
fn observe_as_original(
    paths: &Paths,
    alias: &str,
    request: &issuance::Request<'_>,
    key: Option<&str>,
    until: Instant,
    request_id: &str,
    existing_audit: Option<&audit::Reference>,
) -> Result<Option<OriginalObservation>, Error> {
    let first = issuance::resolve(paths, alias, request, until)?;
    let candidate = candidate(&first, key, request_id)?;
    if key.is_none() {
        return Ok(None);
    }
    let policy = issuance::policy_store(paths, &first.adapter)?
        .acquire(&first.policy.selection, request.operation)
        .map_err(issuance::policy_error)?;
    if policy.policy().map_err(issuance::policy_error)? != &first.policy {
        return Err(Code::Forbidden.into());
    }
    let ledger = store(paths, NoClock)?;
    let original = ledger.lookup(&candidate);
    let pending = matches!(&original, Ok(Some(value)) if matches!(value.state, ledger::State::Prepared | ledger::State::Dispatching));
    let current = issuance::resolve(paths, alias, request, until)?;
    unchanged(&first, &current)?;
    let mut value = match original {
        Ok(None) => return Ok(None),
        Ok(Some(original)) => project(&current, request_id, original)?,
        Err(error) => lookup_error(&current, request_id, error),
    };
    if let Some(payload) = &value.result {
        let bootstrap = super::cached(paths, alias)?;
        let descriptor = bootstrap.descriptor()?;
        let operation = descriptor
            .operation(request.operation)
            .map_err(|_| Code::NotFound)?;
        if connectors_sdk::validate_write_value(&operation.output_schema, payload).is_err() {
            // The durable Completed state still identifies an applied effect.
            // The malformed payload is discarded, never reinterpreted as safe
            // non-dispatch or a reason to repeat the business operation.
            value.result = None;
            value.error = Some(Failure {
                code: FailureCode::Owner(Code::ServiceFailure),
                service_code: Some(connectors_core::ErrorCode::UpstreamProtocol),
            });
            value.mutation.cause = Some(Cause {
                code: connectors_core::ErrorCode::UpstreamProtocol,
                stage: Stage::Response,
            });
        }
    }
    let audits = audit_store(paths)?;
    let facts = anchor(&current, request_id);
    let reference = match existing_audit {
        Some(reference) => Ok(reference.clone()),
        None => admit_audit(&audits, &facts).and_then(|r| {
            audits
                .confirm(r, &facts)
                .map_err(|_| Code::MetadataUnavailable.into())
        }),
    };
    match reference {
        Ok(reference) => finish_audit(&audits, reference, &mut value),
        // No provider is being called here. Preserve the admitted original
        // outcome, with no invented audit acknowledgement.
        Err(_) => value.source_audit.audit_status = AuditStatus::Unavailable,
    }
    unchanged(&current, &issuance::resolve(paths, alias, request, until)?)?;
    Ok(Some(OriginalObservation {
        delivery: value,
        pending,
    }))
}
