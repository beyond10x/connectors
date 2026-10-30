//! Conformance scenarios `v1alpha2-invoke-write-applied-names-attempt`,
//! `v1alpha2-invoke-keyed-write-under-standing-grant`,
//! `v1alpha2-invoke-keyed-replay` and
//! `v1alpha2-invoke-write-outside-grant-refused`
//! (contracts/service/compatibility.md § 2.1), run through the released
//! client's v1alpha2 binding against the real HTTP host.
#![cfg(unix)]
mod support;

use connectors_client::Client;
use connectors_core::v1alpha2::{AuditStatus, EffectKnowledge, ErrorCode};
use connectors_host::local::{
    audit::{self, Reference},
    mutations,
};
use serde_json::json;
use support::{Host, TOKEN};

#[tokio::test]
async fn v1alpha2_invoke_write_applied_names_attempt() {
    let host = Host::start().await;
    let client = Client::new(&host.endpoint, TOKEN.into(), true).unwrap();
    let descriptor = client.describe().await.unwrap();

    // A write is keyed and admitted under the host's standing grant
    // (story:served-keyed-writes-under-a-standing-approval).
    let invoked = client
        .invoke_v1alpha2_keyed(&descriptor, "write", json!({"value":7}), Some("write-7"))
        .await
        .expect("a successful write");

    // mutation: {classification: applied, attempt: {instance, id},
    // original_request_id: <request_id>, replayed: false, cause: null}
    assert_eq!(invoked.result, json!({"value":7}));
    let mutation = invoked.mutation.expect("the write's mutation");
    assert_eq!(mutation.classification, EffectKnowledge::Applied);
    assert!(!mutation.replayed);
    assert_eq!(mutation.cause, None);
    let attempt = mutation.attempt.expect("the attempt reference");
    assert_eq!(attempt.instance, descriptor.instance);
    let request_id = mutation
        .original_request_id
        .expect("the request the attempt answers");

    // The host's AttemptRecord under that id reads back, settled.
    let record = host.state.attempt(attempt.id.as_str()).unwrap();
    assert_eq!(record.request_id, request_id);
    assert_eq!(record.state, mutations::State::Completed);
    assert_eq!(host.state.attempt_count().unwrap(), 1);

    // The audit anchor is complete and names the same attempt; one dispatch.
    assert_eq!(invoked.audit_status, AuditStatus::Complete);
    let anchored = audit::Store::new(&host.path, 100_000)
        .unwrap()
        .observe(&Reference {
            instance: descriptor.instance.clone(),
            audit_ref: invoked.audit_ref.expect("the audit_ref"),
        })
        .unwrap()
        .expect("the anchored record");
    assert_eq!(
        anchored.anchor.attempt_id.map(|id| id.to_string()),
        Some(attempt.id.as_str().to_owned())
    );
    assert!(anchored.final_observation.is_some());
    assert_eq!(host.calls(), 1);
}

/// `v1alpha2-invoke-keyed-write-under-standing-grant`: a keyed write a
/// standing grant covers is dispatched once, its `mutation` names the attempt,
/// and the attempt records the grant it was admitted under.
#[tokio::test]
async fn v1alpha2_invoke_keyed_write_under_standing_grant() {
    let host = Host::start().await;
    let client = Client::new(&host.endpoint, TOKEN.into(), true).unwrap();
    let descriptor = client.describe().await.unwrap();
    let invoked = client
        .invoke_v1alpha2_keyed(&descriptor, "write", json!({"value":1}), Some("k-1"))
        .await
        .expect("a write under the grant");
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
    let host = Host::start().await;
    let client = Client::new(&host.endpoint, TOKEN.into(), true).unwrap();
    let descriptor = client.describe().await.unwrap();
    let write =
        || client.invoke_v1alpha2_keyed(&descriptor, "write", json!({"value":7}), Some("k-7"));
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

/// `v1alpha2-invoke-write-outside-grant-refused`: a write no grant covers is
/// `approval_required`, and a write without a key `invalid_input`, each before
/// the anchor, with no `mutation`, no attempt and no provider request.
#[tokio::test]
async fn v1alpha2_invoke_write_outside_grant_refused() {
    let host = Host::start().await;
    let client = Client::new(&host.endpoint, TOKEN.into(), true).unwrap();
    let descriptor = client.describe().await.unwrap();
    for (key, value, code) in [
        (Some("k-2"), 2, ErrorCode::ApprovalRequired),
        (None, 1, ErrorCode::InvalidInput),
    ] {
        let failure = client
            .invoke_v1alpha2_keyed(&descriptor, "write", json!({"value": value}), key)
            .await
            .unwrap_err();
        assert_eq!(failure.error.code, code);
        assert!(failure.mutation.is_none());
        assert_eq!(failure.audit_ref, None);
        assert_eq!(failure.audit_status, Some(AuditStatus::Unavailable));
    }
    assert_eq!(host.state.attempt_count().unwrap(), 0);
    assert_eq!(host.calls(), 0);
}
