//! Local approval signing-key coordinator. A key lease supplies key policy only;
//! callers must independently authorize the subject and qualify the clock.
use super::{approvals, filesystem as fs, keyring::custody, metadata::Metadata};
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use connectors_sdk::Secret;
use ring::{
    rand::{SecureRandom, SystemRandom},
    signature::{Ed25519KeyPair, KeyPair},
};
use rusqlite::{OptionalExtension, params};
use serde::Serialize;
use std::{
    collections::BTreeMap,
    ffi::OsStr,
    fs::File,
    os::{fd::AsRawFd, unix::fs::MetadataExt},
    path::{Path, PathBuf},
    time::{Duration, Instant},
};
use uuid::Uuid;

const LOCK: &str = "approval-issuer.lock";
const CAPACITY: usize = 128;
const MAX_TIME: i64 = 9_007_199_254_740_991;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Failure {
    InvalidInput,
    Conflict,
    NotFound,
    MetadataUnavailable,
    CustodyUnavailable,
    OutcomeUnknown,
    Capacity,
}
pub type Result<T> = std::result::Result<T, Failure>;
impl From<super::Failure> for Failure {
    fn from(value: super::Failure) -> Self {
        if value == super::Failure::OutcomeUnknown {
            Self::OutcomeUnknown
        } else {
            Self::MetadataUnavailable
        }
    }
}
fn db(_: rusqlite::Error) -> Failure {
    Failure::MetadataUnavailable
}
fn secret_error(value: custody::Failure) -> Failure {
    match value {
        custody::Failure::OutcomeUnknown => Failure::OutcomeUnknown,
        custody::Failure::Conflict | custody::Failure::Denied => Failure::Conflict,
        _ => Failure::CustodyUnavailable,
    }
}
fn custody_guard(
    result: Result<()>,
    failure: &mut Option<Failure>,
) -> std::result::Result<(), custody::Failure> {
    result.map_err(|error| {
        *failure = Some(error);
        custody::Failure::Denied
    })
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum State {
    Candidate,
    Active,
    Retired,
    Retiring,
    Deleted,
}
impl State {
    fn parse(value: &str) -> Result<Self> {
        match value {
            "candidate" => Ok(Self::Candidate),
            "active" => Ok(Self::Active),
            "retired" => Ok(Self::Retired),
            "retiring" => Ok(Self::Retiring),
            "deleted" => Ok(Self::Deleted),
            _ => Err(Failure::MetadataUnavailable),
        }
    }
}
#[derive(Clone, Debug, Serialize)]
pub struct KeyView {
    pub key_id: Uuid,
    pub public_key: String,
    pub state: State,
}
#[derive(Clone, Debug, Serialize)]
pub struct View {
    pub issuer: String,
    pub audience: String,
    pub revision: Uuid,
    pub keys: Vec<KeyView>,
}
impl View {
    fn key(&self, key: Uuid) -> Result<&KeyView> {
        self.keys
            .iter()
            .find(|v| v.key_id == key)
            .ok_or(Failure::NotFound)
    }
    fn active(&self) -> Option<&KeyView> {
        self.keys.iter().find(|v| v.state == State::Active)
    }
    fn candidate(&self) -> Option<&KeyView> {
        self.keys.iter().find(|v| v.state == State::Candidate)
    }
}
struct Record {
    view: View,
    issuer_id: Uuid,
    scope: custody::Scope,
    versions: BTreeMap<Uuid, custody::Version>,
}
impl Record {
    fn version(&self, key: Uuid) -> Result<custody::Version> {
        self.versions.get(&key).copied().ok_or(Failure::NotFound)
    }
    fn expect(&self, revision: Uuid) -> Result<()> {
        if self.view.revision == revision {
            Ok(())
        } else {
            Err(Failure::Conflict)
        }
    }
}
/// Construct only from admitted owner configuration. An adapter receives neither
/// this store nor the local metadata path or Secret Service scope.
pub struct Store {
    path: PathBuf,
    socket: Option<PathBuf>,
    instance: String,
    adapter: String,
    configuration: String,
}
impl Store {
    pub fn new(
        path: &Path,
        socket: Option<&Path>,
        instance: &str,
        adapter: &str,
        configuration: &str,
    ) -> Result<Self> {
        for value in [instance, adapter, configuration] {
            if value.is_empty() || value.len() > 256 || value.chars().any(char::is_control) {
                return Err(Failure::InvalidInput);
            }
        }
        Ok(Self {
            path: path.to_owned(),
            socket: socket.map(Path::to_owned),
            instance: instance.into(),
            adapter: adapter.into(),
            configuration: configuration.into(),
        })
    }
    /// Passive: no key lease creation, migration, secret access or service startup.
    pub fn status(&self) -> Result<Option<View>> {
        let metadata = Metadata::inspect(&self.path)?;
        self.check_binding(&metadata.connection, false)?;
        let version: i64 = metadata
            .connection
            .pragma_query_value(None, "user_version", |r| r.get(0))
            .map_err(db)?;
        if version < 7 {
            return Ok(None);
        }
        Ok(self.load(&metadata)?.map(|r| r.view))
    }
    pub fn init(&self, expected_revision: Option<Uuid>) -> Result<View> {
        self.create(expected_revision, None)
    }
    pub fn rotate(&self, expected_revision: Uuid, expected_key: Uuid) -> Result<View> {
        self.create(Some(expected_revision), Some(expected_key))
    }
    fn create(&self, revision: Option<Uuid>, previous: Option<Uuid>) -> Result<View> {
        let _lease = Lease::acquire(&self.path, true)?;
        let mut seed = Secret(vec![0; 32]);
        SystemRandom::new()
            .fill(&mut seed.0)
            .map_err(|_| Failure::CustodyUnavailable)?;
        let public = public_key(&seed)?;
        let (record, key) = self.stage(revision, previous, &public)?;
        let version = record.version(key)?;
        let custody =
            custody::Store::open_at(record.scope, self.socket.as_deref()).map_err(secret_error)?;
        let mut guard_failure = None;
        let receipt = custody
            .write_new_guarded(version, &seed, || {
                custody_guard(
                    self.candidate_guard(record.view.revision, key, version),
                    &mut guard_failure,
                )
            })
            .map_err(|error| guard_failure.unwrap_or_else(|| secret_error(error)))?;
        self.publish(record.view.revision, key, &receipt)
    }
    fn stage(
        &self,
        revision: Option<Uuid>,
        previous: Option<Uuid>,
        public: &str,
    ) -> Result<(Record, Uuid)> {
        let mut metadata = Metadata::update_approval_keys(&self.path)?;
        self.check_binding(&metadata.connection, false)?;
        let old = self.load(&metadata)?;
        match &old {
            None if revision.is_none() && previous.is_none() => {}
            Some(old)
                if Some(old.view.revision) == revision
                    && old.view.active().map(|k| k.key_id) == previous
                    && old.view.candidate().is_none() => {}
            _ => return Err(Failure::Conflict),
        }
        if old.as_ref().is_some_and(|r| r.view.keys.len() >= CAPACITY) {
            return Err(Failure::Capacity);
        }
        let issuer = old
            .as_ref()
            .map(|r| r.issuer_id)
            .unwrap_or_else(Uuid::new_v4);
        let key = Uuid::new_v4();
        let tx = metadata.connection.transaction().map_err(db)?;
        self.check_binding(&tx, true)?;
        if old.is_none() {
            tx.execute(
                "INSERT INTO local_approval_issuers VALUES (?1,?2,?3,?4)",
                params![
                    issuer.to_string(),
                    self.instance,
                    Uuid::new_v4().to_string(),
                    Uuid::new_v4().to_string()
                ],
            )
            .map_err(db)?;
        }
        tx.execute(
            "INSERT INTO local_approval_keys VALUES (?1,?2,?3,?4,'candidate')",
            params![
                key.to_string(),
                issuer.to_string(),
                public,
                Uuid::new_v4().to_string()
            ],
        )
        .map_err(db)?;
        bump(&tx, issuer)?;
        tx.commit().map_err(|_| Failure::OutcomeUnknown)?;
        let record = self
            .load(&metadata)
            .map_err(|_| Failure::OutcomeUnknown)?
            .ok_or(Failure::OutcomeUnknown)?;
        Ok((record, key))
    }
    pub fn recover(&self, revision: Uuid, candidate: Uuid) -> Result<View> {
        let _lease = Lease::acquire(&self.path, true)?;
        let record = self.current()?;
        record.expect(revision)?;
        if record.view.key(candidate)?.state != State::Candidate {
            return Err(Failure::Conflict);
        }
        let version = record.version(candidate)?;
        let expected = &record.view.key(candidate)?.public_key;
        let custody =
            custody::Store::open_at(record.scope, self.socket.as_deref()).map_err(secret_error)?;
        let mut guard_failure = None;
        let receipt = custody
            .confirm_signing_key_guarded(
                version,
                || {
                    custody_guard(
                        self.candidate_guard(revision, candidate, version),
                        &mut guard_failure,
                    )
                },
                |seed| {
                    if public_key(seed).ok().as_ref() == Some(expected) {
                        Ok(())
                    } else {
                        Err(custody::Failure::InvalidMaterial)
                    }
                },
            )
            .map_err(|error| guard_failure.unwrap_or_else(|| secret_error(error)))?;
        self.publish(revision, candidate, &receipt)
    }
    fn candidate_guard(&self, revision: Uuid, key: Uuid, version: custody::Version) -> Result<()> {
        let record = self.current()?;
        record.expect(revision)?;
        if record.view.key(key)?.state != State::Candidate || record.version(key)? != version {
            return Err(Failure::Conflict);
        }
        Ok(())
    }
    fn publish(
        &self,
        revision: Uuid,
        key: Uuid,
        receipt: &custody::WrittenVersion,
    ) -> Result<View> {
        let mut metadata = Metadata::update_approval_keys(&self.path)?;
        let record = self.load(&metadata)?.ok_or(Failure::NotFound)?;
        record.expect(revision)?;
        if record.view.key(key)?.state != State::Candidate || !receipt.matches(record.version(key)?)
        {
            return Err(Failure::Conflict);
        }
        let tx = metadata.connection.transaction().map_err(db)?;
        tx.execute(
            "UPDATE local_approval_keys SET state='retired' WHERE issuer_id=?1 AND state='active'",
            [record.issuer_id.to_string()],
        )
        .map_err(db)?;
        tx.execute(
            "UPDATE local_approval_keys SET state='active' WHERE key_id=?1 AND state='candidate'",
            [key.to_string()],
        )
        .map_err(db)?;
        bump(&tx, record.issuer_id)?;
        tx.commit().map_err(|_| Failure::OutcomeUnknown)?;
        Ok(self
            .load(&metadata)
            .map_err(|_| Failure::OutcomeUnknown)?
            .ok_or(Failure::OutcomeUnknown)?
            .view)
    }
    pub fn revoke(&self, revision: Uuid, key: Uuid) -> Result<View> {
        let _lease = Lease::acquire(&self.path, true)?;
        let mut metadata = Metadata::update_approval_keys(&self.path)?;
        let record = self.load(&metadata)?.ok_or(Failure::NotFound)?;
        record.expect(revision)?;
        if record.view.active().map(|v| v.key_id) != Some(key) {
            return Err(Failure::Conflict);
        }
        let tx = metadata.connection.transaction().map_err(db)?;
        tx.execute("UPDATE local_approval_keys SET state='retired' WHERE issuer_id=?1 AND state IN ('active','candidate')",[record.issuer_id.to_string()]).map_err(db)?;
        bump(&tx, record.issuer_id)?;
        tx.commit().map_err(|_| Failure::OutcomeUnknown)?;
        Ok(self
            .load(&metadata)
            .map_err(|_| Failure::OutcomeUnknown)?
            .ok_or(Failure::OutcomeUnknown)?
            .view)
    }
    pub fn retire(&self, revision: Uuid, key: Uuid) -> Result<View> {
        let _lease = Lease::acquire(&self.path, true)?;
        let record = self.prepare_retirement(revision, key)?;
        if record.view.key(key)?.state == State::Deleted {
            return Ok(record.view);
        }
        let version = record.version(key)?;
        let custody =
            custody::Store::open_at(record.scope, self.socket.as_deref()).map_err(secret_error)?;
        let mut guard_failure = None;
        let receipt = custody
            .delete_guarded(version, || {
                custody_guard(
                    (|| {
                        let current = self.current()?;
                        current.expect(record.view.revision)?;
                        if current.version(key)? != version
                            || current.view.key(key)?.state != State::Retiring
                        {
                            return Err(Failure::Conflict);
                        }
                        Ok(())
                    })(),
                    &mut guard_failure,
                )
            })
            .map_err(|error| guard_failure.unwrap_or_else(|| secret_error(error)))?;
        self.confirm_retirement(&record, key, &receipt)
    }
    fn prepare_retirement(&self, revision: Uuid, key: Uuid) -> Result<Record> {
        Ok({
            let mut metadata = Metadata::update_approval_keys(&self.path)?;
            let record = self.load(&metadata)?.ok_or(Failure::NotFound)?;
            record.expect(revision)?;
            match record.view.key(key)?.state {
                State::Active => return Err(Failure::Conflict),
                State::Deleted | State::Retiring => record,
                State::Candidate | State::Retired => {
                    let tx = metadata.connection.transaction().map_err(db)?;
                    tx.execute(
                        "UPDATE local_approval_keys SET state='retiring' WHERE key_id=?1",
                        [key.to_string()],
                    )
                    .map_err(db)?;
                    bump(&tx, record.issuer_id)?;
                    tx.commit().map_err(|_| Failure::OutcomeUnknown)?;
                    self.load(&metadata)
                        .map_err(|_| Failure::OutcomeUnknown)?
                        .ok_or(Failure::OutcomeUnknown)?
                }
            }
        })
    }
    fn confirm_retirement(
        &self,
        record: &Record,
        key: Uuid,
        receipt: &custody::DeletedVersion,
    ) -> Result<View> {
        if !receipt.matches(record.version(key)?) {
            return Err(Failure::Conflict);
        }
        let mut metadata = Metadata::update_approval_keys(&self.path)?;
        let current = self.load(&metadata)?.ok_or(Failure::NotFound)?;
        current.expect(record.view.revision)?;
        if current.view.key(key)?.state != State::Retiring {
            return Err(Failure::Conflict);
        }
        let tx = metadata.connection.transaction().map_err(db)?;
        tx.execute(
            "UPDATE local_approval_keys SET state='deleted' WHERE key_id=?1",
            [key.to_string()],
        )
        .map_err(db)?;
        bump(&tx, current.issuer_id)?;
        tx.commit().map_err(|_| Failure::OutcomeUnknown)?;
        Ok(self
            .load(&metadata)
            .map_err(|_| Failure::OutcomeUnknown)?
            .ok_or(Failure::OutcomeUnknown)?
            .view)
    }
    /// Acquire BEFORE an approval preparation/spend transaction. Keep this guard
    /// borrowed by consumer policy until that transaction is acknowledged.
    pub fn acquire_key(&self) -> Result<KeyUse> {
        let lease = Lease::acquire(&self.path, false)?;
        let record = self.current()?;
        let active = record.view.active().ok_or(Failure::NotFound)?;
        let version = record.version(active.key_id)?;
        Ok(KeyUse {
            lease,
            version,
            socket: self.socket.clone(),
            key: approvals::ConfiguredApprovalKey {
                issuer: record.view.issuer.clone(),
                audience: record.view.audience.clone(),
                kid: active.key_id.to_string(),
                public_key: active.public_key.clone(),
                not_before_unix_ms: 0,
                not_after_unix_ms: MAX_TIME,
                revoked: false,
            },
        })
    }
    fn current(&self) -> Result<Record> {
        let metadata = Metadata::inspect(&self.path)?;
        let version: i64 = metadata
            .connection
            .pragma_query_value(None, "user_version", |r| r.get(0))
            .map_err(db)?;
        if version < 7 {
            return Err(Failure::NotFound);
        }
        self.load(&metadata)?.ok_or(Failure::NotFound)
    }
    fn check_binding(&self, connection: &rusqlite::Connection, create: bool) -> Result<()> {
        let version: i64 = connection
            .pragma_query_value(None, "user_version", |r| r.get(0))
            .map_err(db)?;
        if version < 2 {
            return Err(Failure::MetadataUnavailable);
        }
        let binding:Option<(String,String)>=connection.query_row("SELECT adapter_id, configuration_revision FROM registry_instances WHERE instance_id=?1",[&self.instance],|r|Ok((r.get(0)?,r.get(1)?))).optional().map_err(db)?;
        match binding {
            Some((adapter, configuration))
                if adapter == self.adapter && configuration == self.configuration =>
            {
                Ok(())
            }
            Some(_) => Err(Failure::Conflict),
            None if create => {
                connection.execute("INSERT INTO registry_instances(instance_id,adapter_id,configuration_revision) VALUES (?1,?2,?3)",params![self.instance,self.adapter,self.configuration]).map_err(db)?;
                Ok(())
            }
            None => Ok(()),
        }
    }
    fn load(&self, metadata: &Metadata) -> Result<Option<Record>> {
        self.check_binding(&metadata.connection, false)?;
        let row:Option<(String,String,String)>=metadata.connection.query_row("SELECT issuer_id,revision,custody_scope FROM local_approval_issuers WHERE instance_id=?1",[&self.instance],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?))).optional().map_err(db)?;
        let Some((issuer, revision, scope)) = row else {
            return Ok(None);
        };
        let issuer_id = id(&issuer)?;
        let scope = custody::Scope::approval_signing(metadata.authority()?, id(&scope)?)
            .map_err(secret_error)?;
        let mut statement=metadata.connection.prepare("SELECT key_id,public_key,state,material_version FROM local_approval_keys WHERE issuer_id=?1 ORDER BY key_id LIMIT 129").map_err(db)?;
        let rows = statement
            .query_map([&issuer], |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, String>(2)?,
                    r.get::<_, String>(3)?,
                ))
            })
            .map_err(db)?;
        let mut keys = Vec::new();
        let mut versions = BTreeMap::new();
        for row in rows {
            let (key, public, state, version) = row.map_err(db)?;
            let key_id = id(&key)?;
            let decoded = URL_SAFE_NO_PAD
                .decode(&public)
                .map_err(|_| Failure::MetadataUnavailable)?;
            if decoded.len() != 32 || URL_SAFE_NO_PAD.encode(&decoded) != public {
                return Err(Failure::MetadataUnavailable);
            }
            versions.insert(
                key_id,
                custody::Version::new(scope, id(&version)?).map_err(secret_error)?,
            );
            keys.push(KeyView {
                key_id,
                public_key: public,
                state: State::parse(&state)?,
            });
        }
        if keys.len() > CAPACITY
            || keys.iter().filter(|k| k.state == State::Active).count() > 1
            || keys.iter().filter(|k| k.state == State::Candidate).count() > 1
        {
            return Err(Failure::MetadataUnavailable);
        }
        Ok(Some(Record {
            issuer_id,
            scope,
            versions,
            view: View {
                issuer: format!("connectors.local-issuer/{issuer}"),
                audience: format!("connectors.approval/{issuer}"),
                revision: id(&revision)?,
                keys,
            },
        }))
    }
}
fn bump(connection: &rusqlite::Connection, issuer: Uuid) -> Result<()> {
    if connection
        .execute(
            "UPDATE local_approval_issuers SET revision=?1 WHERE issuer_id=?2",
            params![Uuid::new_v4().to_string(), issuer.to_string()],
        )
        .map_err(db)?
        != 1
    {
        return Err(Failure::Conflict);
    }
    Ok(())
}
fn id(value: &str) -> Result<Uuid> {
    let id = Uuid::parse_str(value).map_err(|_| Failure::MetadataUnavailable)?;
    if id.is_nil() || id.to_string() != value {
        return Err(Failure::MetadataUnavailable);
    }
    Ok(id)
}
fn public_key(seed: &Secret) -> Result<String> {
    let key =
        Ed25519KeyPair::from_seed_unchecked(&seed.0).map_err(|_| Failure::CustodyUnavailable)?;
    Ok(URL_SAFE_NO_PAD.encode(key.public_key().as_ref()))
}

