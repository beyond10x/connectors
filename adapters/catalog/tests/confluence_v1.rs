//! Confluence Cloud REST v1 reads through the catalog provider, under the
//! provider id `confluence-v1` (story:confluence-v1-authored-contract).
//!
//! The source is the document this repository writes itself from Atlassian's
//! public v1 reference pages (`adapters/atlassian/contracts/confluence-rest-v1/`);
//! no Atlassian v1 document is vendored. The shipped selection set is pinned by
//! id, source operation and path, matches the reads the Atlassian ESS model
//! names under REST v1, and resolves against the committed bundle compiled from
//! that document. Each read runs through the provider child against a
//! disposable HTTPS fixture answering in the v1 response shape: the exact
//! request (path, query, `Authorization: Basic …`) and the returned body are
//! asserted, and every list is walked through `_links.next` to the page without
//! it, by `start` for the lists and by `cursor` for the two CQL searches. No live
//! credential and no network.
#![cfg(unix)]
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

const PROVIDER: &str = "confluence-v1";
/// The configured `api_base` path: the site's `/wiki` root, as for REST v2.
const BASE: &str = "/wiki";
/// The fictional basic material the Jira and Confluence tests use:
/// `Basic base64("reader@example.test:fixture-jira-token")`.
const ACCOUNT: &str = "reader@example.test";
const TOKEN: &str = "fixture-jira-token";
const HEADER: &str = "Basic cmVhZGVyQGV4YW1wbGUudGVzdDpmaXh0dXJlLWppcmEtdG9rZW4=";
const PROFILE: &str = "atlassian.basic";
/// The authored source, relative to this crate.
const AUTHORED: &str = "../atlassian/contracts/confluence-rest-v1/confluence-rest-v1.yaml";
/// Where every operation's reference page lives.
const REFERENCE: &str = "https://developer.atlassian.com/cloud/confluence/rest/v1/";

/// The shipped ids, their `operationId` in the authored document and the path
/// the bundle records. A renamed, dropped or added id fails here.
const SHIPPED: [(&str, &str, &str); 7] = [
    ("content.search", "searchByCQL", "/wiki/rest/api/search"),
    (
        "page.comments",
        "getContentComments",
        "/wiki/rest/api/content/{id}/child/comment",
    ),
    ("page.get", "getContentById", "/wiki/rest/api/content/{id}"),
    (
        "pages.changed",
        "searchContentByCQL",
        "/wiki/rest/api/content/search",
    ),
    ("space.get", "getSpace", "/wiki/rest/api/space/{spaceKey}"),
    ("space.pages", "getContent", "/wiki/rest/api/content"),
    ("spaces.list", "getSpaces", "/wiki/rest/api/space"),
];

/// The two CQL queries the fixtures answer, and the form each travels in.
const CHANGED_CQL: &str =
    "space = DOCS and type = page and lastmodified >= \"2026-09-01\" order by lastmodified desc";
const CHANGED_SENT: &str = "space+%3D+DOCS+and+type+%3D+page+and+lastmodified+%3E%3D+%222026-09-01%22+order+by+lastmodified+desc";
const SEARCH_CQL: &str = "type = page and text ~ \"release notes\"";
const SEARCH_SENT: &str = "type+%3D+page+and+text+%7E+%22release+notes%22";

