//! Descriptor-relative Linux path admission. No component follows a symlink.
//! Windows admits paths through the owner SID and DACL of each opened handle
//! (`windows`).
use super::{Failure, Result};
#[cfg(unix)]
use std::{
    ffi::CString,
    io::Write,
    os::{
        fd::{AsRawFd, FromRawFd},
        unix::{ffi::OsStrExt, fs::MetadataExt},
    },
    path::Component,
};
use std::{
    ffi::OsStr,
    fs::File,
    io::Read,
    path::{Path, PathBuf},
};

#[cfg(windows)]
pub(crate) mod windows;
#[cfg(windows)]
pub(crate) use windows::{Sidecar, require_absent, sidecar_at};
#[cfg(windows)]
pub use windows::{
    check_private_file, directory, private_file_at, private_file_metadata_if_present_at, uid,
};

#[cfg(unix)]
pub fn uid() -> u32 {
    // SAFETY: geteuid has no preconditions.
    unsafe { libc::geteuid() }
}

#[cfg(unix)]
fn name(value: &OsStr) -> Result<CString> {
    if value.is_empty() || value == "." || value == ".." || value.as_bytes().contains(&b'/') {
        return Err(Failure::InvalidConfiguration);
    }
    CString::new(value.as_bytes()).map_err(|_| Failure::InvalidConfiguration)
}

#[cfg(unix)]
fn open_at(parent: &File, name: &OsStr, flags: i32, mode: u32) -> std::io::Result<File> {
    let name =
        self::name(name).map_err(|_| std::io::Error::from(std::io::ErrorKind::InvalidInput))?;
    // SAFETY: parent and name remain live; successful ownership passes to File.
    let fd = unsafe {
        libc::openat(
            parent.as_raw_fd(),
            name.as_ptr(),
            flags | libc::O_CLOEXEC | libc::O_NOFOLLOW | libc::O_NONBLOCK,
            mode,
        )
    };
    if fd < 0 {
        return Err(std::io::Error::last_os_error());
    }
    // SAFETY: openat returned a new, owned descriptor.
    Ok(unsafe { File::from_raw_fd(fd) })
}

/// Why [`explained_directory`] refused a directory. Fixed words, never OS text.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Reason {
    /// Owned by neither the host user nor root.
    Owner,
    /// Writable by group or others.
    Writable,
    /// The final directory of a private open is not the host user's with
    /// mode `0700`.
    NotPrivate,
}

/// A directory the admission refused: the configured path or one of its
/// ancestors, and why.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Refused {
    pub directory: PathBuf,
    pub reason: Reason,
}

/// A failed [`explained_directory`]: a refused directory, or any other
/// closed failure.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Refusal {
    Refused(Refused),
    Failure(Failure),
}

impl From<Failure> for Refusal {
    fn from(failure: Failure) -> Self {
        Self::Failure(failure)
    }
}

impl Refusal {
    /// The closed failure this refusal is everywhere but at host start.
    #[must_use]
    pub fn failure(self) -> Failure {
        match self {
            Self::Refused(_) => Failure::InvalidConfiguration,
            Self::Failure(failure) => failure,
        }
    }
}

/// The admission of a directory owned by `owner` with `mode`, for the host
/// user `me`: owned by the host user or root and not group- or
/// other-writable; `private`, also the host user's with no group or other bits.
#[cfg(unix)]
fn refusal(owner: u32, mode: u32, private: bool, me: u32) -> Option<Reason> {
    if owner != me && owner != 0 {
        Some(Reason::Owner)
    } else if mode & 0o022 != 0 {
        Some(Reason::Writable)
    } else if private && (owner != me || mode & 0o077 != 0) {
        Some(Reason::NotPrivate)
    } else {
        None
    }
}

