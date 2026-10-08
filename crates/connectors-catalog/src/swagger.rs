//! Swagger 2.0 projected into an OpenAPI 3.1.0 document the catalog ingest
//! reads. Pure: no clock, no network, no file system. The output is canonical
//! JSON — keys sorted at every depth, pretty-printed, one final newline — so two
//! runs over the same bytes give the same bytes.
//!
//! The projection is exact. Every Swagger key is read by one row below, or the
//! document is refused naming that key's JSON pointer; nothing is dropped
//! silently. What a row does not carry into the output is named in the
//! [`ProjectionRecord`] instead. The target is 3.1 rather than 3.0 because its
//! schemas are JSON Schema: a `type` list, the `null` type and a list-form
//! `items`, which pinned Swagger documents use, have an exact 3.1 spelling and
//! none in 3.0.
//!
//! | Swagger 2.0 | OpenAPI 3.1.0 |
//! |---|---|
//! | `swagger` | must be `2.0` |
//! | `info` (`title`, `version` required; `description`, `termsOfService`, `contact` {`name`, `url`, `email`}, `license` {`name` required, `url`}) | `info`, as written |
//! | `host`, `basePath`, `schemes` | one `servers[].url` per scheme, `<scheme>://<host><basePath>`: `host` and `schemes` required (without them the server is wherever the document was fetched from, which a pinned document does not record); `host` is `<host>[:<digits>]`; schemes `http` and `https` only; `basePath` `/` or a path starting with `/` and without a final `/` |
//! | `externalDocs` {`description`, `url` required}, `tags` [{`name` required, `description`, `externalDocs`}] | as written, on the document and on each operation |
//! | `consumes`, `produces` | the media types of each request body and response schema; an operation's own list replaces the document's |
//! | an operation's own `consumes` with no body or form parameter, or `produces` with no response schema | ignored, listed |
//! | `securityDefinitions` | ignored, listed; it declares the schemes and scopes `security` may name |
//! | `security`, on the document or an operation | `x-swagger-security`, as written; each scheme declared, each scope declared by its scheme |
//! | `paths` | `paths`; each key starts with `/`; only `get`, `put`, `post`, `delete`, `options`, `head`, `patch` |
//! | `operationId`, `summary`, `description`, `deprecated` | as written; `operationId` unique |
//! | parameter `in` `query`, `header`, `path` | a parameter with `name`, `in`, `description` and `required` as written, and `schema` holding `type` (`string`, `number`, `integer`, `boolean`), `format`, `enum`, `default`, `pattern`, `minimum`, `maximum`, `minLength`, `maxLength`; a path parameter is `required: true` and named by the path template, and every template name has one; header `Accept`, `Content-Type` and `Authorization` refused (OpenAPI 3 ignores them) |
//! | parameter `in` `formData` | a property of one `requestBody` object schema, its `description` on the property, `required` collected; content under each form media type (`application/x-www-form-urlencoded`, `multipart/form-data`) the operation consumes; any other consumed media type excluded and listed; none of the two refused; `requestBody.required` when a property is |
//! | parameter `in` `body` | `requestBody` with `description`, `required: true` when written so, and `schema` under each consumed media type; one per operation, never beside `formData` |
//! | `responses` | each `default` or three-digit status, `description` required; `schema` under each produced media type |
//! | `definitions` | `components.schemas`; names of letters, digits, `.`, `-`, `_` |
//! | schema `$ref` `#/definitions/<name>` | `#/components/schemas/<name>`; an undeclared name refused; any sibling refused (Swagger 2.0 ignores it, OpenAPI 3.1 applies it) |
//! | schema `type` | a name or a list of names of `array`, `boolean`, `integer`, `null`, `number`, `object`, `string` |
//! | schema `items` | an object as `items`; a list as `prefixItems` (draft 4 tuple form, extra items unconstrained in both) |
//! | schema `properties`, `additionalProperties`, `allOf` | kept, each subschema projected |
//! | schema `title`, `description`, `default`, `enum`, `format`, `pattern`, `minLength`, `maxLength`, `minimum`, `maximum`, `multipleOf`, `minItems`, `maxItems`, `uniqueItems`, `minProperties`, `maxProperties`, `readOnly`, `example`, `required`, `x-*` | kept as written |
//! | any key or value not in this table | refused, naming the JSON pointer |

use crate::SOURCE_LIMIT;
use crate::discovery::{authority, canonical, child};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value, json};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

/// The input format a derivation names.
pub const FORMAT: &str = "swagger/2.0";
/// This projector, versioned: a change to any row is a new version.
pub const PROJECTOR: &str = "swagger2-openapi/1";
/// The OpenAPI version every projection declares.
pub const OPENAPI: &str = "3.1.0";
/// Why a consumed media type beside `formData` parameters is excluded.
pub const FORM_ONLY: &str = "formData parameters are defined only for \
     application/x-www-form-urlencoded and multipart/form-data";

