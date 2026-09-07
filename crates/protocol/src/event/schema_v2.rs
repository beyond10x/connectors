//! Deterministic projection of the endpoint event subscription contract.
use super::v2;
use serde_json::{json, Value};

/// Produce a closed frame graph; byte budgets and provider parameter declarations are runtime checks.
#[must_use]
pub fn event_v2_schema() -> Value {
    #[derive(schemars::JsonSchema)]
    #[serde(untagged)]
    #[allow(dead_code)]
    enum Frame {
        Request(v2::RequestEnvelope),
        Response(v2::ResponseEnvelope),
    }
    let mut schema = serde_json::to_value(schemars::schema_for!(Frame)).expect("schema serializes");
    let defs = schema["$defs"].as_object_mut().expect("typed definitions");
    defs.insert(
        "OwnerContext".into(),
        crate::operation::schema_v3::operation_v3_schema()["$defs"]["OwnerContext"].clone(),
    );
    for name in ["RequestEnvelope", "ResponseEnvelope"] {
        defs[name]["properties"]["protocol"] = json!({"const":v2::CONTRACT});
        defs[name]["properties"]["request_id"] = reference(128);
    }
    for (name, fields) in [
        ("SubscribeRequest", vec!["endpoint_ref", "channel_binding"]),
        ("UnsubscribeRequest", vec!["subscription_ref"]),
        ("ReceiveRequest", vec!["channel_ref"]),
        ("ReplayRequest", vec!["event_ref"]),
        (
            "ChannelSummary",
            vec![
                "channel_ref",
                "connection_ref",
                "integration_ref",
                "binding_ref",
            ],
        ),
        (
            "DataEvent",
            vec![
                "event_ref",
                "channel_ref",
                "connection_ref",
                "integration_ref",
                "event_type",
            ],
        ),
    ] {
        for field in fields {
            defs[name]["properties"][field] = reference(512);
        }
    }
    defs["SubscribeRequest"]["properties"]["parameters"]["maxProperties"] =
        json!(v2::MAX_PARAMETERS);
    defs["SubscribeRequest"]["properties"]["parameters"]["propertyNames"] = reference(128);
    defs["SearchRequest"]["properties"]["query"] = json!({"type":"string","maxLength":512});
    defs["SearchRequest"]["properties"]["limit"] =
        json!({"type":"integer","minimum":1,"maximum":super::MAX_SEARCH_RESULTS});
    defs["ReceiveRequest"]["properties"]["limit"] =
        json!({"type":"integer","minimum":1,"maximum":super::MAX_RECEIVE_RESULTS});
    defs["ReceiveRequest"]["properties"]["wait_ms"] =
        json!({"type":"integer","minimum":0,"maximum":super::MAX_WAIT_MS});
    defs["ReceiveRequest"]["properties"]["after"] = json!({"anyOf":[{"type":"string","minLength":1,"maxLength":128,"pattern":"^[0-9]+$"},{"type":"null"}]});
    defs["ChannelSummary"]["properties"]["events"] =
        json!({"type":"array","minItems":1,"maxItems":64,"items":reference(512)});
    defs["EventError"]["properties"]["message"] =
        json!({"type":"string","minLength":1,"maxLength":4096});
    for variant in defs["EventResult"]["oneOf"]
        .as_array_mut()
        .expect("tagged results")
    {
        let name = variant["properties"]["result"]["const"]
            .as_str()
            .unwrap_or("")
            .to_owned();
        if name == "replay" {
            continue;
        }
        let value = &mut variant["properties"]["value"]["properties"];
        match name.as_str() {
            "subscribe" | "unsubscribe" => value["subscription_ref"] = reference(512),
            "search" => value["channels"]["maxItems"] = json!(super::MAX_SEARCH_RESULTS),
            "receive" => {
                value["events"]["maxItems"] = json!(super::MAX_RECEIVE_RESULTS);
                value["next"] = json!({"type":"string","minLength":1,"pattern":"^[0-9]+$"});
            }
            _ => {}
        }
    }
    defs["ResponseEnvelope"]["oneOf"] = json!([
        {"required":["response"],"properties":{"status":{"const":"ok"},"response":{"$ref":"#/$defs/EventResult"},"error":{"type":"null"}}},
        {"required":["error"],"properties":{"status":{"const":"error"},"error":{"$ref":"#/$defs/EventError"},"response":{"type":"null"}}}
    ]);
    schema["$id"] = json!("https://github.com/beyond10x/connectors/blob/main/contracts/connector-event/v0alpha2/connector-event.schema.json");
    schema["title"] = json!("B10x ConnectorEvent v0alpha2 frame");
    schema["$comment"] = json!("UTF-8 byte budgets, exact provider channel parameters, principal ownership and current subscription grants are additional Rust reader/runtime checks.");
    schema
}
fn reference(max: usize) -> Value {
    json!({"type":"string","minLength":1,"maxLength":max,"pattern":"^[^\\u0000-\\u0020\\u007f]+$"})
}