#[cfg(unix)]
fn admitted_directory(file: &File, private: bool, path: &Path) -> std::result::Result<(), Refusal> {
    let stat = file.metadata().map_err(|_| Failure::InvalidConfiguration)?;
    if !stat.is_dir() {
        return Err(Failure::InvalidConfiguration.into());
    }
    match refusal(stat.uid(), stat.mode(), private, uid()) {
        None => Ok(()),
        Some(reason) => Err(Refusal::Refused(Refused {
            directory: path.to_owned(),
            reason,
        })),
    }
}

/// An absolute local path on a drive (`C:\…`) with plain, literal names.
#[cfg(windows)]
pub fn validate_path(path: &Path) -> Result<()> {
    windows::validate_path(path).map(|_| ())
}

#[cfg(unix)]
pub fn validate_path(path: &Path) -> Result<()> {
    if !path.is_absolute()
        || path.as_os_str().as_bytes().len() > 4096
        || path
            .components()
            .any(|p| !matches!(p, Component::RootDir | Component::Normal(_)))
    {
        return Err(Failure::InvalidConfiguration);
    }
    Ok(())
}

/// Open an admitted directory, optionally creating missing components privately.
/// Existing directories are never chmodded or repaired implicitly.
#[cfg(unix)]
pub fn directory(path: &Path, create: bool, private: bool) -> Result<File> {
    explained_directory(path, create, private).map_err(Refusal::failure)
}

/// [`directory`], naming the directory it refused and why.
#[cfg(unix)]
pub fn explained_directory(
    path: &Path,
    create: bool,
    private: bool,
) -> std::result::Result<File, Refusal> {
    validate_path(path)?;
    let mut current = File::open("/").map_err(|_| Failure::InvalidConfiguration)?;
    let mut walked = PathBuf::from("/");
    admitted_directory(&current, false, &walked)?;
    for component in path.components() {
        let Component::Normal(part) = component else {
            continue;
        };
        walked.push(part);
        let next = match open_at(&current, part, libc::O_RDONLY | libc::O_DIRECTORY, 0) {
            Ok(next) => next,
            Err(error) if create && error.kind() == std::io::ErrorKind::NotFound => {
                let part_c = name(part)?;
                // SAFETY: both the directory descriptor and C string are valid.
                let result = unsafe { libc::mkdirat(current.as_raw_fd(), part_c.as_ptr(), 0o700) };
                if result != 0
                    && std::io::Error::last_os_error().kind() != std::io::ErrorKind::AlreadyExists
                {
                    return Err(Failure::InvalidConfiguration.into());
                }
                let next = open_at(&current, part, libc::O_RDONLY | libc::O_DIRECTORY, 0)
                    .map_err(|_| Failure::InvalidConfiguration)?;
                next.sync_all().map_err(|_| Failure::OutcomeUnknown)?;
                current.sync_all().map_err(|_| Failure::OutcomeUnknown)?;
                next
            }
            Err(_) => return Err(Failure::InvalidConfiguration.into()),
        };
        admitted_directory(&next, false, &walked)?;
        current = next;
    }
    admitted_directory(&current, private, &walked)?;
    Ok(current)
}

/// [`directory`]; Windows names no refused directory.
#[cfg(windows)]
pub fn explained_directory(
    path: &Path,
    create: bool,
    private: bool,
) -> std::result::Result<File, Refusal> {
    directory(path, create, private).map_err(Refusal::Failure)
}

#[cfg(unix)]
pub fn private_file_at(parent: &File, child: &OsStr) -> Result<File> {
    let file =
        open_at(parent, child, libc::O_RDONLY, 0).map_err(|_| Failure::InvalidConfiguration)?;
    check_private_file(&file)?;
    Ok(file)
}

/// Check a private regular file without opening its inode. Closing a raw
/// descriptor for a live SQLite database or WAL sidecar would cancel another
/// connection's process-owned POSIX locks, outside SQLite's deferred-close VFS.
pub fn private_file_metadata_at(parent: &File, child: &OsStr) -> Result<()> {
    match private_file_metadata_if_present_at(parent, child)? {
        true => Ok(()),
        false => Err(Failure::InvalidConfiguration),
    }
}

