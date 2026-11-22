//! Qualification for one audited Linux GNOME Keyring binary and encrypted login
//! collection. Refuse unsupported implementations instead of guessing durability.
use super::{Service, fs};
use crate::{Failure, Result};
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
const DAEMON_SHA256: &str = "b9a71f6b4c4bfaf1759a99036b7b3ab6cf98b2b479c0c2a24f02f5f2ca53958b";
const COLLECTIONS: &str = "/org/freedesktop/secrets/collection/";
const HEADER: &[u8; 20] = b"GnomeKeyring\n\r\0\n\0\0\0\0";

pub(super) struct Persistence {
    process: File,
    pub(super) directory: File,
    path: PathBuf,
    file: String,
    ancestors: Vec<File>,
    #[cfg(any(test, feature = "test-support"))]
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
        let name = collection_name(service.collection.as_str())?;
        let file = collection_file(service.collection.as_str())?;
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
        let mut buffer = [0; 64 * 1024];
        loop {
            let read = executable.read(&mut buffer).map_err(unavailable)?;
            if read == 0 {
                break;
            }
            digest.update(&buffer[..read]);
        }
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
        let directory_file = fs::directory(&directory, true).map_err(unavailable)?;
        // The collection behind the alias is the one this directory's `default`
        // file names; a decoded name the daemon would not read there is refused.
        check_default_alias(&directory_file, &name)?;
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
            ancestors.push(fs::directory(ancestor, false).map_err(unavailable)?);
        }
        let result = Self {
            process,
            directory: directory_file,
            path: directory,
            file,
            ancestors,
            #[cfg(any(test, feature = "test-support"))]
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
        self.encrypted_file_named(&self.file)
    }

    fn encrypted_file_named(&self, file: &str) -> Result<File> {
        let current = fs::directory(&self.path, true).map_err(unavailable)?;
        let current = current.metadata().map_err(unavailable)?;
        let held = self.directory.metadata().map_err(unavailable)?;
        if (current.dev(), current.ino()) != (held.dev(), held.ino()) {
            return Err(Failure::Unavailable);
        }
        let mut file =
            fs::private_file_at(&self.directory, OsStr::new(file)).map_err(unavailable)?;
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

    /// Qualify the file behind `collection`, which may be another than the one
    /// custody writes to: the legacy `login` collection an item was found in.
    pub(super) fn check_encrypted_collection(&self, collection: &str) -> Result<()> {
        self.encrypted_file_named(&collection_file(collection)?)
            .map(|_| ())
    }

    pub(super) fn synchronize(&self) -> Result<()> {
        self.synchronize_file(&self.file)
    }

    /// Synchronize the file behind `collection`, which may be another than the
    /// one custody writes to: the legacy `login` collection after a delete there.
    pub(super) fn synchronize_collection(&self, collection: &str) -> Result<()> {
        self.synchronize_file(&collection_file(collection)?)
    }

    fn synchronize_file(&self, file: &str) -> Result<()> {
        self.check_process()?;
        #[cfg(any(test, feature = "test-support"))]
        if self.fail_sync.load(std::sync::atomic::Ordering::SeqCst) {
            return Err(Failure::Unavailable);
        }
        // GNOME rewrites the collection through rename. Open the current exact
        // encrypted file AFTER the method acknowledgement, not the previous inode.
        self.encrypted_file_named(file)?
            .sync_all()
            .map_err(unavailable)?;
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
/// The identifier GNOME Keyring gave the collection at `collection`. Its
/// object path segment is the identifier as `gkd_secret_util_build_path`
/// writes it: an ASCII letter or digit as itself, every other byte as `_` and
/// two lower-case hex digits. Only that exact form is decoded; anything the
/// encoder never writes is refused, never guessed.
fn collection_name(collection: &str) -> Result<String> {
    let encoded = collection
        .strip_prefix(COLLECTIONS)
        .ok_or(Failure::Unavailable)?
        .as_bytes();
    let hex = |b: u8| match b {
        b'0'..=b'9' => Ok(b - b'0'),
        b'a'..=b'f' => Ok(b - b'a' + 10),
        _ => Err(Failure::Unavailable),
    };
    let mut decoded = Vec::with_capacity(encoded.len());
    let mut rest = encoded;
    while let Some((&byte, tail)) = rest.split_first() {
        if byte.is_ascii_alphanumeric() {
            decoded.push(byte);
            rest = tail;
            continue;
        }
        let [b'_', high, low, ..] = *rest else {
            return Err(Failure::Unavailable);
        };
        let value = hex(high)? << 4 | hex(low)?;
        if value.is_ascii_alphanumeric() {
            return Err(Failure::Unavailable);
        }
        decoded.push(value);
        rest = &rest[3..];
    }
    let name = String::from_utf8(decoded).map_err(unavailable)?;
    if name.is_empty()
        || name == "."
        || name == ".."
        || name.contains('/')
        || name.chars().any(char::is_control)
    {
        return Err(Failure::Unavailable);
    }
    Ok(name)
}

/// The encrypted file behind a collection: `<identifier>.keyring`, the name
/// gkm-secret-module gives it and reads its identifier back from. A second
/// keyring of one label is `<label>_1.keyring` and named `<label>_1`.
fn collection_file(collection: &str) -> Result<String> {
    let file = format!("{}.keyring", collection_name(collection)?);
    // NAME_MAX: a longer name cannot be a file in the directory.
    if file.len() > 255 {
        return Err(Failure::Unavailable);
    }
    Ok(file)
}

/// The identifier gkd-secret-service reads behind the `default` alias from the
/// keyrings directory's `default` file on each client connection: its contents
/// with surrounding whitespace stripped, or `login` when it is missing or empty.
fn alias_identifier(contents: Option<&[u8]>) -> Vec<u8> {
    // g_strstrip strips C isspace, which includes \v, unlike trim_ascii.
    let space = |b: &u8| matches!(b, b' ' | b'\t' | b'\n' | b'\x0b' | b'\x0c' | b'\r');
    let stripped = contents
        .and_then(|c| {
            let start = c.iter().position(|b| !space(b))?;
            let end = c.iter().rposition(|b| !space(b))?;
            Some(&c[start..=end])
        })
        .unwrap_or(b"login");
    stripped.to_vec()
}

/// Cross-check the decoded default identifier with the `default` file the
/// daemon reads it from. The file holds no secret; it is read bounded.
fn check_default_alias(directory: &File, name: &str) -> Result<()> {
    let contents = match fs::owned_file_at(directory, OsStr::new("default")).map_err(unavailable)? {
        Some(file) => {
            let mut contents = Vec::new();
            file.take(4097)
                .read_to_end(&mut contents)
                .map_err(unavailable)?;
            if contents.len() > 4096 {
                return Err(Failure::Unavailable);
            }
            Some(contents)
        }
        None => None,
    };
    if alias_identifier(contents.as_deref()) != name.as_bytes() {
        return Err(Failure::Unavailable);
    }
    Ok(())
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

    fn file_of(segment: &str) -> Result<String> {
        collection_file(&format!("{COLLECTIONS}{segment}"))
    }

    /// gkd_secret_util_build_path writes every byte that is not an ASCII letter
    /// or digit as `_%02x`; the file is the decoded identifier plus `.keyring`.
    #[test]
    fn an_escaped_collection_path_names_the_file_of_its_decoded_identifier() {
        for (segment, file) in [
            ("login", "login.keyring"),
            (
                "Standard_2dSchl_c3_bcsselbund",
                "Standard-Schlüsselbund.keyring",
            ),
            // A second keyring of one label gets `<label>_1.keyring` and, once
            // loaded from it, the identifier `<label>_1`.
            ("login_5f1", "login_1.keyring"),
            ("my_2ering", "my.ring.keyring"),
            ("_2e_2e_2e", "....keyring"),
            ("a_20b", "a b.keyring"),
        ] {
            assert_eq!(file_of(segment).as_deref(), Ok(file), "{segment}");
        }
    }

    #[test]
    fn a_malformed_or_hostile_collection_path_is_refused() {
        let long = "a".repeat(256 - ".keyring".len());
        for segment in [
            "",
            "_",
            "a_",
            "a_2",
            "a_2g",
            "a_g2",
            // Upper-case hex and an escaped letter or digit are never written.
            "a_2F",
            "a_2D",
            "_41",
            "a_30",
            // A bare `_` is always written as `_5f`.
            "a_b",
            "login_1",
            // Separator, NUL, dot names and control characters.
            "_2f",
            "a_2fb",
            "_2e_2e_2fetc",
            "_00",
            "a_00b",
            "_2e",
            "_2e_2e",
            "a_0ab",
            "a_09",
            "_1f",
            "_7f",
            "_c2_85",
            // Not UTF-8: a lone lead byte, a lone continuation, an overlong form.
            "_c3",
            "a_bc",
            "_c0_af",
            "_ff",
            // Not one segment, or not a D-Bus path character.
            "a/b",
            "/a",
            "a-b",
            "a.b",
            "Schlüssel",
            &long,
        ] {
            assert_eq!(file_of(segment), Err(Failure::Unavailable), "{segment:?}");
        }
        assert_eq!(file_of(&long[1..]).map(|f| f.len()), Ok(255));
        for path in [
            "/org/freedesktop/secrets/aliases/default",
            "/org/freedesktop/secrets/collection",
            "/org/freedesktop/secrets/collectionlogin",
            "login",
        ] {
            assert_eq!(collection_file(path), Err(Failure::Unavailable), "{path}");
        }
    }

    /// gkd-secret-service's update_default: the `default` file's contents,
    /// whitespace-stripped, name the default identifier; none or empty is `login`.
    #[test]
    fn the_default_alias_file_names_the_identifier_the_daemon_reads() {
        for (contents, identifier) in [
            (None, b"login".as_slice()),
            (Some(b"".as_slice()), b"login"),
            (Some(b" \t\n\r\x0b\x0c"), b"login"),
            (Some(b"login"), b"login"),
            (
                Some("Standard-Schlüsselbund".as_bytes()),
                "Standard-Schlüsselbund".as_bytes(),
            ),
            (Some(b"\n elsewhere \n"), b"elsewhere"),
            (Some(b"two words"), b"two words"),
        ] {
            assert_eq!(alias_identifier(contents), identifier, "{contents:?}");
        }
    }
}
