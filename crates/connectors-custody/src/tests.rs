use crate::{
    Backend, CustodyBackend, Failure, MAX_BYTES, Material, ReadBackend, Result, SIGNING_SEED_BYTES,
    Scope, State, Version, contract,
    memory::MemoryStore,
    parts::{Header, MAX_PARTS, PART_BYTES, Parted, fake::FakeEntries, header_name, part_name},
};
use std::time::{Duration, Instant};
use uuid::Uuid;

fn until() -> Instant {
    Instant::now() + Duration::from_secs(10)
}

/// A fresh fake Credential Manager and the split layout over it for one
/// credential scope.
fn parted() -> (FakeEntries, Parted<FakeEntries>, Scope) {
    let entries = FakeEntries::default();
    let scope = Scope::new(Uuid::new_v4(), Uuid::new_v4()).unwrap();
    (entries.clone(), Parted::new(entries, scope), scope)
}

/// Distinct bytes at every position, so a part in the wrong place shows.
fn value(len: usize) -> Vec<u8> {
    (0..len).map(|i| (i % 251) as u8).collect()
}

/// The entry names one version holds: its header, if any, and its parts.
fn layout(entries: &FakeEntries, version: Version) -> (bool, Vec<usize>) {
    let names = entries.names();
    let header = names.contains(&header_name(version));
    let parts = (0..MAX_PARTS)
        .filter(|index| names.contains(&part_name(version, *index)))
        .collect();
    (header, parts)
}

#[test]
fn split_layout_answers_the_custody_contract() {
    let entries = FakeEntries::default();
    contract::run(&|scope| Ok(Parted::new(entries.clone(), scope)));
    assert!(entries.names().is_empty(), "the suite deleted every entry");
}

/// Every value is parts then a header, the header written last; a value of
/// one blob or less is one part and a header.
#[test]
fn split_layout_writes_numbered_parts_then_one_header() {
    for (len, parts) in [
        (1, 1),
        (PART_BYTES, 1),
        (PART_BYTES + 1, 2),
        (MAX_BYTES, 26),
    ] {
        let (entries, backend, scope) = parted();
        let version = Version::new(scope, Uuid::new_v4()).unwrap();
        let material = value(len);
        assert_eq!(
            backend.write_new(
                version,
                &Material::from_slice(&material),
                until(),
                || Ok(())
            ),
            Ok(())
        );
        assert_eq!(
            layout(&entries, version),
            (true, (0..parts).collect()),
            "{len} bytes"
        );
        assert_eq!(entries.names().len(), parts + 1);
        assert!(backend.read(version).unwrap().ct_eq(&material));
        // The header is the last write: one mutation fewer leaves no header.
        let (entries, backend, scope) = parted();
        let version = Version::new(scope, Uuid::new_v4()).unwrap();
        entries.crash_after(parts);
        assert_eq!(
            backend.write_new(
                version,
                &Material::from_slice(&material),
                until(),
                || Ok(())
            ),
            Err(Failure::OutcomeUnknown)
        );
        assert_eq!(layout(&entries, version), (false, (0..parts).collect()));
    }
    assert_eq!(MAX_PARTS, 26);
}

/// A crash after the parts and before the header: the version does not exist,
/// a second write of it is `Conflict`, and a delete reclaims the parts.
#[test]
fn parts_without_a_header_are_missing_and_reclaimed() {
    let (entries, backend, scope) = parted();
    let version = Version::new(scope, Uuid::new_v4()).unwrap();
    let material = value(3 * PART_BYTES);
    entries.crash_after(3);
    assert_eq!(
        backend.write_new(
            version,
            &Material::from_slice(&material),
            until(),
            || Ok(())
        ),
        Err(Failure::OutcomeUnknown)
    );
    assert_eq!(layout(&entries, version), (false, vec![0, 1, 2]));
    entries.recover();
    assert!(matches!(backend.read(version), Err(Failure::Missing)));
    assert_eq!(
        backend.write_new(
            version,
            &Material::from_slice(&material),
            until(),
            || Ok(())
        ),
        Err(Failure::Conflict),
        "an unacknowledged version is never overwritten"
    );
    assert_eq!(backend.delete(version, until(), || Ok(())), Ok(()));
    assert!(entries.names().is_empty(), "reclaim removed every part");
    assert!(matches!(backend.read(version), Err(Failure::Missing)));
}

/// A crash after the header is removed and before the parts are: the version
/// is gone, and a following delete reclaims the parts.
#[test]
fn header_removed_and_parts_not_is_missing_and_reclaimed() {
    let (entries, backend, scope) = parted();
    let version = Version::new(scope, Uuid::new_v4()).unwrap();
    let material = value(2 * PART_BYTES + 7);
    assert_eq!(
        backend.write_new(
            version,
            &Material::from_slice(&material),
            until(),
            || Ok(())
        ),
        Ok(())
    );
    entries.crash_after(1);
    assert_eq!(
        backend.delete(version, until(), || Ok(())),
        Err(Failure::OutcomeUnknown)
    );
    assert_eq!(
        layout(&entries, version),
        (false, vec![0, 1, 2]),
        "the header is removed first"
    );
    entries.recover();
    assert!(matches!(backend.read(version), Err(Failure::Missing)));
    assert_eq!(backend.delete(version, until(), || Ok(())), Ok(()));
    assert!(entries.names().is_empty());
}

/// A crash part way through removing the parts leaves a prefix of them, which
/// a following delete reclaims.
#[test]
fn a_delete_interrupted_among_the_parts_is_reclaimed() {
    let (entries, backend, scope) = parted();
    let version = Version::new(scope, Uuid::new_v4()).unwrap();
    assert_eq!(
        backend.write_new(
            version,
            &Material::new(value(MAX_BYTES)),
            until(),
            || Ok(())
        ),
        Ok(())
    );
    // The header, then parts 25 down to 10.
    entries.crash_after(1 + 16);
    assert_eq!(
        backend.delete(version, until(), || Ok(())),
        Err(Failure::OutcomeUnknown)
    );
    assert_eq!(layout(&entries, version), (false, (0..10).collect()));
    entries.recover();
    assert!(matches!(backend.read(version), Err(Failure::Missing)));
    assert_eq!(backend.delete(version, until(), || Ok(())), Ok(()));
    assert!(entries.names().is_empty());
}

/// Deletes that succeed but leave an entry for the next read (seen on the
/// Windows runner) are repeated in a later round; the delete is acknowledged
/// once a read finds nothing.
#[test]
fn an_entry_still_found_after_its_delete_is_deleted_again() {
    let (entries, backend, scope) = parted();
    let version = Version::new(scope, Uuid::new_v4()).unwrap();
    assert_eq!(
        backend.write_new(
            version,
            &Material::new(value(3 * PART_BYTES)),
            until(),
            || Ok(())
        ),
        Ok(())
    );
    entries.leave_after_delete(2);
    assert_eq!(backend.delete(version, until(), || Ok(())), Ok(()));
    assert!(entries.names().is_empty());
    // One round deletes the header and every possible part; a second round
    // ran for the two entries left behind.
    assert!(entries.calls().1 > MAX_PARTS + 1);
}

