use connectors_core::{Error, ErrorCode, Outcome, Response, WIRE_VERSION, canonical};
use serde_json::json;

#[test]
fn response_roundtrips_success_and_error() {
    for outcome in [
        Outcome::Success {
            result: json!({"answer":42}),
        },
        Outcome::Error {
            error: Error::new(ErrorCode::Forbidden, "denied"),
        },
    ] {
        let response = Response {
            version: WIRE_VERSION.into(),
            request_id: "request-1".into(),
            outcome,
        };
        let bytes = serde_json::to_vec(&response).unwrap();
        let read: Response = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(read.request_id, "request-1");
    }
}

#[test]
fn canonicalization_is_recursive_and_order_independent() {
    assert_eq!(
        canonical(&json!({"b": [{"z":1,"a":2}],"a":1})),
        br#"{"a":1,"b":[{"a":2,"z":1}]}"#
    );
}

#[test]
fn duplicate_keys_are_refused_at_every_depth() {
    assert!(
        connectors_core::read_json::<serde_json::Value>(
            br#"{"input":{"scope":"one","scope":"two"}}"#
        )
        .is_err()
    );
    assert!(
        connectors_core::read_json::<serde_json::Value>(br#"{"input":[{"a":1,"a":2}]}"#).is_err()
    );
}

#[test]
fn contradictory_response_payloads_are_refused() {
    assert!(serde_json::from_value::<Response>(json!({"version":WIRE_VERSION,"request_id":"one","status":"success","result":{},"error":{"code":"forbidden","message":"denied"}})).is_err());
    assert!(serde_json::from_value::<Response>(json!({"version":WIRE_VERSION,"request_id":"one","status":"success","result":{},"extra":true})).is_err());
}
