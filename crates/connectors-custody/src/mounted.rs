//! The read-only `mounted_secret` binding (`contracts/auth/custody/v1alpha1/semantics.md`
//! §5.2, docs/local-mounted-secret-custody.md): credentials the platform delivers
//! as files in a read-only mounted directory, such as a Kubernetes Secret volume
//! or a read-only bind mount.
//!
//! It is chosen only by an explicit `[custody]` selection in the owner
//! configuration and is never a fallback. It implements only [`ReadBackend`]:
//! there is no write, confirm or delete, and the compiler refuses one.
//!
//! The read capability compiles against it; each write below does not:
//!
//! ```
//! use connectors_custody::{ReadBackend, mounted::MountedSecret};
//! fn read(binding: &MountedSecret, version: connectors_custody::Version) {
//!     let _ = binding.read(version);
//! }
//! ```
//!
//! ```compile_fail
//! use connectors_custody::{Backend, Material, mounted::MountedSecret};
//! fn write(binding: &MountedSecret, version: connectors_custody::Version) {
//!     let until = std::time::Instant::now();
//!     let _ = binding.write_new(version, &Material::from_slice(b"x"), until, || Ok(()));
//! }
//! ```
//!
//! ```compile_fail
//! use connectors_custody::{Backend, mounted::MountedSecret};
//! fn delete(binding: &MountedSecret, version: connectors_custody::Version) {
//!     let _ = binding.delete(version, std::time::Instant::now(), || Ok(()));
//! }
//! ```
//!
//! ```compile_fail
//! use connectors_custody::{Backend, mounted::MountedSecret};
//! fn confirm(binding: &MountedSecret, version: connectors_custody::Version) {
//!     let _ = binding.confirm(version, std::time::Instant::now(), || Ok(()), |_| Ok(()));
//! }
//! ```
//!
//! An item is named by public, owner-configured coordinates only,
//! `<adapter alias>/<profile>`, and is read under these checks, each on an
//! already opened descriptor (`openat` with `O_NOFOLLOW`):
//!
//! 1. every directory above the configured one is owned by root or the
//!    effective user and is not group- or other-writable;
//! 2. the configured directory is owned by root or the effective user, is on a
//!    read-only mount, and is not group- or other-writable unless that mount is
//!    read-only;
//! 3. in the projected layout, `..data` is a symbolic link owned by root or the
//!    effective user naming one `..`-prefixed sibling, which is a directory
//!    owned by root or the effective user and not group- or other-writable; no
//!    other symbolic link is resolved;
//! 4. the `<alias>` directory is a real directory under the same rule;
//! 5. the item is a regular file with one link and at most [`MAX_BYTES`], with
//!    no write bit and no other bit, owned by the effective user or root; group
//!    read is admitted only on a root-owned file whose group the process holds
//!    (decision-blocker:mounted-secret-custody-trust, Q1);
//! 6. the item's filesystem is mounted read-only;
//! 7. the item is read once from the checked descriptor, bounded.
//!
//! A version pins the bytes it was published with: an HMAC-SHA-256 under a
//! random key held owner-only in the binding's directory in the state
//! directory (Q3). A read whose bytes no longer match answers `Missing`; the
//! connection then needs a revalidation, which pins the new bytes as a new
//! version. Nothing here carries a path, a layout or bytes into an error.
use crate::{
    CustodyBackend, Failure, MAX_BYTES, Material, ReadBackend, Result, Scope, State, Version,
};
use hmac::{Hmac, KeyInit, Mac};
use sha2::Sha256;
use std::{
    ffi::{CString, OsStr},
    fmt,
    fs::File,
    io::{Read, Write},
    os::{
        fd::{AsRawFd, FromRawFd},
        unix::{ffi::OsStrExt, fs::MetadataExt},
    },
    path::{Component, Path, PathBuf},
};
use zeroize::Zeroizing;

#[cfg(any(test, feature = "test-support"))]
pub mod fixture;
#[cfg(test)]
mod tests;

/// The projected layout's indirection: the only symbolic link resolved.
const DATA: &str = "..data";
/// The pin key's file name in the binding's state directory.
const KEY: &str = "pin.key";
const KEY_BYTES: usize = 32;
const PIN_FORMAT: &str = "connectors-mounted-pin/1";
/// A pin record's bound: its format line, the item name and the pin.
const RECORD_BYTES: usize = 1024;

type Pinning = Hmac<Sha256>;

