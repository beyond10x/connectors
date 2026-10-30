//! Keyed writes under a standing grant on `POST /v1alpha2/invoke`
//! (contracts/service/compatibility.md § 2.1, Keyed writes under a standing
//! grant; ess/domains/standing_approvals.yaml;
//! story:served-keyed-writes-under-a-standing-approval).
//!
//! Every case runs against a fixture adapter that counts its provider
//! requests, and every refusal asserts that the count did not move.
//!
//! Unix only: its fixtures use Unix mode bits and symbolic links.
#![cfg(unix)]
use async_trait::async_trait;
use connectors_core::{
    Descriptor, Operation, Result, WIRE_VERSION,
    v1alpha2::{AuditStatus, EffectKnowledge, ErrorCode, InvokeResponse, ResponseStatus},
};
use connectors_host::server::{ServiceState, router_with_state};
use connectors_sdk::{Adapter, Credential, Secret};
use serde_json::{Value, json};
use std::{
    os::unix::fs::PermissionsExt,
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
    time::Duration,
};
use tokio::sync::RwLock;

const INSTANCE: &str = "leaf";
const TOKEN: &str = "service-token";
/// 2033-05-18: a `not_after` every case but the expiry case is before.
const LATER: i64 = 2_000_000_000_000;

struct FixedSecret;
#[async_trait]
impl Credential for FixedSecret {
    async fn resolve(&self) -> Result<Secret> {
        Ok(Secret(TOKEN.as_bytes().to_vec()))
    }
}

/// `issue.read` (`profile: read`) and two writes (`profile: mutation`). Each
/// call is one provider request: it is counted, and a write waits for a
/// shared hold of `gate`, so a test that holds it exclusively keeps the write in flight.
struct Fixture {
    requests: Arc<AtomicUsize>,
    gate: Arc<RwLock<()>>,
}
fn operation(id: &str, profile: &str) -> Operation {
    Operation {
        id: id.into(),
        description: format!("{id} fixture"),
        contract: "operations/v1alpha1".into(),
        profile: profile.into(),
        input_schema: json!({"type":"object"}),
        output_schema: json!({"type":"object"}),
    }
}
#[async_trait]
impl Adapter for Fixture {
    fn descriptor(&self) -> Descriptor {
        Descriptor {
            version: WIRE_VERSION.into(),
            instance: INSTANCE.into(),
            adapter: "fixture".into(),
            revision: "rev-1".into(),
            configuration_schema: json!({"type":"object"}),
            operations: vec![
                operation("issue.read", "read"),
                operation("issue.create", "mutation"),
                operation("issue.close", "mutation"),
            ],
        }
    }
    async fn invoke(&self, operation: &str, input: Value) -> Result<Value> {
        let number = self.requests.fetch_add(1, Ordering::SeqCst) + 1;
        if operation != "issue.read" {
            let _shared = self.gate.read().await;
        }
        Ok(json!({"operation": operation, "number": number, "input": input}))
    }
}

struct Host {
    endpoint: String,
    task: tokio::task::JoinHandle<()>,
    state: Arc<ServiceState>,
    grants: PathBuf,
    requests: Arc<AtomicUsize>,
    gate: Arc<RwLock<()>>,
    root: Arc<tempfile::TempDir>,
}
impl Drop for Host {
    fn drop(&mut self) {
        self.task.abort();
    }
}

/// The grant every case starts from: `issue.create` on `acme/widgets`.
fn grant(id: &str, max_per_hour: u32, not_after: i64) -> Value {
    json!({
        "id": id,
        "instance": INSTANCE,
        "operation": "issue.create",
        "target": [
            {"pointer": "/owner", "value": "acme"},
            {"pointer": "/repo", "value": "widgets"}
        ],
        "not_after_unix_ms": not_after,
        "max_per_hour": max_per_hour
    })
}
fn grant_file(grants: Value, revoked: Value) -> Value {
    json!({"format": "connectors.standing-grants/1", "grants": grants, "revoked": revoked})
}

