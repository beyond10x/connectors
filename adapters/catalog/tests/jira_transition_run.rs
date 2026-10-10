//! story:catalog-guard-postflight-read: Jira `doTransition` as the guarded write
//! `issue.transition.run`.
//!
//! Jira answers the transition `204` with no body, so the guard proves it by a
//! read the engine issues after the write: `getIssue` again, whose status must
//! be the transition's target. Before the write the guard reads twice: the
//! issue (`getIssue`, its current status must be the one the caller names) and
//! the transition (`getTransitions` narrowed by `transitionId`, which Jira
//! answers with the transition only while it is available on the issue, its
//! target status in `to`).
//!
//! Each write runs through a private-protocol-two provider child against a
//! disposable HTTPS Jira, prepared and committed as the host's approval
//! coordinator drives it once it holds a verified, spent approval. The fixture
//! answers in the pinned document's shapes (`IssueBean`, `Transitions`) with
//! synthetic keys and ids, keeps each issue's status, and records each request's
//! method, target and body, so "no request was sent" is a count of what it saw.
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

const BASE: &str = "/rest/api/3";
const INSTANCE: &str = "fixture-jira";
const ACCOUNT: &str = "reader@example.test";
const TOKEN: &str = "fixture-jira-token";
/// `Basic base64("reader@example.test:fixture-jira-token")`, as in `tests/jira.rs`.
const HEADER: &str = "Basic cmVhZGVyQGV4YW1wbGUudGVzdDpmaXh0dXJlLWppcmEtdG9rZW4=";
const RUN: &str = "issue.transition.run";

/// Status ids of the fixture workflow.
const TO_DO: &str = "10000";
const IN_PROGRESS: &str = "3";
const DONE: &str = "10002";
/// Transition ids: `31` Done (to `DONE`) and `11` To Do (to `TO_DO`) are open
/// from In Progress; `41` exists in the workflow but is not open on the issue.
const FINISH: &str = "31";
const REOPEN: &str = "11";
const CLOSED: &str = "41";

/// An issue that moves as Jira would.
const ISSUE: &str = "FIX-1";
/// An issue whose transition Jira accepts with `204` and does not apply, as a
/// Jira whose post function reverted it would.
const STUCK: &str = "FIX-2";
/// An issue whose read after the transition fails with `500`.
const LOST: &str = "FIX-3";
/// An issue Jira does not find.
const ABSENT: &str = "FIX-404";

/// Method, request target and body of each fixture request.
type Requests = Arc<Mutex<Vec<(String, String, Vec<u8>)>>>;

fn root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
}
fn shipped() -> Vec<Selection> {
    let file: Value =
        serde_json::from_slice(&fs::read(root().join("providers/jira/operations.json")).unwrap())
            .unwrap();
    serde_json::from_value(file["operations"].clone()).unwrap()
}
fn bundle() -> connectors_catalog::bundle::Bundle {
    bundle::load(&root().join("generated/bundles"), "jira").unwrap()
}
fn engine() -> Engine {
    Engine::new(&bundle(), BASE, &shipped()).unwrap()
}

/// A `Transition` of the workflow, shaped as the pinned `IssueTransition`.
fn transition(id: &str) -> Value {
    let (name, to, to_name) = match id {
        FINISH => ("Done", DONE, "Done"),
        REOPEN => ("To Do", TO_DO, "To Do"),
        _ => ("Close", DONE, "Done"),
    };
    json!({"id": id, "name": name,
           "to": {"id": to, "name": to_name, "self": format!("https://tracker.example.test/rest/api/3/status/{to}")},
           "hasScreen": false, "isAvailable": true, "isConditional": false,
           "isGlobal": false, "isInitial": false, "looped": false})
}
fn issue(key: &str, status: &str) -> Value {
    json!({"id": "10001", "key": key,
           "self": format!("https://tracker.example.test/rest/api/3/issue/{key}"),
           "fields": {"summary": format!("fixture issue {key}"),
                      "status": {"id": status, "name": "fixture status",
                                 "self": format!("https://tracker.example.test/rest/api/3/status/{status}")}}})
}