const FORM_TYPES: [&str; 2] = ["application/x-www-form-urlencoded", "multipart/form-data"];
const DOCUMENT_KEYS: [&str; 13] = [
    "swagger",
    "info",
    "host",
    "basePath",
    "schemes",
    "consumes",
    "produces",
    "paths",
    "definitions",
    "securityDefinitions",
    "security",
    "tags",
    "externalDocs",
];
const INFO_KEYS: [&str; 6] = [
    "title",
    "version",
    "description",
    "termsOfService",
    "contact",
    "license",
];
const METHODS: [&str; 7] = ["get", "put", "post", "delete", "options", "head", "patch"];
const OPERATION_KEYS: [&str; 11] = [
    "tags",
    "summary",
    "description",
    "externalDocs",
    "operationId",
    "consumes",
    "produces",
    "parameters",
    "responses",
    "deprecated",
    "security",
];
/// The keys of a `query`, `header`, `path` or `formData` parameter.
const PARAMETER_KEYS: [&str; 13] = [
    "name",
    "in",
    "description",
    "required",
    "type",
    "format",
    "enum",
    "default",
    "pattern",
    "minimum",
    "maximum",
    "minLength",
    "maxLength",
];
/// The parameter keys that move onto its schema.
const PARAMETER_SCHEMA_KEYS: [&str; 9] = [
    "type",
    "format",
    "enum",
    "default",
    "pattern",
    "minimum",
    "maximum",
    "minLength",
    "maxLength",
];
const PARAMETER_TYPES: [&str; 4] = ["string", "number", "integer", "boolean"];
const BODY_KEYS: [&str; 5] = ["name", "in", "description", "required", "schema"];
const RESPONSE_KEYS: [&str; 2] = ["description", "schema"];
/// Header parameters OpenAPI 3 says are ignored.
const IGNORED_HEADERS: [&str; 3] = ["accept", "content-type", "authorization"];
const SCHEMA_TYPES: [&str; 7] = [
    "array", "boolean", "integer", "null", "number", "object", "string",
];
const SCHEMA_KEPT: [&str; 19] = [
    "title",
    "description",
    "default",
    "enum",
    "format",
    "pattern",
    "minLength",
    "maxLength",
    "minimum",
    "maximum",
    "multipleOf",
    "minItems",
    "maxItems",
    "uniqueItems",
    "minProperties",
    "maxProperties",
    "readOnly",
    "example",
    "required",
];

/// Why a document was refused.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Reason {
    /// Not strict JSON: malformed, a duplicate key, or trailing content.
    Malformed,
    /// Larger than [`SOURCE_LIMIT`].
    TooLarge,
    /// A key outside the rule table.
    UnknownKey,
    /// A `$ref` naming a definition the document does not declare.
    UnresolvedRef(String),
    /// A key the table reads, holding a value it does not map.
    Invalid(String),
}

impl std::fmt::Display for Reason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Malformed => f.write_str(
                "not strict JSON (malformed, a duplicate key, or content after the document)",
            ),
            Self::TooLarge => f.write_str("larger than the source limit"),
            Self::UnknownKey => f.write_str("a key outside the projection's rule table"),
            Self::UnresolvedRef(name) => write!(
                f,
                "`$ref` names definition `{name}`, which the document does not declare"
            ),
            Self::Invalid(what) => f.write_str(what),
        }
    }
}

/// A refusal: where, by JSON pointer (RFC 6901; empty for the document root),
/// and why.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Refusal {
    pub pointer: String,
    pub reason: Reason,
}

impl std::fmt::Display for Refusal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.pointer.is_empty() {
            write!(f, "swagger document refused at its root: {}", self.reason)
        } else {
            write!(
                f,
                "swagger document refused at `{}`: {}",
                self.pointer, self.reason
            )
        }
    }
}

impl std::error::Error for Refusal {}

impl From<Refusal> for connectors_core::Error {
    fn from(value: Refusal) -> Self {
        let code = match value.reason {
            Reason::TooLarge => connectors_core::ErrorCode::Capacity,
            _ => connectors_core::ErrorCode::InvalidInput,
        };
        connectors_core::Error::new(code, value.to_string())
    }
}

/// A media type an operation consumes that its request body is not projected
/// under.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExcludedMediaType {
    /// The operation's `operationId`, or `<METHOD> <path>` without one.
    pub operation_id: String,
    pub media_type: String,
    pub reason: String,
}

