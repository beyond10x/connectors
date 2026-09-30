//! Adversary pass 2 against `body_keys` and the Gmail draft send it closes:
//! what the declaration promises against what the engine does, every body
//! shape on the send's write path, and the load-time refusals the guide lists.
//! No network, no credential.
use connectors_catalog::bundle;
use connectors_catalog_provider::{Effect, Engine, Selection};
use connectors_core::{ErrorCode, Result};
use connectors_sdk::{AuthenticatedHttp, HttpResponse};
use serde_json::{Value, json};
use std::{path::Path, sync::Mutex};

const DRAFT: &str = "fixture-draft-1";
const DRAFT_MESSAGE: &str = "fixture-draft-message-2";
/// `To: other@example.test`, a body `Other`: a message nobody read.
const UNREAD_RAW: &str = "VG86IG90aGVyQGV4YW1wbGUudGVzdA0KDQpPdGhlcg0K";

/// Records every read; answers none, so any read is visible as a call.
#[derive(Default)]
struct NoReads {
    calls: Mutex<Vec<Vec<String>>>,
}
#[async_trait::async_trait]
impl AuthenticatedHttp for NoReads {
    async fn get(&self, path: &[&str], _query: &[(&str, String)]) -> Result<HttpResponse> {
        self.calls
            .lock()
            .unwrap()
            .push(path.iter().map(|s| s.to_string()).collect());
        Ok(HttpResponse {
            status: 404,
            headers: Default::default(),
            body: Vec::new(),
        })
    }
}

fn root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
}
fn shipped() -> Vec<Selection> {
    let file: Value = serde_json::from_slice(
        &std::fs::read(root().join("providers/google-gmail/operations.json")).unwrap(),
    )
    .unwrap();
    serde_json::from_value(file["operations"].clone()).unwrap()
}
fn gmail(selections: &[Selection]) -> Result<Engine> {
    let bundle = bundle::load(&root().join("generated/bundles"), "google-gmail").unwrap();
    Engine::new(&bundle, "", selections)
}
fn send_declaration() -> connectors_core::Operation {
    gmail(&shipped())
        .unwrap()
        .declarations(&[Effect::Write])
        .into_iter()
        .find(|o| o.id == "users.drafts.send")
        .unwrap()
}

/// The declaration is what `operations describe` shows and what the owner's
/// approval issuance validates against (`validate_write_value`). The engine
/// refuses a `body.id` that is not a scalar before any request, because the
/// guard reads it as one; a top-level guard reference is declared as a scalar
/// for that reason (`declared_type(None)` in `declare`). The closed body
/// declares the guarded `id` as `{}`, any JSON value, so the owner validates
/// and an issuer approves inputs the provider can only refuse — among them a
/// `body.id` carrying the replacement message the closed body exists to keep
/// out.
#[tokio::test]
async fn the_send_declares_its_guarded_draft_id_as_the_scalar_the_engine_accepts() {
    let engine = gmail(&shipped()).unwrap();
    let schema = send_declaration().input_schema;
    let mut admitted = Vec::new();
    for id in [
        json!({"message": {"raw": UNREAD_RAW}}),
        json!(null),
        json!([DRAFT]),
    ] {
        let input = json!({"userId": "me", "messageId": DRAFT_MESSAGE, "body": {"id": id}});
        let http = NoReads::default();
        let error = engine
            .prepare(&http, "fixture", "users.drafts.send", input.clone())
            .await
            .err()
            .unwrap_or_else(|| panic!("{input} was prepared"));
        assert_eq!(error.code, ErrorCode::InvalidInput, "{input}");
        assert!(http.calls.lock().unwrap().is_empty(), "{input} was read");
        if connectors_sdk::validate_write_value(&schema, &input).is_ok() {
            admitted.push(input);
        }
    }
    assert!(
        admitted.is_empty(),
        "the send declaration admits {} input(s) the engine refuses: {:?}; declared body: {}",
        admitted.len(),
        admitted,
        schema["properties"]["body"]
    );
}

/// Every other body shape on the send is refused as `invalid_input` with no
/// Gmail request: absent, null, empty, a key differing only in case, a key
/// beside `id` differing only in case, a scalar body and a second top-level
/// `Body`.
#[tokio::test]
async fn every_body_shape_but_the_draft_id_is_refused_before_any_request() {
    let engine = gmail(&shipped()).unwrap();
    let base = json!({"userId": "me", "messageId": DRAFT_MESSAGE});
    let mut inputs = vec![base.clone()];
    for body in [
        json!(null),
        json!({}),
        json!({"ID": DRAFT}),
        json!({"id": DRAFT, "Id": DRAFT}),
        json!({"id": DRAFT, "message ": {"raw": UNREAD_RAW}}),
        json!(DRAFT),
    ] {
        let mut input = base.clone();
        input["body"] = body;
        inputs.push(input);
    }
    let mut shadowed = base.clone();
    shadowed["body"] = json!({"id": DRAFT});
    shadowed["Body"] = json!({"message": {"raw": UNREAD_RAW}});
    inputs.push(shadowed);
    for input in inputs {
        let http = NoReads::default();
        let error = engine
            .prepare(&http, "fixture", "users.drafts.send", input.clone())
            .await
            .err()
            .unwrap_or_else(|| panic!("{input} was prepared"));
        assert_eq!(error.code, ErrorCode::InvalidInput, "{input}");
        assert!(http.calls.lock().unwrap().is_empty(), "{input} was read");
    }
}

/// The guide lists "an operation without a request body" among the
/// selections refused at load. No shipped test loads a write without one, so
/// the `request_media_types` clause of that refusal could be dropped with the
/// suite green: a `users.drafts.delete` closed to `id` would then load, and
/// every call to it be refused. Guarded here.
#[test]
fn a_write_without_a_request_body_cannot_close_one() {
    let selection: Selection = serde_json::from_value(json!({
        "id": "users.drafts.delete", "operation_id": "gmail.users.drafts.delete",
        "effect": "write", "body_keys": ["id"]
    }))
    .unwrap();
    let error = gmail(&[selection])
        .err()
        .expect("a bodiless write closed its body");
    assert_eq!(error.code, ErrorCode::InvalidInput);
    assert!(error.message.contains("body_keys"), "{}", error.message);
}

/// A postflight check reading a body key the set does not admit, and a
/// nested read under an admitted key, at load.
#[test]
fn a_postflight_reading_outside_the_set_is_refused_and_a_nested_read_is_not() {
    let send = |check: &str| -> Selection {
        serde_json::from_value(json!({
            "id": "users.drafts.send", "operation_id": "gmail.users.drafts.send",
            "effect": "write", "body_keys": ["id"],
            "guard": {
                "preflight": {"operation_id": "gmail.users.drafts.get",
                              "values": {"userId": "userId", "id": "body.id"},
                              "checks": [{"pointer": "/message/id", "expect": {"input": "messageId"}}]},
                "postflight": {"checks": [{"pointer": "/id", "expect": {"input": check}}]}}
        }))
        .unwrap()
    };
    let error = gmail(&[send("body.message.id")])
        .err()
        .expect("a postflight reading body.message loaded");
    assert!(error.message.contains("body_keys"), "{}", error.message);
    gmail(&[send("body.id.x")]).expect("a nested read under an admitted key");
}
