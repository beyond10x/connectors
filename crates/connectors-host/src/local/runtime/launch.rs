//! Consumer launch: one connection's protected document on descriptor 3 of an
//! operator-pinned consumer. See `contracts/cli/v1alpha1/consumer-launch.md`.
//!
//! The owner process reads the credential and writes it into a sealed memfd
//! ([`sealed`]). It passes that descriptor and the consumer image it captured
//! to the launching CLI over the owner socket ([`send`], [`receive`]). The CLI
//! reads neither: [`Consumer::run`] places the credential at descriptor 3 and
//! starts the captured image with its pinned argv prefix and the caller's
//! arguments, only the caller variables its entry passes by prefix, and the
//! caller's stdin, stdout and stderr, then returns the consumer's exit code.
//!
//! macOS keeps every admission rule and changes delivery as section 7 of the
//! contract states: the credential is an unlinked read-only file, the socket
//! carries `SO_NOSIGPIPE`, received descriptors are marked close-on-exec right
//! after the receive, and the image copy is executed by its path.
//!
//! Windows changes delivery as section 8 of the contract states: the
//! credential is a read-only handle on a file whose name was removed with
//! POSIX semantics, and the image and credential handles reach the CLI by
//! `DuplicateHandle` into it, inheritance off, their values in a frame in
//! place of `SCM_RIGHTS` ([`send`], [`receive`]).
use super::{Failure, Result};
#[cfg(unix)]
use std::os::{
    fd::{AsRawFd, FromRawFd, RawFd},
    unix::{ffi::OsStrExt, net::UnixStream, process::CommandExt, process::ExitStatusExt},
};
use std::{
    collections::BTreeSet,
    ffi::OsString,
    fs::File,
    process::{Command, Stdio},
};
#[cfg(unix)]
use std::{io::Write, time::Instant};

/// The consumer image, then the credential.
pub(crate) const DESCRIPTORS: usize = 2;
/// Received descriptors are held at or above this number, clear of the
/// standard streams and of descriptor 3, which belongs to the credential.
#[cfg(unix)]
const FLOOR: RawFd = 10;
#[cfg(unix)]
const MARKER: u8 = 0x4c;

/// The Windows credential, handle passing and their tests.
#[cfg(windows)]
mod windows;
#[cfg(windows)]
pub(crate) use windows::{receive, sealed, send};

/// A sealed, read-only copy of `material`, positioned at its start. The seals
/// forbid every later write, growth, shrink and seal change; the returned
/// descriptor is a fresh read-only open file description, so its holder can
/// neither write the bytes nor move an offset this process shares.
#[cfg(target_os = "linux")]
pub(crate) fn sealed(material: &[u8]) -> Result<File> {
    // SAFETY: a static NUL-terminated name and documented scalar flags.
    let raw = unsafe {
        libc::memfd_create(
            c"connectors-credential".as_ptr(),
            libc::MFD_CLOEXEC | libc::MFD_ALLOW_SEALING,
        )
    };
    if raw < 0 {
        return Err(Failure::Unavailable);
    }
    // SAFETY: memfd_create returned a new owned descriptor.
    let mut writable = unsafe { File::from_raw_fd(raw) };
    writable
        .write_all(material)
        .map_err(|_| Failure::Unavailable)?;
    let seals = libc::F_SEAL_WRITE | libc::F_SEAL_GROW | libc::F_SEAL_SHRINK | libc::F_SEAL_SEAL;
    // SAFETY: the descriptor is live and owned, and never mapped.
    if unsafe { libc::fcntl(writable.as_raw_fd(), libc::F_ADD_SEALS, seals) } != 0 {
        return Err(Failure::Unavailable);
    }
    let path = std::ffi::CString::new(format!("/proc/self/fd/{}", writable.as_raw_fd()))
        .map_err(|_| Failure::Unavailable)?;
    // SAFETY: a live NUL-terminated path; the result is a new owned descriptor.
    let fd = unsafe { libc::open(path.as_ptr(), libc::O_RDONLY | libc::O_CLOEXEC) };
    if fd < 0 {
        return Err(Failure::Unavailable);
    }
    // SAFETY: open returned a new owned descriptor.
    Ok(unsafe { File::from_raw_fd(fd) })
}

