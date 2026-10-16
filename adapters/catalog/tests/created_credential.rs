//! A write whose answer carries a credential it created, shown once
//! (`connectors_catalog.selection.Custody`, design §8a): the engine reads the
//! secret out of the answer into protected memory, builds the result from
//! `returns` alone, and declares an output schema that names nothing of the
//! provider's answer.
use connectors_catalog::{bundle::Bundle, ingest, inventory};
use connectors_catalog_provider::{Effect, Engine, Selection};
use connectors_core::{ErrorCode, Result};
use connectors_sdk::{
    AuthenticatedHttp, AuthenticatedWrite, HttpResponse, WriteMethod, WriteOutcome,
};
use serde_json::{Value, json};
use std::sync::{Arc, Mutex};

/// Fictional created material; never a real key's shape.
const CREATED: &str = "fixture-created-material-one";

fn document() -> Vec<u8> {
    serde_json::to_vec(&json!({
        "openapi": "3.0.0",
        "info": {"title": "fixture", "version": "1"},
        "paths": {
            "/api/v1/keys": {
                "post": {
                    "operationId": "createKey",
                    "requestBody": {"required": true, "content": {"application/json": {"schema": {"type": "object"}}}},
                    "responses": {"201": {"description": "Created", "content": {"application/json": {}}}}
                },
                "get": {
                    "operationId": "listKeys",
                    "responses": {"200": {"description": "OK", "content": {"application/json": {}}}}
                }
            }
        }
    }))
    .unwrap()
}

fn bundle() -> Bundle {
    let bytes = document();
    let source = ingest("fixture.json", &bytes).unwrap();
    let document: Value = serde_json::from_slice(&bytes).unwrap();
    Bundle {
        provider: "fixture".into(),
        source,
        inventory: inventory::extract(&document),
        auth_profile: "fixture.token".into(),
    }
}

fn selection(value: Value) -> Selection {
    serde_json::from_value(value).unwrap()
}

fn create(custody: Value) -> Selection {
    selection(
        json!({"id": "key.create", "operation_id": "createKey", "effect": "write",
        "body_keys": ["name"], "custody": custody}),
    )
}

fn custody() -> Value {
    json!({"secret": "/key", "returns": {"hash": "/data/hash"}})
}

fn engine(selections: &[Selection]) -> Result<Engine> {
    Engine::new(&bundle(), "/api/v1", selections)
}

struct NoReads;
#[async_trait::async_trait]
impl AuthenticatedHttp for NoReads {
    async fn get(&self, _: &[&str], _: &[(&str, String)]) -> Result<HttpResponse> {
        panic!("a write that creates a credential reads nothing")
    }
}

type Sent = Arc<Mutex<Vec<(WriteMethod, Vec<String>, Value)>>>;
struct Send {
    sent: Sent,
    response: Option<HttpResponse>,
}
#[async_trait::async_trait]
impl AuthenticatedWrite for Send {
    async fn send_json(
        self: Box<Self>,
        method: WriteMethod,
        path: &[&str],
        _: &[(&str, String)],
        body: &Value,
    ) -> Result<HttpResponse> {
        self.sent.lock().unwrap().push((
            method,
            path.iter().map(|s| s.to_string()).collect(),
            body.clone(),
        ));
        self.response
            .ok_or_else(connectors_core::Error::unavailable)
    }
}

fn answer(status: u16, value: Value) -> Option<HttpResponse> {
    Some(HttpResponse {
        status,
        headers: Default::default(),
        body: serde_json::to_vec(&value).unwrap(),
    })
}

fn carries(haystack: &[u8], needle: &str) -> bool {
    haystack
        .windows(needle.len())
        .any(|window| window == needle.as_bytes())
}

async fn send(
    engine: &Engine,
    response: Option<HttpResponse>,
) -> (WriteOutcome<connectors_catalog_provider::Held>, Sent) {
    let prepared = engine
        .prepare(
            &NoReads,
            "fixture",
            "key.create",
            json!({"body": {"name": "tenant"}}),
        )
        .await
        .unwrap();
    let sent = Sent::default();
    let outcome = prepared
        .execute_holding(
            Box::new(Send {
                sent: sent.clone(),
                response,
            }),
            None,
        )
        .await;
    (outcome, sent)
}

