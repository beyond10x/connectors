//! The generic HTTP provider realization: one verified bundle, a declarative
//! selection of its operations, and one engine that binds, sends and classifies.
//! No operation here has its own Rust. Adding a supported endpoint changes the
//! selection and the pinned source; it changes nothing in this crate.
//!
//! Authority stays where it is: the host admits the connection, spends the
//! approval, records the attempt and supplies the one-use write capability. The
//! engine turns a declared operation plus validated input into exactly one
//! request and turns the response into an observation.
use connectors_catalog::{
    bundle::Bundle,
    inventory::{Location, Operation},
    template::Template,
};
use connectors_core::{Error, ErrorCode, Result};
use connectors_sdk::{
    AuthenticatedHttp, AuthenticatedWrite, HttpResponse, WriteMethod, WriteOutcome,
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::BTreeMap;

/// What a selected operation does to the provider. Declared, not inferred from
/// the method: an imported verb alone establishes no effect knowledge.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Effect {
    Read,
    Write,
}

/// What an observed value is compared with: the input value a reference names
/// (a top-level key, or `body.<key>`), or a literal written in the selection.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Expectation {
    Input(String),
    Literal(String),
}

/// One comparison: the scalar at `pointer` in an observed body must equal what
/// `expect` resolves to.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Check {
    pub pointer: String,
    pub expect: Expectation,
}

/// A read performed before a write, whose answer must satisfy every check.
/// `values` maps the probe operation's parameter keys to input references.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Preflight {
    pub operation_id: String,
    pub values: BTreeMap<String, String>,
    pub checks: Vec<Check>,
}

/// Comparisons against the write's own response body after dispatch.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Postflight {
    pub checks: Vec<Check>,
}

/// How a 2xx body is read. The bundle's declared media types decide by default;
/// `text` is the reviewed exception for a source that declares JSON where the
/// provider answers with plain text.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResponseKind {
    Json,
    Text,
}

/// The declarative head guard: a preflight read that refuses before any write
/// when the pinned value already differs, and a postflight comparison that
/// leaves the outcome uncertain — never refused — when it differs afterwards.
/// A provider that offers no precondition cannot be made to guard atomically;
/// this names that boundary as data rather than hiding it in a handler.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Guard {
    pub preflight: Preflight,
    pub postflight: Postflight,
}

/// One operation exposed from the bundle under a local id.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Selection {
    pub id: String,
    pub operation_id: String,
    pub effect: Effect,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub guard: Option<Guard>,
    #[serde(default)]
    pub response: Option<ResponseKind>,
}

/// An input reference: a top-level key, or `body.<key>` one level into the body.
fn reference<'a>(input: &'a Value, path: &str) -> Option<&'a Value> {
    let mut current = input;
    for key in path.split('.') {
        current = current.get(key)?;
    }
    Some(current)
}

/// A scalar as the string a parameter or a comparison carries.
fn scalar(value: &Value) -> Option<String> {
    match value {
        Value::String(text) => Some(text.clone()),
        Value::Number(number) => Some(number.to_string()),
        Value::Bool(flag) => Some(flag.to_string()),
        _ => None,
    }
}

fn refuse(message: impl Into<String>) -> Error {
    Error::invalid(message)
}

/// Raw segments under the base path, and raw query pairs.
type Request = (Vec<String>, Vec<(String, String)>);

struct Exposed {
    selection: Selection,
    operation: Operation,
    template: Template,
    declaration: connectors_core::Operation,
    text: bool,
}

struct Probe {
    template: Template,
}

/// The engine over one bundle. Built once per process; every selection is
/// checked against the inventory before any request can be made.
pub struct Engine {
    base_segments: Vec<String>,
    source_revision: String,
    exposed: Vec<Exposed>,
    probes: BTreeMap<String, Probe>,
}

/// One request, bound and ready to be sent exactly once.
pub struct Prepared {
    method: WriteMethod,
    segments: Vec<String>,
    query: Vec<(String, String)>,
    body: Value,
    postflight: Vec<(String, String)>,
    resource: String,
    instance: String,
    source_revision: String,
}