/// The macOS credential (consumer-launch.md §7): there is no memfd, so
/// `material` is written verbatim into a mode 0400 file in a fresh mode 0700
/// directory, a separate read-only descriptor is opened on it, and the file
/// and its directory are removed before that descriptor is returned. Only the
/// descriptor names the bytes: its holder reads them from the start, cannot
/// write, shrink or grow them through it, and finds no path to reopen.
#[cfg(target_os = "macos")]
pub(crate) fn sealed(material: &[u8]) -> Result<File> {
    use std::os::unix::{ffi::OsStringExt, fs::MetadataExt};
    /// Removes the credential file, then its directory, on every exit.
    struct Scratch(std::ffi::CString, std::ffi::CString);
    impl Drop for Scratch {
        fn drop(&mut self) {
            // SAFETY: live NUL-terminated paths this function created.
            unsafe {
                libc::unlink(self.1.as_ptr());
                libc::rmdir(self.0.as_ptr());
            }
        }
    }
    let mut template = std::env::temp_dir()
        .join("connectors-credential.XXXXXX")
        .into_os_string()
        .into_vec();
    template.push(0);
    // SAFETY: a writable NUL-terminated template that mkdtemp fills in place;
    // the directory it creates is mode 0700.
    if unsafe { libc::mkdtemp(template.as_mut_ptr().cast()) }.is_null() {
        return Err(Failure::Unavailable);
    }
    template.pop();
    let directory = std::ffi::CString::new(template).map_err(|_| Failure::Unavailable)?;
    let mut path = directory.as_bytes().to_vec();
    path.extend_from_slice(b"/credential");
    let path = std::ffi::CString::new(path).map_err(|_| Failure::Unavailable)?;
    let scratch = Scratch(directory, path);
    // SAFETY: a live NUL-terminated path; the result is a new owned descriptor.
    let raw = unsafe {
        libc::open(
            scratch.1.as_ptr(),
            libc::O_CREAT | libc::O_EXCL | libc::O_RDWR | libc::O_NOFOLLOW | libc::O_CLOEXEC,
            0o400 as libc::c_uint,
        )
    };
    if raw < 0 {
        return Err(Failure::Unavailable);
    }
    // SAFETY: open returned a new owned descriptor.
    let mut writable = unsafe { File::from_raw_fd(raw) };
    writable
        .write_all(material)
        .map_err(|_| Failure::Unavailable)?;
    // SAFETY: a live NUL-terminated path; the result is a new owned descriptor.
    let fd = unsafe {
        libc::open(
            scratch.1.as_ptr(),
            libc::O_RDONLY | libc::O_NOFOLLOW | libc::O_CLOEXEC,
        )
    };
    if fd < 0 {
        return Err(Failure::Unavailable);
    }
    // SAFETY: open returned a new owned descriptor.
    let readable = unsafe { File::from_raw_fd(fd) };
    let (written, opened) = (
        writable.metadata().map_err(|_| Failure::Unavailable)?,
        readable.metadata().map_err(|_| Failure::Unavailable)?,
    );
    if (written.dev(), written.ino()) != (opened.dev(), opened.ino())
        || opened.len() != material.len() as u64
    {
        return Err(Failure::Unavailable);
    }
    drop(writable);
    drop(scratch);
    if readable
        .metadata()
        .map_err(|_| Failure::Unavailable)?
        .nlink()
        != 0
    {
        return Err(Failure::Unavailable);
    }
    Ok(readable)
}

#[cfg(unix)]
fn remaining(until: Instant) -> Result<std::time::Duration> {
    until
        .checked_duration_since(Instant::now())
        .filter(|d| !d.is_zero())
        .ok_or(Failure::Timeout)
}

#[cfg(unix)]
fn control_space(count: usize) -> usize {
    // SAFETY: CMSG_SPACE is a pure size computation.
    unsafe { libc::CMSG_SPACE((count * std::mem::size_of::<RawFd>()) as u32) as usize }
}

