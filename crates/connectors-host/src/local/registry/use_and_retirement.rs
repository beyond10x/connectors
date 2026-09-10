use super::lifecycle::{bump, custody_version};
use super::*;

fn admitted_read(
    tx: &Transaction<'_>,
    binding: &Binding,
    reference: &str,
    required_scopes: &BTreeSet<String>,
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
    if row.binding != *binding {
        return Err(Failure::Conflict);
    }
    // Custody is only a bounded observation here. The captured material and
    // current fence are checked separately at the final dispatch boundary.
    if observation::readiness(tx, &row, now, true)? != State::Ready {
        return Err(Failure::NotReady);
    }
    let baseline = row.baseline.as_ref().ok_or(Failure::MetadataUnavailable)?;
    if !required_scopes.is_empty()
        && !baseline
            .granted_scopes
            .as_ref()
            .is_some_and(|s| required_scopes.is_subset(s))
    {
        return Err(Failure::InsufficientScope);
    }
    Ok(row)
}

/// One captured generation for a bounded read. This is not write approval,
/// permission evidence, or a resumable provider operation after process loss.
pub struct ReadUse {
    id: String,
    version: custody::Version,
    partition: String,
    expires: u64,
}
impl ReadUse {
    pub fn version(&self) -> custody::Version {
        self.version
    }
    pub fn partition(&self) -> &str {
        &self.partition
    }
    pub fn expires_at_ms(&self) -> u64 {
        self.expires
    }
    #[cfg(test)]
    pub(super) fn fixture_replay(&self) -> Self {
        Self {
            id: self.id.clone(),
            version: self.version,
            partition: self.partition.clone(),
            expires: self.expires,
        }
    }
}
pub struct DispatchedUse {
    id: String,
}

#[derive(Clone, Copy)]
pub enum InvalidCredential {
    Missing,
    Invalid,
    Revoked,
    Insufficient,
    Uncertain,
}
impl InvalidCredential {
    pub(super) fn value(self) -> &'static str {
        match self {
            Self::Missing => "missing",
            Self::Invalid => "invalid",
            Self::Revoked => "revoked",
            Self::Insufficient => "insufficient",
            Self::Uncertain => "uncertain",
        }
    }
}

pub struct Retirement {
    version_id: String,
    fence: String,
    version: custody::Version,
}
impl Retirement {
    pub fn version(&self) -> custody::Version {
        self.version
    }
}

impl Registry {
    /// Preflight without reserving a use or resolving material. Dispatch still
    /// requires a fresh capture and the final generation/fence checks.
    pub fn admit_read(
        &self,
        binding: &Binding,
        reference: &str,
        scopes: &BTreeSet<String>,
        now: u64,
    ) -> Result<()> {
        binding.validate()?;
        if !scopes_valid(scopes) {
            return Err(Failure::InvalidInput);
        }
        self.transaction(now, false, |tx, _, now| {
            admitted_read(tx, binding, reference, scopes, now).map(|_| ())
        })
    }
    pub fn capture_read(
        &self,
        binding: &Binding,
        reference: &str,
        required_scopes: &BTreeSet<String>,
        now: u64,
        expires_at: u64,
    ) -> Result<ReadUse> {
        binding.validate()?;
        if !scopes_valid(required_scopes)
            || expires_at <= now
            || expires_at > deadline(now, USE_MS)?
        {
            return Err(Failure::InvalidInput);
        }
        self.transaction(now, false, |tx, authority, now| {
            if expires_at <= now { return Err(Failure::Expired); }
            tx.execute("DELETE FROM registry_uses WHERE expires_at_ms<=?1", [timestamp(now)?]).map_err(db)?;
            let row = admitted_read(tx, binding, reference, required_scopes, now)?;
            let baseline = row.baseline.as_ref().ok_or(Failure::MetadataUnavailable)?;
            let count: i64 = tx.query_row("SELECT count(*) FROM registry_uses", [], |r| r.get(0)).map_err(db)?;
            if count>=1000 { return Err(Failure::Capacity); }
            let version_id = row.material.as_ref().ok_or(Failure::MetadataUnavailable)?;
            let id = new_id();
            // The original use deadline can only narrow. A later evidence
            // refresh cannot extend this captured admission's lifetime.
            let expires = expires_at.min(baseline.valid_until());
            tx.execute("INSERT INTO registry_uses (use_id,connection_ref,generation_id,version_id,publication_fence,expires_at_ms) VALUES (?1,?2,?3,?4,?5,?6)",
                params![id,reference,row.generation,version_id,row.fence,timestamp(expires)?]).map_err(db)?;
            let partition=connectors_core::digest(&serde_json::json!([authority.to_string(),reference,row.generation]));
            Ok(ReadUse {id,version:custody_version(authority,&row.scope_id,version_id)?,partition,expires})
        })
    }