/// The same check in one `fstatat`, reporting absence rather than refusing it.
/// SQLite retires sidecars on its last close; a separate existence probe
/// before this check would refuse one that was retired in between.
#[cfg(unix)]
pub fn private_file_metadata_if_present_at(parent: &File, child: &OsStr) -> Result<bool> {
    let child = name(child)?;
    let mut stat = std::mem::MaybeUninit::<libc::stat>::uninit();
    // SAFETY: the parent descriptor and NUL-terminated child remain live, and
    // fstatat initializes stat on success without following a final symlink.
    let result = unsafe {
        libc::fstatat(
            parent.as_raw_fd(),
            child.as_ptr(),
            stat.as_mut_ptr(),
            libc::AT_SYMLINK_NOFOLLOW,
        )
    };
    if result != 0 {
        if std::io::Error::last_os_error().raw_os_error() == Some(libc::ENOENT) {
            return Ok(false);
        }
        return Err(Failure::InvalidConfiguration);
    }
    // SAFETY: successful fstatat initialized the stat structure.
    let stat = unsafe { stat.assume_init() };
    // A name unlinked while it was being resolved reports its inode with no
    // remaining links: that file is already retired, not a second name.
    if stat.st_nlink == 0 {
        return Ok(false);
    }
    if stat.st_mode & libc::S_IFMT != libc::S_IFREG
        || stat.st_uid != uid()
        || stat.st_mode & 0o077 != 0
        || stat.st_nlink != 1
    {
        return Err(Failure::InvalidConfiguration);
    }
    Ok(true)
}

#[cfg(unix)]
pub fn check_private_file(file: &File) -> Result<()> {
    let stat = file.metadata().map_err(|_| Failure::InvalidConfiguration)?;
    if !stat.is_file() || stat.uid() != uid() || stat.mode() & 0o077 != 0 || stat.nlink() != 1 {
        return Err(Failure::InvalidConfiguration);
    }
    Ok(())
}

pub fn private_file(path: &Path) -> Result<File> {
    validate_path(path)?;
    let parent = directory(
        path.parent().ok_or(Failure::InvalidConfiguration)?,
        false,
        false,
    )?;
    private_file_at(
        &parent,
        path.file_name().ok_or(Failure::InvalidConfiguration)?,
    )
}

pub fn read_bounded(file: File, limit: usize) -> Result<Vec<u8>> {
    if file
        .metadata()
        .map_err(|_| Failure::InvalidConfiguration)?
        .len()
        > limit as u64
    {
        return Err(Failure::InvalidConfiguration);
    }
    let mut data = Vec::new();
    file.take((limit + 1) as u64)
        .read_to_end(&mut data)
        .map_err(|_| Failure::InvalidConfiguration)?;
    if data.len() > limit {
        return Err(Failure::InvalidConfiguration);
    }
    Ok(data)
}

/// [`read_bounded`] for a file that may hold secrets: the bytes are read into
/// one zeroizing buffer allocated to `limit + 1` (the bound and the byte that
/// detects an oversized file) before the first read, so it never reallocates
/// and no earlier block holding part of the file is released unzeroized.
pub fn read_bounded_zeroizing(mut file: File, limit: usize) -> Result<zeroize::Zeroizing<Vec<u8>>> {
    if file
        .metadata()
        .map_err(|_| Failure::InvalidConfiguration)?
        .len()
        > limit as u64
    {
        return Err(Failure::InvalidConfiguration);
    }
    let capacity = limit.checked_add(1).ok_or(Failure::InvalidConfiguration)?;
    // Filled with zeros to its final length up front and read into in place,
    // then shortened: shortening keeps the allocation, and dropping the
    // Zeroizing wipes its whole capacity.
    let mut data = zeroize::Zeroizing::new(vec![0u8; capacity]);
    let mut filled = 0;
    while filled < capacity {
        match file.read(&mut data[filled..]) {
            Ok(0) => break,
            Ok(n) => filled += n,
            Err(error) if error.kind() == std::io::ErrorKind::Interrupted => {}
            Err(_) => return Err(Failure::InvalidConfiguration),
        }
    }
    if filled > limit {
        return Err(Failure::InvalidConfiguration);
    }
    data.truncate(filled);
    Ok(data)
}

