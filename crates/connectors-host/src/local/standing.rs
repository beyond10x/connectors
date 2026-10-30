//! Standing write grants of a served instance
//! (`ess/domains/standing_approvals.yaml`; contracts/service/compatibility.md
//! § 2.1, Keyed writes under a standing grant;
//! architecture-decision-record:standing-scoped-approval).
//!
//! Two parts, both read for every write on `POST /v1alpha2/invoke`:
//!
//! - [`read`]: the grant file the operator mounts, named by the service
//!   configuration's `standing_grants`. It is read on every call, so a removed
//!   grant or one listed in `revoked` admits nothing from the next call on.
//!   On Linux it is read as the `mounted_secret` binding reads its directory
//!   (`read_file`): a ConfigMap or Secret volume's `<name> -> ..data/<name>`
//!   layout is followed, and that layout's `..data` swap is how a changed
//!   file arrives. The read refuses (`Unreadable`, answered `unavailable`) a
//!   path through a directory another user can write or through any other
//!   symbolic link, a file that is not a regular file, writable by its group
//!   or by others, owned by anyone but the effective user or root, over
//!   [`FILE_LIMIT`] bytes, or not exactly one strict
//!   `connectors.standing-grants/1` document. An absent file holds no grant.
//! - [`Uses`]: the hourly count, `standing-grants.sqlite3` (mode 0600) in the
//!   instance's state directory beside the attempt ledger. Every write
//!   admitted under a grant is one `GrantUse` row; the row names the write's
//!   attempt, which is how the grant id is recorded with the attempt. It
//!   survives a restart. A use that names no attempt is deleted once its hour
//!   has passed, in batches of at most [`PRUNE_BATCH`] per take; a use that
//!   names an attempt is kept, as the attempt is.
//!
//! Nothing here grants caller authority: the host checks admission, the key and
//! the grant, in that order, before any attempt or provider request.
use super::filesystem as fs;
use rusqlite::{Connection, OpenFlags, OptionalExtension, TransactionBehavior, params};
use serde::Deserialize;
use serde_json::Value;
use std::{collections::BTreeSet, path::Path};
use uuid::Uuid;

/// The grant file's `format`.
pub const FORMAT: &str = "connectors.standing-grants/1";
/// The largest grant file read, in bytes.
pub const FILE_LIMIT: usize = 64 * 1024;
/// The window `max_per_hour` counts over.
pub const WINDOW_MS: i64 = 3_600_000;
/// Bounds of one document.
const GRANTS_LIMIT: usize = 256;
const TARGET_LIMIT: usize = 16;
const POINTER_LIMIT: usize = 256;
const MAX_PER_HOUR: u32 = 10_000;
const TIME_LIMIT: i64 = 9_007_199_254_740_991;
/// The use store's file in the state directory.
const USES_FILE: &str = "standing-grants.sqlite3";

/// The grant file could not be read safely. The host answers `unavailable`
/// and makes no provider request; the reason is logged, never answered.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Unreadable {
    /// The file is a symbolic link, not a regular file, writable by its group
    /// or others, or owned by another user.
    Custody,
    /// The file is larger than [`FILE_LIMIT`].
    Oversized,
    /// The bytes are not one strict `connectors.standing-grants/1` document.
    Malformed,
    /// The file could not be opened or read.
    Io,
}

/// Why no grant covers a write. Every one is answered `approval_required`
/// without naming the axis; the reason is for the host's log and its tests.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Uncovered {
    /// No grant file, or one without a grant for this instance.
    NoGrant,
    /// Grants exist for this instance, none for this operation.
    OtherOperation,
    /// A grant's target member is missing from the input.
    MissingMember,
    /// A grant's target member holds another value.
    OtherTarget,
    /// The host's clock reads `not_after` or later.
    Expired,
    /// The grant's id is listed in `revoked`.
    Revoked,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Document {
    format: String,
    grants: Vec<Grant>,
    #[serde(default)]
    revoked: Vec<String>,
}

/// One `connectors.standing_approvals.StandingGrant`.
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Grant {
    pub id: String,
    pub instance: String,
    pub operation: String,
    pub target: Vec<TargetMember>,
    pub not_after_unix_ms: i64,
    pub max_per_hour: u32,
}

/// One target member: an RFC 6901 pointer into the input and its exact value.
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct TargetMember {
    pub pointer: String,
    pub value: Value,
}

