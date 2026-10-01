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
    inventory::{Location, Operation, ValueType},
    template::{Supplied, Template},
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

/// A narrower range for one query parameter than the pinned source declares,
/// such as a provider's page-size cap. The value is read as an integer.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Bound {
    /// Omitted when absent, so a maximum-only bound serialises unchanged.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub minimum: Option<u64>,
    pub maximum: u64,
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
    /// Bounds keyed by query parameter name; omitted when there are none, so an
    /// unbounded selection serialises as it did before bounds existed.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub bounds: BTreeMap<String, Bound>,
    /// Query parameters the provider requires although the pinned document
    /// does not mark them required. Each must be a query parameter of the
    /// operation; it is then declared required and refused when absent, before
    /// any request. Omitted when empty.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub required: Vec<String>,
    /// Declared parameters this selection does not expose, such as a paging
    /// mode the provider cannot follow. Each must be a parameter of the
    /// operation that nothing requires, that is not bounded and that no guard
    /// reads as an input. It is left out of the declaration, so an input
    /// carrying it is refused before any request. Omitted when empty.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub withhold: Vec<String>,
    /// The reasons a provider gives in a `403` answer when it means a quota,
    /// not a permission: a `403` whose JSON body carries one of them in
    /// `error.errors[].reason` or `error.status` is `rate_limited`. Every other
    /// `403` stays `forbidden`. Omitted when empty.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub rate_limit_reasons: Vec<String>,
    /// The only top-level keys a write's JSON body may carry. When set, the
    /// declaration types `body` as an object with exactly these properties,
    /// and a body carrying any key outside the set is refused before any
    /// request. Keys a guard reads are also compared by that guard, and are
    /// declared required, as scalars when the guard reads them directly; a
    /// key no guard reads is admitted with any value and compared by nothing.
    /// Omitted when empty, which leaves the body an open object.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub body_keys: Vec<String>,
}

/// The body paths a guard reads, each the part after `body.` of a reference.
fn guarded_body_paths(guard: &Guard) -> impl Iterator<Item = &str> {
    guard
        .preflight
        .values
        .values()
        .chain(
            guard
                .preflight
                .checks
                .iter()
                .chain(&guard.postflight.checks)
                .filter_map(|check| match &check.expect {
                    Expectation::Input(path) => Some(path),
                    Expectation::Literal(_) => None,
                }),
        )
        .filter_map(|path| path.strip_prefix("body."))
}

/// The top-level body keys a guard reads through `body.<key>` references.
fn guarded_body_keys(guard: &Guard) -> std::collections::BTreeSet<&str> {
    guarded_body_paths(guard)
        .map(|rest| rest.split('.').next().unwrap_or(rest))
        .collect()
}

