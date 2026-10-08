//! Adversarial cases for the Runpod pods selection (`story:catalog-runpod-pods`).
//!
//! The double-billing guard is the claim under attack: a create or terminate whose
//! answer is not a documented definite refusal must never be reported `refused`, and
//! nothing may be sent twice. Each case runs the provider child against its own
//! disposable HTTPS fixture whose answer is fixed per test. The duplicate-`operationId`
//! refusal is driven on the guard preflight and on every feed binding, the two sites
//! the unit's own suite does not reach. No live Runpod account, no network.
use connectors_catalog::{
    bundle::{self, Bundle},
    ingest, inventory,
};
use connectors_catalog_provider::{Engine, Selection, feed::Declaration};
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

const INSTANCE: &str = "fixture-runpod";
const PROFILE: &str = "runpod.api-key";
const KEY: &str = "fixture-runpod-key";

fn root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
}

// ---- duplicate operationId: guard preflight and feed bindings ---------------------------

fn runpod_bundle() -> Bundle {
    bundle::load(&root().join("generated/bundles"), "runpod").unwrap()
}

/// A copy of the operation `id` names, at `<path>/again`, appended to the inventory: the
/// same `operationId` on two GETs, each of which alone would bind.
fn duplicate(bundle: &mut Bundle, id: &str) {
    let mut copy = bundle
        .inventory
        .operations
        .iter()
        .find(|o| o.operation_id.as_deref() == Some(id))
        .unwrap_or_else(|| panic!("the bundle lacks `{id}`"))
        .clone();
    copy.path = format!("{}/again", copy.path);
    bundle.inventory.operations.push(copy);
}

fn guarded_terminate() -> Vec<Selection> {
    vec![
        serde_json::from_value(json!({
            "id": "pod.terminate", "operation_id": "DeletePod", "effect": "write",
            "guard": {
                "preflight": {"operation_id": "GetPod", "values": {"podId": "podId"},
                              "checks": [{"pointer": "/id", "expect": {"input": "podId"}}]},
                "postflight": {"checks": []}
            }
        }))
        .unwrap(),
    ]
}

/// A guard whose preflight names an `operationId` the bundle carries on two GETs is
/// refused when the engine is built, instead of reading whichever comes first.
#[test]
fn a_guard_preflight_naming_a_twice_declared_operation_is_refused() {
    let selections = guarded_terminate();
    // Control: the same guard over the shipped bundle builds.
    Engine::new(&runpod_bundle(), "/v1", &selections)
        .unwrap_or_else(|e| panic!("the control guard was refused: {}", e.message));
    let mut bundle = runpod_bundle();
    duplicate(&mut bundle, "GetPod");
    let refused = Engine::new(&bundle, "/v1", &selections).err();
    assert!(
        refused
            .as_ref()
            .is_some_and(|e| e.message.contains("more than one operation `GetPod`")),
        "a guard over the twice-declared `GetPod` loaded: {:?}",
        refused.map(|e| e.message)
    );
}

fn feed_fixtures() -> PathBuf {
    root().join("tests/feed")
}

fn feed_bundle() -> Bundle {
    let bytes = fs::read(feed_fixtures().join("api.json")).unwrap();
    let document: Value = serde_json::from_slice(&bytes).unwrap();
    let file: Value =
        serde_json::from_slice(&fs::read(feed_fixtures().join("time.operations.json")).unwrap())
            .unwrap();
    Bundle {
        provider: file["provider"].as_str().unwrap().into(),
        source: ingest("api.json", &bytes).unwrap(),
        inventory: inventory::extract(&document),
        auth_profile: "fixture.token".into(),
    }
}

