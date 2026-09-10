//! Explicit local write generation. The v2 projection remains GET-only.
use crate::v2::{self, Parameter, Source, hash, json_bytes, method_name, type_name, word, write};
use connectors_core::{Descriptor, Error, Operation, Result, digest};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::Path,
};

mod generation;
mod upstream;

#[derive(Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum BodyParameter {
    Input { name: String },
    String { value: String },
    Boolean { value: bool },
    Integer { value: i64 },
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WriteMapping {
    pub operation: String,
    pub upstream_operation: String,
    pub path: String,
    pub method: String,
    pub path_parameters: BTreeMap<String, Parameter>,
    pub query_parameters: BTreeMap<String, Parameter>,
    pub body: BTreeMap<String, BodyParameter>,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Write {
    pub operation: Operation,
    pub mapping: WriteMapping,
}

/// Parsed, closed input. Callers cannot mutate an admitted specification before
/// import/generation; source bytes are independently checked against the pin.
pub struct Spec {
    read: v2::Spec,
    writes: Vec<Write>,
    authored: Value,
}
fn refuse(message: impl Into<String>) -> Error {
    Error::invalid(message)
}
fn field_name(name: &str) -> bool {
    word(name) && !matches!(name, "self" | "super" | "crate")
}

impl Spec {
    pub fn parse(bytes: &[u8]) -> Result<Self> {
        if bytes.len() > 1024 * 1024 {
            return Err(refuse("specification exceeds byte limit"));
        }
        let value: Value = connectors_core::read_json(bytes)?;
        let schema: Value =
            serde_json::from_str(include_str!("../../../spec-kinds/adapter/v3/schema.json"))
                .map_err(|_| Error::internal())?;
        if !jsonschema::validator_for(&schema)
            .map_err(|_| Error::internal())?
            .is_valid(&value)
        {
            return Err(refuse(
                "adapter document does not match connectors.adapter/v3",
            ));
        }
        let writes: Vec<Write> = serde_json::from_value(value["writes"].clone())
            .map_err(|_| refuse("invalid closed write declaration"))?;
        let mut projection = value.clone();
        projection
            .as_object_mut()
            .ok_or_else(Error::internal)?
            .remove("writes");
        projection["kind"] = json!("connectors.adapter/v2");
        let read = v2::Spec::parse(&json_bytes(&projection)?)?;
        if writes.len() + read.operations.len() > 256 {
            return Err(refuse("private operation count exceeds 256"));
        }
        let mut ids: BTreeSet<_> = read.operations.iter().map(|o| o.id.clone()).collect();
        let mut methods: BTreeSet<_> = ids.iter().map(|id| method_name(id)).collect();
        let mut types: BTreeSet<_> = ids.iter().map(|id| type_name(id)).collect();
        for w in &writes {
            let op = &w.operation;
            let m = &w.mapping;
            if !op.id.split('.').all(word)
                || !ids.insert(op.id.clone())
                || !methods.insert(method_name(&op.id))
                || !types.insert(type_name(&op.id))
                || m.operation != op.id
                || m.method != "put"
                || m.upstream_operation.is_empty()
            {
                return Err(refuse("invalid or colliding write operation mapping"));
            }
            let fields = input_fields(op)?;
            // Output lowering validates the representable typed subset too.
            generation::output_model(op)?;
            let tail = m
                .path
                .strip_prefix(&format!("{}/", read.upstream.base_path))
                .ok_or_else(|| refuse("write path is outside pinned base path"))?;
            let mut placeholders = BTreeSet::new();
            for segment in tail.split('/') {
                if let Some(name) = segment.strip_prefix('{').and_then(|v| v.strip_suffix('}')) {
                    if !field_name(name) || !placeholders.insert(name) {
                        return Err(refuse("invalid or repeated path placeholder"));
                    }
                } else if segment.is_empty()
                    || matches!(segment, "." | "..")
                    || segment.contains(['{', '}', '?', '#', '%', '\\'])
                {
                    return Err(refuse("unsupported literal write path segment"));
                }
            }
            if placeholders != m.path_parameters.keys().map(String::as_str).collect() {
                return Err(refuse("incomplete write path mapping"));
            }
            for (name, p) in m.path_parameters.iter().chain(&m.query_parameters) {
                if !field_name(name) {
                    return Err(refuse("unsupported parameter name"));
                }
                match p {
                    Parameter::Input { name }
                        if !fields.iter().any(|(n, s)| {
                            *n == name && matches!(s["type"].as_str(), Some("integer" | "string"))
                        }) =>
                    {
                        return Err(refuse("path/query references missing or non-textual input"));
                    }
                    Parameter::Binding { name } if !field_name(name) => {
                        return Err(refuse("invalid binding parameter name"));
                    }
                    _ => {}
                }
            }
            for (name, p) in &m.body {
                if !field_name(name) {
                    return Err(refuse("unsupported body property name"));
                }
                if matches!(p, BodyParameter::Input { name } if !fields.iter().any(|(n, _)| *n == name))
                {
                    return Err(refuse("body references missing or optional input"));
                }
            }
        }
        let spec = Self {
            read,
            writes,
            authored: value,
        };
        generation::validate_names(&spec)?;
        spec.private_descriptor()?;
        Ok(spec)
    }
    pub fn read_spec(&self) -> &v2::Spec {
        &self.read
    }
    pub fn writes(&self) -> &[Write] {
        &self.writes
    }
    pub fn descriptor(&self) -> Result<Descriptor> {
        self.read.descriptor()
    }
    pub fn private_descriptor(&self) -> Result<Descriptor> {
        let mut declaration = crate::AdapterSpec {
            kind: "connectors.adapter/v1".into(),
            id: self.read.id.clone(),
            version: self.read.version.clone(),
            sources: self.read.sources.clone(),
            configuration_schema: self.read.configuration_schema.clone(),
            operations: self.read.operations.clone(),
        };
        declaration
            .operations
            .extend(self.writes.iter().map(|w| w.operation.clone()));
        let mut descriptor = crate::compile(&json_bytes(&declaration)?)?;
        descriptor.revision = digest(
            &json!({"specification":self.authored,"configuration_schema":descriptor.configuration_schema,"projection":"private-required-approval-writes/1"}),
        );
        Ok(descriptor)
    }
}

fn input_fields(op: &Operation) -> Result<Vec<(&str, &Value)>> {
    let properties = generation::object_fields(&op.input_schema)?;
    let mut fields = Vec::new();
    for (name, value) in properties {
        generation::scalar_model(value)?;
        fields.push((name.as_str(), value));
    }
    Ok(fields)
}

pub fn import(spec: &Spec, path: &Path) -> Result<(Value, Value)> {
    upstream::import(spec, path)
}
pub fn generate(path: &Path, out: &Path, executable: &Path, check: bool) -> Result<()> {
    let source = std::fs::read(path).map_err(|_| refuse("cannot read specification"))?;
    let spec = Spec::parse(&source)?;
    let executable = crate::toolchain::resolve(Some(executable))?;
    let temp = tempfile::tempdir().map_err(|_| Error::internal())?;
    let root = temp.path();
    // Validate every write/source binding before spending work on either output
    // projection. Nothing is installed until all generated artifacts are ready.
    let (selected, coverage) = import(&spec, path)?;
    v2::build(&spec.read, path, root, &executable)?;
    write(
        &root.join("write-upstream.openapi.json"),
        &json_bytes(&selected)?,
    )?;
    write(&root.join("write-coverage.json"), &json_bytes(&coverage)?)?;
    write(
        &root.join("private-descriptor.json"),
        &crate::descriptor_bytes(&spec.private_descriptor()?)?,
    )?;
    generation::build(&spec, root, &executable)?;
    v2::finish(&source, out, root, &spec.read, check)
}
