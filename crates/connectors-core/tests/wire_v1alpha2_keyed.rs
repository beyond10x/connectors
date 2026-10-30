//! The keyed codec of the first v1alpha2 invoke binding: what a host with
//! standing grants reads and writes (`contracts/service/compatibility.md`
//! § 2.1, Keyed writes under a standing grant;
//! story:served-keyed-writes-under-a-standing-approval). A host without them
//! reads [`InvokeRequest`] and writes under the unchanged first-binding rules,
//! which `wire_v1alpha2.rs` and `wire_v1alpha2_adversary.rs` hold.
use connectors_core::v1alpha2::{
    ErrorCode, InvokeRequest, InvokeResponse, KEY_LIMIT, KeyedInvokeRequest,
};
use serde_json::{Value, json};

fn request(key: Value) -> Vec<u8> {
    let mut request = json!({
        "version": "v1alpha2",
        "request_id": "write-1",
        "operation": "item.write",
        "revision": "rev-1",
        "input": {}
    });
    request["idempotency_key"] = key;
    serde_json::to_vec(&request).unwrap()
}

fn replayed_write() -> Value {
    json!({
        "version": "v1alpha2",
        "request_id": "write-7",
        "status": "success",
        "result": {},
        "audit_ref": "aud-10",
        "audit_status": "complete",
        "mutation": {
            "classification": "applied",
            "attempt": {"instance": "fixture", "id": "00000000-0000-4000-8000-00000000000A"},
            "original_request_id": "write-original",
            "replayed": true,
            "cause": null
        }
    })
}

fn refusal(code: &str) -> Value {
    json!({
        "version": "v1alpha2",
        "request_id": "write-1",
        "status": "error",
        "error": {"code": code, "message": "refused"},
        "audit_ref": null,
        "audit_status": "unavailable"
    })
}

fn bytes(value: &Value) -> Vec<u8> {
    serde_json::to_vec(value).unwrap()
}

/// A keyed host reads the key, and its encoding round-trips; the unchanged
/// reader still refuses the member `unsupported` with no request ID.
#[test]
fn a_keyed_host_reads_the_key_and_the_unchanged_reader_refuses_it() {
    let decoded = KeyedInvokeRequest::decode(&request(json!("k1"))).unwrap();
    assert_eq!(decoded.idempotency_key.as_deref(), Some("k1"));
    assert_eq!(
        KeyedInvokeRequest::decode(&decoded.encode().unwrap()).unwrap(),
        decoded
    );
    let refused = InvokeRequest::decode(&request(json!("k1"))).unwrap_err();
    assert_eq!(refused.error.code, ErrorCode::Unsupported);
    assert_eq!(refused.request_id, None);
}

/// A request without a key reads the same through both readers.
#[test]
fn a_request_without_a_key_reads_the_same_through_both_readers() {
    let body = bytes(&json!({
        "version": "v1alpha2",
        "request_id": "read-1",
        "operation": "item.read",
        "revision": "rev-1",
        "input": {"id": 1}
    }));
    let keyed = KeyedInvokeRequest::decode(&body).unwrap();
    assert_eq!(keyed.idempotency_key, None);
    assert_eq!(
        keyed,
        KeyedInvokeRequest::from(InvokeRequest::decode(&body).unwrap())
    );
}

/// The key's bounds: an empty or over-long key is `invalid_input` with the
/// request ID; a null key is no valid envelope, so it echoes none.
#[test]
fn a_key_outside_its_bounds_is_invalid_input() {
    for key in [json!(""), json!("k".repeat(KEY_LIMIT + 1))] {
        let refused = KeyedInvokeRequest::decode(&request(key.clone())).unwrap_err();
        assert_eq!(refused.error.code, ErrorCode::InvalidInput, "{key}");
        assert_eq!(refused.request_id.as_deref(), Some("write-1"), "{key}");
    }
    assert!(KeyedInvokeRequest::decode(&request(json!("k".repeat(KEY_LIMIT)))).is_ok());
    let refused = KeyedInvokeRequest::decode(&request(Value::Null)).unwrap_err();
    assert_eq!(refused.error.code, ErrorCode::InvalidInput);
    assert_eq!(refused.request_id, None);
}

/// `connection`, `approval` and `executor` stay `unsupported` on a keyed
/// host; any other unknown member is `invalid_input`.
#[test]
fn a_keyed_host_still_refuses_the_other_section_4_members() {
    for member in ["connection", "approval", "executor"] {
        let mut body: Value = serde_json::from_slice(&request(json!("k1"))).unwrap();
        body[member] = json!("x");
        let refused = KeyedInvokeRequest::decode(&bytes(&body)).unwrap_err();
        assert_eq!(refused.error.code, ErrorCode::Unsupported, "{member}");
        assert_eq!(refused.request_id, None, "{member}");
    }
    let mut body: Value = serde_json::from_slice(&request(json!("k1"))).unwrap();
    body["other"] = json!(1);
    let refused = KeyedInvokeRequest::decode(&bytes(&body)).unwrap_err();
    assert_eq!(refused.error.code, ErrorCode::InvalidInput);
}

/// A keyed replay names the original attempt and its request: the keyed
/// reader admits it, the unchanged reader still refuses it, and a replay
/// naming no attempt is refused by both.
#[test]
fn a_replay_is_read_only_under_the_keyed_rules() {
    let replay = replayed_write();
    let decoded = InvokeResponse::decode_keyed(&bytes(&replay)).unwrap();
    assert_eq!(
        InvokeResponse::decode_keyed(&decoded.encode_keyed().unwrap()).unwrap(),
        decoded
    );
    assert!(InvokeResponse::decode(&bytes(&replay)).is_err());
    assert!(decoded.encode().is_err());
    let mut orphan = replay;
    orphan["mutation"]["attempt"] = Value::Null;
    orphan["mutation"]["original_request_id"] = Value::Null;
    assert!(InvokeResponse::decode_keyed(&bytes(&orphan)).is_err());
}

/// A response that is not a replay still names this request as the
/// original under the keyed rules.
#[test]
fn a_first_answer_names_its_own_request_under_the_keyed_rules() {
    let mut response = replayed_write();
    response["mutation"]["replayed"] = json!(false);
    assert!(InvokeResponse::decode_keyed(&bytes(&response)).is_err());
}

/// The keyed host's codes: `approval_required` and `idempotency_conflict`
/// are read under the keyed rules only; codes it never emits are refused by
/// both readers.
#[test]
fn the_keyed_codes_are_read_only_under_the_keyed_rules() {
    for code in [
        "approval_required",
        "idempotency_conflict",
        "outcome_unknown",
    ] {
        assert!(
            InvokeResponse::decode_keyed(&bytes(&refusal(code))).is_ok(),
            "{code}"
        );
    }
    for code in ["approval_required", "idempotency_conflict"] {
        assert!(
            InvokeResponse::decode(&bytes(&refusal(code))).is_err(),
            "{code}"
        );
    }
    for code in ["approval_refused", "not_granted", "revoked"] {
        assert!(
            InvokeResponse::decode_keyed(&bytes(&refusal(code))).is_err(),
            "{code}"
        );
    }
}
