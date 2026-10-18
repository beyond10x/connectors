use super::*;
use connectors_sdk::Secret;

struct AcquisitionRow {
    connection: String,
    owner: String,
    fence: String,
    state: String,
    expires: u64,
    consumed: Option<u64>,
    generation: String,
    capture: String,
    candidate: Option<String>,
}

impl Registry {
    pub fn begin(&self, binding: &Binding, now: u64) -> Result<Acquisition> {
        binding.validate()?;
        self.transaction(now, true, |tx, authority, now| {
            let begun_at = register(tx, binding)?;
            admit_allocation(tx, binding, now)?;
            allocate(tx, authority, binding, begun_at, now)
        })
    }

    /// `begin` without the host's own cardinality check: what `begin` admitted
    /// before the rule, so a test can build an instance that holds two
    /// connections and reach the model's own refusal.
    #[cfg(test)]
    pub(crate) fn begin_unchecked(&self, binding: &Binding, now: u64) -> Result<Acquisition> {
        binding.validate()?;
        self.transaction(now, true, |tx, authority, now| {
            let begun_at = register(tx, binding)?;
            allocate(tx, authority, binding, begun_at, now)
        })
    }

    pub fn begin_repair(
        &self,
        binding: &Binding,
        reference: &str,
        expected_revision: &str,
        now: u64,
    ) -> Result<Acquisition> {
        binding.validate()?;
        self.transaction(now, true, |tx, authority, now| {
            let row = Self::connection(tx, reference)?;
            if !row.public
                || row.binding.instance_id != binding.instance_id
                || row.binding.adapter_id != binding.adapter_id
            {
                return Err(Failure::NotFound);
            }
            live(&row)?;
            if row.revision != expected_revision {
                return Err(Failure::Conflict);
            }
            // An operation-only or executable-only change leaves the saved
            // binding the instance's (D12): repair keeps it and replaces the
            // credential, which the configured executable validates.
            if !matches!(
                super::follow(&row.binding, binding),
                Follow::Same | Follow::Operations | Follow::Executable
            ) {
                // Repair keeps the binding, so it cannot move a connection
                // saved under another one. Name the step that can, as a read
                // does: revalidation, a new connection, or a new instance id.
                let usable = || {
                    Ok(row.material.is_some()
                        && matches!(
                            super::observation::readiness(tx, &row, now, true)?,
                            State::Ready | State::Pending
                        ))
                };
                return Err(super::revalidation::changed(&row, binding, usable)?);
            }
            create_acquisition(tx, authority, row.reference, row.fence, now)
        })
    }

    /// Durable one-use consumption precedes protected completion/validation.
    /// An uncertain commit returns no claim and grants no continuation authority.
    pub fn consume(&self, acquisition: Acquisition, now: u64) -> Result<Claim> {
        self.transaction(now, false, |tx, _, now| {
            let acquired = acquired(tx, &acquisition.id, &acquisition.owner)?;
            if acquired.state!="pending" { return Err(Failure::Conflict); }
            if now>=acquired.expires { return Err(Failure::Expired); }
            let row = Self::connection(tx, &acquired.connection)?;
            live(&row)?;
            if row.fence!=acquired.fence { return Err(Failure::Conflict); }
            tx.execute("UPDATE registry_acquisitions SET state='completing',consumed_at_ms=?2 WHERE acquisition_ref=?1", params![acquisition.id,timestamp(now)?]).map_err(db)?;
            Ok(())
        })?;
        Ok(Claim { acquisition })
    }