/// Why a check refused; never leaves this module. Every refusal is
/// `Unavailable` to callers, except an absent item, which is `Missing`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Refusal {
    Absent,
    Refused,
}
type Checked<T> = std::result::Result<T, Refusal>;
impl From<Refusal> for Failure {
    fn from(refusal: Refusal) -> Self {
        match refusal {
            Refusal::Absent => Failure::Missing,
            Refusal::Refused => Failure::Unavailable,
        }
    }
}

/// The process identity the checks run against: the effective user, the
/// effective group and the supplementary groups.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Identity {
    uid: u32,
    gid: u32,
    groups: Vec<u32>,
}

impl Identity {
    fn current() -> Result<Self> {
        // SAFETY: geteuid, getegid and a sizing getgroups have no preconditions.
        let (uid, gid, count) = unsafe {
            (
                libc::geteuid(),
                libc::getegid(),
                libc::getgroups(0, std::ptr::null_mut()),
            )
        };
        let count = usize::try_from(count).map_err(|_| Failure::Unavailable)?;
        let mut groups = vec![0; count];
        // SAFETY: the buffer holds `count` entries.
        let written = unsafe { libc::getgroups(count as libc::c_int, groups.as_mut_ptr()) };
        groups.truncate(usize::try_from(written).map_err(|_| Failure::Unavailable)?);
        Ok(Self { uid, gid, groups })
    }

    #[cfg(test)]
    pub(crate) fn of(uid: u32, gid: u32, groups: &[u32]) -> Self {
        Self {
            uid,
            gid,
            groups: groups.to_vec(),
        }
    }

    /// Root or the effective user.
    fn trusted_owner(&self, uid: u32) -> bool {
        uid == 0 || uid == self.uid
    }

    fn holds(&self, gid: u32) -> bool {
        gid == self.gid || self.groups.contains(&gid)
    }
}

/// The public coordinates an item is named by: `<adapter alias>/<profile>`,
/// both selectors (`[A-Za-z0-9._-]{1,128}`, never `.` or `..`). Its `Debug`
/// shows neither.
#[derive(Clone, PartialEq, Eq)]
pub struct Item {
    alias: String,
    profile: String,
}

impl Item {
    pub fn new(alias: &str, profile: &str) -> Result<Self> {
        if !selector(alias) || !selector(profile) {
            return Err(Failure::Denied);
        }
        Ok(Self {
            alias: alias.to_owned(),
            profile: profile.to_owned(),
        })
    }

    fn encoded(&self) -> String {
        format!("{}/{}", self.alias, self.profile)
    }

    fn decode(text: &str) -> Option<Self> {
        let (alias, profile) = text.split_once('/')?;
        Self::new(alias, profile).ok()
    }
}

impl fmt::Debug for Item {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Item(redacted)")
    }
}

fn selector(value: &str) -> bool {
    (1..=128).contains(&value.len())
        && value != "."
        && value != ".."
        && value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"._-".contains(&b))
}

/// The pin of one item's bytes under the binding's key. Its `Debug` shows nothing.
#[derive(Clone, PartialEq, Eq)]
pub struct Pin([u8; 32]);

impl fmt::Debug for Pin {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Pin(redacted)")
    }
}

impl Pin {
    fn matches(&self, other: &Pin) -> bool {
        use subtle::ConstantTimeEq;
        bool::from(self.0.ct_eq(&other.0))
    }
}

/// One checked read of an item: its bytes and their pin.
#[derive(Debug)]
pub struct Reading {
    material: Material,
    pin: Pin,
}

impl Reading {
    pub fn material(&self) -> &Material {
        &self.material
    }
    pub fn into_material(self) -> Material {
        self.material
    }
}

/// The `mounted_secret` binding opened for one scope.
pub struct MountedSecret {
    directory: PathBuf,
    pins: PathBuf,
    scope: Scope,
    identity: Identity,
}

impl fmt::Debug for MountedSecret {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("MountedSecret(redacted)")
    }
}

impl MountedSecret {
    /// Open the binding for one scope. `directory` is the configured mounted
    /// directory; `pins` is the binding's own owner-only directory in the
    /// state directory, created when absent. A directory that fails its
    /// checks or is not on a read-only mount is `Unavailable`.
    pub fn open(scope: Scope, directory: &Path, pins: &Path) -> Result<Self> {
        Self::open_as(scope, directory, pins, Identity::current()?)
    }

    pub(crate) fn open_as(
        scope: Scope,
        directory: &Path,
        pins: &Path,
        identity: Identity,
    ) -> Result<Self> {
        root(directory, &identity)?;
        pin_directory(pins, true)?;
        Ok(Self {
            directory: directory.to_owned(),
            pins: pins.to_owned(),
            scope,
            identity,
        })
    }

