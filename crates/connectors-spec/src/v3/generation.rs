use super::*;
use crate::v2::{ess, ident, tree};
use std::process::Command;

/// A deliberately closed codec subset. Constraints remain in the descriptor
/// and are checked at the boundary; ESS supplies the concrete value types.
enum Ty {
    String,
    Integer,
    Boolean,
    Object(String),
    Optional(Box<Ty>),
}
impl Ty {
    fn ess(&self, domain: &str) -> String {
        match self {
            Self::String => "String".into(),
            Self::Integer => "Integer".into(),
            Self::Boolean => "Boolean".into(),
            Self::Object(name) => format!("{domain}.{name}"),
            Self::Optional(ty) => format!("Optional<{}>", ty.ess(domain)),
        }
    }
    fn encode(&self, value: &str) -> String {
        match self {
            Self::Object(name) => format!("encode_{name}({value})"),
            Self::Optional(ty) => format!(
                "match {value} {{ Some(value) => {}, None => Value::Null }}",
                ty.encode("value")
            ),
            _ => format!("serde_json::json!({value})"),
        }
    }
}
pub(super) struct OutputModel {
    name: String,
    objects: BTreeMap<String, Vec<(String, Ty)>>,
}
fn keys(schema: &Value, permitted: &[&str]) -> Result<()> {
    let object = schema
        .as_object()
        .ok_or_else(|| refuse("schema object required"))?;
    if object.keys().any(|k| !permitted.contains(&k.as_str())) {
        return Err(refuse("unsupported write codec schema keyword"));
    }
    Ok(())
}
pub(super) fn scalar_model(schema: &Value) -> Result<()> {
    let common = ["type", "description", "title", "enum", "const"];
    let mut permitted = common.to_vec();
    match schema["type"].as_str() {
        Some("string") => {
            permitted.extend(["minLength", "maxLength", "pattern", "format"]);
            if schema.get("format").is_some_and(|v| v != "date-time") {
                return Err(refuse("unsupported write string format"));
            }
        }
        Some("integer") => permitted.extend(["minimum", "maximum"]),
        Some("boolean") => {}
        _ => return Err(refuse("unsupported write scalar type")),
    }
    keys(schema, &permitted)
}
pub(super) fn object_fields(schema: &Value) -> Result<&serde_json::Map<String, Value>> {
    keys(
        schema,
        &[
            "type",
            "properties",
            "required",
            "additionalProperties",
            "title",
            "description",
        ],
    )?;
    let properties = schema["properties"]
        .as_object()
        .ok_or_else(|| refuse("write object needs properties"))?;
    let required = schema["required"]
        .as_array()
        .ok_or_else(|| refuse("write object needs required fields"))?;
    if schema["type"] != "object"
        || schema["additionalProperties"] != false
        || properties.len() > 256
        || required.len() != properties.len()
        || required
            .iter()
            .filter_map(Value::as_str)
            .collect::<BTreeSet<_>>()
            != properties.keys().map(String::as_str).collect()
        || properties.keys().any(|name| !field_name(name))
    {
        return Err(refuse(
            "write codecs require closed objects with every field required",
        ));
    }
    Ok(properties)
}
fn output_name(id: &str) -> String {
    format!("{}Output", type_name(id).trim_end_matches("Request"))
}
pub(super) fn output_model(op: &Operation) -> Result<OutputModel> {
    fn lower(schema: &Value, name: String, model: &mut OutputModel, depth: usize) -> Result<Ty> {
        if depth > 16 {
            return Err(refuse("write output exceeds depth bound"));
        }
        if let Some(branches) = schema.get("anyOf") {
            keys(schema, &["anyOf", "title", "description"])?;
            let branches = branches
                .as_array()
                .ok_or_else(|| refuse("invalid nullable output"))?;
            let non_null: Vec<_> = branches
                .iter()
                .filter(|v| **v != json!({"type":"null"}))
                .collect();
            if branches.len() != 2 || non_null.len() != 1 || non_null[0].get("anyOf").is_some() {
                return Err(refuse("output supports only one explicit nullable branch"));
            }
            return Ok(Ty::Optional(Box::new(lower(
                non_null[0],
                name,
                model,
                depth + 1,
            )?)));
        }
        if schema["type"] == "object" {
            if model.objects.len() >= 256 || model.objects.contains_key(&name) {
                return Err(refuse("colliding or excessive generated output types"));
            }
            model.objects.insert(name.clone(), Vec::new());
            let mut fields = Vec::new();
            for (field, schema) in object_fields(schema)? {
                let nested = format!("{name}{}", type_name(field).trim_end_matches("Request"));
                fields.push((field.clone(), lower(schema, nested, model, depth + 1)?));
            }
            model.objects.insert(name.clone(), fields);
            return Ok(Ty::Object(name));
        }
        scalar_model(schema)?;
        Ok(match schema["type"].as_str() {
            Some("string") => Ty::String,
            Some("integer") => Ty::Integer,
            Some("boolean") => Ty::Boolean,
            _ => return Err(refuse("unsupported output type")),
        })
    }
    let name = output_name(&op.id);
    let mut model = OutputModel {
        name: name.clone(),
        objects: BTreeMap::new(),
    };
    if !matches!(
        lower(&op.output_schema, name, &mut model, 0)?,
        Ty::Object(_)
    ) {
        return Err(refuse("write output root must be a closed object"));
    }
    Ok(model)
}
pub(super) fn validate_names(spec: &Spec) -> Result<()> {
    let mut names = BTreeSet::new();
    for w in &spec.writes {
        let name = type_name(&w.operation.id);
        for name in [name.clone(), format!("{name}Context")] {
            if !names.insert(name) {
                return Err(refuse("colliding generated write type"));
            }
        }
        for name in output_model(&w.operation)?.objects.into_keys() {
            if !names.insert(name) {
                return Err(refuse("colliding generated write type"));
            }
        }
    }
    Ok(())
}

