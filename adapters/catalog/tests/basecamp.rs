//! Basecamp projects, message boards, messages, comments and to-dos through the
//! catalog provider (story:catalog-basecamp, batch 1: reads only).
//!
//! The shipped selection is the lowering of the definition
//! `crates/connectors-catalog/tests/fixtures/definitions/connectors/basecamp.yaml`
//! over Basecamp's pinned OpenAPI document: its `base_url`
//! `https://3.basecampapi.com/{accountId}` ends in the configured account, so
//! the bundle's paths start after `/{accountId}` and no read takes `accountId`
//! from the caller; the instance endpoint's path names the account. Each read
//! runs through the provider child against a disposable HTTPS stand-in for
//! Basecamp: the account path, the bearer token and the `User-Agent` of each
//! request, a list walked by the `Link` header's `rel="next"` over three pages
//! to the last, `429` with `Retry-After` refused as rate limited with that
//! delay, and `404` as not found. No live credential and no network.
#![cfg(unix)]
use connectors_catalog::bundle;
use connectors_catalog_provider::{Effect, Engine, Selection};
use connectors_host::local::{
    config::{Adapter, Executable, Restart, Startup},
    filesystem,
    runtime::{Bootstrap, Child, Failure, Refusal},
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

/// The fictional account the instance endpoint names.
const ACCOUNT: &str = "12345";
/// A fictional personal access token, split so no scanner reads it as one.
const TOKEN: &str = concat!("fixture-basecamp-", "personal-access-token");
const HEADER: &str = concat!("Bearer fixture-basecamp-", "personal-access-token");
const PROFILE: &str = "basecamp.pat";
const UPSTREAM: &str = "../basecamp/upstream";
const SOURCE: &str = "openapi.json";
const SOURCE_SHA256: &str = "6f309fd546710c53747ff546e0459083fd565902df07932aed49d9ba1a2603bb";

/// The shipped ids, their pinned `operationId`, the path the bundle records
/// (the document's, after `/{accountId}`) and whether the read takes the
/// `Link` paging style. A renamed, dropped or added id fails here.
const SHIPPED: [(&str, &str, &str, bool); 13] = [
    ("comment.get", "GetComment", "/comments/{commentId}", false),
    (
        "comments.list",
        "ListComments",
        "/recordings/{recordingId}/comments.json",
        true,
    ),
    ("message.get", "GetMessage", "/messages/{messageId}", false),
    (
        "message_board.get",
        "GetMessageBoard",
        "/message_boards/{boardId}",
        false,
    ),
    (
        "messages.list",
        "ListMessages",
        "/message_boards/{boardId}/messages.json",
        true,
    ),
    ("project.get", "GetProject", "/projects/{projectId}", false),
    (
        "project.people.list",
        "ListProjectPeople",
        "/projects/{projectId}/people.json",
        true,
    ),
    ("projects.list", "ListProjects", "/projects.json", true),
    ("todo.get", "GetTodo", "/todos/{todoId}", false),
    (
        "todolist.get",
        "GetTodolistOrGroup",
        "/todolists/{id}",
        false,
    ),
    (
        "todolists.list",
        "ListTodolists",
        "/todosets/{todosetId}/todolists.json",
        true,
    ),
    (
        "todos.list",
        "ListTodos",
        "/todolists/{todolistId}/todos.json",
        true,
    ),
    ("todoset.get", "GetTodoset", "/todosets/{todosetId}", false),
];

fn root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
}
fn root_path(relative: &str) -> PathBuf {
    root().join(relative).canonicalize().unwrap()
}
fn selection_file() -> Value {
    serde_json::from_slice(&fs::read(root().join("providers/basecamp/operations.json")).unwrap())
        .unwrap()
}
fn shipped() -> Vec<Selection> {
    let file = selection_file();
    assert_eq!(file["format"], "connectors-catalog-operations/1");
    assert_eq!(file["provider"], "basecamp");
    serde_json::from_value(file["operations"].clone()).unwrap()
}
fn declaration() -> Value {
    serde_json::from_slice(&fs::read(root().join("providers/basecamp/connector.json")).unwrap())
        .unwrap()
}
fn engine() -> Engine {
    let bundle = bundle::load(&root().join("generated/bundles"), "basecamp").unwrap();
    Engine::new(&bundle, "/", &shipped()).unwrap()
}