    /// Read `item` once under every check and pin its bytes. An absent item is
    /// `Missing`; one that fails a check is `Unavailable`.
    pub fn read_item(&self, item: &Item) -> Result<Reading> {
        let material = read_item(&self.directory, item, &self.identity)?;
        let key = key(&self.pins)?;
        let pin = pin(&key, material.as_bytes())?;
        Ok(Reading { material, pin })
    }

    /// Record that `version` names `item` with the bytes `reading` pinned. The
    /// coordinator's guard runs first, and its refusal is returned with nothing
    /// recorded. The record is written once: the identical record again is
    /// accepted, another one is `Conflict`. Nothing is written to the mounted
    /// directory.
    pub fn pin(
        &self,
        version: Version,
        item: &Item,
        reading: &Reading,
        guard: impl FnOnce() -> Result<()>,
    ) -> Result<()> {
        self.admit(version)?;
        guard()?;
        let record = format!(
            "{PIN_FORMAT}\n{}\n{}\n",
            item.encoded(),
            hex::encode(reading.pin.0)
        );
        let directory = pin_directory(&self.pins, false)?;
        publish(&directory, &record_name(version), record.as_bytes())
    }

    /// Forget a retired version's pin record. The mounted item is not touched,
    /// and an absent record is already released.
    pub fn release(&self, version: Version) -> Result<()> {
        if version.scope() != self.scope {
            return Err(Failure::Denied);
        }
        let directory = pin_directory(&self.pins, false)?;
        let name = CString::new(record_name(version)).map_err(|_| Failure::Unavailable)?;
        // SAFETY: directory and name stay live for the call.
        let removed = unsafe { libc::unlinkat(directory.as_raw_fd(), name.as_ptr(), 0) };
        if removed != 0 && std::io::Error::last_os_error().raw_os_error() != Some(libc::ENOENT) {
            return Err(Failure::Unavailable);
        }
        directory.sync_all().map_err(|_| Failure::Unavailable)
    }

    fn record(&self, version: Version) -> Result<(Item, Pin)> {
        let directory = pin_directory(&self.pins, false)?;
        let file = match open_at(
            &directory,
            OsStr::new(&record_name(version)),
            libc::O_RDONLY,
        ) {
            Ok(file) => file,
            Err(Refusal::Absent) => return Err(Failure::Missing),
            Err(Refusal::Refused) => return Err(Failure::Unavailable),
        };
        owner_only_file(&file, &Identity::current()?)?;
        let bytes = bounded(file, RECORD_BYTES)?;
        let text = std::str::from_utf8(&bytes).map_err(|_| Failure::Unavailable)?;
        let mut lines = text.strip_suffix('\n').unwrap_or_default().split('\n');
        let (Some(PIN_FORMAT), Some(item), Some(pin), None) =
            (lines.next(), lines.next(), lines.next(), lines.next())
        else {
            return Err(Failure::Unavailable);
        };
        let item = Item::decode(item).ok_or(Failure::Unavailable)?;
        let pin: [u8; 32] = hex::decode(pin)
            .ok()
            .and_then(|bytes| bytes.try_into().ok())
            .ok_or(Failure::Unavailable)?;
        Ok((item, Pin(pin)))
    }
}

impl ReadBackend for MountedSecret {
    fn kind(&self) -> CustodyBackend {
        CustodyBackend::MountedSecret
    }

    fn scope(&self) -> Scope {
        self.scope
    }

    fn state(&self) -> State {
        inspect_as(&self.directory, &self.identity)
    }

    fn admit(&self, version: Version) -> Result<()> {
        if version.scope() != self.scope {
            return Err(Failure::Denied);
        }
        root(&self.directory, &self.identity).map(|_| ())
    }

    /// The pinned item's current bytes, only while they are the pinned ones: a
    /// version without a record, an absent item and changed bytes are all
    /// `Missing`; an item that fails a check is `Unavailable`.
    fn read(&self, version: Version) -> Result<Material> {
        self.admit(version)?;
        let (item, pinned) = self.record(version)?;
        let reading = self.read_item(&item)?;
        if !reading.pin.matches(&pinned) {
            return Err(Failure::Missing);
        }
        Ok(reading.material)
    }
}

/// Read `item` from the configured `directory` once under every check and pin
/// its bytes with the key in `pins`, before any version names them: what a
/// connection is validated with before it is published.
pub fn read(directory: &Path, pins: &Path, item: &Item) -> Result<Reading> {
    let identity = Identity::current()?;
    pin_directory(pins, true)?;
    let material = read_item(directory, item, &identity)?;
    let pin = pin(&key(pins)?, material.as_bytes())?;
    Ok(Reading { material, pin })
}

