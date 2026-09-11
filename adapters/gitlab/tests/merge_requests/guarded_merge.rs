use super::*;
use connectors_sdk::{AuthenticatedWrite, WriteOutcome};
use std::sync::atomic::{AtomicUsize, Ordering};

fn input() -> Value {
    json!({"project":"org/project","iid":1,"sha":SHA,"pipeline_id":12})
}
fn ready() -> Value {
    let mut mr = mr(1, "2026-09-11T00:00:00Z");
    mr["detailed_merge_status"] = json!("mergeable");
    mr["head_pipeline"] = json!({"id":12,"project_id":2,"sha":SHA,"status":"success"});
    mr
}
struct Put {
    calls: Arc<AtomicUsize>,
    response: Option<HttpResponse>,
}
#[async_trait::async_trait]
impl AuthenticatedWrite for Put {
    async fn put_json(
        self: Box<Self>,
        path: &[&str],
        query: &[(&str, String)],
        body: &Value,
    ) -> Result<HttpResponse> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        assert_eq!(
            path,
            ["projects", "org/project", "merge_requests", "1", "merge"]
        );
        assert!(query.is_empty());
        assert_eq!(
            body,
            &json!({"sha":SHA,"auto_merge":false,"should_remove_source_branch":false})
        );
        self.response
            .ok_or_else(connectors_core::Error::unavailable)
    }
}
#[tokio::test]
async fn guarded_merge_classifies_exact_target_and_never_repeats_put() {
    let mut merged = ready();
    merged["state"] = json!("merged");
    // Unknown successes include changed numeric identity even for a path selector.
    let mut responses = vec![
        (Some(response(merged.clone(), None)), "applied"),
        (None, "unknown"),
    ];
    for (field, value) in [
        ("id", json!(99)),
        ("iid", json!(2)),
        ("project_id", json!(2)),
        ("sha", json!("a".repeat(40))),
        ("state", json!("opened")),
    ] {
        let mut wrong = merged.clone();
        wrong[field] = value;
        responses.push((Some(response(wrong, None)), "unknown"));
    }
    for status in [403, 405, 409, 422, 500, 429, 307, 202] {
        responses.push((
            Some(HttpResponse {
                status,
                body: b"private provider error".to_vec(),
                headers: BTreeMap::from([("retry-after".into(), "0".into())]),
            }),
            if matches!(status, 403 | 405 | 409 | 422) {
                "refused"
            } else {
                "unknown"
            },
        ));
    }
    responses.push((Some(response(json!({"state":"merged"}), None)), "unknown"));
    for (reply, expected) in responses {
        let (service, read) = adapter(vec![response(ready(), None)]);
        assert!(
            service
                .descriptor()
                .operation("merge_request.merge")
                .is_err()
        );
        assert!(
            service
                .private_descriptor()
                .operation("merge_request.merge")
                .is_ok()
        );
        let preparation = service
            .prepare_merge(read.clone(), "partition", input())
            .await
            .unwrap();
        assert_eq!(read.calls.lock().unwrap().len(), 1);
        let calls = Arc::new(AtomicUsize::new(0));
        let result = preparation
            .execute(Box::new(Put {
                calls: calls.clone(),
                response: reply,
            }))
            .await;
        let actual = match result {
            WriteOutcome::Applied(Ok(value)) => {
                assert_eq!(value["item"]["state"], "merged");
                assert!(value["item"]["merge_commit_sha"].is_null());
                "applied"
            }
            WriteOutcome::Applied(Err(error)) => panic!("unexpected projection error: {error}"),
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
#[tokio::test]
async fn guarded_merge_requires_fresh_head_pipeline_and_allowlisted_target() {
    for (path, value) in [
        ("/sha", json!("a".repeat(40))),
        ("/head_pipeline/id", json!(13)),
        ("/head_pipeline/status", json!("failed")),
        ("/draft", json!(true)),
    ] {
        let mut changed = ready();
        *changed.pointer_mut(path).unwrap() = value;
        let (service, read) = adapter(vec![response(changed, None)]);
        assert!(
            service
                .prepare_merge(read.clone(), "partition", input())
                .await
                .is_err()
        );
        assert_eq!(read.calls.lock().unwrap().len(), 1);
    }
    let (service, read) = adapter(vec![]);
    let mut outside = input();
    outside["project"] = json!("outside");
    assert!(
        service
            .prepare_merge(read.clone(), "partition", outside)
            .await
            .is_err()
    );
    let mut invalid = input();
    invalid["sha"] = json!("main");
    assert!(
        service
            .prepare_merge(read.clone(), "partition", invalid)
            .await
            .is_err()
    );
    assert!(read.calls.lock().unwrap().is_empty());
}
