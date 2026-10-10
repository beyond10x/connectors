//! Jira Cloud issues, one issue, comments, changelog, transitions, create metadata
//! and users through the catalog provider.
//!
//! The shipped selection set is pinned by id and source operation, resolves
//! against the committed bundle compiled from the pinned platform REST v3
//! document, and is cited row by row in `docs/catalog-jira.md`. Each read runs
//! through the provider child against a disposable HTTPS fixture: the exact
//! request (path, query with its time filter, `Authorization: Basic …`) and the
//! returned body are asserted, and every list walks to the end condition the
//! guide documents. Every key, id and name in the fixture is synthetic. No live
//! credential and no network.
use connectors_catalog::bundle;
use connectors_catalog_provider::{Effect, Engine, Selection};
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

/// The base path every Jira platform REST v3 operation sits under.
const BASE: &str = "/rest/api/3";
/// Fictional basic material, and the header the fixture accepts:
/// `Basic base64("reader@example.test:fixture-jira-token")`, computed outside
/// the provider with coreutils `base64`.
const ACCOUNT: &str = "reader@example.test";
const TOKEN: &str = "fixture-jira-token";
const HEADER: &str = "Basic cmVhZGVyQGV4YW1wbGUudGVzdDpmaXh0dXJlLWppcmEtdG9rZW4=";
const PROFILE: &str = "atlassian.basic";

/// The shipped ids, their pinned `operationId` and their pinned path. A renamed,
/// dropped or added id fails here.
const SHIPPED: [(&str, &str, &str); 8] = [
    (
        "attachment.content",
        "getAttachmentContent",
        "/rest/api/3/attachment/content/{id}",
    ),
    (
        "issue.changelog",
        "getChangeLogs",
        "/rest/api/3/issue/{issueIdOrKey}/changelog",
    ),
    (
        "issue.comments",
        "getComments",
        "/rest/api/3/issue/{issueIdOrKey}/comment",
    ),
    (
        "issue.create_meta",
        "getCreateIssueMetaIssueTypes",
        "/rest/api/3/issue/createmeta/{projectIdOrKey}/issuetypes",
    ),
    ("issue.get", "getIssue", "/rest/api/3/issue/{issueIdOrKey}"),
    (
        "issue.transitions",
        "getTransitions",
        "/rest/api/3/issue/{issueIdOrKey}/transitions",
    ),
    (
        "issues.search",
        "searchAndReconsileIssuesUsingJql",
        "/rest/api/3/search/jql",
    ),
    ("users.search", "findUsers", "/rest/api/3/user/search"),
];

fn root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
}
fn shipped() -> Vec<Selection> {
    let file: Value =
        serde_json::from_slice(&fs::read(root().join("providers/jira/operations.json")).unwrap())
            .unwrap();
    assert_eq!(file["format"], "connectors-catalog-operations/1");
    assert_eq!(file["provider"], "jira");
    serde_json::from_value(file["operations"].clone()).unwrap()
}

/// The eight reads, the attachment content among them (a binary read,
/// `tests/jira_attachment.rs`), and one write: `issue.transition.run`, `doTransition`,
/// whose guard `tests/jira_transition_run.rs` pins.
#[test]
fn shipped_jira_selections_are_the_eight_reads_and_the_transition_run() {
    let selections = shipped();
    let bundle = bundle::load(&root().join("generated/bundles"), "jira").unwrap();
    let engine = Engine::new(&bundle, BASE, &selections).unwrap();
    let mut declared: Vec<String> = engine
        .declarations(&[Effect::Read])
        .into_iter()
        .map(|o| o.id)
        .collect();
    declared.sort();
    let expected: Vec<&str> = SHIPPED.iter().map(|(id, _, _)| *id).collect();
    assert_eq!(declared, expected);
    let writes: Vec<String> = engine
        .declarations(&[Effect::Write])
        .into_iter()
        .map(|o| o.id)
        .collect();
    assert_eq!(writes, ["issue.transition.run"]);
    assert_eq!(selections.len(), SHIPPED.len() + 1);
    for (id, operation_id, path) in SHIPPED {
        let selection = selections.iter().find(|s| s.id == id).unwrap();
        assert_eq!(selection.operation_id, operation_id, "`{id}`");
        assert_eq!(selection.effect, Effect::Read, "`{id}`");
        assert_eq!(engine.effect(id), Some(Effect::Read), "`{id}`");
        let operation = bundle
            .inventory
            .operations
            .iter()
            .find(|o| o.operation_id.as_deref() == Some(operation_id))
            .unwrap_or_else(|| panic!("the pinned source lacks `{operation_id}`"));
        assert_eq!(operation.method, "get", "`{id}`");
        assert_eq!(operation.path, path, "`{id}`");
    }
}

