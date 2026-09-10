use super::*;
use crate::local::filesystem;
use std::{
    io::Write,
    os::unix::{fs::PermissionsExt, net::UnixStream},
    path::PathBuf,
    process::{Child, Command, Stdio},
    sync::{Arc, Barrier, atomic::Ordering},
    time::{Duration, Instant},
};

// These fixtures own every process and path they touch. They never connect to
// the desktop bus, use a user's existing collection, or invoke an unlock prompt.
struct OwnedChild(Child);
impl Drop for OwnedChild {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

pub(crate) struct Fixture {
    daemon: Option<OwnedChild>,
    _bus: OwnedChild,
    pub(crate) root: tempfile::TempDir,
    pub(crate) socket: PathBuf,
    data: PathBuf,
    incarnation: u32,
}

impl Fixture {
    pub(crate) fn new() -> Self {
        let temporary = std::env::var_os("TMPDIR").expect("task-owned TMPDIR required");
        let root = tempfile::Builder::new()
            .prefix("secret-service-")
            .tempdir_in(temporary)
            .unwrap();
        // Production socket admission requires a private parent; direct custody
        // fixtures previously bypassed that transport admission via connect().
        std::fs::set_permissions(root.path(), std::fs::Permissions::from_mode(0o700)).unwrap();
        let socket = root.path().join("bus");
        let data = root.path().join("data");
        filesystem::directory(&data.join("keyrings"), true, true).unwrap();
        filesystem::directory(&root.path().join("home"), true, true).unwrap();
        let bus = OwnedChild(
            Command::new("/usr/bin/dbus-daemon")
                .args(["--session", "--nofork", "--nopidfile"])
                .arg(format!("--address=unix:path={}", socket.display()))
                .env_clear()
                .env("PATH", "/usr/bin")
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()
                .unwrap(),
        );
        let until = Instant::now() + Duration::from_secs(5);
        while UnixStream::connect(&socket).is_err() {
            assert!(Instant::now() < until, "private fixture bus unavailable");
            std::thread::sleep(Duration::from_millis(20));
        }
        let mut fixture = Self {
            daemon: None,
            _bus: bus,
            root,
            socket,
            data,
            incarnation: 0,
        };
        fixture.start(true);
        fixture
    }

    pub(crate) fn start(&mut self, unlock: bool) {
        assert!(self.daemon.is_none());
        self.incarnation += 1;
        let runtime = self
            .root
            .path()
            .join(format!("runtime-{}", self.incarnation));
        filesystem::directory(&runtime, true, true).unwrap();
        let mut command = Command::new("/usr/bin/gnome-keyring-daemon");
        command
            .args([
                "--foreground",
                "--components=secrets",
                "--control-directory",
            ])
            .arg(&runtime)
            .env_clear()
            .env("PATH", "/usr/bin")
            .env("HOME", self.root.path().join("home"))
            .env("XDG_DATA_HOME", &self.data)
            .env("XDG_RUNTIME_DIR", &runtime)
            .env(
                "DBUS_SESSION_BUS_ADDRESS",
                format!("unix:path={}", self.socket.display()),
            )
            .stdout(Stdio::null())
            .stderr(Stdio::null());
        if unlock {
            command.arg("--unlock").stdin(Stdio::piped());
        } else {
            command.stdin(Stdio::null());
        }
        let mut daemon = OwnedChild(command.spawn().unwrap());
        if unlock {
            // Fictional fixture password, supplied only over the owned pipe.
            daemon
                .0
                .stdin
                .take()
                .unwrap()
                .write_all(b"connectors-disposable-keyring-password")
                .unwrap();
        }
        self.daemon = Some(daemon);
        let expected = if unlock {
            State::Available
        } else {
            State::Locked
        };
        let until = Instant::now() + Duration::from_secs(10);
        loop {
            let state = UnixStream::connect(&self.socket)
                .ok()
                .and_then(|s| Service::connect(s).ok())
                .and_then(|s| s.state().ok());
            if state == Some(expected) {
                break;
            }
            assert!(
                Instant::now() < until,
                "private fixture collection did not become ready"
            );
            assert!(
                self.daemon
                    .as_mut()
                    .unwrap()
                    .0
                    .try_wait()
                    .unwrap()
                    .is_none(),
                "fixture daemon exited"
            );
            std::thread::sleep(Duration::from_millis(20));
        }
    }