/// Pass `files` to the peer as one marker byte carrying `SCM_RIGHTS`. Only the
/// descriptors travel; no byte of their content is read or written here.
#[cfg(unix)]
pub(crate) fn send(stream: &UnixStream, files: &[&File], until: Instant) -> Result<()> {
    let descriptors: Vec<RawFd> = files.iter().map(|file| file.as_raw_fd()).collect();
    let marker = [MARKER];
    let mut vector = libc::iovec {
        iov_base: marker.as_ptr() as *mut libc::c_void,
        iov_len: 1,
    };
    // u64 storage keeps the control buffer aligned for cmsghdr.
    let mut control = vec![0u64; control_space(descriptors.len()).div_ceil(8)];
    // SAFETY: zero is a valid msghdr; every pointer set below outlives sendmsg.
    let mut message: libc::msghdr = unsafe { std::mem::zeroed() };
    message.msg_iov = &mut vector;
    message.msg_iovlen = 1;
    message.msg_control = control.as_mut_ptr().cast();
    message.msg_controllen = control_space(descriptors.len()) as _;
    // SAFETY: the control buffer holds one header with room for every descriptor.
    unsafe {
        let header = libc::CMSG_FIRSTHDR(&message);
        if header.is_null() {
            return Err(Failure::Unavailable);
        }
        (*header).cmsg_level = libc::SOL_SOCKET;
        (*header).cmsg_type = libc::SCM_RIGHTS;
        (*header).cmsg_len =
            libc::CMSG_LEN((descriptors.len() * std::mem::size_of::<RawFd>()) as u32) as _;
        std::ptr::copy_nonoverlapping(
            descriptors.as_ptr(),
            libc::CMSG_DATA(header).cast::<RawFd>(),
            descriptors.len(),
        );
    }
    stream
        .set_write_timeout(Some(remaining(until)?))
        .map_err(|_| Failure::Unavailable)?;
    #[cfg(target_os = "linux")]
    let flags = libc::MSG_NOSIGNAL;
    // macOS has no MSG_NOSIGNAL: the socket itself carries SO_NOSIGPIPE, with
    // the same effect on this and every later write.
    #[cfg(target_os = "macos")]
    let flags = {
        let enable: libc::c_int = 1;
        // SAFETY: an initialized c_int of the stated size on a held socket.
        if unsafe {
            libc::setsockopt(
                stream.as_raw_fd(),
                libc::SOL_SOCKET,
                libc::SO_NOSIGPIPE,
                (&enable as *const libc::c_int).cast(),
                std::mem::size_of::<libc::c_int>() as libc::socklen_t,
            )
        } != 0
        {
            return Err(Failure::Unavailable);
        }
        0
    };
    loop {
        // SAFETY: message and everything it points at are live for the call.
        let sent = unsafe { libc::sendmsg(stream.as_raw_fd(), &message, flags) };
        if sent == 1 {
            // A captured image copy is the receiver's to launch and remove now.
            #[cfg(target_os = "macos")]
            super::artifact::sent(files);
            return Ok(());
        }
        let error = std::io::Error::last_os_error();
        match error.kind() {
            std::io::ErrorKind::Interrupted => continue,
            std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut => {
                return Err(Failure::Timeout);
            }
            _ => return Err(Failure::Unavailable),
        }
    }
}

