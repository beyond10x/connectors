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
//!
//! story:catalog-zendesk-ticket-writes: the three ticket writes (`ticket.update`,
//! `ticket.note`, `ticket.draft_reply`, each `UpdateTicket`) run through a
//! private-protocol-two child, prepare and commit, the transport the host's
//! approval coordinator drives once it holds a verified, spent approval: the
//! preflight `ShowTicket`, then exactly one `PUT` whose body carries
//! `safe_update: true` and the caller's `updated_stamp`. Without an approval
//! there is no route to the request.
#![cfg(unix)]
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

/// The shipped writes, their pinned `operationId` and their pinned path: three
/// selections of the one `PUT` of a ticket.
const WRITES: [(&str, &str, &str); 3] = [
    (
        "ticket.draft_reply",
        "UpdateTicket",
        "/api/v2/tickets/{ticket_id}",
    ),
    ("ticket.note", "UpdateTicket", "/api/v2/tickets/{ticket_id}"),
    (
        "ticket.update",
        "UpdateTicket",
        "/api/v2/tickets/{ticket_id}",
    ),
];

/// The first line of every draft reply, fixed by the selection.
const DRAFT_MARKER: &str = "[Draft reply - not sent to the requester]\n";
/// The fixture ticket's `updated_at`, which a write's `updated_stamp` must
/// equal before it is sent.
const STAMP: &str = "2025-09-16T10:00:00Z";

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
fn shipped_zendesk_selections_are_the_seven_reads_and_three_writes() {
    let selections = shipped();
    let bundle = committed();
    assert_eq!(bundle.auth_profile, PROFILE);
    let engine = Engine::new(&bundle, BASE, &selections).unwrap();
    let ids = |effects: &[Effect]| {
        let mut declared: Vec<String> = engine
            .declarations(effects)
            .into_iter()
            .map(|o| o.id)
            .collect();
        declared.sort();
        declared
    };
    let expected: Vec<&str> = SHIPPED.iter().map(|(id, _, _)| *id).collect();
    assert_eq!(ids(&[Effect::Read]), expected);
    let expected: Vec<&str> = WRITES.iter().map(|(id, _, _)| *id).collect();
    assert_eq!(ids(&[Effect::Write]), expected);
    for (id, operation_id, path) in WRITES {
        let selection = selections.iter().find(|s| s.id == id).unwrap();
        assert_eq!(selection.operation_id, operation_id, "`{id}`");
        assert_eq!(selection.effect, Effect::Write, "`{id}`");
        assert_eq!(engine.effect(id), Some(Effect::Write), "`{id}`");
        let operation = bundle
            .inventory
            .operations
            .iter()
            .find(|o| o.operation_id.as_deref() == Some(operation_id))
            .unwrap();
        assert_eq!(operation.method, "put", "`{id}`");
        assert_eq!(operation.path, path, "`{id}`");
        // Every write is guarded by the ticket's `updated_at` and answers
        // Zendesk's 409 as a conflict.
        let guard = selection.guard.as_ref().expect("an unguarded write");
        assert_eq!(guard.preflight.operation_id, "ShowTicket", "`{id}`");
        assert_eq!(selection.conflict_status, Some(409), "`{id}`");
        assert_eq!(
            selection.body_fixed.get("/ticket/safe_update"),
            Some(&json!(true)),
            "`{id}`"
        );
    }
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
        // `per_page` is the one cited amendment, not in the pinned document.
        ["start_time", "cursor", "support_type_scope", "per_page"]
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

/// The `auth` profile `profile` the `zendesk` connector declares
/// (`providers/zendesk/connector.json`), with its `token_url` on the origin of
/// the endpoint the guide's instance names, so the fixture runs the profile
/// the guide's instance uses.
fn documented_auth(profile: &str) -> Value {
    let declaration: Value =
        serde_json::from_slice(&fs::read(root().join("providers/zendesk/connector.json")).unwrap())
            .unwrap();
    let mut auth = declaration["profiles"]
        .as_array()
        .unwrap()
        .iter()
        .find(|declared| declared["profile"] == profile)
        .expect("the declared Zendesk profile")
        .clone();
    if let Some(path) = auth["token_url"].as_str().map(str::to_owned) {
        auth["token_url"] = json!(format!("https://your-subdomain.zendesk.com{path}"));
    }
    auth
}

#[test]
fn the_documented_oauth_profile_uses_client_credentials_and_reads_only() {
    let auth = documented_auth("zendesk.oauth");
    assert_eq!(auth["scheme"], "oauth2_client_credentials");
    assert_eq!(auth["header"], "Authorization");
    assert_eq!(auth["bearer"], true);
    assert_eq!(
        auth["token_url"],
        "https://your-subdomain.zendesk.com/oauth/tokens"
    );
    assert_eq!(auth["requested_scopes"], json!(["read"]));
    assert_eq!(auth["authorize_url"], Value::Null);
    assert_eq!(auth["identity"], documented_auth(PROFILE)["identity"]);
}

#[test]
fn the_documented_profile_is_basic_with_the_current_user_as_identity() {
    let auth = documented_auth(PROFILE);
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

/// Route, `Authorization` header, method and body of each fixture request.
type Requests = Arc<Mutex<Vec<(String, Option<String>, String, Vec<u8>)>>>;

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
        "/api/v2/tickets/102" => json!({"ticket": ticket(102)}),
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

/// The fixture's answer to a ticket `PUT`, synthetic and shaped like the
/// pinned `TicketUpdateResponse`: ticket 101 applies the body's ticket members
/// and answers the updated ticket and an audit; ticket 102 changed after the
/// preflight read it, so the safe update is refused with 409.
fn updated(path: &str, body: &[u8]) -> (u16, Value) {
    let route = path.split('?').next().unwrap_or_default();
    let Ok(body) = serde_json::from_slice::<Value>(body) else {
        return (400, json!({"error": "InvalidJSON"}));
    };
    match route {
        "/api/v2/tickets/101" => {
            let mut ticket = ticket(101);
            for (key, value) in body["ticket"].as_object().into_iter().flatten() {
                if !["comment", "safe_update", "updated_stamp"].contains(&key.as_str()) {
                    ticket[key] = value.clone();
                }
            }
            ticket["updated_at"] = json!("2025-09-16T10:05:00Z");
            (
                200,
                json!({"ticket": ticket, "audit": {"id": 9001, "ticket_id": 101, "events": []}}),
            )
        }
        "/api/v2/tickets/102" => (
            409,
            json!({"error": "UpdateConflict",
                   "description": "Safe Update prevented the update due to outdated ticket data"}),
        ),
        _ => (404, json!({"error": "RecordNotFound"})),
    }
}

struct Provider {
    stop: Option<oneshot::Sender<()>>,
    thread: Option<std::thread::JoinHandle<()>>,
    _root: tempfile::TempDir,
    config: PathBuf,
    requests: Requests,
    protocol: Option<PrivateProtocol>,
}
impl Provider {
    fn new() -> Self {
        Self::speaking(None)
    }
    /// A provider on private protocol two, which carries writes.
    fn writing() -> Self {
        Self::speaking(Some(PrivateProtocol::V2))
    }
    fn speaking(protocol: Option<PrivateProtocol>) -> Self {
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
                    let mut words = request.split_whitespace();
                    let method = words.next().unwrap_or_default().to_owned();
                    let path = words.next().unwrap_or_default().to_owned();
                    let length: usize = request
                        .lines()
                        .find_map(|line| {
                            line.split_once(':')
                                .filter(|(name, _)| name.eq_ignore_ascii_case("content-length"))
                                .map(|(_, value)| value.trim().parse().unwrap())
                        })
                        .unwrap_or(0);
                    assert!(length <= 65_536);
                    let mut sent = vec![0; length];
                    if stream.read_exact(&mut sent).await.is_err() {
                        continue;
                    }
                    let authorization = request.lines().find_map(|line| {
                        line.split_once(':')
                            .filter(|(name, _)| name.eq_ignore_ascii_case("authorization"))
                            .map(|(_, value)| value.trim().to_owned())
                    });
                    // Only the fictional header is accepted; raw headers are
                    // compared, never printed.
                    let (status, body) = if authorization.as_deref() != Some(HEADER) {
                        (401, json!({"error": "Couldn't authenticate you"}))
                    } else if method == "PUT" {
                        updated(&path, &sent)
                    } else {
                        match page(&path) {
                            Some(body) => (200, body),
                            None => (404, json!({"error": "RecordNotFound"})),
                        }
                    };
                    observed
                        .lock()
                        .unwrap()
                        .push((path, authorization, method, sent));
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
                "auth": documented_auth(PROFILE),
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
            protocol,
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
            private_protocol: self.protocol,
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
        self.requests
            .lock()
            .unwrap()
            .iter()
            .map(|(path, authorization, _, _)| (path.clone(), authorization.clone()))
            .collect()
    }
    /// Every request as method, target, `Authorization` and body parsed as
    /// JSON (`null` when it carried none).
    fn sent(&self) -> Vec<(String, String, Option<String>, Value)> {
        self.requests
            .lock()
            .unwrap()
            .iter()
            .map(|(path, authorization, method, body)| {
                let body = if body.is_empty() {
                    Value::Null
                } else {
                    serde_json::from_slice(body).unwrap()
                };
                (method.clone(), path.clone(), authorization.clone(), body)
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
            json!({"start_time": SINCE, "support_type_scope": "all", "per_page": 2}),
            format!(
                "/api/v2/incremental/tickets/cursor?start_time={SINCE}&support_type_scope=all&per_page=2"
            ),
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
        assert_eq!(provider.sent()[before].0, "GET", "`{operation}` method");
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
        // The ticket export's `per_page` is bounded to Zendesk's 1 to 1,000.
        (
            "tickets.incremental",
            json!({"start_time": SINCE, "per_page": 0}),
        ),
        (
            "tickets.incremental",
            json!({"start_time": SINCE, "per_page": 1001}),
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
            "/api/v2/incremental/tickets/cursor?cursor=fixture-ticket-cursor-2&support_type_scope=all&per_page=2"
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

/// One call on the read transport, which carries no approval.
fn attempt(child: &mut Child, operation: &str, input: &Value) -> Result<Vec<u8>, Failure> {
    let revision = child.bootstrap().descriptor().unwrap().revision;
    child.invoke(
        operation,
        &revision,
        "one",
        &secret(),
        &serde_json::to_vec(input).unwrap(),
        deadline(),
    )
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

/// Each write's input on ticket `ticket_id`, and the exact body Zendesk must
/// receive for it: the caller's members, `safe_update: true`, and for a note
/// or a draft `public: false`, a draft's text behind its marker line.
fn writes(ticket_id: u64) -> [(&'static str, Value, Value); 3] {
    [
        (
            "ticket.update",
            json!({"ticket_id": ticket_id, "body": {"ticket": {
                "group_id": 7, "tags": ["vip", "billing"], "priority": "high",
                "type": "question", "updated_stamp": STAMP}}}),
            json!({"ticket": {"group_id": 7, "tags": ["vip", "billing"], "priority": "high",
                              "type": "question", "updated_stamp": STAMP, "safe_update": true}}),
        ),
        (
            "ticket.note",
            json!({"ticket_id": ticket_id, "body": {"ticket": {
                "comment": {"body": "Checked the billing history."}, "updated_stamp": STAMP}}}),
            json!({"ticket": {"comment": {"body": "Checked the billing history.", "public": false},
                              "updated_stamp": STAMP, "safe_update": true}}),
        ),
        (
            "ticket.draft_reply",
            json!({"ticket_id": ticket_id, "body": {"ticket": {
                "comment": {"body": "Hello,\nthe refund is on its way."}, "updated_stamp": STAMP}}}),
            json!({"ticket": {"comment": {
                       "body": format!("{DRAFT_MARKER}Hello,\nthe refund is on its way."),
                       "public": false},
                   "updated_stamp": STAMP, "safe_update": true}}),
        ),
    ]
}

/// On private protocol two the bootstrap lists each write as a write
/// requirement of the basic profile, which is what lets an approval policy
/// name it and makes the host require an approval; protocol one lists none.
#[test]
fn the_three_writes_are_write_requirements_only_on_protocol_two() {
    let two = Provider::writing();
    let child = Child::spawn(&two.selection()).unwrap();
    for (id, _, _) in WRITES {
        let requirement = child
            .bootstrap()
            .requirements
            .iter()
            .find(|r| r.operation == id)
            .unwrap_or_else(|| panic!("`{id}` is not a protocol-two requirement"));
        assert!(requirement.effect == runtime::Effect::Write, "`{id}`");
        assert_eq!(requirement.profile, PROFILE, "`{id}`");
    }
    let one = Provider::new();
    let child = Child::spawn(&one.selection()).unwrap();
    for (id, _, _) in WRITES {
        assert!(
            child
                .bootstrap()
                .requirements
                .iter()
                .all(|r| r.operation != id),
            "protocol one lists `{id}`"
        );
    }
    assert!(two.requests().is_empty() && one.requests().is_empty());
}

/// Approved, each write reads the ticket once (`ShowTicket`, whose
/// `updated_at` equals the caller's stamp) and then sends exactly one `PUT`
/// with the basic header and the exact body, and returns Zendesk's answer as
/// applied.
#[test]
fn each_approved_write_sends_its_preflight_and_exactly_one_put_with_the_exact_body() {
    let provider = Provider::writing();
    let mut child = Child::spawn(&provider.selection()).unwrap();
    for (id, input, body) in writes(101) {
        let before = provider.sent().len();
        let result = write(&mut child, id, &input)
            .unwrap_or_else(|failure| panic!("`{id}` refused in prepare: {failure:?}"));
        assert_eq!(result.effect, WriteEffect::Applied, "`{id}`");
        let result = result
            .result
            .unwrap_or_else(|failure| panic!("`{id}` result: {failure:?}"));
        let sent = provider.sent()[before..].to_vec();
        assert_eq!(
            sent,
            [
                (
                    "GET".to_owned(),
                    "/api/v2/tickets/101?".to_owned(),
                    Some(HEADER.to_owned()),
                    Value::Null
                ),
                (
                    "PUT".to_owned(),
                    "/api/v2/tickets/101?".to_owned(),
                    Some(HEADER.to_owned()),
                    body.clone()
                ),
            ],
            "`{id}` requests"
        );
        assert_eq!(result["status"], 200, "`{id}`");
        let served = updated("/api/v2/tickets/101", &serde_json::to_vec(&body).unwrap()).1;
        assert_eq!(result["body"], served, "`{id}` body");
        assert_eq!(result["provenance"]["instance"], "fixture-zendesk");
    }
}

/// The ticket changed between the preflight and the write: Zendesk answers
/// the safe update with 409. Each write is refused with the shared code
/// `conflict`, the provider's own answer, after exactly one `PUT`: nothing is
/// sent again.
#[test]
fn a_409_to_a_safe_update_is_a_conflict_refusal_and_is_not_retried() {
    let provider = Provider::writing();
    let mut child = Child::spawn(&provider.selection()).unwrap();
    for (id, input, body) in writes(102) {
        let before = provider.sent().len();
        let result = write(&mut child, id, &input)
            .unwrap_or_else(|failure| panic!("`{id}` refused in prepare: {failure:?}"));
        assert_eq!(result.effect, WriteEffect::Refused, "`{id}`");
        assert!(
            matches!(result.result, Err(Failure::ProviderConflict)),
            "`{id}`: {:?}",
            result.result
        );
        let methods: Vec<(String, Value)> = provider.sent()[before..]
            .iter()
            .map(|(method, _, _, body)| (method.clone(), body.clone()))
            .collect();
        assert_eq!(
            methods,
            [("GET".to_owned(), Value::Null), ("PUT".to_owned(), body)],
            "`{id}` sent other than one preflight and one write"
        );
    }
}

/// A stale stamp is refused by the preflight, with no `PUT` sent.
#[test]
fn a_stale_stamp_is_refused_before_the_write_is_sent() {
    let provider = Provider::writing();
    let mut child = Child::spawn(&provider.selection()).unwrap();
    for (id, mut input, _) in writes(101) {
        let before = provider.sent().len();
        input["body"]["ticket"]["updated_stamp"] = json!("2025-09-16T09:00:00Z");
        let outcome = write(&mut child, id, &input);
        assert!(
            matches!(outcome, Err(Failure::ProviderConflict)),
            "`{id}`: {:?}",
            outcome.map(|result| result.effect)
        );
        let methods: Vec<String> = provider.sent()[before..]
            .iter()
            .map(|(method, ..)| method.clone())
            .collect();
        assert_eq!(methods, ["GET"], "`{id}`");
    }
}

/// `priority` and `type` admit only the pinned enum values: anything else is
/// `invalid_input` naming the field, with nothing sent, not even the
/// preflight. So is an update that changes nothing, and a tag list that is not
/// a list of strings.
#[test]
fn an_update_outside_the_pinned_values_is_refused_naming_the_field_before_any_request() {
    let engine = Engine::new(&committed(), BASE, &shipped()).unwrap();
    let provider = Provider::writing();
    let mut child = Child::spawn(&provider.selection()).unwrap();
    for (ticket, named) in [
        (
            json!({"priority": "critical", "updated_stamp": STAMP}),
            "priority",
        ),
        (json!({"type": "bug", "updated_stamp": STAMP}), "type"),
        (json!({"tags": "vip", "updated_stamp": STAMP}), "tags"),
        (json!({"group_id": "7", "updated_stamp": STAMP}), "group_id"),
        (json!({"updated_stamp": STAMP}), "group_id"),
        (
            json!({"priority": "high", "status": "solved", "updated_stamp": STAMP}),
            "",
        ),
    ] {
        let input = json!({"ticket_id": 101, "body": {"ticket": ticket}});
        // The engine names the field; the private boundary carries the code.
        let error = match tokio::runtime::Builder::new_current_thread()
            .build()
            .unwrap()
            .block_on(engine.prepare(
                &Unreachable,
                "fixture-zendesk",
                "ticket.update",
                input.clone(),
            )) {
            Ok(_) => panic!("{input} admitted"),
            Err(error) => error,
        };
        assert_eq!(
            error.code,
            connectors_core::ErrorCode::InvalidInput,
            "{input}"
        );
        assert!(error.message.contains(named), "{input}: {}", error.message);
        let outcome = write(&mut child, "ticket.update", &input);
        assert!(
            matches!(outcome, Err(Failure::InvalidInput)),
            "{input}: {:?}",
            outcome.map(|result| result.effect)
        );
    }
    assert!(provider.requests().is_empty(), "a request was sent");
}

/// A transport no refused input may reach: every read panics.
struct Unreachable;
#[async_trait::async_trait]
impl connectors_sdk::AuthenticatedHttp for Unreachable {
    async fn get(
        &self,
        _: &[&str],
        _: &[(&str, String)],
    ) -> connectors_core::Result<connectors_sdk::HttpResponse> {
        panic!("a refused input reached the provider")
    }
}

/// A note and a draft reply are always private, and a draft always opens
/// with its marker line: `public` and `safe_update` are fixed by the selection,
/// undeclared, and refused from the caller whatever their value; the marker
/// is the selection's, before whatever the caller writes.
#[test]
fn a_note_or_draft_cannot_be_made_public_and_the_draft_marker_is_always_first() {
    let engine = Engine::new(&committed(), BASE, &shipped()).unwrap();
    for id in ["ticket.note", "ticket.draft_reply"] {
        let declaration = engine
            .declarations(&[Effect::Write])
            .into_iter()
            .find(|o| o.id == id)
            .unwrap();
        let ticket = &declaration.input_schema["properties"]["body"]["properties"]["ticket"];
        assert_eq!(
            ticket["properties"]["comment"]["properties"],
            json!({"body": {"type": "string", "minLength": 1}}),
            "`{id}` declares more than the comment's text"
        );
        assert!(ticket["properties"].get("safe_update").is_none(), "`{id}`");
        let selection = shipped().into_iter().find(|s| s.id == id).unwrap();
        assert_eq!(
            selection.body_fixed.get("/ticket/comment/public"),
            Some(&json!(false)),
            "`{id}`"
        );
    }
    let draft = shipped()
        .into_iter()
        .find(|s| s.id == "ticket.draft_reply")
        .unwrap();
    assert_eq!(
        draft
            .body_prefixes
            .get("/ticket/comment/body")
            .map(String::as_str),
        Some(DRAFT_MARKER)
    );
    let provider = Provider::writing();
    let mut child = Child::spawn(&provider.selection()).unwrap();
    for id in ["ticket.note", "ticket.draft_reply"] {
        for comment in [
            json!({"body": "text", "public": true}),
            json!({"body": "text", "public": false}),
            json!({"body": ""}),
        ] {
            let input = json!({"ticket_id": 101, "body": {"ticket": {
                "comment": comment, "updated_stamp": STAMP}}});
            let outcome = write(&mut child, id, &input);
            assert!(
                matches!(outcome, Err(Failure::InvalidInput)),
                "`{id}` {input}: {:?}",
                outcome.map(|result| result.effect)
            );
        }
        let input = json!({"ticket_id": 101, "body": {"ticket": {
            "comment": {"body": "text"}, "updated_stamp": STAMP, "safe_update": false}}});
        assert!(
            matches!(write(&mut child, id, &input), Err(Failure::InvalidInput)),
            "`{id}`"
        );
    }
    assert!(provider.requests().is_empty(), "a request was sent");
    // A caller who writes the marker itself still gets it first, once more.
    let input = json!({"ticket_id": 101, "body": {"ticket": {
        "comment": {"body": format!("{DRAFT_MARKER}text")}, "updated_stamp": STAMP}}});
    let result = write(&mut child, "ticket.draft_reply", &input).unwrap();
    assert_eq!(result.effect, WriteEffect::Applied);
    let sent = provider.sent().last().unwrap().3.clone();
    assert_eq!(
        sent["ticket"]["comment"]["body"],
        format!("{DRAFT_MARKER}{DRAFT_MARKER}text")
    );
}

/// Without an approval there is no route to any write: on protocol two the
/// read transport, which carries none, refuses each as unsupported; on
/// protocol one it is not an operation and prepare is unsupported. Nothing
/// reaches Zendesk.
#[test]
fn each_write_without_an_approval_is_refused_before_any_request() {
    let two = Provider::writing();
    let mut child = Child::spawn(&two.selection()).unwrap();
    for (id, input, _) in writes(101) {
        let outcome = attempt(&mut child, id, &input);
        assert!(
            matches!(outcome, Err(Failure::Unsupported)),
            "`{id}` on the protocol-two read transport: {outcome:?}"
        );
    }
    let one = Provider::new();
    let mut child = Child::spawn(&one.selection()).unwrap();
    for (id, input, _) in writes(101) {
        let outcome = attempt(&mut child, id, &input);
        assert!(
            matches!(outcome, Err(Failure::NotFound)),
            "`{id}` on protocol one: {outcome:?}"
        );
        let outcome = write(&mut child, id, &input);
        assert!(
            matches!(outcome, Err(Failure::Unsupported)),
            "`{id}` prepared on protocol one: {:?}",
            outcome.map(|result| result.effect)
        );
    }
    assert!(two.requests().is_empty(), "protocol two sent a request");
    assert!(one.requests().is_empty(), "protocol one sent a request");
}

#[test]
fn guide_cites_each_write_its_guard_and_the_move_steps() {
    let guide = fs::read_to_string(root().join("../../docs/catalog-zendesk.md")).unwrap();
    let rows: Vec<&str> = guide.lines().filter(|l| l.starts_with('|')).collect();
    for (id, operation_id, path) in WRITES {
        assert!(
            rows.iter().any(|row| row.contains(&format!("`{id}`"))
                && row.contains(&format!("`{operation_id}`"))
                && row.contains(&format!("`PUT {path}`"))
                && row.contains("`safe_update`")
                && row.contains("`conflict`")),
            "no row cites `{id}` as `{operation_id}` `PUT {path}` with its guard and its `conflict`"
        );
    }
    for needle in [
        "approvals prepare",
        "approvals issue",
        "operations invoke",
        DRAFT_MARKER.trim_end(),
        "https://github.com/beyond10x/connectors/releases",
    ] {
        assert!(guide.contains(needle), "the guide does not say `{needle}`");
    }
}
