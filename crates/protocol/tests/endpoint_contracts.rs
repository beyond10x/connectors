//! The new schemas are deterministic and enforce target exclusivity like the strict readers.
use protocol::{endpoint, operation::v4};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{fs, path::PathBuf};

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}
fn context() -> Value {
    json!({"tenant_id":"tenant","agent_id":"agent","agent_revision":1,"authority_snapshot_id":"snapshot","authority_snapshot_sha256":"a".repeat(64)})
}

#[test]
fn endpoint_contract_projection_and_manifests_match() {
    for (directory, file, expected) in [
        (
            "contracts/connector-endpoint/v0alpha1",
            "connector-endpoint.schema.json",
            protocol::endpoint_schema::endpoint_schema(),
        ),
        (
            "contracts/connector-operation/v0alpha4",
            "connector-operation.schema.json",
            protocol::operation::schema_v4::operation_v4_schema(),
        ),
    ] {
        let schema: Value =
            serde_json::from_slice(&fs::read(root().join(directory).join(file)).unwrap()).unwrap();
        assert_eq!(schema, expected, "{directory}");
        jsonschema::validator_for(&schema).expect("valid standalone schema");
        let bundle: Value =
            serde_json::from_slice(&fs::read(root().join(directory).join("bundle.json")).unwrap())
                .unwrap();
        for file in bundle["files"].as_array().unwrap() {
            let bytes = fs::read(root().join(file["path"].as_str().unwrap())).unwrap();
            assert_eq!(file["bytes"], json!(bytes.len()));
            assert_eq!(
                file["sha256"],
                json!(format!("{:x}", Sha256::digest(bytes)))
            );
        }
    }
}

#[test]
fn endpoint_operation_reader_and_schema_agree_on_target_selection() {
    let validator =
        jsonschema::validator_for(&protocol::operation::schema_v4::operation_v4_schema()).unwrap();
    for (method, params, expected) in [
        ("invoke", json!({"connection_ref":"connection:one"}), true),
        ("invoke", json!({"endpoint_ref":"endpoint:one"}), true),
        ("invoke", json!({}), false),
        ("invoke", json!({"endpoint_ref":null}), false),
        (
            "invoke",
            json!({"endpoint_ref":"endpoint:one","connection_ref":"connection:one"}),
            false,
        ),
        ("describe", json!({}), true),
        ("describe", json!({"endpoint_ref":"endpoint:one"}), true),
        (
            "describe",
            json!({"endpoint_ref":"endpoint:one","connection_ref":"connection:one"}),
            false,
        ),
    ] {
        let mut params = params;
        params["operation_ref"] = json!("loki-query-range");
        if method == "invoke" {
            params["description_ref"] = json!("description:one");
            params["input"] = json!({});
        }
        let frame = json!({"protocol":v4::CONTRACT,"request_id":"request:one","context":context(),"request":{"method":method,"params":params}});
        let read = serde_json::from_value::<v4::RequestEnvelope>(frame.clone())
            .is_ok_and(|request| request.validate().is_ok());
        assert_eq!(read, expected, "reader: {frame}");
        assert_eq!(validator.is_valid(&frame), expected, "schema: {frame}");
    }
}

#[test]
fn endpoint_inventory_reader_and_schema_agree_on_bounds() {
    let validator =
        jsonschema::validator_for(&protocol::endpoint_schema::endpoint_schema()).unwrap();
    for (limit, expected) in [(0, false), (1, true), (100, true), (101, false)] {
        let frame = json!({"protocol":endpoint::CONTRACT,"request_id":"request:one","context":context(),"request":{"method":"list","params":{"source_ref":null,"query":"","limit":limit,"cursor":null}}});
        let read = serde_json::from_value::<endpoint::RequestEnvelope>(frame.clone())
            .is_ok_and(|request| request.validate().is_ok());
        assert_eq!(read, expected);
        assert_eq!(validator.is_valid(&frame), expected);
    }
}
