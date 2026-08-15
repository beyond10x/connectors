//! Minimal, pure AsyncAPI event ingest.
//!
//! Like the OpenAPI front-end, this makes every declared component message available and selects
//! none. Provider patches decide which messages become catalog events and attach the policy facts
//! a transport description cannot know. V1 intentionally accepts AsyncAPI 3 component messages
//! only; operations and broker bindings are channel/runtime concerns elsewhere in the connector IR.

use serde_json::{Map, Value};

use crate::JsonSchema;

/// One component message available for explicit selection.
#[derive(Debug, Clone, PartialEq)]
pub struct SpecEvent {
    /// Stable component key used by `[[patch.events]] select`.
    pub message_id: String,
    /// Vendor event name published by the message.
    pub name: String,
    /// Vendor summary/description.
    pub description: String,
    /// Normalized event payload schema.
    pub payload: JsonSchema,
}

/// Everything an AsyncAPI document makes selectable.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Ingested {
    pub title: String,
    pub upstream_version: String,
    pub events: Vec<SpecEvent>,
}

impl Ingested {
    pub fn event(&self, message_id: &str) -> Option<&SpecEvent> {
        self.events
            .iter()
            .find(|event| event.message_id == message_id)
    }
}

/// Parses an AsyncAPI 3 document without filesystem or network access.
pub fn ingest(document: &str) -> crate::Result<Ingested> {
    let root = parse(document)?;
    let root = root
        .as_object()
        .ok_or_else(|| invalid("the document root is not a mapping"))?;
    let version = root
        .get("asyncapi")
        .and_then(Value::as_str)
        .ok_or_else(|| invalid("the document declares no string `asyncapi` version"))?;
    if !version.starts_with("3.") {
        return Err(invalid(format!(
            "AsyncAPI {version:?} is not supported; event ingest accepts AsyncAPI 3.x"
        )));
    }
    let info = root.get("info").and_then(Value::as_object);
    let messages = root
        .get("components")
        .and_then(Value::as_object)
        .and_then(|components| components.get("messages"))
        .and_then(Value::as_object)
        .ok_or_else(|| invalid("the document declares no `components.messages` mapping"))?;

    let mut ingested = Ingested {
        title: info
            .and_then(|info| info.get("title"))
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_owned(),
        upstream_version: info
            .and_then(|info| info.get("version"))
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_owned(),
        events: Vec::new(),
    };

    for (message_id, message) in messages {
        let message = resolve_local(root, message)?;
        let message = message
            .as_object()
            .ok_or_else(|| invalid(format!("component message {message_id:?} is not a mapping")))?;
        let name = message
            .get("name")
            .and_then(Value::as_str)
            .unwrap_or(message_id)
            .trim()
            .to_owned();
        if name.is_empty() {
            return Err(invalid(format!(
                "component message {message_id:?} declares an empty `name`"
            )));
        }
        let payload = message.get("payload").ok_or_else(|| {
            invalid(format!(
                "component message {message_id:?} declares no `payload` schema"
            ))
        })?;
        let payload = expand_refs(root, payload, &mut Vec::new(), &mut 0)?;
        ingested.events.push(SpecEvent {
            message_id: message_id.clone(),
            name,
            description: ["summary", "description", "title"]
                .iter()
                .find_map(|key| message.get(*key).and_then(Value::as_str))
                .unwrap_or_default()
                .trim()
                .to_owned(),
            payload,
        });
    }
    Ok(ingested)
}

fn resolve_local(root: &Map<String, Value>, value: &Value) -> crate::Result<Value> {
    let Some(pointer) = value
        .as_object()
        .and_then(|object| object.get("$ref"))
        .and_then(Value::as_str)
    else {
        return Ok(value.clone());
    };
    pointer_value(root, pointer)
}

fn expand_refs(
    root: &Map<String, Value>,
    value: &Value,
    trail: &mut Vec<String>,
    budget: &mut usize,
) -> crate::Result<Value> {
    *budget += 1;
    if *budget > 20_000 {
        return Err(invalid(
            "expanding an event payload exceeded 20,000 nodes; the schema is cyclic or too large",
        ));
    }
    match value {
        Value::Object(object) => {
            if let Some(pointer) = object.get("$ref").and_then(Value::as_str) {
                if trail.iter().any(|seen| seen == pointer) {
                    return Err(invalid(format!(
                        "event payload contains a `$ref` cycle through {pointer:?}"
                    )));
                }
                trail.push(pointer.to_owned());
                let target = pointer_value(root, pointer)?;
                let expanded = expand_refs(root, &target, trail, budget)?;
                trail.pop();
                return Ok(expanded);
            }
            object
                .iter()
                .map(|(key, child)| Ok((key.clone(), expand_refs(root, child, trail, budget)?)))
                .collect::<crate::Result<Map<String, Value>>>()
                .map(Value::Object)
        }
        Value::Array(array) => array
            .iter()
            .map(|child| expand_refs(root, child, trail, budget))
            .collect::<crate::Result<Vec<_>>>()
            .map(Value::Array),
        scalar => Ok(scalar.clone()),
    }
}

fn pointer_value(root: &Map<String, Value>, pointer: &str) -> crate::Result<Value> {
    let path = pointer
        .strip_prefix("#/")
        .ok_or_else(|| invalid(format!("external `$ref` {pointer:?} is not supported")))?;
    let mut current = Value::Object(root.clone());
    for segment in path.split('/') {
        let segment = segment.replace("~1", "/").replace("~0", "~");
        current = current
            .as_object()
            .and_then(|object| object.get(&segment))
            .cloned()
            .ok_or_else(|| invalid(format!("`$ref` {pointer:?} points at nothing")))?;
    }
    Ok(current)
}

fn parse(document: &str) -> crate::Result<Value> {
    if let Ok(value) = serde_json::from_str::<Value>(document) {
        return Ok(value);
    }
    let yaml: serde_norway::Value = serde_norway::from_str(document)
        .map_err(|error| invalid(format!("the document is neither JSON nor YAML: {error}")))?;
    serde_json::to_value(yaml).map_err(|error| {
        invalid(format!(
            "the YAML document cannot be represented as JSON: {error}"
        ))
    })
}

fn invalid(reason: impl Into<String>) -> crate::Error {
    crate::Error::ParseSpec {
        reason: reason.into(),
    }
}
