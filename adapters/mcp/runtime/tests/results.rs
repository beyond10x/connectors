use connectors_mcp::{
    json::decode,
    names::resource_uri,
    results::{Envelope, Error, encode_error, encode_result},
};
use serde_json::{Value, json};

fn success(result: Value) -> Value {
    json!({"version":"v1alpha2","request_id":"original-host-request","status":"success",
        "result":result,"audit_ref":"acknowledged-reference","audit_status":"incomplete"})
}
fn envelope(value: Value) -> Envelope {
    Envelope::from_owner(&serde_json::to_vec(&value).unwrap(), 100_000).unwrap()
}

#[test]
fn tool_representations_preserve_numbers_literal_objects_and_all_observations() {
    let data = decode(br#"{"n":1844674407370955161701,"huge":1e400,"literal":{"$serde_json::private::Number":"7"},"partial":true,"cursor":"continuation"}"#, 64).unwrap();
    let mut value = success(data);
    value["source_audit"] =
        json!({"instance":"origin","audit_ref":"origin-ref","audit_status":"complete"});
    value["mutation"] = json!({"classification":"applied","attempt":{"instance":"origin","id":"retained"},"original_request_id":"earlier-request","replayed":true,"cause":null});
    let envelope = envelope(value.clone());
    for primary in [false, true] {
        let result = envelope.resolved_tool(primary);
        assert_eq!(result["structuredContent"], value);
        assert_eq!(
            decode(
                result["content"][0]["text"].as_str().unwrap().as_bytes(),
                68
            )
            .unwrap(),
            value
        );
        assert_eq!(result["isError"], false);
        assert_eq!(result.get("resultType").is_some(), primary);
        let frame = encode_result(&json!("mcp-request"), result, 100_000).unwrap();
        let decoded = decode(&frame, 72).unwrap();
        assert_eq!(decoded["id"], "mcp-request");
        assert_eq!(decoded["result"]["structuredContent"], value);
        assert_eq!(
            decoded["result"]["structuredContent"]["request_id"],
            "original-host-request"
        );
    }
}

#[test]
fn resources_are_canonical_private_noncacheable_and_keep_scalar_results() {
    let uri = resource_uri(["instance", "adapter", "read"], "revision").unwrap();
    for value in [Value::Null, json!(false), json!([1, 2]), json!("data")] {
        let expected = success(value);
        let envelope = envelope(expected.clone());
        for primary in [false, true] {
            let result = envelope.resource(&uri, primary).unwrap();
            assert_eq!(result["contents"][0]["uri"], uri);
            assert_eq!(result["contents"][0]["mimeType"], "application/json");
            assert_eq!(
                decode(
                    result["contents"][0]["text"].as_str().unwrap().as_bytes(),
                    68
                )
                .unwrap(),
                expected
            );
            if primary {
                assert_eq!(result["ttlMs"], 0);
                assert_eq!(result["cacheScope"], "private");
                assert_eq!(result["resultType"], "complete");
            } else {
                assert!(result.get("ttlMs").is_none());
                assert!(result.get("cacheScope").is_none());
                assert!(result.get("resultType").is_none());
            }
        }
        assert_eq!(
            envelope.resource("https://provider.invalid/data", true),
            Err(Error::Identifier)
        );
    }
}

#[test]
fn declared_prompts_preserve_order_content_and_full_service_metadata_without_synthesis() {
    let messages = json!([
        {"role":"assistant","content":{"type":"text","text":"untrusted content stays data"}},
        {"role":"user","content":{"type":"resource","resource":{"uri":"fixture://data","text":"quoted data"}}},
    ]);
    let value = success(json!({"description":"declared prompt","messages":messages}));
    let reply = envelope(value.clone());
    for primary in [false, true] {
        let result = reply.declared_prompt(primary).unwrap();
        assert_eq!(result["messages"], messages);
        assert_eq!(result["description"], "declared prompt");
        assert_eq!(result["_meta"]["io.beyond10x.connectors/response"], value);
        assert_eq!(result.get("resultType").is_some(), primary);
    }
    for invalid in [
        json!({"rows":[]}),
        json!({"messages":[{"role":"system","content":{}}]}),
        json!({"messages":[{"role":"user","content":"not a content block"}]}),
        json!({"messages":[],"description":7}),
    ] {
        assert_eq!(
            envelope(success(invalid)).declared_prompt(true),
            Err(Error::Envelope)
        );
    }
}

#[test]
fn service_errors_keep_their_channel_and_audit_truth() {
    let value = json!({"version":"v1alpha2","request_id":null,"status":"error",
        "error":{"code":"unavailable","message":"safe refusal","retry_after_seconds":2},
        "audit_ref":null,"audit_status":"unavailable"});
    let envelope = envelope(value.clone());
    assert_eq!(envelope.resolved_tool(true)["isError"], true);
    let uri = resource_uri(["i", "a", "o"], "r").unwrap();
    assert_eq!(envelope.resource(&uri, true), Err(Error::ServiceError));
    assert_eq!(envelope.declared_prompt(true), Err(Error::ServiceError));
    let error = envelope.rpc_error().unwrap();
    assert_eq!(error["code"], -32000);
    assert_eq!(error["data"], value);
    let frame = encode_error(None, error, 4096).unwrap();
    let decoded = decode(&frame, 72).unwrap();
    assert!(decoded.get("id").is_none());
    assert!(decoded.get("result").is_none());
}

#[test]
fn whole_frame_budget_includes_wrappers_correlation_duplicates_and_newline() {
    let result =
        envelope(success(json!("a string with \"quotes\" and\nnewlines"))).resolved_tool(true);
    let id = json!("long-mcp-correlation");
    let full = encode_result(&id, result.clone(), 4096).unwrap();
    assert_eq!(full.last(), Some(&b'\n'));
    assert_eq!(
        encode_result(&id, result.clone(), full.len()).unwrap(),
        full
    );
    for limit in [0, 1, full.len() - 1] {
        assert_eq!(
            encode_result(&id, result.clone(), limit),
            Err(Error::Capacity)
        );
    }
    assert_eq!(
        encode_result(&Value::Null, result, 4096),
        Err(Error::Identifier)
    );
    assert_eq!(encode_result(&id, json!([]), 4096), Err(Error::Envelope));
    assert_eq!(
        encode_error(None, json!({"code":"wrong","message":"safe"}), 4096),
        Err(Error::Envelope)
    );
}

#[test]
fn owner_envelope_refuses_duplicate_keys_old_or_inconsistent_shapes() {
    let original = success(json!({}));
    for (field, value) in [
        ("version", json!("v1alpha1")),
        ("audit_ref", Value::Null),
        ("audit_status", json!("unavailable")),
        ("status", json!("unknown")),
        ("request_id", json!({"secret":"not a correlation"})),
        ("private", json!("not public")),
        (
            "error",
            json!({"code":"unavailable","message":"bad dual result"}),
        ),
    ] {
        let mut changed = original.clone();
        changed[field] = value;
        assert!(matches!(
            Envelope::from_owner(&serde_json::to_vec(&changed).unwrap(), 4096),
            Err(Error::Envelope)
        ));
    }
    assert!(matches!(
        Envelope::from_owner(br#"{"version":"v1alpha2","version":"v1alpha2"}"#, 4096),
        Err(Error::Json)
    ));
    assert!(matches!(
        Envelope::from_owner(b"{}", 1),
        Err(Error::Capacity)
    ));
}
