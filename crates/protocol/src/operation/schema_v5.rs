//! Deterministic projection of the common endpoint operation contract.
use super::{schema_v3, v5};
use serde_json::{json, Value};

/// Project endpoint-only requests and responses with the existing bounded operation semantics.
#[must_use]
pub fn operation_v5_schema() -> Value {
    #[derive(schemars::JsonSchema)]
    #[serde(untagged)]
    #[allow(dead_code)]
    enum Frame { Request(v5::RequestEnvelope), Response(v5::ResponseEnvelope) }
    let mut schema = serde_json::to_value(schemars::schema_for!(Frame)).expect("schema serializes");
    let inherited = schema_v3::operation_v3_schema();
    let defs = schema["$defs"].as_object_mut().expect("typed definitions");
    for (name, definition) in inherited["$defs"].as_object().expect("inherited definitions") {
        if !matches!(name.as_str(), "OperationRequest" | "DescribeRequest" | "InvokeRequest") {
            defs.insert(name.clone(), definition.clone());
        }
    }
    for envelope in ["RequestEnvelope", "ResponseEnvelope"] {
        defs[envelope]["properties"]["protocol"] = json!({"const":v5::CONTRACT});
    }
    for name in ["DescribeRequest", "InvokeRequest"] {
        for field in ["operation_ref", "endpoint_ref"] {
            defs[name]["properties"][field] = json!({"type":"string","minLength":1,"maxLength":512,"pattern":"^[!-~]+$"});
        }
    }
    for field in ["description_ref", "approval_evidence_ref"] {
        defs["InvokeRequest"]["properties"][field] = inherited["$defs"]["InvokeRequest"]["properties"][field].clone();
    }
    schema["$id"] = json!("https://github.com/beyond10x/connectors/blob/main/contracts/connector-operation/v0alpha5/connector-operation.schema.json");
    schema["title"] = json!("Connectors operation v0alpha5 frame");
    schema["$comment"] = inherited["$comment"].clone();
    schema
}
