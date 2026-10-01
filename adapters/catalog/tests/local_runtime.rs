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

#[path = "local_runtime/basic_auth.rs"]
mod basic_auth;
#[path = "local_runtime/basic_auth_adversary.rs"]
mod basic_auth_adversary;
#[path = "local_runtime/basic_auth_adversary_pass2.rs"]
mod basic_auth_adversary_pass2;
#[path = "local_runtime/cli_journey.rs"]
mod cli_journey;
#[path = "local_runtime/oauth2_refresh.rs"]
mod oauth2_refresh;
#[path = "local_runtime/oauth2_refresh_adversary.rs"]
mod oauth2_refresh_adversary;
#[path = "local_runtime/oauth2_refresh_adversary_pass2.rs"]
mod oauth2_refresh_adversary_pass2;

/// Fictional HTTP basic material the fixture accepts, and the exact header it
/// expects: `Basic base64("fixture-account@example.test:fixture-api-token-one")`,
/// computed outside the provider with coreutils `base64`.
const BASIC_ACCOUNT: &str = "fixture-account@example.test";
const BASIC_TOKEN: &str = "fixture-api-token-one";
const BASIC_WRONG_TOKEN: &str = "fixture-api-token-wrong";
const BASIC_HEADER: &str =
    "Basic Zml4dHVyZS1hY2NvdW50QGV4YW1wbGUudGVzdDpmaXh0dXJlLWFwaS10b2tlbi1vbmU=";
const BASIC_PROFILE: &str = "fixture.basic";

/// Route and `Authorization` header of each fixture request.
type Authorizations = Arc<Mutex<Vec<(String, Option<String>)>>>;

