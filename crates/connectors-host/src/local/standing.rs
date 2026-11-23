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
//! A grant may close its input (`input_keys`,
//! story:standing-grant-closes-the-input-to-listed-members): the input may
//! then hold nothing outside the grant's `target` and `input_keys` pointers
//! ([`Grants::covering`]). A grant without the member admits any other input
//! member, as before the member existed.
//!
//! Nothing here grants caller authority: the host checks admission, the key and
//! the grant, in that order, before any attempt or provider request.
use super::approval_keys::Lease;
use super::approvals::{self, ConfiguredApprovalKey, Signer};
use super::filesystem as fs;
use super::mutations::{Clock, ClockInterval, MAX_CLOCK_WIDTH_MS};
use super::runtime::channel::DEPTH;
use rusqlite::{Connection, OpenFlags, OptionalExtension, TransactionBehavior, params};
use serde::{Deserialize, Serialize};
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
const INPUT_KEYS_LIMIT: usize = 16;
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
    /// The grant lists `input_keys` and the input holds a leaf outside the
    /// target and that list, or nests deeper than the walk goes.
    OutsideInput,
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
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Grant {
    pub id: String,
    pub instance: String,
    pub operation: String,
    pub target: Vec<TargetMember>,
    /// Closed input. Absent: any other member of the input is admitted.
    /// Present (it may be empty, and is never `null`): the input holds nothing
    /// outside the `target` and `input_keys` pointers.
    #[serde(
        default,
        deserialize_with = "listed",
        skip_serializing_if = "Option::is_none"
    )]
    pub input_keys: Option<Vec<String>>,
    pub not_after_unix_ms: i64,
    pub max_per_hour: u32,
}

/// `input_keys` where the member is present: a list of strings. `null` is not
/// one, so it is refused instead of read as an absent member.
fn listed<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<Option<Vec<String>>, D::Error> {
    Vec::<String>::deserialize(deserializer).map(Some)
}

