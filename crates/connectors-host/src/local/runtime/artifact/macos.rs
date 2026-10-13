//! macOS capture and process control (contracts/cli/v1alpha1/semantics.md,
//! "macOS platform guarantees").
//!
//! - [`capture`]: `sealed_executable` holds only as `owner_private_copy`.
//! - [`launch_path`]: the copy is executed by its path, then removed.
//! - [`ExactChild`]: `exact_process_signal` holds only as `child_reap_lock`.
use super::super::{Failure, Result};
use sha2::{Digest, Sha256};
use std::{
    ffi::{CString, OsStr, OsString},
    fs::{File, OpenOptions},
    io::{Read, Seek, SeekFrom, Write},
    os::{
        fd::{AsRawFd, FromRawFd, OwnedFd},
        unix::{
            ffi::{OsStrExt, OsStringExt},
            fs::{MetadataExt, OpenOptionsExt, PermissionsExt},
        },
    },
    path::{Path, PathBuf},
    process::{Child, Command, ExitStatus},
    sync::{Arc, Mutex, MutexGuard, PoisonError},
    time::{Duration, Instant},
};

const LIMIT: usize = 512 * 1024 * 1024;
const DIRECTORY_PREFIX: &str = "connectors-image.";
const IMAGE: &str = "image";
/// Mach-O thin 32- and 64-bit headers as stored on disk (little-endian), and
/// the universal (fat) 32- and 64-bit headers.
const MACH_O: [[u8; 4]; 4] = [
    [0xce, 0xfa, 0xed, 0xfe],
    [0xcf, 0xfa, 0xed, 0xfe],
    [0xca, 0xfe, 0xba, 0xbe],
    [0xca, 0xfe, 0xba, 0xbf],
];

/// Copies `source` into a mode 0500 file in a fresh mode 0700 directory and
/// checks the configured SHA-256 and the Mach-O header against that copy, as
/// exec will read it. Returns a read-only descriptor on the copy, held at 10 or
/// above. Other users cannot change the copy; a process of this user can
/// rewrite it before exec (`owner_private_copy`). A refused copy is removed.
pub(crate) fn capture(mut source: File, expected_sha256: &str, until: Instant) -> Result<File> {
    source
        .seek(SeekFrom::Start(0))
        .map_err(|_| Failure::InvalidConfiguration)?;
    let image = private_directory()?.join(IMAGE);
    let captured = copy_and_check(&mut source, &image, expected_sha256, until);
    if captured.is_err() {
        discard(&image);
    }
    captured
}

fn copy_and_check(
    source: &mut File,
    image: &Path,
    expected_sha256: &str,
    until: Instant,
) -> Result<File> {
    let mut copy = OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o700)
        .custom_flags(libc::O_NOFOLLOW)
        .open(image)
        .map_err(|_| Failure::Unavailable)?;
    pump(source, until, |bytes| {
        copy.write_all(bytes).map_err(|_| Failure::Unavailable)
    })?;
    copy.set_permissions(std::fs::Permissions::from_mode(0o500))
        .map_err(|_| Failure::Unavailable)?;
    drop(copy);
    let mut file = OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW)
        .open(image)
        .map_err(|_| Failure::Unavailable)?;
    let mut digest = Sha256::new();
    let mut magic = Vec::with_capacity(4);
    pump(&mut file, until, |bytes| {
        let take = bytes.len().min(4 - magic.len());
        magic.extend_from_slice(&bytes[..take]);
        digest.update(bytes);
        Ok(())
    })?;
    if !MACH_O.iter().any(|header| header[..] == magic[..])
        || hex::encode(digest.finalize()) != expected_sha256
    {
        return Err(Failure::InvalidConfiguration);
    }
    // Descriptor three belongs to the channel. Hold the image above that range.
    // SAFETY: F_DUPFD_CLOEXEC returns a new owned fd for the same open file.
    let fd = unsafe { libc::fcntl(file.as_raw_fd(), libc::F_DUPFD_CLOEXEC, 10) };
    if fd < 0 {
        return Err(Failure::Unavailable);
    }
    // SAFETY: successful duplication transfers ownership to File.
    Ok(unsafe { File::from_raw_fd(fd) })
}

