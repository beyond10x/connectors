//! story:parity-jira-issue-create-and-comment and story:parity-jira-issue-writes:
//! Jira `createIssue`, `addComment`, `editIssue`, `linkIssues` and `deleteIssue`
//! as the guarded writes `issue.create`, `issue.comment.add`, `issue.edit`,
//! `issue.link.add` and `issue.delete`.
//!
//! Each write runs through a private-protocol-two provider child against a
//! disposable HTTPS Jira, prepared and committed as the host's approval
//! coordinator drives it once it holds a verified, spent approval. The fixture
//! answers in the pinned document's shapes (`Project`, `IssueBean`,
//! `CreatedIssue`, `Comment`) with synthetic keys and ids, keeps each issue's
//! `updated` stamp, and records each request's method, target and body, so "no
//! request was sent" is a count of what it saw. Rich text (an issue's
//! description, a comment's content) is an Atlassian Document Format value the
//! caller sends and the provider passes on unchanged.
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

const CREATE: &str = "issue.create";
const COMMENT: &str = "issue.comment.add";
const EDIT: &str = "issue.edit";
const LINK: &str = "issue.link.add";
const DELETE: &str = "issue.delete";

/// The one project the fixture knows.
const PROJECT: &str = "PROJ";
/// An issue that changes as Jira would.
const ISSUE: &str = "PROJ-1";
/// A second issue, the outward end of a link.
const OTHER: &str = "PROJ-2";
/// An issue whose edit Jira answers `409`, a conflicting update.
const BUSY: &str = "PROJ-3";
/// An issue Jira answers `204` to deleting and still finds afterwards.
const STUCK: &str = "PROJ-4";
/// An issue Jira does not find.
const ABSENT: &str = "PROJ-404";
/// The key Jira gives the issue a create makes.
const CREATED: &str = "PROJ-10";
/// The `updated` stamp of every fixture issue, the one an edit moves it to,
/// and one that is out of date.
const STAMP: &str = "2026-10-01T09:00:00.000+0000";
const EDITED: &str = "2026-10-01T09:05:00.000+0000";
const STALE: &str = "2026-09-30T09:00:00.000+0000";

/// Method, request target and body of each fixture request.
type Requests = Arc<Mutex<Vec<(String, String, Vec<u8>)>>>;

fn root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
}
fn selection_file() -> Value {
    serde_json::from_slice(&fs::read(root().join("providers/jira/operations.json")).unwrap())
        .unwrap()
}
fn shipped() -> Vec<Selection> {
    serde_json::from_value(selection_file()["operations"].clone()).unwrap()
}
fn bundle() -> connectors_catalog::bundle::Bundle {
    bundle::load(&root().join("generated/bundles"), "jira").unwrap()
}
fn engine() -> Engine {
    Engine::new(&bundle(), BASE, &shipped()).unwrap()
}

/// One paragraph of text as an Atlassian Document Format document.
fn document(text: &str) -> Value {
    json!({"type": "doc", "version": 1,
           "content": [{"type": "paragraph", "content": [{"type": "text", "text": text}]}]})
}
fn issue(key: &str, updated: &str) -> Value {
    json!({"id": "10001", "key": key,
           "self": format!("https://tracker.example.test/rest/api/3/issue/{key}"),
           "fields": {"summary": format!("fixture issue {key}"), "updated": updated,
                      "status": {"id": "3", "name": "In Progress"}}})
}