fn lower(spec: &Spec, out: &Path) -> Result<()> {
    let system = format!("{}_writes", spec.read.id);
    let domain = format!("{system}.requests");
    let mut types = Vec::new();
    for w in &spec.writes {
        let fields: Vec<_> = input_fields(&w.operation)?
            .into_iter()
            .map(|(name, schema)| {
                let ty = match schema["type"].as_str() {
                    Some("string") => "String",
                    Some("integer") => "Integer",
                    _ => "Boolean",
                };
                json!({"name":name,"type":ty})
            })
            .collect();
        types.push(json!({"name":format!("{domain}.{}",type_name(&w.operation.id)),"kind":"struct","fields":fields}));
        for (name, fields) in output_model(&w.operation)?.objects {
            types.push(json!({"name":format!("{domain}.{name}"),"kind":"struct","fields":fields.into_iter().map(|(name,ty)|json!({"name":name,"type":ty.ess(&domain)})).collect::<Vec<_>>()}));
        }
    }
    write(
        &out.join("system.yaml"),
        &json_bytes(
            &json!({"format":"ess/1","system":system,"version":"v1","domains":[domain],"summary":"Typed local write values. Consumer authority and native effect interpretation remain implementation obligations."}),
        )?,
    )?;
    write(
        &out.join("domains/requests.yaml"),
        &json_bytes(&json!({"domain":domain,"types":types}))?,
    )?;
    write(
        &out.join("components.yaml"),
        &json_bytes(
            &json!({"components":[{"component":format!("{}-writes-adapter",spec.read.id.replace('_',"-")),"owns":{"domains":[domain]},"reached_by":"in_process"}]}),
        )?,
    )
}

