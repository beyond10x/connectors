//! Static-entry connection authority. All records are nonsecret; provider work
//! and custody I/O happen outside SQLite transactions and handle lifetimes.
//! See docs/local-connection-registry.md for the binding and acknowledgement map.
mod lifecycle;
mod observation;
mod use_and_retirement;
pub use observation::{ObservedAcquisition, ObservedConnection, Page, PageOptions, State};
pub use use_and_retirement::{DispatchedUse, InvalidCredential, ReadUse, Retirement};
#[cfg(test)]
mod tests;

use super::{keyring::custody, metadata::Metadata};
use rusqlite::{OptionalExtension, Transaction, TransactionBehavior, params};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use std::{
    collections::BTreeSet,
    path::{Path, PathBuf},
};
use uuid::Uuid;

const ENTRY_MS: u64 = 300_000;
const RETENTION_MS: u64 = 86_400_000;
const USE_MS: u64 = 120_000;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Failure {
    MetadataUnavailable,
    OutcomeUnknown,
    NotFound,
    Conflict,
    Revoked,
    IdentityMismatch,
    Expired,
    InvalidInput,
    StaleCursor,
    Capacity,
    NotReady,
    InsufficientScope,
    CustodyUnavailable,
}
pub type Result<T> = std::result::Result<T, Failure>;

#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Purpose {
    DelegatedUser,
    ServiceAccount,
    AppLevel,
}
#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Subject {
    User,
    App,
}

/// Selected subset of an immutable reviewed AuthProfile declaration. This module
/// admits only credential-bearing static_entry flows, never OAuth or refresh.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StaticProfile {
    pub id: String,
    pub revision: String,
    pub purpose: Purpose,
    pub subject: Subject,
    pub minimum_scopes: BTreeSet<String>,
    pub evidence_lifetime_ms: u64,
}

/// Supplied by the trusted configuration/bootstrap owner, not caller assertions.
/// Native target normalization and auth mechanism selection stay with adapters.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Binding {
    pub instance_id: String,
    pub adapter_id: String,
    pub configuration_revision: String,
    pub provider_authority: String,
    pub profile: StaticProfile,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExternalIdentity {
    pub kind: String,
    pub subject: String,
}

