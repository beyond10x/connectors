//! Google Drive reads — about, files, export and changes — and guarded metadata
//! writes — create, update, copy — through the catalog provider.
//!
//! The shipped selection set is pinned by id and Discovery method id, resolves
//! against the committed bundle compiled from the projection of the pinned
//! Drive v3 Discovery document, and is cited row by row in
//! `docs/catalog-google-drive.md`. Each read runs through the provider child
//! against a disposable HTTPS fixture that serves the Drive routes and an OAuth
//! token route on one host: the child exchanges the fixture refresh entry for an
//! access token, and the exact request (path, query, `Authorization: Bearer …`)
//! and the returned body are asserted. Both lists walk two pages to their end
//! conditions. The profile is the guide's own, pointed at the fixture token
//! route. The fixture secrets are fictional and only ever compared, never
//! printed. No live credential and no network.
//!
//! The writes run through a private-protocol-two child from the guide's write
//! configuration: prepare (with the declared preflight) and commit, the
//! transport the host's approval coordinator drives. The fixture records each
//! write's method, route and body, so "no write was sent" is a count of what
//! the fixture saw.
use connectors_catalog::{bundle, discovery};
use connectors_catalog_provider::{
    Check, Effect, Engine, Expectation, Guard, Postflight, Preflight, ResponseKind, Selection,
};
use connectors_host::local::{
    approvals,
    config::{Adapter, Executable, Restart, Startup},
    filesystem, mutations,
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

/// The path the configured `api_base` carries: the projection's one server,
/// `https://www.googleapis.com/drive/v3`, below its host.
const BASE: &str = "/drive/v3";
const PROVIDER: &str = "google-drive";
/// The bundle's auth profile, and the profile of the guide's configuration.
const PROFILE: &str = "google.oauth";
const DRIVE_SCOPE: &str = "https://www.googleapis.com/auth/drive.readonly";
/// The write scope: files the app created or opened, and nothing else.
const WRITE_SCOPE: &str = "https://www.googleapis.com/auth/drive.file";
/// The fixture file's current `version`; a stale pin is any other value.
const VERSION: &str = "7";
/// Fictional OAuth material. The access token is what the fixture token route
/// issues and the only bearer the fixture Drive routes accept.
const CLIENT_ID: &str = "fixture-client-id.apps.example.test";
const CLIENT_SECRET: &str = "fixture-client-secret-drive";
const REFRESH_TOKEN: &str = "fixture-refresh-token-drive";
const ACCESS_TOKEN: &str = "fixture-access-token-drive";
/// The pinned Discovery document, relative to this crate.
const UPSTREAM: &str = "../google/upstream/drive/drive-api.json";
/// The committed projection, relative to this crate.
const PROJECTED: &str = "../google/generated/drive.openapi.json";

/// The shipped ids, their Discovery method id and the path the bundle records
/// (the Discovery path below the server path `/drive/v3`). A renamed, dropped
/// or added id fails here.
const SHIPPED: [(&str, &str, &str); 6] = [
    ("about.get", "drive.about.get", "/drive/v3/about"),
    (
        "changes.getStartPageToken",
        "drive.changes.getStartPageToken",
        "/drive/v3/changes/startPageToken",
    ),
    ("changes.list", "drive.changes.list", "/drive/v3/changes"),
    (
        "files.export",
        "drive.files.export",
        "/drive/v3/files/{fileId}/export",
    ),
    ("files.get", "drive.files.get", "/drive/v3/files/{fileId}"),
    ("files.list", "drive.files.list", "/drive/v3/files"),
];
/// The shipped writes: id, Discovery method id, method and recorded path.
const WRITES: [(&str, &str, &str, &str); 3] = [
    (
        "files.copy",
        "drive.files.copy",
        "post",
        "/drive/v3/files/{fileId}/copy",
    ),
    (
        "files.create",
        "drive.files.create",
        "post",
        "/drive/v3/files",
    ),
    (
        "files.update",
        "drive.files.update",
        "patch",
        "/drive/v3/files/{fileId}",
    ),
];

fn root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
}
fn shipped() -> Vec<Selection> {
    let file: Value = serde_json::from_slice(
        &fs::read(root().join("providers/google-drive/operations.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(file["format"], "connectors-catalog-operations/1");
    assert_eq!(file["provider"], PROVIDER);
    serde_json::from_value(file["operations"].clone()).unwrap()
}
fn pinned() -> Value {
    serde_json::from_slice(&fs::read(root().join(UPSTREAM)).unwrap()).unwrap()
}
fn committed_bundle() -> bundle::Bundle {
    bundle::load(&root().join("generated/bundles"), PROVIDER).unwrap()
}

#[test]
fn shipped_drive_selections_are_the_six_reads_and_three_writes() {
    let selections = shipped();
    let bundle = committed_bundle();
    assert_eq!(bundle.auth_profile, PROFILE);
    let engine = Engine::new(&bundle, BASE, &selections).unwrap();
    let ids = |effects: &[Effect]| {
        let mut ids: Vec<String> = engine
            .declarations(effects)
            .into_iter()
            .map(|o| o.id)
            .collect();
        ids.sort();
        ids
    };
    let reads: Vec<&str> = SHIPPED.iter().map(|(id, _, _)| *id).collect();
    let writes: Vec<&str> = WRITES.iter().map(|(id, _, _, _)| *id).collect();
    assert_eq!(ids(&[Effect::Read]), reads);
    assert_eq!(ids(&[Effect::Write]), writes);
    assert_eq!(selections.len(), SHIPPED.len() + WRITES.len());
    for (id, operation_id, method, path) in WRITES {
        let selection = selections.iter().find(|s| s.id == id).unwrap();
        assert_eq!(selection.operation_id, operation_id, "`{id}`");
        assert_eq!(Some(id), operation_id.strip_prefix("drive."), "`{id}`");
        assert_eq!(selection.effect, Effect::Write, "`{id}`");
        assert_eq!(engine.effect(id), Some(Effect::Write), "`{id}`");
        assert!(selection.bounds.is_empty(), "`{id}`");
        let operation = bundle
            .inventory
            .operations
            .iter()
            .find(|o| o.operation_id.as_deref() == Some(operation_id))
            .unwrap_or_else(|| panic!("the projection lacks `{operation_id}`"));
        assert_eq!(operation.method, method, "`{id}`");
        assert_eq!(operation.path, path, "`{id}`");
        // Metadata only: the JSON body, never an upload media type.
        assert_eq!(
            operation.request_media_types,
            ["application/json"],
            "`{id}`"
        );
    }
    // A create and a copy have nothing to compare before they run; the update
    // pins the file's `version`, read by `files.get` with the caller's `fields`.
    for id in ["files.create", "files.copy"] {
        let selection = selections.iter().find(|s| s.id == id).unwrap();
        assert_eq!(selection.guard, None, "`{id}`");
    }
    let update = selections.iter().find(|s| s.id == "files.update").unwrap();
    assert_eq!(
        update.guard,
        Some(Guard {
            preflight: Preflight {
                operation_id: "drive.files.get".into(),
                values: [("fileId", "fileId"), ("fields", "fields")]
                    .into_iter()
                    .map(|(k, v)| (k.to_owned(), v.to_owned()))
                    .collect(),
                checks: vec![Check {
                    pointer: "/version".into(),
                    expect: Expectation::Input("version".into()),
                }],
            },
            postflight: Postflight { checks: vec![] },
        })
    );
    // The declared inputs: every write takes a `body`; the update also takes
    // the pinned `version`, which no Drive parameter carries.
    let declared = engine.declarations(&[Effect::Write]);
    let required = |id: &str| -> Vec<String> {
        let declaration = declared.iter().find(|o| o.id == id).unwrap();
        assert_eq!(declaration.profile, "mutation", "`{id}`");
        serde_json::from_value(declaration.input_schema["required"].clone()).unwrap()
    };
    assert_eq!(required("files.create"), ["body"]);
    assert_eq!(required("files.copy"), ["body", "fileId"]);
    assert_eq!(required("files.update"), ["body", "fileId", "version"]);
    for (id, operation_id, path) in SHIPPED {
        let selection = selections.iter().find(|s| s.id == id).unwrap();
        assert_eq!(selection.operation_id, operation_id, "`{id}`");
        // The selection id is the Discovery id without its API-name prefix.
        assert_eq!(
            Some(id),
            operation_id.strip_prefix("drive."),
            "`{id}` is not `{operation_id}` without `drive.`"
        );
        assert_eq!(selection.effect, Effect::Read, "`{id}`");
        assert_eq!(engine.effect(id), Some(Effect::Read), "`{id}`");
        let operation = bundle
            .inventory
            .operations
            .iter()
            .find(|o| o.operation_id.as_deref() == Some(operation_id))
            .unwrap_or_else(|| panic!("the projection lacks `{operation_id}`"));
        assert_eq!(operation.method, "get", "`{id}`");
        assert_eq!(operation.path, path, "`{id}`");
    }
    // Only the export reads its body as text; the projection declares it
    // `application/octet-stream`, which the engine would otherwise parse as JSON.
    for selection in &selections {
        let text = selection.response == Some(ResponseKind::Text);
        assert_eq!(text, selection.id == "files.export", "`{}`", selection.id);
    }
}

#[test]
fn a_selection_the_projection_lacks_is_refused_at_load() {
    let bundle = committed_bundle();
    let mut selections = shipped();
    // Drive has no `files.search` method: search is `files.list` with `q`.
    // The id is absent from the bundle, so the load is refused for its absence
    // and for nothing else.
    let absent = "drive.files.search";
    assert!(
        bundle
            .inventory
            .operations
            .iter()
            .all(|o| o.operation_id.as_deref() != Some(absent)),
        "the bundle carries `{absent}`"
    );
    selections[0].operation_id = absent.into();
    let refusal = Engine::new(&bundle, BASE, &selections)
        .err()
        .expect("refused at load");
    assert_eq!(
        refusal.message,
        format!("bundle carries no operation `{absent}`")
    );
}

/// The bundle is the projection of the pinned document, and records it.
#[test]
fn the_bundle_is_derived_from_the_pinned_discovery_document() {
    let bytes = fs::read(root().join(UPSTREAM)).unwrap();
    let digest = hex::encode(Sha256::digest(&bytes));
    let projection = discovery::project(&bytes).unwrap();
    let committed = fs::read(root().join(PROJECTED)).unwrap();
    assert!(
        committed == projection.openapi,
        "committed projection drifted from the pinned Discovery document"
    );
    let bundle = committed_bundle();
    assert_eq!(bundle.source.file_name, "drive.openapi.json");
    assert_eq!(
        bundle.source.source_sha256,
        hex::encode(Sha256::digest(&committed))
    );
    let derivation = bundle.source.derivation.expect("the bundle's derivation");
    assert_eq!(derivation.from_file, "drive-api.json");
    assert_eq!(derivation.from_sha256, digest);
    assert_eq!(derivation.from_bytes, bytes.len());
    assert_eq!(derivation.format, discovery::FORMAT);
    assert_eq!(derivation.projector, discovery::PROJECTOR);
    assert_eq!(json!(derivation.discovery_revision), pinned()["revision"]);
    let index = bundle::read_index(&root().join("generated/bundles")).unwrap();
    assert!(index.find(PROVIDER).is_some());
}

/// `pageSize` is bounded on both lists at exactly the range the pinned
/// Discovery document states, and no other selection carries a bound.
#[test]
fn both_lists_bound_page_size_at_the_pinned_range() {
    let document = pinned();
    let mut bounded = Vec::new();
    for selection in shipped() {
        let written = serde_json::to_value(&selection).unwrap();
        let (resource, method) = selection.id.split_once('.').unwrap();
        let parameters = &document["resources"][resource]["methods"][method]["parameters"];
        let Some(page_size) = parameters.get("pageSize") else {
            assert_eq!(written["bounds"], Value::Null, "`{}`", selection.id);
            continue;
        };
        bounded.push(selection.id.clone());
        let integer = |key: &str| page_size[key].as_str().unwrap().parse::<u64>().unwrap();
        assert_eq!(
            written["bounds"],
            json!({"pageSize": {"minimum": integer("minimum"), "maximum": integer("maximum")}}),
            "`{}`",
            selection.id
        );
        assert_eq!(integer("minimum"), 1, "`{}`", selection.id);
        assert_eq!(integer("maximum"), 1000, "`{}`", selection.id);
    }
    bounded.sort();
    assert_eq!(bounded, ["changes.list", "files.list"]);
}

#[test]
fn guide_cites_each_operation_its_paging_and_its_deltas() {
    let guide = fs::read_to_string(root().join("../../docs/catalog-google-drive.md")).unwrap();
    let rows: Vec<&str> = guide.lines().filter(|l| l.starts_with('|')).collect();
    for (id, operation_id, path, paging, end, deltas) in [
        (
            "about.get",
            "drive.about.get",
            "/drive/v3/about",
            "single item",
            "n/a",
            "none",
        ),
        (
            "files.list",
            "drive.files.list",
            "/drive/v3/files",
            "`pageToken`, `pageSize`",
            "`nextPageToken` absent",
            "`q`",
        ),
        (
            "files.get",
            "drive.files.get",
            "/drive/v3/files/{fileId}",
            "single item",
            "n/a",
            "`changes.list`",
        ),
        (
            "files.export",
            "drive.files.export",
            "/drive/v3/files/{fileId}/export",
            "single item",
            "n/a",
            "`changes.list`",
        ),
        (
            "changes.getStartPageToken",
            "drive.changes.getStartPageToken",
            "/drive/v3/changes/startPageToken",
            "single item",
            "n/a",
            "`startPageToken`",
        ),
        (
            "changes.list",
            "drive.changes.list",
            "/drive/v3/changes",
            "`pageToken`, `pageSize`",
            "`newStartPageToken` present",
            "`newStartPageToken`",
        ),
    ] {
        let cited = rows.iter().any(|row| {
            row.contains(&format!("`{id}`"))
                && row.contains(&format!("`{operation_id}`"))
                && row.contains(&format!("`GET {path}`"))
        });
        assert!(cited, "no row cites `{id}` as `{operation_id}` `{path}`");
        let paged = rows.iter().any(|row| {
            row.starts_with(&format!("| `{id}` "))
                && row.contains(paging)
                && row.contains(end)
                && row.contains(deltas)
        });
        assert!(paged, "no row states the paging of `{id}`");
    }
}

/// The guide cites each write with its request and guard, names the write
/// scope, and names how an existing connection gains it.
#[test]
fn guide_cites_each_write_its_guard_and_the_write_scope() {
    let guide = fs::read_to_string(root().join("../../docs/catalog-google-drive.md")).unwrap();
    let rows: Vec<&str> = guide.lines().filter(|l| l.starts_with('|')).collect();
    for (id, operation_id, request, guard) in [
        (
            "files.create",
            "drive.files.create",
            "`POST /drive/v3/files`",
            "none",
        ),
        (
            "files.update",
            "drive.files.update",
            "`PATCH /drive/v3/files/{fileId}`",
            "`/version` equals the input `version`",
        ),
        (
            "files.copy",
            "drive.files.copy",
            "`POST /drive/v3/files/{fileId}/copy`",
            "none",
        ),
    ] {
        let cited = rows.iter().any(|row| {
            row.starts_with(&format!("| `{id}` "))
                && row.contains(&format!("`{operation_id}`"))
                && row.contains(request)
                && row.contains(guard)
        });
        assert!(cited, "no row cites `{id}` as {request} with guard {guard}");
    }
    assert!(guide.contains(&format!("`{WRITE_SCOPE}`")));
    // The write scope comes from a separate instance, connected afresh: a
    // repair cannot widen a connection, whose binding holds the configuration
    // revision and the profile.
    let flat = guide.split_whitespace().collect::<Vec<_>>().join(" ");
    assert!(flat.contains("Writes use a separate instance"));
    assert!(flat.contains("`google-drive-write`"));
    assert!(flat.contains("then `connections connect` that instance with the Google client file"));
    assert!(flat.contains("`connections repair` cannot add a scope"));
    assert!(
        !flat.contains("through `connections repair`")
            && !flat.contains("with `connections repair`"),
        "the guide still routes the write scope through `connections repair`"
    );
}

/// The guide's configuration example for this provider: the read-only one, or
/// the one that also asks for the write scope.
fn documented(write: bool) -> Value {
    let guide = fs::read_to_string(root().join("../../docs/catalog-google-drive.md")).unwrap();
    let example = guide
        .split("```json\n")
        .skip(1)
        .filter_map(|rest| rest.split_once("\n```").map(|(body, _)| body))
        .find(|body| {
            body.contains(&format!("\"provider\": \"{PROVIDER}\""))
                && body.contains(WRITE_SCOPE) == write
        })
        .expect("the documented google-drive configuration");
    serde_json::from_str::<Value>(example).unwrap()
}
fn documented_config() -> Value {
    documented(false)
}

/// The write configuration is a separate instance, `google-drive-write`: the
/// read one with its own instance id and the write scope added to both
/// `minimum_scopes` and `requested_scopes`, and nothing else changed.
#[test]
fn guide_documents_the_write_configuration_with_the_drive_file_scope() {
    let read = documented_config();
    let write = documented(true);
    assert_eq!(read["instance"], "google-drive");
    assert_eq!(write["instance"], "google-drive-write");
    assert_eq!(
        write["auth"]["minimum_scopes"],
        json!([DRIVE_SCOPE, WRITE_SCOPE])
    );
    assert_eq!(
        write["auth"]["requested_scopes"],
        json!(["openid", DRIVE_SCOPE, WRITE_SCOPE])
    );
    let strip = |mut config: Value| {
        config.as_object_mut().unwrap().remove("instance");
        config["auth"]
            .as_object_mut()
            .unwrap()
            .retain(|key, _| key != "minimum_scopes" && key != "requested_scopes");
        config
    };
    assert_eq!(strip(write), strip(read));
}

/// The guide configures the bundle's profile as an `oauth2_refresh` profile
/// against Google's token endpoint, with the Drive read-only scope, and the
/// projection's server as the API base.
#[test]
fn guide_documents_the_oauth_refresh_configuration() {
    let config = documented_config();
    assert_eq!(config["api_base"], "https://www.googleapis.com/drive/v3");
    let auth = &config["auth"];
    assert_eq!(auth["profile"], PROFILE);
    assert_eq!(auth["scheme"], "oauth2_refresh");
    assert_eq!(auth["header"], "Authorization");
    assert_eq!(auth["bearer"], true);
    assert_eq!(auth["token_url"], "https://oauth2.googleapis.com/token");
    assert_eq!(
        auth["authorize_url"],
        "https://accounts.google.com/o/oauth2/auth"
    );
    assert_eq!(auth["identity"]["source"], "id_token");
    assert_eq!(auth["minimum_scopes"], json!([DRIVE_SCOPE]));
    assert!(
        auth["requested_scopes"]
            .as_array()
            .unwrap()
            .contains(&json!(DRIVE_SCOPE))
    );
    assert!(auth.get("token_ca_file").is_none());
}

/// Method, route with query, and `Authorization` header of each fixture request.
type Requests = Arc<Mutex<Vec<(String, String, Option<String>)>>>;
/// Method, route with query, and body of each Drive request that carried one.
type Bodies = Arc<Mutex<Vec<(String, String, Vec<u8>)>>>;

fn file(id: &str, name: &str) -> Value {
    json!({"kind": "drive#file", "id": id, "name": name,
           "mimeType": "application/vnd.google-apps.document",
           "modifiedTime": "2026-09-20T10:00:00.000Z", "version": VERSION})
}

/// The recorded Drive answers to the three writes, built from the request
/// body as Drive would: the new or changed file's metadata. `None` for any
/// other write, which the fixture answers 404.
fn write_answer(method: &str, target: &str, body: &[u8]) -> Option<Value> {
    let route = target.split_once('?').map_or(target, |(route, _)| route);
    let body: Value = serde_json::from_slice(body).ok()?;
    match (method, route) {
        ("POST", "/drive/v3/files") => Some(json!({"kind": "drive#file",
            "id": "fixture-created-1", "name": body["name"], "mimeType": body["mimeType"],
            "version": "1"})),
        ("POST", "/drive/v3/files/fixture-doc-1/copy") => Some(json!({"kind": "drive#file",
            "id": "fixture-copy-1", "name": body["name"],
            "mimeType": "application/vnd.google-apps.document", "version": "1"})),
        ("PATCH", "/drive/v3/files/fixture-doc-1") => Some(json!({"kind": "drive#file",
            "id": "fixture-doc-1", "name": body["name"], "version": "8"})),
        _ => None,
    }
}
fn change(file_id: &str, time: &str) -> Value {
    json!({"kind": "drive#change", "changeType": "file", "fileId": file_id, "removed": false,
           "time": time, "file": file(file_id, &format!("fixture {file_id}"))})
}

/// The bytes `files.export` answers, as `text/plain`.
const EXPORTED: &str = "Fixture document\nsecond line, exported as plain text\n";

/// The recorded Drive answers the fixture serves, keyed by route and paging
/// position: a status, a content type and a body. `None` for anything else,
/// which the fixture answers 404.
fn answer(target: &str) -> Option<(&'static str, Vec<u8>)> {
    let (route, query) = target.split_once('?').unwrap_or((target, ""));
    let has = |pair: &str| query.split('&').any(|p| p == pair);
    let json_body = |value: Value| Some(("application/json", serde_json::to_vec(&value).unwrap()));
    if route == "/drive/v3/files/fixture-doc-1/export" && has("mimeType=text%2Fplain") {
        return Some(("text/plain; charset=utf-8", EXPORTED.as_bytes().to_vec()));
    }
    match route {
        "/drive/v3/about" => json_body(about()),
        "/drive/v3/files" if has("pageToken=fixture-files-page-2") => {
            json_body(json!({"kind": "drive#fileList", "incompleteSearch": false,
                   "files": [file("fixture-doc-3", "third")]}))
        }
        "/drive/v3/files" if has("pageSize=2") => json_body(json!({
            "kind": "drive#fileList", "incompleteSearch": false,
            "nextPageToken": "fixture-files-page-2",
            "files": [file("fixture-doc-1", "first"), file("fixture-doc-2", "second")]})),
        // Any other page size: one last page.
        "/drive/v3/files" => {
            json_body(json!({"kind": "drive#fileList", "incompleteSearch": false, "files": []}))
        }
        "/drive/v3/files/fixture-doc-1" => json_body(file("fixture-doc-1", "first")),
        "/drive/v3/changes/startPageToken" => json_body(
            json!({"kind": "drive#startPageToken", "startPageToken": "fixture-start-100"}),
        ),
        "/drive/v3/changes" if has("pageToken=fixture-changes-page-2") => json_body(json!({
            "kind": "drive#changeList", "newStartPageToken": "fixture-start-103",
            "changes": [change("fixture-doc-3", "2026-09-21T10:00:00.000Z")]})),
        "/drive/v3/changes" if has("pageToken=fixture-start-100") && has("pageSize=2") => {
            json_body(json!({
                "kind": "drive#changeList", "nextPageToken": "fixture-changes-page-2",
                "changes": [change("fixture-doc-1", "2026-09-20T10:00:00.000Z"),
                            change("fixture-doc-2", "2026-09-20T11:00:00.000Z")]}))
        }
        // Any other page size from the baseline: one last page.
        "/drive/v3/changes" => json_body(json!({
            "kind": "drive#changeList", "newStartPageToken": "fixture-start-100", "changes": []})),
        _ => None,
    }
}
fn about() -> Value {
    json!({"kind": "drive#about",
           "user": {"kind": "drive#user", "displayName": "Fixture Reader",
                    "emailAddress": "reader@example.test", "permissionId": "fixture-permission"},
           "storageQuota": {"limit": "16106127360", "usage": "1024"}})
}
/// The recorded JSON body of `target`.
fn recorded(target: &str) -> Value {
    serde_json::from_slice(&answer(target).unwrap().1).unwrap()
}

/// An unsigned JWT with `claims` as its payload, as the token answer's
/// `id_token`.
fn id_token() -> String {
    use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
    let part = |value: &Value| URL_SAFE_NO_PAD.encode(serde_json::to_vec(value).unwrap());
    format!(
        "{}.{}.{}",
        part(&json!({"alg": "RS256", "typ": "JWT", "kid": "fixture"})),
        part(
            &json!({"iss": "https://accounts.google.com", "aud": CLIENT_ID, "azp": CLIENT_ID,
                     "sub": "110000000000000000001", "iat": 1, "exp": 4_000_000_000_u64})
        ),
        URL_SAFE_NO_PAD.encode(b"fixture-signature")
    )
}

struct Provider {
    stop: Option<oneshot::Sender<()>>,
    thread: Option<std::thread::JoinHandle<()>>,
    _root: tempfile::TempDir,
    config: PathBuf,
    requests: Requests,
    bodies: Bodies,
    protocol: Option<PrivateProtocol>,
}
impl Provider {
    /// The guide's read configuration on private protocol one; the token route
    /// grants the read scope.
    fn new() -> Self {
        Self::with(documented_config(), format!("openid {DRIVE_SCOPE}"), None)
    }
    /// The guide's write configuration on private protocol two; the token
    /// route grants `grant`.
    fn writes(grant: &str) -> Self {
        Self::with(
            documented(true),
            grant.to_owned(),
            Some(PrivateProtocol::V2),
        )
    }
    fn with(mut config: Value, grant: String, protocol: Option<PrivateProtocol>) -> Self {
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
        let bodies: Bodies = Arc::new(Mutex::new(Vec::new()));
        let carried = bodies.clone();
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
                    let (status, content_type, answer) = if method == "POST" && target == "/token"
                    {
                        // Only the fixture entry is exchanged; the form is
                        // compared, never printed.
                        let form = String::from_utf8(body).unwrap();
                        let mut fields: Vec<&str> = form.split('&').collect();
                        fields.sort();
                        let mut expected = [
                            "grant_type=refresh_token".to_owned(),
                            format!("client_id={CLIENT_ID}"),
                            format!("client_secret={CLIENT_SECRET}"),
                            format!("refresh_token={REFRESH_TOKEN}"),
                        ];
                        expected.sort();
                        if fields == expected {
                            let grant = json!({
                                "access_token": ACCESS_TOKEN, "token_type": "Bearer",
                                "expires_in": 3600,
                                "scope": grant.as_str(),
                                "id_token": id_token()});
                            (200, "application/json", serde_json::to_vec(&grant).unwrap())
                        } else {
                            let refusal = json!({"error": "invalid_grant"});
                            (400, "application/json", serde_json::to_vec(&refusal).unwrap())
                        }
                    } else if authorization.as_deref() != Some(&*format!("Bearer {ACCESS_TOKEN}"))
                    {
                        let refusal = json!({"error": {"code": 401, "message": "fixture refusal"}});
                        (401, "application/json", serde_json::to_vec(&refusal).unwrap())
                    } else {
                        if !body.is_empty() {
                            carried
                                .lock()
                                .unwrap()
                                .push((method.clone(), target.clone(), body.clone()));
                        }
                        let written = write_answer(&method, &target, &body)
                            .map(|value| ("application/json", serde_json::to_vec(&value).unwrap()));
                        match (method.as_str(), answer(&target), written) {
                            ("GET", Some((content_type, body)), _) => (200, content_type, body),
                            ("POST" | "PATCH", _, Some((content_type, body))) => {
                                (200, content_type, body)
                            }
                            _ => {
                                let missing = json!({"error": {"code": 404, "message": "no fixture"}});
                                (404, "application/json", serde_json::to_vec(&missing).unwrap())
                            }
                        }
                    };
                    observed
                        .lock()
                        .unwrap()
                        .push((method, target, authorization));
                    let head = format!(
                        "HTTP/1.1 {status} fixture\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                        answer.len()
                    );
                    let _ = stream.write_all(head.as_bytes()).await;
                    let _ = stream.write_all(&answer).await;
                }
            });
        });
        let address = address_rx.recv_timeout(Duration::from_secs(5)).unwrap();
        // The guide's configuration, with the fixture as API host and token
        // host, trusted through its own CA.
        config["instance"] = json!("fixture-google-drive");
        config["bundle_directory"] = json!(root_path("generated/bundles"));
        config["operations_file"] = json!(root_path("providers/google-drive/operations.json"));
        config["api_base"] = json!(format!("https://localhost:{}/drive/v3", address.port()));
        config["ca_file"] = json!(ca);
        config["auth"]["token_url"] = json!(format!("https://localhost:{}/token", address.port()));
        config["auth"]["token_ca_file"] = json!(ca);
        let path = directory.join("catalog.json");
        private(&path, &serde_json::to_vec(&config).unwrap());
        Self {
            stop: Some(stop),
            thread: Some(thread),
            _root: root,
            config: path,
            requests,
            bodies,
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
            private_protocol: self.protocol,
            permissions: Default::default(),
            instance_id: "fixture-google-drive".into(),
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
    fn requests(&self) -> Vec<(String, String, Option<String>)> {
        self.requests.lock().unwrap().clone()
    }
    /// The Drive requests as method and route with query.
    fn api_calls(&self) -> Vec<(String, String)> {
        self.requests()
            .into_iter()
            .filter(|(_, target, _)| target != "/token")
            .map(|(method, target, _)| (method, target))
            .collect()
    }
    /// The Drive requests that carried a body, the body parsed as JSON.
    fn bodies(&self) -> Vec<(String, String, Value)> {
        self.bodies
            .lock()
            .unwrap()
            .iter()
            .map(|(method, target, body)| {
                (
                    method.clone(),
                    target.clone(),
                    serde_json::from_slice(body).unwrap(),
                )
            })
            .collect()
    }
    /// The Drive requests (everything but the token route), as route with query.
    fn api_targets(&self) -> Vec<String> {
        self.requests()
            .into_iter()
            .filter(|(_, target, _)| target != "/token")
            .map(|(_, target, _)| target)
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
    Secret(
        serde_json::to_vec(&json!({
            "client_id": CLIENT_ID,
            "client_secret": CLIENT_SECRET,
            "refresh_token": REFRESH_TOKEN,
        }))
        .unwrap(),
    )
}
fn attempt(child: &mut Child, operation: &str, input: &Value) -> Result<Vec<u8>, Failure> {
    let revision = child.bootstrap().descriptor().unwrap().revision;
    child.invoke(
        operation,
        &revision,
        "one",
        &secret(),
        &serde_json::to_vec(input).unwrap(),
        connectors_sdk::now_ms() + 30_000,
    )
}
fn invoke(child: &mut Child, operation: &str, input: Value) -> Value {
    let output = attempt(child, operation, &input)
        .unwrap_or_else(|failure| panic!("`{operation}` failed: {failure:?}"));
    serde_json::from_slice(&output).unwrap()
}

/// Each read's input and the exact request the fixture must observe.
fn first_requests() -> [(&'static str, Value, &'static str); 6] {
    [
        (
            "about.get",
            json!({"fields": "user,storageQuota"}),
            "/drive/v3/about?fields=user%2CstorageQuota",
        ),
        (
            "files.list",
            json!({"q": "trashed = false", "pageSize": 2}),
            "/drive/v3/files?pageSize=2&q=trashed+%3D+false",
        ),
        (
            "files.get",
            json!({"fileId": "fixture-doc-1", "fields": "id,name,mimeType,modifiedTime"}),
            "/drive/v3/files/fixture-doc-1?fields=id%2Cname%2CmimeType%2CmodifiedTime",
        ),
        (
            "files.export",
            json!({"fileId": "fixture-doc-1", "mimeType": "text/plain"}),
            "/drive/v3/files/fixture-doc-1/export?mimeType=text%2Fplain",
        ),
        (
            "changes.getStartPageToken",
            json!({}),
            // The transport writes an empty query when no query parameter is
            // bound, so the wire request ends in `?`.
            "/drive/v3/changes/startPageToken?",
        ),
        (
            "changes.list",
            json!({"pageToken": "fixture-start-100", "pageSize": 2}),
            "/drive/v3/changes?pageSize=2&pageToken=fixture-start-100",
        ),
    ]
}

#[test]
fn each_read_sends_the_declared_request_with_the_exchanged_bearer_and_returns_the_recorded_body() {
    let provider = Provider::new();
    let mut child = Child::spawn(&provider.selection()).unwrap();
    for (operation, input, expected) in first_requests() {
        let before = provider.api_targets().len();
        let result = invoke(&mut child, operation, input);
        let targets = provider.api_targets();
        assert_eq!(targets.len(), before + 1, "`{operation}` requests");
        assert_eq!(targets[before], expected, "`{operation}` request");
        let (method, _, authorization) = provider
            .requests()
            .into_iter()
            .rfind(|(_, target, _)| target == expected)
            .unwrap();
        assert_eq!(method, "GET", "`{operation}` method");
        assert!(
            authorization.as_deref() == Some(&*format!("Bearer {ACCESS_TOKEN}")),
            "`{operation}` does not carry the exchanged bearer"
        );
        assert_eq!(result["status"], 200, "`{operation}` status");
        assert_eq!(result["provenance"]["instance"], "fixture-google-drive");
        if operation == "files.export" {
            // The exported bytes, returned as text, unchanged.
            assert_eq!(result["body"], json!(EXPORTED), "`{operation}` body");
        } else {
            // The engine re-serialises the body, so the recorded answer is
            // compared as JSON, not as bytes.
            assert_eq!(result["body"], recorded(expected), "`{operation}` body");
        }
    }
    // The bearer came from the token route: one exchange, a form POST with no
    // credential header, before any Drive request; the cached token served
    // every read after it.
    let requests = provider.requests();
    let token: Vec<_> = requests.iter().filter(|(_, t, _)| t == "/token").collect();
    assert_eq!(token.len(), 1, "token exchanges");
    assert_eq!(token[0].0, "POST");
    assert!(
        token[0].2.is_none(),
        "credential header on the token request"
    );
    assert_eq!(requests[0].1, "/token", "the exchange comes first");
}

/// `files.get` is metadata only: `alt=media` is not projected, so asking for it
/// is refused before any request.
#[test]
fn files_get_refuses_a_media_download() {
    let provider = Provider::new();
    let mut child = Child::spawn(&provider.selection()).unwrap();
    let outcome = attempt(
        &mut child,
        "files.get",
        &json!({"fileId": "fixture-doc-1", "alt": "media"}),
    );
    assert!(matches!(outcome, Err(Failure::InvalidInput)), "{outcome:?}");
    assert!(provider.api_targets().is_empty(), "a request was sent");
}

/// `changes.list` needs a `pageToken`; without one nothing is sent.
#[test]
fn changes_list_requires_a_page_token() {
    let provider = Provider::new();
    let mut child = Child::spawn(&provider.selection()).unwrap();
    let outcome = attempt(&mut child, "changes.list", &json!({"pageSize": 2}));
    assert!(matches!(outcome, Err(Failure::InvalidInput)), "{outcome:?}");
    assert!(provider.api_targets().is_empty(), "a request was sent");
}

#[test]
fn files_list_walks_two_pages_until_next_page_token_is_absent() {
    let provider = Provider::new();
    let mut child = Child::spawn(&provider.selection()).unwrap();
    let mut input = json!({"q": "trashed = false", "pageSize": 2});
    let mut ids = Vec::new();
    let mut pages = 0;
    loop {
        let body = invoke(&mut child, "files.list", input.clone())["body"].clone();
        pages += 1;
        for file in body["files"].as_array().unwrap() {
            ids.push(file["id"].as_str().unwrap().to_owned());
        }
        match body.get("nextPageToken").and_then(Value::as_str) {
            Some(token) => input["pageToken"] = json!(token),
            None => break,
        }
        assert!(pages < 3, "`files.list` did not stop");
    }
    assert_eq!(pages, 2);
    assert_eq!(ids, ["fixture-doc-1", "fixture-doc-2", "fixture-doc-3"]);
    assert_eq!(
        provider.api_targets(),
        [
            "/drive/v3/files?pageSize=2&q=trashed+%3D+false",
            "/drive/v3/files?pageSize=2&pageToken=fixture-files-page-2&q=trashed+%3D+false",
        ]
    );
}

/// The delta walk: a baseline from `changes.getStartPageToken`, then
/// `changes.list` from it, following `nextPageToken`, until a page carries
/// `newStartPageToken` — the baseline for the next walk.
#[test]
fn changes_list_walks_two_pages_until_new_start_page_token() {
    let provider = Provider::new();
    let mut child = Child::spawn(&provider.selection()).unwrap();
    let baseline =
        invoke(&mut child, "changes.getStartPageToken", json!({}))["body"]["startPageToken"]
            .clone();
    assert_eq!(baseline, "fixture-start-100");
    let mut input = json!({"pageToken": baseline, "pageSize": 2});
    let mut files = Vec::new();
    let mut pages = 0;
    let next_baseline = loop {
        let body = invoke(&mut child, "changes.list", input.clone())["body"].clone();
        pages += 1;
        for change in body["changes"].as_array().unwrap() {
            files.push(change["fileId"].as_str().unwrap().to_owned());
        }
        if let Some(token) = body.get("newStartPageToken").and_then(Value::as_str) {
            assert!(body.get("nextPageToken").is_none());
            break token.to_owned();
        }
        let next = body["nextPageToken"].as_str().expect("a page to follow");
        input["pageToken"] = json!(next);
        assert!(pages < 3, "`changes.list` did not stop");
    };
    assert_eq!(pages, 2);
    assert_eq!(next_baseline, "fixture-start-103");
    assert_eq!(files, ["fixture-doc-1", "fixture-doc-2", "fixture-doc-3"]);
    assert_eq!(
        provider.api_targets(),
        [
            "/drive/v3/changes/startPageToken?",
            "/drive/v3/changes?pageSize=2&pageToken=fixture-start-100",
            "/drive/v3/changes?pageSize=2&pageToken=fixture-changes-page-2",
        ]
    );
}

/// `pageSize` 0 and 1001 are refused as `invalid_input` with nothing sent, as
/// numbers and as strings; 1 and 1000 are sent.
fn page_size_bounds(operation: &str, first: Value) {
    let provider = Provider::new();
    let mut child = Child::spawn(&provider.selection()).unwrap();
    // Exchange the token first, so "nothing sent" counts every request.
    invoke(&mut child, operation, first.clone());
    let mut escaped = Vec::new();
    for size in [json!(0), json!(1001), json!("0"), json!("1001")] {
        let mut input = first.clone();
        input["pageSize"] = size.clone();
        let before = provider.requests().len();
        let outcome = attempt(&mut child, operation, &input);
        let sent = provider.requests().len() != before;
        if sent || !matches!(outcome, Err(Failure::InvalidInput)) {
            escaped.push(format!("`{operation}` pageSize={size} sent={sent}"));
        }
    }
    assert!(
        escaped.is_empty(),
        "not refused before any request: {escaped:#?}"
    );
    for size in [1, 1000] {
        let mut input = first.clone();
        input["pageSize"] = json!(size);
        let before = provider.api_targets().len();
        invoke(&mut child, operation, input);
        let targets = provider.api_targets();
        assert_eq!(targets.len(), before + 1, "`{operation}` {size}");
        assert!(
            targets[before].contains(&format!("pageSize={size}&")),
            "`{operation}` sent {}",
            targets[before]
        );
    }
}

#[test]
fn files_list_page_size_bounds() {
    page_size_bounds("files.list", json!({"q": "trashed = false", "pageSize": 2}));
}

#[test]
fn changes_list_page_size_bounds() {
    page_size_bounds(
        "changes.list",
        json!({"pageToken": "fixture-start-100", "pageSize": 2}),
    );
}

/// The grant of a write-enabled consent: the read scope and the write scope.
fn write_grant() -> String {
    format!("openid {DRIVE_SCOPE} {WRITE_SCOPE}")
}

/// Prepare (the declared preflight included) and commit one write on this
/// child: the transport the host's approval coordinator drives once it holds a
/// verified, spent approval. A refusal in prepare is the `Err`.
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
#[track_caller]
fn applied(result: Result<WriteResult, Failure>, operation: &str) -> Value {
    let result =
        result.unwrap_or_else(|failure| panic!("`{operation}` refused in prepare: {failure:?}"));
    assert_eq!(result.effect, WriteEffect::Applied, "`{operation}`");
    result
        .result
        .unwrap_or_else(|failure| panic!("`{operation}` result: {failure:?}"))
}
fn calls(expected: &[(&str, &str)]) -> Vec<(String, String)> {
    expected
        .iter()
        .map(|(method, target)| ((*method).to_owned(), (*target).to_owned()))
        .collect()
}

/// A metadata-only body for each write, and the input that carries it.
fn folder() -> Value {
    json!({"name": "Fixture folder", "mimeType": "application/vnd.google-apps.folder",
           "parents": ["root"]})
}
fn renamed() -> Value {
    json!({"name": "first, renamed", "description": "fixture description"})
}
fn copied() -> Value {
    json!({"name": "Copy of first", "parents": ["fixture-created-1"]})
}
fn create_input() -> Value {
    json!({"fields": "id,name,mimeType,version", "body": folder()})
}
fn update_input(version: Value) -> Value {
    json!({"fileId": "fixture-doc-1", "version": version, "fields": "id,name,version",
           "body": renamed()})
}
fn copy_input() -> Value {
    json!({"fileId": "fixture-doc-1", "body": copied()})
}

/// Each write sends exactly its request — the update after its one preflight
/// read — with the body as supplied and the exchanged bearer, and returns
/// Drive's answer as applied.
#[test]
fn each_write_sends_exactly_its_request_and_body_and_returns_the_answer() {
    let provider = Provider::writes(&write_grant());
    let mut child = Child::spawn(&provider.selection()).unwrap();
    for (operation, input, expected, body) in [
        (
            "files.create",
            create_input(),
            calls(&[(
                "POST",
                "/drive/v3/files?fields=id%2Cname%2CmimeType%2Cversion",
            )]),
            folder(),
        ),
        (
            "files.update",
            update_input(json!(VERSION)),
            calls(&[
                (
                    "GET",
                    "/drive/v3/files/fixture-doc-1?fields=id%2Cname%2Cversion",
                ),
                (
                    "PATCH",
                    "/drive/v3/files/fixture-doc-1?fields=id%2Cname%2Cversion",
                ),
            ]),
            renamed(),
        ),
        (
            "files.copy",
            copy_input(),
            calls(&[("POST", "/drive/v3/files/fixture-doc-1/copy?")]),
            copied(),
        ),
    ] {
        let before = provider.api_calls().len();
        let carried = provider.bodies().len();
        let result = applied(write(&mut child, operation, &input), operation);
        assert_eq!(
            provider.api_calls()[before..],
            expected[..],
            "`{operation}` requests"
        );
        let (method, target) = expected.last().unwrap().clone();
        assert_eq!(
            provider.bodies()[carried..],
            [(method.clone(), target.clone(), body.clone())],
            "`{operation}` body"
        );
        assert_eq!(result["status"], 200, "`{operation}` status");
        let answer = write_answer(&method, &target, &serde_json::to_vec(&body).unwrap()).unwrap();
        assert_eq!(result["body"], answer, "`{operation}` answer");
        assert_eq!(result["provenance"]["instance"], "fixture-google-drive");
    }
    let bearer = format!("Bearer {ACCESS_TOKEN}");
    assert!(
        provider
            .requests()
            .iter()
            .filter(|(_, target, _)| target != "/token")
            .all(|(_, _, authorization)| authorization.as_deref() == Some(bearer.as_str())),
        "a Drive request without the exchanged bearer"
    );
}

/// A pinned `version` other than the file's current one fails the preflight:
/// the one `files.get` is sent and no PATCH follows. The same input at the
/// current version then writes, so the refusal was the pin.
#[test]
fn files_update_with_a_stale_version_fails_the_preflight_and_sends_no_write() {
    let provider = Provider::writes(&write_grant());
    let mut child = Child::spawn(&provider.selection()).unwrap();
    let preflight = (
        "GET",
        "/drive/v3/files/fixture-doc-1?fields=id%2Cname%2Cversion",
    );
    for stale in [json!("6"), json!("8"), json!(6)] {
        let before = provider.api_calls().len();
        let outcome = write(&mut child, "files.update", &update_input(stale.clone()));
        assert!(
            matches!(outcome, Err(Failure::Forbidden)),
            "version {stale}: {:?}",
            outcome.map(|result| result.effect)
        );
        assert_eq!(
            provider.api_calls()[before..],
            calls(&[preflight])[..],
            "version {stale}"
        );
    }
    assert!(provider.bodies().is_empty(), "a write body was sent");
    applied(
        write(&mut child, "files.update", &update_input(json!(VERSION))),
        "files.update",
    );
    assert_eq!(provider.bodies().len(), 1);
}

/// An update without its pin, with a pin that is not a scalar, or without the
/// `fields` its preflight reads `version` through, is refused as invalid input
/// before any Drive request.
#[test]
fn files_update_without_its_pin_or_fields_is_refused_before_any_request() {
    let provider = Provider::writes(&write_grant());
    let mut child = Child::spawn(&provider.selection()).unwrap();
    let mut unpinned = update_input(json!(VERSION));
    unpinned.as_object_mut().unwrap().remove("version");
    let mut unselected = update_input(json!(VERSION));
    unselected.as_object_mut().unwrap().remove("fields");
    for (what, input) in [
        ("no version", unpinned),
        ("an object version", update_input(json!({"value": VERSION}))),
        ("no fields", unselected),
    ] {
        let outcome = write(&mut child, "files.update", &input);
        assert!(
            matches!(outcome, Err(Failure::InvalidInput)),
            "{what}: {:?}",
            outcome.map(|result| result.effect)
        );
    }
    assert!(provider.api_calls().is_empty(), "a Drive request was sent");
}

/// The read transport never carries a write. On private protocol two each
/// write id is unsupported there; on protocol one its bootstrap lists no write
/// at all and prepare is unsupported. Nothing reaches the fixture, not even a
/// token exchange.
#[test]
fn drive_writes_are_refused_on_the_read_transport_before_any_request() {
    let inputs = [
        ("files.create", create_input()),
        ("files.update", update_input(json!(VERSION))),
        ("files.copy", copy_input()),
    ];
    let two = Provider::writes(&write_grant());
    let mut child = Child::spawn(&two.selection()).unwrap();
    for (operation, input) in &inputs {
        let outcome = attempt(&mut child, operation, input);
        assert!(
            matches!(outcome, Err(Failure::Unsupported)),
            "`{operation}` on protocol two: {outcome:?}"
        );
    }
    let one = Provider::with(documented(true), write_grant(), None);
    let mut child = Child::spawn(&one.selection()).unwrap();
    for (operation, input) in &inputs {
        let outcome = attempt(&mut child, operation, input);
        assert!(
            matches!(outcome, Err(Failure::NotFound)),
            "`{operation}` on protocol one: {outcome:?}"
        );
        let outcome = write(&mut child, operation, input);
        assert!(
            matches!(outcome, Err(Failure::Unsupported)),
            "`{operation}` prepared on protocol one: {:?}",
            outcome.map(|result| result.effect)
        );
    }
    assert!(two.requests().is_empty(), "protocol two sent a request");
    assert!(one.requests().is_empty(), "protocol one sent a request");
}

/// The write configuration asks for `drive.file`, so a consent that granted
/// only the read scope fails validation; one that granted both validates.
#[test]
fn drive_write_config_requires_the_drive_file_scope() {
    let deadline = || connectors_sdk::now_ms() + 30_000;
    let provider = Provider::writes(&format!("openid {DRIVE_SCOPE}"));
    let mut child = Child::spawn(&provider.selection()).unwrap();
    assert!(matches!(
        child.validate(PROFILE, &secret(), deadline()),
        Err(Failure::InsufficientScope)
    ));
    let provider = Provider::writes(&write_grant());
    let mut child = Child::spawn(&provider.selection()).unwrap();
    let baseline = child
        .validate(PROFILE, &secret(), deadline())
        .unwrap_or_else(|failure| panic!("validation {failure:?}"));
    assert_eq!(
        baseline.granted_scopes.unwrap(),
        [
            "openid".to_owned(),
            DRIVE_SCOPE.to_owned(),
            WRITE_SCOPE.to_owned()
        ]
        .into()
    );
}

/// Public RFC 8032 §7.1 test material, never a deployment signing key.
const APPROVAL_SEED: &str = "9d61b19deffd5a60ba844af492ec2cc44449c5697b326919703bac031cae7f60";
const APPROVAL_PUBLIC: &str = "d75a980182b10ab7d54bfed3c964073a0ee172f3daa62325af021a68f707511a";
const APPROVAL_NOW: i64 = 1_789_056_000_000;

struct FixedClock;
impl mutations::Clock for FixedClock {
    fn now(&self) -> mutations::Result<mutations::ClockInterval> {
        Ok(mutations::ClockInterval {
            lower_unix_ms: APPROVAL_NOW,
            upper_unix_ms: APPROVAL_NOW + 2000,
        })
    }
}
/// A key admission that accepts every subject under its key id, so a refusal
/// can only come from the proof itself.
struct Key(approvals::ConfiguredApprovalKey);
impl approvals::CurrentAdmission for &Key {
    fn key(&self) -> &approvals::ConfiguredApprovalKey {
        &self.0
    }
}
impl approvals::ReceiverPolicy for Key {
    type Guard<'a>
        = &'a Key
    where
        Self: 'a;
    fn admit<'a>(&'a self, _: &approvals::Subject, kid: &str) -> approvals::Result<&'a Key> {
        if kid == self.0.kid {
            Ok(self)
        } else {
            Err(approvals::Failure::Refused)
        }
    }
}
impl approvals::IssuancePolicy for Key {
    type Guard<'a>
        = &'a Key
    where
        Self: 'a;
    fn authorize<'a>(
        &'a self,
        subject: &approvals::Subject,
        kid: &str,
    ) -> approvals::Result<&'a Key> {
        approvals::ReceiverPolicy::admit(self, subject, kid)
    }
}

