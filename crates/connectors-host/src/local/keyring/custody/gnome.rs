//! Qualification for one audited Linux GNOME Keyring binary and encrypted login
//! collection. Refuse unsupported implementations instead of guessing durability.
use super::{Failure, Result};
use crate::local::{filesystem as fs, keyring::Service};
use sha2::{Digest, Sha256};
use std::{
    ffi::{CString, OsStr},
    fs::File,
    io::Read,
    os::{
        fd::{AsRawFd, FromRawFd},
        unix::{ffi::OsStrExt, fs::MetadataExt},
    },
    path::PathBuf,
};
use zeroize::Zeroizing;

// Arch Linux gnome-keyring 1:50.0-1; source and installed artifact receipts are
// recorded in docs/local-secret-service.md. This pin is intentionally narrow.
pub(super) const DAEMON_SHA256: &str =
    "b9a71f6b4c4bfaf1759a99036b7b3ab6cf98b2b479c0c2a24f02f5f2ca53958b";
const COLLECTION: &str = "/org/freedesktop/secrets/collection/login";
const FILE: &str = "login.keyring";
const HEADER: &[u8; 20] = b"GnomeKeyring\n\r\0\n\0\0\0\0";

pub(super) struct Persistence {
    process: File,
    pub(super) directory: File,
    path: PathBuf,
    ancestors: Vec<File>,
    #[cfg(test)]
    pub(super) fail_sync: std::sync::atomic::AtomicBool,
}

impl Persistence {
    pub(super) fn lock(&self, until: std::time::Instant) -> Result<WriterLock<'_>> {
        loop {
            // SAFETY: a held directory descriptor supports flock on Linux.
            if unsafe { libc::flock(self.directory.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) }
                == 0
            {
                return Ok(WriterLock(&self.directory));
            }
            if std::io::Error::last_os_error().raw_os_error() != Some(libc::EWOULDBLOCK)
                || std::time::Instant::now() >= until
            {
                return Err(Failure::Unavailable);
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
    }
    pub(super) fn admit(service: &Service) -> Result<Self> {
        if service.collection.as_str() != COLLECTION {
            return Err(Failure::Unavailable);
        }
        let bus = zbus::blocking::fdo::DBusProxy::new(&service.connection).map_err(unavailable)?;
        let pid = bus
            .get_connection_unix_process_id(service.owner.clone().into())
            .map_err(unavailable)?;
        let process = process_handle(pid)?;
        let proc = File::open(format!("/proc/{pid}")).map_err(unavailable)?;
        if proc.metadata().map_err(unavailable)?.uid() != fs::uid() {
            return Err(Failure::Unavailable);
        }
        // A path derived from the daemon's environment must identify the same
        // filesystem in this process. Kernel proc links are intentionally read
        // here; they are not caller-selected credential paths.
        for (other, ours) in [
            (format!("/proc/{pid}/ns/mnt"), "/proc/self/ns/mnt"),
            (format!("/proc/{pid}/ns/user"), "/proc/self/ns/user"),
            (format!("/proc/{pid}/root"), "/"),
        ] {
            let other = std::fs::metadata(other).map_err(unavailable)?;
            let ours = std::fs::metadata(ours).map_err(unavailable)?;
            if (other.dev(), other.ino()) != (ours.dev(), ours.ino()) {
                return Err(Failure::Unavailable);
            }
        }
        let mut executable = proc_file(&proc, "exe")?;
        let stat = executable.metadata().map_err(unavailable)?;
        if !stat.is_file()
            || (stat.uid() != 0 && stat.uid() != fs::uid())
            || stat.mode() & 0o022 != 0
            || stat.len() > 32 * 1024 * 1024
        {
            return Err(Failure::Unavailable);
        }
        let mut digest = Sha256::new();
        std::io::copy(&mut executable, &mut digest).map_err(unavailable)?;
        if hex::encode(digest.finalize()) != DAEMON_SHA256 {
            return Err(Failure::Unavailable);
        }
        let mut environment = Zeroizing::new(Vec::new());
        proc_file(&proc, "environ")?
            .take(128 * 1024 + 1)
            .read_to_end(&mut environment)
            .map_err(unavailable)?;
        if environment.len() > 128 * 1024 {
            return Err(Failure::Unavailable);
        }
        let directory = store_directory(&environment)?;
        drop(environment);
        let directory_file = fs::directory(&directory, false, true).map_err(unavailable)?;
        // A successful fsync on a RAM filesystem is not persistent custody.
        // This initial receipt qualifies the local ext filesystem family only.
        let mut kind = std::mem::MaybeUninit::<libc::statfs>::uninit();
        // SAFETY: kind is a correctly sized writable buffer; the fd is held.
        if unsafe { libc::fstatfs(directory_file.as_raw_fd(), kind.as_mut_ptr()) } != 0 {
            return Err(Failure::Unavailable);
        }
        // SAFETY: fstatfs succeeded and initialized the structure.
        if unsafe { kind.assume_init() }.f_type != 0xef53 {
            return Err(Failure::Unavailable);
        }
        let mut ancestors = Vec::new();
        for ancestor in directory.ancestors().skip(1) {
            ancestors.push(fs::directory(ancestor, false, false).map_err(unavailable)?);
        }
        let result = Self {
            process,
            directory: directory_file,
            path: directory,
            ancestors,
            #[cfg(test)]
            fail_sync: std::sync::atomic::AtomicBool::new(false),
        };
        result.check_process()?;
        // Even a previously created directory may not yet have a durable parent
        // entry. Synchronize the complete admitted path before any secret write.
        result.synchronize()?;
        Ok(result)
    }

    pub(super) fn check_process(&self) -> Result<()> {
        let mut descriptor = libc::pollfd {
            fd: self.process.as_raw_fd(),
            events: libc::POLLIN,
            revents: 0,
        };
        // SAFETY: descriptor is a valid initialized buffer and the pidfd is held.
        let status = unsafe { libc::poll(&mut descriptor, 1, 0) };
        if status != 0 {
            return Err(Failure::Unavailable);
        }
        Ok(())
    }

    fn encrypted_file(&self) -> Result<File> {
        let current = fs::directory(&self.path, false, true).map_err(unavailable)?;
        let current = current.metadata().map_err(unavailable)?;
        let held = self.directory.metadata().map_err(unavailable)?;
        if (current.dev(), current.ino()) != (held.dev(), held.ino()) {
            return Err(Failure::Unavailable);
        }
        let mut file =
            fs::private_file_at(&self.directory, OsStr::new(FILE)).map_err(unavailable)?;
        let mut header = [0; 20];
        file.read_exact(&mut header).map_err(unavailable)?;
        if &header != HEADER {
            return Err(Failure::Unavailable);
        }
        Ok(file)
    }

    pub(super) fn check_encrypted(&self) -> Result<()> {
        self.encrypted_file().map(|_| ())
    }

    pub(super) fn synchronize(&self) -> Result<()> {
        self.check_process()?;
        #[cfg(test)]
        if self.fail_sync.load(std::sync::atomic::Ordering::SeqCst) {
            return Err(Failure::Unavailable);
        }
        // GNOME rewrites the collection through rename. Open the current exact
        // encrypted file AFTER the method acknowledgement, not the previous inode.
        self.encrypted_file()?.sync_all().map_err(unavailable)?;
        self.directory.sync_all().map_err(unavailable)?;
        for ancestor in &self.ancestors {
            ancestor.sync_all().map_err(unavailable)?;
        }
        self.check_process()
    }
}

pub(super) struct WriterLock<'a>(&'a File);
impl Drop for WriterLock<'_> {
    fn drop(&mut self) {
        // SAFETY: the borrowed descriptor outlives the guard.
        unsafe {
            libc::flock(self.0.as_raw_fd(), libc::LOCK_UN);
        }
    }
}

