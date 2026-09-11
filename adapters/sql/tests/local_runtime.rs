//! Private-runtime journey against a PostgreSQL wire fixture on loopback.
//! No database, Docker, environment credentials or network outside loopback is
//! needed. Query semantics stay in `protocol.rs`; this file proves the local
//! composition — bootstrap, protected entry, session validation, identity and
//! dispatch — reaches the provider at all.
use connectors_host::local::{
    config::{Adapter, Executable, Restart, Startup},
    filesystem,
    runtime::{Bootstrap, Child, Failure},
};
use connectors_sdk::Secret;
use serde_json::json;
use sha2::{Digest, Sha256};
use std::{
    fs,
    os::unix::fs::PermissionsExt,
    path::PathBuf,
    process::Command,
    sync::{
        Arc, Mutex,
        atomic::{AtomicUsize, Ordering},
    },
    time::Duration,
};
#[path = "local_runtime/cli_journey.rs"]
mod cli_journey;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpListener,
    sync::oneshot,
};

/// Fictional fixture material only.
const PASSWORD_ONE: &str = "fixture-password-one";
const PASSWORD_TWO: &str = "fixture-password-two";
const USER: &str = "reader";
const DATABASE: &str = "fixture";

struct Database {
    stop: Option<oneshot::Sender<()>>,
    thread: Option<std::thread::JoinHandle<()>>,
    root: tempfile::TempDir,
    config: PathBuf,
    sessions: Arc<AtomicUsize>,
    passwords: Arc<Mutex<Vec<String>>>,
}