/// What one projection did, beside the document it wrote.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectionRecord {
    pub projector: String,
    /// SHA-256 over the Swagger document's exact bytes.
    pub source_sha256: String,
    pub source_bytes: usize,
    /// The document's own `info.version`.
    pub info_version: String,
    /// Every operation under `paths`.
    pub operation_count: usize,
    /// Projected operations by `operationId`, or `<METHOD> <path>` without one, sorted.
    pub operations: Vec<String>,
    pub excluded_media_types: Vec<ExcludedMediaType>,
    /// JSON pointers of keys read but not carried into the output, sorted.
    pub ignored_keys: Vec<String>,
}

/// The projected document and its record.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Projection {
    pub openapi: Vec<u8>,
    pub record: ProjectionRecord,
}

impl Projection {
    /// The record in the same canonical form as the document.
    pub fn record_bytes(&self) -> Vec<u8> {
        canonical(&serde_json::to_value(&self.record).expect("a record serializes"))
    }
}

type Result<T> = std::result::Result<T, Refusal>;

fn refuse<T>(pointer: &str, reason: Reason) -> Result<T> {
    Err(Refusal {
        pointer: pointer.to_owned(),
        reason,
    })
}

fn invalid<T>(pointer: &str, what: impl Into<String>) -> Result<T> {
    refuse(pointer, Reason::Invalid(what.into()))
}

fn object<'a>(value: &'a Value, pointer: &str) -> Result<&'a Map<String, Value>> {
    value
        .as_object()
        .map_or_else(|| invalid(pointer, "expected an object"), Ok)
}

fn array<'a>(value: &'a Value, pointer: &str) -> Result<&'a Vec<Value>> {
    value
        .as_array()
        .map_or_else(|| invalid(pointer, "expected an array"), Ok)
}

/// Refuse the first key outside `allowed`.
fn only(map: &Map<String, Value>, pointer: &str, allowed: &[&str]) -> Result<()> {
    match map.keys().find(|key| !allowed.contains(&key.as_str())) {
        Some(key) => refuse(&child(pointer, key), Reason::UnknownKey),
        None => Ok(()),
    }
}

fn string<'a>(map: &'a Map<String, Value>, key: &str, pointer: &str) -> Result<Option<&'a str>> {
    match map.get(key) {
        None => Ok(None),
        Some(Value::String(text)) => Ok(Some(text)),
        Some(_) => invalid(&child(pointer, key), "expected a string"),
    }
}

fn required_string<'a>(map: &'a Map<String, Value>, key: &str, pointer: &str) -> Result<&'a str> {
    string(map, key, pointer)?
        .map_or_else(|| invalid(&child(pointer, key), "required, and absent"), Ok)
}

fn boolean(map: &Map<String, Value>, key: &str, pointer: &str) -> Result<Option<bool>> {
    match map.get(key) {
        None => Ok(None),
        Some(Value::Bool(flag)) => Ok(Some(*flag)),
        Some(_) => invalid(&child(pointer, key), "expected a boolean"),
    }
}

/// A list of distinct strings, each non-empty.
fn strings(value: &Value, pointer: &str) -> Result<Vec<String>> {
    let mut out: Vec<String> = Vec::new();
    for (index, item) in array(value, pointer)?.iter().enumerate() {
        let here = child(pointer, &index.to_string());
        match item {
            Value::String(text) if !text.is_empty() => {
                if out.contains(text) {
                    return invalid(&here, format!("`{text}` is listed twice"));
                }
                out.push(text.clone());
            }
            _ => return invalid(&here, "expected a non-empty string"),
        }
    }
    Ok(out)
}

/// An object of string values under `keys`, with `required` present; copied.
fn plain(value: &Value, pointer: &str, keys: &[&str], required: &[&str]) -> Result<Value> {
    let map = object(value, pointer)?;
    only(map, pointer, keys)?;
    for key in map.keys() {
        string(map, key, pointer)?;
    }
    for key in required {
        required_string(map, key, pointer)?;
    }
    Ok(value.clone())
}

fn external_docs(value: &Value, pointer: &str) -> Result<Value> {
    plain(value, pointer, &["description", "url"], &["url"])
}

fn info(value: &Value) -> Result<(Value, String)> {
    let map = object(value, "/info")?;
    only(map, "/info", &INFO_KEYS)?;
    required_string(map, "title", "/info")?;
    let version = required_string(map, "version", "/info")?;
    string(map, "description", "/info")?;
    string(map, "termsOfService", "/info")?;
    if let Some(contact) = map.get("contact") {
        plain(contact, "/info/contact", &["name", "url", "email"], &[])?;
    }
    if let Some(license) = map.get("license") {
        plain(license, "/info/license", &["name", "url"], &["name"])?;
    }
    Ok((value.clone(), version.to_owned()))
}

