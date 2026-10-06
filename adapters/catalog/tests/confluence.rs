//! Confluence Cloud pages, their bodies and their footer comments through the
//! catalog provider.
//!
//! The shipped selection set is pinned by id and source operation, resolves
//! against the committed bundle compiled from the pinned REST v2 document, and
//! is cited row by row in `docs/catalog-confluence.md`. Each read runs through
//! the provider child against a disposable HTTPS fixture: the exact request
//! (path, query, `Authorization: Basic …`) and the returned body are asserted,
//! and every list walks two pages to the end condition the guide documents,
//! read from the body alone. The credential and the auth profile are the ones
//! the Jira guide uses. No live credential and no network.
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

/// The path the configured `api_base` carries: the Confluence site root, so
/// that both the v2 reads and the identity read sit below it.
const BASE: &str = "/wiki";
/// The same fictional basic material the Jira test uses: one Atlassian
/// credential, `Basic base64("reader@example.test:fixture-jira-token")`.
const ACCOUNT: &str = "reader@example.test";
const TOKEN: &str = "fixture-jira-token";
const HEADER: &str = "Basic cmVhZGVyQGV4YW1wbGUudGVzdDpmaXh0dXJlLWppcmEtdG9rZW4=";
const PROFILE: &str = "atlassian.basic";
/// The pinned source, its manifest and README, relative to this crate.
const UPSTREAM: &str = "../atlassian/upstream/confluence";

/// The shipped ids, their pinned `operationId` and the path the bundle records
/// (the document's path below its server path `/wiki/api/v2`). A renamed,
/// dropped or added id fails here.
const SHIPPED: [(&str, &str, &str); 4] = [
    (
        "page.comments",
        "getPageFooterComments",
        "/wiki/api/v2/pages/{id}/footer-comments",
    ),
    ("page.get", "getPageById", "/wiki/api/v2/pages/{id}"),
    ("pages.changed", "getPages", "/wiki/api/v2/pages"),
    (
        "space.pages",
        "getPagesInSpace",
        "/wiki/api/v2/spaces/{id}/pages",
    ),
];