impl Host {
    /// The operator rewrites the mounted file; the host reads it on the next
    /// call, with no restart.
    fn write_grants(&self, document: &Value) {
        let staged = self.grants.with_extension("new");
        std::fs::write(&staged, serde_json::to_vec(document).unwrap()).unwrap();
        std::fs::set_permissions(&staged, std::fs::Permissions::from_mode(0o644)).unwrap();
        std::fs::rename(&staged, &self.grants).unwrap();
    }
    fn requests(&self) -> usize {
        self.requests.load(Ordering::SeqCst)
    }
}

async fn host(document: Option<Value>) -> Host {
    let root = Arc::new(tempfile::tempdir().unwrap());
    let path = root.path().join("state");
    std::fs::create_dir(&path).unwrap();
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o700)).unwrap();
    let host = start(root).await;
    if let Some(document) = document {
        host.write_grants(&document);
    }
    host
}

/// A host process on `root`'s state directory and grant file; a second call
/// on the same root is that host restarted.
async fn start(root: Arc<tempfile::TempDir>) -> Host {
    let path = root.path().join("state");
    let grants = root.path().join("grants.json");
    let requests = Arc::new(AtomicUsize::new(0));
    let gate = Arc::new(RwLock::new(()));
    let adapter = Arc::new(Fixture {
        requests: requests.clone(),
        gate: gate.clone(),
    });
    let state = Arc::new(
        ServiceState::open(&path, INSTANCE, "fixture", "rev-1")
            .unwrap()
            .with_standing_grants(Some(grants.clone())),
    );
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let app = router_with_state(adapter, Arc::new(FixedSecret), Some(state.clone()));
    let task = tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    Host {
        endpoint: format!("http://{address}/v1alpha2/invoke"),
        task,
        state,
        grants,
        requests,
        gate,
        root,
    }
}

async fn invoke(host: &Host, body: &Value) -> (u16, InvokeResponse) {
    let _ = rustls::crypto::ring::default_provider().install_default();
    let response = reqwest::Client::new()
        .post(&host.endpoint)
        .bearer_auth(TOKEN)
        .body(serde_json::to_vec(body).unwrap())
        .send()
        .await
        .unwrap();
    let status = response.status().as_u16();
    let bytes = response.bytes().await.unwrap();
    (status, InvokeResponse::decode(&bytes).unwrap())
}

fn write(operation: &str, request_id: &str, key: Option<&str>, input: Value) -> Value {
    let mut body = json!({
        "version": "v1alpha2",
        "request_id": request_id,
        "operation": operation,
        "revision": "rev-1",
        "input": input
    });
    if let Some(key) = key {
        body["idempotency_key"] = json!(key);
    }
    body
}
fn widgets(title: &str) -> Value {
    json!({"owner": "acme", "repo": "widgets", "title": title})
}

/// A refusal before the anchor: the named code, the request echoed, no
/// record, no `mutation`, no attempt and no provider request.
fn assert_refused_before_anything(
    host: &Host,
    (status, response): &(u16, InvokeResponse),
    code: ErrorCode,
    http: u16,
    attempts_before: u32,
    requests_before: usize,
) {
    assert_eq!(*status, http, "{response:?}");
    assert_eq!(response.status, ResponseStatus::Error);
    assert_eq!(response.error.as_ref().map(|e| e.code), Some(code));
    assert_eq!(response.audit_status, AuditStatus::Unavailable);
    assert!(response.audit_ref.is_none() && response.mutation.is_none());
    assert_eq!(host.state.attempt_count().unwrap(), attempts_before);
    assert_eq!(
        host.requests(),
        requests_before,
        "a refused write reached the provider"
    );
}

