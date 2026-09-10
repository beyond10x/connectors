use super::*;

fn input() -> Value {
    json!({"project":"1","iid":1,"sha":SHA,"pipeline_id":12})
}
fn ready() -> Value {
    let mut value = mr(1, "2026-09-10T00:00:00Z");
    value["detailed_merge_status"] = json!("mergeable");
    value["head_pipeline"] =
        json!({"id":12,"project_id":2,"sha":SHA,"status":"success","unselected":"discard"});
    value
}
async fn check(value: Value) -> Value {
    let (service, http) = adapter(vec![response(value, None)]);
    let out = invoke(&service, "merge_request.validate", input()).await;
    assert_eq!(out["item"]["merge_performed"], false);
    assert_eq!(out["provenance"]["source_revision"], Value::Null);
    let calls = http.calls.lock().unwrap();
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].0, ["projects", "1", "merge_requests", "1"]);
    assert!(calls[0].1.is_empty());
    out["item"].clone()
}

#[tokio::test]
async fn pinned_validation_passes_then_changed_head_blocks_without_claiming_merge() {
    let item = check(ready()).await;
    assert_eq!(item["checks_passed"], true);
    assert_eq!(item["blockers"], json!([]));
    assert_eq!(item["expected_sha"], SHA);
    assert_eq!(item["expected_pipeline_id"], 12);
    assert!(item["head_pipeline"].get("unselected").is_none());
    assert!(item["merge_request"].get("unselected").is_none());
    let mut changed = ready();
    changed["sha"] = json!("a".repeat(40));
    let item = check(changed).await;
    assert_eq!(item["checks_passed"], false);
    assert_eq!(item["blockers"], json!(["head_changed"]));
}

#[tokio::test]
async fn all_missing_changed_and_unsuccessful_checks_remain_blockers() {
    for (path, value, reason) in [
        ("/sha", Value::Null, "head_unavailable"),
        ("/sha", json!("a".repeat(40)), "head_changed"),
        ("/state", json!("merged"), "not_open"),
        ("/state", json!("future_state"), "not_open"),
        ("/draft", json!(true), "draft"),
        (
            "/detailed_merge_status",
            json!("checking"),
            "merge_checks_pending",
        ),
        (
            "/detailed_merge_status",
            json!("not_approved"),
            "merge_checks_pending",
        ),
        (
            "/detailed_merge_status",
            json!("future_status"),
            "merge_checks_pending",
        ),
        ("/head_pipeline", Value::Null, "pipeline_unavailable"),
        ("/head_pipeline/id", json!(11), "pipeline_changed"),
        (
            "/head_pipeline/sha",
            json!("a".repeat(40)),
            "pipeline_head_mismatch",
        ),
        (
            "/head_pipeline/status",
            json!("failed"),
            "pipeline_not_successful",
        ),
        (
            "/head_pipeline/status",
            json!("running"),
            "pipeline_not_successful",
        ),
        (
            "/head_pipeline/status",
            json!("pending"),
            "pipeline_not_successful",
        ),
        (
            "/head_pipeline/status",
            json!("manual"),
            "pipeline_not_successful",
        ),
        (
            "/head_pipeline/status",
            json!("skipped"),
            "pipeline_not_successful",
        ),
        (
            "/head_pipeline/status",
            json!("future_status"),
            "pipeline_not_successful",
        ),
    ] {
        let mut raw = ready();
        *raw.pointer_mut(path).unwrap() = value;
        let out = check(raw).await;
        assert_eq!(out["checks_passed"], false, "{path}");
        assert_eq!(out["blockers"], json!([reason]), "{path}");
    }
    let mut raw = ready();
    raw["pipeline"] = raw["head_pipeline"].take();
    raw.as_object_mut().unwrap().remove("head_pipeline");
    raw["has_conflicts"] = json!(false);
    assert_eq!(
        check(raw).await["blockers"],
        json!(["pipeline_unavailable"])
    );
}

