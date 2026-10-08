//! Adversarial cases for the first v1alpha2 invoke binding codec.
//!
//! Each case names the rule it drives from: `contracts/service/compatibility.md`
//! § 2.1 (the rules of this binding, which narrow § 5) or a round-trip
//! property of [`InvokeResponse::encode`] / [`InvokeResponse::decode`] and
//! [`InvokeRequest::encode`] / [`InvokeRequest::decode`].
use connectors_core::v1alpha2::{InvokeRequest, InvokeResponse};
use serde_json::{Value, json};

fn decode(value: &Value) -> Result<InvokeResponse, connectors_core::v1alpha2::Error> {
    InvokeResponse::decode(&serde_json::to_vec(value).unwrap())
}

fn applied_write() -> Value {
    json!({
        "version": "v1alpha2",
        "request_id": "write-1",
        "status": "success",
        "result": {"id": "item-1"},
        "audit_ref": "aud-3",
        "audit_status": "complete",
        "mutation": {
            "classification": "applied",
            "attempt": {"instance": "fixture", "id": "00000000-0000-4000-8000-000000000001"},
            "original_request_id": "write-1",
            "replayed": false,
            "cause": null
        }
    })
}

/// § 2.1 `source_audit`: "Always omitted: no trustworthy leaf audit
/// observation exists on this binding (§5)."
#[test]
fn first_binding_refuses_a_source_audit() {
    let mut response = applied_write();
    response["source_audit"] =
        json!({"instance": "origin", "audit_ref": "src-1", "audit_status": "complete"});
    assert!(
        decode(&response).is_err(),
        "§ 2.1: source_audit is always omitted on this binding, yet decode accepted it"
    );
}

/// § 2.1 `audit_status`: "`not_required` is never answered here."
#[test]
fn first_binding_refuses_audit_status_not_required() {
    let response = json!({
        "version": "v1alpha2",
        "request_id": "read-6",
        "status": "success",
        "result": {},
        "audit_ref": null,
        "audit_status": "not_required"
    });
    assert!(
        decode(&response).is_err(),
        "§ 2.1: not_required is never answered on this binding, yet decode accepted it"
    );
}

/// § 2.1 `mutation`: "`replayed: false` (no replay on this binding)".
#[test]
fn first_binding_refuses_a_replayed_mutation() {
    let mut response = applied_write();
    response["mutation"]["replayed"] = json!(true);
    assert!(
        decode(&response).is_err(),
        "§ 2.1: no replay on this binding, yet decode accepted replayed: true"
    );
}

/// § 2.1 `mutation`: "`original_request_id` is this request's ID".
#[test]
fn first_binding_refuses_an_original_request_id_other_than_the_request_id() {
    let mut response = applied_write();
    response["mutation"]["original_request_id"] = json!("some-other-request");
    assert!(
        decode(&response).is_err(),
        "§ 2.1: original_request_id is this request's ID, yet decode accepted another"
    );
}

/// § 2.1 `Response`: "The binding emits the 13 base error codes and
/// `outcome_unknown`; no other extended code."
#[test]
fn first_binding_refuses_an_extended_code_other_than_outcome_unknown() {
    for code in ["approval_required", "not_granted", "idempotency_conflict", "revoked"] {
        let response = json!({
            "version": "v1alpha2",
            "request_id": "write-1",
            "status": "error",
            "error": {"code": code, "message": "refused"},
            "audit_ref": null,
            "audit_status": "unavailable"
        });
        assert!(
            decode(&response).is_err(),
            "§ 2.1: {code} is not emitted on this binding, yet decode accepted it"
        );
        let built: InvokeResponse = serde_json::from_value(response).unwrap();
        assert!(
            built.encode().is_err(),
            "§ 2.1: {code} is not emitted on this binding, yet encode wrote it"
        );
    }
}

/// Round trip: a success whose result holds an object with the key
/// `$serde_json::private::RawValue` (an ordinary provider string) passes
/// `check`, so `encode` writes it; `decode` must read back what `encode`
/// writes.
#[test]
fn encode_never_writes_a_response_decode_refuses_private_token_key() {
    for key in ["$serde_json::private::RawValue", "$serde_json::private::Number"] {
        let mut response = applied_write();
        response["result"] = json!({ key: "provider data" });
        let built: InvokeResponse = serde_json::from_value(response).unwrap();
        let bytes = built
            .encode()
            .expect("check admits any JSON result, so encode writes it");
        let reread = InvokeResponse::decode(&bytes);
        assert_eq!(
            reread.as_ref().ok(),
            Some(&built),
            "{key}: encode wrote bytes decode refused: {reread:?}"
        );
    }
}

/// Round trip: the same key inside a request `input`.
#[test]
fn request_encode_never_writes_bytes_decode_refuses_private_token_key() {
    let request = InvokeRequest {
        version: "v1alpha2".into(),
        request_id: "write-1".into(),
        operation: "item.write".into(),
        revision: "rev-1".into(),
        input: json!({"$serde_json::private::RawValue": "caller data"}),
    };
    let reread = InvokeRequest::decode(&request.encode());
    assert_eq!(
        reread.as_ref().ok(),
        Some(&request),
        "encode wrote bytes decode refused: {reread:?}"
    );
}

/// Round trip: a result at the deepest nesting `serde_json` reads on its own
/// (what an adapter parsing a provider body can hand over) passes `check`,
/// so `encode` writes it inside the envelope one level deeper; `decode` must
/// read it back.
#[test]
fn encode_never_writes_a_response_decode_refuses_nesting_depth() {
    let nested = |depth: usize| format!("{}{}", "[".repeat(depth), "]".repeat(depth));
    let deepest = (1..1024)
        .take_while(|&depth| serde_json::from_str::<Value>(&nested(depth)).is_ok())
        .last()
        .unwrap();
    let result: Value = serde_json::from_str(&nested(deepest)).unwrap();
    let mut response = applied_write();
    response["result"] = result;
    let built: InvokeResponse = serde_json::from_value(response).unwrap();
    let bytes = built
        .encode()
        .expect("check admits any JSON result, so encode writes it");
    let reread = InvokeResponse::decode(&bytes);
    assert!(
        reread.is_ok(),
        "depth {deepest}: encode wrote bytes decode refused: {reread:?}"
    );
}