/// An entry still found after every round is an unknown outcome, and stays
/// for a later delete to reclaim.
#[test]
fn an_entry_found_after_every_delete_round_is_an_unknown_outcome() {
    let (entries, backend, scope) = parted();
    let version = Version::new(scope, Uuid::new_v4()).unwrap();
    assert_eq!(
        backend.write_new(version, &Material::new(value(PART_BYTES)), until(), || Ok(
            ()
        )),
        Ok(())
    );
    entries.leave_after_delete(usize::MAX);
    assert_eq!(
        backend.delete(version, until(), || Ok(())),
        Err(Failure::OutcomeUnknown)
    );
    assert!(!entries.names().is_empty());
    entries.leave_after_delete(0);
    assert_eq!(backend.delete(version, until(), || Ok(())), Ok(()));
    assert!(entries.names().is_empty());
}

#[test]
fn a_part_removed_under_a_header_is_invalid_material() {
    let (entries, backend, scope) = parted();
    let version = Version::new(scope, Uuid::new_v4()).unwrap();
    assert_eq!(
        backend.write_new(
            version,
            &Material::new(value(3 * PART_BYTES)),
            until(),
            || Ok(())
        ),
        Ok(())
    );
    assert!(entries.remove(&part_name(version, 1)));
    assert!(matches!(
        backend.read(version),
        Err(Failure::InvalidMaterial)
    ));
    assert_eq!(
        backend.confirm(version, until(), || Ok(()), |_| Ok(())),
        Err(Failure::InvalidMaterial)
    );
    assert_eq!(backend.delete(version, until(), || Ok(())), Ok(()));
    assert!(entries.names().is_empty());
}

#[test]
fn changed_part_bytes_fail_the_digest_as_invalid_material() {
    for index in [0, 1] {
        let (entries, backend, scope) = parted();
        let version = Version::new(scope, Uuid::new_v4()).unwrap();
        assert_eq!(
            backend.write_new(
                version,
                &Material::new(value(PART_BYTES + 1)),
                until(),
                || Ok(())
            ),
            Ok(())
        );
        assert!(entries.corrupt(&part_name(version, index)));
        assert!(matches!(
            backend.read(version),
            Err(Failure::InvalidMaterial)
        ));
    }
    // A header that is not one.
    let (entries, backend, scope) = parted();
    let version = Version::new(scope, Uuid::new_v4()).unwrap();
    assert_eq!(
        backend.write_new(version, &Material::new(value(10)), until(), || Ok(())),
        Ok(())
    );
    assert!(entries.corrupt(&header_name(version)));
    assert!(matches!(
        backend.read(version),
        Err(Failure::InvalidMaterial)
    ));
}

/// 64 KiB round-trips through the split form; one byte more is refused before
/// the store sees any call.
#[test]
fn split_form_round_trips_64_kib_and_refuses_one_byte_more_before_any_write() {
    let (entries, backend, scope) = parted();
    let fits = Version::new(scope, Uuid::new_v4()).unwrap();
    let largest = value(MAX_BYTES);
    assert_eq!(largest.len(), 65536);
    assert_eq!(
        backend.write_new(fits, &Material::from_slice(&largest), until(), || Ok(())),
        Ok(())
    );
    assert!(backend.read(fits).unwrap().ct_eq(&largest));
    let calls = entries.calls();
    let names = entries.names();
    let over = Version::new(scope, Uuid::new_v4()).unwrap();
    let mut oversized = largest.clone();
    oversized.push(0);
    assert_eq!(
        backend.write_new(over, &Material::new(oversized), until(), || {
            panic!("the guard never runs for an oversized value")
        }),
        Err(Failure::InvalidMaterial)
    );
    assert_eq!(entries.calls(), calls, "no write and no delete");
    assert_eq!(entries.names(), names);
    assert!(matches!(backend.read(over), Err(Failure::Missing)));
}

#[test]
fn header_encodes_length_parts_and_digest_and_refuses_anything_else() {
    let header = Header {
        length: PART_BYTES + 1,
        parts: 2,
        digest: [7; 32],
    };
    let blob = header.encode();
    assert_eq!(blob.len(), 48);
    assert_eq!(&blob[..8], b"CCPARTS1");
    assert_eq!(Header::decode(&blob), Ok(header));
    for broken in [
        Header { parts: 3, ..header },
        Header {
            length: 0,
            parts: 0,
            ..header
        },
        Header {
            length: MAX_BYTES + 1,
            parts: MAX_PARTS + 1,
            ..header
        },
    ] {
        assert_eq!(
            Header::decode(&broken.encode()),
            Err(Failure::InvalidMaterial)
        );
    }
    assert_eq!(Header::decode(&blob[..47]), Err(Failure::InvalidMaterial));
    let mut magic = blob.clone();
    magic[0] = b'X';
    assert_eq!(Header::decode(&magic), Err(Failure::InvalidMaterial));
}

#[test]
fn memory_answers_the_custody_contract() {
    let store = MemoryStore::new(CustodyBackend::SecretService);
    contract::run(&|scope| store.open(scope));
}

/// The read half of the contract against the reference, whose own write
/// places the bytes: the same checks `mounted_secret` answers.
#[test]
fn memory_answers_the_read_only_custody_contract() {
    struct Writes(MemoryStore);
    impl contract::Placement for Writes {
        type Binding = crate::memory::Memory;
        fn open(&self, scope: Scope) -> Result<Self::Binding> {
            self.0.open(scope)
        }
        fn place(&self, binding: &Self::Binding, version: Version, material: &[u8]) {
            binding
                .write_new(version, &Material::from_slice(material), until(), || Ok(()))
                .unwrap();
        }
    }
    contract::run_read_only(&Writes(MemoryStore::new(CustodyBackend::SecretService)));
}

/// Only the read-only binding refuses writes; every platform binding writes.
#[test]
fn only_the_mounted_binding_is_read_only() {
    for (kind, name, writable) in [
        (CustodyBackend::SecretService, "secret_service", true),
        (CustodyBackend::MacosKeychain, "macos_keychain", true),
        (
            CustodyBackend::WindowsCredentialManager,
            "windows_credential_manager",
            true,
        ),
        (CustodyBackend::MountedSecret, "mounted_secret", false),
    ] {
        assert_eq!(kind.as_str(), name);
        assert_eq!(kind.writable(), writable, "{name}");
        assert_eq!(kind.max_bytes(), MAX_BYTES);
    }
    assert_eq!(Failure::ReadOnly.to_string(), "custody binding read-only");
}

#[test]
fn memory_with_the_windows_bound_answers_the_custody_contract() {
    let store = MemoryStore::new(CustodyBackend::WindowsCredentialManager);
    contract::run(&|scope| store.open(scope));
}

#[test]
fn memory_with_the_keychain_bound_answers_the_custody_contract() {
    let store = MemoryStore::new(CustodyBackend::MacosKeychain);
    contract::run(&|scope| store.open(scope));
}