fn store_directory(environment: &[u8]) -> Result<PathBuf> {
    let mut home = None;
    let mut xdg = None;
    for entry in environment.split(|b| *b == 0).filter(|e| !e.is_empty()) {
        let split = entry
            .iter()
            .position(|b| *b == b'=')
            .ok_or(Failure::Unavailable)?;
        let (key, rest) = entry.split_at(split);
        let value = &rest[1..];
        // Debug-only directory overrides and injected shared libraries invalidate
        // the audited mapping. No environment data reaches a diagnostic.
        if (key == b"GNOME_KEYRING_TEST_PATH" || key.starts_with(b"LD_")) && !value.is_empty() {
            return Err(Failure::Unavailable);
        }
        if key == b"HOME" || key == b"XDG_DATA_HOME" {
            let target = if key == b"HOME" { &mut home } else { &mut xdg };
            if target.is_some() {
                return Err(Failure::Unavailable);
            }
            *target = Some(PathBuf::from(OsStr::from_bytes(value)));
        }
    }
    let home = home.ok_or(Failure::Unavailable)?;
    fs::validate_path(&home).map_err(unavailable)?;
    let data = xdg
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or_else(|| home.join(".local/share"));
    fs::validate_path(&data).map_err(unavailable)?;
    let current = data.join("keyrings");
    let legacy = home.join(".gnome2/keyrings");
    // Match the audited module's existing-directory preference, then admit the
    // chosen path descriptor by descriptor. No symlink or unsafe-parent fallback.
    if !current.is_dir() && legacy.is_dir() {
        Ok(legacy)
    } else {
        Ok(current)
    }
}

fn process_handle(pid: u32) -> Result<File> {
    let pid: i32 = pid.try_into().map_err(unavailable)?;
    // SAFETY: pidfd_open creates a new descriptor; flags=0 is the supported form.
    let fd = unsafe { libc::syscall(libc::SYS_pidfd_open, pid, 0) };
    if fd < 0 {
        return Err(Failure::Unavailable);
    }
    // SAFETY: the successful syscall returned an owned descriptor.
    Ok(unsafe { File::from_raw_fd(fd as i32) })
}

fn proc_file(proc: &File, name: &str) -> Result<File> {
    let name = CString::new(name).map_err(unavailable)?;
    // SAFETY: this is a held kernel proc directory and an internal fixed name.
    // Following the kernel's exe magic link is necessary to hash the live image.
    let fd = unsafe {
        libc::openat(
            proc.as_raw_fd(),
            name.as_ptr(),
            libc::O_RDONLY | libc::O_CLOEXEC,
        )
    };
    if fd < 0 {
        return Err(Failure::Unavailable);
    }
    // SAFETY: openat returned a new owned descriptor.
    Ok(unsafe { File::from_raw_fd(fd) })
}
fn unavailable<T>(_: T) -> Failure {
    Failure::Unavailable
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn daemon_environment_mapping_refuses_ambiguous_or_injected_layouts() {
        for env in [
            b"".as_slice(),
            b"HOME=relative\0",
            b"HOME=/a\0HOME=/b\0",
            b"HOME=/a\0XDG_DATA_HOME=relative\0",
            b"HOME=/a\0GNOME_KEYRING_TEST_PATH=/b\0",
            b"HOME=/a\0LD_PRELOAD=secret\0",
        ] {
            assert!(store_directory(env).is_err());
        }
        assert_eq!(
            store_directory(b"HOME=/nonexistent-connectors-test\0").unwrap(),
            std::path::Path::new("/nonexistent-connectors-test/.local/share/keyrings")
        );
    }
}