/// Make a held directory's entries durable: an fsync on Linux and macOS. On
/// Windows the handle is open for listing only and nothing is flushed: a name
/// is durable through its write-through rename (`windows::publish`) and the
/// metadata database through SQLite's own file syncs.
pub fn sync_directory(directory: &File) -> Result<()> {
    #[cfg(unix)]
    directory.sync_all().map_err(|_| Failure::OutcomeUnknown)?;
    #[cfg(windows)]
    let _ = directory;
    Ok(())
}

/// Inspect a held directory without following the destination or creating it.
#[cfg(unix)]
pub(crate) fn require_absent(parent: &File, child: &OsStr) -> Result<()> {
    let child = name(child)?;
    // SAFETY: fstatat initializes the live stat buffer on success. NOFOLLOW
    // makes even a dangling link an existing destination, never an empty name.
    let mut stat = std::mem::MaybeUninit::<libc::stat>::uninit();
    let result = unsafe {
        libc::fstatat(
            parent.as_raw_fd(),
            child.as_ptr(),
            stat.as_mut_ptr(),
            libc::AT_SYMLINK_NOFOLLOW,
        )
    };
    if result == 0 {
        return Err(Failure::ConfigurationExists);
    }
    if std::io::Error::last_os_error().kind() != std::io::ErrorKind::NotFound {
        return Err(Failure::InvalidConfiguration);
    }
    Ok(())
}

/// Publish one durable private file, atomically refusing any existing name.
pub fn publish_new(parent: &File, child: &OsStr, contents: &[u8]) -> Result<()> {
    publish(parent, child, contents, NO_REPLACE)
}

/// The rename flag that refuses an existing destination name atomically.
#[cfg(target_os = "linux")]
const NO_REPLACE: libc::c_uint = libc::RENAME_NOREPLACE;
#[cfg(target_os = "macos")]
const NO_REPLACE: libc::c_uint = libc::RENAME_EXCL;
/// `MoveFileExW` without `MOVEFILE_REPLACE_EXISTING`.
#[cfg(windows)]
const NO_REPLACE: u32 = 1;

#[cfg(windows)]
fn publish(parent: &File, child: &OsStr, contents: &[u8], flags: u32) -> Result<()> {
    windows::publish(parent, child, contents, flags & NO_REPLACE == 0)
}

#[cfg(target_os = "linux")]
fn rename_at(directory: &File, from: &CString, to: &CString, flags: libc::c_uint) -> libc::c_int {
    // SAFETY: valid directory descriptors and C strings; RENAME_NOREPLACE
    // gives concurrent creators a single winner, including symlink targets.
    // Without it the rename replaces the name itself, never a link target.
    unsafe {
        libc::renameat2(
            directory.as_raw_fd(),
            from.as_ptr(),
            directory.as_raw_fd(),
            to.as_ptr(),
            flags,
        )
    }
}

#[cfg(target_os = "macos")]
fn rename_at(directory: &File, from: &CString, to: &CString, flags: libc::c_uint) -> libc::c_int {
    // SAFETY: valid directory descriptors and C strings; RENAME_EXCL gives
    // concurrent creators a single winner and, like RENAME_NOREPLACE, treats
    // any existing name, a dangling link included, as taken. Without it the
    // rename replaces the name itself, never a link target.
    unsafe {
        libc::renameatx_np(
            directory.as_raw_fd(),
            from.as_ptr(),
            directory.as_raw_fd(),
            to.as_ptr(),
            flags,
        )
    }
}

