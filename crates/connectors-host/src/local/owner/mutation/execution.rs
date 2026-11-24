use super::*;
use crate::local::{approvals, clock, keyring::custody, registry, runtime, standing};
use connectors_sdk::Secret;
use std::sync::{
    Mutex,
    atomic::{AtomicBool, Ordering},
};

pub(in crate::local::owner) struct Control<'a> {
    pub lifecycle: &'a Mutex<super::super::lifecycle::Control>,
    pub epoch: u64,
    pub stopped: &'a AtomicBool,
    /// The pool this worker belongs to, through which a created credential is
    /// held on its custody target's own worker, never this one.
    pub pool: &'a std::sync::Weak<super::super::supervisor::Pool>,
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
        approvals::Failure::MetadataSchemaNewer(store) => Error::schema_newer(store).into(),
        approvals::Failure::OutcomeUnknown => Code::OutcomeUnknown.into(),
        approvals::Failure::Capacity => Code::Capacity.into(),
    }
}
pub(super) fn ledger_error(error: ledger::Failure) -> Failure {
    match error {
        ledger::Failure::InvalidInput => Code::InvalidInput,
        ledger::Failure::Capacity => Code::Capacity,
        ledger::Failure::BindingChanged | ledger::Failure::Conflict => Code::LifecycleConflict,
        ledger::Failure::ClockUnavailable => Code::Unavailable,
        ledger::Failure::OutcomeUnknown => Code::OutcomeUnknown,
        ledger::Failure::NotFound => Code::NotFound,
        ledger::Failure::MetadataUnavailable => Code::MetadataUnavailable,
        ledger::Failure::MetadataSchemaNewer(store) => return Error::schema_newer(store).into(),
    }
    .into()
}
/// One reading of the approval clock. A below-quorum attempt is made once
/// more while one query deadline and `reserve` are left before `until`
/// (`clock::acquire_within`); `reserve` is what the write still does inside
/// `until` after this reading, and no budget for the dispatch.
fn time(
    current: &issuance::Resolved,
    until: Instant,
    reserve: std::time::Duration,
) -> Result<clock::BoundedClock, Error> {
    issuance::check(until)?;
    let selected = current
        .config
        .approval_clock
        .as_ref()
        .ok_or(Code::InvalidConfiguration)?;
    let clock = selected
        .acquire_within(until, reserve)
        .map_err(|refused| Error::clock(selected, refused.failure))?;
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
/// What admitted the write: the proof that came with it, or a standing grant
/// (contracts/cli/v1alpha1/semantics.md, "`invoke` under a grant").
enum Approval<'a> {
    Proof(approvals::Evidence),
    Grant {
        /// The use taken under the grant; given back unless it is bound.
        taken: standing::Admitted<'a>,
        /// The input as `resolve` reads it, which the grant is checked against.
        input: Value,
    },
}
/// The owner's answer to a write a grant does not admit, and its line in the
/// owner's log. The answer never says which part of the grant was missed.
fn standing_error(paths: &Paths, refusal: standing::NotAdmitted) -> Failure {
    use standing::NotAdmitted::*;
    let (failure, logged): (Failure, Code) = match refusal {
        Uncovered(_) | Unverified => (ApprovalCode::ApprovalRequired.into(), Code::NotGranted),
        AtCap {
            retry_after_seconds,
        } => (Failure::rate_limited(retry_after_seconds), Code::Capacity),
        FileUnavailable | UsesUnavailable | ClockUnavailable => {
            (Code::Unavailable.into(), Code::Unavailable)
        }
    };
    crate::local::owner::diagnostics::record(&paths.state, refusal.event(), logged);
    failure
}
/// The grant path's recheck where the per-input path verifies its proof
/// again: the clock and the grant file are read afresh, under the held key
/// lease. `latest` is the instance as its configuration names it now; an
/// entry that no longer names a grant file holds no grant.
fn still_admitted(
    paths: &Paths,
    latest: &issuance::Resolved,
    operation: &str,
    input: &Value,
    keys: &crate::local::approval_keys::KeyUse,
    clock: &clock::BoundedClock,
    taken: &standing::Admitted<'_>,
) -> Result<(), Failure> {
    let Some(path) = latest.adapter.standing_grants.as_deref() else {
        return Err(standing_error(
            paths,
            standing::NotAdmitted::Uncovered(standing::Uncovered::NoGrant),
        ));
    };
    standing::still_admitted(
        path,
        keys.key().map_err(issuance::key_error)?,
        standing::Call {
            instance: &latest.adapter.instance_id,
            operation,
            input,
        },
        standing::reading(clock),
        taken,
    )
    .map_err(|refusal| standing_error(paths, refusal))
}
/// The grant path's admission, in place of the proof check and before the
/// audit anchor and the preflight: the published policy and the instance's
/// current key as the proof check holds them, then one call to
/// `standing::admit_in`, which takes one use in the store the state
/// directory holds. The leases end with this call.
fn admit<'a>(
    paths: &Paths,
    current: &issuance::Resolved,
    request: &issuance::Request<'_>,
    clock: &clock::BoundedClock,
    uses: &'a std::cell::OnceCell<standing::Uses>,
) -> Result<Approval<'a>, Failure> {
    let Some(path) = current.adapter.standing_grants.as_deref() else {
        return Err(standing_error(
            paths,
            standing::NotAdmitted::Uncovered(standing::Uncovered::NoGrant),
        ));
    };
    let policy = issuance::policy_store(paths, &current.adapter)?
        .acquire(&current.policy.selection, request.operation)
        .map_err(issuance::policy_error)?;
    if policy.policy().map_err(issuance::policy_error)? != &current.policy {
        return Err(Code::Forbidden.into());
    }
    let keys = issuance::key_store(paths, &current.config, &current.adapter)?
        .acquire_key()
        .map_err(issuance::key_error)?;
    // The value `resolve` checked, from the bytes the adapter is sent.
    let input: Value =
        connectors_core::read_json(request.input.as_bytes()).map_err(|_| Code::InvalidInput)?;
    let taken = standing::admit_in(
        path,
        keys.key().map_err(issuance::key_error)?,
        standing::Call {
            instance: &current.adapter.instance_id,
            operation: request.operation,
            input: &input,
        },
        standing::reading(clock),
        // The store the state directory holds, asked for once a grant covers
        // the write. The owner never creates it: `approvals grant-standing`
        // does, under the signing key, so a removed store answers
        // `unavailable` here and cannot start the hourly count again.
        || match uses.get() {
            Some(uses) => Ok(uses),
            None => standing::Uses::open(&paths.state).map(|opened| uses.get_or_init(|| opened)),
        },
    )
    .map_err(|refusal| standing_error(paths, refusal))?;
    Ok(Approval::Grant { taken, input })
}
/// Names the attempt in the answer and, for a write under a grant, the grant
/// bound to it.
fn correlation(
    value: &mut Delivery,
    current: &issuance::Resolved,
    candidate: &ledger::Candidate,
    reference: ledger::AttemptRef,
    approval: (&Paths, &Approval<'_>),
) {
    value.mutation.attempt = Some(Attempt {
        instance: current.adapter.instance_id.clone(),
        id: reference.attempt_id,
    });
    value.mutation.original_request_id = Some(candidate.request_id.clone());
    if let (paths, Approval::Grant { .. }) = approval {
        standing::named(&mut value.mutation, reference.attempt_id, &paths.state);
    }
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
    // The use store of the grant path, opened on that path only. The use a
    // grant admits borrows it until the write ends.
    let uses = std::cell::OnceCell::new();
    // A failure before prepare is subject to the authoritative winner recheck.
    let initial: Result<_, Failure> = (|| {
        control.check()?;
        // What the owner received decides the path. A proof is the per-input
        // path, in every step as before, and its refusal is final.
        let evidence = match standing::route(proof, key, first.adapter.standing_grants.as_deref()) {
            standing::Route::PerInput(proof) => {
                Some(approvals::Evidence::from_document(proof).map_err(proof_error)?)
            }
            standing::Route::Grant(_) => None,
            standing::Route::ApprovalRequired => {
                return Err(ApprovalCode::ApprovalRequired.into());
            }
        };
        // The second reading below still needs one attempt of its own.
        let clock = time(&first, until, clock::reserve_before_second_reading())?;
        let current = issuance::resolve(paths, alias, request, until)?;
        unchanged(&first, &current)?;
        let approval = match evidence {
            Some(evidence) => {
                verify(paths, &current, &evidence, &clock)?;
                Approval::Proof(evidence)
            }
            None => admit(paths, &current, request, &clock, &uses)?,
        };
        #[cfg(test)]
        seam::reached(seam::Step::Admitted);
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
        // A created credential that could not be held is foreseeable here,
        // before any provider work.
        if let Some(target) = super::custody::expected(&current.adapter.permissions, requirement)? {
            super::custody::admit(paths, target)?;
        }
        let registry = registry::Registry::with_system_clock(&paths.state);
        let remaining = until
            .saturating_duration_since(Instant::now())
            .as_millis()
            .min(120_000) as u64;
        issuance::check(until)?;
        let binding = bootstrap
            .binding(&requirement.profile)
            .map_err(Error::from)?;
        let capture = || {
            registry.capture_read(
                &binding,
                request.connection,
                &requirement.scopes,
                connectors_sdk::now_ms(),
                connectors_sdk::now_ms() + remaining,
            )
        };
        let captured = match capture() {
            // Only the provider executable changed since the connection was
            // saved (D12): validated once under this executable before anything
            // is sent, as a read is (`owner::follow_executable`).
            Err(registry::Failure::ExecutableChanged) => {
                let deadline = connectors_sdk::now_ms() + remaining;
                crate::local::owner::follow_executable(
                    paths,
                    &current.config.selection(),
                    &registry,
                    &binding,
                    request.connection,
                    deadline,
                    || {
                        // Held until the validation's dispatch is recorded.
                        let guard = control.lifecycle.lock().map_err(|_| Code::Unavailable)?;
                        guard.check(control.epoch)?;
                        if control.stopped.load(Ordering::SeqCst) {
                            return Err(Code::Unavailable.into());
                        }
                        Ok(guard)
                    },
                    |material| {
                        let result = child.validate(&requirement.profile, material, deadline);
                        (result, 1, None)
                    },
                )?;
                issuance::check(until)?;
                capture().map_err(Error::from)?
            }
            other => other.map_err(Error::from)?,
        };
        let connection_use = Use {
            registry,
            captured: Some(captured),
            dispatched: None,
        };
        let captured = connection_use.captured.as_ref().ok_or(Code::Unavailable)?;
        let version = captured.version();
        let selection = current.config.selection();
        #[cfg(test)]
        let seamed = seam::credential();
        #[cfg(not(test))]
        let seamed = None;
        let material = match seamed {
            Some(material) => Ok(material),
            None => custody::Store::open_for(version.scope(), &selection, &paths.state)
                .and_then(|store| store.read(version)),
        };
        let material = match material {
            Ok(value) => value,
            Err(error) => {
                if crate::local::owner::read_invalidates(&selection, error) {
                    connection_use
                        .registry
                        .invalidate_read(
                            captured,
                            registry::InvalidCredential::Missing,
                            connectors_sdk::now_ms(),
                        )
                        .map_err(Error::from)?;
                }
                return Err(crate::local::owner::read_error(&selection, error).into());
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
        let clock = time(&current, until, clock::RESERVE_AFTER_CLOCK)?;
        let final_current = issuance::resolve(paths, alias, request, until)?;
        unchanged(&current, &final_current)?;
        // The commit expects created bytes exactly as the configuration it
        // runs under and the adapter agree, whatever changed since admission.
        let custody_target =
            super::custody::expected(&final_current.adapter.permissions, requirement)?.cloned();
        Ok((
            approval,
            final_current,
            clock,
            connection_use,
            native,
            audit,
            facts,
            custody_target,
        ))
    })();
    let (approval, current, clock, mut connection_use, native, audit, facts, custody_target) =
        match initial {
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
                    finish_audit(&audits, reference, &mut value, until);
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
            finish_audit(&audits, reference, &mut value, until);
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
        let latest = issuance::resolve(paths, alias, request, until)?;
        unchanged(&current, &latest)?;
        let verified = match &approval {
            Approval::Proof(evidence) => Some(
                approvals::verify(evidence, &current.preparation.subject, &admitted, &clock)
                    .map_err(proof_error)?,
            ),
            // The first recheck: no attempt is recorded yet, and a refusal
            // here gives the use back.
            Approval::Grant { taken, input } => {
                still_admitted(
                    paths,
                    &latest,
                    request.operation,
                    input,
                    &keys,
                    &clock,
                    taken,
                )?;
                None
            }
        };
        audits
            .confirm(audit, &facts)
            .map_err(|_| Code::MetadataUnavailable)?;
        if let Approval::Proof(evidence) = &approval {
            candidate.approval = ledger::Approval::Required {
                reference: evidence.reference().into(),
            };
        }
        issuance::check(until)?;
        control.check()?;
        match &verified {
            Some(verified) => ledger.prepare_approved(&candidate, verified),
            // Under a grant the attempt records `not_required`, as the served
            // host's do; the grant's id is bound to it in the use store.
            None => ledger.prepare(&candidate),
        }
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
            finish_audit(&audits, reference, &mut value, until);
            observed?;
            return Ok(value);
        }
    };
    let attempt = prepared.reference();
    let pre_gate: Result<_, Failure> = (|| {
        issuance::check(until)?;
        unchanged(&current, &issuance::resolve(paths, alias, request, until)?)?;
        let spent = match &approval {
            Approval::Proof(evidence) => {
                let spends = approvals::Store::new(&paths.state, ClockRef(&clock), 100_000)
                    .map_err(proof_error)?;
                Some(
                    spends
                        .spend(&prepared, evidence, &current.preparation.subject, &admitted)
                        .map_err(proof_error)?,
                )
            }
            // A grant has no spend. Its use is bound to the attempt here and
            // stays counted from now on, whatever becomes of the write.
            Approval::Grant { taken, .. } => {
                taken
                    .bind(attempt.attempt_id)
                    .map_err(|_| standing_error(paths, standing::NotAdmitted::UsesUnavailable))?;
                #[cfg(test)]
                seam::reached(seam::Step::Bound);
                None
            }
        };
        let guard = control.lifecycle.lock().map_err(|_| Code::Unavailable)?;
        guard.check(control.epoch)?;
        if control.stopped.load(Ordering::SeqCst) {
            return Err(Code::Unavailable.into());
        }
        issuance::check(until)?;
        let latest = issuance::resolve(paths, alias, request, until)?;
        unchanged(&current, &latest)?;
        match &approval {
            Approval::Proof(evidence) => {
                approvals::verify(evidence, &current.preparation.subject, &admitted, &clock)
                    .map_err(proof_error)?;
            }
            // The second recheck, and the grant's last: a refusal leaves an
            // aborted attempt that names the grant.
            Approval::Grant { taken, input } => still_admitted(
                paths,
                &latest,
                request.operation,
                input,
                &keys,
                &clock,
                taken,
            )?,
        }
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
            correlation(
                &mut value,
                &current,
                &candidate,
                attempt,
                (paths, &approval),
            );
            if aborted.is_err() {
                value.mutation.cause = Some(Cause {
                    code: connectors_core::ErrorCode::Unavailable,
                    stage: Stage::AttemptStore,
                });
            }
            finish_audit(&audits, reference, &mut value, until);
            return Ok(value);
        }
    };
    let gate = match spent {
        Some(spent) => ledger.open_approved_dispatch(prepared, spent),
        // The ledger's gate does not enforce the grant; the order above does.
        None => ledger.open_dispatch(prepared),
    };
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
            correlation(
                &mut value,
                &current,
                &candidate,
                attempt,
                (paths, &approval),
            );
            finish_audit(&audits, reference, &mut value, until);
            return Ok(value);
        }
    };
    // No network, custody lookup, mutation retry or replacement frame can be
    // introduced between this live gate winner and its matching native commit.
    let final_check = issuance::check(until)
        .and_then(|_| match &approval {
            Approval::Proof(evidence) => {
                approvals::verify(evidence, &current.preparation.subject, &admitted, &clock)
                    .map(|_| ())
                    .map_err(|_| Code::OutcomeUnknown.into())
            }
            // The recheck before the gate was the grant's last: no file is
            // read between the gate and the commit.
            Approval::Grant { .. } => Ok(()),
        })
        .and_then(|_| {
            winner
                .consume()
                .map(|_| ())
                .map_err(|_| Code::OutcomeUnknown.into())
        });
    let (native_result, created) = if final_check.is_err() {
        let _ = native.cancel();
        (
            runtime::WriteResult {
                effect: runtime::WriteEffect::Unknown,
                result: Err(runtime::Failure::Timeout),
            },
            None,
        )
    } else if custody_target.is_some() {
        native.commit_holding()
    } else {
        (native.commit(), None)
    };
    drop(control_guard);
    let (classification, outcome) = native_outcome(native_result);
    // Held before the attempt settles, on the target's own worker. Whatever
    // befalls custody, the write stays applied.
    let outcome = match (custody_target, created, outcome) {
        (Some(target), Some(secret), Ok(value)) if classification == Classification::Applied => {
            super::custody::hold(paths, control.pool, &target, secret, value, until)
        }
        (_, _, outcome) => outcome,
    };
    let outcome = StoredOutcome::from(outcome);
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
    correlation(
        &mut value,
        &current,
        &candidate,
        attempt,
        (paths, &approval),
    );
    if settled.is_err() {
        value.mutation.cause = Some(Cause {
            code: connectors_core::ErrorCode::Unavailable,
            stage: Stage::AttemptStore,
        });
    }
    finish_audit(&audits, reference, &mut value, until);
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

/// What a test of `execute` sets on its own thread: the credential, in place
/// of the custody read (a unit test has no Secret Service and no mount), and
/// a hook called between two steps of the grant path. No production build
/// holds this module.
#[cfg(test)]
pub(super) mod seam {
    use connectors_sdk::Secret;
    use std::cell::RefCell;

    /// A point of `execute` a test acts at.
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub enum Step {
        /// The write is admitted; nothing is prepared and no recheck has run.
        Admitted,
        /// The use is bound to the attempt; the recheck before the gate is next.
        Bound,
    }
    pub struct Seam {
        pub credential: Vec<u8>,
        pub at: Box<dyn Fn(Step)>,
    }
    thread_local! {
        static SEAM: RefCell<Option<Seam>> = const { RefCell::new(None) };
    }
    // The tests that set a seam are Unix-only (`mutation::tests`).
    #[cfg(unix)]
    pub fn set(seam: Option<Seam>) {
        SEAM.with(|slot| *slot.borrow_mut() = seam);
    }
    pub(super) fn credential() -> Option<Secret> {
        SEAM.with(|slot| {
            slot.borrow()
                .as_ref()
                .map(|seam| Secret(seam.credential.clone()))
        })
    }
    pub(super) fn reached(step: Step) {
        SEAM.with(|slot| {
            if let Some(seam) = slot.borrow().as_ref() {
                (seam.at)(step);
            }
        });
    }
}
