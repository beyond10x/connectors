//! **The model-facing contract projection** — the caller-facing symbols, the error-envelope-
//! extended description and the lowered `input_schema` the canonical document stores per
//! operation (S-001; predecessor C-552).
//!
//! The predecessor computed all three off its emitter's own lowering, so the document stored what
//! the `ToolSpec` projection would have derived from the emitted Flux. This workspace has no
//! emitter — the engine is fenced out (`tests/main/engine_free.rs`) — so the lowering is restated
//! here as the small, closed function it always was, and held to the predecessor byte for byte:
//! the one-time S-001 differential compared every shipped operation's stored contract against the
//! engine-derived values the predecessor's C-552 build produced (835 operations, 1518 symbols).
//!
//! # The type narrowing, and what it cannot hold
//!
//! The contract's parameter types are deliberately small: `string`, `number`, `boolean`, a
//! homogeneous `array`, and the unconstrained `{}`. The predecessor's engine could express nothing
//! richer in a declared op, and the shipped documents froze that surface. So the mapping is a
//! *narrowing*, and the interesting part is the edge:
//!
//! | vendor schema | contract type |
//! |---|---|
//! | `{"type": "string"}` | `{"type": "string"}` |
//! | `{"type": "integer"}`, `{"type": "number"}` | `{"type": "number"}` |
//! | `{"type": "boolean"}` | `{"type": "boolean"}` |
//! | `{"type": "array", "items": S}` | `{"type": "array", "items": map(S)}` |
//! | **anything else** | **`{}`** |
//!
//! Everything else — unions (`oneOf`, `type: ["string","null"]`), objects, `$ref`, an absent
//! `type`, every constraint keyword — lands on `{}`, the top type, so the failure mode is a
//! missing check and never a false rejection. **Nothing is lost**: the vendor's full schema stays
//! on the document's `params[].schema`, verbatim; this is the *contract* projection of it.

use std::collections::BTreeMap;

use anyhow::{anyhow, bail, Result};
use serde_json::{json, Map, Value};

use connector_spec::names::Symbols;
use connector_spec::{Operation, Param, FREE_FORM_BODY};

/// The stored contract projection for one operation: the symbols the document writes beside each
/// parameter, and the `contract` object it stores on the operation.
pub struct Contract {
    /// Caller-facing name → symbol, for every **declared** parameter. A `const`-pinned body field
    /// is deliberately absent: it is sent but never declared, exactly as the document omits it —
    /// but its symbol stays **reserved**, so a later parameter whose name normalizes onto it
    /// shifts past it instead of reclaiming it (the predecessor's ADJACENT-2 trap, C-538/C-552).
    pub symbols: BTreeMap<String, String>,
    /// The error-envelope-extended description a model receives — not the operation's one-line
    /// `description` summary.
    pub description: String,
    /// The lowered, caller-typed input schema, keyed by symbol, every declared parameter required.
    pub input_schema: Value,
}