fn render(spec: &Spec) -> Result<String> {
    let mut s = format!(
        "// Generated by connectors-spec. No public write dispatch or host authority.\nuse {}_writes_types::requests::*;\nuse connectors_core::{{Descriptor, Error, ErrorCode, Result}};\nuse connectors_sdk::{{AuthenticatedWrite, HttpResponse, WriteOutcome}};\nuse serde_json::Value;\nuse std::sync::Arc;\n",
        spec.read.id
    );
    for w in &spec.writes {
        let ty = type_name(&w.operation.id);
        let bindings: BTreeSet<_> = w
            .mapping
            .path_parameters
            .values()
            .chain(w.mapping.query_parameters.values())
            .filter_map(|p| {
                if let Parameter::Binding { name } = p {
                    Some(name)
                } else {
                    None
                }
            })
            .collect();
        s += &format!("pub struct {ty}Context {{\n");
        for name in bindings {
            s += &format!("pub {}: String,\n", ident(name));
        }
        s += "}\n";
        for (name, fields) in output_model(&w.operation)?.objects {
            s += &format!(
                "#[allow(non_snake_case)]\nfn encode_{name}(value: {name}) -> Value {{\nserde_json::json!({{\n"
            );
            for (name, ty) in fields {
                s += &format!(
                    "{name:?}: {},\n",
                    ty.encode(&format!("value.{}", ident(&name)))
                );
            }
            s += "})\n}\n";
        }
    }
    s += "pub trait WriteBindings: Send + Sync {\n";
    for w in &spec.writes {
        let ty = type_name(&w.operation.id);
        let method = method_name(&w.operation.id);
        let out = output_name(&w.operation.id);
        s += &format!(
            "fn prepare_{method}(&self, input: &{ty}) -> Result<{ty}Context>;\nfn finish_{method}(&self, input: {ty}, context: {ty}Context, response: Result<HttpResponse>) -> WriteOutcome<{out}>;\n"
        );
    }
    s += "}\n\n// All variants and fields are private. Only prepare constructs this value.\nenum Request {\n";
    for w in &spec.writes {
        let ty = type_name(&w.operation.id);
        s += &format!("{ty}({ty}, {ty}Context),\n");
    }
    s += "}\npub struct PreparedWrite<B: WriteBindings> { request: Request, bindings: Arc<B>, path: Vec<String>, query: Vec<(&'static str, String)>, body: Value, output_schema: Value }\n";
    s += "pub fn prepare<B: WriteBindings>(descriptor: &Descriptor, bindings: Arc<B>, operation: &str, input: Value) -> Result<PreparedWrite<B>> {\nlet expected: Descriptor = serde_json::from_str(include_str!(\"private-descriptor.json\")).map_err(|_| Error::internal())?;\nif descriptor.adapter != expected.adapter || descriptor.configuration_schema != expected.configuration_schema || descriptor.operations != expected.operations { return Err(Error::new(ErrorCode::StaleDescription, \"write declarations do not match generated binding\")); }\nlet op = descriptor.operation(operation)?;\nconnectors_sdk::validate_write_value(&op.input_schema, &input)?;\nmatch operation {\n";
    for w in &spec.writes {
        let ty = type_name(&w.operation.id);
        let method = method_name(&w.operation.id);
        let m = &w.mapping;
        s += &format!("{:?} => {{\nlet args = {ty} {{\n", w.operation.id);
        for (name, schema) in input_fields(&w.operation)? {
            let conversion = match schema["type"].as_str() {
                Some("string") => format!(
                    "input[{name:?}].as_str().ok_or_else(|| Error::invalid(\"invalid write string\"))?.to_owned()"
                ),
                Some("integer") => format!(
                    "input[{name:?}].as_i64().ok_or_else(|| Error::invalid(\"write integer outside supported range\"))?"
                ),
                _ => format!(
                    "input[{name:?}].as_bool().ok_or_else(|| Error::invalid(\"invalid write boolean\"))?"
                ),
            };
            s += &format!("{}: {conversion},\n", ident(name));
        }
        s += &format!("}};\nlet context = bindings.prepare_{method}(&args)?;\n");
        let expression = |p: &Parameter| match p {
            Parameter::Input { name } => format!("args.{}.to_string()", ident(name)),
            Parameter::Constant { value } => format!("{value:?}.to_owned()"),
            Parameter::Binding { name } => format!("context.{}.clone()", ident(name)),
        };
        let segments: Vec<_> = m
            .path
            .strip_prefix(&spec.read.upstream.base_path)
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
            "let path = vec![{}];\nlet query = vec![",
            segments.join(",")
        );
        for (name, p) in &m.query_parameters {
            s += &format!("({name:?}, {}),", expression(p));
        }
        s += "];\nlet body = serde_json::json!({\n";
        for (name, p) in &m.body {
            let expression = match p {
                BodyParameter::Input { name } => format!("args.{}", ident(name)),
                BodyParameter::String { value } => format!("{value:?}"),
                BodyParameter::Boolean { value } => value.to_string(),
                BodyParameter::Integer { value } => format!("{value}i64"),
            };
            s += &format!("{name:?}: {expression},\n");
        }
        s += &format!(
            "}});\nOk(PreparedWrite {{ request: Request::{ty}(args, context), bindings, path, query, body, output_schema: op.output_schema.clone() }})\n}},\n"
        );
    }
    s += "_ => Err(Error::new(ErrorCode::NotFound, \"operation is not a generated write\")),\n}\n}\n";
    s += "impl<B: WriteBindings> PreparedWrite<B> {\npub async fn execute(self, capability: Box<dyn AuthenticatedWrite>) -> WriteOutcome<Value> {\nlet segments: Vec<&str> = self.path.iter().map(String::as_str).collect();\nlet response = capability.put_json(&segments, &self.query, &self.body).await;\nmatch self.request {\n";
    for w in &spec.writes {
        let ty = type_name(&w.operation.id);
        let method = method_name(&w.operation.id);
        let out = output_model(&w.operation)?.name;
        s += &format!(
            "Request::{ty}(args, context) => self.bindings.finish_{method}(args, context, response).try_map(|value| {{\nlet output = encode_{out}(value);\nconnectors_sdk::validate_write_value(&self.output_schema, &output).map_err(|_| Error::new(ErrorCode::UpstreamProtocol, \"invalid native write result\"))?;\nOk(output)\n}}),\n"
        );
    }
    s += "}\n}\n}\n";
    Ok(s)
}

