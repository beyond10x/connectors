//! Runpod pods created, listed and terminated through the catalog provider.
//!
//! The shipped selection set is pinned by id, source operation, method and
//! path, and resolves against the committed bundle compiled from the pinned
//! Runpod REST API document. Each operation runs through the provider child
//! against a disposable HTTPS fixture that plays Runpod: the exact request
//! (method, path, body, `Authorization: Bearer …`) is asserted, and every write
//! outcome the guide documents is produced by the fixture's own answer — a
//! created pod, a 400 refusal, a connection dropped after the request was read,
//! a 5xx, a terminate answered 204, 200 and 404. No live Runpod account, no
//! network, and the key is fictional.
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

/// The document's server is `https://rest.runpod.io/v1`, so every operation
/// sits below `/v1`.
const BASE: &str = "/v1";
const INSTANCE: &str = "fixture-runpod";
const PROFILE: &str = "runpod.api-key";
/// The fictional key the fixture accepts, as `Authorization: Bearer <key>`.
const KEY: &str = "fixture-runpod-key";
/// The pinned source, its manifest and README, relative to this crate.
const UPSTREAM: &str = "../runpod/upstream";
const SOURCE: &str = "runpod-rest-v1.json";
const URL: &str = "https://rest.runpod.io/v1/openapi.json";
const SOURCE_SHA256: &str = "9500a8989878d53d8731f27bf8dbbd57801b328c760bdb32c38ba36d5cb580db";
const SOURCE_BYTES: usize = 154_609;
const PODS: &str = "/v1/pods";

/// The shipped ids, their pinned `operationId`, method, path and effect. A
/// renamed, dropped or added id fails here.
const SHIPPED: [(&str, &str, &str, &str, Effect); 3] = [
    ("pod.create", "CreatePod", "post", "/v1/pods", Effect::Write),
    (
        "pod.terminate",
        "DeletePod",
        "delete",
        "/v1/pods/{podId}",
        Effect::Write,
    ),
    ("pods.list", "ListPods", "get", "/v1/pods", Effect::Read),
];

/// The create body keys the selection admits, in the order the guide lists
/// them.
const BODY_KEYS: [&str; 12] = [
    "name",
    "imageName",
    "gpuTypeIds",
    "gpuCount",
    "containerDiskInGb",
    "volumeInGb",
    "volumeMountPath",
    "ports",
    "env",
    "cloudType",
    "dataCenterIds",
    "interruptible",
];

fn root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
}
fn shipped() -> Vec<Selection> {
    let file: Value =
        serde_json::from_slice(&fs::read(root().join("providers/runpod/operations.json")).unwrap())
            .unwrap();
    assert_eq!(file["format"], "connectors-catalog-operations/1");
    assert_eq!(file["provider"], "runpod");
    serde_json::from_value(file["operations"].clone()).unwrap()
}
fn committed() -> bundle::Bundle {
    bundle::load(&root().join("generated/bundles"), "runpod").unwrap()
}

