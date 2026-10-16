//! Opsgenie alerts, incidents and the alert ids linked to an incident through
//! the catalog provider, read only, over a creation window the caller advances.
//!
//! The shipped selection set is pinned by id and source operation, resolves
//! against the committed bundle compiled from the projection of the pinned
//! Opsgenie Swagger 2.0 document plus one cited added operation, and is cited
//! row by row in `docs/catalog-opsgenie.md`. Each read runs through the
//! provider child against a disposable HTTPS fixture: the exact request (path,
//! query with its `createdAt` window, `Authorization: GenieKey …`) and the
//! returned body bytes are asserted, a window walks two pages to the end the
//! guide documents and the next window starts at the last `createdAt` read, and
//! connecting proves the key with a read and names the configured connection.
//! Every record is synthetic. No live credential and no network.
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
    process::{Command, Output},
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

/// The document declares host `api.opsgenie.com` with no base path, so every
/// operation and the identity read sit below the authority's root.
const BASE: &str = "/";
/// A fictional key, and the header the fixture accepts: the profile's prefix
/// word, one space, the key.
const KEY: &str = "fixture-opsgenie-key";
const HEADER: &str = "GenieKey fixture-opsgenie-key";
const PROFILE: &str = "opsgenie.api-key";
/// The pinned source, its licence, its record and its README, relative to
/// this crate.
const UPSTREAM: &str = "../opsgenie/upstream";
const SOURCE: &str = "swagger.json";
const PROJECTED: &str = "../opsgenie/generated/opsgenie.openapi.json";
const AMENDMENTS: &str = "opsgenie.amendments.json";
const COMMIT: &str = "71c55343d03076e5544c484ed32b37c3fbc54037";
const SOURCE_SHA256: &str = "60901e17d0d4ba28b8d13d3d78f6a5d329b93008c3c1710dfe6a0b6c786ef071";
const SOURCE_BYTES: usize = 659_535;
const LICENSE_SHA256: &str = "b40930bbcf80744c86c46a12bc9da056641d722716c378f5659b9e555ef833e1";
/// The incident API page the added operation cites.
const CITE: &str = "https://docs.opsgenie.com/docs/incident-api#get-associated-alerts";
/// A synthetic window start, 2025-10-09T08:53:20Z, in epoch milliseconds.
const SINCE: u64 = 1_760_000_000_000;

/// The shipped ids, their `operationId` and their path in the bundle. A
/// renamed, dropped or added id fails here.
const SHIPPED: [(&str, &str, &str); 5] = [
    ("alert.get", "getAlert", "/v2/alerts/{identifier}"),
    ("alerts.list", "listAlerts", "/v2/alerts"),
    ("incident.alert_ids", "getAssociatedAlertIds", LINK),
    ("incident.get", "getIncident", "/v1/incidents/{identifier}"),
    ("incidents.list", "ListIncidents", "/v1/incidents/"),
];
const LINK: &str = "/v1/incidents/{identifier}/associated-alert-ids";