/// Scenario `v1alpha2-invoke-keyed-write-under-standing-grant`.
#[tokio::test]
async fn a_keyed_write_under_a_matching_grant_is_dispatched_once_and_records_the_grant_id() {
    let host = host(Some(grant_file(
        json!([grant("g-widgets", 10, LATER)]),
        json!([]),
    )))
    .await;
    let (status, response) = invoke(
        &host,
        &write("issue.create", "w-1", Some("key-1"), widgets("first")),
    )
    .await;
    assert_eq!(status, 200, "{response:?}");
    assert_eq!(response.status, ResponseStatus::Success);
    assert_eq!(response.audit_status, AuditStatus::Complete);
    assert_eq!(host.requests(), 1);
    let mutation = response.mutation.unwrap();
    assert_eq!(mutation.classification, EffectKnowledge::Applied);
    assert!(!mutation.replayed);
    assert_eq!(mutation.original_request_id.as_deref(), Some("w-1"));
    let attempt = mutation.attempt.unwrap();
    assert_eq!(attempt.instance, INSTANCE);
    assert_eq!(
        host.state.grant_of(attempt.id.as_str()).unwrap().as_deref(),
        Some("g-widgets"),
        "the attempt does not record the grant it was admitted under"
    );
    assert_eq!(host.state.attempt_count().unwrap(), 1);
}

/// Scenario `v1alpha2-invoke-keyed-replay`.
#[tokio::test]
async fn the_same_key_and_input_replays_the_original_outcome_with_no_second_request() {
    let host = host(Some(grant_file(
        json!([grant("g-widgets", 1, LATER)]),
        json!([]),
    )))
    .await;
    let (_, original) = invoke(
        &host,
        &write("issue.create", "w-1", Some("key-1"), widgets("first")),
    )
    .await;
    let original_attempt = original.mutation.clone().unwrap().attempt.unwrap();
    // Another request ID, the same key and input; the grant's one write per
    // hour is spent, and a replay is not counted against it.
    let (status, replay) = invoke(
        &host,
        &write("issue.create", "w-1-retry", Some("key-1"), widgets("first")),
    )
    .await;
    assert_eq!(status, 200, "{replay:?}");
    assert_eq!(replay.status, ResponseStatus::Success);
    assert_eq!(
        replay.result, original.result,
        "the replay changed the result"
    );
    assert_eq!(replay.request_id.as_deref(), Some("w-1-retry"));
    let mutation = replay.mutation.unwrap();
    assert!(mutation.replayed);
    assert_eq!(mutation.classification, EffectKnowledge::Applied);
    assert_eq!(mutation.attempt.unwrap(), original_attempt);
    assert_eq!(mutation.original_request_id.as_deref(), Some("w-1"));
    // The replay has its own anchor.
    assert_ne!(replay.audit_ref, original.audit_ref);
    assert_eq!(replay.audit_status, AuditStatus::Complete);
    assert_eq!(host.requests(), 1, "the replay reached the provider");
    assert_eq!(host.state.attempt_count().unwrap(), 1);
}

#[tokio::test]
async fn the_same_key_with_other_input_is_refused_conflict_with_no_request() {
    let host = host(Some(grant_file(
        json!([grant("g-widgets", 10, LATER)]),
        json!([]),
    )))
    .await;
    invoke(
        &host,
        &write("issue.create", "w-1", Some("key-1"), widgets("first")),
    )
    .await;
    let refused = invoke(
        &host,
        &write("issue.create", "w-2", Some("key-1"), widgets("second")),
    )
    .await;
    assert_refused_before_anything(&host, &refused, ErrorCode::IdempotencyConflict, 409, 1, 1);
    assert_eq!(refused.1.request_id.as_deref(), Some("w-2"));
    // The same key for another operation is the same conflict.
    let refused = invoke(
        &host,
        &write("issue.close", "w-3", Some("key-1"), widgets("first")),
    )
    .await;
    assert_refused_before_anything(&host, &refused, ErrorCode::IdempotencyConflict, 409, 1, 1);
}

#[tokio::test]
async fn a_keyed_write_without_a_key_is_refused_with_no_request() {
    let host = host(Some(grant_file(
        json!([grant("g-widgets", 10, LATER)]),
        json!([]),
    )))
    .await;
    let refused = invoke(&host, &write("issue.create", "w-1", None, widgets("first"))).await;
    assert_refused_before_anything(&host, &refused, ErrorCode::InvalidInput, 400, 0, 0);
}

