use crate::{
    Backend, CustodyBackend, Failure, MAX_BYTES, Material, Result, SIGNING_SEED_BYTES, Scope,
    State, Version, contract,
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
            "{name}: {len} bytes"
        );
        assert!(backend.read(version).unwrap().ct_eq(&material), "{name}");
        assert_eq!(backend.delete(version, until(), || Ok(())), Ok(()));
    }
    let signing = Scope::approval_signing(authority, allocation).unwrap();
    let backend = open(signing).unwrap();
    let version = Version::new(signing, Uuid::new_v4()).unwrap();
    let seed = [0x5a; SIGNING_SEED_BYTES];
    assert_eq!(
        backend.write_new(version, &Material::from_slice(&seed), until(), || Ok(())),
        Ok(())
    );
    assert!(backend.read(version).unwrap().ct_eq(&seed));
    assert_eq!(backend.delete(version, until(), || Ok(())), Ok(()));
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