/// A read grant file.
#[derive(Clone, Debug, Default)]
pub struct Grants {
    grants: Vec<Grant>,
    revoked: BTreeSet<String>,
}

/// Reads the grant file at `path` now. `Ok(None)` when no file is there.
pub fn read(path: &Path) -> Result<Option<Grants>, Unreadable> {
    let Some(bytes) = read_file(path)? else {
        return Ok(None);
    };
    parse(&bytes).map(Some)
}

/// Refusal: anything but one strict document within the bounds.
pub fn parse(bytes: &[u8]) -> Result<Grants, Unreadable> {
    let document: Document =
        connectors_core::read_json(bytes).map_err(|_| Unreadable::Malformed)?;
    if document.format != FORMAT
        || document.grants.len() > GRANTS_LIMIT
        || document.revoked.len() > GRANTS_LIMIT
    {
        return Err(Unreadable::Malformed);
    }
    let mut ids = BTreeSet::new();
    for grant in &document.grants {
        if !connectors_core::valid_id(&grant.id)
            || !ids.insert(grant.id.clone())
            || !connectors_core::valid_id(&grant.instance)
            || !connectors_core::valid_id(&grant.operation)
            || !(0..=TIME_LIMIT).contains(&grant.not_after_unix_ms)
            || !(1..=MAX_PER_HOUR).contains(&grant.max_per_hour)
            || grant.target.is_empty()
            || grant.target.len() > TARGET_LIMIT
        {
            return Err(Unreadable::Malformed);
        }
        let mut pointers = BTreeSet::new();
        for member in &grant.target {
            if !valid_pointer(&member.pointer) || !pointers.insert(member.pointer.as_str()) {
                return Err(Unreadable::Malformed);
            }
        }
    }
    if document
        .revoked
        .iter()
        .any(|id| !connectors_core::valid_id(id))
    {
        return Err(Unreadable::Malformed);
    }
    Ok(Grants {
        grants: document.grants,
        revoked: document.revoked.into_iter().collect(),
    })
}

/// A nonempty RFC 6901 pointer: starts with `/`, and `~` only as `~0` or `~1`.
fn valid_pointer(pointer: &str) -> bool {
    if !pointer.starts_with('/') || pointer.len() > POINTER_LIMIT {
        return false;
    }
    let bytes = pointer.as_bytes();
    bytes
        .iter()
        .enumerate()
        .all(|(i, b)| *b != b'~' || matches!(bytes.get(i + 1), Some(b'0' | b'1')))
        && !pointer.chars().any(char::is_control)
}

impl Grants {
    /// The grants that cover a write of `operation` on `instance` with `input`
    /// at `now_ms`, in file order: same instance and operation, every target
    /// member present with its exact value, `now_ms < not_after_unix_ms`, and
    /// not revoked. Empty means no grant covers it; the reason is the most
    /// specific one met.
    pub fn covering(
        &self,
        instance: &str,
        operation: &str,
        input: &Value,
        now_ms: i64,
    ) -> Result<Vec<&Grant>, Uncovered> {
        let mut reason = Uncovered::NoGrant;
        let mut covering = Vec::new();
        for grant in self.grants.iter().filter(|g| g.instance == instance) {
            if grant.operation != operation {
                reason = reason.max_specific(Uncovered::OtherOperation);
                continue;
            }
            let mut matched = true;
            for member in &grant.target {
                match input.pointer(&member.pointer) {
                    None => {
                        reason = reason.max_specific(Uncovered::MissingMember);
                        matched = false;
                        break;
                    }
                    Some(value) if *value != member.value => {
                        reason = reason.max_specific(Uncovered::OtherTarget);
                        matched = false;
                        break;
                    }
                    Some(_) => {}
                }
            }
            if !matched {
                continue;
            }
            if self.revoked.contains(&grant.id) {
                reason = reason.max_specific(Uncovered::Revoked);
                continue;
            }
            if now_ms >= grant.not_after_unix_ms {
                reason = reason.max_specific(Uncovered::Expired);
                continue;
            }
            covering.push(grant);
        }
        if covering.is_empty() {
            Err(reason)
        } else {
            Ok(covering)
        }
    }
}

