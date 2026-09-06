//! Deterministic OpenAPI 3.0 Schema Object to JSON Schema constraint translation.
//!
//! Raw vendor schemas remain on the IR. This translates only the supported schema vocabulary for
//! callers; it does not repair upstream schemas or validate the whole OpenAPI document. In
//! particular an array without `items` stays without an item constraint and earns a source
//! diagnostic. See https://spec.openapis.org/oas/v3.0.3.html#schema-object .

use serde_json::{Map, Value};

/// Translate the supported OpenAPI 3.0 schema vocabulary, preserving every stated constraint.
///
/// References must already be resolved without approximation. Unsupported semantics are an
/// importer gap, never an unconstrained replacement. The returned schema uses Draft 2020-12
/// semantics; the caller envelope declares the dialect once.
pub fn openapi30(schema: &Value) -> Result<Value, String> {
    translate(schema, "#", 0)
}

fn translate(schema: &Value, at: &str, depth: usize) -> Result<Value, String> {
    if depth > 128 {
        return Err(format!(
            "schema {at} exceeds the supported schema nesting depth"
        ));
    }
    let object = schema
        .as_object()
        .ok_or_else(|| format!("schema {at} is not an OpenAPI 3.0 Schema Object"))?;
    let mut out = Map::new();
    for (key, value) in object {
        let location = format!("{at}/{}", key.replace('~', "~0").replace('/', "~1"));
        let translated = match key.as_str() {
            "nullable" => {
                if !value.is_boolean() {
                    return Err(format!("schema {location} is not boolean"));
                }
                continue;
            }
            "exclusiveMinimum" | "exclusiveMaximum" => continue,
            "type" => {
                let kind = value
                    .as_str()
                    .ok_or_else(|| format!("schema {location} is not a single OpenAPI type"))?;
                if !matches!(
                    kind,
                    "string" | "integer" | "number" | "boolean" | "array" | "object"
                ) {
                    return Err(format!(
                        "schema {location} has unsupported OpenAPI type {kind:?}"
                    ));
                }
                if object.get("nullable") == Some(&Value::Bool(true)) {
                    serde_json::json!([kind, "null"])
                } else {
                    value.clone()
                }
            }
            "properties" => {
                let properties = value
                    .as_object()
                    .ok_or_else(|| format!("schema {location} is not a property map"))?;
                Value::Object(
                    properties
                        .iter()
                        .map(|(name, child)| {
                            let at = format!(
                                "{location}/{}",
                                name.replace('~', "~0").replace('/', "~1")
                            );
                            translate(child, &at, depth + 1).map(|child| (name.clone(), child))
                        })
                        .collect::<Result<_, _>>()?,
                )
            }
            "items" | "not" => translate(value, &location, depth + 1)?,
            "additionalProperties" if value.is_boolean() => value.clone(),
            "additionalProperties" => translate(value, &location, depth + 1)?,
            "oneOf" | "anyOf" | "allOf" => {
                let branches = value
                    .as_array()
                    .filter(|branches| !branches.is_empty())
                    .ok_or_else(|| format!("schema {location} is not a nonempty schema list"))?;
                Value::Array(
                    branches
                        .iter()
                        .enumerate()
                        .map(|(index, child)| {
                            translate(child, &format!("{location}/{index}"), depth + 1)
                        })
                        .collect::<Result<_, _>>()?,
                )
            }
            // These are the same constraints in OAS3.0 and Draft 2020-12. Never walk default,
            // enum or example values as schemas: a caller's literal `$ref` is just data there.
            "title" | "description" | "default" | "enum" | "format" | "required" | "multipleOf"
            | "maximum" | "minimum" | "maxLength" | "minLength" | "pattern" | "maxItems"
            | "minItems" | "uniqueItems" | "maxProperties" | "minProperties" | "example"
            | "deprecated" | "externalDocs" => value.clone(),
            "readOnly" | "writeOnly" if value == &Value::Bool(false) => value.clone(),
            key if key.starts_with("x-") => value.clone(),
            _ => {
                return Err(format!(
                    "schema {location} uses unsupported OpenAPI 3.0 semantics"
                ))
            }
        };
        out.insert(key.clone(), translated);
    }
    for (exclusive, bound) in [
        ("exclusiveMinimum", "minimum"),
        ("exclusiveMaximum", "maximum"),
    ] {
        match object.get(exclusive) {
            None | Some(Value::Bool(false)) => {}
            Some(Value::Bool(true)) => {
                let value = out
                    .remove(bound)
                    .filter(Value::is_number)
                    .ok_or_else(|| format!("schema {at}/{exclusive} requires a numeric {bound}"))?;
                out.insert(exclusive.to_owned(), value);
            }
            Some(_) => {
                return Err(format!(
                    "schema {at}/{exclusive} is not an OpenAPI 3.0 boolean"
                ))
            }
        }
    }
    Ok(Value::Object(out))
}

/// Source defects that can be preserved without inventing a constraint. This does not certify
/// structural OpenAPI validity and never substitutes `items: {}` for an omitted keyword.
pub fn source_diagnostics(schema: &Value) -> Vec<String> {
    fn walk(schema: &Value, at: &str, found: &mut Vec<String>) {
        let Some(object) = schema.as_object() else {
            return;
        };
        if object.get("type").and_then(Value::as_str) == Some("array")
            && !object.contains_key("items")
        {
            found.push(format!("source schema {at} declares an array without items; OpenAPI 3.0 requires items, but the absent constraint is preserved (source validity is not claimed)"));
        }
        if let Some(properties) = object.get("properties").and_then(Value::as_object) {
            for (name, child) in properties {
                walk(
                    child,
                    &format!(
                        "{at}/properties/{}",
                        name.replace('~', "~0").replace('/', "~1")
                    ),
                    found,
                );
            }
        }
        for key in ["items", "not", "additionalProperties"] {
            if let Some(child) = object.get(key) {
                walk(child, &format!("{at}/{key}"), found);
            }
        }
        for key in ["oneOf", "anyOf", "allOf"] {
            if let Some(children) = object.get(key).and_then(Value::as_array) {
                for (index, child) in children.iter().enumerate() {
                    walk(child, &format!("{at}/{key}/{index}"), found);
                }
            }
        }
    }
    let mut diagnostics = Vec::new();
    walk(schema, "#", &mut diagnostics);
    diagnostics
}