/// Receive exactly `count` descriptors sent by [`send`]. Anything else — no
/// marker, a truncated or foreign control message, another count — closes
/// whatever arrived and refuses.
#[cfg(unix)]
pub(crate) fn receive(stream: &UnixStream, count: usize, until: Instant) -> Result<Vec<File>> {
    let mut marker = [0u8];
    let mut vector = libc::iovec {
        iov_base: marker.as_mut_ptr().cast(),
        iov_len: 1,
    };
    let space = control_space(count);
    let mut control = vec![0u64; space.div_ceil(8)];
    // SAFETY: zero is a valid msghdr; every pointer set below outlives recvmsg.
    let mut message: libc::msghdr = unsafe { std::mem::zeroed() };
    message.msg_iov = &mut vector;
    message.msg_iovlen = 1;
    message.msg_control = control.as_mut_ptr().cast();
    message.msg_controllen = space as _;
    super::channel::set_read_timeout(stream, remaining(until)?)
        .map_err(|_| Failure::Unavailable)?;
    #[cfg(target_os = "linux")]
    let flags = libc::MSG_CMSG_CLOEXEC;
    // macOS has no MSG_CMSG_CLOEXEC (`atomic_close_on_exec`, weaker form
    // `set_after_receive`): each descriptor is marked close-on-exec below, as
    // soon as the receive returns.
    #[cfg(target_os = "macos")]
    let flags = 0;
    let received = loop {
        // SAFETY: message and everything it points at are live for the call.
        let received = unsafe { libc::recvmsg(stream.as_raw_fd(), &mut message, flags) };
        if received >= 0 {
            break received;
        }
        let error = std::io::Error::last_os_error();
        match error.kind() {
            std::io::ErrorKind::Interrupted => continue,
            std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut => {
                return Err(Failure::Timeout);
            }
            _ => return Err(Failure::Unavailable),
        }
    };
    // Take ownership of every delivered descriptor first, so each is closed on
    // any refusal below.
    let mut files = Vec::new();
    let mut foreign = false;
    // SAFETY: the kernel filled msg_control/msg_controllen; the CMSG macros
    // walk only within them, and each SCM_RIGHTS entry is a new owned fd.
    unsafe {
        let mut header = libc::CMSG_FIRSTHDR(&message);
        while !header.is_null() {
            if (*header).cmsg_level == libc::SOL_SOCKET && (*header).cmsg_type == libc::SCM_RIGHTS {
                let bytes = (*header).cmsg_len as usize - libc::CMSG_LEN(0) as usize;
                let data = libc::CMSG_DATA(header).cast::<RawFd>();
                for index in 0..bytes / std::mem::size_of::<RawFd>() {
                    let fd = std::ptr::read_unaligned(data.add(index));
                    #[cfg(target_os = "macos")]
                    libc::fcntl(fd, libc::F_SETFD, libc::FD_CLOEXEC);
                    files.push(File::from_raw_fd(fd));
                }
            } else {
                foreign = true;
            }
            header = libc::CMSG_NXTHDR(&message, header);
        }
    }
    if received != 1
        || marker[0] != MARKER
        || foreign
        || message.msg_flags & (libc::MSG_CTRUNC | libc::MSG_TRUNC) != 0
        || files.len() != count
    {
        return Err(Failure::Protocol);
    }
    files
        .into_iter()
        .map(|file| {
            // SAFETY: F_DUPFD_CLOEXEC returns a new owned fd for the same file.
            let fd = unsafe { libc::fcntl(file.as_raw_fd(), libc::F_DUPFD_CLOEXEC, FLOOR) };
            if fd < 0 {
                return Err(Failure::Unavailable);
            }
            // SAFETY: successful duplication transfers ownership to File.
            let file = unsafe { File::from_raw_fd(fd) };
            // The owner's image copy is this process's to launch and remove.
            #[cfg(target_os = "macos")]
            super::artifact::received(&file);
            Ok(file)
        })
        .collect()
}

/// An admitted launch, as the owner handed it to the CLI: the captured consumer
/// image, the sealed credential, the consumer's pinned argv prefix and the
/// environment name prefixes its entry passes.
pub struct Consumer {
    executable: File,
    credential: File,
    args: Vec<String>,
    pass_env: BTreeSet<String>,
}

impl Consumer {
    /// `executable` is the captured image, as a capture or as received.
    pub(crate) fn new(
        executable: impl Into<File>,
        credential: File,
        args: Vec<String>,
        pass_env: BTreeSet<String>,
    ) -> Self {
        Self {
            executable: executable.into(),
            credential,
            args,
            pass_env,
        }
    }

    /// The pinned argv prefix after argv\[0\].
    pub fn args(&self) -> &[String] {
        &self.args
    }