/// Reads `from` to its end in bounded chunks, as the Linux capture does: a
/// passed deadline is `Timeout`, a read failure or more than 512 MiB is
/// `InvalidConfiguration`.
fn pump(from: &mut File, until: Instant, mut each: impl FnMut(&[u8]) -> Result<()>) -> Result<()> {
    let mut buffer = [0u8; 65536];
    let mut length = 0usize;
    loop {
        if Instant::now() >= until {
            return Err(Failure::Timeout);
        }
        let count = from
            .read(&mut buffer)
            .map_err(|_| Failure::InvalidConfiguration)?;
        if count == 0 {
            return Ok(());
        }
        length = length
            .checked_add(count)
            .filter(|n| *n <= LIMIT)
            .ok_or(Failure::InvalidConfiguration)?;
        each(&buffer[..count])?;
    }
}

/// A fresh directory that `mkdtemp` creates with mode 0700.
fn private_directory() -> Result<PathBuf> {
    let template = std::env::temp_dir().join(format!("{DIRECTORY_PREFIX}XXXXXXXX"));
    let mut bytes = CString::new(template.into_os_string().into_vec())
        .map_err(|_| Failure::Unavailable)?
        .into_bytes_with_nul();
    // SAFETY: mkdtemp rewrites the trailing X's of this owned NUL-terminated
    // buffer in place and creates that directory.
    if unsafe { libc::mkdtemp(bytes.as_mut_ptr().cast()) }.is_null() {
        return Err(Failure::Unavailable);
    }
    bytes.pop();
    Ok(PathBuf::from(OsString::from_vec(bytes)))
}

/// Removes a copy and then its directory; a directory still holding anything
/// else stays.
fn discard(image: &Path) {
    let _ = std::fs::remove_file(image);
    if let Some(directory) = image.parent() {
        let _ = std::fs::remove_dir(directory);
    }
}

/// The path an open descriptor names now (`F_GETPATH`).
pub(crate) fn path_of(file: &File) -> std::io::Result<PathBuf> {
    let mut buffer = [0u8; libc::MAXPATHLEN as usize];
    // SAFETY: F_GETPATH writes at most MAXPATHLEN bytes, NUL included, into
    // this buffer for a descriptor the caller keeps open.
    if unsafe { libc::fcntl(file.as_raw_fd(), libc::F_GETPATH, buffer.as_mut_ptr()) } != 0 {
        return Err(std::io::Error::last_os_error());
    }
    let length = buffer
        .iter()
        .position(|&byte| byte == 0)
        .ok_or_else(|| std::io::Error::from_raw_os_error(libc::ENAMETOOLONG))?;
    Ok(PathBuf::from(OsStr::from_bytes(&buffer[..length])))
}

/// The path to execute a captured copy by. Dropping it removes the copy and
/// its directory, so it is held until spawn has returned, which is after exec.
pub(crate) struct Launch(PathBuf);
impl Launch {
    pub(crate) fn path(&self) -> &Path {
        &self.0
    }
}
impl Drop for Launch {
    fn drop(&mut self) {
        discard(&self.0);
    }
}

/// The launch path of a descriptor [`capture`] returned. Anything else, by
/// name or by file identity, is `Unavailable` and is never removed.
pub(crate) fn launch_path(image: &File) -> Result<Launch> {
    let path = path_of(image).map_err(|_| Failure::Unavailable)?;
    let ours = path.file_name() == Some(OsStr::new(IMAGE))
        && path
            .parent()
            .and_then(Path::file_name)
            .is_some_and(|name| name.as_bytes().starts_with(DIRECTORY_PREFIX.as_bytes()));
    let (named, open) = (
        std::fs::symlink_metadata(&path).map_err(|_| Failure::Unavailable)?,
        image.metadata().map_err(|_| Failure::Unavailable)?,
    );
    if !ours || named.dev() != open.dev() || named.ino() != open.ino() {
        return Err(Failure::Unavailable);
    }
    Ok(Launch(path))
}

