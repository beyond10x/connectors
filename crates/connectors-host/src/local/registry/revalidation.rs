use super::lifecycle::{bump, custody_version, recollected_snapshot};
use super::*;

/// Private one-use capture. It cannot be serialized, resumed, or converted into
/// business-read authority. The existing use table supplies bounded retention.
#[cfg_attr(test, derive(Clone))]
pub struct Revalidation {
    id: String,
    version: custody::Version,
    captured: u64,
}
impl Revalidation {
    pub fn version(&self) -> custody::Version {
        self.version
    }
}
#[cfg_attr(test, derive(Clone))]
pub struct RevalidationDispatch {
    captured: Revalidation,
    consumed: u64,
}

fn admitted(
    tx: &Transaction<'_>,
    binding: &Binding,
    reference: &str,
    revision: &str,
    now: u64,
) -> Result<ConnectionRow> {
    let row = Registry::connection(tx, reference)?;
    if !row.public
        || row.binding.instance_id != binding.instance_id
        || row.binding.adapter_id != binding.adapter_id
    {
        return Err(Failure::NotFound);
    }
    if row.state == "revoked" {
        return Err(Failure::Revoked);
    }
    if row.binding != *binding || row.revision != revision {
        return Err(Failure::Conflict);
    }
    if row.material.is_none() {
        return Err(Failure::NotReady);
    }
    match observation::readiness(tx, &row, now, true)? {
        State::Ready | State::Pending => Ok(row),
        _ => Err(Failure::NotReady),
    }
}

fn current(
    tx: &Transaction<'_>,
    authority: Uuid,
    captured: &Revalidation,
    dispatched: bool,
    now: u64,
) -> Result<ConnectionRow> {
    let (reference, generation, version, fence, expires, used, released): (String,String,String,String,u64,bool,bool) =
        tx.query_row("SELECT connection_ref,generation_id,version_id,publication_fence,expires_at_ms,dispatched,released FROM registry_uses WHERE use_id=?1",
            [&captured.id], |r| Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?,read_time(r,4)?,r.get(5)?,r.get(6)?)))
            .optional().map_err(db)?.ok_or(Failure::Conflict)?;
    if now >= expires {
        return Err(Failure::Expired);
    }
    if used != dispatched || released {
        return Err(Failure::Conflict);
    }
    let row = Registry::connection(tx, &reference)?;
    if row.state == "revoked" {
        return Err(Failure::Revoked);
    }
    if row.fence != fence
        || row.generation.as_deref() != Some(&generation)
        || row.material.as_deref() != Some(&version)
        || captured.version != custody_version(authority, &row.scope_id, &version)?
    {
        return Err(Failure::Conflict);
    }
    admitted(tx, &row.binding, &reference, &row.revision, now)
}

fn release(tx: &Transaction<'_>, id: &str) -> Result<()> {
    tx.execute("UPDATE registry_uses SET released=1 WHERE use_id=?1", [id])
        .map_err(db)?;
    Ok(())
}

fn invalidate(tx: &Transaction<'_>, row: &ConnectionRow, reason: InvalidCredential) -> Result<()> {
    tx.execute("UPDATE registry_materials SET invalid_reason=COALESCE(invalid_reason,?2) WHERE version_id=?1",
        params![row.material, reason.value()]).map_err(db)?;
    tx.execute(
        "UPDATE registry_uses SET released=1 WHERE version_id=?1 AND dispatched=0",
        [&row.material],
    )
    .map_err(db)?;
    bump(tx, &row.binding.instance_id)
}

impl Registry {
    pub fn admit_revalidation(
        &self,
        binding: &Binding,
        reference: &str,
        revision: &str,
        now: u64,
    ) -> Result<()> {
        binding.validate()?;
        self.transaction(now, false, |tx, _, now| {
            admitted(tx, binding, reference, revision, now).map(|_| ())
        })
    }

