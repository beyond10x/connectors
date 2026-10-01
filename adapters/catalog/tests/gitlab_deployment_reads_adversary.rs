//! Adversary cases for story:catalog-gitlab-deployment-reads: `deployments.list`
//! driven from the pinned document (`adapters/gitlab/upstream/openapi_v3.yaml`,
//! `getApiV4ProjectsIdDeployments`) rather than from the unit's own tests.
use connectors_catalog::bundle;
use connectors_catalog_provider::{Effect, Engine, Selection};
use connectors_core::{ErrorCode, Result};
use connectors_sdk::{AuthenticatedHttp, HttpResponse};
use serde_json::{Value, json};
use std::{collections::VecDeque, path::Path, sync::Mutex};

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
fn raw(responses: Vec<(u16, Vec<u8>)>) -> Reads {
    Reads {
        responses: Mutex::new(
            responses
                .into_iter()
                .map(|(status, body)| HttpResponse {
                    status,
                    headers: Default::default(),
                    body,
                })
                .collect(),
        ),
        calls: Mutex::new(Vec::new()),
    }
}
fn ok(body: Value) -> Reads {
    raw(vec![(200, serde_json::to_vec(&body).unwrap())])
}
fn engine() -> Engine {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let file: Value = serde_json::from_slice(
        &std::fs::read(root.join("providers/gitlab/operations.json")).unwrap(),
    )
    .unwrap();
    let shipped: Vec<Selection> = serde_json::from_value(file["operations"].clone()).unwrap();
    let bundle = bundle::load(&root.join("generated/bundles"), "gitlab").unwrap();
    Engine::new(&bundle, "/api/v4", &shipped).unwrap()
}

/// The pinned `id` is `oneOf: [string, integer]`: a JSON integer project id is
/// accepted and sent as its decimal text in the path. With only `id` given,
/// nothing else is sent: neither the pinned defaults (`page=1`, `per_page=20`,
/// `order_by=id`, `sort=asc`) nor the selection's `per_page` cap.
#[tokio::test]
async fn adversary_integer_id_alone_sends_the_bare_route() {
    let engine = engine();
    let http = ok(json!([]));
    let result = engine
        .read(&http, "one", "deployments.list", json!({"id": 42}))
        .await
        .unwrap();
    assert_eq!(result["status"], 200);
    assert_eq!(result["body"], json!([]));
    let calls = http.calls.lock().unwrap().clone();
    assert_eq!(
        calls,
        vec![(
            vec![
                "projects".to_string(),
                "42".to_string(),
                "deployments".to_string()
            ],
            vec![]
        )]
    );
}

/// `id` is the only required input: without it nothing is sent.
#[tokio::test]
async fn adversary_missing_id_is_refused_before_any_request() {
    let engine = engine();
    let http = ok(json!([]));
    let error = engine
        .read(
            &http,
            "one",
            "deployments.list",
            json!({"status": "success"}),
        )
        .await
        .unwrap_err();
    assert_eq!(error.code, ErrorCode::InvalidInput);
    assert!(http.calls.lock().unwrap().is_empty());
}

/// `per_page` as decimal text at both edges is sent as given; text just
/// outside, `-0`, a signed or padded number and `null` are refused unsent.
#[tokio::test]
async fn adversary_per_page_decimal_text_edges() {
    let engine = engine();
    for per_page in ["1", "100", "001", "0100"] {
        let http = ok(json!([]));
        engine
            .read(
                &http,
                "one",
                "deployments.list",
                json!({"id": "org/project", "per_page": per_page}),
            )
            .await
            .unwrap_or_else(|e| panic!("per_page={per_page}: {e:?}"));
        let calls = http.calls.lock().unwrap().clone();
        assert_eq!(
            calls[0].1,
            vec![("per_page".to_string(), per_page.to_string())],
            "per_page={per_page}"
        );
    }
    for per_page in [
        json!("0"),
        json!("-0"),
        json!("101"),
        json!("+1"),
        json!(" 1"),
        json!("1e2"),
        json!(null),
        json!(true),
        json!([10]),
        json!(100.0),
        json!(u64::MAX),
        json!(i64::MIN),
    ] {
        let http = ok(json!([]));
        let error = engine
            .read(
                &http,
                "one",
                "deployments.list",
                json!({"id": "org/project", "per_page": per_page}),
            )
            .await
            .unwrap_err();
        assert_eq!(error.code, ErrorCode::InvalidInput, "per_page={per_page}");
        assert!(http.calls.lock().unwrap().is_empty(), "per_page={per_page}");
    }
}

