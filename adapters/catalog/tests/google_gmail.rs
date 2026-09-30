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
//!
//! The two draft writes run through the same child over private protocol two,
//! on the guide's write instance with the compose scope granted: the host's
//! prepare and commit, the send's preflight read of the draft, and the exact
//! POST body. A draft whose message changed since the approval sends nothing.
//! The approval binding is exercised with the host's own approval signer and
//! verifier against a subject built from the child's descriptor; the signing
//! key is the public RFC 8032 section 7.1 test vector, never a deployment key.
use connectors_catalog::{bundle, discovery};
use connectors_catalog_provider::{Check, Effect, Engine, Expectation, Selection};
use connectors_host::local::{
    approvals,
    config::{Adapter, Executable, Restart, Startup},
    filesystem,
    metadata::Metadata,
    mutations, registry,
    runtime::{Bootstrap, Child, Failure, PrivateProtocol, WriteEffect},
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
/// The scope the two draft writes and the send's preflight read need, per the
/// pinned document. It does not cover the seven reads.
const COMPOSE_SCOPE: &str = "https://www.googleapis.com/auth/gmail.compose";
/// The instance id of the guide's write instance.
const WRITE_INSTANCE: &str = "google-gmail-write";
/// The `auth_uri` Google writes into a downloaded installed-app client file;
/// the client-file connect compares the profile's `authorize_url` with it
/// byte for byte.
const GOOGLE_CLIENT_FILE_AUTH_URI: &str = "https://accounts.google.com/o/oauth2/auth";
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
/// The shipped writes, their Discovery method id and the path the bundle
/// records. Both are POSTs with a JSON body. Gmail is written through drafts
/// only: `users.messages.send` is not among them.
const WRITES: [(&str, &str, &str); 2] = [
    (
        "users.drafts.create",
        "gmail.users.drafts.create",
        "/gmail/v1/users/{userId}/drafts",
    ),
    (
        "users.drafts.send",
        "gmail.users.drafts.send",
        "/gmail/v1/users/{userId}/drafts/send",
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
fn shipped_gmail_selections_are_the_seven_reads_and_two_draft_writes() {
    let selections = shipped();
    let bundle = committed_bundle();
    assert_eq!(bundle.auth_profile, PROFILE);
    let engine = Engine::new(&bundle, BASE, &selections).unwrap();
    let ids = |effect: Effect| {
        let mut ids: Vec<String> = engine
            .declarations(&[effect])
            .into_iter()
            .map(|o| o.id)
            .collect();
        ids.sort();
        ids
    };
    let expected: Vec<&str> = SHIPPED.iter().map(|(id, _, _)| *id).collect();
    assert_eq!(ids(Effect::Read), expected);
    let expected: Vec<&str> = WRITES.iter().map(|(id, _, _)| *id).collect();
    assert_eq!(ids(Effect::Write), expected);
    assert_eq!(selections.len(), SHIPPED.len() + WRITES.len());
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
    let declarations = engine.declarations(&[Effect::Write]);
    for (id, operation_id, path) in WRITES {
        let selection = selections.iter().find(|s| s.id == id).unwrap();
        assert_eq!(selection.operation_id, operation_id, "`{id}`");
        assert_eq!(Some(id), operation_id.strip_prefix("gmail."), "`{id}`");
        assert_eq!(selection.effect, Effect::Write, "`{id}`");
        assert_eq!(engine.effect(id), Some(Effect::Write), "`{id}`");
        assert_eq!(selection.response, None, "`{id}`");
        assert!(selection.bounds.is_empty(), "`{id}`");
        assert!(selection.required.is_empty(), "`{id}`");
        let operation = bundle
            .inventory
            .operations
            .iter()
            .find(|o| o.operation_id.as_deref() == Some(operation_id))
            .unwrap_or_else(|| panic!("the projection lacks `{operation_id}`"));
        assert_eq!(operation.method, "post", "`{id}`");
        assert_eq!(operation.path, path, "`{id}`");
        assert_eq!(
            operation.request_media_types,
            ["application/json"],
            "`{id}`"
        );
        // A write is a required-approval mutation, and its whole input —
        // including the body the approval digest covers — is closed.
        let declaration = declarations.iter().find(|o| o.id == id).unwrap();
        assert_eq!(declaration.profile, "mutation", "`{id}`");
        let schema = &declaration.input_schema;
        assert_eq!(schema["additionalProperties"], false, "`{id}`");
        let required = schema["required"].as_array().unwrap();
        assert!(required.contains(&json!("body")), "`{id}` {required:?}");
        assert!(required.contains(&json!("userId")), "`{id}` {required:?}");
    }
}

/// Gmail is written through drafts only. The bundle carries
/// `gmail.users.messages.send`, so its absence is the selection's choice: no
/// selection exposes it under any id, and the engine does not know it.
#[test]
fn users_messages_send_is_not_selected() {
    let bundle = committed_bundle();
    let absent = "gmail.users.messages.send";
    assert!(
        bundle
            .inventory
            .operations
            .iter()
            .any(|o| o.operation_id.as_deref() == Some(absent)),
        "the bundle no longer carries `{absent}`"
    );
    let selections = shipped();
    assert!(
        selections.iter().all(|s| s.operation_id != absent),
        "`{absent}` is selected"
    );
    assert!(selections.iter().all(|s| s.id != "users.messages.send"));
    assert_eq!(engine().effect("users.messages.send"), None);
    // Every write sends or makes a draft; nothing else writes.
    for selection in selections.iter().filter(|s| s.effect == Effect::Write) {
        assert!(
            selection.operation_id.starts_with("gmail.users.drafts."),
            "`{}` writes outside drafts",
            selection.id
        );
    }
}

/// `users.drafts.create` carries no guard: nothing exists to compare before a
/// draft exists. `users.drafts.send` reads the draft named by `body.id` with
/// `users.drafts.get` and compares its `message.id` with the input's
/// `messageId`, the message the issuer read. Nothing is compared after
/// dispatch. The pinned document names the fields the guard reads.
#[test]
fn drafts_send_is_guarded_on_the_drafts_message_id() {
    let selections = shipped();
    let create = selections
        .iter()
        .find(|s| s.id == "users.drafts.create")
        .unwrap();
    assert_eq!(create.guard, None);
    let guard = selections
        .iter()
        .find(|s| s.id == "users.drafts.send")
        .unwrap()
        .guard
        .as_ref()
        .expect("`users.drafts.send` is guarded");
    assert_eq!(guard.preflight.operation_id, "gmail.users.drafts.get");
    assert_eq!(
        guard.preflight.values,
        [
            ("id".to_owned(), "body.id".to_owned()),
            ("userId".to_owned(), "userId".to_owned()),
        ]
        .into()
    );
    assert_eq!(
        guard.preflight.checks,
        [Check {
            pointer: "/message/id".into(),
            expect: Expectation::Input("messageId".into()),
        }]
    );
    assert!(guard.postflight.checks.is_empty());
    // `messageId` is no parameter of the send: it is declared, required, and
    // never sent (the fixture test asserts the POST body).
    let send = engine()
        .declarations(&[Effect::Write])
        .into_iter()
        .find(|o| o.id == "users.drafts.send")
        .unwrap();
    let required = send.input_schema["required"].as_array().unwrap();
    assert!(required.contains(&json!("messageId")), "{required:?}");
    let document = pinned();
    let schemas = &document["schemas"];
    assert_eq!(
        schemas["Draft"]["properties"]["id"]["annotations"]["required"],
        json!(["gmail.users.drafts.send"])
    );
    assert_eq!(schemas["Draft"]["properties"]["message"]["$ref"], "Message");
    assert_eq!(
        schemas["Message"]["properties"]["id"]["description"],
        "The immutable ID of the message."
    );
    assert_eq!(
        pinned_method(&document, "users.drafts.send")["response"]["$ref"],
        "Message"
    );
    // Both writes and the preflight read accept the compose scope; neither
    // write accepts the read-only one, and of the shipped reads only
    // `users.getProfile` accepts compose.
    for id in [
        "users.drafts.create",
        "users.drafts.send",
        "users.drafts.get",
    ] {
        let scopes = pinned_method(&document, id)["scopes"].clone();
        assert!(
            scopes.as_array().unwrap().contains(&json!(COMPOSE_SCOPE)),
            "`{id}`"
        );
    }
    for (id, _, _) in WRITES {
        let scopes = pinned_method(&document, id)["scopes"].clone();
        assert!(
            !scopes.as_array().unwrap().contains(&json!(GMAIL_SCOPE)),
            "`{id}`"
        );
    }
    // The guide's other scope sentences: the preflight read accepts the
    // read-only scope as well, and at Google the compose scope also permits
    // the unselected `users.messages.send`.
    for (id, scope) in [
        ("users.drafts.get", GMAIL_SCOPE),
        ("users.messages.send", COMPOSE_SCOPE),
    ] {
        let scopes = pinned_method(&document, id)["scopes"].clone();
        assert!(
            scopes.as_array().unwrap().contains(&json!(scope)),
            "`{id}` {scope}"
        );
    }
    let composing: Vec<&str> = SHIPPED
        .iter()
        .map(|(id, _, _)| *id)
        .filter(|id| {
            pinned_method(&document, id)["scopes"]
                .as_array()
                .unwrap()
                .contains(&json!(COMPOSE_SCOPE))
        })
        .collect();
    assert_eq!(composing, ["users.getProfile"]);
}

/// Each write's description names its effects, and says the approval binds
/// the whole input by digest, so the issuer reads the decoded message first.
#[test]
fn draft_write_descriptions_name_every_effect() {
    let selections = shipped();
    let description = |id: &str| {
        selections
            .iter()
            .find(|s| s.id == id)
            .unwrap()
            .description
            .clone()
            .unwrap_or_default()
    };
    let mut missing = Vec::new();
    for (id, terms) in [
        (
            "users.drafts.create",
            &[
                "body.message.raw",
                "RFC 5322",
                "base64url",
                "sends nothing",
                "To",
                "Cc",
                "Bcc",
                "Subject",
                "From",
                "body.message.threadId",
                "second draft",
                "Unguarded",
                "the approval binds the whole input by digest",
            ][..],
        ),
        (
            "users.drafts.send",
            &[
                "every recipient in its To, Cc and Bcc headers",
                "cannot be recalled",
                "messageId",
                "users.drafts.get",
                "body.message",
                "the approval binds the whole input by digest",
                "decoded",
            ][..],
        ),
    ] {
        let text = description(id);
        for term in terms {
            if !text.contains(term) {
                missing.push(format!("`{id}`: {term}"));
            }
        }
    }
    assert!(missing.is_empty(), "descriptions omit: {missing:#?}");
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

/// The guide's rows, sections and write route for the two draft writes: each
/// write cited with its Discovery id and POST route, the compose scope, the
/// input the send's guard compares, `users.messages.send` named as not
/// selected, and a separate write instance connected afresh. Repair cannot
/// add a scope, and the guide must not say it does.
#[test]
fn guide_cites_the_draft_writes_the_compose_scope_and_the_write_instance() {
    let guide = guide();
    let rows: Vec<&str> = guide.lines().filter(|l| l.starts_with('|')).collect();
    for (id, operation_id, path) in WRITES {
        let cited = rows.iter().any(|row| {
            row.starts_with(&format!("| `{id}` "))
                && row.contains(&format!("`{operation_id}`"))
                && row.contains(&format!("`POST {path}`"))
        });
        assert!(
            cited,
            "no row cites `{id}` as `{operation_id}` `POST {path}`"
        );
    }
    let flat = guide.split_whitespace().collect::<Vec<_>>().join(" ");
    for term in [
        format!("`{COMPOSE_SCOPE}`"),
        "`users.messages.send` is not selected".into(),
        "`message.raw`".into(),
        "base64url".into(),
        "RFC 5322".into(),
        "`messageId`".into(),
        "`message.id`".into(),
        "`users.drafts.get`".into(),
        "cannot be recalled".into(),
        "A changed draft sends nothing".into(),
        "Writes use a separate instance".into(),
        format!("`{WRITE_INSTANCE}`"),
        "`connections connect` that instance with the Google client file".into(),
        "`connections repair` cannot add a scope".into(),
        "`approval_issuance.rs`".into(),
    ] {
        assert!(flat.contains(&term), "the guide does not state {term}");
    }
    assert!(
        !flat.contains("with `connections repair`"),
        "the guide routes the write scope through `connections repair`"
    );
}

/// The JSON block of the guide that contains `marker`.
fn guide_block(marker: &str) -> Value {
    let guide = guide();
    let example = guide
        .split("```json\n")
        .skip(1)
        .filter_map(|rest| rest.split_once("\n```").map(|(body, _)| body))
        .find(|body| body.contains(marker))
        .unwrap_or_else(|| panic!("the guide has no JSON block with {marker}"));
    serde_json::from_str::<Value>(example).unwrap()
}
/// The guide's configuration example for this provider.
fn documented_config() -> Value {
    guide_block(&format!("\"provider\": \"{PROVIDER}\""))
}
/// The guide's write instance: its read configuration with the write values
/// the guide lists applied.
fn documented_write_config() -> Value {
    let values = guide_block(&format!("\"instance\": \"{WRITE_INSTANCE}\""));
    let mut config = documented_config();
    config["instance"] = values["instance"].clone();
    config["auth"]["minimum_scopes"] = values["auth"]["minimum_scopes"].clone();
    config["auth"]["requested_scopes"] = values["auth"]["requested_scopes"].clone();
    config
}

/// The write instance asks for the compose scope and nothing that reads the
/// mailbox, and keeps the read block's `authorize_url`, which must be the
/// client file's `auth_uri` for `connections connect` to accept the file.
#[test]
fn guide_documents_the_write_instance() {
    let config = documented_write_config();
    assert_eq!(config["instance"], WRITE_INSTANCE);
    assert_eq!(config["auth"]["minimum_scopes"], json!([COMPOSE_SCOPE]));
    assert_eq!(
        config["auth"]["requested_scopes"],
        json!(["openid", COMPOSE_SCOPE])
    );
    assert_eq!(config["auth"]["authorize_url"], GOOGLE_CLIENT_FILE_AUTH_URI);
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
    assert_eq!(auth["authorize_url"], GOOGLE_CLIENT_FILE_AUTH_URI);
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
        // The draft as it is now: edited since the issuer read it, so its
        // message is no longer `STALE_MESSAGE`.
        "/gmail/v1/users/me/drafts/fixture-draft-1" => ok(json!({
            "id": DRAFT,
            "message": {"id": DRAFT_MESSAGE, "threadId": "fixture-thread-1",
                        "labelIds": ["DRAFT"], "snippet": "Fixture draft"}})),
        _ => None,
    }
}

const DRAFT: &str = "fixture-draft-1";
/// The fixture draft's current message.
const DRAFT_MESSAGE: &str = "fixture-draft-message-2";
/// The message the draft carried before an edit.
const STALE_MESSAGE: &str = "fixture-draft-message-1";
/// A base64url RFC 5322 message: `To`, `Subject`, a blank line, a body.
const RAW: &str =
    "VG86IHJlY2lwaWVudEBleGFtcGxlLnRlc3QNClN1YmplY3Q6IEZpeHR1cmUNCg0KRml4dHVyZSBib2R5DQo";
/// The recorded answers to the two writes, keyed by route.
fn posted(target: &str) -> Option<Value> {
    let (route, _) = target.split_once('?').unwrap_or((target, ""));
    match route {
        "/gmail/v1/users/me/drafts" => Some(json!({
            "id": "fixture-draft-new",
            "message": {"id": "fixture-draft-message-new", "threadId": "fixture-thread-new",
                        "labelIds": ["DRAFT"]}})),
        "/gmail/v1/users/me/drafts/send" => Some(json!({
            "id": "fixture-sent-message-1", "threadId": "fixture-thread-1",
            "labelIds": ["SENT"]})),
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
    instance: String,
    requests: Requests,
    bodies: Bodies,
}
/// Route with query and body of each POST that reached a Gmail route.
type Bodies = Arc<Mutex<Vec<(String, Vec<u8>)>>>;
impl Provider {
    /// The guide's read configuration against the fixture, whose token route
    /// grants the read-only scope.
    fn new() -> Self {
        Self::with(documented_config(), GMAIL_SCOPE)
    }
    /// The guide's write instance against the fixture, whose token route
    /// grants the compose scope.
    fn writer() -> Self {
        Self::with(documented_write_config(), COMPOSE_SCOPE)
    }
    /// `config` against the fixture, whose token route grants `granted`.
    fn with(mut config: Value, granted: &'static str) -> Self {
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
        let recorded_bodies = bodies.clone();
        let thread = std::thread::spawn(move || {
            let bodies = recorded_bodies;
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
                                    "scope": format!("openid {granted}"),
                                    "id_token": id_token()}),
                            )
                        } else {
                            (400, json!({"error": "invalid_grant"}))
                        }
                    } else if authorization.as_deref() != Some(&*format!("Bearer {ACCESS_TOKEN}"))
                    {
                        (401, json!({"error": {"code": 401, "message": "fixture refusal"}}))
                    } else {
                        if method == "POST" {
                            bodies.lock().unwrap().push((target.clone(), body));
                        }
                        let found = match method.as_str() {
                            "GET" => answer(&target),
                            "POST" => posted(&target).map(|value| (200, value)),
                            _ => None,
                        };
                        found.unwrap_or_else(|| {
                            (404, json!({"error": {"code": 404, "message": "no fixture"}}))
                        })
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
        // The given configuration, with the fixture as API host and token
        // host, trusted through its own CA.
        let instance = format!("fixture-{}", config["instance"].as_str().unwrap());
        config["instance"] = json!(instance);
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
            instance,
            requests,
            bodies,
        }
    }
    /// The same selection over private protocol two, which carries writes.
    fn write_selection(&self) -> Adapter {
        Adapter {
            private_protocol: Some(PrivateProtocol::V2),
            ..self.selection()
        }
    }
    fn bodies(&self) -> Vec<(String, Value)> {
        self.bodies
            .lock()
            .unwrap()
            .iter()
            .map(|(target, body)| (target.clone(), serde_json::from_slice(body).unwrap()))
            .collect()
    }
    fn selection(&self) -> Adapter {
        let bootstrap = print_bootstrap(&self.config);
        let binary = PathBuf::from(env!("CARGO_BIN_EXE_connectors-catalog-provider"))
            .canonicalize()
            .unwrap();
        Adapter {
            private_protocol: None,
            permissions: Default::default(),
            instance_id: self.instance.clone(),
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
/// The bootstrap the provider prints for the configuration at `path`.
fn print_bootstrap(path: &Path) -> Bootstrap {
    let output = Command::new(env!("CARGO_BIN_EXE_connectors-catalog-provider"))
        .arg("--local-config")
        .arg(path)
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
    bootstrap
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

/// Each write's approved input: a draft of one base64url message, and the send
/// of the fixture draft pinned to the message it carries now.
fn write_inputs() -> [(&'static str, Value); 2] {
    [
        (
            "users.drafts.create",
            json!({"userId": "me", "body": {"message": {"raw": RAW}}}),
        ),
        (
            "users.drafts.send",
            json!({"userId": "me", "messageId": DRAFT_MESSAGE, "body": {"id": DRAFT}}),
        ),
    ]
}
/// The same input with what the approval must pin changed: another message
/// for a create, another pinned message for a send.
fn other_input(input: &Value) -> Value {
    let mut other = input.clone();
    if other.get("messageId").is_some() {
        other["messageId"] = json!(STALE_MESSAGE);
    } else {
        other["body"]["message"]["raw"] = json!("VG86IG90aGVyQGV4YW1wbGUudGVzdA0KDQpPdGhlcg0K");
    }
    other
}
/// Prepare through the host's write exchange and, if that succeeds, commit.
fn write(
    child: &mut Child,
    operation: &str,
    input: &Value,
) -> Result<connectors_host::local::runtime::WriteResult, Failure> {
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

/// A write offered on the dispatch path that carries no approval is refused
/// by the host before the child is asked for anything: no token exchange and
/// no Gmail request. Over private protocol one a write is not even described;
/// over private protocol two it is a `mutation`, which only the host's
/// prepare/commit exchange runs, and the owner enters that exchange only with
/// a verified proof (`owner/mutation/execution.rs`). An absent or empty proof
/// document is refused where the owner decodes it.
#[test]
fn each_write_without_an_approval_is_refused_before_any_request() {
    let provider = Provider::writer();
    let mut v1 = Child::spawn(&provider.selection()).unwrap();
    let mut v2 = Child::spawn(&provider.write_selection()).unwrap();
    let described = v2.bootstrap().descriptor().unwrap();
    for (operation, input) in write_inputs() {
        assert!(
            v1.bootstrap()
                .descriptor()
                .unwrap()
                .operation(operation)
                .is_err(),
            "`{operation}` described over private protocol one"
        );
        assert!(
            matches!(attempt(&mut v1, operation, &input), Err(Failure::NotFound)),
            "`{operation}` over private protocol one"
        );
        assert_eq!(
            described.operation(operation).unwrap().profile,
            "mutation",
            "`{operation}`"
        );
        assert!(
            matches!(
                attempt(&mut v2, operation, &input),
                Err(Failure::Unsupported)
            ),
            "`{operation}` dispatched without an approval"
        );
    }
    // `users.messages.send` is not an operation of this provider at all.
    assert!(described.operation("users.messages.send").is_err());
    assert!(matches!(
        attempt(
            &mut v2,
            "users.messages.send",
            &json!({"userId": "me", "body": {"raw": RAW}})
        ),
        Err(Failure::NotFound)
    ));
    for document in [b"".as_slice(), b"{}"] {
        assert!(matches!(
            approvals::Evidence::from_document(Secret(document.to_vec())),
            Err(approvals::Failure::Refused)
        ));
    }
    assert!(provider.requests().is_empty(), "a request was sent");
}

/// The public RFC 8032 section 7.1 test 1 key pair: test material only.
const APPROVAL_SEED: &str = "9d61b19deffd5a60ba844af492ec2cc44449c5697b326919703bac031cae7f60";
const APPROVAL_PUBLIC: &str = "d75a980182b10ab7d54bfed3c964073a0ee172f3daa62325af021a68f707511a";
struct Now;
impl mutations::Clock for Now {
    fn now(&self) -> mutations::Result<mutations::ClockInterval> {
        let now = connectors_sdk::now_ms() as i64;
        Ok(mutations::ClockInterval {
            lower_unix_ms: now,
            upper_unix_ms: now + 1000,
        })
    }
}
struct ApprovalKey(approvals::ConfiguredApprovalKey);
impl approvals::CurrentAdmission for &ApprovalKey {
    fn key(&self) -> &approvals::ConfiguredApprovalKey {
        &self.0
    }
}
/// Admits by key id only, so any refusal is the proof's own subject binding.
impl approvals::ReceiverPolicy for ApprovalKey {
    type Guard<'a> = &'a ApprovalKey;
    fn admit<'a>(&'a self, _: &approvals::Subject, kid: &str) -> approvals::Result<&'a Self> {
        if kid == self.0.kid {
            Ok(self)
        } else {
            Err(approvals::Failure::Refused)
        }
    }
}
impl approvals::IssuancePolicy for ApprovalKey {
    type Guard<'a> = &'a ApprovalKey;
    fn authorize<'a>(
        &'a self,
        subject: &approvals::Subject,
        kid: &str,
    ) -> approvals::Result<&'a Self> {
        approvals::ReceiverPolicy::admit(self, subject, kid)
    }
}
fn approval_key() -> ApprovalKey {
    use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
    ApprovalKey(approvals::ConfiguredApprovalKey {
        issuer: "fixture-issuer".into(),
        audience: "approval:fixture-google-gmail-write".into(),
        kid: "fixture-key".into(),
        public_key: URL_SAFE_NO_PAD.encode(hex::decode(APPROVAL_PUBLIC).unwrap()),
        not_before_unix_ms: 0,
        not_after_unix_ms: 4_000_000_000_000,
        revoked: false,
    })
}
/// The approval subject for `input` to `operation` on this child, with the
/// input digest the owner's issuance computes (`connectors_core::digest`).
fn subject(child: &Child, operation: &str, input: &Value) -> approvals::Subject {
    let bootstrap = child.bootstrap();
    let descriptor = bootstrap.descriptor().unwrap();
    let declared = descriptor.operation(operation).unwrap();
    approvals::Subject {
        format: "connectors.approval-subject/v1".into(),
        target: approvals::Target {
            instance: bootstrap.instance.clone(),
            operation: operation.into(),
            connection: "fixture-connection".into(),
            connection_revision: "fixture-connection-revision".into(),
            contract: declared.contract.clone(),
            profile: declared.profile.clone(),
            descriptor_revision: descriptor.revision.clone(),
            configuration_revision: bootstrap.configuration_revision.clone(),
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
            authority_ref: bootstrap.instance.clone(),
        },
        route: None,
        canonicalization: "adapter-v1-canonical-json".into(),
        input_sha256: connectors_core::digest(input),
        approval_mode: "required".into(),
    }
}

/// An approval binds the whole input: a proof issued for one message does not
/// verify for another, and a proof issued to send the draft's current message
/// does not verify for a send pinned to another one. The proof verifies for
/// the input it was issued for, and that write sends exactly its body.
#[test]
fn a_write_approved_for_a_different_input_is_refused() {
    let provider = Provider::writer();
    let mut child = Child::spawn(&provider.write_selection()).unwrap();
    let key = approval_key();
    let signer = approvals::Signer::from_seed(
        Secret(hex::decode(APPROVAL_SEED).unwrap()),
        "fixture-key".into(),
    )
    .unwrap();
    for (operation, approved) in write_inputs() {
        let proof = signer
            .issue(&subject(&child, operation, &approved), &key, &Now)
            .unwrap();
        let changed = other_input(&approved);
        assert_ne!(changed, approved);
        let before = provider.requests().len();
        assert!(
            matches!(
                approvals::verify(&proof, &subject(&child, operation, &changed), &key, &Now),
                Err(approvals::Failure::Refused)
            ),
            "`{operation}` approval verified for a different input"
        );
        assert_eq!(provider.requests().len(), before, "`{operation}` sent");
        approvals::verify(&proof, &subject(&child, operation, &approved), &key, &Now)
            .unwrap_or_else(|failure| panic!("`{operation}` approval refused: {failure:?}"));
        let result = write(&mut child, operation, &approved).unwrap();
        assert_eq!(result.effect, WriteEffect::Applied, "`{operation}`");
        let bodies = provider.bodies();
        assert_eq!(bodies.last().unwrap().1, approved["body"], "`{operation}`");
    }
    // One POST per approved input, and none for an input that was not approved.
    assert_eq!(provider.bodies().len(), 2);
}

/// A changed draft sends nothing. The send reads the draft named by `body.id`
/// once and, when its `message.id` is not the pinned `messageId` — the draft
/// was edited after the issuer read it — refuses with no POST. An input that
/// pins no message, or names no draft, is refused before any Gmail request.
/// A draft that no longer exists is refused after its preflight read. The
/// draft's current message passes, and the one POST to `/drafts/send` carries
/// the body unchanged, without `messageId`.
#[test]
fn users_drafts_send_of_a_changed_draft_fails_the_preflight_and_sends_nothing() {
    let provider = Provider::writer();
    let mut child = Child::spawn(&provider.write_selection()).unwrap();
    let operation = "users.drafts.send";
    let [_, (_, current)] = write_inputs();
    let draft = "/gmail/v1/users/me/drafts/fixture-draft-1?";

    let mut stale = current.clone();
    stale["messageId"] = json!(STALE_MESSAGE);
    assert!(
        matches!(
            write(&mut child, operation, &stale),
            Err(Failure::Forbidden)
        ),
        "a changed draft was not refused in preflight"
    );
    assert_eq!(provider.api_targets(), [draft]);
    assert!(provider.bodies().is_empty(), "a changed draft was sent");

    let mut unpinned = current.clone();
    unpinned.as_object_mut().unwrap().remove("messageId");
    let mut unnamed = current.clone();
    unnamed["body"] = json!({});
    for input in [unpinned, unnamed] {
        assert!(
            matches!(
                write(&mut child, operation, &input),
                Err(Failure::InvalidInput)
            ),
            "{input} was not refused"
        );
    }
    assert_eq!(provider.api_targets(), [draft], "an unpinned send read");
    assert!(provider.bodies().is_empty(), "a write was sent");

    let mut gone = current.clone();
    gone["body"]["id"] = json!("fixture-draft-gone");
    assert!(
        matches!(write(&mut child, operation, &gone), Err(Failure::Forbidden)),
        "a missing draft was not refused"
    );
    assert_eq!(
        provider.api_targets()[1..],
        ["/gmail/v1/users/me/drafts/fixture-draft-gone?"]
    );
    assert!(provider.bodies().is_empty(), "a write was sent");

    let result = write(&mut child, operation, &current).unwrap();
    assert_eq!(result.effect, WriteEffect::Applied);
    let value = result.result.unwrap();
    assert_eq!(value["status"], 200);
    assert_eq!(
        value["body"],
        posted("/gmail/v1/users/me/drafts/send").unwrap()
    );
    assert_eq!(
        provider.api_targets()[2..],
        [draft, "/gmail/v1/users/me/drafts/send?"]
    );
    assert_eq!(
        provider.bodies(),
        [(
            "/gmail/v1/users/me/drafts/send?".to_owned(),
            json!({"id": DRAFT})
        )]
    );
    let (method, _, authorization) = provider.requests().pop().unwrap();
    assert_eq!(method, "POST");
    assert!(authorization.as_deref() == Some(&*format!("Bearer {ACCESS_TOKEN}")));
}

/// The send's body is closed to `id`: a `body.message`, which Google would
/// send in place of the draft the issuer read, or any other key, is refused as
/// `invalid_input` with no Gmail request, even when `messageId` pins the
/// draft's current message. The declaration says so to every caller.
#[test]
fn users_drafts_send_refuses_a_body_beyond_the_draft_id() {
    let selection = shipped()
        .into_iter()
        .find(|s| s.id == "users.drafts.send")
        .unwrap();
    assert_eq!(selection.body_keys, ["id"]);
    let create = shipped()
        .into_iter()
        .find(|s| s.id == "users.drafts.create")
        .unwrap();
    assert!(create.body_keys.is_empty(), "a create's body stays open");
    let send = engine()
        .declarations(&[Effect::Write])
        .into_iter()
        .find(|o| o.id == "users.drafts.send")
        .unwrap();
    assert_eq!(
        send.input_schema["properties"]["body"],
        json!({"type": "object",
               "properties": {"id": {"type": ["string", "integer", "boolean"]}},
               "required": ["id"], "additionalProperties": false})
    );
    let provider = Provider::writer();
    let mut child = Child::spawn(&provider.write_selection()).unwrap();
    let [_, (operation, current)] = write_inputs();
    for extra in [
        json!({"message": {"raw": RAW}}),
        json!({"message": {"id": DRAFT_MESSAGE}}),
        json!({"threadId": "fixture-thread-1"}),
    ] {
        let mut input = current.clone();
        for (key, value) in extra.as_object().unwrap() {
            input["body"][key] = value.clone();
        }
        assert!(
            matches!(
                write(&mut child, operation, &input),
                Err(Failure::InvalidInput)
            ),
            "{input} was not refused"
        );
    }
    assert!(provider.requests().is_empty(), "a request was sent");
    assert!(provider.bodies().is_empty(), "a write was sent");
}

/// `users.drafts.create` has no preflight: one POST to `/drafts` with the
/// message body unchanged, and the created draft — its `id` and `message.id`,
/// what a send pins — returned.
#[test]
fn users_drafts_create_sends_one_post_with_its_body_and_no_preflight() {
    let provider = Provider::writer();
    let mut child = Child::spawn(&provider.write_selection()).unwrap();
    let [(operation, input), _] = write_inputs();
    let result = write(&mut child, operation, &input).unwrap();
    assert_eq!(result.effect, WriteEffect::Applied);
    let value = result.result.unwrap();
    assert_eq!(value["body"], posted("/gmail/v1/users/me/drafts").unwrap());
    assert!(value["body"]["id"].is_string());
    assert!(value["body"]["message"]["id"].is_string());
    assert_eq!(provider.api_targets(), ["/gmail/v1/users/me/drafts?"]);
    assert_eq!(
        provider.bodies(),
        [(
            "/gmail/v1/users/me/drafts?".to_owned(),
            input["body"].clone()
        )]
    );
}

/// The guide's write instance names the compose scope in `minimum_scopes`: a
/// refresh token granted only the read-only scope refuses validation as
/// insufficient scope, and one granted the compose scope validates. The
/// read-only grant still validates the guide's read configuration.
#[test]
fn gmail_write_config_requires_compose_scope() {
    let deadline = || connectors_sdk::now_ms() + 30_000;
    let read_only = Provider::with(documented_write_config(), GMAIL_SCOPE);
    let mut child = Child::spawn(&read_only.selection()).unwrap();
    assert!(
        matches!(
            child.validate(PROFILE, &secret(), deadline()),
            Err(Failure::InsufficientScope)
        ),
        "a read-only grant validated the write instance"
    );
    assert_eq!(read_only.requests().len(), 1, "one token exchange");
    assert_eq!(read_only.requests()[0].1, "/token");

    let compose = Provider::writer();
    let mut child = Child::spawn(&compose.selection()).unwrap();
    child.validate(PROFILE, &secret(), deadline()).unwrap();

    let reads = Provider::new();
    let mut child = Child::spawn(&reads.selection()).unwrap();
    child.validate(PROFILE, &secret(), deadline()).unwrap();
}

/// The guide's route to a write-capable connection: its write instance,
/// `google-gmail-write`, starts its own acquisition beside an existing
/// read-only connection. The guide's reason repair cannot widen the read
/// connection holds: the compose scope changes the configuration revision and
/// the profile, both part of the binding, and the read instance reconfigured
/// for writes is refused a new acquisition while the read one exists.
#[test]
fn the_documented_write_instance_starts_a_fresh_acquisition() {
    let scratch = tempfile::tempdir().unwrap();
    let directory = scratch.path().join("private");
    filesystem::directory(&directory, true, true).unwrap();
    let bootstrap = |name: &str, mut config: Value| {
        config["bundle_directory"] = json!(root_path("generated/bundles"));
        config["operations_file"] = json!(root_path("providers/google-gmail/operations.json"));
        let path = directory.join(name);
        private(&path, &serde_json::to_vec(&config).unwrap());
        print_bootstrap(&path).binding(PROFILE).unwrap()
    };
    let read = bootstrap("read.json", documented_config());
    let write = bootstrap("write.json", documented_write_config());
    let mut widened = documented_write_config();
    widened["instance"] = documented_config()["instance"].clone();
    let widened = bootstrap("widened.json", widened);

    let state = scratch.path().join("state");
    fs::create_dir(&state).unwrap();
    fs::set_permissions(&state, fs::Permissions::from_mode(0o700)).unwrap();
    filesystem::directory(&state, false, true).unwrap();
    drop(Metadata::initialize(&state).unwrap());
    let registry = registry::Registry::new(&state);
    let now = connectors_sdk::now_ms();
    registry.begin(&read, now).unwrap();
    assert_eq!(write.instance_id, WRITE_INSTANCE);
    assert_ne!(write.instance_id, read.instance_id);
    registry
        .begin(&write, now + 1)
        .unwrap_or_else(|failure| panic!("the write instance was refused: {failure:?}"));
    assert_eq!(widened.instance_id, read.instance_id);
    let read = serde_json::to_value(&read).unwrap();
    let widened_value = serde_json::to_value(&widened).unwrap();
    for field in ["configuration_revision", "profile"] {
        assert_ne!(read[field], widened_value[field], "`{field}` unchanged");
    }
    assert!(
        registry.begin(&widened, now + 2).is_err(),
        "the read instance reconfigured for writes started a second acquisition"
    );
}
