//! OpenRouter management reads through the catalog provider.
//!
//! The shipped selection set is pinned by id, source operation, method and
//! path, and resolves against the committed bundle compiled from the pinned
//! OpenRouter OpenAPI document. Each read runs through the provider child
//! against a disposable HTTPS fixture that plays OpenRouter: the exact request
//! (method, path, query, `Authorization: Bearer …`) is asserted and the answer
//! is the fixture's own. No live account, no network, and the key is
//! fictional.
#![cfg(unix)]
use connectors_catalog::bundle;
use connectors_catalog_provider::{Effect, Engine, Selection};
use connectors_host::local::{
    config::{Adapter, Executable, Restart, Startup},
    filesystem,
    runtime::{Bootstrap, Child, Failure, PrivateProtocol},
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

/// The document's server is `https://openrouter.ai/api/v1`, so every operation
/// sits below `/api/v1`.
const BASE: &str = "/api/v1";
const INSTANCE: &str = "fixture-openrouter";
const PROFILE: &str = "openrouter.api-key";
/// The fictional management key the fixture accepts, as `Authorization: Bearer <key>`.
const KEY: &str = "fixture-openrouter-management-key";
/// The pinned source, its manifest and README, relative to this crate.
const UPSTREAM: &str = "../openrouter/upstream";
const SOURCE: &str = "openrouter-openapi.json";
const URL: &str = "https://openrouter.ai/openapi.json";
const UPSTREAM_SHA256: &str = "8287890b719d375d64b5651ca4fa2bc34be3af84b24e8fceb8ab13ae9894fe12";
const SOURCE_SHA256: &str = "fe4f1ad414f5bd25e69b387598d23f3abd24fe74801cd157c633e30bbf766142";
const SOURCE_BYTES: usize = 2_690_724;
const KEY_PATH: &str = "/api/v1/key";
const WORKSPACES: &str = "/api/v1/workspaces";
const KEYS: &str = "/api/v1/keys";
const LABEL: &str = "fixture-management-key";

/// The shipped ids, their pinned `operationId`, method, path and effect. A
/// renamed, dropped or added id fails here.
const SHIPPED: [(&str, &str, &str, &str, Effect); 3] = [
    (
        "key.get",
        "getCurrentKey",
        "get",
        "/api/v1/key",
        Effect::Read,
    ),
    ("keys.list", "list", "get", "/api/v1/keys", Effect::Read),
    (
        "workspaces.list",
        "listWorkspaces",
        "get",
        "/api/v1/workspaces",
        Effect::Read,
    ),
];

fn root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
}
fn shipped() -> Vec<Selection> {
    let file: Value = serde_json::from_slice(
        &fs::read(root().join("providers/openrouter/operations.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(file["format"], "connectors-catalog-operations/1");
    assert_eq!(file["provider"], "openrouter");
    serde_json::from_value(file["operations"].clone()).unwrap()
}
fn committed() -> bundle::Bundle {
    bundle::load(&root().join("generated/bundles"), "openrouter").unwrap()
}

#[test]
fn shipped_openrouter_selections_are_exactly_the_three_reads() {
    let selections = shipped();
    let bundle = committed();
    assert_eq!(bundle.auth_profile, PROFILE);
    let engine = Engine::new(&bundle, BASE, &selections).unwrap();
    let mut declared: Vec<String> = engine
        .declarations(&[Effect::Read, Effect::Write])
        .into_iter()
        .map(|o| o.id)
        .collect();
    declared.sort();
    let expected: Vec<&str> = SHIPPED.iter().map(|(id, ..)| *id).collect();
    assert_eq!(declared, expected);
    assert_eq!(selections.len(), SHIPPED.len(), "no write is selected");
    for (id, operation_id, method, path, effect) in SHIPPED {
        let selection = selections.iter().find(|s| s.id == id).unwrap();
        assert_eq!(selection.operation_id, operation_id, "`{id}`");
        assert_eq!(selection.effect, effect, "`{id}`");
        assert!(selection.guard.is_none(), "`{id}` carries no guard");
        assert_eq!(engine.effect(id), Some(effect), "`{id}`");
        let operation = bundle
            .inventory
            .operations
            .iter()
            .find(|o| o.operation_id.as_deref() == Some(operation_id))
            .unwrap_or_else(|| panic!("the pinned source lacks `{operation_id}`"));
        assert_eq!(operation.method, method, "`{id}`");
        assert_eq!(operation.path, path, "`{id}`");
    }
}

#[test]
fn the_pinned_source_is_the_one_the_bundle_and_its_record_name() {
    let upstream = root().join(UPSTREAM);
    let bytes = fs::read(upstream.join(SOURCE)).unwrap();
    let digest = hex::encode(Sha256::digest(&bytes));
    assert_eq!(digest, SOURCE_SHA256);
    assert_eq!(bytes.len(), SOURCE_BYTES);
    let index = bundle::read_index(&root().join("generated/bundles")).unwrap();
    let entry = index.find("openrouter").unwrap();
    assert_eq!(entry.source_sha256, digest);
    assert_eq!(entry.operations, 150);
    assert_eq!(entry.unsupported, 2);
    let bundle = committed();
    assert_eq!(bundle.source.source_bytes, bytes.len());
    assert_eq!(bundle.source.file_name, SOURCE);
    let manifest: Value =
        serde_json::from_slice(&fs::read(upstream.join("openrouter-source-hashes.json")).unwrap())
            .unwrap();
    let records = manifest.as_array().unwrap();
    assert_eq!(records.len(), 1, "one pinned OpenRouter document");
    assert_eq!(records[0]["file"], SOURCE);
    assert_eq!(records[0]["url"], URL);
    assert_eq!(records[0]["sha256"], digest.as_str());
    assert_eq!(records[0]["bytes"], bytes.len());
    // Pinned redacted: the record names the rule and the upstream digest.
    assert_eq!(records[0]["redaction"], "upstream-redaction/3");
    assert_eq!(records[0]["upstream_sha256"], UPSTREAM_SHA256);
    let readme = fs::read_to_string(upstream.join("README.md")).unwrap();
    assert!(readme.contains(&digest), "README does not name the digest");
    assert!(
        readme.contains(UPSTREAM_SHA256),
        "README lacks the upstream digest"
    );
    assert!(readme.contains(URL), "README does not name the source URL");
    assert!(readme.contains("upstream-redaction/3"));
    assert!(
        readme.contains("2026-10-16"),
        "README lacks the retrieval date"
    );
}

/// The configuration example in the OpenRouter guide, so the fixture runs the
/// profile the guide tells a reader to write.
fn documented_config() -> Value {
    let guide = fs::read_to_string(root().join("../../docs/catalog-openrouter.md")).unwrap();
    let example = guide
        .split("```json\n")
        .skip(1)
        .filter_map(|rest| rest.split_once("\n```").map(|(body, _)| body))
        .find(|body| body.contains("\"provider\": \"openrouter\""))
        .expect("the documented OpenRouter configuration");
    serde_json::from_str(example).unwrap()
}

/// A bearer management key whose identity is the key record's own label.
#[test]
fn the_documented_profile_is_a_bearer_key_whose_subject_is_the_key_label() {
    let auth = documented_config()["auth"].clone();
    assert_eq!(auth["profile"], PROFILE);
    assert_eq!(auth["scheme"], Value::Null, "the token scheme");
    assert_eq!(auth["header"], "Authorization");
    assert_eq!(auth["bearer"], true);
    assert_eq!(
        auth["identity"],
        json!({"path": "key", "kind": "openrouter.key", "subject_pointer": "/data/label"})
    );
    assert_eq!(auth["scopes"], Value::Null);
    assert_eq!(
        documented_config()["api_base"],
        "https://openrouter.ai/api/v1"
    );
    // The probe is the pinned document's current-key read.
    assert!(
        committed()
            .inventory
            .operations
            .iter()
            .any(|o| o.operation_id.as_deref() == Some("getCurrentKey") && o.path == KEY_PATH)
    );
}

/// Method, request target, `Authorization` header and body of each fixture
/// request the fixture read in full.
type Requests = Arc<Mutex<Vec<(String, String, Option<String>, Vec<u8>)>>>;

fn key_record() -> Value {
    json!({"data": {"label": LABEL, "is_management_key": true, "is_free_tier": false,
                    "limit": 100, "limit_remaining": 74.5, "usage": 25.5}})
}
fn workspace(id: &str, name: &str) -> Value {
    json!({"id": id, "name": name, "slug": name, "created_at": "2026-09-15T15:45:00Z"})
}
fn listed_key(hash: &str, name: &str, workspace_id: &str) -> Value {
    json!({"hash": hash, "name": name, "label": name, "disabled": false,
           "workspace_id": workspace_id, "created_at": "2026-09-15T15:45:00Z"})
}

/// The fixture's answer to one request, decided by what it carries.
enum Answer {
    Status(u16, Option<Value>),
}
fn answer(method: &str, route: &str, query: &str, _body: &[u8]) -> Answer {
    match (method, route, query) {
        ("GET", KEY_PATH, "") => Answer::Status(200, Some(key_record())),
        ("GET", WORKSPACES, "") => Answer::Status(
            200,
            Some(
                json!({"data": [workspace("ws-one", "alpha"), workspace("ws-two", "beta")],
                        "total_count": 2}),
            ),
        ),
        ("GET", WORKSPACES, "offset=1&limit=1") => Answer::Status(
            200,
            Some(json!({"data": [workspace("ws-two", "beta")], "total_count": 2})),
        ),
        ("GET", KEYS, "") => Answer::Status(
            200,
            Some(json!({"data": [listed_key("hash-one", "first", "ws-one"),
                                 listed_key("hash-two", "second", "ws-two")]})),
        ),
        ("GET", KEYS, "offset=1&workspace_id=ws-two") => Answer::Status(
            200,
            Some(json!({"data": [listed_key("hash-two", "second", "ws-two")]})),
        ),
        _ => Answer::Status(
            404,
            Some(json!({"error": {"code": 404, "message": "not found"}})),
        ),
    }
}

struct Provider {
    stop: Option<oneshot::Sender<()>>,
    thread: Option<std::thread::JoinHandle<()>>,
    _root: tempfile::TempDir,
    config: PathBuf,
    requests: Requests,
}
impl Provider {
    fn new() -> Self {
        let root = tempfile::Builder::new()
            .permissions(fs::Permissions::from_mode(0o700))
            .tempdir()
            .unwrap();
        let directory = root.path().join("private");
        filesystem::directory(&directory, true, true).unwrap();
        let cert = rcgen::generate_simple_self_signed(vec!["localhost".into()]).unwrap();
        let ca = directory.join("ca.pem");
        private(&ca, cert.cert.pem().as_bytes());
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
        let requests: Requests = Arc::new(Mutex::new(Vec::new()));
        let observed = requests.clone();
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
                    let mut head = Vec::new();
                    while !head.ends_with(b"\r\n\r\n") {
                        assert!(head.len() < 8192);
                        match stream.read_u8().await {
                            Ok(byte) => head.push(byte),
                            Err(_) => break,
                        }
                    }
                    let head = String::from_utf8(head).unwrap();
                    let header = |wanted: &str| {
                        head.lines().find_map(|line| {
                            line.split_once(':')
                                .filter(|(name, _)| name.eq_ignore_ascii_case(wanted))
                                .map(|(_, value)| value.trim().to_owned())
                        })
                    };
                    let mut words = head.split_whitespace();
                    let method = words.next().unwrap_or_default().to_owned();
                    let target = words.next().unwrap_or_default().to_owned();
                    let length: usize = header("content-length")
                        .map(|value| value.parse().unwrap())
                        .unwrap_or(0);
                    assert!(length <= 65_536);
                    let mut body = vec![0; length];
                    if stream.read_exact(&mut body).await.is_err() {
                        continue;
                    }
                    let authorization = header("authorization");
                    let (route, query) = target.split_once('?').unwrap_or((&target, ""));
                    // Only the fictional key is accepted; headers are compared,
                    // never printed.
                    let reply = if authorization.as_deref() != Some(&format!("Bearer {KEY}")) {
                        Answer::Status(401, Some(json!({"error": "unauthorized"})))
                    } else {
                        answer(&method, route, query, &body)
                    };
                    observed
                        .lock()
                        .unwrap()
                        .push((method, target.clone(), authorization, body));
                    let Answer::Status(status, answer) = reply;
                    let answer = answer
                        .map(|value| serde_json::to_vec(&value).unwrap())
                        .unwrap_or_default();
                    let head = format!(
                        "HTTP/1.1 {status} fixture\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                        answer.len()
                    );
                    let _ = stream.write_all(head.as_bytes()).await;
                    let _ = stream.write_all(&answer).await;
                }
            });
        });
        let address = address_rx.recv_timeout(Duration::from_secs(5)).unwrap();
        let mut config = documented_config();
        config["instance"] = json!(INSTANCE);
        config["bundle_directory"] = json!(root_path("generated/bundles"));
        config["api_base"] = json!(format!("https://localhost:{}/api/v1", address.port()));
        config["ca_file"] = json!(ca);
        config["operations_file"] = json!(root_path("providers/openrouter/operations.json"));
        let path = directory.join("catalog.json");
        private(&path, &serde_json::to_vec(&config).unwrap());
        Self {
            stop: Some(stop),
            thread: Some(thread),
            _root: root,
            config: path,
            requests,
        }
    }
    fn selection(&self, protocol: Option<PrivateProtocol>) -> Adapter {
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
        let bootstrap: Bootstrap = serde_json::from_slice(&output.stdout).unwrap();
        bootstrap.validate().unwrap();
        let binary = PathBuf::from(env!("CARGO_BIN_EXE_connectors-catalog-provider"))
            .canonicalize()
            .unwrap();
        Adapter {
            private_protocol: protocol,
            permissions: Default::default(),
            instance_id: INSTANCE.into(),
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
    /// Every request the fixture read: method, target, `Authorization` and
    /// body as JSON (`null` when it carried none).
    fn requests(&self) -> Vec<(String, String, Option<String>, Value)> {
        self.requests
            .lock()
            .unwrap()
            .iter()
            .map(|(method, target, authorization, body)| {
                let body = if body.is_empty() {
                    Value::Null
                } else {
                    serde_json::from_slice(body).unwrap()
                };
                (method.clone(), target.clone(), authorization.clone(), body)
            })
            .collect()
    }
}
impl Drop for Provider {
    fn drop(&mut self) {
        let _ = self.stop.take().unwrap().send(());
        self.thread.take().unwrap().join().unwrap();
    }
}
fn root_path(relative: &str) -> PathBuf {
    root().join(relative).canonicalize().unwrap()
}
fn private(path: &Path, bytes: &[u8]) {
    fs::write(path, bytes).unwrap();
    fs::set_permissions(path, fs::Permissions::from_mode(0o600)).unwrap();
}
fn secret() -> Secret {
    Secret(serde_json::to_vec(&json!({"token": KEY})).unwrap())
}
fn bearer() -> Option<String> {
    Some(format!("Bearer {KEY}"))
}
fn deadline() -> u64 {
    connectors_sdk::now_ms() + 30_000
}

fn read(child: &mut Child, operation: &str, input: &Value) -> Value {
    let revision = child.bootstrap().descriptor().unwrap().revision;
    let output = child
        .invoke(
            operation,
            &revision,
            "one",
            &secret(),
            &serde_json::to_vec(input).unwrap(),
            deadline(),
        )
        .unwrap_or_else(|failure| panic!("`{operation}` failed: {failure:?}"));
    serde_json::from_slice(&output).unwrap()
}

#[test]
fn connecting_proves_the_key_and_names_its_label_as_the_subject() {
    let provider = Provider::new();
    let mut child = Child::spawn(&provider.selection(None)).unwrap();
    let baseline = child.validate(PROFILE, &secret(), deadline()).unwrap();
    assert_eq!(baseline.identity.kind, "openrouter.key");
    assert_eq!(baseline.identity.subject, LABEL);
    assert_eq!(baseline.granted_scopes, None);
    let requests = provider.requests();
    assert_eq!(requests.len(), 1);
    // The identity probe carries an empty query.
    assert_eq!(requests[0].0, "GET");
    assert_eq!(requests[0].1, format!("{KEY_PATH}?"));
    assert_eq!(requests[0].2, bearer());
    // A wrong key is answered 401 and refuses the connection.
    let wrong = Secret(br#"{"token":"wrong-key"}"#.to_vec());
    assert!(matches!(
        child.validate(PROFILE, &wrong, deadline()),
        Err(Failure::InvalidCredential)
    ));
}

#[test]
fn key_get_returns_the_key_record() {
    let provider = Provider::new();
    let mut child = Child::spawn(&provider.selection(None)).unwrap();
    let output = read(&mut child, "key.get", &json!({}));
    assert_eq!(output["status"], 200);
    assert_eq!(output["body"], key_record());
    assert_eq!(output["provenance"]["instance"], INSTANCE);
    assert_eq!(
        provider.requests(),
        [(
            "GET".to_owned(),
            format!("{KEY_PATH}?"),
            bearer(),
            Value::Null
        )]
    );
}

#[test]
fn workspaces_list_sends_its_paging_and_returns_the_workspaces() {
    let provider = Provider::new();
    let mut child = Child::spawn(&provider.selection(None)).unwrap();
    let all = read(&mut child, "workspaces.list", &json!({}));
    assert_eq!(all["status"], 200);
    assert_eq!(
        all["body"]["data"],
        json!([workspace("ws-one", "alpha"), workspace("ws-two", "beta")])
    );
    let page = read(
        &mut child,
        "workspaces.list",
        &json!({"offset": 1, "limit": 1}),
    );
    assert_eq!(page["body"]["data"], json!([workspace("ws-two", "beta")]));
    let targets: Vec<String> = provider.requests().into_iter().map(|r| r.1).collect();
    assert_eq!(
        targets,
        [
            format!("{WORKSPACES}?"),
            format!("{WORKSPACES}?offset=1&limit=1")
        ]
    );
}

#[test]
fn keys_list_filters_by_workspace_and_never_returns_a_secret() {
    let provider = Provider::new();
    let mut child = Child::spawn(&provider.selection(None)).unwrap();
    let all = read(&mut child, "keys.list", &json!({}));
    assert_eq!(all["body"]["data"].as_array().unwrap().len(), 2);
    let filtered = read(
        &mut child,
        "keys.list",
        &json!({"workspace_id": "ws-two", "offset": 1}),
    );
    assert_eq!(
        filtered["body"]["data"],
        json!([listed_key("hash-two", "second", "ws-two")])
    );
    let requests = provider.requests();
    assert_eq!(
        requests[1].1,
        format!("{KEYS}?offset=1&workspace_id=ws-two")
    );
    for request in &requests {
        assert_eq!(request.2, bearer());
        assert_eq!(request.3, Value::Null);
    }
    assert!(!all.to_string().contains("sk-or-"));
}

#[test]
fn a_parameter_the_source_does_not_declare_is_refused_before_any_request() {
    let provider = Provider::new();
    let mut child = Child::spawn(&provider.selection(None)).unwrap();
    let revision = child.bootstrap().descriptor().unwrap().revision;
    let outcome = child.invoke(
        "keys.list",
        &revision,
        "one",
        &secret(),
        &serde_json::to_vec(&json!({"unreviewed": 1})).unwrap(),
        deadline(),
    );
    assert!(matches!(outcome, Err(Failure::InvalidInput)), "{outcome:?}");
    assert!(provider.requests().is_empty());
}

/// Not a test: serves the fixture so the guide's commands can be run against it.
/// `OPENROUTER_FIXTURE_SECONDS=600 cargo test -p connectors-catalog --test openrouter
/// -- --ignored --nocapture serve_the_fixture` prints the configuration path and
/// answers until the time is up.
#[test]
#[ignore = "serves the fixture for the guide's commands"]
fn serve_the_fixture() {
    let provider = Provider::new();
    println!("FIXTURE_CONFIG={}", provider.config.display());
    println!("FIXTURE_KEY={KEY}");
    let seconds: u64 = std::env::var("OPENROUTER_FIXTURE_SECONDS")
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(300);
    std::thread::sleep(Duration::from_secs(seconds));
}