#[test]
fn shipped_basecamp_selections_are_exactly_the_thirteen_reads() {
    let selections = shipped();
    let bundle = bundle::load(&root().join("generated/bundles"), "basecamp").unwrap();
    assert_eq!(bundle.auth_profile, PROFILE);
    let engine = Engine::new(&bundle, "/", &selections).unwrap();
    let mut declared: Vec<String> = engine
        .declarations(&[Effect::Read, Effect::Write])
        .into_iter()
        .map(|o| o.id)
        .collect();
    declared.sort();
    let expected: Vec<&str> = SHIPPED.iter().map(|(id, _, _, _)| *id).collect();
    assert_eq!(declared, expected);
    assert!(engine.declarations(&[Effect::Write]).is_empty());
    let raw = selection_file();
    for (id, operation_id, path, paged) in SHIPPED {
        let selection = selections.iter().find(|s| s.id == id).unwrap();
        assert_eq!(selection.operation_id, operation_id, "`{id}`");
        assert_eq!(selection.effect, Effect::Read, "`{id}`");
        assert_eq!(engine.effect(id), Some(Effect::Read), "`{id}`");
        let operation = bundle
            .inventory
            .operations
            .iter()
            .find(|o| o.operation_id.as_deref() == Some(operation_id))
            .unwrap_or_else(|| panic!("the bundle lacks `{operation_id}`"));
        assert_eq!(operation.method, "get", "`{id}`");
        assert_eq!(operation.path, path, "`{id}`");
        let written = raw["operations"]
            .as_array()
            .unwrap()
            .iter()
            .find(|o| o["id"] == id)
            .unwrap();
        assert_eq!(
            written.get("link_next"),
            paged.then_some(&json!(true)),
            "`{id}` link_next"
        );
        assert!(
            written["description"]
                .as_str()
                .is_some_and(|d| !d.is_empty() && !d.contains('\n')),
            "`{id}` has a one-line description"
        );
    }
}

/// The account is the instance's: no bundle path names `{accountId}` and no
/// read's input takes it.
#[test]
fn no_read_takes_the_account_id() {
    let bundle = bundle::load(&root().join("generated/bundles"), "basecamp").unwrap();
    for operation in &bundle.inventory.operations {
        assert!(
            !operation.path.contains("{accountId}"),
            "{}",
            operation.path
        );
        assert!(
            operation.parameters.iter().all(|p| p.name != "accountId"),
            "{}",
            operation.path
        );
    }
    for operation in engine().declarations(&[Effect::Read]) {
        assert!(
            operation.input_schema["properties"]
                .get("accountId")
                .is_none(),
            "`{}` takes accountId",
            operation.id
        );
    }
}

#[test]
fn the_pinned_source_is_the_one_the_bundle_and_the_definition_name() {
    let upstream = root().join(UPSTREAM);
    let bytes = fs::read(upstream.join(SOURCE)).unwrap();
    let digest = hex::encode(Sha256::digest(&bytes));
    assert_eq!(digest, SOURCE_SHA256);
    let index = bundle::read_index(&root().join("generated/bundles")).unwrap();
    let entry = index.find("basecamp").unwrap();
    assert_eq!(entry.source_sha256, digest);
    assert_eq!(entry.operations, 300);
    let bundle = bundle::load(&root().join("generated/bundles"), "basecamp").unwrap();
    assert_eq!(bundle.source.source_bytes, bytes.len());
    assert_eq!(bundle.source.file_name, SOURCE);
    assert!(bundle.source.amendments.is_none());
    let readme = fs::read_to_string(upstream.join("README.md")).unwrap();
    assert!(readme.contains(&digest), "README does not name the digest");
    let document: Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(document["openapi"], "3.1.0");
    assert_eq!(document["info"]["license"]["name"], "MIT");
    assert_eq!(document["servers"], Value::Null);
    let license = fs::read_to_string(upstream.join("LICENSE")).unwrap();
    assert!(license.starts_with("MIT License"), "{license}");
    let definition = fs::read_to_string(root().join(
        "../../crates/connectors-catalog/tests/fixtures/definitions/connectors/basecamp.yaml",
    ))
    .unwrap();
    assert!(definition.contains(&format!("sha256: {digest}")));
}

