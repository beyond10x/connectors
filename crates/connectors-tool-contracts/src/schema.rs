//! A descriptor operation's JSON Schemas as `tool-contract/1` field lists and named types.
//!
//! The subset the grammar has: `string` (`Bytes` with `contentEncoding: base64`), `integer`,
//! `boolean`, `array` with `items`, `object` with `properties` (a struct), `object` with only an
//! `additionalProperties` schema (`Map<String, V>`), a string `enum` or `const` (an enum type),
//! nullability (`["T", "null"]`, an `anyOf`/`oneOf` of one schema and `{type: null}`, a `null`
//! variant, or a property outside `required`) as `Optional<T>`, and `$ref` into `$defs`. Any other
//! node is refused naming the operation and its schema path.

use crate::{Refusal, RefusalKind};
use serde_json::{Map, Value};
use std::collections::BTreeSet;
use tool_contract::{Field, TypeDecl, TypeKind};

pub(crate) struct Converter<'a> {
    operation: &'a str,
    prefix: String,
    types: &'a mut Vec<TypeDecl>,
}

/// `foo_bar-baz` as `FooBarBaz`.
fn pascal(text: &str) -> String {
    text.split(['_', '-', '.', ' '])
        .filter(|part| !part.is_empty())
        .map(|part| {
            let mut chars = part.chars();
            chars.next().map_or_else(String::new, |first| {
                first.to_ascii_uppercase().to_string() + chars.as_str()
            })
        })
        .collect()
}

fn description(schema: &Value) -> Option<String> {
    schema
        .get("description")
        .and_then(Value::as_str)
        .map(str::to_owned)
}

impl<'a> Converter<'a> {
    pub(crate) fn new(operation: &'a str, name: &str, types: &'a mut Vec<TypeDecl>) -> Self {
        Self {
            operation,
            prefix: name.to_owned(),
            types,
        }
    }

    fn refuse(&self, refusals: &mut Vec<Refusal>, path: &str, detail: &str) {
        refusals.push(Refusal::new(
            RefusalKind::SchemaUnrepresentable,
            self.operation,
            format!("{path}: {detail}"),
        ));
    }

    /// An operation's input or output schema as its field list, less `omit`.
    pub(crate) fn root(
        &mut self,
        schema: &Value,
        side: &str,
        omit: &[String],
        refusals: &mut Vec<Refusal>,
    ) -> Vec<Field> {
        let path = format!("{side}_schema");
        let properties = schema.get("properties").and_then(Value::as_object);
        for field in omit {
            if properties.is_none_or(|p| !p.contains_key(field)) {
                self.refuse(
                    refusals,
                    &format!("{path}/properties/{field}"),
                    "omit names a field the input does not have",
                );
            }
        }
        if schema.get("type").and_then(Value::as_str) != Some("object") && properties.is_none() {
            self.refuse(refusals, &path, "the root is not an object");
            return Vec::new();
        }
        let mut walk = Walk {
            root: schema,
            pending: BTreeSet::new(),
            refusals,
        };
        self.fields(schema, &path, "", omit, &mut walk)
    }

    fn fields(
        &mut self,
        schema: &Value,
        path: &str,
        hint: &str,
        skip: &[String],
        walk: &mut Walk<'_, '_>,
    ) -> Vec<Field> {
        let empty = Map::new();
        let properties = schema
            .get("properties")
            .and_then(Value::as_object)
            .unwrap_or(&empty);
        let required: BTreeSet<&str> = schema
            .get("required")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
            .collect();
        let mut fields = Vec::new();
        for (name, property) in properties {
            if skip.contains(name) {
                continue;
            }
            let at = format!("{path}/properties/{name}");
            let Some(mut ty) = self.ty(property, &at, &format!("{hint}{}", pascal(name)), walk)
            else {
                continue;
            };
            if !required.contains(name.as_str()) && !ty.starts_with("Optional<") {
                ty = format!("Optional<{ty}>");
            }
            fields.push(Field {
                name: name.clone(),
                ty,
                description: description(property),
            });
        }
        fields
    }

