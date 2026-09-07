use std::fs;
use std::path::{Path, PathBuf};

use serde::Deserialize;
use serde_json::Value;
use sha2::{Digest as _, Sha256};

#[derive(Deserialize)]
struct Bundle {
    files: Vec<BundleFile>,
}

#[derive(Deserialize)]
struct BundleFile {
    path: String,
    bytes: usize,
    sha256: String,
}

#[derive(Deserialize)]
struct OperationVectors {
    contract: String,
    cases: Vec<OperationVector>,
}

#[derive(Deserialize)]
struct OperationVector {
    name: String,
    valid: bool,
    frame: Value,
}

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn check(relative: &str) {
    let root = root();
    let bundle: Bundle =
        serde_json::from_slice(&fs::read(root.join(relative)).expect("bundle is readable"))
            .expect("bundle parses");
    for entry in bundle.files {
        let bytes = fs::read(root.join(&entry.path))
            .unwrap_or_else(|error| panic!("{} is readable: {error}", entry.path));
        assert_eq!(bytes.len(), entry.bytes, "{} byte length", entry.path);
        assert_eq!(
            format!("{:x}", Sha256::digest(&bytes)),
            entry.sha256,
            "{} digest",
            entry.path
        );
    }
}

#[test]
fn owner_contract_bundle_is_immutable() {
    check("contracts/voice-session/v0alpha1/bundle.json");
}

#[test]
fn rtvbp_binding_bundle_is_immutable() {
    check("fixtures/rtvbp-voice-binding/v1/bundle.json");
}

#[test]
fn connector_operation_bundle_is_immutable() {
    check("contracts/connector-operation/v0alpha1/bundle.json");
}

#[test]
fn connector_connection_bundle_is_immutable() {
    check("contracts/connector-connection/v0alpha1/bundle.json");
}

#[test]
fn connector_event_bundle_is_immutable() {
    check("contracts/connector-event/v0alpha1/bundle.json");
}

#[test]
fn connector_datasource_bundle_is_immutable() {
    check("contracts/connector-datasource/v0alpha1/bundle.json");
}

#[test]
fn connector_catalog_bundle_is_immutable() {
    check("contracts/connector-catalog/v0alpha1/bundle.json");
}

#[test]
fn connector_operation_vectors_match_the_strict_reader() {
    let path = root().join("contracts/connector-operation/v0alpha1/vectors.json");
    let vectors: OperationVectors =
        serde_json::from_slice(&fs::read(path).expect("vectors are readable"))
            .expect("vectors parse");
    assert_eq!(vectors.contract, protocol::operation::legacy::CONTRACT);
    for vector in vectors.cases {
        let result =
            serde_json::from_value::<protocol::operation::legacy::RequestEnvelope>(vector.frame)
                .map_err(|error| error.to_string())
                .and_then(|request| request.validate().map_err(|error| error.to_string()));
        assert_eq!(
            result.is_ok(),
            vector.valid,
            "operation vector `{}` disagrees with the reader: {result:?}",
            vector.name
        );
    }
}

#[test]
fn connector_connection_vectors_match_the_strict_reader() {
    let path = root().join("contracts/connector-connection/v0alpha1/vectors.json");
    let vectors: OperationVectors =
        serde_json::from_slice(&fs::read(path).expect("vectors are readable"))
            .expect("vectors parse");
    assert_eq!(vectors.contract, protocol::connection::CONTRACT);
    for vector in vectors.cases {
        let result = serde_json::from_value::<protocol::connection::RequestEnvelope>(vector.frame)
            .map_err(|error| error.to_string())
            .and_then(|request| request.validate().map_err(|error| error.to_string()));
        assert_eq!(
            result.is_ok(),
            vector.valid,
            "connection vector `{}` disagrees with the reader: {result:?}",
            vector.name
        );
    }
}

#[test]
fn connector_catalog_vectors_match_the_strict_reader() {
    let path = root().join("contracts/connector-catalog/v0alpha1/vectors.json");
    let vectors: OperationVectors =
        serde_json::from_slice(&fs::read(path).expect("vectors are readable"))
            .expect("vectors parse");
    assert_eq!(vectors.contract, protocol::catalog::CONTRACT);
    for vector in vectors.cases {
        let result = serde_json::from_value::<protocol::catalog::RequestEnvelope>(vector.frame)
            .map_err(|error| error.to_string())
            .and_then(|request| request.validate().map_err(|error| format!("{error:?}")));
        assert_eq!(
            result.is_ok(),
            vector.valid,
            "catalog vector `{}` disagrees with the reader: {result:?}",
            vector.name
        );
    }
}

#[test]
fn kubernetes_service_route_round_trips_through_the_connection_response() {
    let response: protocol::connection::ResponseEnvelope =
        serde_json::from_value(serde_json::json!({
            "protocol": "b10x.connector-connection.v0alpha1",
            "request_id": "request-materialize-kubernetes-1",
            "status": "ok",
            "response": {
                "result": "materialize",
                "value": {
                    "connection_ref": "connection:prometheus:opaque",
                    "integration_ref": "prometheus",
                    "label": "monitoring/prometheus (prometheus)",
                    "state": "callable",
                    "initiation": ["b10x"],
                    "route": {
                        "kind": "via_connection",
                        "parent_connection_ref": "connection:kubernetes:opaque",
                        "route_adapter": "kubernetes_service_proxy_v1"
                    },
                    "channels": []
                }
            }
        }))
        .unwrap();
    response.validate().unwrap();
    let Some(protocol::connection::ConnectionResult::Materialize(description)) = response.response
    else {
        panic!("materialize response required");
    };
    assert!(matches!(
        description.summary.route,
        protocol::connection::ConnectionRoute::ViaConnection {
            route_adapter: protocol::connection::RouteAdapter::KubernetesServiceProxyV1,
            ..
        }
    ));
}