#[test]
fn custody_loads_only_on_a_plain_write_with_bounded_returns() {
    assert!(engine(&[create(custody())]).is_ok());
    let refused = |selection: Selection| {
        let error = engine(&[selection]).err().expect("refused when it loads");
        assert_eq!(error.code, ErrorCode::InvalidInput);
        error.message
    };
    // On a read.
    let read = selection(
        json!({"id": "keys.list", "operation_id": "listKeys", "effect": "read",
        "custody": custody()}),
    );
    assert!(refused(read).contains("custody"));
    // Beside a guard.
    let guarded = selection(
        json!({"id": "key.create", "operation_id": "createKey", "effect": "write",
        "custody": custody(),
        "guard": {"preflight": {"operation_id": "listKeys", "values": {}, "checks": []},
                  "postflight": {"checks": []}}}),
    );
    assert!(refused(guarded).contains("custody"));
    // Pointers without a leading `/`, an empty secret pointer.
    for written in [
        json!({"secret": "key", "returns": {"hash": "/hash"}}),
        json!({"secret": "", "returns": {"hash": "/hash"}}),
        json!({"secret": "/key", "returns": {"hash": "hash"}}),
        // The owner's own members, and a name that is no id.
        json!({"secret": "/key", "returns": {"custody": "/hash"}}),
        json!({"secret": "/key", "returns": {"connection": "/hash"}}),
        json!({"secret": "/key", "returns": {"not a name": "/hash"}}),
        // Nothing returned, or more than eight members.
        json!({"secret": "/key", "returns": {}}),
        json!({"secret": "/key", "returns": (0..9).map(|i| (format!("m{i}"), json!(format!("/m{i}")))).collect::<serde_json::Map<_, _>>()}),
        // A returned member read from the secret itself.
        json!({"secret": "/key", "returns": {"hash": "/key"}}),
    ] {
        assert!(
            refused(create(written.clone())).contains("custody"),
            "{written}"
        );
    }
    // An unknown member of the custody table does not decode.
    assert!(
        serde_json::from_value::<Selection>(json!({"id": "key.create", "operation_id": "createKey",
            "effect": "write", "custody": {"secret": "/key", "returns": {"hash": "/hash"}, "extra": 1}}))
        .is_err()
    );
}

#[test]
fn the_output_schema_names_the_returns_and_the_owners_members_only() {
    let engine = engine(&[create(custody())]).unwrap();
    let declared = engine.declarations(&[Effect::Write]);
    let schema = &declared[0].output_schema;
    assert_eq!(
        schema,
        &json!({
            "type": "object",
            "properties": {
                "custody": {"type": "string", "enum": ["stored"]},
                "connection": {"type": "string", "minLength": 1},
                "hash": {"type": "string"}
            },
            "required": ["hash"],
            "additionalProperties": false
        })
    );
    // A selection without custody keeps the generic result.
    let plain =
        selection(json!({"id": "key.create", "operation_id": "createKey", "effect": "write"}));
    let declared = self::engine(&[plain])
        .unwrap()
        .declarations(&[Effect::Write]);
    assert!(declared[0].output_schema["properties"]["body"].is_object());
}

#[tokio::test]
async fn the_secret_leaves_the_answer_and_the_result_holds_only_the_returns() {
    let engine = engine(&[create(custody())]).unwrap();
    let (outcome, sent) = send(
        &engine,
        answer(
            201,
            json!({"key": CREATED, "data": {"hash": "fixture-hash-one", "name": "tenant"}}),
        ),
    )
    .await;
    assert_eq!(sent.lock().unwrap().len(), 1);
    let WriteOutcome::Applied(Ok(held)) = outcome else {
        panic!("not applied");
    };
    assert_eq!(held.value, json!({"hash": "fixture-hash-one"}));
    assert!(!carries(&serde_json::to_vec(&held.value).unwrap(), CREATED));
    assert_eq!(held.secret.as_ref().unwrap().0, CREATED.as_bytes());
}

