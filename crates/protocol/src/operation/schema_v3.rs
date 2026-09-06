//! Typed v3 projection, inheriting the complete frozen v2 structural constraints.
use super::{schema, v3};
use serde_json::{json, Value};

/// Draft 2020-12 frame schema with authentication/code/presence constraints.
#[must_use]
pub fn operation_v3_schema() -> Value {
    #[derive(schemars::JsonSchema)]
    #[serde(untagged)]
    #[allow(dead_code)]
    enum Frame {
        Request(v3::RequestEnvelope),
        Response(v3::ResponseEnvelope),
    }
    let mut result = serde_json::to_value(schemars::schema_for!(Frame)).expect("schema serializes");
    let inherited = schema::operation_v2_schema();
    let defs = result["$defs"].as_object_mut().expect("typed definitions");
    for (name, value) in inherited["$defs"].as_object().expect("v2 definitions") {
        if !matches!(name.as_str(), "OperationError" | "OperationErrorCode") {
            defs.insert(name.clone(), value.clone());
        }
    }
    for envelope in ["RequestEnvelope", "ResponseEnvelope"] {
        defs.get_mut(envelope).expect("envelope")["properties"]["protocol"] =
            json!({"const":v3::CONTRACT});
    }
    let error = defs.get_mut("OperationError").expect("error definition");
    error["properties"]["message"] =
        inherited["$defs"]["OperationError"]["properties"]["message"].clone();
    error["properties"]["retry_after_seconds"] =
        inherited["$defs"]["OperationError"]["properties"]["retry_after_seconds"].clone();
    error["allOf"] = json!([
        {"if":{"required":["retry_after_seconds"]},"then":{"properties":{"code":{"const":"rate_limited"}}}},
        {"if":{"properties":{"code":{"const":"authentication_required"}},"required":["code"]},
         "then":{"required":["authentication"],"properties":{"retriable":{"const":false}},"not":{"required":["retry_after_seconds"]}},
         "else":{"not":{"required":["authentication"]}}}
    ]);
    let auth = defs
        .get_mut("AuthenticationRequired")
        .expect("authentication definition");
    for field in ["operation_ref", "connection_ref", "integration_ref"] {
        auth["properties"][field] =
            json!({"type":"string","minLength":1,"maxLength":512,"pattern":"^[!-~]+$"});
    }
    auth["properties"]["auth_profile"] =
        json!({"type":"string","minLength":1,"maxLength":128,"pattern":"^[a-z0-9._-]+$"});
    result["$id"]=json!("https://github.com/beyond10x/connectors/blob/main/contracts/connector-operation/v0alpha3/connector-operation.schema.json");
    result["title"] = json!("B10x ConnectorOperation v0alpha3 frame");
    result["$comment"] = inherited["$comment"].clone();
    result
}
