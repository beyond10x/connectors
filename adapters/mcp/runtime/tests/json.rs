use connectors_mcp::json::{Error, decode};
use serde_json::json;

#[test]
fn numbers_do_not_pass_through_binary_floating_point() {
    for (number, encoded) in [
        (
            "184467440737095516161234567890123456789",
            "184467440737095516161234567890123456789",
        ),
        (
            "-184467440737095516161234567890123456789",
            "-184467440737095516161234567890123456789",
        ),
        (
            "1.23456789012345678901234567890123456789",
            "1.23456789012345678901234567890123456789",
        ),
        ("1e400", "1e+400"),
        (
            "0.0000000000000000000000000000000000000001",
            "0.0000000000000000000000000000000000000001",
        ),
    ] {
        let input = format!("{{\"result\":[{number}]}}");
        let value = decode(input.as_bytes(), 2).unwrap();
        assert_eq!(value["result"][0].to_string(), encoded);
    }
}

#[test]
fn every_json_root_shape_and_escaped_keys_survive() {
    for (input, expected) in [
        ("null", json!(null)),
        ("true", json!(true)),
        ("false", json!(false)),
        ("42", json!(42)),
        ("\"value\"", json!("value")),
        ("[]", json!([])),
        ("{}", json!({})),
        (
            r#"{"\u0061":[null,"é",{"n":7}]}"#,
            json!({"a":[null,"é",{"n":7}]}),
        ),
    ] {
        assert_eq!(decode(input.as_bytes(), 8).unwrap(), expected);
    }
}

#[test]
fn duplicate_keys_are_refused_at_any_depth_after_escape_decoding() {
    assert!(decode(br#"{"a":1,"b":2}"#, 8).is_ok());
    for input in [
        r#"{"a":1,"a":2}"#,
        r#"{"a":1,"\u0061":2}"#,
        r#"{"outer":[{"a":1,"a":2}]}"#,
        r#"{"x":{"inner":{"z":null,"z":true}}}"#,
        " \n{\"a\":1,\"a\":2}\t ",
    ] {
        assert_eq!(decode(input.as_bytes(), 8), Err(Error::DuplicateKey));
    }
}

#[test]
fn serde_private_token_spellings_remain_ordinary_object_members() {
    let input = br#"{"$serde_json::private::Number":"1e400","$serde_json::private::RawValue":"{broken}","nested":{"$serde_json::private::Number":17}}"#;
    let value = decode(input, 2).unwrap();
    assert!(value.is_object());
    assert_eq!(
        value["$serde_json::private::Number"].as_str(),
        Some("1e400")
    );
    assert_eq!(
        value["$serde_json::private::RawValue"].as_str(),
        Some("{broken}")
    );
    assert_eq!(value["nested"]["$serde_json::private::Number"], json!(17));
    assert_eq!(value.as_object().unwrap().len(), 3);
}

#[test]
fn malformed_trailing_and_non_utf8_documents_are_refused() {
    assert!(decode(b" \n null \t", 1).is_ok());
    for input in [
        b"".as_slice(),
        b"null false",
        b"[1,]",
        b"01",
        b"NaN",
        b"1e",
        b"{\"a\":}",
        b"\"\xff\"",
        br#""\uD800""#,
    ] {
        assert_eq!(decode(input, 8), Err(Error::Malformed));
    }
}

#[test]
fn depth_limits_are_explicit_and_count_containers() {
    assert_eq!(decode(b"null", 0), Err(Error::InvalidLimit));
    assert_eq!(decode(b"null", 129), Err(Error::InvalidLimit));
    assert!(decode(br#"{"a":1}"#, 1).is_ok());
    assert!(decode(br#"{"a":[]}"#, 2).is_ok());
    assert_eq!(decode(br#"{"a":[]}"#, 1), Err(Error::Depth));
    assert_eq!(decode(b"[[[]]]", 2), Err(Error::Depth));
    assert!(decode(b"[[[]]]", 3).is_ok());
    let deepest = format!("{}null{}", "[".repeat(128), "]".repeat(128));
    assert!(decode(deepest.as_bytes(), 128).is_ok());
    let beyond = format!("[{deepest}]");
    assert_eq!(decode(beyond.as_bytes(), 128), Err(Error::Depth));
}
