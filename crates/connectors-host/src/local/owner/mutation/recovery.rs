//! Exact-key recovery only on the serialized instance worker, after every
//! earlier invocation on that worker has ended. The owner lifetime lock excludes
//! another process's live dispatcher; a busy hint alone is never ownership.
use super::*;
use crate::local::clock;

struct RecoveryClock(Option<clock::BoundedClock>);
impl ledger::Clock for RecoveryClock {
    fn now(&self) -> ledger::Result<ledger::ClockInterval> {
        ledger::Clock::now(self.0.as_ref().ok_or(ledger::Failure::ClockUnavailable)?)
    }
}

pub(in crate::local::owner) fn recover_observation(
    paths: &Paths,
    alias: &str,
    request: &issuance::Request<'_>,
    key: &str,
    until: Instant,
    quiescent: super::super::supervisor::Quiescent<'_>,
) -> Result<Delivery, Error> {
    let first = issuance::resolve(paths, alias, request, until)?;
    if first.adapter.instance_id != quiescent.instance() {
        return Err(Code::LifecycleConflict.into());
    }
    let candidate = candidate(&first, Some(key), &Uuid::new_v4().to_string())?;
    let ledger = store(paths, NoClock)?;
    let original = ledger.lookup(&candidate);
    let recovery = (|| -> Result<(), Error> {
        let original = match original {
            Ok(Some(original)) => original,
            // The final admitted observation handles conflicts/unavailability.
            _ => return Ok(()),
        };
        if !matches!(
            original.state,
            ledger::State::Prepared | ledger::State::Dispatching
        ) {
            return Ok(());
        }
        // Known non-dispatch needs trusted settlement time for replay retention.
        // Quarantining uncertain dispatch has no expiry and needs no clock.
        let clock = if original.state == ledger::State::Prepared {
            issuance::check(until)?;
            Some(
                first
                    .config
                    .approval_clock
                    .as_ref()
                    .ok_or(Code::Unavailable)?
                    .acquire()
                    .map_err(|_| Code::Unavailable)?,
            )
        } else {
            None
        };
        issuance::check(until)?;
        let policy = issuance::policy_store(paths, &first.adapter)?
            .acquire(&first.policy.selection, request.operation)
            .map_err(issuance::policy_error)?;
        if policy.policy().map_err(issuance::policy_error)? != &first.policy {
            return Err(Code::Forbidden.into());
        }
        unchanged(&first, &issuance::resolve(paths, alias, request, until)?)?;
        store(paths, RecoveryClock(clock))?
            .recover(original.reference)
            .map_err(|_| Code::MetadataUnavailable)?;
        Ok(())
    })();
    // No recovery acknowledgement is a send receipt. On an uncertain commit,
    // read the authoritative original once; never repeat the recovery mutation.
    let mut value =
        observe(paths, alias, request, Some(key), until)?.ok_or(Code::OutcomeUnknown)?;
    if let Err(error) = recovery {
        if matches!(
            error.code,
            Code::Forbidden | Code::Revoked | Code::NotGranted | Code::LifecycleConflict
        ) {
            return Err(error);
        }
        value.mutation.cause = Some(Cause {
            code: connectors_core::ErrorCode::Unavailable,
            stage: Stage::Observation,
        });
    }
    Ok(value)
}
