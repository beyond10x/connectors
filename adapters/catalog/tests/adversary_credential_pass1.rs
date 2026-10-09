//! Adversarial pass on a selection's `credential` field: the parameters through
//! which a pinned document passes the credential the connection already sends
//! in its authentication header. The guide (`docs/local-catalog-provider.md`,
//! selection format) says each "is never declared, never required and never
//! sent, and an input carrying it under any spelling is refused as
//! `invalid_input` before any request". These cases drive that sentence, the
//! shipped Slack selection sets and the configuration revision. Every token,
//! id and message is synthetic. No network and no live credential.
use connectors_catalog::{
    bundle::{self, Bundle},
    ingest, inventory,
};
use connectors_catalog_provider::{Effect, Engine, Selection};
use connectors_core::{ErrorCode, Result};
use connectors_host::local::{filesystem, runtime::Bootstrap};
use connectors_sdk::{AuthenticatedHttp, AuthenticatedWrite, HttpResponse, WriteMethod};
use serde_json::{Value, json};
use std::{
    fs,
    os::unix::fs::PermissionsExt,
    path::Path,
    process::Command,
    sync::{Arc, Mutex},
};

fn root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
}
fn slack_bundle() -> Bundle {
    bundle::load(&root().join("generated/bundles"), "slack").unwrap()
}
fn shipped_in(relative: &str) -> Vec<Selection> {
    let file: Value = serde_json::from_slice(&fs::read(root().join(relative)).unwrap()).unwrap();
    serde_json::from_value(file["operations"].clone()).unwrap()
}
fn slack(selections: Value) -> Result<Engine> {
    let selections: Vec<Selection> = serde_json::from_value(selections).unwrap();
    Engine::new(&slack_bundle(), "/api", &selections)
}

type Call = (Vec<String>, Vec<(String, String)>);
/// Records every GET and answers `{}`.
#[derive(Default, Clone)]
struct Reads {
    calls: Arc<Mutex<Vec<Call>>>,
}
#[async_trait::async_trait]
impl AuthenticatedHttp for Reads {
    async fn get(&self, path: &[&str], query: &[(&str, String)]) -> Result<HttpResponse> {
        self.calls.lock().unwrap().push((
            path.iter().map(|s| s.to_string()).collect(),
            query
                .iter()
                .map(|(k, v)| (k.to_string(), v.clone()))
                .collect(),
        ));
        Ok(HttpResponse {
            status: 200,
            headers: Default::default(),
            body: br#"{"ok":true}"#.to_vec(),
        })
    }
}
impl Reads {
    fn calls(&self) -> Vec<Call> {
        self.calls.lock().unwrap().clone()
    }
}

type Sent = Arc<Mutex<Vec<(Vec<(String, String)>, Value)>>>;
/// Records the one write it is given and answers `200 {"ok":true}`.
struct Write {
    sent: Sent,
}
#[async_trait::async_trait]
impl AuthenticatedWrite for Write {
    async fn send_json(
        self: Box<Self>,
        _method: WriteMethod,
        _segments: &[&str],
        query: &[(&str, String)],
        body: &Value,
    ) -> Result<HttpResponse> {
        self.sent.lock().unwrap().push((
            query
                .iter()
                .map(|(k, v)| (k.to_string(), v.clone()))
                .collect(),
            body.clone(),
        ));
        Ok(HttpResponse {
            status: 200,
            headers: Default::default(),
            body: br#"{"ok":true}"#.to_vec(),
        })
    }
}

fn names(call: &Call) -> Vec<&str> {
    call.1.iter().map(|(k, _)| k.as_str()).collect()
}

// ---------------------------------------------------------------------------
// The shipped Slack selections, through the engine.