    /// Native validation is already complete on the Claim's immutable capture.
    /// Allocate the exact candidate tuple before any keyring write. Candidate
    /// evidence stays in this live handle; it is not a persisted publication permit.
    pub fn prepare(
        &self,
        claim: &Claim,
        baseline: ValidatedBaseline,
        byte_size: usize,
        now: u64,
    ) -> Result<PreparedCandidate> {
        if !(1..=65536).contains(&byte_size) {
            return Err(Failure::InvalidInput);
        }
        self.transaction(now, false, |tx, authority, now| {
            let acquired = completing(tx, &claim.acquisition.id, &claim.acquisition.owner, now)?;
            if acquired.candidate.is_some() { return Err(Failure::Conflict); }
            let row = Self::connection(tx, &acquired.connection)?;
            live(&row)?;
            if row.fence!=acquired.fence { return Err(Failure::Conflict); }
            baseline.validate(&row.binding, acquired.consumed.ok_or(Failure::MetadataUnavailable)?, now)?;
            if row.identity.as_ref().is_some_and(|old| old!=&baseline.identity) { return Err(Failure::IdentityMismatch); }
            let retained: i64 = tx.query_row("SELECT count(*) FROM registry_materials WHERE connection_ref=?1 AND deleted=0", [&row.reference], |r| r.get(0)).map_err(db)?;
            if retained>=8 { return Err(Failure::Capacity); }
            let version_id = new_id();
            tx.execute("INSERT INTO registry_generations VALUES (?1,?2,?3,?4)", params![acquired.generation,row.reference,acquired.capture,encode(&baseline.identity)?]).map_err(db)?;
            tx.execute("INSERT INTO registry_materials (version_id,connection_ref,acquisition_ref,generation_id,byte_size) VALUES (?1,?2,?3,?4,?5)", params![version_id,row.reference,claim.acquisition.id,acquired.generation,byte_size as i64]).map_err(db)?;
            tx.execute("UPDATE registry_acquisitions SET candidate_id=?2 WHERE acquisition_ref=?1", params![claim.acquisition.id,version_id]).map_err(db)?;
            let version = custody_version(authority, &row.scope_id, &version_id)?;
            Ok(PreparedCandidate { acquisition:claim.acquisition.id.clone(), connection:row.reference,
                owner:claim.acquisition.owner.clone(), version_id, generation:acquired.generation,
                binding:row.binding, baseline, version, byte_size })
        })
    }

    /// Called while the custody binding holds its physical writer lock. A stale
    /// paused writer cannot create a version after acknowledged retirement.
    pub(crate) fn write_allowed(&self, prepared: &PreparedCandidate, now: u64) -> Result<()> {
        self.transaction(now, false, |tx, authority, now| {
            let acquired = completing(tx, &prepared.acquisition, &prepared.owner, now)?;
            let row = Self::connection(tx, &prepared.connection)?;
            live(&row)?;
            if acquired.candidate.as_deref()!=Some(&prepared.version_id) || row.fence!=acquired.fence
                || row.binding!=prepared.binding || prepared.version!=custody_version(authority,&row.scope_id,&prepared.version_id)? {
                return Err(Failure::Conflict);
            }
            let (ack, retired, deleted): (bool,bool,bool) = tx.query_row("SELECT acknowledged,retirement_fence IS NOT NULL,deleted FROM registry_materials WHERE version_id=?1", [&prepared.version_id], |r| Ok((r.get(0)?,r.get(1)?,r.get(2)?))).map_err(db)?;
            if ack || retired || deleted { return Err(Failure::Conflict); }
            prepared.baseline.validate(&prepared.binding, acquired.consumed.ok_or(Failure::MetadataUnavailable)?, now)
        })
    }

    /// The receipt can be constructed only by a definitely successful custody
    /// write. Registering it changes no active connection or dispatch authority.
    pub fn acknowledge(
        &self,
        prepared: PreparedCandidate,
        receipt: custody::WrittenVersion,
        now: u64,
    ) -> Result<StoredCandidate> {
        if !receipt.matches(prepared.version) {
            return Err(Failure::Conflict);
        }
        let acknowledged_at = self.transaction(now, false, |tx, _, now| {
            let acquired = acquired(tx, &prepared.acquisition, &prepared.owner)?;
            if acquired.candidate.as_deref()!=Some(&prepared.version_id) { return Err(Failure::Conflict); }
            let (deleted, at): (bool,Option<u64>) = tx.query_row("SELECT deleted,acknowledged_at_ms FROM registry_materials WHERE version_id=?1 AND acquisition_ref=?2 AND generation_id=?3",
                params![prepared.version_id,prepared.acquisition,prepared.generation], |r| Ok((r.get(0)?,read_optional_time(r,1)?))).map_err(db)?;
            if deleted { return Err(Failure::Conflict); }
            if let Some(at) = at { return Ok(at); }
            tx.execute("UPDATE registry_materials SET acknowledged=1,acknowledged_at_ms=?2 WHERE version_id=?1", params![prepared.version_id,timestamp(now)?]).map_err(db)?;
            Ok(now)
        })?;
        Ok(StoredCandidate {
            prepared,
            acknowledged_at,
        })
    }

    /// This single metadata commit couples generation, custody, evidence, the
    /// private fence and the acquisition result. Repair preserves public revision.
    pub fn publish(&self, stored: StoredCandidate, now: u64) -> Result<String> {
        self.publish_with(stored, now, false, None)
    }