#[test]
fn connector_event_vectors_match_the_strict_reader() {
    let path = root().join("contracts/connector-event/v0alpha1/vectors.json");
    let vectors: OperationVectors =
        serde_json::from_slice(&fs::read(path).expect("vectors are readable"))
            .expect("vectors parse");
    assert_eq!(vectors.contract, protocol::event::CONTRACT);
    for vector in vectors.cases {
        let result = serde_json::from_value::<protocol::event::RequestEnvelope>(vector.frame)
            .map_err(|error| error.to_string())
            .and_then(|request| request.validate().map_err(|error| error.to_string()));
        assert_eq!(
            result.is_ok(),
            vector.valid,
            "event vector `{}` disagrees with the reader: {result:?}",
            vector.name
        );
    }
}

#[test]
fn connector_datasource_vectors_match_the_strict_reader() {
    let path = root().join("contracts/connector-datasource/v0alpha1/vectors.json");
    let vectors: OperationVectors =
        serde_json::from_slice(&fs::read(path).expect("vectors are readable"))
            .expect("vectors parse");
    assert_eq!(vectors.contract, protocol::datasource::CONTRACT);
    for vector in vectors.cases {
        let result = serde_json::from_value::<protocol::datasource::RequestEnvelope>(vector.frame)
            .map_err(|error| error.to_string())
            .and_then(|request| request.validate().map_err(|error| error.to_string()));
        assert_eq!(
            result.is_ok(),
            vector.valid,
            "datasource vector `{}` disagrees with the reader: {result:?}",
            vector.name
        );
    }
}

#[test]
fn connector_operation_v2_bundle_is_immutable() {
    check("contracts/connector-operation/v0alpha2/bundle.json");
}

#[test]
fn operation_v2_only_rate_limited_carries_retry_delay() {
    use protocol::operation::wire as v2;
    for (code, delay, valid) in [
        ("rate_limited", serde_json::json!(30), true),
        ("rate_limited", serde_json::json!(0), true),
        ("rate_limited", serde_json::json!(u64::MAX), true),
        ("unavailable", serde_json::json!(30), false),
        ("outcome_unknown", serde_json::json!(30), false),
        ("rate_limited", serde_json::json!(-1), false),
        ("rate_limited", serde_json::json!(30.5), false),
        ("rate_limited", serde_json::json!("30"), false),
    ] {
        let frame = serde_json::json!({
            "protocol": v2::CONTRACT, "request_id": "request-1", "status": "error",
            "error": {"code": code, "message": "Provider refused the request", "retriable": true, "retry_after_seconds": delay}
        });
        let result = serde_json::from_value::<v2::ResponseEnvelope>(frame)
            .map_err(|error| error.to_string())
            .and_then(|frame| frame.validate().map_err(|error| error.to_string()));
        assert_eq!(result.is_ok(), valid, "{code} {delay}: {result:?}");
    }
}

#[test]
fn operation_v2_projects_throttling_to_v1_without_optional_extensions() {
    use protocol::operation::{legacy, wire as v2};
    let frame = v2::ResponseEnvelope::failure(
        "request-1",
        v2::OperationError::rate_limited("Provider refused the request", Some(30)),
    );
    let bytes = v2::Version::V0Alpha1.encode_response(frame).unwrap();
    let json: Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(json["protocol"], legacy::CONTRACT);
    assert_eq!(json["error"]["code"], "unavailable");
    assert_eq!(json["error"]["retriable"], true);
    assert!(json["error"].get("retry_after_seconds").is_none());
    serde_json::from_slice::<legacy::ResponseEnvelope>(&bytes)
        .unwrap()
        .validate()
        .unwrap();
    assert!(
        serde_json::from_slice::<protocol::operation::ResponseEnvelope>(&bytes)
            .unwrap()
            .validate()
            .is_err()
    );
}

#[test]
fn operation_v2_advice_preserves_all_categories_and_checks_the_interval() {
    use protocol::operation::wire as v2;
    let mut advice = v2::OperationRateAdvice {
        fixed: None,
        alternatives: vec![
            v2::ConditionalRateAdvice::new(v2::ConditionalRateLimit {
                applies_when: "Marketplace or internal applications with cursor pagination".into(),
                rate: Some(v2::PublishedRate {
                    requests: 50,
                    per_seconds: 60,
                    basis: v2::RateLimitBasis::MinimumAllowance,
                }),
                source_url: "https://docs.slack.dev/reference/methods/conversations.history/"
                    .into(),
            }),
            v2::ConditionalRateAdvice::new(v2::ConditionalRateLimit {
                applies_when:
                    "New commercial apps or installations outside Marketplace from 2025-05-29"
                        .into(),
                rate: Some(v2::PublishedRate {
                    requests: 1,
                    per_seconds: 60,
                    basis: v2::RateLimitBasis::Ceiling,
                }),
                source_url: "https://docs.slack.dev/reference/methods/conversations.history/"
                    .into(),
            }),
            v2::ConditionalRateAdvice::new(v2::ConditionalRateLimit {
                applies_when: "Existing external installations exempt from the new limit".into(),
                rate: None,
                source_url: "https://docs.slack.dev/reference/methods/conversations.history/"
                    .into(),
            }),
        ],
    };
    advice.validate().unwrap();
    assert_eq!(advice.alternatives[0].suggested_interval_ms, Some(1200));
    assert_eq!(advice.alternatives[1].suggested_interval_ms, Some(60000));
    assert_eq!(advice.alternatives[2].suggested_interval_ms, None);
    advice.alternatives[2].suggested_interval_ms = Some(0);
    assert!(advice.validate().is_err());
    advice.alternatives[2].suggested_interval_ms = None;
    advice.alternatives[0].suggested_interval_ms = Some(1);
    assert!(advice.validate().is_err());
}

#[test]
fn operation_version_reader_rejects_unknown_versions_and_preserves_authority_fields() {
    use protocol::operation::{legacy, wire as v2};
    let vectors: OperationVectors = serde_json::from_slice(
        &fs::read(root().join("contracts/connector-operation/v0alpha1/vectors.json")).unwrap(),
    )
    .unwrap();
    for vector in vectors.cases.into_iter().filter(|vector| vector.valid) {
        let bytes = serde_json::to_vec(&vector.frame).unwrap();
        let (version, request) = v2::decode_request(&bytes).unwrap();
        assert_eq!(version, v2::Version::V0Alpha1);
        assert_eq!(
            serde_json::to_value(request.into_legacy()).unwrap(),
            vector.frame
        );
        let mut current = vector.frame.clone();
        current["protocol"] = serde_json::json!(v2::CONTRACT);
        let (version, request) =
            v2::decode_request(&serde_json::to_vec(&current).unwrap()).unwrap();
        assert_eq!(version, v2::Version::V0Alpha2);
        assert_eq!(serde_json::to_value(request).unwrap(), current);
        current["protocol"] = serde_json::json!("b10x.connector-operation.v0alpha99");
        assert!(v2::decode_request(&serde_json::to_vec(&current).unwrap()).is_err());
        assert!(serde_json::from_value::<legacy::RequestEnvelope>(current)
            .unwrap()
            .validate()
            .is_err());
    }
}