/// Read `item` from the configured `directory` once under checks 1 to 7 and
/// pin nothing: the bytes the platform delivers now. A served instance reads
/// its credential this way at each request, so it follows the platform's
/// rotation; it has no connection whose validated version a pin would hold
/// (docs/running-services.md). Every refusal is the same `Unavailable` (an
/// absent item `Missing`) as [`MountedSecret::read_item`]'s, and carries no
/// path, layout or bytes.
pub fn read_current(directory: &Path, item: &Item) -> Result<Material> {
    read_item(directory, item, &Identity::current()?)
}

/// Passive state of a configured mounted directory: `Available` only when it
/// passes its checks on a read-only mount. Reads no item.
pub fn inspect(directory: &Path) -> State {
    match Identity::current() {
        Ok(identity) => inspect_as(directory, &identity),
        Err(_) => State::Unavailable,
    }
}

fn inspect_as(directory: &Path, identity: &Identity) -> State {
    match root(directory, identity) {
        Ok(_) => State::Available,
        Err(_) => State::Unavailable,
    }
}

/// Whether the configured mounted directory is usable for custody.
pub fn available(directory: &Path) -> bool {
    inspect(directory) == State::Available
}

fn record_name(version: Version) -> String {
    format!("{}.pin", version.item_name().replace('/', "_"))
}

fn pin(key: &[u8], bytes: &[u8]) -> Result<Pin> {
    let mut mac = Pinning::new_from_slice(key).map_err(|_| Failure::Unavailable)?;
    mac.update(bytes);
    let bytes = mac.finalize().into_bytes();
    let mut pinned = [0u8; 32];
    pinned.copy_from_slice(&bytes);
    Ok(Pin(pinned))
}

/// The binding's pin key, created once with random bytes. Two creators race
/// through an exclusive rename, so both use the same key.
fn key(pins: &Path) -> Result<Zeroizing<Vec<u8>>> {
    let directory = pin_directory(pins, false)?;
    loop {
        match open_at(&directory, OsStr::new(KEY), libc::O_RDONLY) {
            Ok(file) => {
                owner_only_file(&file, &Identity::current()?)?;
                let key = bounded(file, KEY_BYTES)?;
                if key.len() != KEY_BYTES {
                    return Err(Failure::Unavailable);
                }
                return Ok(key);
            }
            Err(Refusal::Refused) => return Err(Failure::Unavailable),
            Err(Refusal::Absent) => {}
        }
        let mut fresh = Zeroizing::new(vec![0u8; KEY_BYTES]);
        // SAFETY: the buffer holds KEY_BYTES writable bytes.
        let filled = unsafe { libc::getrandom(fresh.as_mut_ptr().cast(), KEY_BYTES, 0) };
        if usize::try_from(filled).ok() != Some(KEY_BYTES) {
            return Err(Failure::Unavailable);
        }
        match publish(&directory, KEY, &fresh) {
            Ok(()) | Err(Failure::Conflict) => {}
            Err(failure) => return Err(failure),
        }
    }
}

/// Write `bytes` as `name` in `directory` exactly once: through a private
/// temporary file renamed without replacement. An existing identical file is
/// accepted; any other existing file is `Conflict`.
fn publish(directory: &File, name: &str, bytes: &[u8]) -> Result<()> {
    let temporary = format!(".{name}.{}", uuid::Uuid::new_v4().simple());
    let temporary_c = CString::new(temporary.as_str()).map_err(|_| Failure::Unavailable)?;
    let name_c = CString::new(name).map_err(|_| Failure::Unavailable)?;
    // SAFETY: directory and name stay live; a new descriptor passes to File.
    let fd = unsafe {
        libc::openat(
            directory.as_raw_fd(),
            temporary_c.as_ptr(),
            libc::O_WRONLY | libc::O_CREAT | libc::O_EXCL | libc::O_NOFOLLOW | libc::O_CLOEXEC,
            0o600 as libc::c_uint,
        )
    };
    if fd < 0 {
        return Err(Failure::Unavailable);
    }
    // SAFETY: openat returned a new, owned descriptor.
    let mut file = unsafe { File::from_raw_fd(fd) };
    let unlink = || {
        // SAFETY: directory and name stay live for the call.
        unsafe { libc::unlinkat(directory.as_raw_fd(), temporary_c.as_ptr(), 0) };
    };
    if file.write_all(bytes).and_then(|_| file.sync_all()).is_err() {
        unlink();
        return Err(Failure::Unavailable);
    }
    drop(file);
    // SAFETY: both names are live C strings relative to the same directory.
    let renamed = unsafe {
        libc::renameat2(
            directory.as_raw_fd(),
            temporary_c.as_ptr(),
            directory.as_raw_fd(),
            name_c.as_ptr(),
            libc::RENAME_NOREPLACE,
        )
    };
    if renamed != 0 {
        let exists = std::io::Error::last_os_error().raw_os_error() == Some(libc::EEXIST);
        unlink();
        if !exists {
            return Err(Failure::Unavailable);
        }
        let existing = match open_at(directory, OsStr::new(name), libc::O_RDONLY) {
            Ok(file) => bounded(file, RECORD_BYTES.max(KEY_BYTES))?,
            Err(_) => return Err(Failure::Unavailable),
        };
        return if existing.as_slice() == bytes {
            Ok(())
        } else {
            Err(Failure::Conflict)
        };
    }
    directory.sync_all().map_err(|_| Failure::Unavailable)
}

