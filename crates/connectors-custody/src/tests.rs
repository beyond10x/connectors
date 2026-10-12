use crate::{
    Backend, CustodyBackend, Failure, Material, Scope, State, Version, WINDOWS_MAX_BYTES, contract,
    memory::MemoryStore,
};
use std::time::{Duration, Instant};
use uuid::Uuid;

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
    assert_eq!(CustodyBackend::WindowsCredentialManager.max_bytes(), 2560);
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

/// 2,560 bytes are stored and read back; 2,561 bytes are refused before any
/// call and nothing is written.
#[cfg(target_os = "windows")]
#[test]
#[ignore = "touches the platform credential store"]
fn windows_credential_manager_holds_2560_bytes_and_refuses_2561() {
    use crate::credential_manager::CredentialManager;
    let scope = Scope::new(Uuid::new_v4(), Uuid::new_v4()).unwrap();
    let backend = CredentialManager::open(scope).unwrap();
    let until = Instant::now() + Duration::from_secs(10);
    let fits = Version::new(scope, Uuid::new_v4()).unwrap();
    let largest: Vec<u8> = (0..WINDOWS_MAX_BYTES).map(|i| i as u8).collect();
    assert_eq!(largest.len(), 2560);
    assert_eq!(
        backend.write_new(fits, &Material::from_slice(&largest), until, || Ok(())),
        Ok(())
    );
    assert!(backend.read(fits).unwrap().ct_eq(&largest));
    let over = Version::new(scope, Uuid::new_v4()).unwrap();
    let mut oversized = largest.clone();
    oversized.push(0);
    assert_eq!(
        backend.write_new(over, &Material::new(oversized), until, || Ok(())),
        Err(Failure::InvalidMaterial)
    );
    assert!(matches!(backend.read(over), Err(Failure::Missing)));
    assert_eq!(backend.delete(fits, until, || Ok(())), Ok(()));
    assert!(matches!(backend.read(fits), Err(Failure::Missing)));
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

/// The story's first measurement: the largest material each shipped auth
/// profile writes into custody, against the Windows Credential Manager bound.
/// The entry document is what terminal entry writes with every field at its
/// declared bound; a document handed in by file or stdin is bounded only by
/// the adapter's document limit, larger still. The approval signing key the
/// host generates is a 32-byte seed.
#[test]
fn largest_material_of_every_shipped_auth_profile_fits_the_windows_bound() {
    let mut over = Vec::new();
    for profile in PROFILES {
        for (source, declaration) in profile.declares {
            assert!(
                source.contains(declaration),
                "{}: the source no longer declares {declaration:?}; update this record",
                profile.name
            );
        }
        let entry = entry_document_bytes(profile.fields);
        println!(
            "custody material: {:<34} entry document {:>6} bytes; file or stdin document up to {:>6} bytes",
            profile.name, entry, profile.document_limit
        );
        if entry > WINDOWS_MAX_BYTES {
            over.push(format!("{} ({entry} bytes)", profile.name));
        }
    }
    assert!(APPROVAL_KEYS.contains("let mut seed = Secret(vec![0; 32]);"));
    println!(
        "custody material: {:<34} {:>6} bytes",
        "approval signing key", 32
    );
    assert!(
        over.is_empty(),
        "shipped auth profiles whose largest entry exceeds the Windows bound of \
         {WINDOWS_MAX_BYTES} bytes: {}",
        over.join(", ")
    );
}