fn tags(value: &Value) -> Result<Value> {
    for (index, tag) in array(value, "/tags")?.iter().enumerate() {
        let here = child("/tags", &index.to_string());
        let map = object(tag, &here)?;
        only(map, &here, &["name", "description", "externalDocs"])?;
        required_string(map, "name", &here)?;
        string(map, "description", &here)?;
        if let Some(docs) = map.get("externalDocs") {
            external_docs(docs, &child(&here, "externalDocs"))?;
        }
    }
    Ok(value.clone())
}

fn servers(document: &Map<String, Value>) -> Result<Value> {
    let host = string(document, "host", "")?.map_or_else(
        || {
            invalid(
                "/host",
                "required, and absent: without it the server is the host the document was \
                 fetched from, which a pinned document does not record",
            )
        },
        Ok,
    )?;
    if !authority(host) {
        return invalid("/host", format!("host `{host}` is not `<host>[:<port>]`"));
    }
    let base = match string(document, "basePath", "")? {
        None | Some("/") => "",
        Some(path)
            if path.starts_with('/')
                && !path.ends_with('/')
                && !path.contains(['{', '}', '?', '#']) =>
        {
            path
        }
        Some(path) => {
            return invalid(
                "/basePath",
                format!(
                    "basePath `{path}` is not `/` or a path starting with `/`, without a final \
                     `/`, a template, a query or a fragment"
                ),
            );
        }
    };
    let Some(schemes) = document.get("schemes") else {
        return invalid(
            "/schemes",
            "required, and absent: without it the scheme is the one the document was fetched \
             over, which a pinned document does not record",
        );
    };
    let schemes = strings(schemes, "/schemes")?;
    if schemes.is_empty() {
        return invalid("/schemes", "required, and empty");
    }
    let mut servers = Vec::new();
    for (index, scheme) in schemes.iter().enumerate() {
        if scheme != "https" && scheme != "http" {
            return invalid(
                &child("/schemes", &index.to_string()),
                format!(
                    "scheme `{scheme}` has no OpenAPI server: only http and https are projected"
                ),
            );
        }
        servers.push(json!({"url": format!("{scheme}://{host}{base}")}));
    }
    Ok(Value::Array(servers))
}

/// Each declared security scheme with the scopes it declares.
fn security_definitions(
    document: &Map<String, Value>,
) -> Result<BTreeMap<String, BTreeSet<String>>> {
    let mut out = BTreeMap::new();
    let Some(definitions) = document.get("securityDefinitions") else {
        return Ok(out);
    };
    for (name, definition) in object(definitions, "/securityDefinitions")? {
        let here = child("/securityDefinitions", name);
        let map = object(definition, &here)?;
        let kind = required_string(map, "type", &here)?;
        let scopes = match kind {
            "oauth2" => match map.get("scopes") {
                None => BTreeSet::new(),
                Some(scopes) => object(scopes, &child(&here, "scopes"))?
                    .keys()
                    .cloned()
                    .collect(),
            },
            "apiKey" | "basic" => BTreeSet::new(),
            other => {
                return invalid(
                    &child(&here, "type"),
                    format!("security scheme type `{other}` is not apiKey, basic or oauth2"),
                );
            }
        };
        out.insert(name.clone(), scopes);
    }
    Ok(out)
}

fn security(
    value: &Value,
    pointer: &str,
    schemes: &BTreeMap<String, BTreeSet<String>>,
) -> Result<Value> {
    for (index, requirement) in array(value, pointer)?.iter().enumerate() {
        let here = child(pointer, &index.to_string());
        for (name, scopes) in object(requirement, &here)? {
            let at = child(&here, name);
            let Some(declared) = schemes.get(name) else {
                return invalid(
                    &at,
                    format!("security scheme `{name}` is not declared in `securityDefinitions`"),
                );
            };
            for (position, scope) in strings(scopes, &at)?.iter().enumerate() {
                if !declared.contains(scope) {
                    return invalid(
                        &child(&at, &position.to_string()),
                        format!("scope `{scope}` is not declared by security scheme `{name}`"),
                    );
                }
            }
        }
    }
    Ok(value.clone())
}

fn schema_type(value: &Value, pointer: &str) -> Result<()> {
    let known = |name: &str| SCHEMA_TYPES.contains(&name);
    match value {
        Value::String(name) if known(name) => Ok(()),
        Value::String(name) => invalid(
            pointer,
            format!("schema type `{name}` has no exact projection under {PROJECTOR}"),
        ),
        Value::Array(_) => {
            let names = strings(value, pointer)?;
            match names.iter().position(|name| !known(name)) {
                Some(index) => invalid(
                    &child(pointer, &index.to_string()),
                    format!(
                        "schema type `{}` has no exact projection under {PROJECTOR}",
                        names[index]
                    ),
                ),
                None if names.is_empty() => invalid(pointer, "an empty type list"),
                None => Ok(()),
            }
        }
        _ => invalid(pointer, "expected a type name or a list of them"),
    }
}

