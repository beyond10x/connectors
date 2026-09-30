//! Gmail reads — the profile, messages, threads, history and labels — through
//! the catalog provider.
//!
//! The shipped selection set is pinned by id and Discovery method id, resolves
//! against the committed bundle compiled from the projection of the pinned
//! Gmail v1 Discovery document, and is cited row by row in
//! `docs/catalog-google-gmail.md`. Each read runs through the provider child
//! against a disposable HTTPS fixture that serves the Gmail routes and an OAuth
//! token route on one host: the child exchanges the fixture refresh entry for an
//! access token, and the exact request (path, query, `Authorization: Bearer …`)
//! and the returned body are asserted. The message and thread lists walk two
//! pages until `nextPageToken` is absent; the history walk starts from the
//! profile's `historyId` and ends on the page without `nextPageToken`, whose
//! `historyId` is the next baseline; an out-of-date `startHistoryId`'s `404`
//! reaches the caller as `not_found`. The profile is the guide's own, pointed at
//! the fixture token route. The fixture secrets are fictional and only ever
//! compared, never printed. No live credential and no network.
use connectors_catalog::{bundle, discovery};
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

/// The path the configured `api_base` carries: none. The projection's one
/// server is `https://gmail.googleapis.com`, and each operation path carries
/// its own `/gmail/v1`.
const BASE: &str = "";
const PROVIDER: &str = "google-gmail";
/// The bundle's auth profile, and the profile of the guide's configuration.
const PROFILE: &str = "google.oauth";
const GMAIL_SCOPE: &str = "https://www.googleapis.com/auth/gmail.readonly";
/// Fictional OAuth material. The access token is what the fixture token route
/// issues and the only bearer the fixture Gmail routes accept.
const CLIENT_ID: &str = "fixture-client-id.apps.example.test";
const CLIENT_SECRET: &str = "fixture-client-secret-gmail";
const REFRESH_TOKEN: &str = "fixture-refresh-token-gmail";
const ACCESS_TOKEN: &str = "fixture-access-token-gmail";
/// The pinned Discovery document, relative to this crate.
const UPSTREAM: &str = "../google/upstream/gmail/gmail-api.json";
/// The committed projection, relative to this crate.
const PROJECTED: &str = "../google/generated/gmail.openapi.json";
/// The page-size ceiling the three pinned lists state only in the text of
/// their `maxResults` description ("The maximum allowed value for this field
/// is 500.").
const MAX_RESULTS: u64 = 500;

/// The shipped ids, their Discovery method id and the path the bundle records.
/// A renamed, dropped or added id fails here.
const SHIPPED: [(&str, &str, &str); 7] = [
    (
        "users.getProfile",
        "gmail.users.getProfile",
        "/gmail/v1/users/{userId}/profile",
    ),
    (
        "users.history.list",
        "gmail.users.history.list",
        "/gmail/v1/users/{userId}/history",
    ),
    (
        "users.labels.list",
        "gmail.users.labels.list",
        "/gmail/v1/users/{userId}/labels",
    ),
    (
        "users.messages.get",
        "gmail.users.messages.get",
        "/gmail/v1/users/{userId}/messages/{id}",
    ),
    (
        "users.messages.list",
        "gmail.users.messages.list",
        "/gmail/v1/users/{userId}/messages",
    ),
    (
        "users.threads.get",
        "gmail.users.threads.get",
        "/gmail/v1/users/{userId}/threads/{id}",
    ),
    (
        "users.threads.list",
        "gmail.users.threads.list",
        "/gmail/v1/users/{userId}/threads",
    ),
];
/// The three lists, by selection id and Discovery resource.
const LISTS: [(&str, &str); 3] = [
    ("users.messages.list", "messages"),
    ("users.threads.list", "threads"),
    ("users.history.list", "history"),
];