/// Whether a volume reports the exclusive rename as a valid, set capability.
#[cfg(target_os = "macos")]
fn admits_rename_excl(volume: &libc::vol_capabilities_attr_t) -> bool {
    let interfaces = libc::VOL_CAPABILITIES_INTERFACES;
    let bit = libc::VOL_CAP_INT_RENAME_EXCL;
    volume.valid[interfaces] & bit != 0 && volume.capabilities[interfaces] & bit != 0
}

/// Admit a directory for no-replace publication only on a volume that reports
/// `VOL_CAP_INT_RENAME_EXCL` (APFS). Any other volume is refused the way Linux
/// refuses a filesystem without `RENAME_NOREPLACE`: `InvalidConfiguration`.
/// Volume attributes are read from the volume's root, which must be the same
/// device as the directory.
#[cfg(target_os = "macos")]
fn qualify_rename_excl(directory: &File) -> Result<()> {
    use std::os::unix::fs::OpenOptionsExt;
    let mut volume = std::mem::MaybeUninit::<libc::statfs>::uninit();
    // SAFETY: a live descriptor and a writable statfs buffer.
    if unsafe { libc::fstatfs(directory.as_raw_fd(), volume.as_mut_ptr()) } != 0 {
        return Err(Failure::InvalidConfiguration);
    }
    // SAFETY: successful fstatfs initialized the buffer.
    let volume = unsafe { volume.assume_init() };
    let mount = volume.f_mntonname.map(|c| c as u8);
    let mount =
        std::ffi::CStr::from_bytes_until_nul(&mount).map_err(|_| Failure::InvalidConfiguration)?;
    let root = std::fs::OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_DIRECTORY)
        .open(OsStr::from_bytes(mount.to_bytes()))
        .map_err(|_| Failure::InvalidConfiguration)?;
    let device = |file: &File| file.metadata().map(|stat| stat.dev());
    if !matches!((device(&root), device(directory)), (Ok(a), Ok(b)) if a == b) {
        return Err(Failure::InvalidConfiguration);
    }
    let mut request = libc::attrlist {
        bitmapcount: libc::ATTR_BIT_MAP_COUNT,
        reserved: 0,
        commonattr: 0,
        volattr: libc::ATTR_VOL_INFO | libc::ATTR_VOL_CAPABILITIES,
        dirattr: 0,
        fileattr: 0,
        forkattr: 0,
    };
    // getattrlist(2) packs a u32 total length, then the requested attribute.
    #[repr(C)]
    struct Reply {
        length: u32,
        volume: libc::vol_capabilities_attr_t,
    }
    let mut reply = std::mem::MaybeUninit::<Reply>::zeroed();
    // SAFETY: a live descriptor, a valid request and a writable buffer of the
    // size passed; the kernel writes at most that many bytes.
    let status = unsafe {
        libc::fgetattrlist(
            root.as_raw_fd(),
            (&mut request as *mut libc::attrlist).cast(),
            reply.as_mut_ptr().cast(),
            std::mem::size_of::<Reply>(),
            0,
        )
    };
    if status != 0 {
        return Err(Failure::InvalidConfiguration);
    }
    // SAFETY: the buffer was zero-initialized and every field is plain integers.
    let reply = unsafe { reply.assume_init() };
    if (reply.length as usize) < std::mem::size_of::<Reply>() || !admits_rename_excl(&reply.volume)
    {
        return Err(Failure::InvalidConfiguration);
    }
    Ok(())
}

/// Publish one durable private file, atomically replacing any file at the name:
/// a reader sees the old bytes or the new ones, never a partial write.
pub fn publish_replace(parent: &File, child: &OsStr, contents: &[u8]) -> Result<()> {
    publish(parent, child, contents, 0)
}