/// Read at most `limit` bytes; one more is `Unavailable`.
fn bounded(mut file: File, limit: usize) -> Result<Zeroizing<Vec<u8>>> {
    let mut bytes = Zeroizing::new(Vec::with_capacity(limit + 1));
    let mut buffer = Zeroizing::new(vec![0u8; 4096]);
    loop {
        let remaining = (limit + 1 - bytes.len()).min(buffer.len());
        if remaining == 0 {
            return Err(Failure::Unavailable);
        }
        let count = match file.read(&mut buffer[..remaining]) {
            Ok(count) => count,
            Err(error) if error.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(_) => return Err(Failure::Unavailable),
        };
        if count == 0 {
            return Ok(bytes);
        }
        bytes.extend_from_slice(&buffer[..count]);
        if bytes.len() > limit {
            return Err(Failure::Unavailable);
        }
    }
}

fn name(value: &OsStr) -> Checked<CString> {
    if value.is_empty() || value == "." || value == ".." || value.as_bytes().contains(&b'/') {
        return Err(Refusal::Refused);
    }
    CString::new(value.as_bytes()).map_err(|_| Refusal::Refused)
}

/// `openat` relative to an admitted descriptor, never following a symbolic
/// link in the last component.
fn open_at(parent: &File, child: &OsStr, flags: i32) -> Checked<File> {
    let child = name(child)?;
    // SAFETY: parent and child remain live; successful ownership passes to File.
    let fd = unsafe {
        libc::openat(
            parent.as_raw_fd(),
            child.as_ptr(),
            flags | libc::O_CLOEXEC | libc::O_NOFOLLOW | libc::O_NONBLOCK,
            0,
        )
    };
    if fd < 0 {
        return Err(
            if std::io::Error::last_os_error().raw_os_error() == Some(libc::ENOENT) {
                Refusal::Absent
            } else {
                Refusal::Refused
            },
        );
    }
    // SAFETY: openat returned a new, owned descriptor.
    Ok(unsafe { File::from_raw_fd(fd) })
}

fn read_only(file: &File) -> bool {
    let mut stat = std::mem::MaybeUninit::<libc::statvfs>::uninit();
    // SAFETY: file is a live descriptor and stat is writable storage for one statvfs.
    if unsafe { libc::fstatvfs(file.as_raw_fd(), stat.as_mut_ptr()) } != 0 {
        return false;
    }
    // SAFETY: fstatvfs succeeded and initialized stat.
    let stat = unsafe { stat.assume_init() };
    stat.f_flag & libc::ST_RDONLY != 0
}

/// A directory owned by root or the effective user that nobody else can write.
fn trusted_directory(file: &File, identity: &Identity) -> Checked<()> {
    let stat = file.metadata().map_err(|_| Refusal::Refused)?;
    if !stat.is_dir() || !identity.trusted_owner(stat.uid()) || stat.mode() & 0o022 != 0 {
        return Err(Refusal::Refused);
    }
    Ok(())
}

fn validate_path(path: &Path) -> Checked<()> {
    if !path.is_absolute()
        || path.as_os_str().as_bytes().len() > 4096
        || path
            .components()
            .any(|p| !matches!(p, Component::RootDir | Component::Normal(_)))
    {
        return Err(Refusal::Refused);
    }
    Ok(())
}

