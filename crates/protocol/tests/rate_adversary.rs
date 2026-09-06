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