/// [`publish_replace`] whose new file has exactly the permission bits `mode`,
/// which must be owner-only (no group or other bit): a replacement keeps the
/// mode of the owner-only file it replaces (story:connectors-do-short-invocation).
/// On Windows the handle's own admission applies and `mode` is not used.
pub fn publish_replace_keeping(
    parent: &File,
    child: &OsStr,
    contents: &[u8],
    mode: u32,
) -> Result<()> {
    if mode & 0o077 != 0 || mode & !0o700 != 0 {
        return Err(Failure::InvalidConfiguration);
    }
    #[cfg(unix)]
    let published = publish_with_mode(parent, child, contents, 0, Some(mode));
    #[cfg(windows)]
    let published = publish(parent, child, contents, 0);
    published
}

#[cfg(unix)]
fn publish(parent: &File, child: &OsStr, contents: &[u8], flags: libc::c_uint) -> Result<()> {
    publish_with_mode(parent, child, contents, flags, None)
}

/// `publish` with the temporary file's mode set to exactly `mode` before any
/// byte is written, when given; else it is created `0o600` under the umask.
#[cfg(unix)]
fn publish_with_mode(
    parent: &File,
    child: &OsStr,
    contents: &[u8],
    flags: libc::c_uint,
    mode: Option<u32>,
) -> Result<()> {
    #[cfg(target_os = "macos")]
    if flags & NO_REPLACE != 0 {
        qualify_rename_excl(parent)?;
    }
    let temporary = format!(".connectors-{}", uuid::Uuid::new_v4());
    let temp_name = OsStr::new(&temporary);
    let mut file = open_at(
        parent,
        temp_name,
        libc::O_WRONLY | libc::O_CREAT | libc::O_EXCL,
        0o600,
    )
    .map_err(|_| Failure::InvalidConfiguration)?;
    if let Some(mode) = mode {
        use std::os::unix::fs::PermissionsExt;
        if file
            .set_permissions(std::fs::Permissions::from_mode(mode))
            .is_err()
        {
            let temporary_c = name(temp_name)?;
            // SAFETY: only this call's unguessable temporary name is removed.
            unsafe {
                libc::unlinkat(parent.as_raw_fd(), temporary_c.as_ptr(), 0);
            }
            return Err(Failure::InvalidConfiguration);
        }
    }
    let temporary_c = name(temp_name)?;
    let child_c = name(child)?;
    let result = (|| {
        file.write_all(contents)
            .map_err(|_| Failure::InvalidConfiguration)?;
        file.sync_all().map_err(|_| Failure::OutcomeUnknown)?;
        let status = rename_at(parent, &temporary_c, &child_c, flags);
        if status != 0 {
            return Err(
                if std::io::Error::last_os_error().kind() == std::io::ErrorKind::AlreadyExists {
                    Failure::ConfigurationExists
                } else {
                    Failure::InvalidConfiguration
                },
            );
        }
        parent.sync_all().map_err(|_| Failure::OutcomeUnknown)
    })();
    // SAFETY: only this call's unguessable temporary name is removed. After
    // successful rename it no longer exists; never remove the published file.
    unsafe {
        libc::unlinkat(parent.as_raw_fd(), temporary_c.as_ptr(), 0);
    }
    result
}

#[cfg(all(test, unix))]
mod refusal_tests {
    use super::*;

    /// The reasons reproduce the admission exactly: a directory owned by
    /// another user, a group- or other-writable one and a final directory
    /// that is not the host user's `0700` are each refused, in that order.
    #[test]
    fn each_refused_directory_has_one_fixed_reason() {
        let me = 1000;
        assert_eq!(refusal(me, 0o755, false, me), None);
        assert_eq!(refusal(0, 0o755, false, me), None);
        assert_eq!(refusal(me, 0o700, true, me), None);
        assert_eq!(refusal(2000, 0o755, false, me), Some(Reason::Owner));
        assert_eq!(refusal(2000, 0o777, true, me), Some(Reason::Owner));
        assert_eq!(refusal(me, 0o775, false, me), Some(Reason::Writable));
        assert_eq!(refusal(0, 0o777, false, me), Some(Reason::Writable));
        assert_eq!(refusal(me, 0o1777, false, me), Some(Reason::Writable));
        assert_eq!(refusal(me, 0o777, true, me), Some(Reason::Writable));
        assert_eq!(refusal(me, 0o750, true, me), Some(Reason::NotPrivate));
        assert_eq!(refusal(0, 0o700, true, me), Some(Reason::NotPrivate));
    }