#[test]
fn a_selection_the_pinned_source_lacks_is_refused_at_load() {
    let bundle = bundle::load(&root().join("generated/bundles"), "jira").unwrap();
    let mut selections = shipped();
    // A plausible id the pinned source does not carry (its deprecated search
    // is `searchForIssuesUsingJql`, which the shipped set does not use).
    assert!(
        selections
            .iter()
            .all(|s| s.operation_id != "searchForIssuesUsingJql")
    );
    selections[0].operation_id = "searchForIssuesUsingJqlGetDeprecated".into();
    assert!(Engine::new(&bundle, BASE, &selections).is_err());
}

#[test]
fn the_pinned_source_is_the_one_the_bundle_and_its_record_name() {
    let upstream = root().join("../atlassian/upstream");
    let bytes = fs::read(upstream.join("jira-platform-v3.json")).unwrap();
    let digest = hex::encode(Sha256::digest(&bytes));
    let index = bundle::read_index(&root().join("generated/bundles")).unwrap();
    let entry = index.find("jira").unwrap();
    assert_eq!(entry.source_sha256, digest);
    let bundle = bundle::load(&root().join("generated/bundles"), "jira").unwrap();
    assert_eq!(bundle.source.source_bytes, bytes.len());
    assert_eq!(bundle.source.file_name, "jira-platform-v3.json");
    let manifest: Value =
        serde_json::from_slice(&fs::read(upstream.join("jira-source-hashes.json")).unwrap())
            .unwrap();
    let record = manifest
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["file"] == "jira-platform-v3.json")
        .unwrap();
    assert_eq!(record["sha256"], digest.as_str());
    assert_eq!(record["bytes"], bytes.len());
    let readme = fs::read_to_string(upstream.join("README.md")).unwrap();
    assert!(readme.contains(&digest), "README does not name the digest");
}

#[test]
fn guide_cites_each_operation_its_paging_and_its_deltas() {
    let guide = fs::read_to_string(root().join("../../docs/catalog-jira.md")).unwrap();
    let rows: Vec<&str> = guide.lines().filter(|l| l.starts_with('|')).collect();
    // Per operation: id, pinned operationId and path, paging parameters, end
    // condition, and the time filter or how deltas are taken.
    for (id, operation_id, path, paging, end, deltas) in [
        (
            "issues.search",
            "searchAndReconsileIssuesUsingJql",
            "/rest/api/3/search/jql",
            "`nextPageToken`, `maxResults`",
            "`nextPageToken` absent or null",
            "`updated >=",
        ),
        (
            "issue.comments",
            "getComments",
            "/rest/api/3/issue/{issueIdOrKey}/comment",
            "`startAt`, `maxResults`",
            "`startAt + len(comments) >= total`",
            "`issues.search`",
        ),
        (
            "issue.changelog",
            "getChangeLogs",
            "/rest/api/3/issue/{issueIdOrKey}/changelog",
            "`startAt`, `maxResults`",
            "`isLast: true`",
            "`issues.search`",
        ),
        (
            "issue.get",
            "getIssue",
            "/rest/api/3/issue/{issueIdOrKey}",
            "none",
            "one issue per call",
            "`issues.search`",
        ),
        (
            "issue.transitions",
            "getTransitions",
            "/rest/api/3/issue/{issueIdOrKey}/transitions",
            "none",
            "one issue per call",
            "none",
        ),
        (
            "issue.create_meta",
            "getCreateIssueMetaIssueTypes",
            "/rest/api/3/issue/createmeta/{projectIdOrKey}/issuetypes",
            "`startAt`, `maxResults`",
            "`startAt + len(issueTypes) >= total`",
            "none",
        ),
        (
            "users.search",
            "findUsers",
            "/rest/api/3/user/search",
            "`startAt`, `maxResults`",
            "an empty page",
            "none",
        ),
    ] {
        let cited = rows.iter().any(|row| {
            row.contains(&format!("`{id}`"))
                && row.contains(&format!("`{operation_id}`"))
                && row.contains(&format!("`GET {path}`"))
        });
        assert!(cited, "no row cites `{id}` as `{operation_id}` `{path}`");
        let paged = rows.iter().any(|row| {
            row.contains(&format!("`{id}`"))
                && row.contains(paging)
                && row.contains(end)
                && row.contains(deltas)
        });
        assert!(paged, "no row states the paging of `{id}`");
    }
}

/// Route and `Authorization` header of each fixture request.
type Requests = Arc<Mutex<Vec<(String, Option<String>)>>>;

