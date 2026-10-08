//! Conformance scenario `v1alpha2-invoke-write-applied-names-attempt`
//! (contracts/service/compatibility.md § 2.1), run through the released
//! client's v1alpha2 binding against the real HTTP host.
mod support;

use connectors_client::Client;
use connectors_core::v1alpha2::{AuditStatus, EffectKnowledge};
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

    let invoked = client
        .invoke_v1alpha2(&descriptor, "write", json!({"value":7}))
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
