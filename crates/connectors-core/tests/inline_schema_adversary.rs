//! Adversary cases for story:owner-bounded-read-realization-matches-projection.
//!
//! `inline_schema` decides both whether the MCP projection advertises a bounded
//! read and whether the owner serves an implemented one. The cases the unit and
//! the projection carry name `$ref`, `$dynamicRef`, `$anchor` and `$id`, each
//! at most one object deep. Each case here is the only reference, identifier or
//! anchor keyword in its schema, so dropping that keyword from the list, or
//! stopping the walk at an array, turns exactly that case.
use connectors_core::inline_schema;
use serde_json::json;

#[test]
fn every_listed_keyword_alone_makes_a_schema_relocated() {
    for keyword in [
        "$ref",
        "$dynamicRef",
        "$recursiveRef",
        "$id",
        "$anchor",
        "$dynamicAnchor",
        "$recursiveAnchor",
    ] {
        let mut schema = json!({"type":"object","properties":{"value":{"type":"string"}}});
        schema["properties"]["value"][keyword] = json!("#node");
        assert!(!inline_schema(&schema), "{keyword} nested in an object");
        let mut root = json!({"type":"object"});
        root[keyword] = json!("#node");
        assert!(!inline_schema(&root), "{keyword} at the root");
    }
}

#[test]
fn a_reference_inside_an_array_makes_a_schema_relocated() {
    for schema in [
        json!({"allOf":[{"type":"object"},{"$ref":"#/$defs/value"}],
            "$defs":{"value":{"type":"object"}}}),
        json!({"type":"array","prefixItems":[{"type":"string"},{"$recursiveRef":"#"}]}),
        json!({"anyOf":[{"oneOf":[{"type":"null"},{"$dynamicRef":"#node"}]}]}),
    ] {
        assert!(!inline_schema(&schema), "{schema}");
    }
}

#[test]
fn keyword_names_as_values_and_plain_schemas_stay_inline() {
    for schema in [
        json!({"type":"object","properties":{"value":{"type":"string"}}}),
        json!({"type":"string","enum":["$ref","$id","$anchor"]}),
        json!({"allOf":[{"type":"object"},{"required":["value"]}]}),
        json!(true),
        json!({}),
    ] {
        assert!(inline_schema(&schema), "{schema}");
    }
}
