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
    for code in [
        "approval_required",
        "not_granted",
        "idempotency_conflict",
        "revoked",
    ] {
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

/// Round trip: a result holding an object with a serde_json private-token key
/// (an ordinary provider string), at any depth, is bytes `decode` refuses, so
/// `encode` refuses it; the same result without the key encodes and `decode`
/// reads it back.
#[test]
fn encode_never_writes_a_response_decode_refuses_private_token_key() {
    // Set the field directly: under serde_json `arbitrary_precision`,
    // `from_value` itself reads the number token as a number.
    let with_result = |result: Value| -> InvokeResponse {
        let mut built: InvokeResponse = serde_json::from_value(applied_write()).unwrap();
        built.result = Some(result);
        built
    };
    for key in [
        "$serde_json::private::RawValue",
        "$serde_json::private::Number",
    ] {
        for result in [
            json!({ key: "provider data" }),
            json!({"items": [{ key: "provider data" }]}),
        ] {
            let built = with_result(result.clone());
            assert!(
                built.encode().is_err(),
                "{key}: encode wrote {result} although decode refuses it"
            );
        }
    }
    let built = with_result(json!({"items": [{"ordinary": "provider data"}]}));
    let bytes = built.encode().expect("a result without the key encodes");
    let reread = InvokeResponse::decode(&bytes);
    assert_eq!(
        reread.as_ref().ok(),
        Some(&built),
        "encode wrote bytes decode refused: {reread:?}"
    );
}

/// Round trip: the same key inside a request `input`.
#[test]
fn request_encode_never_writes_bytes_decode_refuses_private_token_key() {
    let with_input = |input: Value| InvokeRequest {
        version: "v1alpha2".into(),
        request_id: "write-1".into(),
        operation: "item.write".into(),
        revision: "rev-1".into(),
        input,
    };
    for key in [
        "$serde_json::private::RawValue",
        "$serde_json::private::Number",
    ] {
        for input in [
            json!({ key: "caller data" }),
            json!([{"nested": { key: "caller data" }}]),
        ] {
            assert!(
                with_input(input.clone()).encode().is_err(),
                "{key}: encode wrote {input} although decode refuses it"
            );
        }
    }
    let request = with_input(json!({"ordinary": "caller data"}));
    let bytes = request.encode().expect("an input without the key encodes");
    let reread = InvokeRequest::decode(&bytes);
    assert_eq!(
        reread.as_ref().ok(),
        Some(&request),
        "encode wrote bytes decode refused: {reread:?}"
    );
}

/// Round trip: a result at the deepest nesting `serde_json` reads on its own
/// (what an adapter parsing a provider body can hand over) sits one level
/// deeper inside the envelope, past what `decode` reads, so `encode` refuses
/// it; one level shallower encodes and `decode` reads it back.
#[test]
fn encode_never_writes_a_response_decode_refuses_nesting_depth() {
    let nested = |depth: usize| format!("{}{}", "[".repeat(depth), "]".repeat(depth));
    let deepest = (1..1024)
        .take_while(|&depth| serde_json::from_str::<Value>(&nested(depth)).is_ok())
        .last()
        .unwrap();
    let with_result = |depth: usize| -> InvokeResponse {
        let mut response = applied_write();
        response["result"] = serde_json::from_str(&nested(depth)).unwrap();
        serde_json::from_value(response).unwrap()
    };
    assert!(
        with_result(deepest).encode().is_err(),
        "depth {deepest}: encode wrote an envelope deeper than decode reads"
    );
    let built = with_result(deepest - 1);
    let bytes = built.encode().expect("one level shallower encodes");
    let reread = InvokeResponse::decode(&bytes);
    assert_eq!(
        reread.as_ref().ok(),
        Some(&built),
        "depth {}: encode wrote bytes decode refused: {reread:?}",
        deepest - 1
    );
}
