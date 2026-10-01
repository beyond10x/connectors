//! story:catalog-gitlab-deployment-reads: `deployments.list` in the shipped
//! GitLab selection set, driven through the engine over the committed bundle
//! with a recording transport: its declaration and pinned input types, the
//! `per_page` bounds, the exact request it sends, a two-page walk whose records
//! carry `environment.name` and `deployable.id`, and a project the token cannot
//! read refused as the other GitLab list reads refuse it.
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
/// A transport answering each read in turn with `(status, body)`.
fn answers(responses: Vec<(u16, Value)>) -> Reads {
    Reads {
        responses: Mutex::new(
            responses
                .into_iter()
                .map(|(status, body)| HttpResponse {
                    status,
                    headers: Default::default(),
                    body: serde_json::to_vec(&body).unwrap(),
                })
                .collect(),
        ),
        calls: Mutex::new(Vec::new()),
    }
}
fn reads(bodies: Vec<Value>) -> Reads {
    answers(bodies.into_iter().map(|body| (200, body)).collect())
}
fn shipped() -> Vec<Selection> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let file: Value = serde_json::from_slice(
        &std::fs::read(root.join("providers/gitlab/operations.json")).unwrap(),
    )
    .unwrap();
    serde_json::from_value(file["operations"].clone()).unwrap()
}
fn engine() -> Engine {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let bundle = bundle::load(&root.join("generated/bundles"), "gitlab").unwrap();
    Engine::new(&bundle, "/api/v4", &shipped()).unwrap()
}
fn properties(engine: &Engine, id: &str) -> serde_json::Map<String, Value> {
    engine
        .declarations(&[Effect::Read])
        .into_iter()
        .find(|o| o.id == id)
        .unwrap_or_else(|| panic!("`{id}` is not a declared read"))
        .input_schema["properties"]
        .as_object()
        .unwrap()
        .clone()
}
fn pairs(list: &[(&str, &str)]) -> Vec<(String, String)> {
    list.iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect()
}
fn segments(list: &[&str]) -> Vec<String> {
    list.iter().map(|s| s.to_string()).collect()
}
/// One deployment as GitLab lists it, with the fields the consumer reads:
/// the environment it went to and the job that ran it.
fn deployment(id: u64, environment: &str, job: u64) -> Value {
    json!({
        "id": id,
        "iid": id,
        "ref": "main",
        "sha": format!("c0ffee{id:02}"),
        "status": "success",
        "created_at": "2026-09-10T08:00:00.000+00:00",
        "updated_at": "2026-09-10T08:05:00.000+00:00",
        "finished_at": "2026-09-10T08:05:00.000+00:00",
        "user": {"id": 42, "username": "fixture-user"},
        "environment": {"id": 3, "name": environment, "tier": "production",
                        "external_url": "https://deploy.example.test"},
        "deployable": {"id": job, "name": "release", "stage": "deploy", "status": "success",
                       "ref": "main", "pipeline": {"id": 900 + id, "status": "success"}}
    })
}

/// `deployments.list` is a declared read whose inputs are the pinned
/// document's, typed as it types them: `id` a path parameter typed
/// `oneOf: [string, integer]` like `tags.list`'s and `commits.list`'s,
/// `page` and `per_page` integers with `per_page` bounded 1 to 100, and the
/// filters (`order_by`, `sort`, `status` enums; the four date-time bounds;
/// `environment`) strings. Only `id` is required.
#[test]
fn deployments_list_is_a_declared_read_with_its_pinned_inputs() {
    let engine = engine();
    let reads = engine.declarations(&[Effect::Read]);
    let declaration = reads
        .iter()
        .find(|o| o.id == "deployments.list")
        .expect("`deployments.list` is not a declared read");
    let properties = declaration.input_schema["properties"].as_object().unwrap();
    let mut names: Vec<&str> = properties.keys().map(String::as_str).collect();
    names.sort();
    assert_eq!(
        names,
        [
            "environment",
            "finished_after",
            "finished_before",
            "id",
            "order_by",
            "page",
            "per_page",
            "sort",
            "status",
            "updated_after",
            "updated_before",
        ]
    );
    assert_eq!(
        declaration.input_schema["required"],
        json!(["id"]),
        "required"
    );
    // The pinned document types this `id` `oneOf: [string, integer]`, as it
    // does for `tags.list` and `commits.list` (`pipelines.list` pins `string`),
    // so it is declared as theirs is.
    for sibling in ["tags.list", "commits.list"] {
        assert_eq!(
            properties["id"],
            crate::properties(&engine, sibling)["id"],
            "`id` as `{sibling}` declares it"
        );
    }
    let string = json!({"type": ["string", "integer"]});
    for name in [
        "order_by",
        "sort",
        "status",
        "updated_after",
        "updated_before",
        "finished_after",
        "finished_before",
        "environment",
    ] {
        assert_eq!(properties[name], string, "`{name}`");
    }
    for name in ["page", "per_page"] {
        assert_eq!(
            properties[name]["anyOf"][0],
            json!({"type": "integer"}),
            "`{name}`"
        );
    }
    assert_eq!(properties["per_page"]["minimum"], json!(1));
    assert_eq!(properties["per_page"]["maximum"], json!(100));
    assert_eq!(
        declaration.output_schema["required"],
        json!(["status", "body", "provenance"])
    );
    assert_eq!(engine.effect("deployments.list"), Some(Effect::Read));
    assert!(
        !engine
            .declarations(&[Effect::Write])
            .iter()
            .any(|o| o.id == "deployments.list")
    );
}

