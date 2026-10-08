use super::lifecycle::{bump, custody_version, recollected_snapshot};
use super::*;

/// Private one-use capture. It cannot be serialized, resumed, or converted into
/// business-read authority. The existing use table supplies bounded retention.
#[cfg_attr(test, derive(Clone))]
pub struct Revalidation {
    id: String,
    version: custody::Version,
    captured: u64,
    /// The configured binding a configuration upgrade publishes the
    /// connection under; None when the connection already has it.
    upgrade: Option<Binding>,
}
impl Revalidation {
    pub fn version(&self) -> custody::Version {
        self.version
    }
    /// Whether this revalidation publishes the connection under a newer
    /// configuration revision of its instance.
    pub fn upgrades(&self) -> bool {
        self.upgrade.is_some()
    }
}
#[cfg_attr(test, derive(Clone))]
pub struct RevalidationDispatch {
    captured: Revalidation,
    consumed: u64,
}

/// Also answers whether revalidation upgrades the connection: its binding
/// names another configuration revision of its instance, under the same
/// provider authority and profile declaration as the configured binding. Any
/// other difference refuses as BindingChanged: only a new connection helps.
fn admitted(
    tx: &Transaction<'_>,
    binding: &Binding,
    reference: &str,
    revision: &str,
    now: u64,
) -> Result<(ConnectionRow, bool)> {
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
    if row.revision != revision {
        return Err(Failure::Conflict);
    }
    let upgrade = row.binding != *binding;
    if upgrade && !upgradable(&row.binding, binding) {
        return Err(Failure::BindingChanged);
    }
    if row.material.is_none() {
        return Err(Failure::NotReady);
    }
    match observation::readiness(tx, &row, now, true)? {
        State::Ready | State::Pending => Ok((row, upgrade)),
        _ => Err(Failure::NotReady),
    }
}

/// The refusal of a connection whose binding is not the configured one. It is
/// `pending` and no status change is pending for it, so it names the step that
/// clears it, never the plain `Conflict` a retry would repeat. A revalidation
/// follows a configuration upgrade only when it can: the authentication is
/// unchanged, revalidation admits the credential (`usable`: material that is
/// neither known invalid nor expired, as `admitted` requires), and the new
/// provider has not already refused it under the configured revision. Anything
/// else needs a new connection: repair refuses a changed binding.
pub(super) fn changed(row: &ConnectionRow, configured: &Binding, usable: bool) -> Failure {
    if usable
        && upgradable(&row.binding, configured)
        && row.refused.as_deref() != Some(configured.configuration_revision.as_str())
    {
        Failure::UpgradeRequired
    } else {
        Failure::BindingChanged
    }
}

/// The profile declaration's revision digests its scheme, capability, entry
/// fields, scopes and evidence lifetime, so an equal declaration is unchanged
/// authentication. Only the configuration revision may differ.
fn upgradable(admitted: &Binding, configured: &Binding) -> bool {
    admitted.instance_id == configured.instance_id
        && admitted.adapter_id == configured.adapter_id
        && admitted.provider_authority == configured.provider_authority
        && admitted.profile == configured.profile
        && admitted.configuration_revision != configured.configuration_revision
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
    let binding = captured.upgrade.as_ref().unwrap_or(&row.binding);
    admitted(tx, binding, &reference, &row.revision, now).map(|(row, _)| row)
}

/// A revoked connection is a terminal record: it records no configuration
/// revision of its own and follows its instance's (metadata/er.rs), and no
/// refused upgrade.
pub(super) fn follow_instance(tx: &Transaction<'_>, instance: &str, revision: &str) -> Result<()> {
    let revoked = tx
        .prepare("SELECT connection_ref,binding FROM registry_connections WHERE instance_id=?1 AND state='revoked'")
        .map_err(db)?
        .query_map([instance], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))
        .map_err(db)?
        .collect::<std::result::Result<Vec<_>, _>>()
        .map_err(db)?;
    for (reference, binding) in revoked {
        let (mut binding, refused) = decode_binding(&binding)?;
        if binding.configuration_revision != revision || refused.is_some() {
            binding.configuration_revision = revision.to_owned();
            tx.execute(
                "UPDATE registry_connections SET binding=?2 WHERE connection_ref=?1",
                params![reference, encode(&binding)?],
            )
            .map_err(db)?;
        }
    }
    Ok(())
}