impl Uncovered {
    fn rank(self) -> u8 {
        match self {
            Self::NoGrant => 0,
            Self::OtherOperation => 1,
            Self::MissingMember => 2,
            Self::OtherTarget => 3,
            Self::Revoked => 4,
            Self::Expired => 5,
        }
    }
    fn max_specific(self, other: Self) -> Self {
        if other.rank() > self.rank() {
            other
        } else {
            self
        }
    }
}

/// Unix: the file itself is admitted when it is a regular file, not group-
/// or world-writable, owned by the effective user or root.
#[cfg(unix)]
fn admitted(metadata: &std::fs::Metadata) -> bool {
    use std::os::unix::fs::MetadataExt;
    metadata.is_file()
        && metadata.mode() & 0o022 == 0
        && (metadata.uid() == fs::uid() || metadata.uid() == 0)
}

/// Linux: read as the `mounted_secret` binding reads its directory
/// (`connectors_custody::mounted::open_delivered`). Every directory above the
/// file's own is owned by root or the effective user and writable by nobody
/// else; the file's directory is owned by one of them and is not group- or
/// world-writable unless it is a read-only mount; the name is the file
/// itself or the link `<name> -> ..data/<name>` a ConfigMap or Secret volume
/// presents, resolved only through its `..data` (a link to a `..`-prefixed
/// sibling owned by root or the effective user). No other symbolic link is
/// followed. The file is opened only after that inode is [`admitted`], so a
/// FIFO or a device at the name is refused without being opened. Every
/// refusal is [`Unreadable::Custody`].
#[cfg(target_os = "linux")]
fn read_file(path: &Path) -> Result<Option<Vec<u8>>, Unreadable> {
    match connectors_custody::mounted::open_delivered(path, admitted) {
        Ok(Some(file)) => read_bounded(file).map(Some),
        Ok(None) => Ok(None),
        Err(_) => Err(Unreadable::Custody),
    }
}

/// Other Unix (macOS): every directory to the file's is walked from `/`
/// without following a symbolic link, each owned by root or the effective
/// user and not group- or world-writable; the file is opened with
/// `O_NOFOLLOW | O_NONBLOCK` beneath it and checked on the descriptor. No
/// projected volume layout is admitted there: a symbolic link at the name is
/// refused.
#[cfg(all(unix, not(target_os = "linux")))]
fn read_file(path: &Path) -> Result<Option<Vec<u8>>, Unreadable> {
    use std::os::{fd::AsRawFd, fd::FromRawFd, unix::ffi::OsStrExt};
    let (Some(parent), Some(name)) = (path.parent(), path.file_name()) else {
        return Err(Unreadable::Custody);
    };
    fs::validate_path(path).map_err(|_| Unreadable::Custody)?;
    let directory = fs::directory(parent, false, false).map_err(|_| Unreadable::Custody)?;
    let name = std::ffi::CString::new(name.as_bytes()).map_err(|_| Unreadable::Custody)?;
    // SAFETY: the directory descriptor and NUL-terminated name remain live.
    let fd = unsafe {
        libc::openat(
            directory.as_raw_fd(),
            name.as_ptr(),
            libc::O_RDONLY | libc::O_NOFOLLOW | libc::O_CLOEXEC | libc::O_NONBLOCK,
        )
    };
    if fd < 0 {
        let error = std::io::Error::last_os_error();
        return match error.raw_os_error() {
            Some(libc::ENOENT) => Ok(None),
            // ELOOP: the name is a symbolic link.
            Some(libc::ELOOP) => Err(Unreadable::Custody),
            _ => Err(Unreadable::Io),
        };
    }
    // SAFETY: openat returned a new, owned descriptor.
    let file = unsafe { std::fs::File::from_raw_fd(fd) };
    let metadata = file.metadata().map_err(|_| Unreadable::Io)?;
    if !admitted(&metadata) {
        return Err(Unreadable::Custody);
    }
    read_bounded(file).map(Some)
}

/// At most [`FILE_LIMIT`] bytes of an admitted file.
fn read_bounded(file: std::fs::File) -> Result<Vec<u8>, Unreadable> {
    use std::io::Read;
    if file.metadata().map_err(|_| Unreadable::Io)?.len() > FILE_LIMIT as u64 {
        return Err(Unreadable::Oversized);
    }
    let mut bytes = Vec::new();
    file.take(FILE_LIMIT as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| Unreadable::Io)?;
    if bytes.len() > FILE_LIMIT {
        return Err(Unreadable::Oversized);
    }
    Ok(bytes)
}