fn root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
}
fn shipped() -> Vec<Selection> {
    let file: Value = serde_json::from_slice(
        &fs::read(root().join("providers/google-gmail/operations.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(file["format"], "connectors-catalog-operations/1");
    assert_eq!(file["provider"], PROVIDER);
    serde_json::from_value(file["operations"].clone()).unwrap()
}
fn pinned() -> Value {
    serde_json::from_slice(&fs::read(root().join(UPSTREAM)).unwrap()).unwrap()
}
/// The pinned Discovery method of a selection id such as `users.messages.list`.
fn pinned_method(document: &Value, id: &str) -> Value {
    let parts: Vec<&str> = id.split('.').collect();
    let mut node = &document["resources"][parts[0]];
    for resource in &parts[1..parts.len() - 1] {
        node = &node["resources"][*resource];
    }
    node["methods"][parts[parts.len() - 1]].clone()
}
fn committed_bundle() -> bundle::Bundle {
    bundle::load(&root().join("generated/bundles"), PROVIDER).unwrap()
}
fn engine() -> Engine {
    Engine::new(&committed_bundle(), BASE, &shipped()).unwrap()
}

#[test]
fn shipped_gmail_selections_are_exactly_the_seven_reads() {
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
            operation_id.strip_prefix("gmail."),
            "`{id}` is not `{operation_id}` without `gmail.`"
        );
        assert_eq!(selection.effect, Effect::Read, "`{id}`");
        assert_eq!(selection.response, None, "`{id}` reads JSON");
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
}

#[test]
fn a_selection_the_projection_lacks_is_refused_at_load() {
    let bundle = committed_bundle();
    let mut selections = shipped();
    // Gmail has no `messages.search` method: search is `messages.list` with `q`.
    let absent = "gmail.users.messages.search";
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
    assert_eq!(bundle.source.file_name, "gmail.openapi.json");
    assert_eq!(
        bundle.source.source_sha256,
        hex::encode(Sha256::digest(&committed))
    );
    let derivation = bundle.source.derivation.expect("the bundle's derivation");
    assert_eq!(derivation.from_file, "gmail-api.json");
    assert_eq!(derivation.from_sha256, digest);
    assert_eq!(derivation.from_bytes, bytes.len());
    assert_eq!(derivation.format, discovery::FORMAT);
    assert_eq!(derivation.projector, discovery::PROJECTOR);
    assert_eq!(json!(derivation.discovery_revision), pinned()["revision"]);
    let index = bundle::read_index(&root().join("generated/bundles")).unwrap();
    assert!(index.find(PROVIDER).is_some());
}

/// The three lists bound `maxResults` at 1–500: the ceiling each pinned
/// description states, the document declaring neither bound. No other
/// selection carries a bound.
#[test]
fn the_three_lists_bound_max_results_at_the_documented_range() {
    let document = pinned();
    for (id, _) in LISTS {
        let max_results = &pinned_method(&document, id)["parameters"]["maxResults"];
        assert!(max_results.get("minimum").is_none(), "`{id}`");
        assert!(max_results.get("maximum").is_none(), "`{id}`");
        assert!(
            max_results["description"]
                .as_str()
                .unwrap()
                .contains(&format!(
                    "The maximum allowed value for this field is {MAX_RESULTS}."
                )),
            "`{id}`"
        );
    }
    for selection in shipped() {
        let written = serde_json::to_value(&selection).unwrap();
        let expected = if LISTS.iter().any(|(id, _)| *id == selection.id) {
            json!({"maxResults": {"minimum": 1, "maximum": MAX_RESULTS}})
        } else {
            Value::Null
        };
        assert_eq!(written["bounds"], expected, "`{}`", selection.id);
    }
}

/// The pinned document calls `startHistoryId` "Required." in its description
/// but does not mark it required; the selection does, so the declared input
/// schema requires it. No other selection declares `required`.
#[test]
fn history_list_declares_start_history_id_required() {
    let document = pinned();
    let start = &pinned_method(&document, "users.history.list")["parameters"]["startHistoryId"];
    assert!(start.get("required").is_none());
    assert!(
        start["description"]
            .as_str()
            .unwrap()
            .starts_with("Required.")
    );
    for selection in shipped() {
        let expected: &[&str] = if selection.id == "users.history.list" {
            &["startHistoryId"]
        } else {
            &[]
        };
        assert_eq!(selection.required, expected, "`{}`", selection.id);
    }
    let history = engine()
        .declarations(&[Effect::Read])
        .into_iter()
        .find(|o| o.id == "users.history.list")
        .unwrap();
    let required = history.input_schema["required"].as_array().unwrap();
    assert!(required.contains(&json!("startHistoryId")), "{required:?}");
    assert!(required.contains(&json!("userId")), "{required:?}");
}

/// `labelIds` on both lists and `metadataHeaders` on both gets are repeated in
/// the pinned document, so their declared input schema takes an array.
#[test]
fn repeated_parameters_are_declared_as_arrays() {
    let declarations = engine().declarations(&[Effect::Read]);
    for (id, parameter) in [
        ("users.messages.list", "labelIds"),
        ("users.threads.list", "labelIds"),
        ("users.messages.get", "metadataHeaders"),
        ("users.threads.get", "metadataHeaders"),
    ] {
        let declaration = declarations.iter().find(|o| o.id == id).unwrap();
        let schema = &declaration.input_schema["properties"][parameter];
        assert!(
            schema["type"]
                .as_array()
                .is_some_and(|types| types.contains(&json!("array"))),
            "`{id}` `{parameter}`: {schema}"
        );
    }
}

fn guide() -> String {
    fs::read_to_string(root().join("../../docs/catalog-google-gmail.md")).unwrap()
}

#[test]
fn guide_cites_each_operation_its_paging_and_its_deltas() {
    let guide = guide();
    let rows: Vec<&str> = guide.lines().filter(|l| l.starts_with('|')).collect();
    let list = "`pageToken`, `maxResults` (1–500)";
    for (id, paging, end, deltas) in [
        ("users.getProfile", "single item", "n/a", "`historyId`"),
        ("users.messages.list", list, "`nextPageToken` absent", "`q`"),
        (
            "users.messages.get",
            "single item",
            "n/a",
            "`users.history.list`",
        ),
        ("users.threads.list", list, "`nextPageToken` absent", "`q`"),
        (
            "users.threads.get",
            "single item",
            "n/a",
            "`users.history.list`",
        ),
        (
            "users.history.list",
            list,
            "`nextPageToken` absent",
            "`startHistoryId`",
        ),
        ("users.labels.list", "single item", "n/a", "none"),
    ] {
        let (_, operation_id, path) = SHIPPED.iter().find(|(i, _, _)| *i == id).unwrap();
        let cited = rows.iter().any(|row| {
            row.starts_with(&format!("| `{id}` "))
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

/// The guide states that `userId` is `me`, the message formats, and the delta
/// cycle by `historyId` with its reset rule: a `404` means a full sync.
#[test]
fn guide_documents_user_id_formats_and_history_deltas() {
    let guide = guide();
    for term in [
        "`userId`",
        "`me`",
        "`historyId`",
        "`startHistoryId`",
        "`404`",
        "`not_found`",
        "full sync",
        "`labelIds`",
        "`format`",
        "`full`",
        "`metadata`",
        "`minimal`",
        "`raw`",
        "`rate_limited`",
        "4 MiB",
    ] {
        assert!(guide.contains(term), "the guide does not state {term}");
    }
}

/// The guide's configuration example for this provider.
fn documented_config() -> Value {
    let guide = guide();
    let example = guide
        .split("```json\n")
        .skip(1)
        .filter_map(|rest| rest.split_once("\n```").map(|(body, _)| body))
        .find(|body| body.contains(&format!("\"provider\": \"{PROVIDER}\"")))
        .expect("the documented google-gmail configuration");
    serde_json::from_str::<Value>(example).unwrap()
}

/// The guide configures the bundle's profile as an `oauth2_refresh` profile
/// against Google's token endpoint, with the Gmail read-only scope, and the
/// projection's server as the API base.
#[test]
fn guide_documents_the_oauth_refresh_configuration() {
    let config = documented_config();
    assert_eq!(config["api_base"], "https://gmail.googleapis.com");
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
    assert_eq!(auth["minimum_scopes"], json!([GMAIL_SCOPE]));
    assert!(
        auth["requested_scopes"]
            .as_array()
            .unwrap()
            .contains(&json!(GMAIL_SCOPE))
    );
    assert!(auth.get("token_ca_file").is_none());
    // Every shipped read accepts that scope, per the pinned document.
    let document = pinned();
    for (id, _, _) in SHIPPED {
        let method = pinned_method(&document, id);
        assert!(
            method["scopes"]
                .as_array()
                .unwrap()
                .contains(&json!(GMAIL_SCOPE)),
            "`{id}`"
        );
    }
}

/// Method, route with query, and `Authorization` header of each fixture request.
type Requests = Arc<Mutex<Vec<(String, String, Option<String>)>>>;

fn message_ref(id: &str) -> Value {
    json!({"id": id, "threadId": format!("thread-of-{id}")})
}
fn message(id: &str) -> Value {
    json!({"id": id, "threadId": "fixture-thread-1", "labelIds": ["INBOX", "UNREAD"],
           "snippet": "Fixture message", "historyId": "1001", "internalDate": "1790000000000",
           "sizeEstimate": 512,
           "payload": {"mimeType": "text/plain",
                       "headers": [{"name": "Subject", "value": "Fixture subject"}],
                       "body": {"size": 14, "data": "Rml4dHVyZSBib2R5Cg"}}})
}
fn thread(id: &str) -> Value {
    json!({"id": id, "historyId": "1001", "messages": [message("fixture-message-1")]})
}
fn history(id: &str, message_id: &str) -> Value {
    json!({"id": id, "messages": [message_ref(message_id)],
           "messagesAdded": [{"message": message_ref(message_id)}]})
}

/// The recorded Gmail answers the fixture serves, keyed by route and paging
/// position: a status and a JSON body. `None` for anything else, which the
/// fixture answers 404.
fn answer(target: &str) -> Option<(u16, Value)> {
    let (route, query) = target.split_once('?').unwrap_or((target, ""));
    let has = |pair: &str| query.split('&').any(|p| p == pair);
    let ok = |value: Value| Some((200, value));
    match route {
        "/gmail/v1/users/me/profile" => ok(json!({
            "emailAddress": "reader@example.test", "messagesTotal": 3, "threadsTotal": 2,
            "historyId": "1000"})),
        "/gmail/v1/users/me/labels" => ok(json!({"labels": [
            {"id": "INBOX", "name": "INBOX", "type": "system"},
            {"id": "Label_1", "name": "Fixture label", "type": "user"}]})),
        "/gmail/v1/users/me/messages" if has("pageToken=fixture-messages-page-2") => ok(json!({
            "messages": [message_ref("fixture-message-3")], "resultSizeEstimate": 1})),
        "/gmail/v1/users/me/messages" if has("maxResults=2") => ok(json!({
            "messages": [message_ref("fixture-message-1"), message_ref("fixture-message-2")],
            "nextPageToken": "fixture-messages-page-2", "resultSizeEstimate": 3})),
        // Any other page size: one last page; Gmail omits an empty `messages`.
        "/gmail/v1/users/me/messages" => ok(json!({"resultSizeEstimate": 0})),
        "/gmail/v1/users/me/messages/fixture-message-1" if has("format=raw") => ok(json!({
            "id": "fixture-message-1", "threadId": "fixture-thread-1",
            "raw": "U3ViamVjdDogRml4dHVyZSBzdWJqZWN0DQoNCkZpeHR1cmUgYm9keQ0K"})),
        "/gmail/v1/users/me/messages/fixture-message-1" => ok(message("fixture-message-1")),
        "/gmail/v1/users/me/threads" if has("pageToken=fixture-threads-page-2") => ok(json!({
            "threads": [{"id": "fixture-thread-3", "historyId": "1002"}],
            "resultSizeEstimate": 1})),
        "/gmail/v1/users/me/threads" if has("maxResults=2") => ok(json!({
            "threads": [{"id": "fixture-thread-1", "historyId": "1001"},
                        {"id": "fixture-thread-2", "historyId": "1001"}],
            "nextPageToken": "fixture-threads-page-2", "resultSizeEstimate": 3})),
        "/gmail/v1/users/me/threads" => ok(json!({"resultSizeEstimate": 0})),
        "/gmail/v1/users/me/threads/fixture-thread-1" => ok(thread("fixture-thread-1")),
        "/gmail/v1/users/me/history" if has("startHistoryId=fixture-stale") => Some((
            404,
            json!({"error": {"code": 404, "message": "Requested entity was not found.",
                             "errors": [{"domain": "global", "reason": "notFound",
                                         "message": "Requested entity was not found."}],
                             "status": "NOT_FOUND"}}),
        )),
        "/gmail/v1/users/me/history" if has("pageToken=fixture-history-page-2") => ok(json!({
            "history": [history("1003", "fixture-message-6")], "historyId": "1003"})),
        "/gmail/v1/users/me/history" if has("startHistoryId=1000") && has("maxResults=2") => {
            ok(json!({
                "history": [history("1001", "fixture-message-4"),
                            history("1002", "fixture-message-5")],
                "nextPageToken": "fixture-history-page-2", "historyId": "1003"}))
        }
        // Any other page size from a baseline: no change since it.
        "/gmail/v1/users/me/history" => ok(json!({"historyId": "1003"})),
        _ => None,
    }
}
/// The recorded JSON body of `target`.
fn recorded(target: &str) -> Value {
    answer(target).unwrap().1
}

/// An unsigned JWT as the token answer's `id_token`.
fn id_token() -> String {
    use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
    let part = |value: &Value| URL_SAFE_NO_PAD.encode(serde_json::to_vec(value).unwrap());
    format!(
        "{}.{}.{}",
        part(&json!({"alg": "RS256", "typ": "JWT", "kid": "fixture"})),
        part(
            &json!({"iss": "https://accounts.google.com", "aud": CLIENT_ID, "azp": CLIENT_ID,
                     "sub": "110000000000000000004", "iat": 1, "exp": 4_000_000_000_u64})
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
                    let (status, answer) = if method == "POST" && target == "/token" {
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
                            (
                                200,
                                json!({
                                    "access_token": ACCESS_TOKEN, "token_type": "Bearer",
                                    "expires_in": 3600,
                                    "scope": format!("openid {GMAIL_SCOPE}"),
                                    "id_token": id_token()}),
                            )
                        } else {
                            (400, json!({"error": "invalid_grant"}))
                        }
                    } else if authorization.as_deref() != Some(&*format!("Bearer {ACCESS_TOKEN}"))
                    {
                        (401, json!({"error": {"code": 401, "message": "fixture refusal"}}))
                    } else {
                        match (method.as_str(), answer(&target)) {
                            ("GET", Some(answer)) => answer,
                            _ => (404, json!({"error": {"code": 404, "message": "no fixture"}})),
                        }
                    };
                    observed
                        .lock()
                        .unwrap()
                        .push((method, target, authorization));
                    let answer = serde_json::to_vec(&answer).unwrap();
                    let head = format!(
                        "HTTP/1.1 {status} fixture\r\nContent-Type: application/json; charset=UTF-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
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
        config["instance"] = json!("fixture-google-gmail");
        config["bundle_directory"] = json!(root_path("generated/bundles"));
        config["operations_file"] = json!(root_path("providers/google-gmail/operations.json"));
        config["api_base"] = json!(format!("https://localhost:{}", address.port()));
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
            instance_id: "fixture-google-gmail".into(),
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
    /// The Gmail requests (everything but the token route), as route with
    /// query.
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
fn first_requests() -> [(&'static str, Value, &'static str); 7] {
    [
        (
            "users.getProfile",
            json!({"userId": "me"}),
            // The transport writes an empty query when no query parameter is
            // bound, so the wire request ends in `?`.
            "/gmail/v1/users/me/profile?",
        ),
        (
            "users.labels.list",
            json!({"userId": "me"}),
            "/gmail/v1/users/me/labels?",
        ),
        (
            "users.messages.list",
            json!({"userId": "me", "q": "newer_than:7d", "maxResults": 2}),
            "/gmail/v1/users/me/messages?maxResults=2&q=newer_than%3A7d",
        ),
        (
            "users.messages.get",
            json!({"userId": "me", "id": "fixture-message-1", "format": "full"}),
            "/gmail/v1/users/me/messages/fixture-message-1?format=full",
        ),
        (
            "users.threads.list",
            json!({"userId": "me", "maxResults": 2}),
            "/gmail/v1/users/me/threads?maxResults=2",
        ),
        (
            "users.threads.get",
            json!({"userId": "me", "id": "fixture-thread-1", "format": "metadata",
                   "metadataHeaders": ["Subject", "From"]}),
            "/gmail/v1/users/me/threads/fixture-thread-1?format=metadata\
             &metadataHeaders=Subject&metadataHeaders=From",
        ),
        (
            "users.history.list",
            json!({"userId": "me", "startHistoryId": "1000", "maxResults": 2}),
            "/gmail/v1/users/me/history?maxResults=2&startHistoryId=1000",
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
        assert_eq!(result["provenance"]["instance"], "fixture-google-gmail");
        // The engine re-serialises the body, so the recorded answer is
        // compared as JSON, not as bytes.
        assert_eq!(result["body"], recorded(expected), "`{operation}` body");
    }
    // The bearer came from the token route: one exchange, before any Gmail
    // request; the cached token served every read after it.
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

/// `labelIds` is repeated: two labels are two `labelIds` pairs, in the order
/// given.
#[test]
fn messages_list_repeats_label_ids() {
    let provider = Provider::new();
    let mut child = Child::spawn(&provider.selection()).unwrap();
    invoke(
        &mut child,
        "users.messages.list",
        json!({"userId": "me", "labelIds": ["INBOX", "UNREAD"]}),
    );
    assert_eq!(
        provider.api_targets(),
        ["/gmail/v1/users/me/messages?labelIds=INBOX&labelIds=UNREAD"]
    );
}

/// Each `format` the pinned document enumerates is sent as given; `raw`
/// returns the message as one base64url string in `raw`.
#[test]
fn messages_get_sends_each_format() {
    let provider = Provider::new();
    let mut child = Child::spawn(&provider.selection()).unwrap();
    let enumerated =
        pinned_method(&pinned(), "users.messages.get")["parameters"]["format"]["enum"].clone();
    assert_eq!(enumerated, json!(["minimal", "full", "raw", "metadata"]));
    for format in ["full", "metadata", "minimal", "raw"] {
        let body = invoke(
            &mut child,
            "users.messages.get",
            json!({"userId": "me", "id": "fixture-message-1", "format": format}),
        )["body"]
            .clone();
        if format == "raw" {
            assert!(body["raw"].is_string(), "{body}");
        } else {
            assert_eq!(body["id"], "fixture-message-1");
        }
    }
    assert_eq!(
        provider.api_targets(),
        [
            "/gmail/v1/users/me/messages/fixture-message-1?format=full",
            "/gmail/v1/users/me/messages/fixture-message-1?format=metadata",
            "/gmail/v1/users/me/messages/fixture-message-1?format=minimal",
            "/gmail/v1/users/me/messages/fixture-message-1?format=raw",
        ]
    );
}

/// Walk one list from `input` following `nextPageToken`, and return the ids
/// under `items` (an absent array is an empty page), the page count and the
/// last page.
fn walk(
    child: &mut Child,
    operation: &str,
    items: &str,
    mut input: Value,
) -> (Vec<String>, usize, Value) {
    let mut ids = Vec::new();
    let mut pages = 0;
    loop {
        let body = invoke(child, operation, input.clone())["body"].clone();
        pages += 1;
        for item in body[items].as_array().into_iter().flatten() {
            ids.push(item["id"].as_str().unwrap().to_owned());
        }
        match body.get("nextPageToken").and_then(Value::as_str) {
            Some(token) => input["pageToken"] = json!(token),
            None => return (ids, pages, body),
        }
        assert!(pages < 3, "`{operation}` did not stop");
    }
}

#[test]
fn messages_list_walks_two_pages_until_next_page_token_is_absent() {
    let provider = Provider::new();
    let mut child = Child::spawn(&provider.selection()).unwrap();
    let (ids, pages, _) = walk(
        &mut child,
        "users.messages.list",
        "messages",
        json!({"userId": "me", "q": "newer_than:7d", "maxResults": 2}),
    );
    assert_eq!(pages, 2);
    assert_eq!(
        ids,
        [
            "fixture-message-1",
            "fixture-message-2",
            "fixture-message-3"
        ]
    );
    assert_eq!(
        provider.api_targets(),
        [
            "/gmail/v1/users/me/messages?maxResults=2&q=newer_than%3A7d",
            "/gmail/v1/users/me/messages?maxResults=2&pageToken=fixture-messages-page-2\
             &q=newer_than%3A7d",
        ]
    );
}

#[test]
fn threads_list_walks_two_pages_until_next_page_token_is_absent() {
    let provider = Provider::new();
    let mut child = Child::spawn(&provider.selection()).unwrap();
    let (ids, pages, _) = walk(
        &mut child,
        "users.threads.list",
        "threads",
        json!({"userId": "me", "maxResults": 2}),
    );
    assert_eq!(pages, 2);
    assert_eq!(
        ids,
        ["fixture-thread-1", "fixture-thread-2", "fixture-thread-3"]
    );
    assert_eq!(
        provider.api_targets(),
        [
            "/gmail/v1/users/me/threads?maxResults=2",
            "/gmail/v1/users/me/threads?maxResults=2&pageToken=fixture-threads-page-2",
        ]
    );
}

/// The delta walk: a baseline `historyId` from `users.getProfile`, then
/// `users.history.list` from it, following `nextPageToken`, until a page has
/// none; that page's `historyId` is the baseline for the next walk.
#[test]
fn history_list_walks_two_pages_from_the_profile_baseline() {
    let provider = Provider::new();
    let mut child = Child::spawn(&provider.selection()).unwrap();
    let baseline =
        invoke(&mut child, "users.getProfile", json!({"userId": "me"}))["body"]["historyId"]
            .clone();
    assert_eq!(baseline, "1000");
    let (records, pages, last) = walk(
        &mut child,
        "users.history.list",
        "history",
        json!({"userId": "me", "startHistoryId": baseline, "maxResults": 2}),
    );
    assert_eq!(pages, 2);
    assert_eq!(records, ["1001", "1002", "1003"]);
    assert_eq!(last["historyId"], "1003");
    assert_eq!(
        provider.api_targets(),
        [
            "/gmail/v1/users/me/profile?",
            "/gmail/v1/users/me/history?maxResults=2&startHistoryId=1000",
            "/gmail/v1/users/me/history?maxResults=2&pageToken=fixture-history-page-2\
             &startHistoryId=1000",
        ]
    );
}

/// `users.history.list` needs a `startHistoryId`; without one nothing is sent.
#[test]
fn history_list_requires_a_start_history_id() {
    let provider = Provider::new();
    let mut child = Child::spawn(&provider.selection()).unwrap();
    let outcome = attempt(&mut child, "users.history.list", &json!({"userId": "me"}));
    assert!(matches!(outcome, Err(Failure::InvalidInput)), "{outcome:?}");
    assert!(provider.api_targets().is_empty(), "a request was sent");
}

/// An out-of-date `startHistoryId` is Gmail's `404`; it reaches the caller as
/// `not_found`, the refusal the guide's full-sync rule starts from.
#[test]
fn an_out_of_date_start_history_id_is_refused_as_not_found() {
    let provider = Provider::new();
    let mut child = Child::spawn(&provider.selection()).unwrap();
    let outcome = attempt(
        &mut child,
        "users.history.list",
        &json!({"userId": "me", "startHistoryId": "fixture-stale"}),
    );
    assert!(
        matches!(outcome, Err(Failure::ProviderNotFound)),
        "{outcome:?}"
    );
    assert_eq!(
        provider.api_targets(),
        ["/gmail/v1/users/me/history?startHistoryId=fixture-stale"]
    );
}

/// `maxResults` 0 and 501 are refused as `invalid_input` with nothing sent, as
/// numbers and as strings; 1 and 500 are sent.
fn max_results_bounds(operation: &str, first: Value, route: &str) {
    let provider = Provider::new();
    let mut child = Child::spawn(&provider.selection()).unwrap();
    // Exchange the token first, so "nothing sent" counts every request.
    invoke(&mut child, operation, first.clone());
    let over = MAX_RESULTS + 1;
    let mut escaped = Vec::new();
    for size in [json!(0), json!(over), json!("0"), json!(over.to_string())] {
        let mut input = first.clone();
        input["maxResults"] = size.clone();
        let before = provider.requests().len();
        let outcome = attempt(&mut child, operation, &input);
        let sent = provider.requests().len() != before;
        if sent || !matches!(outcome, Err(Failure::InvalidInput)) {
            escaped.push(format!(
                "`{operation}` maxResults={size} sent={sent} {outcome:?}"
            ));
        }
    }
    assert!(
        escaped.is_empty(),
        "not refused before any request: {escaped:#?}"
    );
    for size in [1, MAX_RESULTS] {
        let mut input = first.clone();
        input["maxResults"] = json!(size);
        let before = provider.api_targets().len();
        invoke(&mut child, operation, input);
        let targets = provider.api_targets();
        assert_eq!(targets.len(), before + 1, "`{operation}` {size}");
        assert_eq!(targets[before], route.replace("{size}", &size.to_string()));
    }
}

#[test]
fn messages_list_max_results_bounds() {
    max_results_bounds(
        "users.messages.list",
        json!({"userId": "me", "maxResults": 2}),
        "/gmail/v1/users/me/messages?maxResults={size}",
    );
}

#[test]
fn threads_list_max_results_bounds() {
    max_results_bounds(
        "users.threads.list",
        json!({"userId": "me", "maxResults": 2}),
        "/gmail/v1/users/me/threads?maxResults={size}",
    );
}

#[test]
fn history_list_max_results_bounds() {
    max_results_bounds(
        "users.history.list",
        json!({"userId": "me", "startHistoryId": "1000", "maxResults": 2}),
        "/gmail/v1/users/me/history?maxResults={size}&startHistoryId=1000",
    );
}
