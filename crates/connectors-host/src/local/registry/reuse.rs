//! story:invoke-definition-reuse-credential: the stored connections a new
//! connection of another instance may copy its credential from, and the one
//! captured version a copy reads (architecture-decision-record:
//! provider-definitions-and-runtimes D12 point 3).
//!
//! A candidate is a published, Live, not Stale connection of `instance` whose
//! credential is held, whose validation evidence is current, that is current
//! under its own configured entry and whose upgrade that entry has not
//! refused, bound to the target's adapter, provider authority and profile
//! authentication, and not of the target's own instance. A revoked, Stale,
//! expired or moved connection is never one. Offering a candidate records
//! nothing. A capture holds the candidate's current version as a read use of
//! the source, which `dispatch_read` and `release_read` (or `cancel_read`)
//! end like any other: the source connection's revision, state, identity and
//! evidence are not changed by it.
use super::lifecycle::custody_version;
use super::observation::{readiness, visible};
use super::*;

/// A connection another instance holds whose stored credential a new
/// connection of the target may be made from. Names no material.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReuseCandidate {
    pub reference: String,
    pub instance: String,
    pub revision: String,
    pub provider_authority: String,
    pub identity: ExternalIdentity,
}

/// One captured version of a candidate, held as a read use of the source
/// until it is dispatched and released, or cancelled.
pub struct ReuseSource {
    read: ReadUse,
    identity: ExternalIdentity,
    guard: ReuseGuard,
}

/// What a copy's publication requires of its source, checked inside the
/// publishing commit (`Registry::store_and_publish_reuse`): still published,
/// Live and not Stale, under the publication fence, generation and material
/// that were captured. A source revoked, marked Stale, reconnected or
/// revalidated while its copy was validated publishes no copy.
#[derive(Clone, Debug)]
pub struct ReuseGuard {
    reference: String,
    fence: String,
    generation: Option<String>,
    material: String,
}
impl ReuseSource {
    pub fn version(&self) -> custody::Version {
        self.read.version()
    }
    /// The source's validated identity, which the copy's validation must answer.
    pub fn identity(&self) -> &ExternalIdentity {
        &self.identity
    }
    /// What the copy's publication requires of the source.
    pub fn guard(&self) -> ReuseGuard {
        self.guard.clone()
    }
    /// The read use this capture holds: dispatched once, then released, or
    /// cancelled.
    pub fn into_read(self) -> ReadUse {
        self.read
    }
    #[cfg(test)]
    pub(super) fn read_replay(&self) -> ReadUse {
        self.read.fixture_replay()
    }
}

/// Whether `row`, visible under `instance` and `adapter`, is a candidate for
/// `target` under its own `configured` entry at `now`.
fn reusable(
    tx: &Transaction<'_>,
    row: &ConnectionRow,
    instance: &str,
    adapter: &str,
    configured: &Configured<'_>,
    target: &Binding,
    now: u64,
) -> Result<bool> {
    if visible(row, instance, adapter).is_err()
        || row.binding.instance_id == target.instance_id
        || row.binding.adapter_id != target.adapter_id
        || row.state != "live"
        || row.kept.stale
        || row.material.is_none()
        || row.identity.is_none()
        || row.kept.refused.as_deref() == Some(configured.configuration_revision)
        || !configured.current(&row.binding)
        || row.binding.provider_authority != target.provider_authority
        || !row.binding.profile.same_authentication(&target.profile)
    {
        return Ok(false);
    }
    Ok(readiness(tx, row, now, true)? == State::Ready)
}

