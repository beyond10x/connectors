//! The host's owned-child mechanics with the catalog provider as the child and
//! a disposable authenticated HTTPS GitLab: spawn against the printed bootstrap,
//! validate a credential through the declared identity and scope probes, read
//! through the shipped selection set, stop by exact incarnation, and refuse a
//! changed artifact, bootstrap or trust root before any provider work. The
//! separate CLI journey exercises the same selection through the production
//! owner and disposable Secret Service.
use connectors_host::local::{
    config::{Adapter, Executable, Restart, Startup},
    filesystem,
    runtime::{Bootstrap, Child, Failure},
};
use connectors_sdk::Secret;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    fs,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    process::Command,
    sync::{Arc, Mutex},
    time::Duration,
};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    sync::oneshot,
};
use tokio_rustls::{
    TlsAcceptor,
    rustls::{self, pki_types::PrivatePkcs8KeyDer},
};

#[path = "local_runtime/cli_journey.rs"]
mod cli_journey;

struct Provider {
    stop: Option<oneshot::Sender<()>>,
    thread: Option<std::thread::JoinHandle<()>>,
    root: tempfile::TempDir,
    ca: PathBuf,
    pem: String,
    config: PathBuf,
    calls: Arc<Mutex<Vec<String>>>,
    pause: Arc<std::sync::atomic::AtomicBool>,
}
impl Provider {
    fn new() -> Self {
        let root = tempfile::tempdir().unwrap();
        let directory = root.path().join("private");
        filesystem::directory(&directory, true, true).unwrap();
        let cert = rcgen::generate_simple_self_signed(vec!["localhost".into()]).unwrap();
        let pem = cert.cert.pem();
        let ca = directory.join("ca.pem");
        private(&ca, pem.as_bytes());
        let tls = rustls::ServerConfig::builder_with_provider(Arc::new(
            rustls::crypto::ring::default_provider(),
        ))
        .with_safe_default_protocol_versions()
        .unwrap()
        .with_no_client_auth()
        .with_single_cert(
            vec![cert.cert.der().clone()],
            PrivatePkcs8KeyDer::from(cert.signing_key.serialize_der()).into(),
        )
        .unwrap();
        let (address_tx, address_rx) = std::sync::mpsc::channel();
        let (stop, mut stopped) = oneshot::channel();
        let calls = Arc::new(Mutex::new(Vec::new()));
        let observed = calls.clone();
        let pause = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let paused = pause.clone();
        let thread = std::thread::spawn(move || {
            let runtime = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap();
            runtime.block_on(async {
                let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
                address_tx.send(listener.local_addr().unwrap()).unwrap();
                let acceptor = TlsAcceptor::from(Arc::new(tls));
                loop {
                    let stream = tokio::select! {_=&mut stopped=>break,value=listener.accept()=>value.unwrap().0};
                    let Ok(mut stream) = acceptor.accept(stream).await else {
                        continue;
                    };
                    let mut header = Vec::new();
                    loop {
                        if header.ends_with(b"\r\n\r\n") {
                            break;
                        }
                        assert!(header.len() < 8192);
                        match stream.read_u8().await {
                            Ok(byte) => header.push(byte),
                            Err(_) => break,
                        }
                    }
                    let request = String::from_utf8(header).unwrap();
                    let path = request
                        .split_whitespace()
                        .nth(1)
                        .unwrap_or_default()
                        .to_owned();
                    let route = path.split('?').next().unwrap();
                    let credential = request.lines().find_map(|line| {
                        line.split_once(':')
                            .filter(|(name, _)| name.eq_ignore_ascii_case("private-token"))
                            .map(|(_, value)| value.trim())
                    });
                    // Only fictional fixture material is accepted. Do not retain
                    // raw headers in observations or failure diagnostics.
                    let valid = matches!(credential, Some("fixture-pat-one" | "fixture-pat-two"));
                    let user = if credential == Some("fixture-pat-two") {
                        43
                    } else {
                        42
                    };
                    observed.lock().unwrap().push(path.clone());
                    if paused.load(std::sync::atomic::Ordering::SeqCst) {
                        tokio::select! {_=&mut stopped=>break,_=tokio::time::sleep(Duration::from_secs(2))=>{}}
                    }
                    let (status, body) = if !valid {
                        (401, json!({"error":"fixture refusal"}))
                    } else if route == "/api/v4/user" {
                        (200, json!({"id":user,"state":"active"}))
                    } else if route == "/api/v4/personal_access_tokens/self" {
                        (
                            200,
                            json!({"id":99,"user_id":user,"active":true,"revoked":false,"scopes":["api"],"expires_at":null}),
                        )
                    } else if path.contains("/issues?") {
                        (200, json!([{"id":1,"title":"fixture"}]))
                    } else if path.contains("/repository/files/") {
                        (
                            200,
                            json!({"content":"Zml4dHVyZQ==","encoding":"base64","size":7,"last_commit_id":"fixture-commit"}),
                        )
                    } else {
                        (200, json!({"id":7,"name":"fixture-project"}))
                    };
                    let body = serde_json::to_vec(&body).unwrap();
                    let header = format!(
                        "HTTP/1.1 {status} fixture\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                        body.len()
                    );
                    let _ = stream.write_all(header.as_bytes()).await;
                    let _ = stream.write_all(&body).await;
                }
            });
        });
        let address = address_rx.recv_timeout(Duration::from_secs(5)).unwrap();
        let repository = Path::new(env!("CARGO_MANIFEST_DIR"));
        let config = directory.join("catalog.json");
        private(
            &config,
            &serde_json::to_vec(&json!({
                "format": "connectors-catalog-local/2",
                "instance": "fixture-gitlab",
                "provider": "gitlab",
                "bundle_directory": repository.join("generated/bundles").canonicalize().unwrap(),
                "api_base": format!("https://localhost:{}/api/v4", address.port()),
                "ca_file": ca,
                "auth": {
                    "profile": "gitlab.pat",
                    "header": "PRIVATE-TOKEN",
                    "bearer": false,
                    "label": "GitLab personal access token",
                    "identity": {"path": "user", "kind": "gitlab.user", "subject_pointer": "/id"},
                    "scopes": {"path": "personal_access_tokens/self", "pointer": "/scopes"},
                    "minimum_scopes": ["api"]
                },
                "operations_file": repository.join("providers/gitlab/operations.json").canonicalize().unwrap(),
            }))
            .unwrap(),
        );
        Self {
            stop: Some(stop),
            thread: Some(thread),
            root,
            ca,
            pem,
            config,
            calls,
            pause,
        }
    }
    fn selection(&self) -> Adapter {
        let output = Command::new(env!("CARGO_BIN_EXE_connectors-catalog-provider"))
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
        let binary = PathBuf::from(env!("CARGO_BIN_EXE_connectors-catalog-provider"))
            .canonicalize()
            .unwrap();
        Adapter {
            private_protocol: None,
            permissions: Default::default(),
            instance_id: "fixture-gitlab".into(),
            adapter_id: "catalog".into(),
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
    fn count(&self) -> usize {
        self.calls.lock().unwrap().len()
    }
}
impl Drop for Provider {
    fn drop(&mut self) {
        let _ = self.stop.take().unwrap().send(());
        self.thread.take().unwrap().join().unwrap();
    }
}
fn private(path: &Path, bytes: &[u8]) {
    fs::write(path, bytes).unwrap();
    fs::set_permissions(path, fs::Permissions::from_mode(0o600)).unwrap();
}
fn token(one: bool) -> Secret {
    Secret(if one {
        br#"{"token":"fixture-pat-one"}"#.to_vec()
    } else {
        br#"{"token":"fixture-pat-two"}"#.to_vec()
    })
}
fn deadline() -> u64 {
    connectors_sdk::now_ms() + 30_000
}
fn invoke(
    child: &mut Child,
    operation: &str,
    partition: &str,
    secret: &Secret,
    input: Value,
) -> Result<Value, Failure> {
    let revision = child.bootstrap().descriptor().unwrap().revision;
    child
        .invoke(
            operation,
            &revision,
            partition,
            secret,
            &serde_json::to_vec(&input).unwrap(),
            deadline(),
        )
        .map(|b| serde_json::from_slice(&b).unwrap())
}

#[test]
fn catalog_child_validates_reads_and_stops_with_exact_incarnations() {
    let provider = Provider::new();
    let selection = provider.selection();
    assert_eq!(provider.count(), 0);
    let mut child = Child::spawn(&selection).unwrap();
    assert_eq!(provider.count(), 0);
    // Changing the path after bootstrap cannot change the captured TLS roots.
    private(
        &provider.ca,
        rcgen::generate_simple_self_signed(vec!["localhost".into()])
            .unwrap()
            .cert
            .pem()
            .as_bytes(),
    );
    let baseline = child
        .validate("gitlab.pat", &token(true), deadline())
        .unwrap_or_else(|failure| {
            panic!(
                "validation {failure:?}; observed paths {:?}",
                provider.calls.lock().unwrap()
            )
        });
    assert_eq!(baseline.identity.subject, "42");
    assert!(baseline.granted_scopes.unwrap().contains("api"));
    assert_eq!(
        child
            .validate("gitlab.pat", &token(false), deadline())
            .unwrap()
            .identity
            .subject,
        "43"
    );
    let before = provider.count();
    assert!(matches!(
        child.validate("unknown", &token(true), deadline()),
        Err(Failure::Unsupported)
    ));
    assert!(matches!(
        child.validate(
            "gitlab.pat",
            &Secret(br#"{"token":"fictional","token":"duplicate"}"#.to_vec()),
            deadline()
        ),
        Err(Failure::InvalidInput)
    ));
    assert_eq!(provider.count(), before);
    // A selection the shipped set does not carry is refused before any request.
    assert!(
        invoke(
            &mut child,
            "merge_request.validate",
            "one",
            &token(true),
            json!({"id":"org/project"})
        )
        .is_err()
    );
    assert_eq!(provider.count(), before);
    let project = invoke(
        &mut child,
        "project.get",
        "one",
        &token(true),
        json!({"id":"org/project"}),
    )
    .unwrap();
    assert_eq!(project["status"], 200);
    assert_eq!(project["body"]["id"], 7);
    assert_eq!(project["provenance"]["instance"], "fixture-gitlab");
    let issues = invoke(
        &mut child,
        "issues.list",
        "one",
        &token(true),
        json!({"id":"org/project","per_page":1}),
    )
    .unwrap();
    assert_eq!(issues["body"].as_array().unwrap().len(), 1);
    assert_eq!(
        invoke(
            &mut child,
            "file.get",
            "one",
            &token(true),
            json!({"id":"org/project","file_path":"README.md","ref":"main"})
        )
        .unwrap()["body"]["content"],
        "Zml4dHVyZQ=="
    );
    assert_eq!(
        child.stop("stale-incarnation"),
        Err(Failure::IncarnationMismatch)
    );
    assert_eq!(
        child
            .validate("gitlab.pat", &token(true), deadline())
            .unwrap()
            .identity
            .subject,
        "42"
    );
    let incarnation = child.incarnation().to_owned();
    child.stop(&incarnation).unwrap();
    assert!(matches!(
        child.validate("gitlab.pat", &token(true), deadline()),
        Err(Failure::Unavailable)
    ));
    // The independently computed revision covers the trust root's bytes.
    assert!(matches!(
        Child::spawn(&selection),
        Err(Failure::ReadinessMismatch)
    ));
    private(&provider.ca, provider.pem.as_bytes());
    let mut restored = Child::spawn(&selection).unwrap();
    assert_eq!(
        restored
            .validate("gitlab.pat", &token(true), deadline())
            .unwrap()
            .identity
            .subject,
        "42"
    );
}

#[test]
fn bootstrap_mismatches_and_changed_artifacts_refuse_before_provider_work() {
    let provider = Provider::new();
    let selection = provider.selection();
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
    let script = provider.root.path().join("private/script");
    private(&script, b"#!/bin/sh\nexit 0\n");
    fs::set_permissions(&script, fs::Permissions::from_mode(0o700)).unwrap();
    wrong.executable.path = script;
    wrong.executable.sha256 = hex::encode(Sha256::digest(b"#!/bin/sh\nexit 0\n"));
    assert!(matches!(
        Child::spawn(&wrong),
        Err(Failure::InvalidConfiguration)
    ));
    assert_eq!(provider.count(), 0);
}

#[test]
fn lost_validation_response_closes_the_owned_channel_without_replay() {
    let provider = Provider::new();
    let mut child = Child::spawn(&provider.selection()).unwrap();
    provider
        .pause
        .store(true, std::sync::atomic::Ordering::SeqCst);
    let result = child.validate("gitlab.pat", &token(true), connectors_sdk::now_ms() + 1000);
    // The peer may hit its same original deadline and close before the parent's
    // socket timeout. Both observations must close ownership and prohibit replay.
    assert!(
        matches!(result, Err(Failure::Timeout | Failure::Unavailable)),
        "failure {:?}",
        result.err()
    );
    assert_eq!(provider.count(), 1);
    assert!(matches!(
        child.validate("gitlab.pat", &token(true), deadline()),
        Err(Failure::Unavailable)
    ));
    assert_eq!(provider.count(), 1);
}
