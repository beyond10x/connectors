//! Adversarial cases for the four GitLab repository reads in the shipped
//! selection set, driven through the engine over the committed bundle with a
//! recording transport.
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
fn reads(bodies: Vec<Value>) -> Reads {
    Reads {
        responses: Mutex::new(
            bodies
                .into_iter()
                .map(|body| HttpResponse {
                    status: 200,
                    headers: Default::default(),
                    body: serde_json::to_vec(&body).unwrap(),
                })
                .collect(),
        ),
        calls: Mutex::new(Vec::new()),
    }
}
fn shipped() -> Vec<Selection> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let file: Value = serde_json::from_slice(
        &std::fs::read(root.join("providers/gitlab/operations.json")).unwrap(),
    )
    .unwrap();
    serde_json::from_value(file["operations"].clone()).unwrap()
}
fn engine(selections: &[Selection]) -> Engine {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let bundle = bundle::load(&root.join("generated/bundles"), "gitlab").unwrap();
    Engine::new(&bundle, "/api/v4", selections).unwrap()
}
fn pairs(list: &[(&str, &str)]) -> Vec<(String, String)> {
    list.iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect()
}

/// A parameter the pinned source does not declare for the operation is refused
/// before anything is sent.
#[tokio::test]
async fn undeclared_parameter_is_refused_before_any_request() {
    let engine = engine(&shipped());
    for (id, input) in [
        ("projects.list", json!({"since": "2026-09-01T00:00:00Z"})),
        (
            "tags.list",
            json!({"id": "org/project", "last_activity_after": "2026-09-01"}),
        ),
        (
            "releases.list",
            json!({"id": "org/project", "after": "2026-09-01"}),
        ),
        (
            "project.events",
            json!({"id": "org/project", "since": "2026-09-01"}),
        ),
    ] {
        let http = reads(vec![]);
        let error = engine.read(&http, "one", id, input).await.unwrap_err();
        assert_eq!(error.code, ErrorCode::InvalidInput, "`{id}`");
        assert!(
            http.calls.lock().unwrap().is_empty(),
            "`{id}` sent a request"
        );
    }
}

/// Time filters reach the transport verbatim, including an offset whose `+`
/// the transport must encode, and `simple=false` bodies come back unchanged.
#[tokio::test]
async fn time_filters_reach_the_transport_verbatim_and_bodies_are_unchanged() {
    let engine = engine(&shipped());
    let project = json!([{
        "id": 1, "archived": false, "created_at": "2026-01-02T03:04:05.000Z",
        "last_activity_at": "2026-09-20T10:00:00.000Z", "path_with_namespace": "org/p",
        "description": null, "topics": [], "default_branch": "main",
        "visibility": "internal", "web_url": "https://gitlab.example.test/org/p",
        "namespace": {"id": 3, "full_path": "org"}, "_links": {"self": "x"}
    }]);
    let http = reads(vec![project.clone(), json!([])]);
    let result = engine
        .read(
            &http,
            "one",
            "projects.list",
            json!({"simple": false, "last_activity_after": "2026-09-01T00:00:00+02:00", "per_page": 100}),
        )
        .await
        .unwrap();
    assert_eq!(result["body"], project);
    engine
        .read(
            &http,
            "one",
            "project.events",
            json!({"id": 7, "after": "2026-09-01", "before": "2026-09-30"}),
        )
        .await
        .unwrap();
    let calls = http.calls.lock().unwrap().clone();
    assert_eq!(
        calls,
        vec![
            (
                vec!["projects".to_owned()],
                pairs(&[
                    ("last_activity_after", "2026-09-01T00:00:00+02:00"),
                    ("per_page", "100"),
                    ("simple", "false"),
                ])
            ),
            (
                vec!["projects".into(), "7".into(), "events".into()],
                pairs(&[("before", "2026-09-30"), ("after", "2026-09-01")])
            ),
        ]
    );
}

/// The documented walk ends on "a page shorter than `per_page`". GitLab caps
/// `per_page` at 100, so a request above the cap gets 100 items per page and a
/// caller following the documented rule stops after page one with the rest of
/// the list unread. The provider must refuse a `per_page` it cannot honour
/// (or document the cap next to the stop rule).
#[tokio::test]
#[ignore = "story:catalog-selection-parameter-bounds: the selection set cannot bound a parameter yet"]
async fn per_page_above_the_provider_cap_is_refused_before_any_request() {
    let engine = engine(&shipped());
    for (id, mut input) in [
        ("projects.list", json!({})),
        ("tags.list", json!({"id": "org/project"})),
        ("releases.list", json!({"id": "org/project"})),
        ("project.events", json!({"id": "org/project"})),
    ] {
        input["per_page"] = json!(101);
        let http = reads(vec![json!([])]);
        let outcome = engine.read(&http, "one", id, input).await;
        assert!(
            outcome.is_err(),
            "`{id}` sent per_page=101: {:?}",
            http.calls.lock().unwrap()
        );
        assert!(
            http.calls.lock().unwrap().is_empty(),
            "`{id}` sent a request"
        );
    }
}

/// `effect: read` is enforced for the new ids: a write-shaped selection of one
/// is refused at load, and the new reads are not declared as writes.
#[test]
fn repository_reads_cannot_be_declared_as_writes() {
    for id in [
        "projects.list",
        "tags.list",
        "releases.list",
        "project.events",
    ] {
        let mut selections = shipped();
        let selection = selections.iter_mut().find(|s| s.id == id).unwrap();
        selection.effect = Effect::Write;
        let root = Path::new(env!("CARGO_MANIFEST_DIR"));
        let bundle = bundle::load(&root.join("generated/bundles"), "gitlab").unwrap();
        assert!(
            Engine::new(&bundle, "/api/v4", &selections).is_err(),
            "`{id}`"
        );
    }
    let engine = engine(&shipped());
    let writes: Vec<String> = engine
        .declarations(&[Effect::Write])
        .into_iter()
        .map(|o| o.id)
        .collect();
    for id in [
        "projects.list",
        "tags.list",
        "releases.list",
        "project.events",
    ] {
        assert!(
            !writes.iter().any(|w| w == id),
            "`{id}` declared as a write"
        );
    }
}

/// A planted wrong-but-existing operation id loads against the bundle and sends
/// a different path; the fixture server's `ends_with("/events")` route would
/// still serve it the events page, so only the exact-path assertion in
/// `local_runtime.rs` stands between this mutant and green.
#[tokio::test]
async fn planted_wrong_operation_id_sends_a_different_path() {
    let mut selections = shipped();
    selections
        .iter_mut()
        .find(|s| s.id == "project.events")
        .unwrap()
        .operation_id = "getApiV4Events".into();
    let engine = engine(&selections);
    let http = reads(vec![json!([])]);
    engine
        .read(
            &http,
            "one",
            "project.events",
            json!({"after": "2026-09-01", "page": 1, "per_page": 2}),
        )
        .await
        .unwrap();
    let calls = http.calls.lock().unwrap().clone();
    assert_eq!(calls[0].0, vec!["events".to_owned()]);
    assert!("/api/v4/events".ends_with("/events"));
}