/// The approval subject of a Drive write, as the owner resolves it: the
/// operation's declared contract and profile, and the digest of the whole
/// input (`crates/connectors-host/src/local/owner/approval_issuance.rs`).
fn drive_subject(operation: &str, input: &Value) -> approvals::Subject {
    let engine = Engine::new(&committed_bundle(), BASE, &shipped()).unwrap();
    let declaration = engine
        .declarations(&[Effect::Write])
        .into_iter()
        .find(|o| o.id == operation)
        .unwrap();
    approvals::Subject {
        format: "connectors.approval-subject/v1".into(),
        target: approvals::Target {
            instance: "fixture-google-drive".into(),
            operation: operation.into(),
            connection: "fixture-connection".into(),
            connection_revision: "fixture-connection-revision".into(),
            contract: declaration.contract,
            profile: declaration.profile,
            descriptor_revision: "fixture-descriptor-revision".into(),
            configuration_revision: "fixture-configuration-revision".into(),
        },
        authority: approvals::Authority {
            scope: approvals::Scope {
                tenant: None,
                realm: None,
                caller: "fixture-caller".into(),
                executor: None,
            },
            current_authority: None,
            executor: None,
        },
        origin: approvals::Origin {
            kind: approvals::OriginKind::Direct,
            authority_ref: "fixture-google-drive".into(),
        },
        route: None,
        canonicalization: "adapter-v1-canonical-json".into(),
        input_sha256: connectors_core::digest(input),
        approval_mode: "required".into(),
    }
}