impl Registry {
    /// The candidates `instance` (configured as `configured` under `adapter`)
    /// holds for a new connection bound to `target`, at most 64, in reference
    /// order. Records nothing but the clock observation every observation
    /// records, and grants nothing: the owner's capture decides again.
    pub fn reuse_candidates<'a>(
        &self,
        instance: &str,
        adapter: &str,
        configured: impl Into<Configured<'a>>,
        target: &Binding,
        now: u64,
    ) -> Result<Vec<ReuseCandidate>> {
        let configured = configured.into();
        target.validate()?;
        self.transaction_observation(now, |tx, _, now| {
            let references = tx
                .prepare("SELECT connection_ref FROM registry_connections WHERE instance_id=?1 AND public=1 AND state!='revoked' ORDER BY connection_ref LIMIT 64")
                .map_err(db)?
                .query_map([instance], |r| r.get::<_, String>(0))
                .map_err(db)?
                .collect::<std::result::Result<Vec<_>, _>>()
                .map_err(db)?;
            let mut candidates = Vec::new();
            for reference in references {
                let row = Self::connection(tx, &reference)?;
                if !reusable(tx, &row, instance, adapter, &configured, target, now)? {
                    continue;
                }
                candidates.push(ReuseCandidate {
                    identity: row.identity.clone().ok_or(Failure::MetadataUnavailable)?,
                    reference: row.reference,
                    instance: row.binding.instance_id,
                    revision: row.revision,
                    provider_authority: row.binding.provider_authority,
                });
            }
            Ok(candidates)
        })
    }

    /// Capture the current version of the candidate `reference`, offered
    /// under `revision`, for a copy bound to `target`: a read use of the
    /// source until `expires_at` at most, narrowed to its evidence. A
    /// connection of another instance or adapter, or one that is no candidate
    /// (another authority or profile, Stale, expired, moved), is `NotFound`;
    /// a revoked one `Revoked`; another revision `Conflict`.
    #[allow(clippy::too_many_arguments)]
    pub fn capture_reuse<'a>(
        &self,
        instance: &str,
        adapter: &str,
        configured: impl Into<Configured<'a>>,
        target: &Binding,
        reference: &str,
        revision: &str,
        now: u64,
        expires_at: u64,
    ) -> Result<ReuseSource> {
        let configured = configured.into();
        target.validate()?;
        if expires_at <= now || expires_at > deadline(now, USE_MS)? {
            return Err(Failure::InvalidInput);
        }
        self.transaction_admission(now, false, |tx, authority, now| {
            if expires_at <= now {
                return Err(Failure::Expired);
            }
            tx.execute("DELETE FROM registry_uses WHERE expires_at_ms<=?1", [timestamp(now)?])
                .map_err(db)?;
            let row = Self::connection(tx, reference)?;
            visible(&row, instance, adapter)?;
            if row.state == "revoked" {
                return Err(Failure::Revoked);
            }
            if row.revision != revision {
                return Err(Failure::Conflict);
            }
            if !reusable(tx, &row, instance, adapter, &configured, target, now)? {
                return Err(Failure::NotFound);
            }
            let baseline = row.baseline.as_ref().ok_or(Failure::MetadataUnavailable)?;
            let count: i64 = tx
                .query_row("SELECT count(*) FROM registry_uses", [], |r| r.get(0))
                .map_err(db)?;
            if count >= 1000 {
                return Err(Failure::Capacity);
            }
            let version_id = row.material.as_ref().ok_or(Failure::MetadataUnavailable)?;
            let id = new_id();
            let expires = expires_at.min(baseline.valid_until());
            tx.execute("INSERT INTO registry_uses (use_id,connection_ref,generation_id,version_id,publication_fence,expires_at_ms) VALUES (?1,?2,?3,?4,?5,?6)",
                params![id,reference,row.generation,version_id,row.fence,timestamp(expires)?]).map_err(db)?;
            let partition = connectors_core::digest(&serde_json::json!([
                authority.to_string(),
                reference,
                row.generation
            ]));
            Ok(ReuseSource {
                read: ReadUse {
                    id,
                    version: custody_version(authority, &row.scope_id, version_id)?,
                    partition,
                    expires,
                },
                identity: row.identity.clone().ok_or(Failure::MetadataUnavailable)?,
                guard: ReuseGuard {
                    reference: row.reference.clone(),
                    fence: row.fence.clone(),
                    generation: row.generation.clone(),
                    material: version_id.clone(),
                },
            })
        })
    }

    /// Dispatch the captured read use once, as `dispatch_read` does; a
    /// refused dispatch cancels the use, so nothing of it is left to lapse.
    pub fn dispatch_reuse(&self, captured: ReuseSource, now: u64) -> Result<DispatchedUse> {
        let replay = ReadUse {
            id: captured.read.id.clone(),
            version: captured.read.version,
            partition: captured.read.partition.clone(),
            expires: captured.read.expires,
        };
        self.dispatch_read(captured.read, now).or_else(|refused| {
            self.cancel_read(replay, now)?;
            Err(refused)
        })
    }
}

/// Inside a copy's publishing commit: the source is still what was
/// captured. Revoked is `Revoked`; anything else that moved, `Conflict`.
pub(super) fn guarded(tx: &Transaction<'_>, guard: &ReuseGuard) -> Result<()> {
    let row = Registry::connection(tx, &guard.reference)?;
    if row.state == "revoked" {
        return Err(Failure::Revoked);
    }
    if !row.public
        || row.kept.stale
        || row.fence != guard.fence
        || row.generation != guard.generation
        || row.material.as_deref() != Some(guard.material.as_str())
    {
        return Err(Failure::Conflict);
    }
    Ok(())
}