/// Record that the new provider refused this connection's credential under
/// the upgrade's configured revision (`Connection.refused_configuration_revision`).
/// It invalidates nothing under the connection's own revision; a read under
/// that configured revision then names a new connection (`changed`).
fn refuse_upgrade(tx: &Transaction<'_>, row: &ConnectionRow, upgrade: &Binding) -> Result<()> {
    if row.refused.as_deref() == Some(upgrade.configuration_revision.as_str()) {
        return Ok(());
    }
    tx.execute(
        "UPDATE registry_connections SET binding=?2 WHERE connection_ref=?1",
        params![
            row.reference,
            encode_binding(&row.binding, Some(&upgrade.configuration_revision))?
        ],
    )
    .map_err(db)?;
    Ok(())
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
    /// Resolve only the retained identity needed to select the configured
    /// profile. This does not sample or persist time and grants no provider
    /// work; admit_revalidation performs the authoritative current check.
    pub fn revalidation_profile(
        &self,
        instance: &str,
        adapter: &str,
        reference: &str,
        revision: &str,
    ) -> Result<String> {
        self.revalidation_binding(instance, adapter, reference, revision)
            .map(|binding| binding.profile.id)
    }

    /// The binding the connection was admitted or last upgraded under, on the
    /// same terms as revalidation_profile.
    pub fn revalidation_binding(
        &self,
        instance: &str,
        adapter: &str,
        reference: &str,
        revision: &str,
    ) -> Result<Binding> {
        let mut metadata =
            Metadata::inspect(&self.path).map_err(|_| Failure::MetadataUnavailable)?;
        let tx = metadata.connection.transaction().map_err(db)?;
        let row = Self::connection(&tx, reference)?;
        observation::visible(&row, instance, adapter)?;
        if row.revision != revision {
            return Err(Failure::Conflict);
        }
        if row.state == "revoked" {
            return Err(Failure::Revoked);
        }
        Ok(row.binding)
    }

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
        self.transaction_admission(now, false, |tx, authority, now| {
            if expires <= now { return Err(Failure::Expired); }
            let (row, upgrade) = admitted(tx,binding,reference,revision,now)?;
            tx.execute("DELETE FROM registry_uses WHERE expires_at_ms<=?1", [timestamp(now)?]).map_err(db)?;
            let count: i64 = tx.query_row("SELECT count(*) FROM registry_uses", [], |r| r.get(0)).map_err(db)?;
            if count >= 1000 { return Err(Failure::Capacity); }
            let version = row.material.as_ref().ok_or(Failure::MetadataUnavailable)?;
            let id = new_id();
            tx.execute("INSERT INTO registry_uses (use_id,connection_ref,generation_id,version_id,publication_fence,expires_at_ms) VALUES (?1,?2,?3,?4,?5,?6)",
                params![id,reference,row.generation,version,row.fence,timestamp(expires)?]).map_err(db)?;
            Ok(Revalidation {id, version:custody_version(authority,&row.scope_id,version)?, captured:now,
                upgrade: upgrade.then(|| binding.clone())})
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
    ///
    /// A configuration upgrade publishes the connection under the configured
    /// binding, and moves its instance to that revision, only when the new
    /// provider's validation answers the recorded identity. Every other outcome
    /// of an upgrade changes nothing: the credential is not proved invalid under
    /// the configuration it was admitted under.
    pub fn finish_revalidation(
        &self,
        dispatched: RevalidationDispatch,
        result: std::result::Result<ValidatedBaseline, Option<InvalidCredential>>,
        now: u64,
    ) -> Result<()> {
        self.transaction(now, false, |tx, authority, now| {
            let row = current(tx,authority,&dispatched.captured,true,now)?;
            let upgrade = dispatched.captured.upgrade.as_ref();
            let binding = upgrade.unwrap_or(&row.binding);
            let decision = match result {
                Ok(mut native) => {
                    // Reject malformed or stale adapter evidence before treating
                    // its identity as a proved mismatch in the current material.
                    native.validate(binding,dispatched.consumed,now)?;
                    if row.identity.as_ref() != Some(&native.identity) {
                        match upgrade { None => invalidate(tx,&row,InvalidCredential::Invalid)?, Some(upgrade) => refuse_upgrade(tx,&row,upgrade)? }
                        Err(if upgrade.is_some() { Failure::UpgradeIdentityMismatch } else { Failure::IdentityMismatch })
                    } else {
                        // Recollection cannot erase a known expiry for these
                        // unchanged bytes, even if an upstream response omits it.
                        if let Some(known) = row.baseline.as_ref().and_then(|b| b.credential_expires_at) {
                            native.credential_expires_at_ms = Some(native.credential_expires_at_ms.map_or(known, |expiry| expiry.min(known)));
                            native.valid_until_ms = native.valid_until_ms.min(known);
                        }
                        native.validate(binding,dispatched.consumed,now)?;
                        let snapshot = recollected_snapshot(binding,&row.reference,
                            row.generation.as_deref().ok_or(Failure::MetadataUnavailable)?,
                            &native,dispatched.captured.captured,dispatched.consumed)?;
                        tx.execute("UPDATE registry_connections SET baseline=?2,publication_fence=?3 WHERE connection_ref=?1",
                            params![row.reference,encode(&snapshot)?,new_id()]).map_err(db)?;
                        if let Some(upgrade) = upgrade {
                            // The binding changed, so the public revision advances.
                            tx.execute("UPDATE registry_connections SET binding=?2,semantic_revision=?3 WHERE connection_ref=?1",
                                params![row.reference,encode(upgrade)?,new_id()]).map_err(db)?;
                            tx.execute("UPDATE registry_instances SET configuration_revision=?3 WHERE instance_id=?1 AND adapter_id=?2",
                                params![upgrade.instance_id,upgrade.adapter_id,upgrade.configuration_revision]).map_err(db)?;
                            follow_instance(tx,&upgrade.instance_id,&upgrade.configuration_revision)?;
                        }
                        bump(tx,&row.binding.instance_id)?;
                        Ok(())
                    }
                }
                Err(reason) => {
                    match (reason, upgrade) {
                        (Some(reason), None) => invalidate(tx,&row,reason)?,
                        (Some(_), Some(upgrade)) => refuse_upgrade(tx,&row,upgrade)?,
                        (None, _) => {}
                    }
                    Ok(())
                }
            };
            release(tx,&dispatched.captured.id)?;
            // Commit positive invalidity before returning the semantic refusal.
            Ok(decision)
        })?
    }
}
