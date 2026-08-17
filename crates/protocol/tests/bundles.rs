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