fn schema(value: &Value, pointer: &str, names: &BTreeSet<String>) -> Result<Value> {
    let map = object(value, pointer)?;
    if let Some(reference) = map.get("$ref") {
        let at = child(pointer, "$ref");
        if let Some(sibling) = map.keys().find(|key| *key != "$ref") {
            return invalid(
                &child(pointer, sibling),
                "a sibling of `$ref`: Swagger 2.0 ignores it and OpenAPI 3.1 applies it, so it \
                 has no exact projection",
            );
        }
        let Value::String(reference) = reference else {
            return invalid(&at, "expected a string");
        };
        let Some(name) = reference.strip_prefix("#/definitions/") else {
            return invalid(
                &at,
                format!("`$ref` `{reference}` is not `#/definitions/<name>`"),
            );
        };
        if !names.contains(name) {
            return refuse(&at, Reason::UnresolvedRef(name.to_owned()));
        }
        return Ok(json!({"$ref": format!("#/components/schemas/{name}")}));
    }
    let mut out = Map::new();
    for (key, item) in map {
        let here = child(pointer, key);
        match key.as_str() {
            "type" => {
                schema_type(item, &here)?;
                out.insert(key.clone(), item.clone());
            }
            "properties" => {
                let mut properties = Map::new();
                for (name, property) in object(item, &here)? {
                    properties.insert(name.clone(), schema(property, &child(&here, name), names)?);
                }
                out.insert(key.clone(), Value::Object(properties));
            }
            "additionalProperties" => {
                let projected = match item {
                    Value::Bool(_) => item.clone(),
                    _ => schema(item, &here, names)?,
                };
                out.insert(key.clone(), projected);
            }
            "items" => match item {
                Value::Array(items) => {
                    let mut projected = Vec::new();
                    for (index, element) in items.iter().enumerate() {
                        projected.push(schema(element, &child(&here, &index.to_string()), names)?);
                    }
                    out.insert("prefixItems".into(), Value::Array(projected));
                }
                _ => {
                    out.insert(key.clone(), schema(item, &here, names)?);
                }
            },
            "allOf" => {
                let members = array(item, &here)?;
                if members.is_empty() {
                    return invalid(&here, "an empty `allOf`");
                }
                let mut projected = Vec::new();
                for (index, member) in members.iter().enumerate() {
                    projected.push(schema(member, &child(&here, &index.to_string()), names)?);
                }
                out.insert(key.clone(), Value::Array(projected));
            }
            "required" => {
                strings(item, &here)?;
                out.insert(key.clone(), item.clone());
            }
            kept if SCHEMA_KEPT.contains(&kept) || kept.starts_with("x-") => {
                out.insert(key.clone(), item.clone());
            }
            _ => return refuse(&here, Reason::UnknownKey),
        }
    }
    Ok(Value::Object(out))
}

/// The names a path template declares, in order.
fn template(path: &str, pointer: &str) -> Result<Vec<String>> {
    let mut names = Vec::new();
    let mut rest = path;
    while let Some(open) = rest.find(['{', '}']) {
        if rest.as_bytes()[open] == b'}' {
            return invalid(
                pointer,
                format!("path `{path}` closes a template it never opened"),
            );
        }
        let after = &rest[open + 1..];
        let Some(close) = after.find('}') else {
            return invalid(pointer, format!("path `{path}` leaves a template open"));
        };
        let name = &after[..close];
        if name.is_empty() || name.contains('{') {
            return invalid(
                pointer,
                format!("path `{path}` has an empty or nested template"),
            );
        }
        if names.iter().any(|known| known == name) {
            return invalid(pointer, format!("path `{path}` names `{{{name}}}` twice"));
        }
        names.push(name.to_owned());
        rest = &after[close + 1..];
    }
    Ok(names)
}

/// What an operation's parameters project to.
#[derive(Default)]
struct Parameters {
    parameters: Vec<Value>,
    /// Each path parameter's name, with the pointer of its declaration.
    path: Vec<(String, String)>,
    form: Map<String, Value>,
    form_required: Vec<String>,
    /// The body's schema, description and `required`.
    body: Option<(Value, Option<String>, bool)>,
}