/// Each shipped credential read, with every optional parameter it carries
/// filled, sends exactly the caller's parameters and no `token` pair, and
/// declares no `token` under any case.
#[tokio::test]
async fn shipped_credential_reads_send_no_token_pair() {
    let bot = Engine::new(
        &slack_bundle(),
        "/api",
        &shipped_in("providers/slack/operations.json"),
    )
    .unwrap();
    let user = Engine::new(
        &slack_bundle(),
        "/api",
        &shipped_in("providers/slack/user-operations.json"),
    )
    .unwrap();
    for (engine, id, input, expected) in [
        (&bot, "auth.test", json!({}), vec![]),
        (&bot, "emoji.list", json!({}), vec![]),
        (
            &bot,
            "team.info",
            json!({"team": "T0FIXTURE"}),
            vec![("team", "T0FIXTURE")],
        ),
        (
            &user,
            "search.messages",
            json!({"query": "fixture", "count": 5, "page": 2, "highlight": true,
                   "sort": "timestamp", "sort_dir": "asc"}),
            vec![
                ("count", "5"),
                ("highlight", "true"),
                ("page", "2"),
                ("query", "fixture"),
                ("sort", "timestamp"),
                ("sort_dir", "asc"),
            ],
        ),
    ] {
        let declaration = engine
            .declarations(&[Effect::Read])
            .into_iter()
            .find(|d| d.id == id)
            .unwrap();
        let properties = declaration.input_schema["properties"].as_object().unwrap();
        assert!(
            properties.keys().all(|k| !k.eq_ignore_ascii_case("token")),
            "`{id}` declares {:?}",
            properties.keys().collect::<Vec<_>>()
        );
        assert_eq!(
            declaration.input_schema["additionalProperties"],
            json!(false),
            "`{id}`"
        );
        let http = Reads::default();
        engine.read(&http, "one", id, input).await.unwrap();
        let calls = http.calls();
        assert_eq!(calls.len(), 1, "`{id}`");
        let mut sent: Vec<(String, String)> = calls[0].1.clone();
        sent.sort();
        let expected: Vec<(String, String)> = expected
            .into_iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect();
        assert_eq!(sent, expected, "`{id}`");
        assert!(!names(&calls[0]).contains(&"token"), "`{id}`");
    }
}

/// Every spelling of `token` and every shape of value a caller could try is
/// refused as `invalid_input` with nothing sent, on each shipped credential
/// read.
#[tokio::test]
async fn every_token_spelling_and_shape_is_refused_before_any_request() {
    let bot = Engine::new(
        &slack_bundle(),
        "/api",
        &shipped_in("providers/slack/operations.json"),
    )
    .unwrap();
    let user = Engine::new(
        &slack_bundle(),
        "/api",
        &shipped_in("providers/slack/user-operations.json"),
    )
    .unwrap();
    let base = |id: &str| {
        if id == "search.messages" {
            json!({"query": "fixture"})
        } else {
            json!({})
        }
    };
    for (engine, id) in [
        (&bot, "auth.test"),
        (&bot, "emoji.list"),
        (&bot, "team.info"),
        (&user, "search.messages"),
    ] {
        let mut attempts: Vec<Value> = Vec::new();
        for spelling in [
            "token",
            "Token",
            "TOKEN",
            "tok%65n",
            "token%00",
            "token ",
            " token",
            "token[]",
            "token[0]",
            "query.token",
            "headers",
            "header",
            "query",
            "extra",
            "params",
            "body",
        ] {
            if id == "search.messages" && spelling == "query" {
                continue;
            }
            let mut input = base(id);
            input[spelling] = json!("fixture-smuggled-token");
            attempts.push(input);
            let mut input = base(id);
            input[spelling] = json!({"token": "fixture-smuggled-token"});
            attempts.push(input);
        }
        for value in [
            json!(["fixture-a", "fixture-b"]),
            json!(""),
            json!(null),
            json!(1),
        ] {
            let mut input = base(id);
            input["token"] = value;
            attempts.push(input);
        }
        for input in attempts {
            let http = Reads::default();
            let error = engine
                .read(&http, "one", id, input.clone())
                .await
                .err()
                .unwrap_or_else(|| panic!("`{id}` accepted {input}"));
            assert_eq!(error.code, ErrorCode::InvalidInput, "`{id}` {input}");
            assert!(http.calls().is_empty(), "`{id}` {input}");
        }
    }
}

/// A value that spells `&token=` inside another parameter stays that
/// parameter's one value: the engine hands the transport a single `query`
/// pair, which the transport percent-encodes (`connectors-host`
/// `http.rs`, `query_pairs_mut`).
#[tokio::test]
async fn a_token_inside_another_value_stays_inside_it() {
    let user = Engine::new(
        &slack_bundle(),
        "/api",
        &shipped_in("providers/slack/user-operations.json"),
    )
    .unwrap();
    let http = Reads::default();
    user.read(
        &http,
        "one",
        "search.messages",
        json!({"query": "fixture&token=fixture-smuggled-token"}),
    )
    .await
    .unwrap();
    assert_eq!(
        http.calls()[0].1,
        vec![(
            "query".to_string(),
            "fixture&token=fixture-smuggled-token".to_string()
        )]
    );
}

