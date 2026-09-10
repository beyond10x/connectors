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

struct Fixture {
    daemon: Option<OwnedChild>,
    _bus: OwnedChild,
    root: tempfile::TempDir,
    socket: PathBuf,
    data: PathBuf,
    incarnation: u32,
}

impl Fixture {
    fn new() -> Self {
        let temporary = std::env::var_os("TMPDIR").expect("task-owned TMPDIR required");
        let root = tempfile::Builder::new()
            .prefix("secret-service-")
            .tempdir_in(temporary)
            .unwrap();
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

    fn start(&mut self, unlock: bool) {
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

    fn stop(&mut self) {
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
    // No caller metadata was published by this backend fixture. Guarded
    // retirement/publication and the CLI journey still need coordinator tests.
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
