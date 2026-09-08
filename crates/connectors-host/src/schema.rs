//! Offline schema imports derived from the actual shared configuration types.
use connectors_core::{Error, Result};
use schemars::{JsonSchema, generate::SchemaSettings};
use serde_json::Value;

fn schema<T: JsonSchema>() -> Result<Value> {
    let settings = SchemaSettings::draft2020_12().with(|s| {
        s.inline_subschemas = true;
        s.meta_schema = None;
    });
    serde_json::to_value(settings.into_generator().into_root_schema_for::<T>())
        .map_err(|_| Error::internal())
}

/// Resolve only this closed, versioned library. No filesystem or network fetches.
/// Sibling constraints survive expansion under JSON Schema's intersection semantics.
pub fn expand(value: &mut Value) -> Result<()> {
    if let Some(reference) = value.get("$ref").and_then(Value::as_str)
        && reference.starts_with("urn:connectors:config:")
    {
        let imported = match reference {
            "urn:connectors:config:v1:service" => schema::<crate::server::ServiceConfig>()?,
            "urn:connectors:config:v1:http" => schema::<crate::http::HttpConfig>()?,
            "urn:connectors:config:v1:credential" => schema::<crate::credentials::CredentialRef>()?,
            _ => return Err(Error::invalid("unknown shared configuration schema")),
        };
        value
            .as_object_mut()
            .ok_or_else(Error::internal)?
            .remove("$ref");
        let siblings = std::mem::take(value);
        *value = serde_json::json!({"allOf":[imported, siblings]});
    }
    if let Value::Object(map) = value {
        // Walk schema locations only: a default/example/const is instance data.
        for keyword in [
            "$defs",
            "definitions",
            "properties",
            "patternProperties",
            "dependentSchemas",
            "dependencies",
        ] {
            if let Some(Value::Object(schemas)) = map.get_mut(keyword) {
                for child in schemas.values_mut() {
                    expand(child)?;
                }
            }
        }
        for keyword in ["allOf", "anyOf", "oneOf", "prefixItems"] {
            if let Some(Value::Array(schemas)) = map.get_mut(keyword) {
                for child in schemas {
                    expand(child)?;
                }
            }
        }
        for keyword in [
            "additionalProperties",
            "unevaluatedProperties",
            "items",
            "contains",
            "not",
            "if",
            "then",
            "else",
            "propertyNames",
            "additionalItems",
            "unevaluatedItems",
        ] {
            if let Some(child) = map.get_mut(keyword) {
                if let Value::Array(schemas) = child {
                    for schema in schemas {
                        expand(schema)?;
                    }
                } else {
                    expand(child)?;
                }
            }
        }
    }
    Ok(())
}