    pub fn capture_revalidation(
        &self,
        binding: &Binding,
        reference: &str,
        revision: &str,
        now: u64,
        expires: u64,
    ) -> Result<Revalidation> {
        binding.validate()?;
        if expires <= now || expires > deadline(now, 30_000)? {
            return Err(Failure::InvalidInput);
        }
        self.transaction(now, false, |tx, authority, now| {
            if expires <= now { return Err(Failure::Expired); }
            let row = admitted(tx,binding,reference,revision,now)?;
            tx.execute("DELETE FROM registry_uses WHERE expires_at_ms<=?1", [timestamp(now)?]).map_err(db)?;
            let count: i64 = tx.query_row("SELECT count(*) FROM registry_uses", [], |r| r.get(0)).map_err(db)?;
            if count >= 1000 { return Err(Failure::Capacity); }
            let version = row.material.as_ref().ok_or(Failure::MetadataUnavailable)?;
            let id = new_id();
            tx.execute("INSERT INTO registry_uses (use_id,connection_ref,generation_id,version_id,publication_fence,expires_at_ms) VALUES (?1,?2,?3,?4,?5,?6)",
                params![id,reference,row.generation,version,row.fence,timestamp(expires)?]).map_err(db)?;
            Ok(Revalidation {id, version:custody_version(authority,&row.scope_id,version)?, captured:now})
        })
    }

    pub fn cancel_revalidation(&self, captured: Revalidation, now: u64) -> Result<()> {
        self.transaction(now, false, |tx, _, _| release(tx, &captured.id))
    }

    /// A positive missing-version observation, never an unavailable service.
    pub fn missing_revalidation(&self, captured: Revalidation, now: u64) -> Result<()> {
        self.transaction(now, false, |tx, authority, now| {
            let row = current(tx, authority, &captured, false, now)?;
            invalidate(tx, &row, InvalidCredential::Missing)?;
            release(tx, &captured.id)
        })
    }

    pub fn dispatch_revalidation(
        &self,
        captured: Revalidation,
        now: u64,
    ) -> Result<RevalidationDispatch> {
        self.transaction(now, false, |tx, authority, now| {
            current(tx, authority, &captured, false, now)?;
            tx.execute(
                "UPDATE registry_uses SET dispatched=1 WHERE use_id=?1",
                [&captured.id],
            )
            .map_err(db)?;
            Ok(RevalidationDispatch {
                captured,
                consumed: now,
            })
        })
    }

    /// Every terminal outcome consumes this dispatch, including transient failure.
    /// A failed transaction/unknown acknowledgement never grants replay authority.
    pub fn finish_revalidation(
        &self,
        dispatched: RevalidationDispatch,
        result: std::result::Result<ValidatedBaseline, Option<InvalidCredential>>,
        now: u64,
    ) -> Result<()> {
        self.transaction(now, false, |tx, authority, now| {
            let row = current(tx,authority,&dispatched.captured,true,now)?;
            let decision = match result {
                Ok(mut native) => {
                    // Reject malformed or stale adapter evidence before treating
                    // its identity as a proved mismatch in the current material.
                    native.validate(&row.binding,dispatched.consumed,now)?;
                    if row.identity.as_ref() != Some(&native.identity) {
                        invalidate(tx,&row,InvalidCredential::Invalid)?;
                        Err(Failure::IdentityMismatch)
                    } else {
                        // Recollection cannot erase a known expiry for these
                        // unchanged bytes, even if an upstream response omits it.
                        if let Some(known) = row.baseline.as_ref().and_then(|b| b.credential_expires_at) {
                            native.credential_expires_at_ms = Some(native.credential_expires_at_ms.map_or(known, |expiry| expiry.min(known)));
                            native.valid_until_ms = native.valid_until_ms.min(known);
                        }
                        native.validate(&row.binding,dispatched.consumed,now)?;
                        let snapshot = recollected_snapshot(&row.binding,&row.reference,
                            row.generation.as_deref().ok_or(Failure::MetadataUnavailable)?,
                            &native,dispatched.captured.captured,dispatched.consumed)?;
                        tx.execute("UPDATE registry_connections SET baseline=?2,publication_fence=?3 WHERE connection_ref=?1",
                            params![row.reference,encode(&snapshot)?,new_id()]).map_err(db)?;
                        bump(tx,&row.binding.instance_id)?;
                        Ok(())
                    }
                }
                Err(reason) => {
                    if let Some(reason) = reason { invalidate(tx,&row,reason)?; }
                    Ok(())
                }
            };
            release(tx,&dispatched.captured.id)?;
            // Commit positive invalidity before returning the semantic refusal.
            Ok(decision)
        })?
    }
}