/// A native validation result tied by the owner to the Claim's exact capture.
/// It never grants authority simply by being deserializable or caller supplied.
pub struct ValidatedBaseline {
    pub identity: ExternalIdentity,
    pub granted_scopes: Option<BTreeSet<String>>,
    pub credential_expires_at_ms: Option<u64>,
    pub collected_at_ms: u64,
    pub valid_until_ms: u64,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct EvidenceBinding {
    instance_id: String,
    connection_ref: String,
    profile_ref: String,
    provider_authority: String,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct EvidenceCheck {
    check: String,
    result: String,
    source: String,
    collected_at: u64,
    valid_until: u64,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct EvidenceSnapshot {
    generation_id: String,
    binding: EvidenceBinding,
    observed_external_identity: ExternalIdentity,
    granted_scopes: Option<BTreeSet<String>>,
    credential_expires_at: Option<u64>,
    checks: Vec<EvidenceCheck>,
}

/// Holds only a path; each operation closes its SQLite handle before returning.
pub struct Registry {
    path: PathBuf,
    system_clock: bool,
    #[cfg(test)]
    lose_next_commit: std::sync::atomic::AtomicBool,
}

pub struct Acquisition {
    id: String,
    connection: String,
    owner: String,
}
impl Acquisition {
    pub fn reference(&self) -> &str {
        &self.id
    }
    pub fn connection_reference(&self) -> &str {
        &self.connection
    }
}

pub struct Claim {
    acquisition: Acquisition,
}
impl Claim {
    pub fn acquisition_reference(&self) -> &str {
        &self.acquisition.id
    }
}

pub struct PreparedCandidate {
    acquisition: String,
    connection: String,
    owner: String,
    version_id: String,
    generation: String,
    binding: Binding,
    baseline: ValidatedBaseline,
    version: custody::Version,
    byte_size: usize,
}
impl PreparedCandidate {
    pub fn version(&self) -> custody::Version {
        self.version
    }
}
pub struct StoredCandidate {
    prepared: PreparedCandidate,
    acknowledged_at: u64,
}

struct ConnectionRow {
    reference: String,
    binding: Binding,
    profile_key: String,
    scope_id: String,
    revision: String,
    fence: String,
    state: String,
    public: bool,
    identity: Option<ExternalIdentity>,
    generation: Option<String>,
    material: Option<String>,
    baseline: Option<EvidenceSnapshot>,
}

impl Registry {
    pub fn new(path: &Path) -> Self {
        Self {
            path: path.to_owned(),
            system_clock: false,
            #[cfg(test)]
            lose_next_commit: std::sync::atomic::AtomicBool::new(false),
        }
    }
    /// Production binding: sample the physical clock after acquiring the
    /// transaction. A timestamp taken before lock contention is not evidence of
    /// clock regression. Absolute request/capture deadlines remain unchanged.
    pub fn with_system_clock(path: &Path) -> Self {
        Self {
            system_clock: true,
            ..Self::new(path)
        }
    }

    fn transaction<T>(
        &self,
        now: u64,
        migrate: bool,
        action: impl FnOnce(&Transaction<'_>, Uuid, u64) -> Result<T>,
    ) -> Result<T> {
        let mut metadata = Metadata::update(&self.path, migrate).map_err(host_failure)?;
        let authority = metadata.authority().map_err(host_failure)?;
        let tx = metadata
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(db)?;
        let now = if self.system_clock {
            connectors_sdk::now_ms()
        } else {
            now
        };
        let now_sql = timestamp(now)?;
        let previous: i64 = tx
            .query_row(
                "SELECT last_seen_ms FROM registry_clock WHERE singleton=1",
                [],
                |r| r.get(0),
            )
            .map_err(db)?;
        if now_sql < previous {
            return Err(Failure::MetadataUnavailable);
        }
        tx.execute(
            "UPDATE registry_clock SET last_seen_ms=?1 WHERE singleton=1",
            [now_sql],
        )
        .map_err(db)?;
        // Time observations persist even when the requested semantic action
        // refuses. A savepoint rolls back its work without forgetting the clock.
        tx.execute_batch("SAVEPOINT action").map_err(db)?;
        let result = action(&tx, authority, now);
        if result.is_err() {
            tx.execute_batch("ROLLBACK TO action").map_err(db)?;
        }
        tx.execute_batch("RELEASE action").map_err(db)?;
        tx.commit().map_err(|_| Failure::OutcomeUnknown)?;
        #[cfg(test)]
        if result.is_ok()
            && self
                .lose_next_commit
                .swap(false, std::sync::atomic::Ordering::SeqCst)
        {
            return Err(Failure::OutcomeUnknown);
        }
        result
    }

    fn connection(tx: &Transaction<'_>, reference: &str) -> Result<ConnectionRow> {
        if !connectors_core::valid_id(reference) {
            return Err(Failure::InvalidInput);
        }
        let raw = tx.query_row(
            "SELECT connection_ref,binding,profile_key,scope_id,semantic_revision,publication_fence,state,public,identity,active_generation,active_material,baseline FROM registry_connections WHERE connection_ref=?1",
            [reference], |r| Ok((r.get::<_,String>(0)?,r.get::<_,String>(1)?,r.get::<_,String>(2)?,r.get::<_,String>(3)?,r.get::<_,String>(4)?,r.get::<_,String>(5)?,r.get::<_,String>(6)?,r.get::<_,bool>(7)?,r.get::<_,Option<String>>(8)?,r.get::<_,Option<String>>(9)?,r.get::<_,Option<String>>(10)?,r.get::<_,Option<String>>(11)?)))
            .optional().map_err(db)?.ok_or(Failure::NotFound)?;
        let row = ConnectionRow {
            reference: raw.0,
            binding: decode(&raw.1)?,
            profile_key: raw.2,
            scope_id: raw.3,
            revision: raw.4,
            fence: raw.5,
            state: raw.6,
            public: raw.7,
            identity: raw.8.as_deref().map(decode).transpose()?,
            generation: raw.9,
            material: raw.10,
            baseline: raw.11.as_deref().map(decode).transpose()?,
        };
        row.binding
            .validate()
            .map_err(|_| Failure::MetadataUnavailable)?;
        let bound: bool = tx.query_row("SELECT EXISTS(SELECT 1 FROM registry_connections c JOIN registry_instances i ON c.instance_id=i.instance_id WHERE c.connection_ref=?1 AND i.instance_id=?2 AND i.adapter_id=?3 AND i.configuration_revision=?4)",
            params![row.reference,row.binding.instance_id,row.binding.adapter_id,row.binding.configuration_revision], |r| r.get(0)).map_err(db)?;
        if !bound {
            return Err(Failure::MetadataUnavailable);
        }
        let profile: String = tx.query_row("SELECT declaration FROM registry_profiles WHERE profile_key=?1 AND adapter_id=?2 AND profile_ref=?3 AND revision=?4",
            params![row.profile_key,row.binding.adapter_id,row.binding.profile.id,row.binding.profile.revision], |r| r.get(0)).map_err(db)?;
        if decode::<StaticProfile>(&profile)? != row.binding.profile
            || row.generation.is_some() != row.material.is_some()
            || row.generation.is_some() != row.baseline.is_some()
            || !matches!(row.state.as_str(), "live" | "revoked")
            || (row.state == "revoked" && row.material.is_some())
        {
            return Err(Failure::MetadataUnavailable);
        }
        uuid(&row.scope_id)?;
        uuid(&row.fence)?;
        if let Some(snapshot) = &row.baseline {
            snapshot.validate(&row)?;
        }
        Ok(row)
    }
}

impl Binding {
    fn validate(&self) -> Result<()> {
        for id in [
            &self.instance_id,
            &self.adapter_id,
            &self.configuration_revision,
            &self.profile.id,
            &self.profile.revision,
        ] {
            if !connectors_core::valid_id(id) {
                return Err(Failure::InvalidInput);
            }
        }
        if !bounded(&self.provider_authority, 4096)
            || !scopes_valid(&self.profile.minimum_scopes)
            || self.profile.evidence_lifetime_ms == 0
            || self.profile.evidence_lifetime_ms > ENTRY_MS
            || (self.profile.purpose == Purpose::DelegatedUser)
                != (self.profile.subject == Subject::User)
        {
            return Err(Failure::InvalidInput);
        }
        Ok(())
    }
}

impl ValidatedBaseline {
    fn validate(&self, binding: &Binding, consumed: u64, now: u64) -> Result<()> {
        if !bounded(&self.identity.kind, 256)
            || !bounded(&self.identity.subject, 4096)
            || self.collected_at_ms < consumed
            || self.collected_at_ms > now
            || self.valid_until_ms <= now
            || self.valid_until_ms
                > deadline(self.collected_at_ms, binding.profile.evidence_lifetime_ms)?
            || self
                .credential_expires_at_ms
                .is_some_and(|t| t <= now || self.valid_until_ms > t)
        {
            return Err(Failure::Expired);
        }
        if self
            .granted_scopes
            .as_ref()
            .is_some_and(|s| !scopes_valid(s))
        {
            return Err(Failure::InvalidInput);
        }
        if !binding.profile.minimum_scopes.is_empty()
            && !self
                .granted_scopes
                .as_ref()
                .is_some_and(|s| binding.profile.minimum_scopes.is_subset(s))
        {
            return Err(Failure::InsufficientScope);
        }
        Ok(())
    }
}

impl EvidenceSnapshot {
    fn validate(&self, row: &ConnectionRow) -> Result<()> {
        let expected = if self.granted_scopes.is_some() { 5 } else { 4 };
        let mut seen = BTreeSet::new();
        if self.generation_id
            != *row
                .generation
                .as_ref()
                .ok_or(Failure::MetadataUnavailable)?
            || self.binding.instance_id != row.binding.instance_id
            || self.binding.connection_ref != row.reference
            || self.binding.profile_ref != row.binding.profile.id
            || self.binding.provider_authority != row.binding.provider_authority
            || row.identity.as_ref() != Some(&self.observed_external_identity)
            || self.checks.len() != expected
            || self
                .granted_scopes
                .as_ref()
                .is_some_and(|s| !scopes_valid(s))
        {
            return Err(Failure::MetadataUnavailable);
        }
        for check in &self.checks {
            let source = match check.check.as_str() {
                "custody_reachable" | "credential_present" => "local_metadata",
                "credential_valid" | "identity_check" | "scope_check" => "provider_read",
                _ => return Err(Failure::MetadataUnavailable),
            };
            if !seen.insert(&check.check)
                || check.result != "ok"
                || check.source != source
                || check.collected_at == 0
                || check.valid_until <= check.collected_at
                || check.valid_until.saturating_sub(check.collected_at) > ENTRY_MS
                || (source == "provider_read"
                    && check.valid_until.saturating_sub(check.collected_at)
                        > row.binding.profile.evidence_lifetime_ms)
                || self
                    .credential_expires_at
                    .is_some_and(|t| check.valid_until > t)
            {
                return Err(Failure::MetadataUnavailable);
            }
        }
        if ![
            "custody_reachable",
            "credential_present",
            "credential_valid",
            "identity_check",
        ]
        .iter()
        .all(|n| seen.contains(&n.to_string()))
            || (self.granted_scopes.is_some() != seen.contains(&"scope_check".to_string()))
        {
            return Err(Failure::MetadataUnavailable);
        }
        Ok(())
    }
    fn valid_until(&self) -> u64 {
        self.checks.iter().map(|c| c.valid_until).min().unwrap_or(0)
    }
    fn current(&self, now: u64) -> bool {
        self.checks
            .iter()
            .all(|c| c.collected_at <= now && now < c.valid_until)
            && self.credential_expires_at.is_none_or(|e| now < e)
    }
}

fn bounded(s: &str, limit: usize) -> bool {
    !s.is_empty() && s.len() <= limit && !s.chars().any(char::is_control)
}
fn scopes_valid(scopes: &BTreeSet<String>) -> bool {
    scopes.len() <= 64 && scopes.iter().all(|s| bounded(s, 256))
}
fn timestamp(now: u64) -> Result<i64> {
    if now == 0 {
        return Err(Failure::MetadataUnavailable);
    }
    now.try_into().map_err(|_| Failure::MetadataUnavailable)
}
fn read_time(row: &rusqlite::Row<'_>, column: usize) -> rusqlite::Result<u64> {
    let value: i64 = row.get(column)?;
    if value <= 0 {
        return Err(rusqlite::Error::IntegralValueOutOfRange(column, value));
    }
    Ok(value as u64)
}
fn read_optional_time(row: &rusqlite::Row<'_>, column: usize) -> rusqlite::Result<Option<u64>> {
    match row.get::<_, Option<i64>>(column)? {
        None => Ok(None),
        Some(_) => read_time(row, column).map(Some),
    }
}
fn deadline(now: u64, span: u64) -> Result<u64> {
    now.checked_add(span)
        .filter(|t| *t <= i64::MAX as u64)
        .ok_or(Failure::MetadataUnavailable)
}
fn uuid(value: &str) -> Result<Uuid> {
    Uuid::parse_str(value)
        .ok()
        .filter(|v| !v.is_nil())
        .ok_or(Failure::MetadataUnavailable)
}
fn new_id() -> String {
    Uuid::new_v4().to_string()
}
fn encode(value: &impl Serialize) -> Result<String> {
    let text = serde_json::to_string(value).map_err(|_| Failure::MetadataUnavailable)?;
    if text.len() > 32768 {
        return Err(Failure::Capacity);
    }
    Ok(text)
}
fn decode<T: DeserializeOwned>(text: &str) -> Result<T> {
    if text.len() > 32768 {
        return Err(Failure::MetadataUnavailable);
    }
    serde_json::from_str(text).map_err(|_| Failure::MetadataUnavailable)
}
fn db(_: rusqlite::Error) -> Failure {
    Failure::MetadataUnavailable
}
fn host_failure(error: super::Failure) -> Failure {
    if error == super::Failure::OutcomeUnknown {
        Failure::OutcomeUnknown
    } else {
        Failure::MetadataUnavailable
    }
}
