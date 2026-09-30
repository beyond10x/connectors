//! The engine over a small fixture bundle: declarations, bound reads, and
//! guarded writes classified across the provider's answers.
use connectors_catalog::{bundle::Bundle, ingest, inventory};
use connectors_catalog_provider::{
    Check, Effect, Engine, Expectation, Guard, Postflight, Preflight, ResponseKind, Selection,
};
use connectors_core::{ErrorCode, Result};
use connectors_sdk::{
    AuthenticatedHttp, AuthenticatedWrite, HttpResponse, WriteMethod, WriteOutcome,
};
use serde_json::{Value, json};
use std::{
    collections::{BTreeMap, VecDeque},
    sync::{Arc, Mutex},
};

const SHA: &str = "0123456789abcdef0123456789abcdef01234567";

fn document() -> Vec<u8> {
    let path = |name: &str| json!({"name": name, "in": "path", "required": true, "schema": {"type": "string"}});
    // As the pinned GitLab document types them: an iid or a job id is an integer.
    let number = |name: &str| json!({"name": name, "in": "path", "required": true, "schema": {"type": "integer"}});
    serde_json::to_vec(&json!({
        "openapi": "3.0.0",
        "info": {"title": "fixture", "version": "1"},
        "paths": {
            "/api/v4/projects/{id}/merge_requests": {
                "post": {
                    "operationId": "createMergeRequest",
                    "parameters": [path("id")],
                    "requestBody": {"required": true, "content": {"application/json": {"schema": {"type": "object"}}}},
                    "responses": {"201": {"description": "Created", "content": {"application/json": {}}}}
                },
                "get": {
                    "operationId": "listMergeRequests",
                    "parameters": [
                        path("id"),
                        {"name": "state", "in": "query", "required": false, "schema": {"type": "string"}},
                        {"name": "per_page", "in": "query", "required": false, "schema": {"type": "integer"}}
                    ],
                    "responses": {"200": {"description": "OK", "content": {"application/json": {}}}}
                }
            },
            "/api/v4/projects/{id}/merge_requests/{merge_request_iid}": {
                "get": {
                    "operationId": "getMergeRequest",
                    "parameters": [path("id"), number("merge_request_iid")],
                    "responses": {"200": {"description": "OK", "content": {"application/json": {}}}}
                }
            },
            "/api/v4/projects/{id}/merge_requests/{merge_request_iid}/merge": {
                "put": {
                    "operationId": "mergeMergeRequest",
                    "parameters": [path("id"), number("merge_request_iid")],
                    "requestBody": {"required": true, "content": {"application/json": {"schema": {"type": "object"}}}},
                    "responses": {"200": {"description": "OK", "content": {"application/json": {}}}}
                }
            },
            "/api/v4/projects/{id}/repository/branches/{branch}": {
                "get": {
                    "operationId": "getBranch",
                    "parameters": [path("id"), path("branch")],
                    "responses": {"200": {"description": "OK", "content": {"application/json": {}}}}
                }
            },
            "/api/v4/projects/{id}/jobs/{job_id}/trace": {
                "get": {
                    "operationId": "getJobTrace",
                    "parameters": [path("id"), number("job_id")],
                    "responses": {"200": {"description": "OK", "content": {"application/json": {}}}}
                }
            },
            "/api/v4/projects/{id}/export": {
                "get": {
                    "operationId": "getExport",
                    "parameters": [path("id")],
                    "responses": {"200": {"description": "OK", "content": {"text/plain": {}}}}
                }
            },
            "/other/{id}": {
                "get": {"operationId": "outside", "parameters": [path("id")], "responses": {}}
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

fn input_check(pointer: &str, input: &str) -> Check {
    Check {
        pointer: pointer.into(),
        expect: Expectation::Input(input.into()),
    }
}
fn literal_check(pointer: &str, literal: &str) -> Check {
    Check {
        pointer: pointer.into(),
        expect: Expectation::Literal(literal.into()),
    }
}
fn select(id: &str, operation_id: &str, effect: Effect) -> Selection {
    Selection {
        id: id.into(),
        operation_id: operation_id.into(),
        effect,
        description: None,
        guard: None,
        response: None,
        bounds: BTreeMap::new(),
    }
}
fn mr_guard(values: &[(&str, &str)], preflight: Vec<Check>, postflight: Vec<Check>) -> Guard {
    Guard {
        preflight: Preflight {
            operation_id: "getMergeRequest".into(),
            values: values
                .iter()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect(),
            checks: preflight,
        },
        postflight: Postflight { checks: postflight },
    }
}

fn selections() -> Vec<Selection> {
    vec![
        select("merge_requests.list", "listMergeRequests", Effect::Read),
        Selection {
            description: Some("Open a merge request at a pinned source head".into()),
            guard: Some(Guard {
                preflight: Preflight {
                    operation_id: "getBranch".into(),
                    values: BTreeMap::from([
                        ("id".to_string(), "id".to_string()),
                        ("branch".to_string(), "body.source_branch".to_string()),
                    ]),
                    checks: vec![input_check("/commit/id", "sha")],
                },
                postflight: Postflight {
                    checks: vec![input_check("/sha", "sha")],
                },
            }),
            ..select("merge_request.create", "createMergeRequest", Effect::Write)
        },
        Selection {
            guard: Some(mr_guard(
                &[("id", "id"), ("merge_request_iid", "merge_request_iid")],
                vec![
                    input_check("/sha", "body.sha"),
                    literal_check("/state", "opened"),
                    input_check("/head_pipeline/id", "pipeline_id"),
                ],
                vec![
                    literal_check("/state", "merged"),
                    input_check("/sha", "body.sha"),
                ],
            )),
            ..select("merge_request.merge", "mergeMergeRequest", Effect::Write)
        },
        Selection {
            response: Some(ResponseKind::Text),
            ..select("job.trace", "getJobTrace", Effect::Read)
        },
        select("export", "getExport", Effect::Read),
    ]
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
fn raw(status: u16, body: &[u8]) -> HttpResponse {
    HttpResponse {
        status,
        headers: Default::default(),
        body: body.to_vec(),
    }
}
/// The one-use write capability: records what was sent, answers once.
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
        query: &[(&str, String)],
        body: &Value,
    ) -> Result<HttpResponse> {
        assert!(query.is_empty());
        self.sent.lock().unwrap().push((
            method,
            path.iter().map(|s| s.to_string()).collect(),
            body.clone(),
        ));
        self.response
            .ok_or_else(connectors_core::Error::unavailable)
    }
}
fn create_input() -> Value {
    json!({"id": "org/project", "sha": SHA, "body": {"source_branch": "fix", "target_branch": "main", "title": "t"}})
}
fn merge_input() -> Value {
    json!({"id": "org/project", "merge_request_iid": 7, "pipeline_id": 42, "body": {"sha": SHA}})
}
fn opened(sha: &str) -> Value {
    json!({"id": 10, "iid": 7, "sha": sha, "state": "opened", "title": "t", "head_pipeline": {"id": 42}})
}
fn classify(outcome: WriteOutcome<Value>) -> &'static str {
    match outcome {
        WriteOutcome::Applied(Ok(value)) => {
            assert_eq!(
                value["provenance"]["source_revision"],
                bundle().source.source_sha256
            );
            "applied"
        }
        WriteOutcome::Applied(Err(error)) => panic!("projection failed: {error}"),
        WriteOutcome::Refused(error) => {
            assert!(!error.message.contains("private provider"));
            "refused"
        }
        WriteOutcome::Unknown(error) => {
            assert!(!error.message.contains("private provider"));
            "unknown"
        }
    }
}

#[test]
fn declarations_carry_parameters_body_and_guard_inputs_and_refuse_bad_selections() {
    let engine = Engine::new(&bundle(), "/api/v4", &selections()).unwrap();
    let reads = engine.declarations(&[Effect::Read]);
    assert_eq!(reads.len(), 3);
    assert_eq!(reads[0].profile, "generic-http");
    assert_eq!(reads[0].input_schema["required"], json!(["id"]));
    assert!(reads[0].input_schema["properties"]["state"].is_object());
    let all = engine.declarations(&[Effect::Read, Effect::Write]);
    let create = all.iter().find(|o| o.id == "merge_request.create").unwrap();
    assert_eq!(create.profile, "mutation");
    assert_eq!(
        create.input_schema["required"],
        json!(["body", "id", "sha"])
    );
    assert_eq!(create.input_schema["additionalProperties"], json!(false));
    // A guard's input references become required inputs; literals add nothing.
    let merge = all.iter().find(|o| o.id == "merge_request.merge").unwrap();
    assert_eq!(
        merge.input_schema["required"],
        json!(["body", "id", "merge_request_iid", "pipeline_id"])
    );
    let refused: Vec<Selection> = vec![
        select("missing", "noSuchOperation", Effect::Read),
        select("wrong_effect", "createMergeRequest", Effect::Read),
        select("wrong_effect_two", "listMergeRequests", Effect::Write),
        select("outside", "outside", Effect::Read),
        select("bad id", "listMergeRequests", Effect::Read),
        Selection {
            response: Some(ResponseKind::Text),
            ..select("text_write", "createMergeRequest", Effect::Write)
        },
        Selection {
            guard: Some(mr_guard(&[("id", "id")], vec![], vec![])),
            ..select("no_checks", "mergeMergeRequest", Effect::Write)
        },
        Selection {
            guard: Some(mr_guard(
                &[("id", "id")],
                vec![literal_check("state", "opened")],
                vec![],
            )),
            ..select("bad_pointer", "mergeMergeRequest", Effect::Write)
        },
        Selection {
            guard: Some(mr_guard(
                &[("id", "id")],
                (0..17).map(|_| literal_check("/state", "opened")).collect(),
                vec![],
            )),
            ..select("too_many", "mergeMergeRequest", Effect::Write)
        },
    ];
    for selection in refused {
        let id = selection.id.clone();
        assert!(
            Engine::new(&bundle(), "/api/v4", &[selection]).is_err(),
            "{id}"
        );
    }
}

#[tokio::test]
async fn read_binds_path_and_query_and_projects_the_observation() {
    let engine = Engine::new(&bundle(), "/api/v4", &selections()).unwrap();
    let http = reads(vec![response(200, json!([{"iid": 1}]))]);
    let value = engine
        .read(
            http.as_ref(),
            "fixture",
            "merge_requests.list",
            json!({"id": "org/project", "state": "opened"}),
        )
        .await
        .unwrap();
    assert_eq!(value["status"], 200);
    assert_eq!(value["body"], json!([{"iid": 1}]));
    assert_eq!(
        value["provenance"]["resource"],
        "/api/v4/projects/{id}/merge_requests"
    );
    assert_eq!(value["provenance"]["instance"], "fixture");
    {
        let calls = http.calls.lock().unwrap();
        assert_eq!(calls[0].0, ["projects", "org/project", "merge_requests"]);
        assert_eq!(calls[0].1, [("state".to_string(), "opened".to_string())]);
    }
    // A provider refusal on a read is an error with a safe code, not a body.
    let http = reads(vec![raw(404, b"private")]);
    let error = engine
        .read(
            http.as_ref(),
            "fixture",
            "merge_requests.list",
            json!({"id": "org/project"}),
        )
        .await
        .err()
        .unwrap();
    assert_eq!(error.code, ErrorCode::NotFound);
    assert!(!error.message.contains("private"));
    // Undeclared input is refused before any provider call.
    let http = reads(vec![]);
    assert!(
        engine
            .read(
                http.as_ref(),
                "fixture",
                "merge_requests.list",
                json!({"id": "org/project", "nope": 1})
            )
            .await
            .is_err()
    );
    assert!(http.calls.lock().unwrap().is_empty());
}

#[tokio::test]
async fn text_bodies_come_from_the_bundle_or_the_reviewed_exception() {
    let engine = Engine::new(&bundle(), "/api/v4", &selections()).unwrap();
    // The source declares JSON for the trace; the selection's exception reads text.
    let http = reads(vec![raw(200, b"Running with gitlab-runner\n")]);
    let value = engine
        .read(
            http.as_ref(),
            "fixture",
            "job.trace",
            json!({"id": "org/project", "job_id": 3}),
        )
        .await
        .unwrap();
    assert_eq!(value["body"], "Running with gitlab-runner\n");
    // The source declares text/plain only: text without an exception.
    let http = reads(vec![raw(200, b"a,b\n")]);
    let value = engine
        .read(
            http.as_ref(),
            "fixture",
            "export",
            json!({"id": "org/project"}),
        )
        .await
        .unwrap();
    assert_eq!(value["body"], "a,b\n");
    // A JSON read still refuses a body that is not JSON.
    let http = reads(vec![raw(200, b"not json")]);
    let error = engine
        .read(
            http.as_ref(),
            "fixture",
            "merge_requests.list",
            json!({"id": "org/project"}),
        )
        .await
        .err()
        .unwrap();
    assert_eq!(error.code, ErrorCode::UpstreamProtocol);
    // Text that is not UTF-8 is refused the same way.
    let http = reads(vec![raw(200, &[0xff, 0xfe])]);
    assert!(
        engine
            .read(
                http.as_ref(),
                "fixture",
                "job.trace",
                json!({"id": "org/project", "job_id": 3})
            )
            .await
            .is_err()
    );
}

#[tokio::test]
async fn guarded_write_refuses_before_dispatch_and_classifies_after() {
    let engine = Engine::new(&bundle(), "/api/v4", &selections()).unwrap();
    // The preflight reads the branch the input names.
    let http = reads(vec![response(
        200,
        json!({"name": "fix", "commit": {"id": SHA}}),
    )]);
    engine
        .prepare(
            http.as_ref(),
            "fixture",
            "merge_request.create",
            create_input(),
        )
        .await
        .unwrap();
    assert_eq!(
        http.calls.lock().unwrap()[0].0,
        ["projects", "org/project", "repository", "branches", "fix"]
    );
    // A head that already differs, or a branch that is absent, is refused with no write.
    for reply in [
        response(200, json!({"commit": {"id": "a".repeat(40)}})),
        raw(404, b""),
    ] {
        let http = reads(vec![reply]);
        let error = engine
            .prepare(
                http.as_ref(),
                "fixture",
                "merge_request.create",
                create_input(),
            )
            .await
            .err()
            .unwrap();
        assert_eq!(error.code, ErrorCode::Forbidden);
    }
    // After dispatch: exactly one POST, classified by the answer.
    let mut cases = vec![
        (Some(response(201, opened(SHA))), "applied"),
        (None, "unknown"),
        (Some(response(201, opened(&"a".repeat(40)))), "unknown"),
        (Some(response(200, opened(SHA))), "applied"),
        (Some(raw(201, b"not json")), "unknown"),
    ];
    for status in [
        400, 401, 403, 404, 405, 409, 410, 412, 415, 422, 429, 500, 502, 307, 202,
    ] {
        cases.push((
            Some(raw(status, b"private provider error")),
            if matches!(
                status,
                400 | 401 | 403 | 404 | 405 | 409 | 410 | 412 | 415 | 422
            ) {
                "refused"
            } else {
                "unknown"
            },
        ));
    }
    for (reply, expected) in cases {
        let http = reads(vec![response(200, json!({"commit": {"id": SHA}}))]);
        let prepared = engine
            .prepare(
                http.as_ref(),
                "fixture",
                "merge_request.create",
                create_input(),
            )
            .await
            .unwrap();
        let sent = Arc::new(Mutex::new(Vec::new()));
        let outcome = prepared
            .execute(Box::new(Send {
                sent: sent.clone(),
                response: reply,
            }))
            .await;
        assert_eq!(classify(outcome), expected);
        let sent = sent.lock().unwrap();
        assert_eq!(sent.len(), 1);
        assert_eq!(sent[0].0, WriteMethod::Post);
        assert_eq!(sent[0].1, ["projects", "org/project", "merge_requests"]);
        assert_eq!(
            sent[0].2,
            json!({"source_branch": "fix", "target_branch": "main", "title": "t"})
        );
    }
}

/// A refused write keeps what the provider answered, as a read does, and is
/// marked as the provider's own answer so the host reports it at dispatch.
#[tokio::test]
async fn a_refused_write_keeps_the_providers_status_as_its_marked_code() {
    let engine = Engine::new(&bundle(), "/api/v4", &selections()).unwrap();
    for (status, code) in [
        (400, ErrorCode::InvalidInput),
        (409, ErrorCode::InvalidInput),
        (412, ErrorCode::InvalidInput),
        (422, ErrorCode::InvalidInput),
        (401, ErrorCode::Unauthorized),
        (403, ErrorCode::Forbidden),
        (404, ErrorCode::NotFound),
        (410, ErrorCode::NotFound),
        (405, ErrorCode::Forbidden),
        (415, ErrorCode::Forbidden),
    ] {
        let http = reads(vec![response(200, json!({"commit": {"id": SHA}}))]);
        let prepared = engine
            .prepare(
                http.as_ref(),
                "fixture",
                "merge_request.create",
                create_input(),
            )
            .await
            .unwrap();
        let outcome = prepared
            .execute(Box::new(Send {
                sent: Arc::new(Mutex::new(Vec::new())),
                response: Some(raw(status, b"private provider error")),
            }))
            .await;
        let WriteOutcome::Refused(error) = outcome else {
            panic!("{status} was not refused");
        };
        assert_eq!(error.code, code, "{status}");
        assert!(error.upstream_answer, "{status} is the provider's answer");
        assert!(!error.message.contains("private provider"));
    }
}

#[tokio::test]
async fn multi_check_guard_holds_every_check_before_and_after_dispatch() {
    let engine = Engine::new(&bundle(), "/api/v4", &selections()).unwrap();
    let merged = |sha: &str| json!({"iid": 7, "sha": sha, "state": "merged"});
    // Every preflight check holds: one PUT with the body as given, applied.
    let http = reads(vec![response(200, opened(SHA))]);
    let prepared = engine
        .prepare(
            http.as_ref(),
            "fixture",
            "merge_request.merge",
            merge_input(),
        )
        .await
        .unwrap();
    assert_eq!(
        http.calls.lock().unwrap()[0].0,
        ["projects", "org/project", "merge_requests", "7"]
    );
    let sent = Arc::new(Mutex::new(Vec::new()));
    let outcome = prepared
        .execute(Box::new(Send {
            sent: sent.clone(),
            response: Some(response(200, merged(SHA))),
        }))
        .await;
    assert_eq!(classify(outcome), "applied");
    {
        let sent = sent.lock().unwrap();
        assert_eq!(sent[0].0, WriteMethod::Put);
        assert_eq!(
            sent[0].1,
            ["projects", "org/project", "merge_requests", "7", "merge"]
        );
        assert_eq!(sent[0].2, json!({"sha": SHA}));
    }
    // Any single preflight check failing refuses before dispatch, by name.
    let mut moved = opened(&"b".repeat(40));
    let mut closed = opened(SHA);
    closed["state"] = json!("closed");
    let mut other_pipeline = opened(SHA);
    other_pipeline["head_pipeline"]["id"] = json!(43);
    moved["sha"] = json!("b".repeat(40));
    for (reply, pointer) in [
        (moved, "/sha"),
        (closed, "/state"),
        (other_pipeline, "/head_pipeline/id"),
    ] {
        let http = reads(vec![response(200, reply)]);
        let error = engine
            .prepare(
                http.as_ref(),
                "fixture",
                "merge_request.merge",
                merge_input(),
            )
            .await
            .err()
            .unwrap();
        assert_eq!(error.code, ErrorCode::Forbidden, "{pointer}");
        assert!(error.message.contains(pointer), "{}", error.message);
    }
    // A preflight body without the pointed value is a protocol error, not a refusal.
    let mut no_pipeline = opened(SHA);
    no_pipeline["head_pipeline"] = Value::Null;
    let http = reads(vec![response(200, no_pipeline)]);
    let error = engine
        .prepare(
            http.as_ref(),
            "fixture",
            "merge_request.merge",
            merge_input(),
        )
        .await
        .err()
        .unwrap();
    assert_eq!(error.code, ErrorCode::UpstreamProtocol);
    // An absent guard input is refused before the preflight is even read.
    let http = reads(vec![]);
    let mut input = merge_input();
    input.as_object_mut().unwrap().remove("pipeline_id");
    assert!(
        engine
            .prepare(http.as_ref(), "fixture", "merge_request.merge", input)
            .await
            .is_err()
    );
    assert!(http.calls.lock().unwrap().is_empty());
    // After dispatch, a postflight check that fails leaves the effect possible.
    let mut still_open = merged(SHA);
    still_open["state"] = json!("opened");
    for (reply, expected) in [
        (merged(&"b".repeat(40)), "unknown"),
        (still_open, "unknown"),
        (merged(SHA), "applied"),
    ] {
        let http = reads(vec![response(200, opened(SHA))]);
        let prepared = engine
            .prepare(
                http.as_ref(),
                "fixture",
                "merge_request.merge",
                merge_input(),
            )
            .await
            .unwrap();
        let outcome = prepared
            .execute(Box::new(Send {
                sent: Arc::new(Mutex::new(Vec::new())),
                response: Some(response(200, reply)),
            }))
            .await;
        assert_eq!(classify(outcome), expected);
    }
}

/// A selection written as data, as a shipped selection file carries it.
fn written(value: Value) -> Selection {
    serde_json::from_value(value).unwrap()
}

#[test]
fn a_bound_must_name_a_declared_query_parameter_and_is_omitted_when_absent() {
    let bounded = written(json!({
        "id": "merge_requests.list", "operation_id": "listMergeRequests", "effect": "read",
        "bounds": {"per_page": {"minimum": 1, "maximum": 100}}
    }));
    let engine = Engine::new(&bundle(), "/api/v4", &[bounded]).unwrap();
    // The declared schema carries the bound for callers.
    let declared = &engine.declarations(&[Effect::Read])[0].input_schema["properties"]["per_page"];
    assert_eq!(declared["maximum"], 100);
    assert_eq!(declared["minimum"], 1);
    // A maximum alone declares no minimum.
    let open = written(json!({
        "id": "merge_requests.list", "operation_id": "listMergeRequests", "effect": "read",
        "bounds": {"per_page": {"maximum": 100}}
    }));
    let engine = Engine::new(&bundle(), "/api/v4", &[open]).unwrap();
    let declared = &engine.declarations(&[Effect::Read])[0].input_schema["properties"]["per_page"];
    assert!(declared.get("minimum").is_none(), "{declared}");
    // A minimum above the maximum admits no value: refused when it loads.
    let empty = written(json!({
        "id": "merge_requests.list", "operation_id": "listMergeRequests", "effect": "read",
        "bounds": {"per_page": {"minimum": 101, "maximum": 100}}
    }));
    let error = Engine::new(&bundle(), "/api/v4", &[empty]).err().unwrap();
    assert_eq!(error.code, ErrorCode::InvalidInput);
    // An unbounded minimum serialises without its key.
    assert_eq!(
        serde_json::to_value(written(json!({
            "id": "merge_requests.list", "operation_id": "listMergeRequests", "effect": "read",
            "bounds": {"per_page": {"maximum": 100}}
        })))
        .unwrap()["bounds"],
        json!({"per_page": {"maximum": 100}})
    );
    // An undeclared parameter, or a path parameter, cannot be bounded: the
    // selection is refused when it loads.
    for (id, parameter) in [("undeclared", "limit"), ("path_parameter", "id")] {
        let selection = written(json!({
            "id": id, "operation_id": "listMergeRequests", "effect": "read",
            "bounds": {parameter: {"maximum": 100}}
        }));
        let error = Engine::new(&bundle(), "/api/v4", &[selection])
            .err()
            .unwrap_or_else(|| panic!("`{id}` loaded"));
        assert_eq!(error.code, ErrorCode::InvalidInput, "`{id}`");
    }
    // A bound carries a maximum and optionally a minimum, nothing else.
    for bound in [
        json!({}),
        json!({"maximum": 100, "exclusive": 1}),
        json!({"minimum": 1}),
        json!({"maximum": -1}),
    ] {
        assert!(
            serde_json::from_value::<Selection>(json!({
                "id": "merge_requests.list", "operation_id": "listMergeRequests",
                "effect": "read", "bounds": {"per_page": bound}
            }))
            .is_err(),
            "{bound}"
        );
    }
    // An unbounded selection serialises without the key, as before bounds.
    let plain = serde_json::to_value(select(
        "merge_requests.list",
        "listMergeRequests",
        Effect::Read,
    ))
    .unwrap();
    assert!(plain.get("bounds").is_none(), "{plain}");
}

#[tokio::test]
async fn a_bounded_parameter_is_checked_as_an_integer_before_any_request() {
    let bounded = written(json!({
        "id": "merge_requests.list", "operation_id": "listMergeRequests", "effect": "read",
        "bounds": {"per_page": {"minimum": 1, "maximum": 100}}
    }));
    let engine = Engine::new(&bundle(), "/api/v4", &[bounded]).unwrap();
    // Above the maximum, below the minimum (`-0` included), or not an integer
    // at all, whether sent as a number, a string or a boolean: refused as
    // invalid input with nothing sent.
    for per_page in [
        json!(101),
        json!("101"),
        json!(0),
        json!("0"),
        json!("-0"),
        json!(-1),
        json!("-100"),
        json!("ten"),
        json!("1.5"),
        json!(1.5),
        json!(""),
        json!(true),
        json!("18446744073709551616"),
    ] {
        let http = reads(vec![]);
        let error = engine
            .read(
                http.as_ref(),
                "fixture",
                "merge_requests.list",
                json!({"id": "org/project", "per_page": per_page}),
            )
            .await
            .err()
            .unwrap_or_else(|| panic!("per_page={per_page} was sent"));
        assert_eq!(error.code, ErrorCode::InvalidInput, "per_page={per_page}");
        assert!(http.calls.lock().unwrap().is_empty(), "per_page={per_page}");
    }
    // At or below the maximum, or absent, the read goes out unchanged.
    for (per_page, query) in [
        (json!(100), vec![("per_page", "100")]),
        (json!("100"), vec![("per_page", "100")]),
        (json!(1), vec![("per_page", "1")]),
        (Value::Null, vec![]),
    ] {
        let mut input = json!({"id": "org/project"});
        if !per_page.is_null() {
            input["per_page"] = per_page.clone();
        }
        let http = reads(vec![response(200, json!([]))]);
        engine
            .read(http.as_ref(), "fixture", "merge_requests.list", input)
            .await
            .unwrap_or_else(|error| panic!("per_page={per_page}: {error}"));
        let expected: Vec<(String, String)> = query
            .into_iter()
            .map(|(k, v)| (k.to_owned(), v.to_owned()))
            .collect();
        assert_eq!(
            http.calls.lock().unwrap()[0].1,
            expected,
            "per_page={per_page}"
        );
    }
}