struct Fixture {
    /// Each issue the fixture finds, by key, with its `updated` stamp.
    issues: BTreeMap<String, String>,
}
impl Fixture {
    fn new() -> Self {
        Self {
            issues: [ISSUE, OTHER, BUSY, STUCK]
                .into_iter()
                .map(|key| (key.to_owned(), STAMP.to_owned()))
                .collect(),
        }
    }
    fn answer(&mut self, method: &str, target: &str, body: &[u8]) -> (u16, Option<Value>) {
        let route = target.split_once('?').map_or(target, |(route, _)| route);
        let input: Value = serde_json::from_slice(body).unwrap_or(Value::Null);
        let missing = || (404, Some(json!({"errorMessages": ["no fixture"]})));
        match (method, route) {
            ("GET", "/rest/api/3/project/PROJ") => {
                return (
                    200,
                    Some(
                        json!({"id": "10000", "key": PROJECT, "name": "Fixture project",
                                "self": "https://tracker.example.test/rest/api/3/project/10000"}),
                    ),
                );
            }
            ("GET", route) if route.starts_with("/rest/api/3/project/") => {
                return (
                    404,
                    Some(json!({"errorMessages": ["No project could be found."]})),
                );
            }
            // `CreatedIssue`.
            ("POST", "/rest/api/3/issue") => {
                return (
                    201,
                    Some(json!({"id": "10010", "key": CREATED,
                                "self": "https://tracker.example.test/rest/api/3/issue/10010"})),
                );
            }
            // Jira answers a link with `201` and no body.
            ("POST", "/rest/api/3/issueLink") => return (201, None),
            _ => {}
        }
        let Some(rest) = route.strip_prefix("/rest/api/3/issue/") else {
            return missing();
        };
        let parts: Vec<&str> = rest.split('/').collect();
        let key = parts[0];
        let Some(updated) = self.issues.get(key).cloned() else {
            return (
                404,
                Some(
                    json!({"errorMessages": ["Issue does not exist or you do not have permission to see it."]}),
                ),
            );
        };
        match (method, &parts[1..]) {
            ("GET", []) => (200, Some(issue(key, &updated))),
            ("PUT", []) if key == BUSY => (
                409,
                Some(json!({"errorMessages": ["The issue was updated by another request."]})),
            ),
            ("PUT", []) => {
                self.issues.insert(key.to_owned(), EDITED.to_owned());
                (204, None)
            }
            ("DELETE", []) => {
                if key != STUCK {
                    self.issues.remove(key);
                }
                (204, None)
            }
            // `Comment`, its body the document as sent.
            ("POST", ["comment"]) => (
                201,
                Some(json!({"id": "10100",
                            "self": format!("https://tracker.example.test/rest/api/3/issue/{key}/comment/10100"),
                            "author": {"accountId": "fixture-account-id"},
                            "body": input["body"],
                            "created": EDITED, "updated": EDITED})),
            ),
            _ => missing(),
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
                    // `204` carries no content at all, as Jira sends it; the
                    // `201` of a link says its empty content's length.
                    let answer = answer
                        .map(|answer| serde_json::to_vec(&answer).unwrap())
                        .unwrap_or_default();
                    let head = if status == 204 {
                        format!("HTTP/1.1 {status} fixture\r\nConnection: close\r\n\r\n")
                    } else if answer.is_empty() {
                        format!(
                            "HTTP/1.1 {status} fixture\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
                        )
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
            credential_revision: None,
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
    /// The requests from `before` on.
    fn since(&self, before: usize) -> Vec<(String, String, Value)> {
        self.requests()[before..].to_vec()
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
fn get(target: &str) -> (String, String, Value) {
    ("GET".to_owned(), format!("{target}?"), Value::Null)
}
fn sent(method: &str, target: &str, body: Value) -> (String, String, Value) {
    (method.to_owned(), format!("{target}?"), body)
}
fn issue_target(key: &str) -> String {
    format!("/rest/api/3/issue/{key}")
}

fn create_input() -> Value {
    json!({"body": {"fields": {
        "project": {"key": PROJECT},
        "issuetype": {"id": "10001"},
        "summary": "Fixture summary",
        "description": document("fixture description"),
        "labels": ["fixture", "parity"],
        "priority": {"id": "3"},
        "assignee": {"accountId": "fixture-user-1"},
        "parent": {"key": ISSUE}}}})
}
fn comment_input(key: &str) -> Value {
    json!({"issueIdOrKey": key,
           "body": {"body": {"content": document("fixture comment")["content"]},
                    "visibility": {"type": "role", "value": "Administrators"}}})
}
fn edit_input(key: &str, stamp: &str) -> Value {
    json!({"issueIdOrKey": key, "updated_stamp": stamp,
           "body": {"fields": {"summary": "Edited summary",
                               "description": document("edited description")}}})
}
fn link_input(inward: &str, outward: &str) -> Value {
    json!({"body": {"type": {"name": "Blocks"},
                    "inwardIssue": {"key": inward}, "outwardIssue": {"key": outward}}})
}
fn delete_input(key: &str, stamp: &str) -> Value {
    json!({"issueIdOrKey": key, "updated_stamp": stamp})
}

/// The shipped entry of `id`, as the selection file states it.
fn entry(id: &str) -> Value {
    selection_file()["operations"]
        .as_array()
        .unwrap()
        .iter()
        .find(|entry| entry["id"] == id)
        .unwrap_or_else(|| panic!("operations.json lacks `{id}`"))
        .clone()
}

/// The five issue writes are shipped beside the transition, each from its pinned
/// operation, with the closed body, guard and conflict answer reviewed here member
/// by member; only the description is left to the guide.
#[test]
fn the_issue_writes_are_shipped_as_reviewed() {
    let engine = engine();
    let mut writes: Vec<String> = engine
        .declarations(&[Effect::Write])
        .into_iter()
        .map(|o| o.id)
        .collect();
    writes.sort();
    assert_eq!(
        writes,
        [COMMENT, CREATE, DELETE, EDIT, LINK, "issue.transition.run"]
    );
    for (id, expected) in [
        (
            CREATE,
            json!({"id": CREATE, "operation_id": "createIssue", "effect": "write",
                   "body_keys": ["/fields/project/key", "/fields/issuetype/id", "/fields/summary",
                                 "/fields/description", "/fields/labels", "/fields/priority/id",
                                 "/fields/assignee/accountId", "/fields/parent/key"],
                   "body_types": {"/fields/project/key": "string", "/fields/issuetype/id": "string",
                                  "/fields/summary": "string", "/fields/labels": "string_list",
                                  "/fields/priority/id": "string",
                                  "/fields/assignee/accountId": "string",
                                  "/fields/parent/key": "string"},
                   "body_required": ["/fields/issuetype/id", "/fields/summary"],
                   "body_nonempty": ["/fields/project/key", "/fields/issuetype/id",
                                     "/fields/summary", "/fields/parent/key"],
                   "guard": {"preflight": {"operation_id": "getProject",
                                           "values": {"projectIdOrKey": "body.fields.project.key"},
                                           "checks": [{"pointer": "/key",
                                                       "expect": {"input": "body.fields.project.key"}}]},
                             "postflight": {"checks": [{"pointer": "/id", "expect": {"present": true}},
                                                       {"pointer": "/key", "expect": {"present": true}}]}}}),
        ),
        (
            COMMENT,
            json!({"id": COMMENT, "operation_id": "addComment", "effect": "write",
                   "body_keys": ["/body/type", "/body/version", "/body/content",
                                 "/visibility/type", "/visibility/value"],
                   "body_types": {"/visibility/type": "string", "/visibility/value": "string"},
                   "body_required": ["/body/content"],
                   "body_fixed": {"/body/type": "doc", "/body/version": 1},
                   "body_enums": {"/visibility/type": ["group", "role"]},
                   "guard": {"preflight": {"operation_id": "getIssue",
                                           "values": {"issueIdOrKey": "issueIdOrKey"},
                                           "checks": [{"pointer": "/key",
                                                       "expect": {"input": "issueIdOrKey"}}]},
                             "postflight": {"checks": [{"pointer": "/body/type",
                                                        "expect": {"literal": "doc"}},
                                                       {"pointer": "/id", "expect": {"present": true}}]}}}),
        ),
        (
            EDIT,
            json!({"id": EDIT, "operation_id": "editIssue", "effect": "write",
                   "withhold": ["overrideScreenSecurity", "overrideEditableFlag", "returnIssue", "expand"],
                   "body_keys": ["/fields/summary", "/fields/description", "/fields/labels",
                                 "/fields/priority", "/fields/assignee"],
                   "body_types": {"/fields/summary": "string", "/fields/labels": "string_list"},
                   "body_nonempty": ["/fields/summary"],
                   "body_any_of": ["/fields/summary", "/fields/description", "/fields/labels",
                                   "/fields/priority", "/fields/assignee"],
                   "conflict_status": 409,
                   "guard": {"preflight": {"operation_id": "getIssue",
                                           "values": {"issueIdOrKey": "issueIdOrKey"},
                                           "checks": [{"pointer": "/key", "expect": {"input": "issueIdOrKey"}},
                                                      {"pointer": "/fields/updated",
                                                       "expect": {"input": "updated_stamp"}}],
                                           "mismatch": "conflict"},
                             "postflight": {"read": {"operation_id": "getIssue",
                                                     "values": {"issueIdOrKey": "issueIdOrKey"}},
                                            "checks": [{"pointer": "/key",
                                                        "expect": {"input": "issueIdOrKey"}}]}}}),
        ),
        (
            LINK,
            json!({"id": LINK, "operation_id": "linkIssues", "effect": "write",
                   "body_keys": ["/type/name", "/inwardIssue/key", "/outwardIssue/key"],
                   "body_types": {"/type/name": "string", "/inwardIssue/key": "string",
                                  "/outwardIssue/key": "string"},
                   "body_required": ["/type/name"],
                   "body_nonempty": ["/type/name", "/inwardIssue/key", "/outwardIssue/key"],
                   "guard": {"preflight": {"operation_id": "getIssue",
                                           "values": {"issueIdOrKey": "body.inwardIssue.key"},
                                           "checks": [{"pointer": "/key",
                                                       "expect": {"input": "body.inwardIssue.key"}}]},
                             "further_preflights": [
                                 {"operation_id": "getIssue",
                                  "values": {"issueIdOrKey": "body.outwardIssue.key"},
                                  "checks": [{"pointer": "/key",
                                              "expect": {"input": "body.outwardIssue.key"}}]}],
                             "postflight": {"checks": []}}}),
        ),
        (
            DELETE,
            json!({"id": DELETE, "operation_id": "deleteIssue", "effect": "write",
                   "withhold": ["deleteSubtasks"],
                   "guard": {"preflight": {"operation_id": "getIssue",
                                           "values": {"issueIdOrKey": "issueIdOrKey"},
                                           "checks": [{"pointer": "/key", "expect": {"input": "issueIdOrKey"}},
                                                      {"pointer": "/fields/updated",
                                                       "expect": {"input": "updated_stamp"}}],
                                           "mismatch": "conflict"},
                             "postflight": {"checks": [],
                                            "read": {"operation_id": "getIssue",
                                                     "values": {"issueIdOrKey": "issueIdOrKey"}},
                                            "absent": true}}}),
        ),
    ] {
        let mut shipped = entry(id);
        let description = shipped
            .as_object_mut()
            .unwrap()
            .remove("description")
            .unwrap_or_else(|| panic!("`{id}` has no description"));
        assert!(
            description.as_str().is_some_and(|text| text.len() > 80),
            "`{id}`"
        );
        assert_eq!(shipped, expected, "`{id}`");
    }
    // The guard's stamp is an input of its own, required by the engine; the
    // body of a delete is nothing.
    for id in [EDIT, DELETE] {
        let declared = engine
            .declarations(&[Effect::Write])
            .into_iter()
            .find(|o| o.id == id)
            .unwrap();
        let required = declared.input_schema["required"].as_array().unwrap();
        assert!(required.contains(&json!("updated_stamp")), "`{id}`");
        assert!(required.contains(&json!("issueIdOrKey")), "`{id}`");
    }
}

/// Approved, a create reads the project, then sends one `POST` with exactly the
/// fields given, the description an Atlassian Document Format document passed on
/// as sent. Jira's `201` names the new issue, whose `id` and `key` must be
/// present; nothing pins them before a create, so the same input sent twice
/// creates twice.
#[test]
fn an_approved_create_reads_the_project_and_posts_the_fields_as_given() {
    let provider = Provider::new();
    let mut child = Child::spawn(&provider.selection()).unwrap();
    let input = create_input();
    let result = write(&mut child, CREATE, &input).unwrap();
    assert_eq!(result.effect, WriteEffect::Applied, "{:?}", result.result);
    let output = result.result.unwrap();
    assert_eq!(output["status"], 201);
    assert_eq!(output["body"]["key"], CREATED);
    assert_eq!(output["provenance"]["instance"], INSTANCE);
    assert_eq!(
        provider.requests(),
        [
            get("/rest/api/3/project/PROJ"),
            sent("POST", "/rest/api/3/issue", input["body"].clone()),
        ]
    );
    let again = write(&mut child, CREATE, &input).unwrap();
    assert_eq!(again.effect, WriteEffect::Applied);
    assert_eq!(provider.requests().len(), 4);
}

/// Approved, a comment reads the issue and sends one `POST` whose body is the
/// fixed document (`type` doc, `version` 1) around the caller's content, and the
/// visibility as given; Jira's `201` answers the comment with its id.
#[test]
fn an_approved_comment_reads_the_issue_and_posts_the_fixed_document() {
    let provider = Provider::new();
    let mut child = Child::spawn(&provider.selection()).unwrap();
    let result = write(&mut child, COMMENT, &comment_input(ISSUE)).unwrap();
    assert_eq!(result.effect, WriteEffect::Applied, "{:?}", result.result);
    let output = result.result.unwrap();
    assert_eq!(output["status"], 201);
    assert_eq!(output["body"]["id"], "10100");
    assert_eq!(
        provider.requests(),
        [
            get(&issue_target(ISSUE)),
            sent(
                "POST",
                &format!("{}/comment", issue_target(ISSUE)),
                json!({"body": document("fixture comment"),
                       "visibility": {"type": "role", "value": "Administrators"}})
            ),
        ]
    );
}

/// Approved, an edit reads the issue, whose key and `updated` stamp must be the
/// caller's, sends one `PUT` with the fields given, and reads the issue again:
/// Jira's `204` carries nothing.
#[test]
fn an_approved_edit_reads_the_stamp_puts_once_and_reads_the_issue_after() {
    let provider = Provider::new();
    let mut child = Child::spawn(&provider.selection()).unwrap();
    let input = edit_input(ISSUE, STAMP);
    let result = write(&mut child, EDIT, &input).unwrap();
    assert_eq!(result.effect, WriteEffect::Applied, "{:?}", result.result);
    let output = result.result.unwrap();
    assert_eq!(output["status"], 204);
    assert_eq!(output["body"], Value::Null);
    assert_eq!(
        provider.requests(),
        [
            get(&issue_target(ISSUE)),
            sent("PUT", &issue_target(ISSUE), input["body"].clone()),
            get(&issue_target(ISSUE)),
        ]
    );
}

/// Approved, a link reads both issues and sends one `POST`; Jira's `201` has no
/// body and the link is not read after it.
#[test]
fn an_approved_link_reads_both_issues_and_posts_once() {
    let provider = Provider::new();
    let mut child = Child::spawn(&provider.selection()).unwrap();
    let input = link_input(ISSUE, OTHER);
    let result = write(&mut child, LINK, &input).unwrap();
    assert_eq!(result.effect, WriteEffect::Applied, "{:?}", result.result);
    let output = result.result.unwrap();
    assert_eq!(output["status"], 201);
    assert_eq!(output["body"], Value::Null);
    assert_eq!(
        provider.requests(),
        [
            get(&issue_target(ISSUE)),
            get(&issue_target(OTHER)),
            sent("POST", "/rest/api/3/issueLink", input["body"].clone()),
        ]
    );
}

/// Approved, a delete reads the issue, sends one `DELETE` without a body, and
/// only a `404` to the read after it proves the issue gone.
#[test]
fn an_approved_delete_is_proved_by_a_404_after_it() {
    let provider = Provider::new();
    let mut child = Child::spawn(&provider.selection()).unwrap();
    let result = write(&mut child, DELETE, &delete_input(ISSUE, STAMP)).unwrap();
    assert_eq!(result.effect, WriteEffect::Applied, "{:?}", result.result);
    assert_eq!(result.result.unwrap()["status"], 204);
    assert_eq!(
        provider.requests(),
        [
            get(&issue_target(ISSUE)),
            sent("DELETE", &issue_target(ISSUE), Value::Null),
            get(&issue_target(ISSUE)),
        ]
    );
}

/// An issue still found after Jira's `204` to its delete leaves the effect
/// unknown, never applied and never sent again.
#[test]
fn a_delete_whose_issue_is_still_found_after_it_is_unknown() {
    let provider = Provider::new();
    let mut child = Child::spawn(&provider.selection()).unwrap();
    let result = write(&mut child, DELETE, &delete_input(STUCK, STAMP)).unwrap();
    assert_eq!(result.effect, WriteEffect::Unknown);
    assert_eq!(result.result, Err(Failure::Protocol));
    assert_eq!(
        provider.requests(),
        [
            get(&issue_target(STUCK)),
            sent("DELETE", &issue_target(STUCK), Value::Null),
            get(&issue_target(STUCK)),
        ]
    );
}

/// An `updated` stamp other than the issue's refuses an edit and a delete as
/// conflict after the first read, with no write sent.
#[test]
fn a_stale_stamp_refuses_edit_and_delete_as_conflict_with_nothing_written() {
    let provider = Provider::new();
    let mut child = Child::spawn(&provider.selection()).unwrap();
    for (id, input) in [
        (EDIT, edit_input(ISSUE, STALE)),
        (DELETE, delete_input(ISSUE, STALE)),
    ] {
        let before = provider.requests().len();
        let outcome = write(&mut child, id, &input).map(|result| result.effect);
        assert_eq!(outcome, Err(Failure::ProviderConflict), "`{id}`");
        assert_eq!(
            provider.since(before),
            [get(&issue_target(ISSUE))],
            "`{id}`"
        );
    }
}

/// Jira's own `409` to an edit is conflict: refused, nothing applied, and the
/// `PUT` is not sent again.
#[test]
fn jira_409_to_an_edit_is_conflict_and_never_retried() {
    let provider = Provider::new();
    let mut child = Child::spawn(&provider.selection()).unwrap();
    let input = edit_input(BUSY, STAMP);
    let result = write(&mut child, EDIT, &input).unwrap();
    assert_eq!(result.effect, WriteEffect::Refused);
    assert_eq!(result.result, Err(Failure::ProviderConflict));
    assert_eq!(
        provider.requests(),
        [
            get(&issue_target(BUSY)),
            sent("PUT", &issue_target(BUSY), input["body"].clone()),
        ]
    );
}

/// A project or issue Jira does not find, or one whose key is not the one named,
/// is refused after the read that found it so, with no write sent.
#[test]
fn a_target_jira_does_not_find_is_refused_before_the_write() {
    let provider = Provider::new();
    let mut child = Child::spawn(&provider.selection()).unwrap();
    let mut no_project = create_input();
    no_project["body"]["fields"]["project"]["key"] = json!("NOPE");
    for (id, input, reads) in [
        (CREATE, no_project, vec![get("/rest/api/3/project/NOPE")]),
        (
            COMMENT,
            comment_input(ABSENT),
            vec![get(&issue_target(ABSENT))],
        ),
        (
            EDIT,
            edit_input(ABSENT, STAMP),
            vec![get(&issue_target(ABSENT))],
        ),
        (
            LINK,
            link_input(ISSUE, ABSENT),
            vec![get(&issue_target(ISSUE)), get(&issue_target(ABSENT))],
        ),
        (
            DELETE,
            delete_input(ABSENT, STAMP),
            vec![get(&issue_target(ABSENT))],
        ),
    ] {
        let before = provider.requests().len();
        let outcome = write(&mut child, id, &input).map(|result| result.effect);
        assert_eq!(outcome, Err(Failure::Forbidden), "`{id}` {input}");
        assert_eq!(provider.since(before), reads, "`{id}` {input}");
    }
    assert!(
        provider
            .requests()
            .iter()
            .all(|(method, _, _)| method == "GET"),
        "a write was sent past a refusing guard"
    );
}

/// What the closed bodies and the guards' inputs refuse as `invalid_input`
/// before any request: a missing required key or guard input, an empty string
/// where one is held non-empty, a key the body does not admit, a fixed key the
/// caller sends, a value outside an enum or of the wrong type, an edit naming
/// none of its fields, and a withheld parameter.
#[test]
fn malformed_inputs_are_refused_before_any_request() {
    let provider = Provider::new();
    let mut child = Child::spawn(&provider.selection()).unwrap();
    let mut cases: Vec<(&str, Value)> = Vec::new();
    let create = |edit: &dyn Fn(&mut Value)| {
        let mut input = create_input();
        edit(&mut input["body"]["fields"]);
        input
    };
    for change in [
        &(|f: &mut Value| {
            f.as_object_mut().unwrap().remove("summary");
        }) as &dyn Fn(&mut Value),
        &|f: &mut Value| f["summary"] = json!(""),
        &|f: &mut Value| {
            f.as_object_mut().unwrap().remove("issuetype");
        },
        &|f: &mut Value| {
            f.as_object_mut().unwrap().remove("project");
        },
        &|f: &mut Value| f["project"]["key"] = json!(""),
        &|f: &mut Value| f["labels"] = json!("fixture"),
        &|f: &mut Value| f["reporter"] = json!({"accountId": "fixture-user-2"}),
    ] {
        cases.push((CREATE, create(change)));
    }
    let comment = |edit: &dyn Fn(&mut Value)| {
        let mut input = comment_input(ISSUE);
        edit(&mut input["body"]);
        input
    };
    for change in [
        &(|b: &mut Value| {
            b["body"].as_object_mut().unwrap().remove("content");
        }) as &dyn Fn(&mut Value),
        &|b: &mut Value| b["body"]["type"] = json!("doc"),
        &|b: &mut Value| b["visibility"]["type"] = json!("user"),
        &|b: &mut Value| b["properties"] = json!([]),
    ] {
        cases.push((COMMENT, comment(change)));
    }
    let mut no_issue = comment_input(ISSUE);
    no_issue.as_object_mut().unwrap().remove("issueIdOrKey");
    cases.push((COMMENT, no_issue));
    let mut no_stamp = edit_input(ISSUE, STAMP);
    no_stamp.as_object_mut().unwrap().remove("updated_stamp");
    cases.push((EDIT, no_stamp));
    let mut no_fields = edit_input(ISSUE, STAMP);
    no_fields["body"] = json!({"fields": {}});
    cases.push((EDIT, no_fields));
    let mut empty_summary = edit_input(ISSUE, STAMP);
    empty_summary["body"]["fields"]["summary"] = json!("");
    cases.push((EDIT, empty_summary));
    let mut overriding = edit_input(ISSUE, STAMP);
    overriding["overrideScreenSecurity"] = json!(true);
    cases.push((EDIT, overriding));
    let mut no_type = link_input(ISSUE, OTHER);
    no_type["body"].as_object_mut().unwrap().remove("type");
    cases.push((LINK, no_type));
    cases.push((LINK, link_input("", OTHER)));
    let mut no_outward = link_input(ISSUE, OTHER);
    no_outward["body"]
        .as_object_mut()
        .unwrap()
        .remove("outwardIssue");
    cases.push((LINK, no_outward));
    cases.push((DELETE, json!({"issueIdOrKey": ISSUE})));
    let mut subtasks = delete_input(ISSUE, STAMP);
    subtasks["deleteSubtasks"] = json!("true");
    cases.push((DELETE, subtasks));
    for (id, input) in cases {
        let outcome = write(&mut child, id, &input).map(|result| result.effect);
        assert_eq!(outcome, Err(Failure::InvalidInput), "`{id}` {input}");
    }
    assert!(provider.requests().is_empty(), "{:?}", provider.requests());
}
