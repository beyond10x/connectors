use connectors_core::{ErrorCode, Result};
use connectors_gitlab::{Config, GitLab};
use connectors_sdk::{Adapter, AuthenticatedHttp, HttpResponse, HttpResponsePrefix};
use serde_json::{Value, json};
use std::{
    collections::{BTreeMap, VecDeque},
    sync::{Arc, Mutex},
};

const SHA: &str = "0123456789abcdef0123456789abcdef01234567";
type Call = (Vec<String>, BTreeMap<String, String>, Option<usize>);
struct Fixture {
    responses: Mutex<VecDeque<HttpResponsePrefix>>,
    calls: Mutex<Vec<Call>>,
}
impl Fixture {
    fn take(
        &self,
        path: &[&str],
        query: &[(&str, String)],
        cap: Option<usize>,
    ) -> HttpResponsePrefix {
        self.calls.lock().unwrap().push((
            path.iter().map(|v| v.to_string()).collect(),
            query
                .iter()
                .map(|(k, v)| (k.to_string(), v.clone()))
                .collect(),
            cap,
        ));
        self.responses
            .lock()
            .unwrap()
            .pop_front()
            .expect("unexpected provider work")
    }
}
#[async_trait::async_trait]
impl AuthenticatedHttp for Fixture {
    async fn get(&self, path: &[&str], query: &[(&str, String)]) -> Result<HttpResponse> {
        let p = self.take(path, query, None);
        assert!(p.complete);
        Ok(HttpResponse {
            status: p.status,
            headers: p.headers,
            body: p.body,
        })
    }
    async fn get_prefix(
        &self,
        path: &[&str],
        query: &[(&str, String)],
        limit: usize,
    ) -> Result<HttpResponsePrefix> {
        Ok(self.take(path, query, Some(limit)))
    }
}
fn raw(status: u16, bytes: &[u8], complete: bool) -> HttpResponsePrefix {
    HttpResponsePrefix {
        status,
        headers: BTreeMap::new(),
        body: bytes.to_vec(),
        complete,
    }
}
fn ok(value: Value, next: Option<&str>) -> HttpResponsePrefix {
    let mut response = raw(200, &serde_json::to_vec(&value).unwrap(), true);
    if let Some(next) = next {
        response.headers.insert("x-next-page".into(), next.into());
    }
    response
}
fn pipeline(id: i64, status: &str) -> Value {
    json!({"id":id,"project_id":1,"sha":SHA,"ref":"main","status":status,"unselected":"ignored"})
}
fn job(id: i64) -> Value {
    json!({"id":id,"pipeline":{"id":11,"sha":SHA},"name":"test","stage":"test","status":"failed","allow_failure":false})
}
fn adapter(responses: Vec<HttpResponsePrefix>) -> (GitLab, Arc<Fixture>) {
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
async fn invoke(service: &GitLab, operation: &str, input: Value) -> Value {
    let result = service.invoke(operation, input).await.unwrap();
    connectors_sdk::validate(
        &service
            .descriptor()
            .operation(operation)
            .unwrap()
            .output_schema,
        &result,
    )
    .unwrap();
    result
}

#[tokio::test]
async fn exact_commit_polling_job_pages_and_failed_trace_use_generated_mappings() {
    let (service, http) = adapter(vec![
        ok(json!([pipeline(11, "pending")]), Some("")),
        ok(pipeline(11, "pending"), None),
        ok(pipeline(11, "running"), None),
        ok(pipeline(11, "failed"), None),
        ok(json!([job(42)]), Some("2")),
        ok(json!([job(41)]), Some("")),
        ok(job(42), None),
        raw(200, b"test failed\n", true),
    ]);
    let p = invoke(
        &service,
        "pipelines.list",
        json!({"project":"org/project","sha":SHA,"limit":10}),
    )
    .await;
    assert_eq!(p["items"][0]["id"], 11);
    assert_eq!(p["provenance"]["source_revision"], SHA);
    assert!(p["items"][0].get("unselected").is_none());
    for status in ["pending", "running", "failed"] {
        let p = invoke(
            &service,
            "pipeline.get",
            json!({"project":"org/project","pipeline_id":11,"sha":SHA}),
        )
        .await;
        assert_eq!(p["item"]["status"], status);
    }
    let args = json!({"project":"org/project","pipeline_id":11,"sha":SHA,"limit":1});
    let first = invoke(&service, "pipeline.jobs", args.clone()).await;
    assert_eq!(first["complete"], false);
    let mut second = args;
    second["cursor"] = first["next_cursor"].clone();
    assert_eq!(
        invoke(&service, "pipeline.jobs", second).await["complete"],
        true
    );
    assert_eq!(
        invoke(
            &service,
            "job.get",
            json!({"project":"org/project","job_id":42,"pipeline_id":11,"sha":SHA})
        )
        .await["item"]["status"],
        "failed"
    );
    let trace = invoke(
        &service,
        "job.trace",
        json!({"project":"org/project","job_id":42,"max_bytes":100}),
    )
    .await;
    assert_eq!(
        trace["item"],
        json!({"job_id":42,"content":"test failed\n","bytes":12,"complete":true})
    );
    assert_eq!(trace["provenance"]["source_revision"], Value::Null);
    let calls = http.calls.lock().unwrap();
    assert_eq!(calls.len(), 8);
    assert_eq!(calls[0].0, ["projects", "org/project", "pipelines"]);
    assert_eq!(
        calls[0].1,
        BTreeMap::from([
            ("sha".into(), SHA.into()),
            ("page".into(), "1".into()),
            ("per_page".into(), "10".into()),
            ("order_by".into(), "id".into()),
            ("sort".into(), "desc".into())
        ])
    );
    assert_eq!(calls[5].1["page"], "2");
    assert!(!calls[4].1.contains_key("include_retried"));
    assert_eq!(
        calls[7].0,
        ["projects", "org/project", "jobs", "42", "trace"]
    );
    assert!(calls[7].1.is_empty());
    assert_eq!(calls[7].2, Some(512000));
}

#[tokio::test]
async fn invalid_selection_and_unknown_fields_refuse_before_any_transport() {
    let (service, http) = adapter(vec![]);
    for sha in [
        "main",
        "abcdef",
        "0123456789ABCDEF0123456789ABCDEF01234567",
        "",
    ] {
        assert_eq!(
            service
                .invoke(
                    "pipelines.list",
                    json!({"project":"org/project","sha":sha,"limit":10})
                )
                .await
                .unwrap_err()
                .code,
            ErrorCode::InvalidInput
        );
    }
    for (op, input, code) in [
        (
            "pipeline.get",
            json!({"project":"other","pipeline_id":11,"sha":SHA}),
            ErrorCode::Forbidden,
        ),
        (
            "pipeline.get",
            json!({"project":"org/project","pipeline_id":0,"sha":SHA}),
            ErrorCode::InvalidInput,
        ),
        (
            "pipeline.jobs",
            json!({"project":"org/project","pipeline_id":11,"sha":SHA,"limit":101}),
            ErrorCode::InvalidInput,
        ),
        (
            "job.get",
            json!({"project":"org/project","job_id":42,"pipeline_id":11,"sha":SHA,"token":"rejected"}),
            ErrorCode::InvalidInput,
        ),
        (
            "job.trace",
            json!({"project":"org/project","job_id":42,"max_bytes":512001}),
            ErrorCode::InvalidInput,
        ),
        (
            "job.trace",
            json!({"project":"org/project","job_id":42,"max_bytes":0}),
            ErrorCode::InvalidInput,
        ),
    ] {
        assert_eq!(service.invoke(op, input).await.unwrap_err().code, code);
    }
    assert!(http.calls.lock().unwrap().is_empty());
}

#[tokio::test]
async fn inconsistent_targets_and_malformed_native_shapes_cannot_satisfy_ci() {
    for field in ["sha", "id", "project_id", "ref", "status"] {
        let mut p = pipeline(11, "success");
        p[field] = match field {
            "sha" => json!("f".repeat(40)),
            "id" | "project_id" => json!(0),
            _ => Value::Null,
        };
        let (service, _) = adapter(vec![ok(p, None)]);
        assert_eq!(
            service
                .invoke(
                    "pipeline.get",
                    json!({"project":"org/project","pipeline_id":11,"sha":SHA})
                )
                .await
                .unwrap_err()
                .code,
            ErrorCode::UpstreamProtocol
        );
    }
    for pointer in [
        "/pipeline/sha",
        "/pipeline/id",
        "/id",
        "/allow_failure",
        "/name",
    ] {
        let mut j = job(42);
        *j.pointer_mut(pointer).unwrap() = match pointer {
            "/pipeline/sha" => json!("f".repeat(40)),
            "/pipeline/id" | "/id" => json!(99),
            _ => Value::Null,
        };
        let (service, _) = adapter(vec![ok(j, None)]);
        assert_eq!(
            service
                .invoke(
                    "job.get",
                    json!({"project":"org/project","job_id":42,"pipeline_id":11,"sha":SHA})
                )
                .await
                .unwrap_err()
                .code,
            ErrorCode::UpstreamProtocol
        );
    }
    let mut wrong = pipeline(99, "success");
    wrong["sha"] = json!("f".repeat(40));
    let (service, _) = adapter(vec![
        ok(json!([pipeline(11, "running"), wrong]), None),
        ok(pipeline(11, "future-status"), None),
    ]);
    assert_eq!(
        service
            .invoke(
                "pipelines.list",
                json!({"project":"org/project","sha":SHA,"limit":10})
            )
            .await
            .unwrap_err()
            .code,
        ErrorCode::UpstreamProtocol
    );
    assert_eq!(
        invoke(
            &service,
            "pipeline.get",
            json!({"project":"org/project","pipeline_id":11,"sha":SHA})
        )
        .await["item"]["status"],
        "future-status"
    );
}

#[tokio::test]
async fn continuations_bind_every_selection_and_connection_partition() {
    let (service, http) = adapter(vec![ok(json!([job(42)]), None), ok(json!([]), None)]);
    let service = service
        .with_authenticated_http(http.clone(), "connection-one")
        .unwrap();
    let other = service
        .with_authenticated_http(http.clone(), "connection-two")
        .unwrap();
    let mut input = json!({"project":"org/project","pipeline_id":11,"sha":SHA,"limit":1});
    let page = invoke(&service, "pipeline.jobs", input.clone()).await;
    assert_eq!(page["complete"], false);
    input["cursor"] = page["next_cursor"].clone();
    for (field, value) in [
        ("project", json!("1")),
        ("pipeline_id", json!(12)),
        ("sha", json!("f".repeat(40))),
        ("limit", json!(2)),
    ] {
        let mut changed = input.clone();
        changed[field] = value;
        assert_eq!(
            service
                .invoke("pipeline.jobs", changed)
                .await
                .unwrap_err()
                .code,
            ErrorCode::StaleCursor
        );
    }
    assert_eq!(
        other
            .invoke("pipeline.jobs", input.clone())
            .await
            .unwrap_err()
            .code,
        ErrorCode::StaleCursor
    );
    let mut changed = input.clone();
    changed.as_object_mut().unwrap().remove("pipeline_id");
    assert_eq!(
        service
            .invoke("pipelines.list", changed)
            .await
            .unwrap_err()
            .code,
        ErrorCode::StaleCursor
    );
    assert_eq!(http.calls.lock().unwrap().len(), 1);
    let page = invoke(&service, "pipeline.jobs", input).await;
    assert_eq!(page["items"], json!([]));
    assert_eq!(page["complete"], true);
}

#[tokio::test]
async fn paging_and_provider_errors_are_explicit_without_retry() {
    for (response, code) in [
        (
            ok(json!([pipeline(11, "pending")]), Some("1")),
            ErrorCode::UpstreamProtocol,
        ),
        (ok(json!([]), Some("NaN")), ErrorCode::UpstreamProtocol),
        (
            ok(
                json!([pipeline(11, "pending"), pipeline(12, "pending")]),
                None,
            ),
            ErrorCode::UpstreamProtocol,
        ),
        (
            ok(
                json!([pipeline(11, "pending"), pipeline(11, "pending")]),
                None,
            ),
            ErrorCode::UpstreamProtocol,
        ),
        (ok(json!({}), None), ErrorCode::UpstreamProtocol),
        (raw(401, b"private-response", true), ErrorCode::Unauthorized),
        (raw(403, b"private-response", true), ErrorCode::Forbidden),
        (raw(404, b"private-response", true), ErrorCode::NotFound),
        (raw(503, b"private-response", true), ErrorCode::Unavailable),
    ] {
        let (service, http) = adapter(vec![response]);
        let error = service
            .invoke(
                "pipelines.list",
                json!({"project":"org/project","sha":SHA,"limit":2}),
            )
            .await
            .unwrap_err();
        assert_eq!(error.code, code);
        assert!(!error.to_string().contains("private-response"));
        assert_eq!(http.calls.lock().unwrap().len(), 1);
    }
}

#[tokio::test]
async fn trace_bytes_utf8_boundaries_and_completeness_are_preserved() {
    for (body, complete, cap, expected, expected_complete) in [
        (b"".as_slice(), true, 1, "", true),
        ("a€b".as_bytes(), true, 5, "a€b", true),
        ("a€b".as_bytes(), true, 3, "a", false),
        (b"a\xe2\x82", false, 100, "a", false),
        (b"abc", false, 100, "abc", false),
        (b"\x1b[31m\n", true, 100, "\x1b[31m\n", true),
    ] {
        let (service, _) = adapter(vec![raw(200, body, complete)]);
        let result = invoke(
            &service,
            "job.trace",
            json!({"project":"org/project","job_id":42,"max_bytes":cap}),
        )
        .await;
        assert_eq!(
            result["item"],
            json!({"job_id":42,"content":expected,"bytes":expected.len(),"complete":expected_complete})
        );
    }
    for (body, complete) in [
        (b"a\xe2\x82".as_slice(), true),
        (b"a\xffz", false),
        (b"a\xffz", true),
    ] {
        let (service, _) = adapter(vec![raw(200, body, complete)]);
        assert_eq!(
            service
                .invoke(
                    "job.trace",
                    json!({"project":"org/project","job_id":42,"max_bytes":1})
                )
                .await
                .unwrap_err()
                .code,
            ErrorCode::UpstreamProtocol
        );
    }
    for (status, code) in [
        (401, ErrorCode::Unauthorized),
        (403, ErrorCode::Forbidden),
        (404, ErrorCode::NotFound),
        (503, ErrorCode::Unavailable),
    ] {
        let (service, _) = adapter(vec![raw(status, b"secret-provider-error", true)]);
        let error = service
            .invoke(
                "job.trace",
                json!({"project":"org/project","job_id":42,"max_bytes":100}),
            )
            .await
            .unwrap_err();
        assert_eq!(error.code, code);
        assert!(!error.to_string().contains("secret-provider-error"));
    }
}
