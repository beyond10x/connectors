//! The engine over a small fixture bundle: declarations, one bound read, and a
//! guarded write classified across the provider's answers.
use connectors_catalog::{bundle::Bundle, ingest, inventory};
use connectors_catalog_provider::{Effect, Engine, Guard, Postflight, Preflight, Selection};
use connectors_core::{ErrorCode, Result};
use connectors_sdk::{
    AuthenticatedHttp, AuthenticatedWrite, HttpResponse, WriteMethod, WriteOutcome,
};
use serde_json::{Value, json};
use std::{
    collections::{BTreeMap, VecDeque},
    sync::{
        Arc, Mutex,
        atomic::{AtomicUsize, Ordering},
    },
};

const SHA: &str = "0123456789abcdef0123456789abcdef01234567";

fn document() -> Vec<u8> {
    serde_json::to_vec(&json!({
        "openapi": "3.0.0",
        "info": {"title": "fixture", "version": "1"},
        "paths": {
            "/api/v4/projects/{id}/merge_requests": {
                "post": {
                    "operationId": "createMergeRequest",
                    "parameters": [{"name": "id", "in": "path", "required": true, "schema": {"type": "string"}}],
                    "requestBody": {"required": true, "content": {"application/json": {"schema": {"type": "object"}}}},
                    "responses": {"201": {"description": "Created"}}
                },
                "get": {
                    "operationId": "listMergeRequests",
                    "parameters": [
                        {"name": "id", "in": "path", "required": true, "schema": {"type": "string"}},
                        {"name": "state", "in": "query", "required": false, "schema": {"type": "string"}}
                    ],
                    "responses": {"200": {"description": "OK"}}
                }
            },
            "/api/v4/projects/{id}/repository/branches/{branch}": {
                "get": {
                    "operationId": "getBranch",
                    "parameters": [
                        {"name": "id", "in": "path", "required": true, "schema": {"type": "string"}},
                        {"name": "branch", "in": "path", "required": true, "schema": {"type": "string"}}
                    ],
                    "responses": {"200": {"description": "OK"}}
                }
            },
            "/other/{id}": {
                "get": {"operationId": "outside", "parameters": [{"name": "id", "in": "path", "required": true}], "responses": {}}
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

fn selections() -> Vec<Selection> {
    vec![
        Selection {
            id: "merge_requests.list".into(),
            operation_id: "listMergeRequests".into(),
            effect: Effect::Read,
            description: None,
            guard: None,
        },
        Selection {
            id: "merge_request.create".into(),
            operation_id: "createMergeRequest".into(),
            effect: Effect::Write,
            description: Some("Open a merge request at a pinned source head".into()),
            guard: Some(Guard {
                preflight: Preflight {
                    operation_id: "getBranch".into(),
                    values: BTreeMap::from([
                        ("id".to_string(), "id".to_string()),
                        ("branch".to_string(), "body.source_branch".to_string()),
                    ]),
                    pointer: "/commit/id".into(),
                    expect: "sha".into(),
                },
                postflight: Postflight {
                    pointer: "/sha".into(),
                    expect: "sha".into(),
                },
            }),
        },
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
struct Send {
    calls: Arc<AtomicUsize>,
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
        self.calls.fetch_add(1, Ordering::SeqCst);
        assert_eq!(method, WriteMethod::Post);
        assert_eq!(path, ["projects", "org/project", "merge_requests"]);
        assert!(query.is_empty());
        assert_eq!(
            body,
            &json!({"source_branch": "fix", "target_branch": "main", "title": "t"})
        );
        self.response
            .ok_or_else(connectors_core::Error::unavailable)
    }
}
fn input() -> Value {
    json!({"id": "org/project", "sha": SHA, "body": {"source_branch": "fix", "target_branch": "main", "title": "t"}})
}
fn opened(sha: &str) -> Value {
    json!({"id": 10, "iid": 7, "sha": sha, "state": "opened", "title": "t"})
}

#[test]
fn declarations_carry_parameters_body_and_guard_inputs_and_refuse_bad_selections() {
    let engine = Engine::new(&bundle(), "/api/v4", &selections()).unwrap();
    let reads = engine.declarations(&[Effect::Read]);
    assert_eq!(reads.len(), 1);
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
    for (id, operation_id, effect) in [
        ("missing", "noSuchOperation", Effect::Read),
        ("wrong_effect", "createMergeRequest", Effect::Read),
        ("wrong_effect_two", "listMergeRequests", Effect::Write),
        ("outside", "outside", Effect::Read),
        ("bad id", "listMergeRequests", Effect::Read),
    ] {
        let selection = Selection {
            id: id.into(),
            operation_id: operation_id.into(),
            effect,
            description: None,
            guard: None,
        };
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
    let http = reads(vec![HttpResponse {
        status: 404,
        headers: Default::default(),
        body: b"private".to_vec(),
    }]);
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
async fn guarded_write_refuses_before_dispatch_and_classifies_after() {
    let engine = Engine::new(&bundle(), "/api/v4", &selections()).unwrap();
    // The preflight reads the branch the input names.
    let http = reads(vec![response(
        200,
        json!({"name": "fix", "commit": {"id": SHA}}),
    )]);
    engine
        .prepare(http.as_ref(), "fixture", "merge_request.create", input())
        .await
        .unwrap();
    assert_eq!(
        http.calls.lock().unwrap()[0].0,
        ["projects", "org/project", "repository", "branches", "fix"]
    );
    // A head that already differs, or a branch that is absent, is refused with no write.
    for reply in [
        response(200, json!({"commit": {"id": "a".repeat(40)}})),
        HttpResponse {
            status: 404,
            headers: Default::default(),
            body: Vec::new(),
        },
    ] {
        let http = reads(vec![reply]);
        let error = engine
            .prepare(http.as_ref(), "fixture", "merge_request.create", input())
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
        (
            Some(HttpResponse {
                status: 201,
                headers: Default::default(),
                body: b"not json".to_vec(),
            }),
            "unknown",
        ),
    ];
    for status in [
        400, 401, 403, 404, 405, 409, 410, 412, 415, 422, 429, 500, 502, 307, 202,
    ] {
        cases.push((
            Some(HttpResponse {
                status,
                headers: Default::default(),
                body: b"private provider error".to_vec(),
            }),
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
            .prepare(http.as_ref(), "fixture", "merge_request.create", input())
            .await
            .unwrap();
        let calls = Arc::new(AtomicUsize::new(0));
        let outcome = prepared
            .execute(Box::new(Send {
                calls: calls.clone(),
                response: reply,
            }))
            .await;
        let actual = match outcome {
            WriteOutcome::Applied(Ok(value)) => {
                assert_eq!(value["body"]["iid"], 7);
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
        };
        assert_eq!(actual, expected);
        assert_eq!(calls.load(Ordering::SeqCst), 1);
    }
}
