//! Deterministic Draft 2020-12 projection of the complete v0alpha2 Rust frame shapes.
//!
//! The schema asserts structural, numeric and conditional presence constraints. JSON Schema cannot
//! compare instance properties arithmetically: exact interval derivation and serialized UTF-8 byte
//! budgets remain reader checks. The conformance vectors identify those limits explicitly.

use super::wire;
use serde::Serialize;
use serde_json::{json, Value};

#[allow(dead_code)]
#[derive(Serialize, schemars::JsonSchema)]
#[serde(untagged)]
enum Frame {
    Request(wire::RequestEnvelope),
    Response(wire::ResponseEnvelope),
}

/// Render the supported wire schema directly from the typed request and response graph.
#[must_use]
pub fn operation_v2_schema() -> Value {
    let mut schema =
        serde_json::to_value(schemars::schema_for!(Frame)).expect("the typed schema is JSON");
    schema["$id"] = json!(
        "https://b10x.dev/contracts/connector-operation/v0alpha2/connector-operation.schema.json"
    );
    schema["title"] = json!("B10x ConnectorOperation v0alpha2 frame");
    schema["$comment"] = json!("Exact suggested_interval_ms arithmetic and serialized UTF-8 byte budgets are additional Rust reader checks. No provider request or response schema is transformed by this projection.");
    let defs = schema["$defs"]
        .as_object_mut()
        .expect("frame definitions exist");
    for name in ["RequestEnvelope", "ResponseEnvelope"] {
        defs[name]["properties"]["protocol"] = json!({"const": wire::CONTRACT});
        defs[name]["properties"]["request_id"] = reference(128);
    }
    for field in ["tenant_id", "agent_id", "authority_snapshot_id"] {
        defs["OwnerContext"]["properties"][field] = reference(256);
    }
    defs["OwnerContext"]["properties"]["agent_revision"] = unsigned(1, u64::MAX);
    defs["OwnerContext"]["properties"]["authority_snapshot_sha256"] =
        json!({"type":"string", "pattern":"^[0-9a-f]{64}$"});
    defs["SearchRequest"]["properties"]["query"] = json!({"type":"string", "maxLength":512});
    defs["SearchRequest"]["properties"]["limit"] = unsigned(1, u64::from(wire::MAX_SEARCH_RESULTS));
    for (name, fields) in [
        ("DescribeRequest", &["operation_ref"][..]),
        (
            "InvokeRequest",
            &["operation_ref", "connection_ref", "description_ref"][..],
        ),
        ("SessionRequest", &["execution_ref"][..]),
        ("SessionTerminateRequest", &["execution_ref"][..]),
        ("SessionSignalRequest", &["execution_ref"][..]),
        ("OperationSummary", &["operation_ref"][..]),
        (
            "OperationDescription",
            &["operation_ref", "description_ref"][..],
        ),
        (
            "InvocationResult",
            &["operation_ref", "connector_audit_ref"][..],
        ),
        (
            "SessionStatus",
            &[
                "execution_ref",
                "operation_ref",
                "connection_ref",
                "connector_audit_ref",
            ][..],
        ),
        ("ConnectionSummary", &["connection_ref"][..]),
    ] {
        for field in fields {
            defs[name]["properties"][*field] = reference(512);
        }
    }
    defs["InvokeRequest"]["properties"]["approval_evidence_ref"] = nullable(reference(512));
    defs["InvocationResult"]["properties"]["execution_ref"] = nullable(reference(512));
    defs["ConnectionSummary"]["properties"]["provider"] = reference(128);
    defs["ConnectionSummary"]["properties"]["label"] = text(1, 1024);
    defs["ConnectionSummary"]["properties"]["audiences"] = json!({
        "type":"array", "maxItems":wire::MAX_CONNECTION_AUDIENCES, "uniqueItems":true,
        "items":reference(64)
    });
    for name in ["OperationSummary", "OperationDescription"] {
        defs[name]["properties"]["title"] = text(1, 1024);
        defs[name]["properties"]["connections"]["maxItems"] = json!(64);
        defs[name]["allOf"] = json!([{
            "if":{"properties":{"effect":{"enum":["mutating","destructive"]}}},
            "then":{"properties":{"approval":{"const":"required"}}}
        }]);
    }
    defs["OperationDescription"]["properties"]["description"] = text(0, 16384);
    for variant in defs["OperationResult"]["oneOf"]
        .as_array_mut()
        .expect("result variants")
    {
        if variant["properties"]["result"]["const"] == "search" {
            variant["properties"]["value"]["properties"]["operations"]["maxItems"] =
                json!(wire::MAX_SEARCH_RESULTS);
        }
    }
    defs["SessionStatus"]["allOf"] = json!([
        {"if":{"properties":{"state":{"enum":["establishing","established","terminating"]}}},
         "then":{"properties":{"termination":{"type":"null"}}}},
        {"if":{"properties":{"state":{"const":"terminated"}}},
         "then":{"required":["termination"],"properties":{"termination":{"enum":["completed","cancelled","revoked","lease_expired","remote_ended","failed"]}}}},
        {"if":{"properties":{"state":{"const":"outcome_unknown"}}},
         "then":{"required":["termination"],"properties":{"termination":{"const":"outcome_unknown"}}}}
    ]);
    defs["OperationError"]["properties"]["message"] = text(1, 4096);
    defs["OperationError"]["properties"]["retry_after_seconds"] = unsigned(0, u64::MAX);
    defs["OperationError"]["allOf"] = json!([{
        "if":{"required":["retry_after_seconds"]},
        "then":{"properties":{"code":{"const":"rate_limited"}}}
    }]);
    defs["ResponseEnvelope"]["oneOf"] = json!([
        {"required":["response"],"properties":{"status":{"const":"ok"},
         "response":{"$ref":"#/$defs/OperationResult"}, "error":{"type":"null"}}},
        {"required":["error"],"properties":{"status":{"const":"error"},
         "error":{"$ref":"#/$defs/OperationError"}, "response":{"type":"null"}}}
    ]);
    for name in ["FixedRateLimit", "PublishedRate"] {
        for field in ["requests", "per_seconds"] {
            defs[name]["properties"][field] = unsigned(1, u64::from(u32::MAX));
        }
    }
    defs["FixedRateLimit"]["properties"]["bucket"] =
        nullable(bounded_text(wire::MAX_RATE_BUCKET_CHARS));
    defs["ConditionalRateLimit"]["properties"]["applies_when"] =
        bounded_text(wire::MAX_APPLICABILITY_CHARS);
    defs["ConditionalRateLimit"]["properties"]["source_url"] = json!({
        "type":"string", "minLength":1,"maxLength":wire::MAX_SOURCE_URL_CHARS,
        "format":"uri",
        "description":"RFC 3986 ASCII URI with literal lowercase https, a nonempty host, no userinfo or fragment, and an optional decimal port from 0 through 65535. Empty ports denote the default; leading zeros and percent-encoded spelling are preserved.",
        "pattern":r"^https://(?:\[[^\]]+\]|[^:/?#@\[\]]+)(?::(?:0*(?:[0-9]{1,4}|[1-5][0-9]{4}|6[0-4][0-9]{3}|65[0-4][0-9]{2}|655[0-2][0-9]|6553[0-5]))?)?(?:[/?][^#]*)?$"
    });
    defs["ConditionalRateAdvice"]["properties"]["suggested_interval_ms"] =
        nullable(unsigned(1, u64::from(u32::MAX) * 1000));
    defs["ConditionalRateAdvice"]["allOf"] = json!([{
        "if":{"properties":{"declaration":{"required":["rate"],"properties":{"rate":{"type":"object"}}}}},
        "then":{"required":["suggested_interval_ms"],"properties":{"suggested_interval_ms":{"type":"integer"}}},
        "else":{"properties":{"suggested_interval_ms":{"type":"null"}}}
    }]);
    defs["OperationRateAdvice"]["properties"]["alternatives"]["maxItems"] =
        json!(wire::MAX_RATE_ALTERNATIVES);
    defs["OperationRateAdvice"]["anyOf"] = json!([
        {"required":["fixed"],"properties":{"fixed":{"type":"object"}}},
        {"properties":{"alternatives":{"minItems":1}}}
    ]);
    // Domain-owned ChannelSignal validates the DTMF alphabet and length in addition to its tag.
    let signal = defs
        .get_mut("ChannelSignal")
        .expect("session signal is part of v2");
    constrain_signal(signal);
    schema
}