    /// `exclusive` is the read-only binding's rule that one item backs one
    /// connection, held inside the publishing commit: two connects that both
    /// passed the early check publish at most one connection of the profile,
    /// and the other is refused as `CustodyReadOnly`. A repair republishes its
    /// own connection and is not counted.
    pub(super) fn publish_with(
        &self,
        stored: StoredCandidate,
        now: u64,
        exclusive: bool,
        guard: Option<&super::reuse::ReuseGuard>,
    ) -> Result<String> {
        let prepared = stored.prepared;
        self.transaction(now, false, |tx, _, now| {
            if let Some(guard) = guard {
                super::reuse::guarded(tx, guard)?;
            }
            let acquired = completing(tx, &prepared.acquisition, &prepared.owner, now)?;
            let row = Self::connection(tx, &prepared.connection)?;
            live(&row)?;
            if acquired.candidate.as_deref()!=Some(&prepared.version_id) || acquired.generation!=prepared.generation
                || row.fence!=acquired.fence || row.binding!=prepared.binding { return Err(Failure::Conflict); }
            if exclusive {
                let held: i64 = tx.query_row("SELECT count(*) FROM registry_connections WHERE instance_id=?1 AND profile_key=?2 AND public=1 AND state!='revoked' AND connection_ref!=?3",
                    params![row.binding.instance_id,profile_key(&row.binding)?,row.reference], |r| r.get(0)).map_err(db)?;
                if held > 0 { return Err(Failure::CustodyReadOnly); }
            }
            if row.identity.as_ref().is_some_and(|old| old!=&prepared.baseline.identity) { return Err(Failure::IdentityMismatch); }
            let consumed = acquired.consumed.ok_or(Failure::MetadataUnavailable)?;
            prepared.baseline.validate(&row.binding, consumed, now)?;
            let current: bool = tx.query_row("SELECT acknowledged=1 AND deleted=0 AND retirement_fence IS NULL AND invalid_reason IS NULL FROM registry_materials WHERE version_id=?1 AND generation_id=?2 AND acquisition_ref=?3",
                params![prepared.version_id,prepared.generation,prepared.acquisition], |r| r.get(0)).map_err(db)?;
            if !current { return Err(Failure::Conflict); }
            let snapshot = snapshot(&prepared, consumed, stored.acknowledged_at)?;
            if !snapshot.current(now) { return Err(Failure::Expired); }
            if let Some(old) = &row.material { retire(tx, old, now)?; }
            tx.execute("UPDATE registry_connections SET public=1,identity=?2,active_generation=?3,active_material=?4,baseline=?5,publication_fence=?6 WHERE connection_ref=?1",
                params![row.reference,encode(&prepared.baseline.identity)?,prepared.generation,prepared.version_id,encode(&snapshot)?,new_id()]).map_err(db)?;
            tx.execute("UPDATE registry_acquisitions SET state='completed' WHERE acquisition_ref=?1", [&prepared.acquisition]).map_err(db)?;
            tx.execute("UPDATE registry_uses SET released=1 WHERE connection_ref=?1 AND dispatched=0", [&row.reference]).map_err(db)?;
            follow(tx, &row)?;
            bump(tx, &row.binding.instance_id)?;
            Ok(row.reference)
        })
    }

    /// Production composition of the separate physical and metadata boundaries.
    pub fn store_and_publish(
        &self,
        prepared: PreparedCandidate,
        material: &Secret,
        store: &custody::Store,
        clock: impl Fn() -> u64,
    ) -> Result<String> {
        self.store_and_publish_guarded(prepared, material, store, clock, None)
    }

    /// `store_and_publish` of a reused credential
    /// (story:invoke-definition-reuse-credential): the publishing commit also
    /// requires the source to be what was captured (`reuse::guarded`), so a
    /// source revoked, made Stale or republished while its copy was validated
    /// publishes nothing.
    pub fn store_and_publish_reuse(
        &self,
        prepared: PreparedCandidate,
        material: &Secret,
        store: &custody::Store,
        clock: impl Fn() -> u64,
        guard: &super::reuse::ReuseGuard,
    ) -> Result<String> {
        self.store_and_publish_guarded(prepared, material, store, clock, Some(guard))
    }