    pub(crate) fn stop(&mut self) {
        drop(self.daemon.take());
    }
    fn store(&self, scope: Scope) -> Result<Store> {
        Store::connect(UnixStream::connect(&self.socket).unwrap(), scope)
    }
}

fn scope() -> Scope {
    Scope::new(Uuid::new_v4(), Uuid::new_v4()).unwrap()
}
fn version(scope: Scope) -> Version {
    Version::new(scope, Uuid::new_v4()).unwrap()
}

#[test]
fn private_custody_identities_are_non_nil() {
    assert!(Scope::new(Uuid::nil(), Uuid::new_v4()).is_err());
    assert!(Scope::new(Uuid::new_v4(), Uuid::nil()).is_err());
    assert!(Version::new(scope(), Uuid::nil()).is_err());
}

#[test]
#[ignore = "requires qualified GNOME Keyring 50.0, dbus-daemon and task-owned TMPDIR"]
fn disposable_secret_service_restart_and_failures() {
    let mut fixture = Fixture::new();
    let own_scope = scope();
    let first = version(own_scope);
    let second = version(own_scope);
    let orphan = version(own_scope);
    let first_bytes = Secret(b"fictional-credential-one\0binary".to_vec());
    let second_bytes = Secret(vec![0x59; MAX_BYTES]);
    let store = fixture
        .store(own_scope)
        .expect("qualified disposable service");
    assert!(matches!(
        store.read(version(own_scope)),
        Err(Failure::Missing)
    ));
    assert_eq!(
        store.write_new(first, &Secret(Vec::new())),
        Err(Failure::InvalidMaterial)
    );
    assert_eq!(
        store.write_new(first, &Secret(vec![1; MAX_BYTES + 1])),
        Err(Failure::InvalidMaterial)
    );
    let written = store.write_new(first, &first_bytes);
    assert_eq!(
        written,
        Ok(()),
        "lookup={:?} read={:?} barrier={:?}",
        store.find(first).map(|v| v.is_some()),
        store.read(first).map(|_| ()),
        store.persistence.synchronize()
    );
    assert_eq!(store.write_new(second, &second_bytes), Ok(()));
    assert_eq!(
        store.write_new(first, &second_bytes),
        Err(Failure::Conflict)
    );
    assert_eq!(store.read(first).unwrap().0, first_bytes.0);
    let other = fixture.store(scope()).unwrap();
    assert!(matches!(other.read(first), Err(Failure::Denied)));
    assert_eq!(other.write_new(first, &first_bytes), Err(Failure::Denied));
    assert_eq!(other.delete_for_qualification(first), Err(Failure::Denied));

    // Changed file ownership permissions refuse before transfer. Restoring the
    // path does not fabricate a version for a write that was never attempted.
    let unsafe_version = version(own_scope);
    let filename = fixture.data.join("keyrings/login.keyring");
    std::fs::set_permissions(&filename, std::fs::Permissions::from_mode(0o644)).unwrap();
    assert_eq!(
        store.write_new(unsafe_version, &first_bytes),
        Err(Failure::Unavailable)
    );
    std::fs::set_permissions(&filename, std::fs::Permissions::from_mode(0o600)).unwrap();
    assert!(matches!(store.read(unsafe_version), Err(Failure::Missing)));

    // A failed barrier after a successful CreateItem is uncertainty, never a
    // durable acknowledgement. Its exact candidate identity survives in caller
    // metadata; reusing the write path refuses instead of replacing it.
    store.persistence.fail_sync.store(true, Ordering::SeqCst);
    assert_eq!(
        store.write_new(orphan, &first_bytes),
        Err(Failure::OutcomeUnknown)
    );
    store.persistence.fail_sync.store(false, Ordering::SeqCst);
    assert_eq!(
        store.write_new(orphan, &second_bytes),
        Err(Failure::Conflict)
    );

    // Failure while the service attempts its native rename is also never acked.
    std::fs::set_permissions(
        fixture.data.join("keyrings"),
        std::fs::Permissions::from_mode(0o500),
    )
    .unwrap();
    let refused = version(own_scope);
    assert_eq!(
        store.write_new(refused, &first_bytes),
        Err(Failure::OutcomeUnknown)
    );
    std::fs::set_permissions(
        fixture.data.join("keyrings"),
        std::fs::Permissions::from_mode(0o700),
    )
    .unwrap();

    // SIGKILL, then a new service owner. Old capabilities cannot follow the name
    // to the replacement, and an existing locked collection is unavailable.
    fixture.stop();
    assert!(matches!(store.read(first), Err(Failure::Unavailable)));
    fixture.start(false);
    assert!(matches!(
        fixture.store(own_scope),
        Err(Failure::Unavailable)
    ));
    fixture.stop();
    fixture.start(true);
    assert!(matches!(store.read(first), Err(Failure::Unavailable)));
    let restored = fixture.store(own_scope).unwrap();
    assert_eq!(restored.read(first).unwrap().0, first_bytes.0);
    assert_eq!(restored.read(second).unwrap().0, second_bytes.0);
    assert_eq!(restored.read(orphan).unwrap().0, first_bytes.0);
    assert!(matches!(restored.read(refused), Err(Failure::Missing)));

    // Independent store handles serialize immutable writes across the native
    // SearchItems/CreateItem gap. A duplicate contender cannot overwrite bytes.
    let contested = version(own_scope);
    let barrier = Arc::new(Barrier::new(2));
    let threads: Vec<_> = (0..2)
        .map(|i| {
            let store = fixture.store(own_scope).unwrap();
            let barrier = barrier.clone();
            std::thread::spawn(move || {
                barrier.wait();
                store.write_new(contested, &Secret(vec![i + 1]))
            })
        })
        .collect();
    let results: Vec<_> = threads.into_iter().map(|t| t.join().unwrap()).collect();
    assert_eq!(results.iter().filter(|r| r.is_ok()).count(), 1);
    assert_eq!(
        results
            .iter()
            .filter(|r| **r == Err(Failure::Conflict))
            .count(),
        1
    );

    restored.delete_for_qualification(second).unwrap();
    restored.delete_for_qualification(second).unwrap();
    fixture.stop();
    fixture.start(true);
    let final_store = fixture.store(own_scope).unwrap();
    assert!(matches!(final_store.read(second), Err(Failure::Missing)));
    assert_eq!(final_store.read(first).unwrap().0, first_bytes.0);

    // Native metadata tampering cannot widen the scoped resolver's authority.
    let changed = version(own_scope);
    final_store.write_new(changed, &first_bytes).unwrap();
    let item = final_store.find(changed).unwrap().unwrap();
    let proxy = final_store
        .service
        .proxy(item.as_str(), ITEM_INTERFACE)
        .unwrap();
    let mut attrs = final_store.attributes(changed);
    attrs.insert("unexpected.attribute", "fictional".to_owned());
    proxy.set_property("Attributes", attrs).unwrap();
    assert!(matches!(final_store.read(changed), Err(Failure::Denied)));
    // This fixture covers the backend alone. Registry publication/retirement and
    // CLI status/revoke are exercised by the composition fixture below.
}

#[test]
#[ignore = "requires qualified GNOME Keyring 50.0, dbus-daemon and task-owned TMPDIR"]
fn disposable_changed_keyring_format_is_refused_before_transfer() {
    let fixture = Fixture::new();
    let own_scope = scope();
    let store = fixture.store(own_scope).unwrap();
    // Inject a non-encrypted backing-file format in the disposable fixture.
    // No native credential is written to this file, and no desktop file is used.
    std::fs::OpenOptions::new()
        .write(true)
        .truncate(true)
        .open(fixture.data.join("keyrings/login.keyring"))
        .unwrap()
        .write_all(b"[keyring]\ndisplay-name=Unprotected fixture\n")
        .unwrap();
    assert!(matches!(
        fixture.store(own_scope),
        Err(Failure::Unavailable)
    ));
    assert_eq!(
        store.write_new(version(own_scope), &Secret(b"fictional".to_vec())),
        Err(Failure::Unavailable)
    );
}

#[test]
#[ignore = "requires qualified GNOME, dbus-daemon, task-owned TMPDIR and CONNECTORS_TEST_CLI"]
fn disposable_registry_publication_cli_and_retirement_restart() {
    use crate::local::{
        config::{Config, Paths},
        registry::{self, Registry},
    };
    use std::collections::BTreeSet;
    let binary = std::env::var_os("CONNECTORS_TEST_CLI").expect("built production CLI required");
    let mut fixture = Fixture::new();
    let paths = Paths::resolve(
        Some(&fixture.root.path().join("config/config.toml")),
        Some(&fixture.root.path().join("state")),
    )
    .unwrap();
    Config::initialize(&paths).unwrap();
    let config = std::fs::read_to_string(&paths.config).unwrap()
        + &format!(
            "\n[adapters.fixture]\ninstance_id='fixture-instance'\nadapter_id='fixture-adapter'\nconfiguration_revision='cfg-1'\nprotocol='v1alpha1'\n[adapters.fixture.executable]\npath='/not-installed/must-not-start'\nsha256='{}'\nargs=[]\n",
            "a".repeat(64)
        );
    std::fs::write(&paths.config, config).unwrap();
    let cli = |args: &[&str]| {
        // Only acquisition status and revoke: neither accesses any keyring bus.
        let output = Command::new(&binary)
            .args(["--output", "json", "--config"])
            .arg(&paths.config)
            .arg("--state-dir")
            .arg(&paths.state)
            .args(["connections"])
            .args(args)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "production CLI refused fixture metadata"
        );
        assert!(output.stderr.is_empty());
        let value: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
        assert_eq!(value["ok"], true);
        assert!(!String::from_utf8_lossy(&output.stdout).contains("fictional-material"));
        value["result"].clone()
    };
    let binding = registry::Binding {
        instance_id: "fixture-instance".into(),
        adapter_id: "fixture-adapter".into(),
        configuration_revision: "cfg-1".into(),
        provider_authority: "https://fixture.invalid".into(),
        profile: registry::StaticProfile {
            id: "fixture-token".into(),
            revision: "profile-1".into(),
            purpose: registry::Purpose::DelegatedUser,
            subject: registry::Subject::User,
            minimum_scopes: BTreeSet::new(),
            evidence_lifetime_ms: 60_000,
        },
    };
    let baseline = |now| registry::ValidatedBaseline {
        identity: registry::ExternalIdentity {
            kind: "fixture".into(),
            subject: "one".into(),
        },
        granted_scopes: Some(BTreeSet::new()),
        credential_expires_at_ms: None,
        collected_at_ms: now,
        valid_until_ms: now + 60_000,
    };
    let clock = connectors_sdk::now_ms;
    let registry = Registry::new(&paths.state);
    let acquisition = registry.begin(&binding, clock()).unwrap();
    let acquisition_ref = acquisition.reference().to_owned();
    let claim = registry.consume(acquisition, clock()).unwrap();
    let material = Secret(b"fictional-material".to_vec());
    let now = clock();
    let prepared = registry
        .prepare(&claim, baseline(now), material.0.len(), now)
        .unwrap();
    let exact = prepared.version();
    let store = fixture.store(exact.scope()).unwrap();
    let connection = registry
        .store_and_publish(prepared, &material, &store, clock)
        .unwrap();
    drop(store);
    drop(registry);
    fixture.stop();
    fixture.start(false);
    assert!(matches!(
        fixture.store(exact.scope()),
        Err(Failure::Unavailable)
    ));
    fixture.stop();
    fixture.start(true);
    let registry = Registry::new(&paths.state);
    let captured = registry
        .capture_read(
            &binding,
            &connection,
            &BTreeSet::new(),
            clock(),
            clock() + 30_000,
        )
        .unwrap();
    assert!(captured.version() == exact);
    let store = fixture.store(captured.version().scope()).unwrap();
    assert_eq!(store.read(captured.version()).unwrap().0, material.0);
    let dispatched = registry.dispatch_read(captured, clock()).unwrap();
    registry.release_read(dispatched, clock()).unwrap();
    let status = cli(&[
        "status",
        "--adapter",
        "fixture",
        "--acquisition",
        &acquisition_ref,
    ]);
    assert_eq!(status["acquisition"]["state"], "completed");
    assert_eq!(status["acquisition"]["connection"], connection);
    let observed = registry
        .describe(
            "fixture-instance",
            "fixture-adapter",
            "cfg-1",
            &connection,
            clock(),
            true,
        )
        .unwrap();
    let revoked = cli(&[
        "revoke",
        "--adapter",
        "fixture",
        "--connection",
        &connection,
        "--expected-revision",
        &observed.revision,
    ]);
    assert_eq!(revoked["local_state"], "revoked");
    assert_eq!(revoked["provider_outcome"], "not_requested");
    let repeated = cli(&[
        "revoke",
        "--adapter",
        "fixture",
        "--connection",
        &connection,
        "--expected-revision",
        &observed.revision,
    ]);
    assert_eq!(repeated["revision"], revoked["revision"]);