    /// Declares `decl`, or reuses an equal declaration of the same name.
    fn declare(&mut self, decl: TypeDecl, path: &str, walk: &mut Walk<'_, '_>) -> Option<String> {
        let name = decl.name.clone();
        match self.types.iter().find(|t| t.name == name) {
            Some(existing) if *existing == decl => Some(name),
            Some(_) => {
                self.refuse(
                    walk.refusals,
                    path,
                    &format!("two different types would be named {name}"),
                );
                None
            }
            None => {
                self.types.push(decl);
                Some(name)
            }
        }
    }

    fn type_name(&self, hint: &str) -> String {
        format!(
            "{}.{}",
            self.prefix,
            if hint.is_empty() { "Value" } else { hint }
        )
    }

    /// The type text of `schema`, declaring the named types it needs.
    fn ty(
        &mut self,
        schema: &Value,
        path: &str,
        hint: &str,
        walk: &mut Walk<'_, '_>,
    ) -> Option<String> {
        let (schema, nullable) = match strip_null(schema) {
            Ok(stripped) => stripped,
            Err(detail) => {
                self.refuse(walk.refusals, path, &detail);
                return None;
            }
        };
        let ty = self.non_null(&schema, path, hint, walk)?;
        Some(if nullable {
            format!("Optional<{ty}>")
        } else {
            ty
        })
    }

    fn non_null(
        &mut self,
        schema: &Value,
        path: &str,
        hint: &str,
        walk: &mut Walk<'_, '_>,
    ) -> Option<String> {
        if let Some(reference) = schema.get("$ref").and_then(Value::as_str) {
            let Some(def) = reference
                .strip_prefix("#/$defs/")
                .or_else(|| reference.strip_prefix("#/definitions/"))
            else {
                self.refuse(walk.refusals, path, "a $ref outside the root's $defs");
                return None;
            };
            let target = walk
                .root
                .get("$defs")
                .or_else(|| walk.root.get("definitions"))
                .and_then(|defs| defs.get(def));
            let Some(target) = target else {
                self.refuse(
                    walk.refusals,
                    path,
                    &format!("$ref {reference} resolves to nothing"),
                );
                return None;
            };
            if !walk.pending.insert(def.to_owned()) {
                self.refuse(
                    walk.refusals,
                    path,
                    &format!("$ref {reference} is recursive"),
                );
                return None;
            }
            let ty = self.ty(
                target,
                &format!("{path}{{{reference}}}"),
                &pascal(def),
                walk,
            );
            walk.pending.remove(def);
            return ty;
        }
        let variants = schema
            .get("enum")
            .and_then(Value::as_array)
            .cloned()
            .or_else(|| schema.get("const").map(|c| vec![c.clone()]));
        if let Some(variants) = variants {
            let strings: Option<Vec<String>> = variants
                .iter()
                .map(|v| v.as_str().map(str::to_owned))
                .collect();
            let Some(variants) = strings.filter(|v| !v.is_empty()) else {
                self.refuse(
                    walk.refusals,
                    path,
                    "an enum whose variants are not all strings",
                );
                return None;
            };
            if schema
                .get("type")
                .is_some_and(|t| t.as_str() != Some("string"))
            {
                self.refuse(walk.refusals, path, "an enum that is not of type string");
                return None;
            }
            return self.declare(
                TypeDecl {
                    name: self.type_name(hint),
                    kind: TypeKind::Enum,
                    summary: description(schema),
                    variants,
                    fields: Vec::new(),
                    ty: None,
                },
                path,
                walk,
            );
        }
        match schema.get("type").and_then(Value::as_str) {
            Some("string") => Some(
                if schema.get("contentEncoding").and_then(Value::as_str) == Some("base64") {
                    "Bytes"
                } else {
                    "String"
                }
                .to_owned(),
            ),
            Some("integer") => Some("Integer".to_owned()),
            Some("boolean") => Some("Boolean".to_owned()),
            Some("array") => {
                let Some(items) = schema.get("items") else {
                    self.refuse(walk.refusals, path, "an array without items");
                    return None;
                };
                let item = self.ty(
                    items,
                    &format!("{path}/items"),
                    &format!("{hint}Item"),
                    walk,
                )?;
                Some(format!("List<{item}>"))
            }
            Some("object") => {
                let properties = schema.get("properties").and_then(Value::as_object);
                if properties.is_some_and(|p| !p.is_empty()) {
                    let fields = self.fields(schema, path, hint, &[], walk);
                    return self.declare(
                        TypeDecl {
                            name: self.type_name(hint),
                            kind: TypeKind::Struct,
                            summary: description(schema),
                            variants: Vec::new(),
                            fields,
                            ty: None,
                        },
                        path,
                        walk,
                    );
                }
                match schema.get("additionalProperties") {
                    Some(value @ Value::Object(_)) => {
                        let value = self.ty(
                            value,
                            &format!("{path}/additionalProperties"),
                            &format!("{hint}Value"),
                            walk,
                        )?;
                        Some(format!("Map<String, {value}>"))
                    }
                    _ => {
                        self.refuse(
                            walk.refusals,
                            path,
                            "an object with neither properties nor a typed additionalProperties",
                        );
                        None
                    }
                }
            }
            Some(other) => {
                self.refuse(
                    walk.refusals,
                    path,
                    &format!("type {other} has no tool-contract/1 type"),
                );
                None
            }
            None => {
                let union = ["oneOf", "anyOf", "allOf"]
                    .into_iter()
                    .find(|key| schema.get(*key).is_some());
                let detail = union.map_or_else(
                    || "a node without a type".to_owned(),
                    |key| format!("a {key} union has no tool-contract/1 type"),
                );
                self.refuse(walk.refusals, path, &detail);
                None
            }
        }
    }
}