/// Scenario `v1alpha2-invoke-write-outside-grant-refused`: another operation.
#[tokio::test]
async fn a_write_of_another_operation_is_refused_approval_required_with_no_request() {
    let host = host(Some(grant_file(
        json!([grant("g-widgets", 10, LATER)]),
        json!([]),
    )))
    .await;
    let refused = invoke(
        &host,
        &write("issue.close", "w-1", Some("key-1"), widgets("first")),
    )
    .await;
    assert_refused_before_anything(&host, &refused, ErrorCode::ApprovalRequired, 403, 0, 0);
}

/// Scenario `v1alpha2-invoke-write-outside-grant-refused`: another target.
#[tokio::test]
async fn a_write_to_another_target_is_refused_with_no_request() {
    let host = host(Some(grant_file(
        json!([grant("g-widgets", 10, LATER)]),
        json!([]),
    )))
    .await;
    for (index, input) in [
        json!({"owner": "acme", "repo": "gadgets", "title": "t"}),
        json!({"owner": "other", "repo": "widgets", "title": "t"}),
        // A value of another JSON type is another target.
        json!({"owner": "acme", "repo": ["widgets"], "title": "t"}),
    ]
    .into_iter()
    .enumerate()
    {
        let refused = invoke(
            &host,
            &write(
                "issue.create",
                &format!("w-{index}"),
                Some(&format!("key-{index}")),
                input,
            ),
        )
        .await;
        assert_refused_before_anything(&host, &refused, ErrorCode::ApprovalRequired, 403, 0, 0);
    }
}

#[tokio::test]
async fn a_write_missing_a_target_member_is_refused_with_no_request() {
    let host = host(Some(grant_file(
        json!([grant("g-widgets", 10, LATER)]),
        json!([]),
    )))
    .await;
    let refused = invoke(
        &host,
        &write(
            "issue.create",
            "w-1",
            Some("key-1"),
            json!({"owner": "acme", "title": "t"}),
        ),
    )
    .await;
    assert_refused_before_anything(&host, &refused, ErrorCode::ApprovalRequired, 403, 0, 0);
}

#[tokio::test]
async fn a_write_after_not_after_is_refused_with_no_request() {
    let host = host(Some(grant_file(
        json!([grant("g-widgets", 10, 1)]),
        json!([]),
    )))
    .await;
    let refused = invoke(
        &host,
        &write("issue.create", "w-1", Some("key-1"), widgets("first")),
    )
    .await;
    assert_refused_before_anything(&host, &refused, ErrorCode::ApprovalRequired, 403, 0, 0);
}

#[tokio::test]
async fn a_write_after_revocation_is_refused_with_no_request() {
    let host = host(Some(grant_file(
        json!([grant("g-widgets", 10, LATER)]),
        json!([]),
    )))
    .await;
    let (status, _) = invoke(
        &host,
        &write("issue.create", "w-1", Some("key-1"), widgets("first")),
    )
    .await;
    assert_eq!(status, 200);
    // Listed as revoked: refused from the next call on, with no restart.
    host.write_grants(&grant_file(
        json!([grant("g-widgets", 10, LATER)]),
        json!(["g-widgets"]),
    ));
    let refused = invoke(
        &host,
        &write("issue.create", "w-2", Some("key-2"), widgets("second")),
    )
    .await;
    assert_refused_before_anything(&host, &refused, ErrorCode::ApprovalRequired, 403, 1, 1);
    // Removed from the file: the same.
    host.write_grants(&grant_file(json!([]), json!([])));
    let refused = invoke(
        &host,
        &write("issue.create", "w-3", Some("key-3"), widgets("third")),
    )
    .await;
    assert_refused_before_anything(&host, &refused, ErrorCode::ApprovalRequired, 403, 1, 1);
    // A replay of the write admitted before the revocation is still a replay.
    let (status, replay) = invoke(
        &host,
        &write("issue.create", "w-1-retry", Some("key-1"), widgets("first")),
    )
    .await;
    assert_eq!(status, 200, "{replay:?}");
    assert!(replay.mutation.unwrap().replayed);
    assert_eq!(host.requests(), 1);
}