#[tokio::test]
async fn an_answer_the_engine_cannot_read_is_applied_and_names_no_secret() {
    let engine = engine(&[create(custody())]).unwrap();
    for body in [
        json!({"data": {"hash": "fixture-hash-one"}}),
        json!({"key": 7, "data": {"hash": "fixture-hash-one"}}),
        json!({"key": "", "data": {"hash": "fixture-hash-one"}}),
        json!({"key": CREATED, "data": {}}),
        json!({"key": CREATED, "data": {"hash": 7}}),
        // A returned value carrying the secret's bytes.
        json!({"key": CREATED, "data": {"hash": format!("h-{CREATED}")}}),
    ] {
        let (outcome, sent) = send(&engine, answer(201, body.clone())).await;
        assert_eq!(sent.lock().unwrap().len(), 1, "{body}");
        let WriteOutcome::Applied(Err(error)) = outcome else {
            panic!("not an applied, unreadable write: {body}");
        };
        assert_eq!(error.code, ErrorCode::UpstreamProtocol, "{body}");
        assert!(!error.message.contains(CREATED), "{body}");
    }
}

#[tokio::test]
async fn a_refusal_and_an_unconfirmed_answer_carry_no_custody() {
    let engine = engine(&[create(custody())]).unwrap();
    let (outcome, _) = send(
        &engine,
        answer(422, json!({"error": {"message": "bad name"}})),
    )
    .await;
    assert!(matches!(outcome, WriteOutcome::Refused(_)));
    let (outcome, _) = send(&engine, answer(503, json!({"key": CREATED}))).await;
    assert!(matches!(outcome, WriteOutcome::Unknown(_)));
    let (outcome, sent) = send(&engine, None).await;
    assert!(matches!(outcome, WriteOutcome::Unknown(_)));
    assert_eq!(sent.lock().unwrap().len(), 1);
}

#[tokio::test]
async fn a_custody_write_is_never_sent_through_the_path_that_would_return_its_answer() {
    let engine = engine(&[create(custody())]).unwrap();
    let prepared = engine
        .prepare(
            &NoReads,
            "fixture",
            "key.create",
            json!({"body": {"name": "tenant"}}),
        )
        .await
        .unwrap();
    let sent = Sent::default();
    let outcome = prepared
        .execute_reading(
            Box::new(Send {
                sent: sent.clone(),
                response: answer(201, json!({"key": CREATED, "data": {"hash": "h"}})),
            }),
            None,
        )
        .await;
    assert!(matches!(outcome, WriteOutcome::Refused(_)));
    assert!(sent.lock().unwrap().is_empty());
}

/// The engine names the operations that create a credential, so the child's
/// bootstrap can tell the owner, which refuses one with no custody target
/// before it is sent (design §8a, "Admit").
#[test]
fn the_engine_names_exactly_the_operations_that_declare_custody() {
    let list = selection(json!({"id": "key.list", "operation_id": "listKeys", "effect": "read"}));
    let engine = engine(&[create(custody()), list]).unwrap();
    assert!(engine.declares_custody("key.create"));
    assert!(!engine.declares_custody("key.list"));
    assert!(!engine.declares_custody("key.absent"));
    let plain =
        selection(json!({"id": "key.create", "operation_id": "createKey", "effect": "write"}));
    assert!(
        !self::engine(&[plain])
            .unwrap()
            .declares_custody("key.create")
    );
}

#[tokio::test]
async fn a_write_without_custody_holds_no_secret() {
    let plain =
        selection(json!({"id": "key.create", "operation_id": "createKey", "effect": "write"}));
    let engine = engine(&[plain]).unwrap();
    let (outcome, _) = send(&engine, answer(201, json!({"key": CREATED}))).await;
    let WriteOutcome::Applied(Ok(held)) = outcome else {
        panic!("not applied");
    };
    assert!(held.secret.is_none());
    assert_eq!(held.value["status"], 201);
    assert_eq!(held.value["body"], json!({"key": CREATED}));
}
