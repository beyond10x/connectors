//! Complete typed structural projection of ordinary and bound Connection v2 frames.
use crate::connection_v2;
use serde_json::{json, Map, Value};

/// Draft 2020-12 schema. Instance equality, UTF-8 byte budgets and legacy URL parsing
/// remain explicitly documented reader checks.
#[must_use]
pub fn connection_v2_schema() -> Value {
    #[derive(schemars::JsonSchema)]
    #[serde(untagged)]
    #[allow(dead_code)]
    enum Frame {
        Request(connection_v2::RequestEnvelope),
        Response(connection_v2::ResponseEnvelope),
    }
    let mut schema = serde_json::to_value(schemars::schema_for!(Frame)).expect("schema serializes");
    schema["$id"]=json!("https://github.com/beyond10x/connectors/blob/main/contracts/connector-connection/v0alpha2/connector-connection.schema.json");
    schema["title"] = json!("B10x ConnectorConnection v0alpha2 frame");
    schema["$comment"]=json!("The reader additionally enforces serialized UTF-8 input/frame/response byte budgets, string byte budgets, equality of independently supplied target/session/expiry values (including mediated-route self-reference refusal), and the frozen v1 WHATWG browser URL parser's route/capability grammar. Schema length counts code points; Draft 2020-12 has no instance-data equality operator. See the bundle README and independent schema/reader vectors.");
    let defs = schema["$defs"].as_object_mut().expect("typed definitions");
    // All ordinary references use the deployed Connection grammar, including non-ASCII
    // characters other than the expressly forbidden ASCII controls and space.
    for (_, definition) in defs.iter_mut() {
        if let Some(properties) = definition
            .get_mut("properties")
            .and_then(Value::as_object_mut)
        {
            for (name, property) in properties {
                if name.ends_with("_ref") {
                    let constraint = if name == "operation_ref" {
                        operation_reference()
                    } else {
                        reference(512)
                    };
                    replace_preserving_null(property, constraint);
                }
            }
        }
    }
    for name in ["RequestEnvelope", "ResponseEnvelope"] {
        property(
            defs,
            name,
            "protocol",
            json!({"const":connection_v2::CONTRACT}),
        );
        property(defs, name, "request_id", reference(128));
    }
    for variant in defs.get_mut("ConnectionRoute").expect("route")["oneOf"]
        .as_array_mut()
        .expect("tagged route")
    {
        if let Some(parent) = variant["properties"].get_mut("parent_connection_ref") {
            *parent = reference(512);
        }
    }
    for field in ["tenant_id", "agent_id", "authority_snapshot_id"] {
        property(defs, "OwnerContext", field, reference(512));
    }
    property(defs, "OwnerContext", "agent_revision", unsigned(1));
    property(defs, "OwnerContext", "authority_snapshot_sha256", digest());
    for name in [
        "CandidateSearchRequest",
        "SearchRequest",
        "ObservationSearchRequest",
    ] {
        property(
            defs,
            name,
            "query",
            json!({"type":"string","maxLength":512}),
        );
        property(
            defs,
            name,
            "limit",
            json!({"type":"integer","minimum":1,"maximum":64}),
        );
    }
    for name in [
        "CandidateActivateRequest",
        "ConnectSessionCreateRequest",
        "ConnectionSummary",
        "ConnectionDescription",
    ] {
        property(defs, name, "label", nonblank(256));
    }
    for name in [
        "ConnectSessionCreateRequest",
        "ConnectionSummary",
        "ConnectionDescription",
        "BoundRemediationStatus",
    ] {
        let field = &mut defs.get_mut(name).expect("profile type")["properties"]["auth_profile"];
        replace_preserving_null(field, profile());
    }
    for name in ["ConnectionSummary", "ConnectionDescription"] {
        // The deployed reader bounds cardinality but does not reject repeated initiators.
        defs.get_mut(name).expect("summary")["properties"]["initiation"]["minItems"] = json!(1);
        defs.get_mut(name).expect("summary")["properties"]["initiation"]["maxItems"] = json!(2);
        defs.get_mut(name).expect("summary")["allOf"] = json!([
            {"if":{"required":["scope"],"properties":{"scope":{"not":{"type":"null"}}}},
             "then":{"required":["actor"],"properties":{"actor":{"not":{"type":"null"}}}},
             "else":{"properties":{"actor":{"type":"null"}}}},
            {"if":{"required":["actor"],"properties":{"actor":{"not":{"type":"null"}}}},
             "then":{"required":["scope"],"properties":{"scope":{"not":{"type":"null"}}}},
             "else":{"properties":{"scope":{"type":"null"}}}}
        ]);
    }
    defs.get_mut("ConnectionDescription").expect("description")["properties"]["channels"]
        ["maxItems"] = json!(64);
    property(
        defs,
        "ChannelSummary",
        "events",
        json!({"type":"array","maxItems":64,"items":reference(512)}),
    );
    for name in ["ConnectionCandidateSummary", "DiscoveryObservationSummary"] {
        property(defs, name, "title", nonblank(256));
        property(defs, name, "evidence_sha256", digest());
    }
    property(
        defs,
        "DiscoveryObservationSummary",
        "evidence_generation",
        unsigned(1),
    );
    property(
        defs,
        "DiscoveryObservationSummary",
        "observed_type",
        json!({"type":"string","minLength":1,"maxLength":128,"pattern":"^[^\\u0000-\\u001f\\u007f]+$"}),
    );
    defs.get_mut("ConnectionCandidateSummary")
        .expect("candidate")["oneOf"] = json!([
        state_fields("detected", &[], &["connection_ref"]),
        state_fields("activated", &["connection_ref"], &[])
    ]);
    defs.get_mut("DiscoveryObservationSummary")
        .expect("observation")["oneOf"] = json!([
        state_fields("observed", &["target_provider_ref"], &["connection_ref"]),
        state_fields(
            "unsupported",
            &[],
            &["target_provider_ref", "connection_ref"]
        ),
        state_fields(
            "materialized",
            &["target_provider_ref", "connection_ref"],
            &[]
        ),
        state_fields("withdrawn", &[], &["connection_ref"])
    ]);
    property(
        defs,
        "ConnectionError",
        "message",
        json!({"type":"string","minLength":1,"maxLength":4096}),
    );
    let response = defs.get_mut("ResponseEnvelope").expect("response");
    response["oneOf"] = json!([
        {"properties":{"status":{"const":"ok"},"response":{"not":{"type":"null"}},"error":{"type":"null"}},"required":["response"]},
        {"properties":{"status":{"const":"error"},"error":{"not":{"type":"null"}},"response":{"type":"null"}},"required":["error"]}
    ]);
    for variant in defs.get_mut("ConnectionResult").expect("results")["oneOf"]
        .as_array_mut()
        .expect("tagged results")
    {
        if let Some(properties) = variant
            .get_mut("properties")
            .and_then(|properties| properties.get_mut("value"))
            .and_then(|value| value.get_mut("properties"))
            .and_then(Value::as_object_mut)
        {
            for field in ["candidates", "connections", "observations"] {
                if let Some(array) = properties.get_mut(field) {
                    array["maxItems"] = json!(64);
                }
            }
        }
    }
    property(
        defs,
        "ConnectSessionStatus",
        "expires_at_unix_ms",
        unsigned(0),
    );
    let session = defs.get_mut("ConnectSessionStatus").expect("session");
    replace_preserving_null(
        &mut session["properties"]["completion_endpoint"],
        json!({"type":"string","minLength":1,"maxLength":4096,"pattern":"^[^\\n]*$"}),
    );
    replace_preserving_null(
        &mut session["properties"]["browser_completion_url"],
        json!({"type":"string","minLength":1,"maxLength":4096,"pattern":"^[^\\r\\n]*$","$comment":"Legacy WHATWG parse, route and fragment-capability checks are enforced by the unchanged v1 reader."}),
    );
    let mut pending = state_fields("pending", &[], &["connection_ref"]);
    pending["anyOf"] = json!([
        {"required":["completion_endpoint"],"properties":{"completion_endpoint":{"not":{"type":"null"}}}},
        {"required":["browser_completion_url"],"properties":{"browser_completion_url":{"not":{"type":"null"}}}}
    ]);
    session["oneOf"] = json!([
        pending,
        state_fields(
            "completed",
            &["connection_ref"],
            &["completion_endpoint", "browser_completion_url"]
        ),
        state_fields(
            "expired",
            &[],
            &[
                "connection_ref",
                "completion_endpoint",
                "browser_completion_url"
            ]
        ),
        state_fields(
            "failed",
            &[],
            &[
                "connection_ref",
                "completion_endpoint",
                "browser_completion_url"
            ]
        )
    ]);
    property(
        defs,
        "BoundRemediationStatus",
        "expires_at_unix_ms",
        unsigned(0),
    );
    let bound = defs
        .get_mut("BoundRemediationStatus")
        .expect("bound status");
    bound["oneOf"] = json!([
        bound_state("pending", "pending"),
        bound_state("ready", "completed"),
        bound_state("consumed", "completed"),
        bound_state("expired", "expired"),
        bound_state("expired", "completed"),
        bound_state("failed", "failed")
    ]);
    schema
}

