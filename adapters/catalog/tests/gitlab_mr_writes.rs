//! story:parity-gitlab-mr-writes: merge-request notes and discussions in the
//! shipped GitLab selection set.
//!
//! - `merge_request.note.create`, `postApiV4ProjectsIdMergeRequestsNoteableIdNotes`
//!   (`POST /projects/{id}/merge_requests/{noteable_id}/notes`), unguarded.
//! - `merge_request.discussion.reply`,
//!   `postApiV4ProjectsIdMergeRequestsNoteableIdDiscussionsDiscussionIdNotes`
//!   (`POST …/discussions/{discussion_id}/notes`), unguarded.
//! - `merge_request.discussion.get`,
//!   `getApiV4ProjectsIdMergeRequestsNoteableIdDiscussionsDiscussionId`, a read
//!   and the resolve guard's preflight.
//! - `merge_request.discussion.resolve`,
//!   `putApiV4ProjectsIdMergeRequestsNoteableIdDiscussionsDiscussionId`
//!   (`PUT …/discussions/{discussion_id}` with `resolved`), guarded: the
//!   discussion must be resolvable before, and must carry the requested
//!   `resolved` in GitLab's answer after.
//!
//! Each write runs through a private-protocol-two provider child against a
//! disposable HTTPS GitLab, prepared and committed as the host's approval
//! coordinator drives it once it holds a verified, spent approval. The
//! fixture answers in the pinned document's shapes (`APIEntitiesNote`,
//! `APIEntitiesDiscussion`) with synthetic ids, and records each request's
//! method, target and body, so "no request was sent" is a count of what it
//! saw. A merge request whose IID is `403`, `404`, `405` or `409` answers every
//! write with that status, so a provider refusal is observed by name; its
//! discussion reads as usual, so the resolve guard's preflight holds first.
use connectors_catalog::bundle;
use connectors_catalog_provider::{Effect, Engine, Selection};
use connectors_host::local::{
    config::{Adapter, Executable, Restart, Startup},
    filesystem,
    runtime::{self, Bootstrap, Child, Failure, PrivateProtocol, WriteEffect, WriteResult},
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

const INSTANCE: &str = "fixture-gitlab";
const PROFILE: &str = "gitlab.pat";
/// The fictional token the fixture accepts in `PRIVATE-TOKEN`.
const TOKEN: &str = "fixture-pat-one";
/// The fixture project `org/project`, its path id encoded as one segment.
const PROJECT: &str = "/api/v4/projects/org%2Fproject";
/// A resolvable thread on merge request 7, unresolved at the start.
const THREAD: &str = "d15c000000000000000000000000000000000001";
/// An individual note: a discussion GitLab does not let anyone resolve.
const SINGLE: &str = "d15c000000000000000000000000000000000002";
/// A resolvable thread whose PUT answers without the requested change, as a
/// GitLab that ignored or lost the change would.
const STUCK: &str = "d15c000000000000000000000000000000000003";
/// A discussion GitLab does not find.
const ABSENT: &str = "d15c000000000000000000000000000000000404";

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
fn engine() -> Engine {
    let bundle = bundle::load(&root().join("generated/bundles"), "gitlab").unwrap();
    Engine::new(&bundle, "/api/v4", &shipped()).unwrap()
}

/// A note GitLab answers, shaped as the pinned `APIEntitiesNote`.
fn note(id: u64, iid: u64, body: &Value, discussion: bool) -> Value {
    json!({"id": id, "type": if discussion { json!("DiscussionNote") } else { Value::Null },
           "body": body["body"], "author": {"id": 11, "username": "fixture-user",
           "name": "Fixture User", "state": "active"},
           "created_at": "2026-10-10T08:00:00.000Z", "updated_at": "2026-10-10T08:00:00.000Z",
           "system": false, "noteable_id": 501, "noteable_type": "MergeRequest",
           "project_id": 7, "resolvable": discussion, "resolved": false,
           "confidential": false, "internal": body["internal"].as_bool().unwrap_or(false),
           "noteable_iid": iid, "commands_changes": {}})
}
/// A discussion GitLab answers, shaped as the pinned `APIEntitiesDiscussion`.
fn discussion(id: &str, resolvable: bool, resolved: bool) -> Value {
    json!({"id": id, "individual_note": !resolvable, "resolvable": resolvable,
           "resolved": resolved,
           "notes": [{"id": 901, "type": if resolvable { "DiscussionNote" } else { "Note" },
                      "body": "Fixture thread", "system": false, "noteable_id": 501,
                      "noteable_type": "MergeRequest", "project_id": 7,
                      "resolvable": resolvable, "resolved": resolved, "noteable_iid": 7}]})
}

struct Fixture {
    /// The `resolved` state of each resolvable discussion, by id.
    resolved: BTreeMap<String, bool>,
}
impl Fixture {
    fn new() -> Self {
        Self {
            resolved: [(THREAD.to_owned(), false), (STUCK.to_owned(), false)]
                .into_iter()
                .collect(),
        }
    }
    fn answer(&mut self, method: &str, route: &str, body: &[u8]) -> (u16, Value) {
        let Some(rest) = route.strip_prefix(&format!("{PROJECT}/merge_requests/")) else {
            return (404, json!({"message": "404 Project Not Found"}));
        };
        let parts: Vec<&str> = rest.split('/').collect();
        let Ok(iid) = parts[0].parse::<u64>() else {
            return (404, json!({"message": "404 Not found"}));
        };
        let refused = matches!(iid, 403 | 404 | 405 | 409);
        let input: Value = serde_json::from_slice(body).unwrap_or(Value::Null);
        let refusal = |status: u16| {
            (
                status,
                json!({"message": format!("{status} fixture refusal")}),
            )
        };
        match (method, &parts[1..]) {
            ("POST", ["notes"]) if refused => refusal(iid as u16),
            ("POST", ["notes"]) => match input["body"].as_str() {
                Some(_) => (201, note(1201, iid, &input, false)),
                None => (400, json!({"error": "body is missing"})),
            },
            ("GET", ["discussions", id]) => match *id {
                THREAD | STUCK => (200, discussion(id, true, self.resolved[*id])),
                SINGLE => (200, discussion(id, false, false)),
                _ => (404, json!({"message": "404 Not found"})),
            },
            ("PUT", ["discussions", _]) if refused => refusal(iid as u16),
            ("PUT", ["discussions", id]) => match (*id, input["resolved"].as_bool()) {
                (THREAD, Some(resolved)) => {
                    self.resolved.insert(THREAD.to_owned(), resolved);
                    (200, discussion(id, true, resolved))
                }
                (STUCK, Some(_)) => (200, discussion(id, true, self.resolved[STUCK])),
                (THREAD | STUCK, None) => (400, json!({"error": "resolved is missing"})),
                _ => (404, json!({"message": "404 Not found"})),
            },
            ("POST", ["discussions", _, "notes"]) if refused => refusal(iid as u16),
            ("POST", ["discussions", id, "notes"]) => match (*id, input["body"].as_str()) {
                (THREAD | STUCK | SINGLE, Some(_)) => (201, note(1202, iid, &input, true)),
                (THREAD | STUCK | SINGLE, None) => (400, json!({"error": "body is missing"})),
                _ => (404, json!({"message": "404 Not found"})),
            },
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
                    "profile": PROFILE,
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
fn deadline() -> u64 {
    connectors_sdk::now_ms() + 30_000
}
/// One call on the read transport, which carries no approval.
fn read(child: &mut Child, operation: &str, input: &Value) -> Result<Value, Failure> {
    let revision = child.bootstrap().descriptor().unwrap().revision;
    child
        .invoke(
            operation,
            &revision,
            "one",
            &secret(),
            &serde_json::to_vec(input).unwrap(),
            deadline(),
        )
        .map(|bytes| serde_json::from_slice(&bytes).unwrap())
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
        deadline(),
    )?;
    Ok(prepared.commit())
}
fn applied(result: WriteResult, operation: &str) -> Value {
    assert_eq!(result.effect, WriteEffect::Applied, "{operation}");
    result
        .result
        .unwrap_or_else(|failure| panic!("`{operation}` result: {failure:?}"))
}
fn declared(operation: &str) -> connectors_core::Operation {
    engine()
        .declarations(&[Effect::Write])
        .into_iter()
        .find(|o| o.id == operation)
        .unwrap_or_else(|| panic!("`{operation}` is not a declared write"))
}
fn keys(schema: &Value) -> Vec<&str> {
    let mut names: Vec<&str> = schema["properties"]
        .as_object()
        .unwrap()
        .keys()
        .map(String::as_str)
        .collect();
    names.sort();
    names
}

fn note_input() -> Value {
    json!({"id": "org/project", "noteable_id": 7,
           "body": {"body": "Looks good; one question on the retry bound."}})
}
fn reply_input(thread: &str) -> Value {
    json!({"id": "org/project", "noteable_id": 7, "discussion_id": thread,
           "body": {"body": "Changed in the next commit."}})
}
fn resolve_input(thread: &str, resolved: bool) -> Value {
    json!({"id": "org/project", "noteable_id": 7, "discussion_id": thread,
           "body": {"resolved": resolved}})
}
fn with_iid(mut input: Value, iid: u64) -> Value {
    input["noteable_id"] = json!(iid);
    input
}

/// The four selections are shipped under these ids, from these pinned
/// operations, with these effects; the three writes are protocol-two write
/// requirements of the token profile, so an approval policy can name them and
/// the host requires an approval for each.
#[test]
fn notes_and_discussions_are_shipped_from_the_pinned_operations() {
    let selections = shipped();
    for (id, operation_id, effect, guarded) in [
        (
            "merge_request.note.create",
            "postApiV4ProjectsIdMergeRequestsNoteableIdNotes",
            Effect::Write,
            false,
        ),
        (
            "merge_request.discussion.get",
            "getApiV4ProjectsIdMergeRequestsNoteableIdDiscussionsDiscussionId",
            Effect::Read,
            false,
        ),
        (
            "merge_request.discussion.reply",
            "postApiV4ProjectsIdMergeRequestsNoteableIdDiscussionsDiscussionIdNotes",
            Effect::Write,
            false,
        ),
        (
            "merge_request.discussion.resolve",
            "putApiV4ProjectsIdMergeRequestsNoteableIdDiscussionsDiscussionId",
            Effect::Write,
            true,
        ),
    ] {
        let selection = selections
            .iter()
            .find(|s| s.id == id)
            .unwrap_or_else(|| panic!("`{id}` is not shipped"));
        assert_eq!(selection.operation_id, operation_id, "`{id}`");
        assert_eq!(selection.effect, effect, "`{id}`");
        assert_eq!(selection.guard.is_some(), guarded, "`{id}`");
        assert_eq!(engine().effect(id), Some(effect), "`{id}`");
    }
    let provider = Provider::new();
    let child = Child::spawn(&provider.selection()).unwrap();
    for id in [
        "merge_request.note.create",
        "merge_request.discussion.reply",
        "merge_request.discussion.resolve",
    ] {
        let requirement = child
            .bootstrap()
            .requirements
            .iter()
            .find(|r| r.operation == id)
            .unwrap_or_else(|| panic!("`{id}` is not a protocol-two requirement"));
        assert!(requirement.effect == runtime::Effect::Write, "`{id}`");
        assert_eq!(requirement.profile, PROFILE, "`{id}`");
    }
    assert!(provider.requests().is_empty());
}

/// The note writes close their body: a note carries `body` and, for a merge
/// request note, `internal`. `created_at` (administrators and owners only),
/// `confidential` (GitLab's deprecated spelling of `internal`) and
/// `merge_request_diff_head_sha` (which GitLab requires before it runs a
/// `/merge` quick action from a note) are not admitted, and a body carrying
/// one is refused before any request.
#[test]
fn note_writes_admit_only_the_note_text_and_refuse_any_other_body_key_before_a_request() {
    let create = declared("merge_request.note.create");
    assert_eq!(keys(&create.input_schema), ["body", "id", "noteable_id"]);
    assert_eq!(
        create.input_schema["required"],
        json!(["body", "id", "noteable_id"])
    );
    assert_eq!(
        keys(&create.input_schema["properties"]["body"]),
        ["body", "internal"]
    );
    assert_eq!(
        create.input_schema["properties"]["body"]["additionalProperties"],
        json!(false)
    );
    let reply = declared("merge_request.discussion.reply");
    assert_eq!(
        keys(&reply.input_schema),
        ["body", "discussion_id", "id", "noteable_id"]
    );
    assert_eq!(keys(&reply.input_schema["properties"]["body"]), ["body"]);
    let provider = Provider::new();
    let mut child = Child::spawn(&provider.selection()).unwrap();
    for (operation, input) in [
        ("merge_request.note.create", note_input()),
        ("merge_request.discussion.reply", reply_input(THREAD)),
    ] {
        for (key, value) in [
            ("created_at", json!("2020-01-01T00:00:00Z")),
            ("confidential", json!(true)),
            ("merge_request_diff_head_sha", json!("a".repeat(40))),
        ] {
            let mut input = input.clone();
            input["body"][key] = value;
            let outcome = write(&mut child, operation, &input).map(|r| r.effect);
            assert!(
                matches!(outcome, Err(Failure::InvalidInput)),
                "`{operation}` with body.{key}: {outcome:?}"
            );
        }
    }
    assert!(provider.requests().is_empty(), "a refused body was sent");
}

/// Approved, a note is one `POST` on the merge request's notes route with the
/// body as supplied, no preflight, and GitLab's 201 note as applied.
#[test]
fn an_approved_note_sends_exactly_one_post_with_the_body_as_supplied() {
    let provider = Provider::new();
    let mut child = Child::spawn(&provider.selection()).unwrap();
    let mut input = note_input();
    input["body"]["internal"] = json!(true);
    let result = applied(
        write(&mut child, "merge_request.note.create", &input).unwrap(),
        "merge_request.note.create",
    );
    assert_eq!(
        provider.requests(),
        [(
            "POST".to_owned(),
            format!("{PROJECT}/merge_requests/7/notes?"),
            input["body"].clone()
        )]
    );
    assert_eq!(result["status"], 201);
    assert_eq!(result["body"], note(1201, 7, &input["body"], false));
    assert_eq!(result["provenance"]["instance"], INSTANCE);
}

/// Approved, a reply is one `POST` on the discussion's notes route with the
/// body as supplied, no preflight, and GitLab's 201 note as applied.
#[test]
fn an_approved_reply_sends_exactly_one_post_to_the_discussion() {
    let provider = Provider::new();
    let mut child = Child::spawn(&provider.selection()).unwrap();
    let input = reply_input(THREAD);
    let result = applied(
        write(&mut child, "merge_request.discussion.reply", &input).unwrap(),
        "merge_request.discussion.reply",
    );
    assert_eq!(
        provider.requests(),
        [(
            "POST".to_owned(),
            format!("{PROJECT}/merge_requests/7/discussions/{THREAD}/notes?"),
            input["body"].clone()
        )]
    );
    assert_eq!(result["status"], 201);
    assert_eq!(result["body"]["type"], "DiscussionNote");
}

/// `merge_request.discussion.get` reads one discussion: one `GET` on its
/// route, GitLab's body unchanged.
#[test]
fn discussion_get_reads_one_discussion() {
    let provider = Provider::new();
    let mut child = Child::spawn(&provider.selection()).unwrap();
    let result = read(
        &mut child,
        "merge_request.discussion.get",
        &json!({"id": "org/project", "noteable_id": 7, "discussion_id": THREAD}),
    )
    .unwrap();
    assert_eq!(
        provider.requests(),
        [(
            "GET".to_owned(),
            format!("{PROJECT}/merge_requests/7/discussions/{THREAD}?"),
            Value::Null
        )]
    );
    assert_eq!(result["status"], 200);
    assert_eq!(result["body"], discussion(THREAD, true, false));
    let absent = read(
        &mut child,
        "merge_request.discussion.get",
        &json!({"id": "org/project", "noteable_id": 7, "discussion_id": ABSENT}),
    );
    assert!(
        matches!(absent, Err(Failure::ProviderNotFound)),
        "{absent:?}"
    );
}

/// The resolve write takes the three path parameters and a closed body of
/// exactly `resolved`, which the guard reads and so is required and a scalar.
#[test]
fn resolve_declares_a_closed_body_of_resolved_which_its_guard_reads() {
    let resolve = declared("merge_request.discussion.resolve");
    assert_eq!(
        keys(&resolve.input_schema),
        ["body", "discussion_id", "id", "noteable_id"]
    );
    let body = &resolve.input_schema["properties"]["body"];
    assert_eq!(keys(body), ["resolved"]);
    assert_eq!(body["required"], json!(["resolved"]));
    assert_eq!(body["additionalProperties"], json!(false));
    let guard = shipped()
        .into_iter()
        .find(|s| s.id == "merge_request.discussion.resolve")
        .unwrap()
        .guard
        .unwrap();
    assert_eq!(
        guard.preflight.operation_id,
        "getApiV4ProjectsIdMergeRequestsNoteableIdDiscussionsDiscussionId"
    );
}

/// Approved, resolving reads the discussion once, then sends one `PUT` with
/// exactly `{"resolved": true}`; GitLab's answer carries the new state, so the
/// postflight proves it and the effect is applied. Unresolving is the same
/// with `false`.
#[test]
fn an_approved_resolve_reads_the_discussion_then_sends_one_put_and_proves_the_new_state() {
    let provider = Provider::new();
    let mut child = Child::spawn(&provider.selection()).unwrap();
    let discussion_route = format!("{PROJECT}/merge_requests/7/discussions/{THREAD}?");
    for resolved in [true, false] {
        let before = provider.requests().len();
        let input = resolve_input(THREAD, resolved);
        let result = applied(
            write(&mut child, "merge_request.discussion.resolve", &input).unwrap(),
            "merge_request.discussion.resolve",
        );
        assert_eq!(
            provider.requests()[before..],
            [
                ("GET".to_owned(), discussion_route.clone(), Value::Null),
                (
                    "PUT".to_owned(),
                    discussion_route.clone(),
                    json!({"resolved": resolved})
                ),
            ]
        );
        assert_eq!(result["status"], 200);
        assert_eq!(result["body"]["resolved"], resolved);
    }
}

/// The guard refuses before the write: a discussion GitLab does not let
/// anyone resolve (an individual note), and a discussion GitLab does not find,
/// each after the one preflight `GET` and with no `PUT` sent. A missing or
/// non-scalar `resolved` and an extra body key are refused before any
/// request at all.
#[test]
fn resolve_refuses_each_preflight_mismatch_before_the_write() {
    let provider = Provider::new();
    let mut child = Child::spawn(&provider.selection()).unwrap();
    for thread in [SINGLE, ABSENT] {
        let before = provider.requests().len();
        let outcome = write(
            &mut child,
            "merge_request.discussion.resolve",
            &resolve_input(thread, true),
        )
        .map(|r| r.effect);
        assert!(
            matches!(outcome, Err(Failure::Forbidden)),
            "{thread}: {outcome:?}"
        );
        let sent = provider.requests()[before..].to_vec();
        assert_eq!(sent.len(), 1, "{thread}: {sent:?}");
        assert_eq!(sent[0].0, "GET", "{thread}");
    }
    let before = provider.requests().len();
    for body in [
        json!({}),
        json!({"resolved": {"value": true}}),
        json!({"resolved": true, "body": "and a note"}),
    ] {
        let mut input = resolve_input(THREAD, true);
        input["body"] = body.clone();
        let outcome =
            write(&mut child, "merge_request.discussion.resolve", &input).map(|r| r.effect);
        assert!(
            matches!(outcome, Err(Failure::InvalidInput)),
            "{body}: {outcome:?}"
        );
    }
    assert_eq!(
        provider.requests().len(),
        before,
        "a refused input was sent"
    );
    assert!(
        provider
            .requests()
            .iter()
            .all(|(method, _, _)| method == "GET"),
        "a write was sent past a refusing guard"
    );
}

/// GitLab's answer to the `PUT` decides the outcome after dispatch: an answer
/// whose `resolved` is not the requested state leaves the effect unknown,
/// never refused and never applied.
#[test]
fn a_resolve_answer_without_the_requested_state_is_unknown() {
    let provider = Provider::new();
    let mut child = Child::spawn(&provider.selection()).unwrap();
    let result = write(
        &mut child,
        "merge_request.discussion.resolve",
        &resolve_input(STUCK, true),
    )
    .unwrap();
    assert_eq!(result.effect, WriteEffect::Unknown);
    let methods: Vec<String> = provider.requests().into_iter().map(|r| r.0).collect();
    assert_eq!(methods, ["GET", "PUT"]);
}

/// A provider refusal of the write itself is refused by name for each write:
/// `403` and `405` as the provider's forbidden, `404` as the provider's
/// not-found, `409` as invalid input, each after exactly the one write
/// request (and, for resolve, its preflight).
#[test]
fn provider_refusals_of_each_write_are_mapped_by_name() {
    let provider = Provider::new();
    let mut child = Child::spawn(&provider.selection()).unwrap();
    for (status, expected) in [
        (403, Failure::ProviderForbidden),
        (404, Failure::ProviderNotFound),
        (405, Failure::ProviderForbidden),
        (409, Failure::InvalidInput),
    ] {
        for (operation, input, sent) in [
            (
                "merge_request.note.create",
                with_iid(note_input(), status),
                vec!["POST"],
            ),
            (
                "merge_request.discussion.reply",
                with_iid(reply_input(THREAD), status),
                vec!["POST"],
            ),
            (
                "merge_request.discussion.resolve",
                with_iid(resolve_input(THREAD, true), status),
                vec!["GET", "PUT"],
            ),
        ] {
            let before = provider.requests().len();
            let result = write(&mut child, operation, &input)
                .unwrap_or_else(|failure| panic!("`{operation}` {status}: {failure:?}"));
            assert_eq!(
                result.effect,
                WriteEffect::Refused,
                "`{operation}` {status}"
            );
            let failure = result.result.err();
            assert_eq!(
                format!("{failure:?}"),
                format!("{:?}", Some(expected)),
                "`{operation}` {status}"
            );
            let methods: Vec<String> = provider.requests()[before..]
                .iter()
                .map(|r| r.0.clone())
                .collect();
            assert_eq!(methods, sent, "`{operation}` {status}");
        }
    }
}

/// The merge and update selections keep their guards unchanged: this story
/// adds no variant of either (see `docs/local-gitlab-merge.md`, *Not
/// selected*), so the five-check merge guard and the open-at-the-pinned-head
/// update guard are exactly as they were.
#[test]
fn merge_and_update_guards_are_unchanged() {
    let selections = shipped();
    let guard = |id: &str| {
        serde_json::to_value(
            selections
                .iter()
                .find(|s| s.id == id)
                .unwrap()
                .guard
                .as_ref()
                .unwrap(),
        )
        .unwrap()
    };
    assert_eq!(
        guard("merge_request.merge"),
        json!({
            "preflight": {"operation_id": "getApiV4ProjectsIdMergeRequestsMergeRequestIid",
                          "values": {"id": "id", "merge_request_iid": "merge_request_iid"},
                          "checks": [{"pointer": "/sha", "expect": {"input": "body.sha"}},
                                     {"pointer": "/state", "expect": {"literal": "opened"}},
                                     {"pointer": "/detailed_merge_status", "expect": {"literal": "mergeable"}},
                                     {"pointer": "/head_pipeline/id", "expect": {"input": "pipeline_id"}},
                                     {"pointer": "/head_pipeline/status", "expect": {"literal": "success"}}]},
            "postflight": {"checks": [{"pointer": "/state", "expect": {"literal": "merged"}},
                                      {"pointer": "/sha", "expect": {"input": "body.sha"}}]}})
    );
    assert_eq!(
        guard("merge_request.update"),
        json!({
            "preflight": {"operation_id": "getApiV4ProjectsIdMergeRequestsMergeRequestIid",
                          "values": {"id": "id", "merge_request_iid": "merge_request_iid"},
                          "checks": [{"pointer": "/sha", "expect": {"input": "sha"}},
                                     {"pointer": "/state", "expect": {"literal": "opened"}}]},
            "postflight": {"checks": [{"pointer": "/sha", "expect": {"input": "sha"}}]}})
    );
}
