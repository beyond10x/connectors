//! Slack conversations through the catalog provider.
//!
//! The shipped selection set is pinned by id and source operation, resolves
//! against the committed bundle compiled from the OpenAPI projection of the
//! pinned Slack Web API Swagger 2.0 document, and is cited row by row in
//! `docs/catalog-slack.md`. Each read runs through the provider child against
//! a disposable HTTPS fixture: the exact request (path, query including the
//! time window, `Authorization: Bearer …`) and the returned body bytes are
//! asserted, every list walks two pages to an empty
//! `response_metadata.next_cursor`, and the `auth.test` identity read yields
//! the token's user id. Every id, name and message is synthetic. No live
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

/// The pinned document's host is `slack.com` with base path `/api`, which
/// the projection's one server carries, so every operation and the identity
/// read sit below `/api`.
const BASE: &str = "/api";
/// A fictional bot token.
const TOKEN: &str = "fixture-slack-bot-token";
const HEADER: &str = "Bearer fixture-slack-bot-token";
const PROFILE: &str = "slack.bot";
/// The pinned document and its committed projection, relative to this crate.
const UPSTREAM: &str = "../slack/upstream/slack_web_openapi_v2_without_examples.json";
const PROJECTION: &str = "../slack/generated/slack-web.openapi.json";
const SOURCE_SHA256: &str = "8b92da26a3c5b11d20042a9f36d81f1fa6fc9382c5ddc471babb68b91936bc3a";

/// The shipped ids, their pinned `operationId` and the path the bundle
/// records. A renamed, dropped or added id fails here.
const SHIPPED: [(&str, &str, &str); 3] = [
    (
        "conversations.history",
        "conversations_history",
        "/api/conversations.history",
    ),
    (
        "conversations.list",
        "conversations_list",
        "/api/conversations.list",
    ),
    (
        "conversations.replies",
        "conversations_replies",
        "/api/conversations.replies",
    ),
];

fn root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
}
fn shipped() -> Vec<Selection> {
    let file: Value =
        serde_json::from_slice(&fs::read(root().join("providers/slack/operations.json")).unwrap())
            .unwrap();
    assert_eq!(file["format"], "connectors-catalog-operations/1");
    assert_eq!(file["provider"], "slack");
    serde_json::from_value(file["operations"].clone()).unwrap()
}
fn committed() -> bundle::Bundle {
    bundle::load(&root().join("generated/bundles"), "slack").unwrap()
}
fn pinned() -> Value {
    serde_json::from_slice(&fs::read(root().join(UPSTREAM)).unwrap()).unwrap()
}

#[test]
fn shipped_slack_selections_are_exactly_the_three_conversation_reads() {
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
        // The same operation, at the same path, in the pinned Swagger document.
        let source_path = path.strip_prefix(BASE).unwrap();
        assert_eq!(
            pinned()["paths"][source_path]["get"]["operationId"],
            operation_id,
            "`{id}`"
        );
        // The credential travels in the header only: the document's `token`
        // query parameter is never exposed.
        assert!(selection.withhold.contains(&"token".to_owned()), "`{id}`");
    }
}

/// A source operation the document lacks, or a write, cannot ship as a read.
#[test]
fn a_missing_or_writing_source_operation_cannot_ship_as_a_read() {
    let bundle = committed();
    let mut selections = shipped();
    selections[0].operation_id = "conversations_threads".into();
    assert!(Engine::new(&bundle, BASE, &selections).is_err());
    selections[0].operation_id = "chat_postMessage".into();
    assert!(Engine::new(&bundle, BASE, &selections).is_err());
}

#[test]
fn the_bundle_is_derived_from_the_pinned_swagger_document() {
    let bytes = fs::read(root().join(UPSTREAM)).unwrap();
    assert_eq!(hex::encode(Sha256::digest(&bytes)), SOURCE_SHA256);
    let projection = fs::read(root().join(PROJECTION)).unwrap();
    let index = bundle::read_index(&root().join("generated/bundles")).unwrap();
    let entry = index.find("slack").unwrap();
    assert_eq!(
        entry.source_sha256,
        hex::encode(Sha256::digest(&projection))
    );
    assert_eq!(entry.operations, 174);
    assert_eq!(entry.unsupported, 0);
    let bundle = committed();
    assert_eq!(bundle.source.file_name, "slack-web.openapi.json");
    let derivation = bundle.source.derivation.expect("a recorded derivation");
    assert_eq!(derivation.from_sha256, SOURCE_SHA256);
    assert_eq!(derivation.from_bytes, bytes.len());
    assert_eq!(
        derivation.from_file,
        "slack_web_openapi_v2_without_examples.json"
    );
    assert_eq!(derivation.format, "swagger/2.0");
    assert_eq!(derivation.discovery_revision, None);
}