#[tokio::test]
async fn a_write_over_max_per_hour_is_refused_with_no_request() {
    let host = host(Some(grant_file(
        json!([grant("g-widgets", 2, LATER)]),
        json!([]),
    )))
    .await;
    for index in 1..=2 {
        let (status, _) = invoke(
            &host,
            &write(
                "issue.create",
                &format!("w-{index}"),
                Some(&format!("key-{index}")),
                widgets(&format!("title {index}")),
            ),
        )
        .await;
        assert_eq!(status, 200);
    }
    let refused = invoke(
        &host,
        &write("issue.create", "w-3", Some("key-3"), widgets("title 3")),
    )
    .await;
    assert_refused_before_anything(&host, &refused, ErrorCode::RateLimited, 429, 2, 2);
    let retry = refused.1.error.unwrap().retry_after_seconds.unwrap();
    assert!((1..=3600).contains(&retry), "retry_after_seconds {retry}");
}

#[tokio::test]
async fn a_write_with_no_grant_is_refused_approval_required_with_no_request() {
    // No grant file at all, then an empty one, then one for another instance.
    let host = host(None).await;
    let refused = invoke(
        &host,
        &write("issue.create", "w-1", Some("key-1"), widgets("first")),
    )
    .await;
    assert_refused_before_anything(&host, &refused, ErrorCode::ApprovalRequired, 403, 0, 0);
    host.write_grants(&grant_file(json!([]), json!([])));
    let refused = invoke(
        &host,
        &write("issue.create", "w-2", Some("key-2"), widgets("first")),
    )
    .await;
    assert_refused_before_anything(&host, &refused, ErrorCode::ApprovalRequired, 403, 0, 0);
    let mut elsewhere = grant("g-other", 10, LATER);
    elsewhere["instance"] = json!("other-instance");
    host.write_grants(&grant_file(json!([elsewhere]), json!([])));
    let refused = invoke(
        &host,
        &write("issue.create", "w-3", Some("key-3"), widgets("first")),
    )
    .await;
    assert_refused_before_anything(&host, &refused, ErrorCode::ApprovalRequired, 403, 0, 0);
}

/// compatibility.md § 2.1: `approval`, `executor` and `connection` stay
/// refused with `unsupported`, even for a write a grant covers.
#[tokio::test]
async fn an_approval_member_is_still_refused_unsupported() {
    let host = host(Some(grant_file(
        json!([grant("g-widgets", 10, LATER)]),
        json!([]),
    )))
    .await;
    for (member, value) in [
        ("approval", json!({"reference": "r", "evidence": "e"})),
        ("executor", json!(null)),
        ("connection", json!("c1")),
    ] {
        let mut body = write("issue.create", "w-1", Some("key-1"), widgets("first"));
        body[member] = value;
        let refused = invoke(&host, &body).await;
        assert_refused_before_anything(&host, &refused, ErrorCode::Unsupported, 400, 0, 0);
        assert!(refused.1.request_id.is_none());
    }
}

#[tokio::test]
async fn reads_are_unchanged_by_a_standing_grant() {
    for document in [
        None,
        Some(grant_file(
            json!([grant("g-widgets", 1, LATER)]),
            json!(["g-widgets"]),
        )),
    ] {
        let host = host(document).await;
        for index in 0..3 {
            let (status, response) = invoke(
                &host,
                &write("issue.read", &format!("r-{index}"), None, widgets("first")),
            )
            .await;
            assert_eq!(status, 200, "{response:?}");
            assert_eq!(response.status, ResponseStatus::Success);
            assert_eq!(response.audit_status, AuditStatus::Complete);
            assert!(response.mutation.is_none());
        }
        assert_eq!(host.requests(), 3);
        assert_eq!(host.state.attempt_count().unwrap(), 0);
    }
}

