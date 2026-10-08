//! `connectors-client`'s v1alpha2 binding against the real HTTP host
//! (contracts/service/compatibility.md § 2, § 2.1). The client crate keeps its
//! Rust 1.88 check with `--all-targets`, so its host-backed cases live here,
//! beside the host.
mod support;

use axum::{Router, http::StatusCode, routing::post};
use connectors_client::Client;
use connectors_core::v1alpha2::{AuditStatus, EffectKnowledge, ErrorCode};
use connectors_sdk::Adapter;
use serde_json::{Value, json};
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};
use support::{Fixture, Host, TOKEN};

#[tokio::test]
async fn the_returned_attempt_id_is_the_one_the_host_recorded() {
    let host = Host::start().await;
    let client = Client::new(&host.endpoint, TOKEN.into(), true).unwrap();
    let descriptor = client.describe().await.unwrap();
    let invoked = client
        .invoke_v1alpha2(&descriptor, "write", json!({"value":1}))
        .await
        .unwrap();
    assert_eq!(invoked.result, json!({"value":1}));
    assert_eq!(invoked.audit_status, AuditStatus::Complete);
    assert!(invoked.audit_ref.is_some());
    let mutation = invoked.mutation.expect("a write names its attempt");
    assert_eq!(mutation.classification, EffectKnowledge::Applied);
    let attempt = mutation.attempt.expect("the recorded attempt");
    assert_eq!(attempt.instance, "leaf");
    let record = host.state.attempt(attempt.id.as_str()).unwrap();
    assert_eq!(record.reference.attempt_id.to_string(), attempt.id.as_str());
    assert_eq!(
        Some(record.request_id.as_str()),
        mutation.original_request_id.as_deref()
    );
    assert_eq!(host.state.attempt_count().unwrap(), 1);
    assert_eq!(host.calls(), 1);

    // A read on the same binding carries no mutation and records nothing.
    let read = client
        .invoke_v1alpha2(&descriptor, "read", json!({"value":2}))
        .await
        .unwrap();
    assert_eq!(read.result, json!({"value":2}));
    assert!(read.mutation.is_none());
    assert_eq!(host.state.attempt_count().unwrap(), 1);

    // `Client::invoke` stays on the legacy binding and returns the bare value.
    let legacy: Value = client
        .invoke(&descriptor, "read", json!({"value":3}))
        .await
        .unwrap();
    assert_eq!(legacy, json!({"value":3}));
    assert_eq!(host.state.attempt_count().unwrap(), 1);
}

#[tokio::test]
async fn a_refusal_carries_its_code_and_no_attempt() {
    let host = Host::start().await;
    let client = Client::new(&host.endpoint, TOKEN.into(), true).unwrap();
    let mut descriptor = client.describe().await.unwrap();
    descriptor.revision = "rev-0".into();
    let failure = client
        .invoke_v1alpha2(&descriptor, "write", json!({"value":1}))
        .await
        .unwrap_err();
    assert_eq!(failure.error.code, ErrorCode::StaleDescription);
    assert!(failure.mutation.is_none());
    assert_eq!(failure.audit_status, Some(AuditStatus::Unavailable));
    assert_eq!(host.state.attempt_count().unwrap(), 0);
    assert_eq!(host.calls(), 0);
}

/// A host released before the binding serves `/v1/*` only, so
/// `/v1alpha2/invoke` is an ordinary route failure: 404 without an envelope.
/// The stand-in counts every `/v1/invoke` it receives.
#[tokio::test]
async fn an_older_host_is_unsupported_and_the_client_does_not_fall_back_to_v1() {
    let legacy_hits = Arc::new(AtomicUsize::new(0));
    let hits = legacy_hits.clone();
    let app = Router::new().route(
        "/v1/invoke",
        post(move || {
            let hits = hits.clone();
            async move {
                hits.fetch_add(1, Ordering::SeqCst);
                StatusCode::INTERNAL_SERVER_ERROR
            }
        }),
    );
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let task = tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
    let client = Client::new(&format!("http://{address}/"), TOKEN.into(), true).unwrap();
    let descriptor = Fixture(Arc::new(AtomicUsize::new(0))).descriptor();
    let failure = client
        .invoke_v1alpha2(&descriptor, "write", json!({"value":1}))
        .await
        .unwrap_err();
    assert_eq!(failure.error.code, ErrorCode::Unsupported);
    assert!(failure.mutation.is_none());
    assert_eq!(failure.audit_ref, None);
    assert_eq!(legacy_hits.load(Ordering::SeqCst), 0, "no fallback to /v1");
    task.abort();
}