    /// Logical dispatch opens exactly once after custody resolution and the
    /// caller's operation-specific policy/permission checks. The transport owner
    /// must consume this result for that one request; it grants no write effect.
    pub fn dispatch_read(&self, captured: ReadUse, now: u64) -> Result<DispatchedUse> {
        self.transaction(now, false, |tx, authority, now| {
            let (reference,generation,version,fence,expires,dispatched,released): (String,String,String,String,u64,bool,bool) = tx.query_row(
                "SELECT connection_ref,generation_id,version_id,publication_fence,expires_at_ms,dispatched,released FROM registry_uses WHERE use_id=?1", [&captured.id],
                |r| Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?,read_time(r,4)?,r.get(5)?,r.get(6)?))).optional().map_err(db)?.ok_or(Failure::Conflict)?;
            if now>=expires { return Err(Failure::Expired); }
            if dispatched || released { return Err(Failure::Conflict); }
            let row = Self::connection(tx, &reference)?;
            if row.state=="revoked" { return Err(Failure::Revoked); }
            if row.fence!=fence || row.generation.as_deref()!=Some(&generation) || row.material.as_deref()!=Some(&version)
                || captured.version!=custody_version(authority,&row.scope_id,&version)? { return Err(Failure::Conflict); }
            if observation::readiness(tx,&row,now,true)?!=State::Ready { return Err(Failure::NotReady); }
            tx.execute("UPDATE registry_uses SET dispatched=1 WHERE use_id=?1", [&captured.id]).map_err(db)?;
            Ok(DispatchedUse {id:captured.id})
        })
    }

    pub fn release_read(&self, dispatched: DispatchedUse, now: u64) -> Result<()> {
        self.transaction(now, false, |tx, _, _| {
            tx.execute(
                "UPDATE registry_uses SET released=1 WHERE use_id=?1 AND dispatched=1",
                [&dispatched.id],
            )
            .map_err(db)?;
            Ok(())
        })
    }

    pub fn cancel_read(&self, captured: ReadUse, now: u64) -> Result<()> {
        self.transaction(now, false, |tx, _, _| {
            tx.execute(
                "UPDATE registry_uses SET released=1 WHERE use_id=?1 AND dispatched=0",
                [&captured.id],
            )
            .map_err(db)?;
            Ok(())
        })
    }

    /// Only positive invalidity of the captured material creates a cutoff.
    /// An unavailable keyring/provider must never call this method as 'missing'.
    pub fn invalidate_read(
        &self,
        captured: &ReadUse,
        reason: InvalidCredential,
        now: u64,
    ) -> Result<()> {
        self.transaction(now, false, |tx, authority, _| {
            let (reference,version): (String,String) = tx.query_row("SELECT connection_ref,version_id FROM registry_uses WHERE use_id=?1", [&captured.id], |r| Ok((r.get(0)?,r.get(1)?))).optional().map_err(db)?.ok_or(Failure::Conflict)?;
            let row = Self::connection(tx,&reference)?;
            if captured.version!=custody_version(authority,&row.scope_id,&version)? { return Err(Failure::Conflict); }
            tx.execute("UPDATE registry_materials SET invalid_reason=COALESCE(invalid_reason,?2) WHERE version_id=?1", params![version,reason.value()]).map_err(db)?;
            tx.execute("UPDATE registry_uses SET released=1 WHERE version_id=?1 AND dispatched=0", [&version]).map_err(db)?;
            if row.material.as_deref()==Some(&version) { bump(tx,&row.binding.instance_id)?; }
            Ok(())
        })
    }

    /// Returns at most sixteen already-fenced versions. It does not establish a
    /// new retirement time or infer that unknown publication never happened.
    pub fn retirements(&self, now: u64) -> Result<Vec<Retirement>> {
        self.transaction(now, false, |tx, authority, now| {
            let candidates = tx.prepare("SELECT m.version_id,m.retirement_fence,c.scope_id FROM registry_materials m JOIN registry_connections c ON c.connection_ref=m.connection_ref WHERE m.deleted=0 AND m.retirement_fence IS NOT NULL AND m.delete_not_before_ms<=?1 ORDER BY m.delete_not_before_ms LIMIT 16").map_err(db)?
                .query_map([timestamp(now)?], |r| Ok((r.get::<_,String>(0)?,r.get::<_,String>(1)?,r.get::<_,String>(2)?))).map_err(db)?.collect::<std::result::Result<Vec<_>,_>>().map_err(db)?;
            let mut ready = Vec::new();
            for (id,fence,scope) in candidates {
                let retirement = Retirement {version_id:id.clone(),fence,version:custody_version(authority,&scope,&id)?};
                match deletion_allowed(tx,&retirement,authority,now) {
                    Ok(()) => ready.push(retirement),
                    Err(Failure::Conflict) => {},
                    Err(error) => return Err(error),
                }
            }
            Ok(ready)
        })
    }

    /// The backend holds its writer lock while this current guard runs. The
    /// metadata fence remains even when Delete or its acknowledgement is lost.
    pub fn delete_retired(
        &self,
        retirement: Retirement,
        store: &custody::Store,
        clock: impl Fn() -> u64,
    ) -> Result<()> {
        let mut refusal = None;
        let receipt = store
            .delete_guarded(retirement.version, || {
                let now = clock();
                self.transaction(now, false, |tx, authority, now| {
                    deletion_allowed(tx, &retirement, authority, now)
                })
                .map_err(|error| {
                    refusal = Some(error);
                    custody::Failure::Denied
                })
            })
            .map_err(|error| {
                refusal.unwrap_or(match error {
                    custody::Failure::OutcomeUnknown => Failure::OutcomeUnknown,
                    _ => Failure::CustodyUnavailable,
                })
            })?;
        if !receipt.matches(retirement.version) {
            return Err(Failure::Conflict);
        }
        let now = clock();
        self.transaction(now, false, |tx, authority, now| {
            deletion_allowed(tx,&retirement,authority,now)?;
            tx.execute("UPDATE registry_materials SET deleted=1,byte_size=NULL WHERE version_id=?1 AND retirement_fence=?2", params![retirement.version_id,retirement.fence]).map_err(db)?;
            Ok(())
        })
    }
}