fn root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
}
fn bundles() -> PathBuf {
    root().join("generated/bundles")
}
fn selection_file(provider: &str) -> Value {
    let file: Value = serde_json::from_slice(
        &fs::read(
            root()
                .join("providers")
                .join(provider)
                .join("operations.json"),
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(file["format"], "connectors-catalog-operations/1");
    assert_eq!(file["provider"], provider);
    file
}
fn shipped() -> Vec<Selection> {
    serde_json::from_value(selection_file(PROVIDER)["operations"].clone()).unwrap()
}
fn authored() -> Value {
    connectors_catalog::parse_document(&fs::read(root().join(AUTHORED)).unwrap()).unwrap()
}

#[test]
fn shipped_confluence_v1_selections_are_exactly_the_seven_reads() {
    let selections = shipped();
    let bundle = bundle::load(&bundles(), PROVIDER).unwrap();
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
        let operation = bundle
            .inventory
            .operations
            .iter()
            .find(|o| o.operation_id.as_deref() == Some(operation_id))
            .unwrap_or_else(|| panic!("the authored source lacks `{operation_id}`"));
        assert_eq!(operation.method, "get", "`{id}`");
        assert_eq!(operation.path, path, "`{id}`");
    }
    // The authored document describes the shipped reads and nothing else.
    assert_eq!(bundle.inventory.operations.len(), SHIPPED.len());
    assert!(bundle.inventory.unsupported.is_empty());
}

/// The ids shared with REST v2 mean the same read there, so a caller's id is
/// the same whichever API serves it; CQL search lives under v1 only.
#[test]
fn shared_ids_match_the_v2_selection_set_and_search_is_v1_only() {
    let v2: Vec<String> = selection_file("confluence")["operations"]
        .as_array()
        .unwrap()
        .iter()
        .map(|s| s["id"].as_str().unwrap().to_owned())
        .collect();
    for id in ["pages.changed", "space.pages", "page.get", "page.comments"] {
        assert!(v2.iter().any(|v| v == id), "`{id}` is not a v2 id");
        assert!(SHIPPED.iter().any(|(v1, _, _)| *v1 == id), "`{id}` not v1");
    }
    for id in ["content.search", "spaces.list", "space.get"] {
        assert!(!v2.iter().any(|v| v == id), "`{id}` is also a v2 id");
    }
    let v2_bundle = bundle::load(&bundles(), "confluence").unwrap();
    assert!(
        !v2_bundle
            .inventory
            .operations
            .iter()
            .any(|o| o.path.starts_with("/wiki/rest/api/search"))
    );
}

/// The variant wire names of one enum in the Atlassian documents domain.
fn model_wires(name: &str) -> Vec<String> {
    let domain = connectors_catalog::parse_document(
        &fs::read(root().join("../atlassian/spec/ess/domains/documents.yaml")).unwrap(),
    )
    .unwrap();
    let ty = domain["types"]
        .as_array()
        .unwrap()
        .iter()
        .find(|t| t["name"] == format!("connectors_atlassian.documents.{name}"))
        .unwrap_or_else(|| panic!("the model lacks `{name}`"));
    assert_eq!(ty["kind"], "enum", "`{name}`");
    let mut wires: Vec<String> = ty["variants"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v["wire"].as_str().unwrap().to_owned())
        .collect();
    wires.sort();
    wires
}
fn selection_ids(provider: &str) -> Vec<String> {
    let mut ids: Vec<String> = selection_file(provider)["operations"]
        .as_array()
        .unwrap()
        .iter()
        .map(|s| {
            assert_eq!(s["effect"], "read", "`{provider}` {}", s["id"]);
            s["id"].as_str().unwrap().to_owned()
        })
        .collect();
    ids.sort();
    ids
}

/// The ESS model of the Atlassian documents contract says which reads exist
/// under each REST API, and each selection set serves exactly those.
#[test]
fn the_atlassian_model_names_the_reads_each_rest_api_serves() {
    assert_eq!(model_wires("ConfluenceRestApi"), ["confluence", PROVIDER]);
    assert_eq!(model_wires("ConfluenceV1Read"), selection_ids(PROVIDER));
    assert_eq!(model_wires("ConfluenceV2Read"), selection_ids("confluence"));
    let index = bundle::read_index(&bundles()).unwrap();
    for provider in model_wires("ConfluenceRestApi") {
        assert!(index.find(&provider).is_some(), "`{provider}` not indexed");
    }
}

/// Each operation of the authored document names the v1 reference page it was
/// written from, and the document is this repository's, not Atlassian's.
#[test]
fn the_authored_document_names_each_reference_page_and_copies_none() {
    let document = authored();
    assert_eq!(
        document["info"]["license"]["identifier"],
        "MIT OR Apache-2.0"
    );
    assert!(document["info"].get("termsOfService").is_none());
    let mut operations = 0;
    for (path, item) in document["paths"].as_object().unwrap() {
        for (method, operation) in item.as_object().unwrap() {
            operations += 1;
            let url = operation["externalDocs"]["url"]
                .as_str()
                .unwrap_or_else(|| {
                    panic!("`{method} {path}` names no reference page");
                });
            assert!(url.starts_with(REFERENCE), "`{method} {path}`: {url}");
            assert!(
                url.contains("#api-wiki-rest-api-"),
                "`{method} {path}`: {url}"
            );
        }
    }
    assert_eq!(operations, SHIPPED.len());
    // No Atlassian v1 document is vendored anywhere under the adapter, and the
    // authored one is small enough to have been written by hand.
    let atlassian = root().join("../atlassian");
    assert!(!atlassian.join("upstream/confluence-v1").exists());
    let bytes = fs::read(root().join(AUTHORED)).unwrap();
    assert!(bytes.len() < 32 * 1024, "{} bytes", bytes.len());
}

/// The bundle and its index entry record the authored document's exact bytes.
#[test]
fn the_bundle_records_the_authored_document() {
    let bytes = fs::read(root().join(AUTHORED)).unwrap();
    let digest = hex::encode(Sha256::digest(&bytes));
    let index = bundle::read_index(&bundles()).unwrap();
    let entry = index.find(PROVIDER).unwrap();
    assert_eq!(entry.source_sha256, digest);
    assert_eq!(entry.operations, SHIPPED.len());
    assert_eq!(entry.unsupported, 0);
    let bundle = bundle::load(&bundles(), PROVIDER).unwrap();
    assert_eq!(bundle.source.file_name, "confluence-rest-v1.yaml");
    assert_eq!(bundle.source.source_bytes, bytes.len());
    assert_eq!(bundle.source.license.as_deref(), Some("MIT OR Apache-2.0"));
}

/// Every list bounds `limit` at 1..=100, the selection set's own choice: the
/// reference states no maximum the authored document could repeat.
#[test]
fn every_v1_list_bounds_limit() {
    let mut bounded = Vec::new();
    for selection in shipped() {
        let written = serde_json::to_value(&selection).unwrap();
        if written["bounds"].is_null() {
            continue;
        }
        assert_eq!(
            written["bounds"],
            json!({"limit": {"minimum": 1, "maximum": 100}}),
            "`{}`",
            selection.id
        );
        bounded.push(selection.id.clone());
    }
    bounded.sort();
    assert_eq!(
        bounded,
        [
            "content.search",
            "page.comments",
            "pages.changed",
            "space.pages",
            "spaces.list"
        ]
    );
}

#[test]
fn guide_cites_each_v1_read_its_request_and_its_paging() {
    let guide = fs::read_to_string(root().join("../../docs/catalog-confluence.md")).unwrap();
    let rows: Vec<&str> = guide.lines().filter(|l| l.starts_with('|')).collect();
    for (id, operation_id, path) in SHIPPED {
        let paging = match id {
            "space.get" | "page.get" => "single item",
            "pages.changed" | "content.search" => "`cursor`, `limit`",
            _ => "`start`, `limit`",
        };
        let cited = rows.iter().any(|row| {
            row.contains(&format!("`{id}`"))
                && row.contains(&format!("`{operation_id}`"))
                && row.contains(&format!("`GET {path}`"))
                && row.contains(paging)
        });
        assert!(
            cited,
            "no row cites `{id}` as `{operation_id}` `{path}` {paging}"
        );
    }
}

/// The `auth` profile the connector `provider` declares
/// (`providers/<provider>/connector.json`), which the guide's instance uses.
fn documented_auth(provider: &str) -> Value {
    let declaration: Value = serde_json::from_slice(
        &fs::read(
            root()
                .join("providers")
                .join(provider)
                .join("connector.json"),
        )
        .unwrap(),
    )
    .unwrap();
    declaration["profiles"][0].clone()
}

/// REST v1 is read with the same Atlassian credential and profile as REST v2
/// and Jira; only its access probe is a v1 read.
#[test]
fn the_v1_configuration_uses_the_shared_profile() {
    let v1 = documented_auth(PROVIDER);
    let v2 = documented_auth("confluence");
    assert_eq!(v1["profile"], PROFILE);
    assert_eq!(v1["access"]["path"], "rest/api/space");
    let without_access = |auth: &Value| {
        let mut auth = auth.clone();
        auth["access"] = Value::Null;
        auth
    };
    assert_eq!(without_access(&v1), without_access(&v2));
}

/// Route and `Authorization` header of each fixture request.
type Requests = Arc<Mutex<Vec<(String, Option<String>)>>>;

fn links(next: Option<&str>) -> Value {
    let mut links = json!({"base": "https://site.example.test/wiki", "context": "/wiki"});
    if let Some(next) = next {
        links["next"] = json!(next);
    }
    links
}
fn space(id: u64, key: &str) -> Value {
    json!({"id": id, "key": key, "name": format!("fixture space {key}"), "type": "global",
           "status": "current", "_links": {"webui": format!("/spaces/{key}")}})
}
fn content(id: u64, kind: &str, when: &str, body: bool) -> Value {
    let mut item = json!({
        "id": id.to_string(), "type": kind, "status": "current",
        "title": format!("fixture {kind} {id}"),
        "space": {"key": "DOCS", "name": "fixture space DOCS"},
        "version": {"number": 4, "when": when},
        "_links": {"webui": format!("/spaces/DOCS/pages/{id}")}});
    if body {
        item["body"] = json!({"storage": {"value": format!("<p>{kind} {id}</p>"),
                                          "representation": "storage"}});
    }
    item
}
fn page_of(results: Vec<Value>, start: u64, next: Option<&str>) -> Value {
    json!({"results": results.clone(), "start": start, "limit": 2, "size": results.len(),
           "_links": links(next)})
}
fn hit(id: u64) -> Value {
    json!({"content": content(id, "page", "2026-09-20T10:00:00.000Z", false),
           "title": format!("fixture page {id}"), "excerpt": "the @@@hl@@@release notes@@@endhl@@@",
           "url": format!("/spaces/DOCS/pages/{id}"), "entityType": "content",
           "lastModified": "2026-09-20T10:00:00.000Z"})
}
fn search_page(results: Vec<Value>, start: u64, next: Option<&str>) -> Value {
    let mut page = page_of(results, start, next);
    page["totalSize"] = json!(3);
    page["cqlQuery"] = json!(SEARCH_CQL);
    page
}

/// One read under test: its input, the requests a walk to the end sends, and
/// the parameter its next page is taken by (`None` for a single item).
struct Read {
    id: &'static str,
    input: Value,
    requests: Vec<String>,
    paging: Option<&'static str>,
    items: usize,
}

fn reads() -> Vec<Read> {
    vec![
        Read {
            id: "spaces.list",
            input: json!({"limit": 2}),
            requests: vec![
                "/wiki/rest/api/space?limit=2".into(),
                "/wiki/rest/api/space?start=2&limit=2".into(),
            ],
            paging: Some("start"),
            items: 3,
        },
        Read {
            id: "space.get",
            input: json!({"spaceKey": "DOCS"}),
            // A request without query pairs ends in `?`, as on every provider.
            requests: vec!["/wiki/rest/api/space/DOCS?".into()],
            paging: None,
            items: 1,
        },
        Read {
            id: "space.pages",
            input: json!({"spaceKey": "DOCS", "type": "page", "limit": 2}),
            requests: vec![
                "/wiki/rest/api/content?spaceKey=DOCS&type=page&limit=2".into(),
                "/wiki/rest/api/content?spaceKey=DOCS&type=page&start=2&limit=2".into(),
            ],
            paging: Some("start"),
            items: 3,
        },
        Read {
            id: "pages.changed",
            input: json!({"cql": CHANGED_CQL, "expand": "version", "limit": 2}),
            requests: vec![
                format!("/wiki/rest/api/content/search?cql={CHANGED_SENT}&expand=version&limit=2"),
                format!(
                    "/wiki/rest/api/content/search?cql={CHANGED_SENT}&expand=version&cursor=fixture-cursor-2&limit=2"
                ),
            ],
            paging: Some("cursor"),
            items: 3,
        },
        Read {
            id: "page.get",
            input: json!({"id": "1001", "expand": "body.storage"}),
            requests: vec!["/wiki/rest/api/content/1001?expand=body.storage".into()],
            paging: None,
            items: 1,
        },
        Read {
            id: "page.comments",
            input: json!({"id": "1001", "expand": "body.storage", "location": "footer",
                          "limit": 2}),
            requests: vec![
                "/wiki/rest/api/content/1001/child/comment?expand=body.storage&location=footer&limit=2".into(),
                "/wiki/rest/api/content/1001/child/comment?expand=body.storage&location=footer&start=2&limit=2".into(),
            ],
            paging: Some("start"),
            items: 3,
        },
        Read {
            id: "content.search",
            input: json!({"cql": SEARCH_CQL, "limit": 2}),
            requests: vec![
                format!("/wiki/rest/api/search?cql={SEARCH_SENT}&limit=2"),
                format!("/wiki/rest/api/search?cql={SEARCH_SENT}&cursor=fixture-cursor-2&limit=2"),
            ],
            paging: Some("cursor"),
            items: 3,
        },
    ]
}

/// The recorded answers, keyed by exact request. `None` for anything else,
/// which the fixture answers 404. Each `_links.next` is written the way the v1
/// API writes it: relative to the `/wiki` root, with the paging pairs of the
/// next page among the request's own.
fn served(request: &str) -> Option<Value> {
    let reads = reads();
    let at = |id: &str, n: usize| -> String {
        reads.iter().find(|r| r.id == id).unwrap().requests[n].clone()
    };
    let when = |day: u8| format!("2026-09-{day:02}T10:00:00.000Z");
    Some(match request {
        r if r == at("spaces.list", 0) => page_of(
            vec![space(65538, "DOCS"), space(98305, "ENG")],
            0,
            Some("/rest/api/space?limit=2&start=2"),
        ),
        r if r == at("spaces.list", 1) => page_of(vec![space(131073, "OPS")], 2, None),
        r if r == at("space.get", 0) => space(65538, "DOCS"),
        r if r == at("space.pages", 0) => page_of(
            vec![
                content(1001, "page", &when(20), false),
                content(1002, "page", &when(18), false),
            ],
            0,
            Some("/rest/api/content?spaceKey=DOCS&type=page&limit=2&start=2"),
        ),
        r if r == at("space.pages", 1) => {
            page_of(vec![content(1003, "page", &when(12), false)], 2, None)
        }
        r if r == at("pages.changed", 0) => page_of(
            vec![
                content(1001, "page", &when(20), false),
                content(1002, "page", &when(18), false),
            ],
            0,
            Some(&format!(
                "/rest/api/content/search?next=true&cursor=fixture-cursor-2&expand=version&limit=2&start=2&cql={CHANGED_SENT}"
            )),
        ),
        r if r == at("pages.changed", 1) => {
            page_of(vec![content(1003, "page", &when(12), false)], 2, None)
        }
        r if r == at("page.get", 0) => content(1001, "page", &when(20), true),
        r if r == at("page.comments", 0) => page_of(
            vec![
                content(5001, "comment", &when(10), true),
                content(5002, "comment", &when(11), true),
            ],
            0,
            Some(
                "/rest/api/content/1001/child/comment?expand=body.storage&location=footer&limit=2&start=2",
            ),
        ),
        r if r == at("page.comments", 1) => {
            page_of(vec![content(5003, "comment", &when(12), true)], 2, None)
        }
        r if r == at("content.search", 0) => search_page(
            vec![hit(1001), hit(1002)],
            0,
            Some(&format!(
                "/rest/api/search?next=true&cursor=fixture-cursor-2&limit=2&start=2&cql={SEARCH_SENT}"
            )),
        ),
        r if r == at("content.search", 1) => search_page(vec![hit(1003)], 2, None),
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
                        match served(&path) {
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
                "instance": "fixture-confluence-v1",
                "provider": PROVIDER,
                "bundle_directory": root_path("generated/bundles"),
                "api_base": format!("https://localhost:{}/wiki", address.port()),
                "ca_file": ca,
                "auth": documented_auth(PROVIDER),
                "operations_file": root_path("providers/confluence-v1/operations.json"),
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
            instance_id: "fixture-confluence-v1".into(),
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
fn try_invoke(child: &mut Child, operation: &str, input: &Value) -> Result<Value, Failure> {
    let revision = child.bootstrap().descriptor().unwrap().revision;
    child
        .invoke(
            operation,
            &revision,
            "one",
            &secret(),
            &serde_json::to_vec(input).unwrap(),
            connectors_sdk::now_ms() + 30_000,
        )
        .map(|output| serde_json::from_slice(&output).unwrap())
}
fn invoke(child: &mut Child, operation: &str, input: &Value) -> Value {
    try_invoke(child, operation, input)
        .unwrap_or_else(|failure| panic!("`{operation}` failed: {failure:?}"))
}

/// The value of `name` in the query of a page's `_links.next`, or `None` when
/// the link is absent: the end condition of every v1 list, read from the body.
fn next_value(body: &Value, name: &str) -> Option<String> {
    let link = body.get("_links")?.get("next")?.as_str()?;
    let (_, query) = link.split_once('?')?;
    query
        .split('&')
        .find_map(|pair| pair.strip_prefix(&format!("{name}=")))
        .map(str::to_owned)
}

#[test]
fn each_v1_read_sends_the_declared_request_and_returns_the_recorded_body() {
    let provider = Provider::new();
    let mut child = Child::spawn(&provider.selection()).unwrap();
    for read in reads() {
        let before = provider.requests().len();
        let outcome = try_invoke(&mut child, read.id, &read.input);
        let requests = provider.requests();
        let result = outcome.unwrap_or_else(|failure| {
            panic!(
                "`{}` failed: {failure:?} after {:?}",
                read.id,
                requests.get(before).map(|(path, _)| path)
            )
        });
        assert_eq!(requests.len(), before + 1, "`{}` requests", read.id);
        assert_eq!(
            requests[before].0, read.requests[0],
            "`{}` request",
            read.id
        );
        assert_eq!(
            requests[before].1.as_deref(),
            Some(HEADER),
            "`{}` authorization",
            read.id
        );
        assert_eq!(result["status"], 200, "`{}` status", read.id);
        assert_eq!(
            Some(&result["body"]),
            served(&read.requests[0]).as_ref(),
            "`{}` body",
            read.id
        );
        assert_eq!(result["provenance"]["instance"], "fixture-confluence-v1");
    }
    let page = invoke(
        &mut child,
        "page.get",
        &json!({"id": "1001", "expand": "body.storage"}),
    );
    assert_eq!(page["body"]["body"]["storage"]["value"], "<p>page 1001</p>");
    let space = invoke(&mut child, "space.get", &json!({"spaceKey": "DOCS"}));
    assert_eq!(space["body"]["key"], "DOCS");
}

/// Every list is walked through `_links.next` to the page without it: the lists
/// by the `start` the link carries, the CQL searches by its `cursor`.
#[test]
fn each_v1_list_walks_its_links_to_the_end() {
    let provider = Provider::new();
    let mut child = Child::spawn(&provider.selection()).unwrap();
    for read in reads() {
        let Some(paging) = read.paging else {
            continue;
        };
        let before = provider.requests().len();
        let mut input = read.input.clone();
        let mut items = 0;
        let mut pages = 0;
        loop {
            let body = invoke(&mut child, read.id, &input)["body"].clone();
            pages += 1;
            items += body["results"].as_array().unwrap().len();
            match next_value(&body, paging) {
                Some(value) if paging == "start" => {
                    input["start"] = json!(value.parse::<u64>().unwrap())
                }
                Some(value) => input[paging] = json!(value),
                None => break,
            }
            assert!(pages < 3, "`{}` did not stop", read.id);
        }
        assert_eq!(pages, 2, "`{}` pages", read.id);
        assert_eq!(items, read.items, "`{}` items", read.id);
        let requests: Vec<String> = provider.requests()[before..]
            .iter()
            .map(|(path, _)| path.clone())
            .collect();
        assert_eq!(requests, read.requests, "`{}` requests", read.id);
    }
}

/// A `limit` outside 1..=100 is refused before any request on each list.
#[test]
fn a_limit_outside_the_bound_is_refused_before_any_request() {
    let provider = Provider::new();
    let mut child = Child::spawn(&provider.selection()).unwrap();
    let mut escaped = Vec::new();
    for read in reads().into_iter().filter(|r| r.paging.is_some()) {
        for limit in [json!(101), json!("101"), json!(0), json!(-1)] {
            let mut input = read.input.clone();
            input["limit"] = limit.clone();
            let before = provider.requests().len();
            let outcome = try_invoke(&mut child, read.id, &input);
            let sent = provider.requests().len() != before;
            if sent || !matches!(outcome, Err(Failure::InvalidInput)) {
                escaped.push(format!("`{}` limit={limit} sent={sent}", read.id));
            }
        }
    }
    assert!(escaped.is_empty(), "not refused: {escaped:#?}");
}

/// Both CQL reads require the query; one without it is refused before any request.
#[test]
fn a_cql_read_without_its_query_is_refused_before_any_request() {
    let provider = Provider::new();
    let mut child = Child::spawn(&provider.selection()).unwrap();
    for id in ["content.search", "pages.changed"] {
        let before = provider.requests().len();
        let outcome = try_invoke(&mut child, id, &json!({"limit": 2}));
        assert!(matches!(outcome, Err(Failure::InvalidInput)), "`{id}`");
        assert_eq!(provider.requests().len(), before, "`{id}` sent");
    }
}
