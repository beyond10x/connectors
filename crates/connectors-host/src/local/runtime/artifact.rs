//! Capture executable content before launch. A retained source inode alone does
//! not prevent an in-place write between verification and exec.
use super::{Failure, Result};
use sha2::{Digest, Sha256};
use std::{
    fs::File,
    io::{Read, Seek, SeekFrom, Write},
    os::{
        fd::{AsRawFd, FromRawFd},
        unix::fs::{MetadataExt, PermissionsExt},
    },
    time::Instant,
};

pub(crate) fn capture(mut source: File, expected_sha256: &str, until: Instant) -> Result<File> {
    source
        .seek(SeekFrom::Start(0))
        .map_err(|_| Failure::InvalidConfiguration)?;
    // SAFETY: a static NUL-terminated name and documented scalar flags. Do not
    // override a kernel policy that creates non-executable memfds.
    let raw = unsafe {
        libc::memfd_create(
            c"connectors-adapter".as_ptr(),
            libc::MFD_CLOEXEC | libc::MFD_ALLOW_SEALING,
        )
    };
    if raw < 0 {
        return Err(Failure::Unavailable);
    }
    // SAFETY: memfd_create returned a new owned descriptor.
    let mut snapshot = unsafe { File::from_raw_fd(raw) };
    if snapshot
        .metadata()
        .map_err(|_| Failure::Unavailable)?
        .mode()
        & 0o111
        == 0
    {
        return Err(Failure::Unavailable);
    }
    snapshot
        .set_permissions(std::fs::Permissions::from_mode(0o500))
        .map_err(|_| Failure::Unavailable)?;
    let mut digest = Sha256::new();
    let mut buffer = [0u8; 65536];
    let mut length = 0usize;
    let mut magic = [0u8; 4];
    loop {
        if Instant::now() >= until {
            return Err(Failure::Timeout);
        }
        let count = source
            .read(&mut buffer)
            .map_err(|_| Failure::InvalidConfiguration)?;
        if count == 0 {
            break;
        }
        let prefix = count.min(4usize.saturating_sub(length));
        if prefix > 0 {
            magic[length..length + prefix].copy_from_slice(&buffer[..prefix]);
        }
        length = length
            .checked_add(count)
            .filter(|n| *n <= 512 * 1024 * 1024)
            .ok_or(Failure::InvalidConfiguration)?;
        snapshot
            .write_all(&buffer[..count])
            .map_err(|_| Failure::Unavailable)?;
        digest.update(&buffer[..count]);
    }
    if magic != *b"\x7fELF" || hex::encode(digest.finalize()) != expected_sha256 {
        return Err(Failure::InvalidConfiguration);
    }
    let seals = libc::F_SEAL_WRITE | libc::F_SEAL_GROW | libc::F_SEAL_SHRINK | libc::F_SEAL_SEAL;
    // SAFETY: snapshot is live, owned and never mapped writable or shared.
    if unsafe { libc::fcntl(snapshot.as_raw_fd(), libc::F_ADD_SEALS, seals) } != 0 {
        return Err(Failure::Unavailable);
    }
    // Descriptor three belongs to the channel. Hold exec above that range.
    // SAFETY: F_DUPFD_CLOEXEC returns a new owned fd referring to the sealed file.
    let fd = unsafe { libc::fcntl(snapshot.as_raw_fd(), libc::F_DUPFD_CLOEXEC, 10) };
    if fd < 0 {
        return Err(Failure::Unavailable);
    }
    // SAFETY: successful duplication transfers ownership to File.
    Ok(unsafe { File::from_raw_fd(fd) })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn configured_capture_requires_both_path_admission_and_matching_content() {
        use crate::local::config::Executable;
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("source");
        let bytes = b"\x7fELFfictional executable content";
        std::fs::write(&path, bytes).unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o700)).unwrap();
        let selected = Executable {
            path: path.clone(),
            sha256: hex::encode(Sha256::digest(bytes)),
            args: Vec::new(),
        };
        let until = Instant::now() + Duration::from_secs(5);
        selected.check().unwrap();
        let mut snapshot = selected.capture(until).unwrap();
        // Neither path replacement nor in-place source writes alter the
        // admitted immutable bytes; later captures still check the selection.
        std::fs::write(&path, b"\x7fELFchanged in place").unwrap();
        assert!(selected.check().is_err());
        assert!(matches!(
            selected.capture(until),
            Err(Failure::InvalidConfiguration)
        ));
        snapshot.seek(SeekFrom::Start(0)).unwrap();
        let mut captured = Vec::new();
        snapshot.read_to_end(&mut captured).unwrap();
        assert_eq!(captured, bytes);
        std::fs::write(&path, bytes).unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o722)).unwrap();
        assert!(matches!(
            selected.capture(until),
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
            .capture(until),
            Err(Failure::InvalidConfiguration)
        ));
        assert!(matches!(
            selected.capture(Instant::now()),
            Err(Failure::Timeout)
        ));
    }

    #[test]
    fn admitted_snapshot_survives_source_changes_and_refuses_mutation() {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("source");
        let bytes = b"\x7fELFfictional executable content";
        std::fs::write(&path, bytes).unwrap();
        let hash = hex::encode(Sha256::digest(bytes));
        let until = Instant::now() + Duration::from_secs(5);
        let mut snapshot = capture(File::open(&path).unwrap(), &hash, until).unwrap();
        // Simulate both forms of installer replacement after capture.
        std::fs::write(&path, b"changed in place").unwrap();
        std::fs::rename(&path, root.path().join("old")).unwrap();
        std::fs::write(&path, b"replacement path").unwrap();
        snapshot.seek(SeekFrom::Start(0)).unwrap();
        let mut captured = Vec::new();
        snapshot.read_to_end(&mut captured).unwrap();
        assert_eq!(captured, bytes);
        snapshot.seek(SeekFrom::Start(0)).unwrap();
        assert_eq!(
            snapshot.write(b"changed").unwrap_err().raw_os_error(),
            Some(libc::EPERM)
        );
        assert_eq!(
            snapshot.set_len(0).unwrap_err().raw_os_error(),
            Some(libc::EPERM)
        );
        assert_eq!(
            snapshot.set_len(1024).unwrap_err().raw_os_error(),
            Some(libc::EPERM)
        );
        assert!(matches!(
            capture(File::open(path).unwrap(), &hash, until),
            Err(Failure::InvalidConfiguration)
        ));
    }
}