#[derive(Deserialize)]
struct OperationV2Vectors {
    contract: String,
    cases: Vec<OperationV2Vector>,
}
#[derive(Deserialize)]
struct OperationV2Vector {
    name: String,
    kind: String,
    valid: bool,
    schema_valid: bool,
    note: String,
    frame: Value,
}
fn operation_v2_vectors() -> OperationV2Vectors {
    serde_json::from_slice(
        &fs::read(root().join("contracts/connector-operation/v0alpha2/vectors.json")).unwrap(),
    )
    .unwrap()
}

#[test]
fn operation_v2_complete_request_and_response_vectors_match_rust() {
    use protocol::operation::wire as v2;
    let vectors = operation_v2_vectors();
    assert_eq!(vectors.contract, v2::CONTRACT);
    let mut names = std::collections::BTreeSet::new();
    for vector in vectors.cases {
        assert!(names.insert(vector.name.clone()), "duplicate vector name");
        let valid = match vector.kind.as_str() {
            "request" => serde_json::from_value::<v2::RequestEnvelope>(vector.frame)
                .is_ok_and(|frame| frame.validate().is_ok()),
            "response" => serde_json::from_value::<v2::ResponseEnvelope>(vector.frame)
                .is_ok_and(|frame| frame.validate().is_ok()),
            _ => panic!("unrecognized vector kind"),
        };
        assert_eq!(
            valid, vector.valid,
            "Rust vector {}: {}",
            vector.name, vector.note
        );
        assert!(
            vector.valid == vector.schema_valid || !vector.note.is_empty(),
            "schema limitation must be explained"
        );
    }
}

