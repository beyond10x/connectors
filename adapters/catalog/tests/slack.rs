//! Slack conversations, users, workspace, emoji and search through the
//! catalog provider.
//!
//! The two shipped selection sets — the bot token's and the user token's
//! (`search.messages` alone) — are pinned by id and source operation, resolve
//! against the committed bundle compiled from the OpenAPI projection of the
//! pinned Slack Web API Swagger 2.0 document, and are cited row by row in
//! `docs/catalog-slack.md`. Each read runs through the provider child against
//! a disposable HTTPS fixture: the exact request (path, query including the
//! time window, `Authorization: Bearer …`, never a `token` query parameter)
//! and the returned body bytes are asserted, every list walks two pages to an
//! empty `response_metadata.next_cursor`, and the `auth.test` identity read
//! yields the token's user id. `auth.test`, `team.info`, `emoji.list` and
//! `search.messages`, whose pinned `token` parameter is required, ship with
//! `token` named as the selection's credential; withholding it instead is
//! still refused. The four writes, `chat.postMessage`, `conversations.open`,
//! `chat.update` and `chat.delete`, run on private protocol two through
//! prepare and commit, the transport the host drives once it holds an
//! approval: the exact JSON request after the guard's `auth.test` read,
//! refusal without an approval, an ambiguous dispatch left unknown and sent
//! once, and Slack's `200` with `ok: false` never applied: refused when an
//! edit or a delete names a declared refusal, unknown otherwise. Every id,
//! name and message is synthetic. No live credential and no network.
#![cfg(unix)]
use connectors_catalog::bundle;
use connectors_catalog_provider::{Effect, Engine, Selection};
use connectors_host::local::{
    config::{Adapter, Executable, Restart, Startup},
    filesystem,
    runtime::{
        self, Bootstrap, Child, Failure, PrivateProtocol, Refusal, WriteEffect, WriteResult,
    },
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
/// A fictional user token, for the search connection.
const USER_TOKEN: &str = "fixture-slack-user-token";
const USER_HEADER: &str = "Bearer fixture-slack-user-token";
const USER_PROFILE: &str = "slack.user";
/// The pinned document and its committed projection, relative to this crate.
const UPSTREAM: &str = "../slack/upstream/slack_web_openapi_v2_without_examples.json";
const PROJECTION: &str = "../slack/generated/slack-web.openapi.json";
const SOURCE_SHA256: &str = "8b92da26a3c5b11d20042a9f36d81f1fa6fc9382c5ddc471babb68b91936bc3a";

/// The bot selection set's ids, their pinned `operationId` and the path the
/// bundle records. A renamed, dropped or added id fails here.
const SHIPPED: [(&str, &str, &str); 9] = [
    ("auth.test", "auth_test", "/api/auth.test"),
    ("files.info", "files_info", "/api/files.info"),
    ("files.list", "files_list", "/api/files.list"),
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
    ("emoji.list", "emoji_list", "/api/emoji.list"),
    ("team.info", "team_info", "/api/team.info"),
    ("users.list", "users_list", "/api/users.list"),
];
/// The user selection set: the three conversation reads and `users.list`,
/// which a user token answers for every conversation that user can read, and
/// search. No write.
const USER_SHIPPED: [(&str, &str, &str); 5] = [
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
    ("search.messages", "search_messages", "/api/search.messages"),
    ("users.list", "users_list", "/api/users.list"),
];
/// The methods whose pinned `token` is required, so their selections name it
/// as the credential instead of withholding it.
const CREDENTIAL: [&str; 4] = ["auth_test", "emoji_list", "search_messages", "team_info"];

fn root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
}
fn shipped() -> Vec<Selection> {
    shipped_in("providers/slack/operations.json")
}
/// The user token's selection set, for the search connection.
fn user_shipped() -> Vec<Selection> {
    shipped_in("providers/slack/user-operations.json")
}
fn shipped_in(relative: &str) -> Vec<Selection> {
    let file: Value = serde_json::from_slice(&fs::read(root().join(relative)).unwrap()).unwrap();
    assert_eq!(file["format"], "connectors-catalog-operations/1");
    assert_eq!(file["provider"], "slack");
    assert!(file.get("feed").is_none(), "{relative}");
    serde_json::from_value(file["operations"].clone()).unwrap()
}
fn committed() -> bundle::Bundle {
    bundle::load(&root().join("generated/bundles"), "slack").unwrap()
}
fn pinned() -> Value {
    serde_json::from_slice(&fs::read(root().join(UPSTREAM)).unwrap()).unwrap()
}

#[test]
fn shipped_slack_selections_are_exactly_the_bot_reads_and_writes_and_the_user_reads() {
    let bundle = committed();
    assert_eq!(bundle.auth_profile, PROFILE);
    for (selections, shipped) in [
        (shipped(), &SHIPPED[..]),
        (user_shipped(), &USER_SHIPPED[..]),
    ] {
        let engine = Engine::new(&bundle, BASE, &selections).unwrap();
        let mut declared: Vec<String> = engine
            .declarations(&[Effect::Read, Effect::Write])
            .into_iter()
            .map(|o| o.id)
            .collect();
        declared.sort();
        // The bot set's file download names no bundle operation; it is pinned
        // in `tests/slack_files.rs`.
        // The bot set's four writes are pinned in
        // `the_two_writes_ship_as_guarded_writes_with_a_closed_json_body`.
        let mut expected: Vec<&str> = shipped.iter().map(|(id, _, _)| *id).collect();
        let mut writes: Vec<&str> = Vec::new();
        if shipped.len() == SHIPPED.len() {
            expected.push("file.download");
            writes = WRITES.iter().map(|(id, _, _)| *id).collect();
            expected.extend(&writes);
        }
        expected.sort();
        assert_eq!(declared, expected);
        let mut declared_writes: Vec<String> = engine
            .declarations(&[Effect::Write])
            .into_iter()
            .map(|o| o.id)
            .collect();
        declared_writes.sort();
        assert_eq!(declared_writes, writes);
        for (id, operation_id, path) in shipped.iter().copied() {
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
            let document = pinned();
            let source = &document["paths"][source_path]["get"];
            assert_eq!(source["operationId"], operation_id, "`{id}`");
            // The credential travels in the header only. Where the document
            // marks its `token` required, the selection names it as the
            // credential; otherwise it withholds it. Either way `token` is
            // never declared.
            let token_required = source["parameters"]
                .as_array()
                .unwrap()
                .iter()
                .any(|p| p["name"] == "token" && p["required"] == true);
            assert_eq!(token_required, CREDENTIAL.contains(&operation_id), "`{id}`");
            let token = vec!["token".to_owned()];
            if token_required {
                assert_eq!(selection.credential, token, "`{id}`");
                assert!(selection.withhold.is_empty(), "`{id}`");
            } else {
                assert_eq!(selection.withhold, token, "`{id}`");
                assert!(selection.credential.is_empty(), "`{id}`");
            }
            let declaration = engine
                .declarations(&[Effect::Read])
                .into_iter()
                .find(|o| o.id == id)
                .unwrap();
            assert!(
                declaration.input_schema["properties"]
                    .get("token")
                    .is_none(),
                "`{id}` declares token"
            );
        }
    }
    // Search is the user token's alone: the bot connection never exposes it.
    assert!(
        shipped()
            .iter()
            .all(|selection| selection.operation_id != "search_messages")
    );
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

fn selection(id: &str, operation_id: &str, withhold: &[&str]) -> Selection {
    serde_json::from_value(json!({
        "id": id, "operation_id": operation_id, "effect": "read", "withhold": withhold
    }))
    .unwrap()
}
fn refusal(selections: &[Selection]) -> String {
    match Engine::new(&committed(), BASE, selections) {
        Ok(_) => panic!("loaded"),
        Err(error) => error.message,
    }
}

/// `team.info`, `emoji.list` and `search.messages` declare `token` as a
/// required query parameter and `auth.test` as a required header
/// (`docs/catalog-slack.md`, "Credential parameters"). `withhold` still
/// refuses a required parameter, and without either field the bot token would
/// be a required input sent in the query, or `auth.test` would be refused for
/// its header; so each ships with `token` named as its credential, and only
/// that way.
#[test]
fn methods_whose_pinned_token_is_required_ship_with_it_as_the_credential() {
    for (id, operation_id) in [
        ("team.info", "team_info"),
        ("emoji.list", "emoji_list"),
        ("search.messages", "search_messages"),
    ] {
        let message = refusal(&[selection(id, operation_id, &["token"])]);
        assert!(
            message.contains("withholds `token`, which is required"),
            "`{id}`: {message}"
        );
        let engine = Engine::new(&committed(), BASE, &[selection(id, operation_id, &[])])
            .unwrap_or_else(|error| panic!("`{id}`: {}", error.message));
        let declaration = engine.declarations(&[Effect::Read]).remove(0);
        assert!(
            declaration.input_schema["required"]
                .as_array()
                .unwrap()
                .contains(&json!("token")),
            "`{id}` would take the token as input"
        );
    }
    for withhold in [&["token"][..], &[][..]] {
        let message = refusal(&[selection("auth.test", "auth_test", withhold)]);
        assert!(
            message.contains("needs a header parameter this transport does not carry"),
            "{message}"
        );
    }
    // Each shipped selection of the four names `token` as its credential.
    let all: Vec<Selection> = shipped().into_iter().chain(user_shipped()).collect();
    for operation_id in CREDENTIAL {
        let selection = all
            .iter()
            .find(|s| s.operation_id == operation_id)
            .unwrap_or_else(|| panic!("`{operation_id}` is not shipped"));
        assert_eq!(selection.credential, ["token"], "`{operation_id}`");
    }
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
/// `cursor` and `limit` on all four, `oldest` and `latest` on the two
/// message reads, `ts` and `channel` on replies. The only stated page bound is
/// `conversations.list`'s 1,000, and only it carries a bound: `users.list`'s
/// `limit` states none, only that its absence asks for the whole list.
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
        ("/users.list", &["cursor", "limit", "include_locale"][..]),
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
    let users_limit = document["paths"]["/users.list"]["get"]["parameters"]
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["name"] == "limit")
        .unwrap()["description"]
        .as_str()
        .unwrap()
        .to_owned();
    assert!(
        users_limit.contains("deliver you the entire result set"),
        "{users_limit}"
    );
    // The same bound on both sets' `conversations.list`; search is pinned
    // below.
    for selection in shipped()
        .into_iter()
        .chain(user_shipped())
        .filter(|s| s.id != "search.messages")
    {
        let bound = selection.bounds.get("limit");
        if selection.id == "conversations.list" {
            assert_eq!(bound.and_then(|b| b.maximum), Some(1000));
        } else {
            assert!(selection.bounds.is_empty(), "`{}`", selection.id);
        }
    }
    // Search pages by `page` and `count`, not by cursor, and the pinned
    // `count` states its maximum of 100, which the user selection bounds.
    let search = names("/search.messages");
    for name in ["query", "count", "page", "sort", "sort_dir", "highlight"] {
        assert!(search.iter().any(|d| d == name), "search lacks `{name}`");
    }
    assert!(!search.iter().any(|d| d == "cursor"));
    let count = document["paths"]["/search.messages"]["get"]["parameters"]
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["name"] == "count")
        .unwrap()["description"]
        .as_str()
        .unwrap()
        .to_owned();
    assert!(count.contains("Maximum of `100`"), "{count}");
    let selections = user_shipped();
    let search = selections
        .iter()
        .find(|s| s.id == "search.messages")
        .unwrap();
    let bounds = &search.bounds;
    assert_eq!(bounds.len(), 1);
    assert_eq!(
        bounds.get("count").map(|b| (b.minimum, b.maximum)),
        Some((Some(1), Some(100)))
    );
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
        ("users.list", "users_list", "/api/users.list", "none"),
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