/// One target member: an RFC 6901 pointer into the input and its exact value.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
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
        // A closed grant: its pointers are distinct and none lies below
        // another, so each names its own part of the input.
        if let Some(keys) = &grant.input_keys {
            if keys.len() > INPUT_KEYS_LIMIT || !keys.iter().all(|key| valid_pointer(key)) {
                return Err(Unreadable::Malformed);
            }
            let all = grant.pointers();
            let overlaps = all.iter().enumerate().any(|(i, one)| {
                all.iter()
                    .enumerate()
                    .any(|(j, other)| i != j && other.starts_with(one))
            });
            if overlaps {
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

/// The reference tokens of a valid pointer, still escaped: `~0` and `~1` are
/// compared as written, which is exact because `valid_pointer` admits no other
/// use of `~`.
fn tokens(pointer: &str) -> Vec<&str> {
    pointer.split('/').skip(1).collect()
}

/// A member name as a reference token (RFC 6901: `~` as `~0`, `/` as `~1`).
fn token(name: &str) -> String {
    name.replace('~', "~0").replace('/', "~1")
}

impl Grant {
    /// The reference tokens of every `target` and `input_keys` pointer.
    fn pointers(&self) -> Vec<Vec<&str>> {
        self.target
            .iter()
            .map(|member| member.pointer.as_str())
            .chain(self.input_keys.iter().flatten().map(String::as_str))
            .map(tokens)
            .collect()
    }

    /// Whether `input` holds nothing outside this grant's pointers. The walk
    /// starts at the root: a non-empty object is descended; every other value
    /// (a scalar, `null`, an array, an empty object) is a leaf. A leaf is
    /// inside when a pointer equals its own or lies above it by reference
    /// tokens, so a listed pointer admits the whole value under it. Nesting
    /// is counted as `runtime::channel::depth` counts it (objects and arrays,
    /// the root is level 1); an input nested deeper than [`DEPTH`] is outside.
    fn closes(&self, input: &Value) -> bool {
        let allowed = self.pointers();
        let mut path = Vec::new();
        inside(input, &mut path, &allowed, 0)
    }
}

/// The walk of [`Grant::closes`] at `value`, whose pointer is `path` and which
/// lies inside `depth` containers.
fn inside(value: &Value, path: &mut Vec<String>, allowed: &[Vec<&str>], depth: usize) -> bool {
    let covered = allowed.iter().any(|pointer| {
        pointer.len() <= path.len() && pointer.iter().zip(path.iter()).all(|(a, b)| a == b)
    });
    if covered {
        return within(value, depth);
    }
    match value {
        Value::Object(members) if !members.is_empty() => {
            depth < DEPTH
                && members.iter().all(|(name, member)| {
                    path.push(token(name));
                    let admitted = inside(member, path, allowed, depth + 1);
                    path.pop();
                    admitted
                })
        }
        _ => false,
    }
}

/// Whether a covered value, lying inside `depth` containers, nests no deeper
/// than [`DEPTH`].
fn within(value: &Value, depth: usize) -> bool {
    match value {
        Value::Object(members) => {
            depth < DEPTH && members.values().all(|member| within(member, depth + 1))
        }
        Value::Array(elements) => {
            depth < DEPTH && elements.iter().all(|element| within(element, depth + 1))
        }
        _ => true,
    }
}

impl Grants {
    /// The grants that cover a write of `operation` on `instance` with `input`
    /// at `now_ms`, in file order: same instance and operation, every target
    /// member present with its exact value, nothing in `input` outside the
    /// grant's `target` and `input_keys` when it lists them
    /// ([`Uncovered::OutsideInput`]), `now_ms < not_after_unix_ms`, and
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
            if grant.input_keys.is_some() && !grant.closes(input) {
                reason = reason.max_specific(Uncovered::OutsideInput);
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
            Self::OutsideInput => 4,
            Self::Revoked => 5,
            Self::Expired => 6,
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

// The local signed grant file
// (story:local-invoke-admits-a-keyed-write-under-a-standing-grant;
// contracts/cli/v1alpha1/semantics.md, "Standing grants").
//
// The local CLI binding keeps one instance's grants in one compact JWS the
// instance's approval key signed: header `{alg, kid, typ}` with the `typ`
// [`SIGNED_TYPE`], payload `{iss, aud, document}`, `document` a
// `connectors.standing-grants/1` document under every rule of [`parse`]. One
// decoder ([`verify`]) reads it, for the three commands and for `invoke`; a
// bare document at the path does not verify. The commands that write
// ([`grant`], [`revoke`]) hold [`lock`] from reading the file to replacing it.

/// The `typ` of the signed grant file's header. A grant file is not an
/// approval proof and a proof is not a grant file.
pub const SIGNED_TYPE: &str = "b10x.connectors-standing-grants.v1+jws";
/// The lock file of [`lock`], in the state directory.
pub const LOCK: &str = "standing-grants.lock";

/// Why the signed grant file gave no grants.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FileRefusal {
    /// The file or a directory to it fails the custody rule of [`read_file`],
    /// or the read failed: answered `unavailable`.
    Unavailable,
    /// The path holds a file that does not verify (module comment above; a
    /// file over [`FILE_LIMIT`] included). It admits nothing.
    Unverified,
}

/// Why [`sign`], [`grant`], [`revoke`] or [`list`] refused. Nothing was
/// written, except on [`Refused::OutcomeUnknown`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Refused {
    /// The grant request, or the file it would make: `invalid_input`.
    InvalidInput,
    /// The path holds a file that does not verify, or names no file a
    /// replacement can be published at: `invalid_configuration`.
    InvalidConfiguration,
    /// The file holds no grant with the id, or the path holds no file.
    NotFound,
    /// The custody rule: `unavailable`.
    Unavailable,
    /// The approval key does not sign at this clock reading (revoked, outside
    /// its window, or not the signer's pair), as a proof's issuance refuses.
    Forbidden,
    /// The replacement was renamed into place and its durability is unknown.
    OutcomeUnknown,
}

/// The exclusive lock of the commands that write the grant file.
pub struct Lock {
    _lease: Lease,
}

/// The exclusive `flock` on [`LOCK`] in the state directory `state`, held
/// until the value is dropped. A command takes it before the key lease; the
/// executor holds the shared key lease and never takes this lock.
pub fn lock(state: &Path) -> super::approval_keys::Result<Lock> {
    Lease::acquire(state, LOCK, true).map(|lease| Lock { _lease: lease })
}

/// One reading of `clock` a grant file is signed and verified at: inside
/// the JSON-safe range and no wider than the clock rule admits. `None` is
/// answered `unavailable`.
pub fn reading(clock: &impl Clock) -> Option<ClockInterval> {
    let t = clock.now().ok()?;
    let width = t.upper_unix_ms.checked_sub(t.lower_unix_ms)?;
    (t.lower_unix_ms >= 0
        && t.upper_unix_ms <= TIME_LIMIT
        && (0..=MAX_CLOCK_WIDTH_MS).contains(&width))
    .then_some(t)
}

#[derive(Serialize)]
struct SignedDocument<'a> {
    format: &'a str,
    grants: &'a [Grant],
    revoked: [&'a str; 0],
}

#[derive(Serialize)]
struct SignedClaims<'a> {
    iss: &'a str,
    aud: &'a str,
    document: SignedDocument<'a>,
}

/// The signed grant file holding exactly `grants`, `revoked` empty, signed
/// by `signer` as `key` at the reading `now`. Refused when the document is
/// not one [`parse`] admits or the file would be over [`FILE_LIMIT`]
/// (`InvalidInput`), or when the key does not sign at `now` (`Forbidden`).
pub fn sign(
    signer: &Signer,
    key: &ConfiguredApprovalKey,
    now: ClockInterval,
    grants: &[Grant],
) -> Result<Vec<u8>, Refused> {
    let document = SignedDocument {
        format: FORMAT,
        grants,
        revoked: [],
    };
    let admitted = serde_json::to_vec(&document)
        .ok()
        .is_some_and(|bytes| parse(&bytes).is_ok());
    if !admitted {
        return Err(Refused::InvalidInput);
    }
    let claims = serde_json::to_value(SignedClaims {
        iss: &key.issuer,
        aud: &key.audience,
        document,
    })
    .map_err(|_| Refused::InvalidInput)?;
    let claims = connectors_core::canonical(&claims);
    if claims.len() > FILE_LIMIT {
        return Err(Refused::InvalidInput);
    }
    let signed = signer
        .seal(SIGNED_TYPE, key, now, &claims)
        .map_err(|failure| match failure {
            approvals::Failure::Unavailable => Refused::Unavailable,
            _ => Refused::Forbidden,
        })?;
    if signed.len() > FILE_LIMIT {
        return Err(Refused::InvalidInput);
    }
    Ok(signed)
}

/// The grants of the signed file `bytes`, when it verifies for `key` at the
/// reading `now`: one compact JWS of [`SIGNED_TYPE`] the key signed, the key
/// not revoked and `now` inside its window, header and payload canonical, the
/// payload exactly `{iss, aud, document}` with the key's issuer and audience,
/// and `document` one [`parse`] admits.
fn open(bytes: &[u8], key: &ConfiguredApprovalKey, now: ClockInterval) -> Option<Grants> {
    let claims = key.open(SIGNED_TYPE, bytes, now, FILE_LIMIT).ok()?;
    let value: Value = connectors_core::read_json(&claims).ok()?;
    if connectors_core::canonical(&value) != claims {
        return None;
    }
    let members = value.as_object()?;
    if members.len() != 3
        || members.get("iss")?.as_str()? != key.issuer
        || members.get("aud")?.as_str()? != key.audience
    {
        return None;
    }
    parse(&serde_json::to_vec(members.get("document")?).ok()?).ok()
}

/// The verified grants of the signed grant file at `path`, read on every
/// call under the custody rule of [`read_file`]. `Ok(None)`: the path holds
/// no file, which holds no grant.
pub fn verify(
    path: &Path,
    key: &ConfiguredApprovalKey,
    now: ClockInterval,
) -> Result<Option<Grants>, FileRefusal> {
    let bytes = match read_file(path) {
        Ok(Some(bytes)) => bytes,
        Ok(None) => return Ok(None),
        Err(Unreadable::Oversized | Unreadable::Malformed) => return Err(FileRefusal::Unverified),
        Err(Unreadable::Custody | Unreadable::Io) => return Err(FileRefusal::Unavailable),
    };
    open(&bytes, key, now)
        .map(Some)
        .ok_or(FileRefusal::Unverified)
}

impl Grants {
    /// Every grant of the file, in file order, expired ones included.
    pub fn grants(&self) -> &[Grant] {
        &self.grants
    }

    /// The grants a command writes back: those the file does not list in
    /// `revoked`. The commands write `revoked` empty, so a listed grant is
    /// removed, never admitted again by the list being dropped.
    fn kept(self) -> Vec<Grant> {
        let Self { grants, revoked } = self;
        grants
            .into_iter()
            .filter(|grant| !revoked.contains(&grant.id))
            .collect()
    }
}

/// The file's grants for a command: a file that does not verify is
/// `InvalidConfiguration` and is left as it is.
fn current(
    path: &Path,
    key: &ConfiguredApprovalKey,
    now: ClockInterval,
) -> Result<Option<Grants>, Refused> {
    verify(path, key, now).map_err(|refusal| match refusal {
        FileRefusal::Unavailable => Refused::Unavailable,
        FileRefusal::Unverified => Refused::InvalidConfiguration,
    })
}

/// Signs `grants` and replaces the file at `path` (mode 0600), or creates it.
fn publish(
    path: &Path,
    signer: &Signer,
    key: &ConfiguredApprovalKey,
    now: ClockInterval,
    grants: &[Grant],
) -> Result<(), Refused> {
    let signed = sign(signer, key, now, grants)?;
    fs::validate_path(path).map_err(|_| Refused::InvalidConfiguration)?;
    let (Some(parent), Some(name)) = (path.parent(), path.file_name()) else {
        return Err(Refused::InvalidConfiguration);
    };
    let directory = fs::directory(parent, false, false).map_err(|_| Refused::Unavailable)?;
    fs::publish_replace(&directory, name, &signed).map_err(|failure| match failure {
        super::Failure::OutcomeUnknown => Refused::OutcomeUnknown,
        _ => Refused::Unavailable,
    })
}

/// The document `approvals grant-standing --input-file` reads
/// (`connectors.standing_approvals.GrantRequest`): a grant without `id` and
/// `instance`. Any other member is refused.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct GrantRequest {
    operation: String,
    target: Vec<TargetMember>,
    #[serde(default, deserialize_with = "listed")]
    input_keys: Option<Vec<String>>,
    not_after_unix_ms: i64,
    max_per_hour: u32,
}