impl Engine {
    /// `base_path` is the path the provider authority already carries, such as
    /// `/api/v4`; every selected operation's declared path must start with it.
    pub fn new(bundle: &Bundle, base_path: &str, selections: &[Selection]) -> Result<Self> {
        let base_segments: Vec<String> = base_path
            .split('/')
            .filter(|s| !s.is_empty())
            .map(str::to_owned)
            .collect();
        if selections.is_empty() || selections.len() > 256 {
            return Err(refuse("select between one and 256 operations"));
        }
        let find = |operation_id: &str| -> Result<&Operation> {
            bundle
                .inventory
                .operations
                .iter()
                .find(|o| o.operation_id.as_deref() == Some(operation_id))
                .ok_or_else(|| refuse(format!("bundle carries no operation `{operation_id}`")))
        };
        let mut exposed = Vec::new();
        let mut probes = BTreeMap::new();
        let mut ids = std::collections::BTreeSet::new();
        for selection in selections {
            if !connectors_core::valid_id(&selection.id) || !ids.insert(selection.id.clone()) {
                return Err(refuse(format!(
                    "invalid or repeated selection id `{}`",
                    selection.id
                )));
            }
            let operation = find(&selection.operation_id)?.clone();
            let method = write_method(&operation.method);
            match (selection.effect, method) {
                (Effect::Read, None) if operation.method == "get" => {}
                (Effect::Write, Some(_)) => {}
                _ => {
                    return Err(refuse(format!(
                        "selection `{}` declares effect `{:?}` for method `{}`",
                        selection.id, selection.effect, operation.method
                    )));
                }
            }
            if operation
                .parameters
                .iter()
                .any(|p| p.location == Location::Header && p.required)
            {
                return Err(refuse(format!(
                    "selection `{}` needs a header parameter this transport does not carry",
                    selection.id
                )));
            }
            if !path_within(&operation.path, &base_segments) {
                return Err(refuse(format!(
                    "selection `{}` is outside the configured base path",
                    selection.id
                )));
            }
            let template =
                Template::from_operation(&operation).map_err(|refusal| refuse(refusal.reason()))?;
            if let Some(guard) = &selection.guard {
                if selection.effect != Effect::Write {
                    return Err(refuse(format!(
                        "selection `{}` guards a read",
                        selection.id
                    )));
                }
                let probe = find(&guard.preflight.operation_id)?.clone();
                if probe.method != "get" || !path_within(&probe.path, &base_segments) {
                    return Err(refuse(format!(
                        "guard of `{}` must read through a GET under the base path",
                        selection.id
                    )));
                }
                let checks = guard
                    .preflight
                    .checks
                    .iter()
                    .chain(&guard.postflight.checks)
                    .collect::<Vec<_>>();
                if guard.preflight.checks.is_empty()
                    || checks.len() > 16
                    || checks.iter().any(|c| !c.pointer.starts_with('/'))
                {
                    return Err(refuse(format!(
                        "guard of `{}` needs one to sixteen checks with JSON pointers",
                        selection.id
                    )));
                }
                let template =
                    Template::from_operation(&probe).map_err(|refusal| refuse(refusal.reason()))?;
                probes.insert(guard.preflight.operation_id.clone(), Probe { template });
            }
            if selection.response == Some(ResponseKind::Text) && selection.effect != Effect::Read {
                return Err(refuse(format!(
                    "selection `{}` declares a text response for a write",
                    selection.id
                )));
            }
            let declaration = declare(selection, &operation);
            let text = expects_text(selection, &operation);
            exposed.push(Exposed {
                selection: selection.clone(),
                operation,
                template,
                declaration,
                text,
            });
        }
        Ok(Self {
            base_segments,
            source_revision: bundle.source.source_sha256.clone(),
            exposed,
            probes,
        })
    }

    /// The declarations for the selected operations with these effects.
    pub fn declarations(&self, effects: &[Effect]) -> Vec<connectors_core::Operation> {
        self.exposed
            .iter()
            .filter(|e| effects.contains(&e.selection.effect))
            .map(|e| e.declaration.clone())
            .collect()
    }

    pub fn effect(&self, id: &str) -> Option<Effect> {
        self.exposed
            .iter()
            .find(|e| e.selection.id == id)
            .map(|e| e.selection.effect)
    }

    fn exposed(&self, id: &str) -> Result<&Exposed> {
        self.exposed
            .iter()
            .find(|e| e.selection.id == id)
            .ok_or_else(|| Error::new(ErrorCode::NotFound, "operation is not provided"))
    }