fn parameters(value: &Value, pointer: &str, names: &BTreeSet<String>) -> Result<Parameters> {
    let mut out = Parameters::default();
    let mut seen = BTreeSet::new();
    for (index, parameter) in array(value, pointer)?.iter().enumerate() {
        let here = child(pointer, &index.to_string());
        let map = object(parameter, &here)?;
        let location = string(map, "in", &here)?;
        only(
            map,
            &here,
            if location == Some("body") {
                &BODY_KEYS
            } else {
                &PARAMETER_KEYS
            },
        )?;
        let location = required_string(map, "in", &here)?;
        let name = required_string(map, "name", &here)?;
        if !seen.insert((location.to_owned(), name.to_owned())) {
            return invalid(
                &child(&here, "name"),
                format!("parameter `{name}` in `{location}` is declared twice"),
            );
        }
        let description = string(map, "description", &here)?;
        let required = boolean(map, "required", &here)?;
        if location == "body" {
            if out.body.is_some() {
                return invalid(&here, "a second body parameter");
            }
            let Some(body) = map.get("schema") else {
                return invalid(&child(&here, "schema"), "required, and absent");
            };
            out.body = Some((
                schema(body, &child(&here, "schema"), names)?,
                description.map(str::to_owned),
                required == Some(true),
            ));
            continue;
        }
        if !matches!(location, "query" | "header" | "path" | "formData") {
            return invalid(
                &child(&here, "in"),
                format!(
                    "parameter location `{location}` is not query, header, path, formData or body"
                ),
            );
        }
        let kind = required_string(map, "type", &here)?;
        if !PARAMETER_TYPES.contains(&kind) {
            return invalid(
                &child(&here, "type"),
                format!("parameter type `{kind}` has no exact projection under {PROJECTOR}"),
            );
        }
        let mut parameter_schema = Map::new();
        for key in PARAMETER_SCHEMA_KEYS {
            if let Some(item) = map.get(key) {
                parameter_schema.insert(key.to_owned(), item.clone());
            }
        }
        if location == "formData" {
            if let Some(description) = description {
                parameter_schema.insert("description".into(), json!(description));
            }
            out.form
                .insert(name.to_owned(), Value::Object(parameter_schema));
            if required == Some(true) {
                out.form_required.push(name.to_owned());
            }
            continue;
        }
        if location == "header" && IGNORED_HEADERS.contains(&name.to_ascii_lowercase().as_str()) {
            return invalid(
                &child(&here, "name"),
                format!(
                    "header parameter `{name}` is ignored by OpenAPI 3, so it has no exact projection"
                ),
            );
        }
        if location == "path" {
            if required != Some(true) {
                return invalid(
                    &child(&here, "required"),
                    format!("path parameter `{name}` must be `required: true`"),
                );
            }
            out.path.push((name.to_owned(), here.clone()));
        }
        let mut projected = Map::new();
        projected.insert("name".into(), json!(name));
        projected.insert("in".into(), json!(location));
        if let Some(description) = description {
            projected.insert("description".into(), json!(description));
        }
        if let Some(required) = required {
            projected.insert("required".into(), json!(required));
        }
        projected.insert("schema".into(), Value::Object(parameter_schema));
        out.parameters.push(Value::Object(projected));
    }
    if out.body.is_some() && !out.form.is_empty() {
        return invalid(pointer, "a body parameter beside formData parameters");
    }
    Ok(out)
}

/// What the whole document lends each operation.
struct Context<'a> {
    names: &'a BTreeSet<String>,
    schemes: &'a BTreeMap<String, BTreeSet<String>>,
    consumes: Option<&'a [String]>,
    produces: Option<&'a [String]>,
}

/// What projecting the operations collected beside the paths.
#[derive(Default)]
struct Collected {
    ids: BTreeMap<String, String>,
    operations: Vec<String>,
    excluded_media_types: Vec<ExcludedMediaType>,
    ignored: BTreeSet<String>,
}

fn media_types(map: &Map<String, Value>, key: &str, pointer: &str) -> Result<Option<Vec<String>>> {
    map.get(key)
        .map(|value| strings(value, &child(pointer, key)))
        .transpose()
}