/// The top-level body keys a guard reads as scalars: referenced as
/// `body.<key>` itself, not only through a path nested under it.
fn scalar_body_keys(guard: &Guard) -> std::collections::BTreeSet<&str> {
    guarded_body_paths(guard)
        .filter(|rest| !rest.contains('.'))
        .collect()
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

/// Every bounded value present must be a decimal integer within its bound; a
/// repeated parameter's bound holds for each element. Checked on the bound
/// value strings, before any request.
fn check_bounds(selection: &Selection, values: &BTreeMap<String, Supplied>) -> Result<()> {
    for (name, bound) in &selection.bounds {
        let texts = match values.get(name) {
            None => continue,
            Some(Supplied::One(text)) => std::slice::from_ref(text),
            Some(Supplied::Many(texts)) => texts.as_slice(),
        };
        for text in texts {
            check_bound(name, bound, text)?;
        }
    }
    Ok(())
}

fn check_bound(name: &str, bound: &Bound, text: &str) -> Result<()> {
    let digits = text.strip_prefix('-').unwrap_or(text);
    let value = (!digits.is_empty() && digits.bytes().all(|b| b.is_ascii_digit()))
        .then(|| text.parse::<i128>().ok())
        .flatten()
        .ok_or_else(|| refuse(format!("parameter `{name}` is not an integer")))?;
    if value > i128::from(bound.maximum) {
        return Err(refuse(format!(
            "parameter `{name}` is above its maximum of {}",
            bound.maximum
        )));
    }
    if let Some(minimum) = bound.minimum
        && value < i128::from(minimum)
    {
        return Err(refuse(format!(
            "parameter `{name}` is below its minimum of {minimum}"
        )));
    }
    Ok(())
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
    rate_limit_reasons: Vec<String>,
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
            let mut operation = find(&selection.operation_id)?.clone();
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
            if let Some(name) = selection.bounds.keys().find(|name| {
                !operation
                    .parameters
                    .iter()
                    .any(|p| &p.name == *name && p.location == Location::Query)
            }) {
                return Err(refuse(format!(
                    "selection `{}` bounds `{name}`, which is not a query parameter of its operation",
                    selection.id
                )));
            }
            if let Some((name, _)) = selection
                .bounds
                .iter()
                .find(|(_, bound)| bound.minimum.is_some_and(|minimum| minimum > bound.maximum))
            {
                return Err(refuse(format!(
                    "selection `{}` bounds `{name}` with a minimum above its maximum",
                    selection.id
                )));
            }
            // The provider's requirement, where the document omits it: marked on
            // the operation this selection exposes, so the declaration lists it
            // and the template refuses its absence, both before any request.
            for name in &selection.required {
                let mut marked = false;
                for parameter in operation
                    .parameters
                    .iter_mut()
                    .filter(|p| &p.name == name && p.location == Location::Query)
                {
                    parameter.required = true;
                    marked = true;
                }
                if !marked {
                    return Err(refuse(format!(
                        "selection `{}` marks `{name}` required, which is not a query parameter of its operation",
                        selection.id
                    )));
                }
            }
            // A withheld parameter leaves the operation this selection
            // exposes: undeclared, so the closed input schema refuses it, and
            // absent from the template, so it is never bound into a request.
            let guard_inputs: Vec<&str> = selection
                .guard
                .iter()
                .flat_map(|guard| {
                    guard.preflight.values.values().chain(
                        guard
                            .preflight
                            .checks
                            .iter()
                            .chain(&guard.postflight.checks)
                            .filter_map(|check| match &check.expect {
                                Expectation::Input(path) => Some(path),
                                Expectation::Literal(_) => None,
                            }),
                    )
                })
                .map(|path| path.split('.').next().unwrap_or(path))
                .collect();
            for name in &selection.withhold {
                let declared: Vec<_> = operation
                    .parameters
                    .iter()
                    .filter(|p| &p.name == name)
                    .collect();
                let reason = if declared.is_empty() {
                    Some("which is not a parameter of its operation")
                } else if declared.iter().any(|p| p.required) {
                    Some("which is required")
                } else if selection.bounds.contains_key(name) {
                    Some("which it also bounds")
                } else if guard_inputs.contains(&name.as_str()) {
                    Some("which its guard reads as an input")
                } else {
                    None
                };
                if let Some(reason) = reason {
                    return Err(refuse(format!(
                        "selection `{}` withholds `{name}`, {reason}",
                        selection.id
                    )));
                }
            }
            operation
                .parameters
                .retain(|p| !selection.withhold.contains(&p.name));
            if selection.rate_limit_reasons.iter().any(String::is_empty) {
                return Err(refuse(format!(
                    "selection `{}` declares an empty rate-limit reason",
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
            if !selection.body_keys.is_empty() {
                let mut names = std::collections::BTreeSet::new();
                if selection.effect != Effect::Write
                    || operation.request_media_types.is_empty()
                    || selection
                        .body_keys
                        .iter()
                        .any(|key| key.is_empty() || key.contains('.') || !names.insert(key))
                {
                    return Err(refuse(format!(
                        "selection `{}` declares body_keys that do not close a write's body with distinct plain names",
                        selection.id
                    )));
                }
                // A guard reading a key the closed body cannot carry could
                // never be satisfied: refused here, not at every write.
                if let Some(key) = selection.guard.as_ref().and_then(|guard| {
                    guarded_body_keys(guard)
                        .into_iter()
                        .find(|key| !selection.body_keys.iter().any(|k| k == key))
                }) {
                    return Err(refuse(format!(
                        "guard of `{}` reads `body.{key}`, which its body_keys do not admit",
                        selection.id
                    )));
                }
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
    fn resolve(&self, template: &Template, values: BTreeMap<String, Supplied>) -> Result<Request> {
        let resolved = template
            .resolve_raw_values(&values)
            .map_err(|refusal| refuse(refusal.reason()))?;
        let segments = resolved.segments[self.base_segments.len()..].to_vec();
        if segments.is_empty() {
            return Err(refuse("operation resolves to the bare base path"));
        }
        Ok((segments, resolved.query))
    }

    /// The value each supplied parameter carries. A JSON array is read only
    /// for a repeated query parameter, and each of its elements must be a
    /// scalar; one scalar for a repeated parameter is one value, typed as
    /// [`joined_fits`] reads it.
    fn parameter_values(
        operation: &Operation,
        input: &Value,
    ) -> Result<BTreeMap<String, Supplied>> {
        let mut values = BTreeMap::new();
        for parameter in &operation.parameters {
            let Some(value) = input.get(&parameter.name) else {
                continue;
            };
            let repeated = parameter.repeated && parameter.location == Location::Query;
            let supplied = match value {
                Value::Array(elements) if repeated => Supplied::Many(
                    elements
                        .iter()
                        .map(|element| Self::element(parameter, element, true))
                        .collect::<Result<_>>()?,
                ),
                // One value for a repeated parameter is typed like its
                // elements, or is a comma-joined list of such elements, the
                // form callers sent before arrays were read.
                _ if repeated => {
                    if !joined_fits(parameter.value_type, value) {
                        return Err(refuse(format!(
                            "parameter `{}` is neither one of its elements nor a comma-joined list of them",
                            parameter.name
                        )));
                    }
                    Supplied::One(Self::element(parameter, value, true)?)
                }
                _ => Supplied::One(Self::element(parameter, value, true)?),
            };
            values.insert(parameter.name.clone(), supplied);
        }
        Ok(values)
    }

    /// One scalar value as the text a parameter carries.
    fn element(
        parameter: &connectors_catalog::inventory::Parameter,
        value: &Value,
        typed: bool,
    ) -> Result<String> {
        // JSON Schema counts `2.0` as an integer; a typed parameter never
        // sends a fraction or an exponent.
        if typed
            && parameter.value_type.is_some()
            && value
                .as_number()
                .is_some_and(|n| !n.is_i64() && !n.is_u64())
        {
            return Err(refuse(format!(
                "parameter `{}` is not an integer literal",
                parameter.name
            )));
        }
        scalar(value)
            .ok_or_else(|| refuse(format!("parameter `{}` is not a scalar", parameter.name)))
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
        check_bounds(&exposed.selection, &values)?;
        let (segments, query) = self.resolve(&exposed.template, values)?;
        let borrowed: Vec<&str> = segments.iter().map(String::as_str).collect();
        let query: Vec<(&str, String)> =
            query.iter().map(|(k, v)| (k.as_str(), v.clone())).collect();
        let response = http.get(&borrowed, &query).await?;
        let body = read_body(
            &response,
            exposed.text,
            &exposed.selection.rate_limit_reasons,
        )?;
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
        // The closed body is held here as well as in the declaration, so it
        // does not rest on the schema validator alone.
        let body_keys = &exposed.selection.body_keys;
        if !body_keys.is_empty()
            && !input
                .get("body")
                .and_then(Value::as_object)
                .is_some_and(|body| body.keys().all(|key| body_keys.contains(key)))
        {
            return Err(refuse("body carries a key its selection does not admit"));
        }
        let values = Self::parameter_values(&exposed.operation, &input)?;
        check_bounds(&exposed.selection, &values)?;
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
                values.insert(parameter.clone(), Supplied::One(value));
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
            let observed = read_body(&response, false, &exposed.selection.rate_limit_reasons)?;
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
            rate_limit_reasons: exposed.selection.rate_limit_reasons.clone(),
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
            // The same split a read's refusal gets; each is the provider's answer.
            let code = match response.status {
                400 | 409 | 412 | 422 => ErrorCode::InvalidInput,
                401 => ErrorCode::Unauthorized,
                403 if quota(&response.body, &self.rate_limit_reasons) => ErrorCode::RateLimited,
                403 => ErrorCode::Forbidden,
                404 | 410 => ErrorCode::NotFound,
                _ => ErrorCode::Forbidden,
            };
            return WriteOutcome::Refused(
                Error::new(code, "provider refused the write").answered(),
            );
        }
        if !(200..300).contains(&response.status) {
            return WriteOutcome::Unknown(Error::new(
                ErrorCode::UpstreamProtocol,
                "unconfirmed write outcome",
            ));
        }
        let body = match read_body(&response, false, &[]) {
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

/// Whether a `403` body names one of the selection's rate-limit reasons, in
/// `error.errors[].reason` or in `error.status` (the two places Google's JSON
/// error answers carry one). A body that is not JSON, or carries the reason
/// anywhere else, names none: the answer stays `forbidden`.
fn quota(body: &[u8], reasons: &[String]) -> bool {
    if reasons.is_empty() {
        return false;
    }
    let Ok(body) = connectors_core::read_json::<Value>(body) else {
        return false;
    };
    let Some(error) = body.get("error") else {
        return false;
    };
    let named = |value: Option<&Value>| {
        value
            .and_then(Value::as_str)
            .is_some_and(|reason| reasons.iter().any(|r| r == reason))
    };
    named(error.get("status"))
        || error
            .get("errors")
            .and_then(Value::as_array)
            .is_some_and(|errors| errors.iter().any(|e| named(e.get("reason"))))
}

fn read_body(response: &HttpResponse, text: bool, rate_limit_reasons: &[String]) -> Result<Value> {
    if !(200..300).contains(&response.status) {
        let code = match response.status {
            400 | 409 | 412 | 422 => ErrorCode::InvalidInput,
            401 => ErrorCode::Unauthorized,
            403 if quota(&response.body, rate_limit_reasons) => ErrorCode::RateLimited,
            403 => ErrorCode::Forbidden,
            404 | 410 => ErrorCode::NotFound,
            429 => ErrorCode::RateLimited,
            500..=599 => ErrorCode::Unavailable,
            _ => ErrorCode::UpstreamProtocol,
        };
        return Err(Error::new(code, "provider refused the request").answered());
    }
    // An empty text body is the empty text; an empty JSON body carries no
    // JSON value at all.
    if response.body.is_empty() {
        return Ok(if text {
            Value::String(String::new())
        } else {
            Value::Null
        });
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

/// Whether a text is one integer element as its decimal text: an optional
/// leading `-` and at least one ASCII digit, the `^-?[0-9]+$` the integer
/// schemas declare. One piece of a comma-joined integer list is read by it.
fn integer_text(text: &str) -> bool {
    let digits = text.strip_prefix('-').unwrap_or(text);
    !digits.is_empty() && digits.bytes().all(|b| b.is_ascii_digit())
}

/// Whether one scalar may stand for a repeated parameter: an element of its
/// type, or a string listing such elements separated by commas. Integer
/// elements take a JSON integer or `1,2,-3`; boolean elements `true`/`false`
/// or a comma list of them; string elements any string or a JSON integer, as
/// a string parameter does; untyped elements any scalar. The same rule is the
/// scalar half of [`repeated_schema`].
fn joined_fits(value_type: Option<ValueType>, value: &Value) -> bool {
    let joined =
        |each: fn(&str) -> bool| value.as_str().is_some_and(|text| text.split(',').all(each));
    match value_type {
        Some(ValueType::Integer) => {
            value.as_number().is_some_and(|n| n.is_i64() || n.is_u64()) || joined(integer_text)
        }
        Some(ValueType::Boolean) => {
            value.is_boolean() || joined(|piece| piece == "true" || piece == "false")
        }
        Some(ValueType::String) => value.is_string() || value.is_i64() || value.is_u64(),
        None => scalar(value).is_some(),
    }
}

/// The declared schema of a repeated query parameter: an array of its element
/// type, or one scalar read as [`joined_fits`] reads it. The type stays a union
/// with the scalar types because callers already send a comma-joined string
/// (Jira's `fields`, Confluence's `space-id`; `docs/catalog-jira.md`,
/// `docs/catalog-confluence.md`). `pattern` applies only to a string, and
/// `minItems` only to an array.
fn repeated_schema(value_type: Option<ValueType>, required: bool) -> Value {
    let mut schema = match value_type {
        Some(ValueType::Integer) => json!({
            "type": ["array", "integer", "string"],
            "pattern": "^-?[0-9]+(,-?[0-9]+)*$",
        }),
        Some(ValueType::Boolean) => json!({
            "type": ["array", "boolean", "string"],
            "pattern": "^(true|false)(,(true|false))*$",
        }),
        Some(ValueType::String) => json!({"type": ["array", "string", "integer"]}),
        None => json!({"type": ["array", "string", "integer", "boolean"]}),
    };
    schema["items"] = declared_type(value_type);
    if required {
        // The engine refuses an empty list for a required parameter as absent;
        // the declaration says the same.
        schema["minItems"] = json!(1);
    }
    schema
}

/// The input schema of one parameter, from the type its pinned document gives.
/// An integer also takes its decimal string, a string also takes a JSON integer
/// (sent as its decimal text), and a boolean the strings `true` and `false`:
/// the forms callers already send and the engine sends on unchanged. A JSON
/// number that is not an integer literal is refused by `parameter_values`. A
/// parameter the bundle records no type for takes any scalar.
fn declared_type(value_type: Option<ValueType>) -> Value {
    match value_type {
        Some(ValueType::String) => json!({"type": ["string", "integer"]}),
        Some(ValueType::Integer) => json!({"anyOf": [
            {"type": "integer"},
            {"type": "string", "pattern": "^-?[0-9]+$"}
        ]}),
        Some(ValueType::Boolean) => json!({"anyOf": [
            {"type": "boolean"},
            {"type": "string", "enum": ["true", "false"]}
        ]}),
        None => json!({"type": ["string", "integer", "boolean"]}),
    }
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
        let bounded = |schema: &mut Value| {
            if let Some(bound) = selection.bounds.get(&parameter.name) {
                // Advisory for callers: it constrains only numbers, so the engine
                // checks the bound itself on every value.
                schema["maximum"] = json!(bound.maximum);
                if let Some(minimum) = bound.minimum {
                    schema["minimum"] = json!(minimum);
                }
            }
        };
        let schema = if parameter.repeated && parameter.location == Location::Query {
            // An array of the declared element type, sent one pair per
            // element; or one value typed like its elements, a comma-joined
            // list of them included, sent as it was given.
            let mut schema = repeated_schema(parameter.value_type, parameter.required);
            bounded(&mut schema["items"]);
            bounded(&mut schema);
            schema
        } else {
            let mut schema = declared_type(parameter.value_type);
            bounded(&mut schema);
            schema
        };
        properties.insert(parameter.name.clone(), schema);
        if parameter.required {
            required.push(parameter.name.clone());
        }
    }
    if !operation.request_media_types.is_empty() {
        let body = if selection.body_keys.is_empty() {
            json!({"type": "object"})
        } else {
            // A closed body: exactly these keys. A key a guard reads directly
            // is declared as the scalar the guard compares; any other key takes
            // any JSON value. Every key a guard reads is required.
            let scalars = selection
                .guard
                .as_ref()
                .map(scalar_body_keys)
                .unwrap_or_default();
            let keys: serde_json::Map<String, Value> = selection
                .body_keys
                .iter()
                .map(|key| {
                    let schema = if scalars.contains(key.as_str()) {
                        declared_type(None)
                    } else {
                        json!({})
                    };
                    (key.clone(), schema)
                })
                .collect();
            let mut body = json!({
                "type": "object",
                "properties": keys,
                "additionalProperties": false,
            });
            let guarded: Vec<&str> = selection
                .guard
                .as_ref()
                .map(|guard| guarded_body_keys(guard).into_iter().collect())
                .unwrap_or_default();
            if !guarded.is_empty() {
                body["required"] = json!(guarded);
            }
            body
        };
        properties.insert("body".into(), body);
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
                // A comparison reference names no pinned parameter, so it has
                // no declared type; it keeps the any-scalar schema.
                properties.insert(reference.clone(), declared_type(None));
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
