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
use super::{Failure, Result};
use std::{
    collections::BTreeSet,
    ffi::OsString,
    fs::File,
    io::Write,
    os::{
        fd::{AsRawFd, FromRawFd, RawFd},
        unix::{ffi::OsStrExt, net::UnixStream, process::CommandExt, process::ExitStatusExt},
    },
    process::{Command, Stdio},
    time::Instant,
};

/// The consumer image, then the credential.
pub(crate) const DESCRIPTORS: usize = 2;
/// Received descriptors are held at or above this number, clear of the
/// standard streams and of descriptor 3, which belongs to the credential.
const FLOOR: RawFd = 10;
const MARKER: u8 = 0x4c;

/// A sealed, read-only copy of `material`, positioned at its start. The seals
/// forbid every later write, growth, shrink and seal change; the returned
/// descriptor is a fresh read-only open file description, so its holder can
/// neither write the bytes nor move an offset this process shares.
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

fn remaining(until: Instant) -> Result<std::time::Duration> {
    until
        .checked_duration_since(Instant::now())
        .filter(|d| !d.is_zero())
        .ok_or(Failure::Timeout)
}

fn control_space(count: usize) -> usize {
    // SAFETY: CMSG_SPACE is a pure size computation.
    unsafe { libc::CMSG_SPACE((count * std::mem::size_of::<RawFd>()) as u32) as usize }
}

/// Pass `files` to the peer as one marker byte carrying `SCM_RIGHTS`. Only the
/// descriptors travel; no byte of their content is read or written here.
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
    loop {
        // SAFETY: message and everything it points at are live for the call.
        let sent = unsafe { libc::sendmsg(stream.as_raw_fd(), &message, libc::MSG_NOSIGNAL) };
        if sent == 1 {
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
    stream
        .set_read_timeout(Some(remaining(until)?))
        .map_err(|_| Failure::Unavailable)?;
    let received = loop {
        // SAFETY: message and everything it points at are live for the call.
        let received =
            unsafe { libc::recvmsg(stream.as_raw_fd(), &mut message, libc::MSG_CMSG_CLOEXEC) };
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
                    files.push(File::from_raw_fd(std::ptr::read_unaligned(data.add(index))));
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
            Ok(unsafe { File::from_raw_fd(fd) })
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
    pub(crate) fn new(
        executable: File,
        credential: File,
        args: Vec<String>,
        pass_env: BTreeSet<String>,
    ) -> Self {
        Self {
            executable,
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
    /// signal ended it. Ends the consumer with `SIGTERM` if this thread exits first.
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
        let mut command = Command::new(format!("/proc/self/fd/{}", self.executable.as_raw_fd()));
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

#[cfg(test)]
mod tests;