/// Each operation a feed declaration binds — the container list, its lookup and the item
/// list — is refused when the bundle carries its `operationId` twice.
#[test]
fn a_feed_binding_naming_a_twice_declared_operation_is_refused() {
    let file: Value =
        serde_json::from_slice(&fs::read(feed_fixtures().join("time.operations.json")).unwrap())
            .unwrap();
    let feed: Declaration = serde_json::from_value(file["feed"].clone()).unwrap();
    Engine::with_feed(&feed_bundle(), "/v1", &[], Some(&feed))
        .unwrap_or_else(|e| panic!("the control feed was refused: {}", e.message));
    for id in ["listRooms", "getRoom", "listMessages"] {
        let mut bundle = feed_bundle();
        duplicate(&mut bundle, id);
        let refused = Engine::with_feed(&bundle, "/v1", &[], Some(&feed)).err();
        assert!(
            refused
                .as_ref()
                .is_some_and(|e| e.message.contains(&format!("more than one operation `{id}`"))),
            "a feed over the twice-declared `{id}` loaded: {:?}",
            refused.map(|e| e.message)
        );
    }
}

/// Every shipped selection set, with its feed, still loads over its committed bundle now
/// that a twice-declared `operationId` is refused.
#[test]
fn every_shipped_selection_set_still_loads() {
    let index = bundle::read_index(&root().join("generated/bundles")).unwrap();
    let mut loaded = 0;
    for entry in fs::read_dir(root().join("providers")).unwrap() {
        let provider = entry.unwrap().file_name().to_string_lossy().into_owned();
        let file: Value = serde_json::from_slice(
            &fs::read(root().join("providers").join(&provider).join("operations.json")).unwrap(),
        )
        .unwrap();
        let selections: Vec<Selection> =
            serde_json::from_value(file["operations"].clone()).unwrap();
        let feed: Option<Declaration> = file
            .get("feed")
            .map(|f| serde_json::from_value(f.clone()).unwrap());
        let bundle = bundle::load(&root().join("generated/bundles"), &provider).unwrap();
        assert!(index.find(&provider).is_some(), "`{provider}` is not indexed");
        // The document base is every shipped selection's longest common leading path.
        let paths: Vec<Vec<String>> = selections
            .iter()
            .map(|s| {
                bundle
                    .inventory
                    .operations
                    .iter()
                    .find(|o| o.operation_id.as_deref() == Some(&s.operation_id))
                    .map(|o| o.path.split('/').filter(|x| !x.is_empty()).map(str::to_owned).collect())
                    .unwrap_or_default()
            })
            .collect();
        let mut base: Vec<String> = paths.first().cloned().unwrap_or_default();
        for p in &paths {
            let n = base.iter().zip(p).take_while(|(a, b)| a == b).count();
            base.truncate(n.min(p.len().saturating_sub(1)));
        }
        let base = format!("/{}", base.join("/"));
        let built = Engine::with_feed(&bundle, &base, &selections, feed.as_ref());
        if let Err(e) = &built {
            assert!(
                !e.message.contains("more than one operation"),
                "`{provider}` no longer loads: {}",
                e.message
            );
        }
        loaded += 1;
    }
    assert!(loaded >= 10, "only {loaded} shipped selection sets were read");
}

// ---- the provider child against a fixed-answer fixture -------------------------------------