/// compatibility.md § 2.1, `v1alpha2-invoke-refuses-idempotency-key`: a key on
/// an operation that is not a write is `unsupported` before the anchor.
#[tokio::test]
async fn a_key_on_a_read_is_refused_unsupported_with_no_request() {
    let host = host(Some(grant_file(
        json!([grant("g-widgets", 10, LATER)]),
        json!([]),
    )))
    .await;
    let refused = invoke(
        &host,
        &write("issue.read", "r-1", Some("key-1"), widgets("first")),
    )
    .await;
    assert_refused_before_anything(&host, &refused, ErrorCode::Unsupported, 400, 0, 0);
    assert_eq!(refused.1.request_id.as_deref(), Some("r-1"));
}

/// The host-side gate against one host: a write is dispatched only when it
/// is keyed and a standing grant covers it; every other request below is
/// refused before the anchor with its named code. Of the six requests only
/// the last reaches the provider. This replaces the first binding's
/// `v1alpha2-invoke-keyed-or-approval-operation-unsupported`
/// (story:served-keyed-writes-under-a-standing-approval).
#[tokio::test]
async fn the_host_gate_dispatches_only_a_keyed_write_a_grant_covers() {
    let host = host(Some(grant_file(
        json!([grant("g-widgets", 10, LATER)]),
        json!([]),
    )))
    .await;
    let mut approval = write("issue.create", "w-approval", Some("k-a"), widgets("t"));
    approval["approval"] = json!({"reference": "r", "evidence": "e"});
    let gadgets = json!({"owner": "acme", "repo": "gadgets", "title": "t"});
    let refusals = [
        // A § 4 approval member: the binding carries no per-request approval.
        (approval, ErrorCode::Unsupported, 400),
        // A key on a read: only a write's attempt can be replayed.
        (
            write("issue.read", "r-keyed", Some("k-r"), widgets("t")),
            ErrorCode::Unsupported,
            400,
        ),
        // A write without a key.
        (
            write("issue.create", "w-unkeyed", None, widgets("t")),
            ErrorCode::InvalidInput,
            400,
        ),
        // A keyed write of an operation no grant names.
        (
            write(
                "issue.close",
                "w-other-operation",
                Some("k-c"),
                widgets("t"),
            ),
            ErrorCode::ApprovalRequired,
            403,
        ),
        // A keyed write to a target the grant does not name.
        (
            write("issue.create", "w-other-target", Some("k-t"), gadgets),
            ErrorCode::ApprovalRequired,
            403,
        ),
    ];
    for (body, code, http) in refusals {
        let refused = invoke(&host, &body).await;
        assert_refused_before_anything(&host, &refused, code, http, 0, 0);
    }
    let (status, response) = invoke(
        &host,
        &write("issue.create", "w-admitted", Some("k-ok"), widgets("t")),
    )
    .await;
    assert_eq!(status, 200, "{response:?}");
    let mutation = response.mutation.unwrap();
    assert_eq!(mutation.classification, EffectKnowledge::Applied);
    assert_eq!(host.requests(), 1);
    assert_eq!(host.state.attempt_count().unwrap(), 1);
}

/// The keyed reservation holds across concurrent requests: eight requests
/// with one key and input, all in flight while the original write is held at
/// the provider, give exactly one provider request and one attempt; every
/// other answer is a replay of it.
#[tokio::test]
async fn concurrent_requests_with_one_key_make_one_provider_request() {
    let host = host(Some(grant_file(
        json!([grant("g-widgets", 100, LATER)]),
        json!([]),
    )))
    .await;
    let held = host.gate.clone().write_owned().await;
    let host = Arc::new(host);
    let mut calls = Vec::new();
    for index in 0..8 {
        let host = host.clone();
        calls.push(tokio::spawn(async move {
            invoke(
                &host,
                &write(
                    "issue.create",
                    &format!("w-{index}"),
                    Some("one-key"),
                    widgets("first"),
                ),
            )
            .await
        }));
    }
    // Let every request reach the host before the provider answers.
    tokio::time::sleep(Duration::from_millis(500)).await;
    drop(held);
    let mut originals = 0;
    let mut attempts = std::collections::BTreeSet::new();
    for call in calls {
        let (status, response) = call.await.unwrap();
        assert_eq!(status, 200, "{response:?}");
        let mutation = response.mutation.unwrap();
        assert_eq!(mutation.classification, EffectKnowledge::Applied);
        if !mutation.replayed {
            originals += 1;
        }
        attempts.insert(mutation.attempt.unwrap().id.as_str().to_owned());
    }
    assert_eq!(originals, 1);
    assert_eq!(attempts.len(), 1);
    assert_eq!(host.requests(), 1, "one key reached the provider twice");
    assert_eq!(host.state.attempt_count().unwrap(), 1);
}