#[test]
fn operation_v2_complete_request_and_response_vectors_match_schema() {
    let schema = protocol::operation::schema::operation_v2_schema();
    let artifact: Value = serde_json::from_slice(
        &fs::read(
            root().join("contracts/connector-operation/v0alpha2/connector-operation.schema.json"),
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(
        artifact, schema,
        "schema artifact must match the typed projection"
    );
    let validator = jsonschema::options()
        .with_draft(jsonschema::Draft::Draft202012)
        .should_validate_formats(true)
        .build(&schema)
        .unwrap();
    for vector in operation_v2_vectors().cases {
        let errors: Vec<_> = validator
            .iter_errors(&vector.frame)
            .map(|error| error.to_string())
            .collect();
        assert_eq!(
            errors.is_empty(),
            vector.schema_valid,
            "schema vector {}: {errors:?}",
            vector.name
        );
    }
}

#[test]
fn operation_v2_vectors_cover_every_request_result_and_error_variant() {
    let schema = protocol::operation::schema::operation_v2_schema();
    let vectors = operation_v2_vectors();
    for (definition, tag, frame_path) in [
        ("OperationRequest", "method", "/request/method"),
        ("OperationResult", "result", "/response/result"),
    ] {
        for variant in schema["$defs"][definition]["oneOf"].as_array().unwrap() {
            let name = variant["properties"][tag]["const"].as_str().unwrap();
            assert!(
                vectors.cases.iter().any(|vector| vector.valid
                    && vector.frame.pointer(frame_path).and_then(Value::as_str) == Some(name)),
                "missing positive {definition} vector {name}"
            );
        }
    }
    for code in schema["$defs"]["OperationErrorCode"]["enum"]
        .as_array()
        .unwrap()
    {
        assert!(
            vectors
                .cases
                .iter()
                .any(|vector| vector.valid && vector.frame.pointer("/error/code") == Some(code)),
            "missing error vector {code}"
        );
    }
}

#[test]
fn operation_legacy_snapshot_preserves_deployed_schema_discrepancies() {
    use protocol::operation::{legacy, wire as v2};
    let frozen: Value = serde_json::from_slice(
        &fs::read(
            root().join("contracts/connector-operation/v0alpha1/connector-operation.schema.json"),
        )
        .unwrap(),
    )
    .unwrap();
    let validator = jsonschema::validator_for(&frozen).unwrap();
    for vector in operation_v2_vectors().cases.into_iter().filter(|vector| {
        matches!(
            vector.name.as_str(),
            "request-session_signal" | "response-search" | "response-session_signal"
        )
    }) {
        let mut old = vector.frame;
        old["protocol"] = serde_json::json!(legacy::CONTRACT);
        if vector.kind == "request" {
            let bytes = serde_json::to_vec(&old).unwrap();
            let original = serde_json::from_slice::<legacy::RequestEnvelope>(&bytes).unwrap();
            original.validate().unwrap();
            let (version, current) = v2::decode_request(&bytes).unwrap();
            assert_eq!(version, v2::Version::V0Alpha1);
            assert_eq!(current.into_legacy(), original);
        } else {
            let original = serde_json::from_value::<legacy::ResponseEnvelope>(old.clone()).unwrap();
            original.validate().unwrap();
            let current = v2::ResponseEnvelope::from(original.clone());
            let bytes = v2::Version::V0Alpha1.encode_response(current).unwrap();
            assert_eq!(
                serde_json::from_slice::<legacy::ResponseEnvelope>(&bytes).unwrap(),
                original
            );
        }
        assert!(
            !validator.is_valid(&old),
            "frozen schema discrepancy must remain explicit"
        );
    }
}

#[test]
fn operation_version_decoder_refuses_duplicate_fields_before_dispatch() {
    use protocol::operation::{legacy, wire as v2};
    let vector = operation_v2_vectors()
        .cases
        .into_iter()
        .find(|vector| vector.name == "request-invoke")
        .unwrap();
    for contract in [legacy::CONTRACT, v2::CONTRACT] {
        let mut frame = vector.frame.clone();
        frame["protocol"] = serde_json::json!(contract);
        let json = serde_json::to_string(&frame).unwrap();
        let duplicated = json.replacen(
            "\"protocol\":",
            &format!("\"protocol\":\"{contract}\",\"protocol\":"),
            1,
        );
        assert!(v2::decode_request(duplicated.as_bytes()).is_err());
    }
}

#[test]
fn operation_v2_description_downgrade_loses_only_advice() {
    use protocol::operation::{legacy, wire as v2};
    let vector = operation_v2_vectors()
        .cases
        .into_iter()
        .find(|vector| vector.name == "response-describe")
        .unwrap();
    let current: v2::ResponseEnvelope = serde_json::from_value(vector.frame.clone()).unwrap();
    let bytes = v2::Version::V0Alpha1.encode_response(current).unwrap();
    let mut expected = vector.frame;
    expected["protocol"] = serde_json::json!(legacy::CONTRACT);
    expected["response"]["value"]
        .as_object_mut()
        .unwrap()
        .remove("rate_advice");
    assert_eq!(serde_json::from_slice::<Value>(&bytes).unwrap(), expected);
}

#[test]
fn rate_repair_source_uri_grammar_matches_wire_and_published_schema() {
    use protocol::operation::wire as v2;
    let schema: Value = serde_json::from_slice(
        &fs::read(
            root().join("contracts/connector-operation/v0alpha2/connector-operation.schema.json"),
        )
        .unwrap(),
    )
    .unwrap();
    let source_schema = &schema["$defs"]["ConditionalRateLimit"]["properties"]["source_url"];
    let validator = jsonschema::options()
        .with_draft(jsonschema::Draft::Draft202012)
        .should_validate_formats(true)
        .build(source_schema)
        .unwrap();
    let check = |source: &str, expected: bool| {
        let advice = v2::OperationRateAdvice {
            fixed: None,
            alternatives: vec![v2::ConditionalRateAdvice::new(v2::ConditionalRateLimit {
                applies_when: "a documented category".into(),
                rate: None,
                source_url: source.into(),
            })],
        };
        let rust = advice.validate().is_ok();
        let schema = validator.is_valid(&serde_json::json!(source));
        assert_eq!(
            (rust, schema),
            (expected, expected),
            "{source:?}: wire/schema URL grammar"
        );
    };
    for (source, expected) in [
        ("https://docs.example.test", true),
        ("https://DOCS.example.test/rate", true),
        ("https://docs.example.test:443/rate", true),
        ("https://docs.example.test:/rate", true),
        ("https://127.0.0.1:443/rate", true),
        ("https://[2001:db8::1]:443/rate", true),
        ("https://docs.example.test/a%20b?category=a/b?c", true),
        ("https://docs.example.test/%E2%82%AC", true),
        ("https://docs.example.test/rate?email=a@b", true),
        ("HTTPS://docs.example.test/rate", false),
        ("hTtPs://docs.example.test/rate", false),
        ("http://docs.example.test/rate", false),
        ("https:/docs.example.test/rate", false),
        ("https:///rate", false),
        ("https://:443/rate", false),
        ("https://user@docs.example.test/rate", false),
        ("https://user:pass@docs.example.test/rate", false),
        ("https://@docs.example.test/rate", false),
        ("https://docs.example.test/rate#", false),
        ("https://docs.example.test/rate#part", false),
        (" https://docs.example.test/rate", false),
        ("https://docs.example.test/a b", false),
        ("https://docs.example.test/rate?x=a b", false),
        ("https://docs.example.test/rate\n", false),
        ("https://docs.example.test/a\\b", false),
        ("https://docs.example.test/a%", false),
        ("https://docs.example.test/a%2", false),
        ("https://docs.example.test/a%GG", false),
        ("https://docs.example.test/€", false),
        ("https://döcs.example.test/rate", false),
        ("https://[2001:db8::zz]/rate", false),
        ("https://[2001:db8::1/rate", false),
    ] {
        check(source, expected);
    }

    for port in [
        0_u32, 9, 10, 99, 100, 999, 1000, 9999, 10000, 59999, 60000, 64999, 65000, 65499, 65500,
        65529, 65530, 65535, 65536, 99999, 100000,
    ] {
        for spelling in [port.to_string(), format!("000{port}")] {
            check(
                &format!("https://docs.example.test:{spelling}/rate"),
                port <= u32::from(u16::MAX),
            );
        }
    }
    for port in ["-1", "+1", "1.0", "1e2", "18446744073709551616"] {
        check(&format!("https://docs.example.test:{port}/rate"), false);
    }
    for source in [
        "https://%64ocs.example.test/a%2fb",
        "https://[v1.fe80]/rate",
        "https://docs.example.test/a%23b?x=%40",
    ] {
        check(source, true);
    }
    let prefix = "https://docs.example.test/";
    check(
        &format!("{prefix}{}", "a".repeat(2048 - prefix.len())),
        true,
    );
    check(
        &format!("{prefix}{}", "a".repeat(2049 - prefix.len())),
        false,
    );
    check("", false);
}

// Additive authentication contracts. Frozen predecessor tests above remain authoritative.
fn authentication_vectors(directory: &str) -> OperationV2Vectors {
    serde_json::from_slice(&fs::read(root().join(directory).join("vectors.json")).unwrap()).unwrap()
}

#[test]
fn authentication_operation_v3_vectors_have_independent_reader_and_schema_results() {
    use protocol::operation::v3;
    let schema = protocol::operation::schema_v3::operation_v3_schema();
    let validator = jsonschema::options()
        .with_draft(jsonschema::Draft::Draft202012)
        .should_validate_formats(true)
        .build(&schema)
        .unwrap();
    for case in authentication_vectors("contracts/connector-operation/v0alpha3").cases {
        let bytes = serde_json::to_vec(&case.frame).unwrap();
        let rust = match case.kind.as_str() {
            "request" => serde_json::from_slice::<v3::RequestEnvelope>(&bytes)
                .is_ok_and(|value| value.validate().is_ok()),
            "response" => serde_json::from_slice::<v3::ResponseEnvelope>(&bytes)
                .is_ok_and(|value| value.validate().is_ok()),
            _ => panic!("unexpected vector kind"),
        };
        let errors: Vec<_> = validator
            .iter_errors(&case.frame)
            .map(|e| e.to_string())
            .collect();
        assert_eq!(
            (rust, errors.is_empty()),
            (case.valid, case.schema_valid),
            "{}: {errors:?}",
            case.name
        );
    }
}

#[test]
fn authentication_connection_v2_vectors_have_independent_reader_and_schema_results() {
    use protocol::connection_v2 as v2;
    let schema = protocol::connection_v2_schema::connection_v2_schema();
    let validator = jsonschema::options()
        .with_draft(jsonschema::Draft::Draft202012)
        .should_validate_formats(true)
        .build(&schema)
        .unwrap();
    for case in authentication_vectors("contracts/connector-connection/v0alpha2").cases {
        let bytes = serde_json::to_vec(&case.frame).unwrap();
        let rust = match case.kind.as_str() {
            "request" => serde_json::from_slice::<v2::RequestEnvelope>(&bytes)
                .is_ok_and(|value| value.validate().is_ok()),
            "response" => serde_json::from_slice::<v2::ResponseEnvelope>(&bytes)
                .is_ok_and(|value| value.validate().is_ok()),
            _ => panic!("unexpected vector kind"),
        };
        let errors: Vec<_> = validator
            .iter_errors(&case.frame)
            .map(|e| e.to_string())
            .collect();
        assert_eq!(
            (rust, errors.is_empty()),
            (case.valid, case.schema_valid),
            "{}: {errors:?}",
            case.name
        );
    }
}

#[test]
fn authentication_v3_inherits_every_v2_vector_and_schema_constraint() {
    let v2_schema = protocol::operation::schema::operation_v2_schema();
    let v3_schema = protocol::operation::schema_v3::operation_v3_schema();
    for (name, definition) in v2_schema["$defs"].as_object().unwrap() {
        if !matches!(
            name.as_str(),
            "RequestEnvelope" | "ResponseEnvelope" | "OperationError" | "OperationErrorCode"
        ) {
            assert_eq!(
                &v3_schema["$defs"][name], definition,
                "inherited schema {name}"
            );
        }
    }
    let old = operation_v2_vectors();
    let new = authentication_vectors("contracts/connector-operation/v0alpha3");
    assert_eq!(old.cases.len(), 85);
    for previous in old.cases {
        let next = new
            .cases
            .iter()
            .find(|case| case.name == previous.name)
            .unwrap();
        let mut expected = previous.frame;
        if expected["protocol"] == protocol::operation::wire::CONTRACT {
            expected["protocol"] = serde_json::json!(protocol::operation::v3::CONTRACT);
        }
        assert_eq!(
            next.frame, expected,
            "{} changed beyond identity",
            previous.name
        );
        assert_eq!(
            (next.valid, next.schema_valid),
            (previous.valid, previous.schema_valid)
        );
    }
}

#[test]
fn authentication_downgrade_is_exact_neutral_and_non_retriable() {
    use protocol::operation::{legacy, v3, versions::Version, wire};
    for case in authentication_vectors("contracts/connector-operation/v0alpha3")
        .cases
        .into_iter()
        .filter(|case| {
            case.valid
                && case.frame.pointer("/error/code")
                    == Some(&serde_json::json!("authentication_required"))
        })
    {
        let mut current: v3::ResponseEnvelope = serde_json::from_value(case.frame).unwrap();
        current.error.as_mut().unwrap().message = "Private contextual text must be lost.".into();
        for (version, identity) in [
            (Version::V0Alpha1, legacy::CONTRACT),
            (Version::V0Alpha2, wire::CONTRACT),
        ] {
            let bytes = version.encode_response(current.clone()).unwrap();
            assert_eq!(
                serde_json::from_slice::<Value>(&bytes).unwrap(),
                serde_json::json!({
                    "protocol":identity,"request_id":"request-1","status":"error",
                    "error":{"code":"unavailable","message":"The operation is unavailable.","retriable":false}
                })
            );
        }
        let bytes = Version::V0Alpha3.encode_response(current.clone()).unwrap();
        assert_eq!(
            protocol::operation::versions::decode_response(&bytes).unwrap(),
            (Version::V0Alpha3, current)
        );
    }
}

#[test]
fn authentication_version_adapters_preserve_ordinary_and_rate_frames() {
    use protocol::operation::{
        legacy, v3,
        versions::{self, Version},
        wire,
    };
    for case in operation_v2_vectors()
        .cases
        .into_iter()
        .filter(|case| case.valid)
    {
        if case.kind == "request" {
            let original: wire::RequestEnvelope = serde_json::from_value(case.frame).unwrap();
            for version in [Version::V0Alpha1, Version::V0Alpha2, Version::V0Alpha3] {
                let bytes = version.encode_request(original.clone()).unwrap();
                assert_eq!(
                    versions::decode_request(&bytes).unwrap(),
                    (version, original.clone())
                );
            }
        } else {
            let original: wire::ResponseEnvelope = serde_json::from_value(case.frame).unwrap();
            let lifted = v3::ResponseEnvelope::from(original.clone());
            for (version, old_version) in [
                (Version::V0Alpha1, wire::Version::V0Alpha1),
                (Version::V0Alpha2, wire::Version::V0Alpha2),
            ] {
                let bytes = version.encode_response(lifted.clone()).unwrap();
                assert_eq!(
                    bytes,
                    old_version.encode_response(original.clone()).unwrap()
                );
                let (_, decoded) = versions::decode_response(&bytes).unwrap();
                if version == Version::V0Alpha1 {
                    let previous: legacy::ResponseEnvelope =
                        serde_json::from_slice(&bytes).unwrap();
                    assert_eq!(
                        decoded,
                        v3::ResponseEnvelope::from(wire::ResponseEnvelope::from(previous))
                    );
                } else {
                    assert_eq!(decoded, lifted);
                }
            }
        }
    }
}

#[test]
fn authentication_bound_requests_cannot_downgrade_to_unbound_creation() {
    use protocol::{connection as old, connection_v2 as new};
    for case in authentication_vectors("contracts/connector-connection/v0alpha2")
        .cases
        .into_iter()
        .filter(|case| case.valid)
    {
        let bound = case
            .frame
            .pointer("/request/method")
            .or_else(|| case.frame.pointer("/response/result"))
            .and_then(Value::as_str)
            .is_some_and(|tag| tag.starts_with("remediation_"));
        if case.kind == "request" {
            let current: new::RequestEnvelope = serde_json::from_value(case.frame).unwrap();
            if bound {
                let error = current.clone().into_v1().unwrap_err();
                assert_eq!(error.code, old::ConnectionErrorCode::Protocol);
                assert!(!error.retriable);
                assert!(new::Version::V0Alpha1.encode_request(current).is_err());
            } else {
                let previous = current.clone().into_v1().unwrap();
                assert_eq!(new::RequestEnvelope::from(previous.clone()), current);
                let bytes = new::Version::V0Alpha1.encode_request(current).unwrap();
                assert_eq!(
                    serde_json::from_slice::<old::RequestEnvelope>(&bytes).unwrap(),
                    previous
                );
            }
        } else {
            let current: new::ResponseEnvelope = serde_json::from_value(case.frame).unwrap();
            if bound {
                assert!(current.clone().into_v1().is_err());
                assert!(new::Version::V0Alpha1.encode_response(current).is_err());
            } else {
                let previous = current.clone().into_v1().unwrap();
                assert_eq!(new::ResponseEnvelope::from(previous), current);
            }
        }
    }
}

#[test]
fn authentication_selected_decoders_preserve_original_duplicate_fields() {
    use protocol::operation::{legacy, v3, versions, wire};
    use protocol::{connection, connection_v2};
    let op_request = operation_v2_vectors()
        .cases
        .into_iter()
        .find(|case| case.name == "request-invoke")
        .unwrap()
        .frame;
    let op_response = operation_v2_vectors()
        .cases
        .into_iter()
        .find(|case| case.name == "error-unavailable")
        .unwrap()
        .frame;
    let conn_request = authentication_vectors("contracts/connector-connection/v0alpha2")
        .cases
        .into_iter()
        .find(|case| case.name == "request-connect_session_status")
        .unwrap()
        .frame;
    let conn_response = authentication_vectors("contracts/connector-connection/v0alpha2")
        .cases
        .into_iter()
        .find(|case| case.name == "error-conflict")
        .unwrap()
        .frame;
    for (identities, request, response, operation) in [
        (
            vec![legacy::CONTRACT, wire::CONTRACT, v3::CONTRACT],
            op_request,
            op_response,
            true,
        ),
        (
            vec![connection::CONTRACT, connection_v2::CONTRACT],
            conn_request,
            conn_response,
            false,
        ),
    ] {
        for identity in identities {
            for (mut frame, is_request) in [(request.clone(), true), (response.clone(), false)] {
                frame["protocol"] = serde_json::json!(identity);
                let json = serde_json::to_string(&frame).unwrap();
                let accepts = |bytes: &[u8]| match (operation, is_request) {
                    (true, true) => versions::decode_request(bytes).is_ok(),
                    (true, false) => versions::decode_response(bytes).is_ok(),
                    (false, true) => connection_v2::decode_request(bytes).is_ok(),
                    (false, false) => connection_v2::decode_response(bytes).is_ok(),
                };
                assert!(accepts(json.as_bytes()), "positive {identity}");
                for (key, value) in [
                    ("protocol", serde_json::json!(identity)),
                    ("request_id", serde_json::json!("request-1")),
                    (
                        if is_request { "context" } else { "error" },
                        frame[if is_request { "context" } else { "error" }].clone(),
                    ),
                ] {
                    let duplicate = json.replacen(
                        &format!("\"{key}\":"),
                        &format!("\"{key}\":{value},\"{key}\":"),
                        1,
                    );
                    assert!(
                        !accepts(duplicate.as_bytes()),
                        "duplicate {key} in {identity}"
                    );
                }
                if is_request {
                    let duplicate = json.replacen(
                        "\"agent_id\":",
                        "\"agent_id\":\"agent-duplicate\",\"agent_id\":",
                        1,
                    );
                    assert!(!accepts(duplicate.as_bytes()));
                } else {
                    let duplicate =
                        json.replacen("\"code\":", "\"code\":\"unavailable\",\"code\":", 1);
                    assert!(!accepts(duplicate.as_bytes()));
                }
                for wrong in [
                    "unknown",
                    "b10x.connector-operation.v0alpha4",
                    "b10x.connector-connection.v0alpha3",
                    "",
                ] {
                    frame["protocol"] = serde_json::json!(wrong);
                    assert!(!accepts(&serde_json::to_vec(&frame).unwrap()));
                }
            }
        }
    }
}

#[test]
fn authentication_connection_frame_and_input_budgets_are_version_specific() {
    use protocol::{connection, connection_v2 as v2};
    let vectors = authentication_vectors("contracts/connector-connection/v0alpha2");
    let frame = vectors
        .cases
        .iter()
        .find(|case| case.name == "request-search")
        .unwrap()
        .frame
        .clone();
    for (identity, limit, version) in [
        (
            connection::CONTRACT,
            connection::MAX_FRAME_BYTES,
            v2::Version::V0Alpha1,
        ),
        (v2::CONTRACT, v2::MAX_FRAME_BYTES, v2::Version::V0Alpha2),
    ] {
        let mut frame = frame.clone();
        frame["protocol"] = serde_json::json!(identity);
        let mut bytes = serde_json::to_vec(&frame).unwrap();
        bytes.resize(limit, b' ');
        assert_eq!(v2::decode_request(&bytes).unwrap().0, version);
        bytes.push(b' ');
        assert!(v2::decode_request(&bytes).is_err());
    }
    for (name, expected) in [
        ("request-start-input-at-byte-limit", true),
        ("request-start-input-over-byte-limit", false),
        ("request-start-unicode-input-over-byte-limit", false),
    ] {
        let frame = &vectors
            .cases
            .iter()
            .find(|case| case.name == name)
            .unwrap()
            .frame;
        assert_eq!(
            v2::decode_request(&serde_json::to_vec(frame).unwrap()).is_ok(),
            expected,
            "{name}"
        );
    }
}

#[test]
fn authentication_bundle_manifests_match_all_published_bytes() {
    check("contracts/connector-operation/v0alpha3/bundle.json");
    check("contracts/connector-connection/v0alpha2/bundle.json");
}

#[test]
fn authentication_vectors_cover_every_request_result_and_error_variant() {
    for (directory, schema, prefix) in [
        (
            "contracts/connector-operation/v0alpha3",
            protocol::operation::schema_v3::operation_v3_schema(),
            "Operation",
        ),
        (
            "contracts/connector-connection/v0alpha2",
            protocol::connection_v2_schema::connection_v2_schema(),
            "Connection",
        ),
    ] {
        let vectors = authentication_vectors(directory);
        for (suffix, tag, path) in [
            ("Request", "method", "/request/method"),
            ("Result", "result", "/response/result"),
        ] {
            for variant in schema["$defs"][format!("{prefix}{suffix}")]["oneOf"]
                .as_array()
                .unwrap()
            {
                let name = variant["properties"][tag]["const"].as_str().unwrap();
                assert!(
                    vectors.cases.iter().any(|case| case.valid
                        && case.frame.pointer(path).and_then(Value::as_str) == Some(name)),
                    "{directory}: missing {name}"
                );
            }
        }
        for code in schema["$defs"][format!("{prefix}ErrorCode")]["enum"]
            .as_array()
            .unwrap()
        {
            assert!(
                vectors
                    .cases
                    .iter()
                    .any(|case| case.valid && case.frame.pointer("/error/code") == Some(code)),
                "{directory}: missing {code}"
            );
        }
    }
}

#[test]
fn authentication_new_payloads_refuse_every_predecessor_identity() {
    use protocol::operation::{legacy, v3, versions, wire};
    use protocol::{connection, connection_v2};
    let mut auth = authentication_vectors("contracts/connector-operation/v0alpha3")
        .cases
        .into_iter()
        .find(|case| case.name == "authentication-authorize_configured")
        .unwrap()
        .frame;
    for identity in [legacy::CONTRACT, wire::CONTRACT, connection_v2::CONTRACT] {
        auth["protocol"] = serde_json::json!(identity);
        assert!(versions::decode_response(&serde_json::to_vec(&auth).unwrap()).is_err());
    }
    auth["protocol"] = serde_json::json!(v3::CONTRACT);
    assert!(versions::decode_response(&serde_json::to_vec(&auth).unwrap()).is_ok());
    for case in authentication_vectors("contracts/connector-connection/v0alpha2")
        .cases
        .into_iter()
        .filter(|case| {
            case.valid
                && case
                    .frame
                    .pointer("/request/method")
                    .or_else(|| case.frame.pointer("/response/result"))
                    .and_then(Value::as_str)
                    .is_some_and(|tag| tag.starts_with("remediation_"))
        })
    {
        let mut old = case.frame;
        old["protocol"] = serde_json::json!(connection::CONTRACT);
        let bytes = serde_json::to_vec(&old).unwrap();
        if case.kind == "request" {
            assert!(connection_v2::decode_request(&bytes).is_err());
        } else {
            assert!(connection_v2::decode_response(&bytes).is_err());
        }
    }
}

#[test]
fn authentication_new_nested_dtos_refuse_original_duplicate_fields() {
    use protocol::{connection_v2, operation::versions};
    for (directory, name, key, operation, request) in [
        (
            "contracts/connector-operation/v0alpha3",
            "authentication-authorize_configured",
            "need",
            true,
            false,
        ),
        (
            "contracts/connector-operation/v0alpha3",
            "request-invoke",
            "operation_ref",
            true,
            true,
        ),
        (
            "contracts/connector-connection/v0alpha2",
            "request-remediation_start",
            "connection_ref",
            false,
            true,
        ),
        (
            "contracts/connector-connection/v0alpha2",
            "response-remediation_status",
            "expires_at_unix_ms",
            false,
            false,
        ),
        (
            "contracts/connector-connection/v0alpha2",
            "response-remediation_acknowledge",
            "next_action",
            false,
            false,
        ),
    ] {
        let case = authentication_vectors(directory)
            .cases
            .into_iter()
            .find(|case| case.name == name)
            .unwrap();
        let text = serde_json::to_string(&case.frame).unwrap();
        let key_token = format!("\"{key}\":");
        // Duplicate the original value, not an invalid extra value: the duplicate itself decides.
        let from = text.find(&key_token).unwrap() + key_token.len();
        let value: Value = serde_json::Deserializer::from_str(&text[from..])
            .into_iter::<Value>()
            .next()
            .unwrap()
            .unwrap();
        let duplicate = text.replacen(&key_token, &format!("{key_token}{value},{key_token}"), 1);
        let valid = match (operation, request) {
            (true, true) => versions::decode_request(duplicate.as_bytes()).is_ok(),
            (true, false) => versions::decode_response(duplicate.as_bytes()).is_ok(),
            (false, true) => connection_v2::decode_request(duplicate.as_bytes()).is_ok(),
            (false, false) => connection_v2::decode_response(duplicate.as_bytes()).is_ok(),
        };
        assert!(!valid, "{directory}/{name}: duplicate {key}");
    }
}

#[test]
fn authentication_predecessor_artifacts_and_reviewed_readers_stay_pinned() {
    for (path, expected) in [
        (
            "contracts/artifact-bundle.schema.json",
            "5e6fbffee97d998df53c94394de1c53c75a6066f2807c9c00374b6ea15ea969b",
        ),
        (
            "contracts/connector-connection/v0alpha1/README.md",
            "aafcaddc3170869735002248441925a30a90c525604f3a9813d9bfc3025d6402",
        ),
        (
            "contracts/connector-connection/v0alpha1/bundle.json",
            "0ec3d6d830a438074aea3e944478bfc60691befe8bebff00c93c8b9d80f1d3f9",
        ),
        (
            "contracts/connector-connection/v0alpha1/connector-connection.schema.json",
            "4c80f6cfda35ba7504d56c138cb4602432abf8e93934fb21d21524b64a9c3c84",
        ),
        (
            "contracts/connector-connection/v0alpha1/vectors.json",
            "5169efc1c46cd435520229c34d737303131d145e2b70dc8c4ca2fadde94b8506",
        ),
        (
            "contracts/connector-connection/v0alpha1/vectors.schema.json",
            "8ed192a244e4ff3b1b0ed486f9298e6c587a038ac06699e0acf2a3bfa3b7d5c9",
        ),
        (
            "contracts/connector-operation/v0alpha1/README.md",
            "7df2ae9fbf43afea0a6e81f163d0de22fc15fc0df435332c7d21591e3f2fe099",
        ),
        (
            "contracts/connector-operation/v0alpha1/bundle.json",
            "7d238998ffe6e8b099d7f12608dbae4cbe1c97bab6e816e07b76f290458db1ef",
        ),
        (
            "contracts/connector-operation/v0alpha1/connector-operation.schema.json",
            "eac0ef273d7f5b6565ff9b2cf3366ed4fab686b4dce0b913581bef10431d35ce",
        ),
        (
            "contracts/connector-operation/v0alpha1/vectors.json",
            "081f913e62c7d2729503b100fb3e7ccff7587228a9793b750f4cd82bb064cfd8",
        ),
        (
            "contracts/connector-operation/v0alpha1/vectors.schema.json",
            "e5525324c6d15e60292c4caf3b7251b916e5d73354f9878873c3d62476624086",
        ),
        (
            "contracts/connector-operation/v0alpha2/README.md",
            "ac3076100daddcab6e18a6be7bcfa97bd4804f4231ec5da898c75479c6f8830e",
        ),
        (
            "contracts/connector-operation/v0alpha2/bundle.json",
            "329d8db6c8710fbc512f44432061560fb109b9962118f176945800073e82be63",
        ),
        (
            "contracts/connector-operation/v0alpha2/connector-operation.schema.json",
            "e961d094eb82c4fbb825f7109d904c210d42ac5bcb34a033ba17db58a42f6d79",
        ),
        (
            "contracts/connector-operation/v0alpha2/vectors.json",
            "9244045b3eb449b2dfbc4ad83b201d6ec4bc35bb4f6ff0cf8d897dfb9e2dfee1",
        ),
        (
            "contracts/connector-operation/v0alpha2/vectors.schema.json",
            "ff17bdea4b6100f8669db076ecfa12188a629ebd7f706fe86a4c91449f357c03",
        ),
        (
            "crates/protocol/examples/operation_v2_bundle.rs",
            "450adf436312cfb8e1dc5526f2855036bdd0e0ea071bc0803f96f028ef22c220",
        ),
        (
            "crates/protocol/src/connection.rs",
            // Reviewed compatibility correction: pending status may omit the completion
            // capability. Creation, URL and terminal checks remain strict; the v1
            // bundle above is unchanged. connect_session_status.rs checks both readers.
            "de405ce18fa32ed14c33255642ad3750c61d55bd671c4cb5025cc047cfb19f5d",
        ),
        (
            "crates/protocol/src/operation/legacy.rs",
            "b97996a726f6da233daa7f5b14f56f7ec5487bbf5fa3ba24152b7496dad26624",
        ),
        (
            "crates/protocol/src/operation/schema.rs",
            "2359ca91b9d2877cfbf2736587f312720aada9c94d0b7864185c4365d16abd04",
        ),
        (
            "crates/protocol/src/operation/wire.rs",
            "04bf208c61259c4d48f7bfeaeadac0048512de5e22ead733ac69a51a0b3d8330",
        ),
    ] {
        let bytes = fs::read(root().join(path)).unwrap();
        assert_eq!(
            format!("{:x}", Sha256::digest(&bytes)),
            expected,
            "pinned predecessor {path}"
        );
    }
}

#[test]
fn auth_adversary_original_vector_bytes_reject_escaped_duplicates_and_padded_overflow() {
    use protocol::{
        connection_v2,
        operation::{self, versions},
    };
    let mut reached = 0;
    for (directory, connection) in [
        ("contracts/connector-operation/v0alpha3", false),
        ("contracts/connector-connection/v0alpha2", true),
    ] {
        for vector in authentication_vectors(directory)
            .cases
            .into_iter()
            .filter(|v| v.valid)
        {
            let request = vector.kind == "request";
            let relevant = if connection {
                let side = if request { "request" } else { "response" };
                let tag = if request { "method" } else { "result" };
                vector.frame[side][tag]
                    .as_str()
                    .is_some_and(|v| v.starts_with("remediation_"))
            } else {
                request || vector.frame["error"]["code"] == "authentication_required"
            };
            if !relevant {
                continue;
            }
            let accepts = |bytes: &[u8]| match (connection, request) {
                (true, true) => connection_v2::decode_request(bytes).is_ok(),
                (true, false) => connection_v2::decode_response(bytes).is_ok(),
                (false, true) => versions::decode_request(bytes).is_ok(),
                (false, false) => versions::decode_response(bytes).is_ok(),
            };
            let original = serde_json::to_string(&vector.frame).unwrap();
            assert!(accepts(original.as_bytes()), "{}", vector.name);
            for (plain, escaped) in [
                ("protocol", r#"pro\u0074ocol"#),
                ("request_id", r#"request_\u0069d"#),
            ] {
                let value = serde_json::to_string(&vector.frame[plain]).unwrap();
                let key = format!("\"{plain}\":");
                let duplicate = original.replacen(&key, &format!("\"{escaped}\":{value},{key}"), 1);
                assert_ne!(duplicate, original);
                assert!(!accepts(duplicate.as_bytes()), "{} / {plain}", vector.name);
            }
            let maximum = match (connection, request) {
                (true, true) => connection_v2::MAX_FRAME_BYTES,
                (true, false) => connection_v2::MAX_RESPONSE_BYTES,
                (false, true) => operation::MAX_FRAME_BYTES,
                (false, false) => operation::MAX_RESULT_BYTES,
            };
            let mut padded = original.into_bytes();
            assert!(padded.len() < maximum);
            padded.resize(maximum, b' ');
            assert!(accepts(&padded), "exact byte ceiling: {}", vector.name);
            padded.push(b' ');
            assert!(!accepts(&padded), "original byte overflow: {}", vector.name);
            reached += 1;
        }
    }
    assert!(reached >= 10, "actual positive vectors selected: {reached}");
}