    fn store_and_publish_guarded(
        &self,
        prepared: PreparedCandidate,
        material: &Secret,
        store: &custody::Store,
        clock: impl Fn() -> u64,
        guard: Option<&super::reuse::ReuseGuard>,
    ) -> Result<String> {
        if material.0.len() != prepared.byte_size {
            return Err(Failure::InvalidInput);
        }
        let mut refusal = None;
        let receipt = store
            .write_new_guarded(prepared.version, material, || {
                self.write_allowed(&prepared, clock()).map_err(|error| {
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
        let stored = self.acknowledge(prepared, receipt, clock())?;
        self.publish_with(stored, clock(), false, guard)
    }

    /// The read-only binding's composition of the same boundaries: no write.
    /// `reading` is the item read once under every check, the bytes the
    /// provider validated; the binding pins them to the prepared version while
    /// the coordinator's guard holds, and that acknowledgement, not a write,
    /// is what publication consumes.
    #[cfg(target_os = "linux")]
    pub fn pin_and_publish(
        &self,
        prepared: PreparedCandidate,
        item: &custody::Item,
        reading: &connectors_custody::mounted::Reading,
        store: &custody::Store,
        clock: impl Fn() -> u64,
    ) -> Result<String> {
        if reading.material().len() != prepared.byte_size {
            return Err(Failure::InvalidInput);
        }
        let mut refusal = None;
        let receipt = store
            .pin_guarded(prepared.version, item, reading, || {
                self.write_allowed(&prepared, clock()).map_err(|error| {
                    refusal = Some(error);
                    custody::Failure::Denied
                })
            })
            .map_err(|error| {
                refusal.unwrap_or(match error {
                    custody::Failure::OutcomeUnknown => Failure::OutcomeUnknown,
                    custody::Failure::ReadOnly => Failure::CustodyReadOnly,
                    _ => Failure::CustodyUnavailable,
                })
            })?;
        let stored = self.acknowledge(prepared, receipt, clock())?;
        self.publish_with(stored, clock(), true, None)
    }

    /// The acquisition of the instance's connect in flight, beside which
    /// `begin` refuses another connect as `Conflict`: what `connections status
    /// --acquisition` follows until it ends. `None` once it published, failed
    /// or passed its deadline.
    pub fn in_flight(&self, instance: &str, now: u64) -> Result<Option<String>> {
        self.transaction_observation(now, |tx, _, now| in_flight(tx, instance, now))
    }

    /// The instance's published Live or Stale connections, at most two: a
    /// command that names no connection selects the one an instance holds
    /// (story:connect-refused-while-instance-holds-connection), and two say
    /// only that it must be named.
    pub fn held(&self, instance: &str, now: u64) -> Result<Vec<String>> {
        self.transaction_observation(now, |tx, _, _| {
            tx.prepare("SELECT connection_ref FROM registry_connections WHERE instance_id=?1 AND public=1 AND state!='revoked' ORDER BY connection_ref LIMIT 2")
                .map_err(db)?
                .query_map([instance], |r| r.get::<_, String>(0))
                .map_err(db)?
                .collect::<std::result::Result<Vec<_>, _>>()
                .map_err(db)
        })
    }

    pub fn fail(&self, claim: &Claim, now: u64) -> Result<()> {
        self.transaction(now, false, |tx, _, now| {
            let acquired = acquired(tx, &claim.acquisition.id, &claim.acquisition.owner)?;
            if acquired.state == "completed" {
                return Err(Failure::Conflict);
            }
            if matches!(acquired.state.as_str(), "failed" | "expired") {
                return Ok(());
            }
            let expired = now >= acquired.expires;
            tx.execute(
                "UPDATE registry_acquisitions SET state=?2,failure=?3 WHERE acquisition_ref=?1",
                params![
                    claim.acquisition.id,
                    if expired { "expired" } else { "failed" },
                    if expired { "expired" } else { "rejected" }
                ],
            )
            .map_err(db)?;
            if let Some(candidate) = acquired.candidate {
                retire(tx, &candidate, now)?;
            }
            Ok(())
        })
    }

    /// Passive observation does not sweep expiry. An admitted local owner calls
    /// this bounded sweep; retirement starts at this acknowledged decision.
    pub fn expire(&self, now: u64) -> Result<usize> {
        self.transaction(now, false, |tx, _, now| {
            let mut stmt = tx.prepare("SELECT acquisition_ref,candidate_id FROM registry_acquisitions WHERE state IN ('pending','completing') AND expires_at_ms<=?1 ORDER BY expires_at_ms LIMIT 1000").map_err(db)?;
            let rows = stmt.query_map([timestamp(now)?], |r| Ok((r.get::<_,String>(0)?,r.get::<_,Option<String>>(1)?))).map_err(db)?.collect::<std::result::Result<Vec<_>,_>>().map_err(db)?;
            for (id, candidate) in &rows {
                tx.execute("UPDATE registry_acquisitions SET state='expired',failure='expired' WHERE acquisition_ref=?1", [id]).map_err(db)?;
                if let Some(candidate) = candidate { retire(tx, candidate, now)?; }
            }
            Ok(rows.len())
        })
    }

    pub fn revoke(
        &self,
        instance: &str,
        adapter_id: &str,
        reference: &str,
        expected_revision: &str,
        now: u64,
    ) -> Result<String> {
        self.revoke_admitted(adapter_id, reference, expected_revision, now, |recorded| {
            recorded == instance
        })
    }

    /// Revoke through the adapter entry of `instance` a connection of that
    /// entry's adapter recorded under `instance`, or under an instance id no
    /// entry configures any longer (`configured`, every configured instance
    /// id): a changed provider authority or profile authentication moves the
    /// entry to a new instance id, and the connections left under the earlier
    /// one stay revocable. A connection of an instance another entry still
    /// configures is that entry's, and stays `NotFound` here.
    pub fn revoke_left_behind(
        &self,
        instance: &str,
        adapter_id: &str,
        configured: &BTreeSet<String>,
        reference: &str,
        expected_revision: &str,
        now: u64,
    ) -> Result<String> {
        self.revoke_admitted(adapter_id, reference, expected_revision, now, |recorded| {
            recorded == instance || !configured.contains(recorded)
        })
    }

    fn revoke_admitted(
        &self,
        adapter_id: &str,
        reference: &str,
        expected_revision: &str,
        now: u64,
        admits: impl Fn(&str) -> bool,
    ) -> Result<String> {
        self.transaction(now, false, |tx, _, now| {
            let row = Self::connection(tx, reference)?;
            if !row.public || !admits(&row.binding.instance_id) || row.binding.adapter_id!=adapter_id { return Err(Failure::NotFound); }
            let instance = row.binding.instance_id.as_str();
            if row.state=="revoked" { return Ok(row.revision); }
            if row.revision!=expected_revision { return Err(Failure::Conflict); }
            let revision = new_id();
            tx.execute("UPDATE registry_connections SET state='revoked',revoked_at_ms=?2,semantic_revision=?3,publication_fence=?4,active_generation=NULL,active_material=NULL,baseline=NULL WHERE connection_ref=?1",
                params![reference,timestamp(now)?,revision,new_id()]).map_err(db)?;
            tx.execute("UPDATE registry_acquisitions SET state='failed',failure='rejected' WHERE connection_ref=?1 AND state IN ('pending','completing')", [reference]).map_err(db)?;
            let versions = tx.prepare("SELECT version_id FROM registry_materials WHERE connection_ref=?1 AND retirement_fence IS NULL").map_err(db)?
                .query_map([reference], |r| r.get::<_,String>(0)).map_err(db)?.collect::<std::result::Result<Vec<_>,_>>().map_err(db)?;
            for version in versions { retire(tx, &version, now)?; }
            tx.execute("UPDATE registry_uses SET released=1 WHERE connection_ref=?1 AND dispatched=0", [reference]).map_err(db)?;
            let configured: String = tx.query_row("SELECT configuration_revision FROM registry_instances WHERE instance_id=?1", [instance], |r| r.get(0)).map_err(db)?;
            super::revalidation::follow_instance(tx, instance, &configured)?;
            bump(tx, instance)?;
            Ok(revision)
        })
    }
}

/// Whether a connection of the binding's instance, revoked ones included, was
/// admitted under another provider authority, or under another declaration of
/// the binding's profile. The scan is bounded by the per-instance capacity.
fn authentication_changed(tx: &Transaction<'_>, binding: &Binding) -> Result<bool> {
    let stored = tx
        .prepare("SELECT binding FROM registry_connections WHERE instance_id=?1")
        .map_err(db)?
        .query_map([&binding.instance_id], |r| r.get::<_, String>(0))
        .map_err(db)?
        .collect::<std::result::Result<Vec<_>, _>>()
        .map_err(db)?;
    for text in stored {
        let (admitted, _) = decode_stored(&text)?;
        if super::revalidation::identity_changed(&admitted, binding) {
            return Ok(true);
        }
    }
    Ok(false)
}

/// Registers the instance and profile; answers the instance's recorded
/// revision when the binding names another one (`Kept::begun_at`).
fn register(tx: &Transaction<'_>, binding: &Binding) -> Result<Option<String>> {
    let existing = tx
        .query_row(
            "SELECT adapter_id,configuration_revision FROM registry_instances WHERE instance_id=?1",
            [&binding.instance_id],
            |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)),
        )
        .optional()
        .map_err(db)?;
    let begun_at = match existing {
        Some((adapter, _)) if adapter != binding.adapter_id => {
            return Err(Failure::Conflict);
        }
        // A connection begun under another revision of its instance is admitted:
        // its instance moves there only when it is published (`follow`), as
        // after an upgrading revalidation. Refusing it here left an instance
        // whose connections never followed an upgrade (the new provider
        // refused the credential) admitting no connection at all.
        // The provider authority and the profile's authentication still bind
        // the instance id: a new revision that changes either for a profile the
        // instance already holds needs a new instance id. A declaration that
        // differs only in its evidence lifetime is the same authentication
        // (`StaticProfile::same_authentication`): that connection is admitted.
        Some((_, revision)) if revision != binding.configuration_revision => {
            if authentication_changed(tx, binding)? {
                return Err(Failure::AuthenticationChanged);
            }
            Some(revision)
        }
        Some(_) => None,
        None => {
            tx.execute("INSERT INTO registry_instances (instance_id,adapter_id,configuration_revision) VALUES (?1,?2,?3)", params![binding.instance_id,binding.adapter_id,binding.configuration_revision]).map_err(db)?;
            None
        }
    };
    let declaration = encode(&binding.profile)?;
    let existing: Option<String> = tx.query_row("SELECT declaration FROM registry_profiles WHERE adapter_id=?1 AND profile_ref=?2 AND revision=?3", params![binding.adapter_id,binding.profile.id,binding.profile.revision], |r| r.get(0)).optional().map_err(db)?;
    if let Some(existing) = existing {
        if existing != declaration {
            return Err(Failure::Conflict);
        }
    } else {
        tx.execute(
            "INSERT INTO registry_profiles VALUES (?1,?2,?3,?4,?5)",
            params![
                profile_key(binding)?,
                binding.adapter_id,
                binding.profile.id,
                binding.profile.revision,
                declaration
            ],
        )
        .map_err(db)?;
    }
    Ok(begun_at)
}
fn profile_key(binding: &Binding) -> Result<String> {
    Ok(connectors_core::digest(&serde_json::json!([
        binding.adapter_id,
        serde_json::to_value(&binding.profile).map_err(|_| Failure::MetadataUnavailable)?
    ])))
}
fn qualified(prefix: &str, authority: Uuid) -> String {
    format!(
        "{prefix}_{}_{}",
        authority.simple(),
        Uuid::new_v4().simple()
    )
}
/// An instance holds at most one published non-revoked connection
/// (contracts/auth/connection/v1alpha1/semantics.md §4.2): a connect is refused
/// while it holds a published Live or Stale connection (Stale is a live row
/// whose binding records the lifecycle), and while another connect of it is in
/// flight, so two concurrent connects cannot both publish. A connection never
/// published holds nothing once its connect failed or passed its deadline,
/// whether or not an expiry sweep recorded that yet. The refusal names the
/// step that clears it, as a repair of the held connection classifies it
/// (`revalidation::changed`): a reconnect while it is held under the
/// configured binding, which repair admits; a revalidation when it can follow
/// the configuration upgrade, after which it is the instance's connection; a
/// revocation when repair refuses it as changed, or when the instance holds
/// more than one.
fn admit_allocation(tx: &Transaction<'_>, binding: &Binding, now: u64) -> Result<()> {
    let held = tx
        .prepare("SELECT connection_ref FROM registry_connections WHERE instance_id=?1 AND public=1 AND state!='revoked' ORDER BY connection_ref LIMIT 2")
        .map_err(db)?
        .query_map([&binding.instance_id], |r| r.get::<_, String>(0))
        .map_err(db)?
        .collect::<std::result::Result<Vec<_>, _>>()
        .map_err(db)?;
    match held.as_slice() {
        [] => {}
        [reference] => {
            let row = Registry::connection(tx, reference)?;
            if matches!(
                super::follow(&row.binding, binding),
                Follow::Same | Follow::Operations | Follow::Executable
            ) {
                return Err(Failure::InstanceConnected);
            }
            let usable = || {
                Ok(row.material.is_some()
                    && matches!(
                        super::observation::readiness(tx, &row, now, true)?,
                        State::Ready | State::Pending
                    ))
            };
            return Err(match super::revalidation::changed(&row, binding, usable)? {
                Failure::BindingChanged => Failure::InstanceConnectionChanged,
                other => other,
            });
        }
        _ => return Err(Failure::InstanceConnectionChanged),
    }
    if in_flight(tx, &binding.instance_id, now)?.is_some() {
        return Err(Failure::Conflict);
    }
    Ok(())
}

/// The acquisition of a connect of the instance that allocated a connection
/// not yet published and has neither ended nor passed its deadline.
fn in_flight(tx: &Transaction<'_>, instance: &str, now: u64) -> Result<Option<String>> {
    tx.query_row(
        "SELECT a.acquisition_ref FROM registry_acquisitions a JOIN registry_connections c ON c.connection_ref=a.connection_ref WHERE c.instance_id=?1 AND c.public=0 AND c.state!='revoked' AND a.state IN ('pending','completing') AND a.expires_at_ms>?2 ORDER BY a.created_at_ms, a.acquisition_ref LIMIT 1",
        params![instance, timestamp(now)?],
        |r| r.get(0),
    )
    .optional()
    .map_err(db)
}

/// Allocates a private, unpublished connection of the binding and its first
/// acquisition (`connectors.auth_bindings.AllocateConnection`).
fn allocate(
    tx: &Transaction<'_>,
    authority: Uuid,
    binding: &Binding,
    begun_at: Option<String>,
    now: u64,
) -> Result<Acquisition> {
    let per_instance: i64 = tx
        .query_row(
            "SELECT count(*) FROM registry_connections WHERE instance_id=?1",
            [&binding.instance_id],
            |r| r.get(0),
        )
        .map_err(db)?;
    let total: i64 = tx
        .query_row("SELECT count(*) FROM registry_connections", [], |r| {
            r.get(0)
        })
        .map_err(db)?;
    if per_instance >= 1000 || total >= 10_000 {
        return Err(Failure::Capacity);
    }
    let connection = qualified("conn", authority);
    let fence = new_id();
    tx.execute("INSERT INTO registry_connections (connection_ref,instance_id,profile_key,binding,scope_id,semantic_revision,publication_fence,state,created_at_ms) VALUES (?1,?2,?3,?4,?5,?6,?7,'live',?8)",
        params![connection,binding.instance_id,profile_key(binding)?,encode_stored(binding, &Kept { begun_at, ..Kept::default() })?,new_id(),new_id(),fence,timestamp(now)?]).map_err(db)?;
    create_acquisition(tx, authority, connection, fence, now)
}

fn create_acquisition(
    tx: &Transaction<'_>,
    authority: Uuid,
    connection: String,
    fence: String,
    now: u64,
) -> Result<Acquisition> {
    let count: i64 = tx
        .query_row("SELECT count(*) FROM registry_acquisitions", [], |r| {
            r.get(0)
        })
        .map_err(db)?;
    if count >= 100_000 {
        return Err(Failure::Capacity);
    }
    let id = qualified("acq", authority);
    let owner = new_id();
    tx.execute("INSERT INTO registry_acquisitions (acquisition_ref,connection_ref,owner_token,expected_fence,state,created_at_ms,expires_at_ms,generation_id,capture_id) VALUES (?1,?2,?3,?4,'pending',?5,?6,?7,?8)",
        params![id,connection,owner,fence,timestamp(now)?,timestamp(deadline(now,ENTRY_MS)?)?,new_id(),new_id()]).map_err(db)?;
    Ok(Acquisition {
        id,
        connection,
        owner,
    })
}
fn acquired(tx: &Transaction<'_>, id: &str, owner: &str) -> Result<AcquisitionRow> {
    let row = tx.query_row("SELECT connection_ref,owner_token,expected_fence,state,expires_at_ms,consumed_at_ms,generation_id,capture_id,candidate_id FROM registry_acquisitions WHERE acquisition_ref=?1", [id], |r| Ok(AcquisitionRow {
        connection:r.get(0)?,owner:r.get(1)?,fence:r.get(2)?,state:r.get(3)?,expires:read_time(r,4)?,consumed:read_optional_time(r,5)?,generation:r.get(6)?,capture:r.get(7)?,candidate:r.get(8)?
    })).optional().map_err(db)?.ok_or(Failure::NotFound)?;
    if row.owner != owner {
        return Err(Failure::NotFound);
    }
    Ok(row)
}
fn completing(tx: &Transaction<'_>, id: &str, owner: &str, now: u64) -> Result<AcquisitionRow> {
    let row = acquired(tx, id, owner)?;
    if row.state != "completing" {
        return Err(Failure::Conflict);
    }
    if now >= row.expires {
        return Err(Failure::Expired);
    }
    Ok(row)
}
fn live(row: &ConnectionRow) -> Result<()> {
    if row.state == "revoked" {
        Err(Failure::Revoked)
    } else {
        Ok(())
    }
}
/// `connectors.auth_bindings.MarkConnectionStale`: the provider refused this
/// connection's active credential, so only a publication (a reconnect, or a
/// revalidation the provider accepts) returns it to Live (`follow`,
/// revalidation.rs `finish_revalidation`). The lifecycle moves only Live to
/// Stale: a revoked or already Stale connection is left as it is. The caller
/// advances the instance epoch, as for the invalidity it records beside it.
pub(super) fn mark_stale(tx: &Transaction<'_>, row: &ConnectionRow) -> Result<()> {
    if row.state == "revoked" || row.kept.stale {
        return Ok(());
    }
    let kept = Kept {
        stale: true,
        ..row.kept.clone()
    };
    tx.execute(
        "UPDATE registry_connections SET binding=?2 WHERE connection_ref=?1",
        params![row.reference, encode_stored(&row.binding, &kept)?],
    )
    .map_err(db)?;
    Ok(())
}
pub(super) fn bump(tx: &Transaction<'_>, instance: &str) -> Result<()> {
    tx.execute(
        "UPDATE registry_instances SET epoch=epoch+1 WHERE instance_id=?1",
        [instance],
    )
    .map_err(db)?;
    Ok(())
}
pub(super) fn retire(tx: &Transaction<'_>, version: &str, now: u64) -> Result<()> {
    tx.execute("UPDATE registry_materials SET retirement_fence=?2,retired_at_ms=?3,delete_not_before_ms=?4 WHERE version_id=?1 AND retirement_fence IS NULL",
        params![version,new_id(),timestamp(now)?,timestamp(deadline(now,RETENTION_MS)?)?]).map_err(db)?;
    Ok(())
}
pub(super) fn custody_version(
    authority: Uuid,
    scope: &str,
    version: &str,
) -> Result<custody::Version> {
    let scope =
        custody::Scope::new(authority, uuid(scope)?).map_err(|_| Failure::MetadataUnavailable)?;
    custody::Version::new(scope, uuid(version)?).map_err(|_| Failure::MetadataUnavailable)
}
fn snapshot(
    prepared: &PreparedCandidate,
    consumed: u64,
    acknowledged: u64,
) -> Result<EvidenceSnapshot> {
    recollected_snapshot(
        &prepared.binding,
        &prepared.connection,
        &prepared.generation,
        &prepared.baseline,
        consumed,
        acknowledged,
    )
}

pub(super) fn recollected_snapshot(
    binding: &Binding,
    connection: &str,
    generation: &str,
    native: &ValidatedBaseline,
    consumed: u64,
    acknowledged: u64,
) -> Result<EvidenceSnapshot> {
    let until = native
        .valid_until_ms
        .min(deadline(consumed, ENTRY_MS)?)
        .min(deadline(acknowledged, ENTRY_MS)?);
    let mut checks = vec![
        EvidenceCheck {
            check: "custody_reachable".into(),
            result: "ok".into(),
            source: "local_metadata".into(),
            collected_at: acknowledged,
            valid_until: until,
        },
        EvidenceCheck {
            check: "credential_present".into(),
            result: "ok".into(),
            source: "local_metadata".into(),
            collected_at: consumed,
            valid_until: until,
        },
    ];
    for kind in ["credential_valid", "identity_check"] {
        checks.push(EvidenceCheck {
            check: kind.into(),
            result: "ok".into(),
            source: "provider_read".into(),
            collected_at: native.collected_at_ms,
            valid_until: until,
        });
    }
    if native.granted_scopes.is_some() {
        checks.push(EvidenceCheck {
            check: "scope_check".into(),
            result: "ok".into(),
            source: "provider_read".into(),
            collected_at: native.collected_at_ms,
            valid_until: until,
        });
    }
    Ok(EvidenceSnapshot {
        generation_id: generation.to_owned(),
        binding: EvidenceBinding {
            instance_id: binding.instance_id.clone(),
            connection_ref: connection.to_owned(),
            profile_ref: binding.profile.id.clone(),
            provider_authority: binding.provider_authority.clone(),
        },
        observed_external_identity: native.identity.clone(),
        granted_scopes: native.granted_scopes.clone(),
        credential_expires_at: native.credential_expires_at_ms,
        checks,
    })
}

/// A publication clears what the binding column kept for the connection. A
/// refused upgrade was recorded for the credential this publication replaces
/// (a repair), and the begin-time revision is spent. A new connection begun
/// under another revision of its instance moves the instance to its own
/// revision, as an upgrading revalidation does, but only if the instance still
/// records the revision it had when the connection was begun: a connection
/// begun under an older configuration never moves the instance back, and a
/// repair never moves it. Older connections keep their revision and can
/// still follow on revalidation.
fn follow(tx: &Transaction<'_>, row: &ConnectionRow) -> Result<()> {
    if row.kept != Kept::default() {
        tx.execute(
            "UPDATE registry_connections SET binding=?2 WHERE connection_ref=?1",
            params![row.reference, encode(&row.binding)?],
        )
        .map_err(db)?;
    }
    let Some(begun_at) = &row.kept.begun_at else {
        return Ok(());
    };
    let binding = &row.binding;
    let revision: String = tx
        .query_row(
            "SELECT configuration_revision FROM registry_instances WHERE instance_id=?1",
            [&binding.instance_id],
            |r| r.get(0),
        )
        .map_err(db)?;
    if revision != *begun_at {
        return Ok(());
    }
    tx.execute(
        "UPDATE registry_instances SET configuration_revision=?2 WHERE instance_id=?1",
        params![binding.instance_id, binding.configuration_revision],
    )
    .map_err(db)?;
    super::revalidation::follow_instance(tx, &binding.instance_id, &binding.configuration_revision)
}