/// A locked or absent store answers `Unavailable` to every operation and is
/// never unlocked or replaced by another store.
#[test]
fn memory_locked_or_absent_store_is_unavailable() {
    let store = MemoryStore::new(CustodyBackend::SecretService);
    let scope = Scope::new(Uuid::new_v4(), Uuid::new_v4()).unwrap();
    let backend = store.open(scope).unwrap();
    let version = Version::new(scope, Uuid::new_v4()).unwrap();
    let until = Instant::now() + Duration::from_secs(1);
    backend
        .write_new(version, &Material::from_slice(b"fictional"), until, || {
            Ok(())
        })
        .unwrap();
    for state in [State::Locked, State::Unavailable] {
        store.set_state(state);
        assert_eq!(backend.state(), state);
        assert!(matches!(backend.read(version), Err(Failure::Unavailable)));
        let fresh = Version::new(scope, Uuid::new_v4()).unwrap();
        assert_eq!(
            backend.write_new(fresh, &Material::from_slice(b"fictional"), until, || Ok(())),
            Err(Failure::Unavailable)
        );
        assert_eq!(
            backend.delete(version, until, || Ok(())),
            Err(Failure::Unavailable)
        );
    }
    store.set_state(State::Available);
    assert!(backend.read(version).unwrap().ct_eq(b"fictional"));
}

#[test]
fn platform_binding_is_fixed_by_the_build_target() {
    let expected = if cfg!(target_os = "linux") {
        Some("secret_service")
    } else if cfg!(target_os = "macos") {
        Some("macos_keychain")
    } else if cfg!(target_os = "windows") {
        Some("windows_credential_manager")
    } else {
        None
    };
    assert_eq!(crate::PLATFORM.map(CustodyBackend::as_str), expected);
    assert_eq!(
        CustodyBackend::WindowsCredentialManager.max_bytes(),
        64 * 1024
    );
    assert_eq!(CustodyBackend::MacosKeychain.max_bytes(), 64 * 1024);
    assert_eq!(CustodyBackend::SecretService.max_bytes(), 64 * 1024);
}

#[test]
fn custody_identities_are_non_nil_and_item_names_carry_only_ids() {
    assert!(Scope::new(Uuid::nil(), Uuid::new_v4()).is_err());
    assert!(Scope::new(Uuid::new_v4(), Uuid::nil()).is_err());
    let scope = Scope::new(Uuid::new_v4(), Uuid::new_v4()).unwrap();
    assert!(Version::new(scope, Uuid::nil()).is_err());
    let version = Version::new(scope, Uuid::new_v4()).unwrap();
    assert_eq!(
        version.item_name(),
        format!(
            "credential/{}/{}/{}",
            scope.authority(),
            scope.allocation(),
            version.id()
        )
    );
    let signing = Scope::approval_signing(scope.authority(), scope.allocation()).unwrap();
    let twin = Version::new(signing, version.id()).unwrap();
    assert_ne!(twin.item_name(), version.item_name());
}

/// The real Secret Service, in a private fixture: its own bus and the
/// qualified GNOME Keyring, never the desktop's.
#[cfg(target_os = "linux")]
#[test]
#[ignore = "requires qualified GNOME Keyring 50.0, dbus-daemon and task-owned TMPDIR"]
fn secret_service_answers_the_custody_contract() {
    let fixture = crate::secret_service::fixture::Fixture::new();
    assert_eq!(
        crate::secret_service::inspect_at(Some(&fixture.socket)),
        State::Available
    );
    contract::run(&|scope| fixture.backend(scope));
}

/// The user's login keychain, with user interaction disallowed.
#[cfg(target_os = "macos")]
#[test]
#[ignore = "touches the platform credential store"]
fn macos_login_keychain_answers_the_custody_contract() {
    contract::run(&crate::keychain::Keychain::open);
}

/// A temporary keychain this test creates, locks and deletes: every operation
/// answers `Unavailable` and no unlock prompt is shown.
#[cfg(target_os = "macos")]
#[test]
#[ignore = "touches the platform credential store"]
fn macos_locked_keychain_is_unavailable_without_a_prompt() {
    use crate::keychain::{Keychain, TemporaryKeychain};
    let directory = tempfile::tempdir().unwrap();
    let temporary = TemporaryKeychain::create(&directory.path().join("custody-test.keychain"))
        .expect("a temporary keychain");
    let scope = Scope::new(Uuid::new_v4(), Uuid::new_v4()).unwrap();
    let until = Instant::now() + Duration::from_secs(10);
    let unlocked = Keychain::with_keychain(scope, temporary.keychain());
    assert_eq!(unlocked.state(), State::Available);
    let version = Version::new(scope, Uuid::new_v4()).unwrap();
    unlocked
        .write_new(version, &Material::from_slice(b"fictional"), until, || {
            Ok(())
        })
        .unwrap();
    assert!(unlocked.read(version).unwrap().ct_eq(b"fictional"));
    temporary.lock().expect("lock the temporary keychain");
    let locked = Keychain::with_keychain(scope, temporary.keychain());
    assert_eq!(locked.state(), State::Locked);
    assert!(matches!(locked.read(version), Err(Failure::Unavailable)));
    let fresh = Version::new(scope, Uuid::new_v4()).unwrap();
    assert_eq!(
        locked.write_new(fresh, &Material::from_slice(b"fictional"), until, || Ok(())),
        Err(Failure::Unavailable)
    );
    assert_eq!(
        locked.delete(version, until, || Ok(())),
        Err(Failure::Unavailable)
    );
    drop(temporary);
}

/// The variable that makes this test binary, started from a launch copy, the
/// child of [`macos_login_keychain_writes_from_a_launch_copy_left_on_disk_and_removed_after_start`];
/// its value names the variant, and the child's answer is the stdout text that
/// starts at it (the harness may print its own text before it on that line).
#[cfg(target_os = "macos")]
const LAUNCH_COPY_CHILD: &str = "CONNECTORS_CUSTODY_LAUNCH_COPY_CHILD";

