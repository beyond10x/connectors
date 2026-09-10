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
            register(tx, binding)?;
            let per_instance: i64 = tx.query_row("SELECT count(*) FROM registry_connections WHERE instance_id=?1", [&binding.instance_id], |r| r.get(0)).map_err(db)?;
            let total: i64 = tx.query_row("SELECT count(*) FROM registry_connections", [], |r| r.get(0)).map_err(db)?;
            if per_instance>=1000 || total>=10_000 { return Err(Failure::Capacity); }
            let connection = qualified("conn", authority);
            let fence = new_id();
            tx.execute("INSERT INTO registry_connections (connection_ref,instance_id,profile_key,binding,scope_id,semantic_revision,publication_fence,state,created_at_ms) VALUES (?1,?2,?3,?4,?5,?6,?7,'live',?8)",
                params![connection,binding.instance_id,profile_key(binding)?,encode(binding)?,new_id(),new_id(),fence,timestamp(now)?]).map_err(db)?;
            create_acquisition(tx, authority, connection, fence, now)
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
            if row.binding != *binding {
                return Err(Failure::IdentityMismatch);
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
        let prepared = stored.prepared;
        self.transaction(now, false, |tx, _, now| {
            let acquired = completing(tx, &prepared.acquisition, &prepared.owner, now)?;
            let row = Self::connection(tx, &prepared.connection)?;
            live(&row)?;
            if acquired.candidate.as_deref()!=Some(&prepared.version_id) || acquired.generation!=prepared.generation
                || row.fence!=acquired.fence || row.binding!=prepared.binding { return Err(Failure::Conflict); }
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
        self.publish(stored, clock())
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
        self.transaction(now, false, |tx, _, now| {
            let row = Self::connection(tx, reference)?;
            if !row.public || row.binding.instance_id!=instance || row.binding.adapter_id!=adapter_id { return Err(Failure::NotFound); }
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
            bump(tx, instance)?;
            Ok(revision)
        })
    }
}

fn register(tx: &Transaction<'_>, binding: &Binding) -> Result<()> {
    let existing = tx
        .query_row(
            "SELECT adapter_id,configuration_revision FROM registry_instances WHERE instance_id=?1",
            [&binding.instance_id],
            |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)),
        )
        .optional()
        .map_err(db)?;
    match existing {
        Some((adapter, revision))
            if adapter != binding.adapter_id || revision != binding.configuration_revision =>
        {
            return Err(Failure::Conflict);
        }
        Some(_) => {}
        None => {
            tx.execute("INSERT INTO registry_instances (instance_id,adapter_id,configuration_revision) VALUES (?1,?2,?3)", params![binding.instance_id,binding.adapter_id,binding.configuration_revision]).map_err(db)?;
        }
    }
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
    Ok(())
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
    let native = &prepared.baseline;
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
        generation_id: prepared.generation.clone(),
        binding: EvidenceBinding {
            instance_id: prepared.binding.instance_id.clone(),
            connection_ref: prepared.connection.clone(),
            profile_ref: prepared.binding.profile.id.clone(),
            provider_authority: prepared.binding.provider_authority.clone(),
        },
        observed_external_identity: native.identity.clone(),
        granted_scopes: native.granted_scopes.clone(),
        credential_expires_at: native.credential_expires_at_ms,
        checks,
    })
}
