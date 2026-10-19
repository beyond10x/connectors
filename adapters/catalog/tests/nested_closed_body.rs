//! A write's closed body addressed by RFC 6901 pointer, and the members that
//! close, type, fix, enumerate, require one of and prefix its nested values
//! (`connectors_catalog.selection.ClosedBody`), and a declared conflict status
//! (`connectors_catalog.selection.ConflictStatus`), over a small fixture
//! shaped like Zendesk's ticket update: `PUT /api/v2/tickets/{ticket_id}` with
//! a `{"ticket": {...}}` body, guarded by `GET` of the same ticket.
//!
//! Each refusal is asserted with nothing sent: no preflight read and no write.
use connectors_catalog::{bundle::Bundle, ingest, inventory};
use connectors_catalog_provider::{Effect, Engine, Selection};
use connectors_core::{ErrorCode, Result};
use connectors_sdk::{
    AuthenticatedHttp, AuthenticatedWrite, HttpResponse, WriteMethod, WriteOutcome,
};
use serde_json::{Value, json};
use std::{
    collections::VecDeque,
    sync::{Arc, Mutex},
};

const STAMP: &str = "2026-10-16T10:00:00Z";
const MARKER: &str = "[Draft reply - not sent to the requester]\n";

fn document() -> Vec<u8> {
    let ticket_id =
        json!({"name": "ticket_id", "in": "path", "required": true, "schema": {"type": "integer"}});
    serde_json::to_vec(&json!({
        "openapi": "3.0.0",
        "info": {"title": "fixture", "version": "1"},
        "paths": {
            "/api/v2/tickets/{ticket_id}": {
                "get": {
                    "operationId": "ShowTicket",
                    "parameters": [ticket_id],
                    "responses": {"200": {"description": "OK", "content": {"application/json": {}}}}
                },
                "put": {
                    "operationId": "UpdateTicket",
                    "parameters": [ticket_id],
                    "requestBody": {"content": {"application/json": {"schema": {"type": "object"}}}},
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
        auth_profile: "fixture.basic".into(),
    }
}

fn written(value: Value) -> Selection {
    serde_json::from_value(value.clone()).unwrap_or_else(|error| panic!("{error}: {value}"))
}

/// The guard every ticket write carries: the caller's stamp must equal the
/// ticket's `updated_at` before the write is sent, and the answer names the
/// ticket.
fn guard() -> Value {
    json!({
        "preflight": {"operation_id": "ShowTicket", "values": {"ticket_id": "ticket_id"},
                      "checks": [{"pointer": "/ticket/updated_at",
                                  "expect": {"input": "body.ticket.updated_stamp"}}],
                      "mismatch": "conflict"},
        "postflight": {"checks": [{"pointer": "/ticket/id", "expect": {"input": "ticket_id"}}]}
    })
}

/// A ticket update shaped like the shipped `ticket.update`.
fn update() -> Value {
    json!({
        "id": "ticket.update", "operation_id": "UpdateTicket", "effect": "write",
        "body_keys": ["/ticket/group_id", "/ticket/tags", "/ticket/priority", "/ticket/type",
                      "/ticket/updated_stamp", "/ticket/safe_update"],
        "body_types": {"/ticket/group_id": "integer", "/ticket/tags": "string_list",
                       "/ticket/priority": "string", "/ticket/type": "string",
                       "/ticket/updated_stamp": "string"},
        "body_enums": {"/ticket/priority": ["urgent", "high", "normal", "low"],
                       "/ticket/type": ["problem", "incident", "question", "task"]},
        "body_any_of": ["/ticket/group_id", "/ticket/tags", "/ticket/priority", "/ticket/type"],
        "body_fixed": {"/ticket/safe_update": true},
        "conflict_status": 409,
        "guard": guard()
    })
}

/// A private note shaped like the shipped `ticket.draft_reply`.
fn draft() -> Value {
    json!({
        "id": "ticket.draft_reply", "operation_id": "UpdateTicket", "effect": "write",
        "body_keys": ["/ticket/comment/body", "/ticket/comment/public",
                      "/ticket/updated_stamp", "/ticket/safe_update"],
        "body_types": {"/ticket/comment/body": "string", "/ticket/updated_stamp": "string"},
        "body_required": ["/ticket/comment/body"],
        "body_nonempty": ["/ticket/comment/body"],
        "body_fixed": {"/ticket/comment/public": false, "/ticket/safe_update": true},
        "body_prefixes": {"/ticket/comment/body": MARKER},
        "conflict_status": 409,
        "guard": guard()
    })
}

fn engine(selections: &[Value]) -> Result<Engine> {
    let selections: Vec<Selection> = selections.iter().cloned().map(written).collect();
    Engine::new(&bundle(), "/", &selections)
}

type Call = (Vec<String>, Vec<(String, String)>);
struct Reads {
    responses: Mutex<VecDeque<HttpResponse>>,
    calls: Mutex<Vec<Call>>,
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
        Ok(self
            .responses
            .lock()
            .unwrap()
            .pop_front()
            .expect("unexpected provider read"))
    }
}
fn reads(responses: Vec<HttpResponse>) -> Arc<Reads> {
    Arc::new(Reads {
        responses: Mutex::new(responses.into()),
        calls: Mutex::new(Vec::new()),
    })
}
fn response(status: u16, value: Value) -> HttpResponse {
    HttpResponse {
        status,
        headers: Default::default(),
        body: serde_json::to_vec(&value).unwrap(),
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
        _query: &[(&str, String)],
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

fn ticket(updated_at: &str) -> Value {
    json!({"ticket": {"id": 101, "updated_at": updated_at, "priority": "normal"}})
}

/// Prepare against a preflight answering `updated_at`, then send once with
/// the provider answering `answer`; the body sent and the outcome.
async fn send(
    engine: &Engine,
    id: &str,
    input: Value,
    answer: HttpResponse,
) -> (Vec<Value>, WriteOutcome<Value>) {
    let http = reads(vec![response(200, ticket(STAMP))]);
    let prepared = engine
        .prepare(http.as_ref(), "fixture", id, input)
        .await
        .unwrap_or_else(|error| panic!("`{id}` refused in prepare: {error:?}"));
    let sent: Sent = Arc::default();
    let outcome = prepared
        .execute(Box::new(Send {
            sent: sent.clone(),
            response: Some(answer),
        }))
        .await;
    let bodies = sent
        .lock()
        .unwrap()
        .iter()
        .map(|(method, path, body)| {
            assert_eq!(*method, WriteMethod::Put);
            assert_eq!(path, &["api", "v2", "tickets", "101"]);
            body.clone()
        })
        .collect();
    (bodies, outcome)
}

/// Prepare an input that must be refused before anything is sent: no read.
async fn refused(engine: &Engine, id: &str, input: Value) -> connectors_core::Error {
    let http = reads(vec![]);
    let error = match engine
        .prepare(http.as_ref(), "fixture", id, input.clone())
        .await
    {
        Ok(_) => panic!("`{id}` admitted {input}"),
        Err(error) => error,
    };
    assert!(
        http.calls.lock().unwrap().is_empty(),
        "`{id}` read before refusing {input}"
    );
    error
}

fn code(error: &connectors_core::Error) -> String {
    serde_json::to_value(&error.code)
        .unwrap()
        .as_str()
        .unwrap()
        .to_owned()
}

#[test]
fn pointer_keys_declare_nested_closed_objects_without_the_fixed_values() {
    let engine = engine(&[update(), draft()]).unwrap();
    let declared = |id: &str| {
        engine
            .declarations(&[Effect::Write])
            .into_iter()
            .find(|o| o.id == id)
            .unwrap()
            .input_schema["properties"]["body"]
            .clone()
    };
    let body = declared("ticket.update");
    assert_eq!(body["type"], "object");
    assert_eq!(body["additionalProperties"], false);
    assert_eq!(body["required"], json!(["ticket"]));
    let ticket = &body["properties"]["ticket"];
    assert_eq!(ticket["type"], "object");
    assert_eq!(ticket["additionalProperties"], false);
    let mut names: Vec<&str> = ticket["properties"]
        .as_object()
        .unwrap()
        .keys()
        .map(String::as_str)
        .collect();
    names.sort();
    // `safe_update` is fixed: the selection's, never the caller's.
    assert_eq!(
        names,
        ["group_id", "priority", "tags", "type", "updated_stamp"]
    );
    assert_eq!(ticket["properties"]["group_id"], json!({"type": "integer"}));
    assert_eq!(
        ticket["properties"]["tags"],
        json!({"type": "array", "items": {"type": "string"}})
    );
    assert_eq!(
        ticket["properties"]["priority"],
        json!({"type": "string", "enum": ["urgent", "high", "normal", "low"]})
    );
    // The guard reads the stamp, so it is required where it lives.
    assert_eq!(ticket["required"], json!(["updated_stamp"]));
    // At least one of the four changes.
    assert_eq!(
        ticket["anyOf"],
        json!([{"required": ["group_id"]}, {"required": ["tags"]},
               {"required": ["priority"]}, {"required": ["type"]}])
    );

    let body = declared("ticket.draft_reply");
    let ticket = &body["properties"]["ticket"];
    let comment = &ticket["properties"]["comment"];
    assert_eq!(comment["additionalProperties"], false);
    assert_eq!(
        comment["properties"],
        json!({"body": {"type": "string", "minLength": 1}})
    );
    assert_eq!(comment["required"], json!(["body"]));
    assert_eq!(ticket["required"], json!(["comment", "updated_stamp"]));
}

#[tokio::test]
async fn fixed_values_are_sent_at_their_pointers_and_the_prefix_leads_the_callers_text() {
    let engine = engine(&[update(), draft()]).unwrap();
    let (bodies, outcome) = send(
        &engine,
        "ticket.update",
        json!({"ticket_id": 101, "body": {"ticket": {"priority": "high", "tags": ["vip", "billing"],
                                                     "updated_stamp": STAMP}}}),
        response(200, ticket("2026-10-16T10:05:00Z")),
    )
    .await;
    assert_eq!(
        bodies,
        [
            json!({"ticket": {"priority": "high", "tags": ["vip", "billing"],
                           "updated_stamp": STAMP, "safe_update": true}})
        ]
    );
    assert!(matches!(outcome, WriteOutcome::Applied(Ok(_))));

    let (bodies, outcome) = send(
        &engine,
        "ticket.draft_reply",
        json!({"ticket_id": 101, "body": {"ticket": {"comment": {"body": "Hello,\nthanks."},
                                                     "updated_stamp": STAMP}}}),
        response(200, ticket("2026-10-16T10:05:00Z")),
    )
    .await;
    assert_eq!(
        bodies,
        [
            json!({"ticket": {"comment": {"body": format!("{MARKER}Hello,\nthanks."), "public": false},
                           "updated_stamp": STAMP, "safe_update": true}})
        ]
    );
    assert!(matches!(outcome, WriteOutcome::Applied(Ok(_))));
}

#[tokio::test]
async fn a_nested_body_outside_its_closed_shape_is_refused_before_any_request() {
    let engine = engine(&[update(), draft()]).unwrap();
    let ticket = |members: Value| {
        let mut members = members;
        members["updated_stamp"] = json!(STAMP);
        json!({"ticket_id": 101, "body": {"ticket": members}})
    };
    for (id, input, named) in [
        // A member of `ticket` the selection does not admit.
        (
            "ticket.update",
            ticket(json!({"priority": "high", "status": "solved"})),
            "",
        ),
        // A top-level key beside `ticket`.
        (
            "ticket.update",
            json!({"ticket_id": 101, "body": {"ticket": {"priority": "high", "updated_stamp": STAMP}, "system_metadata": {}}}),
            "",
        ),
        // An object above a key that is not an object.
        (
            "ticket.update",
            json!({"ticket_id": 101, "body": {"ticket": "high"}}),
            "",
        ),
        // A fixed nested value, with its own value or any other.
        (
            "ticket.update",
            ticket(json!({"priority": "high", "safe_update": true})),
            "safe_update",
        ),
        (
            "ticket.update",
            ticket(json!({"priority": "high", "safe_update": false})),
            "safe_update",
        ),
        (
            "ticket.draft_reply",
            ticket(json!({"comment": {"body": "x", "public": true}})),
            "public",
        ),
        (
            "ticket.draft_reply",
            ticket(json!({"comment": {"body": "x", "public": false}})),
            "public",
        ),
        // A value outside its enum names its key.
        (
            "ticket.update",
            ticket(json!({"priority": "critical"})),
            "priority",
        ),
        ("ticket.update", ticket(json!({"type": "bug"})), "type"),
        // None of the four changes.
        ("ticket.update", ticket(json!({})), ""),
        // A typed value of another type.
        (
            "ticket.update",
            ticket(json!({"group_id": "7"})),
            "group_id",
        ),
        ("ticket.update", ticket(json!({"tags": "vip"})), "tags"),
        ("ticket.update", ticket(json!({"tags": ["vip", 7]})), "tags"),
        // The note's text is required and not empty.
        ("ticket.draft_reply", ticket(json!({"comment": {}})), "body"),
        (
            "ticket.draft_reply",
            ticket(json!({"comment": {"body": ""}})),
            "body",
        ),
        // The guard's stamp is required.
        (
            "ticket.update",
            json!({"ticket_id": 101, "body": {"ticket": {"priority": "high"}}}),
            "",
        ),
    ] {
        let error = refused(&engine, id, input.clone()).await;
        assert_eq!(error.code, ErrorCode::InvalidInput, "`{id}` {input}");
        assert!(
            error.message.contains(named),
            "`{id}` {input}: {}",
            error.message
        );
    }
}

#[tokio::test]
async fn a_declared_conflict_status_is_a_conflict_refusal_and_is_not_retried() {
    let engine = engine(&[update()]).unwrap();
    let input =
        json!({"ticket_id": 101, "body": {"ticket": {"priority": "high", "updated_stamp": STAMP}}});
    let (bodies, outcome) = send(
        &engine,
        "ticket.update",
        input.clone(),
        response(
            409,
            json!({"error": "UpdateConflict", "description": "Safe Update prevented the update"}),
        ),
    )
    .await;
    assert_eq!(bodies.len(), 1, "sent once");
    match outcome {
        WriteOutcome::Refused(error) => {
            assert_eq!(code(&error), "conflict");
            assert!(error.upstream_answer, "the provider's own answer");
        }
        _ => panic!("a 409 under conflict_status is not refused"),
    }
    // Without the declaration a 409 stays the `invalid_input` it was.
    let mut plain = update();
    plain.as_object_mut().unwrap().remove("conflict_status");
    let engine = engine_of(plain);
    let (_, outcome) = send(&engine, "ticket.update", input, response(409, json!({}))).await;
    match outcome {
        WriteOutcome::Refused(error) => assert_eq!(error.code, ErrorCode::InvalidInput),
        _ => panic!("a 409 is not refused"),
    }
}

fn engine_of(selection: Value) -> Engine {
    engine(&[selection]).unwrap()
}

#[tokio::test]
async fn a_stale_stamp_is_refused_by_the_preflight_with_nothing_sent() {
    let engine = engine(&[update()]).unwrap();
    let http = reads(vec![response(200, ticket("2026-10-16T11:00:00Z"))]);
    let error = match engine
        .prepare(
            http.as_ref(),
            "fixture",
            "ticket.update",
            json!({"ticket_id": 101, "body": {"ticket": {"priority": "high", "updated_stamp": STAMP}}}),
        )
        .await
    {
        Ok(_) => panic!("a stale stamp was admitted"),
        Err(error) => error,
    };
    assert_eq!(code(&error), "conflict");
    assert!(
        error.upstream_answer,
        "the provider's own answer, read before the write"
    );
    assert_eq!(http.calls.lock().unwrap().len(), 1, "one preflight read");
    // Without `mismatch` a differing check stays the `forbidden` it was.
    let mut plain = update();
    plain["guard"]["preflight"]
        .as_object_mut()
        .unwrap()
        .remove("mismatch");
    let engine = engine_of(plain);
    let http = reads(vec![response(200, ticket("2026-10-16T11:00:00Z"))]);
    let error = match engine
        .prepare(
            http.as_ref(),
            "fixture",
            "ticket.update",
            json!({"ticket_id": 101, "body": {"ticket": {"priority": "high", "updated_stamp": STAMP}}}),
        )
        .await
    {
        Ok(_) => panic!("a stale stamp was admitted"),
        Err(error) => error,
    };
    assert_eq!(error.code, ErrorCode::Forbidden);
}

#[test]
fn malformed_nested_members_are_refused_when_the_selection_loads() {
    let change = |edit: &dyn Fn(&mut Value)| {
        let mut selection = update();
        edit(&mut selection);
        selection
    };
    let plain = |extra: Value| {
        let mut selection =
            json!({"id": "thing.update", "operation_id": "UpdateTicket", "effect": "write"});
        for (k, v) in extra.as_object().unwrap() {
            selection[k] = v.clone();
        }
        selection
    };
    let cases: Vec<(&str, Value)> = vec![
        (
            "a type named twice by a plain name and its pointer",
            plain(json!({"body_keys": ["/s"], "body_types": {"/s": "string", "s": "integer"}})),
        ),
        (
            "an enum named twice by a plain name and its pointer",
            plain(json!({"body_keys": ["/s"], "body_types": {"/s": "string"},
                         "body_enums": {"/s": ["a"], "s": ["b"]}})),
        ),
        (
            "a fixed value named twice by a plain name and its pointer",
            plain(json!({"body_keys": ["/flag"], "body_fixed": {"/flag": true, "flag": false}})),
        ),
        (
            "a prefix named twice by a plain name and its pointer",
            plain(json!({"body_keys": ["/s"], "body_types": {"/s": "string"},
                         "body_prefixes": {"/s": "X", "s": "Y"}})),
        ),
        (
            "a guard reading a list key",
            change(&|s| {
                s["guard"]["preflight"]["checks"][0]["expect"]["input"] = json!("body.ticket.tags")
            }),
        ),
        (
            "an empty segment",
            change(&|s| s["body_keys"][0] = json!("/ticket//group_id")),
        ),
        (
            "a `~` segment",
            change(&|s| s["body_keys"][0] = json!("/ticket/group~1id")),
        ),
        (
            "a trailing slash",
            change(&|s| s["body_keys"][0] = json!("/ticket/group_id/")),
        ),
        (
            "a key inside another",
            change(&|s| {
                s["body_keys"]
                    .as_array_mut()
                    .unwrap()
                    .push(json!("/ticket"))
            }),
        ),
        (
            "a plain name and its pointer",
            change(&|s| {
                s["body_keys"]
                    .as_array_mut()
                    .unwrap()
                    .extend([json!("system_metadata"), json!("/system_metadata")])
            }),
        ),
        (
            "an enum on an integer",
            change(&|s| s["body_enums"]["/ticket/group_id"] = json!(["1"])),
        ),
        (
            "an enum on an untyped key",
            change(&|s| {
                s["body_types"]
                    .as_object_mut()
                    .unwrap()
                    .remove("/ticket/priority");
            }),
        ),
        (
            "an empty enum",
            change(&|s| s["body_enums"]["/ticket/priority"] = json!([])),
        ),
        (
            "a repeated enum value",
            change(&|s| s["body_enums"]["/ticket/priority"] = json!(["high", "high"])),
        ),
        (
            "an empty enum value",
            change(&|s| s["body_enums"]["/ticket/priority"] = json!(["high", ""])),
        ),
        (
            "an enum on a fixed key",
            change(&|s| {
                s["body_types"]["/ticket/safe_update"] = json!("string");
                s["body_fixed"]["/ticket/safe_update"] = json!("yes");
                s["body_enums"]["/ticket/safe_update"] = json!(["yes"]);
            }),
        ),
        (
            "an any-of of one key",
            change(&|s| s["body_any_of"] = json!(["/ticket/priority"])),
        ),
        (
            "an any-of key not admitted",
            change(&|s| s["body_any_of"][0] = json!("/ticket/status")),
        ),
        (
            "an any-of key fixed",
            change(&|s| s["body_any_of"][0] = json!("/ticket/safe_update")),
        ),
        (
            "a repeated any-of key",
            change(&|s| s["body_any_of"][1] = json!("/ticket/group_id")),
        ),
        (
            "any-of keys under two parents",
            change(&|s| {
                s["body_keys"].as_array_mut().unwrap().push(json!("/other"));
                s["body_any_of"][0] = json!("/other");
            }),
        ),
        (
            "a prefix on an untyped key",
            change(&|s| {
                s["body_keys"]
                    .as_array_mut()
                    .unwrap()
                    .push(json!("/ticket/note"));
                s["body_prefixes"] = json!({"/ticket/note": "x"});
            }),
        ),
        (
            "a prefix on a list",
            change(&|s| s["body_prefixes"] = json!({"/ticket/tags": "x"})),
        ),
        (
            "an empty prefix",
            change(&|s| s["body_prefixes"] = json!({"/ticket/priority": ""})),
        ),
        (
            "a prefix not admitted",
            change(&|s| s["body_prefixes"] = json!({"/ticket/status": "x"})),
        ),
        (
            "a conflict status of 400",
            change(&|s| s["conflict_status"] = json!(400)),
        ),
        (
            "a conflict status of 500",
            change(&|s| s["conflict_status"] = json!(500)),
        ),
        (
            "a guard reading an unadmitted nested key",
            change(&|s| {
                s["guard"]["preflight"]["checks"][0]["expect"]["input"] = json!("body.ticket.stamp")
            }),
        ),
    ];
    for (name, selection) in cases {
        let outcome = engine(std::slice::from_ref(&selection));
        assert!(outcome.is_err(), "{name}: {selection}");
        assert_eq!(
            outcome.err().unwrap().code,
            ErrorCode::InvalidInput,
            "{name}"
        );
    }
    // A conflict status on a read.
    let read = json!({"id": "ticket.show", "operation_id": "ShowTicket", "effect": "read",
                      "conflict_status": 409});
    assert!(engine(&[read]).is_err(), "a conflict status on a read");
}

#[test]
fn the_new_members_are_omitted_when_absent_and_read_back_unchanged() {
    for raw in [update(), draft()] {
        let selection = written(raw.clone());
        let mut back = serde_json::to_value(&selection).unwrap();
        back.as_object_mut()
            .unwrap()
            .retain(|key, value| !value.is_null() || raw.get(key).is_some());
        assert_eq!(back, raw);
    }
    let plain =
        written(json!({"id": "ticket.show", "operation_id": "ShowTicket", "effect": "read"}));
    let back = serde_json::to_value(&plain).unwrap();
    for member in [
        "body_enums",
        "body_any_of",
        "body_prefixes",
        "conflict_status",
    ] {
        assert!(back.get(member).is_none(), "`{member}` written when absent");
    }
}

#[tokio::test]
async fn an_absent_or_non_object_body_is_refused_as_missing_not_as_a_foreign_key() {
    let engine = engine(&[update(), draft()]).unwrap();
    for (id, input) in [
        ("ticket.update", json!({"ticket_id": 101})),
        ("ticket.update", json!({"ticket_id": 101, "body": 5})),
        ("ticket.update", json!({"ticket_id": 101, "body": null})),
        ("ticket.draft_reply", json!({"ticket_id": 101})),
    ] {
        let error = refused(&engine, id, input.clone()).await;
        assert_eq!(code(&error), "invalid_input", "`{id}` {input}");
        assert!(
            error.message.contains("missing or not an object"),
            "`{id}` {input}: {}",
            error.message
        );
        assert!(
            !error.message.contains("carries a key"),
            "`{id}` {input}: {}",
            error.message
        );
    }
}

#[test]
fn an_explicit_null_rate_limit_headers_or_embedded_text_is_refused_at_load() {
    for member in ["rate_limit_headers", "embedded_text", "conflict_status"] {
        let mut selection =
            json!({"id": "thing.show", "operation_id": "ShowTicket", "effect": "read"});
        selection[member] = Value::Null;
        let loaded = serde_json::from_value::<Selection>(selection);
        assert!(loaded.is_err(), "`{member}: null` loaded as absent");
    }
}
