//! Zendesk Support tickets, comments, users and organizations through the
//! catalog provider.
//!
//! The shipped selection set is pinned by id and source operation, resolves
//! against the committed bundle compiled from the pinned Support API document,
//! and is cited row by row in `docs/catalog-zendesk.md`. Each read runs through
//! the provider child against a disposable HTTPS fixture: the exact request
//! (path, query with its `start_time` filter, `Authorization: Basic …`) and the
//! returned body bytes are asserted, every list walks two pages to the end
//! condition the guide documents, and the current-user identity read yields the
//! user id. Every record is synthetic. No live credential and no network.
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

/// The document's server is `https://{subdomain}.{domain}.com` with no path,
/// so every operation and the identity read sit below the authority's root.
const BASE: &str = "/";
/// Fictional basic material, and the header the fixture accepts:
/// `Basic base64("reader@example.com/token:fixture-zendesk-token")`, computed
/// outside the provider with coreutils `base64`.
const ACCOUNT: &str = "reader@example.com/token";
const TOKEN: &str = "fixture-zendesk-token";
const HEADER: &str = "Basic cmVhZGVyQGV4YW1wbGUuY29tL3Rva2VuOmZpeHR1cmUtemVuZGVzay10b2tlbg==";
const PROFILE: &str = "zendesk.basic";
/// The pinned source, its manifest and README, relative to this crate.
const UPSTREAM: &str = "../zendesk/upstream";
const SOURCE: &str = "zendesk-support.yaml";
const URL: &str = "https://developer.zendesk.com/zendesk/oas.yaml";
/// The digest of the bytes Zendesk serves, before redaction.
const UPSTREAM_SHA256: &str = "3a477ea89b274f4d3731f1c7ff93dc93d4520de871ac759b06d3297798fd685d";
/// A synthetic `start_time` (2025-09-16T05:20:00Z) and the `end_time` of the
/// first organization page.
const SINCE: u64 = 1_758_000_000;
const ORGANIZATION_END: u64 = 1_758_003_600;

/// The shipped ids, their pinned `operationId` and their pinned path. A
/// renamed, dropped or added id fails here.
const SHIPPED: [(&str, &str, &str); 7] = [
    (
        "organization.show",
        "ShowOrganization",
        "/api/v2/organizations/{organization_id}",
    ),
    (
        "organizations.incremental",
        "IncrementalOrganizationExport",
        "/api/v2/incremental/organizations",
    ),
    (
        "ticket.comments",
        "ListTicketComments",
        "/api/v2/tickets/{ticket_id}/comments",
    ),
    ("ticket.show", "ShowTicket", "/api/v2/tickets/{ticket_id}"),
    (
        "tickets.incremental",
        "IncrementalTicketExportCursor",
        "/api/v2/incremental/tickets/cursor",
    ),
    ("user.show", "ShowUser", "/api/v2/users/{user_id}"),
    (
        "users.incremental",
        "IncrementalUserExportCursor",
        "/api/v2/incremental/users/cursor",
    ),
];

