//! Google Drive reads — about, files, export and changes — through the catalog
//! provider.
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
use connectors_catalog::{bundle, discovery};
use connectors_catalog_provider::{Effect, Engine, ResponseKind, Selection};
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

/// The path the configured `api_base` carries: the projection's one server,
/// `https://www.googleapis.com/drive/v3`, below its host.
const BASE: &str = "/drive/v3";
const PROVIDER: &str = "google-drive";
/// The bundle's auth profile, and the profile of the guide's configuration.
const PROFILE: &str = "google.oauth";
const DRIVE_SCOPE: &str = "https://www.googleapis.com/auth/drive.readonly";
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
fn shipped_drive_selections_are_exactly_the_six_reads() {
    let selections = shipped();
    let bundle = committed_bundle();
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
    // `alt=media` downloads are excluded from the projection, and no method
    // named for them exists.
    selections[0].operation_id = "drive.files.download".into();
    assert!(Engine::new(&bundle, BASE, &selections).is_err());
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

/// The guide's configuration example for this provider.
fn documented_config() -> Value {
    let guide = fs::read_to_string(root().join("../../docs/catalog-google-drive.md")).unwrap();
    let example = guide
        .split("```json\n")
        .skip(1)
        .filter_map(|rest| rest.split_once("\n```").map(|(body, _)| body))
        .find(|body| body.contains(&format!("\"provider\": \"{PROVIDER}\"")))
        .expect("the documented google-drive configuration");
    serde_json::from_str::<Value>(example).unwrap()
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
        "https://accounts.google.com/o/oauth2/v2/auth"
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

fn file(id: &str, name: &str) -> Value {
    json!({"kind": "drive#file", "id": id, "name": name,
           "mimeType": "application/vnd.google-apps.document",
           "modifiedTime": "2026-09-20T10:00:00.000Z"})
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
                                "scope": format!("openid {DRIVE_SCOPE}"),
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
                        match (method.as_str(), answer(&target)) {
                            ("GET", Some((content_type, body))) => (200, content_type, body),
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
        let mut config = documented_config();
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
