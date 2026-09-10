use connectors_core::{ErrorCode, Result};
use connectors_gitlab::{Config, GitLab};
use connectors_sdk::{Adapter, AuthenticatedHttp, HttpResponse};
use serde_json::{Value, json};
use std::{
    collections::{BTreeMap, VecDeque},
    sync::{Arc, Mutex},
};
const SHA: &str = "0123456789abcdef0123456789abcdef01234567";
#[path = "merge_requests/validation.rs"]
mod validation;
type Call = (Vec<String>, BTreeMap<String, String>);
struct Fixture {
    responses: Mutex<VecDeque<HttpResponse>>,
    calls: Mutex<Vec<Call>>,
}
#[async_trait::async_trait]
impl AuthenticatedHttp for Fixture {
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
            .expect("unexpected provider call"))
    }
}
fn response(value: Value, next: Option<&str>) -> HttpResponse {
    HttpResponse {
        status: 200,
        body: serde_json::to_vec(&value).unwrap(),
        headers: next
            .map(|n| BTreeMap::from([("x-next-page".into(), n.into())]))
            .unwrap_or_default(),
    }
}
fn mr(iid: i64, updated: &str) -> Value {
    json!({"id":1000+iid,"iid":iid,"project_id":1,"target_project_id":1,"source_project_id":2,"title":"Inspect change","description":"body\ntext","source_branch":"fix","target_branch":"main","state":"opened","draft":false,"detailed_merge_status":"checking","sha":SHA,"merge_commit_sha":null,"squash_commit_sha":null,"updated_at":updated,"unselected":"discard"})
}
fn args() -> Value {
    json!({"project":"org/project","state":"all","updated_after":"2026-09-01T00:00:00Z","updated_before":"2026-09-10T00:00:00Z","limit":2})
}
fn adapter(responses: Vec<HttpResponse>) -> (GitLab, Arc<Fixture>) {
    let http = Arc::new(Fixture {
        responses: Mutex::new(responses.into()),
        calls: Mutex::new(vec![]),
    });
    let config = Config {
        allowed_projects: vec!["org/project".into(), "1".into()],
    };
    let effective = json!({"service":{"instance":"test","listen":"127.0.0.1:0","service_credential":{"kind":"environment","name":"UNUSED"}},"http":{"base_url":"https://gitlab.example/api/v4/","credential":null,"credential_header":"private-token","bearer":false},"adapter":config});
    (
        GitLab::new("test", config, effective, http.clone()).unwrap(),
        http,
    )
}
async fn invoke(service: &GitLab, op: &str, input: Value) -> Value {
    let output = service.invoke(op, input).await.unwrap();
    connectors_sdk::validate(
        &service.descriptor().operation(op).unwrap().output_schema,
        &output,
    )
    .unwrap();
    output
}
#[tokio::test]
async fn window_traversal_and_project_local_get_preserve_observations() {
    let first = mr(3, "2026-09-01T00:00:00Z");
    let mut last = mr(4, "2026-09-10T00:00:00Z");
    last["source_project_id"] = Value::Null;
    last["sha"] = Value::Null;
    last["detailed_merge_status"] = json!("future_status");
    let (service, http) = adapter(vec![
        response(json!([first, last.clone()]), None),
        response(json!([]), None),
        response(last, None),
    ]);
    let page = invoke(&service, "merge_requests.list", args()).await;
    assert_eq!(page["complete"], false);
    assert!(page["items"][0].get("unselected").is_none());
    assert_eq!(page["items"][1]["sha"], Value::Null);
    let mut input = args();
    input["cursor"] = page["next_cursor"].clone();
    let end = invoke(&service, "merge_requests.list", input).await;
    assert_eq!(end["complete"], true);
    assert_eq!(end["next_cursor"], Value::Null);
    let item = invoke(
        &service,
        "merge_request.get",
        json!({"project":"org/project","iid":4}),
    )
    .await;
    assert_eq!(item["item"]["detailed_merge_status"], "future_status");
    assert_eq!(item["provenance"]["source_revision"], Value::Null);
    let calls = http.calls.lock().unwrap();
    assert_eq!(calls[0].0, ["projects", "org/project", "merge_requests"]);
    for (key, value) in [
        ("scope", "all"),
        ("state", "all"),
        ("sort", "asc"),
        ("order_by", "updated_at"),
        ("page", "1"),
        ("per_page", "2"),
        ("updated_after", "2026-09-01T00:00:00Z"),
        ("updated_before", "2026-09-10T00:00:00Z"),
    ] {
        assert_eq!(calls[0].1[key], value);
    }
    assert_eq!(calls[1].1["page"], "2");
    assert_eq!(
        calls[2].0,
        ["projects", "org/project", "merge_requests", "4"]
    );
    assert!(calls[2].1.is_empty());
}
#[tokio::test]
async fn invalid_selections_refuse_before_transport() {
    let (service, http) = adapter(vec![]);
    for (key, value) in [
        ("state", json!("ready")),
        ("updated_after", json!("2026-02-30T00:00:00Z")),
        ("updated_before", json!("2025-09-01T00:00:00Z")),
        ("updated_after", json!("2026-09-01")),
        ("updated_before", json!("2026-09-10T25:00:00Z")),
        ("updated_after", json!("2026-09-01T00:00:60Z")),
        ("limit", json!(101)),
        ("token", json!("forbidden-input")),
    ] {
        let mut input = args();
        input[key] = value;
        assert_eq!(
            service
                .invoke("merge_requests.list", input)
                .await
                .unwrap_err()
                .code,
            ErrorCode::InvalidInput
        );
    }
    assert_eq!(
        service
            .invoke("merge_request.get", json!({"project":"outside","iid":1}))
            .await
            .unwrap_err()
            .code,
        ErrorCode::Forbidden
    );
    assert_eq!(
        service
            .invoke("merge_request.get", json!({"project":"1","iid":0}))
            .await
            .unwrap_err()
            .code,
        ErrorCode::InvalidInput
    );
    assert!(http.calls.lock().unwrap().is_empty());
}
#[tokio::test]
async fn malformed_identity_and_projection_never_become_partial_success() {
    for (key, value) in [
        ("iid", json!(2)),
        ("id", json!(0)),
        ("project_id", json!(9)),
        ("target_project_id", json!(2)),
        ("source_project_id", json!(-1)),
        ("sha", json!("abc")),
        ("sha", json!("F".repeat(40))),
        ("draft", Value::Null),
        ("updated_at", json!("2026-02-30T00:00:00Z")),
        ("title", json!("é".repeat(513))),
        ("description", json!("x".repeat(16385))),
        ("state", json!("")),
        ("detailed_merge_status", json!(false)),
    ] {
        let mut item = mr(1, "2026-09-02T00:00:00Z");
        item[key] = value;
        let (service, _) = adapter(vec![response(item, None)]);
        assert_eq!(
            service
                .invoke("merge_request.get", json!({"project":"1","iid":1}))
                .await
                .unwrap_err()
                .code,
            ErrorCode::UpstreamProtocol,
            "{key}"
        );
    }
    let mut item = mr(1, "2026-09-02T01:00:00+01:00");
    for key in [
        "description",
        "sha",
        "merge_commit_sha",
        "squash_commit_sha",
        "source_project_id",
    ] {
        item.as_object_mut().unwrap().remove(key);
    }
    let (service, _) = adapter(vec![response(item, None)]);
    let item = invoke(
        &service,
        "merge_request.get",
        json!({"project":"1","iid":1}),
    )
    .await;
    assert_eq!(item["item"]["description"], Value::Null);
}
#[tokio::test]
async fn pages_refuse_filter_window_order_duplicates_and_invalid_continuations() {
    let a = mr(1, "2026-09-02T00:00:00Z");
    let b = mr(2, "2026-09-03T00:00:00Z");
    let mut duplicate_iid = b.clone();
    duplicate_iid["iid"] = a["iid"].clone();
    let mut duplicate_id = b.clone();
    duplicate_id["id"] = a["id"].clone();
    for (value, next) in [
        (json!({}), None),
        (
            json!([a.clone(), b.clone(), mr(3, "2026-09-04T00:00:00Z")]),
            None,
        ),
        (json!([b, a.clone()]), None),
        (json!([a.clone(), duplicate_iid]), None),
        (json!([a.clone(), duplicate_id]), None),
        (json!([mr(1, "2026-08-31T23:59:59Z")]), None),
        (json!([mr(1, "2026-09-10T00:00:00.001Z")]), None),
        (json!([a.clone()]), Some("1")),
        (json!([a.clone()]), Some("bogus")),
    ] {
        let (service, _) = adapter(vec![response(value, next)]);
        assert_eq!(
            service
                .invoke("merge_requests.list", args())
                .await
                .unwrap_err()
                .code,
            ErrorCode::UpstreamProtocol
        );
    }
    let (service, _) = adapter(vec![response(json!([a]), Some(""))]);
    let mut input = args();
    input["state"] = json!("merged");
    assert_eq!(
        service
            .invoke("merge_requests.list", input)
            .await
            .unwrap_err()
            .code,
        ErrorCode::UpstreamProtocol
    );
}
#[tokio::test]
async fn cursors_bind_every_window_axis_and_authenticated_partition() {
    let (service, http) = adapter(vec![response(
        json!([mr(1, "2026-09-02T00:00:00Z")]),
        Some("2"),
    )]);
    let service = service
        .with_authenticated_http(http.clone(), "connection-a")
        .unwrap();
    let page = invoke(&service, "merge_requests.list", args()).await;
    for (key, value) in [
        ("project", json!("1")),
        ("state", json!("opened")),
        ("updated_after", json!("2026-09-02T00:00:00Z")),
        ("updated_before", json!("2026-09-09T00:00:00Z")),
        ("limit", json!(1)),
    ] {
        let mut input = args();
        input["cursor"] = page["next_cursor"].clone();
        input[key] = value;
        assert_eq!(
            service
                .invoke("merge_requests.list", input)
                .await
                .unwrap_err()
                .code,
            ErrorCode::StaleCursor
        );
    }
    let other = service
        .with_authenticated_http(http.clone(), "connection-b")
        .unwrap();
    let mut input = args();
    input["cursor"] = page["next_cursor"].clone();
    assert_eq!(
        other
            .invoke("merge_requests.list", input)
            .await
            .unwrap_err()
            .code,
        ErrorCode::StaleCursor
    );
    assert_eq!(http.calls.lock().unwrap().len(), 1);
}
#[tokio::test]
async fn provider_failures_are_safe_and_never_empty_success() {
    for (status, code) in [
        (401, ErrorCode::Unauthorized),
        (403, ErrorCode::Forbidden),
        (404, ErrorCode::NotFound),
        (429, ErrorCode::RateLimited),
        (503, ErrorCode::Unavailable),
    ] {
        let mut r = response(json!({"credential":"secret-fixture"}), None);
        r.status = status;
        let (service, _) = adapter(vec![r]);
        let e = service
            .invoke("merge_requests.list", args())
            .await
            .unwrap_err();
        assert_eq!(e.code, code);
        assert!(!e.message.contains("secret-fixture"));
    }
}