fn operation(
    value: &Value,
    pointer: &str,
    method: &str,
    path: &str,
    template_names: &[String],
    context: &Context<'_>,
    collected: &mut Collected,
) -> Result<Value> {
    let map = object(value, pointer)?;
    only(map, pointer, &OPERATION_KEYS)?;
    let id = string(map, "operationId", pointer)?;
    let designation = id.map_or_else(
        || format!("{} {path}", method.to_ascii_uppercase()),
        str::to_owned,
    );
    if let Some(id) = id
        && let Some(first) = collected.ids.insert(id.to_owned(), pointer.to_owned())
    {
        return invalid(
            &child(pointer, "operationId"),
            format!("operationId `{id}` is already used at `{first}`"),
        );
    }

    let mut out = Map::new();
    for key in ["operationId", "summary", "description"] {
        if let Some(text) = string(map, key, pointer)? {
            out.insert(key.into(), json!(text));
        }
    }
    if let Some(deprecated) = boolean(map, "deprecated", pointer)? {
        out.insert("deprecated".into(), json!(deprecated));
    }
    if let Some(docs) = map.get("externalDocs") {
        out.insert(
            "externalDocs".into(),
            external_docs(docs, &child(pointer, "externalDocs"))?,
        );
    }
    if let Some(tags) = map.get("tags") {
        strings(tags, &child(pointer, "tags"))?;
        out.insert("tags".into(), tags.clone());
    }
    if let Some(requirements) = map.get("security") {
        out.insert(
            "x-swagger-security".into(),
            security(requirements, &child(pointer, "security"), context.schemes)?,
        );
    }

    let declared = match map.get("parameters") {
        Some(value) => parameters(value, &child(pointer, "parameters"), context.names)?,
        None => Parameters::default(),
    };
    for (name, at) in &declared.path {
        if !template_names.contains(name) {
            return invalid(
                &child(at, "name"),
                format!("path parameter `{name}` is not in the path template `{path}`"),
            );
        }
    }
    for name in template_names {
        if !declared.path.iter().any(|(declared, _)| declared == name) {
            return invalid(
                &child(pointer, "parameters"),
                format!(
                    "path template `{path}` names `{{{name}}}`, and no path parameter declares it"
                ),
            );
        }
    }
    if !declared.parameters.is_empty() {
        out.insert("parameters".into(), Value::Array(declared.parameters));
    }

    let own_consumes = media_types(map, "consumes", pointer)?;
    let consumes = own_consumes.as_deref().or(context.consumes);
    let consumes_at = child(pointer, "consumes");
    if let Some((body, description, required)) = declared.body {
        let Some(types) = consumes.filter(|types| !types.is_empty()) else {
            return invalid(
                &consumes_at,
                "required, and absent: a body parameter needs a media type",
            );
        };
        let mut request = Map::new();
        if let Some(description) = description {
            request.insert("description".into(), json!(description));
        }
        if required {
            request.insert("required".into(), json!(true));
        }
        let content: Map<String, Value> = types
            .iter()
            .map(|media| (media.clone(), json!({"schema": body})))
            .collect();
        request.insert("content".into(), Value::Object(content));
        out.insert("requestBody".into(), Value::Object(request));
    } else if !declared.form.is_empty() {
        let types = consumes.unwrap_or_default();
        let (form, other): (Vec<&String>, Vec<&String>) = types
            .iter()
            .partition(|media| FORM_TYPES.contains(&media.as_str()));
        if form.is_empty() {
            return invalid(
                &consumes_at,
                "no form media type: formData parameters need \
                 application/x-www-form-urlencoded or multipart/form-data",
            );
        }
        for media in other {
            collected.excluded_media_types.push(ExcludedMediaType {
                operation_id: designation.clone(),
                media_type: media.clone(),
                reason: FORM_ONLY.to_owned(),
            });
        }
        let mut body = Map::new();
        body.insert("type".into(), json!("object"));
        body.insert("properties".into(), Value::Object(declared.form));
        if !declared.form_required.is_empty() {
            body.insert("required".into(), json!(declared.form_required));
        }
        let body = Value::Object(body);
        let mut request = Map::new();
        if !declared.form_required.is_empty() {
            request.insert("required".into(), json!(true));
        }
        let content: Map<String, Value> = form
            .into_iter()
            .map(|media| (media.clone(), json!({"schema": body})))
            .collect();
        request.insert("content".into(), Value::Object(content));
        out.insert("requestBody".into(), Value::Object(request));
    } else if own_consumes.is_some() {
        collected.ignored.insert(consumes_at);
    }

    let own_produces = media_types(map, "produces", pointer)?;
    let produces = own_produces.as_deref().or(context.produces);
    let produces_at = child(pointer, "produces");
    let responses_at = child(pointer, "responses");
    let Some(responses) = map.get("responses") else {
        return invalid(&responses_at, "required, and absent");
    };
    let responses = object(responses, &responses_at)?;
    if responses.is_empty() {
        return invalid(&responses_at, "required, and empty");
    }
    let mut projected = Map::new();
    let mut produced = false;
    for (status, response) in responses {
        let here = child(&responses_at, status);
        let is_status = status.len() == 3
            && status.bytes().all(|b| b.is_ascii_digit())
            && (b'1'..=b'5').contains(&status.as_bytes()[0]);
        if status != "default" && !is_status {
            return invalid(
                &here,
                format!("response key `{status}` is not `default` or a three-digit status"),
            );
        }
        let response_map = object(response, &here)?;
        only(response_map, &here, &RESPONSE_KEYS)?;
        let description = required_string(response_map, "description", &here)?;
        let mut out_response = Map::new();
        out_response.insert("description".into(), json!(description));
        if let Some(body) = response_map.get("schema") {
            let Some(types) = produces.filter(|types| !types.is_empty()) else {
                return invalid(
                    &produces_at,
                    "required, and absent: a response schema needs a media type",
                );
            };
            let body = schema(body, &child(&here, "schema"), context.names)?;
            produced = true;
            let content: Map<String, Value> = types
                .iter()
                .map(|media| (media.clone(), json!({"schema": body})))
                .collect();
            out_response.insert("content".into(), Value::Object(content));
        }
        projected.insert(status.clone(), Value::Object(out_response));
    }
    if !produced && own_produces.is_some() {
        collected.ignored.insert(produces_at);
    }
    out.insert("responses".into(), Value::Object(projected));
    collected.operations.push(designation);
    Ok(Value::Object(out))
}