/// The configured directory, after checks 1 and 2.
fn root(directory: &Path, identity: &Identity) -> Result<File> {
    let root = configured(directory, identity)?;
    if !read_only(&root) {
        return Err(Failure::Unavailable);
    }
    // A writable bit on a read-only mount admits no writer: the projected
    // layout's world-writable sticky volume root is trusted only through `..data`.
    Ok(root)
}

/// Check 1, and check 2 but for the read-only mount: `directory` opened below
/// directories owned by root or the effective user that nobody else can
/// write, itself a directory owned by root or the effective user.
fn configured(directory: &Path, identity: &Identity) -> Result<File> {
    validate_path(directory)?;
    let parts = directory
        .components()
        .filter_map(|component| match component {
            Component::Normal(part) => Some(part),
            _ => None,
        })
        .collect::<Vec<_>>();
    let (last, above) = parts.split_last().ok_or(Failure::Unavailable)?;
    let mut current = File::open("/").map_err(|_| Failure::Unavailable)?;
    trusted_directory(&current, identity)?;
    for part in above {
        let next = open_at(&current, part, libc::O_RDONLY | libc::O_DIRECTORY)
            .map_err(|_| Failure::Unavailable)?;
        trusted_directory(&next, identity)?;
        current = next;
    }
    let root = open_at(&current, last, libc::O_RDONLY | libc::O_DIRECTORY)
        .map_err(|_| Failure::Unavailable)?;
    let stat = root.metadata().map_err(|_| Failure::Unavailable)?;
    if !stat.is_dir() || !identity.trusted_owner(stat.uid()) {
        return Err(Failure::Unavailable);
    }
    Ok(root)
}

/// The directory items are read from: the `..data` sibling in the projected
/// layout, the configured directory itself in the flat one (check 3).
fn layout(root: File, identity: &Identity) -> Checked<File> {
    match data(&root, identity)? {
        Some(sibling) => Ok(sibling),
        None => Ok(root),
    }
}

/// Check 3: the `..data` sibling of the projected layout, `None` in the flat
/// one (no `..data` at all).
fn data(root: &File, identity: &Identity) -> Checked<Option<File>> {
    let data = name(OsStr::new(DATA))?;
    let mut stat = std::mem::MaybeUninit::<libc::stat>::uninit();
    // SAFETY: root and data are live; stat is writable storage for one stat.
    let found = unsafe {
        libc::fstatat(
            root.as_raw_fd(),
            data.as_ptr(),
            stat.as_mut_ptr(),
            libc::AT_SYMLINK_NOFOLLOW,
        )
    };
    if found != 0 {
        return if std::io::Error::last_os_error().raw_os_error() == Some(libc::ENOENT) {
            Ok(None)
        } else {
            Err(Refusal::Refused)
        };
    }
    // SAFETY: fstatat succeeded and initialized stat.
    let stat = unsafe { stat.assume_init() };
    if stat.st_mode & libc::S_IFMT != libc::S_IFLNK || !identity.trusted_owner(stat.st_uid) {
        return Err(Refusal::Refused);
    }
    let mut target = [0u8; 256];
    // SAFETY: root and data are live; target has the stated capacity.
    let length = unsafe {
        libc::readlinkat(
            root.as_raw_fd(),
            data.as_ptr(),
            target.as_mut_ptr().cast(),
            target.len(),
        )
    };
    let length = usize::try_from(length).map_err(|_| Refusal::Refused)?;
    if length == 0 || length >= target.len() {
        return Err(Refusal::Refused);
    }
    let target = &target[..length];
    // One `..`-prefixed sibling: no separator, not `..` itself.
    if !target.starts_with(b"..") || target == b".." || target.contains(&b'/') {
        return Err(Refusal::Refused);
    }
    let sibling = open_at(
        root,
        OsStr::from_bytes(target),
        libc::O_RDONLY | libc::O_DIRECTORY,
    )
    .map_err(|_| Refusal::Refused)?;
    trusted_directory(&sibling, identity)?;
    Ok(Some(sibling))
}

