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
    assert_eq!(vectors.contract, protocol::operation::CONTRACT);
    for vector in vectors.cases {
        let result = serde_json::from_value::<protocol::operation::RequestEnvelope>(vector.frame)
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
    serde_json::from_slice::<protocol::operation::ResponseEnvelope>(&bytes)
        .unwrap()
        .validate()
        .unwrap();
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
