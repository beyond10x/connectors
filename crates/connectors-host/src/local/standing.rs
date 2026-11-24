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
/// Every number in the value is an integer in the range of `i64` or `u64`.
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
            if !valid_pointer(&member.pointer)
                || !pointers.insert(member.pointer.as_str())
                || !exact(&member.value)
            {
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

/// Whether every number in `value` is one its reader holds digit for digit:
/// an integer in the range of `i64` or `u64`. `connectors_core::read_json`
/// holds every other number (a fraction, an exponent, `-0`, an integer
/// outside that range) as the nearest `f64`, so two numbers written with
/// different digits can be one value. A grant's target value holds no such
/// number ([`parse`]), and an input that holds one at a target pointer
/// matches no grant ([`Grants::covering`]).
fn exact(value: &Value) -> bool {
    match value {
        Value::Number(number) => number.is_i64() || number.is_u64(),
        Value::Array(elements) => elements.iter().all(exact),
        Value::Object(members) => members.values().all(exact),
        Value::Null | Value::Bool(_) | Value::String(_) => true,
    }
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
    /// member present with its exact value (a number only as an integer in
    /// the range of `i64` or `u64`), nothing in `input` outside the
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
                    // A number the reader holds as an `f64` is no exact
                    // value: it matches nothing, whatever the grant holds.
                    Some(value) if !exact(value) || *value != member.value => {
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
    ///
    /// A store begins in two places only: a served host creates its own when
    /// it starts, and in the local CLI binding [`grant`] creates it, under
    /// the instance's signing key. The local owner never calls this: a store
    /// created where a write is admitted would start the hourly count at
    /// zero for whoever removed the file ([`Uses::open`]).
    pub fn create(directory: &Path) -> Result<Self, UsesUnavailable> {
        create_private(directory)?;
        let connection = Self::writable(directory)?;
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

    /// The store the state directory `directory` already holds, for taking
    /// uses, under the custody check of [`Uses::create`]. Nothing is created
    /// here, neither the file nor its table: a directory without the store,
    /// or with a file that holds no `grant_uses` table, is `UsesUnavailable`.
    /// The local owner admits a write through this, so a removed store
    /// refuses every write under a grant until [`grant`] creates it again.
    pub fn open(directory: &Path) -> Result<Self, UsesUnavailable> {
        let name = std::ffi::OsStr::new(USES_FILE);
        let parent = fs::directory(directory, false, false).map_err(|_| UsesUnavailable)?;
        if !fs::private_file_metadata_if_present_at(&parent, name).map_err(|_| UsesUnavailable)? {
            return Err(UsesUnavailable);
        }
        let connection = Self::writable(directory)?;
        let table: bool = connection
            .query_row(
                "SELECT EXISTS(
                    SELECT 1 FROM sqlite_master WHERE type='table' AND name='grant_uses')",
                [],
                |row| row.get(0),
            )
            .map_err(|_| UsesUnavailable)?;
        if !table {
            return Err(UsesUnavailable);
        }
        Ok(Self {
            connection: std::sync::Mutex::new(connection),
        })
    }

    /// A read-write connection to the store file, which SQLite never creates
    /// here (no `SQLITE_OPEN_CREATE`).
    fn writable(directory: &Path) -> Result<Connection, UsesUnavailable> {
        let connection = Connection::open_with_flags(
            directory.join(USES_FILE),
            OpenFlags::SQLITE_OPEN_READ_WRITE
                | OpenFlags::SQLITE_OPEN_NO_MUTEX
                | OpenFlags::SQLITE_OPEN_NOFOLLOW,
        )
        .map_err(|_| UsesUnavailable)?;
        connection
            .busy_timeout(BUSY_TIMEOUT)
            .map_err(|_| UsesUnavailable)?;
        Ok(connection)
    }

    /// The store of the state directory `directory` for reading, when it
    /// holds one: opened read-only, under the custody check of [`Uses::open`].
    /// `None` when the directory holds no store file; none is ever created
    /// here, so reading for a grant leaves no store behind.
    pub fn existing(directory: &Path) -> Result<Option<Self>, UsesUnavailable> {
        let name = std::ffi::OsStr::new(USES_FILE);
        let parent = fs::directory(directory, false, false).map_err(|_| UsesUnavailable)?;
        if !fs::private_file_metadata_if_present_at(&parent, name).map_err(|_| UsesUnavailable)? {
            return Ok(None);
        }
        let connection = Connection::open_with_flags(
            directory.join(USES_FILE),
            OpenFlags::SQLITE_OPEN_READ_ONLY
                | OpenFlags::SQLITE_OPEN_NO_MUTEX
                | OpenFlags::SQLITE_OPEN_NOFOLLOW,
        )
        .map_err(|_| UsesUnavailable)?;
        connection
            .busy_timeout(BUSY_TIMEOUT)
            .map_err(|_| UsesUnavailable)?;
        Ok(Some(Self {
            connection: std::sync::Mutex::new(connection),
        }))
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

/// Why [`sign`], [`grant`], [`revoke`] or [`list`] refused. The grant file
/// is as it was, except on [`Refused::OutcomeUnknown`]; a [`grant`] refused
/// after it signed may leave the empty use store it created.
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

/// The exclusive lock of the commands that write the grant file, and the
/// state directory it was taken in, whose use store [`grant`] creates.
pub struct Lock {
    _lease: Lease,
    state: std::path::PathBuf,
}

/// The exclusive `flock` on [`LOCK`] in the state directory `state`, held
/// until the value is dropped. A command takes it before the key lease; the
/// executor holds the shared key lease and never takes this lock.
pub fn lock(state: &Path) -> super::approval_keys::Result<Lock> {
    Lease::acquire(state, LOCK, true).map(|lease| Lock {
        _lease: lease,
        state: state.to_owned(),
    })
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

/// Replaces the file at `path` with the signed file `signed` (mode 0600), or
/// creates it.
fn publish(path: &Path, signed: &[u8]) -> Result<(), Refused> {
    fs::validate_path(path).map_err(|_| Refused::InvalidConfiguration)?;
    let (Some(parent), Some(name)) = (path.parent(), path.file_name()) else {
        return Err(Refused::InvalidConfiguration);
    };
    let directory = fs::directory(parent, false, false).map_err(|_| Refused::Unavailable)?;
    fs::publish_replace(&directory, name, signed).map_err(|failure| match failure {
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
///
/// This is where the use store of the lock's state directory begins
/// ([`Uses::create`]): once the file is signed, so only under the instance's
/// signing key, and before it is replaced. A store that is there is left as
/// it is. The owner refuses a write under a grant while the directory holds
/// no store, so after a removal the next grant is what admits writes again,
/// and their count starts at zero. A store that cannot be created or fails
/// its custody rule is `Unavailable`, and the grant file is left as it was.
pub fn grant(
    lock: &Lock,
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
    let signed = sign(signer, key, now, &grants)?;
    Uses::create(&lock.state).map_err(|UsesUnavailable| Refused::Unavailable)?;
    publish(path, &signed)?;
    Ok(id)
}

/// Removes the grant `grant_id` from the signed file at `path` and replaces
/// the file. `NotFound`: the file holds no grant with that id, or the path
/// holds no file. The caller holds `lock` until this returns. The use store
/// is neither read nor created.
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
    publish(path, &sign(signer, key, now, &grants)?)
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

// A keyed write of `invoke` under the signed grant file
// (contracts/cli/v1alpha1/semantics.md, "`invoke` under a grant"). The
// owner's executor calls [`route`], [`admit`], [`still_admitted`] and
// [`named`] and adds nothing of its own to a decision.

/// The path a write takes in the owner, from what the owner received.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Route<P, M> {
    /// A proof came with the write: the per-input path, in every byte as
    /// without grants. Its refusal is final; nothing falls back to a grant.
    PerInput(P),
    /// No proof, an idempotency key and an instance that names its grant
    /// file: the grant path, under that file.
    Grant(M),
    /// No proof and no key, or no grant file: `approval_required`.
    ApprovalRequired,
}

/// [`Route`] for a write with `proof`, the idempotency key `key` and the
/// instance's `standing_grants` member `member`.
pub fn route<P, M>(proof: Option<P>, key: Option<&str>, member: Option<M>) -> Route<P, M> {
    match (proof, key, member) {
        (Some(proof), ..) => Route::PerInput(proof),
        (None, Some(_), Some(member)) => Route::Grant(member),
        (None, ..) => Route::ApprovalRequired,
    }
}

/// The write a grant is checked against: the instance's `instance_id`, the
/// operation and the input as `connectors_core::read_json` reads it.
#[derive(Clone, Copy, Debug)]
pub struct Call<'a> {
    pub instance: &'a str,
    pub operation: &'a str,
    pub input: &'a Value,
}

/// Why a write is not admitted under a grant, or no longer is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NotAdmitted {
    /// No grant of the verified file covers the write, or the path holds no
    /// file: `approval_required`. The reason is logged, never answered.
    Uncovered(Uncovered),
    /// The path holds a file that does not verify: `approval_required`.
    Unverified,
    /// Every covering grant admitted `max_per_hour` writes in the last hour:
    /// `rate_limited` with this number.
    AtCap { retry_after_seconds: u64 },
    /// The grant file or a directory to it fails the custody rule, or the
    /// read failed: `unavailable`.
    FileUnavailable,
    /// The use store failed: `unavailable`.
    UsesUnavailable,
    /// The clock gave no reading: `unavailable`, never `approval_required`.
    ClockUnavailable,
}

impl NotAdmitted {
    /// The fixed event the owner's log names this refusal by. It holds
    /// nothing of the request, the grant or the file.
    pub fn event(self) -> &'static str {
        match self {
            Self::Uncovered(Uncovered::NoGrant) => "standing grant: none for this instance",
            Self::Uncovered(Uncovered::OtherOperation) => "standing grant: another operation",
            Self::Uncovered(Uncovered::MissingMember) => "standing grant: target member missing",
            Self::Uncovered(Uncovered::OtherTarget) => "standing grant: another target value",
            Self::Uncovered(Uncovered::OutsideInput) => "standing grant: input outside input_keys",
            Self::Uncovered(Uncovered::Expired) => "standing grant: expired",
            Self::Uncovered(Uncovered::Revoked) => "standing grant: revoked",
            Self::Unverified => "standing grant: file does not verify",
            Self::AtCap { .. } => "standing grant: at its hourly cap",
            Self::FileUnavailable => "standing grant: file fails the custody rule",
            Self::UsesUnavailable => "standing grant: use store unavailable",
            Self::ClockUnavailable => "standing grant: no clock reading",
        }
    }
}

/// One use taken under a grant by [`admit`]. Dropped before [`Admitted::bind`]
/// it gives the use back, on every exit; once bound, the use stays counted
/// whatever becomes of the write.
#[must_use]
pub struct Admitted<'a> {
    uses: &'a Uses,
    taken: Use,
    bound: std::cell::Cell<bool>,
}

impl Admitted<'_> {
    /// The id of the grant the use was taken under.
    pub fn grant_id(&self) -> &str {
        &self.taken.grant_id
    }

    /// Records the grant with the write's attempt ([`Uses::bind`]). After a
    /// failure the use is still given back when the guard is dropped.
    pub fn bind(&self, attempt_id: Uuid) -> Result<(), UsesUnavailable> {
        self.uses.bind(&self.taken, attempt_id)?;
        self.bound.set(true);
        Ok(())
    }
}

impl Drop for Admitted<'_> {
    fn drop(&mut self) {
        if !self.bound.get() {
            // A failed release leaves the use counting until its hour ends.
            let _ = self.uses.release(&self.taken);
        }
    }
}

impl std::fmt::Debug for Admitted<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Admitted")
            .field("grant_id", &self.taken.grant_id)
            .field("bound", &self.bound.get())
            .finish()
    }
}