fn reference(maximum: usize) -> Value {
    json!({"type":"string", "minLength":1, "maxLength":maximum, "pattern":"^[!-~]+$"})
}
fn text(minimum: usize, maximum: usize) -> Value {
    json!({"type":"string", "minLength":minimum, "maxLength":maximum})
}
fn bounded_text(maximum: usize) -> Value {
    json!({"type":"string", "minLength":1, "maxLength":maximum, "pattern":"^[^\\u0000-\\u001f\\u007f-\\u009f]+$"})
}
fn unsigned(minimum: u64, maximum: u64) -> Value {
    json!({"type":"integer", "minimum":minimum, "maximum":maximum})
}
fn nullable(value: Value) -> Value {
    json!({"anyOf":[value,{"type":"null"}]})
}
fn constrain_signal(value: &mut Value) {
    match value {
        Value::Object(object) => {
            if let Some(properties) = object.get_mut("properties").and_then(Value::as_object_mut) {
                if properties.contains_key("digits") {
                    properties.insert("digits".into(), json!({"type":"string", "minLength":1, "maxLength":32, "pattern":"^[0-9A-D*#]+$"}));
                }
            }
            for child in object.values_mut() {
                constrain_signal(child);
            }
        }
        Value::Array(array) => {
            for child in array {
                constrain_signal(child);
            }
        }
        _ => {}
    }
}