/// Every list's end condition and time window is in the pinned parameters:
/// `cursor` and `limit` on all three, `oldest` and `latest` on the two
/// message reads, `ts` and `channel` on replies. The only stated page bound is
/// `conversations.list`'s 1,000, and only it carries a bound.
#[test]
fn the_paging_and_window_parameters_are_in_the_pinned_document() {
    let document = pinned();
    let names = |path: &str| -> Vec<String> {
        document["paths"][path]["get"]["parameters"]
            .as_array()
            .unwrap()
            .iter()
            .map(|p| p["name"].as_str().unwrap().to_owned())
            .collect()
    };
    for (path, wanted) in [
        ("/conversations.list", &["cursor", "limit", "types"][..]),
        (
            "/conversations.history",
            &[
                "cursor",
                "limit",
                "channel",
                "oldest",
                "latest",
                "inclusive",
            ][..],
        ),
        (
            "/conversations.replies",
            &[
                "cursor",
                "limit",
                "channel",
                "ts",
                "oldest",
                "latest",
                "inclusive",
            ][..],
        ),
    ] {
        let declared = names(path);
        for name in wanted {
            assert!(
                declared.iter().any(|d| d == name),
                "`{path}` lacks `{name}`"
            );
        }
    }
    let limit = document["paths"]["/conversations.list"]["get"]["parameters"]
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["name"] == "limit")
        .unwrap()["description"]
        .as_str()
        .unwrap()
        .to_owned();
    assert!(limit.contains("no larger than 1000"), "{limit}");
    for selection in shipped() {
        let bound = selection.bounds.get("limit");
        if selection.id == "conversations.list" {
            assert_eq!(bound.map(|b| b.maximum), Some(1000));
        } else {
            assert!(selection.bounds.is_empty(), "`{}`", selection.id);
        }
    }
}

#[test]
fn guide_cites_each_operation_its_paging_and_its_time_window() {
    let guide = fs::read_to_string(root().join("../../docs/catalog-slack.md")).unwrap();
    let rows: Vec<&str> = guide.lines().filter(|l| l.starts_with('|')).collect();
    let end = "empty `response_metadata.next_cursor`";
    for (id, operation_id, path, window) in [
        (
            "conversations.list",
            "conversations_list",
            "/api/conversations.list",
            "none",
        ),
        (
            "conversations.history",
            "conversations_history",
            "/api/conversations.history",
            "`oldest`, `latest`",
        ),
        (
            "conversations.replies",
            "conversations_replies",
            "/api/conversations.replies",
            "`oldest`, `latest`",
        ),
    ] {
        let row = rows
            .iter()
            .find(|row| row.contains(&format!("| `{id}` |")))
            .unwrap_or_else(|| panic!("no row for `{id}`"));
        assert!(row.contains(&format!("`{operation_id}`")), "{row}");
        assert!(row.contains(&format!("`GET {path}`")), "{row}");
        assert!(row.contains("`cursor`, `limit`"), "{row}");
        assert!(row.contains(end), "{row}");
        assert!(row.contains(window), "{row}");
    }
}

/// The guide's configuration example.
fn documented_config() -> Value {
    let guide = fs::read_to_string(root().join("../../docs/catalog-slack.md")).unwrap();
    let example = guide
        .split("```json\n")
        .skip(1)
        .filter_map(|rest| rest.split_once("\n```").map(|(body, _)| body))
        .find(|body| body.contains("\"provider\": \"slack\""))
        .expect("the documented slack configuration");
    serde_json::from_str::<Value>(example).unwrap()
}

#[test]
fn the_documented_profile_is_a_bearer_bot_token_with_auth_test_as_identity() {
    let config = documented_config();
    assert_eq!(config["api_base"], "https://slack.com/api");
    let auth = &config["auth"];
    assert_eq!(auth["profile"], PROFILE);
    assert_eq!(auth["scheme"], Value::Null);
    assert_eq!(auth["header"], "Authorization");
    assert_eq!(auth["bearer"], true);
    assert_eq!(auth["identity"]["path"], "auth.test");
    assert_eq!(auth["identity"]["kind"], "slack.user");
    assert_eq!(auth["identity"]["subject_pointer"], "/user_id");
    assert_eq!(auth["scopes"], Value::Null);
}