/// The owner writes custody from a copy of its executable, which it keeps until
/// it stops serving (`connectors-host`, `runtime::artifact::macos`
/// and the owner spawn). This runs one login keychain write, read-back and
/// delete in a child started as the CLI starts the owner: a mode 0500 copy of
/// this test binary in a fresh `connectors-image.*` directory, executed by its
/// path, with an empty environment, in its own session. The copy is left on
/// disk for one child and removed after the start, before the write, for the
/// other. Both answers are printed with every keychain status the binding
/// reported, so one run tells whether the removed executable is what fails
/// the owner's write.
#[cfg(target_os = "macos")]
#[test]
#[ignore = "touches the platform credential store"]
fn macos_login_keychain_writes_from_a_launch_copy_left_on_disk_and_removed_after_start() {
    if let Some(variant) = std::env::var_os(LAUNCH_COPY_CHILD) {
        return launch_copy_child(&variant.to_string_lossy());
    }
    let answers: Vec<(&str, String)> = ["left_on_disk", "removed_after_start"]
        .into_iter()
        .map(|variant| (variant, launch_copy(variant)))
        .collect();
    for (variant, answer) in &answers {
        eprintln!("launch copy {variant}: {answer}");
    }
    let [(_, left), (_, removed)] = &answers[..] else {
        unreachable!()
    };
    // The executable left on disk writes, reads back and deletes.
    assert!(
        left.contains(" write=Ok(()) ")
            && left.contains(" delete=Ok(()) ")
            && left.ends_with(" reported=[]"),
        "a login keychain write from a launch copy left on disk failed: {left}"
    );
    // Pinned macOS fact, which is why the owner keeps its launch copy until it
    // stops serving: SecItemAdd from a process whose executable is gone answers
    // OSStatus 100002 (ENOENT), which the binding reports as outcome unknown.
    assert!(
        removed.contains(" executable_on_disk=Ok(false) ")
            && removed.contains(" write=Err(OutcomeUnknown) ")
            && removed.contains("SecItemAdd answered OSStatus 100002"),
        "a login keychain write from a removed executable no longer fails as pinned: {removed}"
    );
}

/// Starts this test binary from a launch copy, as the CLI starts the owner,
/// and answers the child's line, or what it printed when it gave none.
#[cfg(target_os = "macos")]
fn launch_copy(variant: &str) -> String {
    use std::{
        io::Write,
        os::unix::{fs::PermissionsExt, process::CommandExt},
        process::{Command, Stdio},
    };
    let directory = tempfile::Builder::new()
        .prefix("connectors-image.")
        .tempdir()
        .unwrap();
    let outputs = tempfile::tempdir().unwrap();
    let image = directory.path().join("image");
    // As the host makes a copy: the bytes written into a new file, then mode 0500.
    let bytes = std::fs::read(std::env::current_exe().unwrap()).unwrap();
    std::fs::write(&image, bytes).unwrap();
    std::fs::set_permissions(&image, std::fs::Permissions::from_mode(0o500)).unwrap();
    let (stdout, stderr) = (outputs.path().join("stdout"), outputs.path().join("stderr"));
    let mut command = Command::new(&image);
    command
        .args([
            "--exact",
            "tests::macos_login_keychain_writes_from_a_launch_copy_left_on_disk_and_removed_after_start",
            "--include-ignored",
            "--nocapture",
            "--test-threads=1",
        ])
        .env_clear()
        .env(LAUNCH_COPY_CHILD, variant)
        .stdin(Stdio::piped())
        .stdout(std::fs::File::create(&stdout).unwrap())
        .stderr(std::fs::File::create(&stderr).unwrap());
    // SAFETY: setsid is async-signal-safe; the owner's spawn starts its own
    // session the same way.
    unsafe {
        command.pre_exec(|| {
            if libc::setsid() < 0 {
                return Err(std::io::Error::last_os_error());
            }
            Ok(())
        });
    }
    let mut child = command.spawn().unwrap();
    if variant == "removed_after_start" {
        std::fs::remove_file(&image).unwrap();
    }
    child.stdin.take().unwrap().write_all(b"go\n").unwrap();
    let until = Instant::now() + Duration::from_secs(60);
    let status = loop {
        if let Some(status) = child.try_wait().unwrap() {
            break Some(status);
        }
        if Instant::now() >= until {
            let _ = child.kill();
            let _ = child.wait();
            break None;
        }
        std::thread::sleep(Duration::from_millis(20));
    };
    let stdout = std::fs::read_to_string(&stdout).unwrap_or_default();
    // The libtest harness prints "test <name> ... " before the child's line on
    // the same line, so the answer starts where the marker does, not at a line start.
    match stdout
        .lines()
        .find_map(|line| line.find(LAUNCH_COPY_CHILD).map(|at| &line[at..]))
    {
        Some(line) => line.to_owned(),
        None => format!(
            "no answer from the child (exit {status:?}); stdout: {stdout:?}; stderr: {:?}",
            std::fs::read_to_string(&stderr).unwrap_or_default()
        ),
    }
}

/// The child: waits for the parent's go (after the copy was removed, for that
/// variant), then writes, reads back and deletes one fresh login keychain item.
#[cfg(target_os = "macos")]
fn launch_copy_child(variant: &str) {
    use crate::keychain::Keychain;
    let mut go = String::new();
    std::io::stdin().read_line(&mut go).unwrap();
    let on_disk = std::env::current_exe().map(|path| path.exists());
    let scope = Scope::new(Uuid::new_v4(), Uuid::new_v4()).unwrap();
    let version = Version::new(scope, Uuid::new_v4()).unwrap();
    let keychain = Keychain::open(scope);
    let write = keychain
        .as_ref()
        .map_err(|failure| *failure)
        .and_then(|keychain| {
            keychain.write_new(
                version,
                &Material::from_slice(b"fictional"),
                until(),
                || Ok(()),
            )
        });
    let delete = keychain
        .as_ref()
        .map_err(|failure| *failure)
        .and_then(|keychain| keychain.delete(version, until(), || Ok(())));
    println!(
        "{LAUNCH_COPY_CHILD} variant={variant} executable_on_disk={on_disk:?} write={write:?} delete={delete:?} reported={:?}",
        crate::keychain::reported()
    );
}

/// The Windows Credential Manager, generic credentials persisted on the
/// local machine.
#[cfg(target_os = "windows")]
#[test]
#[ignore = "touches the platform credential store"]
fn windows_credential_manager_answers_the_custody_contract() {
    contract::run(&crate::credential_manager::CredentialManager::open);
}

/// Values of one blob, one byte over it and 64 KiB are split over part
/// entries in the real store and read back; 64 KiB and one byte is refused
/// and nothing is written; a delete leaves no entry.
#[cfg(target_os = "windows")]
#[test]
#[ignore = "touches the platform credential store"]
fn windows_credential_manager_splits_values_up_to_64_kib() {
    use crate::{credential_manager::CredentialManager, parts::Entries};
    let scope = Scope::new(Uuid::new_v4(), Uuid::new_v4()).unwrap();
    let backend = CredentialManager::open(scope).unwrap();
    let present = |name: String| backend.entries().read(&name).unwrap().is_some();
    for (len, parts) in [(PART_BYTES, 1), (PART_BYTES + 1, 2), (MAX_BYTES, 26)] {
        let version = Version::new(scope, Uuid::new_v4()).unwrap();
        let material = value(len);
        assert_eq!(
            backend.write_new(
                version,
                &Material::from_slice(&material),
                until(),
                || Ok(())
            ),
            Ok(()),
            "{len} bytes"
        );
        assert!(backend.read(version).unwrap().ct_eq(&material));
        assert!(present(header_name(version)));
        assert!((0..parts).all(|index| present(part_name(version, index))));
        assert!(!present(part_name(version, parts)));
        assert_eq!(backend.delete(version, until(), || Ok(())), Ok(()));
        assert!(matches!(backend.read(version), Err(Failure::Missing)));
        assert!(!present(header_name(version)));
        assert!((0..MAX_PARTS).all(|index| !present(part_name(version, index))));
    }
    let over = Version::new(scope, Uuid::new_v4()).unwrap();
    assert_eq!(
        backend.write_new(over, &Material::new(value(MAX_BYTES + 1)), until(), || Ok(
            ()
        )),
        Err(Failure::InvalidMaterial)
    );
    assert!(matches!(backend.read(over), Err(Failure::Missing)));
    assert!(!present(part_name(over, 0)));
}