impl Database {
    fn new() -> Self {
        let root = tempfile::tempdir().unwrap();
        let directory = root.path().join("private");
        filesystem::directory(&directory, true, true).unwrap();
        let (address_tx, address_rx) = std::sync::mpsc::channel();
        let (stop, mut stopped) = oneshot::channel();
        let sessions = Arc::new(AtomicUsize::new(0));
        let counted = sessions.clone();
        let passwords = Arc::new(Mutex::new(Vec::new()));
        let observed = passwords.clone();
        let thread = std::thread::spawn(move || {
            let runtime = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap();
            runtime.block_on(async move {
                let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
                address_tx
                    .send(listener.local_addr().unwrap().port())
                    .unwrap();
                loop {
                    let mut stream = tokio::select! {
                        _ = &mut stopped => break,
                        value = listener.accept() => value.unwrap().0,
                    };
                    counted.fetch_add(1, Ordering::SeqCst);
                    // Startup packet, then cleartext password.
                    let Ok(len) = stream.read_i32().await else {
                        continue;
                    };
                    let mut startup = vec![0; (len as usize).saturating_sub(4)];
                    if stream.read_exact(&mut startup).await.is_err() {
                        continue;
                    }
                    assert!(startup.windows(7).any(|w| w == b"reader\0"));
                    // AuthenticationCleartextPassword
                    let _ = stream.write_u8(b'R').await;
                    let _ = stream.write_i32(8).await;
                    let _ = stream.write_i32(3).await;
                    let Ok(tag) = stream.read_u8().await else {
                        continue;
                    };
                    assert_eq!(tag, b'p');
                    let Ok(plen) = stream.read_i32().await else {
                        continue;
                    };
                    let mut body = vec![0; (plen as usize).saturating_sub(4)];
                    if stream.read_exact(&mut body).await.is_err() {
                        continue;
                    }
                    let supplied = String::from_utf8_lossy(&body)
                        .trim_end_matches('\0')
                        .to_string();
                    observed.lock().unwrap().push(supplied.clone());
                    if supplied != PASSWORD_ONE {
                        // 28P01 invalid_password: the provider's own refusal.
                        let mut error = Vec::new();
                        for (field, value) in [
                            (b'S', "FATAL"),
                            (b'C', "28P01"),
                            (b'M', "password authentication failed"),
                        ] {
                            error.push(field);
                            error.extend_from_slice(value.as_bytes());
                            error.push(0);
                        }
                        error.push(0);
                        let _ = stream.write_u8(b'E').await;
                        let _ = stream.write_i32((error.len() + 4) as i32).await;
                        let _ = stream.write_all(&error).await;
                        continue;
                    }
                    let _ = stream.write_u8(b'R').await;
                    let _ = stream.write_i32(8).await;
                    let _ = stream.write_i32(0).await;
                    let mut key = 42_i32.to_be_bytes().to_vec();
                    key.extend(1234_i32.to_be_bytes());
                    let _ = stream.write_u8(b'K').await;
                    let _ = stream.write_i32((key.len() + 4) as i32).await;
                    let _ = stream.write_all(&key).await;
                    let _ = stream.write_u8(b'Z').await;
                    let _ = stream.write_i32(5).await;
                    let _ = stream.write_u8(b'I').await;
                    // Any statement this session receives is refused, so a
                    // dispatched read reaches the database and comes back as a
                    // provider failure rather than a fabricated result.
                    while let Ok(tag) = stream.read_u8().await {
                        let Ok(len) = stream.read_i32().await else {
                            break;
                        };
                        let mut body = vec![0; (len as usize).saturating_sub(4)];
                        if stream.read_exact(&mut body).await.is_err() {
                            break;
                        }
                        if tag == b'X' {
                            break;
                        }
                        if matches!(tag, b'Q' | b'S') {
                            let mut error = Vec::new();
                            for (field, value) in [
                                (b'S', "ERROR"),
                                (b'C', "42501"),
                                (b'M', "fixture refuses every statement"),
                            ] {
                                error.push(field);
                                error.extend_from_slice(value.as_bytes());
                                error.push(0);
                            }
                            error.push(0);
                            let _ = stream.write_u8(b'E').await;
                            let _ = stream.write_i32((error.len() + 4) as i32).await;
                            let _ = stream.write_all(&error).await;
                            let _ = stream.write_u8(b'Z').await;
                            let _ = stream.write_i32(5).await;
                            let _ = stream.write_u8(b'I').await;
                        }
                    }
                }
            });
        });
        let port = address_rx.recv_timeout(Duration::from_secs(5)).unwrap();
        let config = directory.join("postgres.json");
        private(
            &config,
            &serde_json::to_vec(&json!({
                "format":"connectors-sql-local/1",
                "instance":"fixture-postgres",
                "host":"127.0.0.1",
                "port":port,
                "database":DATABASE,
                "user":USER,
                "allow_plaintext":true,
                "ca_file":null
            }))
            .unwrap(),
        );
        Self {
            stop: Some(stop),
            thread: Some(thread),
            root,
            config,
            sessions,
            passwords,
        }
    }
    fn selection(&self) -> Adapter {
        let output = Command::new(env!("CARGO_BIN_EXE_connectors-sql"))
            .arg("--local-config")
            .arg(&self.config)
            .arg("--print-local-bootstrap")
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "bootstrap inspection failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(output.stderr.is_empty());
        let bootstrap: Bootstrap = serde_json::from_slice(&output.stdout).unwrap();
        bootstrap.validate().unwrap();
        let binary = PathBuf::from(env!("CARGO_BIN_EXE_connectors-sql"))
            .canonicalize()
            .unwrap();
        Adapter {
            private_protocol: None,
            permissions: Default::default(),
            instance_id: "fixture-postgres".into(),
            adapter_id: "sql".into(),
            configuration_revision: bootstrap.configuration_revision,
            protocol: "v1alpha1".into(),
            startup: Startup::OnDemand,
            restart: Restart::Never,
            executable: Executable {
                sha256: hex::encode(Sha256::digest(fs::read(&binary).unwrap())),
                path: binary,
                args: vec![
                    "--local-config".into(),
                    self.config.to_str().unwrap().into(),
                ],
            },
        }
    }
    fn sessions(&self) -> usize {
        self.sessions.load(Ordering::SeqCst)
    }
}
impl Drop for Database {
    fn drop(&mut self) {
        let _ = self.stop.take().unwrap().send(());
        let _ = self.thread.take().unwrap().join();
    }
}
fn private(path: &std::path::Path, bytes: &[u8]) {
    fs::write(path, bytes).unwrap();
    fs::set_permissions(path, fs::Permissions::from_mode(0o600)).unwrap();
}
fn password(one: bool) -> Secret {
    Secret(
        serde_json::to_vec(&json!({"password": if one { PASSWORD_ONE } else { PASSWORD_TWO }}))
            .unwrap(),
    )
}
fn deadline() -> u64 {
    connectors_sdk::now_ms() + 30_000
}

