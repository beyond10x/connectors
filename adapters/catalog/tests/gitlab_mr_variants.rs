//! story:catalog-selection-fixed-body-value: two guarded variants of existing
//! GitLab merge-request writes, each sending one body value the selection fixes.
//!
//! - `merge_request.auto_merge`, `putApiV4ProjectsIdMergeRequestsMergeRequestIidMerge`
//!   (`PUT /projects/{id}/merge_requests/{merge_request_iid}/merge`) with
//!   `auto_merge` fixed to `true`: GitLab merges once the pipeline succeeds,
//!   or at once when it already has. The guard pins the head and the open
//!   state, and deliberately not a succeeded pipeline. Its postflight accepts
//!   either answer: auto-merge set, or merged.
//! - `merge_request.reopen`, `putApiV4ProjectsIdMergeRequestsMergeRequestIid`
//!   (`PUT /projects/{id}/merge_requests/{merge_request_iid}`) with exactly
//!   `state_event: reopen`: the merge request must be closed at the pinned head
//!   before, and opened after.
//!
//! A caller's body carrying a fixed key is refused before any request, whatever
//! its value; the fixed value is always what is sent.
//!
//! Each write runs through a private-protocol-two provider child against a
//! disposable HTTPS GitLab, prepared and committed as the host's approval
//! coordinator drives it once it holds a verified, spent approval. The fixture
//! answers in the pinned document's merge request shape (`APIEntitiesMergeRequest`,
//! the fields the guards read) with synthetic ids, keeps each merge request's
//! state, and records each request's method, target and body, so "no request
//! was sent" is a count of what it saw.
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
    collections::BTreeMap,
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

const BASE: &str = "/api/v4";
const INSTANCE: &str = "fixture-gitlab";
/// The fictional token the fixture accepts in `PRIVATE-TOKEN`.
const TOKEN: &str = "fixture-pat-one";
/// The fixture project `org/project`, its path id encoded as one segment.
const PROJECT: &str = "/api/v4/projects/org%2Fproject";
const AUTO_MERGE: &str = "merge_request.auto_merge";
const REOPEN: &str = "merge_request.reopen";
/// The head of every fixture merge request.
const HEAD: &str = "0123456789abcdef0123456789abcdef01234567";
const OTHER: &str = "89abcdef0123456789abcdef0123456789abcdef";

/// Open, its head pipeline still running: auto-merge is set and it stays open.
const RUNNING: u64 = 7;
/// Open, its head pipeline already succeeded: auto-merge merges it at once.
const PASSED: u64 = 8;
/// Open, but GitLab answers the merge PUT without merging or setting
/// auto-merge, as a GitLab that ignored `auto_merge` would.
const IGNORED: u64 = 9;
/// Closed: reopening opens it.
const CLOSED: u64 = 10;
/// Closed, and GitLab answers the reopen PUT still closed.
const STUCK: u64 = 11;
/// Merged: neither variant may act on it.
const MERGED: u64 = 12;
/// A merge request GitLab does not find.
const ABSENT: u64 = 404;

/// Method, request target and body of each fixture request.
type Requests = Arc<Mutex<Vec<(String, String, Vec<u8>)>>>;

fn root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
}
fn shipped() -> Vec<Selection> {
    let file: Value =
        serde_json::from_slice(&fs::read(root().join("providers/gitlab/operations.json")).unwrap())
            .unwrap();
    serde_json::from_value(file["operations"].clone()).unwrap()
}
fn bundle() -> connectors_catalog::bundle::Bundle {
    bundle::load(&root().join("generated/bundles"), "gitlab").unwrap()
}
fn engine() -> Engine {
    Engine::new(&bundle(), BASE, &shipped()).unwrap()
}

/// A merge request GitLab answers, with the fields of the pinned
/// `APIEntitiesMergeRequest` the guards read.
fn merge_request(iid: u64, state: &str, auto_merge: bool, pipeline: &str) -> Value {
    json!({"id": 500 + iid, "iid": iid, "project_id": 7, "title": format!("fixture {iid}"),
           "state": state, "sha": HEAD, "merge_when_pipeline_succeeds": auto_merge,
           "detailed_merge_status": if state == "opened" { "mergeable" } else { "not_open" },
           "head_pipeline": {"id": 900 + iid, "sha": HEAD, "status": pipeline}})
}