    /// Bind an operation's parameters from the input. Path and query values
    /// come from top-level keys named after the parameters.
    fn resolve(&self, template: &Template, values: BTreeMap<String, String>) -> Result<Request> {
        let resolved = template
            .resolve_raw(&values)
            .map_err(|refusal| refuse(refusal.reason()))?;
        let segments = resolved.segments[self.base_segments.len()..].to_vec();
        if segments.is_empty() {
            return Err(refuse("operation resolves to the bare base path"));
        }
        Ok((segments, resolved.query))
    }

    fn parameter_values(operation: &Operation, input: &Value) -> Result<BTreeMap<String, String>> {
        let mut values = BTreeMap::new();
        for parameter in &operation.parameters {
            if let Some(value) = input.get(&parameter.name) {
                let text = scalar(value).ok_or_else(|| {
                    refuse(format!("parameter `{}` is not a scalar", parameter.name))
                })?;
                values.insert(parameter.name.clone(), text);
            }
        }
        Ok(values)
    }

    /// One GET, projected as an observation.
    pub async fn read(
        &self,
        http: &dyn AuthenticatedHttp,
        instance: &str,
        id: &str,
        input: Value,
    ) -> Result<Value> {
        let exposed = self.exposed(id)?;
        if exposed.selection.effect != Effect::Read {
            return Err(Error::new(ErrorCode::Forbidden, "operation is a write"));
        }
        connectors_sdk::validate(&exposed.declaration.input_schema, &input)?;
        let values = Self::parameter_values(&exposed.operation, &input)?;
        let (segments, query) = self.resolve(&exposed.template, values)?;
        let borrowed: Vec<&str> = segments.iter().map(String::as_str).collect();
        let query: Vec<(&str, String)> =
            query.iter().map(|(k, v)| (k.as_str(), v.clone())).collect();
        let response = http.get(&borrowed, &query).await?;
        let body = read_body(&response, exposed.text)?;
        Ok(json!({
            "status": response.status,
            "body": body,
            "provenance": provenance(instance, &exposed.operation.path, &self.source_revision),
        }))
    }

    /// Bind a write and run its declared preflight. The returned request holds
    /// no read capability and sends exactly once.
    pub async fn prepare(
        &self,
        http: &dyn AuthenticatedHttp,
        instance: &str,
        id: &str,
        input: Value,
    ) -> Result<Prepared> {
        let exposed = self.exposed(id)?;
        let method = match (
            exposed.selection.effect,
            write_method(&exposed.operation.method),
        ) {
            (Effect::Write, Some(method)) => method,
            _ => return Err(Error::new(ErrorCode::Forbidden, "operation is a read")),
        };
        connectors_sdk::validate(&exposed.declaration.input_schema, &input)?;
        let values = Self::parameter_values(&exposed.operation, &input)?;
        let (segments, query) = self.resolve(&exposed.template, values)?;
        let body = if exposed.operation.request_media_types.is_empty() {
            Value::Null
        } else {
            input.get("body").cloned().unwrap_or(Value::Null)
        };
        let mut postflight = Vec::new();
        if let Some(guard) = &exposed.selection.guard {
            // Every expectation resolves before any request: an absent input is
            // a refusal with nothing sent, not a surprise after the preflight.
            let expected = |checks: &[Check]| -> Result<Vec<(String, String)>> {
                checks
                    .iter()
                    .map(|check| {
                        let value = match &check.expect {
                            Expectation::Input(path) => {
                                reference(&input, path).and_then(scalar).ok_or_else(|| {
                                    refuse(format!("guard expects input `{path}`, which is absent"))
                                })?
                            }
                            Expectation::Literal(text) => text.clone(),
                        };
                        Ok((check.pointer.clone(), value))
                    })
                    .collect()
            };
            let preflight = expected(&guard.preflight.checks)?;
            postflight = expected(&guard.postflight.checks)?;
            let probe = self
                .probes
                .get(&guard.preflight.operation_id)
                .ok_or_else(Error::internal)?;
            let mut values = BTreeMap::new();
            for (parameter, path) in &guard.preflight.values {
                let value = reference(&input, path)
                    .and_then(scalar)
                    .ok_or_else(|| refuse(format!("guard value `{path}` is absent")))?;
                values.insert(parameter.clone(), value);
            }
            let (segments, query) = self.resolve(&probe.template, values)?;
            let borrowed: Vec<&str> = segments.iter().map(String::as_str).collect();
            let query: Vec<(&str, String)> =
                query.iter().map(|(k, v)| (k.as_str(), v.clone())).collect();
            let response = http.get(&borrowed, &query).await?;
            // A target the preflight cannot find is a definite refusal: nothing
            // has been sent.
            if response.status == 404 {
                return Err(Error::new(
                    ErrorCode::Forbidden,
                    "guard target was not found before dispatch",
                ));
            }
            let observed = read_body(&response, false)?;
            for (pointer, expected) in &preflight {
                let current = observed.pointer(pointer).and_then(scalar).ok_or_else(|| {
                    Error::new(
                        ErrorCode::UpstreamProtocol,
                        format!("guard preflight answered without a value at `{pointer}`"),
                    )
                })?;
                // The only point at which a difference is definite. After
                // dispatch the same difference is uncertainty, not a refusal.
                if current != *expected {
                    return Err(Error::new(
                        ErrorCode::Forbidden,
                        format!("value at `{pointer}` differs from the pinned one before dispatch"),
                    ));
                }
            }
        }
        Ok(Prepared {
            method,
            segments,
            query,
            body,
            postflight,
            resource: exposed.operation.path.clone(),
            instance: instance.to_owned(),
            source_revision: self.source_revision.clone(),
        })
    }
}

