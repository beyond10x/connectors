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
        let expected = serde_json::to_value(&outcome).unwrap();
        let response = Response {
            version: WIRE_VERSION.into(),
            request_id: "request-1".into(),
            outcome,
        };
        let bytes = serde_json::to_vec(&response).unwrap();
        let read: Response = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(read.request_id, "request-1");
        assert_eq!(read.version, WIRE_VERSION);
        assert_eq!(serde_json::to_value(read.outcome).unwrap(), expected);
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

// Entity Runtime forces serde_json `arbitrary_precision` on in every host
// build. These cases hold in both builds: `cargo test -p connectors-core`
// (feature off) and `cargo test -p connectors-core --features
// serde_json/arbitrary_precision` (feature on).

#[test]
fn the_strict_reader_admits_typed_floats() {
    #[derive(serde::Deserialize)]
    struct Typed {
        ratio: f64,
        wide: f64,
    }
    let typed: Typed =
        connectors_core::read_json(br#"{"ratio":1.5,"wide":18446744073709551616}"#).unwrap();
    assert_eq!(typed.ratio, 1.5);
    assert_eq!(typed.wide, 18446744073709551616.0);
    assert!(connectors_core::read_json::<serde_json::Value>(br#"{"x":1e400}"#).is_err());
}

#[test]
fn canonical_bytes_do_not_depend_on_number_spelling_or_build() {
    let spelled =
        br#"{"a":1.50,"b":1e2,"c":18446744073709551616,"d":-0.0,"e":7,"f":-7,"g":2.5E-3}"#;
    let written =
        br#"{"a":1.5,"b":100.0,"c":1.8446744073709552e+19,"d":-0.0,"e":7,"f":-7,"g":0.0025}"#;
    let strict: serde_json::Value = connectors_core::read_json(spelled).unwrap();
    let plain: serde_json::Value = serde_json::from_slice(spelled).unwrap();
    let computed =
        json!({"a":1.5,"b":100.0,"c":18446744073709551616.0_f64,"d":-0.0,"e":7,"f":-7,"g":0.0025});
    for value in [&strict, &plain, &computed] {
        assert_eq!(
            String::from_utf8(canonical(value)).unwrap(),
            std::str::from_utf8(written).unwrap()
        );
        assert_eq!(
            connectors_core::digest(value),
            connectors_core::digest(&computed)
        );
    }
    assert_eq!(strict, computed);
}

#[test]
fn negative_zero_integer_reads_as_the_float_the_default_build_reads() {
    let read: serde_json::Value = connectors_core::read_json(b"[-0,0,-0.0]").unwrap();
    assert_eq!(canonical(&read), b"[-0.0,0,-0.0]");
}

/// serde_json's `Value` reader reinterprets an object whose first key is one
/// of its private tokens, so a document spelling one is refused in every
/// build rather than silently read as a number or raw JSON downstream.
#[test]
fn document_keys_spelled_like_serde_json_private_tokens_are_refused() {
    for bytes in [
        &br#"{"$serde_json::private::Number":"12"}"#[..],
        br#"{"$serde_json::private::Number":"1\u0032"}"#,
        br#"{"$serde_json::private::Number":"not a number"}"#,
        br#"{"$serde_json::private::Numbe\u0072":"12"}"#,
        br#"{"a":1,"$serde_json::private::Number":"12"}"#,
        br#"{"outer":[{"$serde_json::private::Number":"1.5"}]}"#,
        br#"{"$serde_json::private::RawValue":"{\"a\":1}"}"#,
    ] {
        assert!(
            connectors_core::read_json::<serde_json::Value>(bytes).is_err(),
            "{}",
            String::from_utf8_lossy(bytes)
        );
    }
    // Ordinary keys and values that merely look numeric are unchanged.
    let read: serde_json::Value =
        connectors_core::read_json(br#"{"$serde_json":"12","n":"1.5"}"#).unwrap();
    assert_eq!(canonical(&read), br#"{"$serde_json":"12","n":"1.5"}"#);
}

#[test]
fn contradictory_response_payloads_are_refused() {
    assert!(serde_json::from_value::<Response>(json!({"version":WIRE_VERSION,"request_id":"one","status":"success","result":{},"error":{"code":"forbidden","message":"denied"}})).is_err());
    assert!(serde_json::from_value::<Response>(json!({"version":WIRE_VERSION,"request_id":"one","status":"success","result":{},"extra":true})).is_err());
}