/// A spawned child that is signalled by process id only under the lock that
/// reaps it (`child_reap_lock`). While the lock shows it unreaped, the id is
/// still this child's, alive or a zombie, and cannot name a reused process.
/// Every reap goes through [`ExactChild::try_wait`]; kqueue `EVFILT_PROC` with
/// `NOTE_EXIT` reports exit without reaping. Clones share the one lock.
#[derive(Clone)]
pub(crate) struct ExactChild(Arc<Shared>);
struct Shared {
    pid: libc::pid_t,
    queue: OwnedFd,
    reap: Mutex<Reap>,
}
struct Reap {
    process: Child,
    status: Option<ExitStatus>,
    exited: bool,
}

impl ExactChild {
    pub(crate) fn spawn(command: &mut Command) -> std::io::Result<Self> {
        let mut process = command.spawn()?;
        let pid = process.id() as libc::pid_t;
        match watch(pid) {
            Ok((queue, exited)) => Ok(Self(Arc::new(Shared {
                pid,
                queue,
                reap: Mutex::new(Reap {
                    process,
                    status: None,
                    exited,
                }),
            }))),
            Err(error) => {
                let _ = process.kill();
                let _ = process.wait();
                Err(error)
            }
        }
    }
    fn reap(&self) -> MutexGuard<'_, Reap> {
        self.0.reap.lock().unwrap_or_else(PoisonError::into_inner)
    }
    pub(crate) fn try_clone(&self) -> std::io::Result<Self> {
        Ok(self.clone())
    }
    /// Reaps the child if it has exited, without blocking, under the lock.
    pub(crate) fn try_wait(&mut self) -> std::io::Result<Option<ExitStatus>> {
        let mut reap = self.reap();
        if reap.status.is_none() {
            reap.status = reap.process.try_wait()?;
        }
        Ok(reap.status)
    }
    /// Waits for exit and reaps; the lock is held only while reaping.
    pub(crate) fn wait(&mut self) -> std::io::Result<ExitStatus> {
        loop {
            if let Some(status) = self.try_wait()? {
                return Ok(status);
            }
            self.observe(Duration::from_millis(10))?;
        }
    }
    /// Sends `signal` while the child is unreaped, under the reap lock. A
    /// reaped child, or one already gone (`ESRCH`), is sent nothing: `false`.
    pub(crate) fn signal(&self, signal: libc::c_int) -> std::io::Result<bool> {
        let reap = self.reap();
        if reap.status.is_some() {
            return Ok(false);
        }
        // SAFETY: the reap lock is held and the child is unreaped, so this id
        // is still the child's and no other process can hold it.
        if unsafe { libc::kill(self.0.pid, signal) } == 0 {
            return Ok(true);
        }
        let error = std::io::Error::last_os_error();
        if error.raw_os_error() == Some(libc::ESRCH) {
            Ok(false)
        } else {
            Err(error)
        }
    }
    pub(crate) fn kill(&mut self) -> std::io::Result<()> {
        self.signal(libc::SIGKILL).map(|_| ())
    }
    /// Whether the child has exited, from `NOTE_EXIT` or the reap, without
    /// blocking and without reaping.
    pub(crate) fn exited(&self) -> std::io::Result<bool> {
        self.observe(Duration::ZERO)
    }
    fn observe(&self, timeout: Duration) -> std::io::Result<bool> {
        let fired = next_exit(&self.0.queue, timeout)?;
        let mut reap = self.reap();
        reap.exited |= fired;
        Ok(reap.exited || reap.status.is_some())
    }
}