/// Each discovery read the bot set adds, and search, has its row in the guide.
#[test]
fn guide_cites_each_discovery_read_and_search() {
    let guide = fs::read_to_string(root().join("../../docs/catalog-slack.md")).unwrap();
    let rows: Vec<&str> = guide.lines().filter(|l| l.starts_with('|')).collect();
    for (id, operation_id, path, scope) in [
        ("team.info", "team_info", "/api/team.info", "`team:read`"),
        (
            "emoji.list",
            "emoji_list",
            "/api/emoji.list",
            "`emoji:read`",
        ),
        ("auth.test", "auth_test", "/api/auth.test", "none"),
        (
            "search.messages",
            "search_messages",
            "/api/search.messages",
            "`search:read`",
        ),
    ] {
        let row = rows
            .iter()
            .find(|row| row.contains(&format!("| `{id}` |")))
            .unwrap_or_else(|| panic!("no row for `{id}`"));
        assert!(row.contains(&format!("`{operation_id}`")), "{row}");
        assert!(row.contains(&format!("`GET {path}`")), "{row}");
        assert!(row.contains(scope), "{row}");
    }
}

/// The guide's user-token section has a row for each read the user set adds,
/// naming the user scopes it needs, and states that the set has no write.
#[test]
fn guide_cites_each_user_token_read_with_its_user_scopes() {
    let guide = fs::read_to_string(root().join("../../docs/catalog-slack.md")).unwrap();
    let heading = "## Reads and search on a user-token connection";
    let section = guide
        .split_once(heading)
        .unwrap_or_else(|| panic!("no section `{heading}`"))
        .1;
    let section = section.split("\n## ").next().unwrap();
    let rows: Vec<&str> = section.lines().filter(|l| l.starts_with('|')).collect();
    for (id, operation_id, path, scopes) in [
        (
            "conversations.list",
            "conversations_list",
            "/api/conversations.list",
            &["`channels:read`", "`groups:read`"][..],
        ),
        (
            "conversations.history",
            "conversations_history",
            "/api/conversations.history",
            &[
                "`channels:history`",
                "`groups:history`",
                "`im:history`",
                "`mpim:history`",
            ][..],
        ),
        (
            "conversations.replies",
            "conversations_replies",
            "/api/conversations.replies",
            &[
                "`channels:history`",
                "`groups:history`",
                "`im:history`",
                "`mpim:history`",
            ][..],
        ),
        (
            "users.list",
            "users_list",
            "/api/users.list",
            &["`users:read`"][..],
        ),
        (
            "search.messages",
            "search_messages",
            "/api/search.messages",
            &["`search:read`"][..],
        ),
    ] {
        let row = rows
            .iter()
            .find(|row| row.contains(&format!("| `{id}` |")))
            .unwrap_or_else(|| panic!("no user-token row for `{id}`"));
        assert!(row.contains(&format!("`{operation_id}`")), "{row}");
        assert!(row.contains(&format!("`GET {path}`")), "{row}");
        for scope in scopes {
            assert!(row.contains(scope), "`{id}` lacks {scope}: {row}");
        }
    }
    assert!(
        section.contains("no write"),
        "the section must say the user set has no write"
    );
}

/// The parity page scores the conversation and user-list rows by what the
/// user-token profile serves, naming `slack.user` and its selection file.
#[test]
fn the_parity_page_names_the_user_profile_for_the_user_token_reads() {
    let page = fs::read_to_string(root().join("../../docs/fluxplane-plugin-parity.md")).unwrap();
    let section = page.split_once("\n## slack\n").unwrap().1;
    let section = section.split("\n## ").next().unwrap();
    for (row_id, operation) in [
        ("slack.thread", "conversations.replies"),
        ("slack.message.list", "conversations.history"),
        ("slack.channel.list", "conversations.list"),
        ("slack.user.list", "users.list"),
    ] {
        let row = section
            .lines()
            .find(|l| l.starts_with(&format!("| `{row_id}` |")))
            .unwrap_or_else(|| panic!("no parity row `{row_id}`"));
        assert!(row.contains(&format!("`{operation}`")), "{row}");
        assert!(
            row.contains("`slack.user`"),
            "`{row_id}` names no user profile: {row}"
        );
        assert!(row.contains("user-operations.json"), "{row}");
        assert!(row.contains("| covered |"), "{row}");
    }
}

/// The guide's bot configuration example.
fn documented_config() -> Value {
    documented_config_for(PROFILE)
}
/// The guide's configuration example for the connection under `profile`.
fn documented_config_for(profile: &str) -> Value {
    let guide = fs::read_to_string(root().join("../../docs/catalog-slack.md")).unwrap();
    let example = guide
        .split("```json\n")
        .skip(1)
        .filter_map(|rest| rest.split_once("\n```").map(|(body, _)| body))
        .find(|body| {
            body.contains("\"provider\": \"slack\"")
                && body.contains(&format!("\"profile\": \"{profile}\""))
        })
        .unwrap_or_else(|| panic!("the documented `{profile}` configuration"));
    serde_json::from_str::<Value>(example).unwrap()
}

/// The search connection: the same bundle and base, a user token under
/// `slack.user`, the same `auth.test` identity read, and the user selection set.
#[test]
fn the_documented_user_profile_is_a_bearer_user_token_selecting_the_user_reads() {
    let config = documented_config_for(USER_PROFILE);
    let bot = documented_config();
    assert_eq!(config["api_base"], bot["api_base"]);
    assert_eq!(config["provider"], "slack");
    assert_ne!(config["instance"], bot["instance"]);
    let auth = &config["auth"];
    assert_eq!(auth["profile"], USER_PROFILE);
    assert_eq!(auth["scheme"], Value::Null);
    assert_eq!(auth["header"], "Authorization");
    assert_eq!(auth["bearer"], true);
    assert_eq!(auth["identity"], bot["auth"]["identity"]);
    assert_eq!(auth["scopes"], Value::Null);
    assert!(
        config["operations_file"]
            .as_str()
            .unwrap()
            .ends_with("adapters/catalog/providers/slack/user-operations.json")
    );
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

/// Route, `Authorization` header, and whether a `token` header was sent, of
/// each fixture request.
type Requests = Arc<Mutex<Vec<(String, Option<String>, bool)>>>;
/// Method, route, `Content-Type` and JSON body (`null` when none) of each
/// fixture request.
type Exchanges = Arc<Mutex<Vec<(String, String, Option<String>, Value)>>>;

/// The `ts` the fixture gives every posted message.
const POSTED_TS: &str = "1780000700.000700";
/// The direct-message conversation the fixture opens with `U0FIXTURE11`.
const DM: &str = "D0FIXTURE11";

/// What `chat.postMessage` answers: to `C0FIXTURE01` or the opened direct
/// message, the pinned success schema's `ok`, `channel`, `ts` and `message`;
/// to any other channel, Slack's refusal, `200` with `ok: false`.
fn posted(body: &Value) -> Value {
    match body["channel"].as_str() {
        Some(channel @ ("C0FIXTURE01" | DM)) => json!({
            "ok": true, "channel": channel, "ts": POSTED_TS,
            "message": {"type": "message", "user": "U0FIXTURE01", "bot_id": "B0FIXTURE01",
                        "text": body["text"], "ts": POSTED_TS}}),
        _ => json!({"ok": false, "error": "channel_not_found"}),
    }
}
/// What `conversations.open` answers: the direct message with `U0FIXTURE11`,
/// or Slack's refusal for any other user.
fn opened(body: &Value) -> Value {
    if body["users"] == "U0FIXTURE11" {
        json!({"ok": true, "channel": {"id": DM}})
    } else {
        json!({"ok": false, "error": "user_not_found"})
    }
}

/// A message in `C0FIXTURE01` that another user posted: the bot token may
/// neither edit nor delete it.
const FOREIGN_TS: &str = "1780000300.000300";
/// A message in `C0FIXTURE01` whose edit Slack answers with an `ok: false`
/// the selection does not declare a refusal (`is_inactive`).
const INACTIVE_TS: &str = "1780000800.000800";

/// Slack's answer to an edit or a delete in `C0FIXTURE01`: the bot's own
/// message (`POSTED_TS`) is `None`, the method's success; any other is its
/// refusal, `200` with `ok: false`, as the pinned error enum names it.
fn refused(body: &Value, cant: &str) -> Option<Value> {
    let error = match (body["channel"].as_str(), body["ts"].as_str()) {
        (Some("C0FIXTURE01"), Some(POSTED_TS)) => return None,
        (Some("C0FIXTURE01"), Some(FOREIGN_TS)) => cant,
        (Some("C0FIXTURE01"), Some(INACTIVE_TS)) => "is_inactive",
        (Some("C0FIXTURE01"), _) => "message_not_found",
        _ => "channel_not_found",
    };
    Some(json!({"ok": false, "error": error}))
}
/// What `chat.update` answers: the pinned success schema's `ok`, `channel`,
/// `ts`, `text` and `message`, or its refusal.
fn edited(body: &Value) -> Value {
    refused(body, "cant_update_message").unwrap_or_else(|| {
        json!({"ok": true, "channel": body["channel"], "ts": body["ts"], "text": body["text"],
               "message": {"type": "message", "user": "U0FIXTURE01", "bot_id": "B0FIXTURE01",
                           "text": body["text"], "edited": {"user": "U0FIXTURE01",
                           "ts": "1780000900.000900"}}})
    })
}
/// What `chat.delete` answers: the pinned success schema's `ok`, `channel`
/// and `ts`, or its refusal.
fn deleted(body: &Value) -> Value {
    refused(body, "cant_delete_message")
        .unwrap_or_else(|| json!({"ok": true, "channel": body["channel"], "ts": body["ts"]}))
}

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
/// An `objs_user` with the members both of its pinned variants require.
fn user(n: u64) -> Value {
    json!({"id": format!("U0FIXTURE{n:02}"), "name": format!("fixture-user-{n}"),
           "deleted": false, "is_bot": false, "is_app_user": false,
           "profile": {"real_name": format!("Fixture User {n}")},
           "team_id": "T0FIXTURE01", "updated": 1_780_000_000u64 + n})
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
        // The private channel only the user token reads: the bot is not in it.
        "/api/conversations.history" | "/api/conversations.replies"
            if has(&format!("channel={PRIVATE}")) =>
        {
            json!({"ok": false, "error": "not_in_channel"})
        }
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
        // Slack's `200` with `ok: false`, the error named by the cursor, and
        // answers that are not Slack's envelope at all.
        "/api/users.list" if query.contains("cursor=fixture-error-") => {
            let name = query
                .split('&')
                .find_map(|p| p.strip_prefix("cursor=fixture-error-"))
                .unwrap();
            json!({"ok": false, "error": name})
        }
        "/api/users.list" if has("cursor=fixture-ratelimited-soon") => {
            json!({"ok": false, "error": "ratelimited"})
        }
        "/api/users.list" if has("cursor=fixture-no-ok") => json!({"members": []}),
        "/api/users.list" if has("cursor=fixture-ok-string") => {
            json!({"ok": "true", "members": []})
        }
        "/api/users.list" if has("cursor=fixture-not-object") => json!([{"ok": true}]),
        "/api/users.list" if has("cursor=fixture-users-2") => json!({
            "ok": true, "members": [user(13)], "cache_ts": 1_780_000_500u64,
            "response_metadata": more("")}),
        "/api/users.list" => json!({
            "ok": true, "members": [user(11), user(12)], "cache_ts": 1_780_000_500u64,
            "response_metadata": more("fixture-users-2")}),
        // An `objs_team` with the members the pinned document requires.
        "/api/team.info" => json!({
            "ok": true,
            "team": {"id": "T0FIXTURE01", "name": "Fixture Team", "domain": "fixture-team",
                     "email_domain": "", "icon": {"image_default": true}}}),
        "/api/emoji.list" => json!({
            "ok": true, "cache_ts": "1780000600.000000",
            "emoji": {"fixture-alias": "alias:fixture-party",
                      "fixture-party": "https://emoji.fixture.example.test/fixture-party.png"}}),
        // `files.list` pages by `page` and `count`, with `paging` (`objs_paging`).
        "/api/files.list" if has("page=2") => json!({
            "ok": true, "files": [file(3)],
            "paging": {"count": 2, "page": 2, "pages": 2, "total": 3}}),
        "/api/files.list" => json!({
            "ok": true, "files": [file(1), file(2)],
            "paging": {"count": 2, "page": 1, "pages": 2, "total": 3}}),
        "/api/files.info" if has("file=F0FIXTURE01") => json!({
            "ok": true, "file": file(1), "comments": []}),
        "/api/files.info" => json!({"ok": false, "error": "file_not_found"}),
        _ => return None,
    })
}