fn root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
}
fn shipped() -> Vec<Selection> {
    let file: Value = serde_json::from_slice(
        &fs::read(root().join("providers/confluence/operations.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(file["format"], "connectors-catalog-operations/1");
    assert_eq!(file["provider"], "confluence");
    serde_json::from_value(file["operations"].clone()).unwrap()
}
fn pinned() -> Value {
    serde_json::from_slice(&fs::read(root().join(UPSTREAM).join("confluence-v2.json")).unwrap())
        .unwrap()
}

#[test]
fn shipped_confluence_selections_are_exactly_the_four_reads() {
    let selections = shipped();
    let bundle = bundle::load(&root().join("generated/bundles"), "confluence").unwrap();
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
fn a_selection_the_pinned_source_lacks_is_refused_at_load() {
    let bundle = bundle::load(&root().join("generated/bundles"), "confluence").unwrap();
    let mut selections = shipped();
    // CQL search lives only in the v1 document, which is not pinned.
    selections[0].operation_id = "searchByCQL".into();
    assert!(Engine::new(&bundle, BASE, &selections).is_err());
}

#[test]
fn the_pinned_source_is_the_one_the_bundle_and_its_record_name() {
    let upstream = root().join(UPSTREAM);
    let bytes = fs::read(upstream.join("confluence-v2.json")).unwrap();
    let digest = hex::encode(Sha256::digest(&bytes));
    let index = bundle::read_index(&root().join("generated/bundles")).unwrap();
    let entry = index.find("confluence").unwrap();
    assert_eq!(entry.source_sha256, digest);
    let bundle = bundle::load(&root().join("generated/bundles"), "confluence").unwrap();
    // The document's gaps are writes whose request body is a `$ref`; none is a
    // shipped read, so every shipped read is inventoried whole.
    assert_eq!(entry.unsupported, 30);
    for gap in &bundle.inventory.unsupported {
        assert!(
            SHIPPED.iter().all(|(_, id, _)| gap.designation != *id),
            "shipped `{}` is unsupported",
            gap.designation
        );
        assert_eq!(
            gap.reason,
            "request body is a $ref this pass does not resolve"
        );
    }
    assert_eq!(bundle.source.source_bytes, bytes.len());
    assert_eq!(bundle.source.file_name, "confluence-v2.json");
    let manifest: Value =
        serde_json::from_slice(&fs::read(upstream.join("confluence-source-hashes.json")).unwrap())
            .unwrap();
    let records = manifest.as_array().unwrap();
    assert_eq!(records.len(), 1, "one pinned Confluence document");
    assert_eq!(records[0]["file"], "confluence-v2.json");
    assert_eq!(records[0]["sha256"], digest.as_str());
    assert_eq!(records[0]["bytes"], bytes.len());
    let readme = fs::read_to_string(upstream.join("README.md")).unwrap();
    assert!(readme.contains(&digest), "README does not name the digest");
    let terms = pinned()["info"]["termsOfService"]
        .as_str()
        .unwrap()
        .to_owned();
    assert!(readme.contains(&terms), "README does not name the terms");
}

/// The time window of `pages.changed` is a stop rule on each listed page's
/// last-modification time, so that field must be in the list schema the
/// pinned document gives `getPages`.
#[test]
fn the_cutoff_field_is_in_the_pinned_list_schema() {
    let document = pinned();
    let resolve = |schema: &Value| -> Value {
        match schema.get("$ref").and_then(Value::as_str) {
            Some(reference) => document
                .pointer(reference.trim_start_matches('#'))
                .unwrap()
                .clone(),
            None => schema.clone(),
        }
    };
    let get = &document["paths"]["/pages"]["get"];
    assert_eq!(get["operationId"], "getPages");
    let names: Vec<&str> = get["parameters"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|p| p["name"].as_str())
        .collect();
    for name in ["space-id", "sort", "body-format", "cursor", "limit"] {
        assert!(names.contains(&name), "`getPages` lacks `{name}`");
    }
    let sort = resolve(
        &get["parameters"]
            .as_array()
            .unwrap()
            .iter()
            .find(|p| p["name"] == "sort")
            .unwrap()["schema"],
    );
    assert!(
        sort["enum"]
            .as_array()
            .unwrap()
            .contains(&json!("-modified-date"))
    );
    let list = resolve(&get["responses"]["200"]["content"]["application/json"]["schema"]);
    let page = resolve(&list["properties"]["results"]["items"]);
    let version = resolve(&page["properties"]["version"]);
    assert_eq!(version["properties"]["createdAt"]["format"], "date-time");
    let links = resolve(&list["properties"]["_links"]);
    assert_eq!(links["properties"]["next"]["type"], "string");
}

#[test]
fn guide_cites_each_operation_its_paging_and_its_deltas() {
    let guide = fs::read_to_string(root().join("../../docs/catalog-confluence.md")).unwrap();
    let rows: Vec<&str> = guide.lines().filter(|l| l.starts_with('|')).collect();
    // Per operation: id, pinned operationId and path, paging parameters, end
    // condition, and the time filter or how deltas are taken.
    for (id, operation_id, path, paging, end, deltas) in [
        (
            "pages.changed",
            "getPages",
            "/wiki/api/v2/pages",
            "`cursor`, `limit`",
            "`_links.next` absent or null",
            "`version.createdAt` older than the cutoff",
        ),
        (
            "space.pages",
            "getPagesInSpace",
            "/wiki/api/v2/spaces/{id}/pages",
            "`cursor`, `limit`",
            "`_links.next` absent or null",
            "`pages.changed`",
        ),
        (
            "page.get",
            "getPageById",
            "/wiki/api/v2/pages/{id}",
            "single item",
            "n/a",
            "`pages.changed`",
        ),
        (
            "page.comments",
            "getPageFooterComments",
            "/wiki/api/v2/pages/{id}/footer-comments",
            "`cursor`, `limit`",
            "`_links.next` absent or null",
            "`pages.changed`",
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

/// The `auth` object of the guide's configuration example for `provider`.
fn documented_auth(guide: &str, provider: &str) -> Value {
    let guide = fs::read_to_string(root().join("../../docs").join(guide)).unwrap();
    let example = guide
        .split("```json\n")
        .skip(1)
        .filter_map(|rest| rest.split_once("\n```").map(|(body, _)| body))
        .find(|body| body.contains(&format!("\"provider\": \"{provider}\"")))
        .unwrap_or_else(|| panic!("the documented {provider} configuration"));
    serde_json::from_str::<Value>(example).unwrap()["auth"].clone()
}

/// Jira and Confluence use one profile: the same id, scheme, labels and
/// identity kind, so one credential document connects both. Only the identity
/// read differs, because each product answers it at its own path, and Confluence
/// also declares a v2 page read as its access probe.
#[test]
fn confluence_and_jira_document_one_profile() {
    let confluence = documented_auth("catalog-confluence.md", "confluence");
    let jira = documented_auth("catalog-jira.md", "jira");
    assert_eq!(confluence["profile"], PROFILE);
    assert_eq!(confluence["scheme"], "basic");
    assert_eq!(confluence["identity"]["kind"], "atlassian.account");
    assert_eq!(confluence["identity"]["subject_pointer"], "/accountId");
    let without_identity_path = |auth: &Value| {
        let mut auth = auth.clone();
        auth["identity"]["path"] = Value::Null;
        // Confluence also checks a v2 page read at validation (#102).
        auth["access"] = Value::Null;
        auth
    };
    assert_eq!(
        without_identity_path(&confluence),
        without_identity_path(&jira)
    );
}

/// Route and `Authorization` header of each fixture request.
type Requests = Arc<Mutex<Vec<(String, Option<String>)>>>;

/// A page as the v2 list and single reads return it; the listed form carries
/// the body only when `body-format` asks for it.
fn page_item(id: u64, modified: &str, body: bool) -> Value {
    let mut page = json!({
        "id": id.to_string(), "status": "current", "title": format!("fixture page {id}"),
        "spaceId": "65538", "parentId": "1000", "parentType": "page",
        "authorId": "fixture-author", "createdAt": "2026-08-01T08:00:00.000Z",
        "version": {"number": 3, "message": "", "minorEdit": false,
                    "authorId": "fixture-author", "createdAt": modified},
        "_links": {"webui": format!("/spaces/FIX/pages/{id}")}});
    if body {
        page["body"] = json!({"storage": {"representation": "storage",
                                          "value": format!("<p>page {id}</p>")}});
    }
    page
}
fn comment(id: u64) -> Value {
    json!({"id": id.to_string(), "status": "current", "title": "Re: fixture page 1001",
           "pageId": "1001",
           "version": {"number": 1, "authorId": "fixture-author",
                       "createdAt": "2026-09-10T08:00:00.000Z"},
           "body": {"storage": {"representation": "storage",
                                "value": format!("<p>comment {id}</p>")}}})
}
/// A list body: its results and its links, with `next` as given (a string,
/// JSON null, or absent).
fn list(results: Vec<Value>, next: Option<Value>) -> Value {
    let mut links = json!({"base": "https://site.example.test/wiki"});
    if let Some(next) = next {
        links["next"] = next;
    }
    json!({"results": results, "_links": links})
}

/// The recorded pages the fixture serves, keyed by route and paging position.
/// `None` for anything else, which the fixture answers 404.
fn page(path: &str) -> Option<Value> {
    let (route, query) = path.split_once('?').unwrap_or((path, ""));
    let has = |pair: &str| query.split('&').any(|p| p == pair);
    let second = has("cursor=fixture-page-2");
    Some(match route {
        "/wiki/rest/api/user/current" => {
            json!({"type": "known", "accountId": "fixture-account-id"})
        }
        // Space 65538, newest first: the second page reaches back past the
        // caller's cutoff and still offers a third, which is never served.
        "/wiki/api/v2/pages" if has("space-id=65538") && second => list(
            vec![
                page_item(1003, "2026-09-12T10:00:00.000Z", true),
                page_item(1004, "2026-08-20T10:00:00.000Z", true),
            ],
            Some(json!(
                "/wiki/api/v2/pages?space-id=65538&sort=-modified-date&body-format=storage&cursor=fixture-page-3&limit=2"
            )),
        ),
        "/wiki/api/v2/pages" if has("space-id=65538") => list(
            vec![
                page_item(1001, "2026-09-20T10:00:00.000Z", true),
                page_item(1002, "2026-09-18T10:00:00.000Z", true),
            ],
            Some(json!(
                "/wiki/api/v2/pages?space-id=65538&sort=-modified-date&body-format=storage&cursor=fixture-page-2&limit=2"
            )),
        ),
        // Space 98305: every page is inside the window, one lists no `version`
        // (the pinned schema does not require it), and the walk ends on a null
        // link.
        "/wiki/api/v2/pages" if has("space-id=98305") && second => list(
            vec![page_item(2003, "2026-09-12T10:00:00.000Z", true), {
                let mut unversioned = page_item(2004, "", true);
                unversioned.as_object_mut().unwrap().remove("version");
                unversioned
            }],
            Some(Value::Null),
        ),
        "/wiki/api/v2/pages" if has("space-id=98305") => list(
            vec![
                page_item(2001, "2026-09-20T10:00:00.000Z", true),
                page_item(2002, "2026-09-18T10:00:00.000Z", true),
            ],
            Some(json!(
                "/wiki/api/v2/pages?space-id=98305&sort=-modified-date&body-format=storage&cursor=fixture-page-2&limit=2"
            )),
        ),
        // The last page carries no `next` at all.
        "/wiki/api/v2/spaces/65538/pages" if second => list(
            vec![page_item(1003, "2026-09-12T10:00:00.000Z", false)],
            None,
        ),
        "/wiki/api/v2/spaces/65538/pages" => list(
            vec![
                page_item(1001, "2026-09-20T10:00:00.000Z", false),
                page_item(1002, "2026-09-18T10:00:00.000Z", false),
            ],
            Some(json!(
                "/wiki/api/v2/spaces/65538/pages?cursor=fixture-page-2&limit=2"
            )),
        ),
        "/wiki/api/v2/pages/1001" if has("body-format=storage") => {
            page_item(1001, "2026-09-20T10:00:00.000Z", true)
        }
        // The last page carries a null `next`.
        "/wiki/api/v2/pages/1001/footer-comments" if second => {
            list(vec![comment(3)], Some(Value::Null))
        }
        "/wiki/api/v2/pages/1001/footer-comments" => list(
            vec![comment(1), comment(2)],
            Some(json!(
                "/wiki/api/v2/pages/1001/footer-comments?body-format=storage&cursor=fixture-page-2&limit=2"
            )),
        ),
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
                        (401, json!({"message": "fixture refusal"}))
                    } else {
                        match page(&path) {
                            Some(body) => (200, body),
                            None => (404, json!({"message": "no fixture"})),
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
                "instance": "fixture-confluence",
                "provider": "confluence",
                "bundle_directory": root_path("generated/bundles"),
                "api_base": format!("https://localhost:{}/wiki", address.port()),
                "ca_file": ca,
                "auth": documented_auth("catalog-confluence.md", "confluence"),
                "operations_file": root_path("providers/confluence/operations.json"),
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
            instance_id: "fixture-confluence".into(),
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

/// The pinned document caps `limit` on every shipped list read (its schema
/// states `minimum` 1 and `maximum` 250), so each shipped selection whose
/// source operation takes a `limit` query parameter bounds it at exactly that
/// range, read from the document, and no other selection carries a bound.
#[test]
fn every_shipped_list_bounds_limit_at_the_pinned_maximum() {
    use connectors_catalog::inventory::Location;
    let document = pinned();
    let bundle = bundle::load(&root().join("generated/bundles"), "confluence").unwrap();
    let mut bounded = Vec::new();
    for selection in shipped() {
        let operation = bundle
            .inventory
            .operations
            .iter()
            .find(|o| o.operation_id.as_deref() == Some(selection.operation_id.as_str()))
            .unwrap();
        let takes_limit = operation
            .parameters
            .iter()
            .any(|p| p.name == "limit" && p.location == Location::Query);
        let written = serde_json::to_value(&selection).unwrap();
        if !takes_limit {
            assert_eq!(written["bounds"], Value::Null, "`{}`", selection.id);
            continue;
        }
        bounded.push(selection.id.clone());
        let written_path = operation.path.trim_start_matches("/wiki/api/v2");
        let schema = document["paths"][written_path]["get"]["parameters"]
            .as_array()
            .unwrap()
            .iter()
            .find(|p| p["name"] == "limit")
            .unwrap()["schema"]
            .clone();
        assert_eq!(
            written["bounds"],
            json!({"limit": {"minimum": schema["minimum"], "maximum": schema["maximum"]}}),
            "`{}`",
            selection.id
        );
        assert_eq!(schema["maximum"], 250, "`{}`", selection.id);
    }
    bounded.sort();
    assert_eq!(bounded, ["page.comments", "pages.changed", "space.pages"]);
}

/// A `limit` outside 1..=250 is refused before any request, as a number and
/// as a string, on each list read.
#[test]
fn a_limit_outside_the_pinned_range_is_refused_before_any_request() {
    let provider = Provider::new();
    let mut child = Child::spawn(&provider.selection()).unwrap();
    let revision = child.bootstrap().descriptor().unwrap().revision;
    let mut escaped = Vec::new();
    for (operation, first, _) in first_pages() {
        if operation == "page.get" {
            continue;
        }
        for limit in [json!(251), json!("251"), json!(0), json!("0"), json!(-1)] {
            let mut input = first.clone();
            input["limit"] = limit.clone();
            let before = provider.requests().len();
            let outcome = child.invoke(
                operation,
                &revision,
                "one",
                &secret(),
                &serde_json::to_vec(&input).unwrap(),
                connectors_sdk::now_ms() + 30_000,
            );
            let sent = provider.requests().len() != before;
            if sent || !matches!(outcome, Err(Failure::InvalidInput)) {
                escaped.push(format!("`{operation}` limit={limit} sent={sent}"));
            }
        }
        // The maximum itself is sent.
        let mut input = first.clone();
        input["limit"] = json!(250);
        let before = provider.requests().len();
        let _ = child.invoke(
            operation,
            &revision,
            "one",
            &secret(),
            &serde_json::to_vec(&input).unwrap(),
            connectors_sdk::now_ms() + 30_000,
        );
        assert_eq!(provider.requests().len(), before + 1, "`{operation}` 250");
    }
    assert!(
        escaped.is_empty(),
        "not refused before any request: {escaped:#?}"
    );
}

/// Each read's first request: the input, and the exact request the fixture
/// must observe.
fn first_pages() -> [(&'static str, Value, &'static str); 4] {
    [
        (
            "pages.changed",
            json!({"space-id": "65538", "sort": "-modified-date", "body-format": "storage",
                   "limit": 2}),
            "/wiki/api/v2/pages?space-id=65538&sort=-modified-date&body-format=storage&limit=2",
        ),
        (
            "space.pages",
            json!({"id": "65538", "limit": 2}),
            "/wiki/api/v2/spaces/65538/pages?limit=2",
        ),
        (
            "page.get",
            json!({"id": "1001", "body-format": "storage"}),
            "/wiki/api/v2/pages/1001?body-format=storage",
        ),
        (
            "page.comments",
            json!({"id": "1001", "body-format": "storage", "limit": 2}),
            "/wiki/api/v2/pages/1001/footer-comments?body-format=storage&limit=2",
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
        assert_eq!(result["provenance"]["instance"], "fixture-confluence");
    }
    // The single read asked for the storage format and received the body in it.
    let result = invoke(
        &mut child,
        "page.get",
        json!({"id": "1001", "body-format": "storage"}),
    );
    assert_eq!(
        result["body"]["body"]["storage"]["value"], "<p>page 1001</p>",
        "`page.get` storage body"
    );
    assert!(
        provider
            .requests()
            .last()
            .unwrap()
            .0
            .ends_with("?body-format=storage")
    );
}

/// The `cursor` of a page's `_links.next`, or `None` when the link is absent
/// or null: the documented end condition of every list, read from the body.
fn next_cursor(body: &Value) -> Option<String> {
    let link = body.get("_links")?.get("next")?.as_str()?;
    let (_, query) = link.split_once('?')?;
    query
        .split('&')
        .find_map(|pair| pair.strip_prefix("cursor="))
        .map(str::to_owned)
}

/// Walk one list from `first`, one request per page. `cutoff` applies the
/// `pages.changed` window: keep the pages modified at or after it, and stop at
/// the first page that lists one modified before it; a page without
/// `version.createdAt` is kept and does not stop the walk. Timestamps share the
/// document's one fixed format, so they compare as text.
fn walk(
    child: &mut Child,
    operation: &str,
    first: Value,
    cutoff: Option<&str>,
) -> (usize, Vec<Value>) {
    let mut input = first;
    let mut items = Vec::new();
    let mut pages = 0;
    loop {
        let body = invoke(child, operation, input.clone())["body"].clone();
        pages += 1;
        let mut past = false;
        for item in body["results"].as_array().unwrap() {
            match cutoff {
                // A page without `version.createdAt` counts as changed.
                Some(cutoff)
                    if item["version"]["createdAt"]
                        .as_str()
                        .is_some_and(|modified| modified < cutoff) =>
                {
                    past = true;
                }
                _ => items.push(item.clone()),
            }
        }
        if past {
            break;
        }
        match next_cursor(&body) {
            Some(cursor) => input["cursor"] = json!(cursor),
            None => break,
        }
        assert!(pages < 3, "`{operation}` did not stop");
    }
    (pages, items)
}

#[test]
fn each_list_walks_two_pages_and_stops_at_its_documented_end_condition() {
    let provider = Provider::new();
    let mut child = Child::spawn(&provider.selection()).unwrap();
    // `space.pages` ends on a missing link, `page.comments` on a null one.
    for (operation, first, expected) in first_pages() {
        if !matches!(operation, "space.pages" | "page.comments") {
            continue;
        }
        let before = provider.requests().len();
        let (pages, items) = walk(&mut child, operation, first, None);
        assert_eq!(pages, 2, "`{operation}` pages walked");
        assert_eq!(items.len(), 3, "`{operation}` items");
        let second = expected.replace("&limit=2", "&cursor=fixture-page-2&limit=2");
        let second = second.replace("?limit=2", "?cursor=fixture-page-2&limit=2");
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
fn pages_changed_stops_at_the_cutoff_or_at_the_last_link() {
    let provider = Provider::new();
    let mut child = Child::spawn(&provider.selection()).unwrap();
    let first = |space: &str| json!({"space-id": space, "sort": "-modified-date", "body-format": "storage", "limit": 2});
    let request = |space: &str, cursor: &str| {
        format!(
            "/wiki/api/v2/pages?space-id={space}&sort=-modified-date&body-format=storage{cursor}&limit=2"
        )
    };

    // Space 65538: the second page still links a third, but lists a page
    // modified before the cutoff, so the walk stops there and never asks for
    // the third (the fixture would answer it 404).
    let before = provider.requests().len();
    let (pages, items) = walk(
        &mut child,
        "pages.changed",
        first("65538"),
        Some("2026-09-01T00:00:00.000Z"),
    );
    assert_eq!(pages, 2);
    let ids: Vec<&str> = items.iter().map(|p| p["id"].as_str().unwrap()).collect();
    assert_eq!(ids, ["1001", "1002", "1003"]);
    let requests: Vec<String> = provider.requests()[before..]
        .iter()
        .map(|(path, _)| path.clone())
        .collect();
    assert_eq!(
        requests,
        [
            request("65538", ""),
            request("65538", "&cursor=fixture-page-2")
        ]
    );

    // Space 98305: every page is inside the window; a page without
    // `version.createdAt` counts as changed and does not stop the walk, which
    // ends on the second page's null `_links.next`.
    let before = provider.requests().len();
    let (pages, items) = walk(
        &mut child,
        "pages.changed",
        first("98305"),
        Some("2026-09-01T00:00:00.000Z"),
    );
    assert_eq!(pages, 2);
    let ids: Vec<&str> = items.iter().map(|p| p["id"].as_str().unwrap()).collect();
    assert_eq!(ids, ["2001", "2002", "2003", "2004"]);
    let requests: Vec<String> = provider.requests()[before..]
        .iter()
        .map(|(path, _)| path.clone())
        .collect();
    assert_eq!(
        requests,
        [
            request("98305", ""),
            request("98305", "&cursor=fixture-page-2")
        ]
    );
}
