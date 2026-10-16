//! OpenRouter management reads and the key create through the catalog provider.
//!
//! The shipped selection set is pinned by id, source operation, method and
//! path, and resolves against the committed bundle compiled from the pinned
//! OpenRouter OpenAPI document. Each operation runs through the provider child
//! against a disposable HTTPS fixture that plays OpenRouter: the exact request
//! (method, path, query, `Authorization: Bearer …`) is asserted and the answer
//! is the fixture's own. No live account, no network, and every key is
//! fictional. `key.create` is the write whose answer carries the created key
//! (design §8a): the child hands it over only in the protected slot, and a
//! second child, configured as the guide's inference alias, proves and sends
//! exactly those bytes, as the owner does when it holds them in custody.
#![cfg(unix)]
use connectors_catalog::bundle;
use connectors_catalog_provider::{Effect, Engine, Selection};
use connectors_host::local::{
    config::{Adapter, Executable, Restart, Startup},
    filesystem,
    runtime::{Bootstrap, Child, Failure, PrivateProtocol, WriteEffect, WriteResult},
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
/// The fictional key the fixture's create mints, shown once in its answer.
const CREATED: &str = "fixture-openrouter-created-inference-key";
/// The created key's public hash, the only member the caller receives.
const CREATED_HASH: &str = "fixture-created-key-hash";
/// The created key's own label, which its connection names as the subject.
const CREATED_LABEL: &str = "fixture-inference-key";
/// The guide's inference alias runs the same provider under this instance.
const INFERENCE_INSTANCE: &str = "fixture-openrouter-inference";

/// The shipped ids, their pinned `operationId`, method, path and effect. A
/// renamed, dropped or added id fails here.
const SHIPPED: [(&str, &str, &str, &str, Effect); 4] = [
    (
        "key.create",
        "createKeys",
        "post",
        "/api/v1/keys",
        Effect::Write,
    ),
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
fn shipped_openrouter_selections_are_exactly_the_three_reads_and_the_create() {
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
    assert_eq!(selections.len(), SHIPPED.len(), "one write is selected");
    // The create declares its created key and returns only the key's hash.
    let create = selections.iter().find(|s| s.id == "key.create").unwrap();
    assert!(engine.declares_custody("key.create"));
    assert_eq!(
        serde_json::to_value(create.custody.as_ref().unwrap()).unwrap(),
        json!({"secret": "/key", "returns": {"hash": "/data/hash"}})
    );
    for (id, ..) in SHIPPED.iter().filter(|(id, ..)| *id != "key.create") {
        assert!(!engine.declares_custody(id), "`{id}`");
    }
    // A closed body, without the partner `external` object (it carries a
    // partner-supplied key) or the organisation-only creator.
    assert_eq!(
        create.body_keys,
        [
            "name",
            "limit",
            "limit_reset",
            "include_byok_in_limit",
            "expires_at",
            "workspace_id"
        ]
    );
    assert_eq!(create.body_required, ["name"]);
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

/// The guide's configuration of the alias that holds created inference keys.
fn documented_inference_config() -> Value {
    let guide = fs::read_to_string(root().join("../../docs/catalog-openrouter.md")).unwrap();
    let example = guide
        .split("```json\n")
        .skip(1)
        .filter_map(|rest| rest.split_once("\n```").map(|(body, _)| body))
        .find(|body| body.contains("\"instance\": \"openrouter-inference\""))
        .expect("the documented OpenRouter inference configuration");
    serde_json::from_str(example).unwrap()
}

/// The inference alias runs the management alias's profile: a created key is
/// an ordinary OpenRouter API key, sent the same way and proved by the same
/// current-key read, whose subject is the created key's own label.
#[test]
fn the_documented_inference_alias_uses_the_same_profile_and_identity_read() {
    let management = documented_config();
    let inference = documented_inference_config();
    assert_eq!(inference["provider"], "openrouter");
    assert_eq!(inference["api_base"], management["api_base"]);
    let mut auth = inference["auth"].clone();
    assert_eq!(auth["label"], "OpenRouter inference key");
    auth["label"] = management["auth"]["label"].clone();
    assert_eq!(auth, management["auth"]);
    assert_ne!(inference["instance"], management["instance"]);
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

/// The created key's own record, as `GET /key` answers it to that key.
fn created_record() -> Value {
    json!({"data": {"label": CREATED_LABEL, "is_management_key": false, "is_free_tier": false,
                    "limit": 5, "limit_remaining": 5, "usage": 0}})
}
/// The create's answer: the one-time `key` beside the created key's record.
fn created(name: &str) -> Value {
    json!({"key": CREATED,
           "data": {"hash": CREATED_HASH, "name": name, "label": CREATED_LABEL, "disabled": false,
                    "limit": 5, "limit_remaining": 5, "limit_reset": null,
                    "include_byok_in_limit": false, "usage": 0, "workspace_id": "ws-one",
                    "created_at": "2026-10-16T10:00:00Z", "updated_at": null}})
}

/// The fixture's answer to one request, decided by what it carries.
enum Answer {
    Status(u16, Option<Value>),
    /// Read in full and never answered: the client's deadline passes after
    /// the provider received the request.
    Hold,
}
/// The create by body `name`: `refused` is a documented 400, `held` is never
/// answered, any other name mints [`CREATED`].
fn create(body: &[u8]) -> Answer {
    let body: Value = serde_json::from_slice(body).unwrap_or(Value::Null);
    match body["name"].as_str() {
        Some("held") => Answer::Hold,
        Some("refused") | None => Answer::Status(
            400,
            Some(json!({"error": {"code": 400, "message": "invalid request parameters"}})),
        ),
        Some(name) => Answer::Status(201, Some(created(name))),
    }
}
/// The created key is an ordinary API key: it reads its own record and is
/// refused every management route.
fn answer_created(method: &str, route: &str, query: &str) -> Answer {
    match (method, route, query) {
        ("GET", KEY_PATH, "") => Answer::Status(200, Some(created_record())),
        _ => Answer::Status(
            403,
            Some(json!({"error": {"code": 403, "message": "a management key is required"}})),
        ),
    }
}
fn answer(method: &str, route: &str, query: &str, body: &[u8]) -> Answer {
    match (method, route, query) {
        ("POST", KEYS, "") => create(body),
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
    inference: PathBuf,
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
                    // Only the fictional management key and the key it
                    // created are accepted; headers are compared, never
                    // printed.
                    let reply = if authorization.as_deref() == Some(&format!("Bearer {KEY}")) {
                        answer(&method, route, query, &body)
                    } else if authorization.as_deref() == Some(&format!("Bearer {CREATED}")) {
                        answer_created(&method, route, query)
                    } else {
                        Answer::Status(401, Some(json!({"error": "unauthorized"})))
                    };
                    observed
                        .lock()
                        .unwrap()
                        .push((method, target.clone(), authorization, body));
                    let (status, answer) = match reply {
                        Answer::Status(status, answer) => (status, answer),
                        Answer::Hold => {
                            // Held open past any write deadline; dropped with
                            // the fixture.
                            tokio::spawn(async move {
                                tokio::time::sleep(Duration::from_secs(60)).await;
                                drop(stream);
                            });
                            continue;
                        }
                    };
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
        // The guide's management and inference configurations, each pointed
        // at the fixture.
        let write = |mut config: Value, instance: &str, file: &str| {
            config["instance"] = json!(instance);
            config["bundle_directory"] = json!(root_path("generated/bundles"));
            config["api_base"] = json!(format!("https://localhost:{}/api/v1", address.port()));
            config["ca_file"] = json!(ca);
            config["operations_file"] = json!(root_path("providers/openrouter/operations.json"));
            let path = directory.join(file);
            private(&path, &serde_json::to_vec(&config).unwrap());
            path
        };
        let config = write(documented_config(), INSTANCE, "catalog.json");
        let inference = write(
            documented_inference_config(),
            INFERENCE_INSTANCE,
            "inference.json",
        );
        Self {
            stop: Some(stop),
            thread: Some(thread),
            _root: root,
            config,
            inference,
            requests,
        }
    }
    fn selection(&self, protocol: Option<PrivateProtocol>) -> Adapter {
        self.selection_of(&self.config, INSTANCE, protocol)
    }
    /// The guide's inference alias: the same provider under its own instance.
    fn inference(&self) -> Adapter {
        self.selection_of(&self.inference, INFERENCE_INSTANCE, None)
    }
    fn selection_of(
        &self,
        config: &Path,
        instance: &str,
        protocol: Option<PrivateProtocol>,
    ) -> Adapter {
        let output = Command::new(env!("CARGO_BIN_EXE_connectors-catalog-provider"))
            .arg("--local-config")
            .arg(config)
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
            instance_id: instance.into(),
            adapter_id: "catalog".into(),
            configuration_revision: bootstrap.configuration_revision,
            protocol: "v1alpha1".into(),
            startup: Startup::OnDemand,
            restart: Restart::Never,
            executable: Executable {
                sha256: hex::encode(Sha256::digest(fs::read(&binary).unwrap())),
                path: binary,
                args: vec!["--local-config".into(), config.to_str().unwrap().into()],
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

fn create_input(name: &str) -> Value {
    json!({"body": {"name": name, "limit": 5, "workspace_id": "ws-one"}})
}

/// Prepare and commit one `key.create` on this child, holding what its
/// protected slot carried: the transport the owner drives once it holds a
/// verified, spent approval and has admitted the custody target.
fn create_key(
    child: &mut Child,
    input: &Value,
    deadline_ms: u64,
) -> Result<(WriteResult, Option<Secret>), Failure> {
    let revision = child.bootstrap().descriptor().unwrap().revision;
    let prepared = child.prepare_write(
        "key.create",
        &revision,
        "one",
        &secret(),
        &serde_json::to_vec(input).unwrap(),
        deadline_ms,
    )?;
    Ok(prepared.commit_holding())
}

fn carries(haystack: &[u8], needle: &str) -> bool {
    haystack
        .windows(needle.len())
        .any(|window| window == needle.as_bytes())
}

/// No file under `root` carries `needle`; at least `minimum` files are read.
fn nowhere_under(root: &Path, needle: &str, minimum: usize) {
    let mut pending = vec![root.to_path_buf()];
    let mut searched = 0;
    while let Some(path) = pending.pop() {
        let metadata = fs::symlink_metadata(&path).unwrap();
        if metadata.is_dir() {
            for entry in fs::read_dir(&path).unwrap() {
                pending.push(entry.unwrap().path());
            }
        } else if metadata.is_file() {
            searched += 1;
            assert!(
                !carries(&fs::read(&path).unwrap(), needle),
                "{}",
                path.display()
            );
        }
    }
    assert!(searched >= minimum, "{searched} files searched");
}

#[test]
fn the_bootstrap_marks_only_the_create_as_creating_a_credential() {
    let provider = Provider::new();
    let child = Child::spawn(&provider.selection(Some(PrivateProtocol::V2))).unwrap();
    let requirements = &child.bootstrap().requirements;
    assert_eq!(requirements.len(), SHIPPED.len());
    for requirement in requirements {
        assert_eq!(
            requirement.custody,
            requirement.operation == "key.create",
            "`{}`",
            requirement.operation
        );
        assert_eq!(requirement.profile, PROFILE);
    }
}

/// `key.create` answers only the created key's hash; the key itself crosses
/// to the owner in the protected slot, and the inference alias proves and
/// sends exactly those bytes, as the owner's custody does (design §8a,
/// "Store": `{"token": <key>}`, the document a prompt builds). The created
/// bytes are in no result, no answer the caller sees and no file of the run.
#[test]
fn key_create_hands_over_the_created_key_and_returns_only_its_hash() {
    let provider = Provider::new();
    let mut child = Child::spawn(&provider.selection(Some(PrivateProtocol::V2))).unwrap();
    let (result, held) = create_key(&mut child, &create_input("tenant-one"), deadline()).unwrap();
    assert_eq!(result.effect, WriteEffect::Applied, "{:?}", result.result);
    let value = result.result.unwrap();
    assert_eq!(value, json!({"hash": CREATED_HASH}));
    let held = held.expect("the created key in the protected slot");
    assert_eq!(held.0, CREATED.as_bytes());
    let requests = provider.requests();
    assert_eq!(requests.len(), 1);
    assert_eq!(requests[0].0, "POST");
    assert_eq!(requests[0].1.split('?').next(), Some(KEYS));
    assert_eq!(requests[0].2, bearer());
    assert_eq!(requests[0].3, create_input("tenant-one")["body"]);

    // The owner's custody: the target profile's single entry field, then the
    // target alias's own identity probe, then a read through it.
    let mut inference = Child::spawn(&provider.inference()).unwrap();
    let profile = inference.bootstrap().profile(PROFILE).unwrap().clone();
    let [field] = profile.fields.as_slice() else {
        panic!("the profile declares one entry field");
    };
    assert!(profile.acquisition.is_none());
    let mut document = serde_json::Map::new();
    document.insert(
        field.name.clone(),
        Value::from(String::from_utf8(held.0.clone()).unwrap()),
    );
    let stored = Secret(serde_json::to_vec(&document).unwrap());
    let baseline = inference.validate(PROFILE, &stored, deadline()).unwrap();
    assert_eq!(baseline.identity.kind, "openrouter.key");
    assert_eq!(baseline.identity.subject, CREATED_LABEL);
    let revision = inference.bootstrap().descriptor().unwrap().revision;
    let output = inference
        .invoke("key.get", &revision, "one", &stored, b"{}", deadline())
        .unwrap();
    let output: Value = serde_json::from_slice(&output).unwrap();
    assert_eq!(output["body"], created_record());
    assert_eq!(output["provenance"]["instance"], INFERENCE_INSTANCE);
    let sent: Vec<_> = provider.requests()[1..]
        .iter()
        .map(|(method, target, authorization, _)| {
            (method.clone(), target.clone(), authorization.clone())
        })
        .collect();
    let created = Some(format!("Bearer {CREATED}"));
    assert_eq!(
        sent,
        [
            ("GET".to_owned(), format!("{KEY_PATH}?"), created.clone()),
            ("GET".to_owned(), format!("{KEY_PATH}?"), created),
        ]
    );
    // The created key is refused the management routes: OpenRouter's 403.
    let refused = inference.invoke("keys.list", &revision, "one", &stored, b"{}", deadline());
    assert_eq!(refused, Err(Failure::ProviderForbidden));
    assert!(!carries(&serde_json::to_vec(&value).unwrap(), CREATED));
    assert!(!carries(&serde_json::to_vec(&output).unwrap(), CREATED));
    drop((child, inference));
    nowhere_under(provider._root.path(), CREATED, 3);
}

/// A documented 4xx is refused: nothing is held, and the create is sent once.
#[test]
fn a_refused_create_holds_nothing() {
    let provider = Provider::new();
    let mut child = Child::spawn(&provider.selection(Some(PrivateProtocol::V2))).unwrap();
    let (result, held) = create_key(&mut child, &create_input("refused"), deadline()).unwrap();
    assert_eq!(result.effect, WriteEffect::Refused, "{:?}", result.result);
    assert!(result.result.is_err());
    assert!(held.is_none());
    let creates = provider.requests();
    assert_eq!(creates.len(), 1);
    assert_eq!(creates[0].0, "POST");
}

/// A create the provider received and never answered is `outcome_unknown`
/// once its deadline passes: nothing is held and it was sent exactly once.
/// Re-dispatch across a retry or a restart is the owner's attempt ledger's
/// to fence (docs/local-mutation-ledger.md).
#[test]
fn a_create_received_and_never_answered_is_unknown_and_sent_once() {
    let provider = Provider::new();
    let mut child = Child::spawn(&provider.selection(Some(PrivateProtocol::V2))).unwrap();
    let (result, held) = create_key(
        &mut child,
        &create_input("held"),
        connectors_sdk::now_ms() + 3_000,
    )
    .unwrap();
    assert_eq!(result.effect, WriteEffect::Unknown, "{:?}", result.result);
    assert!(held.is_none());
    let creates: Vec<_> = provider
        .requests()
        .into_iter()
        .filter(|(method, ..)| method == "POST")
        .collect();
    assert_eq!(creates.len(), 1);
    assert_eq!(creates[0].3, create_input("held")["body"]);
}

/// The partner `external` object carries a partner-supplied key, and the
/// selection's body is closed without it: refused before any request.
#[test]
fn a_create_body_outside_the_selection_is_refused_before_any_request() {
    let provider = Provider::new();
    let mut child = Child::spawn(&provider.selection(Some(PrivateProtocol::V2))).unwrap();
    for body in [
        json!({"name": "tenant-one", "external": {"user": "partner-user"}}),
        json!({"name": "tenant-one", "creator_user_id": "user-one"}),
        json!({"limit": 5}),
        json!({"name": ""}),
    ] {
        let refused = create_key(&mut child, &json!({"body": body}), deadline());
        assert!(
            matches!(refused, Err(Failure::InvalidInput)),
            "{body}: {:?}",
            refused.as_ref().map(|(result, _)| &result.result)
        );
    }
    assert!(provider.requests().is_empty());
}

/// Not a test: serves the fixture so the guide's commands can be run against it.
/// `OPENROUTER_FIXTURE_SECONDS=600 cargo test -p connectors-catalog-provider --test openrouter
/// -- --ignored --nocapture serve_the_fixture` prints the configuration path and
/// answers until the time is up.
#[test]
#[ignore = "serves the fixture for the guide's commands"]
fn serve_the_fixture() {
    let provider = Provider::new();
    println!("FIXTURE_CONFIG={}", provider.config.display());
    println!("FIXTURE_INFERENCE_CONFIG={}", provider.inference.display());
    println!("FIXTURE_KEY={KEY}");
    let seconds: u64 = std::env::var("OPENROUTER_FIXTURE_SECONDS")
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(300);
    std::thread::sleep(Duration::from_secs(seconds));
}