/// Compute the [`Contract`] for one operation.
///
/// Symbols are allocated in the request-position order the shipped catalogue froze — path, query,
/// header, body (a `const`-pinned body field allocates and is then omitted), the free-form body
/// under [`FREE_FORM_BODY`], and finally the constant headers, which never reach the declared list
/// but allocate last so adding one can never rename a symbol that already travelled.
///
/// # Errors
///
/// A parameter name the allocator refuses (empty, or brace-bearing), or an operation declaring
/// both named body fields and a free-form body — which the loader already refuses, restated here
/// so an IR assembled in memory cannot slip past it.
pub fn contract_of(operation: &Operation) -> Result<Contract> {
    let mut allocator = Symbols::new();
    let mut symbols = BTreeMap::new();
    // (symbol, vendor schema) in declaration order — the input schema's property source.
    let mut declared: Vec<(String, &Value)> = Vec::new();

    let mut allocate =
        |param: &Param| -> Result<String> { Ok(allocator.allocate(&operation.id, &param.name)?) };

    for group in [
        &operation.params.path,
        &operation.params.query,
        &operation.params.header,
    ] {
        for param in group {
            let symbol = allocate(param)?;
            declared.push((symbol.clone(), &param.schema));
            symbols.insert(param.name.clone(), symbol);
        }
    }
    for param in &operation.params.body {
        // Allocated for every body field, `const`-pinned ones included, so the reservation holds;
        // only the declared ones reach the map and the schema.
        let symbol = allocate(param)?;
        if constant(param).is_none() {
            declared.push((symbol.clone(), &param.schema));
            symbols.insert(param.name.clone(), symbol);
        }
    }

    if operation.params.body_schema.is_some() && !operation.params.body.is_empty() {
        bail!(
            "operation `{}` declares both named body fields and a free-form body",
            operation.id
        );
    }
    if let Some(schema) = &operation.params.body_schema {
        let symbol = allocator
            .allocate(&operation.id, FREE_FORM_BODY)
            .map_err(|error| anyhow!(error))?;
        declared.push((symbol.clone(), schema));
        symbols.insert(FREE_FORM_BODY.to_string(), symbol);
    }

    // Constant headers allocate last, from the same allocator, exactly as the predecessor's
    // request assembly did: they are not parameters and never reach the declared list, but a
    // parameter must never lose its symbol to one.
    for name in operation.params.const_headers.keys() {
        allocator
            .allocate(&operation.id, name)
            .map_err(|error| anyhow!(error))?;
    }

    let mut properties = Map::new();
    let mut required: Vec<Value> = Vec::new();
    for (symbol, schema) in &declared {
        properties.insert(symbol.clone(), lowered(schema));
        required.push(Value::String(symbol.clone()));
    }

    Ok(Contract {
        symbols,
        description: description(operation),
        input_schema: json!({
            "type": "object",
            "properties": Value::Object(properties),
            "required": Value::Array(required),
        }),
    })
}

/// The operation's description, extended with the vendor's error envelope when it declares one.
///
/// The envelope lands in prose because the response travels as one flat value: the contract tells
/// the model where the vendor's error message is, rather than pretending a non-2xx response is a
/// failure. Ported verbatim from the predecessor (C-552); the shipped catalogue froze the wording.
fn description(operation: &Operation) -> String {
    let mut out = operation.description.clone();
    let Some(envelope) = &operation.error_envelope else {
        return out;
    };
    if !out.is_empty() && !out.ends_with(['.', '!', '?']) {
        out.push('.');
    }
    if !out.is_empty() {
        out.push(' ');
    }
    out.push_str(
        "A non-2xx response is returned as data, not a failure: the vendor's error message is at `",
    );
    out.push_str(&envelope.message_pointer);
    out.push('`');
    if let Some(code) = &envelope.code_pointer {
        out.push_str(", its error code at `");
        out.push_str(code);
        out.push('`');
    }
    out.push_str(" in the response body.");
    out
}

/// The contract type for a parameter's vendor schema — the module-level narrowing table.
fn lowered(schema: &Value) -> Value {
    // A `type` that is not a single string — absent, or the `["string","null"]` union form — has
    // no contract spelling and takes the `{}` fallback.
    let Some(kind) = schema.get("type").and_then(|t| t.as_str()) else {
        return json!({});
    };
    match kind {
        "string" => json!({ "type": "string" }),
        "integer" | "number" => json!({ "type": "number" }),
        "boolean" => json!({ "type": "boolean" }),
        // An array with no declared `items` is a list of unknowns, not a non-list.
        "array" => json!({
            "type": "array",
            "items": schema.get("items").map(lowered).unwrap_or(json!({})),
        }),
        _ => json!({}),
    }
}

/// The value a body field is pinned to, when its schema pins one — JSON Schema's `const` read as
/// the declaration it already is (`document.rs` reads the same fact through its own copy).
fn constant(param: &Param) -> Option<&Value> {
    param.schema.get("const")
}

#[cfg(test)]
mod tests {
    use super::*;
    use connector_spec::{
        ErrorEnvelope, HttpMethod, Idempotency, OperationDirection, ParamSet, Risk,
    };