/// The connector declares one bearer profile, whose identity is the token's
/// person under the account, and fills the account from the endpoint's path.
#[test]
fn the_declared_profile_is_a_bearer_token_with_the_person_as_identity() {
    let declaration = declaration();
    assert_eq!(declaration["connector_id"], "basecamp");
    assert_eq!(declaration["catalog_provider"], "basecamp");
    assert_eq!(declaration["endpoint_path"], "/{accountId}");
    assert_eq!(declaration["default_endpoint"], Value::Null);
    let profiles = declaration["profiles"].as_array().unwrap();
    assert_eq!(profiles.len(), 1);
    let auth = &profiles[0];
    assert_eq!(auth["profile"], PROFILE);
    assert_eq!(auth["header"], "Authorization");
    assert_eq!(auth["bearer"], true);
    assert_eq!(auth["identity"]["path"], "my/profile.json");
    assert_eq!(auth["identity"]["kind"], "basecamp.person");
    assert_eq!(auth["identity"]["subject_pointer"], "/id");
    assert_eq!(auth["scopes"], Value::Null);
    assert_eq!(auth["oauth2"], Value::Null);
}

/// One request the stand-in answered: target, `Authorization` and `User-Agent`.
#[derive(Clone, Debug, PartialEq)]
struct Seen {
    target: String,
    authorization: Option<String>,
    user_agent: Option<String>,
}
type Requests = Arc<Mutex<Vec<Seen>>>;

/// The `User-Agent` every provider request carries.
fn user_agent() -> String {
    format!(
        "connectors/{} (https://github.com/beyond10x/connectors)",
        env!("CARGO_PKG_VERSION")
    )
}