/// The grants of the verified file at `path` that cover `call` when the
/// reading `now` ends, in file order.
fn covering_now(
    path: &Path,
    key: &ConfiguredApprovalKey,
    call: Call<'_>,
    now: ClockInterval,
) -> Result<Vec<Grant>, NotAdmitted> {
    let grants = match verify(path, key, now) {
        Ok(Some(grants)) => grants,
        Ok(None) => return Err(NotAdmitted::Uncovered(Uncovered::NoGrant)),
        Err(FileRefusal::Unverified) => return Err(NotAdmitted::Unverified),
        Err(FileRefusal::Unavailable) => return Err(NotAdmitted::FileUnavailable),
    };
    grants
        .covering(call.instance, call.operation, call.input, now.upper_unix_ms)
        .map(|covering| covering.into_iter().cloned().collect())
        .map_err(NotAdmitted::Uncovered)
}

/// Admits `call` under the signed grant file at `path`: reads and verifies
/// the file for `key` at the reading `now` ([`verify`]), finds the grants that
/// cover the write ([`Grants::covering`]) and takes one use in `uses` under
/// the first with room ([`Uses::take`]). Expiry and the hour are read at the
/// upper bound of `now`, a fresh reading of the trusted clock
/// ([`reading`]); `None`, a clock that gave none, is `ClockUnavailable`.
pub fn admit<'a>(
    path: &Path,
    key: &ConfiguredApprovalKey,
    call: Call<'_>,
    now: Option<ClockInterval>,
    uses: &'a Uses,
) -> Result<Admitted<'a>, NotAdmitted> {
    admit_in(path, key, call, now, || Ok(uses))
}

/// [`admit`] with the use store asked for only once a grant covers the
/// write: `uses` is not called for a write no grant covers, which is refused
/// as it is without a store, and its refusal is `UsesUnavailable`. The local
/// owner passes [`Uses::open`], so a state directory without a store answers
/// a covered write `unavailable` and admits none.
pub fn admit_in<'a>(
    path: &Path,
    key: &ConfiguredApprovalKey,
    call: Call<'_>,
    now: Option<ClockInterval>,
    uses: impl FnOnce() -> Result<&'a Uses, UsesUnavailable>,
) -> Result<Admitted<'a>, NotAdmitted> {
    let now = now.ok_or(NotAdmitted::ClockUnavailable)?;
    let covering = covering_now(path, key, call, now)?;
    let uses = uses().map_err(|UsesUnavailable| NotAdmitted::UsesUnavailable)?;
    let covering: Vec<&Grant> = covering.iter().collect();
    match uses.take(call.instance, &covering, now.upper_unix_ms) {
        Ok(Taken::Slot(taken)) => Ok(Admitted {
            uses,
            taken,
            bound: std::cell::Cell::new(false),
        }),
        Ok(Taken::AtCap {
            retry_after_seconds,
        }) => Err(NotAdmitted::AtCap {
            retry_after_seconds,
        }),
        Err(UsesUnavailable) => Err(NotAdmitted::UsesUnavailable),
    }
}

/// The recheck of a write [`admit`] admitted: the file at `path` is read and
/// verified again at the fresh reading `now`, and the grant the use was taken
/// under must still be in it, still cover `call` and not be expired when the
/// reading ends. Another grant that covers the write does not stand in for
/// it. No use is taken or given back.
pub fn still_admitted(
    path: &Path,
    key: &ConfiguredApprovalKey,
    call: Call<'_>,
    now: Option<ClockInterval>,
    admitted: &Admitted<'_>,
) -> Result<(), NotAdmitted> {
    let now = now.ok_or(NotAdmitted::ClockUnavailable)?;
    if covering_now(path, key, call, now)?
        .iter()
        .any(|grant| grant.id == admitted.taken.grant_id)
    {
        Ok(())
    } else {
        Err(NotAdmitted::Uncovered(Uncovered::NoGrant))
    }
}

