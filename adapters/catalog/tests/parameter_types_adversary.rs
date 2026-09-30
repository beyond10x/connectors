//! Adversary cases for story:catalog-parameters-declare-their-type.
use connectors_catalog::bundle;
use connectors_catalog_provider::{Engine, Selection};
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
fn reads() -> Reads {
    Reads {
        responses: Mutex::new(
            [HttpResponse {
                status: 200,
                headers: Default::default(),
                body: b"[]".to_vec(),
            }]
            .into(),
        ),
        calls: Mutex::new(Vec::new()),
    }
}
fn engine(provider: &str, base: &str) -> Engine {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let file: Value = serde_json::from_slice(
        &std::fs::read(root.join(format!("providers/{provider}/operations.json"))).unwrap(),
    )
    .unwrap();
    let selections: Vec<Selection> = serde_json::from_value(file["operations"].clone()).unwrap();
    let bundle = bundle::load(&root.join("generated/bundles"), provider).unwrap();
    Engine::new(&bundle, base, &selections).unwrap()
}

/// The pinned GitLab document calls `id` "The project ID or URL-encoded path"
/// (example "11") and types it `string` on these four operations, `oneOf
/// [string, integer]` on the others. At the base commit every one of them took
/// a numeric project id, as `project.events` still does
/// (tests/gitlab_repository_reads_adversary.rs sends `{"id": 7}`). A caller that
/// feeds a response's numeric `project_id` back in must not be refused on some
/// GitLab reads and accepted on others.
#[tokio::test]
async fn adversary_numeric_project_id_is_still_accepted_where_the_document_types_it_string() {
    let gitlab = engine("gitlab", "/api/v4");
    let mut refused = Vec::new();
    for (id, input) in [
        ("project.events", json!({"id": 7})),
        ("pipelines.list", json!({"id": 7})),
        ("pipeline.get", json!({"id": 7, "pipeline_id": 19})),
        ("pipeline.jobs", json!({"id": 7, "pipeline_id": 19})),
        (
            "file.get",
            json!({"id": 7, "file_path": ".gitlab-ci.yml", "ref": "main"}),
        ),
    ] {
        let http = reads();
        match gitlab.read(&http, "one", id, input.clone()).await {
            Ok(_) => {
                let calls = http.calls.lock().unwrap().clone();
                assert_eq!(calls[0].0[1], "7", "{id} {input}");
            }
            Err(error) => refused.push(format!("{id} {input}: {error:?}")),
        }
    }
    assert!(
        refused.is_empty(),
        "numeric project id refused: {refused:#?}"
    );
}

/// The descriptor now says `integer`, and the doc says any other value is
/// refused before any request. JSON Schema counts `1.0` as an integer, so the
/// schema admits it; the engine must then send `1`, or refuse it, never `1.0`.
#[tokio::test]
async fn adversary_a_declared_integer_is_never_sent_as_a_float() {
    let mut escaped = Vec::new();
    let gitlab = engine("gitlab", "/api/v4");
    for (id, input) in [
        ("issues.list", json!({"id": "org/project", "page": 2.0})),
        (
            "issues.list",
            json!({"id": "org/project", "closed_by_id": 1e2}),
        ),
        ("job.get", json!({"id": "org/project", "job_id": 23.0})),
    ] {
        let http = reads();
        let outcome = gitlab.read(&http, "one", id, input.clone()).await;
        let calls = http.calls.lock().unwrap().clone();
        let refused_cleanly = calls.is_empty()
            && outcome.as_ref().err().map(|e| &e.code) == Some(&ErrorCode::InvalidInput);
        let sent_float = calls.iter().any(|(path, query)| {
            path.iter().any(|s| s.contains('.'))
                || query
                    .iter()
                    .any(|(_, v)| v.contains('.') || v.contains('e'))
        });
        if !refused_cleanly && sent_float {
            escaped.push(format!("{id} {input}: sent {calls:?}"));
        }
    }
    assert!(
        escaped.is_empty(),
        "non-integer text sent for a declared integer: {escaped:#?}"
    );
}

/// Boundary forms the implementor's cases do not name. Expected green.
#[tokio::test]
async fn adversary_integer_and_boolean_boundary_forms_are_refused_before_any_request() {
    let gitlab = engine("gitlab", "/api/v4");
    let mut escaped = Vec::new();
    for input in [
        json!({"id": "o/p", "page": "+1"}),
        json!({"id": "o/p", "page": " 1"}),
        json!({"id": "o/p", "page": "1 "}),
        json!({"id": "o/p", "page": "1\n"}),
        json!({"id": "o/p", "page": "1e2"}),
        json!({"id": "o/p", "page": "0x10"}),
        json!({"id": "o/p", "page": "１"}),
        json!({"id": "o/p", "page": "-"}),
        json!({"id": "o/p", "page": null}),
        json!({"id": "o/p", "page": [1]}),
        json!({"id": "o/p", "with_labels_details": "TRUE"}),
        json!({"id": "o/p", "with_labels_details": "1"}),
        json!({"id": "o/p", "with_labels_details": 1}),
        json!({"id": "o/p", "with_labels_details": "True"}),
        json!({"id": "o/p", "state": 1.5}),
        json!({"id": "o/p", "per_page": "abc"}),
        json!({"id": "o/p", "per_page": "1.0"}),
        json!({"id": "o/p", "per_page": 1.5}),
    ] {
        let http = reads();
        let outcome = gitlab
            .read(&http, "one", "issues.list", input.clone())
            .await;
        let calls = http.calls.lock().unwrap().clone();
        if !calls.is_empty()
            || outcome.as_ref().err().map(|e| &e.code) != Some(&ErrorCode::InvalidInput)
        {
            escaped.push(format!("{input}: {outcome:?}, sent {calls:?}"));
        }
    }
    assert!(escaped.is_empty(), "reached the provider: {escaped:#?}");
}