/// Every shipped auth profile's largest material round-trips through the
/// split form in the real Credential Manager.
#[cfg(target_os = "windows")]
#[test]
#[ignore = "touches the platform credential store"]
fn windows_credential_manager_holds_every_shipped_profiles_largest_material() {
    largest_materials_round_trip(&crate::credential_manager::CredentialManager::open);
}
const CATALOG: &str = include_str!("../../../adapters/catalog/src/local.rs");
const KUBERNETES: &str = include_str!("../../../adapters/kubernetes/src/local.rs");
const KUBERNETES_AUTH: &str = include_str!("../../../adapters/kubernetes/src/auth.rs");
const LOKI: &str = include_str!("../../../adapters/loki/src/auth.rs");
const PROMETHEUS: &str = include_str!("../../../adapters/prometheus/src/auth.rs");
const GRAFANA: &str = include_str!("../../../adapters/grafana/src/auth.rs");
const TAVILY: &str = include_str!("../../../adapters/tavily/src/local.rs");
const TAVILY_AUTH: &str = include_str!("../../../adapters/tavily/src/auth.rs");
const SQL: &str = include_str!("../../../adapters/sql/src/local.rs");
const SQL_AUTH: &str = include_str!("../../../adapters/sql/src/auth.rs");
const APPROVAL_KEYS: &str =
    include_str!("../../../crates/connectors-host/src/local/approval_keys.rs");

/// One shipped auth profile's protected entry: the fields the host's terminal
/// entry asks for with their declared bounds, the bound the adapter puts on a
/// document handed in by file or stdin, and the declarations both are read
/// from. A declaration that changes fails the test until this record follows.
struct Profile {
    name: &'static str,
    fields: &'static [(&'static str, usize)],
    document_limit: usize,
    declares: &'static [(&'static str, &'static str)],
}

/// The largest document `protected::terminal` writes for these fields:
/// `{"name":"value",...}` with every value at its bound, printable ASCII.
fn entry_document_bytes(fields: &[(&str, usize)]) -> usize {
    2 + fields
        .iter()
        .map(|(name, max)| name.len() + 2 + 1 + max + 2)
        .sum::<usize>()
        + fields.len().saturating_sub(1)
}

const PROFILES: &[Profile] = &[
    Profile {
        name: "catalog token",
        fields: &[("token", 8192)],
        document_limit: 64 * 1024,
        declares: &[
            (CATALOG, "vec![field(\"token\", &config.auth.label, 8192)]"),
            (CATALOG, "const DOCUMENT_LIMIT: usize = 64 * 1024;"),
        ],
    },
    Profile {
        name: "catalog basic",
        fields: &[("account", 1024), ("token", 8192)],
        document_limit: 64 * 1024,
        declares: &[
            (CATALOG, "field(\"account\", label, ACCOUNT_LIMIT as u32)"),
            (CATALOG, "const ACCOUNT_LIMIT: usize = 1024;"),
            (CATALOG, "field(\"token\", &config.auth.label, 8192),"),
        ],
    },
    Profile {
        name: "catalog oauth2_refresh",
        fields: &[
            ("client_id", 8192),
            ("client_secret", 8192),
            ("refresh_token", 8192),
        ],
        document_limit: 64 * 1024,
        declares: &[
            (CATALOG, "field(\"client_id\", \"OAuth client ID\", 8192)"),
            (
                CATALOG,
                "field(\"client_secret\", \"OAuth client secret\", 8192)",
            ),
            (
                CATALOG,
                "field(\"refresh_token\", &config.auth.label, 8192)",
            ),
        ],
    },
    Profile {
        name: "catalog oauth2_client_credentials",
        fields: &[("client_id", 8192), ("client_secret", 8192)],
        document_limit: 64 * 1024,
        declares: &[(
            CATALOG,
            "field(\"client_secret\", &config.auth.label, 8192)",
        )],
    },
    Profile {
        name: "catalog github_app",
        fields: &[
            ("app_id", 128),
            ("installation_id", 20),
            ("private_key", 16 * 1024),
        ],
        document_limit: 64 * 1024,
        declares: &[
            (
                CATALOG,
                "field(\"app_id\", \"GitHub App ID\", APP_ID_LIMIT as u32)",
            ),
            (CATALOG, "const APP_ID_LIMIT: usize = 128;"),
            (CATALOG, "INSTALLATION_ID_LIMIT as u32,"),
            (CATALOG, "const INSTALLATION_ID_LIMIT: usize = 20;"),
            (
                CATALOG,
                "field(\"private_key\", &config.auth.label, APP_KEY_LIMIT as u32)",
            ),
            (CATALOG, "const APP_KEY_LIMIT: usize = 16 * 1024;"),
        ],
    },
    Profile {
        name: "kubernetes token",
        fields: &[("token", 8192)],
        document_limit: 64 * 1024,
        declares: &[
            (
                KUBERNETES,
                "label: \"Kubernetes bearer token\".into(),\n                max_bytes: 8192,",
            ),
            (
                KUBERNETES_AUTH,
                "pub const DOCUMENT_LIMIT: usize = 64 * 1024;",
            ),
        ],
    },
    Profile {
        name: "loki token",
        fields: &[("token", 8192)],
        document_limit: 16 * 1024,
        declares: &[
            (LOKI, "pub const TOKEN_BYTES: usize = 8192;"),
            (LOKI, "pub const DOCUMENT_LIMIT: usize = 16 * 1024;"),
        ],
    },
    Profile {
        name: "prometheus token",
        fields: &[("token", 8192)],
        document_limit: 16 * 1024,
        declares: &[
            (PROMETHEUS, "pub const TOKEN_BYTES: usize = 8192;"),
            (PROMETHEUS, "pub const DOCUMENT_LIMIT: usize = 16 * 1024;"),
        ],
    },
    Profile {
        name: "grafana token",
        fields: &[("token", 8192)],
        document_limit: 16 * 1024,
        declares: &[
            (GRAFANA, "pub const TOKEN_BYTES: usize = 8192;"),
            (GRAFANA, "pub const DOCUMENT_LIMIT: usize = 16 * 1024;"),
        ],
    },
    Profile {
        name: "tavily api_key",
        fields: &[("api_key", 512)],
        document_limit: 64 * 1024,
        declares: &[
            (
                TAVILY,
                "name: \"api_key\".into(),\n                label: \"Tavily API key\".into(),\n                max_bytes: 512,",
            ),
            (TAVILY_AUTH, "pub const DOCUMENT_LIMIT: usize = 64 * 1024;"),
        ],
    },
    Profile {
        name: "sql password",
        fields: &[("password", 8192)],
        document_limit: 64 * 1024,
        declares: &[
            (
                SQL,
                "name: \"password\".into(),\n                label: config.engine.profile_label().into(),\n                max_bytes: 8192,",
            ),
            (SQL_AUTH, "pub const DOCUMENT_LIMIT: usize = 64 * 1024;"),
        ],
    },
];