/// The fixture's answer to one request.
#[derive(Clone)]
enum Reply {
    /// A status line, extra header lines, and a body.
    Answer(u16, &'static str, Vec<u8>),
    /// A status line and a `Content-Length` larger than the body, then the connection
    /// closes: the provider was reached and its answer was cut off.
    Truncated(u16),
    /// Read the request in full, then close without a reply.
    Drop,
}
type Answer = fn(&str, &str, &[u8]) -> Reply;
type Requests = Arc<Mutex<Vec<(String, String, Vec<u8>)>>>;

fn json_reply(status: u16, value: Value) -> Reply {
    Reply::Answer(status, "", serde_json::to_vec(&value).unwrap())
}

/// The probe and every list answer 200 with two pods; anything else is the case's.
fn pods() -> Value {
    json!([{"id": "fixturepod001", "name": "worker-a"}])
}

struct Provider {
    stop: Option<oneshot::Sender<()>>,
    thread: Option<std::thread::JoinHandle<()>>,
    _root: tempfile::TempDir,
    config: PathBuf,
    requests: Requests,
}
impl Provider {
    fn new(answer: Answer) -> Self {
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
                        if head.len() > 8192 {
                            break;
                        }
                        match stream.read_u8().await {
                            Ok(byte) => head.push(byte),
                            Err(_) => break,
                        }
                    }
                    let head = String::from_utf8_lossy(&head).into_owned();
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
                        .and_then(|value| value.parse().ok())
                        .unwrap_or(0)
                        .min(65_536);
                    let mut body = vec![0; length];
                    if stream.read_exact(&mut body).await.is_err() {
                        continue;
                    }
                    let route = target.split_once('?').map_or(target.as_str(), |(r, _)| r);
                    let reply = if header("authorization").as_deref()
                        != Some(&format!("Bearer {KEY}"))
                    {
                        json_reply(401, json!({"error": "unauthorized"}))
                    } else if method == "GET" && route == "/v1/pods" {
                        json_reply(200, pods())
                    } else {
                        answer(&method, route, &body)
                    };
                    observed
                        .lock()
                        .unwrap()
                        .push((method, target.clone(), body));
                    match reply {
                        Reply::Drop => {
                            let _ = stream.shutdown().await;
                        }
                        Reply::Truncated(status) => {
                            let head = format!(
                                "HTTP/1.1 {status} fixture\r\nContent-Type: application/json\r\nContent-Length: 4096\r\nConnection: close\r\n\r\n{{\"id\":\"fixture"
                            );
                            let _ = stream.write_all(head.as_bytes()).await;
                            let _ = stream.flush().await;
                            let _ = stream.shutdown().await;
                        }
                        Reply::Answer(status, extra, answer) => {
                            let head = format!(
                                "HTTP/1.1 {status} fixture\r\nContent-Type: application/json\r\n{extra}Content-Length: {}\r\nConnection: close\r\n\r\n",
                                answer.len()
                            );
                            let _ = stream.write_all(head.as_bytes()).await;
                            let _ = stream.write_all(&answer).await;
                        }
                    }
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
    fn child(&self) -> Child {
        let output = Command::new(env!("CARGO_BIN_EXE_connectors-catalog-provider"))
            .arg("--local-config")
            .arg(&self.config)
            .arg("--print-local-bootstrap")
            .output()
            .unwrap();
        assert!(output.status.success(), "bootstrap inspection failed");
        let bootstrap: Bootstrap = serde_json::from_slice(&output.stdout).unwrap();
        let binary = PathBuf::from(env!("CARGO_BIN_EXE_connectors-catalog-provider"))
            .canonicalize()
            .unwrap();
        Child::spawn(&Adapter {
            private_protocol: Some(PrivateProtocol::V2),
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
        })
        .unwrap()
    }
    /// Requests other than the identity probe and pod list.
    fn writes(&self) -> Vec<(String, String, Vec<u8>)> {
        self.requests
            .lock()
            .unwrap()
            .iter()
            .filter(|(m, ..)| m != "GET")
            .cloned()
            .collect()
    }
    fn all(&self) -> usize {
        self.requests.lock().unwrap().len()
    }
}
impl Drop for Provider {
    fn drop(&mut self) {
        let _ = self.stop.take().unwrap().send(());
        self.thread.take().unwrap().join().unwrap();
    }
}

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
fn deadline() -> u64 {
    connectors_sdk::now_ms() + 30_000
}
fn write_bytes(child: &mut Child, operation: &str, input: &[u8]) -> Result<WriteResult, Failure> {
    let revision = child.bootstrap().descriptor().unwrap().revision;
    let prepared =
        child.prepare_write(operation, &revision, "one", &secret(), input, deadline())?;
    Ok(prepared.commit())
}
fn write(child: &mut Child, operation: &str, input: &Value) -> Result<WriteResult, Failure> {
    write_bytes(child, operation, &serde_json::to_vec(input).unwrap())
}
fn create_body() -> Value {
    json!({"body": {"name": "worker-a", "imageName": "registry.example.com/fixture/worker:1",
                    "gpuTypeIds": ["NVIDIA GeForce RTX 4090"], "gpuCount": 1}})
}

/// The answer each create gets, chosen by its body's `name`.
fn create_answer(method: &str, route: &str, body: &[u8]) -> Reply {
    // The identity probe of `the_configured_identity_admits_only_a_200`.
    match (method, route) {
        ("GET", "/v1/probe/204") => return Reply::Answer(204, "", Vec::new()),
        ("GET", "/v1/probe/202") => return Reply::Answer(202, "", b"[]".to_vec()),
        ("GET", "/v1/probe/301") => return Reply::Answer(301, "Location: /v1/pods\r\n", Vec::new()),
        ("GET", "/v1/probe/307") => return Reply::Answer(307, "Location: /v1/pods\r\n", Vec::new()),
        ("GET", "/v1/probe/302") => {
            return Reply::Answer(302, "Location: https://example.invalid/v1/pods\r\n", Vec::new());
        }
        _ => {}
    }
    if method != "POST" || route != "/v1/pods" {
        return json_reply(404, json!({"error": "not found"}));
    }
    let input: Value = serde_json::from_slice(body).unwrap_or(Value::Null);
    let name = input["name"].as_str().unwrap_or_default();
    match name {
        "redirect-307" => Reply::Answer(
            307,
            "Location: /v1/pods\r\n",
            br#"{"error":"moved"}"#.to_vec(),
        ),
        "redirect-302" => Reply::Answer(302, "Location: /v1/pods\r\n", Vec::new()),
        "created-unreadable" => Reply::Answer(201, "", b"<html>created</html>".to_vec()),
        "created-truncated" => Reply::Truncated(201),
        "accepted-empty" => Reply::Answer(202, "", Vec::new()),
        "too-many" => json_reply(429, json!({"error": "rate limited"})),
        "timeout-408" => json_reply(408, json!({"error": "request timeout"})),
        "gateway-502" => json_reply(502, json!({"error": "bad gateway"})),
        "unavailable-503" => json_reply(503, json!({"error": "unavailable"})),
        "gateway-504" => json_reply(504, json!({"error": "gateway timeout"})),
        "origin-timeout-524" => Reply::Answer(524, "", b"origin timed out".to_vec()),
        "server-500-truncated" => Reply::Truncated(500),
        "dropped" => Reply::Drop,
        _ => json_reply(400, json!({"error": "invalid"})),
    }
}

/// Once the request reached the provider, every answer that is not a documented definite
/// refusal leaves the pod possibly created: none of them is `refused`, and none is sent
/// twice (a redirect is not followed).
#[test]
fn a_create_the_provider_may_have_acted_on_is_never_refused_and_never_resent() {
    let provider = Provider::new(create_answer);
    let mut child = provider.child();
    let cases = [
        "redirect-307",
        "redirect-302",
        "created-unreadable",
        "created-truncated",
        "accepted-empty",
        "too-many",
        "timeout-408",
        "gateway-502",
        "unavailable-503",
        "gateway-504",
        "origin-timeout-524",
        "server-500-truncated",
        "dropped",
    ];
    let mut outcomes = Vec::new();
    for name in cases {
        let mut input = create_body();
        input["body"]["name"] = json!(name);
        let result = write(&mut child, "pod.create", &input)
            .unwrap_or_else(|failure| panic!("`{name}` refused in prepare: {failure:?}"));
        outcomes.push((name, result.effect));
        if result.effect != WriteEffect::Applied {
            // The child may have been terminated on an unknown; start another.
            child = provider.child();
        }
    }
    let refused: Vec<_> = outcomes
        .iter()
        .filter(|(_, effect)| *effect == WriteEffect::Refused)
        .collect();
    assert!(refused.is_empty(), "refused after reaching the provider: {refused:?}");
    let writes = provider.writes();
    assert_eq!(writes.len(), cases.len(), "a create was sent other than once");
    for ((method, target, _), name) in writes.iter().zip(cases) {
        assert_eq!(method, "POST", "`{name}`");
        assert!(target.starts_with("/v1/pods"), "`{name}`: {target}");
    }
}

/// A duplicate JSON key, a case variant of an admitted key and a key outside the set are
/// each refused before any request; none of them reaches Runpod.
#[test]
fn a_create_body_cannot_smuggle_a_key_past_the_reviewed_set() {
    let provider = Provider::new(create_answer);
    let mut child = provider.child();
    let raw: [&[u8]; 6] = [
        br#"{"body":{"name":"worker-a","imageName":"i","name":"worker-b"}}"#,
        br#"{"body":{"name":"worker-a","imageName":"i","Name":"worker-b"}}"#,
        br#"{"body":{"name":"worker-a","imageName":"i","templateid":"t"}}"#,
        br#"{"body":{"name":"worker-a","imageName":"i","dockerStartCmd":["sh"]}}"#,
        br#"{"body":{"name":"worker-a","imageName":"i"},"body":{"templateId":"t"}}"#,
        br#"{"body":{"name":"worker-a","imageName":"i"},"templateId":"t"}"#,
    ];
    for input in raw {
        let outcome = write_bytes(&mut child, "pod.create", input);
        assert!(
            matches!(outcome, Err(Failure::InvalidInput | Failure::Protocol)),
            "{}: {:?}",
            String::from_utf8_lossy(input),
            outcome.map(|r| r.effect)
        );
        child = provider.child();
    }
    assert_eq!(provider.all(), 0, "a request was sent");
}

/// The identity probe admits only a `200` from the configured read: a redirect or any
/// other 2xx does not connect, and a redirect is not followed.
#[test]
fn the_configured_identity_admits_only_a_200() {
    for (status, extra) in [
        (204, ""),
        (202, ""),
        (301, "Location: /v1/pods\r\n"),
        (307, "Location: /v1/pods\r\n"),
        (302, "Location: https://example.invalid/v1/pods\r\n"),
    ] {
        // The probe path is answered by the case, not by the 200 list.
        let provider = Provider::new(create_answer);
        let config: Value = serde_json::from_slice(&fs::read(&provider.config).unwrap()).unwrap();
        let mut config = config;
        config["auth"]["identity"]["path"] = json!(format!("probe/{status}"));
        private(&provider.config, &serde_json::to_vec(&config).unwrap());
        let _ = extra;
        let mut child = provider.child();
        let validated = child.validate(PROFILE, &secret(), deadline());
        assert!(
            validated.is_err(),
            "a probe answered {status} admitted the key"
        );
        // Exactly one request, to the configured probe: the refusal is the answer's,
        // not a load failure, and the redirect was not followed.
        let requests = provider.requests.lock().unwrap().clone();
        assert_eq!(requests.len(), 1, "the probe answered {status}: {requests:?}");
        assert_eq!(requests[0].1, format!("/v1/probe/{status}?"));
    }
}

/// Terminate: a 404 is Runpod's definite answer; any other answer that is not a
/// documented refusal leaves the pod possibly running and is not `refused`.
#[test]
fn a_terminate_without_a_definite_answer_is_not_refused() {
    fn answer(method: &str, route: &str, _: &[u8]) -> Reply {
        match (method, route) {
            ("DELETE", "/v1/pods/gone") => json_reply(404, json!({"error": "pod not found"})),
            ("DELETE", "/v1/pods/failed") => json_reply(500, json!({"error": "internal"})),
            ("DELETE", "/v1/pods/dropped") => Reply::Drop,
            ("DELETE", "/v1/pods/redirected") => {
                Reply::Answer(307, "Location: /v1/pods/redirected\r\n", Vec::new())
            }
            ("DELETE", "/v1/pods/truncated") => Reply::Truncated(200),
            _ => json_reply(400, json!({"error": "invalid"})),
        }
    }
    let provider = Provider::new(answer);
    let mut child = provider.child();
    let result = write(&mut child, "pod.terminate", &json!({"podId": "gone"})).unwrap();
    assert_eq!(result.effect, WriteEffect::Refused);
    assert!(matches!(result.result, Err(Failure::NotFound | Failure::ProviderNotFound)));
    for pod in ["failed", "dropped", "redirected", "truncated"] {
        let mut child = provider.child();
        let result = write(&mut child, "pod.terminate", &json!({"podId": pod})).unwrap();
        assert_eq!(result.effect, WriteEffect::Unknown, "`{pod}`");
    }
    assert_eq!(provider.writes().len(), 5, "a terminate was sent other than once");
}