/// The user-token connection reaches `search.messages` and nothing the bot
/// set selects; the bot connection does not reach search. Each refusal is
/// `not_found` before any request.
#[tokio::test]
async fn the_user_connection_cannot_reach_bot_operations_nor_the_bot_search() {
    let bot = Engine::new(
        &slack_bundle(),
        "/api",
        &shipped_in("providers/slack/operations.json"),
    )
    .unwrap();
    let user = Engine::new(
        &slack_bundle(),
        "/api",
        &shipped_in("providers/slack/user-operations.json"),
    )
    .unwrap();
    let ids: Vec<String> = user
        .declarations(&[Effect::Read, Effect::Write])
        .into_iter()
        .map(|d| d.id)
        .collect();
    assert_eq!(ids, vec!["search.messages".to_string()]);
    for id in [
        "auth.test",
        "team.info",
        "emoji.list",
        "conversations.list",
        "conversations.history",
        "conversations.replies",
        "users.list",
    ] {
        let http = Reads::default();
        let error = user
            .read(&http, "one", id, json!({}))
            .await
            .err()
            .unwrap_or_else(|| panic!("the user connection reached `{id}`"));
        assert_eq!(error.code, ErrorCode::NotFound, "`{id}`");
        assert!(http.calls().is_empty(), "`{id}`");
    }
    let http = Reads::default();
    let error = bot
        .read(&http, "one", "search.messages", json!({"query": "fixture"}))
        .await
        .err()
        .expect("the bot connection reached search.messages");
    assert_eq!(error.code, ErrorCode::NotFound);
    assert!(http.calls().is_empty());
}

/// `withhold` and `bounds` on other parameters still hold beside a
/// credential, and `withhold` still refuses the required `query`.
#[tokio::test]
async fn withhold_and_bounds_on_other_parameters_still_hold_beside_a_credential() {
    let engine = slack(json!([
        {"id": "team.info", "operation_id": "team_info", "effect": "read",
         "credential": ["token"], "withhold": ["team"]},
        {"id": "search.messages", "operation_id": "search_messages", "effect": "read",
         "credential": ["token"], "bounds": {"count": {"minimum": 1, "maximum": 100}},
         "withhold": ["highlight"]}
    ]))
    .unwrap();
    for (id, input) in [
        ("team.info", json!({"team": "T0FIXTURE"})),
        ("search.messages", json!({"query": "q", "count": 0})),
        ("search.messages", json!({"query": "q", "count": 101})),
        ("search.messages", json!({"query": "q", "count": "1e2"})),
        ("search.messages", json!({"query": "q", "highlight": true})),
        ("search.messages", json!({"count": 5})),
    ] {
        let http = Reads::default();
        let error = engine
            .read(&http, "one", id, input.clone())
            .await
            .err()
            .unwrap_or_else(|| panic!("`{id}` accepted {input}"));
        assert_eq!(error.code, ErrorCode::InvalidInput, "`{id}` {input}");
        assert!(http.calls().is_empty(), "`{id}` {input}");
    }
    let message = slack(json!([
        {"id": "search.messages", "operation_id": "search_messages", "effect": "read",
         "credential": ["token"], "withhold": ["query"]}
    ]))
    .err()
    .expect("withholding the required query loaded")
    .message;
    assert!(message.contains("withholds `query`, which is required"), "{message}");
}

// ---------------------------------------------------------------------------
// A fixture document: the same name in the query and a header, and a second
// required header beside a credential.