/// Open the file the platform delivers at `path` for reading, under the
/// directory checks an item is read under, for a file that is configuration
/// rather than a credential (a served host's standing grant file,
/// docs/running-services.md):
///
/// 1. every directory above the file's directory passes check 1;
/// 2. the file's directory is owned by root or the effective user, and is
///    not group- or other-writable unless it is on a read-only mount (the
///    projected layout's sticky volume root);
/// 3. the last path component is either the file itself (the flat layout),
///    or the symbolic link `<name> -> ..data/<name>` owned by root or the
///    effective user that a kubelet writes for a ConfigMap or Secret volume,
///    which is resolved only through check 3's `..data`. No other symbolic
///    link is followed anywhere on the path;
/// 4. `admit` decides on the file's metadata, taken from a descriptor that
///    cannot read, before the file is opened for reading: whatever stands at
///    the name, a FIFO or a device node included, is opened only once that
///    exact inode is admitted.
///
/// Nothing at `path` is `Ok(None)`. Every refusal is `Unavailable` and carries
/// no path, layout or byte. Unlike an item, the file need not be on a
/// read-only mount, and its own mode is `admit`'s to decide.
pub fn open_delivered(
    path: &Path,
    admit: impl FnOnce(&std::fs::Metadata) -> bool,
) -> Result<Option<File>> {
    validate_path(path)?;
    let identity = Identity::current()?;
    let (Some(parent), Some(file_name)) = (path.parent(), path.file_name()) else {
        return Err(Failure::Unavailable);
    };
    let directory = configured(parent, &identity)?;
    let stat = directory.metadata().map_err(|_| Failure::Unavailable)?;
    if stat.mode() & 0o022 != 0 && !read_only(&directory) {
        return Err(Failure::Unavailable);
    }
    let file_c = name(file_name)?;
    let mut found = std::mem::MaybeUninit::<libc::stat>::uninit();
    // SAFETY: directory and file_c are live; found is writable storage for one stat.
    let looked = unsafe {
        libc::fstatat(
            directory.as_raw_fd(),
            file_c.as_ptr(),
            found.as_mut_ptr(),
            libc::AT_SYMLINK_NOFOLLOW,
        )
    };
    if looked != 0 {
        return if std::io::Error::last_os_error().raw_os_error() == Some(libc::ENOENT) {
            Ok(None)
        } else {
            Err(Failure::Unavailable)
        };
    }
    // SAFETY: fstatat succeeded and initialized found.
    let found = unsafe { found.assume_init() };
    let base = if found.st_mode & libc::S_IFMT == libc::S_IFLNK {
        // The projected layout: exactly `..data/<name>`, then `..data` itself.
        let mut expected = format!("{DATA}/").into_bytes();
        expected.extend_from_slice(file_name.as_bytes());
        let mut target = [0u8; 512];
        // SAFETY: directory and file_c are live; target has the stated capacity.
        let length = unsafe {
            libc::readlinkat(
                directory.as_raw_fd(),
                file_c.as_ptr(),
                target.as_mut_ptr().cast(),
                target.len(),
            )
        };
        if !identity.trusted_owner(found.st_uid)
            || usize::try_from(length).ok() != Some(expected.len())
            || target[..expected.len()] != expected[..]
        {
            return Err(Failure::Unavailable);
        }
        data(&directory, &identity)?.ok_or(Failure::Unavailable)?
    } else {
        directory
    };
    let held = match open_at(&base, file_name, libc::O_PATH) {
        Ok(held) => held,
        // Removed since it was found: no file is there now.
        Err(Refusal::Absent) if found.st_mode & libc::S_IFMT != libc::S_IFLNK => {
            return Ok(None);
        }
        Err(_) => return Err(Failure::Unavailable),
    };
    let metadata = held.metadata().map_err(|_| Failure::Unavailable)?;
    if !metadata.is_file() || !admit(&metadata) {
        return Err(Failure::Unavailable);
    }
    reopen_for_reading(&held, &metadata).map(Some)
}

/// What check 5 reads of an opened item.
#[derive(Clone, Copy, Debug)]
struct ItemStat {
    regular: bool,
    links: u64,
    size: u64,
    mode: u32,
    uid: u32,
    gid: u32,
}

/// Check 5: a regular file with one link and at most [`MAX_BYTES`], with no
/// write bit and no other bit, owned by the effective user or root; group read
/// only on a root-owned file whose group the process holds.
fn admits(item: &ItemStat, identity: &Identity) -> Result<()> {
    let group_read = item.mode & 0o040 != 0;
    if !item.regular
        || item.links != 1
        || item.size > MAX_BYTES as u64
        || item.mode & 0o222 != 0
        || item.mode & 0o007 != 0
        || !identity.trusted_owner(item.uid)
        || (group_read && (item.uid != 0 || !identity.holds(item.gid)))
    {
        return Err(Failure::Unavailable);
    }
    Ok(())
}