    /// The refused directory is the one the walk stood on: an ancestor
    /// names that ancestor, and nothing below it is created.
    #[test]
    fn a_refused_ancestor_is_named_and_nothing_is_created() {
        let root = tempfile::tempdir().unwrap();
        let volume = root.path().join("volume");
        std::fs::create_dir(&volume).unwrap();
        std::fs::set_permissions(&volume, std::os::unix::fs::PermissionsExt::from_mode(0o777))
            .unwrap();
        let state = volume.join("state");
        assert_eq!(
            explained_directory(&state, true, true).err(),
            Some(Refusal::Refused(Refused {
                directory: volume,
                reason: Reason::Writable,
            }))
        );
        assert!(!state.exists());
        assert_eq!(
            directory(&state, true, true).err(),
            Some(Failure::InvalidConfiguration)
        );
    }
}

#[cfg(all(test, target_os = "macos"))]
mod macos_tests {
    use super::*;

    fn capabilities(valid: u32, set: u32) -> libc::vol_capabilities_attr_t {
        let mut value = libc::vol_capabilities_attr_t {
            capabilities: [0; 4],
            valid: [0; 4],
        };
        value.valid[libc::VOL_CAPABILITIES_INTERFACES] = valid;
        value.capabilities[libc::VOL_CAPABILITIES_INTERFACES] = set;
        value
    }

    /// A volume admits the exclusive rename only when it reports the
    /// capability as both valid and set.
    #[test]
    fn rename_excl_admission_needs_a_valid_set_capability() {
        let bit = libc::VOL_CAP_INT_RENAME_EXCL;
        assert!(admits_rename_excl(&capabilities(bit, bit)));
        assert!(!admits_rename_excl(&capabilities(bit, 0)));
        assert!(!admits_rename_excl(&capabilities(0, bit)));
        assert!(!admits_rename_excl(&capabilities(0, 0)));
    }

    /// `atomic_no_replace_rename` holds on APFS: the temporary directory
    /// qualifies, and an existing name, a dangling link included, keeps its
    /// content while the second publication is refused.
    #[test]
    fn atomic_no_replace_rename_holds_on_apfs() {
        let root = tempfile::tempdir().unwrap();
        let directory = File::open(root.path()).unwrap();
        qualify_rename_excl(&directory).unwrap();
        publish_new(&directory, OsStr::new("first"), b"one").unwrap();
        assert_eq!(
            publish_new(&directory, OsStr::new("first"), b"two"),
            Err(Failure::ConfigurationExists)
        );
        assert_eq!(std::fs::read(root.path().join("first")).unwrap(), b"one");
        std::os::unix::fs::symlink(root.path().join("absent"), root.path().join("link")).unwrap();
        assert_eq!(
            publish_new(&directory, OsStr::new("link"), b"three"),
            Err(Failure::ConfigurationExists)
        );
        assert!(!root.path().join("absent").exists());
        publish_replace(&directory, OsStr::new("first"), b"four").unwrap();
        assert_eq!(std::fs::read(root.path().join("first")).unwrap(), b"four");
        let names: Vec<_> = std::fs::read_dir(root.path())
            .unwrap()
            .map(|entry| entry.unwrap().file_name())
            .collect();
        assert_eq!(names.len(), 2, "no temporary file is left: {names:?}");
    }
}
