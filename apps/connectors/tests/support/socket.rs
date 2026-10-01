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

    pub fn path(&self, name: &str) -> PathBuf {
        PathBuf::from(format!("/proc/self/fd/{}/{name}", self.0.as_raw_fd()))
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