/// The grant `request` describes, with `id` and `instance` set. Refused
/// (`InvalidInput`) unless `request` is one JSON object that is a strict
/// `GrantRequest` with a `not_after_unix_ms` and a `max_per_hour` of at least
/// 1, and the grant is one [`parse`] admits. The clock is not read here.
fn requested(request: &[u8], id: String, instance: &str) -> Result<Grant, Refused> {
    if request.len() > FILE_LIMIT || super::runtime::channel::depth(request).is_err() {
        return Err(Refused::InvalidInput);
    }
    // One JSON object: a struct also reads from an array of its members.
    let request: Value = connectors_core::read_json(request).map_err(|_| Refused::InvalidInput)?;
    if !request.is_object() {
        return Err(Refused::InvalidInput);
    }
    let request: GrantRequest =
        serde_json::from_value(request).map_err(|_| Refused::InvalidInput)?;
    if request.not_after_unix_ms < 1 || !(1..=MAX_PER_HOUR).contains(&request.max_per_hour) {
        return Err(Refused::InvalidInput);
    }
    let grant = Grant {
        id,
        instance: instance.to_owned(),
        operation: request.operation,
        target: request.target,
        input_keys: request.input_keys,
        not_after_unix_ms: request.not_after_unix_ms,
        max_per_hour: request.max_per_hour,
    };
    let alone = SignedDocument {
        format: FORMAT,
        grants: std::slice::from_ref(&grant),
        revoked: [],
    };
    let admitted = serde_json::to_vec(&alone)
        .ok()
        .is_some_and(|bytes| parse(&bytes).is_ok());
    if !admitted {
        return Err(Refused::InvalidInput);
    }
    Ok(grant)
}

/// Whether `request` is a grant request [`grant`] can take for `instance`,
/// decided without the clock, the key or the file: what a command refuses
/// before it reads any of them.
pub fn check_request(request: &[u8], instance: &str) -> Result<(), Refused> {
    requested(request, Uuid::new_v4().to_string(), instance).map(drop)
}

/// Adds the grant `request` describes to the signed file at `path` for the
/// instance `instance` and returns its id, a new UUID. Grants whose
/// `not_after_unix_ms` is at or before the reading's lower bound are dropped
/// first. `InvalidInput`: a request [`check_request`] refuses, a
/// `not_after_unix_ms` at or before the reading's upper bound, 256 grants
/// left in the file, a signed file over [`FILE_LIMIT`]. The caller holds
/// `lock` until this returns.
pub fn grant(
    _lock: &Lock,
    path: &Path,
    signer: &Signer,
    key: &ConfiguredApprovalKey,
    instance: &str,
    now: ClockInterval,
    request: &[u8],
) -> Result<String, Refused> {
    let new = requested(request, Uuid::new_v4().to_string(), instance)?;
    if new.not_after_unix_ms <= now.upper_unix_ms {
        return Err(Refused::InvalidInput);
    }
    let mut grants = current(path, key, now)?
        .map(Grants::kept)
        .unwrap_or_default();
    grants.retain(|grant| grant.not_after_unix_ms > now.lower_unix_ms);
    if grants.len() >= GRANTS_LIMIT {
        return Err(Refused::InvalidInput);
    }
    let id = new.id.clone();
    grants.push(new);
    publish(path, signer, key, now, &grants)?;
    Ok(id)
}

/// Removes the grant `grant_id` from the signed file at `path` and replaces
/// the file. `NotFound`: the file holds no grant with that id, or the path
/// holds no file. The caller holds `lock` until this returns.
pub fn revoke(
    _lock: &Lock,
    path: &Path,
    signer: &Signer,
    key: &ConfiguredApprovalKey,
    now: ClockInterval,
    grant_id: &str,
) -> Result<(), Refused> {
    let Some(file) = current(path, key, now)? else {
        return Err(Refused::NotFound);
    };
    if !file.grants().iter().any(|grant| grant.id == grant_id) {
        return Err(Refused::NotFound);
    }
    let mut grants = file.kept();
    grants.retain(|grant| grant.id != grant_id);
    publish(path, signer, key, now, &grants)
}