    /// Start the consumer and wait for it. No shell and no PATH lookup: the
    /// captured image is executed through its descriptor with the pinned argv
    /// prefix followed by `extra` verbatim; the environment holds only the
    /// `caller` variables whose names start with a passed prefix; stdin, stdout
    /// and stderr are the caller's; the credential is on descriptor 3. An
    /// `extra` argument holding NUL is `InvalidInput` and nothing starts.
    /// Returns the consumer's exit code, or 128 plus the signal number when a
    /// signal ended it. Ends the consumer with `SIGTERM` if this thread exits
    /// first on Linux; on macOS, which has no parent-death signal, the
    /// consumer keeps running, and the image copy is executed by its path.
    pub fn run(
        self,
        extra: &[String],
        caller: impl IntoIterator<Item = (OsString, OsString)>,
    ) -> Result<i32> {
        if extra.iter().any(|arg| arg.contains('\0')) {
            return Err(Failure::InvalidInput);
        }
        // Capture and receipt hold the image at FLOOR or above; at 0 to 3 it
        // would be a standard stream or replaced by the credential before exec.
        if self.executable.as_raw_fd() <= 3 {
            return Err(Failure::Unavailable);
        }
        let credential = self.credential.as_raw_fd();
        // SAFETY: getpid has no preconditions.
        let parent = unsafe { libc::getpid() };
        #[cfg(target_os = "linux")]
        let mut command = Command::new(format!("/proc/self/fd/{}", self.executable.as_raw_fd()));
        // macOS has no /proc/self/fd: the image copy is executed by its path,
        // which becomes argv[0] (consumer-launch.md §7).
        #[cfg(target_os = "macos")]
        let launch = super::artifact::launch_path(&self.executable)?;
        #[cfg(target_os = "macos")]
        let mut command = Command::new(launch.path());
        #[cfg(target_os = "macos")]
        let end = descriptor_end()?;
        command
            .args(&self.args)
            .args(extra)
            .env_clear()
            .envs(caller.into_iter().filter(|(name, _)| {
                self.pass_env
                    .iter()
                    .any(|prefix| name.as_bytes().starts_with(prefix.as_bytes()))
            }))
            .stdin(Stdio::inherit())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit());
        // SAFETY: the hook calls only async-signal-safe functions on captured
        // scalars; the credential descriptor stays open until spawn returns.
        #[cfg(target_os = "linux")]
        unsafe {
            command.pre_exec(move || {
                if libc::prctl(libc::PR_SET_PDEATHSIG, libc::SIGTERM) != 0
                    || libc::getppid() != parent
                {
                    return Err(std::io::Error::from_raw_os_error(libc::ECHILD));
                }
                if libc::dup2(credential, 3) < 0 || libc::fcntl(3, libc::F_SETFD, 0) < 0 {
                    return Err(std::io::Error::last_os_error());
                }
                close_on_exec_above_three()
            });
        }
        // macOS has no parent-death signal (`parent_death_kill`, weaker form
        // `adapter_channel_close`, which covers adapters only): a consumer
        // whose launcher dies first keeps running.
        // SAFETY: the hook calls only async-signal-safe functions on captured
        // scalars; the credential descriptor stays open until spawn returns.
        #[cfg(target_os = "macos")]
        unsafe {
            command.pre_exec(move || {
                if libc::getppid() != parent {
                    return Err(std::io::Error::from_raw_os_error(libc::ECHILD));
                }
                if libc::dup2(credential, 3) < 0 || libc::fcntl(3, libc::F_SETFD, 0) < 0 {
                    return Err(std::io::Error::last_os_error());
                }
                close_on_exec_from_four(end);
                Ok(())
            });
        }
        let mut child = command.spawn().map_err(|_| Failure::Unavailable)?;
        drop(self.credential);
        drop(self.executable);
        // As system(3) does: an interrupt or quit from the terminal reaches the
        // consumer, which decides; this process waits for its answer.
        let interrupts = Interrupts::ignore();
        let status = child.wait();
        drop(interrupts);
        status.map(code).map_err(|_| Failure::Unavailable)
    }
}

/// Exactly descriptors 0 to 3 reach a consumer: mark every descriptor from 4
/// up close-on-exec, including any the caller left open without it. Marking
/// instead of closing keeps the image's descriptor, which exec opens through
/// `/proc/self/fd/N`, and the spawn's own error pipe usable until exec closes
/// them. Runs between fork and exec, so it is async-signal-safe: no
/// allocation and no `/proc` read. `close_range(CLOSE_RANGE_CLOEXEC)` needs
/// Linux 5.11; before that (`ENOSYS`, or `EINVAL` for the flag) every number up
/// to the `RLIMIT_NOFILE` soft limit is marked one by one.
#[cfg(target_os = "linux")]
fn close_on_exec_above_three() -> std::io::Result<()> {
    // SAFETY: close_range only changes descriptor flags of this process.
    if unsafe {
        libc::close_range(
            4,
            libc::c_uint::MAX,
            libc::CLOSE_RANGE_CLOEXEC as libc::c_int,
        )
    } == 0
    {
        return Ok(());
    }
    let error = std::io::Error::last_os_error();
    if !matches!(error.raw_os_error(), Some(libc::ENOSYS | libc::EINVAL)) {
        return Err(error);
    }
    let mut limit = libc::rlimit {
        rlim_cur: 0,
        rlim_max: 0,
    };
    // SAFETY: getrlimit writes into the live, owned struct.
    if unsafe { libc::getrlimit(libc::RLIMIT_NOFILE, &mut limit) } != 0 {
        return Err(std::io::Error::last_os_error());
    }
    let end = limit.rlim_cur.min(libc::c_int::MAX as libc::rlim_t) as libc::c_int;
    for fd in 4..end {
        // SAFETY: F_SETFD on a number that is not open fails with EBADF, harmlessly.
        unsafe { libc::fcntl(fd, libc::F_SETFD, libc::FD_CLOEXEC) };
    }
    Ok(())
}