/// The recorded pages the fixture serves, keyed by route and paging position.
/// Each list has three items over two pages, and the user search a third,
/// empty page; `None` for anything else, which the fixture answers 404.
fn page(path: &str) -> Option<Value> {
    let (route, query) = path.split_once('?').unwrap_or((path, ""));
    let has = |pair: &str| query.split('&').any(|p| p == pair);
    let issue = |id: u64, key: &str| {
        json!({"id": id.to_string(), "key": key,
               "self": format!("https://tracker.example.test/rest/api/3/issue/{id}"),
               "fields": {"summary": format!("fixture issue {key}"),
                          "updated": "2026-09-20T10:00:00.000+0000"}})
    };
    let comment = |id: u64| {
        json!({"id": id.to_string(),
               "author": {"accountId": "fixture-author", "displayName": "Fixture Author"},
               "body": {"type": "doc", "version": 1, "content": [{"type": "paragraph",
                        "content": [{"type": "text", "text": format!("comment {id}")}]}]},
               "created": "2026-09-10T08:00:00.000+0000",
               "updated": "2026-09-10T08:00:00.000+0000"})
    };
    let change = |id: u64| {
        json!({"id": id.to_string(),
               "author": {"accountId": "fixture-author"},
               "created": "2026-09-11T09:00:00.000+0000",
               "items": [{"field": "status", "fieldtype": "jira",
                          "fromString": "Open", "toString": "Done"}]})
    };
    // An issue type of `PageOfCreateMetaIssueTypes.issueTypes`.
    let issue_type = |id: u64, name: &str| {
        json!({"id": id.to_string(), "name": name, "subtask": false,
               "description": format!("fixture issue type {name}"),
               "iconUrl": format!("https://tracker.example.test/images/icons/issuetypes/{id}.png"),
               "self": format!("https://tracker.example.test/rest/api/3/issuetype/{id}")})
    };
    // A `User` of the `findUsers` array; no email address, as privacy
    // controls may withhold it.
    let user = |n: u64| {
        json!({"accountId": format!("fixture-user-{n}"), "accountType": "atlassian",
               "active": true, "displayName": format!("Fixture User {n}"),
               "self": format!("https://tracker.example.test/rest/api/3/user?accountId=fixture-user-{n}")})
    };
    // An `IssueTransition` of `Transitions.transitions`, with its target
    // status in `to` (a `StatusDetails`); `available` false is what
    // `includeUnavailableTransitions=true` adds.
    let transition = |id: &str, name: &str, to: (&str, &str), available: bool| {
        json!({"id": id, "name": name,
               "to": {"id": to.0, "name": to.1, "description": format!("fixture status {}", to.1),
                      "iconUrl": "https://tracker.example.test/images/icons/status.png",
                      "self": format!("https://tracker.example.test/rest/api/3/status/{}", to.0),
                      "statusCategory": {"id": 4, "key": "indeterminate", "name": "In Progress"}},
               "hasScreen": false, "isAvailable": available, "isConditional": false,
               "isGlobal": true, "isInitial": false, "looped": false})
    };
    Some(match route {
        "/rest/api/3/myself" => json!({"accountId": "fixture-account-id", "active": true}),
        // `getTransitions`: one transition when `transitionId` names it, the
        // transition screen's fields under `expand=transitions.fields`, and
        // a transition that fails a condition only when asked for.
        "/rest/api/3/issue/FIX-1/transitions" if has("transitionId=31") => json!({
            "transitions": [transition("31", "Done", ("10002", "Done"), true)]}),
        "/rest/api/3/issue/FIX-1/transitions" if has("expand=transitions.fields") => {
            let mut done = transition("31", "Done", ("10002", "Done"), true);
            done["hasScreen"] = json!(true);
            done["fields"] = json!({"resolution": {
                "key": "resolution", "name": "Resolution", "required": true,
                "operations": ["set"], "schema": {"type": "resolution", "system": "resolution"},
                "allowedValues": [{"id": "10000", "name": "Done"}]}});
            json!({"expand": "transitions", "transitions": [done]})
        }
        "/rest/api/3/issue/FIX-1/transitions" if has("includeUnavailableTransitions=true") => {
            json!({"transitions": [
                transition("11", "To Do", ("10000", "To Do"), true),
                transition("31", "Done", ("10002", "Done"), true),
                transition("41", "Reopen", ("10003", "Reopened"), false)]})
        }
        "/rest/api/3/issue/FIX-1/transitions" => json!({"transitions": [
            transition("11", "To Do", ("10000", "To Do"), true),
            transition("31", "Done", ("10002", "Done"), true)]}),
        // `getIssue` with `fields` and `expand=renderedFields,names`: the
        // `IssueBean` keeps the ADF body in `fields` and the rendered HTML in
        // `renderedFields`; attachments come as the `attachment` field.
        "/rest/api/3/issue/FIX-1" => json!({
            "expand": "renderedFields,names",
            "id": "10001", "key": "FIX-1",
            "self": "https://tracker.example.test/rest/api/3/issue/10001",
            "fields": {
                "summary": "fixture issue FIX-1",
                "status": {"id": "3", "name": "In Progress"},
                "description": {"type": "doc", "version": 1, "content": [{"type": "paragraph",
                                "content": [{"type": "text", "text": "fixture description"}]}]},
                "attachment": [{"id": "20001", "filename": "fixture.txt", "mimeType": "text/plain",
                                "size": 7, "created": "2026-09-12T08:00:00.000+0000",
                                "content": "https://tracker.example.test/rest/api/3/attachment/content/20001"}]
            },
            "renderedFields": {"description": "<p>fixture description</p>"},
            "names": {"summary": "Summary", "status": "Status",
                      "description": "Description", "attachment": "Attachment"}}),
        "/rest/api/3/issue/createmeta/FIX/issuetypes" if has("startAt=2") => json!({
            "startAt": 2, "maxResults": 2, "total": 3, "issueTypes": [issue_type(3, "Bug")]}),
        "/rest/api/3/issue/createmeta/FIX/issuetypes" => json!({
            "startAt": 0, "maxResults": 2, "total": 3,
            "issueTypes": [issue_type(1, "Task"), issue_type(2, "Story")]}),
        "/rest/api/3/user/search" if has("startAt=3") => json!([]),
        "/rest/api/3/user/search" if has("startAt=2") => json!([user(3)]),
        "/rest/api/3/user/search" => json!([user(1), user(2)]),
        "/rest/api/3/search/jql" if has("nextPageToken=fixture-page-2") => {
            json!({"issues": [issue(10003, "FIX-3")], "isLast": true})
        }
        "/rest/api/3/search/jql" => json!({
            "issues": [issue(10001, "FIX-1"), issue(10002, "FIX-2")],
            "nextPageToken": "fixture-page-2", "isLast": false}),
        "/rest/api/3/issue/FIX-1/comment" if has("startAt=2") => {
            json!({"startAt": 2, "maxResults": 2, "total": 3, "comments": [comment(3)]})
        }
        "/rest/api/3/issue/FIX-1/comment" => json!({
            "startAt": 0, "maxResults": 2, "total": 3, "comments": [comment(1), comment(2)]}),
        "/rest/api/3/issue/FIX-1/changelog" if has("startAt=2") => json!({
            "startAt": 2, "maxResults": 2, "total": 3, "isLast": true, "values": [change(3)]}),
        "/rest/api/3/issue/FIX-1/changelog" => json!({
            "startAt": 0, "maxResults": 2, "total": 3, "isLast": false,
            "values": [change(1), change(2)]}),
        _ => return None,
    })
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
                address_tx.send(listener.local_addr().unwrap()).unwrap();
                let acceptor = TlsAcceptor::from(Arc::new(tls));
                loop {
                    let stream = tokio::select! {_=&mut stopped=>break,value=listener.accept()=>value.unwrap().0};
                    let Ok(mut stream) = acceptor.accept(stream).await else {
                        continue;
                    };
                    let mut header = Vec::new();
                    while !header.ends_with(b"\r\n\r\n") {
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
                    let authorization = request.lines().find_map(|line| {
                        line.split_once(':')
                            .filter(|(name, _)| name.eq_ignore_ascii_case("authorization"))
                            .map(|(_, value)| value.trim().to_owned())
                    });
                    // Only the fictional header is accepted; raw headers are
                    // compared, never printed.
                    let (status, body) = if authorization.as_deref() != Some(HEADER) {
                        (401, json!({"errorMessages": ["fixture refusal"]}))
                    } else {
                        match page(&path) {
                            Some(body) => (200, body),
                            None => (404, json!({"errorMessages": ["no fixture"]})),
                        }
                    };
                    observed.lock().unwrap().push((path, authorization));
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
        let config = directory.join("catalog.json");
        private(
            &config,
            &serde_json::to_vec(&json!({
                "format": "connectors-catalog-local/2",
                "instance": "fixture-jira",
                "provider": "jira",
                "bundle_directory": root_path("generated/bundles"),
                "api_base": format!("https://localhost:{}/rest/api/3", address.port()),
                "ca_file": ca,
                "auth": documented_auth(),
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
            private_protocol: None,
            permissions: Default::default(),
            instance_id: "fixture-jira".into(),
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
    fn requests(&self) -> Vec<(String, Option<String>)> {
        self.requests.lock().unwrap().clone()
    }
}
impl Drop for Provider {
    fn drop(&mut self) {
        let _ = self.stop.take().unwrap().send(());
        self.thread.take().unwrap().join().unwrap();
    }
}
/// The `auth` profile of the configuration example in the Jira guide, so the
/// fixture runs the profile the guide tells a reader to write.
fn documented_auth() -> Value {
    let guide = fs::read_to_string(root().join("../../docs/catalog-jira.md")).unwrap();
    let example = guide
        .split("```json\n")
        .skip(1)
        .filter_map(|rest| rest.split_once("\n```").map(|(body, _)| body))
        .find(|body| body.contains("\"provider\": \"jira\""))
        .expect("the documented Jira configuration");
    let auth = serde_json::from_str::<Value>(example).unwrap()["auth"].clone();
    assert_eq!(auth["profile"], PROFILE);
    assert_eq!(auth["scheme"], "basic");
    assert_eq!(auth["identity"]["kind"], "atlassian.account");
    auth
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
fn invoke(child: &mut Child, operation: &str, input: Value) -> Value {
    let revision = child.bootstrap().descriptor().unwrap().revision;
    let output = child
        .invoke(
            operation,
            &revision,
            "one",
            &secret(),
            &serde_json::to_vec(&input).unwrap(),
            connectors_sdk::now_ms() + 30_000,
        )
        .unwrap_or_else(|failure| panic!("`{operation}` failed: {failure:?}"));
    serde_json::from_slice(&output).unwrap()
}

/// Each read's first page: the input, and the exact request the fixture must
/// observe, including the JQL time filter on the search.
fn first_pages() -> [(&'static str, Value, &'static str); 4] {
    [
        (
            "issues.search",
            json!({"jql": "project = FIX AND updated >= \"2026-09-01 00:00\" ORDER BY updated ASC",
                   "fields": "summary,updated", "maxResults": 2}),
            "/rest/api/3/search/jql?jql=project+%3D+FIX+AND+updated+%3E%3D+%222026-09-01+00%3A00%22\
             +ORDER+BY+updated+ASC&maxResults=2&fields=summary%2Cupdated",
        ),
        (
            "issue.comments",
            json!({"issueIdOrKey": "FIX-1", "startAt": 0, "maxResults": 2}),
            "/rest/api/3/issue/FIX-1/comment?startAt=0&maxResults=2",
        ),
        (
            "issue.changelog",
            json!({"issueIdOrKey": "FIX-1", "startAt": 0, "maxResults": 2}),
            "/rest/api/3/issue/FIX-1/changelog?startAt=0&maxResults=2",
        ),
        (
            "issue.create_meta",
            json!({"projectIdOrKey": "FIX", "startAt": 0, "maxResults": 2}),
            "/rest/api/3/issue/createmeta/FIX/issuetypes?startAt=0&maxResults=2",
        ),
    ]
}

#[test]
fn each_read_sends_the_declared_request_with_basic_auth_and_returns_the_recorded_body() {
    let provider = Provider::new();
    let mut child = Child::spawn(&provider.selection()).unwrap();
    for (operation, input, expected) in first_pages() {
        let before = provider.requests().len();
        let result = invoke(&mut child, operation, input);
        let requests = provider.requests();
        assert_eq!(requests.len(), before + 1, "`{operation}` requests");
        assert_eq!(requests[before].0, expected, "`{operation}` request");
        assert_eq!(
            requests[before].1.as_deref(),
            Some(HEADER),
            "`{operation}` authorization"
        );
        assert_eq!(result["status"], 200, "`{operation}` status");
        // The engine re-serialises the body, so the recorded page is compared
        // as JSON, not as bytes.
        assert_eq!(
            Some(&result["body"]),
            page(expected).as_ref(),
            "`{operation}` body"
        );
        assert_eq!(result["provenance"]["instance"], "fixture-jira");
    }
}

/// The documented end condition of each list, read from the page body alone,
/// and the input for the next page when there is one.
fn next(operation: &str, input: &Value, body: &Value) -> Option<Value> {
    let mut input = input.clone();
    match operation {
        // No `nextPageToken` in the response.
        "issues.search" => {
            let token = body.get("nextPageToken")?.as_str()?;
            input["nextPageToken"] = json!(token);
        }
        // `startAt + len(comments) >= total`.
        "issue.comments" => {
            let start = body["startAt"].as_u64().unwrap();
            let seen = start + body["comments"].as_array().unwrap().len() as u64;
            if seen >= body["total"].as_u64().unwrap() {
                return None;
            }
            input["startAt"] = json!(seen);
        }
        // `startAt + len(issueTypes) >= total`.
        "issue.create_meta" => {
            let start = body["startAt"].as_u64().unwrap();
            let seen = start + body["issueTypes"].as_array().unwrap().len() as u64;
            if seen >= body["total"].as_u64().unwrap() {
                return None;
            }
            input["startAt"] = json!(seen);
        }
        // An empty page: the body is the bare `User` array.
        "users.search" => {
            let users = body.as_array().unwrap().len() as u64;
            if users == 0 {
                return None;
            }
            input["startAt"] = json!(input["startAt"].as_u64().unwrap() + users);
        }
        // `isLast: true`.
        "issue.changelog" => {
            if body["isLast"] == json!(true) {
                return None;
            }
            let start = body["startAt"].as_u64().unwrap();
            input["startAt"] = json!(start + body["values"].as_array().unwrap().len() as u64);
        }
        other => panic!("`{other}` is not a list"),
    }
    Some(input)
}

#[test]
fn each_list_walks_two_pages_and_stops_at_its_documented_end_condition() {
    let provider = Provider::new();
    let mut child = Child::spawn(&provider.selection()).unwrap();
    for (operation, first, expected) in first_pages() {
        let before = provider.requests().len();
        let items_key = match operation {
            "issues.search" => "issues",
            "issue.comments" => "comments",
            "issue.create_meta" => "issueTypes",
            _ => "values",
        };
        let mut input = first;
        let mut items = Vec::new();
        let mut pages = 0;
        loop {
            let body = invoke(&mut child, operation, input.clone())["body"].clone();
            pages += 1;
            items.extend(body[items_key].as_array().unwrap().clone());
            match next(operation, &input, &body) {
                Some(following) => input = following,
                None => break,
            }
            assert!(pages < 3, "`{operation}` did not stop");
        }
        assert_eq!(pages, 2, "`{operation}` pages walked");
        assert_eq!(items.len(), 3, "`{operation}` items");
        let second = match operation {
            "issues.search" => expected.replace(
                "&maxResults=2",
                "&nextPageToken=fixture-page-2&maxResults=2",
            ),
            _ => expected.replace("startAt=0", "startAt=2"),
        };
        let requests: Vec<String> = provider.requests()[before..]
            .iter()
            .map(|(path, _)| path.clone())
            .collect();
        assert_eq!(
            requests,
            [expected.to_owned(), second],
            "`{operation}` requests"
        );
    }
}

#[test]
fn issue_get_reads_one_issue_by_key_with_its_fields_expansions_and_attachments() {
    let provider = Provider::new();
    let mut child = Child::spawn(&provider.selection()).unwrap();
    // `fields` as one comma-separated string and `expand` as Jira's own
    // comma-separated list, the shape `jira.issue.show` sends.
    for (input, expected) in [
        (
            json!({"issueIdOrKey": "FIX-1", "fields": "summary,status,description,attachment",
                   "expand": "renderedFields,names"}),
            "/rest/api/3/issue/FIX-1?fields=summary%2Cstatus%2Cdescription%2Cattachment\
             &expand=renderedFields%2Cnames",
        ),
        // `fields` as a JSON array: one pair per element, in the order given.
        (
            json!({"issueIdOrKey": "FIX-1", "fields": ["summary", "attachment"]}),
            "/rest/api/3/issue/FIX-1?fields=summary&fields=attachment",
        ),
    ] {
        let before = provider.requests().len();
        let result = invoke(&mut child, "issue.get", input);
        let requests = provider.requests();
        assert_eq!(requests.len(), before + 1);
        assert_eq!(requests[before].0, expected);
        assert_eq!(requests[before].1.as_deref(), Some(HEADER));
        assert_eq!(result["status"], 200);
        assert_eq!(Some(&result["body"]), page(expected).as_ref());
        assert_eq!(result["provenance"]["instance"], "fixture-jira");
    }
    let body = invoke(&mut child, "issue.get", json!({"issueIdOrKey": "FIX-1"}))["body"].clone();
    assert_eq!(body["key"], "FIX-1");
    assert_eq!(body["fields"]["status"]["name"], "In Progress");
    // The description stays ADF; the rendered HTML comes with the expansion.
    assert_eq!(body["fields"]["description"]["type"], "doc");
    assert_eq!(
        body["renderedFields"]["description"],
        "<p>fixture description</p>"
    );
    // Attachment metadata, what `jira.issue.attachment.list` lists.
    assert_eq!(body["fields"]["attachment"][0]["filename"], "fixture.txt");
}

/// Jira answers `404` for an issue that does not exist or that the account
/// may not see; the read refuses with the provider's not-found, after exactly
/// one request to the declared path (no query, so the path ends in `?`).
#[test]
fn an_issue_jira_does_not_find_is_refused_as_provider_not_found() {
    let provider = Provider::new();
    let mut child = Child::spawn(&provider.selection()).unwrap();
    let revision = child.bootstrap().descriptor().unwrap().revision;
    let outcome = child.invoke(
        "issue.get",
        &revision,
        "one",
        &secret(),
        &serde_json::to_vec(&json!({"issueIdOrKey": "FIX-404"})).unwrap(),
        connectors_sdk::now_ms() + 30_000,
    );
    assert!(
        matches!(outcome, Err(Failure::ProviderNotFound)),
        "{:?}",
        outcome.map(|_| "an answer")
    );
    let requests = provider.requests();
    assert_eq!(requests.len(), 1);
    assert_eq!(requests[0].0, "/rest/api/3/issue/FIX-404?");
}

#[test]
fn users_search_sends_the_query_and_walks_to_an_empty_page() {
    let provider = Provider::new();
    let mut child = Child::spawn(&provider.selection()).unwrap();
    let mut input = json!({"query": "fixture", "startAt": 0, "maxResults": 2});
    let mut users = Vec::new();
    let mut pages = 0;
    loop {
        let result = invoke(&mut child, "users.search", input.clone());
        assert_eq!(result["status"], 200);
        let body = result["body"].clone();
        pages += 1;
        users.extend(
            body.as_array()
                .unwrap()
                .iter()
                .map(|u| u["accountId"].as_str().unwrap().to_owned()),
        );
        match next("users.search", &input, &body) {
            Some(following) => input = following,
            None => break,
        }
        assert!(pages < 4, "`users.search` did not stop");
    }
    assert_eq!(pages, 3);
    assert_eq!(
        users,
        ["fixture-user-1", "fixture-user-2", "fixture-user-3"]
    );
    let requests = provider.requests();
    assert_eq!(
        requests.iter().map(|(p, _)| p.as_str()).collect::<Vec<_>>(),
        [
            "/rest/api/3/user/search?query=fixture&startAt=0&maxResults=2",
            "/rest/api/3/user/search?query=fixture&startAt=2&maxResults=2",
            "/rest/api/3/user/search?query=fixture&startAt=3&maxResults=2",
        ]
    );
    assert!(requests.iter().all(|(_, a)| a.as_deref() == Some(HEADER)));
}

#[test]
fn issue_create_meta_lists_the_issue_types_a_project_can_create() {
    let provider = Provider::new();
    let mut child = Child::spawn(&provider.selection()).unwrap();
    let result = invoke(
        &mut child,
        "issue.create_meta",
        json!({"projectIdOrKey": "FIX"}),
    );
    assert_eq!(
        provider.requests().last().unwrap().0,
        "/rest/api/3/issue/createmeta/FIX/issuetypes?"
    );
    let names: Vec<&str> = result["body"]["issueTypes"]
        .as_array()
        .unwrap()
        .iter()
        .map(|t| t["name"].as_str().unwrap())
        .collect();
    assert_eq!(names, ["Task", "Story"]);
}

/// `issue.transitions` reads the transitions Jira offers the account on one
/// issue, each with the status it leads to in `to`: one request to the pinned
/// path, the basic header, and Jira's `Transitions` body unchanged.
#[test]
fn issue_transitions_lists_the_transitions_on_one_issue_with_their_target_status() {
    let provider = Provider::new();
    let mut child = Child::spawn(&provider.selection()).unwrap();
    let result = invoke(
        &mut child,
        "issue.transitions",
        json!({"issueIdOrKey": "FIX-1"}),
    );
    let requests = provider.requests();
    assert_eq!(requests.len(), 1);
    assert_eq!(requests[0].0, "/rest/api/3/issue/FIX-1/transitions?");
    assert_eq!(requests[0].1.as_deref(), Some(HEADER));
    assert_eq!(result["status"], 200);
    assert_eq!(
        Some(&result["body"]),
        page("/rest/api/3/issue/FIX-1/transitions?").as_ref()
    );
    assert_eq!(result["provenance"]["instance"], "fixture-jira");
    let offered: Vec<(&str, &str, &str)> = result["body"]["transitions"]
        .as_array()
        .unwrap()
        .iter()
        .map(|t| {
            (
                t["id"].as_str().unwrap(),
                t["name"].as_str().unwrap(),
                t["to"]["name"].as_str().unwrap(),
            )
        })
        .collect();
    assert_eq!(offered, [("11", "To Do", "To Do"), ("31", "Done", "Done")]);
}

/// The optional parameters travel by name, in the pinned document's order:
/// `expand`, `transitionId`, `includeUnavailableTransitions` and
/// `sortByOpsBarAndStatus` (`skipRemoteOnlyCondition` is in the next case).
#[test]
fn issue_transitions_sends_each_kept_parameter_by_name() {
    let provider = Provider::new();
    let mut child = Child::spawn(&provider.selection()).unwrap();
    for (input, expected) in [
        (
            json!({"issueIdOrKey": "FIX-1", "transitionId": "31"}),
            "/rest/api/3/issue/FIX-1/transitions?transitionId=31",
        ),
        (
            json!({"issueIdOrKey": "FIX-1", "expand": "transitions.fields"}),
            "/rest/api/3/issue/FIX-1/transitions?expand=transitions.fields",
        ),
        (
            json!({"issueIdOrKey": "FIX-1", "includeUnavailableTransitions": true}),
            "/rest/api/3/issue/FIX-1/transitions?includeUnavailableTransitions=true",
        ),
        (
            json!({"issueIdOrKey": "FIX-1", "sortByOpsBarAndStatus": true,
                   "includeUnavailableTransitions": false, "transitionId": "31",
                   "expand": "transitions.fields"}),
            "/rest/api/3/issue/FIX-1/transitions?expand=transitions.fields&transitionId=31\
             &includeUnavailableTransitions=false&sortByOpsBarAndStatus=true",
        ),
    ] {
        let before = provider.requests().len();
        let result = invoke(&mut child, "issue.transitions", input);
        let requests = provider.requests();
        assert_eq!(requests.len(), before + 1);
        assert_eq!(requests[before].0, expected);
        assert_eq!(Some(&result["body"]), page(expected).as_ref());
    }
    // One transition by id, the screen's fields, and one that fails a condition.
    let one = invoke(
        &mut child,
        "issue.transitions",
        json!({"issueIdOrKey": "FIX-1", "transitionId": "31"}),
    );
    assert_eq!(one["body"]["transitions"][0]["to"]["id"], "10002");
    assert_eq!(one["body"]["transitions"].as_array().unwrap().len(), 1);
    let fields = invoke(
        &mut child,
        "issue.transitions",
        json!({"issueIdOrKey": "FIX-1", "expand": "transitions.fields"}),
    );
    assert_eq!(
        fields["body"]["transitions"][0]["fields"]["resolution"]["required"],
        true
    );
    let all = invoke(
        &mut child,
        "issue.transitions",
        json!({"issueIdOrKey": "FIX-1", "includeUnavailableTransitions": true}),
    );
    assert_eq!(all["body"]["transitions"][2]["isAvailable"], false);
}

/// The selection keeps every parameter the pinned document declares, and
/// nothing more: `skipRemoteOnlyCondition` too, which the document gives effect
/// only for Connect and Forge apps with *Administer Jira*; Jira, not the
/// provider, decides whether it applies to an account connection. A parameter
/// the document does not declare for `getTransitions`, such as `startAt`, is
/// refused before any request.
#[test]
fn issue_transitions_declares_the_pinned_parameters_and_refuses_any_other_before_a_request() {
    let selections = shipped();
    let selection = selections
        .iter()
        .find(|s| s.id == "issue.transitions")
        .unwrap();
    assert!(selection.withhold.is_empty() && selection.guard.is_none());
    let bundle = bundle::load(&root().join("generated/bundles"), "jira").unwrap();
    let engine = Engine::new(&bundle, BASE, &selections).unwrap();
    let declaration = engine
        .declarations(&[Effect::Read])
        .into_iter()
        .find(|o| o.id == "issue.transitions")
        .unwrap();
    let mut properties: Vec<String> = declaration.input_schema["properties"]
        .as_object()
        .unwrap()
        .keys()
        .cloned()
        .collect();
    properties.sort();
    assert_eq!(
        properties,
        [
            "expand",
            "includeUnavailableTransitions",
            "issueIdOrKey",
            "skipRemoteOnlyCondition",
            "sortByOpsBarAndStatus",
            "transitionId"
        ]
    );
    assert_eq!(
        declaration.input_schema["required"],
        json!(["issueIdOrKey"])
    );
    let provider = Provider::new();
    let mut child = Child::spawn(&provider.selection()).unwrap();
    let revision = child.bootstrap().descriptor().unwrap().revision;
    for input in [
        json!({"issueIdOrKey": "FIX-1", "startAt": 0}),
        json!({"transitionId": "31"}),
        json!({"issueIdOrKey": "FIX-1", "includeUnavailableTransitions": "yes"}),
    ] {
        let outcome = child.invoke(
            "issue.transitions",
            &revision,
            "one",
            &secret(),
            &serde_json::to_vec(&input).unwrap(),
            connectors_sdk::now_ms() + 30_000,
        );
        assert!(
            matches!(outcome, Err(Failure::InvalidInput)),
            "{input}: {:?}",
            outcome.map(|_| "an answer")
        );
    }
    assert!(provider.requests().is_empty());
    invoke(
        &mut child,
        "issue.transitions",
        json!({"issueIdOrKey": "FIX-1", "skipRemoteOnlyCondition": true}),
    );
    assert_eq!(
        provider.requests()[0].0,
        "/rest/api/3/issue/FIX-1/transitions?skipRemoteOnlyCondition=true"
    );
}

/// Jira answers `404` when the issue does not exist or the account may not
/// see it; the read is refused as the provider's not-found after one request.
#[test]
fn transitions_of_an_issue_jira_does_not_find_are_refused_as_provider_not_found() {
    let provider = Provider::new();
    let mut child = Child::spawn(&provider.selection()).unwrap();
    let revision = child.bootstrap().descriptor().unwrap().revision;
    let outcome = child.invoke(
        "issue.transitions",
        &revision,
        "one",
        &secret(),
        &serde_json::to_vec(&json!({"issueIdOrKey": "FIX-404"})).unwrap(),
        connectors_sdk::now_ms() + 30_000,
    );
    assert!(
        matches!(outcome, Err(Failure::ProviderNotFound)),
        "{:?}",
        outcome.map(|_| "an answer")
    );
    let requests = provider.requests();
    assert_eq!(requests.len(), 1);
    assert_eq!(requests[0].0, "/rest/api/3/issue/FIX-404/transitions?");
}