fn fixture() -> Bundle {
    let string = json!({"type": "string"});
    let ok = json!({"200": {"description": "OK", "content": {"application/json": {}}}});
    let bytes = serde_json::to_vec(&json!({
        "openapi": "3.0.0",
        "info": {"title": "credential-adversary", "version": "1"},
        "paths": {
            "/v1/both": {"get": {
                "operationId": "both",
                "parameters": [
                    {"name": "token", "in": "query", "required": true, "schema": string},
                    {"name": "token", "in": "header", "required": true, "schema": string},
                    {"name": "q", "in": "query", "schema": string}
                ],
                "responses": ok
            }},
            "/v1/two-headers": {"get": {
                "operationId": "twoHeaders",
                "parameters": [
                    {"name": "token", "in": "header", "required": true, "schema": string},
                    {"name": "X-Tenant", "in": "header", "required": true, "schema": string}
                ],
                "responses": ok
            }},
            "/v1/cookie": {"get": {
                "operationId": "cookie",
                "parameters": [
                    {"name": "token", "in": "cookie", "required": false, "schema": string}
                ],
                "responses": ok
            }}
        }
    }))
    .unwrap();
    Bundle {
        provider: "credential-adversary".into(),
        source: ingest("credential-adversary.json", &bytes).unwrap(),
        inventory: inventory::extract(&serde_json::from_slice(&bytes).unwrap()),
        auth_profile: "fixture.token".into(),
    }
}
fn load(selection: Value) -> Result<Engine> {
    Engine::new(
        &fixture(),
        "/v1",
        &[serde_json::from_value(selection).unwrap()],
    )
}

/// One name in the query and in a header: both leave, the selection loads,
/// and neither the declaration nor the request carries it.
#[tokio::test]
async fn a_credential_in_the_query_and_a_header_leaves_from_both() {
    let engine = load(json!({"id": "both", "operation_id": "both", "effect": "read",
                            "credential": ["token"]}))
    .unwrap();
    let declaration = engine.declarations(&[Effect::Read]).remove(0);
    assert!(declaration.input_schema["properties"].get("token").is_none());
    let http = Reads::default();
    engine
        .read(&http, "one", "both", json!({"q": "x"}))
        .await
        .unwrap();
    assert_eq!(
        http.calls()[0].1,
        vec![("q".to_string(), "x".to_string())]
    );
}

/// A credential frees only its own header: another required header still
/// refuses the selection, and a cookie of the credential's name is refused
/// as not a query or header parameter.
#[test]
fn a_credential_frees_only_its_own_header() {
    let message = load(json!({"id": "two", "operation_id": "twoHeaders", "effect": "read",
                             "credential": ["token"]}))
    .err()
    .expect("a second required header loaded")
    .message;
    assert!(
        message.contains("needs a header parameter this transport does not carry"),
        "{message}"
    );
    let message = load(json!({"id": "cookie", "operation_id": "cookie", "effect": "read",
                             "credential": ["token"]}))
    .err()
    .expect("a cookie credential loaded")
    .message;
    assert!(message.contains("as the credential"), "{message}");
}

// ---------------------------------------------------------------------------
// Writes: the pinned Slack document passes `token` as a required header on
// every write (`chat_postMessage` among 101 operations), so a write selection
// naming it as the credential loads.

/// The guide: an input carrying the credential "under any spelling is
/// refused as `invalid_input` before any request". A write's open body is
/// part of that input, and a `token` inside it is sent.
#[tokio::test]
async fn a_credential_in_a_writes_body_is_refused_before_any_request() {
    let engine = slack(json!([
        {"id": "chat.postMessage", "operation_id": "chat_postMessage", "effect": "write",
         "credential": ["token"]}
    ]))
    .expect("chat.postMessage with token as the credential loads");
    let http = Reads::default();
    let input = json!({"body": {"channel": "C0FIXTURE01", "text": "fixture",
                                "token": "fixture-smuggled-token"}});
    let outcome = engine
        .prepare(&http, "one", "chat.postMessage", input)
        .await;
    let sent: Sent = Arc::default();
    if let Ok(prepared) = outcome {
        let _ = prepared
            .execute(Box::new(Write { sent: sent.clone() }))
            .await;
    }
    let sent = sent.lock().unwrap().clone();
    assert!(
        sent.is_empty(),
        "a body carrying the credential was sent: {:?}",
        sent.iter().map(|(_, body)| body.clone()).collect::<Vec<_>>()
    );
}

/// The same body admitted by declaration: `body_keys` naming the credential
/// is not refused at load, so the selection explicitly admits sending it.
#[test]
fn body_keys_naming_the_credential_is_refused_at_load() {
    let loaded = slack(json!([
        {"id": "chat.postMessage", "operation_id": "chat_postMessage", "effect": "write",
         "credential": ["token"], "body_keys": ["channel", "text", "token"]}
    ]));
    assert!(
        loaded.is_err(),
        "a selection whose body_keys admit its credential loaded"
    );
}

