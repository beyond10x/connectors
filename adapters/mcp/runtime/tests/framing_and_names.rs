//! Actual native codec checks derived from the landed projection/framing contracts.
use connectors_mcp::{framing::LineDecoder, names};

#[test]
fn qualified_names_are_exact_injective_and_reversible() {
    for (parts, expected) in [
        (["a", "bc", "d"], "c1_61_6263_64"),
        (["ab", "c", "d"], "c1_6162_63_64"),
        (["z", "bc", "d"], "c1_7a_6263_64"),
        (["é", "a_b", "a.b"], "c1_c3a9_615f62_612e62"),
    ] {
        assert_eq!(names::encode_name(parts).as_deref(), Some(expected));
        assert_eq!(names::decode_name(expected), Some(parts.map(str::to_owned)));
    }
    assert_ne!(
        names::encode_name(["é", "a", "b"]),
        names::encode_name(["e\u{301}", "a", "b"])
    );
}
#[test]
fn malformed_and_over_bound_names_never_gain_a_target() {
    assert!(names::encode_name(["", "a", "b"]).is_none());
    assert!(names::encode_name([&"x".repeat(59), "a", "b"]).is_some());
    assert!(names::encode_name([&"x".repeat(60), "a", "b"]).is_none());
    for bad in [
        "c1_61_62",
        "c1_61_62_63_64",
        "c1__62_63",
        "c1_6_62_63",
        "C1_61_62_63",
        "c1_6A_62_63",
        "c1_ff_62_63",
        "c1_61_62_%63",
    ] {
        assert!(names::decode_name(bad).is_none(), "{bad}");
    }
    let too_long = format!("c1_{}_61_62", "78".repeat(60));
    assert!(names::decode_name(&too_long).is_none());
}
#[test]
fn resources_have_one_exact_revision_and_no_uri_normalization() {
    let uri = "connectors-mcp:///c1_61_62_63?revision=7231";
    assert_eq!(
        names::resource_uri(["a", "b", "c"], "r1").as_deref(),
        Some(uri)
    );
    assert_eq!(
        names::decode_resource_uri(uri),
        Some((["a".into(), "b".into(), "c".into()], "r1".into()))
    );
    assert!(names::resource_uri(["a", "b", "c"], "").is_none());
    for bad in [
        "connectors-mcp://host/c1_61_62_63?revision=7231",
        "CONNECTORS-MCP:///c1_61_62_63?revision=7231",
        "connectors-mcp:///c1_61_62_63?revision=",
        "connectors-mcp:///c1_61_62_63?revision=ff",
        "connectors-mcp:///c1_61_62_63?revision=6A",
        "connectors-mcp:///c1_61_62_63?revision=7231#fragment",
        "connectors-mcp:///c1_61_62_63?revision=7231&revision=7232",
        "connectors-mcp:///c1_61_62_63?revision=7231&extra=1",
        "connectors-mcp:///c1_61_62_63?revision=%72%31",
    ] {
        assert!(names::decode_resource_uri(bad).is_none(), "{bad}");
    }
}
#[test]
fn line_is_not_visible_until_its_newline_and_wire_bytes_are_preserved() {
    let mut decoder = LineDecoder::new(128).unwrap();
    let bytes = b"{\"id\":123456789012345678901234567890,\"text\":\"\xc3\xa9\"}\r\n";
    for byte in &bytes[..bytes.len() - 1] {
        let (used, output) = decoder.feed(std::slice::from_ref(byte));
        assert_eq!(used, 1);
        assert!(output.is_none());
    }
    let (_, frame) = decoder.feed(b"\n");
    assert_eq!(frame.unwrap().unwrap(), bytes[..bytes.len() - 1]);
    assert!(!decoder.finish());
}
#[test]
fn exact_bound_includes_newline_and_oversized_lines_resynchronize() {
    let mut decoder = LineDecoder::new(4).unwrap();
    let (used, frame) = decoder.feed(b"abc\nnext");
    assert_eq!(used, 4);
    assert_eq!(frame.unwrap().unwrap(), b"abc");
    assert!(decoder.feed(b"abcd").1.is_none());
    assert_eq!(decoder.retained_octets(), 0);
    let (used, frame) = decoder.feed(b"discard\nok\n");
    assert_eq!(used, 8);
    assert_eq!(
        frame.unwrap().unwrap_err().kind(),
        std::io::ErrorKind::InvalidData
    );
    assert_eq!(decoder.feed(b"ok\n").1.unwrap().unwrap(), b"ok");
}
#[test]
fn long_unterminated_input_is_bounded_and_eof_never_executes_a_prefix() {
    let mut decoder = LineDecoder::new(16).unwrap();
    for _ in 0..4096 {
        let (used, frame) = decoder.feed(b"xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx");
        assert_eq!(used, 32);
        assert!(frame.is_none());
        assert!(decoder.retained_octets() <= 15);
    }
    assert!(decoder.finish());
    assert_eq!(decoder.feed(b"{}\n").1.unwrap().unwrap(), b"{}");
    assert!(decoder.feed(b"{\"id\":1}").1.is_none());
    assert!(decoder.finish());
    assert_eq!(decoder.retained_octets(), 0);
    assert!(!decoder.finish());
}
#[test]
fn invalid_utf8_is_preserved_for_protocol_parse_error_and_zero_limit_refuses() {
    assert!(LineDecoder::new(0).is_err());
    let mut decoder = LineDecoder::new(8).unwrap();
    assert_eq!(decoder.feed(b"\xff\n").1.unwrap().unwrap(), b"\xff");
    assert_eq!(decoder.feed(b"\n").1.unwrap().unwrap(), b"");
}
