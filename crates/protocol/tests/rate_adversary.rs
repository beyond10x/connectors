use protocol::operation::{legacy, wire};
use serde_json::{json, Value};

#[test]
fn rate_adversary_published_schema_matches_source_url_reader() {
    let vectors: Value = serde_json::from_str(include_str!(
        "../../../contracts/connector-operation/v0alpha2/vectors.json"
    ))
    .unwrap();
    let mut frame = vectors["cases"]
        .as_array()
        .unwrap()
        .iter()
        .find(|case| case["valid"] == true && case["frame"]["response"]["result"] == "describe")
        .unwrap()["frame"]
        .clone();
    frame["response"]["value"]["rate_advice"] = json!({"alternatives":[{
        "declaration":{"applies_when":"the documented category", "source_url":"https://docs.example.test/rate",
        "rate":{"requests":3,"per_seconds":1,"basis":"minimum_allowance"}},"suggested_interval_ms":334}]});
    let schema: Value = serde_json::from_str(include_str!(
        "../../../contracts/connector-operation/v0alpha2/connector-operation.schema.json"
    ))
    .unwrap();
    let validator = jsonschema::options()
        .with_draft(jsonschema::Draft::Draft202012)
        .should_validate_formats(true)
        .build(&schema)
        .unwrap();
    let mut mismatches = Vec::new();
    for source in [
        "https://docs.example.test/rate",
        "https://docs.example.test:443/rate",
        "HTTPS://docs.example.test/rate",
        "https://docs.example.test:65536/rate",
        "https://docs.example.test/a b",
        "https://user:pass@docs.example.test/rate",
        "http://docs.example.test/rate",
        "https://docs.example.test/rate#fragment",
    ] {
        frame["response"]["value"]["rate_advice"]["alternatives"][0]["declaration"]["source_url"] =
            json!(source);
        let rust = serde_json::from_value::<wire::ResponseEnvelope>(frame.clone())
            .is_ok_and(|response| response.validate().is_ok());
        let schema = validator.is_valid(&frame);
        if rust != schema {
            mismatches.push(format!("{source:?}: Rust={rust}, schema={schema}"));
        }
    }
    for delay in [None, Some(0), Some(u64::MAX)] {
        let response = wire::ResponseEnvelope::failure(
            "correlation",
            wire::OperationError::rate_limited("refused", delay),
        );
        response.validate().unwrap();
        let old: legacy::ResponseEnvelope =
            serde_json::from_slice(&wire::Version::V0Alpha1.encode_response(response).unwrap())
                .unwrap();
        old.validate().unwrap();
        assert_eq!(
            old.error.unwrap().code,
            legacy::OperationErrorCode::Unavailable
        );
    }
    assert!(
        mismatches.is_empty(),
        "source-URL validity is not one of the two documented schema limitations:\n{}",
        mismatches.join("\n")
    );
}

#[test]
fn rate_final_uri_composition_and_downgrade_validate_complete_frames() {
    let vectors: Value = serde_json::from_str(include_str!(
        "../../../contracts/connector-operation/v0alpha2/vectors.json"
    ))
    .unwrap();
    let original = vectors["cases"]
        .as_array()
        .unwrap()
        .iter()
        .find(|case| case["valid"] == true && case["frame"]["response"]["result"] == "describe")
        .unwrap()["frame"]
        .clone();
    let schema: Value = serde_json::from_str(include_str!(
        "../../../contracts/connector-operation/v0alpha2/connector-operation.schema.json"
    ))
    .unwrap();
    let validator = jsonschema::options()
        .with_draft(jsonschema::Draft::Draft202012)
        .should_validate_formats(true)
        .build(&schema)
        .unwrap();
    let mut sources = vec![
        (
            "https://%41.example.test:00065535/a%2fb?x=%23%40".to_owned(),
            true,
        ),
        ("https://[v1.a:b]:00000/a?b/c?d".to_owned(), true),
        ("https://[2001:db8::1]:/?x=a@b".to_owned(), true),
        ("https://docs.example.test:?x=a/b?c".to_owned(), true),
        ("https://docs.example.test/%00%1F%7f?x=%ff".to_owned(), true),
        ("https://docs.example.test:00065536/a%2fb".to_owned(), false),
        ("https://[v1.a:b]:65536/".to_owned(), false),
        ("https://docs.example.test/%2g".to_owned(), false),
        ("https://docs.example.test:1:2/".to_owned(), false),
        ("https://user%40name@docs.example.test/".to_owned(), false),
        ("https://docs.example.test/a%23b#".to_owned(), false),
    ];
    let prefix = "https://docs.example.test:";
    sources.push((
        format!("{prefix}{}65535/", "0".repeat(2048 - prefix.len() - 6)),
        true,
    ));
    sources.push((
        format!("{prefix}{}65535/", "0".repeat(2049 - prefix.len() - 6)),
        false,
    ));
    for (source, expected) in sources {
        let mut frame = original.clone();
        frame["response"]["value"]["rate_advice"] = json!({"alternatives":[{"declaration":{"applies_when":"all documented contexts","source_url":source,"rate":{"requests":3,"per_seconds":1,"basis":"minimum_allowance"}},"suggested_interval_ms":334}]});
        let response: wire::ResponseEnvelope = serde_json::from_value(frame.clone()).unwrap();
        assert_eq!(response.validate().is_ok(), expected, "reader {source:?}");
        assert_eq!(validator.is_valid(&frame), expected, "schema {source:?}");
        for version in [wire::Version::V0Alpha1, wire::Version::V0Alpha2] {
            let encoded = version.encode_response(response.clone());
            assert_eq!(
                encoded.is_ok(),
                expected,
                "downgrade must not launder invalid advice: {source:?}"
            );
            if let Ok(bytes) = encoded {
                let value: Value = serde_json::from_slice(&bytes).unwrap();
                let mut wanted = frame.clone();
                if version == wire::Version::V0Alpha1 {
                    wanted["protocol"] = json!(legacy::CONTRACT);
                    wanted["response"]["value"]
                        .as_object_mut()
                        .unwrap()
                        .remove("rate_advice");
                }
                assert_eq!(value, wanted, "only declared v1 advice loss is permitted");
            }
        }
    }
}
