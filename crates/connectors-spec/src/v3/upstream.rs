use super::*;

fn resolve<'a>(source: &'a Value, mut value: &'a Value) -> Result<&'a Value> {
    let mut seen = BTreeSet::new();
    while let Some(reference) = value.get("$ref") {
        let r = reference
            .as_str()
            .ok_or_else(|| refuse("invalid source reference"))?;
        if value.as_object().is_none_or(|v| v.len() != 1)
            || !(r.starts_with("#/components/schemas/")
                || r.starts_with("#/components/requestBodies/"))
            || !seen.insert(r)
            || seen.len() > 64
        {
            return Err(refuse("unsupported, cyclic or ambiguous source reference"));
        }
        value = source
            .pointer(&r[1..])
            .ok_or_else(|| refuse("unresolved source reference"))?;
    }
    Ok(value)
}
fn scalar(source: &Value, ty: &str) -> Result<()> {
    let object = source
        .as_object()
        .ok_or_else(|| refuse("source scalar schema required"))?;
    for key in object.keys() {
        if ![
            "type",
            "oneOf",
            "nullable",
            "enum",
            "default",
            "description",
            "example",
            "deprecated",
            "minimum",
            "maximum",
            "minLength",
            "maxLength",
            "pattern",
            "format",
        ]
        .contains(&key.as_str())
        {
            return Err(refuse(format!("unsupported source scalar meaning: {key}")));
        }
    }
    if source.get("nullable").is_some_and(|v| !v.is_boolean()) {
        return Err(refuse("invalid source nullable declaration"));
    }
    let admitted = if let Some(branches) = source.get("oneOf") {
        let branches = branches
            .as_array()
            .ok_or_else(|| refuse("invalid scalar union"))?;
        let mut types = BTreeSet::new();
        if source.get("type").is_some()
            || branches.is_empty()
            || branches.len() > 3
            || branches.iter().any(|s| {
                s.as_object().is_none_or(|o| o.len() != 1)
                    || !matches!(s["type"].as_str(), Some("string" | "integer" | "boolean"))
                    || !types.insert(s["type"].as_str().unwrap_or(""))
            })
        {
            return Err(refuse("unsupported or overlapping scalar union"));
        }
        types.contains(ty)
    } else {
        source["type"] == ty
    };
    if !admitted
        || (source.get("format").is_some() && (ty != "string" || source["format"] != "date-time"))
    {
        return Err(refuse("source scalar type/format does not admit mapping"));
    }
    jsonschema::options()
        .should_validate_formats(true)
        .build(source)
        .map_err(|_| refuse("invalid source scalar constraints"))?;
    Ok(())
}
const CONSTRAINTS: [&str; 8] = [
    "minimum",
    "maximum",
    "minLength",
    "maxLength",
    "pattern",
    "enum",
    "format",
    "type",
];
fn input_compatible(source: &Value, input: &Value) -> Result<()> {
    let ty = input["type"]
        .as_str()
        .ok_or_else(|| refuse("missing scalar input type"))?;
    scalar(source, ty)?;
    for key in CONSTRAINTS.into_iter().filter(|k| *k != "type") {
        if source.get(key).is_some() && source[key] != input[key] {
            return Err(refuse(format!(
                "source constraint needs an exact input mapping: {key}"
            )));
        }
    }
    Ok(())
}
fn constant(source: &Value, value: Value) -> Result<()> {
    let ty = match &value {
        Value::Bool(_) => "boolean",
        Value::Number(n) if n.is_i64() => "integer",
        Value::String(_) => "string",
        _ => return Err(refuse("unsupported scalar constant")),
    };
    scalar(source, ty)?;
    if !jsonschema::options()
        .should_validate_formats(true)
        .build(source)
        .map_err(|_| refuse("invalid source scalar schema"))?
        .is_valid(&value)
    {
        return Err(refuse("constant violates source schema"));
    }
    Ok(())
}

