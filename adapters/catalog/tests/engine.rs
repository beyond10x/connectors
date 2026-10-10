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
        binary: None,
        download: None,
        bounds: BTreeMap::new(),
        required: Vec::new(),
        withhold: Vec::new(),
        credential: Vec::new(),
        rate_limit_reasons: Vec::new(),
        body_keys: Vec::new(),
        body_types: BTreeMap::new(),
        body_required: Vec::new(),
        body_fixed: BTreeMap::new(),
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
        further_preflights: vec![],
        postflight: Postflight {
            checks: postflight,
            read: None,
            any_of: vec![],
            absent: None,
        },
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
                further_preflights: vec![],
                postflight: Postflight {
                    checks: vec![input_check("/sha", "sha")],
                    read: None,
                    any_of: vec![],
                    absent: None,
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

/// story:service-failure-carries-upstream-reason: a refusal keeps the
/// provider's own top-level message as its bounded reason, beside its code and
/// outside its message; a secret-shaped one is withheld.
#[tokio::test]
async fn a_refusal_carries_the_providers_bounded_reason_and_withholds_a_secret_shaped_one() {
    let engine = Engine::new(&bundle(), "/api/v4", &selections()).unwrap();
    let read = |response: HttpResponse| {
        let engine = &engine;
        async move {
            let http = reads(vec![response]);
            engine
                .read(
                    http.as_ref(),
                    "fixture",
                    "merge_requests.list",
                    json!({"id": "org/project"}),
                )
                .await
                .err()
                .unwrap()
        }
    };
    let scope = json!({"code": 401, "message": "Unauthorized; scope does not match"});
    let error = read(response(401, scope.clone())).await;
    assert_eq!(error.code, ErrorCode::Unauthorized);
    assert!(error.upstream_answer);
    assert_eq!(
        error.upstream_reason.as_deref(),
        Some("Unauthorized; scope does not match")
    );
    assert!(!error.message.contains("scope"));
    // The reason is in-process only: the service wire never carries it.
    assert!(
        !serde_json::to_string(&error)
            .unwrap()
            .contains("scope does not match")
    );
    let leaky = json!({"message": "Unauthorized; token fixtok-AbCdEfGhIjKl0123456789 revoked"});
    let error = read(response(401, leaky)).await;
    assert_eq!(error.code, ErrorCode::Unauthorized);
    assert_eq!(error.upstream_reason, None);
    let long = json!({"message": format!("Unauthorized {}", "scopé ".repeat(40))});
    let reason = read(response(403, long)).await.upstream_reason.unwrap();
    assert!(reason.len() <= 256 && reason.starts_with("Unauthorized scopé"));
    assert_eq!(read(raw(404, b"private")).await.upstream_reason, None);
    // A refused write keeps the same reason.
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
    let WriteOutcome::Refused(error) = prepared
        .execute(Box::new(Send {
            sent: Arc::new(Mutex::new(Vec::new())),
            response: Some(response(401, scope)),
        }))
        .await
    else {
        panic!("401 was not refused");
    };
    assert_eq!(
        error.upstream_reason.as_deref(),
        Some("Unauthorized; scope does not match")
    );
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

#[test]
fn a_selection_can_withhold_only_an_optional_declared_parameter() {
    // A name the operation does not declare, a path parameter (required
    // upstream), one the selection itself requires, one it also bounds, and
    // one a guard reads as an input: each is refused when the selection loads.
    for (id, selection) in [
        (
            "undeclared",
            json!({"id": "one", "operation_id": "listMergeRequests", "effect": "read",
                   "withhold": ["limit"]}),
        ),
        (
            "empty",
            json!({"id": "one", "operation_id": "listMergeRequests", "effect": "read",
                   "withhold": [""]}),
        ),
        (
            "path_parameter",
            json!({"id": "one", "operation_id": "listMergeRequests", "effect": "read",
                   "withhold": ["id"]}),
        ),
        (
            "selection_required",
            json!({"id": "one", "operation_id": "listMergeRequests", "effect": "read",
                   "required": ["state"], "withhold": ["state"]}),
        ),
        (
            "bounded",
            json!({"id": "one", "operation_id": "listMergeRequests", "effect": "read",
                   "bounds": {"per_page": {"maximum": 100}}, "withhold": ["per_page"]}),
        ),
    ] {
        let error = Engine::new(&bundle(), "/api/v4", &[written(selection)])
            .err()
            .unwrap_or_else(|| panic!("`{id}` loaded"));
        assert_eq!(error.code, ErrorCode::InvalidInput, "`{id}`");
        assert!(
            error.message.contains("withholds"),
            "`{id}`: {}",
            error.message
        );
    }
    // A guard that reads a withheld name as an input would declare it again.
    let bytes = serde_json::to_vec(&json!({
        "openapi": "3.0.0",
        "info": {"title": "guarded", "version": "1"},
        "paths": {"/v1/things/{id}": {
            "get": {
                "operationId": "getThing",
                "parameters": [{"name": "id", "in": "path", "required": true, "schema": {"type": "string"}}],
                "responses": {"200": {"description": "OK", "content": {"application/json": {}}}}
            },
            "put": {
                "operationId": "putThing",
                "parameters": [
                    {"name": "id", "in": "path", "required": true, "schema": {"type": "string"}},
                    {"name": "mode", "in": "query", "schema": {"type": "string"}}
                ],
                "requestBody": {"required": true, "content": {"application/json": {"schema": {"type": "object"}}}},
                "responses": {"200": {"description": "OK", "content": {"application/json": {}}}}
            }
        }}
    }))
    .unwrap();
    let guarded = Bundle {
        provider: "guarded".into(),
        source: ingest("guarded.json", &bytes).unwrap(),
        inventory: inventory::extract(&serde_json::from_slice(&bytes).unwrap()),
        auth_profile: "fixture.token".into(),
    };
    let guard = |reads: &str| {
        json!({
            "preflight": {"operation_id": "getThing", "values": {"id": "id"},
                          "checks": [{"pointer": "/mode", "expect": {"input": reads}}]},
            "postflight": {"checks": []}
        })
    };
    let error = Engine::new(
        &guarded,
        "/v1",
        &[written(json!({
            "id": "thing.put", "operation_id": "putThing", "effect": "write",
            "withhold": ["mode"], "guard": guard("mode")
        }))],
    )
    .err()
    .expect("a guard reading a withheld name loaded");
    assert_eq!(error.code, ErrorCode::InvalidInput);
    assert!(error.message.contains("withholds"), "{}", error.message);
    // The same guard reading another input loads, and `mode` is not declared.
    let engine = Engine::new(
        &guarded,
        "/v1",
        &[written(json!({
            "id": "thing.put", "operation_id": "putThing", "effect": "write",
            "withhold": ["mode"], "guard": guard("expected_mode")
        }))],
    )
    .unwrap();
    let declared = &engine.declarations(&[Effect::Write])[0].input_schema["properties"];
    assert!(declared.get("mode").is_none(), "{declared}");
    // The field round-trips, and a selection without it serialises as before.
    let withheld = written(json!({
        "id": "merge_requests.list", "operation_id": "listMergeRequests", "effect": "read",
        "withhold": ["state"]
    }));
    assert_eq!(
        serde_json::to_value(&withheld).unwrap()["withhold"],
        json!(["state"])
    );
    let plain = serde_json::to_value(select(
        "merge_requests.list",
        "listMergeRequests",
        Effect::Read,
    ))
    .unwrap();
    assert!(plain.get("withhold").is_none(), "{plain}");
}

#[tokio::test]
async fn a_withheld_parameter_is_not_declared_and_is_refused_before_any_request() {
    let withheld = written(json!({
        "id": "merge_requests.list", "operation_id": "listMergeRequests", "effect": "read",
        "bounds": {"per_page": {"minimum": 1, "maximum": 100}}, "withhold": ["state"]
    }));
    let engine = Engine::new(&bundle(), "/api/v4", &[withheld]).unwrap();
    // Describe lists every other parameter, and not the withheld one.
    let declared = &engine.declarations(&[Effect::Read])[0].input_schema;
    let names: Vec<&String> = declared["properties"].as_object().unwrap().keys().collect();
    assert_eq!(names, ["id", "per_page"], "{declared}");
    // An input carrying it, with any value, is refused with nothing sent.
    for state in [json!("opened"), json!(""), json!(1), json!(true)] {
        assert_eq!(
            refused_with_nothing_sent(
                &engine,
                "merge_requests.list",
                json!({"id": "org/project", "state": state}),
            )
            .await,
            ErrorCode::InvalidInput,
            "state={state}"
        );
    }
    // Without it the read goes out with the parameters it does carry.
    let http = reads(vec![response(200, json!([]))]);
    engine
        .read(
            http.as_ref(),
            "fixture",
            "merge_requests.list",
            json!({"id": "org/project", "per_page": 20}),
        )
        .await
        .unwrap();
    assert_eq!(
        http.calls.lock().unwrap()[0].1,
        [("per_page".to_string(), "20".to_string())]
    );
}

/// A second fixture for the parameter and answer shapes: a repeated string, a
/// repeated integer, a scalar, a query parameter the document does not require,
/// a text read and a plain write.
fn shapes_bundle() -> Bundle {
    let bytes = serde_json::to_vec(&json!({
        "openapi": "3.0.0",
        "info": {"title": "shapes", "version": "1"},
        "paths": {
            "/v1/users/{userId}/messages": {"get": {
                "operationId": "listMessages",
                "parameters": [
                    {"name": "userId", "in": "path", "required": true, "schema": {"type": "string"}},
                    {"name": "labelIds", "in": "query", "schema": {"type": "array", "items": {"type": "string"}}},
                    {"name": "ids", "in": "query", "style": "form", "explode": true,
                     "schema": {"type": "array", "items": {"type": "integer"}}},
                    {"name": "flags", "in": "query", "schema": {"type": "array", "items": {"type": "boolean"}}},
                    {"name": "q", "in": "query", "schema": {"type": "string"}}
                ],
                "responses": {"200": {"description": "OK", "content": {"application/json": {}}}}
            }},
            "/v1/about": {"get": {
                "operationId": "getAbout",
                "parameters": [
                    {"name": "fields", "in": "query", "schema": {"type": "string"}},
                    {"name": "trace", "in": "header", "schema": {"type": "string"}}
                ],
                "responses": {"200": {"description": "OK", "content": {"application/json": {}}}}
            }},
            "/v1/files/{fileId}/export": {"get": {
                "operationId": "exportFile",
                "parameters": [{"name": "fileId", "in": "path", "required": true, "schema": {"type": "string"}}],
                "responses": {"200": {"description": "OK", "content": {"text/csv": {}}}}
            }},
            "/v1/things": {"post": {
                "operationId": "createThing",
                "requestBody": {"required": true, "content": {"application/json": {"schema": {"type": "object"}}}},
                "responses": {"200": {"description": "OK", "content": {"application/json": {}}}}
            }}
        }
    }))
    .unwrap();
    let source = ingest("shapes.json", &bytes).unwrap();
    let document: Value = serde_json::from_slice(&bytes).unwrap();
    Bundle {
        provider: "shapes".into(),
        source,
        inventory: inventory::extract(&document),
        auth_profile: "fixture.token".into(),
    }
}

fn messages() -> Engine {
    let selection = written(json!({
        "id": "messages.list", "operation_id": "listMessages", "effect": "read",
        "bounds": {"ids": {"minimum": 1, "maximum": 10}}
    }));
    Engine::new(&shapes_bundle(), "/v1", &[selection]).unwrap()
}

async fn refused_with_nothing_sent(engine: &Engine, id: &str, input: Value) -> ErrorCode {
    let http = reads(vec![]);
    let error = engine
        .read(http.as_ref(), "fixture", id, input.clone())
        .await
        .err()
        .unwrap_or_else(|| panic!("{input} was sent"));
    assert!(http.calls.lock().unwrap().is_empty(), "{input}");
    error.code
}

#[tokio::test]
async fn engine_sends_repeated_pairs() {
    let engine = messages();
    for (input, expected) in [
        (
            json!({"userId": "me", "labelIds": ["INBOX", "UNREAD"]}),
            vec![("labelIds", "INBOX"), ("labelIds", "UNREAD")],
        ),
        // Integers are sent as their decimal text, one pair each, in input order.
        (
            json!({"userId": "me", "ids": [3, "1", 2], "q": "is:unread"}),
            vec![("ids", "3"), ("ids", "1"), ("ids", "2"), ("q", "is:unread")],
        ),
        // One value is still one pair, as it was before arrays were read.
        (
            json!({"userId": "me", "labelIds": "INBOX,UNREAD"}),
            vec![("labelIds", "INBOX,UNREAD")],
        ),
        // An empty array sends nothing.
        (json!({"userId": "me", "labelIds": []}), vec![]),
    ] {
        let http = reads(vec![response(200, json!({"messages": []}))]);
        engine
            .read(http.as_ref(), "fixture", "messages.list", input.clone())
            .await
            .unwrap_or_else(|error| panic!("{input}: {error}"));
        let calls = http.calls.lock().unwrap();
        assert_eq!(calls.len(), 1, "{input}");
        assert_eq!(calls[0].0, ["users", "me", "messages"]);
        let expected: Vec<(String, String)> = expected
            .into_iter()
            .map(|(k, v)| (k.to_owned(), v.to_owned()))
            .collect();
        assert_eq!(calls[0].1, expected, "{input}");
    }
}

#[test]
fn engine_declares_array_schema() {
    let engine = messages();
    let declared = &engine.declarations(&[Effect::Read])[0].input_schema["properties"];
    // An array of the element type, or one scalar typed like the elements (a
    // comma-joined list of them included), which is what callers sent before
    // arrays were read.
    assert_eq!(
        declared["labelIds"],
        json!({"type": ["array", "string", "integer"], "items": {"type": ["string", "integer"]}})
    );
    assert_eq!(
        declared["ids"],
        json!({
            "type": ["array", "integer", "string"],
            "pattern": "^-?[0-9]+(,-?[0-9]+)*$",
            "items": {"anyOf": [{"type": "integer"}, {"type": "string", "pattern": "^-?[0-9]+$"}],
                      "minimum": 1, "maximum": 10},
            "minimum": 1, "maximum": 10
        })
    );
    // A parameter that is not repeated declares no array.
    assert_eq!(declared["q"], json!({"type": ["string", "integer"]}));
}

#[tokio::test]
async fn engine_refuses_array_on_scalar_parameter() {
    let engine = messages();
    for input in [
        json!({"userId": "me", "q": ["a", "b"]}),
        json!({"userId": "me", "q": []}),
        json!({"userId": ["me", "you"]}),
    ] {
        assert_eq!(
            refused_with_nothing_sent(&engine, "messages.list", input).await,
            ErrorCode::InvalidInput
        );
    }
}

#[tokio::test]
async fn engine_refuses_nested_array_element() {
    let engine = messages();
    for input in [
        json!({"userId": "me", "labelIds": [["INBOX"]]}),
        json!({"userId": "me", "labelIds": ["INBOX", {"id": "UNREAD"}]}),
        json!({"userId": "me", "labelIds": [null]}),
        json!({"userId": "me", "ids": [1, 2.5]}),
        json!({"userId": "me", "ids": [1, "two"]}),
    ] {
        assert_eq!(
            refused_with_nothing_sent(&engine, "messages.list", input).await,
            ErrorCode::InvalidInput
        );
    }
}

#[tokio::test]
async fn engine_bounds_apply_per_element() {
    let engine = messages();
    for ids in [
        json!([1, 11]),
        json!([0]),
        json!(["10", "11"]),
        json!([5, "-1"]),
    ] {
        assert_eq!(
            refused_with_nothing_sent(
                &engine,
                "messages.list",
                json!({"userId": "me", "ids": ids})
            )
            .await,
            ErrorCode::InvalidInput,
            "{ids}"
        );
    }
    let http = reads(vec![response(200, json!({}))]);
    engine
        .read(
            http.as_ref(),
            "fixture",
            "messages.list",
            json!({"userId": "me", "ids": [1, "10"]}),
        )
        .await
        .unwrap();
    assert_eq!(
        http.calls.lock().unwrap()[0].1,
        [
            ("ids".to_string(), "1".to_string()),
            ("ids".to_string(), "10".to_string())
        ]
    );
}

#[tokio::test]
async fn a_selection_required_parameter_is_declared_and_refused_before_any_request() {
    let selection = written(json!({
        "id": "about.get", "operation_id": "getAbout", "effect": "read", "required": ["fields"]
    }));
    let engine = Engine::new(&shapes_bundle(), "/v1", std::slice::from_ref(&selection)).unwrap();
    let declared = &engine.declarations(&[Effect::Read])[0].input_schema;
    assert_eq!(declared["required"], json!(["fields"]));
    assert_eq!(
        refused_with_nothing_sent(&engine, "about.get", json!({})).await,
        ErrorCode::InvalidInput
    );
    let http = reads(vec![response(200, json!({"user": {}}))]);
    engine
        .read(
            http.as_ref(),
            "fixture",
            "about.get",
            json!({"fields": "user"}),
        )
        .await
        .unwrap();
    assert_eq!(
        http.calls.lock().unwrap()[0].1,
        [("fields".to_string(), "user".to_string())]
    );
    // The field round-trips, and a selection without it serialises as before.
    assert_eq!(
        serde_json::to_value(&selection).unwrap()["required"],
        json!(["fields"])
    );
    let plain = serde_json::to_value(select("about.get", "getAbout", Effect::Read)).unwrap();
    assert!(plain.get("required").is_none(), "{plain}");
    // Without the selection's requirement the document's own applies: none.
    let open = Engine::new(
        &shapes_bundle(),
        "/v1",
        &[select("about.get", "getAbout", Effect::Read)],
    )
    .unwrap();
    assert_eq!(
        open.declarations(&[Effect::Read])[0].input_schema["required"],
        json!([])
    );
}

#[test]
fn a_selection_can_require_only_a_query_parameter_of_its_operation() {
    for (operation_id, name) in [
        ("getAbout", "nope"),
        ("getAbout", "trace"),
        ("listMessages", "userId"),
        ("getAbout", ""),
    ] {
        let selection = written(json!({
            "id": "one", "operation_id": operation_id, "effect": "read", "required": [name]
        }));
        let error = Engine::new(&shapes_bundle(), "/v1", &[selection])
            .err()
            .unwrap_or_else(|| panic!("`{name}` on `{operation_id}` loaded"));
        assert_eq!(error.code, ErrorCode::InvalidInput, "`{name}`");
        assert!(error.message.contains("required"), "{}", error.message);
    }
}

fn text_export() -> Engine {
    let selection = written(json!({
        "id": "file.export", "operation_id": "exportFile", "effect": "read",
        "response": "text",
        "rate_limit_reasons": ["rateLimitExceeded", "userRateLimitExceeded"]
    }));
    Engine::new(&shapes_bundle(), "/v1", &[selection]).unwrap()
}

#[tokio::test]
async fn an_empty_text_body_is_the_empty_string() {
    let http = reads(vec![raw(200, b"")]);
    let value = text_export()
        .read(
            http.as_ref(),
            "fixture",
            "file.export",
            json!({"fileId": "f"}),
        )
        .await
        .unwrap();
    assert_eq!(value["body"], json!(""));
    // An empty JSON body is still null: there is no JSON value to read.
    let engine = Engine::new(
        &shapes_bundle(),
        "/v1",
        &[select("about.get", "getAbout", Effect::Read)],
    )
    .unwrap();
    let http = reads(vec![raw(200, b"")]);
    let value = engine
        .read(http.as_ref(), "fixture", "about.get", json!({}))
        .await
        .unwrap();
    assert_eq!(value["body"], Value::Null);
}

fn quota(reason: &str) -> Value {
    json!({"error": {"code": 403, "message": "private provider detail",
        "errors": [{"domain": "usageLimits", "reason": reason, "message": "private provider detail"}]}})
}

#[tokio::test]
async fn a_declared_rate_limit_reason_on_a_403_is_rate_limited() {
    let engine = text_export();
    for body in [
        quota("userRateLimitExceeded"),
        quota("rateLimitExceeded"),
        json!({"error": {"errors": [{"reason": "other"}, {"reason": "rateLimitExceeded"}]}}),
        json!({"error": {"code": 403, "status": "rateLimitExceeded"}}),
    ] {
        let http = reads(vec![response(403, body.clone())]);
        let error = engine
            .read(
                http.as_ref(),
                "fixture",
                "file.export",
                json!({"fileId": "f"}),
            )
            .await
            .err()
            .unwrap();
        assert_eq!(error.code, ErrorCode::RateLimited, "{body}");
        assert!(!error.message.contains("private"), "{}", error.message);
    }
}

#[tokio::test]
async fn every_other_403_stays_forbidden() {
    let undeclared = Engine::new(
        &shapes_bundle(),
        "/v1",
        &[Selection {
            response: Some(ResponseKind::Text),
            ..select("file.export", "exportFile", Effect::Read)
        }],
    )
    .unwrap();
    for (engine, body) in [
        // A reason the selection does not declare.
        (
            text_export(),
            serde_json::to_vec(&quota("insufficientPermissions")).unwrap(),
        ),
        (
            text_export(),
            serde_json::to_vec(&json!({"error": {"status": "PERMISSION_DENIED"}})).unwrap(),
        ),
        // The reason, somewhere other than the two places it is read from.
        (
            text_export(),
            serde_json::to_vec(&json!({"reason": "rateLimitExceeded"})).unwrap(),
        ),
        (
            text_export(),
            serde_json::to_vec(&json!({"error": {"message": "rateLimitExceeded"}})).unwrap(),
        ),
        (
            text_export(),
            serde_json::to_vec(&json!({"error": {"errors": "rateLimitExceeded"}})).unwrap(),
        ),
        // A body that is not JSON, or no body.
        (text_export(), b"rateLimitExceeded".to_vec()),
        (text_export(), Vec::new()),
        // A selection that declares no reasons.
        (
            undeclared,
            serde_json::to_vec(&quota("rateLimitExceeded")).unwrap(),
        ),
    ] {
        let http = reads(vec![raw(403, &body)]);
        let error = engine
            .read(
                http.as_ref(),
                "fixture",
                "file.export",
                json!({"fileId": "f"}),
            )
            .await
            .err()
            .unwrap();
        assert_eq!(
            error.code,
            ErrorCode::Forbidden,
            "{}",
            String::from_utf8_lossy(&body)
        );
    }
    // A declared reason on another status is that status's answer.
    let http = reads(vec![response(400, quota("rateLimitExceeded"))]);
    let error = text_export()
        .read(
            http.as_ref(),
            "fixture",
            "file.export",
            json!({"fileId": "f"}),
        )
        .await
        .err()
        .unwrap();
    assert_eq!(error.code, ErrorCode::InvalidInput);
}

#[tokio::test]
async fn a_write_refused_for_a_declared_rate_limit_reason_is_refused_as_rate_limited() {
    let selection = written(json!({
        "id": "thing.create", "operation_id": "createThing", "effect": "write",
        "rate_limit_reasons": ["userRateLimitExceeded"]
    }));
    let engine = Engine::new(&shapes_bundle(), "/v1", &[selection]).unwrap();
    for (reason, expected) in [
        ("userRateLimitExceeded", ErrorCode::RateLimited),
        ("insufficientPermissions", ErrorCode::Forbidden),
    ] {
        let http = reads(vec![]);
        let prepared = engine
            .prepare(
                http.as_ref(),
                "fixture",
                "thing.create",
                json!({"body": {}}),
            )
            .await
            .unwrap();
        let sent: Sent = Arc::default();
        let outcome = prepared
            .execute(Box::new(Send {
                sent: sent.clone(),
                response: Some(response(403, quota(reason))),
            }))
            .await;
        match outcome {
            WriteOutcome::Refused(error) => assert_eq!(error.code, expected, "{reason}"),
            _ => panic!("{reason}: not refused"),
        }
    }
}

#[test]
fn a_rate_limit_reason_must_be_a_nonempty_string() {
    for reasons in [json!([""]), json!(["rateLimitExceeded", ""])] {
        let selection = written(json!({
            "id": "file.export", "operation_id": "exportFile", "effect": "read",
            "rate_limit_reasons": reasons
        }));
        let error = Engine::new(&shapes_bundle(), "/v1", &[selection])
            .err()
            .unwrap_or_else(|| panic!("{reasons} loaded"));
        assert_eq!(error.code, ErrorCode::InvalidInput);
    }
    let plain = serde_json::to_value(select("file.export", "exportFile", Effect::Read)).unwrap();
    assert!(plain.get("rate_limit_reasons").is_none(), "{plain}");
}

/// A write whose body is closed to `id`, as a draft send is.
fn closed_thing() -> Engine {
    let selection = written(json!({
        "id": "thing.create", "operation_id": "createThing", "effect": "write",
        "body_keys": ["id"]
    }));
    Engine::new(&shapes_bundle(), "/v1", &[selection]).unwrap()
}

/// `body_keys` closes the body: the declaration types it as an object with
/// exactly those properties and no others. The field round-trips, and a
/// selection without it serialises and declares its body as before.
#[test]
fn body_keys_close_the_declared_body() {
    let engine = closed_thing();
    let declared = &engine.declarations(&[Effect::Write])[0].input_schema;
    assert_eq!(
        declared["properties"]["body"],
        json!({"type": "object", "properties": {"id": {}}, "additionalProperties": false})
    );
    assert!(
        declared["required"]
            .as_array()
            .unwrap()
            .contains(&json!("body"))
    );
    let closed = written(json!({
        "id": "thing.create", "operation_id": "createThing", "effect": "write",
        "body_keys": ["id"]
    }));
    assert_eq!(
        serde_json::to_value(&closed).unwrap()["body_keys"],
        json!(["id"])
    );
    let plain = select("thing.create", "createThing", Effect::Write);
    assert!(
        serde_json::to_value(&plain)
            .unwrap()
            .get("body_keys")
            .is_none()
    );
    let open = Engine::new(&shapes_bundle(), "/v1", &[plain]).unwrap();
    assert_eq!(
        open.declarations(&[Effect::Write])[0].input_schema["properties"]["body"],
        json!({"type": "object"})
    );
}

/// A body key outside the closed set is `invalid_input` before any request:
/// no read and no write is sent. A body inside the set is sent unchanged.
#[tokio::test]
async fn a_body_key_outside_the_closed_set_is_refused_before_any_request() {
    let engine = closed_thing();
    for body in [
        json!({"id": "t-1", "message": {"raw": "x"}}),
        json!({"message": {"raw": "x"}}),
        json!({"id": "t-1", "": 1}),
        json!(["id"]),
    ] {
        let http = reads(vec![]);
        let error = engine
            .prepare(
                http.as_ref(),
                "fixture",
                "thing.create",
                json!({"body": body}),
            )
            .await
            .err()
            .unwrap_or_else(|| panic!("{body} was prepared"));
        assert_eq!(error.code, ErrorCode::InvalidInput, "{body}");
        assert!(http.calls.lock().unwrap().is_empty(), "{body}");
    }
    let http = reads(vec![]);
    let prepared = engine
        .prepare(
            http.as_ref(),
            "fixture",
            "thing.create",
            json!({"body": {"id": "t-1"}}),
        )
        .await
        .unwrap();
    let sent: Sent = Arc::default();
    let outcome = prepared
        .execute(Box::new(Send {
            sent: sent.clone(),
            response: Some(response(200, json!({"id": "t-1"}))),
        }))
        .await;
    assert!(matches!(outcome, WriteOutcome::Applied(Ok(_))));
    assert_eq!(sent.lock().unwrap()[0].2, json!({"id": "t-1"}));
}

/// `body_keys` is refused when the selection loads unless it closes a
/// write's JSON body with distinct plain names, and a guard that reads a
/// `body.<key>` must read a key the set admits. A guarded key is required in
/// the declared body.
#[test]
fn body_keys_are_refused_at_load_unless_they_close_a_write_body() {
    for (label, selection) in [
        (
            "a read",
            json!({"id": "about.get", "operation_id": "getAbout", "effect": "read",
                   "body_keys": ["id"]}),
        ),
        (
            "an empty name",
            json!({"id": "thing.create", "operation_id": "createThing", "effect": "write",
                   "body_keys": [""]}),
        ),
        (
            "a repeated name",
            json!({"id": "thing.create", "operation_id": "createThing", "effect": "write",
                   "body_keys": ["id", "id"]}),
        ),
        (
            "a dotted name",
            json!({"id": "thing.create", "operation_id": "createThing", "effect": "write",
                   "body_keys": ["message.raw"]}),
        ),
    ] {
        let error = Engine::new(&shapes_bundle(), "/v1", &[written(selection)])
            .err()
            .unwrap_or_else(|| panic!("{label} loaded"));
        assert_eq!(error.code, ErrorCode::InvalidInput, "{label}");
        assert!(error.message.contains("body_keys"), "{}", error.message);
    }
    let guarded = |keys: &[&str]| {
        written(json!({
            "id": "merge_request.merge", "operation_id": "mergeMergeRequest", "effect": "write",
            "body_keys": keys,
            "guard": {
                "preflight": {"operation_id": "getMergeRequest",
                              "values": {"id": "id", "merge_request_iid": "merge_request_iid"},
                              "checks": [{"pointer": "/sha", "expect": {"input": "body.sha"}}]},
                "postflight": {"checks": []}}
        }))
    };
    let error = Engine::new(&bundle(), "/api/v4", &[guarded(&["other"])])
        .err()
        .expect("a guard reading a body key the set refuses loaded");
    assert!(error.message.contains("body_keys"), "{}", error.message);
    // The key the guard compares is declared as the scalar it compares; a key
    // no guard reads takes any value and is not required.
    let engine = Engine::new(&bundle(), "/api/v4", &[guarded(&["sha", "note"])]).unwrap();
    assert_eq!(
        engine.declarations(&[Effect::Write])[0].input_schema["properties"]["body"],
        json!({"type": "object",
               "properties": {"sha": {"type": ["string", "integer", "boolean"]}, "note": {}},
               "required": ["sha"], "additionalProperties": false})
    );
}

/// A write closed to `id`, `flag`, `count` and `note`, typing the first three
/// and requiring `id`, as a pinned request body schema would.
fn typed_thing() -> Engine {
    let selection = written(json!({
        "id": "thing.create", "operation_id": "createThing", "effect": "write",
        "body_keys": ["id", "flag", "count", "note"],
        "body_types": {"id": "string", "flag": "boolean", "count": "integer"},
        "body_required": ["id"]
    }));
    Engine::new(&shapes_bundle(), "/v1", &[selection]).unwrap()
}

/// `body_types` declares a closed body key as exactly one JSON type, with no
/// string or number spelling of it, and `body_required` requires a key. The
/// declaration says both; the fields round-trip and are omitted when empty.
#[test]
fn body_types_and_body_required_declare_the_closed_body() {
    let engine = typed_thing();
    assert_eq!(
        engine.declarations(&[Effect::Write])[0].input_schema["properties"]["body"],
        json!({"type": "object",
               "properties": {"id": {"type": "string"}, "flag": {"type": "boolean"},
                              "count": {"type": "integer"}, "note": {}},
               "required": ["id"], "additionalProperties": false})
    );
    let typed = written(json!({
        "id": "thing.create", "operation_id": "createThing", "effect": "write",
        "body_keys": ["id", "flag"], "body_types": {"flag": "boolean"}, "body_required": ["id"]
    }));
    let value = serde_json::to_value(&typed).unwrap();
    assert_eq!(value["body_types"], json!({"flag": "boolean"}));
    assert_eq!(value["body_required"], json!(["id"]));
    let plain = serde_json::to_value(select("thing.create", "createThing", Effect::Write)).unwrap();
    assert!(plain.get("body_types").is_none(), "{plain}");
    assert!(plain.get("body_required").is_none(), "{plain}");
}

/// A body missing a required key, or carrying a typed key as any other JSON
/// type (a string `"true"` for a boolean, `1` for a boolean, `"3"` or `1.5`
/// for an integer, `null` for anything), is `invalid_input` before any
/// request. A body that fits is sent unchanged, an untyped key with any value.
#[tokio::test]
async fn a_body_missing_a_required_key_or_mistyped_is_refused_before_any_request() {
    let engine = typed_thing();
    for body in [
        json!({}),
        json!({"flag": true}),
        json!({"id": null}),
        json!({"id": 7}),
        json!({"id": "t-1", "flag": "true"}),
        json!({"id": "t-1", "flag": 1}),
        json!({"id": "t-1", "flag": null}),
        json!({"id": "t-1", "count": "3"}),
        json!({"id": "t-1", "count": 1.5}),
        json!({"id": "t-1", "count": true}),
    ] {
        let http = reads(vec![]);
        let error = engine
            .prepare(
                http.as_ref(),
                "fixture",
                "thing.create",
                json!({"body": body}),
            )
            .await
            .err()
            .unwrap_or_else(|| panic!("{body} was prepared"));
        assert_eq!(error.code, ErrorCode::InvalidInput, "{body}");
        assert!(http.calls.lock().unwrap().is_empty(), "{body}");
    }
    let body = json!({"id": "t-1", "flag": false, "count": 3, "note": {"any": [1]}});
    let http = reads(vec![]);
    let prepared = engine
        .prepare(
            http.as_ref(),
            "fixture",
            "thing.create",
            json!({"body": body.clone()}),
        )
        .await
        .unwrap();
    let sent: Sent = Arc::default();
    let outcome = prepared
        .execute(Box::new(Send {
            sent: sent.clone(),
            response: Some(response(200, json!({"id": "t-1"}))),
        }))
        .await;
    assert!(matches!(outcome, WriteOutcome::Applied(Ok(_))));
    assert_eq!(sent.lock().unwrap()[0].2, body);
}

/// `body_types` and `body_required` name keys of a closed body: either one
/// without `body_keys`, naming a key outside them, or repeating a key is
/// refused when the selection loads, and so is a guard reading a path nested
/// under a key typed as a scalar. A guarded key typed by the selection is
/// declared as that type.
#[test]
fn body_types_and_body_required_are_refused_at_load_unless_they_name_closed_keys() {
    for (label, selection) in [
        (
            "types without body_keys",
            json!({"id": "thing.create", "operation_id": "createThing", "effect": "write",
                   "body_types": {"id": "string"}}),
        ),
        (
            "required without body_keys",
            json!({"id": "thing.create", "operation_id": "createThing", "effect": "write",
                   "body_required": ["id"]}),
        ),
        (
            "a type for a key outside the set",
            json!({"id": "thing.create", "operation_id": "createThing", "effect": "write",
                   "body_keys": ["id"], "body_types": {"other": "string"}}),
        ),
        (
            "a required key outside the set",
            json!({"id": "thing.create", "operation_id": "createThing", "effect": "write",
                   "body_keys": ["id"], "body_required": ["other"]}),
        ),
        (
            "a repeated required key",
            json!({"id": "thing.create", "operation_id": "createThing", "effect": "write",
                   "body_keys": ["id"], "body_required": ["id", "id"]}),
        ),
    ] {
        let error = Engine::new(&shapes_bundle(), "/v1", &[written(selection)])
            .err()
            .unwrap_or_else(|| panic!("{label} loaded"));
        assert_eq!(error.code, ErrorCode::InvalidInput, "{label}");
        assert!(
            error.message.contains("body_types") || error.message.contains("body_required"),
            "{label}: {}",
            error.message
        );
    }
    let guarded = |pointer_input: &str| {
        written(json!({
            "id": "merge_request.merge", "operation_id": "mergeMergeRequest", "effect": "write",
            "body_keys": ["sha"], "body_types": {"sha": "string"},
            "guard": {
                "preflight": {"operation_id": "getMergeRequest",
                              "values": {"id": "id", "merge_request_iid": "merge_request_iid"},
                              "checks": [{"pointer": "/sha", "expect": {"input": pointer_input}}]},
                "postflight": {"checks": []}}
        }))
    };
    let error = Engine::new(&bundle(), "/api/v4", &[guarded("body.sha.value")])
        .err()
        .expect("a guard reading under a scalar-typed key loaded");
    assert!(error.message.contains("body_types"), "{}", error.message);
    let engine = Engine::new(&bundle(), "/api/v4", &[guarded("body.sha")]).unwrap();
    assert_eq!(
        engine.declarations(&[Effect::Write])[0].input_schema["properties"]["body"],
        json!({"type": "object", "properties": {"sha": {"type": "string"}},
               "required": ["sha"], "additionalProperties": false})
    );
}

#[tokio::test]
async fn a_scalar_for_a_repeated_parameter_is_typed_like_its_elements() {
    let engine = Engine::new(
        &shapes_bundle(),
        "/v1",
        &[select("messages.list", "listMessages", Effect::Read)],
    )
    .unwrap();
    // One element, or a comma-joined list of elements: sent as one pair, as
    // it was given.
    for (name, value, sent) in [
        ("ids", json!(3), "3"),
        ("ids", json!("-3"), "-3"),
        ("ids", json!("1,2,-3"), "1,2,-3"),
        ("flags", json!(true), "true"),
        ("flags", json!("true,false"), "true,false"),
        ("labelIds", json!("INBOX,UNREAD"), "INBOX,UNREAD"),
        ("labelIds", json!("any text at all"), "any text at all"),
        ("labelIds", json!(7), "7"),
    ] {
        let http = reads(vec![response(200, json!({}))]);
        engine
            .read(
                http.as_ref(),
                "fixture",
                "messages.list",
                json!({"userId": "me", name: value}),
            )
            .await
            .unwrap_or_else(|error| panic!("{name}={value}: {error}"));
        assert_eq!(
            http.calls.lock().unwrap()[0].1,
            [(name.to_string(), sent.to_string())],
            "{name}={value}"
        );
    }
    // Anything else is refused as invalid input with nothing sent.
    for (name, value) in [
        ("ids", json!("abc")),
        ("ids", json!("1,,2")),
        ("ids", json!("1, 2")),
        ("ids", json!("")),
        ("ids", json!(2.5)),
        ("ids", json!(2.0)),
        ("ids", json!(true)),
        ("flags", json!("yes")),
        ("flags", json!("true,1")),
        ("flags", json!(1)),
        ("labelIds", json!(true)),
        ("labelIds", json!(1.5)),
    ] {
        assert_eq!(
            refused_with_nothing_sent(
                &engine,
                "messages.list",
                json!({"userId": "me", name: value})
            )
            .await,
            ErrorCode::InvalidInput,
            "{name}={value}"
        );
    }
}

#[tokio::test]
async fn a_required_repeated_parameter_declares_and_refuses_an_empty_list() {
    let selection = written(json!({
        "id": "messages.list", "operation_id": "listMessages", "effect": "read",
        "required": ["labelIds"]
    }));
    let engine = Engine::new(&shapes_bundle(), "/v1", &[selection]).unwrap();
    let schema = &engine.declarations(&[Effect::Read])[0].input_schema;
    assert_eq!(schema["properties"]["labelIds"]["minItems"], json!(1));
    assert!(connectors_sdk::validate(schema, &json!({"userId": "me", "labelIds": []})).is_err());
    assert_eq!(
        refused_with_nothing_sent(
            &engine,
            "messages.list",
            json!({"userId": "me", "labelIds": []})
        )
        .await,
        ErrorCode::InvalidInput
    );
    // Optional, it declares no minimum and an empty list sends nothing.
    assert!(
        messages().declarations(&[Effect::Read])[0].input_schema["properties"]["labelIds"]
            .get("minItems")
            .is_none()
    );
}

/// A `429` naming `retry_after` in a `Retry-After` header, or naming nothing.
fn limited(retry_after: Option<&str>) -> HttpResponse {
    HttpResponse {
        status: 429,
        headers: retry_after
            .map(|value| BTreeMap::from([("retry-after".to_owned(), value.to_owned())]))
            .unwrap_or_default(),
        body: serde_json::to_vec(&json!({"message": "Too Many Requests"})).unwrap(),
    }
}

async fn refused_read(answer: HttpResponse) -> (connectors_core::Error, usize) {
    let engine = Engine::new(&bundle(), "/api/v4", &selections()).unwrap();
    let http = reads(vec![answer]);
    let error = engine
        .read(
            http.as_ref(),
            "fixture",
            "merge_requests.list",
            json!({"id": "org/project"}),
        )
        .await
        .expect_err("a 429 is a refusal");
    let calls = http.calls.lock().unwrap().len();
    (error, calls)
}

/// story:catalog-honours-retry-after: a read answered `429` is the provider's
/// `rate_limited` and carries the delay its `Retry-After` names, in either
/// form RFC 9110 gives it (delta-seconds or an HTTP-date, the latter in all
/// three date formats a recipient must accept), in whole seconds rounded up.
/// The engine sends exactly one request; waiting is the composition's.
#[tokio::test]
async fn a_read_answered_429_carries_the_delay_its_retry_after_names() {
    let now = std::time::SystemTime::now();
    let ahead = httpdate::fmt_http_date(now + std::time::Duration::from_secs(90));
    for (header, low, high) in [
        ("7".to_owned(), 7, 7),
        ("0".to_owned(), 0, 0),
        (" 120 ".to_owned(), 120, 120),
        ("4294967295".to_owned(), 4_294_967_295, 4_294_967_295),
        (ahead, 89, 91),
        // A date already past names no wait at all.
        ("Sun, 06 Nov 1994 08:49:37 GMT".to_owned(), 0, 0),
        ("Sunday, 06-Nov-94 08:49:37 GMT".to_owned(), 0, 0),
        ("Sun Nov  6 08:49:37 1994".to_owned(), 0, 0),
    ] {
        let (error, calls) = refused_read(limited(Some(&header))).await;
        assert_eq!(error.code, ErrorCode::RateLimited, "{header:?}");
        assert!(error.upstream_answer, "{header:?}");
        let named = error.retry_after_seconds.unwrap_or_else(|| {
            panic!("`Retry-After: {header}` named no delay");
        });
        assert!((low..=high).contains(&named), "{header:?} named {named}");
        assert_eq!(calls, 1, "{header:?}");
    }
}

/// story:catalog-honours-retry-after: a `Retry-After` the engine cannot read,
/// or none, leaves the refusal without a delay, after one request. A delay
/// that does not fit 32 bits is not read either.
#[tokio::test]
async fn a_429_without_a_readable_retry_after_names_no_delay() {
    let mut answers = vec![limited(None)];
    for header in [
        "",
        "soon",
        "-1",
        "+7",
        "1.5",
        "7 seconds",
        "0x10",
        "4294967296",
        "99999999999999999999999",
        "Sun, 06 Nov 1994 08:49:37 PST",
        "2026-10-06T12:00:00Z",
        // Two disagreeing field lines, as the transport combines them.
        "3600, 1",
    ] {
        answers.push(limited(Some(header)));
    }
    for answer in answers {
        let header = answer.headers.get("retry-after").cloned();
        let (error, calls) = refused_read(answer).await;
        assert_eq!(error.code, ErrorCode::RateLimited, "{header:?}");
        assert_eq!(error.retry_after_seconds, None, "{header:?}");
        assert_eq!(calls, 1, "{header:?}");
    }
    // Only a 429 is read for a delay: an unavailable answer carries none.
    let mut unavailable = limited(Some("7"));
    unavailable.status = 503;
    let (error, _) = refused_read(unavailable).await;
    assert_eq!(error.code, ErrorCode::Unavailable);
    assert_eq!(error.retry_after_seconds, None);
}

/// story:catalog-honours-retry-after: a write answered `429` is sent once
/// and never again, whatever delay it names; its outcome stays `unknown`, as
/// the guides say, because a `429` is not among the documented definite
/// refusals of a write.
#[tokio::test]
async fn a_write_answered_429_is_sent_once_and_not_retried() {
    let engine = Engine::new(&bundle(), "/api/v4", &selections()).unwrap();
    for header in [Some("1"), Some("0"), None] {
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
        let sent: Sent = Arc::new(Mutex::new(Vec::new()));
        let outcome = prepared
            .execute(Box::new(Send {
                sent: sent.clone(),
                response: Some(limited(header)),
            }))
            .await;
        assert_eq!(classify(outcome), "unknown", "{header:?}");
        assert_eq!(sent.lock().unwrap().len(), 1, "{header:?}");
        // Only the preflight read went through the read port.
        assert_eq!(http.calls.lock().unwrap().len(), 1, "{header:?}");
    }
}