    // A failed physical acknowledgement leaves a private candidate, never a
    // fabricated connection. Its exact material can still exist after restart.
    let acquisition = registry.begin(&binding, clock()).unwrap();
    let failed_ref = acquisition.reference().to_owned();
    let claim = registry.consume(acquisition, clock()).unwrap();
    let now = clock();
    let prepared = registry
        .prepare(&claim, baseline(now), material.0.len(), now)
        .unwrap();
    let unknown = prepared.version();
    let unknown_store = fixture.store(unknown.scope()).unwrap();
    unknown_store
        .persistence
        .fail_sync
        .store(true, Ordering::SeqCst);
    assert_eq!(
        registry.store_and_publish(prepared, &material, &unknown_store, clock),
        Err(registry::Failure::OutcomeUnknown)
    );
    unknown_store
        .persistence
        .fail_sync
        .store(false, Ordering::SeqCst);
    assert_eq!(
        cli(&[
            "status",
            "--adapter",
            "fixture",
            "--acquisition",
            &failed_ref
        ])["acquisition"]["state"],
        "pending"
    );
    registry.fail(&claim, clock()).unwrap();
    assert_eq!(
        cli(&[
            "status",
            "--adapter",
            "fixture",
            "--acquisition",
            &failed_ref
        ])["acquisition"]["reason"],
        "rejected"
    );
    let acquisition = registry.begin(&binding, clock()).unwrap();
    let claim = registry.consume(acquisition, clock()).unwrap();
    let now = clock();
    let paused = registry
        .prepare(&claim, baseline(now), material.0.len(), now)
        .unwrap();
    let absent = paused.version();
    registry.fail(&claim, clock()).unwrap();
    let target = fixture.store(absent.scope()).unwrap();
    assert_eq!(
        registry.store_and_publish(paused, &material, &target, clock),
        Err(registry::Failure::Conflict)
    );
    assert!(matches!(target.read(absent), Err(Failure::Missing)));
    let later = clock() + 86_400_001;
    let mut retirements = registry.retirements(later).unwrap();
    assert_eq!(retirements.len(), 3);
    let retirement = retirements.remove(0);
    let target = fixture.store(retirement.version().scope()).unwrap();
    target.persistence.fail_sync.store(true, Ordering::SeqCst);
    assert_eq!(
        registry.delete_retired(retirement, &target, || later),
        Err(registry::Failure::OutcomeUnknown)
    );
    target.persistence.fail_sync.store(false, Ordering::SeqCst);
    // The unknown delete response retains the fence and cleanup obligation.
    // Recovery repeats guarded deletion only, never a provider effect or write.
    let registry = Registry::new(&paths.state);
    assert_eq!(registry.retirements(later).unwrap().len(), 3);
    for retirement in registry.retirements(later).unwrap() {
        let target = fixture.store(retirement.version().scope()).unwrap();
        registry
            .delete_retired(retirement, &target, || later)
            .unwrap();
    }
    fixture.stop();
    fixture.start(true);
    assert!(matches!(
        fixture.store(exact.scope()).unwrap().read(exact),
        Err(Failure::Missing)
    ));
    assert!(matches!(
        fixture.store(unknown.scope()).unwrap().read(unknown),
        Err(Failure::Missing)
    ));
    assert!(registry.retirements(later).unwrap().is_empty());
}