/// Both stores survive a restart: the keyed reservation (the ledger in
/// `metadata.sqlite3`) and the hourly count (`standing-grants.sqlite3`). A
/// replay against the restarted host dispatches nothing and answers the
/// original outcome; a new key under the spent grant is still `rate_limited`.
#[tokio::test]
async fn a_restart_keeps_the_key_and_the_hourly_count() {
    let first = host(Some(grant_file(
        json!([grant("g-widgets", 1, LATER)]),
        json!([]),
    )))
    .await;
    let (_, original) = invoke(
        &first,
        &write("issue.create", "w-1", Some("key-1"), widgets("first")),
    )
    .await;
    let root = first.root.clone();
    drop(first);
    let restarted = start(root).await;
    let (status, replay) = invoke(
        &restarted,
        &write(
            "issue.create",
            "w-after-restart",
            Some("key-1"),
            widgets("first"),
        ),
    )
    .await;
    assert_eq!(status, 200, "{replay:?}");
    assert_eq!(replay.result, original.result);
    assert!(replay.mutation.unwrap().replayed);
    let refused = invoke(
        &restarted,
        &write("issue.create", "w-2", Some("key-2"), widgets("second")),
    )
    .await;
    assert_refused_before_anything(&restarted, &refused, ErrorCode::RateLimited, 429, 1, 0);
}

/// The grant file is custody: a symbolic link, a group- or world-writable
/// file, an oversized file and a malformed one are each `unavailable`, with
/// no provider request.
#[tokio::test]
async fn an_unsafe_grant_file_is_refused_unavailable_with_no_request() {
    let host = host(Some(grant_file(
        json!([grant("g-widgets", 10, LATER)]),
        json!([]),
    )))
    .await;
    let good = std::fs::read(&host.grants).unwrap();
    let elsewhere = host.grants.with_extension("real");
    std::fs::write(&elsewhere, &good).unwrap();
    std::fs::remove_file(&host.grants).unwrap();
    std::os::unix::fs::symlink(&elsewhere, &host.grants).unwrap();
    let refused = invoke(
        &host,
        &write("issue.create", "w-1", Some("key-1"), widgets("first")),
    )
    .await;
    assert_refused_before_anything(&host, &refused, ErrorCode::Unavailable, 503, 0, 0);
    std::fs::remove_file(&host.grants).unwrap();

    for mode in [0o664, 0o646] {
        std::fs::write(&host.grants, &good).unwrap();
        std::fs::set_permissions(&host.grants, std::fs::Permissions::from_mode(mode)).unwrap();
        let refused = invoke(
            &host,
            &write("issue.create", "w-1", Some("key-1"), widgets("first")),
        )
        .await;
        assert_refused_before_anything(&host, &refused, ErrorCode::Unavailable, 503, 0, 0);
        std::fs::remove_file(&host.grants).unwrap();
    }

    let mut oversized = grant_file(json!([grant("g-widgets", 10, LATER)]), json!([]));
    oversized["revoked"] = json!(vec!["x".repeat(100); 700]);
    for document in [
        serde_json::to_vec(&oversized).unwrap(),
        br#"{"format":"connectors.standing-grants/1","grants":[{"id":"g"}]}"#.to_vec(),
        b"not json".to_vec(),
    ] {
        std::fs::write(&host.grants, &document).unwrap();
        std::fs::set_permissions(&host.grants, std::fs::Permissions::from_mode(0o644)).unwrap();
        let refused = invoke(
            &host,
            &write("issue.create", "w-1", Some("key-1"), widgets("first")),
        )
        .await;
        assert_refused_before_anything(&host, &refused, ErrorCode::Unavailable, 503, 0, 0);
    }
}