/// Project the Swagger 2.0 document in `bytes`.
pub fn project(bytes: &[u8]) -> std::result::Result<Projection, Refusal> {
    if bytes.len() > SOURCE_LIMIT {
        return refuse("", Reason::TooLarge);
    }
    let parsed: Value =
        connectors_core::read_json(bytes).or_else(|_| refuse("", Reason::Malformed))?;
    let document = object(&parsed, "")?;
    only(document, "", &DOCUMENT_KEYS)?;
    if string(document, "swagger", "")? != Some("2.0") {
        return invalid("/swagger", "must be `2.0`");
    }
    let Some(info_value) = document.get("info") else {
        return invalid("/info", "required, and absent");
    };
    let (info, info_version) = info(info_value)?;
    let servers = servers(document)?;
    let schemes = security_definitions(document)?;
    let consumes = media_types(document, "consumes", "")?;
    let produces = media_types(document, "produces", "")?;

    let mut names = BTreeSet::new();
    let none = Map::new();
    let definitions = match document.get("definitions") {
        Some(definitions) => object(definitions, "/definitions")?,
        None => &none,
    };
    for name in definitions.keys() {
        if name.is_empty()
            || !name
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'-' | b'_'))
        {
            return invalid(
                &child("/definitions", name),
                format!("definition name `{name}` is not an OpenAPI component name"),
            );
        }
        names.insert(name.clone());
    }
    let mut components = Map::new();
    for (name, value) in definitions {
        components.insert(
            name.clone(),
            schema(value, &child("/definitions", name), &names)?,
        );
    }

    let context = Context {
        names: &names,
        schemes: &schemes,
        consumes: consumes.as_deref(),
        produces: produces.as_deref(),
    };
    let mut collected = Collected::default();
    if document.contains_key("securityDefinitions") {
        collected.ignored.insert("/securityDefinitions".to_owned());
    }
    let Some(paths) = document.get("paths") else {
        return invalid("/paths", "required, and absent");
    };
    let mut projected_paths = Map::new();
    let mut operation_count = 0;
    for (path, item) in object(paths, "/paths")? {
        let here = child("/paths", path);
        if !path.starts_with('/') {
            return invalid(&here, format!("path `{path}` does not start with `/`"));
        }
        let template_names = template(path, &here)?;
        let item_map = object(item, &here)?;
        only(item_map, &here, &METHODS)?;
        let mut projected = Map::new();
        for (method, value) in item_map {
            operation_count += 1;
            projected.insert(
                method.clone(),
                operation(
                    value,
                    &child(&here, method),
                    method,
                    path,
                    &template_names,
                    &context,
                    &mut collected,
                )?,
            );
        }
        projected_paths.insert(path.clone(), Value::Object(projected));
    }

    let mut openapi = Map::new();
    openapi.insert("openapi".into(), json!(OPENAPI));
    openapi.insert("info".into(), info);
    openapi.insert("servers".into(), servers);
    if let Some(docs) = document.get("externalDocs") {
        openapi.insert("externalDocs".into(), external_docs(docs, "/externalDocs")?);
    }
    if let Some(value) = document.get("tags") {
        openapi.insert("tags".into(), tags(value)?);
    }
    if let Some(requirements) = document.get("security") {
        openapi.insert(
            "x-swagger-security".into(),
            security(requirements, "/security", &schemes)?,
        );
    }
    openapi.insert("paths".into(), Value::Object(projected_paths));
    if !components.is_empty() {
        openapi.insert("components".into(), json!({"schemas": components}));
    }

    let mut operations = collected.operations;
    operations.sort();
    let record = ProjectionRecord {
        projector: PROJECTOR.to_owned(),
        source_sha256: hex::encode(Sha256::digest(bytes)),
        source_bytes: bytes.len(),
        info_version,
        operation_count,
        operations,
        excluded_media_types: collected.excluded_media_types,
        ignored_keys: collected.ignored.into_iter().collect(),
    };
    Ok(Projection {
        openapi: canonical(&Value::Object(openapi)),
        record,
    })
}