/// The body is GitLab's, returned unchanged, including the shapes the unit's
/// fixture does not carry: a deployment with no job (`deployable: null`), an
/// environment name outside ASCII, an id above `i64::MAX`, keys the pinned
/// entity does not declare and an empty page.
#[tokio::test]
async fn adversary_body_is_returned_unchanged_for_unusual_records() {
    let engine = engine();
    let body = json!([
        {"id": u64::MAX, "iid": 1, "status": "blocked",
         "environment": {"id": 1, "name": "prüfung/ü-1", "tier": null},
         "deployable": null,
         "approvals": [], "pending_approval_count": 0,
         "x-unknown": {"nested": [1, "two", null, false]}},
        {"id": 2, "environment": {"id": 2, "name": ""},
         "deployable": {"id": 0, "pipeline": null}}
    ]);
    let http = ok(body.clone());
    let result = engine
        .read(
            &http,
            "one",
            "deployments.list",
            json!({"id": "org/project", "status": "blocked"}),
        )
        .await
        .unwrap();
    assert_eq!(result["body"], body);
    assert_eq!(result["body"][0]["id"].as_u64(), Some(u64::MAX));
    assert_eq!(result["body"][0]["environment"]["name"], "prüfung/ü-1");
    assert!(result["body"][0]["deployable"].is_null());
    assert_eq!(result["body"][1]["deployable"]["id"], 0);

    let http = ok(json!([]));
    let empty = engine
        .read(
            &http,
            "one",
            "deployments.list",
            json!({"id": "org/project", "page": 9999}),
        )
        .await
        .unwrap();
    assert_eq!(empty["body"], json!([]));
}

/// An unreadable project, as GitLab answers it (401, 403, 404, with a JSON
/// message or an HTML page), is refused by `deployments.list` with the code
/// every other GitLab project list read answers for the same response, not
/// only `pipelines.list`.
#[tokio::test]
async fn adversary_unreadable_project_matches_every_project_list_read() {
    let engine = engine();
    let responses: Vec<(u16, Vec<u8>)> = vec![
        (401, br#"{"message":"401 Unauthorized"}"#.to_vec()),
        (403, br#"{"message":"403 Forbidden"}"#.to_vec()),
        (404, br#"{"message":"404 Project Not Found"}"#.to_vec()),
        (404, b"<html><body>Not Found</body></html>".to_vec()),
        (403, Vec::new()),
    ];
    for (status, body) in responses {
        let mut codes = Vec::new();
        for id in [
            "deployments.list",
            "pipelines.list",
            "tags.list",
            "commits.list",
            "merge_requests.list",
            "issues.list",
            "releases.list",
        ] {
            let http = raw(vec![(status, body.clone())]);
            let error = engine
                .read(&http, "one", id, json!({"id": "org/hidden"}))
                .await
                .expect_err("a refusal");
            assert_eq!(http.calls.lock().unwrap().len(), 1, "`{id}` {status}");
            codes.push((id, error.code));
        }
        let deployments = codes[0].1.clone();
        for (id, code) in &codes[1..] {
            assert_eq!(
                deployments.clone(),
                code.clone(),
                "{status} {:?}: `deployments.list` vs `{id}`",
                String::from_utf8_lossy(&body)
            );
        }
        assert_ne!(deployments, ErrorCode::InvalidInput, "{status}");
    }
}

/// An input the pinned document does not declare (a misspelt filter) is
/// refused rather than silently dropped, which would return every deployment
/// as if the filter had matched them all.
#[tokio::test]
async fn adversary_undeclared_filter_is_refused_not_dropped() {
    let engine = engine();
    for input in [
        json!({"id": "org/project", "environment_name": "production"}),
        json!({"id": "org/project", "environment_scope": "production"}),
    ] {
        let http = ok(json!([]));
        let error = engine
            .read(&http, "one", "deployments.list", input.clone())
            .await
            .unwrap_err();
        assert_eq!(error.code, ErrorCode::InvalidInput, "{input}");
        assert!(http.calls.lock().unwrap().is_empty(), "{input}");
    }
}

/// The declaration does not offer `deployments.list` as a write, and its
/// output is the provider envelope, not GitLab's `APIEntitiesDeployment`.
#[test]
fn adversary_declared_once_as_a_read() {
    let engine = engine();
    let reads: Vec<_> = engine
        .declarations(&[Effect::Read])
        .into_iter()
        .filter(|o| o.id == "deployments.list")
        .collect();
    assert_eq!(reads.len(), 1);
    assert_eq!(reads[0].input_schema["required"], json!(["id"]));
}