/// The largest material each shipped auth profile writes into custody, read
/// from the declarations that set it: the larger of the entry document
/// terminal entry writes with every field at its declared bound and the
/// adapter's limit on a document handed in by file or stdin. The approval
/// signing key the host generates is a 32-byte seed.
fn largest_materials() -> Vec<(&'static str, usize)> {
    let mut largest = Vec::new();
    for profile in PROFILES {
        for (source, declaration) in profile.declares {
            assert!(
                // A Windows checkout has CRLF line endings; declarations are matched
                // as LF text.
                source.replace("\r\n", "\n").contains(declaration),
                "{}: the source no longer declares {declaration:?}; update this record",
                profile.name
            );
        }
        let entry = entry_document_bytes(profile.fields);
        println!(
            "custody material: {:<34} entry document {:>6} bytes; file or stdin document up to {:>6} bytes",
            profile.name, entry, profile.document_limit
        );
        largest.push((profile.name, entry.max(profile.document_limit)));
    }
    assert!(APPROVAL_KEYS.contains("let mut seed = Secret(vec![0; 32]);"));
    println!(
        "custody material: {:<34} {:>6} bytes",
        "approval signing key", SIGNING_SEED_BYTES
    );
    largest
}

/// Every profile's largest material is within the 64 KiB bound and is
/// written, read back exactly and deleted through `open`; so is the signing
/// key.
/// The Credential Manager calls that failed in this test binary, for an
/// `OutcomeUnknown` that the closed `Failure` cannot explain.
fn os_errors() -> String {
    #[cfg(target_os = "windows")]
    {
        format!(
            "{:?}; delete remainders: {:?}",
            crate::credential_manager::OS_ERRORS
                .lock()
                .map(|e| e.clone()),
            crate::parts::REMAINDERS.lock().map(|r| r.clone())
        )
    }
    #[cfg(not(target_os = "windows"))]
    {
        format!(
            "none recorded on this platform; delete remainders: {:?}",
            crate::parts::REMAINDERS.lock().map(|r| r.clone())
        )
    }
}

fn largest_materials_round_trip<B: Backend>(open: &dyn Fn(Scope) -> Result<B>) {
    let (authority, allocation) = (Uuid::new_v4(), Uuid::new_v4());
    for (name, len) in largest_materials() {
        assert!(
            len <= MAX_BYTES,
            "{name}: {len} bytes exceed the custody bound of {MAX_BYTES}"
        );
        let scope = Scope::new(Uuid::new_v4(), Uuid::new_v4()).unwrap();
        let backend = open(scope).unwrap();
        let version = Version::new(scope, Uuid::new_v4()).unwrap();
        let material: Vec<u8> = (0..len).map(|i| b'!' + (i % 94) as u8).collect();
        assert_eq!(
            backend.write_new(
                version,
                &Material::from_slice(&material),
                until(),
                || Ok(())
            ),
            Ok(()),
            "{name}: {len} bytes; failing OS calls: {}",
            os_errors()
        );
        assert!(
            backend.read(version).unwrap().ct_eq(&material),
            "{name}; failing OS calls: {}",
            os_errors()
        );
        assert_eq!(
            backend.delete(version, until(), || Ok(())),
            Ok(()),
            "{name}: delete; failing OS calls: {}",
            os_errors()
        );
    }
    let signing = Scope::approval_signing(authority, allocation).unwrap();
    let backend = open(signing).unwrap();
    let version = Version::new(signing, Uuid::new_v4()).unwrap();
    let seed = [0x5a; SIGNING_SEED_BYTES];
    assert_eq!(
        backend.write_new(version, &Material::from_slice(&seed), until(), || Ok(())),
        Ok(()),
        "signing seed; failing OS calls: {}",
        os_errors()
    );
    assert!(backend.read(version).unwrap().ct_eq(&seed));
    assert_eq!(
        backend.delete(version, until(), || Ok(())),
        Ok(()),
        "signing seed: delete; failing OS calls: {}",
        os_errors()
    );
    println!("custody delete rounds: {}", os_errors());
}

/// The story's measurement, held to the 64 KiB bound of every platform: every
/// shipped profile's largest material round-trips through the split form of
/// the Windows binding, here over a fake entry store.
#[test]
fn largest_material_of_every_shipped_auth_profile_round_trips_through_the_split_form() {
    let entries = FakeEntries::default();
    largest_materials_round_trip(&|scope| Ok(Parted::new(entries.clone(), scope)));
    assert!(entries.names().is_empty());
}

#[cfg(target_os = "linux")]
mod session_keyring {
    use super::*;
    use crate::secret_service::{items_at, orphans_at, remove_authority_at, session_bus_refusal};
    use std::{collections::HashSet, path::Path};

    #[test]
    fn a_test_run_may_not_open_the_session_bus_secret_service() {
        let session = format!("unix:path=/run/user/{}/bus", unsafe { libc::geteuid() });
        let named = session_bus_refusal(None, Some(&session)).expect("refused");
        assert!(named.contains("DBUS_SESSION_BUS_ADDRESS"), "{named}");
        let unset = session_bus_refusal(None, None).expect("refused");
        assert!(unset.contains("secret_service_socket"), "{unset}");
        let selected = session_bus_refusal(
            Some(Path::new(&session["unix:path=".len()..])),
            Some(&session),
        )
        .expect("refused");
        assert!(selected.contains("DBUS_SESSION_BUS_ADDRESS"), "{selected}");
        assert_eq!(
            session_bus_refusal(
                Some(Path::new("/srv/fixture/private/bus")),
                Some("unix:path=/srv/fixture/private/bus")
            ),
            None
        );
    }

    fn put(fixture: &crate::secret_service::fixture::Fixture, scope: Scope) {
        let version = Version::new(scope, Uuid::new_v4()).unwrap();
        let length = if scope.purpose() == crate::Purpose::ApprovalSigning {
            SIGNING_SEED_BYTES
        } else {
            16
        };
        fixture
            .backend(scope)
            .unwrap()
            .write_new(version, &Material::new(vec![7u8; length]), until(), || {
                Ok(())
            })
            .unwrap();
    }