/// A guarded write whose preflight reads an operation that also passes the
/// credential as `token`: the guard maps a caller input onto the probe's
/// `token`, the selection loads, and the caller's value is sent as `token`
/// on the preflight GET.
#[tokio::test]
async fn a_guard_cannot_send_a_caller_value_as_the_credential() {
    let selection = json!([
        {"id": "chat.postMessage", "operation_id": "chat_postMessage", "effect": "write",
         "credential": ["token"],
         "guard": {
             "preflight": {"operation_id": "team_info", "values": {"token": "team_ref"},
                           "checks": [{"pointer": "/ok", "expect": {"literal": "true"}}]},
             "postflight": {"checks": []}
         }}
    ]);
    let Ok(engine) = slack(selection) else {
        // Refused at load: nothing a caller supplies reaches the probe.
        return;
    };
    let http = Reads::default();
    let _ = engine
        .prepare(
            &http,
            "one",
            "chat.postMessage",
            json!({"team_ref": "fixture-smuggled-token",
                   "body": {"channel": "C0FIXTURE01", "text": "fixture"}}),
        )
        .await;
    let sent: Vec<(String, String)> = http.calls().into_iter().flat_map(|c| c.1).collect();
    assert!(
        !sent.iter().any(|(k, _)| k == "token"),
        "the preflight sent a caller value as the credential: {sent:?}"
    );
}

// ---------------------------------------------------------------------------
// The configuration revision, through the provider process.

fn private(path: &Path, bytes: &[u8]) {
    fs::write(path, bytes).unwrap();
    fs::set_permissions(path, fs::Permissions::from_mode(0o600)).unwrap();
}

/// The guide's Slack bot configuration with the given operations, made
/// loadable here against an `api_base` nothing listens on.
fn bootstrap(directory: &Path, name: &str, operations: Value) -> Bootstrap {
    let guide = fs::read_to_string(root().join("../../docs/catalog-slack.md")).unwrap();
    let mut config = guide
        .split("```json\n")
        .skip(1)
        .filter_map(|rest| rest.split_once("\n```").map(|(body, _)| body))
        .filter_map(|body| serde_json::from_str::<Value>(body).ok())
        .find(|config| config["provider"] == "slack" && config["auth"]["profile"] == "slack.bot")
        .expect("the guide documents the Slack bot configuration");
    let operations_file = directory.join(format!("{name}.operations.json"));
    private(
        &operations_file,
        &serde_json::to_vec(&json!({
            "format": "connectors-catalog-operations/1",
            "provider": "slack",
            "operations": operations,
        }))
        .unwrap(),
    );
    config["bundle_directory"] = json!(root().join("generated/bundles").canonicalize().unwrap());
    config["operations_file"] = json!(operations_file);
    config["api_base"] = json!("https://localhost:9/api");
    config.as_object_mut().unwrap().remove("ca_file");
    let path = directory.join(format!("{name}.json"));
    private(&path, &serde_json::to_vec(&config).unwrap());
    let output = Command::new(env!("CARGO_BIN_EXE_connectors-catalog-provider"))
        .arg("--local-config")
        .arg(&path)
        .arg("--print-local-bootstrap")
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{name}: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).unwrap()
}

/// Naming `token` as the credential changes the configuration revision and
/// the descriptor revision, so a cache keyed on either is refused; the same
/// selection written twice keeps both.
#[test]
fn naming_a_credential_changes_the_configuration_revision() {
    let directory = tempfile::tempdir().unwrap();
    let private_directory = directory.path().join("private");
    filesystem::directory(&private_directory, true, true).unwrap();
    let plain = json!([{"id": "team.info", "operation_id": "team_info", "effect": "read"}]);
    let named = json!([{"id": "team.info", "operation_id": "team_info", "effect": "read",
                        "credential": ["token"]}]);
    let before = bootstrap(&private_directory, "plain", plain);
    let after = bootstrap(&private_directory, "named", named.clone());
    let again = bootstrap(&private_directory, "again", named);
    assert_ne!(before.configuration_revision, after.configuration_revision);
    assert_ne!(
        before.descriptor().unwrap().revision,
        after.descriptor().unwrap().revision
    );
    assert_eq!(after.configuration_revision, again.configuration_revision);
    assert_eq!(
        after.descriptor().unwrap().revision,
        again.descriptor().unwrap().revision
    );
}