pub(super) fn import(spec: &Spec, path: &Path) -> Result<(Value, Value)> {
    let Source {
        sha256,
        path: relative,
        ..
    } = &spec.read.upstream;
    let bytes = std::fs::read(path.parent().unwrap_or(Path::new(".")).join(relative))
        .map_err(|_| refuse("cannot read pinned upstream source"))?;
    if bytes.len() > 16 * 1024 * 1024 || hash(&bytes) != *sha256 {
        return Err(refuse("upstream digest mismatch or byte limit exceeded"));
    }
    let yaml: serde_yaml_ng::Value =
        serde_yaml_ng::from_slice(&bytes).map_err(|_| refuse("invalid upstream YAML"))?;
    let source = serde_json::to_value(yaml).map_err(|_| refuse("source is not JSON-compatible"))?;
    if !source["openapi"]
        .as_str()
        .is_some_and(|v| v.starts_with("3.0.") || v.starts_with("3.1."))
    {
        return Err(refuse("unsupported OpenAPI version"));
    }
    let mut paths = json!({});
    let mut coverage = Vec::new();
    for w in &spec.writes {
        let m = &w.mapping;
        let item = &source["paths"][&m.path];
        let operation = &item["put"];
        if operation["operationId"] != m.upstream_operation || item.get("$ref").is_some() {
            return Err(refuse("missing or changed upstream write operation"));
        }
        if [item, operation]
            .iter()
            .any(|v| v.get("parameters").is_some_and(|p| !p.is_array()))
        {
            return Err(refuse("source parameters must be arrays"));
        }
        let parameters: Vec<_> = item["parameters"]
            .as_array()
            .into_iter()
            .flatten()
            .chain(operation["parameters"].as_array().into_iter().flatten())
            .collect();
        let mut source_parameters = BTreeMap::new();
        for p in parameters {
            if p.get("$ref").is_some() {
                return Err(refuse("referenced parameters need a supported import"));
            }
            if [
                "required",
                "explode",
                "allowReserved",
                "allowEmptyValue",
                "deprecated",
            ]
            .iter()
            .any(|k| p.get(k).is_some_and(|v| !v.is_boolean()))
            {
                return Err(refuse("invalid source parameter boolean"));
            }
            let name = p["name"]
                .as_str()
                .ok_or_else(|| refuse("missing source parameter name"))?;
            let place = p["in"]
                .as_str()
                .ok_or_else(|| refuse("missing source parameter location"))?;
            if source_parameters.insert((place, name), p).is_some() {
                return Err(refuse("ambiguous duplicate source parameter"));
            }
        }
        let mut mapped = BTreeSet::new();
        for (place, bindings) in [("path", &m.path_parameters), ("query", &m.query_parameters)] {
            for (name, binding) in bindings {
                let parameter = source_parameters
                    .get(&(place, name.as_str()))
                    .ok_or_else(|| refuse("mapped parameter absent upstream"))?;
                if parameter.get("content").is_some()
                    || parameter
                        .get("style")
                        .is_some_and(|s| s != if place == "path" { "simple" } else { "form" })
                    || parameter["allowReserved"] == true
                    || parameter["allowEmptyValue"] == true
                    || (place == "path" && parameter["required"] != true)
                {
                    return Err(refuse("unsupported source parameter encoding"));
                }
                let schema = resolve(&source, &parameter["schema"])?;
                match binding {
                    Parameter::Input { name } => {
                        input_compatible(schema, &w.operation.input_schema["properties"][name])?
                    }
                    Parameter::Constant { value } => constant(schema, json!(value))?,
                    Parameter::Binding { .. } => {
                        // Context fields have the concrete type String. An
                        // integer-only source needs an integer input mapping.
                        scalar(schema, "string")?;
                        if CONSTRAINTS
                            .into_iter()
                            .filter(|k| *k != "type")
                            .any(|k| schema.get(k).is_some())
                        {
                            return Err(refuse(
                                "constrained context parameter needs typed mapping",
                            ));
                        }
                    }
                }
                mapped.insert((place, name.as_str()));
            }
        }
        let mut excluded = Vec::new();
        for ((place, name), p) in source_parameters {
            if !mapped.contains(&(place, name)) {
                if p["required"] == true {
                    return Err(refuse("required source parameter is unbound"));
                }
                excluded.push(format!("{place}:{name}"));
            }
        }
        let request = resolve(&source, &operation["requestBody"])?;
        if request.get("required").is_some_and(|v| !v.is_boolean()) {
            return Err(refuse("invalid body requirement"));
        }
        let content = request["content"]
            .as_object()
            .ok_or_else(|| refuse("write needs JSON request body"))?;
        let media = content
            .get("application/json")
            .ok_or_else(|| refuse("JSON request body unavailable"))?;
        if media.get("encoding").is_some() {
            return Err(refuse("custom body encoding unsupported"));
        }
        let body = resolve(&source, &media["schema"])?;
        if body["type"] != "object"
            || body.as_object().is_none_or(|o| {
                o.keys().any(|k| {
                    ![
                        "type",
                        "properties",
                        "required",
                        "additionalProperties",
                        "description",
                        "title",
                    ]
                    .contains(&k.as_str())
                })
            })
        {
            return Err(refuse("unsupported request body object semantics"));
        }
        let properties = body["properties"]
            .as_object()
            .ok_or_else(|| refuse("body properties missing"))?;
        if body.get("required").is_some_and(|v| !v.is_array())
            || body
                .get("additionalProperties")
                .is_some_and(|v| !v.is_boolean())
        {
            return Err(refuse("invalid or unsupported body object constraint"));
        }
        let required: Vec<_> = body["required"].as_array().into_iter().flatten().collect();
        if required
            .iter()
            .filter_map(|v| v.as_str())
            .collect::<BTreeSet<_>>()
            .len()
            != required.len()
        {
            return Err(refuse("invalid or duplicate required body property"));
        }
        for name in required {
            if name
                .as_str()
                .is_none_or(|n| !properties.contains_key(n) || !m.body.contains_key(n))
            {
                return Err(refuse("required source body property is unbound"));
            }
        }
        for (name, p) in &m.body {
            let property = resolve(
                &source,
                properties
                    .get(name)
                    .ok_or_else(|| refuse("mapped body property absent upstream"))?,
            )?;
            match p {
                BodyParameter::Input { name } => {
                    input_compatible(property, &w.operation.input_schema["properties"][name])?
                }
                BodyParameter::String { value } => constant(property, json!(value))?,
                BodyParameter::Boolean { value } => constant(property, json!(value))?,
                BodyParameter::Integer { value } => constant(property, json!(value))?,
            }
        }
        let mut selected = json!({"put":operation});
        if let Some(p) = item.get("parameters") {
            selected["parameters"] = p.clone();
        }
        paths[&m.path] = selected;
        coverage.push(json!({"operation":w.operation.id,"source_operation":m.upstream_operation,"method":"put","path":m.path,"mapped_body_properties":m.body.keys().collect::<Vec<_>>(),"excluded_optional_parameters":excluded,"excluded_optional_body_properties":properties.keys().filter(|n|!m.body.contains_key(*n)).collect::<Vec<_>>(),"generated":["typed_input","typed_output","immutable_request","consuming_dispatch","private_descriptor"],"authority":"consumer-supplied one-use capability; no approval or credential grant","requires_implementation":["native read-only preflight","native effect and response interpretation"]}));
    }
    let mut selected =
        json!({"openapi":source["openapi"],"info":source["info"],"paths":paths,"components":{}});
    for k in ["servers", "security"] {
        if let Some(v) = source.get(k) {
            selected[k] = v.clone();
        }
    }
    if let Some(v) = source.pointer("/components/securitySchemes") {
        selected["components"]["securitySchemes"] = v.clone();
    }
    fn refs(v: &Value, out: &mut BTreeSet<String>) {
        match v {
            Value::Object(o) => {
                if let Some(Value::String(s)) = o.get("$ref") {
                    out.insert(s.clone());
                }
                for v in o.values() {
                    refs(v, out)
                }
            }
            Value::Array(a) => {
                for v in a {
                    refs(v, out)
                }
            }
            _ => {}
        }
    }
    let mut retained = BTreeSet::new();
    loop {
        let mut found = BTreeSet::new();
        refs(&selected, &mut found);
        let pending: Vec<_> = found.difference(&retained).cloned().collect();
        if pending.is_empty() {
            break;
        }
        for r in pending {
            let rest = r
                .strip_prefix("#/components/")
                .ok_or_else(|| refuse("external reference in selected source"))?;
            let (kind, name) = rest
                .split_once('/')
                .ok_or_else(|| refuse("invalid component reference"))?;
            if !["schemas", "requestBodies"].contains(&kind) || name.contains('/') {
                return Err(refuse("unsupported component reference"));
            }
            let value = source
                .pointer(&r[1..])
                .ok_or_else(|| refuse("unresolved selected reference"))?;
            if selected["components"].get(kind).is_none() {
                selected["components"][kind] = json!({});
            }
            selected["components"][kind][name.replace("~1", "/").replace("~0", "~")] =
                value.clone();
            retained.insert(r);
            if retained.len() > 4096 {
                return Err(refuse("reference closure exceeds bound"));
            }
        }
    }
    Ok((
        selected,
        json!({"format":"connectors.write-import-coverage/v1","source":spec.read.upstream,"selected_operations":coverage,"retained_schema_references":retained}),
    ))
}
