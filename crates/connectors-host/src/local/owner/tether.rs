//! What a Unix owner's lifetime is tied to besides idle retirement and a
//! shutdown request (contracts/cli/v1alpha1/owner.md): the socket it listens on,
//! the state directory it opened and, in a test build only, a named process.
//! Once any of them is gone nothing can reach the owner or keep its state, and
//! it exits.
use std::{
    fs::File,
    os::unix::fs::{FileTypeExt, MetadataExt},
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

/// How often the accept loop looks; the contract allows at most 250 ms.
const EVERY: Duration = Duration::from_millis(200);

/// The environment variable a debug-build CLI passes to the owner it starts.
#[cfg(debug_assertions)]
pub(super) const PARENT: &str = "CONNECTORS_TEST_OWNER_PARENT";

/// Why the owner is leaving.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Loss {
    Socket,
    Directory,
    Parent,
}

pub(super) struct Tether {
    socket: PathBuf,
    state: PathBuf,
    identity: (u64, u64),
    parent: Option<Parent>,
    next: Instant,
}

impl Tether {
    /// `directory` is the state directory the owner opened by `state`, and
    /// `socket` the path its listener is bound at.
    pub(super) fn new(directory: &File, state: &Path, socket: &Path) -> Option<Self> {
        let metadata = directory.metadata().ok()?;
        Some(Self {
            socket: socket.to_path_buf(),
            state: state.to_path_buf(),
            identity: (metadata.dev(), metadata.ino()),
            parent: Parent::from_environment(),
            next: Instant::now(),
        })
    }

    /// What is gone, looked at no more often than [`EVERY`].
    pub(super) fn lost(&mut self) -> Option<Loss> {
        let now = Instant::now();
        if now < self.next {
            return None;
        }
        self.next = now + EVERY;
        self.lost_now()
    }

    fn lost_now(&self) -> Option<Loss> {
        match std::fs::metadata(&self.state) {
            Ok(metadata) if (metadata.dev(), metadata.ino()) == self.identity => {}
            Err(error) if error.kind() != std::io::ErrorKind::NotFound => {}
            _ => return Some(Loss::Directory),
        }
        match std::fs::symlink_metadata(&self.socket) {
            Ok(metadata) if metadata.file_type().is_socket() => {}
            Err(error) if error.kind() != std::io::ErrorKind::NotFound => {}
            _ => return Some(Loss::Socket),
        }
        if self.parent.as_ref().is_some_and(Parent::gone) {
            return Some(Loss::Parent);
        }
        None
    }
}

/// The process a test build's owner outlives by no more than one check: a
/// pidfd on Linux, which a reused process id cannot alias; elsewhere the id.
#[cfg_attr(not(debug_assertions), allow(dead_code))]
enum Parent {
    #[cfg(target_os = "linux")]
    Handle(std::os::fd::OwnedFd),
    Id(libc::pid_t),
    Gone,
}

impl Parent {
    #[cfg(debug_assertions)]
    fn from_environment() -> Option<Self> {
        let pid: libc::pid_t = std::env::var(PARENT).ok()?.parse().ok()?;
        if pid <= 0 {
            return None;
        }
        Some(Self::open(pid))
    }
    /// A release build reads no variable: its owner is tied to no process.
    #[cfg(not(debug_assertions))]
    fn from_environment() -> Option<Self> {
        None
    }

    #[cfg_attr(not(debug_assertions), allow(dead_code))]
    fn open(pid: libc::pid_t) -> Self {
        #[cfg(target_os = "linux")]
        {
            // SAFETY: pidfd_open takes a scalar id and flags; a successful
            // call returns a new descriptor this process owns.
            let raw = unsafe { libc::syscall(libc::SYS_pidfd_open, pid, 0) };
            if raw >= 0 {
                use std::os::fd::FromRawFd;
                // SAFETY: the descriptor was just returned to this process.
                return Self::Handle(unsafe { std::os::fd::OwnedFd::from_raw_fd(raw as i32) });
            }
            if std::io::Error::last_os_error().raw_os_error() == Some(libc::ESRCH) {
                return Self::Gone;
            }
        }
        Self::Id(pid)
    }

    fn gone(&self) -> bool {
        match self {
            #[cfg(target_os = "linux")]
            Self::Handle(fd) => {
                use std::os::fd::AsRawFd;
                let mut event = libc::pollfd {
                    fd: fd.as_raw_fd(),
                    events: libc::POLLIN,
                    revents: 0,
                };
                // SAFETY: one valid pollfd, no wait. A pidfd is readable once
                // its process has exited.
                let ready = unsafe { libc::poll(&mut event, 1, 0) };
                ready == 1 && event.revents & libc::POLLIN != 0
            }
            Self::Id(pid) => {
                // SAFETY: signal 0 only checks that the process exists.
                let alive = unsafe { libc::kill(*pid, 0) } == 0;
                !alive && std::io::Error::last_os_error().raw_os_error() == Some(libc::ESRCH)
            }
            Self::Gone => true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> (tempfile::TempDir, PathBuf, File, PathBuf) {
        let root = tempfile::tempdir().unwrap();
        let state = root.path().join("state");
        std::fs::create_dir(&state).unwrap();
        let directory = File::open(&state).unwrap();
        let socket = state.join("owner.sock");
        let listener = std::os::unix::net::UnixListener::bind(&socket).unwrap();
        std::mem::forget(listener);
        (root, state, directory, socket)
    }

    #[test]
    fn nothing_is_lost_while_socket_directory_and_parent_remain() {
        let (_root, state, directory, socket) = fixture();
        let mut tether = Tether::new(&directory, &state, &socket).unwrap();
        tether.parent = Some(Parent::open(std::process::id() as libc::pid_t));
        assert_eq!(tether.lost_now(), None);
    }

    #[test]
    fn a_removed_socket_is_lost() {
        let (_root, state, directory, socket) = fixture();
        let tether = Tether::new(&directory, &state, &socket).unwrap();
        std::fs::remove_file(&socket).unwrap();
        assert_eq!(tether.lost_now(), Some(Loss::Socket));
    }

    #[test]
    fn a_removed_or_replaced_directory_is_lost() {
        let (_root, state, directory, socket) = fixture();
        let tether = Tether::new(&directory, &state, &socket).unwrap();
        std::fs::remove_dir_all(&state).unwrap();
        assert_eq!(tether.lost_now(), Some(Loss::Directory));
        std::fs::create_dir(&state).unwrap();
        assert_eq!(tether.lost_now(), Some(Loss::Directory));
    }

    #[test]
    fn an_ended_parent_is_lost() {
        let (_root, state, directory, socket) = fixture();
        let mut child = std::process::Command::new("/bin/sh")
            .args(["-c", "exec sleep 30"])
            .spawn()
            .unwrap();
        let mut tether = Tether::new(&directory, &state, &socket).unwrap();
        tether.parent = Some(Parent::open(child.id() as libc::pid_t));
        assert_eq!(tether.lost_now(), None);
        child.kill().unwrap();
        child.wait().unwrap();
        assert_eq!(tether.lost_now(), Some(Loss::Parent));
    }
}