    fn operation(params: ParamSet) -> Operation {
        Operation {
            id: "vendor-thing-get".to_string(),
            service: connector_spec::DEFAULT_SERVICE.to_string(),
            method: HttpMethod::Get,
            direction: OperationDirection::Read,
            path: "/thing".to_string(),
            description: "Get a thing.".to_string(),
            risk: Risk::Low,
            idempotency: Idempotency::Idempotent,
            effects: vec![
                connector_spec::HostEffect::Read,
                connector_spec::HostEffect::Network,
            ],
            interaction_shape: connector_spec::InteractionShape::Unary,
            protocol_driver: connector_spec::ProtocolDriver::HttpV1,
            placement_requirement: connector_spec::PlacementRequirement::ConnectorsDeployment,
            implementation_form: connector_spec::ImplementationForm::BuiltIn,
            required_capabilities: vec![connector_spec::RequiredCapability::PublicNetwork],
            semantic_effects: Vec::new(),
            repeatable_because: None,
            expose: true,
            auth: None,
            params,
            response_schema: None,
            credential_response: Vec::new(),
            produces_credential: None,
            pagination: None,

            rate_limit: None,

            error_envelope: None,
        }
    }

    fn param(name: &str, schema: Value) -> Param {
        Param {
            name: name.to_string(),
            wire: None,
            description: String::new(),
            required: true,
            schema,
        }
    }

    #[test]
    fn scalars_map_across_and_constraints_are_dropped() {
        assert_eq!(
            lowered(&json!({"type": "string", "maxLength": 8})),
            json!({"type": "string"})
        );
        assert_eq!(
            lowered(&json!({"type": "integer", "format": "int64"})),
            json!({"type": "number"})
        );
        assert_eq!(
            lowered(&json!({"type": "number"})),
            json!({"type": "number"})
        );
        assert_eq!(
            lowered(&json!({"type": "boolean"})),
            json!({"type": "boolean"})
        );
    }

    #[test]
    fn arrays_carry_their_item_type_and_the_fallback_reaches_inside() {
        assert_eq!(
            lowered(&json!({"type": "array", "items": {"type": "string"}})),
            json!({"type": "array", "items": {"type": "string"}})
        );
        assert_eq!(
            lowered(&json!({"type": "array"})),
            json!({"type": "array", "items": {}})
        );
        assert_eq!(
            lowered(&json!({"type": "array", "items": {"oneOf": []}})),
            json!({"type": "array", "items": {}})
        );
    }

    #[test]
    fn shapes_the_contract_cannot_express_fall_back_to_the_top_type() {
        for schema in [
            json!({"type": "object"}),
            json!({"oneOf": [{"type": "string"}]}),
            json!({"type": ["string", "null"]}),
            json!({"$ref": "#/x"}),
            json!({}),
        ] {
            assert_eq!(lowered(&schema), json!({}));
        }
    }

    /// **The ADJACENT-2 trap** (predecessor C-538, closed by C-552): a `const`-pinned body field
    /// reserves its symbol, so a later declared field whose name normalizes onto it must shift
    /// past it — while the pinned field itself stays out of the declared list.
    #[test]
    fn a_const_pinned_body_field_reserves_the_symbol_a_later_field_must_shift_past() {
        let contract = contract_of(&operation(ParamSet {
            body: vec![
                param("a-b", json!({"type": "string", "const": "pinned"})),
                param("a_b", json!({"type": "string"})),
            ],
            ..ParamSet::default()
        }))
        .unwrap();
        assert_eq!(contract.symbols.get("a_b").unwrap(), "a_b_2");
        assert!(!contract.symbols.contains_key("a-b"));
        assert_eq!(
            contract.input_schema,
            json!({
                "type": "object",
                "properties": { "a_b_2": { "type": "string" } },
                "required": ["a_b_2"],
            })
        );
    }

    #[test]
    fn the_description_extends_with_the_error_envelope() {
        let mut operation = operation(ParamSet::default());
        operation.error_envelope = Some(ErrorEnvelope {
            message_pointer: "/error/message".to_string(),
            code_pointer: Some("/error/type".to_string()),
        });
        assert_eq!(
            description(&operation),
            "Get a thing. A non-2xx response is returned as data, not a failure: the vendor's \
             error message is at `/error/message`, its error code at `/error/type` in the \
             response body."
        );
    }

    #[test]
    fn an_operation_with_no_parameters_states_the_empty_object() {
        let contract = contract_of(&operation(ParamSet::default())).unwrap();
        assert_eq!(
            contract.input_schema,
            json!({"type": "object", "properties": {}, "required": []})
        );
    }
}
