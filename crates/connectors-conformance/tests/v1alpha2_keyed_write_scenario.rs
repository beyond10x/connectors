//! Conformance scenarios `v1alpha2-invoke-keyed-write-under-standing-grant`,
//! `v1alpha2-invoke-keyed-replay` and
//! `v1alpha2-invoke-write-outside-grant-refused`
//! (contracts/service/compatibility.md § 2.1, Keyed writes under a standing
//! grant; story:served-keyed-writes-under-a-standing-approval), run through
//! the released client's v1alpha2 binding against the real HTTP host whose
//! configuration sets `standing_grants`. A host without them is held by
//! `v1alpha2_write_scenario.rs` and `client_v1alpha2.rs`.
#![cfg(unix)]
mod support;

use connectors_client::Client;
use connectors_core::v1alpha2::{AuditStatus, EffectKnowledge, ErrorCode};
use serde_json::json;
use support::{Host, TOKEN};

/// `v1alpha2-invoke-keyed-write-under-standing-grant`: a keyed write a
/// standing grant covers is dispatched once, its `mutation` names the attempt,
/// and the attempt records the grant it was admitted under.
#[tokio::test]
async fn v1alpha2_invoke_keyed_write_under_standing_grant() {
    let host = Host::start_with_standing_grants().await;
    let client = Client::new(&host.endpoint, TOKEN.into(), true).unwrap();
    let descriptor = client.describe().await.unwrap();
    let invoked = client
        .invoke_v1alpha2_keyed(&descriptor, "write", json!({"value":1}), "k-1")
        .await
        .expect("a write under the grant");
    assert_eq!(invoked.result, json!({"value":1}));
    let mutation = invoked.mutation.expect("the write's mutation");
    assert_eq!(mutation.classification, EffectKnowledge::Applied);
    assert!(!mutation.replayed);
    let attempt = mutation.attempt.expect("the attempt reference");
    assert_eq!(
        host.state.grant_of(attempt.id.as_str()).unwrap().as_deref(),
        Some("g-write-1")
    );
    assert_eq!(host.calls(), 1);
}

/// `v1alpha2-invoke-keyed-replay`: the same key and input again is the
/// original outcome, `replayed: true` with the original attempt and request,
/// and no second provider request.
#[tokio::test]
async fn v1alpha2_invoke_keyed_replay() {
    let host = Host::start_with_standing_grants().await;
    let client = Client::new(&host.endpoint, TOKEN.into(), true).unwrap();
    let descriptor = client.describe().await.unwrap();
    let write = || client.invoke_v1alpha2_keyed(&descriptor, "write", json!({"value":7}), "k-7");
    let original = write().await.expect("the original write");
    let replay = write().await.expect("the replay");
    assert_eq!(replay.result, original.result);
    let (original, replay) = (original.mutation.unwrap(), replay.mutation.unwrap());
    assert!(replay.replayed);
    assert_eq!(replay.classification, EffectKnowledge::Applied);
    assert_eq!(replay.attempt, original.attempt);
    assert_eq!(replay.original_request_id, original.original_request_id);
    assert_eq!(host.state.attempt_count().unwrap(), 1);
    assert_eq!(host.calls(), 1);
}

/// `v1alpha2-invoke-write-outside-grant-refused`: on a host with standing
/// grants, a keyed write no grant covers is `approval_required` and an
/// unkeyed write `invalid_input`, each before the anchor, with no `mutation`,
/// no attempt and no provider request.
#[tokio::test]
async fn v1alpha2_invoke_write_outside_grant_refused() {
    let host = Host::start_with_standing_grants().await;
    let client = Client::new(&host.endpoint, TOKEN.into(), true).unwrap();
    let descriptor = client.describe().await.unwrap();
    let outside = client
        .invoke_v1alpha2_keyed(&descriptor, "write", json!({"value": 2}), "k-2")
        .await
        .unwrap_err();
    let unkeyed = client
        .invoke_v1alpha2(&descriptor, "write", json!({"value": 1}))
        .await
        .unwrap_err();
    for (failure, code) in [
        (outside, ErrorCode::ApprovalRequired),
        (unkeyed, ErrorCode::InvalidInput),
    ] {
        assert_eq!(failure.error.code, code);
        assert!(failure.mutation.is_none());
        assert_eq!(failure.audit_ref, None);
        assert_eq!(failure.audit_status, Some(AuditStatus::Unavailable));
    }
    assert_eq!(host.state.attempt_count().unwrap(), 0);
    assert_eq!(host.calls(), 0);
}