/// A kqueue watching `pid` for `NOTE_EXIT`, and whether it had already exited.
/// The child is unreaped, so `ESRCH` at registration means it has exited.
fn watch(pid: libc::pid_t) -> std::io::Result<(OwnedFd, bool)> {
    // SAFETY: kqueue has no preconditions; a kqueue is not inherited by fork.
    let raw = unsafe { libc::kqueue() };
    if raw < 0 {
        return Err(std::io::Error::last_os_error());
    }
    // SAFETY: kqueue returned a new owned descriptor.
    let queue = unsafe { OwnedFd::from_raw_fd(raw) };
    let change = libc::kevent {
        ident: pid as libc::uintptr_t,
        filter: libc::EVFILT_PROC,
        flags: libc::EV_ADD | libc::EV_ONESHOT,
        fflags: libc::NOTE_EXIT,
        data: 0,
        udata: std::ptr::null_mut(),
    };
    // SAFETY: one initialized change and no event buffer.
    let result = unsafe {
        libc::kevent(
            queue.as_raw_fd(),
            &change,
            1,
            std::ptr::null_mut(),
            0,
            std::ptr::null(),
        )
    };
    if result == 0 {
        return Ok((queue, false));
    }
    let error = std::io::Error::last_os_error();
    if error.raw_os_error() == Some(libc::ESRCH) {
        Ok((queue, true))
    } else {
        Err(error)
    }
}