/// One past the highest descriptor number this process can hold: the
/// `RLIMIT_NOFILE` soft limit, bounded by `kern.maxfilesperproc`. Read before
/// the spawn, so the child marks a bounded range without a syscall it cannot
/// make between fork and exec.
#[cfg(target_os = "macos")]
fn descriptor_end() -> Result<libc::c_int> {
    let mut limit = libc::rlimit {
        rlim_cur: 0,
        rlim_max: 0,
    };
    // SAFETY: getrlimit writes into the live, owned struct.
    if unsafe { libc::getrlimit(libc::RLIMIT_NOFILE, &mut limit) } != 0 {
        return Err(Failure::Unavailable);
    }
    let mut most: libc::c_int = 0;
    let mut size = std::mem::size_of::<libc::c_int>();
    // SAFETY: a static NUL-terminated name and a c_int buffer of the stated size.
    if unsafe {
        libc::sysctlbyname(
            c"kern.maxfilesperproc".as_ptr(),
            (&mut most as *mut libc::c_int).cast(),
            &mut size,
            std::ptr::null_mut(),
            0,
        )
    } != 0
        || size != std::mem::size_of::<libc::c_int>()
        || most <= 0
    {
        return Err(Failure::Unavailable);
    }
    Ok(limit.rlim_cur.min(most as libc::rlim_t) as libc::c_int)
}

/// Exactly descriptors 0 to 3 reach a consumer on macOS, which has no
/// `close_range`: every number from 4 up to `end` is marked close-on-exec one
/// by one, including any the caller left open without it. Runs between fork
/// and exec, so it only calls `fcntl`.
#[cfg(target_os = "macos")]
fn close_on_exec_from_four(end: libc::c_int) {
    for fd in 4..end {
        // SAFETY: F_SETFD on a number that is not open fails with EBADF, harmlessly.
        unsafe { libc::fcntl(fd, libc::F_SETFD, libc::FD_CLOEXEC) };
    }
}

/// A consumer's exit code, or 128 plus the signal number that ended it.
pub(crate) fn code(status: std::process::ExitStatus) -> i32 {
    status
        .code()
        .unwrap_or_else(|| 128 + status.signal().unwrap_or(0))
}

/// `SIGINT` and `SIGQUIT` ignored until dropped, then restored.
struct Interrupts([libc::sigaction; 2]);
impl Interrupts {
    fn ignore() -> Self {
        // SAFETY: zeroed sigaction is valid; sigaction only reads `ignore` and
        // writes the previous action into owned storage.
        unsafe {
            let mut previous: [libc::sigaction; 2] = std::mem::zeroed();
            let mut ignore: libc::sigaction = std::mem::zeroed();
            ignore.sa_sigaction = libc::SIG_IGN;
            for (index, signal) in [libc::SIGINT, libc::SIGQUIT].into_iter().enumerate() {
                libc::sigaction(signal, &ignore, &mut previous[index]);
            }
            Self(previous)
        }
    }
}
impl Drop for Interrupts {
    fn drop(&mut self) {
        for (index, signal) in [libc::SIGINT, libc::SIGQUIT].into_iter().enumerate() {
            // SAFETY: restores the exact action read in `ignore`.
            unsafe {
                libc::sigaction(signal, &self.0[index], std::ptr::null_mut());
            }
        }
    }
}

#[cfg(all(test, unix))]
mod tests;