/// The grants of the verified file at `path` in file order, expired ones
/// included; none when the path holds no file. Nothing is written or signed.
pub fn list(
    path: &Path,
    key: &ConfiguredApprovalKey,
    now: ClockInterval,
) -> Result<Vec<Grant>, Refused> {
    Ok(current(path, key, now)?
        .map(|file| file.grants)
        .unwrap_or_default())
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

    /// A grant for `chat.postMessage` on `leaf`, with `input_keys` when given.
    fn closed(id: &str, target: Value, input_keys: Option<Value>) -> Value {
        let mut grant = json!({
            "id": id, "instance": "leaf", "operation": "chat.postMessage",
            "target": target,
            "not_after_unix_ms": 2_000_000_000_000_i64, "max_per_hour": 1
        });
        if let Some(input_keys) = input_keys {
            grant["input_keys"] = input_keys;
        }
        grant
    }

    /// The target of every closed-input case: one channel.
    fn channel() -> Value {
        json!([{"pointer": "/body/channel", "value": "C1"}])
    }

    fn file(grants: Value) -> Result<Grants, Unreadable> {
        parse(&document(grants, json!([])))
    }

    /// The ids of the grants covering `input`, in file order.
    fn covers(grants: &Grants, input: &Value) -> Result<Vec<String>, Uncovered> {
        grants
            .covering("leaf", "chat.postMessage", input, 1_000)
            .map(|covering| covering.iter().map(|grant| grant.id.clone()).collect())
    }

    /// `levels` containers nested in each other: objects under the member
    /// `a`, or arrays of one element. The innermost holds one scalar.
    fn nested(levels: usize, array: bool) -> Value {
        let mut value = if array { json!([1]) } else { json!({"a": 1}) };
        for _ in 1..levels {
            value = if array {
                json!([value])
            } else {
                json!({"a": value})
            };
        }
        value
    }

    /// Acceptance 1 of story:standing-grant-closes-the-input-to-listed-members.
    #[test]
    fn closed_input_refuses_every_leaf_outside_the_grant() {
        let grants = file(json!([closed(
            "g1",
            channel(),
            Some(json!(["/body/text", "/body/thread_ts"]))
        )]))
        .unwrap();
        let admitted = Ok(vec!["g1".to_owned()]);
        assert_eq!(
            covers(&grants, &json!({"body": {"channel": "C1", "text": "t"}})),
            admitted
        );
        assert_eq!(
            covers(
                &grants,
                &json!({"body": {"channel": "C1", "text": "t", "thread_ts": "1.2"}})
            ),
            admitted
        );
        for (what, input) in [
            (
                "/body/mrkdwn added",
                json!({"body": {"channel": "C1", "text": "t", "mrkdwn": true}}),
            ),
            (
                "/body/text2 added",
                json!({"body": {"channel": "C1", "text": "t", "text2": "u"}}),
            ),
            (
                "a second top-level member",
                json!({"body": {"channel": "C1", "text": "t"}, "query": "q"}),
            ),
            (
                "/body/extra holding {}",
                json!({"body": {"channel": "C1", "text": "t", "extra": {}}}),
            ),
            (
                "/body/extra holding []",
                json!({"body": {"channel": "C1", "text": "t", "extra": []}}),
            ),
            (
                "/body/extra holding null",
                json!({"body": {"channel": "C1", "text": "t", "extra": null}}),
            ),
            (
                "a member named te/xt",
                json!({"body": {"channel": "C1", "te/xt": "t"}}),
            ),
        ] {
            assert_eq!(
                covers(&grants, &input),
                Err(Uncovered::OutsideInput),
                "{what}"
            );
        }
        assert_eq!(
            covers(&grants, &json!("C1")),
            Err(Uncovered::MissingMember),
            "a scalar root"
        );
    }

    /// Acceptance 2: a listed pointer admits the subtree under it, an array
    /// is a leaf, and the walk stops at the depth the local channel admits.
    #[test]
    fn a_listed_pointer_admits_its_subtree() {
        let blocks = file(json!([closed(
            "g1",
            channel(),
            Some(json!(["/body/blocks"]))
        )]))
        .unwrap();
        let admitted = Ok(vec!["g1".to_owned()]);
        for value in [
            json!([{"type": "section"}, {"type": "divider"}]),
            json!({"first": {"type": "section", "fields": [1, 2]}}),
        ] {
            assert_eq!(
                covers(
                    &blocks,
                    &json!({"body": {"channel": "C1", "blocks": value}})
                ),
                admitted
            );
        }
        // A pointer into an array covers nothing: the array is the leaf.
        let element = file(json!([closed(
            "g1",
            channel(),
            Some(json!(["/body/blocks/0"]))
        )]))
        .unwrap();
        assert_eq!(
            covers(
                &element,
                &json!({"body": {"channel": "C1", "blocks": [{"type": "section"}]}})
            ),
            Err(Uncovered::OutsideInput)
        );
        // Depth, counted as `runtime::channel::depth` counts it: the root is
        // 1, `body` 2, and `/body/blocks` holds the rest.
        for array in [false, true] {
            let at = |levels| json!({"body": {"channel": "C1", "blocks": nested(levels, array)}});
            let (deepest_64, deepest_65) = (at(62), at(63));
            let channel_depth = |input: &Value| {
                super::super::runtime::channel::depth(&serde_json::to_vec(input).unwrap())
            };
            assert!(channel_depth(&deepest_64).is_ok(), "array: {array}");
            assert!(channel_depth(&deepest_65).is_err(), "array: {array}");
            assert_eq!(covers(&blocks, &deepest_64), admitted, "array: {array}");
            assert_eq!(
                covers(&blocks, &deepest_65),
                Err(Uncovered::OutsideInput),
                "array: {array}"
            );
        }
        // A target pointer into an array matches its member, and the walk
        // then meets the array as a leaf no pointer covers.
        let into_array = file(json!([closed(
            "g1",
            json!([{"pointer": "/body/blocks/0/type", "value": "section"}]),
            Some(json!([]))
        )]))
        .unwrap();
        assert_eq!(
            covers(
                &into_array,
                &json!({"body": {"blocks": [{"type": "section"}]}})
            ),
            Err(Uncovered::OutsideInput)
        );
    }

    /// Acceptance 3, first part: a grant without `input_keys` admits what it
    /// admitted before the member existed.
    #[test]
    fn an_absent_input_keys_leaves_the_input_open() {
        let grants = file(json!([closed("g1", channel(), None)])).unwrap();
        assert_eq!(
            covers(
                &grants,
                &json!({"body": {"channel": "C1", "text": "t", "mrkdwn": true}, "query": "q"})
            ),
            Ok(vec!["g1".to_owned()])
        );
        // No depth bound either: the caller's own bound is the only one.
        assert_eq!(
            covers(
                &grants,
                &json!({"body": {"channel": "C1", "blocks": nested(80, false)}})
            ),
            Ok(vec!["g1".to_owned()])
        );
    }

    /// Acceptance 3, second part.
    #[test]
    fn an_empty_input_keys_admits_only_the_target() {
        let grants = file(json!([closed("g1", channel(), Some(json!([])))])).unwrap();
        assert_eq!(
            covers(&grants, &json!({"body": {"channel": "C1"}})),
            Ok(vec!["g1".to_owned()])
        );
        assert_eq!(
            covers(&grants, &json!({"body": {"channel": "C1", "text": "t"}})),
            Err(Uncovered::OutsideInput)
        );
    }

    /// Acceptance 3, third part: a closed list limits its own grant only.
    #[test]
    fn the_second_grant_covers_when_the_first_is_closed() {
        let grants = file(json!([
            closed("g1", channel(), Some(json!([]))),
            closed("g2", channel(), Some(json!(["/body/text"])))
        ]))
        .unwrap();
        assert_eq!(
            covers(&grants, &json!({"body": {"channel": "C1", "text": "t"}})),
            Ok(vec!["g2".to_owned()])
        );
        // Outside both lists the reason is the closed input.
        assert_eq!(
            covers(
                &grants,
                &json!({"body": {"channel": "C1", "text": "t", "mrkdwn": true}})
            ),
            Err(Uncovered::OutsideInput)
        );
    }

    /// Acceptance 4.
    #[test]
    fn input_keys_are_validated_at_parse() {
        let pointers =
            |count: usize| -> Value { (0..count).map(|n| json!(format!("/body/k{n}"))).collect() };
        assert!(
            file(json!([closed("g1", channel(), Some(pointers(16)))])).is_ok(),
            "16 pointers"
        );
        assert!(
            file(json!([closed(
                "g1",
                channel(),
                Some(json!(["/body/text", "/body/text2"]))
            )]))
            .is_ok(),
            "/body/text beside /body/text2"
        );
        let long = format!("/{}", "k".repeat(256));
        assert_eq!(long.len(), 257);
        assert!(
            file(json!([closed(
                "g1",
                channel(),
                Some(json!([&long[..256]]))
            )]))
            .is_ok(),
            "a pointer of 256 bytes"
        );
        for (what, input_keys) in [
            ("17 pointers", pointers(17)),
            ("a pointer of 257 bytes", json!([long])),
            ("no leading /", json!(["body/text"])),
            ("the root pointer", json!([""])),
            ("~2", json!(["/body/te~2xt"])),
            (
                "the same pointer twice",
                json!(["/body/text", "/body/text"]),
            ),
            ("equal to a target pointer", json!(["/body/channel"])),
            ("/body beside target /body/channel", json!(["/body"])),
            ("below the target pointer", json!(["/body/channel/id"])),
            (
                "/body/text beside /body/text/x",
                json!(["/body/text", "/body/text/x"]),
            ),
            ("not a list", json!("/body/text")),
            ("null", Value::Null),
            ("a pointer that is not a string", json!([1])),
        ] {
            assert_eq!(
                file(json!([closed("g1", channel(), Some(input_keys))])).err(),
                Some(Unreadable::Malformed),
                "{what}"
            );
        }
        // Without `input_keys` a target reads as before the member existed:
        // one target pointer below another is no refusal there.
        let below = json!([
            {"pointer": "/body", "value": {"channel": "C1"}},
            {"pointer": "/body/channel", "value": "C1"}
        ]);
        assert!(file(json!([closed("g1", below.clone(), None)])).is_ok());
        assert_eq!(
            file(json!([closed("g1", below, Some(json!([])))])).err(),
            Some(Unreadable::Malformed),
            "one target pointer below another, with input_keys"
        );
    }
}