/// The answer to `target` (path and query): status, extra headers, body.
/// Anything under another account, or not listed, is `404`.
fn answer(target: &str, origin: &str) -> (u16, Vec<(String, String)>, Value) {
    let (route, query) = target.split_once('?').unwrap_or((target, ""));
    let Some(route) = route.strip_prefix(&format!("/{ACCOUNT}")) else {
        return (404, vec![], json!({"error": "Not found"}));
    };
    let link = |page: u32| {
        vec![(
            "Link".to_owned(),
            format!("<{origin}/{ACCOUNT}{route}?page={page}>; rel=\"next\""),
        )]
    };
    let ok = |body: Value| (200, vec![], body);
    match (route, query) {
        ("/my/profile.json", "") => ok(json!({"id": 1049715913, "name": "Fixture Person",
            "email_address": "person@example.test", "personable_type": "User"})),
        // Three pages by `Link`, the last without one.
        ("/projects.json", "") => (200, link(2), json!([project(1), project(2)])),
        ("/projects.json", "page=2") => (200, link(3), json!([project(3), project(4)])),
        ("/projects.json", "page=3") => ok(json!([project(5)])),
        // Rate limited, with the wait in `Retry-After`.
        ("/projects.json", "status=trashed") => (
            429,
            vec![("Retry-After".to_owned(), "120".to_owned())],
            json!({"error": "Too many requests"}),
        ),
        // A next page under another account is not the instance's.
        ("/projects.json", "status=archived") => (
            200,
            vec![(
                "Link".to_owned(),
                format!("<{origin}/99999/projects.json?status=archived&page=2>; rel=\"next\""),
            )],
            json!([project(9)]),
        ),
        ("/projects/2085958499", "") => ok(project(2085958499)),
        ("/projects/2085958499/people.json", "") => {
            ok(json!([{"id": 1049715913, "name": "Fixture Person"}]))
        }
        ("/message_boards/1069479338", "") => {
            ok(json!({"id": 1069479338, "type": "Message::Board", "messages_count": 1}))
        }
        ("/message_boards/1069479338/messages.json", "sort=updated_at&direction=desc") => {
            ok(json!([message()]))
        }
        ("/messages/1069479351", "") => ok(message()),
        ("/recordings/1069479351/comments.json", "") => ok(json!([comment()])),
        ("/comments/1069479361", "") => ok(comment()),
        ("/todosets/1069479339", "") => {
            ok(json!({"id": 1069479339, "type": "Todoset", "todolists_count": 1}))
        }
        ("/todosets/1069479339/todolists.json", "status=archived") => {
            ok(json!([{"id": 1069479520, "type": "Todolist", "name": "Launch"}]))
        }
        ("/todolists/1069479520", "") => {
            ok(json!({"id": 1069479520, "type": "Todolist", "name": "Launch"}))
        }
        ("/todolists/1069479520/todos.json", "completed=true") => ok(json!([todo()])),
        ("/todos/1069479521", "") => ok(todo()),
        _ => (404, vec![], json!({"error": "Not found"})),
    }
}
fn project(id: u64) -> Value {
    json!({"id": id, "status": "active", "name": format!("Project {id}"),
           "dock": [{"id": 1069479338, "name": "message_board", "title": "Message Board",
                     "enabled": true, "position": 1}]})
}
fn message() -> Value {
    json!({"id": 1069479351, "type": "Message", "subject": "Kickoff",
           "content": "<div>Welcome</div>"})
}
fn comment() -> Value {
    json!({"id": 1069479361, "type": "Comment", "content": "<div>Thanks</div>",
           "parent": {"id": 1069479351, "type": "Message"}})
}
fn todo() -> Value {
    json!({"id": 1069479521, "type": "Todo", "content": "Ship it", "completed": true,
           "due_on": "2026-11-20", "assignees": []})
}