fn deletion_allowed(
    tx: &Transaction<'_>,
    retirement: &Retirement,
    authority: Uuid,
    now: u64,
) -> Result<()> {
    let row: Option<(String,String,u64,bool,String,bool)> = tx.query_row(
        "SELECT c.scope_id,m.retirement_fence,m.delete_not_before_ms,m.deleted,a.state,c.active_material IS m.version_id FROM registry_materials m JOIN registry_connections c ON c.connection_ref=m.connection_ref JOIN registry_acquisitions a ON a.acquisition_ref=m.acquisition_ref WHERE m.version_id=?1 AND m.retirement_fence IS NOT NULL",
        [&retirement.version_id], |r| Ok((r.get(0)?,r.get(1)?,read_time(r,2)?,r.get(3)?,r.get(4)?,r.get(5)?))).optional().map_err(db)?;
    let (scope, fence, after, deleted, state, active) = row.ok_or(Failure::Conflict)?;
    if active
        || deleted
        || after > now
        || fence != retirement.fence
        || !matches!(state.as_str(), "completed" | "failed" | "expired")
        || retirement.version != custody_version(authority, &scope, &retirement.version_id)?
    {
        return Err(Failure::Conflict);
    }
    let used: bool = tx.query_row("SELECT EXISTS(SELECT 1 FROM registry_uses WHERE version_id=?1 AND released=0 AND expires_at_ms>?2)", params![retirement.version_id,timestamp(now)?], |r| r.get(0)).map_err(db)?;
    if used {
        return Err(Failure::Conflict);
    }
    Ok(())
}
