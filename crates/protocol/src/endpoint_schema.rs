//! Deterministic schema for credential-free endpoint discovery and binding.

use crate::endpoint;
use serde_json::{json, Value};

/// Project the new endpoint frame graph and its shared context and bounded response invariants.
#[must_use]
pub fn endpoint_schema() -> Value {
    #[derive(schemars::JsonSchema)]
    #[serde(untagged)]
    #[allow(dead_code)]
    enum Frame {
        Request(endpoint::RequestEnvelope),
        Response(endpoint::ResponseEnvelope),
    }
    let mut result = serde_json::to_value(schemars::schema_for!(Frame)).expect("schema serializes");
    let inherited = crate::operation::schema_v3::operation_v3_schema();
    let defs = result["$defs"].as_object_mut().expect("typed definitions");
    defs.insert(
        "OwnerContext".into(),
        inherited["$defs"]["OwnerContext"].clone(),
    );
    for envelope in ["RequestEnvelope", "ResponseEnvelope"] {
        defs[envelope]["properties"]["protocol"] = json!({"const":endpoint::CONTRACT});
        defs[envelope]["properties"]["request_id"] = reference(128);
    }
    for name in ["ShowRequest", "BindRequest"] {
        defs[name]["properties"]["endpoint_ref"] = reference(512);
    }
    for name in ["ListRequest", "RefreshRequest"] {
        defs[name]["properties"]["source_ref"] = json!({"anyOf":[reference(512),{"type":"null"}]});
    }
    defs["ListRequest"]["properties"]["query"] = json!({"type":"string","maxLength":512});
    defs["ListRequest"]["properties"]["limit"] =
        json!({"type":"integer","minimum":1,"maximum":endpoint::MAX_RESULTS});
    defs["ListRequest"]["properties"]["cursor"] =
        json!({"anyOf":[reference(4096),{"type":"null"}]});
    for field in [
        "endpoint_ref",
        "source_ref",
        "resource_kind",
        "resource_name",
        "resource_uid",
        "interface",
    ] {
        defs["Endpoint"]["properties"][field] = reference(512);
    }
    for field in ["namespace", "port_name"] {
        defs["Endpoint"]["properties"][field] = json!({"anyOf":[reference(512),{"type":"null"}]});
    }
    defs["Endpoint"]["properties"]["port"] =
        json!({"anyOf":[{"type":"integer","minimum":1,"maximum":65535},{"type":"null"}]});
    let provider =
        json!({"type":"string","minLength":1,"maxLength":128,"pattern":"^[a-z0-9._-]+$"});
    defs["Endpoint"]["properties"]["provider"] =
        json!({"anyOf":[provider.clone(),{"type":"null"}]});
    defs["EndpointBinding"]["properties"]["provider"] = provider;
    defs["EndpointError"]["properties"]["message"] =
        json!({"type":"string","minLength":1,"maxLength":4096,"pattern":"^[^\\x00-\\x1f\\x7f]*$"});
    defs["ResponseEnvelope"]["oneOf"] = json!([
        {"required":["response"],"properties":{"status":{"const":"ok"},"response":{"$ref":"#/$defs/EndpointResult"},"error":{"type":"null"}}},
        {"required":["error"],"properties":{"status":{"const":"error"},"error":{"$ref":"#/$defs/EndpointError"},"response":{"type":"null"}}}
    ]);
    result["$id"] = json!("https://github.com/beyond10x/connectors/blob/main/contracts/connector-endpoint/v0alpha1/connector-endpoint.schema.json");
    result["title"] = json!("B10x ConnectorEndpoint v0alpha1 frame");
    result["$comment"] = json!("Serialized UTF-8 byte budgets, exact source policy, route and credential validation are additional Rust reader and runtime checks. Inventory is never an invocation Grant.");
    result
}

fn reference(maximum: usize) -> Value {
    json!({"type":"string","minLength":1,"maxLength":maximum,"pattern":"^[!-~]+$"})
}