/// Windows: a final reparse point refuses, and the opened handle must be
/// owner-only, as the service credential file is.
#[cfg(windows)]
fn read_file(path: &Path) -> Result<Option<Vec<u8>>, Unreadable> {
    let file = match fs::windows::open_no_follow(path) {
        Ok(file) => file,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(_) => return Err(Unreadable::Custody),
    };
    let metadata = file.metadata().map_err(|_| Unreadable::Io)?;
    if !metadata.is_file() || fs::windows::check_owner_only(&file).is_err() {
        return Err(Unreadable::Custody);
    }
    read_bounded(file).map(Some)
}

/// The use store failed; the host answers `unavailable` and dispatches nothing.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct UsesUnavailable;

/// The most rows one pass deletes from the use store.
const PRUNE_BATCH: u32 = 256;

/// One taken slot under a grant.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Use {
    pub use_id: Uuid,
    pub grant_id: String,
}

/// What [`Uses::take`] found.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Taken {
    /// A slot under this grant; it counts from now on.
    Slot(Use),
    /// Every covering grant has `max_per_hour` uses in the window; the
    /// earliest slot frees in this many seconds (1 to 3,600).
    AtCap { retry_after_seconds: u64 },
}

/// `connectors.standing_approvals.GrantUse` rows (module docs).
///
/// One handle holds one SQLite connection, and every call on it runs behind
/// its mutex: the uses one host process takes are serialized in that process
/// and never wait on each other in SQLite's busy handler, where a burst of
/// writers on slow storage could each wait out the busy timeout and answer
/// `unavailable` (the cause of a 503 under parallel load). Every write is one
/// `BEGIN IMMEDIATE` transaction, which takes SQLite's write lock before it
/// reads, so another handle on the same file (another process, or a second
/// handle in this one) waits in the busy handler and never meets the lock
/// upgrade SQLite refuses without waiting.
pub struct Uses {
    connection: std::sync::Mutex<Connection>,
}

/// How long one SQLite call waits for another handle's write lock.
const BUSY_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(5);

impl Uses {
    /// The store in the state directory `directory`, created when absent as
    /// an owner-only file (0600), as the metadata authority's are. An existing
    /// store that is not an owner-only regular file with one link, owned by
    /// the effective user, is refused (`UsesUnavailable`).
    pub fn open(directory: &Path) -> Result<Self, UsesUnavailable> {
        let path = directory.join(USES_FILE);
        create_private(directory)?;
        let connection = Connection::open_with_flags(
            &path,
            OpenFlags::SQLITE_OPEN_READ_WRITE
                | OpenFlags::SQLITE_OPEN_NO_MUTEX
                | OpenFlags::SQLITE_OPEN_NOFOLLOW,
        )
        .map_err(|_| UsesUnavailable)?;
        connection
            .busy_timeout(BUSY_TIMEOUT)
            .map_err(|_| UsesUnavailable)?;
        connection
            .execute_batch(
                "BEGIN IMMEDIATE;
                CREATE TABLE IF NOT EXISTS grant_uses(
                    use_id TEXT PRIMARY KEY NOT NULL,
                    grant_id TEXT NOT NULL,
                    instance_id TEXT NOT NULL,
                    admitted_at_ms INTEGER NOT NULL,
                    attempt_id TEXT UNIQUE,
                    state TEXT NOT NULL CHECK (state IN ('counted','bound','released')),
                    CHECK ((state='bound') = (attempt_id IS NOT NULL))
                ) STRICT;
                CREATE INDEX IF NOT EXISTS grant_uses_window
                    ON grant_uses(instance_id, grant_id, admitted_at_ms);
                COMMIT;",
            )
            .map_err(|_| UsesUnavailable)?;
        Ok(Self {
            connection: std::sync::Mutex::new(connection),
        })
    }

    fn connection(&self) -> std::sync::MutexGuard<'_, Connection> {
        // A panic while the lock was held left no open transaction: rusqlite
        // rolls back a dropped one.
        self.connection.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// One `BEGIN IMMEDIATE` transaction on this handle's connection.
    fn write<T>(
        &self,
        action: impl FnOnce(&rusqlite::Transaction<'_>) -> rusqlite::Result<T>,
    ) -> Result<T, UsesUnavailable> {
        let mut connection = self.connection();
        let tx = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|_| UsesUnavailable)?;
        let value = action(&tx).map_err(|_| UsesUnavailable)?;
        tx.commit().map_err(|_| UsesUnavailable)?;
        Ok(value)
    }