/// Whether `NOTE_EXIT` arrived within `timeout`. An interrupted wait is `false`.
fn next_exit(queue: &OwnedFd, timeout: Duration) -> std::io::Result<bool> {
    let timeout = libc::timespec {
        tv_sec: timeout.as_secs() as libc::time_t,
        tv_nsec: timeout.subsec_nanos() as libc::c_long,
    };
    // SAFETY: an all-zero kevent is a valid value to be overwritten.
    let mut event: libc::kevent = unsafe { std::mem::zeroed() };
    // SAFETY: no changes; at most one event is written into `event`.
    let count = unsafe {
        libc::kevent(
            queue.as_raw_fd(),
            std::ptr::null(),
            0,
            &mut event,
            1,
            &timeout,
        )
    };
    match count {
        0 => Ok(false),
        1 => Ok(event.fflags & libc::NOTE_EXIT != 0),
        _ => {
            let error = std::io::Error::last_os_error();
            if error.raw_os_error() == Some(libc::EINTR) {
                Ok(false)
            } else {
                Err(error)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sha2::{Digest, Sha256};
    use std::{
        io::{Read, Seek, SeekFrom, Write},
        os::unix::fs::{MetadataExt, PermissionsExt},
        time::Duration,
    };

    const MACH_O_64: &[u8] = b"\xcf\xfa\xed\xfefictional executable content";

    fn until() -> Instant {
        Instant::now() + Duration::from_secs(5)
    }

    fn source(root: &Path, bytes: &[u8]) -> (PathBuf, String) {
        let path = root.join("source");
        std::fs::write(&path, bytes).unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o700)).unwrap();
        (path, hex::encode(Sha256::digest(bytes)))
    }

    /// `sealed_executable`: `owner_private_copy`. The image is a mode 0500
    /// copy, owned by this user, in a fresh mode 0700 directory; writes to the
    /// original and path replacement leave it as admitted, and the descriptor
    /// capture returns cannot write it.
    #[test]
    fn a_capture_is_an_owner_private_copy_in_a_private_directory() {
        let root = tempfile::tempdir().unwrap();
        let (path, hash) = source(root.path(), MACH_O_64);
        let mut snapshot = capture(File::open(&path).unwrap(), &hash, until()).unwrap();
        let copy = path_of(&snapshot).unwrap();
        assert_ne!(copy, path);
        let image = std::fs::symlink_metadata(&copy).unwrap();
        assert!(image.file_type().is_file());
        assert_eq!(image.mode() & 0o7777, 0o500);
        // SAFETY: getuid has no preconditions.
        assert_eq!(image.uid(), unsafe { libc::getuid() });
        let directory = std::fs::symlink_metadata(copy.parent().unwrap()).unwrap();
        assert!(directory.file_type().is_dir());
        assert_eq!(directory.mode() & 0o7777, 0o700);
        assert_eq!(directory.uid(), image.uid());
        assert!(snapshot.as_raw_fd() >= 10);

        std::fs::write(&path, b"\xcf\xfa\xed\xfechanged in place").unwrap();
        std::fs::rename(&path, root.path().join("old")).unwrap();
        std::fs::write(&path, b"replacement path").unwrap();
        snapshot.seek(SeekFrom::Start(0)).unwrap();
        let mut captured = Vec::new();
        snapshot.read_to_end(&mut captured).unwrap();
        assert_eq!(captured, MACH_O_64);
        assert!(snapshot.write(b"changed").is_err());
        assert!(matches!(
            capture(File::open(&path).unwrap(), &hash, until()),
            Err(Failure::InvalidConfiguration)
        ));
    }

    /// The format check is Mach-O: each thin and universal header is admitted,
    /// an ELF image is refused even with the configured digest, and so is a
    /// Mach-O image whose digest differs. The deadline still bounds the copy.
    #[test]
    fn only_a_mach_o_image_with_the_configured_digest_is_admitted() {
        let root = tempfile::tempdir().unwrap();
        for magic in [
            b"\xce\xfa\xed\xfe",
            b"\xcf\xfa\xed\xfe",
            b"\xca\xfe\xba\xbe",
            b"\xca\xfe\xba\xbf",
        ] {
            let bytes = [magic.as_slice(), b"fictional"].concat();
            let (path, hash) = source(root.path(), &bytes);
            capture(File::open(&path).unwrap(), &hash, until()).unwrap();
        }
        let (path, hash) = source(root.path(), b"\x7fELFfictional executable content");
        assert!(matches!(
            capture(File::open(&path).unwrap(), &hash, until()),
            Err(Failure::InvalidConfiguration)
        ));
        let (path, _) = source(root.path(), MACH_O_64);
        assert!(matches!(
            capture(File::open(&path).unwrap(), &"0".repeat(64), until()),
            Err(Failure::InvalidConfiguration)
        ));
        let (path, hash) = source(root.path(), MACH_O_64);
        assert!(matches!(
            capture(File::open(&path).unwrap(), &hash, Instant::now()),
            Err(Failure::Timeout)
        ));
    }

    /// The configured form of the Linux case: capture repeats path admission,
    /// and a source changed after an earlier capture is refused.
    #[test]
    fn configured_capture_requires_both_path_admission_and_matching_content() {
        use crate::local::config::Executable;
        let root = tempfile::tempdir().unwrap();
        let (path, sha256) = source(root.path(), MACH_O_64);
        let selected = Executable {
            path: path.clone(),
            sha256,
            args: Vec::new(),
        };
        selected.check().unwrap();
        let mut snapshot = selected.capture(until()).unwrap();
        std::fs::write(&path, b"\xcf\xfa\xed\xfechanged in place").unwrap();
        assert!(selected.check().is_err());
        assert!(matches!(
            selected.capture(until()),
            Err(Failure::InvalidConfiguration)
        ));
        snapshot.seek(SeekFrom::Start(0)).unwrap();
        let mut captured = Vec::new();
        snapshot.read_to_end(&mut captured).unwrap();
        assert_eq!(captured, MACH_O_64);
        std::fs::write(&path, MACH_O_64).unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o722)).unwrap();
        assert!(matches!(
            selected.capture(until()),
            Err(Failure::InvalidConfiguration)
        ));
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o700)).unwrap();
        let link = root.path().join("link");
        std::os::unix::fs::symlink(&path, &link).unwrap();
        assert!(matches!(
            Executable {
                path: link,
                ..selected.clone()
            }
            .capture(until()),
            Err(Failure::InvalidConfiguration)
        ));
    }

    /// Launch executes the copy by its path; dropping the launch removes the
    /// copy and its directory. A descriptor capture did not make is refused,
    /// so nothing else is ever removed.
    #[test]
    fn a_launch_names_the_copy_and_removes_it_with_its_directory() {
        let root = tempfile::tempdir().unwrap();
        let (path, hash) = source(root.path(), MACH_O_64);
        let snapshot = capture(File::open(&path).unwrap(), &hash, until()).unwrap();
        let launch = launch_path(&snapshot).unwrap();
        assert_eq!(launch.path(), path_of(&snapshot).unwrap());
        let directory = launch.path().parent().unwrap().to_path_buf();
        drop(launch);
        assert!(!directory.exists());
        assert!(matches!(
            launch_path(&File::open(&path).unwrap()),
            Err(Failure::Unavailable)
        ));
        assert!(path.exists());
    }

    /// `exact_process_signal`: `child_reap_lock`. A clone signals the live
    /// child; once the child is reaped no clone sends anything, so a reused
    /// process id is never signalled.
    #[test]
    fn a_reaped_child_is_never_signalled() {
        let mut child = ExactChild::spawn(Command::new("/bin/sleep").arg("30")).unwrap();
        let handle = child.try_clone().unwrap();
        assert!(!handle.exited().unwrap());
        assert!(handle.signal(libc::SIGKILL).unwrap());
        let status = child.wait().unwrap();
        assert_eq!(
            std::os::unix::process::ExitStatusExt::signal(&status),
            Some(libc::SIGKILL)
        );
        assert!(!handle.signal(libc::SIGKILL).unwrap());
        assert!(!handle.signal(libc::SIGTERM).unwrap());
        assert!(handle.exited().unwrap());
        assert_eq!(child.try_wait().unwrap(), Some(status));
    }

    /// kqueue `EVFILT_PROC` with `NOTE_EXIT` reports the exit before anything
    /// reaps the child: exit is observed without consuming the process id.
    #[test]
    fn exit_is_observed_through_kqueue_before_the_child_is_reaped() {
        let mut child = ExactChild::spawn(&mut Command::new("/usr/bin/true")).unwrap();
        let deadline = Instant::now() + Duration::from_secs(5);
        while !child.exited().unwrap() {
            assert!(Instant::now() < deadline, "NOTE_EXIT never arrived");
            std::thread::sleep(Duration::from_millis(5));
        }
        assert!(child.reap().status.is_none(), "exited() must not reap");
        assert!(child.wait().unwrap().success());
        assert!(child.exited().unwrap());
    }

    /// A capture that is never launched, as on every refusal between capture
    /// and spawn (the consumer launch's custody and lifecycle refusals after
    /// its capture, the owner spawn's channel setup), must not leave its copy
    /// and directory behind in the temporary directory.
    #[test]
    fn a_capture_dropped_without_a_launch_leaves_no_copy_behind() {
        let root = tempfile::tempdir().unwrap();
        let (path, hash) = source(root.path(), MACH_O_64);
        let snapshot = capture(File::open(&path).unwrap(), &hash, until()).unwrap();
        let directory = path_of(&snapshot).unwrap().parent().unwrap().to_path_buf();
        assert!(directory.exists());
        drop(snapshot);
        assert!(
            !directory.exists(),
            "an unlaunched capture left {directory:?} behind"
        );
    }

    /// `launch_path` promises that a descriptor capture did not make is
    /// refused and never removed. A file of this user's own, merely named
    /// like a capture, is not one capture made.
    #[test]
    fn launch_path_refuses_a_file_named_like_a_capture() {
        let root = tempfile::tempdir().unwrap();
        let forged = root.path().join(format!("{DIRECTORY_PREFIX}forged"));
        std::fs::create_dir(&forged).unwrap();
        let (path, _) = source(&forged, MACH_O_64);
        let image = forged.join(IMAGE);
        std::fs::rename(&path, &image).unwrap();
        let launched = launch_path(&File::open(&image).unwrap());
        let refused = matches!(launched, Err(Failure::Unavailable));
        drop(launched);
        assert!(image.exists(), "launch_path removed a file it did not make");
        assert!(refused, "launch_path admitted a file it did not make");
    }
}