/// The local signed grant file
/// (story:local-invoke-admits-a-keyed-write-under-a-standing-grant).
#[cfg(all(test, unix))]
mod signed_tests {
    use super::*;
    use crate::local::approvals::{
        Evidence,
        tests::{PUBLIC, signer},
    };
    use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
    use connectors_sdk::Secret;
    use ring::{
        rand::{SecureRandom, SystemRandom},
        signature::{Ed25519KeyPair, KeyPair},
    };
    use serde_json::json;
    use std::os::unix::fs::PermissionsExt;
    use std::path::PathBuf;

    const NOW: i64 = 1_789_056_000_000;
    /// The seed of `approvals::tests::signer`: public RFC 8032 §7.1 test
    /// material, never a deployment signing key.
    const SEED: &str = "9d61b19deffd5a60ba844af492ec2cc44449c5697b326919703bac031cae7f60";
    const PROOF_TYPE: &str = "b10x.connectors-approval.v1+jws";

    fn now() -> ClockInterval {
        ClockInterval {
            lower_unix_ms: NOW,
            upper_unix_ms: NOW + 2_000,
        }
    }

    /// The approval key of the instance `leaf`, the pair of `signer()`.
    fn key() -> ConfiguredApprovalKey {
        ConfiguredApprovalKey {
            issuer: "connectors.local-issuer/leaf".into(),
            audience: "connectors.approval/leaf".into(),
            kid: "key-1".into(),
            public_key: URL_SAFE_NO_PAD.encode(hex::decode(PUBLIC).unwrap()),
            not_before_unix_ms: 0,
            not_after_unix_ms: NOW + 1_000_000,
            revoked: false,
        }
    }

    /// A pair generated here, its signer under `kid` and its configured key.
    fn other(kid: &str) -> (Signer, ConfiguredApprovalKey) {
        let mut seed = [0u8; 32];
        SystemRandom::new().fill(&mut seed).unwrap();
        let pair = Ed25519KeyPair::from_seed_unchecked(&seed).unwrap();
        let configured = ConfiguredApprovalKey {
            kid: kid.into(),
            public_key: URL_SAFE_NO_PAD.encode(pair.public_key().as_ref()),
            ..key()
        };
        (
            Signer::from_seed(Secret(seed.to_vec()), kid.into()).unwrap(),
            configured,
        )
    }

    /// A state directory and, beside it, the path of the instance's grant file.
    struct Site {
        _root: tempfile::TempDir,
        state: PathBuf,
        path: PathBuf,
    }

    fn site() -> Site {
        let root = tempfile::tempdir().unwrap();
        let state = root.path().join("state");
        std::fs::create_dir(&state).unwrap();
        std::fs::set_permissions(&state, std::fs::Permissions::from_mode(0o700)).unwrap();
        let path = root.path().join("leaf.grants");
        Site {
            _root: root,
            state,
            path,
        }
    }

