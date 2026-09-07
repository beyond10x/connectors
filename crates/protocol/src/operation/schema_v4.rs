//! Deterministic endpoint-aware request projection, retaining existing result constraints.

use super::{schema_v3, v4};
use serde_json::{json, Value};

/// Publish the complete v4 frame schema without modifying predecessor definitions.
#[must_use]
pub fn operation_v4_schema() -> Value {
    #[derive(schemars::JsonSchema)]
    #[serde(untagged)]
    #[allow(dead_code)]
    enum Frame {
        Request(v4::RequestEnvelope),
        Response(v4::ResponseEnvelope),
    }
    let mut result = serde_json::to_value(schemars::schema_for!(Frame)).expect("schema serializes");
    let inherited = schema_v3::operation_v3_schema();
    let defs = result["$defs"].as_object_mut().expect("typed definitions");
    for (name, value) in inherited["$defs"]
        .as_object()
        .expect("inherited definitions")
    {
        if !matches!(
            name.as_str(),
            "OperationRequest" | "DescribeRequest" | "InvokeRequest"
        ) {
            defs.insert(name.clone(), value.clone());
        }
    }
    for envelope in ["RequestEnvelope", "ResponseEnvelope"] {
        defs[envelope]["properties"]["protocol"] = json!({"const":v4::CONTRACT});
    }
    for name in ["DescribeRequest", "InvokeRequest"] {
        for field in ["operation_ref", "connection_ref", "endpoint_ref"] {
            defs[name]["properties"][field] =
                json!({"type":"string","minLength":1,"maxLength":512,"pattern":"^[!-~]+$"});
        }
    }
    defs["DescribeRequest"]["not"] = json!({"required":["connection_ref","endpoint_ref"]});
    defs["InvokeRequest"]["oneOf"] = json!([
        {"required":["connection_ref"],"not":{"required":["endpoint_ref"]}},
        {"required":["endpoint_ref"],"not":{"required":["connection_ref"]}}
    ]);
    for field in ["description_ref", "approval_evidence_ref"] {
        defs["InvokeRequest"]["properties"][field] =
            inherited["$defs"]["InvokeRequest"]["properties"][field].clone();
    }
    result["$id"] = json!("https://github.com/beyond10x/connectors/blob/main/contracts/connector-operation/v0alpha4/connector-operation.schema.json");
    result["title"] = json!("B10x ConnectorOperation v0alpha4 frame");
    result["$comment"] = inherited["$comment"].clone();
    result
}