    /// Takes one slot under the first of `grants` with room in the window
    /// ending at `now_ms`, in one immediate transaction, so concurrent writers
    /// (in this process or another on the same state directory) never admit
    /// more than `max_per_hour`. A use counts while `counted` or `bound`, and
    /// every use admitted at or after `now_ms - WINDOW_MS` counts, later ones
    /// included, so a clock that moves backwards widens no window.
    pub fn take(
        &self,
        instance: &str,
        grants: &[&Grant],
        now_ms: i64,
    ) -> Result<Taken, UsesUnavailable> {
        let mut connection = self.connection();
        let tx = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|_| UsesUnavailable)?;
        let since = now_ms.saturating_sub(WINDOW_MS);
        // Retention: uses whose hour has ended and that name no attempt are
        // dropped, at most PRUNE_BATCH per take. A bound use is kept: it
        // records the grant with its attempt, which the ledger keeps.
        tx.execute(
            "DELETE FROM grant_uses WHERE rowid IN (SELECT rowid FROM grant_uses
             WHERE state!='bound' AND admitted_at_ms<?1 LIMIT ?2)",
            params![since, PRUNE_BATCH],
        )
        .map_err(|_| UsesUnavailable)?;
        let mut retry_after: Option<i64> = None;
        for grant in grants {
            let (count, oldest): (u32, Option<i64>) = tx
                .query_row(
                    "SELECT count(*), min(admitted_at_ms) FROM grant_uses
                     WHERE instance_id=?1 AND grant_id=?2 AND state!='released' AND admitted_at_ms>=?3",
                    params![instance, grant.id, since],
                    |row| Ok((row.get(0)?, row.get(1)?)),
                )
                .map_err(|_| UsesUnavailable)?;
            if count < grant.max_per_hour {
                let taken = Use {
                    use_id: Uuid::new_v4(),
                    grant_id: grant.id.clone(),
                };
                tx.execute(
                    "INSERT INTO grant_uses(use_id,grant_id,instance_id,admitted_at_ms,attempt_id,state)
                     VALUES (?1,?2,?3,?4,NULL,'counted')",
                    params![taken.use_id.to_string(), grant.id, instance, now_ms],
                )
                .map_err(|_| UsesUnavailable)?;
                tx.commit().map_err(|_| UsesUnavailable)?;
                return Ok(Taken::Slot(taken));
            }
            let frees = oldest.unwrap_or(now_ms).saturating_add(WINDOW_MS) - now_ms;
            retry_after = Some(retry_after.map_or(frees, |r| r.min(frees)));
        }
        let millis = retry_after.unwrap_or(WINDOW_MS).clamp(1, WINDOW_MS);
        Ok(Taken::AtCap {
            retry_after_seconds: u64::try_from((millis + 999) / 1000).unwrap_or(3600),
        })
    }

    /// Records the grant with the write's attempt: the use becomes `bound`.
    pub fn bind(&self, taken: &Use, attempt_id: Uuid) -> Result<(), UsesUnavailable> {
        let changed = self.write(|tx| {
            tx.execute(
                "UPDATE grant_uses SET state='bound', attempt_id=?2 WHERE use_id=?1 AND state='counted'",
                params![taken.use_id.to_string(), attempt_id.to_string()],
            )
        })?;
        if changed == 1 {
            Ok(())
        } else {
            Err(UsesUnavailable)
        }
    }

    /// No attempt was prepared for this use: it stops counting. A failed
    /// release leaves it counting until its hour ends, never the other way.
    pub fn release(&self, taken: &Use) -> Result<(), UsesUnavailable> {
        self.write(|tx| {
            tx.execute(
                "UPDATE grant_uses SET state='released' WHERE use_id=?1 AND state='counted'",
                [taken.use_id.to_string()],
            )
        })?;
        Ok(())
    }

    /// The grant recorded with `attempt_id`, if a grant admitted it.
    pub fn grant_of(&self, attempt_id: Uuid) -> Result<Option<String>, UsesUnavailable> {
        self.connection()
            .query_row(
                "SELECT grant_id FROM grant_uses WHERE attempt_id=?1 AND state='bound'",
                [attempt_id.to_string()],
                |row| row.get(0),
            )
            .optional()
            .map_err(|_| UsesUnavailable)
    }

    /// How many uses the store holds, counting or not.
    pub fn rows(&self) -> Result<u32, UsesUnavailable> {
        self.connection()
            .query_row("SELECT count(*) FROM grant_uses", [], |row| row.get(0))
            .map_err(|_| UsesUnavailable)
    }
}