fn root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
}
fn shipped() -> Vec<Selection> {
    let file: Value = serde_json::from_slice(
        &fs::read(root().join("providers/zendesk/operations.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(file["format"], "connectors-catalog-operations/1");
    assert_eq!(file["provider"], "zendesk");
    serde_json::from_value(file["operations"].clone()).unwrap()
}
fn committed() -> bundle::Bundle {
    bundle::load(&root().join("generated/bundles"), "zendesk").unwrap()
}

#[test]
fn shipped_zendesk_selections_are_exactly_the_seven_reads() {
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
    let expected: Vec<&str> = SHIPPED.iter().map(|(id, _, _)| *id).collect();
    assert_eq!(declared, expected);
    assert!(engine.declarations(&[Effect::Write]).is_empty());
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
fn a_selection_the_pinned_source_lacks_or_a_write_is_refused_at_load() {
    let bundle = committed();
    let mut selections = shipped();
    // The pinned document has no cursor export for organizations.
    assert!(
        bundle
            .inventory
            .operations
            .iter()
            .all(|o| o.operation_id.as_deref() != Some("IncrementalOrganizationExportCursor"))
    );
    selections[0].operation_id = "IncrementalOrganizationExportCursor".into();
    assert!(Engine::new(&bundle, BASE, &selections).is_err());
    // A PUT cannot ship as a read.
    selections[0].operation_id = "UpdateTicket".into();
    assert!(Engine::new(&bundle, BASE, &selections).is_err());
}

#[test]
fn the_pinned_source_is_the_one_the_bundle_and_its_record_name() {
    let upstream = root().join(UPSTREAM);
    let bytes = fs::read(upstream.join(SOURCE)).unwrap();
    let digest = hex::encode(Sha256::digest(&bytes));
    let index = bundle::read_index(&root().join("generated/bundles")).unwrap();
    let entry = index.find("zendesk").unwrap();
    assert_eq!(entry.source_sha256, digest);
    assert_eq!(entry.operations, 652);
    assert_eq!(entry.unsupported, 35);
    let bundle = committed();
    assert_eq!(bundle.source.source_bytes, bytes.len());
    assert_eq!(bundle.source.file_name, SOURCE);
    let manifest: Value =
        serde_json::from_slice(&fs::read(upstream.join("zendesk-source-hashes.json")).unwrap())
            .unwrap();
    let records = manifest.as_array().unwrap();
    assert_eq!(records.len(), 1, "one pinned Zendesk document");
    assert_eq!(records[0]["file"], SOURCE);
    assert_eq!(records[0]["url"], URL);
    assert_eq!(records[0]["sha256"], digest.as_str());
    assert_eq!(records[0]["bytes"], bytes.len());
    // The committed file is the redacted copy of the upstream bytes, which are
    // not committed; the record names the rule and the upstream digest.
    assert_eq!(records[0]["redaction"], "upstream-redaction/2");
    assert_eq!(records[0]["upstream_sha256"], UPSTREAM_SHA256);
    assert_ne!(digest, UPSTREAM_SHA256);
    assert!(
        !upstream.join("vendor").exists(),
        "no archive of the upstream bytes"
    );
    let readme = fs::read_to_string(upstream.join("README.md")).unwrap();
    assert!(readme.contains(&digest), "README does not name the digest");
    assert!(
        readme.contains(UPSTREAM_SHA256),
        "README does not name the upstream digest"
    );
    assert!(
        readme.contains("upstream-redaction/2"),
        "README does not name the rule"
    );
    assert!(readme.contains(URL), "README does not name the source URL");
}

/// The parameters each shipped read declares, in the bundle's order, after
/// the selection's withholding. Path ids are integers; the comment cursor is
/// the `deepObject` `page` read as `page[...]` pairs; offset paging on the
/// comments is withheld.
#[test]
fn each_read_declares_the_pinned_parameters_and_the_comment_cursor_pairs() {
    let bundle = committed();
    let names = |operation_id: &str| -> Vec<String> {
        bundle
            .inventory
            .operations
            .iter()
            .find(|o| o.operation_id.as_deref() == Some(operation_id))
            .unwrap()
            .parameters
            .iter()
            .map(|p| p.name.clone())
            .collect()
    };
    assert_eq!(
        names("IncrementalTicketExportCursor"),
        ["start_time", "cursor", "support_type_scope"]
    );
    assert_eq!(
        names("IncrementalUserExportCursor"),
        ["start_time", "cursor", "per_page"]
    );
    assert_eq!(
        names("IncrementalOrganizationExport"),
        ["start_time", "per_page"]
    );
    assert_eq!(
        names("ListTicketComments"),
        [
            "ticket_id",
            "include_inline_images",
            "include",
            "per_page",
            "sort_order",
            "page[after]",
            "page[before]",
            "page[size]"
        ]
    );
    let organizations = bundle
        .inventory
        .operations
        .iter()
        .find(|o| o.operation_id.as_deref() == Some("IncrementalOrganizationExport"))
        .unwrap();
    assert!(organizations.parameters[0].required, "start_time");
    let comments = shipped()
        .into_iter()
        .find(|s| s.id == "ticket.comments")
        .unwrap();
    assert_eq!(comments.withhold, ["per_page", "sort_order"]);
}

#[test]
fn guide_cites_each_operation_its_paging_and_its_deltas() {
    let guide = fs::read_to_string(root().join("../../docs/catalog-zendesk.md")).unwrap();
    let rows: Vec<&str> = guide.lines().filter(|l| l.starts_with('|')).collect();
    // Per operation: id, pinned operationId and path, paging parameters, end
    // condition, and the time filter or how deltas are taken.
    for (id, operation_id, path, paging, end, deltas) in [
        (
            "tickets.incremental",
            "IncrementalTicketExportCursor",
            "/api/v2/incremental/tickets/cursor",
            "`start_time` on the first call, then `cursor`",
            "`end_of_stream: true`",
            "`start_time`, Unix seconds",
        ),
        (
            "ticket.show",
            "ShowTicket",
            "/api/v2/tickets/{ticket_id}",
            "single item",
            "n/a",
            "none",
        ),
        (
            "ticket.comments",
            "ListTicketComments",
            "/api/v2/tickets/{ticket_id}/comments",
            "`page[size]`, `page[after]`",
            "`meta.has_more: false`",
            "`tickets.incremental`",
        ),
        (
            "users.incremental",
            "IncrementalUserExportCursor",
            "/api/v2/incremental/users/cursor",
            "`start_time` on the first call, then `cursor`",
            "`end_of_stream: true`",
            "`start_time`, Unix seconds",
        ),
        (
            "user.show",
            "ShowUser",
            "/api/v2/users/{user_id}",
            "single item",
            "n/a",
            "none",
        ),
        (
            "organizations.incremental",
            "IncrementalOrganizationExport",
            "/api/v2/incremental/organizations",
            "the previous page's `end_time` as `start_time`",
            "`end_of_stream: true`",
            "`start_time`, Unix seconds",
        ),
        (
            "organization.show",
            "ShowOrganization",
            "/api/v2/organizations/{organization_id}",
            "single item",
            "n/a",
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

/// The `auth` profile of the configuration example in the Zendesk guide, so the
/// fixture runs the profile the guide tells a reader to write.
fn documented_auth() -> Value {
    let guide = fs::read_to_string(root().join("../../docs/catalog-zendesk.md")).unwrap();
    let example = guide
        .split("```json\n")
        .skip(1)
        .filter_map(|rest| rest.split_once("\n```").map(|(body, _)| body))
        .find(|body| body.contains("\"provider\": \"zendesk\""))
        .expect("the documented Zendesk configuration");
    serde_json::from_str::<Value>(example).unwrap()["auth"].clone()
}

#[test]
fn the_documented_profile_is_basic_with_the_current_user_as_identity() {
    let auth = documented_auth();
    assert_eq!(auth["profile"], PROFILE);
    assert_eq!(auth["scheme"], "basic");
    assert_eq!(auth["header"], "Authorization");
    assert_eq!(auth["bearer"], false);
    assert_eq!(auth["identity"]["path"], "api/v2/users/me");
    assert_eq!(auth["identity"]["kind"], "zendesk.user");
    assert_eq!(auth["identity"]["subject_pointer"], "/user/id");
    assert_eq!(auth["scopes"], Value::Null);
    // The identity read is the pinned document's current-user read.
    assert!(committed().inventory.operations.iter().any(|o| {
        o.operation_id.as_deref() == Some("ShowCurrentUser") && o.path == "/api/v2/users/me"
    }));
}

/// Route and `Authorization` header of each fixture request.
type Requests = Arc<Mutex<Vec<(String, Option<String>)>>>;

fn ticket(id: u64) -> Value {
    json!({"id": id, "url": format!("https://fixture.example.com/api/v2/tickets/{id}.json"),
           "subject": format!("fixture ticket {id}"), "status": "open",
           "requester_id": 201, "submitter_id": 201, "assignee_id": 200,
           "organization_id": 301, "created_at": "2025-09-15T08:00:00Z",
           "updated_at": "2025-09-16T10:00:00Z"})
}
fn user(id: u64) -> Value {
    json!({"id": id, "url": format!("https://fixture.example.com/api/v2/users/{id}.json"),
           "name": format!("Fixture User {id}"), "email": format!("user{id}@example.com"),
           "role": "end-user", "organization_id": 301,
           "created_at": "2025-09-01T08:00:00Z", "updated_at": "2025-09-16T10:00:00Z"})
}
fn organization(id: u64) -> Value {
    json!({"id": id, "url": format!("https://fixture.example.com/api/v2/organizations/{id}.json"),
           "name": format!("Fixture Organization {id}"), "domain_names": ["example.com"],
           "created_at": "2025-09-01T08:00:00Z", "updated_at": "2025-09-16T10:00:00Z"})
}
fn comment(id: u64) -> Value {
    json!({"id": id, "type": "Comment", "author_id": 201, "public": true,
           "body": format!("fixture comment {id}"), "created_at": "2025-09-15T09:00:00Z"})
}

/// The recorded bodies the fixture serves, keyed by route and paging position.
/// Each list has three items over two pages; `None` for anything else, which
/// the fixture answers 404.
fn page(path: &str) -> Option<Value> {
    let (route, query) = path.split_once('?').unwrap_or((path, ""));
    let has = |pair: &str| query.split('&').any(|p| p == pair);
    let since = format!("start_time={SINCE}");
    let after_organizations = format!("start_time={ORGANIZATION_END}");
    Some(match route {
        "/api/v2/users/me" => json!({"user": {"id": 200, "name": "Fixture Reader",
                                              "email": "reader@example.com", "role": "admin"}}),
        "/api/v2/incremental/tickets/cursor" if has("cursor=fixture-ticket-cursor-2") => json!({
            "tickets": [ticket(103)], "after_cursor": "fixture-ticket-cursor-3",
            "after_url": "https://fixture.example.com/api/v2/incremental/tickets/cursor?cursor=fixture-ticket-cursor-3",
            "before_cursor": "fixture-ticket-cursor-2", "before_url": null, "end_of_stream": true}),
        "/api/v2/incremental/tickets/cursor" if has(&since) => json!({
            "tickets": [ticket(101), ticket(102)], "after_cursor": "fixture-ticket-cursor-2",
            "after_url": "https://fixture.example.com/api/v2/incremental/tickets/cursor?cursor=fixture-ticket-cursor-2",
            "before_cursor": null, "before_url": null, "end_of_stream": false}),
        "/api/v2/tickets/101" => json!({"ticket": ticket(101)}),
        "/api/v2/tickets/101/comments" if has("page%5Bafter%5D=fixture-comment-cursor-2") => {
            json!({
            "comments": [comment(3)],
            "meta": {"has_more": false, "after_cursor": "fixture-comment-cursor-3",
                     "before_cursor": "fixture-comment-cursor-2"},
            "links": {"next": null, "prev": null}})
        }
        "/api/v2/tickets/101/comments" => json!({
            "comments": [comment(1), comment(2)],
            "meta": {"has_more": true, "after_cursor": "fixture-comment-cursor-2",
                     "before_cursor": "fixture-comment-cursor-1"},
            "links": {"next": "https://fixture.example.com/api/v2/tickets/101/comments?page%5Bafter%5D=fixture-comment-cursor-2&page%5Bsize%5D=2",
                      "prev": null}}),
        "/api/v2/incremental/users/cursor" if has("cursor=fixture-user-cursor-2") => json!({
            "users": [user(203)], "after_cursor": "fixture-user-cursor-3",
            "after_url": "https://fixture.example.com/api/v2/incremental/users/cursor?cursor=fixture-user-cursor-3",
            "before_cursor": "fixture-user-cursor-2", "before_url": null, "end_of_stream": true}),
        "/api/v2/incremental/users/cursor" if has(&since) => json!({
            "users": [user(201), user(202)], "after_cursor": "fixture-user-cursor-2",
            "after_url": "https://fixture.example.com/api/v2/incremental/users/cursor?cursor=fixture-user-cursor-2",
            "before_cursor": null, "before_url": null, "end_of_stream": false}),
        "/api/v2/users/201" => json!({"user": user(201)}),
        "/api/v2/incremental/organizations" if has(&after_organizations) => json!({
            "organizations": [organization(303)], "count": 1, "end_of_stream": true,
            "end_time": 1_758_007_200u64,
            "next_page": "https://fixture.example.com/api/v2/incremental/organizations?start_time=1758007200"}),
        "/api/v2/incremental/organizations" if has(&since) => json!({
            "organizations": [organization(301), organization(302)], "count": 2,
            "end_of_stream": false, "end_time": ORGANIZATION_END,
            "next_page": format!("https://fixture.example.com/api/v2/incremental/organizations?start_time={ORGANIZATION_END}")}),
        "/api/v2/organizations/301" => json!({"organization": organization(301)}),
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
                        (401, json!({"error": "Couldn't authenticate you"}))
                    } else {
                        match page(&path) {
                            Some(body) => (200, body),
                            None => (404, json!({"error": "RecordNotFound"})),
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
                "instance": "fixture-zendesk",
                "provider": "zendesk",
                "bundle_directory": root_path("generated/bundles"),
                "api_base": format!("https://localhost:{}", address.port()),
                "ca_file": ca,
                "auth": documented_auth(),
                "operations_file": root_path("providers/zendesk/operations.json"),
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
            instance_id: "fixture-zendesk".into(),
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
fn deadline() -> u64 {
    connectors_sdk::now_ms() + 30_000
}
/// The provider's raw output for one invocation.
fn invoke_raw(child: &mut Child, operation: &str, input: &Value) -> Vec<u8> {
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
        .unwrap_or_else(|failure| panic!("`{operation}` failed: {failure:?}"))
}
fn invoke(child: &mut Child, operation: &str, input: &Value) -> Value {
    serde_json::from_slice(&invoke_raw(child, operation, input)).unwrap()
}

#[test]
fn connecting_records_the_current_user_as_the_identity() {
    let provider = Provider::new();
    let mut child = Child::spawn(&provider.selection()).unwrap();
    let baseline = child.validate(PROFILE, &secret(), deadline()).unwrap();
    assert_eq!(baseline.identity.kind, "zendesk.user");
    assert_eq!(baseline.identity.subject, "200");
    assert_eq!(baseline.granted_scopes, None);
    let requests = provider.requests();
    assert_eq!(requests.len(), 1);
    // The identity probe carries an empty query.
    assert_eq!(requests[0].0, "/api/v2/users/me?");
    assert_eq!(requests[0].1.as_deref(), Some(HEADER));
    // A wrong token is answered 401 and refuses the connection.
    let wrong =
        Secret(serde_json::to_vec(&json!({"account": ACCOUNT, "token": "wrong-token"})).unwrap());
    assert!(matches!(
        child.validate(PROFILE, &wrong, deadline()),
        Err(Failure::InvalidCredential)
    ));
    assert_eq!(
        provider.requests()[1].1.as_deref(),
        Some("Basic cmVhZGVyQGV4YW1wbGUuY29tL3Rva2VuOndyb25nLXRva2Vu")
    );
    // A token-only credential document is not this profile's entry.
    let token_only = Secret(br#"{"token":"t"}"#.to_vec());
    assert!(matches!(
        child.validate(PROFILE, &token_only, deadline()),
        Err(Failure::InvalidInput)
    ));
}

/// Each read's first request: the input, and the exact request the fixture
/// must observe, including the `start_time` filter on the exports.
fn first_pages() -> [(&'static str, Value, String); 7] {
    [
        (
            "tickets.incremental",
            json!({"start_time": SINCE, "support_type_scope": "all"}),
            format!("/api/v2/incremental/tickets/cursor?start_time={SINCE}&support_type_scope=all"),
        ),
        (
            "ticket.show",
            json!({"ticket_id": 101}),
            "/api/v2/tickets/101?".into(),
        ),
        (
            "ticket.comments",
            json!({"ticket_id": 101, "page[size]": 2}),
            "/api/v2/tickets/101/comments?page%5Bsize%5D=2".into(),
        ),
        (
            "users.incremental",
            json!({"start_time": SINCE, "per_page": 2}),
            format!("/api/v2/incremental/users/cursor?start_time={SINCE}&per_page=2"),
        ),
        (
            "user.show",
            json!({"user_id": 201}),
            "/api/v2/users/201?".into(),
        ),
        (
            "organizations.incremental",
            json!({"start_time": SINCE, "per_page": 2}),
            format!("/api/v2/incremental/organizations?start_time={SINCE}&per_page=2"),
        ),
        (
            "organization.show",
            json!({"organization_id": 301}),
            "/api/v2/organizations/301?".into(),
        ),
    ]
}

#[test]
fn each_read_sends_the_declared_request_with_basic_auth_and_returns_the_recorded_body() {
    let provider = Provider::new();
    let mut child = Child::spawn(&provider.selection()).unwrap();
    for (operation, input, expected) in first_pages() {
        let before = provider.requests().len();
        let raw = invoke_raw(&mut child, operation, &input);
        let requests = provider.requests();
        assert_eq!(requests.len(), before + 1, "`{operation}` requests");
        assert_eq!(requests[before].0, expected, "`{operation}` request");
        assert_eq!(
            requests[before].1.as_deref(),
            Some(HEADER),
            "`{operation}` authorization"
        );
        let result: Value = serde_json::from_slice(&raw).unwrap();
        assert_eq!(result["status"], 200, "`{operation}` status");
        assert_eq!(result["provenance"]["instance"], "fixture-zendesk");
        // The bytes the fixture served appear unchanged in the provider's
        // output, as the value of `body`.
        let served = serde_json::to_vec(&page(&expected).unwrap()).unwrap();
        assert_eq!(
            serde_json::to_vec(&result["body"]).unwrap(),
            served,
            "`{operation}` body"
        );
        let mut field = b"\"body\":".to_vec();
        field.extend_from_slice(&served);
        assert!(
            raw.windows(field.len()).any(|window| window == field),
            "`{operation}` output does not carry the served body bytes"
        );
    }
}

#[test]
fn a_withheld_or_undeclared_parameter_is_refused_before_any_request() {
    let provider = Provider::new();
    let mut child = Child::spawn(&provider.selection()).unwrap();
    let revision = child.bootstrap().descriptor().unwrap().revision;
    for (operation, input) in [
        // Withheld offset paging on the comments.
        ("ticket.comments", json!({"ticket_id": 101, "per_page": 2})),
        (
            "ticket.comments",
            json!({"ticket_id": 101, "sort_order": "desc"}),
        ),
        // The pinned ticket export declares no `per_page`.
        (
            "tickets.incremental",
            json!({"start_time": SINCE, "per_page": 2}),
        ),
        // The organization export requires `start_time`.
        ("organizations.incremental", json!({"per_page": 2})),
    ] {
        let outcome = child.invoke(
            operation,
            &revision,
            "one",
            &secret(),
            &serde_json::to_vec(&input).unwrap(),
            deadline(),
        );
        assert!(
            matches!(outcome, Err(Failure::InvalidInput)),
            "`{operation}` {input}: {outcome:?}"
        );
    }
    assert!(provider.requests().is_empty());
}

/// The documented end condition of each list, read from the page body alone,
/// and the input for the next page when there is one.
fn next(operation: &str, input: &Value, body: &Value) -> Option<Value> {
    let mut input = input.clone();
    match operation {
        // `end_of_stream: true`; otherwise `after_cursor` as `cursor`, without
        // `start_time`.
        "tickets.incremental" | "users.incremental" => {
            if body["end_of_stream"] == json!(true) {
                return None;
            }
            let object = input.as_object_mut().unwrap();
            object.remove("start_time");
            object.insert("cursor".into(), body["after_cursor"].clone());
        }
        // `end_of_stream: true`; otherwise `end_time` as `start_time`.
        "organizations.incremental" => {
            if body["end_of_stream"] == json!(true) {
                return None;
            }
            input["start_time"] = body["end_time"].clone();
        }
        // `meta.has_more: false`; otherwise `meta.after_cursor` as `page[after]`.
        "ticket.comments" => {
            if body["meta"]["has_more"] == json!(false) {
                return None;
            }
            input["page[after]"] = body["meta"]["after_cursor"].clone();
        }
        other => panic!("`{other}` is not a list"),
    }
    Some(input)
}

#[test]
fn each_list_walks_two_pages_and_stops_at_its_documented_end_condition() {
    let provider = Provider::new();
    let mut child = Child::spawn(&provider.selection()).unwrap();
    for (operation, items_key, second) in [
        (
            "tickets.incremental",
            "tickets",
            "/api/v2/incremental/tickets/cursor?cursor=fixture-ticket-cursor-2&support_type_scope=all"
                .to_owned(),
        ),
        (
            "ticket.comments",
            "comments",
            "/api/v2/tickets/101/comments?page%5Bafter%5D=fixture-comment-cursor-2&page%5Bsize%5D=2"
                .to_owned(),
        ),
        (
            "users.incremental",
            "users",
            "/api/v2/incremental/users/cursor?cursor=fixture-user-cursor-2&per_page=2".to_owned(),
        ),
        (
            "organizations.incremental",
            "organizations",
            format!("/api/v2/incremental/organizations?start_time={ORGANIZATION_END}&per_page=2"),
        ),
    ] {
        let (_, first, expected) = first_pages()
            .into_iter()
            .find(|(id, _, _)| *id == operation)
            .unwrap();
        let before = provider.requests().len();
        let mut input = first;
        let mut items = Vec::new();
        let mut pages = 0;
        loop {
            let body = invoke(&mut child, operation, &input)["body"].clone();
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
        let requests: Vec<String> = provider.requests()[before..]
            .iter()
            .map(|(path, _)| path.clone())
            .collect();
        assert_eq!(requests, [expected, second], "`{operation}` requests");
    }
}
