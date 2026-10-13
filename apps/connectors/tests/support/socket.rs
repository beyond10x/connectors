//! Reaches a Unix socket through a descriptor on its directory, as the host
//! does (`crates/connectors-host/src/local/owner/transport.rs`, `socket_path`).
//! `/proc/self/fd/<fd>/<name>` stays short however long the directory's own
//! path is, so a test under a long `TMPDIR` never exceeds `SUN_LEN`.
#![allow(dead_code)]

use std::{
    fs::File,
    os::{
        fd::AsRawFd,
        unix::net::{UnixListener, UnixStream},
    },
    path::{Path, PathBuf},
};

/// An open directory. Paths from [`Directory::path`] name sockets through its
/// descriptor and are valid only while this value is alive.
pub struct Directory(File);

impl Directory {
    pub fn open(path: &Path) -> Self {
        Self(File::open(path).unwrap())
    }

    #[cfg(target_os = "linux")]
    pub fn path(&self, name: &str) -> PathBuf {
        PathBuf::from(format!("/proc/self/fd/{}/{name}", self.0.as_raw_fd()))
    }

    /// macOS has no `/proc/self/fd`: the directory's own path, which
    /// `F_GETPATH` reads from the descriptor, as the host names the owner
    /// socket there (contracts/cli/v1alpha1/owner.md, "macOS"). The path must
    /// fit the 104-byte `sun_path`.
    #[cfg(target_os = "macos")]
    pub fn path(&self, name: &str) -> PathBuf {
        use std::os::unix::ffi::OsStrExt;
        let mut buffer = [0u8; libc::PATH_MAX as usize];
        // SAFETY: F_GETPATH writes at most PATH_MAX bytes, NUL included, into
        // this buffer for the open descriptor this value owns.
        let result =
            unsafe { libc::fcntl(self.0.as_raw_fd(), libc::F_GETPATH, buffer.as_mut_ptr()) };
        assert_eq!(result, 0, "F_GETPATH: {}", std::io::Error::last_os_error());
        let length = buffer.iter().position(|&byte| byte == 0).unwrap();
        Path::new(std::ffi::OsStr::from_bytes(&buffer[..length])).join(name)
    }

    pub fn bind(&self, name: &str) -> UnixListener {
        UnixListener::bind(self.path(name)).unwrap()
    }

    pub fn connect(&self, name: &str) -> UnixStream {
        UnixStream::connect(self.path(name)).unwrap()
    }
}

/// Binds `directory/name`; the listener does not need the directory afterwards.
pub fn bind(directory: &Path, name: &str) -> UnixListener {
    Directory::open(directory).bind(name)
}

/// Connects to `directory/name`; the stream does not need the directory afterwards.
pub fn connect(directory: &Path, name: &str) -> UnixStream {
    Directory::open(directory).connect(name)
}