struct Provider {
    stop: Option<oneshot::Sender<()>>,
    thread: Option<std::thread::JoinHandle<()>>,
    root: tempfile::TempDir,
    ca: PathBuf,
    pem: String,
    config: PathBuf,
    profile: &'static str,
    calls: Arc<Mutex<Vec<String>>>,
    /// The route and `Authorization` header of every request. Only compared,
    /// never printed, so fixture credentials stay out of failure diagnostics.
    authorizations: Authorizations,
    pause: Arc<std::sync::atomic::AtomicBool>,
}
impl Provider {
    fn new() -> Self {
        Self::with_auth(None)
    }
    /// A provider whose profile is HTTP basic and requires `minimum_scope`;
    /// the fixture grants only `api`.
    fn basic(minimum_scope: &str) -> Self {
        Self::with_auth(Some(minimum_scope))
    }
    fn with_auth(basic: Option<&str>) -> Self {
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
        let authorizations = Arc::new(Mutex::new(Vec::new()));
        let observed_authorizations = authorizations.clone();
        let basic_mode = basic.is_some();
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
                    let header = |wanted: &str| {
                        request.lines().find_map(|line| {
                            line.split_once(':')
                                .filter(|(name, _)| name.eq_ignore_ascii_case(wanted))
                                .map(|(_, value)| value.trim().to_owned())
                        })
                    };
                    let credential = header("private-token");
                    let authorization = header("authorization");
                    // Only fictional fixture material is accepted. Do not retain
                    // raw headers in failure diagnostics.
                    let valid = if basic_mode {
                        authorization.as_deref() == Some(BASIC_HEADER)
                    } else {
                        matches!(credential.as_deref(), Some("fixture-pat-one" | "fixture-pat-two"))
                    };
                    let user = if credential.as_deref() == Some("fixture-pat-two") {
                        43
                    } else {
                        42
                    };
                    observed.lock().unwrap().push(path.clone());
                    observed_authorizations
                        .lock()
                        .unwrap()
                        .push((route.to_owned(), authorization));
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
                    } else if route.ends_with("fixture-refused") {
                        (403, json!({"message":"403 Forbidden"}))
                    } else if route.ends_with("fixture-missing") {
                        (404, json!({"message":"404 Project Not Found"}))
                    } else if let Some(page) = repository_page(route, &path) {
                        (200, page)
                    } else if let Some(body) = commit_graph_page(route, &path) {
                        (200, body)
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
        let (profile, auth) = match basic {
            None => (
                "gitlab.pat",
                json!({
                    "profile": "gitlab.pat",
                    "header": "PRIVATE-TOKEN",
                    "bearer": false,
                    "label": "GitLab personal access token",
                    "identity": {"path": "user", "kind": "gitlab.user", "subject_pointer": "/id"},
                    "scopes": {"path": "personal_access_tokens/self", "pointer": "/scopes"},
                    "minimum_scopes": ["api"]
                }),
            ),
            Some(minimum) => (
                BASIC_PROFILE,
                json!({
                    "profile": BASIC_PROFILE,
                    "scheme": "basic",
                    "header": "Authorization",
                    "bearer": false,
                    "account_label": "Fixture account email",
                    "label": "Fixture API token",
                    "identity": {"path": "user", "kind": "fixture.user", "subject_pointer": "/id"},
                    "scopes": {"path": "personal_access_tokens/self", "pointer": "/scopes"},
                    "minimum_scopes": [minimum]
                }),
            ),
        };
        private(
            &config,
            &serde_json::to_vec(&json!({
                "format": "connectors-catalog-local/2",
                "instance": "fixture-gitlab",
                "provider": "gitlab",
                "bundle_directory": repository.join("generated/bundles").canonicalize().unwrap(),
                "api_base": format!("https://localhost:{}/api/v4", address.port()),
                "ca_file": ca,
                "auth": auth,
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
            profile,
            calls,
            authorizations,
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
/// The recorded GitLab repository listings the fixture serves: projects, tags,
/// releases and project events, two items on page one and one on page two, so
/// a walk at `per_page=2` ends on the short second page. `None` for any other
/// route.
fn repository_page(route: &str, path: &str) -> Option<Value> {
    let second = path
        .split_once('?')
        .is_some_and(|(_, query)| query.split('&').any(|pair| pair == "page=2"));
    let items: Vec<Value> = if route == "/api/v4/projects" {
        let project = |id: u64, archived: bool| {
            json!({
                "id": id,
                "path_with_namespace": format!("org/project-{id}"),
                "description": format!("fixture project {id}"),
                "archived": archived,
                "created_at": "2026-01-02T03:04:05.000Z",
                "last_activity_at": "2026-09-20T10:00:00.000Z",
                "topics": ["fixture", "knowledge"],
                "default_branch": "main",
                "visibility": "private",
                "web_url": format!("https://gitlab.example.test/org/project-{id}")
            })
        };
        if second {
            vec![project(3, true)]
        } else {
            vec![project(1, false), project(2, false)]
        }
    } else if route.ends_with("/repository/tags") {
        let tag = |name: &str, commit: &str| {
            json!({"name": name, "message": "", "target": commit,
                   "commit": {"id": commit, "created_at": "2026-09-10T08:00:00.000Z"},
                   "release": null, "protected": false})
        };
        if second {
            vec![tag("v0.1.0", "c0ffee01")]
        } else {
            vec![tag("v0.3.0", "c0ffee03"), tag("v0.2.0", "c0ffee02")]
        }
    } else if route.ends_with("/releases") {
        let release = |tag: &str, at: &str| {
            json!({"tag_name": tag, "name": format!("Release {tag}"),
                   "description": "fixture notes", "released_at": at,
                   "created_at": at, "upcoming_release": false})
        };
        if second {
            vec![release("v0.1.0", "2026-07-01T00:00:00.000Z")]
        } else {
            vec![
                release("v0.3.0", "2026-09-01T00:00:00.000Z"),
                release("v0.2.0", "2026-08-01T00:00:00.000Z"),
            ]
        }
    } else if route.ends_with("/events") {
        let event = |id: u64, action: &str| {
            json!({"id": id, "project_id": 7, "action_name": action,
                   "target_type": null, "author_id": 42,
                   "created_at": "2026-09-15T12:00:00.000Z"})
        };
        if second {
            vec![event(501, "created")]
        } else {
            vec![event(503, "pushed to"), event(502, "pushed new")]
        }
    } else {
        return None;
    };
    Some(Value::Array(items))
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
fn basic(token: &str) -> Secret {
    Secret(serde_json::to_vec(&json!({"account": BASIC_ACCOUNT, "token": token})).unwrap())
}
/// Whether `haystack` carries the basic account or any basic token, raw or as
/// the encoded header value.
fn carries_basic_material(haystack: &[u8]) -> bool {
    [
        BASIC_ACCOUNT,
        BASIC_TOKEN,
        BASIC_WRONG_TOKEN,
        BASIC_HEADER.trim_start_matches("Basic "),
    ]
    .iter()
    .any(|needle| {
        haystack
            .windows(needle.len())
            .any(|window| window == needle.as_bytes())
    })
}
/// Live processes whose argv names `config`: the provider children started for
/// that configuration, whoever spawned them.
fn provider_children(config: &Path) -> Vec<u32> {
    let needle = config.to_str().unwrap().as_bytes().to_vec();
    let own = std::process::id();
    fs::read_dir("/proc")
        .unwrap()
        .filter_map(|entry| entry.ok()?.file_name().to_str()?.parse::<u32>().ok())
        .filter(|pid| *pid != own)
        .filter(|pid| {
            fs::read(format!("/proc/{pid}/cmdline")).is_ok_and(|cmdline| {
                cmdline
                    .split(|b| *b == 0)
                    .any(|argument| argument == needle.as_slice())
            })
        })
        .collect()
}
/// Neither argv nor the environment of any provider child for `config` carries
/// basic material, and at least one such child is observed.
fn assert_children_carry_no_basic_material(config: &Path) {
    let children = provider_children(config);
    assert!(!children.is_empty(), "no provider child observed");
    for pid in children {
        let cmdline = fs::read(format!("/proc/{pid}/cmdline")).unwrap();
        let environ = fs::read(format!("/proc/{pid}/environ")).unwrap();
        assert!(!carries_basic_material(&cmdline), "argv of {pid}");
        assert!(!carries_basic_material(&environ), "environment of {pid}");
    }
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

/// The four repository reads the knowledge-ingest consumer selects, each with
/// the exact request the fixture must observe for its first page at
/// `per_page=2`, including the time filter where the operation has one.
fn repository_reads() -> [(&'static str, Value, &'static str); 4] {
    [
        (
            "projects.list",
            json!({"membership": true, "simple": false, "archived": false,
                   "order_by": "last_activity_at",
                   "last_activity_after": "2026-09-01T00:00:00Z",
                   "page": 1, "per_page": 2}),
            "/api/v4/projects?order_by=last_activity_at&archived=false&membership=true\
             &last_activity_after=2026-09-01T00%3A00%3A00Z&page=1&per_page=2&simple=false",
        ),
        (
            "tags.list",
            json!({"id": "org/project", "page": 1, "per_page": 2}),
            "/api/v4/projects/org%2Fproject/repository/tags?page=1&per_page=2",
        ),
        (
            "releases.list",
            json!({"id": "org/project", "page": 1, "per_page": 2}),
            "/api/v4/projects/org%2Fproject/releases?page=1&per_page=2",
        ),
        (
            "project.events",
            json!({"id": "org/project", "after": "2026-09-01", "before": "2026-09-30",
                   "page": 1, "per_page": 2}),
            "/api/v4/projects/org%2Fproject/events?before=2026-09-30&after=2026-09-01\
             &page=1&per_page=2",
        ),
    ]
}

#[test]
fn repository_reads_send_the_declared_request_and_return_the_recorded_body() {
    let provider = Provider::new();
    let mut child = Child::spawn(&provider.selection()).unwrap();
    for (operation, input, expected) in repository_reads() {
        let before = provider.count();
        let result = invoke(&mut child, operation, "one", &token(true), input)
            .unwrap_or_else(|failure| panic!("`{operation}` failed: {failure:?}"));
        let calls = provider.calls.lock().unwrap().clone();
        assert_eq!(calls.len(), before + 1, "`{operation}` requests");
        assert_eq!(calls[before], expected, "`{operation}` request");
        assert_eq!(result["status"], 200, "`{operation}` status");
        let route = expected.split('?').next().unwrap();
        assert_eq!(
            Some(&result["body"]),
            repository_page(route, expected).as_ref(),
            "`{operation}` body"
        );
        assert_eq!(result["provenance"]["instance"], "fixture-gitlab");
    }
}

#[test]
fn repository_list_reads_walk_two_pages_and_stop_on_a_short_page() {
    let provider = Provider::new();
    let mut child = Child::spawn(&provider.selection()).unwrap();
    for (operation, first, expected) in repository_reads() {
        let before = provider.count();
        let per_page = first["per_page"].as_u64().unwrap() as usize;
        let mut items = Vec::new();
        let mut page = 1;
        loop {
            let mut input = first.clone();
            input["page"] = json!(page);
            let result = invoke(&mut child, operation, "one", &token(true), input)
                .unwrap_or_else(|failure| panic!("`{operation}` page {page}: {failure:?}"));
            let body = result["body"].as_array().unwrap().clone();
            let short = body.len() < per_page;
            items.extend(body);
            if short {
                break;
            }
            page += 1;
            assert!(page <= 3, "`{operation}` did not stop");
        }
        assert_eq!(page, 2, "`{operation}` pages walked");
        assert_eq!(items.len(), 3, "`{operation}` items");
        let calls = provider.calls.lock().unwrap().clone();
        assert_eq!(
            calls[before..],
            [expected.to_owned(), expected.replace("page=1", "page=2")],
            "`{operation}` requests"
        );
    }
}

/// The recorded GitLab commit graph the fixture serves: `commits.list` pages
/// of two and then one commit, and three compares keyed by `to`: `v0.3.0`
/// has commits, `v0.1.0` is empty, and `main` is one GitLab cut short with
/// `compare_timeout: true`. `None` for any other route.
fn commit_graph_page(route: &str, path: &str) -> Option<Value> {
    let query: Vec<&str> = path
        .split_once('?')
        .map(|(_, query)| query.split('&').collect())
        .unwrap_or_default();
    let commit = |id: &str, parents: &[&str]| {
        json!({"id": id, "parent_ids": parents,
               "created_at": "2026-09-10T08:00:00.000+00:00",
               "committed_date": "2026-09-10T08:00:00.000+00:00",
               "title": format!("fixture commit {id}"),
               "author_name": "Fixture Author", "author_email": "author@example.test"})
    };
    if route.ends_with("/repository/commits") {
        Some(if query.contains(&"page=2") {
            json!([commit("c0ffee01", &[])])
        } else {
            json!([
                commit("c0ffee03", &["c0ffee02"]),
                commit("c0ffee02", &["c0ffee01", "beef0001"])
            ])
        })
    } else if route.ends_with("/repository/compare") {
        let (commits, timeout) = if query.contains(&"to=main") {
            (json!([]), true)
        } else if query.contains(&"to=v0.1.0") {
            (json!([]), false)
        } else {
            (
                json!([
                    commit("c0ffee02", &["c0ffee01"]),
                    commit("c0ffee03", &["c0ffee02"])
                ]),
                false,
            )
        };
        Some(json!({"commits": commits, "diffs": [],
                    "compare_timeout": timeout, "compare_same_ref": false}))
    } else {
        None
    }
}

/// `commits.list` and `repository.compare` through the owned child and the TLS
/// fixture: the exact request each sends, a `commits.list` walk that stops on
/// the short second page, and what a caller receives for an empty compare and
/// for one GitLab cut short. Both answer 200 with an empty `commits`; only
/// `body.compare_timeout` tells them apart.
#[test]
fn commit_graph_reads_send_the_declared_request_and_tell_a_timeout_from_an_empty_compare() {
    let provider = Provider::new();
    let mut child = Child::spawn(&provider.selection()).unwrap();
    let first = "/api/v4/projects/org%2Fproject/repository/commits?ref_name=main\
                 &since=2026-09-01T00%3A00%3A00Z&until=2026-09-30T00%3A00%3A00Z\
                 &first_parent=true&page=1&per_page=2";
    let mut items = Vec::new();
    let mut page = 1;
    loop {
        let before = provider.count();
        let result = invoke(
            &mut child,
            "commits.list",
            "one",
            &token(true),
            json!({"id": "org/project", "ref_name": "main",
                   "since": "2026-09-01T00:00:00Z", "until": "2026-09-30T00:00:00Z",
                   "first_parent": true, "page": page, "per_page": 2}),
        )
        .unwrap_or_else(|failure| panic!("`commits.list` page {page}: {failure:?}"));
        let calls = provider.calls.lock().unwrap().clone();
        assert_eq!(calls.len(), before + 1);
        assert_eq!(
            calls[before],
            first.replace("page=1", &format!("page={page}")),
            "page {page}"
        );
        assert_eq!(result["status"], 200);
        let body = result["body"].as_array().unwrap().clone();
        let short = body.len() < 2;
        items.extend(body);
        if short {
            break;
        }
        page += 1;
        assert!(page <= 3, "`commits.list` did not stop");
    }
    assert_eq!(page, 2);
    let ids: Vec<&str> = items.iter().map(|c| c["id"].as_str().unwrap()).collect();
    assert_eq!(ids, ["c0ffee03", "c0ffee02", "c0ffee01"]);
    assert_eq!(items[1]["parent_ids"], json!(["c0ffee01", "beef0001"]));

    let mut compare = |to: &str| {
        let before = provider.count();
        let result = invoke(
            &mut child,
            "repository.compare",
            "one",
            &token(true),
            json!({"id": "org/project", "from": "v0.1.0", "to": to, "straight": false}),
        )
        .unwrap_or_else(|failure| panic!("`repository.compare` to {to}: {failure:?}"));
        let calls = provider.calls.lock().unwrap().clone();
        assert_eq!(calls.len(), before + 1);
        assert_eq!(
            calls[before],
            format!(
                "/api/v4/projects/org%2Fproject/repository/compare?from=v0.1.0&to={to}&straight=false"
            )
        );
        assert_eq!(result["status"], 200, "to {to}");
        assert_eq!(
            Some(&result["body"]),
            commit_graph_page(
                "/api/v4/projects/org%2Fproject/repository/compare",
                &calls[before]
            )
            .as_ref(),
            "to {to}"
        );
        result["body"].clone()
    };
    let full = compare("v0.3.0");
    let empty = compare("v0.1.0");
    let timed_out = compare("main");
    assert_eq!(full["commits"].as_array().unwrap().len(), 2);
    assert_eq!(full["compare_timeout"], json!(false));
    assert_eq!(empty["commits"], json!([]));
    assert_eq!(empty["compare_timeout"], json!(false));
    assert_eq!(timed_out["commits"], json!([]));
    assert_eq!(timed_out["compare_timeout"], json!(true));
}