    #[test]
    #[ignore = "requires qualified GNOME Keyring 50.0, dbus-daemon and task-owned TMPDIR"]
    fn custody_writes_reach_the_default_alias_collection() {
        let fixture = crate::secret_service::fixture::Fixture::new();
        let scope = Scope::new(Uuid::new_v4(), Uuid::new_v4()).unwrap();
        put(&fixture, scope);
        let target = fixture.alias_target("default");
        let items = items_at(Some(&fixture.socket)).unwrap();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].collection, target);
        assert!(items[0].path.starts_with(&format!("{target}/")));
    }

    #[test]
    #[ignore = "requires qualified GNOME Keyring 50.0, dbus-daemon and task-owned TMPDIR"]
    fn removing_an_authority_leaves_no_item_carrying_it() {
        let fixture = crate::secret_service::fixture::Fixture::new();
        let (a, b) = (Uuid::new_v4(), Uuid::new_v4());
        for _ in 0..3 {
            put(&fixture, Scope::new(a, Uuid::new_v4()).unwrap());
        }
        put(
            &fixture,
            Scope::approval_signing(a, Uuid::new_v4()).unwrap(),
        );
        put(&fixture, Scope::new(b, Uuid::new_v4()).unwrap());
        let removal = remove_authority_at(Some(&fixture.socket), a).unwrap();
        assert_eq!(removal.deleted.len(), 4);
        assert!(removal.failed.is_empty());
        let left = items_at(Some(&fixture.socket)).unwrap();
        assert_eq!(left.len(), 1);
        assert_eq!(left[0].authority, b.to_string());
    }

    #[test]
    #[ignore = "requires qualified GNOME Keyring 50.0, dbus-daemon and task-owned TMPDIR"]
    fn orphans_are_listed_and_removed_only_on_the_flag() {
        let fixture = crate::secret_service::fixture::Fixture::new();
        let (held, lost) = (Uuid::new_v4(), Uuid::new_v4());
        put(&fixture, Scope::new(held, Uuid::new_v4()).unwrap());
        put(&fixture, Scope::new(lost, Uuid::new_v4()).unwrap());
        put(
            &fixture,
            Scope::approval_signing(lost, Uuid::new_v4()).unwrap(),
        );
        let known = HashSet::from([held]);
        let listed = orphans_at(Some(&fixture.socket), &known, false).unwrap();
        assert_eq!(listed.orphans.len(), 2);
        assert!(
            listed
                .orphans
                .iter()
                .all(|i| i.authority == lost.to_string())
        );
        assert!(listed.removal.is_none());
        assert_eq!(items_at(Some(&fixture.socket)).unwrap().len(), 3);
        let removed = orphans_at(Some(&fixture.socket), &known, true).unwrap();
        assert_eq!(removed.removal.unwrap().deleted.len(), 2);
        let left = items_at(Some(&fixture.socket)).unwrap();
        assert_eq!(left.len(), 1);
        assert_eq!(left[0].authority, held.to_string());
    }
}

// Security review of story:custody-keeps-out-of-the-session-keyring: integrity
// conformance checks of the removal boundary, the default alias and the
// session bus guard's wiring, against a private Secret Service only.
#[cfg(target_os = "linux")]
mod session_keyring_review {
    use super::*;
    use crate::secret_service::{
        SecretService, fixture::Fixture, items_at, orphans_at, remove_authority_at,
    };
    use std::{
        collections::{HashMap, HashSet},
        os::unix::net::UnixStream,
        panic::{AssertUnwindSafe, catch_unwind},
    };
    use zbus::zvariant::{OwnedObjectPath, OwnedValue, Value};

    const LOGIN: &str = "/org/freedesktop/secrets/collection/login";

    /// A raw client of the fixture's Secret Service, for state the binding
    /// never builds itself: a second collection, a moved alias and items of
    /// another application.
    struct Raw {
        connection: zbus::blocking::Connection,
        session: OwnedObjectPath,
    }

    impl Raw {
        fn new(fixture: &Fixture) -> Self {
            let connection = zbus::blocking::connection::Builder::async_io_unix_stream(
                UnixStream::connect(&fixture.socket).unwrap(),
            )
            .build()
            .unwrap();
            let reply = connection
                .call_method(
                    Some("org.freedesktop.secrets"),
                    "/org/freedesktop/secrets",
                    Some("org.freedesktop.Secret.Service"),
                    "OpenSession",
                    &("plain", Value::from("")),
                )
                .unwrap();
            let (_, session): (OwnedValue, OwnedObjectPath) = reply.body().deserialize().unwrap();
            Self {
                connection,
                session,
            }
        }

        /// A new unlocked collection, created without a prompt.
        fn create_collection(&self, label: &str) -> String {
            let mut properties: HashMap<&str, Value<'_>> = HashMap::new();
            properties.insert(
                "org.freedesktop.Secret.Collection.Label",
                Value::from(label),
            );
            let master = (
                &self.session,
                Vec::<u8>::new(),
                b"fictional-review-password".to_vec(),
                "text/plain",
            );
            let reply = self
                .connection
                .call_method(
                    Some("org.freedesktop.secrets"),
                    "/org/freedesktop/secrets",
                    Some("org.gnome.keyring.InternalUnsupportedGuiltRiddenInterface"),
                    "CreateWithMasterPassword",
                    &(properties, master),
                )
                .unwrap();
            let path: OwnedObjectPath = reply.body().deserialize().unwrap();
            path.as_str().to_owned()
        }

        fn set_alias(&self, alias: &str, collection: &str) {
            self.connection
                .call_method(
                    Some("org.freedesktop.secrets"),
                    "/org/freedesktop/secrets",
                    Some("org.freedesktop.Secret.Service"),
                    "SetAlias",
                    &(alias, OwnedObjectPath::try_from(collection).unwrap()),
                )
                .unwrap();
        }

        /// An item carrying exactly `attributes`, as any application could write.
        fn create_item(&self, collection: &str, attributes: &[(&str, &str)]) -> String {
            let attributes: HashMap<String, String> = attributes
                .iter()
                .map(|(k, v)| ((*k).to_owned(), (*v).to_owned()))
                .collect();
            let mut properties: HashMap<&str, Value<'_>> = HashMap::new();
            properties.insert("org.freedesktop.Secret.Item.Label", Value::from("review"));
            properties.insert(
                "org.freedesktop.Secret.Item.Attributes",
                Value::from(attributes),
            );
            let secret = (
                &self.session,
                Vec::<u8>::new(),
                b"fictional".to_vec(),
                "application/octet-stream",
            );
            let reply = self
                .connection
                .call_method(
                    Some("org.freedesktop.secrets"),
                    collection,
                    Some("org.freedesktop.Secret.Collection"),
                    "CreateItem",
                    &(properties, secret, false),
                )
                .unwrap();
            let (item, prompt): (OwnedObjectPath, OwnedObjectPath) =
                reply.body().deserialize().unwrap();
            assert_eq!(prompt.as_str(), "/");
            item.as_str().to_owned()
        }

        /// Whether the item still exists in `collection`.
        fn holds(&self, collection: &str, item: &str) -> bool {
            let proxy = zbus::blocking::Proxy::new(
                &self.connection,
                "org.freedesktop.secrets",
                collection,
                "org.freedesktop.Secret.Collection",
            )
            .unwrap();
            let items: Vec<OwnedObjectPath> = proxy.get_property("Items").unwrap();
            items.iter().any(|i| i.as_str() == item)
        }
    }

