use super::*;
use crate::local::{approvals, clock, keyring::custody, registry, runtime};
use connectors_sdk::Secret;
use std::sync::{
    Mutex,
    atomic::{AtomicBool, Ordering},
};

pub(in crate::local::owner) struct Control<'a> {
    pub lifecycle: &'a Mutex<super::super::lifecycle::Control>,
    pub epoch: u64,
    pub stopped: &'a AtomicBool,
}
pub(in crate::local::owner) struct Invocation<'a> {
    pub request: issuance::Request<'a>,
    pub key: Option<&'a str>,
    pub proof: Option<Secret>,
    pub until: Instant,
}
impl Control<'_> {
    fn check(&self) -> Result<(), Error> {
        self.lifecycle
            .lock()
            .map_err(|_| Code::Unavailable)?
            .check(self.epoch)?;
        if self.stopped.load(Ordering::SeqCst) {
            return Err(Code::Unavailable.into());
        }
        Ok(())
    }
}
struct ClockRef<'a>(&'a clock::BoundedClock);
impl ledger::Clock for ClockRef<'_> {
    fn now(&self) -> ledger::Result<ledger::ClockInterval> {
        ledger::Clock::now(self.0)
    }
}
struct Use {
    registry: registry::Registry,
    captured: Option<registry::ReadUse>,
    dispatched: Option<registry::DispatchedUse>,
}
impl Drop for Use {
    fn drop(&mut self) {
        if let Some(captured) = self.captured.take() {
            let _ = self
                .registry
                .cancel_read(captured, connectors_sdk::now_ms());
        }
        if let Some(dispatched) = self.dispatched.take() {
            let _ = self
                .registry
                .release_read(dispatched, connectors_sdk::now_ms());
        }
    }
}
fn proof_error(error: approvals::Failure) -> Failure {
    match error {
        approvals::Failure::Refused => ApprovalCode::ApprovalRefused.into(),
        approvals::Failure::Replayed => ApprovalCode::ApprovalReplayed.into(),
        approvals::Failure::Unavailable => Code::Unavailable.into(),
        approvals::Failure::MetadataUnavailable => Code::MetadataUnavailable.into(),
        approvals::Failure::OutcomeUnknown => Code::OutcomeUnknown.into(),
        approvals::Failure::Capacity => Code::Capacity.into(),
    }
}
fn ledger_error(error: ledger::Failure) -> Failure {
    match error {
        ledger::Failure::InvalidInput => Code::InvalidInput,
        ledger::Failure::Capacity => Code::Capacity,
        ledger::Failure::BindingChanged | ledger::Failure::Conflict => Code::LifecycleConflict,
        ledger::Failure::ClockUnavailable => Code::Unavailable,
        ledger::Failure::OutcomeUnknown => Code::OutcomeUnknown,
        ledger::Failure::NotFound => Code::NotFound,
        ledger::Failure::MetadataUnavailable => Code::MetadataUnavailable,
    }
    .into()
}
fn time(current: &issuance::Resolved, until: Instant) -> Result<clock::BoundedClock, Error> {
    issuance::check(until)?;
    let clock = current
        .config
        .approval_clock
        .as_ref()
        .ok_or(Code::InvalidConfiguration)?
        .acquire()
        .map_err(|_| Code::Unavailable)?;
    issuance::check(until)?;
    Ok(clock)
}
fn verify(
    paths: &Paths,
    current: &issuance::Resolved,
    evidence: &approvals::Evidence,
    clock: &clock::BoundedClock,
) -> Result<approvals::VerifiedApproval, Failure> {
    let subject = &current.preparation.subject;
    let policy = issuance::policy_store(paths, &current.adapter)?
        .acquire(&current.policy.selection, &subject.target.operation)
        .map_err(issuance::policy_error)?;
    if policy.policy().map_err(issuance::policy_error)? != &current.policy {
        return Err(Code::Forbidden.into());
    }
    let key = issuance::key_store(paths, &current.config, &current.adapter)?
        .acquire_key()
        .map_err(issuance::key_error)?;
    let admitted = issuance::Admission {
        policy: &policy,
        key: &key,
        subject,
    };
    approvals::verify(evidence, subject, &admitted, clock).map_err(proof_error)
}
fn correlation(
    value: &mut Delivery,
    current: &issuance::Resolved,
    candidate: &ledger::Candidate,
    reference: ledger::AttemptRef,
) {
    value.mutation.attempt = Some(Attempt {
        instance: current.adapter.instance_id.clone(),
        id: reference.attempt_id,
    });
    value.mutation.original_request_id = Some(candidate.request_id.clone());
}
fn safe_failure(error: Failure) -> StoredOutcome {
    StoredOutcome::Failure { error }
}
fn stored_value(value: &StoredOutcome) -> Result<Value, Error> {
    serde_json::to_value(value).map_err(|_| Code::Unavailable.into())
}