impl Prepared {
    /// Send once and classify. Only documented definite refusals are refused;
    /// everything else that is not a success leaves the effect possible.
    pub async fn execute(self, capability: Box<dyn AuthenticatedWrite>) -> WriteOutcome<Value> {
        let segments: Vec<&str> = self.segments.iter().map(String::as_str).collect();
        let query: Vec<(&str, String)> = self
            .query
            .iter()
            .map(|(k, v)| (k.as_str(), v.clone()))
            .collect();
        let response = match capability
            .send_json(self.method, &segments, &query, &self.body)
            .await
        {
            Ok(response) => response,
            Err(_) => return WriteOutcome::Unknown(Error::unavailable()),
        };
        if matches!(
            response.status,
            400 | 401 | 403 | 404 | 405 | 409 | 410 | 412 | 415 | 422
        ) {
            return WriteOutcome::Refused(Error::new(
                ErrorCode::Forbidden,
                "provider refused the write",
            ));
        }
        if !(200..300).contains(&response.status) {
            return WriteOutcome::Unknown(Error::new(
                ErrorCode::UpstreamProtocol,
                "unconfirmed write outcome",
            ));
        }
        let body = match read_body(&response, false) {
            Ok(body) => body,
            Err(_) => {
                return WriteOutcome::Unknown(Error::new(
                    ErrorCode::UpstreamProtocol,
                    "write acknowledged with a body this engine cannot read",
                ));
            }
        };
        // The accepted race boundary: the provider offered no precondition, so
        // a pinned value that differs in the acknowledgement leaves the effect
        // possible. It is never reported as refused, and no corrective request
        // is issued. The comparison is best effort, not detection.
        for (pointer, expected) in &self.postflight {
            if body.pointer(pointer).and_then(scalar).as_deref() != Some(expected.as_str()) {
                return WriteOutcome::Unknown(Error::new(
                    ErrorCode::UpstreamProtocol,
                    format!(
                        "write acknowledged with a value at `{pointer}` other than the pinned one; the effect is possible"
                    ),
                ));
            }
        }
        WriteOutcome::Applied(Ok(json!({
            "status": response.status,
            "body": body,
            "provenance": provenance(&self.instance, &self.resource, &self.source_revision),
        })))
    }
}

fn write_method(method: &str) -> Option<WriteMethod> {
    match method {
        "post" => Some(WriteMethod::Post),
        "put" => Some(WriteMethod::Put),
        "patch" => Some(WriteMethod::Patch),
        "delete" => Some(WriteMethod::Delete),
        _ => None,
    }
}

fn path_within(path: &str, base: &[String]) -> bool {
    let segments: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();
    segments.len() > base.len() && segments.iter().zip(base).all(|(s, b)| *s == b)
}

/// Whether a read's 2xx body is text. The selection's reviewed exception wins;
/// otherwise the bundle decides: text when every declared 2xx media type is a
/// `text/` type, JSON when it declares JSON or nothing at all.
fn expects_text(selection: &Selection, operation: &Operation) -> bool {
    match selection.response {
        Some(kind) => kind == ResponseKind::Text,
        None => {
            let declared: Vec<&String> = operation
                .responses
                .iter()
                .filter(|r| r.status.starts_with('2'))
                .flat_map(|r| r.media_types.iter())
                .collect();
            !declared.is_empty() && declared.iter().all(|m| m.starts_with("text/"))
        }
    }
}