pub(super) fn build(spec: &Spec, root: &Path, executable: &Path) -> Result<()> {
    lower(spec, &root.join("write-ess"))?;
    let model = root.join("write-ess");
    let generated = root.join("write-rust");
    let model = model.to_str().ok_or_else(Error::internal)?;
    ess(executable, &["specify", "validate", "--path", model])?;
    write(
        &root.join("write-ess-ir.json"),
        &ess(
            executable,
            &["specify", "compile", "--path", model, "--format", "json"],
        )?,
    )?;
    ess(
        executable,
        &[
            "generate",
            "synthesize",
            "--path",
            model,
            "--target",
            "rust",
            "--out",
            generated.to_str().ok_or_else(Error::internal)?,
        ],
    )?;
    write(&root.join("writes.rs"), render(spec)?.as_bytes())?;
    let paths = std::iter::once(root.join("writes.rs")).chain(
        tree(&generated)?
            .into_keys()
            .filter(|p| p.ends_with(".rs"))
            .map(|p| generated.join(p)),
    );
    for path in paths {
        if !Command::new("rustfmt")
            .args(["--edition", "2021"])
            .arg(path)
            .status()
            .map_err(|_| refuse("cannot format generated write Rust"))?
            .success()
        {
            return Err(refuse("generated write Rust did not format"));
        }
    }
    Ok(())
}