/// Non-clone, process-bound current-key exclusion. No automatic caller policy.
pub struct KeyUse {
    lease: Lease,
    key: approvals::ConfiguredApprovalKey,
    version: custody::Version,
    socket: Option<PathBuf>,
}
impl KeyUse {
    pub fn key(&self) -> Result<&approvals::ConfiguredApprovalKey> {
        self.lease.check()?;
        Ok(&self.key)
    }
    pub fn with_signer<T>(
        &self,
        use_signer: impl FnOnce(&approvals::Signer) -> Result<T>,
    ) -> Result<T> {
        self.lease.check()?;
        let custody = custody::Store::open_at(self.version.scope(), self.socket.as_deref())
            .map_err(secret_error)?;
        let seed = custody.read(self.version).map_err(secret_error)?;
        if public_key(&seed)? != self.key.public_key {
            return Err(Failure::CustodyUnavailable);
        }
        let signer = approvals::Signer::from_seed(seed, self.key.kid.clone())
            .map_err(|_| Failure::CustodyUnavailable)?;
        let result = use_signer(&signer)?;
        self.lease.check()?;
        Ok(result)
    }
}
struct Lease {
    _file: File,
    pid: u32,
}
impl Lease {
    fn acquire(path: &Path, exclusive: bool) -> Result<Self> {
        let directory = fs::directory(path, false, true)?;
        if exclusive {
            match fs::publish_new(&directory, OsStr::new(LOCK), &[]) {
                Ok(()) | Err(super::Failure::ConfigurationExists) => {}
                Err(error) => return Err(error.into()),
            }
        }
        let file = fs::private_file_at(&directory, OsStr::new(LOCK))?;
        let until = Instant::now() + Duration::from_secs(2);
        loop {
            // SAFETY: file owns a live descriptor, and flock is released at close.
            let result = unsafe {
                libc::flock(
                    file.as_raw_fd(),
                    libc::LOCK_NB
                        | if exclusive {
                            libc::LOCK_EX
                        } else {
                            libc::LOCK_SH
                        },
                )
            };
            if result == 0 {
                break;
            }
            if std::io::Error::last_os_error().kind() != std::io::ErrorKind::WouldBlock
                || Instant::now() >= until
            {
                return Err(Failure::Conflict);
            }
            std::thread::sleep(Duration::from_millis(5));
        }
        let current = fs::private_file_at(&directory, OsStr::new(LOCK))?;
        let a = file.metadata().map_err(|_| Failure::MetadataUnavailable)?;
        let b = current
            .metadata()
            .map_err(|_| Failure::MetadataUnavailable)?;
        if (a.dev(), a.ino()) != (b.dev(), b.ino()) {
            return Err(Failure::Conflict);
        }
        Ok(Self {
            _file: file,
            pid: std::process::id(),
        })
    }
    fn check(&self) -> Result<()> {
        if self.pid == std::process::id() {
            Ok(())
        } else {
            Err(Failure::Conflict)
        }
    }
}

#[cfg(test)]
mod tests;