/// An `objs_file` with the members the file record models.
fn file(n: u64) -> Value {
    json!({"id": format!("F0FIXTURE{n:02}"), "name": format!("fixture-{n}.pdf"),
           "filetype": "pdf", "mimetype": "application/pdf", "size": FILE_BYTES.len(),
           "url_private": format!("https://files.slack.com/files-pri/T0FIXTURE01-F0FIXTURE{n:02}/fixture-{n}.pdf")})
}
/// A file's content: not UTF-8, so only a binary read can answer it.
const FILE_BYTES: &[u8] = b"%PDF-1.7\n\xff\xfe\x00\x01 fixture slack file\n";
/// Where the fixture serves file content, as Slack's file host serves
/// `url_private`.
const FILE_PATH: &str = "/files-pri/T0FIXTURE01-F0FIXTURE01/fixture-1.pdf";

/// The `Retry-After` the fixture names on a `ratelimited` answer: beyond the
/// invocation deadline, so the provider does not send the read again.
const RATE_LIMIT_DELAY: u64 = 120;
/// A private channel the user is a member of and the bot is not.
const PRIVATE: &str = "C0FIXTURE09";
/// The parent `ts` of the one thread in [`PRIVATE`].
const PRIVATE_THREAD: &str = "1780000150.000150";
fn private_message(ts: &str, text: &str) -> Value {
    json!({"type": "message", "user": "U0FIXTURE02", "text": text, "ts": ts,
           "thread_ts": PRIVATE_THREAD})
}

/// What the fixture answers the user token: its own identity, search, and
/// the conversation and user reads, where the user also reads [`PRIVATE`].
/// `None` for anything else, which the fixture answers 404.
fn user_page(path: &str) -> Option<Value> {
    let (route, query) = path.split_once('?').unwrap_or((path, ""));
    let has = |pair: &str| query.split('&').any(|p| p == pair);
    let private = has(&format!("channel={PRIVATE}"));
    Some(match route {
        "/api/conversations.list" if !has("cursor=fixture-list-2") => json!({
            "ok": true,
            "channels": [channel(1),
                         json!({"id": PRIVATE, "name": "fixture-private", "is_channel": false,
                                "is_group": true, "is_private": true, "is_archived": false,
                                "created": 1_780_000_009u64})],
            "response_metadata": more("fixture-list-2")}),
        "/api/conversations.history" if private => json!({
            "ok": true,
            "messages": [private_message(PRIVATE_THREAD, "fixture private parent")],
            "has_more": false, "pin_count": 0, "response_metadata": more("")}),
        "/api/conversations.replies" if private && has(&format!("ts={PRIVATE_THREAD}")) => json!({
            "ok": true,
            "messages": [private_message(PRIVATE_THREAD, "fixture private parent"),
                         private_message("1780000160.000160", "fixture private reply")],
            "has_more": false, "response_metadata": more("")}),
        "/api/conversations.list"
        | "/api/conversations.history"
        | "/api/conversations.replies"
        | "/api/users.list" => return page(path),
        "/api/auth.test" => json!({
            "ok": true, "url": "https://fixture.example.test/", "team": "Fixture Team",
            "user": "fixture-user-2", "team_id": "T0FIXTURE01", "user_id": "U0FIXTURE02",
            "is_enterprise_install": false}),
        "/api/search.messages" if has("query=fixture") => json!({
            "ok": true, "query": "fixture",
            "messages": {
                "total": 1,
                "matches": [{
                    "type": "message", "user": "U0FIXTURE02", "text": "fixture message one",
                    "ts": "1780000300.000300",
                    "channel": {"id": "C0FIXTURE01", "name": "fixture-channel-1"},
                    "permalink": "https://fixture.example.test/archives/C0FIXTURE01/p1780000300000300"}],
                "paging": {"count": 20, "page": 1, "pages": 1, "total": 1}}}),
        "/api/search.messages" => json!({
            "ok": true, "query": "",
            "messages": {"total": 0, "matches": [],
                         "paging": {"count": 20, "page": 1, "pages": 0, "total": 0}}}),
        _ => return None,
    })
}