struct Fixture {
    /// Each merge request's state and whether auto-merge is set, by IID.
    state: BTreeMap<u64, (String, bool)>,
}
impl Fixture {
    fn new() -> Self {
        Self {
            state: [
                (RUNNING, "opened"),
                (PASSED, "opened"),
                (IGNORED, "opened"),
                (CLOSED, "closed"),
                (STUCK, "closed"),
                (MERGED, "merged"),
            ]
            .into_iter()
            .map(|(iid, state)| (iid, (state.to_owned(), false)))
            .collect(),
        }
    }
    fn pipeline(iid: u64) -> &'static str {
        if iid == RUNNING { "running" } else { "success" }
    }
    fn current(&self, iid: u64) -> Value {
        let (state, auto_merge) = &self.state[&iid];
        merge_request(iid, state, *auto_merge, Self::pipeline(iid))
    }
    fn answer(&mut self, method: &str, route: &str, body: &[u8]) -> (u16, Value) {
        let Some(rest) = route.strip_prefix(&format!("{PROJECT}/merge_requests/")) else {
            return (404, json!({"message": "404 Project Not Found"}));
        };
        let parts: Vec<&str> = rest.split('/').collect();
        let Some(iid) = parts[0]
            .parse::<u64>()
            .ok()
            .filter(|iid| self.state.contains_key(iid))
        else {
            return (404, json!({"message": "404 Not found"}));
        };
        let input: Value = serde_json::from_slice(body).unwrap_or(Value::Null);
        let state = self.state[&iid].0.clone();
        match (method, &parts[1..]) {
            ("GET", []) => (200, self.current(iid)),
            ("PUT", ["merge"]) => {
                if state != "opened" {
                    return (405, json!({"message": "405 Method Not Allowed"}));
                }
                if input["sha"].as_str() != Some(HEAD) {
                    return (
                        409,
                        json!({"message": "SHA does not match HEAD of source branch"}),
                    );
                }
                let wait =
                    input["auto_merge"].as_bool() == Some(true) && Self::pipeline(iid) != "success";
                match iid {
                    IGNORED => {}
                    _ if wait => {
                        self.state.insert(iid, ("opened".into(), true));
                    }
                    _ => {
                        self.state.insert(iid, ("merged".into(), false));
                    }
                }
                (200, self.current(iid))
            }
            ("PUT", []) => {
                match (input["state_event"].as_str(), state.as_str()) {
                    (Some("reopen"), "closed") if iid != STUCK => {
                        self.state.insert(iid, ("opened".into(), false));
                    }
                    (Some("close"), "opened") => {
                        self.state.insert(iid, ("closed".into(), false));
                    }
                    _ => {}
                }
                (200, self.current(iid))
            }
            _ => (404, json!({"message": "404 Not found"})),
        }
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
                let mut fixture = Fixture::new();
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
                    let route = target.split('?').next().unwrap_or_default().to_owned();
                    let (status, answer) = if header("private-token").as_deref() != Some(TOKEN) {
                        (401, json!({"message": "401 Unauthorized"}))
                    } else {
                        fixture.answer(&method, &route, &body)
                    };
                    observed.lock().unwrap().push((method, target, body));
                    let answer = serde_json::to_vec(&answer).unwrap();
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
        let config = directory.join("catalog.json");
        private(
            &config,
            &serde_json::to_vec(&json!({
                "format": "connectors-catalog-local/2",
                "instance": INSTANCE,
                "provider": "gitlab",
                "bundle_directory": root_path("generated/bundles"),
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
                "operations_file": root_path("providers/gitlab/operations.json"),
            }))
            .unwrap(),
        );
        Self {
            stop: Some(stop),
            thread: Some(thread),
            _root: root,
            config,
            requests,
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
        let bootstrap: Bootstrap = serde_json::from_slice(&output.stdout).unwrap();
        bootstrap.validate().unwrap();
        let binary = PathBuf::from(env!("CARGO_BIN_EXE_connectors-catalog-provider"))
            .canonicalize()
            .unwrap();
        Adapter {
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
        }
    }
    /// Every request the fixture saw, as method, target and body parsed as
    /// JSON (`null` when it carried none).
    fn requests(&self) -> Vec<(String, String, Value)> {
        self.requests
            .lock()
            .unwrap()
            .iter()
            .map(|(method, target, body)| {
                let body = if body.is_empty() {
                    Value::Null
                } else {
                    serde_json::from_slice(body).unwrap()
                };
                (method.clone(), target.clone(), body)
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
    Secret(format!(r#"{{"token":"{TOKEN}"}}"#).into_bytes())
}
/// Prepare and commit one write on this child: the transport the host's
/// approval coordinator drives once it holds a verified, spent approval. A
/// refusal in prepare is the `Err`.
fn write(child: &mut Child, operation: &str, input: &Value) -> Result<WriteResult, Failure> {
    let revision = child.bootstrap().descriptor().unwrap().revision;
    let prepared = child.prepare_write(
        operation,
        &revision,
        "one",
        &secret(),
        &serde_json::to_vec(input).unwrap(),
        connectors_sdk::now_ms() + 30_000,
    )?;
    Ok(prepared.commit())
}
fn declared(operation: &str) -> connectors_core::Operation {
    engine()
        .declarations(&[Effect::Write])
        .into_iter()
        .find(|o| o.id == operation)
        .unwrap_or_else(|| panic!("`{operation}` is not a declared write"))
}

fn auto_merge_input(iid: u64, sha: &str) -> Value {
    json!({"id": "org/project", "merge_request_iid": iid, "body": {"sha": sha}})
}
fn reopen_input(iid: u64, sha: &str) -> Value {
    json!({"id": "org/project", "merge_request_iid": iid, "sha": sha})
}
fn get(iid: u64) -> (String, String, Value) {
    (
        "GET".to_owned(),
        format!("{PROJECT}/merge_requests/{iid}?"),
        Value::Null,
    )
}
fn put_merge(iid: u64) -> (String, String, Value) {
    (
        "PUT".to_owned(),
        format!("{PROJECT}/merge_requests/{iid}/merge?"),
        json!({"sha": HEAD, "auto_merge": true}),
    )
}
fn put_reopen(iid: u64) -> (String, String, Value) {
    (
        "PUT".to_owned(),
        format!("{PROJECT}/merge_requests/{iid}?"),
        json!({"state_event": "reopen"}),
    )
}

/// Both variants are shipped from the pinned operations of the existing merge
/// and update, as guarded writes whose selections fix one body value, close
/// the body around it, and leave the fixed key out of the declared input.
#[test]
fn both_variants_are_shipped_as_guarded_writes_with_one_fixed_body_value() {
    let selections = shipped();
    let selection =
        |id: &str| serde_json::to_value(selections.iter().find(|s| s.id == id).unwrap()).unwrap();
    assert_eq!(
        selection(AUTO_MERGE),
        json!({
            "id": AUTO_MERGE,
            "operation_id": "putApiV4ProjectsIdMergeRequestsMergeRequestIidMerge",
            "effect": "write",
            "description": "Set an open merge request at the pinned head to merge when its pipeline succeeds (auto_merge, fixed to true), or merge it at once when the pipeline already has",
            "guard": {
                "preflight": {"operation_id": "getApiV4ProjectsIdMergeRequestsMergeRequestIid",
                              "values": {"id": "id", "merge_request_iid": "merge_request_iid"},
                              "checks": [{"pointer": "/sha", "expect": {"input": "body.sha"}},
                                         {"pointer": "/state", "expect": {"literal": "opened"}}]},
                "postflight": {"checks": [{"pointer": "/sha", "expect": {"input": "body.sha"}}],
                               "any_of": [{"pointer": "/merge_when_pipeline_succeeds", "expect": {"literal": "true"}},
                                          {"pointer": "/state", "expect": {"literal": "merged"}}]}},
            "response": null,
            "body_keys": ["auto_merge", "sha"],
            "body_fixed": {"auto_merge": true}
        })
    );
    assert_eq!(
        selection(REOPEN),
        json!({
            "id": REOPEN,
            "operation_id": "putApiV4ProjectsIdMergeRequestsMergeRequestIid",
            "effect": "write",
            "description": "Reopen a closed merge request whose head is the pinned sha (state_event, fixed to reopen)",
            "guard": {
                "preflight": {"operation_id": "getApiV4ProjectsIdMergeRequestsMergeRequestIid",
                              "values": {"id": "id", "merge_request_iid": "merge_request_iid"},
                              "checks": [{"pointer": "/sha", "expect": {"input": "sha"}},
                                         {"pointer": "/state", "expect": {"literal": "closed"}}]},
                "postflight": {"checks": [{"pointer": "/state", "expect": {"literal": "opened"}}]}},
            "response": null,
            "body_keys": ["state_event"],
            "body_fixed": {"state_event": "reopen"}
        })
    );
    // The fixed key is not caller input: the auto-merge body declares only
    // `sha`, and the reopen body declares nothing and may be omitted.
    let auto_merge = declared(AUTO_MERGE);
    assert_eq!(
        auto_merge.input_schema["properties"]["body"],
        json!({"type": "object",
               "properties": {"sha": {"type": ["string", "integer", "boolean"]}},
               "required": ["sha"], "additionalProperties": false})
    );
    let reopen = declared(REOPEN);
    assert_eq!(
        reopen.input_schema["properties"]["body"],
        json!({"type": "object", "properties": {}, "additionalProperties": false})
    );
    assert!(
        !reopen.input_schema["required"]
            .as_array()
            .unwrap()
            .contains(&json!("body")),
        "{}",
        reopen.input_schema
    );
}

/// Approved, the auto-merge variant reads the merge request, sends one `PUT
/// …/merge` whose body is the caller's `sha` and the fixed `auto_merge: true`,
/// and is applied when GitLab answers with auto-merge set (the pipeline still
/// runs) and when it answers merged (the pipeline had already succeeded).
#[test]
fn auto_merge_sends_the_fixed_value_and_accepts_either_answer() {
    let provider = Provider::new();
    let mut child = Child::spawn(&provider.selection()).unwrap();
    for (iid, state, waiting) in [(RUNNING, "opened", true), (PASSED, "merged", false)] {
        let before = provider.requests().len();
        let result = write(&mut child, AUTO_MERGE, &auto_merge_input(iid, HEAD)).unwrap();
        assert_eq!(
            result.effect,
            WriteEffect::Applied,
            "{iid}: {:?}",
            result.result
        );
        let output = result.result.unwrap();
        assert_eq!(output["status"], 200);
        assert_eq!(output["body"]["state"], state, "{iid}");
        assert_eq!(
            output["body"]["merge_when_pipeline_succeeds"], waiting,
            "{iid}"
        );
        assert_eq!(
            provider.requests()[before..],
            [get(iid), put_merge(iid)],
            "{iid}"
        );
    }
}

/// A merge PUT answered with neither auto-merge set nor the merge request
/// merged leaves the effect unknown, never refused and never applied.
#[test]
fn an_auto_merge_answer_with_neither_accepted_value_is_unknown() {
    let provider = Provider::new();
    let mut child = Child::spawn(&provider.selection()).unwrap();
    let result = write(&mut child, AUTO_MERGE, &auto_merge_input(IGNORED, HEAD)).unwrap();
    assert_eq!(result.effect, WriteEffect::Unknown);
    assert_eq!(result.result, Err(Failure::Protocol));
    assert_eq!(provider.requests(), [get(IGNORED), put_merge(IGNORED)]);
}

/// Approved, reopen reads the merge request, sends one `PUT` whose body is
/// exactly `{"state_event": "reopen"}`, with the caller's body omitted or
/// empty, and is applied when GitLab answers it opened; an answer still
/// closed leaves the effect unknown.
#[test]
fn reopen_sends_exactly_the_fixed_state_event_and_proves_the_open_state() {
    let provider = Provider::new();
    let mut child = Child::spawn(&provider.selection()).unwrap();
    let result = write(&mut child, REOPEN, &reopen_input(CLOSED, HEAD)).unwrap();
    assert_eq!(result.effect, WriteEffect::Applied, "{:?}", result.result);
    assert_eq!(result.result.unwrap()["body"]["state"], "opened");
    assert_eq!(provider.requests(), [get(CLOSED), put_reopen(CLOSED)]);

    let mut empty = reopen_input(STUCK, HEAD);
    empty["body"] = json!({});
    let before = provider.requests().len();
    let result = write(&mut child, REOPEN, &empty).unwrap();
    assert_eq!(result.effect, WriteEffect::Unknown);
    assert_eq!(result.result, Err(Failure::Protocol));
    assert_eq!(
        provider.requests()[before..],
        [get(STUCK), put_reopen(STUCK)]
    );
}

/// The guards refuse before the `PUT`: auto-merge of a merge request at
/// another head, or not open; reopen of one at another head, open already, or
/// merged; and either of one GitLab does not find. Each sends only the read.
#[test]
fn each_preflight_mismatch_is_refused_before_the_put() {
    let provider = Provider::new();
    let mut child = Child::spawn(&provider.selection()).unwrap();
    for (operation, input, iid) in [
        (AUTO_MERGE, auto_merge_input(RUNNING, OTHER), RUNNING),
        (AUTO_MERGE, auto_merge_input(CLOSED, HEAD), CLOSED),
        (AUTO_MERGE, auto_merge_input(MERGED, HEAD), MERGED),
        (AUTO_MERGE, auto_merge_input(ABSENT, HEAD), ABSENT),
        (REOPEN, reopen_input(CLOSED, OTHER), CLOSED),
        (REOPEN, reopen_input(RUNNING, HEAD), RUNNING),
        (REOPEN, reopen_input(MERGED, HEAD), MERGED),
        (REOPEN, reopen_input(ABSENT, HEAD), ABSENT),
    ] {
        let before = provider.requests().len();
        let outcome = write(&mut child, operation, &input).map(|result| result.effect);
        assert_eq!(outcome, Err(Failure::Forbidden), "{operation} {input}");
        assert_eq!(
            provider.requests()[before..],
            [get(iid)],
            "{operation} {input}"
        );
    }
}

/// A caller's body naming a fixed key is refused before any request, whatever
/// the value: the one the selection fixes, any other, or `null`. So is a key
/// the closed body does not admit, such as GitLab's deprecated
/// `merge_when_pipeline_succeeds` or the update's `title`, and an absent `sha`.
#[test]
fn a_caller_value_for_a_fixed_key_is_refused_before_any_request() {
    let provider = Provider::new();
    let mut child = Child::spawn(&provider.selection()).unwrap();
    let mut cases = Vec::new();
    for value in [json!(true), json!(false), json!("true"), Value::Null] {
        let mut input = auto_merge_input(RUNNING, HEAD);
        input["body"]["auto_merge"] = value;
        cases.push((AUTO_MERGE, input));
    }
    for value in [json!("reopen"), json!("close"), Value::Null] {
        let mut input = reopen_input(CLOSED, HEAD);
        input["body"] = json!({"state_event": value});
        cases.push((REOPEN, input));
    }
    let mut deprecated = auto_merge_input(RUNNING, HEAD);
    deprecated["body"]["merge_when_pipeline_succeeds"] = json!(true);
    cases.push((AUTO_MERGE, deprecated));
    cases.push((
        AUTO_MERGE,
        json!({"id": "org/project", "merge_request_iid": RUNNING, "body": {}}),
    ));
    let mut retitled = reopen_input(CLOSED, HEAD);
    retitled["body"] = json!({"title": "reopened"});
    cases.push((REOPEN, retitled));
    for (operation, input) in cases {
        let outcome = write(&mut child, operation, &input).map(|result| result.effect);
        assert_eq!(outcome, Err(Failure::InvalidInput), "{operation} {input}");
    }
    assert!(provider.requests().is_empty(), "{:?}", provider.requests());
}

/// The engine refuses the same at its own boundary, without the host: a body
/// naming a fixed key is refused by `prepare` as invalid input, and nothing
/// is read.
#[test]
fn the_engine_refuses_a_fixed_key_itself() {
    let engine = engine();
    let mut input = auto_merge_input(RUNNING, HEAD);
    input["body"]["auto_merge"] = json!(true);
    let http = Unreachable;
    let refusal = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(engine.prepare(&http, INSTANCE, AUTO_MERGE, input))
        .err()
        .expect("a fixed key was admitted");
    assert_eq!(refusal.code, connectors_core::ErrorCode::InvalidInput);
}

/// A read port the refused write must never reach.
struct Unreachable;
#[async_trait::async_trait]
impl connectors_sdk::AuthenticatedHttp for Unreachable {
    async fn get(
        &self,
        _: &[&str],
        _: &[(&str, String)],
    ) -> connectors_core::Result<connectors_sdk::HttpResponse> {
        panic!("a request was sent past a refusal")
    }
}

/// The selection format refuses, when the selection loads, a fixed key its
/// `body_keys` do not admit (or with no `body_keys` at all), a fixed value that
/// is not a string, an integer or a boolean, one that is not of the key's
/// `body_types` type, a fixed key a guard reads or `body_required` names, and
/// an `any_of` of fewer than two comparisons.
#[test]
fn malformed_fixed_values_and_alternatives_are_refused_when_the_selection_loads() {
    let bundle = bundle();
    let base = shipped();
    let with = |id: &str, change: &dyn Fn(&mut Value)| -> Result<Engine, String> {
        let mut selections: Vec<Value> = base
            .iter()
            .map(|s| serde_json::to_value(s).unwrap())
            .collect();
        let selection = selections.iter_mut().find(|s| s["id"] == id).unwrap();
        change(selection);
        let selections: Vec<Selection> =
            serde_json::from_value(Value::Array(selections)).map_err(|e| e.to_string())?;
        Engine::new(&bundle, BASE, &selections).map_err(|e| e.to_string())
    };
    assert!(with(AUTO_MERGE, &|_| {}).is_ok());
    for (name, id, change) in [
        (
            "fixed key outside body_keys",
            AUTO_MERGE,
            Box::new(|s: &mut Value| s["body_keys"] = json!(["sha"])) as Box<dyn Fn(&mut Value)>,
        ),
        (
            "fixed key without body_keys",
            REOPEN,
            Box::new(|s: &mut Value| {
                s.as_object_mut().unwrap().remove("body_keys");
            }),
        ),
        (
            "fixed object",
            REOPEN,
            Box::new(|s: &mut Value| s["body_fixed"]["state_event"] = json!({"event": "reopen"})),
        ),
        (
            "fixed null",
            REOPEN,
            Box::new(|s: &mut Value| s["body_fixed"]["state_event"] = Value::Null),
        ),
        (
            "fixed fraction",
            REOPEN,
            Box::new(|s: &mut Value| s["body_fixed"]["state_event"] = json!(1.5)),
        ),
        (
            "fixed value of another body_type",
            AUTO_MERGE,
            Box::new(|s: &mut Value| s["body_types"] = json!({"auto_merge": "string"})),
        ),
        (
            "fixed key a guard reads",
            AUTO_MERGE,
            Box::new(|s: &mut Value| {
                s["guard"]["preflight"]["checks"][0]["expect"] = json!({"input": "body.auto_merge"})
            }),
        ),
        (
            "fixed key body_required names",
            AUTO_MERGE,
            Box::new(|s: &mut Value| s["body_required"] = json!(["auto_merge"])),
        ),
        (
            "any_of of one comparison",
            AUTO_MERGE,
            Box::new(|s: &mut Value| {
                let one = s["guard"]["postflight"]["any_of"][0].clone();
                s["guard"]["postflight"]["any_of"] = json!([one]);
            }),
        ),
        (
            "any_of comparison without a pointer",
            AUTO_MERGE,
            Box::new(|s: &mut Value| {
                s["guard"]["postflight"]["any_of"][1]["pointer"] = json!("state")
            }),
        ),
    ] {
        assert!(with(id, &*change).is_err(), "{name} loaded");
    }
    // A fixed value of the key's declared type loads.
    assert!(
        with(AUTO_MERGE, &|s: &mut Value| s["body_types"] =
            json!({"auto_merge": "boolean"}))
        .is_ok()
    );
}