/// The store file, created as the metadata authority creates its own
/// (`filesystem::publish_new`: an owner-only file, mode 0600 on Unix,
/// published under its name only when the name is free), then admitted as
/// that authority admits its own before SQLite opens it: a regular file
/// owned by the effective user, no group or other access, one link
/// (`filesystem::private_file_metadata_at`). The check reads the name's
/// metadata and opens nothing, so it cancels no lock another connection
/// holds. SQLite creates its journal with the database file's mode. The
/// directory is reached without following a symbolic link, below directories
/// no other user can write (`filesystem::directory`); that it is owner-only
/// is the state directory's own check at host start.
fn create_private(directory: &Path) -> Result<(), UsesUnavailable> {
    use super::Failure;
    let name = std::ffi::OsStr::new(USES_FILE);
    let parent = fs::directory(directory, false, false).map_err(|_| UsesUnavailable)?;
    match fs::publish_new(&parent, name, &[]) {
        Ok(()) | Err(Failure::ConfigurationExists) => {}
        Err(_) => return Err(UsesUnavailable),
    }
    fs::private_file_metadata_at(&parent, name).map_err(|_| UsesUnavailable)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn document(grants: Value, revoked: Value) -> Vec<u8> {
        serde_json::to_vec(&json!({"format": FORMAT, "grants": grants, "revoked": revoked}))
            .unwrap()
    }

    fn grant(id: &str, max: u32) -> Value {
        json!({
            "id": id, "instance": "leaf", "operation": "issue.create",
            "target": [{"pointer": "/owner", "value": "acme"}, {"pointer": "/repo", "value": "widgets"}],
            "not_after_unix_ms": 2_000_000_000_000_i64, "max_per_hour": max
        })
    }

    /// A state directory as the host admits one: owner-only.
    fn state_directory() -> (tempfile::TempDir, std::path::PathBuf) {
        let root = tempfile::tempdir().unwrap();
        let state = root.path().join("state");
        std::fs::create_dir(&state).unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&state, std::fs::Permissions::from_mode(0o700)).unwrap();
        }
        #[cfg(windows)]
        fs::windows::owner_only(&state);
        (root, state)
    }

    #[test]
    fn a_document_outside_the_format_is_malformed() {
        for bytes in [
            br#"{"format":"connectors.standing-grants/2","grants":[]}"#.to_vec(),
            br#"{"format":"connectors.standing-grants/1","grants":[],"extra":1}"#.to_vec(),
            br#"{"format":"connectors.standing-grants/1","grants":[],"grants":[]}"#.to_vec(),
            document(json!([grant("g1", 0)]), json!([])),
            document(json!([grant("g1", 1), grant("g1", 1)]), json!([])),
            document(
                json!([{"id":"g1","instance":"leaf","operation":"issue.create","target":[],"not_after_unix_ms":1,"max_per_hour":1}]),
                json!([]),
            ),
            document(
                json!([{"id":"g1","instance":"leaf","operation":"issue.create","target":[{"pointer":"owner","value":"a"}],"not_after_unix_ms":1,"max_per_hour":1}]),
                json!([]),
            ),
            document(
                json!([{"id":"g1","instance":"leaf","operation":"issue.create","target":[{"pointer":"/a~2","value":"a"}],"not_after_unix_ms":1,"max_per_hour":1}]),
                json!([]),
            ),
        ] {
            assert_eq!(parse(&bytes).err(), Some(Unreadable::Malformed));
        }
    }

    #[test]
    fn only_a_grant_matching_every_axis_covers_a_write() {
        let grants = parse(&document(
            json!([grant("g1", 1), {
                "id": "g2", "instance": "leaf", "operation": "issue.close",
                "target": [{"pointer": "/owner", "value": "acme"}],
                "not_after_unix_ms": 10, "max_per_hour": 1
            }]),
            json!([]),
        ))
        .unwrap();
        let input = json!({"owner": "acme", "repo": "widgets", "title": "t"});
        let now = 1_000;
        assert_eq!(
            grants
                .covering("leaf", "issue.create", &input, now)
                .unwrap()[0]
                .id,
            "g1"
        );
        assert_eq!(
            grants.covering("other", "issue.create", &input, now).err(),
            Some(Uncovered::NoGrant)
        );
        assert_eq!(
            grants.covering("leaf", "issue.delete", &input, now).err(),
            Some(Uncovered::OtherOperation)
        );
        assert_eq!(
            grants
                .covering("leaf", "issue.create", &json!({"owner": "acme"}), now)
                .err(),
            Some(Uncovered::MissingMember)
        );
        assert_eq!(
            grants
                .covering(
                    "leaf",
                    "issue.create",
                    &json!({"owner": "acme", "repo": "other"}),
                    now
                )
                .err(),
            Some(Uncovered::OtherTarget)
        );
        assert_eq!(
            grants.covering("leaf", "issue.close", &input, 10).err(),
            Some(Uncovered::Expired)
        );
        let revoked = parse(&document(json!([grant("g1", 1)]), json!(["g1"]))).unwrap();
        assert_eq!(
            revoked.covering("leaf", "issue.create", &input, now).err(),
            Some(Uncovered::Revoked)
        );
    }

    #[test]
    fn the_cap_counts_the_window_and_a_released_use_frees_its_slot() {
        let (_root, state) = state_directory();
        let uses = Uses::open(&state).unwrap();
        let parsed: Grant = serde_json::from_value(grant("g1", 2)).unwrap();
        let grants = [&parsed];
        let Taken::Slot(first) = uses.take("leaf", &grants, 10_000).unwrap() else {
            panic!("first slot refused")
        };
        let Taken::Slot(second) = uses.take("leaf", &grants, 20_000).unwrap() else {
            panic!("second slot refused")
        };
        assert_eq!(
            uses.take("leaf", &grants, 30_000).unwrap(),
            Taken::AtCap {
                retry_after_seconds: 3_580
            }
        );
        uses.release(&second).unwrap();
        let Taken::Slot(third) = uses.take("leaf", &grants, 30_000).unwrap() else {
            panic!("a released use still counted")
        };
        let attempt = Uuid::new_v4();
        uses.bind(&first, attempt).unwrap();
        assert_eq!(uses.grant_of(attempt).unwrap().as_deref(), Some("g1"));
        assert!(uses.bind(&first, Uuid::new_v4()).is_err());
        drop(third);
        // The first use leaves the window an hour after it was admitted.
        assert!(matches!(
            uses.take("leaf", &grants, 10_000 + WINDOW_MS + 1).unwrap(),
            Taken::Slot(_)
        ));
    }

    /// Retention of the use store: a use past its hour that names no attempt
    /// (released, or counted by a write that never prepared one) is deleted
    /// by the next take; a bound use stays, so the grant stays recorded with
    /// its attempt.
    #[test]
    fn uses_past_their_hour_that_name_no_attempt_are_deleted() {
        let (_root, state) = state_directory();
        let uses = Uses::open(&state).unwrap();
        let parsed: Grant = serde_json::from_value(grant("g1", 10)).unwrap();
        let grants = [&parsed];
        let slot = |now| match uses.take("leaf", &grants, now).unwrap() {
            Taken::Slot(taken) => taken,
            Taken::AtCap { .. } => panic!("slot refused"),
        };
        let bound = slot(1_000);
        let attempt = Uuid::new_v4();
        uses.bind(&bound, attempt).unwrap();
        let released = slot(2_000);
        uses.release(&released).unwrap();
        let _abandoned = slot(3_000);
        assert_eq!(uses.rows().unwrap(), 3);
        // Within the hour nothing is deleted.
        let _fresh = slot(WINDOW_MS);
        assert_eq!(uses.rows().unwrap(), 4);
        // An hour after the last of them: released and abandoned are gone.
        let _late = slot(3_000 + WINDOW_MS + 1);
        assert_eq!(uses.rows().unwrap(), 3);
        assert_eq!(uses.grant_of(attempt).unwrap().as_deref(), Some("g1"));
    }

    /// The use store is created owner-only, and an existing one that is not
    /// is refused, as the metadata authority refuses its own.
    #[cfg(unix)]
    #[test]
    fn the_use_store_is_owner_only() {
        use std::os::unix::fs::{MetadataExt, PermissionsExt};
        let (_root, state) = state_directory();
        drop(Uses::open(&state).unwrap());
        let store = state.join(USES_FILE);
        assert_eq!(std::fs::metadata(&store).unwrap().mode() & 0o777, 0o600);
        std::fs::set_permissions(&store, std::fs::Permissions::from_mode(0o644)).unwrap();
        assert!(Uses::open(&state).is_err());
    }

    #[cfg(unix)]
    fn put(path: &Path, bytes: &[u8], mode: u32) {
        use std::os::unix::fs::PermissionsExt;
        std::fs::write(path, bytes).unwrap();
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(mode)).unwrap();
    }

    /// The flat layout: the file itself at the name.
    #[cfg(unix)]
    #[test]
    fn a_regular_grant_file_is_read_and_an_absent_one_holds_no_grant() {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("grants.json");
        assert!(read(&path).unwrap().is_none());
        put(&path, &document(json!([grant("g1", 1)]), json!([])), 0o644);
        assert!(read(&path).unwrap().is_some());
    }

    /// A symbolic link at the name to any file but `..data/<name>` is
    /// refused, as is a file in a directory another user can write.
    #[cfg(unix)]
    #[test]
    fn a_foreign_link_or_a_writable_directory_is_refused() {
        use std::os::unix::fs::PermissionsExt;
        let root = tempfile::tempdir().unwrap();
        let bytes = document(json!([grant("g1", 1)]), json!([]));
        let real = root.path().join("real.json");
        put(&real, &bytes, 0o644);
        let link = root.path().join("grants.json");
        std::os::unix::fs::symlink(&real, &link).unwrap();
        assert_eq!(read(&link).err(), Some(Unreadable::Custody));

        let shared = root.path().join("shared");
        std::fs::create_dir(&shared).unwrap();
        put(&shared.join("grants.json"), &bytes, 0o644);
        std::fs::set_permissions(&shared, std::fs::Permissions::from_mode(0o777)).unwrap();
        assert_eq!(
            read(&shared.join("grants.json")).err(),
            Some(Unreadable::Custody)
        );
        // The same file below a writable directory is refused as well.
        let below = shared.join("below");
        std::fs::create_dir(&below).unwrap();
        put(&below.join("grants.json"), &bytes, 0o644);
        assert_eq!(
            read(&below.join("grants.json")).err(),
            Some(Unreadable::Custody)
        );
    }

    /// The projected layout of a ConfigMap or Secret volume:
    /// `<name> -> ..data/<name>`, `..data -> ..<generation>`. It is read, a
    /// swapped `..data` is read from the next call, and a link at the name to
    /// another file inside `..data` is refused.
    #[cfg(target_os = "linux")]
    #[test]
    fn the_projected_volume_layout_is_read_through_its_data_link() {
        let root = tempfile::tempdir().unwrap();
        let volume = root.path().join("volume");
        std::fs::create_dir(&volume).unwrap();
        let publish = |generation: &str, revoked: Value| {
            let data = volume.join(format!("..{generation}"));
            std::fs::create_dir(&data).unwrap();
            put(
                &data.join("grants.json"),
                &document(json!([grant("g1", 1)]), revoked),
                0o644,
            );
            put(&data.join("other.json"), b"{}", 0o644);
            let staged = volume.join("..data_tmp");
            std::os::unix::fs::symlink(format!("..{generation}"), &staged).unwrap();
            std::fs::rename(&staged, volume.join("..data")).unwrap();
        };
        publish("2026_10_30_01", json!([]));
        std::os::unix::fs::symlink("..data/grants.json", volume.join("grants.json")).unwrap();
        let input = json!({"owner": "acme", "repo": "widgets"});
        let first = read(&volume.join("grants.json")).unwrap().unwrap();
        assert!(first.covering("leaf", "issue.create", &input, 0).is_ok());
        publish("2026_10_30_02", json!(["g1"]));
        let second = read(&volume.join("grants.json")).unwrap().unwrap();
        assert_eq!(
            second.covering("leaf", "issue.create", &input, 0).err(),
            Some(Uncovered::Revoked)
        );
        std::os::unix::fs::symlink("..data/grants.json", volume.join("renamed.json")).unwrap();
        assert_eq!(
            read(&volume.join("renamed.json")).err(),
            Some(Unreadable::Custody)
        );
        std::os::unix::fs::symlink("..data/other.json", volume.join("grants2.json")).unwrap();
        assert_eq!(
            read(&volume.join("grants2.json")).err(),
            Some(Unreadable::Custody)
        );
    }
}