/// The item's bytes, after checks 3 to 7.
fn read_item(directory: &Path, item: &Item, identity: &Identity) -> Result<Material> {
    let base = layout(root(directory, identity)?, identity)?;
    let alias = open_at(
        &base,
        OsStr::new(&item.alias),
        libc::O_RDONLY | libc::O_DIRECTORY,
    )?;
    trusted_directory(&alias, identity)?;
    // Check 5 runs on a descriptor that cannot read: whatever stands at the
    // item's name, a device node or a FIFO included, is not opened for reading
    // until that exact inode is admitted.
    let held = open_at(&alias, OsStr::new(&item.profile), libc::O_PATH)?;
    let stat = held.metadata().map_err(|_| Failure::Unavailable)?;
    admits(
        &ItemStat {
            regular: stat.is_file(),
            links: stat.nlink(),
            size: stat.len(),
            mode: stat.mode(),
            uid: stat.uid(),
            gid: stat.gid(),
        },
        identity,
    )?;
    if !read_only(&held) {
        return Err(Failure::Unavailable);
    }
    let file = reopen_for_reading(&held, &stat)?;
    let bytes = bounded(file, MAX_BYTES)?;
    if bytes.is_empty() {
        return Err(Failure::Unavailable);
    }
    Ok(Material::from_slice(&bytes))
}

/// Open the inode `held` (an `O_PATH` descriptor already admitted) for
/// reading, through its `/proc/self/fd` entry rather than by name again, so
/// nothing swapped in at the name since is opened. The reopened descriptor
/// must be the same inode the checks ran on.
fn reopen_for_reading(held: &File, admitted: &std::fs::Metadata) -> Result<File> {
    let path = CString::new(format!("/proc/self/fd/{}", held.as_raw_fd()))
        .map_err(|_| Failure::Unavailable)?;
    // SAFETY: path is NUL-terminated and live; ownership of a new descriptor
    // passes to File.
    let fd = unsafe {
        libc::open(
            path.as_ptr(),
            libc::O_RDONLY | libc::O_CLOEXEC | libc::O_NOCTTY | libc::O_NONBLOCK,
        )
    };
    if fd < 0 {
        return Err(Failure::Unavailable);
    }
    // SAFETY: open returned a new, owned descriptor.
    let file = unsafe { File::from_raw_fd(fd) };
    let reopened = file.metadata().map_err(|_| Failure::Unavailable)?;
    if (reopened.dev(), reopened.ino()) != (admitted.dev(), admitted.ino()) || !reopened.is_file() {
        return Err(Failure::Unavailable);
    }
    Ok(file)
}

/// An owner-only regular file with one link, owned by the effective user.
fn owner_only_file(file: &File, identity: &Identity) -> Result<()> {
    let stat = file.metadata().map_err(|_| Failure::Unavailable)?;
    if !stat.is_file()
        || stat.uid() != identity.uid
        || stat.mode() & 0o077 != 0
        || stat.nlink() != 1
    {
        return Err(Failure::Unavailable);
    }
    Ok(())
}

/// The binding's own directory in the state directory: owned by the
/// effective user, mode 0700, below directories owned by root or the
/// effective user that nobody else can write. Created when `create`.
fn pin_directory(path: &Path, create: bool) -> Result<File> {
    validate_path(path)?;
    let identity = Identity::current()?;
    let parts = path
        .components()
        .filter_map(|component| match component {
            Component::Normal(part) => Some(part),
            _ => None,
        })
        .collect::<Vec<_>>();
    let (last, above) = parts.split_last().ok_or(Failure::Unavailable)?;
    let mut current = File::open("/").map_err(|_| Failure::Unavailable)?;
    trusted_directory(&current, &identity)?;
    for part in above {
        let next = open_at(&current, part, libc::O_RDONLY | libc::O_DIRECTORY)
            .map_err(|_| Failure::Unavailable)?;
        trusted_directory(&next, &identity)?;
        current = next;
    }
    if create {
        let child = name(last)?;
        // SAFETY: current and child stay live for the call.
        let made = unsafe { libc::mkdirat(current.as_raw_fd(), child.as_ptr(), 0o700) };
        if made != 0 && std::io::Error::last_os_error().raw_os_error() != Some(libc::EEXIST) {
            return Err(Failure::Unavailable);
        }
        if made == 0 {
            current.sync_all().map_err(|_| Failure::Unavailable)?;
        }
    }
    let directory = open_at(&current, last, libc::O_RDONLY | libc::O_DIRECTORY)
        .map_err(|_| Failure::Unavailable)?;
    let stat = directory.metadata().map_err(|_| Failure::Unavailable)?;
    if !stat.is_dir() || stat.uid() != identity.uid || stat.mode() & 0o077 != 0 {
        return Err(Failure::Unavailable);
    }
    Ok(directory)
}