    fn put(path: &Path, bytes: &[u8], mode: u32) {
        std::fs::write(path, bytes).unwrap();
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(mode)).unwrap();
    }

    fn mode(path: &Path) -> u32 {
        std::fs::metadata(path).unwrap().permissions().mode() & 0o7777
    }

    /// A grant of `leaf` for `chat.postMessage` to the channel `C1`.
    fn channel_grant(id: &str, not_after_unix_ms: i64) -> Grant {
        Grant {
            id: id.into(),
            instance: "leaf".into(),
            operation: "chat.postMessage".into(),
            target: vec![TargetMember {
                pointer: "/body/channel".into(),
                value: json!("C1"),
            }],
            input_keys: Some(vec!["/body/text".into()]),
            not_after_unix_ms,
            max_per_hour: 2,
        }
    }

    /// The request document of `approvals grant-standing --input-file`.
    fn request() -> Value {
        json!({
            "operation": "chat.postMessage",
            "target": [{"pointer": "/body/channel", "value": "C1"}],
            "input_keys": ["/body/text"],
            "not_after_unix_ms": NOW + 600_000,
            "max_per_hour": 2
        })
    }

    fn bytes(value: &Value) -> Vec<u8> {
        serde_json::to_vec(value).unwrap()
    }

    fn write() -> Value {
        json!({"body": {"channel": "C1", "text": "hello"}})
    }

    /// One compact JWS of the RFC 8032 test pair over `header` and `claims`.
    fn compact(header: &Value, claims: &[u8]) -> Vec<u8> {
        let pair = Ed25519KeyPair::from_seed_unchecked(&hex::decode(SEED).unwrap()).unwrap();
        let input = format!(
            "{}.{}",
            URL_SAFE_NO_PAD.encode(connectors_core::canonical(header)),
            URL_SAFE_NO_PAD.encode(claims)
        );
        let signature = URL_SAFE_NO_PAD.encode(pair.sign(input.as_bytes()).as_ref());
        format!("{input}.{signature}").into_bytes()
    }

    /// The claims bytes of a signed grant file.
    fn claims_of(signed: &[u8]) -> Vec<u8> {
        let text = std::str::from_utf8(signed).unwrap();
        URL_SAFE_NO_PAD
            .decode(text.split('.').nth(1).unwrap())
            .unwrap()
    }

    #[derive(Debug, PartialEq)]
    enum Outcome {
        /// Admitted under the grant of this id; one use is counted.
        Admitted(String),
        ApprovalRequired,
        RateLimited,
        Unavailable,
    }

    /// The composition run 3 gives `standing::admit`, here only as far as
    /// this run's seam reaches: the verified file, the covering grants at the
    /// reading's upper bound, one use.
    fn admit(site: &Site, key: &ConfiguredApprovalKey, input: &Value) -> Outcome {
        let grants = match verify(&site.path, key, now()) {
            Ok(Some(grants)) => grants,
            Ok(None) | Err(FileRefusal::Unverified) => return Outcome::ApprovalRequired,
            Err(FileRefusal::Unavailable) => return Outcome::Unavailable,
        };
        let at = now().upper_unix_ms;
        let Ok(covering) = grants.covering("leaf", "chat.postMessage", input, at) else {
            return Outcome::ApprovalRequired;
        };
        match Uses::open(&site.state).unwrap().take("leaf", &covering, at) {
            Ok(Taken::Slot(taken)) => Outcome::Admitted(taken.grant_id),
            Ok(Taken::AtCap { .. }) => Outcome::RateLimited,
            Err(_) => Outcome::Unavailable,
        }
    }

    fn uses(site: &Site) -> u32 {
        Uses::open(&site.state).unwrap().rows().unwrap()
    }

    /// Acceptance 1.
    #[test]
    fn a_signed_grant_file_admits_the_granted_write_and_counts_one_use() {
        let site = site();
        let signed = sign(
            &signer(),
            &key(),
            now(),
            &[channel_grant("g1", NOW + 600_000)],
        )
        .unwrap();
        // Header and payload are the contract's: exactly these members.
        let text = std::str::from_utf8(&signed).unwrap();
        let header: Value = serde_json::from_slice(
            &URL_SAFE_NO_PAD
                .decode(text.split('.').next().unwrap())
                .unwrap(),
        )
        .unwrap();
        assert_eq!(
            header,
            json!({"alg": "Ed25519", "kid": "key-1", "typ": "b10x.connectors-standing-grants.v1+jws"})
        );
        let claims: Value = serde_json::from_slice(&claims_of(&signed)).unwrap();
        assert_eq!(claims["iss"], "connectors.local-issuer/leaf");
        assert_eq!(claims["aud"], "connectors.approval/leaf");
        assert_eq!(claims["document"]["format"], FORMAT);
        assert_eq!(claims["document"]["revoked"], json!([]));
        assert_eq!(claims.as_object().unwrap().len(), 3);
        put(&site.path, &signed, 0o600);
        assert_eq!(uses(&site), 0);
        assert_eq!(
            admit(&site, &key(), &write()),
            Outcome::Admitted("g1".into())
        );
        assert_eq!(uses(&site), 1);
        // A write outside the grant counts none; no file admits nothing.
        let outside = json!({"body": {"channel": "C2", "text": "hello"}});
        assert_eq!(admit(&site, &key(), &outside), Outcome::ApprovalRequired);
        std::fs::remove_file(&site.path).unwrap();
        assert_eq!(
            verify(&site.path, &key(), now()).map(|file| file.is_none()),
            Ok(true)
        );
        assert_eq!(admit(&site, &key(), &write()), Outcome::ApprovalRequired);
        assert_eq!(uses(&site), 1);
    }

    /// Acceptance 2.
    #[test]
    fn a_grant_file_that_does_not_verify_admits_nothing() {
        let grants = [channel_grant("g1", NOW + 600_000)];
        let signed = sign(&signer(), &key(), now(), &grants).unwrap();
        // The control: the file as signed is admitted.
        let control = site();
        put(&control.path, &signed, 0o600);
        assert_eq!(
            admit(&control, &key(), &write()),
            Outcome::Admitted("g1".into())
        );

        // One changed byte, in the payload and in the signature.
        let dot = signed.iter().position(|b| *b == b'.').unwrap();
        let changed = |at: usize| {
            let mut bytes = signed.clone();
            bytes[at] = if bytes[at] == b'A' { b'B' } else { b'A' };
            bytes
        };
        // Signed by another key under the same kid.
        let (other_signer, other_key) = other("key-1");
        let by_other = sign(&other_signer, &other_key, now(), &grants).unwrap();
        // A per-input proof's `typ` over the same payload, and a file whose
        // header carries one member more.
        let proof_header = json!({"alg": "Ed25519", "kid": "key-1", "typ": PROOF_TYPE});
        let as_proof = compact(&proof_header, &claims_of(&signed));
        let wide_header = json!({"alg": "Ed25519", "kid": "key-1", "typ": SIGNED_TYPE, "crit": []});
        let wide = compact(&wide_header, &claims_of(&signed));
        // A bare unsigned grant document, the served host's form.
        let bare = bytes(&json!({"format": FORMAT, "grants": [grants[0]], "revoked": []}));
        assert!(parse(&bare).is_ok());
        // Another issuer, another audience, one payload member more.
        let iss = ConfiguredApprovalKey {
            issuer: "connectors.local-issuer/other".into(),
            ..key()
        };
        let aud = ConfiguredApprovalKey {
            audience: "connectors.approval/other".into(),
            ..key()
        };
        let by_iss = sign(&signer(), &iss, now(), &grants).unwrap();
        let by_aud = sign(&signer(), &aud, now(), &grants).unwrap();
        let mut wide_claims: Value = serde_json::from_slice(&claims_of(&signed)).unwrap();
        wide_claims["exp"] = json!(NOW);
        let good_header = json!({"alg": "Ed25519", "kid": "key-1", "typ": SIGNED_TYPE});
        let by_member = compact(&good_header, &connectors_core::canonical(&wide_claims));
        // The same members in another order: not canonical.
        let reordered = format!(
            r#"{{"iss":"connectors.local-issuer/leaf","aud":"connectors.approval/leaf","document":{}}}"#,
            String::from_utf8(connectors_core::canonical(
                &serde_json::from_slice::<Value>(&claims_of(&signed)).unwrap()["document"]
            ))
            .unwrap()
        );
        let by_order = compact(&good_header, reordered.as_bytes());
        for (case, file) in [
            ("a changed payload byte", changed(dot + 8)),
            ("a changed signature byte", changed(signed.len() - 8)),
            ("another key", by_other),
            ("a proof's typ", as_proof),
            ("a header member more", wide),
            ("a bare document", bare),
            ("another iss", by_iss),
            ("another aud", by_aud),
            ("a payload member more", by_member),
            ("a payload that is not canonical", by_order),
        ] {
            let site = site();
            put(&site.path, &file, 0o600);
            assert_eq!(
                verify(&site.path, &key(), now()).err(),
                Some(FileRefusal::Unverified),
                "{case}"
            );
            assert_eq!(
                admit(&site, &key(), &write()),
                Outcome::ApprovalRequired,
                "{case}"
            );
            assert_eq!(uses(&site), 0, "{case}");
        }

        // The good file under a key that no longer admits it: the key was
        // rotated (the configured key is the next one), retired at this
        // reading, not yet valid, or revoked.
        let (_, rotated) = other("key-2");
        let retired = ConfiguredApprovalKey {
            not_after_unix_ms: NOW + 2_000,
            ..key()
        };
        let early = ConfiguredApprovalKey {
            not_before_unix_ms: NOW + 1,
            ..key()
        };
        let revoked = ConfiguredApprovalKey {
            revoked: true,
            ..key()
        };
        for (case, configured) in [
            ("a rotated key", rotated),
            ("a retired key", retired),
            ("a key not yet valid", early),
            ("a revoked key", revoked),
        ] {
            let site = site();
            put(&site.path, &signed, 0o600);
            assert_eq!(
                verify(&site.path, &configured, now()).err(),
                Some(FileRefusal::Unverified),
                "{case}"
            );
            assert_eq!(
                admit(&site, &configured, &write()),
                Outcome::ApprovalRequired,
                "{case}"
            );
            assert_eq!(uses(&site), 0, "{case}");
        }
        // Such a key signs no file either.
        assert_eq!(
            sign(&signer(), &revoked_key(), now(), &grants).err(),
            Some(Refused::Forbidden)
        );
    }

    fn revoked_key() -> ConfiguredApprovalKey {
        ConfiguredApprovalKey {
            revoked: true,
            ..key()
        }
    }

    /// Acceptance 3.
    #[test]
    fn a_grant_file_is_not_a_proof() {
        let signed = sign(
            &signer(),
            &key(),
            now(),
            &[channel_grant("g1", NOW + 600_000)],
        )
        .unwrap();
        // The grant file's bytes, given to the proof decoder, are refused.
        let evidence = Evidence::from_protected("0".repeat(64), Secret(signed.clone())).unwrap();
        assert!(!evidence.decodes());
        // And the grant file's signer makes nothing of a proof's `typ`.
        assert!(
            signer()
                .seal(PROOF_TYPE, &key(), now(), &claims_of(&signed))
                .is_err()
        );
        assert!(key().open(PROOF_TYPE, &signed, now(), FILE_LIMIT).is_err());
    }

    /// Acceptance 4. A foreign owner is covered by `admitted`'s own test.
    #[test]
    fn an_unsafe_grant_file_is_unavailable() {
        let signed = sign(
            &signer(),
            &key(),
            now(),
            &[channel_grant("g1", NOW + 600_000)],
        )
        .unwrap();
        // The control: the same file at mode 0600 is admitted.
        let control = site();
        put(&control.path, &signed, 0o600);
        assert_eq!(
            admit(&control, &key(), &write()),
            Outcome::Admitted("g1".into())
        );
        fn group_writable(site: &Site, signed: &[u8]) {
            put(&site.path, signed, 0o620);
        }
        fn world_writable(site: &Site, signed: &[u8]) {
            put(&site.path, signed, 0o602);
        }
        fn linked(site: &Site, signed: &[u8]) {
            let real = site.path.with_extension("real");
            put(&real, signed, 0o600);
            std::os::unix::fs::symlink(&real, &site.path).unwrap();
        }
        fn directory(site: &Site, _: &[u8]) {
            std::fs::create_dir(&site.path).unwrap();
        }
        type Place = fn(&Site, &[u8]);
        let cases: [(&str, Place); 4] = [
            ("mode 0620", group_writable),
            ("mode 0602", world_writable),
            ("a symlink to it", linked),
            ("a directory at the path", directory),
        ];
        for (case, place) in cases {
            let site = site();
            place(&site, &signed);
            assert_eq!(
                verify(&site.path, &key(), now()).err(),
                Some(FileRefusal::Unavailable),
                "{case}"
            );
            assert_eq!(
                admit(&site, &key(), &write()),
                Outcome::Unavailable,
                "{case}"
            );
            assert_eq!(uses(&site), 0, "{case}");
            // The commands answer the same and write nothing.
            let lock = lock(&site.state).unwrap();
            assert_eq!(
                grant(
                    &lock,
                    &site.path,
                    &signer(),
                    &key(),
                    "leaf",
                    now(),
                    &bytes(&request())
                )
                .err(),
                Some(Refused::Unavailable),
                "{case}"
            );
            assert_eq!(
                revoke(&lock, &site.path, &signer(), &key(), now(), "g1").err(),
                Some(Refused::Unavailable),
                "{case}"
            );
            assert_eq!(
                list(&site.path, &key(), now()).err(),
                Some(Refused::Unavailable),
                "{case}"
            );
        }
    }

    /// Acceptance 11.
    #[test]
    fn grant_revoke_and_list_keep_the_file() {
        let site = site();
        let (signer, key) = (signer(), key());
        let lock = lock(&site.state).unwrap();
        assert!(site.state.join("standing-grants.lock").is_file());
        // An absent file: nothing listed, nothing to revoke, and a grant creates it.
        assert_eq!(list(&site.path, &key, now()), Ok(vec![]));
        assert_eq!(
            revoke(&lock, &site.path, &signer, &key, now(), "g1").err(),
            Some(Refused::NotFound)
        );
        assert!(!site.path.exists());
        let add = |request: &Value| {
            grant(
                &lock,
                &site.path,
                &signer,
                &key,
                "leaf",
                now(),
                &bytes(request),
            )
        };
        let first = add(&request()).unwrap();
        assert_eq!(first.parse::<Uuid>().unwrap().get_version_num(), 4);
        assert_eq!(first, first.parse::<Uuid>().unwrap().to_string());
        assert_eq!(mode(&site.path), 0o600);
        // A second grant keeps the first; `list` shows both in file order.
        let mut open = request();
        open.as_object_mut().unwrap().remove("input_keys");
        let second = add(&open).unwrap();
        assert_ne!(first, second);
        assert_eq!(mode(&site.path), 0o600);
        let listed = list(&site.path, &key, now()).unwrap();
        assert_eq!(
            listed
                .iter()
                .map(|grant| grant.id.as_str())
                .collect::<Vec<_>>(),
            [first.as_str(), second.as_str()]
        );
        assert!(listed.iter().all(|grant| grant.instance == "leaf"));
        assert_eq!(listed[0].input_keys, Some(vec!["/body/text".to_owned()]));
        assert_eq!(listed[1].input_keys, None);
        assert_eq!(
            admit(&site, &key, &write()),
            Outcome::Admitted(first.clone())
        );
        // `revoke` removes one; an id the file does not hold is `not_found`.
        revoke(&lock, &site.path, &signer, &key, now(), &first).unwrap();
        assert_eq!(mode(&site.path), 0o600);
        assert_eq!(
            list(&site.path, &key, now())
                .unwrap()
                .iter()
                .map(|grant| grant.id.clone())
                .collect::<Vec<_>>(),
            std::slice::from_ref(&second)
        );
        let before = std::fs::read(&site.path).unwrap();
        for absent in [first.as_str(), "no-such-grant"] {
            assert_eq!(
                revoke(&lock, &site.path, &signer, &key, now(), absent).err(),
                Some(Refused::NotFound)
            );
        }
        assert_eq!(std::fs::read(&site.path).unwrap(), before);

        // A file signed by a rotated key: all three are `invalid_configuration`
        // and the file's bytes are unchanged.
        let (next_signer, next_key) = other("key-2");
        assert_eq!(
            list(&site.path, &next_key, now()).err(),
            Some(Refused::InvalidConfiguration)
        );
        assert_eq!(
            grant(
                &lock,
                &site.path,
                &next_signer,
                &next_key,
                "leaf",
                now(),
                &bytes(&request())
            )
            .err(),
            Some(Refused::InvalidConfiguration)
        );
        assert_eq!(
            revoke(&lock, &site.path, &next_signer, &next_key, now(), &second).err(),
            Some(Refused::InvalidConfiguration)
        );
        assert_eq!(std::fs::read(&site.path).unwrap(), before);

        // A file that lists a grant in `revoked` (no command writes one): the
        // next grant removes the listed grant with the list.
        let pair = [
            channel_grant("g1", NOW + 600_000),
            channel_grant("g2", NOW + 600_000),
        ];
        let claims = json!({
            "iss": key.issuer, "aud": key.audience,
            "document": {"format": FORMAT, "grants": pair, "revoked": ["g1"]}
        });
        let header = json!({"alg": "Ed25519", "kid": "key-1", "typ": SIGNED_TYPE});
        put(
            &site.path,
            &compact(&header, &connectors_core::canonical(&claims)),
            0o600,
        );
        assert_eq!(list(&site.path, &key, now()).unwrap().len(), 2);
        let added = add(&request()).unwrap();
        assert_eq!(
            list(&site.path, &key, now())
                .unwrap()
                .iter()
                .map(|grant| grant.id.clone())
                .collect::<Vec<_>>(),
            ["g2".to_owned(), added]
        );

        // 256 grants of which one is expired: `grant` succeeds and the
        // expired one is gone.
        let small = |id: String, not_after_unix_ms: i64| Grant {
            target: vec![TargetMember {
                pointer: "/c".into(),
                value: json!(1),
            }],
            input_keys: None,
            ..channel_grant(&id, not_after_unix_ms)
        };
        let mut full: Vec<Grant> = (0..255)
            .map(|n| small(format!("g{n}"), NOW + 600_000))
            .collect();
        full.push(small("expired".into(), NOW));
        put(
            &site.path,
            &sign(&signer, &key, now(), &full).unwrap(),
            0o600,
        );
        assert_eq!(list(&site.path, &key, now()).unwrap().len(), 256);
        let added = add(&request()).unwrap();
        let listed = list(&site.path, &key, now()).unwrap();
        assert_eq!(listed.len(), 256);
        assert!(listed.iter().all(|grant| grant.id != "expired"));
        assert_eq!(listed.last().unwrap().id, added);
        // 256 unexpired grants: `grant` is `invalid_input`, nothing written.
        let before = std::fs::read(&site.path).unwrap();
        assert_eq!(add(&request()).err(), Some(Refused::InvalidInput));
        assert_eq!(std::fs::read(&site.path).unwrap(), before);

        // A grant that would make the signed file larger than 64 KiB is
        // `invalid_input` and the file's bytes are unchanged.
        let wide = |id: String| Grant {
            target: vec![TargetMember {
                pointer: "/c".into(),
                value: json!("v".repeat(3_000)),
            }],
            input_keys: None,
            ..channel_grant(&id, NOW + 600_000)
        };
        let near: Vec<Grant> = (0..15).map(|n| wide(format!("w{n}"))).collect();
        let near_file = sign(&signer, &key, now(), &near).unwrap();
        assert!(near_file.len() <= FILE_LIMIT);
        put(&site.path, &near_file, 0o600);
        let mut large = request();
        large["target"] = json!([{"pointer": "/c", "value": "v".repeat(6_000)}]);
        assert_eq!(add(&large).err(), Some(Refused::InvalidInput));
        assert_eq!(std::fs::read(&site.path).unwrap(), near_file);
        // A file over the limit at the path does not verify.
        put(&site.path, &vec![b'A'; FILE_LIMIT + 1], 0o600);
        assert_eq!(
            verify(&site.path, &key, now()).err(),
            Some(FileRefusal::Unverified)
        );
        assert_eq!(add(&request()).err(), Some(Refused::InvalidConfiguration));
        assert_eq!(
            std::fs::metadata(&site.path).unwrap().len(),
            FILE_LIMIT as u64 + 1
        );
    }

    /// Acceptance 12.
    #[test]
    fn eight_concurrent_grants_keep_all_eight() {
        let site = site();
        let ids: Vec<String> = std::thread::scope(|scope| {
            let threads: Vec<_> = (0..8)
                .map(|_| {
                    scope.spawn(|| {
                        let lock = lock(&site.state).unwrap();
                        grant(
                            &lock,
                            &site.path,
                            &signer(),
                            &key(),
                            "leaf",
                            now(),
                            &bytes(&request()),
                        )
                        .unwrap()
                    })
                })
                .collect();
            threads
                .into_iter()
                .map(|thread| thread.join().unwrap())
                .collect()
        });
        let listed: BTreeSet<String> = list(&site.path, &key(), now())
            .unwrap()
            .into_iter()
            .map(|grant| grant.id)
            .collect();
        assert_eq!(listed.len(), 8);
        assert_eq!(listed, ids.into_iter().collect::<BTreeSet<_>>());
        assert_eq!(mode(&site.path), 0o600);
    }

    /// Acceptance 13.
    #[test]
    fn a_grant_input_is_validated() {
        let site = site();
        let lock = lock(&site.state).unwrap();
        let add =
            |request: &[u8]| grant(&lock, &site.path, &signer(), &key(), "leaf", now(), request);
        let without = |member: &str| {
            let mut value = request();
            value.as_object_mut().unwrap().remove(member);
            value
        };
        let with = |member: &str, value: Value| {
            let mut request = request();
            request[member] = value;
            request
        };
        for (case, refused) in [
            ("no not_after_unix_ms", without("not_after_unix_ms")),
            (
                "a zero not_after_unix_ms",
                with("not_after_unix_ms", json!(0)),
            ),
            (
                "one before the clock",
                with("not_after_unix_ms", json!(NOW - 1)),
            ),
            (
                "one at the reading's upper bound",
                with("not_after_unix_ms", json!(NOW + 2_000)),
            ),
            (
                "one over the JSON-safe range",
                with("not_after_unix_ms", json!(9_007_199_254_740_992_i64)),
            ),
            ("no max_per_hour", without("max_per_hour")),
            ("a zero max_per_hour", with("max_per_hour", json!(0))),
            ("10,001 an hour", with("max_per_hour", json!(10_001))),
            (
                "an id member",
                with("id", json!("7b0c4a52-55a4-4a5c-9a39-0d6b1c0e7a11")),
            ),
            ("an instance member", with("instance", json!("leaf"))),
            ("input_keys null", with("input_keys", Value::Null)),
            ("no target", with("target", json!([]))),
            (
                "a pointer that is not RFC 6901",
                with("target", json!([{"pointer": "body", "value": 1}])),
            ),
            (
                "an input key below the target",
                with("input_keys", json!(["/body/channel/id"])),
            ),
            ("no operation", without("operation")),
            ("an array", json!([request()])),
            (
                "the members as an array",
                json!(["chat.postMessage", [{"pointer": "/body/channel", "value": "C1"}], ["/body/text"], NOW + 600_000, 2]),
            ),
        ] {
            assert_eq!(
                add(&bytes(&refused)).err(),
                Some(Refused::InvalidInput),
                "{case}"
            );
            assert!(!site.path.exists(), "{case}");
        }
        // A member named twice, and bytes that are not one JSON document.
        let twice =
            String::from_utf8(bytes(&request()))
                .unwrap()
                .replacen('{', r#"{"max_per_hour":1,"#, 1);
        for refused in [twice.as_bytes(), b"".as_slice(), b"{} {}".as_slice()] {
            assert_eq!(add(refused).err(), Some(Refused::InvalidInput));
            assert!(!site.path.exists());
        }
        // The control: the request itself is granted, one millisecond past
        // the reading's upper bound included.
        add(&bytes(&with("not_after_unix_ms", json!(NOW + 2_001)))).unwrap();
        add(&bytes(&request())).unwrap();
        assert_eq!(list(&site.path, &key(), now()).unwrap().len(), 2);
        // The clock reading the commands sign at is a bounded one.
        struct At(ClockInterval);
        impl Clock for At {
            fn now(&self) -> crate::local::mutations::Result<ClockInterval> {
                Ok(self.0)
            }
        }
        assert_eq!(
            reading(&At(now())).map(|t| (t.lower_unix_ms, t.upper_unix_ms)),
            Some((NOW, NOW + 2_000))
        );
        for refused in [
            ClockInterval {
                lower_unix_ms: -1,
                upper_unix_ms: 1,
            },
            ClockInterval {
                lower_unix_ms: NOW,
                upper_unix_ms: NOW - 1,
            },
            ClockInterval {
                lower_unix_ms: NOW,
                upper_unix_ms: NOW + MAX_CLOCK_WIDTH_MS + 1,
            },
        ] {
            assert!(reading(&At(refused)).is_none());
        }
    }
}