struct Walk<'r, 'f> {
    root: &'r Value,
    pending: BTreeSet<String>,
    refusals: &'f mut Vec<Refusal>,
}

/// `schema` without its nullability, and whether it had one.
fn strip_null(schema: &Value) -> Result<(Value, bool), String> {
    let mut schema = schema.clone();
    let mut nullable = false;
    if let Some(Value::Array(types)) = schema.get("type").cloned() {
        let rest: Vec<&Value> = types
            .iter()
            .filter(|t| t.as_str() != Some("null"))
            .collect();
        nullable = rest.len() != types.len();
        match rest.as_slice() {
            [one] => {
                schema["type"] = (*one).clone();
            }
            _ => return Err("a type list of several non-null types".to_owned()),
        }
    }
    if let Some(Value::Array(variants)) = schema.get("enum").cloned() {
        let rest: Vec<Value> = variants.into_iter().filter(|v| !v.is_null()).collect();
        if let Some(Value::Array(all)) = schema.get("enum") {
            nullable |= rest.len() != all.len();
        }
        schema["enum"] = Value::Array(rest);
    }
    for key in ["anyOf", "oneOf"] {
        if let Some(Value::Array(options)) = schema.get(key).cloned() {
            let is_null = |o: &Value| o.get("type").and_then(Value::as_str) == Some("null");
            if options.len() == 2 && options.iter().filter(|o| is_null(o)).count() == 1 {
                let other = options
                    .iter()
                    .find(|o| !is_null(o))
                    .cloned()
                    .unwrap_or_default();
                let (inner, _) = strip_null(&other)?;
                return Ok((inner, true));
            }
        }
    }
    Ok((schema, nullable))
}

/// `schema` with every string enum typed `string`, the shape the runner's check reads, and the
/// root's `omit` properties taken out.
pub(crate) fn normalized(schema: &Value, omit: &[String]) -> Value {
    let mut schema = typed_enums(schema);
    if let Some(Value::Object(properties)) = schema.get_mut("properties") {
        for field in omit {
            properties.remove(field);
        }
    }
    if let Some(Value::Array(required)) = schema.get_mut("required") {
        required.retain(|r| r.as_str().is_none_or(|r| !omit.iter().any(|o| o == r)));
    }
    schema
}

fn typed_enums(schema: &Value) -> Value {
    match schema {
        Value::Object(map) => {
            let mut out: Map<String, Value> = map
                .iter()
                .map(|(key, value)| (key.clone(), typed_enums(value)))
                .collect();
            let all_strings = map
                .get("enum")
                .and_then(Value::as_array)
                .is_some_and(|v| v.iter().filter(|x| !x.is_null()).all(Value::is_string));
            if all_strings && !map.contains_key("type") {
                out.insert("type".into(), Value::String("string".into()));
            }
            Value::Object(out)
        }
        Value::Array(items) => Value::Array(items.iter().map(typed_enums).collect()),
        other => other.clone(),
    }
}
