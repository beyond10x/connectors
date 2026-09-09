//! A deliberately bounded OpenAPI-to-SDK frontend, owned by Connectors.
use connectors_core::{Descriptor, Error, Operation, Result, canonical, digest};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::{Path, PathBuf},
    process::Command,
};

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Source {
    pub path: PathBuf,
    pub url: String,
    pub revision: String,
    pub sha256: String,
    pub base_path: String,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Parameter {
    Input { name: String },
    Constant { value: String },
    Binding { name: String },
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Mapping {
    pub operation: String,
    pub upstream_operation: String,
    pub path: String,
    pub method: String,
    pub path_parameters: BTreeMap<String, Parameter>,
    pub query_parameters: BTreeMap<String, Parameter>,
    pub obligations: Vec<String>,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Spec {
    pub kind: String,
    pub id: String,
    pub version: String,
    pub sources: Vec<String>,
    pub configuration_schema: Value,
    pub operations: Vec<Operation>,
    pub upstream: Source,
    pub mappings: Vec<Mapping>,
}
pub fn hash(bytes: &[u8]) -> String {
    hex::encode(Sha256::digest(bytes))
}
fn refuse(message: impl Into<String>) -> Error {
    Error::invalid(message)
}
fn word(s: &str) -> bool {
    !s.is_empty()
        && s.as_bytes()[0].is_ascii_lowercase()
        && s.bytes()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == b'_')
}
fn method_name(id: &str) -> String {
    id.replace('.', "_")
}
fn type_name(id: &str) -> String {
    id.split(['.', '_'])
        .filter(|s| !s.is_empty())
        .map(|s| format!("{}{}", s[..1].to_uppercase(), &s[1..]))
        .collect::<String>()
        + "Request"
}
fn ident(name: &str) -> String {
    format!("r#{name}")
}
#[derive(Clone)]
struct Field {
    name: String,
    ty: &'static str,
    nullable: bool,
}
fn fields(operation: &Operation) -> Result<Vec<Field>> {
    let schema = &operation.input_schema;
    if schema["type"] != "object" || schema["additionalProperties"] != false {
        return Err(refuse("generated inputs require closed object schemas"));
    }
    let properties = schema["properties"]
        .as_object()
        .ok_or_else(|| refuse("missing input properties"))?;
    let required = schema["required"]
        .as_array()
        .ok_or_else(|| refuse("missing required input declaration"))?;
    if required
        .iter()
        .any(|v| v.as_str().is_none_or(|name| !properties.contains_key(name)))
    {
        return Err(refuse("required input names an undeclared field"));
    }
    properties
        .iter()
        .map(|(name, value)| {
            if !word(name) || matches!(name.as_str(), "self" | "super" | "crate") {
                return Err(refuse("unsupported input field identifier"));
            }
            let (ty, nullable) = if value["type"] == "string" {
                ("String", false)
            } else if value["type"] == "integer" {
                ("Integer", false)
            } else if value["anyOf"].as_array().is_some_and(|a| {
                a.len() == 2
                    && a.iter().any(|v| v["type"] == "string")
                    && a.iter().any(|v| v["type"] == "null")
            }) {
                ("Optional<String>", true)
            } else {
                return Err(refuse(format!(
                    "unsupported required input type: {}.{name}",
                    operation.id
                )));
            };
            if !nullable && !required.iter().any(|v| v == name) {
                return Err(refuse(
                    "optional scalar needs an explicit nullable contract",
                ));
            }
            Ok(Field {
                name: name.clone(),
                ty,
                nullable,
            })
        })
        .collect()
}
impl Spec {
    pub fn parse(bytes: &[u8]) -> Result<Self> {
        if bytes.len() > 1024 * 1024 {
            return Err(refuse("specification exceeds byte limit"));
        }
        let value: Value = connectors_core::read_json(bytes)?;
        let schema: Value =
            serde_json::from_str(include_str!("../../../spec-kinds/adapter/v2/schema.json"))
                .map_err(|_| Error::internal())?;
        if !jsonschema::validator_for(&schema)
            .map_err(|_| Error::internal())?
            .is_valid(&value)
        {
            return Err(refuse(
                "adapter document does not match connectors.adapter/v2",
            ));
        }
        let spec: Self =
            serde_json::from_value(value).map_err(|_| refuse("invalid v2 specification"))?;
        if spec.kind != "connectors.adapter/v2"
            || !word(&spec.id)
            || spec.upstream.sha256.len() != 64
            || !spec.upstream.sha256.bytes().all(|c| c.is_ascii_hexdigit())
            || spec.upstream.revision.len() != 40
            || !spec
                .upstream
                .revision
                .bytes()
                .all(|c| c.is_ascii_hexdigit())
            || !spec.upstream.url.starts_with("https://")
            || !spec.upstream.url.contains(&spec.upstream.revision)
            || !spec.upstream.base_path.starts_with('/')
            || spec.upstream.base_path.ends_with('/')
            || spec.upstream.path.is_absolute()
        {
            return Err(refuse("invalid v2 identity or immutable upstream source"));
        }
        spec.descriptor()?;
        let mut names = BTreeSet::new();
        let mut methods = BTreeSet::new();
        let mut types = BTreeSet::new();
        for m in &spec.mappings {
            let op = spec
                .operations
                .iter()
                .find(|o| o.id == m.operation)
                .ok_or_else(|| refuse("mapping names undeclared operation"))?;
            if !m.operation.split('.').all(word)
                || !names.insert(&m.operation)
                || !methods.insert(method_name(&m.operation))
                || !types.insert(type_name(&m.operation))
                || m.method != "get"
                || m.upstream_operation.is_empty()
                || !m.path.starts_with(&format!("{}/", spec.upstream.base_path))
                || m.obligations != ["prepare", "finish"]
            {
                return Err(refuse(
                    "unsupported or duplicate operation mapping / missing obligations",
                ));
            }
            let f = fields(op)?;
            let placeholders: BTreeSet<_> = m
                .path
                .split('/')
                .filter_map(|s| s.strip_prefix('{').and_then(|v| v.strip_suffix('}')))
                .collect();
            if placeholders != m.path_parameters.keys().map(String::as_str).collect() {
                return Err(refuse("path parameter mapping is incomplete"));
            }
            for (name, p) in m.path_parameters.iter().chain(&m.query_parameters) {
                if !word(name) {
                    return Err(refuse("unsupported upstream parameter name"));
                }
                match p {
                    Parameter::Input { name }
                        if !f.iter().any(|f| f.name == *name && !f.nullable) =>
                    {
                        return Err(refuse("parameter references missing or nullable input"));
                    }
                    Parameter::Binding { name }
                        if !word(name)
                            || matches!(name.as_str(), "self" | "super" | "crate" | "Self") =>
                    {
                        return Err(refuse("invalid binding parameter identifier"));
                    }
                    _ => {}
                }
            }
            for segment in m.path.trim_start_matches('/').split('/') {
                if segment.is_empty()
                    || segment == "."
                    || segment == ".."
                    || segment.contains(['?', '#'])
                    || (segment.contains(['{', '}'])
                        && !placeholders.contains(segment.trim_matches(['{', '}'])))
                {
                    return Err(refuse("unsupported request path"));
                }
            }
        }
        if names.len() != spec.operations.len() {
            return Err(refuse("every operation requires exactly one mapping"));
        }
        Ok(spec)
    }
    pub fn descriptor(&self) -> Result<Descriptor> {
        let mut descriptor = self.legacy_descriptor()?;
        descriptor.revision = digest(
            &json!({"specification":self, "configuration_schema":descriptor.configuration_schema}),
        );
        Ok(descriptor)
    }
    fn legacy_descriptor(&self) -> Result<Descriptor> {
        let legacy = super::AdapterSpec {
            kind: "connectors.adapter/v1".into(),
            id: self.id.clone(),
            version: self.version.clone(),
            sources: self.sources.clone(),
            configuration_schema: self.configuration_schema.clone(),
            operations: self.operations.clone(),
        };
        super::compile(&serde_json::to_vec(&legacy).map_err(|_| Error::internal())?)
    }
}

/// Import exact selected source operations and their local reference closure.
/// Response bodies remain source facts: provider shape corrections belong to bindings.
pub fn import(spec: &Spec, spec_path: &Path) -> Result<(Value, Value)> {
    let path = spec_path
        .parent()
        .unwrap_or(Path::new("."))
        .join(&spec.upstream.path);
    let bytes = std::fs::read(path).map_err(|_| refuse("cannot read pinned upstream source"))?;
    if bytes.len() > 16 * 1024 * 1024 || hash(&bytes) != spec.upstream.sha256 {
        return Err(refuse(
            "upstream source digest mismatch or byte limit exceeded",
        ));
    }
    let yaml: serde_yaml_ng::Value = serde_yaml_ng::from_slice(&bytes)
        .map_err(|_| refuse("invalid or ambiguous upstream YAML"))?;
    let source =
        serde_json::to_value(yaml).map_err(|_| refuse("upstream must be JSON-compatible YAML"))?;
    if !source["openapi"]
        .as_str()
        .is_some_and(|v| v.starts_with("3.0.") || v.starts_with("3.1."))
    {
        return Err(refuse("unsupported OpenAPI version"));
    }
    let mut paths = serde_json::Map::new();
    let mut coverage = Vec::new();
    for mapping in &spec.mappings {
        let path_item = &source["paths"][&mapping.path];
        let operation = &path_item[&mapping.method];
        if operation["operationId"] != mapping.upstream_operation {
            return Err(refuse(format!(
                "missing or changed upstream operation {}",
                mapping.upstream_operation
            )));
        }
        if operation.get("requestBody").is_some() {
            return Err(refuse(
                "GET request bodies are outside this generator profile",
            ));
        }
        let parameters: Vec<_> = path_item["parameters"]
            .as_array()
            .into_iter()
            .flatten()
            .chain(operation["parameters"].as_array().into_iter().flatten())
            .collect();
        let mut mapped = BTreeSet::new();
        for (place, values) in [
            ("path", &mapping.path_parameters),
            ("query", &mapping.query_parameters),
        ] {
            for (name, value) in values {
                let parameter = parameters
                    .iter()
                    .find(|p| p["name"] == *name && p["in"] == place)
                    .ok_or_else(|| {
                        refuse(format!(
                            "unmapped upstream parameter {place}:{name} for {}",
                            mapping.operation
                        ))
                    })?;
                let schema = &parameter["schema"];
                if schema.get("oneOf").is_some_and(|branches| {
                    branches.as_array().is_none_or(|branches| {
                        branches.iter().any(|branch| {
                            branch.as_object().is_none_or(|o| {
                                o.keys()
                                    .any(|k| !["type", "nullable"].contains(&k.as_str()))
                            })
                        })
                    })
                }) {
                    return Err(refuse(
                        "constrained source union needs an explicit supported mapping",
                    ));
                }
                if parameter.get("content").is_some()
                    || parameter
                        .get("style")
                        .is_some_and(|s| s != if place == "path" { "simple" } else { "form" })
                    || parameter["allowReserved"] == true
                    || parameter["allowEmptyValue"] == true
                {
                    return Err(refuse("unsupported required parameter serialization"));
                }
                // This profile only proves scalar encodings. Constraints on a
                // mapped vendor parameter must be preserved explicitly by the
                // caller contract; unfamiliar schema semantics are refused.
                if schema.as_object().is_none_or(|o| {
                    o.keys().any(|k| {
                        ![
                            "type",
                            "oneOf",
                            "nullable",
                            "enum",
                            "default",
                            "description",
                            "example",
                            "minimum",
                            "maximum",
                            "minLength",
                            "maxLength",
                            "pattern",
                        ]
                        .contains(&k.as_str())
                    })
                }) {
                    return Err(refuse("unsupported mapped source schema semantics"));
                }
                let admits = |ty: &str| {
                    schema["type"] == ty
                        || schema["oneOf"]
                            .as_array()
                            .is_some_and(|a| a.iter().any(|s| s["type"] == ty))
                };
                match value {
                    Parameter::Input { name } => {
                        let op = spec
                            .operations
                            .iter()
                            .find(|o| o.id == mapping.operation)
                            .ok_or_else(|| refuse("mapping names undeclared operation"))?;
                        let f = fields(op)?
                            .into_iter()
                            .find(|f| f.name == *name)
                            .ok_or_else(|| refuse("parameter references missing input"))?;
                        let ty = if f.ty == "Integer" {
                            "integer"
                        } else {
                            "string"
                        };
                        if !admits(ty) {
                            return Err(refuse(format!(
                                "source parameter type does not admit mapped {ty}: {name}"
                            )));
                        }
                        let input = &op.input_schema["properties"][name];
                        for key in [
                            "minimum",
                            "maximum",
                            "minLength",
                            "maxLength",
                            "pattern",
                            "enum",
                        ] {
                            if schema.get(key).is_some() && schema[key] != input[key] {
                                return Err(refuse(format!(
                                    "source constraint requires an explicit matching input constraint: {name}.{key}"
                                )));
                            }
                        }
                    }
                    Parameter::Constant { value } => {
                        if !admits("string")
                            || !jsonschema::validator_for(schema)
                                .map_err(|_| refuse("invalid constant schema"))?
                                .is_valid(&json!(value))
                            || schema["enum"]
                                .as_array()
                                .is_some_and(|a| !a.iter().any(|v| v == value))
                        {
                            return Err(refuse(
                                "constant is outside the upstream parameter schema",
                            ));
                        }
                    }
                    Parameter::Binding { .. } => {
                        if !admits("integer") && !admits("string") {
                            return Err(refuse("binding parameter requires a scalar source type"));
                        }
                        if [
                            "minimum",
                            "maximum",
                            "minLength",
                            "maxLength",
                            "pattern",
                            "enum",
                        ]
                        .iter()
                        .any(|key| schema.get(key).is_some())
                        {
                            return Err(refuse(
                                "constrained binding parameter needs a supported typed mapping",
                            ));
                        }
                    }
                }
                mapped.insert(format!("{place}:{name}"));
            }
        }
        let mut excluded = Vec::new();
        for parameter in &parameters {
            if parameter.get("$ref").is_some() {
                return Err(refuse(
                    "referenced parameter requires an explicit supported import mapping",
                ));
            }
            let key = format!(
                "{}:{}",
                parameter["in"].as_str().unwrap_or("?"),
                parameter["name"].as_str().unwrap_or("?")
            );
            if !mapped.contains(&key) {
                if parameter["required"] == true {
                    return Err(refuse(format!(
                        "required source parameter is unbound: {key}"
                    )));
                }
                excluded.push(key);
            }
        }
        let mut selected = json!({mapping.method.clone():operation.clone()});
        if let Some(parameters) = path_item.get("parameters") {
            selected["parameters"] = parameters.clone();
        }
        paths.insert(mapping.path.clone(), selected);
        coverage.push(json!({"operation":mapping.operation,"source_operation":mapping.upstream_operation,"source_pointer":format!("/paths/{}/{}",mapping.path.replace('~',"~0").replace('/',"~1"),mapping.method),"generated":["typed_input","request","dispatch","descriptor"],"mapped_parameters":mapped,"excluded_optional_parameters":excluded,"requires_implementation":["prepare: admission and continuation inputs","finish: provider response interpretation, completeness and provenance"],"response_types":"source retained; opaque JSON interpretation is an explicit binding obligation","authentication":"host-bound capability; source security declarations grant no credentials"}));
    }
    let mut selected =
        json!({"openapi":source["openapi"],"info":source["info"],"paths":paths,"components":{}});
    for key in ["servers", "security"] {
        if let Some(v) = source.get(key) {
            selected[key] = v.clone();
        }
    }
    if let Some(v) = source.pointer("/components/securitySchemes") {
        selected["components"]["securitySchemes"] = v.clone();
    }
    let mut seen = BTreeSet::new();
    loop {
        let mut references = BTreeSet::new();
        collect_refs(&selected, &mut references);
        let pending: Vec<_> = references.difference(&seen).cloned().collect();
        if pending.is_empty() {
            break;
        }
        for reference in pending {
            if !reference.starts_with("#/components/schemas/") {
                return Err(refuse(format!(
                    "unsupported external/non-schema reference {reference}"
                )));
            }
            let value = source
                .pointer(&reference[1..])
                .ok_or_else(|| refuse(format!("unresolved source reference {reference}")))?
                .clone();
            let name = reference
                .trim_start_matches("#/components/schemas/")
                .replace("~1", "/")
                .replace("~0", "~");
            if selected["components"].get("schemas").is_none() {
                selected["components"]["schemas"] = json!({});
            }
            selected["components"]["schemas"][name] = value;
            seen.insert(reference);
            if seen.len() > 4096 {
                return Err(refuse("reference closure exceeds bounds"));
            }
        }
    }
    let report = json!({"format":"connectors.import-coverage/v1","source":spec.upstream,"selected_operations":coverage,"retained_schema_references":seen,"unselected_paths":"excluded by explicit operation selection; no generated capability claimed"});
    Ok((selected, report))
}
fn collect_refs(v: &Value, refs: &mut BTreeSet<String>) {
    match v {
        Value::Object(o) => {
            if let Some(Value::String(r)) = o.get("$ref") {
                refs.insert(r.clone());
            }
            for v in o.values() {
                collect_refs(v, refs);
            }
        }
        Value::Array(a) => {
            for v in a {
                collect_refs(v, refs);
            }
        }
        _ => {}
    }
}
pub fn json_bytes(v: &impl Serialize) -> Result<Vec<u8>> {
    let mut b = canonical(&serde_json::to_value(v).map_err(|_| Error::internal())?);
    b.push(b'\n');
    Ok(b)
}
pub fn write(path: &Path, bytes: &[u8]) -> Result<()> {
    std::fs::create_dir_all(path.parent().ok_or_else(Error::internal)?)
        .map_err(|e| refuse(format!("create {}: {e}", path.display())))?;
    std::fs::write(path, bytes).map_err(|e| refuse(format!("write {}: {e}", path.display())))
}
pub fn ess(executable: &Path, args: &[&str]) -> Result<Vec<u8>> {
    let output = Command::new(executable)
        .args(args)
        .output()
        .map_err(|_| refuse("cannot run pinned ESS executable"))?;
    if !output.status.success() {
        return Err(refuse(format!(
            "ESS refused: {}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        )));
    }
    Ok(output.stdout)
}
pub fn check_ess(executable: &Path) -> Result<()> {
    crate::toolchain::check(executable)
}

pub fn generate(spec_path: &Path, out: &Path, executable: &Path, check: bool) -> Result<()> {
    let spec =
        Spec::parse(&std::fs::read(spec_path).map_err(|_| refuse("cannot read specification"))?)?;
    let resolved = crate::toolchain::resolve(Some(executable))?;
    let executable = resolved.as_path();
    let temp = tempfile::tempdir().map_err(|_| Error::internal())?;
    let root = temp.path();
    let (selected, coverage) =
        import(&spec, spec_path).map_err(|e| refuse(format!("import: {e}")))?;
    write(&root.join("upstream.openapi.json"), &json_bytes(&selected)?)?;
    write(&root.join("coverage.json"), &json_bytes(&coverage)?)?;
    // ESS imports source facts independently. Its coverage is retained even when
    // opaque response schemas are outside its supported interface subset.
    let imported = Command::new(executable)
        .args(["infra", "import", "openapi", "--path"])
        .arg(root.join("upstream.openapi.json"))
        .args(["--format", "json"])
        .output()
        .map_err(|e| refuse(format!("run ESS import: {e}")))?;
    write(
        &root.join("ess-import.json"),
        &json_bytes(
            &json!({"exit_code":imported.status.code(),"stdout":String::from_utf8_lossy(&imported.stdout),"stderr":String::from_utf8_lossy(&imported.stderr)}),
        )?,
    )?;
    lower(&spec, &root.join("ess"))?;
    ess(
        executable,
        &[
            "specify",
            "validate",
            "--path",
            root.join("ess").to_str().unwrap(),
        ],
    )?;
    let ir = ess(
        executable,
        &[
            "specify",
            "compile",
            "--path",
            root.join("ess").to_str().unwrap(),
            "--format",
            "json",
        ],
    )?;
    write(&root.join("ess-ir.json"), &ir)?;
    ess(
        executable,
        &[
            "generate",
            "synthesize",
            "--path",
            root.join("ess").to_str().unwrap(),
            "--target",
            "rust",
            "--out",
            root.join("rust").to_str().unwrap(),
        ],
    )?;
    write(
        &root.join("descriptor.json"),
        &super::descriptor_bytes(&spec.descriptor()?)?,
    )?;
    write(&root.join("runtime.rs"), render(&spec)?.as_bytes())?;
    let formatted = Command::new("rustfmt")
        .args(["--edition", "2024"])
        .arg(root.join("runtime.rs"))
        .status()
        .map_err(|_| Error::internal())?;
    if !formatted.success() {
        return Err(refuse("generated Rust did not format"));
    }
    // Normalize the pinned ESS emitter's Rust as part of this generation
    // pipeline, so Cargo formatting never becomes a second output writer.
    for path in tree(&root.join("rust"))?
        .keys()
        .filter(|p| p.ends_with(".rs"))
    {
        let formatted = Command::new("rustfmt")
            .args(["--edition", "2021"])
            .arg(root.join("rust").join(path))
            .status()
            .map_err(|_| refuse("cannot format synthesized Rust"))?;
        if !formatted.success() {
            return Err(refuse("synthesized Rust did not format"));
        }
    }
    let rustfmt = Command::new("rustfmt")
        .arg("--version")
        .output()
        .map_err(|_| refuse("cannot identify rustfmt"))?;
    if !rustfmt.status.success() {
        return Err(refuse("cannot identify rustfmt"));
    }
    let mut files = tree(root).map_err(|e| refuse(format!("output tree: {e}")))?;
    // ESS owns this temporary anchor's recovery state. It contains native anchor
    // identity, not projection artifacts, and must never enter the portable bundle.
    // Keep `tree` complete for public-output audits and all other callers.
    files.retain(|path, _| !path.starts_with("rust/.ess-output/"));
    let manifest = json!({"format":"connectors.generated-bundle/v1","specification_sha256":hash(&std::fs::read(spec_path).map_err(|_|Error::internal())?),"upstream_sha256":spec.upstream.sha256,"ess":crate::toolchain::version()?,"ess_source":crate::toolchain::source()?,"rustfmt":String::from_utf8_lossy(&rustfmt.stdout).trim(),"files":files.iter().map(|(path,b)|(path.clone(),hash(b))).collect::<BTreeMap<_,_>>()});
    install(out, &files, &manifest, &spec, check)?;
    Ok(())
}

fn safe_output(root: &Path, name: &str) -> Result<PathBuf> {
    let relative = Path::new(name);
    if name.is_empty()
        || relative
            .components()
            .any(|c| !matches!(c, std::path::Component::Normal(_)))
    {
        return Err(refuse("invalid owned output path"));
    }
    let path = root.join(relative);
    // Reject symlinks in the root, ancestors and generated paths before writing.
    for ancestor in path.ancestors() {
        match std::fs::symlink_metadata(ancestor) {
            Ok(m) if m.file_type().is_symlink() => {
                return Err(refuse("output path contains a symlink"));
            }
            Err(e) if e.kind() != std::io::ErrorKind::NotFound => {
                return Err(refuse(format!("inspect output: {e}")));
            }
            _ => {}
        }
    }
    Ok(path)
}

fn install(
    out: &Path,
    files: &BTreeMap<String, Vec<u8>>,
    manifest: &Value,
    spec: &Spec,
    check: bool,
) -> Result<()> {
    let manifest_path = safe_output(out, "manifest.json")?;
    let old: Value = match std::fs::read(&manifest_path) {
        Ok(bytes) => {
            let value: Value = connectors_core::read_json(&bytes)?;
            if value["format"] != "connectors.generated-bundle/v1" || !value["files"].is_object() {
                return Err(refuse("invalid prior generation manifest"));
            }
            value
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => json!({"files":{}}),
        Err(e) => return Err(refuse(format!("read generation manifest: {e}"))),
    };
    let old_files = old["files"]
        .as_object()
        .ok_or_else(|| refuse("invalid owned file map"))?;
    // Preflight every ownership/path decision, including retirement, before any mutation.
    for (name, digest) in old_files {
        if name == "manifest.json"
            || digest
                .as_str()
                .is_none_or(|s| s.len() != 64 || !s.bytes().all(|b| b.is_ascii_hexdigit()))
        {
            return Err(refuse("invalid owned file digest"));
        }
        safe_output(out, name)?;
    }
    for (name, bytes) in files {
        let path = safe_output(out, name)?;
        if check {
            if std::fs::read(&path).ok().as_ref() != Some(bytes) {
                return Err(refuse(format!("generated drift: {name}")));
            }
        } else if path.exists() && !old_files.contains_key(name) {
            // Adopt only the exact v1 descriptor produced from this document's
            // unchanged v1 fields. Other files require prior manifest ownership.
            let legacy = name == "descriptor.json"
                && old_files.is_empty()
                && std::fs::read(&path).ok()
                    == Some(super::descriptor_bytes(&spec.legacy_descriptor()?)?);
            if !legacy {
                return Err(refuse(format!("refusing to overwrite unowned file {name}")));
            }
        }
    }
    let manifest_bytes = json_bytes(manifest)?;
    if check {
        if std::fs::read(manifest_path).ok() != Some(manifest_bytes) {
            return Err(refuse("generated manifest drift"));
        }
        return Ok(());
    }
    for (name, bytes) in files {
        write(&out.join(name), bytes)?;
    }
    for name in old_files.keys().filter(|name| !files.contains_key(*name)) {
        match std::fs::remove_file(out.join(name)) {
            Ok(()) => {}
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => return Err(refuse(format!("retire generated file: {e}"))),
        }
    }
    write(&manifest_path, &manifest_bytes)
}

pub fn tree(root: &Path) -> Result<BTreeMap<String, Vec<u8>>> {
    fn visit(root: &Path, path: &Path, files: &mut BTreeMap<String, Vec<u8>>) -> Result<()> {
        for entry in std::fs::read_dir(path).map_err(|_| Error::internal())? {
            let entry = entry.map_err(|_| Error::internal())?;
            let path = entry.path();
            let ty = entry.file_type().map_err(|_| Error::internal())?;
            if ty.is_symlink() {
                return Err(refuse("generated tree contains a symlink"));
            }
            if ty.is_dir() {
                visit(root, &path, files)?;
            } else {
                files.insert(
                    path.strip_prefix(root)
                        .unwrap()
                        .to_string_lossy()
                        .into_owned(),
                    std::fs::read(path).map_err(|_| Error::internal())?,
                );
            }
        }
        Ok(())
    }
    let mut files = BTreeMap::new();
    visit(root, root, &mut files)?;
    Ok(files)
}

fn lower(spec: &Spec, out: &Path) -> Result<()> {
    let domain = format!("{}.requests", spec.id);
    let types:Vec<_>=spec.operations.iter().map(|o|Ok(json!({"name":format!("{domain}.{}",type_name(&o.id)),"kind":"struct","fields":fields(o)?.iter().map(|f|json!({"name":f.name,"type":f.ty})).collect::<Vec<_>>()}))).collect::<Result<_>>()?;
    write(
        &out.join("system.yaml"),
        &json_bytes(
            &json!({"format":"ess/1","system":spec.id,"version":"v1","domains":[domain],"summary":"Typed adapter request inputs. Connectors owns operation/result and provider-binding semantics."}),
        )?,
    )?;
    write(
        &out.join("domains/requests.yaml"),
        &json_bytes(&json!({"domain":domain,"types":types}))?,
    )?;
    write(
        &out.join("components.yaml"),
        &json_bytes(
            &json!({"components":[{"component":format!("{}-adapter",spec.id),"owns":{"domains":[domain]},"reached_by":"in_process"}]}),
        )?,
    )?;
    Ok(())
}

fn render(spec: &Spec) -> Result<String> {
    let mut s = format!(
        "// Generated by connectors-spec; ESS-generated inputs are used at dispatch.\nuse {}_types::requests::*;\nuse connectors_core::{{Descriptor,Error,ErrorCode,Result}};\nuse connectors_sdk::{{AuthenticatedHttp,HttpResponse}};\nuse serde_json::Value;\nuse std::sync::Arc;\n",
        spec.id
    );
    s += &format!(
        "pub fn verify_descriptor(descriptor: &Descriptor) -> Result<()> {{ connectors_sdk::verify_handlers(descriptor, &{:?}) }}\n",
        spec.mappings
            .iter()
            .map(|m| m.operation.as_str())
            .collect::<Vec<_>>()
    );
    for m in &spec.mappings {
        let ty = type_name(&m.operation);
        let bindings: BTreeSet<_> = m
            .path_parameters
            .values()
            .chain(m.query_parameters.values())
            .filter_map(|p| {
                if let Parameter::Binding { name } = p {
                    Some(name)
                } else {
                    None
                }
            })
            .collect();
        s += &format!("pub struct {ty}Context {{\n");
        for b in bindings {
            s += &format!("pub {}: String,\n", ident(b));
        }
        s += "}\n";
    }
    s += "pub trait Bindings: Send + Sync {\n";
    for m in &spec.mappings {
        let ty = type_name(&m.operation);
        let method = method_name(&m.operation);
        s += &format!(
            "fn prepare_{method}(&self, input: &{ty}) -> Result<{ty}Context>;\nfn finish_{method}(&self, input: {ty}, context: {ty}Context, response: HttpResponse) -> Result<Value>;\n"
        );
    }
    s += "}\n";
    s += "pub struct GeneratedAdapter<B: Bindings> { pub(crate) descriptor: Descriptor, pub(crate) http: Arc<dyn AuthenticatedHttp>, pub(crate) bindings: B }\n#[async_trait::async_trait]\nimpl<B: Bindings> connectors_sdk::Adapter for GeneratedAdapter<B> {\nfn descriptor(&self) -> Descriptor { self.descriptor.clone() }\nasync fn invoke(&self, operation: &str, input: Value) -> Result<Value> {\nlet op = self.descriptor.operation(operation)?; connectors_sdk::validate(&op.input_schema, &input)?;\nmatch operation {\n";
    for m in &spec.mappings {
        let op = spec
            .operations
            .iter()
            .find(|o| o.id == m.operation)
            .unwrap();
        let ty = type_name(&m.operation);
        let method = method_name(&m.operation);
        s += &format!("{:?} => {{ let args = {ty} {{\n", m.operation);
        for f in fields(op)? {
            let value = if f.nullable {
                format!("input[{:?}].as_str().map(str::to_owned)", f.name)
            } else if f.ty == "Integer" {
                format!(
                    "input[{:?}].as_i64().ok_or_else(|| Error::invalid(\"integer outside supported range\"))?",
                    f.name
                )
            } else {
                format!(
                    "input[{:?}].as_str().ok_or_else(|| Error::invalid(\"invalid string input\"))?.to_owned()",
                    f.name
                )
            };
            s += &format!("{}: {value},\n", ident(&f.name));
        }
        s += "};\n";
        s += &format!("let context = self.bindings.prepare_{method}(&args)?;\n");
        let expression = |p: &Parameter| match p {
            Parameter::Input { name } => format!("args.{}.to_string()", ident(name)),
            Parameter::Constant { value } => format!("{value:?}.to_owned()"),
            Parameter::Binding { name } => format!("context.{}.clone()", ident(name)),
        };
        let segments: Vec<_> = m
            .path
            .strip_prefix(&spec.upstream.base_path)
            .unwrap()
            .trim_start_matches('/')
            .split('/')
            .map(|segment| {
                if let Some(key) = segment.strip_prefix('{').and_then(|s| s.strip_suffix('}')) {
                    expression(&m.path_parameters[key])
                } else {
                    format!("{segment:?}.to_owned()")
                }
            })
            .collect();
        s += &format!(
            "let path = [{}];\nlet segments: Vec<&str> = path.iter().map(String::as_str).collect();\nlet query = [",
            segments.join(",")
        );
        for (name, value) in &m.query_parameters {
            s += &format!("({name:?}, {}),", expression(value));
        }
        s += "];\n";
        s += &format!(
            "let response = self.http.get(&segments, &query).await?;\nself.bindings.finish_{method}(args, context, response)\n}},\n"
        );
    }
    s += "_ => Err(Error::new(ErrorCode::NotFound, \"operation is not implemented\")),\n}\n}\n}\n";
    Ok(s)
}