struct Fixture {
    status: BTreeMap<String, String>,
    /// Whether the read of `LOST` after its transition has been answered.
    posted: BTreeMap<String, bool>,
}
impl Fixture {
    fn new() -> Self {
        Self {
            status: [ISSUE, STUCK, LOST]
                .into_iter()
                .map(|key| (key.to_owned(), IN_PROGRESS.to_owned()))
                .collect(),
            posted: BTreeMap::new(),
        }
    }
    /// The transitions open on an issue in `status`.
    fn open(status: &str) -> &'static [&'static str] {
        match status {
            IN_PROGRESS => &[REOPEN, FINISH],
            DONE => &[REOPEN],
            _ => &[FINISH],
        }
    }
    fn answer(&mut self, method: &str, target: &str, body: &[u8]) -> (u16, Option<Value>) {
        let (route, query) = target.split_once('?').unwrap_or((target, ""));
        let Some(rest) = route.strip_prefix("/rest/api/3/issue/") else {
            return (404, Some(json!({"errorMessages": ["no fixture"]})));
        };
        let parts: Vec<&str> = rest.split('/').collect();
        let key = parts[0];
        let Some(status) = self.status.get(key).cloned() else {
            return (
                404,
                Some(
                    json!({"errorMessages": ["Issue does not exist or you do not have permission to see it."]}),
                ),
            );
        };
        match (method, &parts[1..]) {
            ("GET", []) => {
                if key == LOST && self.posted.contains_key(key) {
                    return (500, Some(json!({"errorMessages": ["fixture outage"]})));
                }
                (200, Some(issue(key, &status)))
            }
            ("GET", ["transitions"]) => {
                let wanted = query
                    .split('&')
                    .find_map(|pair| pair.strip_prefix("transitionId="));
                let transitions: Vec<Value> = Self::open(&status)
                    .iter()
                    .filter(|id| wanted.is_none_or(|wanted| wanted == **id))
                    .map(|id| transition(id))
                    .collect();
                (200, Some(json!({"transitions": transitions})))
            }
            ("POST", ["transitions"]) => {
                let input: Value = serde_json::from_slice(body).unwrap_or(Value::Null);
                let Some(id) = input["transition"]["id"].as_str() else {
                    return (
                        400,
                        Some(json!({"errorMessages": ["transition is missing"]})),
                    );
                };
                if !Self::open(&status).contains(&id) {
                    return (
                        400,
                        Some(
                            json!({"errorMessages": ["Transition id is not valid for this issue."]}),
                        ),
                    );
                }
                self.posted.insert(key.to_owned(), true);
                if key != STUCK {
                    let to = transition(id)["to"]["id"].as_str().unwrap().to_owned();
                    self.status.insert(key.to_owned(), to);
                }
                (204, None)
            }
            _ => (404, Some(json!({"errorMessages": ["no fixture"]}))),
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
                    let (status, answer) = if header("authorization").as_deref() != Some(HEADER) {
                        (401, Some(json!({"errorMessages": ["fixture refusal"]})))
                    } else {
                        fixture.answer(&method, &target, &body)
                    };
                    observed.lock().unwrap().push((method, target, body));
                    // `204` carries no content at all, as Jira sends it.
                    let answer = answer
                        .map(|answer| serde_json::to_vec(&answer).unwrap())
                        .unwrap_or_default();
                    let head = if answer.is_empty() {
                        format!("HTTP/1.1 {status} fixture\r\nConnection: close\r\n\r\n")
                    } else {
                        format!(
                            "HTTP/1.1 {status} fixture\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                            answer.len()
                        )
                    };
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
                "provider": "jira",
                "bundle_directory": root_path("generated/bundles"),
                "api_base": format!("https://localhost:{}/rest/api/3", address.port()),
                "ca_file": ca,
                "auth": {
                    "profile": "atlassian.basic",
                    "scheme": "basic",
                    "header": "Authorization",
                    "bearer": false,
                    "account_label": "Account email",
                    "label": "API token",
                    "identity": {"path": "myself", "kind": "atlassian.account",
                                 "subject_pointer": "/accountId"}
                },
                "operations_file": root_path("providers/jira/operations.json"),
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
    Secret(serde_json::to_vec(&json!({"account": ACCOUNT, "token": TOKEN})).unwrap())
}
/// Prepare and commit one write on this child: the transport the host's
/// approval coordinator drives once it holds a verified, spent approval. A
/// refusal in prepare is the `Err`.
fn write(child: &mut Child, input: &Value) -> Result<WriteResult, Failure> {
    let revision = child.bootstrap().descriptor().unwrap().revision;
    let prepared = child.prepare_write(
        RUN,
        &revision,
        "one",
        &secret(),
        &serde_json::to_vec(input).unwrap(),
        connectors_sdk::now_ms() + 30_000,
    )?;
    Ok(prepared.commit())
}
fn run_input(key: &str, transition: &str, current: &str, target: &str) -> Value {
    json!({"issueIdOrKey": key, "current_status": current, "target_status": target,
           "body": {"transition": {"id": transition}}})
}
fn issue_target(key: &str) -> String {
    format!("/rest/api/3/issue/{key}?")
}
fn transitions_target(key: &str, transition: &str) -> String {
    format!("/rest/api/3/issue/{key}/transitions?transitionId={transition}")
}
fn post(key: &str, transition: &str) -> (String, String, Value) {
    (
        "POST".to_owned(),
        format!("/rest/api/3/issue/{key}/transitions?"),
        json!({"transition": {"id": transition}}),
    )
}
fn get(target: String) -> (String, String, Value) {
    ("GET".to_owned(), target, Value::Null)
}

/// `issue.transition.run` is shipped from the pinned `doTransition` as a write
/// beside the seven reads. Its input is the issue, the caller's current and
/// target status ids, and a body closed to `transition`, `fields` and `update`;
/// its guard reads the issue and the transition before, and the issue again
/// after.
#[test]
fn transition_run_is_shipped_as_a_guarded_write_of_do_transition() {
    let selections = shipped();
    let run = selections.iter().find(|s| s.id == RUN).expect(RUN);
    assert_eq!(run.operation_id, "doTransition");
    assert_eq!(run.effect, Effect::Write);
    let engine = engine();
    let writes: Vec<String> = engine
        .declarations(&[Effect::Write])
        .into_iter()
        .map(|o| o.id)
        .collect();
    assert_eq!(writes, [RUN]);
    let declared = engine
        .declarations(&[Effect::Write])
        .into_iter()
        .next()
        .unwrap();
    let mut keys: Vec<&str> = declared.input_schema["properties"]
        .as_object()
        .unwrap()
        .keys()
        .map(String::as_str)
        .collect();
    keys.sort();
    assert_eq!(
        keys,
        ["body", "current_status", "issueIdOrKey", "target_status"]
    );
    assert_eq!(
        declared.input_schema["properties"]["body"],
        json!({"type": "object",
               "properties": {"fields": {}, "transition": {}, "update": {}},
               "required": ["transition"], "additionalProperties": false})
    );
    assert_eq!(
        serde_json::to_value(run.guard.as_ref().unwrap()).unwrap(),
        json!({
            "preflight": {"operation_id": "getIssue",
                          "values": {"issueIdOrKey": "issueIdOrKey"},
                          "checks": [{"pointer": "/fields/status/id",
                                      "expect": {"input": "current_status"}}]},
            "further_preflights": [
                {"operation_id": "getTransitions",
                 "values": {"issueIdOrKey": "issueIdOrKey", "transitionId": "body.transition.id"},
                 "checks": [{"pointer": "/transitions/0/id", "expect": {"input": "body.transition.id"}},
                            {"pointer": "/transitions/0/isAvailable", "expect": {"literal": "true"}},
                            {"pointer": "/transitions/0/to/id", "expect": {"input": "target_status"}}]}],
            "postflight": {"read": {"operation_id": "getIssue",
                                    "values": {"issueIdOrKey": "issueIdOrKey"}},
                           "checks": [{"pointer": "/fields/status/id",
                                       "expect": {"input": "target_status"}}]}})
    );
}

/// Approved, a transition reads the issue, then the transition, sends one
/// `POST` with exactly the body given, and reads the issue again: Jira's `204`
/// carries nothing, and the re-read's status is the target, so the effect is
/// applied. The answer is the write's own: `204` and no body.
#[test]
fn an_approved_transition_reads_twice_posts_once_and_proves_the_target_by_a_read_after() {
    let provider = Provider::new();
    let mut child = Child::spawn(&provider.selection()).unwrap();
    let result = write(&mut child, &run_input(ISSUE, FINISH, IN_PROGRESS, DONE)).unwrap();
    assert_eq!(result.effect, WriteEffect::Applied, "{:?}", result.result);
    let output = result.result.unwrap();
    assert_eq!(output["status"], 204);
    assert_eq!(output["body"], Value::Null);
    assert_eq!(output["provenance"]["instance"], INSTANCE);
    assert_eq!(
        provider.requests(),
        [
            get(issue_target(ISSUE)),
            get(transitions_target(ISSUE, FINISH)),
            post(ISSUE, FINISH),
            get(issue_target(ISSUE)),
        ]
    );
    // And back: the issue is Done now, and To Do is open from there.
    let before = provider.requests().len();
    let result = write(&mut child, &run_input(ISSUE, REOPEN, DONE, TO_DO)).unwrap();
    assert_eq!(result.effect, WriteEffect::Applied, "{:?}", result.result);
    assert_eq!(provider.requests()[before..].len(), 4);
}

/// The guard refuses before the `POST`: a current status other than the
/// caller's after the first read alone; a transition that is not open on the
/// issue, and one whose target is not the caller's, after the second read. An
/// issue Jira does not find is refused after the first read. No `POST` is sent.
#[test]
fn the_preflight_refuses_a_wrong_status_and_an_unavailable_transition_before_the_post() {
    let provider = Provider::new();
    let mut child = Child::spawn(&provider.selection()).unwrap();
    for (input, reads, refusal) in [
        // The issue is In Progress, not To Do.
        (
            run_input(ISSUE, FINISH, TO_DO, DONE),
            vec![get(issue_target(ISSUE))],
            Failure::Forbidden,
        ),
        (
            run_input(ABSENT, FINISH, IN_PROGRESS, DONE),
            vec![get(issue_target(ABSENT))],
            Failure::Forbidden,
        ),
        // `41` is not open from In Progress: Jira's narrowed answer is empty.
        (
            run_input(ISSUE, CLOSED, IN_PROGRESS, DONE),
            vec![
                get(issue_target(ISSUE)),
                get(transitions_target(ISSUE, CLOSED)),
            ],
            Failure::Protocol,
        ),
        // `31` is open, but it goes to Done, not To Do.
        (
            run_input(ISSUE, FINISH, IN_PROGRESS, TO_DO),
            vec![
                get(issue_target(ISSUE)),
                get(transitions_target(ISSUE, FINISH)),
            ],
            Failure::Forbidden,
        ),
    ] {
        let before = provider.requests().len();
        let outcome = write(&mut child, &input).map(|result| result.effect);
        assert_eq!(outcome, Err(refusal), "{input}");
        assert_eq!(provider.requests()[before..], reads, "{input}");
    }
    assert!(
        provider
            .requests()
            .iter()
            .all(|(method, _, _)| method == "GET"),
        "a transition was posted past a refusing guard"
    );
}

/// After the `204` only the read decides: a re-read whose status is not the
/// target (Jira accepted the transition and did not apply it), and a re-read
/// that fails, each leave the effect unknown, never refused and never applied.
#[test]
fn a_read_after_the_transition_that_disagrees_or_fails_leaves_the_effect_unknown() {
    let provider = Provider::new();
    let mut child = Child::spawn(&provider.selection()).unwrap();
    for key in [STUCK, LOST] {
        let before = provider.requests().len();
        let result = write(&mut child, &run_input(key, FINISH, IN_PROGRESS, DONE)).unwrap();
        assert_eq!(result.effect, WriteEffect::Unknown, "{key}");
        assert_eq!(result.result, Err(Failure::Protocol), "{key}");
        assert_eq!(
            provider.requests()[before..],
            [
                get(issue_target(key)),
                get(transitions_target(key, FINISH)),
                post(key, FINISH),
                get(issue_target(key)),
            ],
            "{key}"
        );
    }
}

/// Inputs the guard needs, and a body key the selection does not admit, are
/// refused before any request.
#[test]
fn absent_guard_inputs_and_unadmitted_body_keys_are_refused_before_any_request() {
    let provider = Provider::new();
    let mut child = Child::spawn(&provider.selection()).unwrap();
    let full = run_input(ISSUE, FINISH, IN_PROGRESS, DONE);
    let mut cases = Vec::new();
    for key in ["current_status", "target_status", "body"] {
        let mut input = full.clone();
        input.as_object_mut().unwrap().remove(key);
        cases.push(input);
    }
    let mut no_id = full.clone();
    no_id["body"] = json!({"transition": {"name": "Done"}});
    cases.push(no_id);
    let mut extra = full.clone();
    extra["body"]["historyMetadata"] = json!({"type": "fixture"});
    cases.push(extra);
    for input in cases {
        let outcome = write(&mut child, &input).map(|result| result.effect);
        assert_eq!(outcome, Err(Failure::InvalidInput), "{input}");
    }
    assert!(provider.requests().is_empty(), "{:?}", provider.requests());
}

/// The selection format refuses, when the selection loads, a postflight read
/// with no check to make of it, a read through anything but a GET under the
/// base path, a further preflight without checks, more than three further
/// preflights, and a credential parameter bound by a read's values.
#[test]
fn malformed_guard_reads_are_refused_when_the_selection_loads() {
    let bundle = bundle();
    let base = shipped();
    let with = |change: &dyn Fn(&mut Value)| -> Result<Engine, String> {
        let mut selections: Vec<Value> = base
            .iter()
            .map(|s| serde_json::to_value(s).unwrap())
            .collect();
        let run = selections.iter_mut().find(|s| s["id"] == RUN).unwrap();
        change(&mut run["guard"]);
        let selections: Vec<Selection> =
            serde_json::from_value(Value::Array(selections)).map_err(|e| e.to_string())?;
        Engine::new(&bundle, BASE, &selections).map_err(|e| e.to_string())
    };
    assert!(with(&|_| {}).is_ok());
    for (name, change) in [
        (
            "postflight read without checks",
            Box::new(|guard: &mut Value| guard["postflight"]["checks"] = json!([]))
                as Box<dyn Fn(&mut Value)>,
        ),
        (
            "postflight read through the write itself",
            Box::new(|guard: &mut Value| {
                guard["postflight"]["read"]["operation_id"] = json!("doTransition")
            }),
        ),
        (
            "postflight read of an operation the bundle lacks",
            Box::new(|guard: &mut Value| {
                guard["postflight"]["read"]["operation_id"] = json!("getIssueEverywhere")
            }),
        ),
        (
            "further preflight without checks",
            Box::new(|guard: &mut Value| guard["further_preflights"][0]["checks"] = json!([])),
        ),
        (
            "four further preflights",
            Box::new(|guard: &mut Value| {
                let one = guard["further_preflights"][0].clone();
                guard["further_preflights"] = json!([one.clone(), one.clone(), one.clone(), one]);
            }),
        ),
        (
            "further preflight check without a pointer",
            Box::new(|guard: &mut Value| {
                guard["further_preflights"][0]["checks"][0]["pointer"] = json!("transitions/0/id")
            }),
        ),
    ] {
        assert!(with(&*change).is_err(), "{name} loaded");
    }
    // An unknown member of a read is refused by the format itself.
    assert!(
        with(&|guard: &mut Value| guard["postflight"]["read"]["checks"] = json!([]))
            .err()
            .unwrap()
            .contains("unknown field")
    );
}