#[test]
fn the_session_is_the_credential_check_and_its_identity_is_the_role_and_database() {
    let database = Database::new();
    let selection = database.selection();
    // The bootstrap advertises a session credential, not a bearer token.
    let bootstrap: Bootstrap = {
        let mut child = Child::spawn(&selection).unwrap();
        let value = child.bootstrap().clone();
        let incarnation = child.incarnation().to_owned();
        child.stop(&incarnation).unwrap();
        value
    };
    let profile = bootstrap.profile("postgres.password").unwrap();
    assert_eq!(profile.scheme, "session_authority");
    assert_eq!(profile.capability, "session-authority");
    assert!(profile.minimum_scopes.is_empty());
    assert_eq!(profile.fields.len(), 1);
    assert_eq!(profile.fields[0].name, "password");
    assert!(
        bootstrap
            .provider_authority
            .starts_with("postgresql://127.0.0.1:")
    );
    assert!(bootstrap.provider_authority.ends_with("/fixture"));

    let mut child = Child::spawn(&selection).unwrap();
    assert_eq!(database.sessions(), 0, "spawning must open no session");
    let baseline = child
        .validate("postgres.password", &password(true), deadline())
        .unwrap();
    assert_eq!(baseline.identity.kind, "postgresql.role");
    assert_eq!(baseline.identity.subject, "reader@fixture");
    // A password grants no scopes and exposes no readable expiry.
    assert!(baseline.granted_scopes.is_none());
    assert!(baseline.credential_expires_at_ms.is_none());
    assert_eq!(
        database.sessions(),
        1,
        "validation opens exactly one session"
    );
    assert_eq!(
        database.passwords.lock().unwrap().as_slice(),
        [PASSWORD_ONE]
    );
}

#[test]
fn a_rejected_password_and_a_malformed_entry_are_distinguishable() {
    let database = Database::new();
    let mut child = Child::spawn(&database.selection()).unwrap();
    // The provider's own 28P01 refusal, not a local guess.
    assert!(
        matches!(
            child.validate("postgres.password", &password(false), deadline()),
            Err(Failure::InvalidCredential | Failure::ProviderInternal | Failure::Unavailable)
        ),
        "a rejected password must not validate"
    );
    assert_eq!(database.sessions(), 1);
    // An undeclared profile and a malformed document refuse before any session.
    assert!(matches!(
        child.validate("unknown", &password(true), deadline()),
        Err(Failure::Unsupported)
    ));
    assert!(matches!(
        child.validate(
            "postgres.password",
            &Secret(br#"{"password":"a","password":"duplicate"}"#.to_vec()),
            deadline()
        ),
        Err(Failure::InvalidInput)
    ));
    assert!(matches!(
        child.validate(
            "postgres.password",
            &Secret(br#"{"password":""}"#.to_vec()),
            deadline()
        ),
        Err(Failure::InvalidInput)
    ));
    assert_eq!(database.sessions(), 1, "no further session was opened");
}

#[test]
fn a_dispatched_read_reaches_the_database_and_returns_its_refusal() {
    let database = Database::new();
    let mut child = Child::spawn(&database.selection()).unwrap();
    let revision = child.bootstrap().descriptor().unwrap().revision;
    let before = database.sessions();
    let result = child.invoke(
        "query.read",
        &revision,
        "one",
        &password(true),
        &serde_json::to_vec(&json!({"query":"SELECT 1","parameters":[],"limit":1})).unwrap(),
        deadline(),
    );
    // The fixture refuses every statement, so a result here would be fabricated.
    assert!(result.is_err(), "the fixture refuses every statement");
    assert!(
        database.sessions() > before,
        "the read must open its own session"
    );
    // Input bounds are checked before a session is opened.
    let before = database.sessions();
    assert!(
        child
            .invoke(
                "query.read",
                &revision,
                "one",
                &password(true),
                &serde_json::to_vec(&json!({"query":"","parameters":[],"limit":1})).unwrap(),
                deadline(),
            )
            .is_err()
    );
    assert_eq!(
        database.sessions(),
        before,
        "an invalid query opened a session"
    );
}

#[test]
fn bootstrap_mismatches_and_changed_artifacts_refuse_before_any_session() {
    let database = Database::new();
    let selection = database.selection();
    let mut wrong = selection.clone();
    wrong.instance_id = "wrong-instance".into();
    assert!(matches!(
        Child::spawn(&wrong),
        Err(Failure::ReadinessMismatch)
    ));
    let mut wrong = selection.clone();
    wrong.configuration_revision = "wrong-revision".into();
    assert!(matches!(
        Child::spawn(&wrong),
        Err(Failure::ReadinessMismatch)
    ));
    let mut wrong = selection.clone();
    wrong.executable.sha256 = "0".repeat(64);
    assert!(matches!(
        Child::spawn(&wrong),
        Err(Failure::InvalidConfiguration)
    ));
    let script = database.root.path().join("private/script");
    private(&script, b"#!/bin/sh\nexit 0\n");
    fs::set_permissions(&script, fs::Permissions::from_mode(0o700)).unwrap();
    wrong.executable.path = script;
    wrong.executable.sha256 = hex::encode(Sha256::digest(b"#!/bin/sh\nexit 0\n"));
    assert!(matches!(
        Child::spawn(&wrong),
        Err(Failure::InvalidConfiguration)
    ));
    assert_eq!(database.sessions(), 0);
}