#[test]
fn shipped_runpod_selections_are_exactly_create_list_and_terminate() {
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

/// The create body is closed to the reviewed keys, and the declared input
/// says so; terminate takes the pod id and nothing else.
#[test]
fn the_create_body_is_closed_to_the_reviewed_keys() {
    let selection = shipped()
        .into_iter()
        .find(|s| s.id == "pod.create")
        .unwrap();
    assert_eq!(selection.body_keys, BODY_KEYS.map(str::to_owned).to_vec());
    let engine = Engine::new(&committed(), BASE, &shipped()).unwrap();
    let writes = engine.declarations(&[Effect::Write]);
    let create = writes.iter().find(|o| o.id == "pod.create").unwrap();
    let body = &create.input_schema["properties"]["body"];
    assert_eq!(body["type"], "object");
    assert_eq!(body["additionalProperties"], json!(false));
    let mut keys: Vec<&str> = body["properties"]
        .as_object()
        .unwrap()
        .keys()
        .map(String::as_str)
        .collect();
    keys.sort();
    let mut expected = BODY_KEYS.to_vec();
    expected.sort();
    assert_eq!(keys, expected);
    let terminate = writes.iter().find(|o| o.id == "pod.terminate").unwrap();
    assert_eq!(terminate.input_schema["required"], json!(["podId"]));
    let names: Vec<&str> = terminate.input_schema["properties"]
        .as_object()
        .unwrap()
        .keys()
        .map(String::as_str)
        .collect();
    assert_eq!(names, ["podId"]);
}

/// The pinned document names `UpdatePod` on both `PATCH /pods/{podId}` and
/// `POST /pods/{podId}/update`. The inventory keeps both; neither ships, and a
/// selection naming an `operationId` the bundle carries twice is refused at
/// load instead of resolving to whichever comes first.
#[test]
fn the_duplicate_update_pod_is_kept_and_cannot_be_selected() {
    let bundle = committed();
    let mut updates: Vec<(&str, &str)> = bundle
        .inventory
        .operations
        .iter()
        .filter(|o| o.operation_id.as_deref() == Some("UpdatePod"))
        .map(|o| (o.method.as_str(), o.path.as_str()))
        .collect();
    updates.sort();
    assert_eq!(
        updates,
        [
            ("patch", "/v1/pods/{podId}"),
            ("post", "/v1/pods/{podId}/update")
        ]
    );
    let mut selections = shipped();
    selections[0].operation_id = "UpdatePod".into();
    selections[0].body_keys.clear();
    let refused = Engine::new(&bundle, BASE, &selections).err();
    assert!(
        refused
            .as_ref()
            .is_some_and(|error| error.message.contains("UpdatePod")),
        "a selection of the twice-declared `UpdatePod` loaded: {refused:?}"
    );
    // A missing operation and a write shipped as a read are refused too.
    let mut selections = shipped();
    selections[0].operation_id = "TerminatePod".into();
    assert!(Engine::new(&bundle, BASE, &selections).is_err());
    let mut selections = shipped();
    let list = selections.iter_mut().find(|s| s.id == "pods.list").unwrap();
    list.operation_id = "StopPod".into();
    assert!(Engine::new(&bundle, BASE, &selections).is_err());
}

#[test]
fn the_pinned_source_is_the_one_the_bundle_and_its_record_name() {
    let upstream = root().join(UPSTREAM);
    let bytes = fs::read(upstream.join(SOURCE)).unwrap();
    let digest = hex::encode(Sha256::digest(&bytes));
    assert_eq!(digest, SOURCE_SHA256);
    assert_eq!(bytes.len(), SOURCE_BYTES);
    let index = bundle::read_index(&root().join("generated/bundles")).unwrap();
    let entry = index.find("runpod").unwrap();
    assert_eq!(entry.source_sha256, digest);
    assert_eq!(entry.operations, 37);
    assert_eq!(entry.unsupported, 0);
    let bundle = committed();
    assert_eq!(bundle.source.source_bytes, bytes.len());
    assert_eq!(bundle.source.file_name, SOURCE);
    let manifest: Value =
        serde_json::from_slice(&fs::read(upstream.join("runpod-source-hashes.json")).unwrap())
            .unwrap();
    let records = manifest.as_array().unwrap();
    assert_eq!(records.len(), 1, "one pinned Runpod document");
    assert_eq!(records[0]["file"], SOURCE);
    assert_eq!(records[0]["url"], URL);
    assert_eq!(records[0]["sha256"], digest.as_str());
    assert_eq!(records[0]["bytes"], bytes.len());
    // Pinned as served: no redaction rule and no separate upstream digest.
    assert_eq!(records[0]["redaction"], Value::Null);
    assert_eq!(records[0]["upstream_sha256"], Value::Null);
    // The gate's archived-source check (`connectors-build source-hashes`)
    // re-derives the digest and length from this gzip copy of the same bytes.
    assert_eq!(records[0]["archive"], "vendor/runpod-rest-v1.json.gz");
    let archived = Command::new("gzip")
        .arg("-dc")
        .arg(upstream.join("vendor/runpod-rest-v1.json.gz"))
        .output()
        .unwrap();
    assert!(archived.status.success());
    assert!(archived.stdout == bytes, "the archive holds other bytes");
    let readme = fs::read_to_string(upstream.join("README.md")).unwrap();
    assert!(readme.contains(&digest), "README does not name the digest");
    assert!(readme.contains(URL), "README does not name the source URL");
}

/// The configuration example in the Runpod guide, so the fixture runs the
/// profile the guide tells a reader to write.
fn documented_config() -> Value {
    let guide = fs::read_to_string(root().join("../../docs/catalog-runpod.md")).unwrap();
    let example = guide
        .split("```json\n")
        .skip(1)
        .filter_map(|rest| rest.split_once("\n```").map(|(body, _)| body))
        .find(|body| body.contains("\"provider\": \"runpod\""))
        .expect("the documented Runpod configuration");
    serde_json::from_str(example).unwrap()
}

/// A bearer API key, and an identity that is the configured connection: the
/// pinned document has no account or user read, so the probe is `GET /pods`,
/// whose 200 proves the key, and the subject is the instance id.
#[test]
fn the_documented_profile_is_a_bearer_key_whose_subject_is_the_connection() {
    let auth = documented_config()["auth"].clone();
    assert_eq!(auth["profile"], PROFILE);
    assert_eq!(auth["scheme"], Value::Null, "the token scheme");
    assert_eq!(auth["header"], "Authorization");
    assert_eq!(auth["bearer"], true);
    assert_eq!(
        auth["identity"],
        json!({"source": "configuration", "path": "pods", "kind": "runpod.connection"})
    );
    assert_eq!(auth["scopes"], Value::Null);
    assert_eq!(documented_config()["api_base"], "https://rest.runpod.io/v1");
    // The probe is the pinned document's pod list.
    assert!(
        committed()
            .inventory
            .operations
            .iter()
            .any(|o| o.operation_id.as_deref() == Some("ListPods") && o.path == PODS)
    );
}

/// The guide's approval policy entry names exactly the two writes.
#[test]
fn the_documented_policy_admits_exactly_the_two_writes() {
    let guide = fs::read_to_string(root().join("../../docs/catalog-runpod.md")).unwrap();
    assert!(
        guide.contains(r#"{"operations": ["pod.create", "pod.terminate"]}"#),
        "the guide does not show the policy entry for the two writes"
    );
    for (id, ..) in SHIPPED {
        assert!(
            guide.contains(&format!("--operation {id}")),
            "the guide has no example invocation of `{id}`"
        );
    }
}

/// Method, request target, `Authorization` header and body of each fixture
/// request the fixture read in full.
type Requests = Arc<Mutex<Vec<(String, String, Option<String>, Vec<u8>)>>>;

fn pod(id: &str, name: &str) -> Value {
    json!({"id": id, "name": name, "desiredStatus": "RUNNING",
           "imageName": "registry.example.com/fixture/worker:1", "gpuCount": 1,
           "costPerHr": 0.69, "consumerUserId": "fixture-user",
           "ports": ["8000/http"], "env": {"MODE": "fixture"}})
}

/// The fixture's answer to one request, decided by what it carries: a create
/// is answered by its body's `name`, a terminate by its pod id.
enum Answer {
    Status(u16, Option<Value>),
    /// Read the whole request, then close the connection without a reply.
    Drop,
}
fn answer(method: &str, route: &str, query: &str, body: &[u8]) -> Answer {
    match (method, route) {
        ("GET", PODS) if query.is_empty() => Answer::Status(
            200,
            Some(json!([
                pod("fixturepod001", "worker-a"),
                pod("fixturepod002", "worker-b")
            ])),
        ),
        ("GET", PODS) if query == "desiredStatus=RUNNING&name=worker-a" => {
            Answer::Status(200, Some(json!([pod("fixturepod001", "worker-a")])))
        }
        ("POST", PODS) => {
            let input: Value = serde_json::from_slice(body).unwrap_or(Value::Null);
            match input["name"].as_str() {
                Some("created-201") => {
                    Answer::Status(201, Some(pod("fixturepod201", "created-201")))
                }
                Some("created-200") => {
                    Answer::Status(200, Some(pod("fixturepod200", "created-200")))
                }
                Some("dropped") => Answer::Drop,
                Some("failed") => Answer::Status(500, Some(json!({"error": "internal"}))),
                _ => Answer::Status(400, Some(json!({"error": "invalid input"}))),
            }
        }
        ("DELETE", "/v1/pods/fixturepod204") => Answer::Status(204, None),
        ("DELETE", "/v1/pods/fixturepod200") => Answer::Status(200, None),
        _ => Answer::Status(404, Some(json!({"error": "pod not found"}))),
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
                    let (status, answer) = match reply {
                        Answer::Drop => {
                            // The request was read in full; no reply follows.
                            let _ = stream.shutdown().await;
                            continue;
                        }
                        Answer::Status(status, answer) => (status, answer),
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
        let mut config = documented_config();
        config["instance"] = json!(INSTANCE);
        config["bundle_directory"] = json!(root_path("generated/bundles"));
        config["api_base"] = json!(format!("https://localhost:{}/v1", address.port()));
        config["ca_file"] = json!(ca);
        config["operations_file"] = json!(root_path("providers/runpod/operations.json"));
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
/// Prepare and commit one write: the transport the host's approval
/// coordinator drives once it holds a verified, spent approval. A refusal in
/// prepare is the `Err`.
fn write(child: &mut Child, operation: &str, input: &Value) -> Result<WriteResult, Failure> {
    let revision = child.bootstrap().descriptor().unwrap().revision;
    let prepared = child.prepare_write(
        operation,
        &revision,
        "one",
        &secret(),
        &serde_json::to_vec(input).unwrap(),
        deadline(),
    )?;
    Ok(prepared.commit())
}
fn create_body(name: &str) -> Value {
    json!({"name": name, "imageName": "registry.example.com/fixture/worker:1",
           "gpuTypeIds": ["NVIDIA GeForce RTX 4090"], "gpuCount": 1,
           "containerDiskInGb": 20, "volumeInGb": 0, "ports": ["8000/http"],
           "env": {"MODE": "fixture"}, "cloudType": "SECURE"})
}

/// The model's invariants (`connectors_catalog.identity.Probe`): a configured
/// identity reads a path and names no pointer, an API identity names both.
/// The provider refuses any other shape when the configuration loads.
#[test]
fn a_configured_identity_needs_a_path_and_no_pointer() {
    let provider = Provider::new();
    let documented: Value = serde_json::from_slice(&fs::read(&provider.config).unwrap()).unwrap();
    let loads = |identity: Value| {
        let mut config = documented.clone();
        config["auth"]["identity"] = identity;
        private(&provider.config, &serde_json::to_vec(&config).unwrap());
        Command::new(env!("CARGO_BIN_EXE_connectors-catalog-provider"))
            .arg("--local-config")
            .arg(&provider.config)
            .arg("--print-local-bootstrap")
            .output()
            .unwrap()
            .status
            .success()
    };
    assert!(loads(
        json!({"source": "configuration", "path": "pods", "kind": "runpod.connection"})
    ));
    for refused in [
        json!({"source": "configuration", "kind": "runpod.connection"}),
        json!({"source": "configuration", "path": "/", "kind": "runpod.connection"}),
        json!({"source": "configuration", "path": "pods", "kind": "runpod.connection",
               "subject_pointer": "/0/consumerUserId"}),
        json!({"path": "pods", "kind": "runpod.connection"}),
        json!({"source": "id_token", "kind": "runpod.connection"}),
        json!({"source": "configured", "path": "pods", "kind": "runpod.connection"}),
    ] {
        assert!(!loads(refused.clone()), "loaded {refused}");
    }
    assert!(provider.requests().is_empty());
}

#[test]
fn connecting_proves_the_key_with_the_pod_list_and_names_the_connection() {
    let provider = Provider::new();
    let mut child = Child::spawn(&provider.selection(None)).unwrap();
    let baseline = child.validate(PROFILE, &secret(), deadline()).unwrap();
    assert_eq!(baseline.identity.kind, "runpod.connection");
    assert_eq!(baseline.identity.subject, INSTANCE);
    assert_eq!(baseline.granted_scopes, None);
    let requests = provider.requests();
    assert_eq!(requests.len(), 1);
    // The identity probe carries an empty query.
    assert_eq!(requests[0].0, "GET");
    assert_eq!(requests[0].1, format!("{PODS}?"));
    assert_eq!(requests[0].2, bearer());
    // A wrong key is answered 401 and refuses the connection.
    let wrong = Secret(br#"{"token":"wrong-key"}"#.to_vec());
    assert!(matches!(
        child.validate(PROFILE, &wrong, deadline()),
        Err(Failure::InvalidCredential)
    ));
}

#[test]
fn pods_list_sends_its_filters_and_returns_the_pods() {
    let provider = Provider::new();
    let mut child = Child::spawn(&provider.selection(None)).unwrap();
    let revision = child.bootstrap().descriptor().unwrap().revision;
    let output = child
        .invoke(
            "pods.list",
            &revision,
            "one",
            &secret(),
            &serde_json::to_vec(&json!({"desiredStatus": "RUNNING", "name": "worker-a"})).unwrap(),
            deadline(),
        )
        .unwrap_or_else(|failure| panic!("`pods.list` failed: {failure:?}"));
    let output: Value = serde_json::from_slice(&output).unwrap();
    assert_eq!(output["status"], 200);
    assert_eq!(output["body"], json!([pod("fixturepod001", "worker-a")]));
    assert_eq!(output["provenance"]["instance"], INSTANCE);
    assert_eq!(
        provider.requests(),
        [(
            "GET".to_owned(),
            format!("{PODS}?desiredStatus=RUNNING&name=worker-a"),
            bearer(),
            Value::Null
        )]
    );
}

#[test]
fn a_created_pod_is_applied_and_returned() {
    let provider = Provider::new();
    let mut child = Child::spawn(&provider.selection(Some(PrivateProtocol::V2))).unwrap();
    for (name, status, id) in [
        ("created-201", 201, "fixturepod201"),
        ("created-200", 200, "fixturepod200"),
    ] {
        let result = write(
            &mut child,
            "pod.create",
            &json!({"body": create_body(name)}),
        )
        .unwrap_or_else(|failure| panic!("`pod.create` refused in prepare: {failure:?}"));
        assert_eq!(result.effect, WriteEffect::Applied, "`{name}`");
        let output = result.result.unwrap();
        assert_eq!(output["status"], status);
        assert_eq!(output["body"], pod(id, name));
    }
    let requests = provider.requests();
    assert_eq!(requests.len(), 2, "one request per create, no preflight");
    for ((method, target, authorization, body), name) in
        requests.into_iter().zip(["created-201", "created-200"])
    {
        assert_eq!(method, "POST");
        assert_eq!(target, format!("{PODS}?"));
        assert_eq!(authorization, bearer());
        assert_eq!(body, create_body(name));
    }
}

/// A 400 is Runpod's documented definite refusal: nothing was created.
#[test]
fn a_create_answered_400_is_refused() {
    let provider = Provider::new();
    let mut child = Child::spawn(&provider.selection(Some(PrivateProtocol::V2))).unwrap();
    let result = write(
        &mut child,
        "pod.create",
        &json!({"body": create_body("rejected")}),
    )
    .unwrap();
    assert_eq!(result.effect, WriteEffect::Refused);
    assert!(result.result.is_err());
    assert_eq!(provider.requests().len(), 1);
}

/// The double-billing guard: once the request has been sent, a connection
/// lost before any answer, or a 5xx, leaves the pod possibly created. Both
/// are `unknown`, never `refused`, and each was sent exactly once.
#[test]
fn a_create_without_a_definite_answer_is_outcome_unknown() {
    let provider = Provider::new();
    let mut child = Child::spawn(&provider.selection(Some(PrivateProtocol::V2))).unwrap();
    for name in ["dropped", "failed"] {
        let result = write(
            &mut child,
            "pod.create",
            &json!({"body": create_body(name)}),
        )
        .unwrap();
        assert_eq!(result.effect, WriteEffect::Unknown, "`{name}`");
        assert!(result.result.is_err(), "`{name}`");
    }
    let requests = provider.requests();
    assert_eq!(requests.len(), 2, "each create was sent exactly once");
    assert_eq!(requests[0].3, create_body("dropped"));
    assert_eq!(requests[1].3, create_body("failed"));
}

/// A body key outside `body_keys` is refused as invalid input in prepare,
/// before any request.
#[test]
fn a_create_body_key_outside_the_reviewed_set_is_refused_before_any_request() {
    let provider = Provider::new();
    let mut child = Child::spawn(&provider.selection(Some(PrivateProtocol::V2))).unwrap();
    let mut body = create_body("created-201");
    body["templateId"] = json!("fixture-template");
    let outcome = write(&mut child, "pod.create", &json!({"body": body}));
    assert!(
        matches!(outcome, Err(Failure::InvalidInput)),
        "an unreviewed key: {:?}",
        outcome.map(|result| result.effect)
    );
    assert!(provider.requests().is_empty(), "a request was sent");
}

/// Runpod documents `204` for a deleted pod; a `200` is a success too. A pod
/// that is already gone is answered `404`, which is the provider's definite
/// answer: `refused` with `not_found`, and nothing was terminated by this call.
#[test]
fn terminate_reports_204_and_200_as_applied_and_404_as_refused() {
    let provider = Provider::new();
    let mut child = Child::spawn(&provider.selection(Some(PrivateProtocol::V2))).unwrap();
    for (pod, status) in [("fixturepod204", 204), ("fixturepod200", 200)] {
        let result = write(&mut child, "pod.terminate", &json!({"podId": pod})).unwrap();
        assert_eq!(result.effect, WriteEffect::Applied, "`{pod}`");
        let output = result.result.unwrap();
        assert_eq!(output["status"], status);
        assert_eq!(output["body"], Value::Null);
    }
    let result = write(
        &mut child,
        "pod.terminate",
        &json!({"podId": "fixturegone"}),
    )
    .unwrap();
    assert_eq!(result.effect, WriteEffect::Refused);
    assert!(
        matches!(
            result.result,
            Err(Failure::NotFound | Failure::ProviderNotFound)
        ),
        "{:?}",
        result.result
    );
    let requests: Vec<(String, String)> = provider
        .requests()
        .into_iter()
        .map(|(method, target, authorization, body)| {
            assert_eq!(authorization, bearer());
            assert_eq!(body, Value::Null);
            (method, target)
        })
        .collect();
    assert_eq!(
        requests,
        [
            ("DELETE".into(), "/v1/pods/fixturepod204?".into()),
            ("DELETE".into(), "/v1/pods/fixturepod200?".into()),
            ("DELETE".into(), "/v1/pods/fixturegone?".into()),
        ]
    );
}

/// The writes need an approval: the read transport, which carries none,
/// refuses them on protocol two, and protocol one declares no write at all.
#[test]
fn the_writes_are_not_reachable_without_an_approval() {
    let provider = Provider::new();
    let mut child = Child::spawn(&provider.selection(Some(PrivateProtocol::V2))).unwrap();
    for operation in ["pod.create", "pod.terminate"] {
        let requirement = child
            .bootstrap()
            .requirements
            .iter()
            .find(|r| r.operation == operation)
            .unwrap_or_else(|| panic!("`{operation}` is not a protocol-two requirement"));
        assert_eq!(requirement.profile, PROFILE);
        let revision = child.bootstrap().descriptor().unwrap().revision;
        let outcome = child.invoke(
            operation,
            &revision,
            "one",
            &secret(),
            br#"{"podId":"fixturepod204"}"#,
            deadline(),
        );
        assert!(
            matches!(outcome, Err(Failure::Unsupported)),
            "`{operation}` on the read transport: {outcome:?}"
        );
    }
    let mut one = Child::spawn(&provider.selection(None)).unwrap();
    assert!(
        one.bootstrap()
            .requirements
            .iter()
            .all(|r| r.operation != "pod.create" && r.operation != "pod.terminate")
    );
    let outcome = write(
        &mut one,
        "pod.terminate",
        &json!({"podId": "fixturepod204"}),
    );
    assert!(matches!(outcome, Err(Failure::Unsupported)));
    assert!(provider.requests().is_empty(), "a request was sent");
}