fn root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
}
fn shipped() -> Vec<Selection> {
    let file: Value = serde_json::from_slice(
        &fs::read(root().join("providers/opsgenie/operations.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(file["format"], "connectors-catalog-operations/1");
    assert_eq!(file["provider"], "opsgenie");
    serde_json::from_value(file["operations"].clone()).unwrap()
}
fn committed() -> bundle::Bundle {
    bundle::load(&root().join("generated/bundles"), "opsgenie").unwrap()
}

#[test]
fn shipped_opsgenie_selections_are_exactly_the_five_reads() {
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
        assert!(selection.guard.is_none(), "`{id}` needs no approval");
        assert_eq!(engine.effect(id), Some(Effect::Read), "`{id}`");
        let operation = bundle
            .inventory
            .operations
            .iter()
            .find(|o| o.operation_id.as_deref() == Some(operation_id))
            .unwrap_or_else(|| panic!("the bundle lacks `{operation_id}`"));
        assert_eq!(operation.method, "get", "`{id}`");
        assert_eq!(operation.path, path, "`{id}`");
    }
    // Saved searches are withheld from the list of alerts.
    let alerts = selections.iter().find(|s| s.id == "alerts.list").unwrap();
    assert_eq!(
        alerts.withhold,
        ["searchIdentifier", "searchIdentifierType"]
    );
}

#[test]
fn the_pinned_source_is_the_one_the_bundle_and_its_record_name() {
    let upstream = root().join(UPSTREAM);
    let bytes = fs::read(upstream.join(SOURCE)).unwrap();
    assert_eq!(hex::encode(Sha256::digest(&bytes)), SOURCE_SHA256);
    assert_eq!(bytes.len(), SOURCE_BYTES);
    let license = fs::read(upstream.join("LICENSE")).unwrap();
    assert_eq!(hex::encode(Sha256::digest(&license)), LICENSE_SHA256);
    assert!(String::from_utf8_lossy(&license).contains("Apache License"));

    // The bundle is built from the projection, which records the pinned
    // document it was derived from.
    let projected = fs::read(root().join(PROJECTED)).unwrap();
    let index = bundle::read_index(&root().join("generated/bundles")).unwrap();
    let entry = index.find("opsgenie").unwrap();
    assert_eq!(entry.source_sha256, hex::encode(Sha256::digest(&projected)));
    let bundle = committed();
    let derivation = bundle.source.derivation.as_ref().unwrap();
    assert_eq!(derivation.from_file, SOURCE);
    assert_eq!(derivation.from_sha256, SOURCE_SHA256);
    assert_eq!(derivation.from_bytes, SOURCE_BYTES);
    assert_eq!(derivation.format, "swagger/2.0");
    assert_eq!(derivation.projector, "swagger2-openapi/2");
    // The 99 operations of the document's 170 that project, and the one added
    // by amendment; the 71 the projection lists unprojected are among the
    // bundle's 76 unsupported (`bundle_drift.rs` names all of them).
    assert_eq!(entry.operations, 100);
    assert_eq!(entry.unsupported, 76);
    let record: Value = serde_json::from_slice(
        &fs::read(root().join("../opsgenie/generated/opsgenie.openapi.projection.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(record["operation_count"], 170);
    assert_eq!(record["operations"].as_array().unwrap().len(), 99);
    assert_eq!(
        record["unprojected_operations"].as_array().unwrap().len(),
        71
    );
    assert_eq!(record["source_sha256"], SOURCE_SHA256);

    // The amendment file is bound to the projected source the bundle names.
    let amendments = fs::read(upstream.join(AMENDMENTS)).unwrap();
    let applied = bundle.source.amendments.as_ref().unwrap();
    assert_eq!(applied.file_name, AMENDMENTS);
    assert_eq!(applied.sha256, hex::encode(Sha256::digest(&amendments)));
    let file: Value = serde_json::from_slice(&amendments).unwrap();
    assert_eq!(file["source_sha256"], entry.source_sha256.as_str());

    // One record names the document and the licence at one commit.
    let manifest: Value =
        serde_json::from_slice(&fs::read(upstream.join("opsgenie-source-hashes.json")).unwrap())
            .unwrap();
    let records = manifest.as_array().unwrap();
    assert_eq!(records.len(), 2, "the document and its licence");
    for (record, file, sha256, length) in [
        (&records[0], SOURCE, SOURCE_SHA256, bytes.len()),
        (&records[1], "LICENSE", LICENSE_SHA256, license.len()),
    ] {
        assert_eq!(record["file"], file);
        assert_eq!(record["commit"], COMMIT);
        assert_eq!(
            record["url"],
            format!("https://raw.githubusercontent.com/opsgenie/opsgenie-oas/{COMMIT}/{file}")
        );
        assert_eq!(record["sha256"], sha256);
        assert_eq!(record["bytes"], length);
    }
    let readme = fs::read_to_string(upstream.join("README.md")).unwrap();
    for named in [SOURCE_SHA256, LICENSE_SHA256, COMMIT, "Apache-2.0"] {
        assert!(readme.contains(named), "README does not name `{named}`");
    }
}

/// The parameters each shipped read declares, in the bundle's order.
#[test]
fn each_read_declares_the_pinned_parameters() {
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
        names("listAlerts"),
        [
            "query",
            "searchIdentifier",
            "searchIdentifierType",
            "offset",
            "limit",
            "sort",
            "order"
        ]
    );
    assert_eq!(names("getAlert"), ["identifier", "identifierType"]);
    assert_eq!(
        names("ListIncidents"),
        ["query", "offset", "limit", "sort", "order"]
    );
    assert_eq!(names("getIncident"), ["identifier", "identifierType"]);
    assert_eq!(
        names("getAssociatedAlertIds"),
        [
            "identifier",
            "identifierType",
            "offset",
            "direction",
            "limit",
            "order"
        ]
    );
}

#[test]
fn guide_cites_each_operation_its_window_and_its_paging() {
    let guide = fs::read_to_string(root().join("../../docs/catalog-opsgenie.md")).unwrap();
    let rows: Vec<&str> = guide.lines().filter(|l| l.starts_with('|')).collect();
    // Per operation: id, operationId and path, since parameter, paging and
    // the end condition.
    for (id, operation_id, path, since, paging, end) in [
        (
            "alerts.list",
            "listAlerts",
            "/v2/alerts",
            "`query` `createdAt >= <epoch ms>`",
            "`offset` 0..19899, `limit` 1..100",
            "`paging.next` absent",
        ),
        (
            "alert.get",
            "getAlert",
            "/v2/alerts/{identifier}",
            "n/a",
            "single item",
            "n/a",
        ),
        (
            "incidents.list",
            "ListIncidents",
            "/v1/incidents/",
            "`query` `createdAt >= <epoch ms>`",
            "`offset` 0..19899, `limit` 1..100",
            "`paging.next` absent",
        ),
        (
            "incident.get",
            "getIncident",
            "/v1/incidents/{identifier}",
            "n/a",
            "single item",
            "n/a",
        ),
        (
            "incident.alert_ids",
            "getAssociatedAlertIds",
            LINK,
            "n/a",
            "`offset` from 1, `limit` 1..25",
            "fewer ids than `limit`",
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
                && row.contains(since)
                && row.contains(paging)
                && row.contains(end)
        });
        assert!(paged, "no row states the window and paging of `{id}`");
    }
    for stated in [
        // The documented cap on alert paging.
        "lower than 20K",
        // The provider's end of service.
        "2027-04-05",
        // The added operation's citation.
        CITE,
        // At least once: the caller deduplicates by id.
        "deduplicates by `id`",
    ] {
        assert!(
            guide.contains(stated),
            "the guide does not state `{stated}`"
        );
    }
}

/// The `auth` profile the `opsgenie` connector declares
/// (`providers/opsgenie/connector.json`), so the fixture runs the profile the
/// guide's instance uses.
fn documented_auth() -> Value {
    let declaration: Value = serde_json::from_slice(
        &fs::read(root().join("providers/opsgenie/connector.json")).unwrap(),
    )
    .unwrap();
    declaration["profiles"][0].clone()
}

#[test]
fn the_documented_profile_sends_a_geniekey_and_names_the_connection() {
    let auth = documented_auth();
    assert_eq!(auth["profile"], PROFILE);
    assert_eq!(auth["scheme"], Value::Null, "a token profile");
    assert_eq!(auth["header"], "Authorization");
    assert_eq!(auth["bearer"], false);
    assert_eq!(auth["prefix"], "GenieKey");
    assert_eq!(auth["identity"]["path"], "v2/alerts");
    assert_eq!(auth["identity"]["kind"], "opsgenie.integration");
    assert_eq!(auth["identity"]["source"], "configuration");
    assert_eq!(auth["identity"]["subject_pointer"], Value::Null);
}

/// Route and `Authorization` header of each fixture request.
type Requests = Arc<Mutex<Vec<(String, Option<String>)>>>;

fn alert(n: u64, created_at: &str) -> Value {
    json!({"id": format!("fixture-alert-{n}"), "tinyId": n.to_string(),
           "alias": format!("fixture-alias-{n}"), "message": format!("fixture alert {n}"),
           "status": "closed", "acknowledged": false, "isSeen": true, "tags": [],
           "snoozed": false, "count": 1, "lastOccurredAt": created_at,
           "createdAt": created_at, "updatedAt": created_at, "source": "fixture",
           "owner": "", "priority": "P3", "responders": [],
           "integration": {"id": "fixture-integration", "name": "Fixture", "type": "API"}})
}
fn incident(n: u64, created_at: &str) -> Value {
    json!({"id": format!("fixture-incident-{n}"), "tinyId": n.to_string(),
           "message": format!("fixture incident {n}"), "status": "resolved", "tags": [],
           "createdAt": created_at, "updatedAt": created_at, "priority": "P2",
           "ownerTeam": "", "responders": [], "extraProperties": {}})
}
fn answer(data: Value, next: Option<&str>) -> Value {
    let mut paging = json!({"first": "https://api.opsgenie.com/fixture-first"});
    if let Some(next) = next {
        paging["next"] = json!(next);
    }
    json!({"data": data, "paging": paging, "took": 0.012, "requestId": "fixture-request"})
}

/// The three alerts of the first window, two on a full page and one on a
/// short one, and the instant of the last.
const FIRST: &str = "2025-10-09T09:00:00.000Z";
const SECOND: &str = "2025-10-09T09:30:00.000Z";
const LAST: &str = "2025-10-09T10:15:00.000Z";

/// Epoch milliseconds of an RFC 3339 instant, as a caller stores its cursor.
fn epoch_ms(instant: &str) -> u64 {
    let parsed =
        time::OffsetDateTime::parse(instant, &time::format_description::well_known::Rfc3339)
            .unwrap();
    u64::try_from(parsed.unix_timestamp_nanos() / 1_000_000).unwrap()
}

/// Percent- and form-decodes one query component.
fn decode(component: &str) -> String {
    let bytes = component.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'+' => out.push(b' '),
            b'%' if i + 2 < bytes.len() => {
                out.push(u8::from_str_radix(&component[i + 1..i + 3], 16).unwrap());
                i += 2;
            }
            byte => out.push(byte),
        }
        i += 1;
    }
    String::from_utf8(out).unwrap()
}
fn pairs(query: &str) -> Vec<(String, String)> {
    query
        .split('&')
        .filter(|pair| !pair.is_empty())
        .map(|pair| {
            let (name, value) = pair.split_once('=').unwrap_or((pair, ""));
            (decode(name), decode(value))
        })
        .collect()
}

/// The recorded bodies the fixture serves, keyed by route and query; `None`
/// for anything else, which the fixture answers 404.
fn page(path: &str) -> Option<Value> {
    let (route, query) = path.split_once('?').unwrap_or((path, ""));
    let pairs = pairs(query);
    let get = |name: &str| {
        pairs
            .iter()
            .find(|(n, _)| n == name)
            .map(|(_, v)| v.as_str())
    };
    let since = format!("createdAt >= {SINCE}");
    let resumed = format!("createdAt >= {}", epoch_ms(LAST));
    Some(match route {
        // The identity read carries no query.
        "/v2/alerts" if pairs.is_empty() => answer(json!([]), None),
        "/v2/alerts" if get("query") == Some(&since) && get("offset") == Some("2") => {
            answer(json!([alert(3, LAST)]), None)
        }
        "/v2/alerts" if get("query") == Some(&since) => answer(
            json!([alert(1, FIRST), alert(2, SECOND)]),
            Some(
                "https://api.opsgenie.com/v2/alerts?query=createdAt%20%3E%3D%201760000000000&offset=2&limit=2&sort=createdAt&order=asc",
            ),
        ),
        // The next window re-reads the alert at its lower bound.
        "/v2/alerts" if get("query") == Some(&resumed) => answer(json!([alert(3, LAST)]), None),
        "/v2/alerts/fixture-alert-1" => {
            json!({"data": alert(1, FIRST), "took": 0.004, "requestId": "fixture-request"})
        }
        "/v1/incidents" | "/v1/incidents/" if get("query") == Some(&since) => {
            answer(json!([incident(1, FIRST)]), None)
        }
        "/v1/incidents/fixture-incident-1" => {
            json!({"data": incident(1, FIRST), "took": 0.004, "requestId": "fixture-request"})
        }
        "/v1/incidents/fixture-incident-1/associated-alert-ids" => json!({
            "data": ["fixture-alert-1", "fixture-alert-2"],
            "took": 0.019, "requestId": "fixture-request"}),
        _ => return None,
    })
}

struct Provider {
    stop: Option<oneshot::Sender<()>>,
    thread: Option<std::thread::JoinHandle<()>>,
    _root: tempfile::TempDir,
    directory: PathBuf,
    ca: PathBuf,
    port: u16,
    config: PathBuf,
    requests: Requests,
}
impl Provider {
    fn new() -> Self {
        Self::with_auth(documented_auth())
    }
    fn with_auth(auth: Value) -> Self {
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
                        (
                            401,
                            json!({"message": "Could not authenticate", "took": 0.0,
                                   "requestId": "fixture-request"}),
                        )
                    } else {
                        match page(&path) {
                            Some(body) => (200, body),
                            None => (
                                404,
                                json!({"message": "Not found", "took": 0.0,
                                       "requestId": "fixture-request"}),
                            ),
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
        let provider = Self {
            stop: Some(stop),
            thread: Some(thread),
            _root: root,
            directory,
            ca,
            port: address.port(),
            config,
            requests,
        };
        provider.write_config(&provider.config, auth);
        provider
    }
    fn configuration(&self, auth: Value) -> Value {
        json!({
            "format": "connectors-catalog-local/2",
            "instance": "fixture-opsgenie",
            "provider": "opsgenie",
            "bundle_directory": root_path("generated/bundles"),
            "api_base": format!("https://localhost:{}", self.port),
            "ca_file": self.ca,
            "auth": auth,
            "operations_file": root_path("providers/opsgenie/operations.json"),
        })
    }
    fn write_config(&self, path: &Path, auth: Value) {
        private(
            path,
            &serde_json::to_vec(&self.configuration(auth)).unwrap(),
        );
    }
    fn selection(&self) -> Adapter {
        let output = print_bootstrap(&self.config);
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
            instance_id: "fixture-opsgenie".into(),
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
fn print_bootstrap(config: &Path) -> Output {
    Command::new(env!("CARGO_BIN_EXE_connectors-catalog-provider"))
        .arg("--local-config")
        .arg(config)
        .arg("--print-local-bootstrap")
        .output()
        .unwrap()
}
fn root_path(relative: &str) -> PathBuf {
    root().join(relative).canonicalize().unwrap()
}
fn private(path: &Path, bytes: &[u8]) {
    fs::write(path, bytes).unwrap();
    fs::set_permissions(path, fs::Permissions::from_mode(0o600)).unwrap();
}
fn secret() -> Secret {
    Secret(serde_json::to_vec(&json!({"token": KEY})).unwrap())
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
/// One read: the request the fixture saw, its authorization, and the body
/// bytes the provider returned, which must be the served ones.
fn read_once(
    provider: &Provider,
    child: &mut Child,
    operation: &str,
    input: &Value,
) -> (String, Value) {
    let before = provider.requests().len();
    let raw = invoke_raw(child, operation, input);
    let requests = provider.requests();
    assert_eq!(requests.len(), before + 1, "`{operation}` requests");
    let (path, authorization) = requests[before].clone();
    assert_eq!(
        authorization.as_deref(),
        Some(HEADER),
        "`{operation}` authorization"
    );
    let result: Value = serde_json::from_slice(&raw).unwrap();
    assert_eq!(result["status"], 200, "`{operation}` status");
    assert_eq!(result["provenance"]["instance"], "fixture-opsgenie");
    let served = serde_json::to_vec(&page(&path).unwrap()).unwrap();
    let mut field = b"\"body\":".to_vec();
    field.extend_from_slice(&served);
    assert!(
        raw.windows(field.len()).any(|window| window == field),
        "`{operation}` output does not carry the served body bytes"
    );
    (path, result["body"].clone())
}

#[test]
fn connecting_records_the_configured_instance_as_the_identity() {
    let provider = Provider::new();
    let mut child = Child::spawn(&provider.selection()).unwrap();
    let baseline = child.validate(PROFILE, &secret(), deadline()).unwrap();
    assert_eq!(baseline.identity.kind, "opsgenie.integration");
    assert_eq!(baseline.identity.subject, "fixture-opsgenie");
    assert_eq!(baseline.granted_scopes, None);
    let requests = provider.requests();
    assert_eq!(requests.len(), 1);
    // The identity probe carries an empty query.
    assert_eq!(requests[0].0, "/v2/alerts?");
    assert_eq!(requests[0].1.as_deref(), Some(HEADER));
    // A wrong key is answered 401 and refuses the connection.
    let wrong = Secret(serde_json::to_vec(&json!({"token": "wrong-key"})).unwrap());
    assert!(matches!(
        child.validate(PROFILE, &wrong, deadline()),
        Err(Failure::InvalidCredential)
    ));
    assert_eq!(
        provider.requests()[1].1.as_deref(),
        Some("GenieKey wrong-key")
    );
    // A basic entry is not this profile's.
    let basic = Secret(br#"{"account":"a","token":"t"}"#.to_vec());
    assert!(matches!(
        child.validate(PROFILE, &basic, deadline()),
        Err(Failure::InvalidInput)
    ));
}

#[test]
fn alerts_list_sends_the_since_query_with_geniekey_and_returns_the_recorded_body() {
    let provider = Provider::new();
    let mut child = Child::spawn(&provider.selection()).unwrap();
    let (path, body) = read_once(
        &provider,
        &mut child,
        "alerts.list",
        &json!({"query": format!("createdAt >= {SINCE}"), "sort": "createdAt",
                "order": "asc", "offset": 0, "limit": 100}),
    );
    // Declared order: query, offset, limit, sort, order; a space is sent as
    // `+`, the form encoding of a query component.
    assert_eq!(
        path,
        "/v2/alerts?query=createdAt+%3E%3D+1760000000000&offset=0&limit=100&sort=createdAt&order=asc"
    );
    assert_eq!(body["data"][0]["id"], "fixture-alert-1");
}

#[test]
fn alerts_list_holds_sort_and_order_and_bounds_offset_and_limit() {
    let provider = Provider::new();
    let mut child = Child::spawn(&provider.selection()).unwrap();
    let revision = child.bootstrap().descriptor().unwrap().revision;
    let window = |extra: Value| {
        let mut input = json!({"query": format!("createdAt >= {SINCE}"),
                               "sort": "createdAt", "order": "asc"});
        for (key, value) in extra.as_object().unwrap() {
            input[key] = value.clone();
        }
        input
    };
    let mut refused = vec![
        ("alerts.list", window(json!({"sort": "updatedAt"}))),
        ("alerts.list", window(json!({"sort": "lastOccurredAt"}))),
        ("alerts.list", window(json!({"order": "desc"}))),
        ("alerts.list", window(json!({"limit": 0}))),
        ("alerts.list", window(json!({"limit": 101}))),
        ("alerts.list", window(json!({"offset": 19900}))),
        // A saved search is withheld.
        (
            "alerts.list",
            window(json!({"searchIdentifier": "fixture"})),
        ),
        // The walk is always ascending by creation: both are required.
        (
            "alerts.list",
            json!({"query": format!("createdAt >= {SINCE}"), "order": "asc"}),
        ),
        (
            "alerts.list",
            json!({"query": format!("createdAt >= {SINCE}"), "sort": "createdAt"}),
        ),
        ("incidents.list", window(json!({"sort": "tinyId"}))),
        ("incidents.list", window(json!({"order": "desc"}))),
        ("incidents.list", window(json!({"limit": 101}))),
        ("incidents.list", window(json!({"offset": 19900}))),
        // The document requires the query of an incident list.
        (
            "incidents.list",
            json!({"sort": "createdAt", "order": "asc"}),
        ),
        (
            "alert.get",
            json!({"identifier": "fixture-alert-1", "identifierType": "name"}),
        ),
        (
            "incident.get",
            json!({"identifier": "fixture-incident-1", "identifierType": "alias"}),
        ),
        (
            "incident.alert_ids",
            json!({"identifier": "fixture-incident-1", "limit": 26}),
        ),
        (
            "incident.alert_ids",
            json!({"identifier": "fixture-incident-1", "order": "newest"}),
        ),
    ];
    // Each list's largest page and offset are admitted, as a control.
    let admitted = window(json!({"offset": 19899, "limit": 100}));
    refused.retain(|(_, input)| input != &admitted);
    for (operation, input) in refused {
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
    let outcome = child.invoke(
        "alerts.list",
        &revision,
        "one",
        &secret(),
        &serde_json::to_vec(&admitted).unwrap(),
        deadline(),
    );
    assert!(outcome.is_ok(), "{outcome:?}");
    assert_eq!(provider.requests().len(), 1);
}

#[test]
fn a_window_walks_two_pages_and_resumes_from_the_last_created_at() {
    let provider = Provider::new();
    let mut child = Child::spawn(&provider.selection()).unwrap();
    let mut input = json!({"query": format!("createdAt >= {SINCE}"), "sort": "createdAt",
                           "order": "asc", "offset": 0, "limit": 2});
    let mut seen = Vec::new();
    let mut paths = Vec::new();
    loop {
        let body = invoke(&mut child, "alerts.list", &input)["body"].clone();
        paths.push(provider.requests().last().unwrap().0.clone());
        let data = body["data"].as_array().unwrap();
        seen.extend(data.iter().cloned());
        // The documented end: no `paging.next`.
        if body["paging"]["next"].is_null() {
            break;
        }
        assert_eq!(data.len(), 2, "a page with a next page is full");
        input["offset"] = json!(input["offset"].as_u64().unwrap() + 2);
        assert!(paths.len() < 3, "the walk did not end");
    }
    assert_eq!(
        paths,
        [
            "/v2/alerts?query=createdAt+%3E%3D+1760000000000&offset=0&limit=2&sort=createdAt&order=asc",
            "/v2/alerts?query=createdAt+%3E%3D+1760000000000&offset=2&limit=2&sort=createdAt&order=asc",
        ]
    );
    let ids: Vec<&str> = seen.iter().map(|a| a["id"].as_str().unwrap()).collect();
    assert_eq!(
        ids,
        ["fixture-alert-1", "fixture-alert-2", "fixture-alert-3"]
    );
    // The caller stores the last `createdAt` it read, in epoch ms, and sends
    // it back as the next window's lower bound.
    let cursor = epoch_ms(seen.last().unwrap()["createdAt"].as_str().unwrap());
    assert_eq!(cursor, 1_760_004_900_000);
    let next = json!({"query": format!("createdAt >= {cursor}"), "sort": "createdAt",
                      "order": "asc", "offset": 0, "limit": 2});
    let body = invoke(&mut child, "alerts.list", &next)["body"].clone();
    assert_eq!(
        provider.requests().last().unwrap().0,
        "/v2/alerts?query=createdAt+%3E%3D+1760004900000&offset=0&limit=2&sort=createdAt&order=asc"
    );
    // The alert at the bound is read again: the caller deduplicates by id.
    assert_eq!(body["data"][0]["id"], "fixture-alert-3");
    assert!(body["paging"]["next"].is_null());
}

#[test]
fn incidents_list_sends_the_documented_path() {
    let provider = Provider::new();
    let mut child = Child::spawn(&provider.selection()).unwrap();
    let (path, body) = read_once(
        &provider,
        &mut child,
        "incidents.list",
        &json!({"query": format!("createdAt >= {SINCE}"), "sort": "createdAt",
                "order": "asc", "offset": 0, "limit": 100}),
    );
    // The document writes `/v1/incidents/`; the request goes to
    // `/v1/incidents`, the path the incident API reference writes.
    assert_eq!(
        path,
        "/v1/incidents?query=createdAt+%3E%3D+1760000000000&offset=0&limit=100&sort=createdAt&order=asc"
    );
    assert_eq!(body["data"][0]["id"], "fixture-incident-1");
}

#[test]
fn each_single_read_sends_its_identifier_and_returns_the_recorded_body() {
    let provider = Provider::new();
    let mut child = Child::spawn(&provider.selection()).unwrap();
    for (operation, input, expected) in [
        (
            "alert.get",
            json!({"identifier": "fixture-alert-1", "identifierType": "id"}),
            "/v2/alerts/fixture-alert-1?identifierType=id",
        ),
        (
            "incident.get",
            json!({"identifier": "fixture-incident-1", "identifierType": "id"}),
            "/v1/incidents/fixture-incident-1?identifierType=id",
        ),
    ] {
        let (path, body) = read_once(&provider, &mut child, operation, &input);
        assert_eq!(path, expected, "`{operation}`");
        assert_eq!(body["data"]["id"], input["identifier"], "`{operation}`");
    }
}

#[test]
fn incident_alert_ids_is_added_by_a_cited_amendment() {
    // The pinned document lacks the operation; the amendment adds it.
    let projected: Value =
        serde_json::from_slice(&fs::read(root().join(PROJECTED)).unwrap()).unwrap();
    assert!(
        projected["paths"]
            .as_object()
            .unwrap()
            .keys()
            .all(|path| !path.contains("associated")),
        "the pinned document now declares the link"
    );
    let file: Value =
        serde_json::from_slice(&fs::read(root().join(UPSTREAM).join(AMENDMENTS)).unwrap()).unwrap();
    let added: Vec<&Value> = file["amendments"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|a| a.get("add_operation").is_some())
        .collect();
    assert_eq!(added.len(), 1);
    assert_eq!(added[0]["operation_id"], "getAssociatedAlertIds");
    assert_eq!(added[0]["cite"], CITE);
    assert_eq!(added[0]["add_operation"]["method"], "get");
    assert_eq!(added[0]["add_operation"]["path"], LINK);
    let bundle = committed();
    assert_eq!(
        file["source_sha256"],
        bundle.source.source_sha256.as_str(),
        "bound to the pinned source's projection"
    );

    let provider = Provider::new();
    let mut child = Child::spawn(&provider.selection()).unwrap();
    let (path, body) = read_once(
        &provider,
        &mut child,
        "incident.alert_ids",
        &json!({"identifier": "fixture-incident-1", "identifierType": "id", "limit": 25}),
    );
    assert_eq!(
        path,
        "/v1/incidents/fixture-incident-1/associated-alert-ids?identifierType=id&limit=25"
    );
    assert_eq!(body["data"], json!(["fixture-alert-1", "fixture-alert-2"]));
}

#[test]
fn a_prefix_profile_sends_the_word_and_refuses_bearer_with_prefix() {
    let provider = Provider::new();
    // The word, one space and the token: what every request above carried.
    let mut child = Child::spawn(&provider.selection()).unwrap();
    child.validate(PROFILE, &secret(), deadline()).unwrap();
    assert_eq!(
        provider.requests()[0].1.as_deref(),
        Some("GenieKey fixture-opsgenie-key")
    );

    let loads = |auth: Value| -> bool {
        let path = provider.directory.join("variant.json");
        provider.write_config(&path, auth);
        print_bootstrap(&path).status.success()
    };
    let documented = documented_auth();
    let with = |key: &str, value: Value| {
        let mut auth = documented.clone();
        auth[key] = value;
        auth
    };
    assert!(loads(documented.clone()), "the documented profile loads");
    for (refused, why) in [
        (with("bearer", json!(true)), "bearer beside a prefix"),
        (with("prefix", json!("")), "an empty prefix"),
        (
            with("prefix", json!("Genie Key")),
            "a prefix holding a space",
        ),
        (
            with("prefix", json!("GenieKey\t")),
            "a prefix holding a control byte",
        ),
        (with("prefix", json!("Clé")), "a prefix outside ASCII"),
        (
            with("prefix", json!("K".repeat(33))),
            "a prefix longer than 32 bytes",
        ),
        (with("prefix", json!(7)), "a prefix that is not a string"),
        (
            {
                let mut auth = with("scheme", json!("basic"));
                auth["account_label"] = json!("Account");
                auth
            },
            "a prefix on a basic profile",
        ),
    ] {
        assert!(!loads(refused), "{why} was accepted");
    }
    assert!(loads(with("prefix", json!("K".repeat(32)))));

    // The prefix is part of the configuration: dropping it changes the
    // revision. A configuration that never had one prints the bootstrap it
    // printed before the field existed (`gateway_prefix.rs`,
    // `a_configuration_without_a_prefix_prints_the_bootstrap_it_printed_before`).
    let revision = |auth: Value| -> String {
        let path = provider.directory.join("variant.json");
        provider.write_config(&path, auth);
        let output = print_bootstrap(&path);
        assert!(output.status.success());
        serde_json::from_slice::<Value>(&output.stdout).unwrap()["configuration_revision"]
            .as_str()
            .unwrap()
            .to_owned()
    };
    let mut bare = documented.clone();
    bare.as_object_mut().unwrap().remove("prefix");
    let prefixed = revision(documented.clone());
    // Written last, so `variant.json` holds the configuration without a prefix.
    let unprefixed = revision(bare);
    assert_ne!(prefixed, unprefixed);
    // Without the word the key travels bare, and the fixture refuses it.
    let mut adapter = provider.selection();
    adapter.configuration_revision = unprefixed;
    adapter.executable.args[1] = provider
        .directory
        .join("variant.json")
        .to_str()
        .unwrap()
        .into();
    let mut child = Child::spawn(&adapter).unwrap();
    assert!(matches!(
        child.validate(PROFILE, &secret(), deadline()),
        Err(Failure::InvalidCredential)
    ));
    assert_eq!(provider.requests().last().unwrap().1.as_deref(), Some(KEY));
}