/// Route and `Authorization` header of each fixture request.
type Requests = Arc<Mutex<Vec<(String, Option<String>)>>>;

fn channel(n: u64) -> Value {
    json!({"id": format!("C0FIXTURE{n:02}"), "name": format!("fixture-channel-{n}"),
           "is_channel": true, "is_private": false, "is_archived": false,
           "created": 1_780_000_000u64 + n})
}
fn message(ts: &str, text: &str) -> Value {
    json!({"type": "message", "user": "U0FIXTURE02", "text": text, "ts": ts})
}
fn reply(ts: &str, text: &str) -> Value {
    json!({"type": "message", "user": "U0FIXTURE02", "text": text, "ts": ts,
           "thread_ts": "1780000100.000100"})
}
fn more(cursor: &str) -> Value {
    json!({"next_cursor": cursor})
}

/// The recorded bodies the fixture serves, keyed by route and paging position.
/// `None` for anything else, which the fixture answers 404. A last page
/// carries an empty `next_cursor`, as Slack's pagination guide describes.
fn page(path: &str) -> Option<Value> {
    let (route, query) = path.split_once('?').unwrap_or((path, ""));
    let has = |pair: &str| query.split('&').any(|p| p == pair);
    Some(match route {
        "/api/auth.test" => json!({
            "ok": true, "url": "https://fixture.example.test/", "team": "Fixture Team",
            "user": "fixture-bot", "team_id": "T0FIXTURE01", "user_id": "U0FIXTURE01",
            "bot_id": "B0FIXTURE01", "is_enterprise_install": false}),
        "/api/conversations.list" if has("cursor=fixture-list-2") => json!({
            "ok": true, "channels": [channel(3)], "response_metadata": more("")}),
        "/api/conversations.list" => json!({
            "ok": true, "channels": [channel(1), channel(2)],
            "response_metadata": more("fixture-list-2")}),
        "/api/conversations.history" if !has("channel=C0FIXTURE01") => json!({
            "ok": false, "error": "channel_not_found"}),
        "/api/conversations.history" if has("cursor=fixture-history-2") => json!({
            "ok": true, "messages": [message("1780000100.000100", "fixture message three")],
            "has_more": false, "pin_count": 0, "response_metadata": more("")}),
        "/api/conversations.history" => json!({
            "ok": true,
            "messages": [message("1780000300.000300", "fixture message one"),
                         message("1780000200.000200", "fixture message two")],
            "has_more": true, "pin_count": 0,
            "response_metadata": more("fixture-history-2")}),
        "/api/conversations.replies" if has("cursor=fixture-replies-2") => json!({
            "ok": true, "messages": [reply("1780000400.000400", "fixture reply two")],
            "has_more": false, "response_metadata": more("")}),
        "/api/conversations.replies" => json!({
            "ok": true,
            "messages": [reply("1780000100.000100", "fixture thread parent"),
                         reply("1780000300.000300", "fixture reply one")],
            "has_more": true, "response_metadata": more("fixture-replies-2")}),
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
        Self::serving(false)
    }
    /// `refuse_identity`: answer `auth.test` as Slack answers a revoked
    /// token, `200` with `ok: false` and no `user_id`.
    fn serving(refuse_identity: bool) -> Self {
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
                    // Slack answers a bad token `200` with `ok: false`; only
                    // the fictional header is accepted, and raw headers are
                    // compared, never printed.
                    let body = if authorization.as_deref() != Some(HEADER)
                        || (refuse_identity && path.starts_with("/api/auth.test"))
                    {
                        Some(json!({"ok": false, "error": "invalid_auth"}))
                    } else {
                        page(&path)
                    };
                    let (status, body) = match body {
                        Some(body) => (200, body),
                        None => (404, json!({"ok": false, "error": "no fixture"})),
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
        let mut document = documented_config();
        document["instance"] = json!("fixture-slack");
        document["bundle_directory"] = json!(root_path("generated/bundles"));
        document["api_base"] = json!(format!("https://localhost:{}/api", address.port()));
        document["ca_file"] = json!(ca);
        document["operations_file"] = json!(root_path("providers/slack/operations.json"));
        private(&config, &serde_json::to_vec(&document).unwrap());
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
            instance_id: "fixture-slack".into(),
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
    Secret(serde_json::to_vec(&json!({"token": TOKEN})).unwrap())
}
fn deadline() -> u64 {
    connectors_sdk::now_ms() + 30_000
}
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
fn invoke_raw(child: &mut Child, operation: &str, input: &Value) -> Vec<u8> {
    attempt(child, operation, input)
        .unwrap_or_else(|failure| panic!("`{operation}` failed: {failure:?}"))
}
fn invoke(child: &mut Child, operation: &str, input: &Value) -> Value {
    serde_json::from_slice(&invoke_raw(child, operation, input)).unwrap()
}

#[test]
fn connecting_records_the_token_user_as_the_identity() {
    let provider = Provider::new();
    let mut child = Child::spawn(&provider.selection()).unwrap();
    let baseline = child.validate(PROFILE, &secret(), deadline()).unwrap();
    assert_eq!(baseline.identity.kind, "slack.user");
    assert_eq!(baseline.identity.subject, "U0FIXTURE01");
    assert_eq!(baseline.granted_scopes, None);
    let requests = provider.requests();
    assert_eq!(requests.len(), 1);
    // The identity probe carries an empty query and the bearer header, never
    // the token as a query parameter.
    assert_eq!(requests[0].0, "/api/auth.test?");
    assert_eq!(requests[0].1.as_deref(), Some(HEADER));
    assert!(matches!(
        child.validate("atlassian.basic", &secret(), deadline()),
        Err(Failure::Unsupported)
    ));
}

/// Slack answers a token it does not accept `200` with `ok: false` and no
/// `user_id`; the connection is refused, as an answer the identity read
/// cannot read, not admitted.
#[test]
fn an_identity_answer_without_a_user_id_refuses_the_connection() {
    let provider = Provider::serving(true);
    let mut child = Child::spawn(&provider.selection()).unwrap();
    assert!(matches!(
        child.validate(PROFILE, &secret(), deadline()),
        Err(Failure::Protocol)
    ));
}

/// Each read's first request: the input, and the exact request the fixture
/// must observe. Query parameters go out in the order the pinned document
/// declares them, not the order of the input.
fn first_pages() -> [(&'static str, Value, &'static str); 3] {
    [
        (
            "conversations.list",
            json!({"types": "public_channel,private_channel", "exclude_archived": true,
                   "limit": 2}),
            "/api/conversations.list?exclude_archived=true&types=public_channel%2Cprivate_channel&limit=2",
        ),
        (
            "conversations.history",
            json!({"channel": "C0FIXTURE01", "oldest": "1780000000.000000",
                   "latest": "1780000400.000000", "inclusive": true, "limit": 2}),
            "/api/conversations.history?channel=C0FIXTURE01&latest=1780000400.000000&oldest=1780000000.000000&inclusive=true&limit=2",
        ),
        (
            "conversations.replies",
            json!({"channel": "C0FIXTURE01", "ts": "1780000100.000100",
                   "oldest": "1780000000.000000", "latest": "1780000500.000000", "limit": 2}),
            "/api/conversations.replies?channel=C0FIXTURE01&ts=1780000100.000100&latest=1780000500.000000&oldest=1780000000.000000&limit=2",
        ),
    ]
}

#[test]
fn each_read_sends_the_declared_request_with_bearer_auth_and_returns_the_recorded_body() {
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
        assert_eq!(result["provenance"]["instance"], "fixture-slack");
        // The bytes the fixture served appear unchanged in the provider's
        // output, as the value of `body`.
        let served = serde_json::to_vec(&page(expected).unwrap()).unwrap();
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

/// The token stays out of the query, the message reads need their channel
/// (and replies its thread `ts`), and a list page larger than the pinned
/// 1,000 is refused: each before any request.
#[test]
fn a_withheld_missing_or_out_of_bound_parameter_is_refused_before_any_request() {
    let provider = Provider::new();
    let mut child = Child::spawn(&provider.selection()).unwrap();
    for (operation, input) in [
        ("conversations.list", json!({"token": TOKEN})),
        (
            "conversations.history",
            json!({"channel": "C0FIXTURE01", "token": TOKEN}),
        ),
        ("conversations.history", json!({"oldest": "1780000000"})),
        ("conversations.replies", json!({"channel": "C0FIXTURE01"})),
        ("conversations.replies", json!({"ts": "1780000100.000100"})),
        ("conversations.list", json!({"limit": 1001})),
        (
            "conversations.list",
            json!({"cursor": "c", "undeclared": 1}),
        ),
    ] {
        let outcome = attempt(&mut child, operation, &input);
        assert!(
            matches!(outcome, Err(Failure::InvalidInput)),
            "`{operation}` {input}: {outcome:?}"
        );
    }
    assert!(provider.requests().is_empty());
}

/// Slack reports a failed read as `200` with `ok: false`. The provider returns
/// that answer as it came: the caller reads `ok`.
#[test]
fn a_slack_error_answer_is_returned_with_status_200_and_ok_false() {
    let provider = Provider::new();
    let mut child = Child::spawn(&provider.selection()).unwrap();
    let result = invoke(
        &mut child,
        "conversations.history",
        &json!({"channel": "C0FIXTURE99"}),
    );
    assert_eq!(result["status"], 200);
    assert_eq!(
        result["body"],
        json!({"ok": false, "error": "channel_not_found"})
    );
}

/// Walk one list from `first`, sending each page's non-empty
/// `response_metadata.next_cursor` as `cursor`, and stop on an empty one.
/// Returns the items read under `items` and the requests sent.
fn walk(operation: &str, first: Value, items: &str) -> (Vec<String>, Vec<String>) {
    let provider = Provider::new();
    let mut child = Child::spawn(&provider.selection()).unwrap();
    let mut input = first;
    let mut read = Vec::new();
    let mut pages = 0;
    loop {
        let body = invoke(&mut child, operation, &input)["body"].clone();
        pages += 1;
        assert_eq!(body["ok"], true, "`{operation}` page {pages}");
        for item in body[items].as_array().unwrap() {
            let key = item.get("id").or_else(|| item.get("ts")).unwrap();
            read.push(key.as_str().unwrap().to_owned());
        }
        let cursor = body
            .pointer("/response_metadata/next_cursor")
            .and_then(Value::as_str)
            .expect("every page carries response_metadata.next_cursor");
        if cursor.is_empty() {
            break;
        }
        input["cursor"] = json!(cursor);
        assert!(pages < 3, "the `{operation}` walk did not stop");
    }
    assert_eq!(pages, 2, "`{operation}` pages");
    let requests = provider
        .requests()
        .into_iter()
        .map(|(path, _)| path)
        .collect();
    (read, requests)
}

#[test]
fn conversations_list_walks_two_pages_and_stops_on_an_empty_next_cursor() {
    let (read, requests) = walk("conversations.list", json!({"limit": 2}), "channels");
    assert_eq!(read, ["C0FIXTURE01", "C0FIXTURE02", "C0FIXTURE03"]);
    assert_eq!(
        requests,
        [
            "/api/conversations.list?limit=2",
            "/api/conversations.list?limit=2&cursor=fixture-list-2",
        ]
    );
}

#[test]
fn conversations_history_walks_two_pages_and_stops_on_an_empty_next_cursor() {
    let (read, requests) = walk(
        "conversations.history",
        json!({"channel": "C0FIXTURE01", "oldest": "1780000000.000000", "limit": 2}),
        "messages",
    );
    assert_eq!(
        read,
        [
            "1780000300.000300",
            "1780000200.000200",
            "1780000100.000100"
        ]
    );
    assert_eq!(
        requests,
        [
            "/api/conversations.history?channel=C0FIXTURE01&oldest=1780000000.000000&limit=2",
            "/api/conversations.history?channel=C0FIXTURE01&oldest=1780000000.000000&limit=2&cursor=fixture-history-2",
        ]
    );
}

#[test]
fn conversations_replies_walks_two_pages_and_stops_on_an_empty_next_cursor() {
    let (read, requests) = walk(
        "conversations.replies",
        json!({"channel": "C0FIXTURE01", "ts": "1780000100.000100", "limit": 2}),
        "messages",
    );
    assert_eq!(
        read,
        [
            "1780000100.000100",
            "1780000300.000300",
            "1780000400.000400"
        ]
    );
    assert_eq!(
        requests,
        [
            "/api/conversations.replies?channel=C0FIXTURE01&ts=1780000100.000100&limit=2",
            "/api/conversations.replies?channel=C0FIXTURE01&ts=1780000100.000100&limit=2&cursor=fixture-replies-2",
        ]
    );
}