/// Sole coordinator for a live owned child. Connection capture is credential
/// generation authority only (the existing registry port is named ReadUse).
/// It never replaces proof, audit, attempt or final dispatch admission.
pub(in crate::local::owner) fn execute(
    paths: &Paths,
    alias: &str,
    invocation: Invocation<'_>,
    child: &mut runtime::Child,
    control: Control<'_>,
) -> Result<Delivery, Error> {
    let Invocation {
        request,
        key,
        proof,
        until,
    } = invocation;
    let request = &request;
    let first = issuance::resolve(paths, alias, request, until)?;
    let request_id = Uuid::new_v4().to_string();
    if let Some(existing) = observe_as(paths, alias, request, key, until, &request_id, None)? {
        return Ok(existing);
    }
    let mut candidate = candidate(&first, key, &request_id)?;
    let mut audit_reference = None;
    let audits = audit_store(paths)?;
    // A failure before prepare is subject to the authoritative winner recheck.
    let initial: Result<_, Failure> = (|| {
        control.check()?;
        let proof = proof.ok_or(ApprovalCode::ApprovalRequired)?;
        let evidence = approvals::Evidence::from_document(proof).map_err(proof_error)?;
        let clock = time(&first, until)?;
        let current = issuance::resolve(paths, alias, request, until)?;
        unchanged(&first, &current)?;
        verify(paths, &current, &evidence, &clock)?;
        let bootstrap = child.bootstrap().clone();
        if serde_json::to_value(&bootstrap).map_err(|_| Code::Unavailable)?
            != serde_json::to_value(super::super::cached(paths, alias)?)
                .map_err(|_| Code::Unavailable)?
        {
            return Err(Code::ReadinessMismatch.into());
        }
        let requirement = bootstrap
            .requirements
            .iter()
            .find(|r| r.operation == request.operation)
            .ok_or(Code::NotFound)?;
        if requirement.effect != runtime::Effect::Write {
            return Err(Code::Unsupported.into());
        }
        let registry = registry::Registry::with_system_clock(&paths.state);
        let remaining = until
            .saturating_duration_since(Instant::now())
            .as_millis()
            .min(120_000) as u64;
        issuance::check(until)?;
        let captured = registry
            .capture_read(
                &bootstrap
                    .binding(&requirement.profile)
                    .map_err(Error::from)?,
                request.connection,
                &requirement.scopes,
                connectors_sdk::now_ms(),
                connectors_sdk::now_ms() + remaining,
            )
            .map_err(Error::from)?;
        let connection_use = Use {
            registry,
            captured: Some(captured),
            dispatched: None,
        };
        let captured = connection_use.captured.as_ref().ok_or(Code::Unavailable)?;
        let version = captured.version();
        let material = custody::Store::open_at(
            version.scope(),
            current.config.secret_service_socket.as_deref(),
        )
        .and_then(|store| store.read(version));
        let material = match material {
            Ok(value) => value,
            Err(error) => {
                if matches!(error, custody::Failure::Missing) {
                    connection_use
                        .registry
                        .invalidate_read(
                            captured,
                            registry::InvalidCredential::Missing,
                            connectors_sdk::now_ms(),
                        )
                        .map_err(Error::from)?;
                }
                return Err(Code::CustodyUnavailable.into());
            }
        };
        control.check()?;
        unchanged(&current, &issuance::resolve(paths, alias, request, until)?)?;
        // The declared preflight is provider work too; anchor it first. Confirm
        // this original receipt again at the business attempt boundary.
        let facts = anchor(&current, &request_id);
        let audit = admit_audit(&audits, &facts)?;
        audit_reference = Some(audit.reference().clone());
        let native = child
            .prepare_write_until(
                request.operation,
                request.revision,
                captured.partition(),
                &material,
                request.input.as_bytes(),
                (captured.expires_at_ms(), until),
            )
            .map_err(Error::from)?;
        drop(material);
        issuance::check(until)?;
        // No policy/key/metadata lock is held during native preflight or fresh
        // clock acquisition. Every subsequent decision uses current guards.
        let clock = time(&current, until)?;
        let final_current = issuance::resolve(paths, alias, request, until)?;
        unchanged(&current, &final_current)?;
        Ok((
            evidence,
            final_current,
            clock,
            connection_use,
            native,
            audit,
            facts,
        ))
    })();
    let (evidence, current, clock, mut connection_use, native, audit, facts) = match initial {
        Ok(value) => value,
        Err(error) => {
            let observed = observe_as(
                paths,
                alias,
                request,
                key,
                until,
                &request_id,
                audit_reference.as_ref(),
            );
            if let Ok(Some(existing)) = observed {
                return Ok(existing);
            }
            let mut value = failure(&first, &request_id, Classification::NotAttempted, error);
            if let Some(reference) = audit_reference {
                finish_audit(&audits, reference, &mut value);
            }
            // Current disclosure may be revoked while native preflight runs.
            // Complete our admitted audit before propagating that refusal.
            observed?;
            return Ok(value);
        }
    };
    let reference = audit_reference.ok_or(Code::MetadataUnavailable)?;
    let leases: Result<_, Failure> = (|| {
        let ledger = store(paths, ClockRef(&clock))?;
        let policy = issuance::policy_store(paths, &current.adapter)?
            .acquire(&current.policy.selection, request.operation)
            .map_err(issuance::policy_error)?;
        let keys = issuance::key_store(paths, &current.config, &current.adapter)?
            .acquire_key()
            .map_err(issuance::key_error)?;
        Ok((ledger, policy, keys))
    })();
    let (ledger, policy, keys) = match leases {
        Ok(value) => value,
        Err(error) => {
            let _ = native.cancel();
            let observed = observe_as(
                paths,
                alias,
                request,
                key,
                until,
                &request_id,
                Some(&reference),
            );
            if let Ok(Some(value)) = observed {
                return Ok(value);
            }
            let mut value = failure(&current, &request_id, Classification::NotAttempted, error);
            finish_audit(&audits, reference, &mut value);
            observed?;
            return Ok(value);
        }
    };
    let admitted = issuance::Admission {
        policy: &policy,
        key: &keys,
        subject: &current.preparation.subject,
    };
    let final_admission: Result<_, Failure> = (|| {
        unchanged(&current, &issuance::resolve(paths, alias, request, until)?)?;
        let verified =
            approvals::verify(&evidence, &current.preparation.subject, &admitted, &clock)
                .map_err(proof_error)?;
        audits
            .confirm(audit, &facts)
            .map_err(|_| Code::MetadataUnavailable)?;
        candidate.approval = ledger::Approval::Required {
            reference: evidence.reference().into(),
        };
        issuance::check(until)?;
        control.check()?;
        ledger
            .prepare_approved(&candidate, &verified)
            .map_err(ledger_error)
    })();
    let prepared = match final_admission {
        Ok(ledger::Preparation::Prepared(prepared)) => prepared,
        other => {
            // Destroy the native preparation before disclosing another winner
            // or returning a refusal. Drop terminates the exact child on loss.
            let _ = native.cancel();
            drop(keys);
            drop(policy);
            let observed = observe_as(
                paths,
                alias,
                request,
                key,
                until,
                &request_id,
                Some(&reference),
            );
            if let Ok(Some(existing)) = observed {
                return Ok(existing);
            }
            let error = match other {
                Err(error) => error,
                _ => Code::OutcomeUnknown.into(),
            };
            let mut value = failure(&current, &request_id, Classification::NotAttempted, error);
            finish_audit(&audits, reference, &mut value);
            observed?;
            return Ok(value);
        }
    };
    let attempt = prepared.reference();
    let pre_gate: Result<_, Failure> = (|| {
        issuance::check(until)?;
        unchanged(&current, &issuance::resolve(paths, alias, request, until)?)?;
        let spends =
            approvals::Store::new(&paths.state, ClockRef(&clock), 100_000).map_err(proof_error)?;
        let spent = spends
            .spend(
                &prepared,
                &evidence,
                &current.preparation.subject,
                &admitted,
            )
            .map_err(proof_error)?;
        let guard = control.lifecycle.lock().map_err(|_| Code::Unavailable)?;
        guard.check(control.epoch)?;
        if control.stopped.load(Ordering::SeqCst) {
            return Err(Code::Unavailable.into());
        }
        issuance::check(until)?;
        unchanged(&current, &issuance::resolve(paths, alias, request, until)?)?;
        approvals::verify(&evidence, &current.preparation.subject, &admitted, &clock)
            .map_err(proof_error)?;
        let captured = connection_use.captured.take().ok_or(Code::Unavailable)?;
        connection_use.dispatched = Some(
            connection_use
                .registry
                .dispatch_read(captured, connectors_sdk::now_ms())
                .map_err(Error::from)?,
        );
        Ok((spent, guard))
    })();
    let (spent, control_guard) = match pre_gate {
        Ok(value) => value,
        Err(error) => {
            let _ = native.cancel();
            let outcome = safe_failure(error.clone());
            let aborted = ledger.abort(attempt, stored_value(&outcome)?);
            let mut value = delivery(&current, &request_id, Classification::NotAttempted, outcome);
            correlation(&mut value, &current, &candidate, attempt);
            if aborted.is_err() {
                value.mutation.cause = Some(Cause {
                    code: connectors_core::ErrorCode::Unavailable,
                    stage: Stage::AttemptStore,
                });
            }
            finish_audit(&audits, reference, &mut value);
            return Ok(value);
        }
    };
    let gate = ledger.open_approved_dispatch(prepared, spent);
    let winner = match gate {
        Ok(winner) => winner,
        Err(error) => {
            let _ = native.cancel();
            let unknown = error == ledger::Failure::OutcomeUnknown;
            let outcome = safe_failure(if unknown {
                Code::OutcomeUnknown.into()
            } else {
                ledger_error(error)
            });
            if !unknown {
                let _ = ledger.abort(attempt, stored_value(&outcome)?);
            }
            let mut value = delivery(
                &current,
                &request_id,
                if unknown {
                    Classification::Unknown
                } else {
                    Classification::NotAttempted
                },
                outcome,
            );
            correlation(&mut value, &current, &candidate, attempt);
            finish_audit(&audits, reference, &mut value);
            return Ok(value);
        }
    };
    // No network, custody lookup, mutation retry or replacement frame can be
    // introduced between this live gate winner and its matching native commit.
    let final_check = issuance::check(until)
        .and_then(|_| {
            approvals::verify(&evidence, &current.preparation.subject, &admitted, &clock)
                .map(|_| ())
                .map_err(|_| Code::OutcomeUnknown.into())
        })
        .and_then(|_| {
            winner
                .consume()
                .map(|_| ())
                .map_err(|_| Code::OutcomeUnknown.into())
        });
    let native_result = if final_check.is_ok() {
        native.commit()
    } else {
        let _ = native.cancel();
        runtime::WriteResult {
            effect: runtime::WriteEffect::Unknown,
            result: Err(runtime::Failure::Timeout),
        }
    };
    drop(control_guard);
    let (classification, outcome) = match native_result.effect {
        runtime::WriteEffect::Applied => (
            Classification::Applied,
            match native_result.result {
                Ok(value) => StoredOutcome::Success { value },
                Err(error) => safe_failure(Error::from(error).into()),
            },
        ),
        runtime::WriteEffect::Refused => (
            Classification::Refused,
            safe_failure(
                native_result
                    .result
                    .err()
                    .map(Error::from)
                    .unwrap_or_else(|| Code::ServiceFailure.into())
                    .into(),
            ),
        ),
        runtime::WriteEffect::Unknown => (
            Classification::Unknown,
            safe_failure(Code::OutcomeUnknown.into()),
        ),
    };
    let payload = stored_value(&outcome)?;
    let settled = ledger.settle(
        attempt,
        &match classification {
            Classification::Applied => ledger::Outcome::Applied(payload),
            Classification::Refused => ledger::Outcome::Refused(payload),
            _ => ledger::Outcome::Unknown(payload),
        },
    );
    let mut value = delivery(&current, &request_id, classification, outcome);
    correlation(&mut value, &current, &candidate, attempt);
    if settled.is_err() {
        value.mutation.cause = Some(Cause {
            code: connectors_core::ErrorCode::Unavailable,
            stage: Stage::AttemptStore,
        });
    }
    finish_audit(&audits, reference, &mut value);
    // Final disclosure still requires current target admission. Policy and key
    // leases remain live through commit, settlement and this response decision.
    match issuance::resolve(paths, alias, request, until) {
        Ok(latest) => unchanged(&current, &latest)?,
        Err(error) if matches!(error.code, Code::Timeout | Code::Interrupted) => {
            value.mutation.cause = Some(Cause {
                code: connectors_core::ErrorCode::Timeout,
                stage: Stage::Response,
            });
            if value.result.is_some() {
                value.result = None;
                value.error = Some(error.into());
            }
        }
        Err(error) => return Err(error),
    }
    Ok(value)
}