    fn write(fixture: &Fixture, scope: Scope, byte: u8) -> Version {
        let version = Version::new(scope, Uuid::new_v4()).unwrap();
        let length = if scope.purpose() == crate::Purpose::ApprovalSigning {
            SIGNING_SEED_BYTES
        } else {
            16
        };
        fixture
            .backend(scope)
            .unwrap()
            .write_new(version, &Material::new(vec![byte; length]), until(), || {
                Ok(())
            })
            .unwrap();
        version
    }

    /// The panic text of `call`, which must panic.
    fn refusal(call: impl FnOnce()) -> String {
        let payload = catch_unwind(AssertUnwindSafe(call)).expect_err("guard did not refuse");
        payload
            .downcast_ref::<String>()
            .cloned()
            .or_else(|| payload.downcast_ref::<&str>().map(|s| (*s).to_owned()))
            .unwrap_or_default()
    }

    /// Invariant 3, wiring: every write-capable or inventory entry point
    /// refuses the session bus before connecting, not only the pure helper.
    /// A guard removed from any one call site turns this red.
    #[test]
    fn every_entry_point_refuses_the_session_bus_before_connecting() {
        let scope = Scope::new(Uuid::new_v4(), Uuid::new_v4()).unwrap();
        let named = "refusing to open the session bus Secret Service";
        let open = refusal(|| {
            let _ = SecretService::open_at(scope, None);
        });
        assert!(open.contains(named), "open_at: {open}");
        let open = refusal(|| {
            let _ = SecretService::open(scope);
        });
        assert!(open.contains(named), "open: {open}");
        let listed = refusal(|| {
            let _ = items_at(None);
        });
        assert!(listed.contains(named), "items_at: {listed}");
        let removed = refusal(|| {
            let _ = remove_authority_at(None, Uuid::new_v4());
        });
        assert!(removed.contains(named), "remove_authority_at: {removed}");
        let orphans = refusal(|| {
            let _ = orphans_at(None, &HashSet::new(), false);
        });
        assert!(orphans.contains(named), "orphans_at: {orphans}");
    }

    /// Invariants 1 and 2 on one private daemon whose `default` alias is moved
    /// off `login`: a new write lands behind the alias, the item already in
    /// `login` still reads, and removing the authority deletes both and
    /// nothing else: not another spelling of the id, not a longer id it
    /// prefixes, not a Connectors item without the attribute, and not another
    /// application's item carrying the same `connectors.authority`.
    #[test]
    #[ignore = "requires qualified GNOME Keyring 50.0, dbus-daemon and task-owned TMPDIR"]
    fn moved_alias_writes_behind_it_reads_login_and_removal_stays_inside_the_authority() {
        let fixture = Fixture::new();
        let raw = Raw::new(&fixture);
        let authority = Uuid::new_v4();
        let wanted = authority.to_string();
        let old_scope = Scope::new(authority, Uuid::new_v4()).unwrap();
        let old = write(&fixture, old_scope, 3);
        assert_eq!(fixture.alias_target("default"), LOGIN);

        let elsewhere = raw.create_collection("elsewhere");
        assert_ne!(elsewhere, LOGIN);
        raw.set_alias("default", &elsewhere);
        assert_eq!(fixture.alias_target("default"), elsewhere);

        let new_scope = Scope::approval_signing(authority, Uuid::new_v4()).unwrap();
        write(&fixture, new_scope, 5);
        let items = items_at(Some(&fixture.socket)).unwrap();
        let mine: Vec<_> = items.iter().filter(|i| i.authority == wanted).collect();
        assert_eq!(mine.len(), 2, "{items:?}");
        assert!(
            mine.iter()
                .any(|i| i.collection == elsewhere && i.path.starts_with(&format!("{elsewhere}/"))),
            "the new write is not behind the alias: {items:?}"
        );
        assert!(
            mine.iter().any(|i| i.collection == LOGIN),
            "the login item is not listed: {items:?}"
        );
        let read = fixture.backend(old_scope).unwrap().read(old).unwrap();
        assert!(read.ct_eq(&[3u8; 16]), "the login item no longer reads");

        let credential = "org.beyond10x.Connectors.Credential";
        let upper = wanted.to_uppercase();
        let longer = format!("{wanted}0");
        let keep = [
            (
                LOGIN.to_owned(),
                raw.create_item(
                    LOGIN,
                    &[("xdg:schema", credential), ("connectors.authority", &upper)],
                ),
            ),
            (
                elsewhere.clone(),
                raw.create_item(
                    &elsewhere,
                    &[
                        ("xdg:schema", credential),
                        ("connectors.authority", &longer),
                    ],
                ),
            ),
            (
                elsewhere.clone(),
                raw.create_item(&elsewhere, &[("xdg:schema", credential)]),
            ),
            (
                LOGIN.to_owned(),
                raw.create_item(
                    LOGIN,
                    &[
                        ("xdg:schema", "org.example.Unrelated"),
                        ("connectors.authority", &wanted),
                    ],
                ),
            ),
            (
                elsewhere.clone(),
                raw.create_item(
                    &elsewhere,
                    &[
                        ("xdg:schema", "org.example.Unrelated"),
                        ("connectors.authority", &wanted),
                    ],
                ),
            ),
        ];

        let removal = remove_authority_at(Some(&fixture.socket), authority).unwrap();
        assert!(removal.failed.is_empty(), "{removal:?}");
        assert_eq!(removal.deleted.len(), 2, "{removal:?}");
        assert!(
            removal.deleted.iter().all(|i| i.authority == wanted),
            "{removal:?}"
        );
        let left = items_at(Some(&fixture.socket)).unwrap();
        assert!(left.iter().all(|i| i.authority != wanted), "{left:?}");
        for (collection, item) in &keep {
            assert!(raw.holds(collection, item), "{item} was deleted");
        }
    }

    /// Acceptance 3, literally: a private daemon that serves only the alias's
    /// collection (no `login` collection at all) receives the write.
    #[test]
    #[ignore = "requires qualified GNOME Keyring 50.0, dbus-daemon and task-owned TMPDIR"]
    fn a_daemon_without_login_receives_the_write_behind_the_alias() {
        let fixture = Fixture::new();
        let raw = Raw::new(&fixture);
        let elsewhere = raw.create_collection("onlyalias");
        raw.set_alias("default", &elsewhere);
        let reply = raw
            .connection
            .call_method(
                Some("org.freedesktop.secrets"),
                LOGIN,
                Some("org.freedesktop.Secret.Collection"),
                "Delete",
                &(),
            )
            .unwrap();
        let prompt: OwnedObjectPath = reply.body().deserialize().unwrap();
        assert_eq!(prompt.as_str(), "/", "login delete prompted");
        let scope = Scope::new(Uuid::new_v4(), Uuid::new_v4()).unwrap();
        write(&fixture, scope, 9);
        let items = items_at(Some(&fixture.socket)).unwrap();
        assert_eq!(items.len(), 1, "{items:?}");
        assert_eq!(items[0].collection, elsewhere);
    }
}