/// An approval issued for one Drive write verifies for exactly that input and
/// is refused for any other: a changed body member, an added body member, a
/// changed pin, target file or query parameter.
#[test]
fn a_drive_write_approval_for_a_different_body_is_refused() {
    use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
    let key = Key(approvals::ConfiguredApprovalKey {
        issuer: "fixture-issuer".into(),
        audience: "fixture-audience".into(),
        kid: "fixture-key".into(),
        public_key: URL_SAFE_NO_PAD.encode(hex::decode(APPROVAL_PUBLIC).unwrap()),
        not_before_unix_ms: 0,
        not_after_unix_ms: APPROVAL_NOW + 1_000_000,
        revoked: false,
    });
    let signer = approvals::Signer::from_seed(
        Secret(hex::decode(APPROVAL_SEED).unwrap()),
        "fixture-key".into(),
    )
    .unwrap();
    let change = |input: &Value, pointer: &str, value: Value| {
        let mut changed = input.clone();
        let (parent, key) = pointer.rsplit_once('/').unwrap();
        changed.pointer_mut(parent).unwrap()[key] = value;
        changed
    };
    let update = update_input(json!(VERSION));
    for (operation, approved, others) in [
        (
            "files.create",
            create_input(),
            vec![
                change(&create_input(), "/body/name", json!("Another folder")),
                change(&create_input(), "/body/parents", json!(["fixture-other"])),
                change(&create_input(), "/body/description", json!("added")),
                change(&create_input(), "/fields", json!("id")),
            ],
        ),
        (
            "files.update",
            update.clone(),
            vec![
                change(&update, "/body/name", json!("renamed otherwise")),
                change(&update, "/body/trashed", json!(true)),
                change(&update, "/version", json!("8")),
                change(&update, "/fileId", json!("fixture-doc-2")),
            ],
        ),
        (
            "files.copy",
            copy_input(),
            vec![
                change(&copy_input(), "/body/name", json!("Another copy")),
                change(&copy_input(), "/body/parents", json!(["fixture-other"])),
                change(&copy_input(), "/fileId", json!("fixture-doc-2")),
            ],
        ),
    ] {
        let subject = drive_subject(operation, &approved);
        let proof = signer.issue(&subject, &key, &FixedClock).unwrap();
        assert!(
            approvals::verify(&proof, &subject, &key, &FixedClock).is_ok(),
            "`{operation}` refused its own approval"
        );
        for other in others {
            assert_ne!(other, approved);
            let outcome =
                approvals::verify(&proof, &drive_subject(operation, &other), &key, &FixedClock);
            assert!(
                matches!(outcome, Err(approvals::Failure::Refused)),
                "`{operation}` accepted its approval for {other}"
            );
        }
    }
}