fn read_body(response: &HttpResponse, text: bool) -> Result<Value> {
    if !(200..300).contains(&response.status) {
        let code = match response.status {
            400 | 409 | 412 | 422 => ErrorCode::InvalidInput,
            401 => ErrorCode::Unauthorized,
            403 => ErrorCode::Forbidden,
            404 | 410 => ErrorCode::NotFound,
            429 => ErrorCode::RateLimited,
            500..=599 => ErrorCode::Unavailable,
            _ => ErrorCode::UpstreamProtocol,
        };
        return Err(Error::new(code, "provider refused the request"));
    }
    if response.body.is_empty() {
        return Ok(Value::Null);
    }
    let unreadable = || {
        Error::new(
            ErrorCode::UpstreamProtocol,
            "provider returned a body this engine cannot read",
        )
    };
    if text {
        return String::from_utf8(response.body.clone())
            .map(Value::String)
            .map_err(|_| unreadable());
    }
    connectors_core::read_json(&response.body).map_err(|_| unreadable())
}

fn provenance(instance: &str, resource: &str, source_revision: &str) -> Value {
    json!({
        "instance": instance,
        "resource": resource,
        "observed_at_unix_ms": connectors_sdk::now_ms(),
        "source_revision": source_revision,
    })
}

/// The descriptor operation for a selection: one property per declared path or
/// query parameter, a `body` object when the operation takes one, and one string
/// property per input reference a guard reads that no parameter covers.
fn declare(selection: &Selection, operation: &Operation) -> connectors_core::Operation {
    let mut properties = serde_json::Map::new();
    let mut required = Vec::new();
    for parameter in &operation.parameters {
        if parameter.location == Location::Header || parameter.location == Location::Cookie {
            continue;
        }
        properties.insert(
            parameter.name.clone(),
            json!({"type": ["string", "integer", "boolean"]}),
        );
        if parameter.required {
            required.push(parameter.name.clone());
        }
    }
    if !operation.request_media_types.is_empty() {
        properties.insert("body".into(), json!({"type": "object"}));
        required.push("body".into());
    }
    if let Some(guard) = &selection.guard {
        let mut references: Vec<&String> = guard.preflight.values.values().collect();
        references.extend(
            guard
                .preflight
                .checks
                .iter()
                .chain(&guard.postflight.checks)
                .filter_map(|check| match &check.expect {
                    Expectation::Input(path) => Some(path),
                    Expectation::Literal(_) => None,
                }),
        );
        for reference in references {
            if reference
                .split_once('.')
                .is_some_and(|(head, _)| head == "body")
            {
                continue;
            }
            if !properties.contains_key(reference.as_str()) {
                properties.insert(
                    reference.clone(),
                    json!({"type": ["string", "integer", "boolean"]}),
                );
                required.push(reference.clone());
            }
        }
    }
    required.sort();
    required.dedup();
    connectors_core::Operation {
        id: selection.id.clone(),
        description: selection.description.clone().unwrap_or_else(|| {
            format!(
                "{} {} from the {} bundle",
                operation.method.to_uppercase(),
                operation.path,
                selection.operation_id
            )
        }),
        contract: "operations/v1alpha1".into(),
        profile: match selection.effect {
            Effect::Read => "generic-http".into(),
            Effect::Write => "mutation".into(),
        },
        input_schema: json!({
            "type": "object",
            "properties": properties,
            "required": required,
            "additionalProperties": false,
        }),
        output_schema: json!({
            "type": "object",
            "properties": {
                "status": {"type": "integer", "minimum": 100, "maximum": 599},
                "body": {},
                "provenance": {
                    "type": "object",
                    "properties": {
                        "instance": {"type": "string", "minLength": 1, "maxLength": 512},
                        "resource": {"type": "string"},
                        "observed_at_unix_ms": {"type": "integer", "minimum": 0},
                        "source_revision": {"anyOf": [{"type": "string"}, {"type": "null"}]}
                    },
                    "required": ["instance", "resource", "observed_at_unix_ms", "source_revision"],
                    "additionalProperties": false
                }
            },
            "required": ["status", "body", "provenance"],
            "additionalProperties": false,
        }),
    }
}