/// Names the grant in `observation` when a grant admitted the attempt
/// `attempt_id`: the id [`Uses::bind`] recorded with it in the use store of
/// the state directory `state`. The member stays absent for an attempt no
/// grant admitted, when the directory holds no use store (none is created),
/// and when the store cannot be read: naming the grant never fails a write
/// or its replay.
pub fn named(
    observation: &mut crate::local::owner::mutation::MutationObservation,
    attempt_id: Uuid,
    state: &Path,
) {
    if let Ok(Some(uses)) = Uses::existing(state)
        && let Ok(Some(grant_id)) = uses.grant_of(attempt_id)
    {
        observation.standing = Some(crate::local::owner::mutation::Standing { grant_id });
    }
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
        let uses = Uses::create(&state).unwrap();
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
        let uses = Uses::create(&state).unwrap();
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
        drop(Uses::create(&state).unwrap());
        let store = state.join(USES_FILE);
        assert_eq!(std::fs::metadata(&store).unwrap().mode() & 0o777, 0o600);
        std::fs::set_permissions(&store, std::fs::Permissions::from_mode(0o644)).unwrap();
        assert!(Uses::open(&state).is_err());
        assert!(Uses::create(&state).is_err());
    }

    /// `Uses::open` takes the store a directory holds and creates nothing:
    /// without the file it refuses and leaves none, and a file without the
    /// table (an empty one is a database to SQLite) is refused as it is.
    #[cfg(unix)]
    #[test]
    fn open_never_creates_the_use_store_or_its_table() {
        use std::os::unix::fs::PermissionsExt;
        let (_root, state) = state_directory();
        assert!(Uses::open(&state).is_err());
        assert!(Uses::existing(&state).unwrap().is_none());
        assert_eq!(std::fs::read_dir(&state).unwrap().count(), 0);
        let store = state.join(USES_FILE);
        std::fs::write(&store, b"").unwrap();
        std::fs::set_permissions(&store, std::fs::Permissions::from_mode(0o600)).unwrap();
        assert!(Uses::open(&state).is_err());
        assert_eq!(std::fs::metadata(&store).unwrap().len(), 0);
        // `create` is what makes the table, and then `open` takes the store.
        drop(Uses::create(&state).unwrap());
        assert_eq!(Uses::open(&state).unwrap().rows().unwrap(), 0);
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

    /// A site whose state directory holds the use store, as the instance's
    /// first `grant-standing` leaves it: the owner admits writes there.
    fn site() -> Site {
        let site = bare_site();
        drop(Uses::create(&site.state).unwrap());
        site
    }

    /// A site no grant was ever made in: its state directory is empty.
    fn bare_site() -> Site {
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
        /// Admitted under the grant of this id; its use is bound and counts.
        Admitted(String),
        ApprovalRequired,
        RateLimited,
        Unavailable,
    }

    /// A write of `leaf`'s `chat.postMessage` with `input`.
    fn call(input: &Value) -> Call<'_> {
        Call {
            instance: "leaf",
            operation: "chat.postMessage",
            input,
        }
    }

    /// The owner's admission of that write at `now()`, by the code the owner
    /// answers: `standing::admit_in` with the store the state directory
    /// holds, opened as the owner opens it and never created. An admitted
    /// use is bound to a new attempt, so it stays counted after the guard is
    /// dropped.
    fn admit(site: &Site, key: &ConfiguredApprovalKey, input: &Value) -> Outcome {
        let store = std::cell::OnceCell::new();
        let admitted = super::admit_in(&site.path, key, call(input), Some(now()), || {
            Uses::open(&site.state).map(|opened| store.get_or_init(|| opened))
        });
        match admitted {
            Ok(admitted) => {
                admitted.bind(Uuid::new_v4()).unwrap();
                Outcome::Admitted(admitted.grant_id().to_owned())
            }
            Err(NotAdmitted::Uncovered(_) | NotAdmitted::Unverified) => Outcome::ApprovalRequired,
            Err(NotAdmitted::AtCap { .. }) => Outcome::RateLimited,
            Err(
                NotAdmitted::FileUnavailable
                | NotAdmitted::UsesUnavailable
                | NotAdmitted::ClockUnavailable,
            ) => Outcome::Unavailable,
        }
    }

    /// The uses the store holds, given back or not: a refusal adds none.
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

    /// A reading two seconds wide that ends at `upper`.
    fn ending(upper: i64) -> ClockInterval {
        ClockInterval {
            lower_unix_ms: upper - 2_000,
            upper_unix_ms: upper,
        }
    }

    /// The site's grant file holding `grants`, signed by the test key.
    fn granted(site: &Site, grants: &[Grant]) {
        put(
            &site.path,
            &sign(&signer(), &key(), now(), grants).unwrap(),
            0o600,
        );
    }

    /// Acceptance 5.
    #[test]
    fn a_write_outside_the_grant_is_refused_and_counts_no_use() {
        let site = site();
        granted(&site, &[channel_grant("g1", NOW + 600_000)]);
        let uses = Uses::open(&site.state).unwrap();
        let input = write();
        // The control: the granted write is admitted under this file, key
        // and clock, and its use is given back when the guard is dropped.
        let admitted = super::admit(&site.path, &key(), call(&input), Some(now()), &uses).unwrap();
        assert_eq!(admitted.grant_id(), "g1");
        drop(admitted);
        assert_eq!(uses.rows().unwrap(), 1);

        let other_channel = json!({"body": {"channel": "C2", "text": "hello"}});
        let no_channel = json!({"body": {"text": "hello"}});
        let wider = json!({"body": {"channel": "C1", "text": "hello", "mrkdwn": false}});
        let cases = [
            (
                "another instance",
                Call {
                    instance: "other",
                    ..call(&input)
                },
                Uncovered::NoGrant,
            ),
            (
                "another operation",
                Call {
                    operation: "chat.update",
                    ..call(&input)
                },
                Uncovered::OtherOperation,
            ),
            (
                "another channel",
                call(&other_channel),
                Uncovered::OtherTarget,
            ),
            (
                "the channel missing",
                call(&no_channel),
                Uncovered::MissingMember,
            ),
            ("/body/mrkdwn added", call(&wider), Uncovered::OutsideInput),
        ];
        for (case, call, reason) in cases {
            assert_eq!(
                super::admit(&site.path, &key(), call, Some(now()), &uses).err(),
                Some(NotAdmitted::Uncovered(reason)),
                "{case}"
            );
            assert_eq!(uses.rows().unwrap(), 1, "{case}");
        }
        std::fs::remove_file(&site.path).unwrap();
        assert_eq!(
            super::admit(&site.path, &key(), call(&input), Some(now()), &uses).err(),
            Some(NotAdmitted::Uncovered(Uncovered::NoGrant)),
            "no file"
        );
        assert_eq!(uses.rows().unwrap(), 1, "no file");
        // A clock that gave no reading is `unavailable`, with or without a
        // file, and counts none either.
        granted(&site, &[channel_grant("g1", NOW + 600_000)]);
        assert_eq!(
            super::admit(&site.path, &key(), call(&input), None, &uses).err(),
            Some(NotAdmitted::ClockUnavailable)
        );
        assert_eq!(uses.rows().unwrap(), 1);
        // No refusal's log event holds anything of the write.
        assert_eq!(
            NotAdmitted::Uncovered(Uncovered::OtherTarget).event(),
            "standing grant: another target value"
        );
    }

    /// Acceptance 6.
    #[test]
    fn expiry_reads_the_upper_bound_of_the_clock() {
        let site = site();
        let not_after = NOW + 600_000;
        granted(&site, &[channel_grant("g1", not_after)]);
        let uses = Uses::open(&site.state).unwrap();
        let input = write();
        let at = |upper: i64| {
            super::admit(&site.path, &key(), call(&input), Some(ending(upper)), &uses)
                .map(|admitted| admitted.grant_id().to_owned())
        };
        // A reading that ends before `not_after` is admitted.
        assert_eq!(at(not_after - 1), Ok("g1".to_owned()));
        // One that straddles it is refused: its lower bound is before
        // `not_after`, its upper bound is not.
        assert_eq!(
            at(not_after + 1_000),
            Err(NotAdmitted::Uncovered(Uncovered::Expired))
        );
        // One whose upper bound reaches it is refused too.
        assert_eq!(
            at(not_after),
            Err(NotAdmitted::Uncovered(Uncovered::Expired))
        );
        assert_eq!(uses.rows().unwrap(), 1);
    }

    /// Acceptance 7.
    #[test]
    fn a_fresh_key_at_the_cap_is_rate_limited() {
        let site = site();
        // A key and a grant that outlive the hour the test moves through.
        let key = ConfiguredApprovalKey {
            not_after_unix_ms: NOW + 3 * WINDOW_MS,
            ..key()
        };
        let grant = Grant {
            max_per_hour: 1,
            ..channel_grant("g1", NOW + 2 * WINDOW_MS)
        };
        put(
            &site.path,
            &sign(&signer(), &key, now(), &[grant]).unwrap(),
            0o600,
        );
        let uses = Uses::open(&site.state).unwrap();
        let input = write();
        let at = |reading: ClockInterval| {
            super::admit(&site.path, &key, call(&input), Some(reading), &uses).map(|admitted| {
                admitted.bind(Uuid::new_v4()).unwrap();
                admitted.grant_id().to_owned()
            })
        };
        let first = now().upper_unix_ms;
        assert_eq!(at(now()), Ok("g1".to_owned()));
        // The second write of the hour is at the cap.
        let Err(NotAdmitted::AtCap {
            retry_after_seconds,
        }) = at(now())
        else {
            panic!("the second write was not at the cap");
        };
        assert!(
            (1..=3600).contains(&retry_after_seconds),
            "{retry_after_seconds}"
        );
        assert_eq!(retry_after_seconds, 3600);
        // 3,600 s later the first use is still inside the hour.
        assert_eq!(
            at(ending(first + WINDOW_MS)),
            Err(NotAdmitted::AtCap {
                retry_after_seconds: 1
            })
        );
        // 3,600 s and 1 ms later it has left it.
        assert_eq!(at(ending(first + WINDOW_MS + 1)), Ok("g1".to_owned()));
        assert_eq!(uses.rows().unwrap(), 2);
    }

    /// Acceptance 8.
    #[test]
    fn a_use_not_bound_is_released_and_a_bound_use_stays_counted() {
        let site = site();
        let grant = Grant {
            max_per_hour: 1,
            ..channel_grant("g1", NOW + 600_000)
        };
        granted(&site, &[grant]);
        let uses = Uses::open(&site.state).unwrap();
        let input = write();
        let admit = || super::admit(&site.path, &key(), call(&input), Some(now()), &uses);
        // A guard dropped before `bind` gives its use back: the grant's one
        // use of the hour is free again.
        let unbound = admit().unwrap();
        assert!(matches!(admit(), Err(NotAdmitted::AtCap { .. })));
        drop(unbound);
        // A guard that was bound keeps its use counted after the drop, and
        // the store names the grant for the attempt.
        let bound = admit().unwrap();
        let attempt = Uuid::new_v4();
        bound.bind(attempt).unwrap();
        drop(bound);
        assert!(matches!(admit(), Err(NotAdmitted::AtCap { .. })));
        assert_eq!(uses.grant_of(attempt).unwrap().as_deref(), Some("g1"));
        // A bind that fails leaves the use to be given back: the attempt id
        // is taken, so the second bind is refused by the store.
        let site = self::site();
        granted(&site, &[channel_grant("g1", NOW + 600_000)]);
        let uses = Uses::open(&site.state).unwrap();
        let admit = || super::admit(&site.path, &key(), call(&input), Some(now()), &uses);
        let first = admit().unwrap();
        first.bind(attempt).unwrap();
        let second = admit().unwrap();
        assert_eq!(second.bind(attempt), Err(UsesUnavailable));
        drop(second);
        // `max_per_hour` is 2: one bound use, and the failed one given back.
        drop(admit().unwrap());
        let third = admit().unwrap();
        assert!(matches!(admit(), Err(NotAdmitted::AtCap { .. })));
        drop(third);
    }

    /// Acceptance 9.
    #[test]
    fn a_recheck_refuses_a_grant_that_left_the_file_and_takes_no_use() {
        let input = write();
        let signed_g1 = sign(
            &signer(),
            &key(),
            now(),
            &[channel_grant("g1", NOW + 600_000)],
        )
        .unwrap();
        // The control: the recheck passes on the unchanged file, at a later
        // reading, and takes no second use.
        let control = site();
        put(&control.path, &signed_g1, 0o600);
        let uses = Uses::open(&control.state).unwrap();
        let admitted =
            super::admit(&control.path, &key(), call(&input), Some(now()), &uses).unwrap();
        assert_eq!(
            still_admitted(
                &control.path,
                &key(),
                call(&input),
                Some(ending(NOW + 30_000)),
                &admitted
            ),
            Ok(())
        );
        assert_eq!(uses.rows().unwrap(), 1);
        drop(admitted);

        fn unchanged(_: &Site) {}
        // The same grant under another id covers the same write: it does not
        // stand in for the grant the use was taken under.
        fn replaced(site: &Site) {
            granted(site, &[channel_grant("g2", NOW + 600_000)]);
        }
        fn removed(site: &Site) {
            std::fs::remove_file(&site.path).unwrap();
        }
        type Change = fn(&Site);
        let (_, rotated) = other("key-2");
        let wider = json!({"body": {"channel": "C1", "text": "hello", "mrkdwn": false}});
        let cases: [(
            &str,
            Change,
            ConfiguredApprovalKey,
            &Value,
            i64,
            NotAdmitted,
        ); 5] = [
            (
                "the file replaced without the grant",
                replaced,
                key(),
                &input,
                NOW + 30_000,
                NotAdmitted::Uncovered(Uncovered::NoGrant),
            ),
            (
                "the file removed",
                removed,
                key(),
                &input,
                NOW + 30_000,
                NotAdmitted::Uncovered(Uncovered::NoGrant),
            ),
            (
                "the clock past not_after",
                unchanged,
                key(),
                &input,
                NOW + 600_000,
                NotAdmitted::Uncovered(Uncovered::Expired),
            ),
            (
                "the key rotated",
                unchanged,
                rotated,
                &input,
                NOW + 30_000,
                NotAdmitted::Unverified,
            ),
            (
                "the input given one more member",
                unchanged,
                key(),
                &wider,
                NOW + 30_000,
                NotAdmitted::Uncovered(Uncovered::OutsideInput),
            ),
        ];
        for (case, change, configured, rechecked, upper, refusal) in cases {
            let site = site();
            put(&site.path, &signed_g1, 0o600);
            let uses = Uses::open(&site.state).unwrap();
            let admitted =
                super::admit(&site.path, &key(), call(&input), Some(now()), &uses).unwrap();
            change(&site);
            assert_eq!(
                still_admitted(
                    &site.path,
                    &configured,
                    call(rechecked),
                    Some(ending(upper)),
                    &admitted
                ),
                Err(refusal),
                "{case}"
            );
            assert_eq!(uses.rows().unwrap(), 1, "{case}");
            drop(admitted);
        }
        // A clock that gives no reading at the recheck is `unavailable`,
        // never `approval_required`.
        let site = site();
        put(&site.path, &signed_g1, 0o600);
        let uses = Uses::open(&site.state).unwrap();
        let admitted = super::admit(&site.path, &key(), call(&input), Some(now()), &uses).unwrap();
        assert_eq!(
            still_admitted(&site.path, &key(), call(&input), None, &admitted),
            Err(NotAdmitted::ClockUnavailable)
        );
    }

    /// Security review: one signature verifies exactly one file. Every other
    /// spelling of the signed file is refused, the old signature carries no
    /// other document, and no other spelling of a payload or a header the key
    /// did sign is read.
    #[test]
    fn security_sg_one_signature_admits_exactly_one_file() {
        /// One compact JWS of the RFC 8032 test pair over exactly these
        /// header and payload bytes.
        fn raw(header: &[u8], claims: &[u8]) -> Vec<u8> {
            let pair = Ed25519KeyPair::from_seed_unchecked(&hex::decode(SEED).unwrap()).unwrap();
            let input = format!(
                "{}.{}",
                URL_SAFE_NO_PAD.encode(header),
                URL_SAFE_NO_PAD.encode(claims)
            );
            let signature = URL_SAFE_NO_PAD.encode(pair.sign(input.as_bytes()).as_ref());
            format!("{input}.{signature}").into_bytes()
        }
        const ALPHABET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";
        const HEADER: &str =
            r#"{"alg":"Ed25519","kid":"key-1","typ":"b10x.connectors-standing-grants.v1+jws"}"#;

        let grants = [channel_grant("g1", NOW + 600_000)];
        let signed = sign(&signer(), &key(), now(), &grants).unwrap();
        let text = String::from_utf8(signed.clone()).unwrap();
        let parts: Vec<&str> = text.split('.').collect();
        assert_eq!(parts.len(), 3);
        let claims = claims_of(&signed);
        let payload = String::from_utf8(claims.clone()).unwrap();
        // The controls: the helper builds the signed file itself (an Ed25519
        // signature is deterministic), and that file is admitted.
        assert_eq!(URL_SAFE_NO_PAD.decode(parts[0]).unwrap(), HEADER.as_bytes());
        assert_eq!(raw(HEADER.as_bytes(), &claims), signed);
        let control = site();
        put(&control.path, &signed, 0o600);
        assert_eq!(
            admit(&control, &key(), &write()),
            Outcome::Admitted("g1".into())
        );

        // The signed bytes with bytes around them.
        let around = |before: &str, after: &str| format!("{before}{text}{after}").into_bytes();
        // 64 signature bytes are 86 characters, the last one with four unused
        // bits: the same signature with one of those bits set.
        let last = *signed.last().unwrap();
        let at = ALPHABET.iter().position(|c| *c == last).unwrap();
        assert_eq!(at % 16, 0);
        let mut respelled = signed.clone();
        *respelled.last_mut().unwrap() = ALPHABET[at + 1];
        // The old header and signature around a document with one number
        // changed: what extending a grant without the key would be.
        let under_the_old_signature = |member: &str, value: i64| {
            let mut changed: Value = serde_json::from_slice(&claims).unwrap();
            changed["document"]["grants"][0][member] = json!(value);
            format!(
                "{}.{}.{}",
                parts[0],
                URL_SAFE_NO_PAD.encode(connectors_core::canonical(&changed)),
                parts[2]
            )
            .into_bytes()
        };
        assert_eq!(under_the_old_signature("max_per_hour", 2), signed);
        // Another spelling of the payload, signed by the key.
        let spelled = |from: &str, to: &str| {
            assert!(payload.contains(from), "{from}");
            raw(HEADER.as_bytes(), payload.replacen(from, to, 1).as_bytes())
        };
        let document = String::from_utf8(connectors_core::canonical(
            &serde_json::from_slice::<Value>(&claims).unwrap()["document"],
        ))
        .unwrap();
        let named = |members: String| {
            format!(
                r#"{{"aud":"connectors.approval/leaf",{members},"iss":"connectors.local-issuer/leaf"}}"#
            )
        };
        assert_eq!(named(format!(r#""document":{document}"#)), payload);
        let twice = named(format!(r#""document":{document},"document":{document}"#));
        let end = format!(r#""not_after_unix_ms":{},"#, NOW + 600_000);
        let end_with_fraction = format!(r#""not_after_unix_ms":{}.0,"#, NOW + 600_000);
        let after = |tail: &str| format!("{payload}{tail}");
        let reordered =
            r#"{"typ":"b10x.connectors-standing-grants.v1+jws","kid":"key-1","alg":"Ed25519"}"#;
        let kid_twice = r#"{"alg":"Ed25519","kid":"key-1","kid":"key-1","typ":"b10x.connectors-standing-grants.v1+jws"}"#;
        for (case, file) in [
            ("a byte order mark before it", around("\u{feff}", "")),
            ("a space before it", around(" ", "")),
            ("a line feed after it", around("", "\n")),
            ("a NUL after it", around("", "\0")),
            ("an empty fourth part", around("", ".")),
            ("base64 padding after the signature", around("", "==")),
            (
                "the file twice, a line feed between",
                format!("{text}\n{text}").into_bytes(),
            ),
            (
                "the file twice, a dot between",
                format!("{text}.{text}").into_bytes(),
            ),
            (
                "an unused bit set in the signature's last character",
                respelled,
            ),
            (
                "a larger cap under the old signature",
                under_the_old_signature("max_per_hour", 10_000),
            ),
            (
                "a later end under the old signature",
                under_the_old_signature("not_after_unix_ms", NOW + 86_400_000),
            ),
            (
                "a space after the payload",
                raw(HEADER.as_bytes(), after(" ").as_bytes()),
            ),
            (
                "a space before the payload",
                raw(HEADER.as_bytes(), format!(" {payload}").as_bytes()),
            ),
            (
                "a second document after the payload",
                raw(HEADER.as_bytes(), after(&payload).as_bytes()),
            ),
            (
                "document named twice",
                raw(HEADER.as_bytes(), twice.as_bytes()),
            ),
            (
                "max_per_hour spelled 2.0",
                spelled(r#""max_per_hour":2,"#, r#""max_per_hour":2.0,"#),
            ),
            (
                "max_per_hour spelled 2e0",
                spelled(r#""max_per_hour":2,"#, r#""max_per_hour":2e0,"#),
            ),
            (
                "not_after_unix_ms with a fraction",
                spelled(&end, &end_with_fraction),
            ),
            (
                "an escaped letter in a member name",
                spelled(r#""iss":"#, &format!(r#""is{}0073":"#, r"\u")),
            ),
            (
                "the header's members in another order",
                raw(reordered.as_bytes(), &claims),
            ),
            (
                "kid named twice in the header",
                raw(kid_twice.as_bytes(), &claims),
            ),
            (
                "a space after the header",
                raw(format!("{HEADER} ").as_bytes(), &claims),
            ),
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
    }

    /// Security review: nothing self-issues. Only the configured pair under
    /// the configured kid signs a grant file; any other signer is `forbidden`
    /// and the commands write nothing.
    #[test]
    fn security_sg_only_the_configured_pair_signs_a_grant_file() {
        let grants = [channel_grant("g1", NOW + 600_000)];
        // The control: the configured pair signs.
        assert!(sign(&signer(), &key(), now(), &grants).is_ok());
        // Another pair under the configured kid, and the configured pair
        // under another kid.
        let (stranger, _) = other("key-1");
        let renamed =
            Signer::from_seed(Secret(hex::decode(SEED).unwrap()), "key-2".into()).unwrap();
        for (case, holder) in [("another pair", &stranger), ("another kid", &renamed)] {
            assert_eq!(
                sign(holder, &key(), now(), &grants).err(),
                Some(Refused::Forbidden),
                "{case}"
            );
            let site = site();
            let lock = lock(&site.state).unwrap();
            assert_eq!(
                grant(
                    &lock,
                    &site.path,
                    holder,
                    &key(),
                    "leaf",
                    now(),
                    &bytes(&request())
                )
                .err(),
                Some(Refused::Forbidden),
                "{case}"
            );
            assert!(!site.path.exists(), "{case}");
            // A file that is there keeps its bytes.
            granted(&site, &grants);
            let before = std::fs::read(&site.path).unwrap();
            assert_eq!(
                revoke(&lock, &site.path, holder, &key(), now(), "g1").err(),
                Some(Refused::Forbidden),
                "{case}"
            );
            assert_eq!(
                grant(
                    &lock,
                    &site.path,
                    holder,
                    &key(),
                    "leaf",
                    now(),
                    &bytes(&request())
                )
                .err(),
                Some(Refused::Forbidden),
                "{case}"
            );
            assert_eq!(std::fs::read(&site.path).unwrap(), before, "{case}");
        }
    }

    /// Security review: the hourly cap holds on the handle that took the use,
    /// on a second handle beside it (a second host on the state directory),
    /// on a handle opened after both closed (a restart), under a clock set
    /// back, and across every rewrite of the grant file within the hour.
    #[test]
    fn security_sg_the_cap_holds_across_handles_a_clock_set_back_and_a_rewrite() {
        let site = site();
        // A key and a grant that outlive the hour the test moves through.
        let key = ConfiguredApprovalKey {
            not_after_unix_ms: NOW + 3 * WINDOW_MS,
            ..key()
        };
        let one_an_hour = Grant {
            max_per_hour: 1,
            ..channel_grant("g1", NOW + 2 * WINDOW_MS)
        };
        let original = sign(&signer(), &key, now(), std::slice::from_ref(&one_an_hour)).unwrap();
        put(&site.path, &original, 0o600);
        let input = write();
        let at = |uses: &Uses, reading: ClockInterval| {
            super::admit(&site.path, &key, call(&input), Some(reading), uses).map(|admitted| {
                admitted.bind(Uuid::new_v4()).unwrap();
                admitted.grant_id().to_owned()
            })
        };
        let capped =
            |result: Result<String, NotAdmitted>| matches!(result, Err(NotAdmitted::AtCap { .. }));
        // The one write of the hour.
        let first = Uses::open(&site.state).unwrap();
        assert_eq!(at(&first, now()), Ok("g1".to_owned()));
        assert!(capped(at(&first, now())), "the handle that took the use");
        let second = Uses::open(&site.state).unwrap();
        assert!(capped(at(&second, now())), "a second handle beside it");
        drop(first);
        drop(second);
        let restarted = Uses::open(&site.state).unwrap();
        assert!(capped(at(&restarted, now())), "a handle opened afterwards");
        // A clock set back by a millisecond, by two hours and to the start
        // of time widens no window.
        for upper in [NOW + 1_999, NOW - 2 * WINDOW_MS, 2_000] {
            assert!(capped(at(&restarted, ending(upper))), "{upper}");
        }
        // Rewrites of the file within the hour: a grant for another channel
        // is added and revoked, then the first file's bytes are put back.
        let lock = lock(&site.state).unwrap();
        let mut elsewhere = request();
        elsewhere["target"] = json!([{"pointer": "/body/channel", "value": "C2"}]);
        let added = grant(
            &lock,
            &site.path,
            &signer(),
            &key,
            "leaf",
            now(),
            &bytes(&elsewhere),
        )
        .unwrap();
        assert!(capped(at(&restarted, now())), "after a grant was added");
        revoke(&lock, &site.path, &signer(), &key, now(), &added).unwrap();
        assert!(capped(at(&restarted, now())), "after a grant was revoked");
        put(&site.path, &original, 0o600);
        assert!(
            capped(at(&restarted, now())),
            "after the bytes were put back"
        );
        assert_eq!(restarted.rows().unwrap(), 1);
        // The control: one millisecond past the hour the grant admits again.
        assert_eq!(
            at(&restarted, ending(NOW + 2_000 + WINDOW_MS + 1)),
            Ok("g1".to_owned())
        );
        assert_eq!(restarted.rows().unwrap(), 2);
    }

    /// Security review: the hourly cap is not lost with the use store. The
    /// store file is in the state directory of the host's own UID; removing
    /// it needs no key and no signature and leaves the grant file untouched.
    /// The owner creates no store: from the removal on it answers a write a
    /// grant covers `unavailable` and admits none, whatever `revoke-standing`
    /// and `list-standing` do meanwhile; neither creates a store, and no
    /// refused `grant-standing` does. Only a `grant-standing` the signing key
    /// signed creates it again, and the count then starts at zero.
    #[test]
    fn security_sg_a_removed_use_store_does_not_free_the_hourly_cap() {
        let site = bare_site();
        let store = site.state.join("standing-grants.sqlite3");
        let lock = lock(&site.state).unwrap();
        // A request for one write an hour to `channel`.
        let capped = |channel: &str| {
            let mut request = request();
            request["target"][0]["value"] = json!(channel);
            request["max_per_hour"] = json!(1);
            bytes(&request)
        };
        let grant = |signer: &Signer, request: &[u8]| {
            grant(&lock, &site.path, signer, &key(), "leaf", now(), request)
        };
        // Before the first grant nothing covers the write, and the owner's
        // refusal leaves no store behind.
        assert_eq!(admit(&site, &key(), &write()), Outcome::ApprovalRequired);
        assert!(!store.exists());
        // The first `grant-standing` creates the store.
        let kept = grant(&signer(), &capped("C1")).unwrap();
        let gone = grant(&signer(), &capped("C9")).unwrap();
        assert!(store.exists());
        assert_eq!(mode(&store), 0o600);
        assert_eq!(
            admit(&site, &key(), &write()),
            Outcome::Admitted(kept.clone())
        );
        // The control: with the store in place the next write is at the cap.
        assert_eq!(admit(&site, &key(), &write()), Outcome::RateLimited);
        std::fs::remove_file(&store).unwrap();
        // The next writes of the same hour, as the owner admits them: never
        // admitted, and no store is made for them.
        let refused = |step: &str| {
            assert_eq!(
                admit(&site, &key(), &write()),
                Outcome::Unavailable,
                "{step}"
            );
            assert!(!store.exists(), "{step}");
        };
        refused("after the removal");
        refused("a second time");
        // A write no grant covers is refused as it is with a store.
        let outside = json!({"body": {"channel": "C2", "text": "hello"}});
        assert_eq!(admit(&site, &key(), &outside), Outcome::ApprovalRequired);
        // `list-standing` and `revoke-standing` create nothing.
        assert_eq!(list(&site.path, &key(), now()).unwrap().len(), 2);
        refused("after list-standing");
        revoke(&lock, &site.path, &signer(), &key(), now(), &gone).unwrap();
        refused("after revoke-standing");
        // Neither does a `grant-standing` that is refused: for its request,
        // or because the signer is not the instance's key.
        assert_eq!(grant(&signer(), b"{}").err(), Some(Refused::InvalidInput));
        refused("after a refused request");
        let (stranger, _) = other("key-1");
        assert_eq!(
            grant(&stranger, &capped("C9")).err(),
            Some(Refused::Forbidden)
        );
        refused("after a grant another pair signed");
        // The key holder grants again: the store is created, and writes are
        // admitted again with the count at zero, so the kept grant admits
        // one more write in the hour it had already spent.
        let again = grant(&signer(), &capped("C9")).unwrap();
        assert!(store.exists());
        assert_ne!(again, gone);
        assert_eq!(admit(&site, &key(), &write()), Outcome::Admitted(kept));
        assert_eq!(admit(&site, &key(), &write()), Outcome::RateLimited);
        assert_eq!(uses(&site), 1);
    }

    /// Security review: what bounds an older signed file put back at the path
    /// after a revocation (the limit the contract states). It admits again
    /// without any key; the uses taken before the revocation still count; it
    /// ends at the grant's `not_after_unix_ms` and at a rotation of the key.
    #[test]
    fn security_sg_a_put_back_file_admits_only_until_not_after_or_a_key_rotation() {
        let site = site();
        let lock = lock(&site.state).unwrap();
        let id = grant(
            &lock,
            &site.path,
            &signer(),
            &key(),
            "leaf",
            now(),
            &bytes(&request()),
        )
        .unwrap();
        let kept = std::fs::read(&site.path).unwrap();
        assert_eq!(
            admit(&site, &key(), &write()),
            Outcome::Admitted(id.clone())
        );
        revoke(&lock, &site.path, &signer(), &key(), now(), &id).unwrap();
        assert_eq!(list(&site.path, &key(), now()), Ok(vec![]));
        assert_eq!(admit(&site, &key(), &write()), Outcome::ApprovalRequired);
        // The older signed file is put back.
        put(&site.path, &kept, 0o600);
        // `max_per_hour` is 2 and one use was taken before the revocation:
        // one more write is admitted, the next is at the cap.
        assert_eq!(
            admit(&site, &key(), &write()),
            Outcome::Admitted(id.clone())
        );
        assert_eq!(admit(&site, &key(), &write()), Outcome::RateLimited);
        let uses = Uses::open(&site.state).unwrap();
        let input = write();
        // The first bound: a reading that reaches the grant's end.
        assert_eq!(
            super::admit(
                &site.path,
                &key(),
                call(&input),
                Some(ending(NOW + 600_000)),
                &uses
            )
            .err(),
            Some(NotAdmitted::Uncovered(Uncovered::Expired))
        );
        // The second: the approval key rotated, to another kid or to another
        // pair under the same kid.
        for kid in ["key-2", "key-1"] {
            let (_, rotated) = other(kid);
            assert_eq!(
                super::admit(&site.path, &rotated, call(&input), Some(now()), &uses).err(),
                Some(NotAdmitted::Unverified),
                "{kid}"
            );
        }
        assert_eq!(uses.rows().unwrap(), 2);
    }

    /// Security review: a target member holds "the exact JSON value", and
    /// the reader holds an integer outside `i64` and `u64` and every fraction
    /// as the nearest `f64`: 18446744073709551616 and 18446744073709551617
    /// are one value to it, while the adapter is sent the caller's bytes. So
    /// a target value holds no such number. `grant-standing` refuses a
    /// request that has one, a file that has one is refused whole, and an
    /// input that has one at a target pointer is covered by no grant.
    #[test]
    fn security_sg_a_bound_number_is_matched_digit_for_digit() {
        const LARGEST: &str = "18446744073709551615";
        const INEXACT: [&str; 7] = [
            "18446744073709551616",
            "-9223372036854775809",
            "1.5",
            "1.0",
            "1e2",
            "-0",
            r#"{"ids":[7,2.5]}"#,
        ];
        let site = site();
        let lock = lock(&site.state).unwrap();
        // A grant request whose one target member holds `value` as written.
        let request = |value: &str| {
            format!(
                r#"{{"operation":"chat.postMessage","target":[{{"pointer":"/body/id","value":{value}}}],"not_after_unix_ms":{},"max_per_hour":2}}"#,
                NOW + 600_000
            )
        };
        let add = |value: &str| {
            grant(
                &lock,
                &site.path,
                &signer(),
                &key(),
                "leaf",
                now(),
                request(value).as_bytes(),
            )
        };
        // A value as the reader holds it.
        let read = |text: &str| -> Value { connectors_core::read_json(text.as_bytes()).unwrap() };

        // One: the request is refused as any other invalid one, before the
        // clock, the key or the file are read, and nothing is written.
        for value in INEXACT {
            assert_eq!(
                check_request(request(value).as_bytes(), "leaf").err(),
                Some(Refused::InvalidInput),
                "{value}"
            );
            assert_eq!(add(value).err(), Some(Refused::InvalidInput), "{value}");
            assert!(!site.path.exists(), "{value}");
        }
        // The controls: the ends of the two ranges, and integers below
        // other JSON, are taken.
        for value in ["-9223372036854775808", r#"{"ids":[7,-2]}"#] {
            assert!(check_request(request(value).as_bytes(), "leaf").is_ok());
        }
        let id = add(LARGEST).unwrap();

        // Two: a file that holds such a number is refused whole, bare and
        // signed by the instance's key (as a build before this rule wrote
        // it): it admits nothing, not even under its other grants.
        let signed = std::fs::read(&site.path).unwrap();
        let header = json!({"alg": "Ed25519", "kid": "key-1", "typ": SIGNED_TYPE});
        let holding = |value: &str| {
            let mut claims = read(std::str::from_utf8(&claims_of(&signed)).unwrap());
            let mut second = claims["document"]["grants"][0].clone();
            second["id"] = json!("g2");
            second["target"][0]["value"] = read(value);
            claims["document"]["grants"]
                .as_array_mut()
                .unwrap()
                .push(second);
            claims
        };
        let exact_write = read(&format!(r#"{{"body":{{"id":{LARGEST}}}}}"#));
        // The control: the same file with an exact number verifies and admits.
        let control = holding("7");
        assert!(parse(&bytes(&control["document"])).is_ok());
        put(
            &site.path,
            &compact(&header, &connectors_core::canonical(&control)),
            0o600,
        );
        assert_eq!(list(&site.path, &key(), now()).unwrap().len(), 2);
        assert_eq!(
            admit(&site, &key(), &exact_write),
            Outcome::Admitted(id.clone())
        );
        for value in INEXACT {
            let claims = holding(value);
            assert_eq!(
                parse(&bytes(&claims["document"])).err(),
                Some(Unreadable::Malformed),
                "{value}"
            );
            put(
                &site.path,
                &compact(&header, &connectors_core::canonical(&claims)),
                0o600,
            );
            assert_eq!(
                verify(&site.path, &key(), now()).err(),
                Some(FileRefusal::Unverified),
                "{value}"
            );
            assert_eq!(
                admit(&site, &key(), &exact_write),
                Outcome::ApprovalRequired,
                "{value}"
            );
        }

        // Three: under the grant for the largest exact number, an input
        // whose member is a number the reader does not hold exactly is
        // covered by no grant, the nearest one included.
        put(&site.path, &signed, 0o600);
        for member in [
            "18446744073709551616",
            "18446744073709551617",
            "18446744073709551615.0",
            "1.8446744073709551615e19",
        ] {
            assert_eq!(
                admit(
                    &site,
                    &key(),
                    &read(&format!(r#"{{"body":{{"id":{member}}}}}"#))
                ),
                Outcome::ApprovalRequired,
                "a write whose member is {member} was admitted under a grant for {LARGEST}"
            );
        }
        assert_eq!(admit(&site, &key(), &exact_write), Outcome::Admitted(id));
        assert_eq!(admit(&site, &key(), &exact_write), Outcome::RateLimited);
    }

    /// Security review: fail closed. The custody of the grant file is decided
    /// before its bytes are judged, and a use store that fails its own
    /// custody rule, or is no database, is not opened.
    #[test]
    fn security_sg_custody_is_decided_before_the_bytes_and_a_broken_use_store_is_not_opened() {
        fn store(state: &Path) -> PathBuf {
            state.join("standing-grants.sqlite3")
        }
        fn group_readable(state: &Path) {
            std::fs::set_permissions(store(state), std::fs::Permissions::from_mode(0o640)).unwrap();
        }
        fn second_name(state: &Path) {
            std::fs::hard_link(store(state), state.join("second.sqlite3")).unwrap();
        }
        fn linked(state: &Path) {
            let real = state.join("real.sqlite3");
            std::fs::rename(store(state), &real).unwrap();
            std::os::unix::fs::symlink(&real, store(state)).unwrap();
        }
        fn not_a_database(state: &Path) {
            std::fs::write(store(state), b"not a database ".repeat(64)).unwrap();
        }
        fn directory(state: &Path) {
            std::fs::remove_file(store(state)).unwrap();
            std::fs::create_dir(store(state)).unwrap();
        }

        let signed = sign(
            &signer(),
            &key(),
            now(),
            &[channel_grant("g1", NOW + 600_000)],
        )
        .unwrap();
        let mut over = signed.clone();
        over.resize(FILE_LIMIT + 1, b'A');
        for (case, content) in [
            ("bytes that are no grant file", b"{}".to_vec()),
            ("more than 64 KiB", over),
        ] {
            // The control: at mode 0600 the bytes are read and do not verify.
            let site = site();
            put(&site.path, &content, 0o600);
            assert_eq!(
                verify(&site.path, &key(), now()).err(),
                Some(FileRefusal::Unverified),
                "{case}"
            );
            // Writable by the group, the same bytes are not judged.
            put(&site.path, &content, 0o620);
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
        }

        type Break = fn(&Path);
        let cases: [(&str, Break); 5] = [
            ("readable by the group", group_readable),
            ("a second name for the store", second_name),
            ("a symlink to the store", linked),
            ("not a database", not_a_database),
            ("a directory at the name", directory),
        ];
        for (case, break_it) in cases {
            let site = site();
            put(&site.path, &signed, 0o600);
            // The control: the store as the host made it takes a use.
            assert_eq!(
                admit(&site, &key(), &write()),
                Outcome::Admitted("g1".into()),
                "{case}"
            );
            break_it(&site.state);
            assert!(Uses::open(&site.state).is_err(), "{case}");
        }
    }

    /// Acceptance 10.
    #[test]
    fn a_present_proof_never_falls_back_to_a_grant() {
        let member = Path::new("/etc/connectors/grants/leaf.grants");
        // The control: no proof, a key and the member give the grant path.
        assert_eq!(
            route(None::<&str>, Some("k"), Some(member)),
            Route::Grant(member)
        );
        // A proof gives the per-input path, whatever else is there.
        for (key, grants) in [
            (Some("k"), Some(member)),
            (Some("k"), None),
            (None, Some(member)),
            (None, None),
        ] {
            assert_eq!(route(Some("proof"), key, grants), Route::PerInput("proof"));
        }
        // No proof and no key, or no member: `approval_required`.
        assert_eq!(
            route(None::<&str>, None, Some(member)),
            Route::ApprovalRequired
        );
        assert_eq!(
            route(None::<&str>, Some("k"), None::<&Path>),
            Route::ApprovalRequired
        );
        assert_eq!(
            route(None::<&str>, None, None::<&Path>),
            Route::ApprovalRequired
        );
    }

    // Adversarial cases. Each is named `adversary_sg_…`.

    /// Eight writers at once against a cap of three: on one use store handle
    /// (the threads of one owner) and each on a handle of its own that it
    /// opens in the race (as each write of an owner opens one, and as a second
    /// process on the same state directory does). Never more than three.
    #[test]
    fn adversary_sg_concurrent_writers_never_pass_the_cap() {
        const WRITERS: usize = 8;
        const CAP: usize = 3;
        for shared in [true, false] {
            let site = site();
            let grant = Grant {
                max_per_hour: CAP as u32,
                ..channel_grant("g1", NOW + 600_000)
            };
            granted(&site, &[grant]);
            let input = write();
            let one = shared.then(|| Uses::open(&site.state).unwrap());
            let barrier = std::sync::Barrier::new(WRITERS);
            let outcomes: Vec<Outcome> = std::thread::scope(|scope| {
                let threads: Vec<_> = (0..WRITERS)
                    .map(|_| {
                        scope.spawn(|| {
                            barrier.wait();
                            let own;
                            let uses = match &one {
                                Some(one) => one,
                                None => match Uses::open(&site.state) {
                                    Ok(opened) => {
                                        own = opened;
                                        &own
                                    }
                                    Err(UsesUnavailable) => return Outcome::Unavailable,
                                },
                            };
                            let admitted =
                                super::admit(&site.path, &key(), call(&input), Some(now()), uses);
                            match admitted {
                                Ok(admitted) => match admitted.bind(Uuid::new_v4()) {
                                    Ok(()) => Outcome::Admitted(admitted.grant_id().to_owned()),
                                    Err(UsesUnavailable) => Outcome::Unavailable,
                                },
                                Err(NotAdmitted::AtCap { .. }) => Outcome::RateLimited,
                                Err(NotAdmitted::UsesUnavailable) => Outcome::Unavailable,
                                Err(other) => panic!("{other:?}"),
                            }
                        })
                    })
                    .collect();
                threads
                    .into_iter()
                    .map(|thread| thread.join().unwrap())
                    .collect()
            });
            let count = |wanted: &Outcome| outcomes.iter().filter(|o| *o == wanted).count();
            let admitted = count(&Outcome::Admitted("g1".into()));
            let unavailable = count(&Outcome::Unavailable);
            assert_eq!(
                admitted + count(&Outcome::RateLimited) + unavailable,
                WRITERS,
                "shared {shared}: {outcomes:?}"
            );
            assert!(admitted <= CAP, "shared {shared}: {outcomes:?}");
            if unavailable == 0 {
                assert_eq!(admitted, CAP, "shared {shared}: {outcomes:?}");
            }
            // What the store holds afterwards is what was answered: the cap
            // is reached after exactly the uses that are left, on a new handle.
            let mut total = admitted;
            loop {
                match admit(&site, &key(), &input) {
                    Outcome::Admitted(_) => total += 1,
                    Outcome::RateLimited => break,
                    other => panic!("shared {shared}: {other:?}"),
                }
                assert!(total <= CAP, "shared {shared}: {total}");
            }
            assert_eq!(total, CAP, "shared {shared}");
        }
    }

    /// Four processes at once on one state directory and one grant file, as
    /// two owners or an owner and a second host would be: each is this test
    /// again in a process of its own, creates or opens the use store in the
    /// race and tries four writes. Together they never pass a cap of five.
    #[test]
    fn adversary_sg_processes_on_one_state_directory_never_pass_the_cap() {
        const NAME: &str = "local::standing::signed_tests::adversary_sg_processes_on_one_state_directory_never_pass_the_cap";
        const STATE: &str = "ADVERSARY_SG_STATE";
        const FILE: &str = "ADVERSARY_SG_FILE";
        const GO: &str = "ADVERSARY_SG_GO";
        const PROCESSES: usize = 4;
        const TRIES: usize = 4;
        const CAP: usize = 5;
        let patience = std::time::Duration::from_secs(20);
        let pause = std::time::Duration::from_millis(2);
        let input = write();
        let child = (
            std::env::var_os(STATE),
            std::env::var_os(FILE),
            std::env::var_os(GO),
        );
        if let (Some(state), Some(file), Some(go)) = child {
            // The child: says it is ready, waits for the parent's signal,
            // then races the others for the store and the uses.
            let go = PathBuf::from(go);
            let ready = go.with_extension(format!("ready-{}", std::process::id()));
            std::fs::write(ready, b"").unwrap();
            let started = std::time::Instant::now();
            while !go.exists() {
                assert!(started.elapsed() < patience, "no signal");
                std::thread::sleep(pause);
            }
            let (mut admitted, mut unavailable) = (0, 0);
            match Uses::open(Path::new(&state)) {
                Ok(uses) => {
                    for _ in 0..TRIES {
                        let taken = super::admit(
                            Path::new(&file),
                            &key(),
                            call(&input),
                            Some(now()),
                            &uses,
                        );
                        match taken {
                            Ok(taken) => match taken.bind(Uuid::new_v4()) {
                                Ok(()) => admitted += 1,
                                Err(UsesUnavailable) => unavailable += 1,
                            },
                            Err(NotAdmitted::AtCap { .. }) => {}
                            Err(NotAdmitted::UsesUnavailable) => unavailable += 1,
                            Err(other) => panic!("{other:?}"),
                        }
                    }
                }
                Err(UsesUnavailable) => unavailable = TRIES,
            }
            println!("\nadversary_sg child admitted={admitted} unavailable={unavailable}");
            return;
        }
        let site = site();
        let grant = Grant {
            max_per_hour: CAP as u32,
            ..channel_grant("g1", NOW + 600_000)
        };
        granted(&site, &[grant]);
        let go = site.path.with_extension("go");
        let children: Vec<_> = (0..PROCESSES)
            .map(|_| {
                std::process::Command::new(std::env::current_exe().unwrap())
                    .args(["--exact", NAME, "--nocapture", "--test-threads=1"])
                    .env(STATE, &site.state)
                    .env(FILE, &site.path)
                    .env(GO, &go)
                    .stdout(std::process::Stdio::piped())
                    .stderr(std::process::Stdio::piped())
                    .spawn()
                    .unwrap()
            })
            .collect();
        let ready = || {
            std::fs::read_dir(site.path.parent().unwrap())
                .unwrap()
                .filter(|entry| {
                    let name = entry.as_ref().unwrap().file_name();
                    name.to_string_lossy().starts_with("leaf.ready-")
                })
                .count()
        };
        let started = std::time::Instant::now();
        while ready() < PROCESSES && started.elapsed() < patience {
            std::thread::sleep(pause);
        }
        std::fs::write(&go, b"").unwrap();
        let (mut admitted, mut unavailable) = (0, 0);
        for child in children {
            let output = child.wait_with_output().unwrap();
            assert!(output.status.success(), "{output:?}");
            let text = String::from_utf8_lossy(&output.stdout).into_owned();
            let number = |name: &str| -> usize {
                text.split_whitespace()
                    .find_map(|part| part.strip_prefix(name))
                    .unwrap_or_else(|| panic!("no {name} in {text}"))
                    .parse()
                    .unwrap()
            };
            assert!(text.contains("adversary_sg child "), "{text}");
            admitted += number("admitted=");
            unavailable += number("unavailable=");
        }
        assert_eq!(ready(), PROCESSES);
        assert!(
            admitted <= CAP,
            "{admitted} admitted, {unavailable} unavailable"
        );
        if unavailable == 0 {
            assert_eq!(admitted, CAP);
            assert_eq!(admit(&site, &key(), &input), Outcome::RateLimited);
            // Every admitted write holds one bound use; a refusal holds none.
            assert_eq!(uses(&site) as usize, admitted);
        }
    }

    /// The count is the store's, not the handle's and not the file's: it is
    /// there on a new handle, and `grant-standing` and `revoke-standing` of
    /// another grant, which replace the file, leave it where it was.
    #[test]
    fn adversary_sg_the_count_outlives_the_handle_and_a_rewrite_of_the_file() {
        let site = site();
        let (signer, key) = (signer(), key());
        let lock = lock(&site.state).unwrap();
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
            .unwrap()
        };
        let first = add(&request());
        // `admit` opens a new handle for every write.
        assert_eq!(
            admit(&site, &key, &write()),
            Outcome::Admitted(first.clone())
        );
        assert_eq!(
            admit(&site, &key, &write()),
            Outcome::Admitted(first.clone())
        );
        assert_eq!(admit(&site, &key, &write()), Outcome::RateLimited);
        // A grant for another channel: the file is replaced, the first grant
        // keeps its id and its count, and the new one counts on its own.
        let mut elsewhere = request();
        elsewhere["target"] = json!([{"pointer": "/body/channel", "value": "C2"}]);
        let second = add(&elsewhere);
        assert_eq!(admit(&site, &key, &write()), Outcome::RateLimited);
        let other_channel = json!({"body": {"channel": "C2", "text": "hello"}});
        assert_eq!(
            admit(&site, &key, &other_channel),
            Outcome::Admitted(second.clone())
        );
        revoke(&lock, &site.path, &signer, &key, now(), &second).unwrap();
        assert_eq!(admit(&site, &key, &write()), Outcome::RateLimited);
        assert_eq!(
            admit(&site, &key, &other_channel),
            Outcome::ApprovalRequired
        );
        assert_eq!(uses(&site), 3);
    }

    /// The hour is counted from the upper bound of each reading, whatever the
    /// reading's width, and a clock that steps back frees nothing.
    #[test]
    fn adversary_sg_the_hour_is_counted_from_the_upper_bound_of_each_reading() {
        let site = site();
        let key = ConfiguredApprovalKey {
            not_after_unix_ms: NOW + 3 * WINDOW_MS,
            ..key()
        };
        let grant = Grant {
            max_per_hour: 1,
            ..channel_grant("g1", NOW + 2 * WINDOW_MS)
        };
        put(
            &site.path,
            &sign(&signer(), &key, now(), &[grant]).unwrap(),
            0o600,
        );
        let uses = Uses::open(&site.state).unwrap();
        let input = write();
        let at = |lower_unix_ms: i64, upper_unix_ms: i64| {
            let reading = ClockInterval {
                lower_unix_ms,
                upper_unix_ms,
            };
            super::admit(&site.path, &key, call(&input), Some(reading), &uses).map(|admitted| {
                admitted.bind(Uuid::new_v4()).unwrap();
                admitted.grant_id().to_owned()
            })
        };
        // The first use, at the widest reading the clock rule admits.
        let first = NOW + MAX_CLOCK_WIDTH_MS;
        assert_eq!(at(NOW, first), Ok("g1".to_owned()));
        // An exact reading 3,600 s after that upper bound is inside the hour,
        // though it is 3,608 s after the first reading's lower bound.
        assert_eq!(
            at(first + WINDOW_MS, first + WINDOW_MS),
            Err(NotAdmitted::AtCap {
                retry_after_seconds: 1
            })
        );
        // One millisecond later the use has left it.
        assert_eq!(
            at(first + WINDOW_MS + 1, first + WINDOW_MS + 1),
            Ok("g1".to_owned())
        );
        // The clock steps back an hour: both uses count, none is freed.
        assert_eq!(
            at(NOW, NOW + 2_000),
            Err(NotAdmitted::AtCap {
                retry_after_seconds: 3600
            })
        );
        assert_eq!(uses.rows().unwrap(), 2);
    }

    /// A use is given back once, whatever ends the write before `bind`, and a
    /// use that was given back or bound cannot be moved again.
    #[test]
    fn adversary_sg_a_use_is_given_back_once_whatever_ends_the_write() {
        let site = site();
        let grant = Grant {
            max_per_hour: 1,
            ..channel_grant("g1", NOW + 600_000)
        };
        granted(&site, &[grant]);
        let uses = Uses::open(&site.state).unwrap();
        let input = write();
        let admit = || super::admit(&site.path, &key(), call(&input), Some(now()), &uses);
        // A panic between `admit` and `bind` unwinds through the guard.
        let unwound = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let held = admit().unwrap();
            assert_eq!(held.grant_id(), "no such grant", "the write panics here");
        }));
        assert!(unwound.is_err());
        // The grant's one use of the hour is free again.
        let held = admit().unwrap();
        assert!(matches!(admit(), Err(NotAdmitted::AtCap { .. })));
        // Given back by hand, twice, and once more by the drop: one use is
        // free afterwards, not three. A use given back is not bound.
        let taken = held.taken.clone();
        uses.release(&taken).unwrap();
        uses.release(&taken).unwrap();
        assert_eq!(held.bind(Uuid::new_v4()), Err(UsesUnavailable));
        drop(held);
        let bound = admit().unwrap();
        assert!(matches!(admit(), Err(NotAdmitted::AtCap { .. })));
        // A bound use stays counted when it is released, and stays with its
        // attempt when it is bound again.
        let attempt = Uuid::new_v4();
        bound.bind(attempt).unwrap();
        uses.release(&bound.taken).unwrap();
        assert_eq!(bound.bind(Uuid::new_v4()), Err(UsesUnavailable));
        drop(bound);
        assert!(matches!(admit(), Err(NotAdmitted::AtCap { .. })));
        assert_eq!(uses.grant_of(attempt).unwrap().as_deref(), Some("g1"));
        assert_eq!(uses.rows().unwrap(), 3);
    }

    /// The recheck refuses every file the admission refuses: an unsafe one, one
    /// that does not verify, and one whose grant of the same id no longer
    /// covers the write. It takes no use.
    #[test]
    fn adversary_sg_a_recheck_refuses_every_file_the_admission_refuses() {
        let input = write();
        let signed = sign(
            &signer(),
            &key(),
            now(),
            &[channel_grant("g1", NOW + 600_000)],
        )
        .unwrap();
        /// The file's claims after `change`, signed again by the test key.
        fn resigned(signed: &[u8], change: impl FnOnce(&mut Value)) -> Vec<u8> {
            let mut claims: Value = serde_json::from_slice(&claims_of(signed)).unwrap();
            change(&mut claims);
            let header = json!({"alg": "Ed25519", "kid": "key-1", "typ": SIGNED_TYPE});
            compact(&header, &connectors_core::canonical(&claims))
        }
        // The control of `resigned`: with no change it is the file itself.
        assert_eq!(resigned(&signed, |_| {}), signed);
        fn group_writable(site: &Site, signed: &[u8]) {
            put(&site.path, signed, 0o620);
        }
        fn linked(site: &Site, signed: &[u8]) {
            std::fs::remove_file(&site.path).unwrap();
            let real = site.path.with_extension("real");
            put(&real, signed, 0o600);
            std::os::unix::fs::symlink(&real, &site.path).unwrap();
        }
        fn directory(site: &Site, _: &[u8]) {
            std::fs::remove_file(&site.path).unwrap();
            std::fs::create_dir(&site.path).unwrap();
        }
        fn emptied(site: &Site, _: &[u8]) {
            put(&site.path, b"", 0o600);
        }
        fn cut(site: &Site, signed: &[u8]) {
            put(&site.path, &signed[..signed.len() / 2], 0o600);
        }
        fn oversized(site: &Site, signed: &[u8]) {
            let mut bytes = signed.to_vec();
            bytes.resize(FILE_LIMIT + 1, b'A');
            put(&site.path, &bytes, 0o600);
        }
        fn by_another_key(site: &Site, _: &[u8]) {
            let (signer, key) = other("key-1");
            let grants = [channel_grant("g1", NOW + 600_000)];
            put(
                &site.path,
                &sign(&signer, &key, now(), &grants).unwrap(),
                0o600,
            );
        }
        fn listed_revoked(site: &Site, signed: &[u8]) {
            let file = resigned(signed, |claims| {
                claims["document"]["revoked"] = json!(["g1"]);
            });
            put(&site.path, &file, 0o600);
        }
        /// The grant `g1` with one member changed.
        fn regranted(site: &Site, signed: &[u8], member: &str, value: Value) {
            let file = resigned(signed, |claims| {
                claims["document"]["grants"][0][member] = value;
            });
            put(&site.path, &file, 0o600);
        }
        fn other_channel(site: &Site, signed: &[u8]) {
            let target = json!([{"pointer": "/body/channel", "value": "C2"}]);
            regranted(site, signed, "target", target);
        }
        fn other_instance(site: &Site, signed: &[u8]) {
            regranted(site, signed, "instance", json!("other"));
        }
        fn other_operation(site: &Site, signed: &[u8]) {
            regranted(site, signed, "operation", json!("chat.update"));
        }
        fn ending_sooner(site: &Site, signed: &[u8]) {
            regranted(site, signed, "not_after_unix_ms", json!(NOW + 10_000));
        }
        fn closed(site: &Site, signed: &[u8]) {
            regranted(site, signed, "input_keys", json!([]));
        }
        fn joined(site: &Site, _: &[u8]) {
            granted(
                site,
                &[
                    channel_grant("g0", NOW + 600_000),
                    channel_grant("g1", NOW + 600_000),
                ],
            );
        }
        type Change = fn(&Site, &[u8]);
        let cases: [(&str, Change, Result<(), NotAdmitted>); 14] = [
            (
                "mode 0620",
                group_writable,
                Err(NotAdmitted::FileUnavailable),
            ),
            (
                "a symlink at the path",
                linked,
                Err(NotAdmitted::FileUnavailable),
            ),
            (
                "a directory at the path",
                directory,
                Err(NotAdmitted::FileUnavailable),
            ),
            ("no byte", emptied, Err(NotAdmitted::Unverified)),
            ("half the file", cut, Err(NotAdmitted::Unverified)),
            ("over 64 KiB", oversized, Err(NotAdmitted::Unverified)),
            (
                "another key under the same kid",
                by_another_key,
                Err(NotAdmitted::Unverified),
            ),
            (
                "the grant listed in revoked",
                listed_revoked,
                Err(NotAdmitted::Uncovered(Uncovered::Revoked)),
            ),
            (
                "the same id for another channel",
                other_channel,
                Err(NotAdmitted::Uncovered(Uncovered::OtherTarget)),
            ),
            (
                "the same id for another instance",
                other_instance,
                Err(NotAdmitted::Uncovered(Uncovered::NoGrant)),
            ),
            (
                "the same id for another operation",
                other_operation,
                Err(NotAdmitted::Uncovered(Uncovered::OtherOperation)),
            ),
            (
                "the same id ending sooner",
                ending_sooner,
                Err(NotAdmitted::Uncovered(Uncovered::Expired)),
            ),
            (
                "the same id with its input closed",
                closed,
                Err(NotAdmitted::Uncovered(Uncovered::OutsideInput)),
            ),
            // The control: the grant is still there, behind a new one.
            ("a second grant before it", joined, Ok(())),
        ];
        /// Removes whatever a case left at the path and beside it.
        fn clear(site: &Site) {
            for path in [site.path.clone(), site.path.with_extension("real")] {
                match std::fs::symlink_metadata(&path) {
                    Ok(found) if found.is_dir() => std::fs::remove_dir(&path).unwrap(),
                    Ok(_) => std::fs::remove_file(&path).unwrap(),
                    Err(_) => {}
                }
            }
        }
        // One write, admitted once; every case is a recheck of that write.
        let site = site();
        put(&site.path, &signed, 0o600);
        let uses = Uses::open(&site.state).unwrap();
        let admitted = super::admit(&site.path, &key(), call(&input), Some(now()), &uses).unwrap();
        let recheck = || {
            still_admitted(
                &site.path,
                &key(),
                call(&input),
                Some(ending(NOW + 30_000)),
                &admitted,
            )
        };
        for (case, change, expected) in cases {
            change(&site, &signed);
            assert_eq!(recheck(), expected, "{case}");
            // The control: with the file as it was, the same recheck passes.
            clear(&site);
            put(&site.path, &signed, 0o600);
            assert_eq!(recheck(), Ok(()), "after {case}");
        }
        assert_eq!(uses.rows().unwrap(), 1);
    }

    /// While `grant-standing` and `revoke-standing` replace the file for
    /// another grant, a write under the first grant is admitted and rechecked
    /// every time: no reader meets a file that is half written or missing.
    #[test]
    fn adversary_sg_a_file_being_replaced_is_never_read_half_written() {
        let site = site();
        let key = key();
        let lock = lock(&site.state).unwrap();
        let mut wide = request();
        wide["max_per_hour"] = json!(10_000);
        let first = grant(
            &lock,
            &site.path,
            &signer(),
            &key,
            "leaf",
            now(),
            &bytes(&wide),
        )
        .unwrap();
        let mut elsewhere = request();
        elsewhere["target"] = json!([{"pointer": "/body/channel", "value": "C2"}]);
        let uses = Uses::open(&site.state).unwrap();
        let input = write();
        let done = std::sync::atomic::AtomicBool::new(false);
        let (written, refusals) = std::thread::scope(|scope| {
            let readers: Vec<_> = (0..2)
                .map(|_| {
                    scope.spawn(|| {
                        // One write admitted while the file is being
                        // replaced, then rechecked until the last replacement.
                        let mut refusals: Vec<String> = Vec::new();
                        let admitted =
                            super::admit(&site.path, &key, call(&input), Some(now()), &uses);
                        let admitted = match admitted {
                            Ok(admitted) => admitted,
                            Err(refusal) => return vec![format!("admit: {refusal:?}")],
                        };
                        if admitted.grant_id() != first.as_str() {
                            refusals.push(format!("under {}", admitted.grant_id()));
                        }
                        let mut rounds = 0;
                        while rounds < 50 || !done.load(std::sync::atomic::Ordering::SeqCst) {
                            rounds += 1;
                            let recheck = still_admitted(
                                &site.path,
                                &key,
                                call(&input),
                                Some(now()),
                                &admitted,
                            );
                            if let Err(refusal) = recheck {
                                refusals.push(format!("recheck {rounds}: {refusal:?}"));
                            }
                        }
                        refusals
                    })
                })
                .collect();
            let signer = signer();
            let written: Result<(), Refused> = (0..6).try_for_each(|_| {
                let id = grant(
                    &lock,
                    &site.path,
                    &signer,
                    &key,
                    "leaf",
                    now(),
                    &bytes(&elsewhere),
                )?;
                revoke(&lock, &site.path, &signer, &key, now(), &id)
            });
            done.store(true, std::sync::atomic::Ordering::SeqCst);
            let refusals: Vec<String> = readers
                .into_iter()
                .flat_map(|reader| reader.join().unwrap())
                .collect();
            (written, refusals)
        });
        assert_eq!(written, Ok(()));
        assert_eq!(refusals, Vec::<String>::new());
        assert_eq!(
            list(&site.path, &key, now())
                .unwrap()
                .into_iter()
                .map(|grant| grant.id)
                .collect::<Vec<_>>(),
            [first]
        );
    }

    /// A target member is its exact JSON value at its exact pointer, compared
    /// on the input as the owner reads it (`connectors_core::read_json`).
    #[test]
    fn adversary_sg_a_target_value_is_compared_exactly() {
        fn read(text: &str) -> Value {
            connectors_core::read_json(text.as_bytes()).expect("one JSON document")
        }
        // An open grant of one target member, through the signed file.
        let covers = |pointer: &str, value: Value, input: &str| {
            let site = site();
            let grant = Grant {
                target: vec![TargetMember {
                    pointer: pointer.into(),
                    value,
                }],
                input_keys: None,
                ..channel_grant("g1", NOW + 600_000)
            };
            granted(&site, &[grant]);
            let grants = verify(&site.path, &key(), now()).unwrap().unwrap();
            grants
                .covering("leaf", "chat.postMessage", &read(input), NOW)
                .map(|covering| covering.len())
        };
        let other = Err(Uncovered::OtherTarget);
        let missing = Err(Uncovered::MissingMember);
        for (case, pointer, value, input, expected) in [
            ("1", "/body/n", json!(1), r#"{"body":{"n":1}}"#, Ok(1)),
            ("\"1\"", "/body/n", json!(1), r#"{"body":{"n":"1"}}"#, other),
            ("1.0", "/body/n", json!(1), r#"{"body":{"n":1.0}}"#, other),
            ("1e0", "/body/n", json!(1), r#"{"body":{"n":1e0}}"#, other),
            ("true", "/body/n", json!(1), r#"{"body":{"n":true}}"#, other),
            ("[1]", "/body/n", json!(1), r#"{"body":{"n":[1]}}"#, other),
            ("null", "/body/n", json!(1), r#"{"body":{"n":null}}"#, other),
            ("no member", "/body/n", json!(1), r#"{"body":{}}"#, missing),
            ("N", "/body/n", json!(1), r#"{"body":{"N":1}}"#, missing),
            ("an array", "/body/n", json!(1), r#"{"body":[1]}"#, missing),
            (
                "the string for a flag",
                "/body/flag",
                json!(true),
                r#"{"body":{"flag":"true"}}"#,
                other,
            ),
            (
                "the flag",
                "/body/flag",
                json!(true),
                r#"{"body":{"flag":true}}"#,
                Ok(1),
            ),
            (
                "null for a granted null",
                "/body/thread",
                Value::Null,
                r#"{"body":{"thread":null}}"#,
                Ok(1),
            ),
            (
                "no member for a granted null",
                "/body/thread",
                Value::Null,
                r#"{"body":{}}"#,
                missing,
            ),
            (
                "another case",
                "/body/channel",
                json!("C1"),
                r#"{"body":{"channel":"c1"}}"#,
                other,
            ),
            (
                "a trailing space",
                "/body/channel",
                json!("C1"),
                r#"{"body":{"channel":"C1 "}}"#,
                other,
            ),
            (
                "a trailing NUL",
                "/body/channel",
                json!("C1"),
                r#"{"body":{"channel":"C1\u0000"}}"#,
                other,
            ),
            (
                "the same string, escaped",
                "/body/channel",
                json!("C1"),
                r#"{"body":{"channel":"C1"}}"#,
                Ok(1),
            ),
            (
                "the decomposed form of a composed letter",
                "/body/channel",
                json!("\u{e9}"),
                r#"{"body":{"channel":"é"}}"#,
                other,
            ),
            (
                "a member named a/b",
                "/body/a~1b",
                json!("x"),
                r#"{"body":{"a/b":"x"}}"#,
                Ok(1),
            ),
            (
                "b below a, for a member named a/b",
                "/body/a~1b",
                json!("x"),
                r#"{"body":{"a":{"b":"x"}}}"#,
                missing,
            ),
            (
                "a member named m~n",
                "/body/m~0n",
                json!("x"),
                r#"{"body":{"m~n":"x"}}"#,
                Ok(1),
            ),
            (
                "a member named m~0n, for one named m~n",
                "/body/m~0n",
                json!("x"),
                r#"{"body":{"m~0n":"x"}}"#,
                missing,
            ),
            (
                "the first element",
                "/body/to/0",
                json!("x"),
                r#"{"body":{"to":["x","y"]}}"#,
                Ok(1),
            ),
            (
                "the elements in another order",
                "/body/to/0",
                json!("x"),
                r#"{"body":{"to":["y","x"]}}"#,
                other,
            ),
            (
                "an index with a leading zero",
                "/body/to/00",
                json!("x"),
                r#"{"body":{"to":["x"]}}"#,
                missing,
            ),
        ] {
            assert_eq!(covers(pointer, value, input), expected, "{case}");
        }
        // A member named twice never reaches a grant: the owner's reader
        // refuses the input.
        let twice = br#"{"body":{"channel":"C1","channel":"C2","text":"hello"}}"#;
        assert!(connectors_core::read_json::<Value>(twice).is_err());
    }

    /// `check_request` refuses, without a clock, a key or a file, what `grant`
    /// refuses for the request alone, and takes the smallest and largest
    /// values `grant` takes.
    #[test]
    fn adversary_sg_check_request_refuses_without_a_clock() {
        let with = |member: &str, value: Value| {
            let mut request = request();
            request[member] = value;
            bytes(&request)
        };
        assert_eq!(check_request(&bytes(&request()), "leaf"), Ok(()));
        for (case, refused) in [
            (
                "a zero not_after_unix_ms",
                with("not_after_unix_ms", json!(0)),
            ),
            (
                "a negative not_after_unix_ms",
                with("not_after_unix_ms", json!(-1)),
            ),
            (
                "a not_after_unix_ms past the JSON-safe range",
                with("not_after_unix_ms", json!(TIME_LIMIT + 1)),
            ),
            ("a zero max_per_hour", with("max_per_hour", json!(0))),
            ("max_per_hour 10,001", with("max_per_hour", json!(10_001))),
            ("a fraction of a use", with("max_per_hour", json!(1.5))),
            ("an id member", with("id", json!("g1"))),
            ("no target", with("target", json!([]))),
        ] {
            assert_eq!(
                check_request(&refused, "leaf"),
                Err(Refused::InvalidInput),
                "{case}"
            );
        }
        for (case, taken) in [
            ("not_after_unix_ms 1", with("not_after_unix_ms", json!(1))),
            (
                "the last JSON-safe not_after_unix_ms",
                with("not_after_unix_ms", json!(TIME_LIMIT)),
            ),
            ("max_per_hour 1", with("max_per_hour", json!(1))),
            ("max_per_hour 10,000", with("max_per_hour", json!(10_000))),
        ] {
            assert_eq!(check_request(&taken, "leaf"), Ok(()), "{case}");
        }
        assert_eq!(
            check_request(&bytes(&request()), "not an id"),
            Err(Refused::InvalidInput)
        );
    }

    /// The file is one compact JWS in its one spelling: a byte before or
    /// after it, padding, or a signature whose last character carries other
    /// unused bits does not verify, though the signed bytes are the same.
    #[test]
    fn adversary_sg_the_file_is_one_canonical_jws_and_no_byte_more() {
        let signed = String::from_utf8(
            sign(
                &signer(),
                &key(),
                now(),
                &[channel_grant("g1", NOW + 600_000)],
            )
            .unwrap(),
        )
        .unwrap();
        // 64 signature bytes are 86 characters; the last one carries four
        // unused bits, zero in the canonical spelling.
        let (rest, last) = signed.split_at(signed.len() - 1);
        let alias = match last {
            "A" => "B",
            "Q" => "R",
            "g" => "h",
            "w" => "x",
            other => panic!("the signature ends in {other}"),
        };
        let site = site();
        let uses = Uses::open(&site.state).unwrap();
        let input = write();
        let admit = || {
            super::admit(&site.path, &key(), call(&input), Some(now()), &uses)
                .map(|admitted| admitted.grant_id().to_owned())
        };
        for (case, file) in [
            ("a newline after it", format!("{signed}\n")),
            ("a space before it", format!(" {signed}")),
            ("a NUL after it", format!("{signed}\0")),
            ("padding", format!("{signed}==")),
            ("a fourth part", format!("{signed}.")),
            ("the file twice", format!("{signed}{signed}")),
            ("other unused bits", format!("{rest}{alias}")),
        ] {
            put(&site.path, file.as_bytes(), 0o600);
            assert_eq!(
                verify(&site.path, &key(), now()).err(),
                Some(FileRefusal::Unverified),
                "{case}"
            );
            assert_eq!(admit(), Err(NotAdmitted::Unverified), "{case}");
        }
        assert_eq!(uses.rows().unwrap(), 0);
        // The control: the file as signed, at the same path, is admitted.
        put(&site.path, signed.as_bytes(), 0o600);
        assert_eq!(admit(), Ok("g1".to_owned()));
        assert_eq!(uses.rows().unwrap(), 1);
    }

    /// Bytes in an approval file that are no proof stay on the per-input
    /// path and are refused there. The owner reads only a file of no byte as
    /// no proof (`owner::transport::write_action`).
    #[test]
    fn adversary_sg_an_approval_file_that_is_no_proof_is_refused_as_a_proof() {
        let member = Path::new("leaf.grants");
        let documents: [&[u8]; 8] = [
            b" ",
            b"\n",
            b"\t\r\n",
            b"\0",
            b"{}",
            b"null",
            b"\"\"",
            br#"{"reference":"","evidence":""}"#,
        ];
        for document in documents {
            assert_eq!(
                route(Some(document), Some("k"), Some(member)),
                Route::PerInput(document)
            );
            assert!(
                Evidence::from_document(Secret(document.to_vec())).is_err(),
                "{document:?}"
            );
        }
    }
}