#[tokio::test]
async fn blockers_are_ordered_and_sha256_and_target_project_pipelines_are_supported() {
    let mut raw = ready();
    raw["sha"] = Value::Null;
    raw["state"] = json!("closed");
    raw["draft"] = json!(true);
    raw["detailed_merge_status"] = json!("conflict");
    raw["head_pipeline"]["id"] = json!(11);
    raw["head_pipeline"]["sha"] = json!("b".repeat(40));
    raw["head_pipeline"]["status"] = json!("failed");
    assert_eq!(
        check(raw).await["blockers"],
        json!([
            "head_unavailable",
            "not_open",
            "draft",
            "merge_checks_pending",
            "pipeline_changed",
            "pipeline_head_mismatch",
            "pipeline_not_successful"
        ])
    );
    let mut raw = ready();
    raw["sha"] = json!("b".repeat(64));
    raw["head_pipeline"]["sha"] = raw["sha"].clone();
    raw["head_pipeline"]["project_id"] = json!(1);
    raw["source_project_id"] = Value::Null;
    let (service, _) = adapter(vec![response(raw, None)]);
    let mut request = input();
    request["sha"] = json!("b".repeat(64));
    let out = invoke(&service, "merge_request.validate", request).await;
    assert_eq!(out["item"]["checks_passed"], true);
}

#[tokio::test]
async fn malformed_pipeline_or_wrong_mr_identity_is_a_protocol_error() {
    for (path, value) in [
        ("/iid", json!(2)),
        ("/head_pipeline", json!(false)),
        ("/head_pipeline/id", json!(0)),
        ("/head_pipeline/id", json!("12")),
        ("/head_pipeline/project_id", json!(3)),
        ("/head_pipeline/sha", Value::Null),
        ("/head_pipeline/sha", json!("short")),
        ("/head_pipeline/sha", json!("F".repeat(40))),
        ("/head_pipeline/status", json!(false)),
        ("/head_pipeline/status", json!("")),
        ("/head_pipeline/status", json!("é".repeat(33))),
    ] {
        let mut raw = ready();
        *raw.pointer_mut(path).unwrap() = value;
        let (service, _) = adapter(vec![response(raw, None)]);
        assert_eq!(
            service
                .invoke("merge_request.validate", input())
                .await
                .unwrap_err()
                .code,
            ErrorCode::UpstreamProtocol,
            "{path}"
        );
    }
}

#[tokio::test]
async fn validation_schema_and_allowlist_refuse_before_provider_io() {
    let (service, http) = adapter(vec![]);
    for (key, value) in [
        ("iid", json!(0)),
        ("pipeline_id", json!(-1)),
        ("pipeline_id", json!("12")),
        ("sha", json!("main")),
        ("sha", json!("A".repeat(40))),
        ("approval", json!("caller-made")),
    ] {
        let mut request = input();
        request[key] = value;
        assert_eq!(
            service
                .invoke("merge_request.validate", request)
                .await
                .unwrap_err()
                .code,
            ErrorCode::InvalidInput
        );
    }
    for key in ["sha", "pipeline_id"] {
        let mut request = input();
        request.as_object_mut().unwrap().remove(key);
        assert_eq!(
            service
                .invoke("merge_request.validate", request)
                .await
                .unwrap_err()
                .code,
            ErrorCode::InvalidInput
        );
    }
    let mut request = input();
    request["project"] = json!("outside");
    assert_eq!(
        service
            .invoke("merge_request.validate", request)
            .await
            .unwrap_err()
            .code,
        ErrorCode::Forbidden
    );
    assert!(http.calls.lock().unwrap().is_empty());
}

#[tokio::test]
async fn validation_provider_failure_does_not_become_negative_check_success() {
    for (status, code) in [
        (401, ErrorCode::Unauthorized),
        (403, ErrorCode::Forbidden),
        (404, ErrorCode::NotFound),
        (429, ErrorCode::RateLimited),
        (503, ErrorCode::Unavailable),
    ] {
        let mut r = response(json!({"token":"private-fixture"}), None);
        r.status = status;
        let (service, _) = adapter(vec![r]);
        let error = service
            .invoke("merge_request.validate", input())
            .await
            .unwrap_err();
        assert_eq!(error.code, code);
        assert!(!error.message.contains("private-fixture"));
    }
}