fn reference(max: usize) -> Value {
    json!({"type":"string","minLength":1,"maxLength":max,"pattern":"^[^\\u0000-\\u0020\\u007f]+$"})
}
fn operation_reference() -> Value {
    json!({"type":"string","minLength":1,"maxLength":512,"pattern":"^[!-~]+$"})
}
fn profile() -> Value {
    json!({"type":"string","minLength":1,"maxLength":128,"pattern":"^[a-z0-9._-]+$"})
}
fn digest() -> Value {
    json!({"type":"string","pattern":"^[0-9a-fA-F]{64}$"})
}
fn unsigned(min: u64) -> Value {
    json!({"type":"integer","minimum":min,"maximum":u64::MAX})
}
fn nonblank(max: usize) -> Value {
    json!({"type":"string","minLength":1,"maxLength":max,"pattern":"[^\\u0009-\\u000d\\u0020\\u0085\\u00a0\\u1680\\u2000-\\u200a\\u2028\\u2029\\u202f\\u205f\\u3000]"})
}
fn property(defs: &mut Map<String, Value>, name: &str, field: &str, value: Value) {
    defs.get_mut(name).expect("typed definition")["properties"][field] = value;
}
fn admits_null(value: &Value) -> bool {
    value.get("type").is_some_and(|kind| {
        kind == "null"
            || kind
                .as_array()
                .is_some_and(|types| types.iter().any(|t| t == "null"))
    }) || value
        .get("anyOf")
        .and_then(Value::as_array)
        .is_some_and(|choices| choices.iter().any(admits_null))
}
fn replace_preserving_null(property: &mut Value, value: Value) {
    *property = if admits_null(property) {
        json!({"anyOf":[value,{"type":"null"}]})
    } else {
        value
    };
}
fn state_fields(state: &str, present: &[&str], absent: &[&str]) -> Value {
    let mut value = json!({"properties":{"state":{"const":state}}});
    if !present.is_empty() {
        value["required"] = json!(present);
    }
    for field in present {
        value["properties"][*field] = json!({"not":{"type":"null"}});
    }
    for field in absent {
        value["properties"][*field] = json!({"type":"null"});
    }
    value
}
fn bound_state(resume: &str, state: &str) -> Value {
    json!({"properties":{"resume_state":{"const":resume},"session_state":{"const":state},"session":{"properties":{"state":{"const":state}}}}})
}
