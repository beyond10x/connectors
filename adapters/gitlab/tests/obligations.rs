use connectors_core::{ErrorCode, Result};
use connectors_gitlab::{Config, GitLab};
use connectors_sdk::{Adapter, AuthenticatedHttp, HttpResponse};
use serde_json::{Value, json};
use std::{
    collections::BTreeMap,
    sync::{Arc, Mutex},
};

type CapturedRequest = (Vec<String>, BTreeMap<String, String>);
struct Fixture {
    responses: Mutex<Vec<HttpResponse>>,
    calls: Mutex<Vec<CapturedRequest>>,
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
        Ok(self.responses.lock().unwrap().remove(0))
    }
}
fn response(status: u16, body: Value, next: Option<&str>) -> HttpResponse {
    HttpResponse {
        status,
        body: serde_json::to_vec(&body).unwrap(),
        headers: next
            .map(|v| BTreeMap::from([("x-next-page".into(), v.into())]))
            .unwrap_or_default(),
    }
}
fn adapter(responses: Vec<HttpResponse>) -> (GitLab, Arc<Fixture>) {
    let http = Arc::new(Fixture {
        responses: Mutex::new(responses),
        calls: Mutex::new(vec![]),
    });
    let config = Config {
        allowed_projects: vec!["org/project".into()],
    };
    let effective = json!({"service":{"instance":"test","listen":"127.0.0.1:0","service_credential":{"kind":"environment","name":"UNUSED"}},"http":{"base_url":"https://gitlab.example/api/v4/","credential":null,"credential_header":"private-token","bearer":false},"adapter":config});
    (
        GitLab::new("test", config, effective, http.clone()).unwrap(),
        http,
    )
}
#[tokio::test]
async fn generated_dispatch_rejects_invalid_inputs_before_transport() {
    let (service, http) = adapter(vec![]);
    for (op, input, code) in [
        (
            "project.get",
            json!({"project":"outside"}),
            ErrorCode::Forbidden,
        ),
        (
            "project.get",
            json!({"project":"org/project","token":"never-accepted"}),
            ErrorCode::InvalidInput,
        ),
        (
            "issues.list",
            json!({"project":"org/project","limit":101}),
            ErrorCode::InvalidInput,
        ),
        (
            "issues.list",
            json!({"project":"org/project","limit":1,"cursor":"untrusted"}),
            ErrorCode::StaleCursor,
        ),
        (
            "file.get",
            json!({"project":"org/project","path":"README.md","ref":""}),
            ErrorCode::InvalidInput,
        ),
        ("unimplemented", json!({}), ErrorCode::NotFound),
    ] {
        assert_eq!(service.invoke(op, input).await.unwrap_err().code, code);
    }
    assert!(http.calls.lock().unwrap().is_empty());
}
#[tokio::test]
async fn response_failures_and_nonadvancing_pagination_remain_explicit() {
    for (op, input, response, code) in [
        (
            "project.get",
            json!({"project":"org/project"}),
            response(200, json!([]), None),
            ErrorCode::UpstreamProtocol,
        ),
        (
            "project.get",
            json!({"project":"org/project"}),
            response(401, json!({"secret":"not returned"}), None),
            ErrorCode::Unauthorized,
        ),
        (
            "project.get",
            json!({"project":"org/project"}),
            response(429, json!({}), None),
            ErrorCode::RateLimited,
        ),
        (
            "issues.list",
            json!({"project":"org/project","limit":1}),
            response(200, json!({"id":1}), None),
            ErrorCode::UpstreamProtocol,
        ),
        (
            "issues.list",
            json!({"project":"org/project","limit":1}),
            response(200, json!([{}, {}]), None),
            ErrorCode::UpstreamProtocol,
        ),
        (
            "issues.list",
            json!({"project":"org/project","limit":1}),
            response(200, json!([]), Some("1")),
            ErrorCode::UpstreamProtocol,
        ),
        (
            "issues.list",
            json!({"project":"org/project","limit":1}),
            response(200, json!([]), Some("broken")),
            ErrorCode::UpstreamProtocol,
        ),
        (
            "file.get",
            json!({"project":"org/project","path":"file","ref":"main"}),
            response(200, json!({"encoding":"raw","content":"bad"}), None),
            ErrorCode::UpstreamProtocol,
        ),
    ] {
        let (service, http) = adapter(vec![response]);
        let error = service.invoke(op, input).await.unwrap_err();
        assert_eq!(error.code, code);
        assert!(!error.message.contains("not returned"));
        assert_eq!(http.calls.lock().unwrap().len(), 1); // no automatic retries
    }
}
#[tokio::test]
async fn empty_pages_with_continuation_and_full_pages_without_headers_are_partial() {
    for next in [Some("2"), None] {
        let body = if next.is_some() {
            json!([])
        } else {
            json!([{"id":1}])
        };
        let (service, http) = adapter(vec![
            response(200, body, next),
            response(200, json!([]), Some("")),
        ]);
        let first = service
            .invoke("issues.list", json!({"project":"org/project","limit":1}))
            .await
            .unwrap();
        assert_eq!(first["complete"], false);
        let last = service
            .invoke(
                "issues.list",
                json!({"project":"org/project","limit":1,"cursor":first["next_cursor"]}),
            )
            .await
            .unwrap();
        assert_eq!(last["complete"], true);
        let calls = http.calls.lock().unwrap();
        assert_eq!(calls[0].0, ["projects", "org/project", "issues"]);
        assert_eq!(
            calls[0].1,
            BTreeMap::from([
                ("order_by".into(), "created_at".into()),
                ("page".into(), "1".into()),
                ("per_page".into(), "1".into()),
                ("sort".into(), "asc".into())
            ])
        );
        assert_eq!(calls[1].1["page"], "2");
    }
}
#[tokio::test]
async fn generated_file_mapping_passes_unencoded_segments_to_the_scoped_transport() {
    let (service, http) = adapter(vec![response(
        200,
        json!({"encoding":"base64","content":"","commit_id":"exact-revision"}),
        None,
    )]);
    let result = service
        .invoke(
            "file.get",
            json!({"project":"org/project","path":"dir/file #?.md","ref":"feature/a&b"}),
        )
        .await
        .unwrap();
    assert_eq!(result["provenance"]["source_revision"], "exact-revision");
    let calls = http.calls.lock().unwrap();
    assert_eq!(
        calls[0].0,
        [
            "projects",
            "org/project",
            "repository",
            "files",
            "dir/file #?.md"
        ]
    );
    assert_eq!(calls[0].1["ref"], "feature/a&b");
}