/// `deployments.list` holds `per_page` to GitLab's cap like the other list
/// reads, and sends nothing for a value outside it.
#[tokio::test]
async fn deployments_list_refuses_per_page_outside_one_to_one_hundred() {
    let engine = engine();
    for per_page in [json!(0), json!(-1), json!(101), json!("101"), json!(1.5)] {
        let http = reads(vec![]);
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
    for per_page in [1, 100] {
        let http = reads(vec![json!([])]);
        engine
            .read(
                &http,
                "one",
                "deployments.list",
                json!({"id": "org/project", "per_page": per_page}),
            )
            .await
            .unwrap_or_else(|e| panic!("per_page={per_page}: {e:?}"));
        assert_eq!(http.calls.lock().unwrap().len(), 1);
    }
}

/// `deployments.list` sends every filter verbatim to the project deployments
/// route and returns each record unchanged, each carrying `environment.name`
/// and `deployable.id`; a walk at `per_page=2` reads two pages and stops on
/// the short second one.
#[tokio::test]
async fn deployments_list_sends_its_filters_and_walks_to_a_short_page() {
    let engine = engine();
    let first = json!([
        deployment(3, "production", 7003),
        deployment(2, "staging", 7002)
    ]);
    let second = json!([deployment(1, "review/fix-1", 7001)]);
    let http = reads(vec![first.clone(), second.clone()]);
    let input = json!({
        "id": "org/project", "order_by": "finished_at", "sort": "desc",
        "updated_after": "2026-09-01T00:00:00Z", "updated_before": "2026-09-30T23:59:59Z",
        "finished_after": "2026-09-01T00:00:00+02:00", "finished_before": "2026-09-30T00:00:00Z",
        "environment": "production", "status": "success", "per_page": 2
    });
    let mut items = Vec::new();
    let mut page = 1;
    loop {
        let mut input = input.clone();
        input["page"] = json!(page);
        let result = engine
            .read(&http, "one", "deployments.list", input)
            .await
            .unwrap();
        assert_eq!(result["status"], 200);
        let body = result["body"].as_array().unwrap().clone();
        let short = body.len() < 2;
        items.extend(body);
        if short {
            break;
        }
        page += 1;
        assert!(page <= 3, "walk did not stop");
    }
    assert_eq!(page, 2);
    let mut expected = first.as_array().unwrap().clone();
    expected.extend(second.as_array().unwrap().clone());
    assert_eq!(items, expected);
    let environments: Vec<&str> = items
        .iter()
        .map(|d| d["environment"]["name"].as_str().unwrap())
        .collect();
    assert_eq!(environments, ["production", "staging", "review/fix-1"]);
    let jobs: Vec<u64> = items
        .iter()
        .map(|d| d["deployable"]["id"].as_u64().unwrap())
        .collect();
    assert_eq!(jobs, [7003, 7002, 7001]);

    let calls = http.calls.lock().unwrap().clone();
    let path = segments(&["projects", "org/project", "deployments"]);
    let query = |page: &str| {
        let mut query = pairs(&[
            ("order_by", "finished_at"),
            ("sort", "desc"),
            ("updated_after", "2026-09-01T00:00:00Z"),
            ("updated_before", "2026-09-30T23:59:59Z"),
            ("finished_after", "2026-09-01T00:00:00+02:00"),
            ("finished_before", "2026-09-30T00:00:00Z"),
            ("environment", "production"),
            ("status", "success"),
            ("page", page),
            ("per_page", "2"),
        ]);
        query.sort();
        query
    };
    assert_eq!(calls.len(), 2);
    for (call, page) in calls.iter().zip(["1", "2"]) {
        assert_eq!(call.0, path);
        let mut sent = call.1.clone();
        sent.sort();
        assert_eq!(sent, query(page), "page {page}");
    }
}

/// A project the token cannot read, as GitLab answers it: `403 Forbidden`
/// where the project is visible but the role is short, `404 Project Not
/// Found` where it is not visible at all. `deployments.list` answers each with
/// the refusal code `pipelines.list` answers for the same response.
#[tokio::test]
async fn an_unreadable_project_is_refused_as_pipelines_list_refuses_it() {
    let engine = engine();
    for (status, body) in [
        (403, json!({"message": "403 Forbidden"})),
        (404, json!({"message": "404 Project Not Found"})),
    ] {
        let refusal = |id: &'static str| {
            let engine = &engine;
            let body = body.clone();
            async move {
                let http = answers(vec![(status, body)]);
                let error = engine
                    .read(&http, "one", id, json!({"id": "org/hidden"}))
                    .await
                    .expect_err("a refusal");
                assert_eq!(http.calls.lock().unwrap().len(), 1, "`{id}` {status}");
                error.code
            }
        };
        let pipelines = refusal("pipelines.list").await;
        let deployments = refusal("deployments.list").await;
        assert_eq!(deployments, pipelines, "{status}");
    }
}