/// A disposable HTTPS stand-in for `3.basecampapi.com` on loopback. It
/// answers `400` to a request whose `User-Agent` names no contact, as
/// Basecamp does, and `401` to any credential but the fictional one.
struct Basecamp {
    stop: Option<oneshot::Sender<()>>,
    thread: Option<std::thread::JoinHandle<()>>,
    _root: tempfile::TempDir,
    config: PathBuf,
    origin: String,
    requests: Requests,
}
impl Basecamp {
    fn new() -> Self {
        let root = tempfile::tempdir().unwrap();
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
                let port = listener.local_addr().unwrap().port();
                address_tx.send(port).unwrap();
                let origin = format!("https://localhost:{port}");
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
                    let seen = Seen {
                        target: head.split_whitespace().nth(1).unwrap_or_default().to_owned(),
                        authorization: header("authorization"),
                        user_agent: header("user-agent"),
                    };
                    // Raw headers are compared, never printed.
                    let (status, headers, body) = if !seen
                        .user_agent
                        .as_deref()
                        .is_some_and(|agent| agent.contains("(https://"))
                    {
                        (400, vec![], json!({"error": "User-Agent names no contact"}))
                    } else if seen.authorization.as_deref() != Some(HEADER) {
                        (401, vec![], json!({"error": "fixture refusal"}))
                    } else {
                        answer(&seen.target, &origin)
                    };
                    observed.lock().unwrap().push(seen);
                    let body = serde_json::to_vec(&body).unwrap();
                    let extra: String = headers
                        .iter()
                        .map(|(name, value)| format!("{name}: {value}\r\n"))
                        .collect();
                    let head = format!(
                        "HTTP/1.1 {status} fixture\r\nContent-Type: application/json; charset=utf-8\r\n{extra}Content-Length: {}\r\nConnection: close\r\n\r\n",
                        body.len()
                    );
                    let _ = stream.write_all(head.as_bytes()).await;
                    let _ = stream.write_all(&body).await;
                }
            });
        });
        let port = address_rx.recv_timeout(Duration::from_secs(5)).unwrap();
        let origin = format!("https://localhost:{port}");
        let config = directory.join("catalog.json");
        private(
            &config,
            &serde_json::to_vec(&json!({
                "format": "connectors-catalog-local/2",
                "instance": "basecamp.work",
                "provider": "basecamp",
                "bundle_directory": root_path("generated/bundles"),
                "api_base": format!("{origin}/{ACCOUNT}"),
                "request_prefix": format!("/{ACCOUNT}"),
                "ca_file": ca,
                "auth": declaration()["profiles"][0],
                "operations_file": root_path("providers/basecamp/operations.json"),
            }))
            .unwrap(),
        );
        Self {
            stop: Some(stop),
            thread: Some(thread),
            _root: root,
            config,
            origin,
            requests,
        }
    }
    fn bootstrap(&self) -> Bootstrap {
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
        bootstrap
    }
    fn selection(&self) -> Adapter {
        let bootstrap = self.bootstrap();
        let binary = PathBuf::from(env!("CARGO_BIN_EXE_connectors-catalog-provider"))
            .canonicalize()
            .unwrap();
        Adapter {
            credential_revision: None,
            private_protocol: None,
            permissions: Default::default(),
            instance_id: "basecamp.work".into(),
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
    fn requests(&self) -> Vec<Seen> {
        self.requests.lock().unwrap().clone()
    }
    fn targets(&self) -> Vec<String> {
        self.requests()
            .into_iter()
            .map(|seen| seen.target)
            .collect()
    }
}
impl Drop for Basecamp {
    fn drop(&mut self) {
        let _ = self.stop.take().unwrap().send(());
        self.thread.take().unwrap().join().unwrap();
    }
}
fn private(path: &Path, bytes: &[u8]) {
    fs::write(path, bytes).unwrap();
    fs::set_permissions(path, fs::Permissions::from_mode(0o600)).unwrap();
}
fn secret() -> Secret {
    Secret(serde_json::to_vec(&json!({"token": TOKEN})).unwrap())
}
fn deadline() -> u64 {
    connectors_sdk::now_ms() + 30_000
}
fn explained(child: &mut Child, operation: &str, input: &Value) -> Result<Value, Refusal> {
    let revision = child.bootstrap().descriptor().unwrap().revision;
    child
        .invoke_explained(
            operation,
            &revision,
            "one",
            &secret(),
            &serde_json::to_vec(input).unwrap(),
            deadline(),
        )
        .map(|output| serde_json::from_slice(&output).unwrap())
}
fn invoke(child: &mut Child, operation: &str, input: Value) -> Value {
    explained(child, operation, &input)
        .unwrap_or_else(|refusal| panic!("`{operation}` failed: {:?}", refusal.failure))
}

#[test]
fn connecting_records_the_person_under_the_account_as_the_identity() {
    let basecamp = Basecamp::new();
    let mut child = Child::spawn(&basecamp.selection()).unwrap();
    let baseline = child.validate(PROFILE, &secret(), deadline()).unwrap();
    assert_eq!(baseline.identity.kind, "basecamp.person");
    assert_eq!(baseline.identity.subject, "1049715913");
    assert_eq!(baseline.granted_scopes, None);
    assert_eq!(
        basecamp.requests(),
        [Seen {
            target: format!("/{ACCOUNT}/my/profile.json"),
            authorization: Some(HEADER.to_owned()),
            user_agent: Some(user_agent()),
        }]
    );
}

/// Each read's input and the exact request the stand-in must observe.
fn first_requests() -> [(&'static str, Value, &'static str); 13] {
    [
        ("projects.list", json!({}), "/12345/projects.json"),
        (
            "project.get",
            json!({"projectId": 2085958499u64}),
            "/12345/projects/2085958499",
        ),
        (
            "project.people.list",
            json!({"projectId": 2085958499u64}),
            "/12345/projects/2085958499/people.json",
        ),
        (
            "message_board.get",
            json!({"boardId": 1069479338u64}),
            "/12345/message_boards/1069479338",
        ),
        (
            "messages.list",
            json!({"boardId": 1069479338u64, "sort": "updated_at", "direction": "desc"}),
            "/12345/message_boards/1069479338/messages.json?sort=updated_at&direction=desc",
        ),
        (
            "message.get",
            json!({"messageId": 1069479351u64}),
            "/12345/messages/1069479351",
        ),
        (
            "comments.list",
            json!({"recordingId": 1069479351u64}),
            "/12345/recordings/1069479351/comments.json",
        ),
        (
            "comment.get",
            json!({"commentId": 1069479361u64}),
            "/12345/comments/1069479361",
        ),
        (
            "todoset.get",
            json!({"todosetId": 1069479339u64}),
            "/12345/todosets/1069479339",
        ),
        (
            "todolists.list",
            json!({"todosetId": 1069479339u64, "status": "archived"}),
            "/12345/todosets/1069479339/todolists.json?status=archived",
        ),
        (
            "todolist.get",
            json!({"id": 1069479520u64}),
            "/12345/todolists/1069479520",
        ),
        (
            "todos.list",
            json!({"todolistId": 1069479520u64, "completed": true}),
            "/12345/todolists/1069479520/todos.json?completed=true",
        ),
        (
            "todo.get",
            json!({"todoId": 1069479521u64}),
            "/12345/todos/1069479521",
        ),
    ]
}

#[test]
fn each_read_is_sent_under_the_account_with_the_token_and_the_user_agent() {
    let basecamp = Basecamp::new();
    let mut child = Child::spawn(&basecamp.selection()).unwrap();
    let ids: Vec<&str> = first_requests().iter().map(|(id, _, _)| *id).collect();
    let mut sorted = ids.clone();
    sorted.sort();
    assert_eq!(
        sorted,
        SHIPPED.iter().map(|(id, _, _, _)| *id).collect::<Vec<_>>()
    );
    for (operation, input, expected) in first_requests() {
        let before = basecamp.requests().len();
        let result = explained(&mut child, operation, &input).unwrap_or_else(|refusal| {
            panic!(
                "`{operation}` failed: {:?} after {:?}",
                refusal.failure,
                &basecamp.targets()[before..]
            )
        });
        let requests = basecamp.requests();
        assert_eq!(requests.len(), before + 1, "`{operation}` requests");
        assert_eq!(
            requests[before],
            Seen {
                target: expected.to_owned(),
                authorization: Some(HEADER.to_owned()),
                user_agent: Some(user_agent()),
            },
            "`{operation}` request"
        );
        assert_eq!(result["status"], 200, "`{operation}` status");
        assert_eq!(
            result["body"],
            answer(expected, &basecamp.origin).2,
            "`{operation}` body"
        );
        assert_eq!(result["provenance"]["instance"], "basecamp.work");
    }
}

/// `projects.list` is walked by `Link`: each page's `rel="next"` is answered
/// as `next`, the caller sends its `page`, and the third page, which names
/// none, is the last.
#[test]
fn a_list_walks_three_pages_by_the_link_header_to_the_last() {
    let basecamp = Basecamp::new();
    let mut child = Child::spawn(&basecamp.selection()).unwrap();
    let mut input = json!({});
    let mut ids = Vec::new();
    let mut nexts = Vec::new();
    let mut pages = 0;
    loop {
        let answer = invoke(&mut child, "projects.list", input.clone());
        pages += 1;
        for project in answer["body"].as_array().unwrap() {
            ids.push(project["id"].as_u64().unwrap());
        }
        let Some(next) = answer.get("next").and_then(Value::as_str) else {
            break;
        };
        nexts.push(next.to_owned());
        let page: u32 = next.split_once("page=").unwrap().1.parse().unwrap();
        input = json!({"page": page});
        assert!(pages < 4, "the walk did not stop");
    }
    assert_eq!(pages, 3);
    assert_eq!(ids, [1, 2, 3, 4, 5]);
    let origin = &basecamp.origin;
    assert_eq!(
        nexts,
        [
            format!("{origin}/12345/projects.json?page=2"),
            format!("{origin}/12345/projects.json?page=3"),
        ]
    );
    assert_eq!(
        basecamp.targets(),
        [
            "/12345/projects.json",
            "/12345/projects.json?page=2",
            "/12345/projects.json?page=3",
        ]
    );
}

/// A `rel="next"` under another account is not followed or answered: the
/// read is refused as an upstream protocol failure.
#[test]
fn a_next_page_under_another_account_refuses_the_read() {
    let basecamp = Basecamp::new();
    let mut child = Child::spawn(&basecamp.selection()).unwrap();
    let refusal = explained(&mut child, "projects.list", &json!({"status": "archived"}))
        .expect_err("another account's page is refused");
    assert_eq!(refusal.failure, Failure::Protocol);
    assert_eq!(basecamp.targets(), ["/12345/projects.json?status=archived"]);
}

/// `429` with `Retry-After: 120`, a wait the 30 s deadline cannot hold, is
/// refused as rate limited with that delay after one request.
#[test]
fn a_429_with_retry_after_is_rate_limited_with_its_delay() {
    let basecamp = Basecamp::new();
    let mut child = Child::spawn(&basecamp.selection()).unwrap();
    let refusal = explained(&mut child, "projects.list", &json!({"status": "trashed"}))
        .expect_err("a 429 is refused");
    assert_eq!(refusal.failure, Failure::ProviderRateLimited);
    assert_eq!(refusal.retry_after_seconds, Some(120));
    assert_eq!(basecamp.targets(), ["/12345/projects.json?status=trashed"]);
}

#[test]
fn a_404_is_not_found() {
    let basecamp = Basecamp::new();
    let mut child = Child::spawn(&basecamp.selection()).unwrap();
    let refusal = explained(&mut child, "todo.get", &json!({"todoId": 404404}))
        .expect_err("a 404 is refused");
    assert_eq!(refusal.failure, Failure::ProviderNotFound);
    assert_eq!(basecamp.targets(), ["/12345/todos/404404"]);
}

/// A caller cannot name the account: `accountId` is no input of any read,
/// so it is refused before any request, and neither can a page under
/// another account be reached through a path parameter.
#[test]
fn a_caller_cannot_pass_the_account_id() {
    let basecamp = Basecamp::new();
    let mut child = Child::spawn(&basecamp.selection()).unwrap();
    for (operation, input) in [
        ("projects.list", json!({"accountId": "99999"})),
        (
            "project.get",
            json!({"accountId": "99999", "projectId": 2085958499u64}),
        ),
        ("todo.get", json!({"todoId": "../../99999/todos/1"})),
        ("project.get", json!({"projectId": "1/../../99999"})),
    ] {
        let refusal = explained(&mut child, operation, &input).expect_err("refused");
        assert_eq!(
            refusal.failure,
            Failure::InvalidInput,
            "{operation} {input}"
        );
    }
    assert!(basecamp.requests().is_empty());
}

/// No write is offered: the instance describes the thirteen reads, each
/// required under the one profile as a read, and nothing else.
#[test]
fn no_write_is_offered() {
    let basecamp = Basecamp::new();
    let bootstrap = basecamp.bootstrap();
    let descriptor = bootstrap.descriptor().unwrap();
    let mut described: Vec<String> = descriptor
        .operations
        .iter()
        .map(|operation| operation.id.clone())
        .collect();
    described.sort();
    assert_eq!(
        described,
        SHIPPED.iter().map(|(id, _, _, _)| *id).collect::<Vec<_>>()
    );
    assert_eq!(bootstrap.requirements.len(), 13);
    for requirement in &bootstrap.requirements {
        assert!(
            matches!(
                requirement.effect,
                connectors_host::local::runtime::Effect::Read
            ),
            "{}",
            requirement.operation
        );
        assert_eq!(requirement.profile, PROFILE);
    }
    assert_eq!(bootstrap.profiles.len(), 1);
    assert!(basecamp.requests().is_empty());
}