struct Provider {
    stop: Option<oneshot::Sender<()>>,
    thread: Option<std::thread::JoinHandle<()>>,
    _root: tempfile::TempDir,
    config: PathBuf,
    requests: Requests,
    /// Method, route, `Content-Type` and body of each fixture request.
    exchanges: Exchanges,
    /// The private protocol the child speaks; two carries writes.
    protocol: Option<PrivateProtocol>,
    /// The fixture's own origin, `https://localhost:<port>`.
    origin: String,
}
impl Provider {
    fn new() -> Self {
        Self::serving(false)
    }
    /// The search connection: the guide's `slack.user` configuration and the
    /// user selection set.
    fn user() -> Self {
        Self::start(false, true, None, None)
    }
    /// `refuse_identity`: answer `auth.test` as Slack answers a revoked
    /// token, `200` with `ok: false` and no `user_id`.
    fn serving(refuse_identity: bool) -> Self {
        Self::start(refuse_identity, false, None, None)
    }
    /// The bot connection on private protocol two, which carries the two
    /// writes; `refuse_identity` as for [`Provider::serving`].
    fn writing(refuse_identity: bool) -> Self {
        Self::start(refuse_identity, false, None, Some(PrivateProtocol::V2))
    }
    /// The bot connection admitting the fixture's own origin as a file host,
    /// with or without the credential, beside the shipped set: one more
    /// selection, `fixture.download`, shaped as the shipped `file.download`
    /// but reaching that origin, since the fixture cannot serve
    /// `files.slack.com`.
    fn files(credential: bool) -> Self {
        Self::start(false, false, Some(credential), None)
    }
    fn start(
        refuse_identity: bool,
        user: bool,
        files: Option<bool>,
        protocol: Option<PrivateProtocol>,
    ) -> Self {
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
        let exchanges: Exchanges = Arc::new(Mutex::new(Vec::new()));
        let exchanged = exchanges.clone();
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
                    let token_header = request.lines().any(|line| {
                        line.split_once(':')
                            .is_some_and(|(name, _)| name.eq_ignore_ascii_case("token"))
                    });
                    let header_value = |wanted: &str| {
                        request.lines().find_map(|line| {
                            line.split_once(':')
                                .filter(|(name, _)| name.eq_ignore_ascii_case(wanted))
                                .map(|(_, value)| value.trim().to_owned())
                        })
                    };
                    let method = request.split_whitespace().next().unwrap_or_default().to_owned();
                    let length: usize = header_value("content-length")
                        .map(|value| value.parse().unwrap())
                        .unwrap_or(0);
                    assert!(length <= 65_536);
                    let mut sent = vec![0; length];
                    if stream.read_exact(&mut sent).await.is_err() {
                        continue;
                    }
                    let sent: Value = if sent.is_empty() {
                        Value::Null
                    } else {
                        serde_json::from_slice(&sent).unwrap_or(Value::Null)
                    };
                    let route = path.split('?').next().unwrap_or_default().to_owned();
                    exchanged.lock().unwrap().push((
                        method.clone(),
                        route.clone(),
                        header_value("content-type"),
                        sent.clone(),
                    ));
                    // The two writes, to the bot token alone. A post to
                    // `C0FIXTUREDROP` is read and then left unanswered, the
                    // connection closed: the dispatch is ambiguous.
                    if method == "POST" && authorization.as_deref() == Some(HEADER) {
                        let answer = match route.as_str() {
                            "/api/chat.postMessage" if sent["channel"] == "C0FIXTUREDROP" => {
                                observed.lock().unwrap().push((path, authorization, token_header));
                                continue;
                            }
                            "/api/chat.postMessage" if sent["channel"] == "C0FIXTURE500" => {
                                Some((500, json!({"ok": false, "error": "fatal_error"})))
                            }
                            "/api/chat.postMessage" => Some((200, posted(&sent))),
                            "/api/conversations.open" => Some((200, opened(&sent))),
                            "/api/chat.update" | "/api/chat.delete"
                                if sent["channel"] == "C0FIXTUREDROP" =>
                            {
                                observed.lock().unwrap().push((path, authorization, token_header));
                                continue;
                            }
                            "/api/chat.update" => Some((200, edited(&sent))),
                            "/api/chat.delete" => Some((200, deleted(&sent))),
                            _ => None,
                        };
                        if let Some((status, answer)) = answer {
                            observed.lock().unwrap().push((path, authorization, token_header));
                            let answer = serde_json::to_vec(&answer).unwrap();
                            let header = format!(
                                "HTTP/1.1 {status} fixture\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                                answer.len()
                            );
                            let _ = stream.write_all(header.as_bytes()).await;
                            let _ = stream.write_all(&answer).await;
                            continue;
                        }
                    }
                    // The file host: the content to the bot token alone, a
                    // `403` without it, as bytes that are not JSON.
                    if path.starts_with("/files-pri/") {
                        let bot = authorization.as_deref() == Some(HEADER);
                        let (status, kind, bytes) = if !bot {
                            (403, "text/plain", b"fixture forbidden".to_vec())
                        } else if path.trim_end_matches('?') == FILE_PATH {
                            (200, "application/pdf", FILE_BYTES.to_vec())
                        } else {
                            (404, "text/plain", b"no fixture".to_vec())
                        };
                        observed
                            .lock()
                            .unwrap()
                            .push((path, authorization, token_header));
                        let header = format!(
                            "HTTP/1.1 {status} fixture\r\nContent-Type: {kind}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                            bytes.len()
                        );
                        let _ = stream.write_all(header.as_bytes()).await;
                        let _ = stream.write_all(&bytes).await;
                        continue;
                    }
                    // Slack answers a bad token `200` with `ok: false`; only
                    // the two fictional headers are accepted, each answered
                    // as its token, and raw headers are compared, never
                    // printed.
                    let body = if refuse_identity && path.starts_with("/api/auth.test") {
                        Some(json!({"ok": false, "error": "invalid_auth"}))
                    } else if authorization.as_deref() == Some(HEADER) {
                        page(&path)
                    } else if authorization.as_deref() == Some(USER_HEADER) {
                        user_page(&path)
                    } else {
                        Some(json!({"ok": false, "error": "invalid_auth"}))
                    };
                    let (status, body) = match body {
                        Some(body) => (200, body),
                        None => (404, json!({"ok": false, "error": "no fixture"})),
                    };
                    // Slack's `ratelimited` names its delay in `Retry-After`:
                    // one that fits the deadline for the `soon` cursor, one
                    // that does not otherwise.
                    let retry = if body["error"] != "ratelimited" {
                        String::new()
                    } else if path.contains("cursor=fixture-ratelimited-soon") {
                        "Retry-After: 1\r\n".to_owned()
                    } else {
                        format!("Retry-After: {RATE_LIMIT_DELAY}\r\n")
                    };
                    observed
                        .lock()
                        .unwrap()
                        .push((path, authorization, token_header));
                    let body = serde_json::to_vec(&body).unwrap();
                    let header = format!(
                        "HTTP/1.1 {status} fixture\r\nContent-Type: application/json\r\nContent-Length: {}\r\n{retry}Connection: close\r\n\r\n",
                        body.len()
                    );
                    let _ = stream.write_all(header.as_bytes()).await;
                    let _ = stream.write_all(&body).await;
                }
            });
        });
        let address = address_rx.recv_timeout(Duration::from_secs(5)).unwrap();
        let config = directory.join("catalog.json");
        let (profile, operations) = if user {
            (USER_PROFILE, "providers/slack/user-operations.json")
        } else {
            (PROFILE, "providers/slack/operations.json")
        };
        let mut document = documented_config_for(profile);
        document["instance"] = json!("fixture-slack");
        document["bundle_directory"] = json!(root_path("generated/bundles"));
        document["api_base"] = json!(format!("https://localhost:{}/api", address.port()));
        document["ca_file"] = json!(ca);
        document["operations_file"] = json!(root_path(operations));
        let origin = format!("https://localhost:{}", address.port());
        if let Some(credential) = files {
            let mut download = shipped()
                .into_iter()
                .find(|s| s.id == "file.download")
                .expect("the shipped file download");
            download.id = "fixture.download".into();
            download.binary.as_mut().unwrap().hosts = vec![origin.clone()];
            document["operations"] = json!([download]);
            document["hosts"] = json!([{"origin": origin, "credential": credential}]);
        }
        private(&config, &serde_json::to_vec(&document).unwrap());
        Self {
            stop: Some(stop),
            thread: Some(thread),
            _root: root,
            config,
            requests,
            exchanges,
            protocol,
            origin,
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
    fn requests(&self) -> Vec<(String, Option<String>, bool)> {
        self.requests.lock().unwrap().clone()
    }
    fn exchanges(&self) -> Vec<(String, String, Option<String>, Value)> {
        self.exchanges.lock().unwrap().clone()
    }
    /// Only the POSTs the fixture saw, as route and body.
    fn posts(&self) -> Vec<(String, Value)> {
        self.exchanges()
            .into_iter()
            .filter(|(method, ..)| method == "POST")
            .map(|(_, route, _, body)| (route, body))
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
    Secret(serde_json::to_vec(&json!({"token": TOKEN})).unwrap())
}
fn deadline() -> u64 {
    connectors_sdk::now_ms() + 30_000
}
fn user_secret() -> Secret {
    Secret(serde_json::to_vec(&json!({"token": USER_TOKEN})).unwrap())
}
fn attempt(child: &mut Child, operation: &str, input: &Value) -> Result<Vec<u8>, Failure> {
    attempt_as(child, operation, input, &secret())
}
fn attempt_as(
    child: &mut Child,
    operation: &str,
    input: &Value,
    secret: &Secret,
) -> Result<Vec<u8>, Failure> {
    let revision = child.bootstrap().descriptor().unwrap().revision;
    child.invoke(
        operation,
        &revision,
        "one",
        secret,
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
fn first_pages() -> [(&'static str, Value, &'static str); 4] {
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
        (
            "users.list",
            json!({"include_locale": true, "limit": 2}),
            "/api/users.list?limit=2&include_locale=true",
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
        ("users.list", json!({"limit": 2, "token": TOKEN})),
        ("users.list", json!({"presence": true})),
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
        // A credential parameter, in every spelling, on each method that
        // names `token` as its credential.
        ("team.info", json!({"token": TOKEN})),
        ("team.info", json!({"query:token": TOKEN})),
        ("emoji.list", json!({"token": TOKEN})),
        ("emoji.list", json!({"header:token": TOKEN})),
        ("auth.test", json!({"token": TOKEN})),
        ("auth.test", json!({"header:token": TOKEN})),
        ("auth.test", json!({"query:token": TOKEN})),
    ] {
        let outcome = attempt(&mut child, operation, &input);
        assert!(
            matches!(outcome, Err(Failure::InvalidInput)),
            "`{operation}` {input}: {outcome:?}"
        );
    }
    assert!(provider.requests().is_empty());
}

/// One answered request: its exact route, the bearer header it carried, no
/// `token` header, and the served body bytes returned unchanged as `body`.
fn assert_answered(
    provider: &Provider,
    before: usize,
    raw: &[u8],
    operation: &str,
    expected: &str,
    header: &str,
    served: Value,
) {
    let requests = provider.requests();
    assert_eq!(requests.len(), before + 1, "`{operation}` requests");
    assert_eq!(requests[before].0, expected, "`{operation}` request");
    assert_eq!(
        requests[before].1.as_deref(),
        Some(header),
        "`{operation}` authorization"
    );
    assert!(!requests[before].2, "`{operation}` sent a token header");
    let result: Value = serde_json::from_slice(raw).unwrap();
    assert_eq!(result["status"], 200, "`{operation}` status");
    assert_eq!(result["provenance"]["instance"], "fixture-slack");
    let served = serde_json::to_vec(&served).unwrap();
    assert_eq!(
        serde_json::to_vec(&result["body"]).unwrap(),
        served,
        "`{operation}` body"
    );
}

/// `team.info`, `emoji.list` and `auth.test` on the bot connection: the token
/// travels in the `Authorization` header only — no `token` query parameter,
/// no `token` header — and Slack's answer comes back as served. An empty
/// input sends an empty query.
#[test]
fn each_discovery_read_sends_the_bearer_header_and_no_token() {
    let provider = Provider::new();
    let mut child = Child::spawn(&provider.selection()).unwrap();
    for (operation, input, expected) in [
        ("team.info", json!({}), "/api/team.info?"),
        (
            "team.info",
            json!({"team": "T0FIXTURE01"}),
            "/api/team.info?team=T0FIXTURE01",
        ),
        ("emoji.list", json!({}), "/api/emoji.list?"),
        ("auth.test", json!({}), "/api/auth.test?"),
    ] {
        let before = provider.requests().len();
        let raw = invoke_raw(&mut child, operation, &input);
        assert_answered(
            &provider,
            before,
            &raw,
            operation,
            expected,
            HEADER,
            page(expected).unwrap(),
        );
    }
    // The bot connection does not expose search.
    let before = provider.requests().len();
    assert!(attempt(&mut child, "search.messages", &json!({"query": "fixture"})).is_err());
    assert_eq!(provider.requests().len(), before);
}

/// The search connection: a user token under `slack.user`, proved by the same
/// `auth.test` identity read, whose `user_id` is the person's. Search sends
/// its query in the document's parameter order with the user token in the
/// header only, and the conversation reads are not exposed on it.
#[test]
fn search_runs_on_the_user_token_connection() {
    let provider = Provider::user();
    let mut child = Child::spawn(&provider.selection()).unwrap();
    let baseline = child
        .validate(USER_PROFILE, &user_secret(), deadline())
        .unwrap();
    assert_eq!(baseline.identity.kind, "slack.user");
    assert_eq!(baseline.identity.subject, "U0FIXTURE02");
    let requests = provider.requests();
    assert_eq!(requests.len(), 1);
    assert_eq!(requests[0].0, "/api/auth.test?");
    assert_eq!(requests[0].1.as_deref(), Some(USER_HEADER));
    assert!(matches!(
        child.validate(PROFILE, &user_secret(), deadline()),
        Err(Failure::Unsupported)
    ));

    let expected =
        "/api/search.messages?count=20&page=1&query=fixture&sort=timestamp&sort_dir=desc";
    let before = provider.requests().len();
    let raw = attempt_as(
        &mut child,
        "search.messages",
        &json!({"sort_dir": "desc", "sort": "timestamp", "query": "fixture",
                "page": 1, "count": 20}),
        &user_secret(),
    )
    .unwrap_or_else(|failure| panic!("search failed: {failure:?}"));
    assert_answered(
        &provider,
        before,
        &raw,
        "search.messages",
        expected,
        USER_HEADER,
        user_page(expected).unwrap(),
    );

    // The bot set's other reads are not exposed on it.
    let before = provider.requests().len();
    for operation in ["team.info", "files.list"] {
        assert!(
            attempt_as(&mut child, operation, &json!({}), &user_secret()).is_err(),
            "`{operation}` on the user connection"
        );
    }
    assert_eq!(provider.requests().len(), before);
}

/// The user-token connection: connected under `slack.user`, each of the
/// conversation reads and `users.list` sends its request with the user token
/// in the header only and returns Slack's answer, including a private channel
/// the bot is not in, whose history the bot connection is answered
/// `not_in_channel`. No write is exposed on it, and nothing is sent for one.
#[test]
fn history_and_replies_read_on_the_user_token_connection() {
    let provider = Provider::user();
    let mut child = Child::spawn(&provider.selection()).unwrap();
    let baseline = child
        .validate(USER_PROFILE, &user_secret(), deadline())
        .unwrap();
    assert_eq!(baseline.identity.subject, "U0FIXTURE02");
    let history = format!("/api/conversations.history?channel={PRIVATE}&limit=20");
    let replies =
        format!("/api/conversations.replies?channel={PRIVATE}&ts={PRIVATE_THREAD}&limit=20");
    for (operation, input, expected) in [
        (
            "conversations.list",
            json!({"types": "public_channel,private_channel", "limit": 2}),
            "/api/conversations.list?types=public_channel%2Cprivate_channel&limit=2",
        ),
        (
            "conversations.history",
            json!({"channel": PRIVATE, "limit": 20}),
            history.as_str(),
        ),
        (
            "conversations.replies",
            json!({"channel": PRIVATE, "ts": PRIVATE_THREAD, "limit": 20}),
            replies.as_str(),
        ),
        ("users.list", json!({"limit": 2}), "/api/users.list?limit=2"),
    ] {
        let before = provider.requests().len();
        let raw = attempt_as(&mut child, operation, &input, &user_secret())
            .unwrap_or_else(|failure| panic!("`{operation}` failed: {failure:?}"));
        assert_answered(
            &provider,
            before,
            &raw,
            operation,
            expected,
            USER_HEADER,
            user_page(expected).unwrap(),
        );
    }
    // No write: each is refused with nothing sent.
    let before = provider.requests().len();
    for operation in [
        "chat.postMessage",
        "chat.update",
        "chat.delete",
        "conversations.open",
    ] {
        assert!(
            attempt_as(&mut child, operation, &json!({}), &user_secret()).is_err(),
            "`{operation}` on the user connection"
        );
    }
    assert_eq!(provider.requests().len(), before);
    // The bot is not in the private channel.
    assert_eq!(
        page(&history).unwrap(),
        json!({"ok": false, "error": "not_in_channel"})
    );
}

/// The token in any spelling, a missing `query`, a `count` outside the
/// pinned 1 to 100, and a cursor search does not take: each refused before
/// any request.
#[test]
fn search_refuses_a_token_a_missing_query_or_an_out_of_bound_count_before_any_request() {
    let provider = Provider::user();
    let mut child = Child::spawn(&provider.selection()).unwrap();
    for input in [
        json!({"query": "fixture", "token": USER_TOKEN}),
        json!({"query": "fixture", "query:token": USER_TOKEN}),
        json!({"query": "fixture", "header:token": USER_TOKEN}),
        json!({"count": 20}),
        json!({"query": "fixture", "count": 0}),
        json!({"query": "fixture", "count": 101}),
        json!({"query": "fixture", "cursor": "c"}),
    ] {
        let outcome = attempt_as(&mut child, "search.messages", &input, &user_secret());
        assert!(
            matches!(outcome, Err(Failure::InvalidInput)),
            "{input}: {outcome:?}"
        );
    }
    assert!(provider.requests().is_empty());
}

/// Slack reports a failed read as `200` with `ok: false`. The provider refuses
/// it: Slack's `error` is the reason and its classification the code, so a
/// failed read never looks like an empty one.
#[test]
fn a_slack_error_answer_is_a_refusal_naming_slacks_error() {
    let provider = Provider::new();
    let mut child = Child::spawn(&provider.selection()).unwrap();
    let before = provider.requests().len();
    let refusal = explained(
        &mut child,
        "conversations.history",
        &json!({"channel": "C0FIXTURE99"}),
    )
    .expect_err("an ok: false answer");
    assert_eq!(provider.requests().len(), before + 1);
    assert_eq!(refusal.failure, Failure::ProviderNotFound);
    assert_eq!(refusal.reason.as_deref(), Some("channel_not_found"));
    // The bot is not in the private channel the user token reads.
    let refusal = explained(
        &mut child,
        "conversations.history",
        &json!({"channel": PRIVATE}),
    )
    .expect_err("not_in_channel");
    assert_eq!(refusal.failure, Failure::ProviderForbidden);
    assert_eq!(refusal.reason.as_deref(), Some("not_in_channel"));
}

/// How the Slack sets classify Slack's `200` with `ok: false`: each named
/// `error` and the code it is refused with. `otherwise` is every other name.
const MAPPED: [(&str, &str); 22] = [
    ("missing_scope", "forbidden"),
    ("not_in_channel", "forbidden"),
    ("no_permission", "forbidden"),
    ("user_is_bot", "forbidden"),
    ("not_authed", "unauthorized"),
    ("invalid_auth", "unauthorized"),
    ("account_inactive", "unauthorized"),
    ("token_revoked", "unauthorized"),
    ("channel_not_found", "not_found"),
    ("thread_not_found", "not_found"),
    ("file_not_found", "not_found"),
    ("file_deleted", "not_found"),
    ("user_not_found", "not_found"),
    ("ratelimited", "rate_limited"),
    ("invalid_cursor", "invalid_input"),
    ("invalid_limit", "invalid_input"),
    ("limit_required", "invalid_input"),
    ("invalid_ts_latest", "invalid_input"),
    ("invalid_ts_oldest", "invalid_input"),
    ("invalid_arg_name", "invalid_input"),
    ("invalid_array_arg", "invalid_input"),
    ("unknown_type", "invalid_input"),
];
/// The names the pinned document lists for a selected read that are left to
/// `otherwise` (`internal`): a request the engine formed wrongly, or a
/// failure on Slack's side.
const INTERNAL: [&str; 11] = [
    "invalid_charset",
    "invalid_form_data",
    "invalid_post_type",
    "missing_post_type",
    "invalid_json",
    "json_not_object",
    "request_timeout",
    "upgrade_required",
    "fatal_error",
    "team_added_to_org",
    "timezone_count_failed",
];

/// One read through the child, refused with its reason and delay.
fn explained(child: &mut Child, operation: &str, input: &Value) -> Result<Vec<u8>, Refusal> {
    explained_as(child, operation, input, &secret())
}
fn explained_as(
    child: &mut Child,
    operation: &str,
    input: &Value,
    secret: &Secret,
) -> Result<Vec<u8>, Refusal> {
    let revision = child.bootstrap().descriptor().unwrap().revision;
    child.invoke_explained(
        operation,
        &revision,
        "one",
        secret,
        &serde_json::to_vec(input).unwrap(),
        deadline(),
    )
}

/// Every error name the pinned document lists for a selected JSON read, in
/// its `default` answer's `error` enum, is classified: named by the envelope
/// or one of the [`INTERNAL`] names left to `otherwise`. A source refresh
/// that adds a name fails here until it is classified.
#[test]
fn every_documented_error_of_a_slack_read_is_classified() {
    let pinned = pinned();
    let mut documented = 0;
    for selection in shipped().into_iter().chain(user_shipped()) {
        if selection.effect == Effect::Write || selection.id == "file.download" {
            continue;
        }
        let path = format!("/{}", selection.id);
        let names =
            pinned["paths"][&path]["get"]["responses"]["default"]["schema"]["properties"]["error"]
                ["enum"]
                .as_array()
                .cloned()
                .unwrap_or_default();
        for name in names {
            let name = name.as_str().unwrap();
            assert!(
                MAPPED.iter().any(|(mapped, _)| *mapped == name) || INTERNAL.contains(&name),
                "`{}` documents `{name}`, which no class names",
                selection.id
            );
            documented += 1;
        }
    }
    assert!(
        documented > 100,
        "the pinned enums were not read: {documented}"
    );
}

/// Every Slack read that answers JSON, in both sets, declares the `ok`
/// envelope: `ok` at `/ok`, Slack's error name at `/error`, the same
/// classification on each, and `internal` for any other name. The download,
/// which answers bytes, and the writes, whose guard reads `ok`, declare none.
#[test]
fn every_slack_read_declares_the_ok_envelope_and_no_write_does() {
    let all: Vec<(&str, Selection)> = shipped()
        .into_iter()
        .map(|s| ("bot", s))
        .chain(user_shipped().into_iter().map(|s| ("user", s)))
        .collect();
    let mut enveloped = 0;
    for (set, selection) in &all {
        let raw = serde_json::to_value(selection).unwrap();
        let envelope = raw.get("envelope");
        if selection.effect == Effect::Write || selection.id == "file.download" {
            assert!(envelope.is_none(), "{set} `{}`", selection.id);
            continue;
        }
        let envelope =
            envelope.unwrap_or_else(|| panic!("{set} `{}` declares no envelope", selection.id));
        enveloped += 1;
        assert_eq!(envelope["ok"], "/ok", "{set} `{}`", selection.id);
        assert_eq!(envelope["error"], "/error", "{set} `{}`", selection.id);
        assert_eq!(
            envelope["otherwise"], "internal",
            "{set} `{}`",
            selection.id
        );
        let refusals = envelope["refusals"].as_array().unwrap();
        let named: usize = refusals
            .iter()
            .map(|r| r["values"].as_array().unwrap().len())
            .sum();
        assert_eq!(named, MAPPED.len(), "{set} `{}` names more", selection.id);
        for (name, code) in MAPPED {
            let found: Vec<&Value> = refusals
                .iter()
                .filter(|r| r["values"].as_array().unwrap().iter().any(|v| v == name))
                .collect();
            assert_eq!(found.len(), 1, "{set} `{}` `{name}`", selection.id);
            assert_eq!(found[0]["code"], code, "{set} `{}` `{name}`", selection.id);
        }
    }
    assert_eq!(enveloped, 9 + 5, "the enveloped reads");
}

/// Each mapped Slack error, served `200` with `ok: false`, is a refusal
/// through the provider child: its code, Slack's `error` as the reason, and
/// for `ratelimited` the `Retry-After` delay, sent once when the delay does
/// not fit the deadline and twice when it does, as a `429` is. Any other name
/// is `internal`, and an answer whose `ok` is absent or not the boolean, or
/// that is not an object, is unreadable. Never a success.
#[test]
fn each_slack_error_answered_ok_false_is_a_refusal_naming_it() {
    let provider = Provider::new();
    let mut child = Child::spawn(&provider.selection()).unwrap();
    let expect = |code: &str| match code {
        "forbidden" => Failure::ProviderForbidden,
        "unauthorized" => Failure::InvalidCredential,
        "not_found" => Failure::ProviderNotFound,
        "rate_limited" => Failure::ProviderRateLimited,
        "invalid_input" => Failure::InvalidInput,
        "internal" => Failure::ProviderInternal,
        other => panic!("unmapped `{other}`"),
    };
    for (name, code) in MAPPED
        .into_iter()
        .chain(INTERNAL.into_iter().map(|name| (name, "internal")))
    {
        let before = provider.requests().len();
        let refusal = explained(
            &mut child,
            "users.list",
            &json!({"limit": 2, "cursor": format!("fixture-error-{name}")}),
        )
        .expect_err(name);
        assert_eq!(provider.requests().len(), before + 1, "`{name}` sent once");
        assert_eq!(refusal.failure, expect(code), "`{name}`");
        assert_eq!(refusal.reason.as_deref(), Some(name), "`{name}`");
        let delay = (name == "ratelimited").then_some(RATE_LIMIT_DELAY);
        assert_eq!(refusal.retry_after_seconds, delay, "`{name}`");
    }
    // A delay that fits the deadline: sent once more, as a `429` is, and
    // refused again with the delay.
    let before = provider.requests().len();
    let refusal = explained(
        &mut child,
        "users.list",
        &json!({"limit": 2, "cursor": "fixture-ratelimited-soon"}),
    )
    .expect_err("ratelimited soon");
    assert_eq!(provider.requests().len(), before + 2);
    assert_eq!(refusal.failure, Failure::ProviderRateLimited);
    assert_eq!(refusal.retry_after_seconds, Some(1));
    for cursor in ["fixture-no-ok", "fixture-ok-string", "fixture-not-object"] {
        let refusal = explained(
            &mut child,
            "users.list",
            &json!({"limit": 2, "cursor": cursor}),
        )
        .expect_err(cursor);
        assert_eq!(refusal.failure, Failure::Protocol, "`{cursor}`");
    }
    // `ok: true` is still the answer as served.
    let raw = invoke_raw(&mut child, "users.list", &json!({"limit": 2}));
    let result: Value = serde_json::from_slice(&raw).unwrap();
    assert_eq!(result["status"], 200);
    assert_eq!(result["body"]["ok"], true);
}

/// No Slack read answers success on `ok: false`: a token Slack does not
/// accept is answered `200` with `ok: false` and `invalid_auth` on every
/// route, and every JSON read of both sets is refused as an invalid
/// credential naming it.
#[test]
fn no_slack_read_answers_success_on_ok_false() {
    let revoked =
        Secret(serde_json::to_vec(&json!({"token": "fixture-slack-revoked-token"})).unwrap());
    let input = |id: &str| match id {
        "conversations.history" => json!({"channel": "C0FIXTURE01"}),
        "conversations.replies" => json!({"channel": "C0FIXTURE01", "ts": "1780000100.000100"}),
        "files.info" => json!({"file": "F0FIXTURE01"}),
        "search.messages" => json!({"query": "fixture"}),
        _ => json!({}),
    };
    let mut read = 0;
    for (provider, selections) in [
        (Provider::new(), shipped()),
        (Provider::user(), user_shipped()),
    ] {
        let mut child = Child::spawn(&provider.selection()).unwrap();
        for selection in selections
            .iter()
            .filter(|s| s.effect == Effect::Read && s.id != "file.download")
        {
            let id = selection.id.as_str();
            let before = provider.requests().len();
            let refusal = explained_as(&mut child, id, &input(id), &revoked).expect_err(id);
            assert_eq!(provider.requests().len(), before + 1, "`{id}` sent once");
            assert_eq!(refusal.failure, Failure::InvalidCredential, "`{id}`");
            assert_eq!(refusal.reason.as_deref(), Some("invalid_auth"), "`{id}`");
            read += 1;
        }
    }
    assert_eq!(read, 9 + 5);
}

/// The envelope is a read's: on a write, a download, a text read or a binary
/// read it is refused when the selection loads, as are a pointer without a
/// leading `/`, an empty value, a value two refusals name, a refusal of no
/// value and more than sixteen refusals.
#[test]
fn an_envelope_outside_a_json_read_or_malformed_is_refused_when_the_selection_loads() {
    let envelope = || {
        json!({"ok": "/ok", "error": "/error",
               "refusals": [{"values": ["missing_scope"], "code": "forbidden"}],
               "otherwise": "internal"})
    };
    let with = |id: &str, change: &dyn Fn(&mut Value)| -> Vec<Selection> {
        let mut raw: Vec<Value> = serde_json::from_value(
            serde_json::from_slice::<Value>(
                &fs::read(root().join("providers/slack/operations.json")).unwrap(),
            )
            .unwrap()["operations"]
                .clone(),
        )
        .unwrap();
        for operation in raw.iter_mut().filter(|o| o["id"] == id) {
            change(operation);
        }
        serde_json::from_value(Value::Array(raw)).unwrap()
    };
    // The shipped sets load.
    Engine::new(&committed(), BASE, &shipped()).unwrap();
    Engine::new(&committed(), BASE, &user_shipped()).unwrap();
    for id in ["chat.postMessage", "file.download"] {
        let message = refusal(&with(id, &|o| o["envelope"] = envelope()));
        assert!(message.contains("envelope"), "`{id}`: {message}");
    }
    let text = refusal(&with("users.list", &|o| o["response"] = json!("text")));
    assert!(text.contains("envelope"), "text: {text}");
    let mut bad: Vec<(&str, Value)> = Vec::new();
    let mut e = envelope();
    e["ok"] = json!("ok");
    bad.push(("pointer", e));
    let mut e = envelope();
    e["error"] = json!("");
    bad.push(("empty error pointer", e));
    let mut e = envelope();
    e["refusals"][0]["values"] = json!([""]);
    bad.push(("empty value", e));
    let mut e = envelope();
    e["refusals"] = json!([{"values": ["a"], "code": "forbidden"},
                           {"values": ["a"], "code": "not_found"}]);
    bad.push(("repeated value", e));
    let mut e = envelope();
    e["refusals"][0]["values"] = json!([]);
    bad.push(("no values", e));
    let mut e = envelope();
    e["refusals"] = Value::Array(
        (0..17)
            .map(|n| json!({"values": [format!("v{n}")], "code": "forbidden"}))
            .collect(),
    );
    bad.push(("seventeen", e));
    for (what, envelope) in bad {
        let message = refusal(&with("users.list", &|o| o["envelope"] = envelope.clone()));
        assert!(message.contains("envelope"), "{what}: {message}");
    }
    // An unknown code or member is not a selection at all.
    for change in [
        &(|e: &mut Value| e["otherwise"] = json!("capacity")) as &dyn Fn(&mut Value),
        &|e: &mut Value| e["extra"] = json!(true),
    ] {
        let mut e = envelope();
        change(&mut e);
        assert!(
            serde_json::from_value::<Selection>(json!({
                "id": "users.list", "operation_id": "users_list", "effect": "read",
                "envelope": e
            }))
            .is_err()
        );
    }
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
        .map(|(path, _, _)| path)
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

#[test]
fn users_list_walks_two_pages_and_stops_on_an_empty_next_cursor() {
    let (read, requests) = walk("users.list", json!({"limit": 2}), "members");
    assert_eq!(read, ["U0FIXTURE11", "U0FIXTURE12", "U0FIXTURE13"]);
    assert_eq!(
        requests,
        [
            "/api/users.list?limit=2",
            "/api/users.list?limit=2&cursor=fixture-users-2",
        ]
    );
}

/// The file reads: `files.list` and `files.info` select the pinned
/// `files_list` and `files_info` (`objs_file` carries the record's name,
/// filetype, mimetype, size and url_private), withhold `token`, and
/// `files.info` requires `file`. The download names no bundle operation: it is
/// a binary read of `url_private` on `https://files.slack.com` alone, under
/// `/files-pri/`, bounded at 2 MiB.
#[test]
fn the_file_reads_and_the_download_are_shipped_as_reviewed() {
    let document = pinned();
    let file = &document["definitions"]["objs_file"]["properties"];
    for (member, kind) in [
        ("name", "string"),
        ("filetype", "string"),
        ("mimetype", "string"),
        ("size", "integer"),
        ("url_private", "string"),
    ] {
        assert_eq!(file[member]["type"], kind, "objs_file `{member}`");
    }
    let selections = shipped();
    let info = selections.iter().find(|s| s.id == "files.info").unwrap();
    assert_eq!(info.required, ["file"]);
    let download = selections.iter().find(|s| s.id == "file.download").unwrap();
    assert!(download.operation_id.is_empty());
    assert_eq!(download.effect, Effect::Read);
    assert_eq!(
        download.response,
        Some(connectors_catalog_provider::ResponseKind::Binary)
    );
    let binary = download.binary.as_ref().unwrap();
    assert_eq!(binary.hosts, ["https://files.slack.com"]);
    assert_eq!(binary.max_bytes, connectors_catalog_provider::BINARY_LIMIT);
    assert_eq!(
        download.download.as_ref().unwrap().path_prefix,
        "/files-pri/"
    );
    let engine = Engine::new(&committed(), BASE, &selections).unwrap();
    assert_eq!(
        engine.reached_hosts().into_iter().collect::<Vec<_>>(),
        ["https://files.slack.com"]
    );
    // The user token's connection reaches no other host.
    let engine = Engine::new(&committed(), BASE, &user_shipped()).unwrap();
    assert!(engine.reached_hosts().is_empty());
}

#[test]
fn files_list_walks_two_pages_by_page_and_files_info_reads_one_file() {
    let provider = Provider::new();
    let mut child = Child::spawn(&provider.selection()).unwrap();
    let first = invoke(&mut child, "files.list", &json!({"count": "2"}));
    let second = invoke(
        &mut child,
        "files.list",
        &json!({"count": "2", "page": "2"}),
    );
    let ids: Vec<&str> = [&first, &second]
        .iter()
        .flat_map(|page| page["body"]["files"].as_array().unwrap())
        .map(|file| file["id"].as_str().unwrap())
        .collect();
    assert_eq!(ids, ["F0FIXTURE01", "F0FIXTURE02", "F0FIXTURE03"]);
    assert_eq!(
        second["body"]["paging"]["page"],
        second["body"]["paging"]["pages"]
    );
    let info = invoke(&mut child, "files.info", &json!({"file": "F0FIXTURE01"}));
    assert_eq!(info["body"]["file"], file(1));
    assert_eq!(
        info["body"]["file"]["url_private"],
        "https://files.slack.com/files-pri/T0FIXTURE01-F0FIXTURE01/fixture-1.pdf"
    );
    let requests = provider.requests();
    let routes: Vec<&str> = requests.iter().map(|(path, _, _)| path.as_str()).collect();
    assert_eq!(
        routes,
        [
            "/api/files.list?count=2",
            "/api/files.list?count=2&page=2",
            "/api/files.info?file=F0FIXTURE01",
        ]
    );
    for (_, authorization, token) in &requests {
        assert_eq!(authorization.as_deref(), Some(HEADER));
        assert!(!token);
    }
    // `file` is required, and `token` is never input.
    for input in [json!({}), json!({"file": "F0FIXTURE01", "token": TOKEN})] {
        assert!(matches!(
            attempt(&mut child, "files.info", &input),
            Err(Failure::InvalidInput)
        ));
    }
    assert_eq!(provider.requests().len(), 3);
}

/// The download through the provider child on a connection that admits the
/// fixture's origin as a file host with the credential: `url_private` is read
/// with the bot token, and the bytes come back with their media type, length
/// and SHA-256.
#[test]
fn the_download_reads_url_private_with_the_token_on_an_admitted_host() {
    use base64::{Engine as _, engine::general_purpose::STANDARD};
    let provider = Provider::files(true);
    let mut child = Child::spawn(&provider.selection()).unwrap();
    let url = format!("{}{FILE_PATH}", provider.origin);
    let result = invoke(&mut child, "fixture.download", &json!({"url": url}));
    assert_eq!(result["status"], 200);
    let body = &result["body"];
    assert_eq!(body["media_type"], "application/pdf");
    assert_eq!(body["length"], FILE_BYTES.len());
    assert_eq!(body["sha256"], hex::encode(Sha256::digest(FILE_BYTES)));
    assert_eq!(
        STANDARD
            .decode(body["content_base64"].as_str().unwrap())
            .unwrap(),
        FILE_BYTES
    );
    let requests = provider.requests();
    assert_eq!(requests.len(), 1);
    assert_eq!(requests[0].0.trim_end_matches('?'), FILE_PATH);
    assert_eq!(requests[0].1.as_deref(), Some(HEADER));
    // Off the prefix, or on a host the selection does not reach: refused, unsent.
    for url in [
        format!("{}/api/auth.test", provider.origin),
        format!("https://files.slack.com{FILE_PATH}"),
    ] {
        let outcome = attempt(&mut child, "fixture.download", &json!({"url": url}));
        assert!(outcome.is_err(), "{url}: {outcome:?}");
    }
    assert_eq!(provider.requests().len(), 1);
}

/// A file host admitted without the credential is sent none: the fixture's
/// file host refuses it, and that answer is the provider's `403`.
#[test]
fn a_host_admitted_without_the_credential_is_sent_none() {
    let provider = Provider::files(false);
    let mut child = Child::spawn(&provider.selection()).unwrap();
    let url = format!("{}{FILE_PATH}", provider.origin);
    let outcome = attempt(&mut child, "fixture.download", &json!({"url": url}));
    assert!(
        matches!(outcome, Err(Failure::ProviderForbidden)),
        "{outcome:?}"
    );
    let requests = provider.requests();
    assert_eq!(requests.len(), 1);
    assert_eq!(requests[0].1, None);
}

/// The shipped download on a connection that does not admit
/// `https://files.slack.com` (the guide's bot configuration without `hosts`)
/// still loads, and the download is refused as `forbidden` with nothing sent.
#[test]
fn the_download_is_refused_on_a_connection_that_does_not_admit_the_file_host() {
    let provider = Provider::new();
    let mut child = Child::spawn(&provider.selection()).unwrap();
    let outcome = attempt(
        &mut child,
        "file.download",
        &json!({"url": format!("https://files.slack.com{FILE_PATH}")}),
    );
    assert!(matches!(outcome, Err(Failure::Forbidden)), "{outcome:?}");
    assert!(provider.requests().is_empty());
}

/// A connection may admit only hosts a selected binary read names, each a
/// distinct canonical https origin: anything else does not load.
#[test]
fn a_connection_admitting_a_host_no_selection_reaches_does_not_load() {
    let provider = Provider::files(true);
    let original: Value = serde_json::from_slice(&fs::read(&provider.config).unwrap()).unwrap();
    for hosts in [
        json!([{"origin": "https://media.example.test", "credential": true}]),
        json!([{"origin": format!("{}/", provider.origin)}]),
        json!([{"origin": provider.origin}, {"origin": provider.origin}]),
        json!([{"origin": provider.origin, "credential": true, "extra": 1}]),
    ] {
        let mut document = original.clone();
        document["hosts"] = hosts.clone();
        private(&provider.config, &serde_json::to_vec(&document).unwrap());
        let output = Command::new(env!("CARGO_BIN_EXE_connectors-catalog-provider"))
            .arg("--local-config")
            .arg(&provider.config)
            .arg("--print-local-bootstrap")
            .output()
            .unwrap();
        assert!(!output.status.success(), "{hosts} loaded");
    }
}

// ── Message send: `chat.postMessage` and `conversations.open` ─────────────────

/// The bot set's four writes, their pinned `operationId` and the path the
/// bundle records. A renamed, dropped or added write fails here.
const WRITES: [(&str, &str, &str); 4] = [
    ("chat.delete", "chat_delete", "/api/chat.delete"),
    (
        "chat.postMessage",
        "chat_postMessage",
        "/api/chat.postMessage",
    ),
    ("chat.update", "chat_update", "/api/chat.update"),
    (
        "conversations.open",
        "conversations_open",
        "/api/conversations.open",
    ),
];

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
fn post_input(channel: &str) -> Value {
    json!({"body": {"channel": channel, "text": "fixture message posted",
                    "thread_ts": "1780000100.000100", "unfurl_links": false}})
}

/// Every write ships as reviewed: `effect: write` over the pinned POST, the
/// token as the credential, a closed typed body with its required keys, and a
/// guard that reads `auth.test` before the write and requires `ok` true both
/// there and in Slack's answer. The bundle records each with a JSON request
/// body by a cited amendment, since the engine sends nothing else. On private
/// protocol two the bootstrap lists each as a write of `slack.bot`, which is
/// what makes the host require an approval; protocol one lists no write.
#[test]
fn the_two_writes_ship_as_guarded_writes_with_a_closed_json_body() {
    let bundle = committed();
    let amendments = bundle
        .source
        .amendments
        .as_ref()
        .expect("an amendment record");
    assert_eq!(amendments.file_name, "slack-web.amendments.json");
    let engine = Engine::new(&bundle, BASE, &shipped()).unwrap();
    let mut writes: Vec<String> = engine
        .declarations(&[Effect::Write])
        .into_iter()
        .map(|o| o.id)
        .collect();
    writes.sort();
    let ids: Vec<&str> = WRITES.iter().map(|(id, _, _)| *id).collect();
    assert_eq!(writes, ids);
    let document = pinned();
    for (id, operation_id, path) in WRITES {
        let selection = shipped().into_iter().find(|s| s.id == id).unwrap();
        assert_eq!(selection.operation_id, operation_id, "`{id}`");
        assert_eq!(selection.effect, Effect::Write, "`{id}`");
        assert_eq!(selection.credential, ["token"], "`{id}`");
        assert!(selection.withhold.is_empty(), "`{id}`");
        let operation = bundle
            .inventory
            .operations
            .iter()
            .find(|o| o.operation_id.as_deref() == Some(operation_id))
            .unwrap();
        assert_eq!(operation.method, "post", "`{id}`");
        assert_eq!(operation.path, path, "`{id}`");
        assert_eq!(
            operation.request_media_types,
            ["application/json"],
            "`{id}`"
        );
        let source = &document["paths"][path.strip_prefix(BASE).unwrap()]["post"];
        assert_eq!(source["operationId"], operation_id, "`{id}`");
        // Every admitted body key is a member the pinned document declares.
        for key in &selection.body_keys {
            assert!(
                source["parameters"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|p| p["name"] == key.as_str() && p["in"] == "formData"),
                "`{id}` admits `{key}`, which the pinned document does not declare"
            );
        }
        let guard = selection.guard.as_ref().expect("a guard");
        assert_eq!(guard.preflight.operation_id, "auth_test", "`{id}`");
        // `ok` as the JSON boolean Slack's schema pins, never compared as text.
        let ok = json!([{"pointer": "/ok", "expect": {"boolean": true}}]);
        assert_eq!(serde_json::to_value(&guard.preflight.checks).unwrap(), ok);
        assert_eq!(serde_json::to_value(&guard.postflight.checks).unwrap(), ok);
        let declaration = engine
            .declarations(&[Effect::Write])
            .into_iter()
            .find(|o| o.id == id)
            .unwrap();
        let schema = &declaration.input_schema;
        assert_eq!(schema["required"], json!(["body"]), "`{id}`");
        assert_eq!(
            schema["properties"]["body"]["additionalProperties"],
            json!(false)
        );
        assert!(schema["properties"].get("token").is_none(), "`{id}`");
    }
    let post = shipped()
        .into_iter()
        .find(|s| s.id == "chat.postMessage")
        .unwrap();
    assert_eq!(
        post.body_keys,
        ["channel", "text", "thread_ts", "mrkdwn", "unfurl_links"]
    );
    assert_eq!(post.body_required, ["channel", "text"]);
    assert_eq!(post.body_nonempty, ["channel", "text", "thread_ts"]);
    let open = shipped()
        .into_iter()
        .find(|s| s.id == "conversations.open")
        .unwrap();
    assert_eq!(open.body_keys, ["users", "return_im"]);
    assert_eq!(open.body_required, ["users"]);
    assert_eq!(open.body_nonempty, ["users"]);

    let two = Provider::writing(false);
    let child = Child::spawn(&two.selection()).unwrap();
    for (id, _, _) in WRITES {
        let requirement = child
            .bootstrap()
            .requirements
            .iter()
            .find(|r| r.operation == id)
            .unwrap_or_else(|| panic!("`{id}` is not a protocol-two requirement"));
        assert!(requirement.effect == runtime::Effect::Write, "`{id}`");
        assert_eq!(requirement.profile, PROFILE);
    }
    let one = Provider::new();
    let child = Child::spawn(&one.selection()).unwrap();
    assert!(
        child
            .bootstrap()
            .requirements
            .iter()
            .all(|r| WRITES.iter().all(|(id, _, _)| r.operation != *id)),
        "protocol one lists a Slack write"
    );
    assert!(two.exchanges().is_empty() && one.exchanges().is_empty());
}

/// Approved, `chat.postMessage` reads `auth.test`, then sends exactly one
/// `POST /api/chat.postMessage` with the body as supplied, as JSON, with the
/// bearer token and no `token` parameter, and returns Slack's answer, whose
/// `channel` and `ts` are the posted message's address.
#[test]
fn an_approved_post_sends_one_json_body_and_returns_channel_and_ts() {
    let provider = Provider::writing(false);
    let mut child = Child::spawn(&provider.selection()).unwrap();
    let input = post_input("C0FIXTURE01");
    let result = write(&mut child, "chat.postMessage", &input)
        .unwrap_or_else(|failure| panic!("`chat.postMessage` refused in prepare: {failure:?}"));
    assert_eq!(result.effect, WriteEffect::Applied);
    let result = result.result.unwrap();
    assert_eq!(result["status"], 200);
    assert_eq!(result["body"]["ok"], true);
    assert_eq!(result["body"]["channel"], "C0FIXTURE01");
    assert_eq!(result["body"]["ts"], POSTED_TS);
    assert_eq!(result["body"]["message"]["text"], "fixture message posted");
    assert_eq!(result["provenance"]["instance"], "fixture-slack");
    let exchanges = provider.exchanges();
    assert_eq!(exchanges.len(), 2, "{exchanges:?}");
    assert_eq!(exchanges[0].0, "GET");
    assert_eq!(exchanges[0].1, "/api/auth.test");
    assert_eq!(exchanges[1].0, "POST");
    assert_eq!(exchanges[1].1, "/api/chat.postMessage");
    assert!(
        exchanges[1]
            .2
            .as_deref()
            .is_some_and(|kind| kind.starts_with("application/json")),
        "{:?}",
        exchanges[1].2
    );
    assert_eq!(exchanges[1].3, input["body"]);
    for (path, authorization, token_header) in provider.requests() {
        assert_eq!(authorization.as_deref(), Some(HEADER), "{path}");
        assert!(!token_header && !path.contains("token="), "{path}");
    }
}

/// Without an approval there is no route to the request: on protocol two the
/// read transport refuses both writes as unsupported, and on protocol one
/// neither is an operation and prepare is unsupported. An input outside the
/// closed body is refused in prepare. Nothing reaches Slack.
#[test]
fn a_send_without_an_approval_is_refused_before_any_request() {
    let two = Provider::writing(false);
    let mut child = Child::spawn(&two.selection()).unwrap();
    for (id, input) in [
        ("chat.postMessage", post_input("C0FIXTURE01")),
        (
            "conversations.open",
            json!({"body": {"users": "U0FIXTURE11"}}),
        ),
    ] {
        let outcome = attempt(&mut child, id, &input);
        assert!(
            matches!(outcome, Err(Failure::Unsupported)),
            "`{id}`: {outcome:?}"
        );
    }
    for input in [
        json!({"body": {"channel": "C0FIXTURE01"}}),
        json!({"body": {"channel": "C0FIXTURE01", "text": "x", "as_user": true}}),
        json!({"body": {"channel": "C0FIXTURE01", "text": "x", "token": TOKEN}}),
        json!({"body": {"channel": "C0FIXTURE01", "text": "x", "mrkdwn": "false"}}),
    ] {
        let outcome = write(&mut child, "chat.postMessage", &input);
        assert!(
            matches!(outcome, Err(Failure::InvalidInput)),
            "{input}: {:?}",
            outcome.map(|result| result.effect)
        );
    }
    let one = Provider::new();
    let mut child = Child::spawn(&one.selection()).unwrap();
    let outcome = attempt(&mut child, "chat.postMessage", &post_input("C0FIXTURE01"));
    assert!(matches!(outcome, Err(Failure::NotFound)), "{outcome:?}");
    let outcome = write(&mut child, "chat.postMessage", &post_input("C0FIXTURE01"));
    assert!(
        matches!(outcome, Err(Failure::Unsupported)),
        "{:?}",
        outcome.map(|result| result.effect)
    );
    assert!(two.exchanges().is_empty(), "protocol two sent a request");
    assert!(one.exchanges().is_empty(), "protocol one sent a request");
}

/// A token Slack does not accept answers `auth.test` with `ok: false`: the
/// guard refuses before the write, and no post is sent.
#[test]
fn a_token_slack_refuses_stops_the_post_before_dispatch() {
    let provider = Provider::writing(true);
    let mut child = Child::spawn(&provider.selection()).unwrap();
    let outcome = write(&mut child, "chat.postMessage", &post_input("C0FIXTURE01"));
    assert!(
        matches!(outcome, Err(Failure::Forbidden)),
        "{:?}",
        outcome.map(|result| result.effect)
    );
    assert!(provider.posts().is_empty(), "a post was sent");
}

/// A post whose connection closes unanswered, or that Slack answers `500`,
/// may have been posted: the outcome is unknown, and the post was sent
/// exactly once, never again by itself.
#[test]
fn an_ambiguous_post_is_outcome_unknown_and_sent_once() {
    let provider = Provider::writing(false);
    let mut child = Child::spawn(&provider.selection()).unwrap();
    for channel in ["C0FIXTUREDROP", "C0FIXTURE500"] {
        let result = write(&mut child, "chat.postMessage", &post_input(channel))
            .unwrap_or_else(|failure| panic!("`{channel}` refused in prepare: {failure:?}"));
        assert_eq!(result.effect, WriteEffect::Unknown, "`{channel}`");
        assert!(result.result.is_err(), "`{channel}`");
    }
    let posts = provider.posts();
    assert_eq!(posts.len(), 2, "each post was sent exactly once: {posts:?}");
    assert_eq!(posts[0].1, post_input("C0FIXTUREDROP")["body"]);
    assert_eq!(posts[1].1, post_input("C0FIXTURE500")["body"]);
}

/// Slack refuses a post as `200` with `ok: false` (`channel_not_found`).
/// The engine classifies by status, so the guard's check of `ok` after the
/// write is what keeps such an answer from being recorded as applied: it is
/// unknown, never applied, and not sent again.
#[test]
fn a_slack_refusal_answered_ok_false_is_never_applied() {
    let provider = Provider::writing(false);
    let mut child = Child::spawn(&provider.selection()).unwrap();
    let result = write(&mut child, "chat.postMessage", &post_input("C0FIXTURE99")).unwrap();
    assert_eq!(result.effect, WriteEffect::Unknown);
    assert!(result.result.is_err());
    assert_eq!(provider.posts().len(), 1);
}

/// `conversations.open` naming one user returns the direct message's channel
/// id, and `chat.postMessage` to that channel posts there: two writes, each
/// prepared and committed on its own, as each needs its own approval.
#[test]
fn conversations_open_returns_the_dm_channel_and_a_post_to_it_lands_there() {
    let provider = Provider::writing(false);
    let mut child = Child::spawn(&provider.selection()).unwrap();
    let open = json!({"body": {"users": "U0FIXTURE11"}});
    let result = write(&mut child, "conversations.open", &open).unwrap();
    assert_eq!(result.effect, WriteEffect::Applied);
    let channel = result.result.unwrap()["body"]["channel"]["id"].clone();
    assert_eq!(channel, DM);
    let input = post_input(channel.as_str().unwrap());
    let result = write(&mut child, "chat.postMessage", &input).unwrap();
    assert_eq!(result.effect, WriteEffect::Applied);
    let result = result.result.unwrap();
    assert_eq!(result["body"]["channel"], DM);
    assert_eq!(result["body"]["ts"], POSTED_TS);
    assert_eq!(
        provider.posts(),
        [
            ("/api/conversations.open".to_owned(), open["body"].clone()),
            ("/api/chat.postMessage".to_owned(), input["body"].clone()),
        ]
    );
    // A user Slack cannot open is answered `ok: false`: unknown, not applied.
    let result = write(
        &mut child,
        "conversations.open",
        &json!({"body": {"users": "U0NOBODY"}}),
    )
    .unwrap();
    assert_eq!(result.effect, WriteEffect::Unknown);
}

// ── Message edit and delete: `chat.update` and `chat.delete` ──────────────────

fn edit_input(channel: &str, ts: &str) -> Value {
    json!({"body": {"channel": channel, "ts": ts, "text": "fixture message edited"}})
}
fn delete_input(channel: &str, ts: &str) -> Value {
    json!({"body": {"channel": channel, "ts": ts}})
}

/// The refusals each of edit and delete declares, as `(pointer, values, code)`:
/// Slack's `200` with `ok: false` and one of these in `error` left the message
/// as it was.
fn declared(id: &str) -> Value {
    let not_found = json!({"pointer": "/error",
                           "values": ["message_not_found", "channel_not_found"],
                           "code": "not_found"});
    match id {
        "chat.update" => json!([
            not_found,
            {"pointer": "/error", "values": ["cant_update_message", "edit_window_closed"],
             "code": "forbidden"},
            {"pointer": "/error", "values": ["msg_too_long", "no_text"], "code": "invalid_input"}
        ]),
        "chat.delete" => json!([
            not_found,
            {"pointer": "/error",
             "values": ["cant_delete_message", "compliance_exports_prevent_deletion"],
             "code": "forbidden"}
        ]),
        _ => unreachable!(),
    }
}

/// story:parity-slack-message-writes: edit and delete name the message by
/// `channel` and `ts`, both required non-empty strings (the pinned document
/// types `chat.delete`'s `ts` as a number; Slack's `ts` is a string), edit
/// also its new `text`; nothing else is admitted. Each declares Slack's
/// refusals by name, every value one the pinned error enum of that method
/// lists, while send declares none.
#[test]
fn edit_and_delete_ship_closed_bodies_and_declare_slacks_refusals_by_name() {
    let document = pinned();
    for (id, keys) in [
        ("chat.update", vec!["channel", "ts", "text"]),
        ("chat.delete", vec!["channel", "ts"]),
    ] {
        let selection = shipped().into_iter().find(|s| s.id == id).unwrap();
        assert_eq!(selection.body_keys, keys, "`{id}`");
        assert_eq!(selection.body_required, keys, "`{id}`");
        assert_eq!(selection.body_nonempty, keys, "`{id}`");
        for key in &keys {
            assert_eq!(
                serde_json::to_value(selection.body_types[*key]).unwrap(),
                "string",
                "`{id}` `{key}`"
            );
        }
        let guard = selection.guard.as_ref().unwrap();
        let refusals = serde_json::to_value(&guard.postflight).unwrap()["refusals"].clone();
        assert_eq!(refusals, declared(id), "`{id}`");
        let path = format!("/{id}");
        let errors = &document["paths"][path.as_str()]["post"]["responses"]["default"]["schema"]["properties"]
            ["error"]["enum"];
        for value in refusals
            .as_array()
            .unwrap()
            .iter()
            .flat_map(|r| r["values"].as_array().unwrap())
        {
            assert!(
                errors.as_array().unwrap().contains(value),
                "`{id}` declares {value}, which its pinned error enum does not list"
            );
        }
    }
    for id in ["chat.postMessage", "conversations.open"] {
        let selection = shipped().into_iter().find(|s| s.id == id).unwrap();
        let postflight = serde_json::to_value(&selection.guard.unwrap().postflight).unwrap();
        assert!(postflight.get("refusals").is_none(), "`{id}`");
    }
}

/// Approved, `chat.update` reads `auth.test`, then sends exactly one
/// `POST /api/chat.update` with the body as supplied, as JSON, with the
/// bearer token and no `token` parameter, and returns Slack's answer: the
/// same `channel` and `ts` and the message's new text.
#[test]
fn an_approved_edit_sends_one_json_body_and_returns_the_edited_message() {
    let provider = Provider::writing(false);
    let mut child = Child::spawn(&provider.selection()).unwrap();
    let input = edit_input("C0FIXTURE01", POSTED_TS);
    let result = write(&mut child, "chat.update", &input)
        .unwrap_or_else(|failure| panic!("`chat.update` refused in prepare: {failure:?}"));
    assert_eq!(result.effect, WriteEffect::Applied);
    let result = result.result.unwrap();
    assert_eq!(result["status"], 200);
    assert_eq!(result["body"]["ok"], true);
    assert_eq!(result["body"]["channel"], "C0FIXTURE01");
    assert_eq!(result["body"]["ts"], POSTED_TS);
    assert_eq!(result["body"]["text"], "fixture message edited");
    assert_eq!(result["body"]["message"]["text"], "fixture message edited");
    assert_eq!(result["provenance"]["instance"], "fixture-slack");
    let exchanges = provider.exchanges();
    assert_eq!(exchanges.len(), 2, "{exchanges:?}");
    assert_eq!(
        (exchanges[0].0.as_str(), exchanges[0].1.as_str()),
        ("GET", "/api/auth.test")
    );
    assert_eq!(
        (exchanges[1].0.as_str(), exchanges[1].1.as_str()),
        ("POST", "/api/chat.update")
    );
    assert!(
        exchanges[1]
            .2
            .as_deref()
            .is_some_and(|kind| kind.starts_with("application/json")),
        "{:?}",
        exchanges[1].2
    );
    assert_eq!(exchanges[1].3, input["body"]);
    for (path, authorization, token_header) in provider.requests() {
        assert_eq!(authorization.as_deref(), Some(HEADER), "{path}");
        assert!(!token_header && !path.contains("token="), "{path}");
    }
}

/// Approved, `chat.delete` reads `auth.test`, then sends exactly one
/// `POST /api/chat.delete` with the body as supplied, as JSON, and returns
/// Slack's answer, the deleted message's `channel` and `ts`.
#[test]
fn an_approved_delete_sends_one_json_body_and_returns_channel_and_ts() {
    let provider = Provider::writing(false);
    let mut child = Child::spawn(&provider.selection()).unwrap();
    let input = delete_input("C0FIXTURE01", POSTED_TS);
    let result = write(&mut child, "chat.delete", &input)
        .unwrap_or_else(|failure| panic!("`chat.delete` refused in prepare: {failure:?}"));
    assert_eq!(result.effect, WriteEffect::Applied);
    let result = result.result.unwrap();
    assert_eq!(
        result["body"],
        json!({"ok": true, "channel": "C0FIXTURE01", "ts": POSTED_TS})
    );
    let exchanges = provider.exchanges();
    assert_eq!(exchanges.len(), 2, "{exchanges:?}");
    assert_eq!(
        (exchanges[0].0.as_str(), exchanges[0].1.as_str()),
        ("GET", "/api/auth.test")
    );
    assert_eq!(
        (exchanges[1].0.as_str(), exchanges[1].1.as_str()),
        ("POST", "/api/chat.delete")
    );
    assert!(
        exchanges[1]
            .2
            .as_deref()
            .is_some_and(|kind| kind.starts_with("application/json"))
    );
    assert_eq!(exchanges[1].3, input["body"]);
    for (path, authorization, token_header) in provider.requests() {
        assert_eq!(authorization.as_deref(), Some(HEADER), "{path}");
        assert!(!token_header && !path.contains("token="), "{path}");
    }
}

/// A bot token edits and deletes only its own messages: Slack answers another
/// user's message `200` with `ok: false` and `cant_update_message` or
/// `cant_delete_message`, which is refused as the provider's `forbidden`; a
/// message or a channel Slack does not find is refused as the provider's
/// `not_found`. Each was sent exactly once and never again.
#[test]
fn an_edit_or_delete_slack_refuses_by_name_is_refused_and_sent_once() {
    let provider = Provider::writing(false);
    let mut child = Child::spawn(&provider.selection()).unwrap();
    let cases = [
        (
            "chat.update",
            edit_input("C0FIXTURE01", FOREIGN_TS),
            Failure::ProviderForbidden,
        ),
        (
            "chat.delete",
            delete_input("C0FIXTURE01", FOREIGN_TS),
            Failure::ProviderForbidden,
        ),
        (
            "chat.update",
            edit_input("C0FIXTURE01", "1780000999.000999"),
            Failure::ProviderNotFound,
        ),
        (
            "chat.delete",
            delete_input("C0FIXTURE01", "1780000999.000999"),
            Failure::ProviderNotFound,
        ),
        (
            "chat.update",
            edit_input("C0FIXTURE99", POSTED_TS),
            Failure::ProviderNotFound,
        ),
        (
            "chat.delete",
            delete_input("C0FIXTURE99", POSTED_TS),
            Failure::ProviderNotFound,
        ),
    ];
    for (id, input, expected) in &cases {
        let result = write(&mut child, id, input)
            .unwrap_or_else(|failure| panic!("`{id}` refused in prepare: {failure:?}"));
        assert_eq!(result.effect, WriteEffect::Refused, "`{id}` {input}");
        assert_eq!(
            format!("{:?}", result.result.err()),
            format!("{:?}", Some(expected)),
            "`{id}` {input}"
        );
    }
    let posts = provider.posts();
    let expected: Vec<(String, Value)> = cases
        .iter()
        .map(|(id, input, _)| (format!("/api/{id}"), input["body"].clone()))
        .collect();
    assert_eq!(posts, expected, "each was sent exactly once");
}

/// Without an approval there is no route to either write, and an input
/// outside the closed body is refused in prepare: a missing `ts`, a `ts` sent
/// as a number, an empty `text`, `as_user`, `blocks` or `token`. A token Slack
/// refuses stops either write at the `auth.test` read. Nothing is sent.
#[test]
fn an_edit_or_delete_without_an_approval_or_outside_the_closed_body_sends_nothing() {
    let provider = Provider::writing(false);
    let mut child = Child::spawn(&provider.selection()).unwrap();
    for (id, input) in [
        ("chat.update", edit_input("C0FIXTURE01", POSTED_TS)),
        ("chat.delete", delete_input("C0FIXTURE01", POSTED_TS)),
    ] {
        let outcome = attempt(&mut child, id, &input);
        assert!(
            matches!(outcome, Err(Failure::Unsupported)),
            "`{id}`: {outcome:?}"
        );
    }
    for (id, input) in [
        (
            "chat.update",
            json!({"body": {"channel": "C0FIXTURE01", "text": "x"}}),
        ),
        (
            "chat.update",
            json!({"body": {"channel": "C0FIXTURE01", "ts": POSTED_TS, "text": ""}}),
        ),
        (
            "chat.update",
            json!({"body": {"channel": "C0FIXTURE01", "ts": POSTED_TS, "text": "x",
                            "blocks": "[]"}}),
        ),
        (
            "chat.update",
            json!({"body": {"channel": "C0FIXTURE01", "ts": POSTED_TS, "text": "x",
                            "as_user": "true"}}),
        ),
        ("chat.delete", json!({"body": {"channel": "C0FIXTURE01"}})),
        (
            "chat.delete",
            json!({"body": {"channel": "C0FIXTURE01", "ts": 1780000700.0007}}),
        ),
        (
            "chat.delete",
            json!({"body": {"channel": "", "ts": POSTED_TS}}),
        ),
        (
            "chat.delete",
            json!({"body": {"channel": "C0FIXTURE01", "ts": POSTED_TS, "token": TOKEN}}),
        ),
    ] {
        let outcome = write(&mut child, id, &input);
        assert!(
            matches!(outcome, Err(Failure::InvalidInput)),
            "`{id}` {input}: {:?}",
            outcome.map(|result| result.effect)
        );
    }
    assert!(provider.exchanges().is_empty(), "a request was sent");
    let rejected = Provider::writing(true);
    let mut child = Child::spawn(&rejected.selection()).unwrap();
    for (id, input) in [
        ("chat.update", edit_input("C0FIXTURE01", POSTED_TS)),
        ("chat.delete", delete_input("C0FIXTURE01", POSTED_TS)),
    ] {
        let outcome = write(&mut child, id, &input);
        assert!(
            matches!(outcome, Err(Failure::Forbidden)),
            "`{id}`: {:?}",
            outcome.map(|result| result.effect)
        );
    }
    assert!(rejected.posts().is_empty(), "a write was sent");
}

/// An `ok: false` the selection does not declare (`is_inactive`), and a
/// dispatch whose connection closes unanswered, may or may not have changed
/// the message: each is unknown, never applied, and sent exactly once.
#[test]
fn an_undeclared_or_unanswered_edit_or_delete_is_unknown_and_sent_once() {
    let provider = Provider::writing(false);
    let mut child = Child::spawn(&provider.selection()).unwrap();
    let cases = [
        ("chat.update", edit_input("C0FIXTURE01", INACTIVE_TS)),
        ("chat.update", edit_input("C0FIXTUREDROP", POSTED_TS)),
        ("chat.delete", delete_input("C0FIXTUREDROP", POSTED_TS)),
    ];
    for (id, input) in &cases {
        let result = write(&mut child, id, input).unwrap();
        assert_eq!(result.effect, WriteEffect::Unknown, "`{id}` {input}");
        assert!(result.result.is_err(), "`{id}` {input}");
    }
    let expected: Vec<(String, Value)> = cases
        .iter()
        .map(|(id, input)| (format!("/api/{id}"), input["body"].clone()))
        .collect();
    assert_eq!(provider.posts(), expected);
}
