//! Google Discovery (`discovery#restDescription`, `discoveryVersion` `v1`) projected
//! into an OpenAPI 3.0.3 document the catalog ingest reads. Pure: no clock, no
//! network, no file system. The output is canonical JSON — keys sorted at every
//! depth, pretty-printed, one final newline — so two runs over the same bytes give
//! the same bytes.
//!
//! The projection is exact. Every Discovery key is read by one row below, or the
//! document is refused naming that key's JSON pointer; nothing is dropped
//! silently. What a row does not carry into the output is named in the
//! [`ProjectionRecord`] instead.
//!
//! | Discovery | OpenAPI 3.0.3 |
//! |---|---|
//! | `kind`, `discoveryVersion`, `protocol` | must be `discovery#restDescription`, `v1`, `rest` |
//! | `title`, `version`, `description` | `info.title`, `info.version`, `info.description` |
//! | `documentationLink` | `externalDocs.url` |
//! | `revision` | the record's `discovery_revision` |
//! | `rootUrl` + `servicePath` | one `servers[].url`, without its final `/` (`rootUrl` exactly `https://<host>[:<digits>]/`, the host non-empty dot-separated labels of letters, digits and inner hyphens; anything else refused) |
//! | `baseUrl`, `basePath` | `baseUrl` must equal `rootUrl` + `servicePath`; `basePath` must equal `/` + `servicePath` exactly, or be empty when `servicePath` is (the pinned Gmail and Slides form) |
//! | `auth`, `batchPath`, `canonicalName`, `fullyEncodeReservedExpansion`, `icons`, `id`, `mtlsRootUrl`, `name`, `ownerDomain`, `ownerName`, `version_module` | ignored, listed in the record (`auth` still declares the scopes methods may name) |
//! | method `id` | `operationId`, verbatim; unique |
//! | method `description` | the operation's `description` |
//! | `path` | `/` + the path under the server; `{+x}` becomes `{x}` and is listed in the record (narrowing: the template escapes `/`) |
//! | `httpMethod` | GET, POST, PUT, PATCH, DELETE; anything else refused |
//! | `parameters` with `location` `path` / `query` | `in: path` / `in: query`; path parameters `required: true`, in template order, then query parameters by name |
//! | parameter `required`, `description`, `deprecated` | on the parameter, as written |
//! | parameter `type`, `format`, `enum`, `default`, `pattern`, `minimum`, `maximum` | on its `schema` (`default`, `minimum`, `maximum` are strings in Discovery and become the schema type's own values; a value that does not parse, or does not fit its `format` — `int32`, `uint32`, `float`, and the digits of a string `int64`/`uint64` — is refused) |
//! | `enumDescriptions` | `x-google-enum-descriptions` beside the `enum` |
//! | `repeated: true` (query only) | `schema: {type: array, items: …}`, `style: form`, `explode: true` |
//! | `request.$ref` | `requestBody` `application/json` → `#/components/schemas/<name>`, `required: true` |
//! | `request.parameterName` | ignored, listed |
//! | `response.$ref` | `200` `application/json` → `#/components/schemas/<name>` |
//! | no `response` | `200` with no content, or `application/octet-stream` when `supportsMediaDownload` |
//! | `supportsMediaDownload` beside a `response` | ignored, listed: the media form needs `alt=media`, which is excluded |
//! | `supportsMediaUpload` / `mediaUpload` | the metadata path is projected as above; each upload protocol path is excluded and listed; `mediaUpload.accept`, `maxSize` and `protocols.*.multipart` are ignored, listed |
//! | `useMediaDownloadService`, `supportsSubscription`, `parameterOrder`, `flatPath` | ignored, listed |
//! | `scopes` | `x-google-scopes` on the operation; each must be declared in `auth`; no `securitySchemes` |
//! | document `parameters` | `fields` projected on every operation; `$.xgafv`, `access_token`, `alt`, `callback`, `key`, `oauth_token`, `prettyPrint`, `quotaUser`, `uploadType`, `upload_protocol`, `userIp` excluded and listed; a method parameter of any of those twelve names is refused |
//! | `schemas` | `components.schemas`, the key as the name (`id` must equal it) |
//! | schema `type` `any` | `{}` |
//! | `integer` (`int32`, `uint32`), `number` (`double`, `float`), `boolean` | kept, with `format` |
//! | `string` formats `int64`, `uint64`, `date`, `date-time`, `byte` | `type: string` + `format` |
//! | `string` format `google-datetime` | `date-time` |
//! | `string` formats `google-fieldmask`, `google-duration` | `string` + `x-google-format` |
//! | `properties`, `items`, `additionalProperties`, `enum`, `readOnly`, `deprecated`, `description`, `default` | kept (`default` typed as for parameters) |
//! | `annotations.required` | `x-google-required-for` |
//! | `$ref` inside schemas | `#/components/schemas/<name>`, beside the node's `description`, `readOnly`, `deprecated`, `annotations`; an unresolved name is refused |
//! | nested `resources` and top-level `methods` | walked recursively; method ids are already fully qualified |
//! | paths that differ only in template names, with different HTTP methods | the spelling most methods use (the first in order on a tie) is projected; each method on another spelling is excluded, named with its reason in the record (pinned: `gmail.users.settings.cse.identities.patch`) |
//! | paths that differ only in template names, with the same HTTP method | refused: one OpenAPI path and one HTTP method |
//! | two methods on one path and one HTTP method, or with one `id` | refused |
//! | any key or value not in this table | refused, naming the JSON pointer |
//!
//! OpenAPI 3.0.3 readers ignore the siblings of a `$ref`; they are kept so that
//! nothing is lost, and a 3.0 reader reads only the reference.

use crate::SOURCE_LIMIT;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value, json};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

/// The input format a derivation names.
pub const FORMAT: &str = "google-discovery/v1";
/// This projector, versioned: a change to any row is a new version.
pub const PROJECTOR: &str = "discovery-openapi/1";
/// The OpenAPI version every projection declares.
pub const OPENAPI: &str = "3.0.3";
/// OpenAPI requires a description on every response; Discovery gives none.
pub const RESPONSE_DESCRIPTION: &str = "Successful response";

/// Document parameters no operation carries: they authenticate, select a wire
/// format or an upload protocol, or only shape the response text.
pub const EXCLUDED_PARAMETERS: [&str; 11] = [
    "$.xgafv",
    "access_token",
    "alt",
    "callback",
    "key",
    "oauth_token",
    "prettyPrint",
    "quotaUser",
    "uploadType",
    "upload_protocol",
    "userIp",
];
/// The one document parameter every operation carries.
pub const PROJECTED_PARAMETER: &str = "fields";

const DOCUMENT_KEYS: [&str; 15] = [
    "kind",
    "discoveryVersion",
    "protocol",
    "title",
    "version",
    "description",
    "documentationLink",
    "revision",
    "rootUrl",
    "servicePath",
    "baseUrl",
    "basePath",
    "parameters",
    "schemas",
    "resources",
];
const IGNORED_DOCUMENT_KEYS: [&str; 11] = [
    "auth",
    "batchPath",
    "canonicalName",
    "fullyEncodeReservedExpansion",
    "icons",
    "id",
    "mtlsRootUrl",
    "name",
    "ownerDomain",
    "ownerName",
    "version_module",
];
const METHOD_KEYS: [&str; 15] = [
    "id",
    "path",
    "httpMethod",
    "description",
    "parameters",
    "parameterOrder",
    "request",
    "response",
    "scopes",
    "supportsMediaDownload",
    "useMediaDownloadService",
    "supportsMediaUpload",
    "mediaUpload",
    "supportsSubscription",
    "flatPath",
];
const PARAMETER_KEYS: [&str; 13] = [
    "type",
    "format",
    "location",
    "required",
    "repeated",
    "enum",
    "enumDescriptions",
    "default",
    "pattern",
    "minimum",
    "maximum",
    "description",
    "deprecated",
];
const SCHEMA_KEYS: [&str; 13] = [
    "id",
    "type",
    "format",
    "description",
    "properties",
    "items",
    "additionalProperties",
    "enum",
    "enumDescriptions",
    "readOnly",
    "deprecated",
    "default",
    "annotations",
];
const REFERENCE_KEYS: [&str; 5] = [
    "$ref",
    "description",
    "readOnly",
    "deprecated",
    "annotations",
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
    /// A `$ref` naming a schema the document does not declare.
    UnresolvedRef(String),
    /// `rootUrl` is not an absolute `https://<host>/`.
    NotAbsoluteHttps,
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
            Self::UnresolvedRef(name) => {
                write!(
                    f,
                    "`$ref` names schema `{name}`, which the document does not declare"
                )
            }
            Self::NotAbsoluteHttps => {
                f.write_str("not an absolute https URL of the form `https://<host>/`")
            }
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
            write!(f, "discovery document refused at its root: {}", self.reason)
        } else {
            write!(
                f,
                "discovery document refused at `{}`: {}",
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

/// A method the projection names instead of projecting. The one reason under
/// `discovery-openapi/1`: its path differs from a projected path only in
/// template names, which OpenAPI 3.0.3 does not admit (see `select`).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExcludedMethod {
    pub id: String,
    pub reason: String,
}

/// A path whose reserved expansion `{+x}` was written as `{x}`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RewrittenPath {
    pub operation_id: String,
    pub discovery_path: String,
    pub path: String,
}

/// A media upload protocol path, excluded; its method's metadata path is projected.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UploadPath {
    pub operation_id: String,
    pub protocol: String,
    pub path: String,
}

/// What one projection did, beside the document it wrote.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectionRecord {
    pub projector: String,
    /// SHA-256 over the Discovery document's exact bytes.
    pub source_sha256: String,
    pub source_bytes: usize,
    pub discovery_revision: String,
    /// Every method under `methods` and `resources`, at any depth.
    pub method_count: usize,
    /// Projected operation ids, sorted.
    pub operations: Vec<String>,
    pub excluded_methods: Vec<ExcludedMethod>,
    pub rewritten_paths: Vec<RewrittenPath>,
    /// Document parameters no operation carries, by name, sorted.
    pub excluded_parameters: Vec<String>,
    pub excluded_upload_paths: Vec<UploadPath>,
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

/// Canonical bytes: keys sorted at every depth, pretty-printed, one final newline.
///
/// The sorted value is serialised as it stands and never read back: a re-read
/// goes through serde_json's float parser, whose result depends on which of its
/// features the crate graph enables, so one document would project to different
/// bytes in different builds (adversary pass 1, F2).
fn canonical(value: &Value) -> Vec<u8> {
    fn sorted(value: &Value) -> Value {
        match value {
            Value::Object(map) => {
                let mut keys: Vec<&String> = map.keys().collect();
                keys.sort();
                Value::Object(
                    keys.into_iter()
                        .map(|key| (key.clone(), sorted(&map[key])))
                        .collect(),
                )
            }
            Value::Array(items) => Value::Array(items.iter().map(sorted).collect()),
            other => other.clone(),
        }
    }
    let mut out = serde_json::to_vec_pretty(&sorted(value)).expect("JSON Value is serializable");
    out.push(b'\n');
    out
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

/// A child pointer, escaped per RFC 6901.
fn child(pointer: &str, key: &str) -> String {
    format!("{pointer}/{}", key.replace('~', "~0").replace('/', "~1"))
}

fn object<'a>(value: &'a Value, pointer: &str) -> Result<&'a Map<String, Value>> {
    value
        .as_object()
        .map_or_else(|| invalid(pointer, "expected an object"), Ok)
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

fn strings<'a>(
    map: &'a Map<String, Value>,
    key: &str,
    pointer: &str,
) -> Result<Option<Vec<&'a str>>> {
    let Some(value) = map.get(key) else {
        return Ok(None);
    };
    let at = child(pointer, key);
    let Some(items) = value.as_array() else {
        return invalid(&at, "expected a list of strings");
    };
    items
        .iter()
        .enumerate()
        .map(|(index, item)| {
            item.as_str().map_or_else(
                || invalid(&child(&at, &index.to_string()), "expected a string"),
                Ok,
            )
        })
        .collect::<Result<Vec<_>>>()
        .map(Some)
}

fn fixed(map: &Map<String, Value>, key: &str, expected: &str) -> Result<()> {
    let found = required_string(map, key, "")?;
    if found != expected {
        return invalid(
            &child("", key),
            format!("must be `{expected}`, found `{found}`"),
        );
    }
    Ok(())
}

/// The scalar table shared by parameters and schemas: a Discovery `type` and
/// `format` as an OpenAPI `type` and `format`.
fn scalar(kind: &str, format: Option<&str>, pointer: &str) -> Result<Map<String, Value>> {
    let mut out = Map::new();
    out.insert("type".into(), json!(kind));
    let at = child(pointer, "format");
    match (kind, format) {
        ("string" | "integer" | "number" | "boolean", None) => {}
        ("string", Some(format @ ("byte" | "date" | "date-time" | "int64" | "uint64"))) => {
            out.insert("format".into(), json!(format));
        }
        ("string", Some("google-datetime")) => {
            out.insert("format".into(), json!("date-time"));
        }
        ("string", Some(format @ ("google-fieldmask" | "google-duration"))) => {
            out.insert("x-google-format".into(), json!(format));
        }
        ("integer", Some(format @ ("int32" | "uint32")))
        | ("number", Some(format @ ("double" | "float"))) => {
            out.insert("format".into(), json!(format));
        }
        ("string" | "integer" | "number" | "boolean", Some(format)) => {
            return invalid(
                &at,
                format!("format `{format}` of type `{kind}` is not in the table"),
            );
        }
        (kind, _) => {
            return invalid(
                &child(pointer, "type"),
                format!("type `{kind}` is not in the table here"),
            );
        }
    }
    Ok(out)
}

/// A Discovery string holding a value of `kind` and `format`, as that kind's own
/// JSON value. The value must fit its format: `int32` and `uint32` bound an
/// integer, `float` bounds a number, and the string formats `int64` and `uint64`
/// bound the digits a string carries. A value outside its format is refused, so
/// no schema contradicts its own `format` (adversary pass 1, F6).
fn typed(kind: &str, format: Option<&str>, text: &str, pointer: &str) -> Result<Value> {
    let parsed = match (kind, format) {
        ("string", Some("int64")) => text.parse::<i64>().ok().map(|_| json!(text)),
        ("string", Some("uint64")) => text.parse::<u64>().ok().map(|_| json!(text)),
        ("string", _) => Some(json!(text)),
        ("boolean", _) => match text {
            "true" => Some(json!(true)),
            "false" => Some(json!(false)),
            _ => None,
        },
        ("integer", Some("int32")) => text.parse::<i32>().ok().map(Value::from),
        ("integer", Some("uint32")) => text.parse::<u32>().ok().map(Value::from),
        ("integer", _) => text
            .parse::<i64>()
            .ok()
            .map(Value::from)
            .or_else(|| text.parse::<u64>().ok().map(Value::from)),
        ("number", _) => text
            .parse::<f64>()
            .ok()
            .filter(|number| number.is_finite())
            .filter(|number| format != Some("float") || (*number as f32).is_finite())
            .and_then(serde_json::Number::from_f64)
            .map(Value::Number),
        _ => {
            return invalid(
                pointer,
                format!("a type `{kind}` value is not in the table"),
            );
        }
    };
    let named = format.map_or_else(String::new, |format| format!(" (`{format}`)"));
    parsed.map_or_else(
        || invalid(pointer, format!("`{text}` is not a `{kind}`{named} value")),
        Ok,
    )
}

/// A numeric bound, only on a numeric type.
fn bound(kind: &str, format: Option<&str>, text: &str, pointer: &str) -> Result<Value> {
    if kind != "integer" && kind != "number" {
        return invalid(
            pointer,
            format!("a bound on type `{kind}` is not in the table"),
        );
    }
    typed(kind, format, text, pointer)
}

/// `enum` and `enumDescriptions`, into `out`; only on a string.
fn enumeration(
    map: &Map<String, Value>,
    kind: &str,
    pointer: &str,
    out: &mut Map<String, Value>,
) -> Result<()> {
    let values = strings(map, "enum", pointer)?;
    let descriptions = strings(map, "enumDescriptions", pointer)?;
    if let Some(values) = &values {
        if kind != "string" {
            return invalid(
                &child(pointer, "enum"),
                format!("an enum on type `{kind}` is not in the table"),
            );
        }
        out.insert("enum".into(), json!(values));
    }
    match (values, descriptions) {
        (Some(values), Some(descriptions)) => {
            if values.len() != descriptions.len() {
                return invalid(
                    &child(pointer, "enumDescriptions"),
                    "does not describe each enum value once",
                );
            }
            out.insert("x-google-enum-descriptions".into(), json!(descriptions));
        }
        (None, Some(_)) => {
            return invalid(&child(pointer, "enumDescriptions"), "describes no enum");
        }
        _ => {}
    }
    Ok(())
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Location {
    Path,
    Query,
}

/// One projected parameter.
struct Parameter {
    name: String,
    location: Location,
    value: Value,
}

fn parameter(name: &str, value: &Value, pointer: &str) -> Result<Parameter> {
    let map = object(value, pointer)?;
    only(map, pointer, &PARAMETER_KEYS)?;
    let location = match required_string(map, "location", pointer)? {
        "path" => Location::Path,
        "query" => Location::Query,
        other => {
            return invalid(
                &child(pointer, "location"),
                format!("location `{other}` is not in the table"),
            );
        }
    };
    let kind = required_string(map, "type", pointer)?;
    let format = string(map, "format", pointer)?;
    let mut schema = scalar(kind, format, pointer)?;
    enumeration(map, kind, pointer, &mut schema)?;
    if let Some(pattern) = string(map, "pattern", pointer)? {
        if kind != "string" {
            return invalid(&child(pointer, "pattern"), "a pattern on a non-string");
        }
        schema.insert("pattern".into(), json!(pattern));
    }
    for key in ["minimum", "maximum"] {
        if let Some(text) = string(map, key, pointer)? {
            schema.insert(key.into(), bound(kind, format, text, &child(pointer, key))?);
        }
    }
    let repeated = boolean(map, "repeated", pointer)?.unwrap_or(false);
    if let Some(text) = string(map, "default", pointer)? {
        if repeated {
            return invalid(
                &child(pointer, "default"),
                "a default on a repeated parameter",
            );
        }
        schema.insert(
            "default".into(),
            typed(kind, format, text, &child(pointer, "default"))?,
        );
    }

    let mut out = Map::new();
    out.insert("name".into(), json!(name));
    let required = boolean(map, "required", pointer)?;
    match location {
        Location::Path => {
            if required == Some(false) {
                return invalid(&child(pointer, "required"), "a path parameter is required");
            }
            if repeated {
                return invalid(
                    &child(pointer, "repeated"),
                    "a repeated path parameter is not in the table",
                );
            }
            out.insert("in".into(), json!("path"));
            out.insert("required".into(), json!(true));
        }
        Location::Query => {
            out.insert("in".into(), json!("query"));
            if let Some(required) = required {
                out.insert("required".into(), json!(required));
            }
        }
    }
    if let Some(description) = string(map, "description", pointer)? {
        out.insert("description".into(), json!(description));
    }
    if let Some(deprecated) = boolean(map, "deprecated", pointer)? {
        out.insert("deprecated".into(), json!(deprecated));
    }
    let schema = Value::Object(schema);
    if repeated {
        out.insert("style".into(), json!("form"));
        out.insert("explode".into(), json!(true));
        out.insert("schema".into(), json!({"type": "array", "items": schema}));
    } else {
        out.insert("schema".into(), schema);
    }
    Ok(Parameter {
        name: name.to_owned(),
        location,
        value: Value::Object(out),
    })
}

/// A schema reference, resolved against the document's schema names.
fn reference(value: &Value, pointer: &str, names: &BTreeSet<String>) -> Result<Value> {
    let Some(name) = value.as_str() else {
        return invalid(pointer, "expected a schema name");
    };
    if !names.contains(name) {
        return refuse(pointer, Reason::UnresolvedRef(name.to_owned()));
    }
    Ok(json!(format!("#/components/schemas/{name}")))
}

/// `description`, `readOnly`, `deprecated` and `annotations`, shared by a `$ref`
/// node and a typed one.
fn schema_notes(
    map: &Map<String, Value>,
    pointer: &str,
    out: &mut Map<String, Value>,
) -> Result<()> {
    if let Some(description) = string(map, "description", pointer)? {
        out.insert("description".into(), json!(description));
    }
    for key in ["readOnly", "deprecated"] {
        if let Some(flag) = boolean(map, key, pointer)? {
            out.insert(key.into(), json!(flag));
        }
    }
    if let Some(annotations) = map.get("annotations") {
        let at = child(pointer, "annotations");
        let annotations = object(annotations, &at)?;
        only(annotations, &at, &["required"])?;
        if let Some(required) = strings(annotations, "required", &at)? {
            out.insert("x-google-required-for".into(), json!(required));
        }
    }
    Ok(())
}

fn schema(
    value: &Value,
    pointer: &str,
    names: &BTreeSet<String>,
    top: Option<&str>,
) -> Result<Value> {
    let map = object(value, pointer)?;
    let mut out = Map::new();
    if let Some(target) = map.get("$ref") {
        only(map, pointer, &REFERENCE_KEYS)?;
        out.insert(
            "$ref".into(),
            reference(target, &child(pointer, "$ref"), names)?,
        );
        schema_notes(map, pointer, &mut out)?;
        return Ok(Value::Object(out));
    }
    only(map, pointer, &SCHEMA_KEYS)?;
    match (top, string(map, "id", pointer)?) {
        (Some(name), Some(id)) if id != name => {
            return invalid(
                &child(pointer, "id"),
                format!("`{id}` differs from its schema name `{name}`"),
            );
        }
        (None, Some(_)) => {
            return invalid(&child(pointer, "id"), "an id below a named schema");
        }
        _ => {}
    }
    let Some(kind) = string(map, "type", pointer)? else {
        return invalid(pointer, "a schema with neither `type` nor `$ref`");
    };
    let refuse_here = |key: &str| -> Result<()> {
        if map.contains_key(key) {
            return invalid(
                &child(pointer, key),
                format!("`{key}` on type `{kind}` is not in the table"),
            );
        }
        Ok(())
    };
    match kind {
        "any" => {
            for key in [
                "format",
                "properties",
                "items",
                "additionalProperties",
                "enum",
                "enumDescriptions",
                "default",
            ] {
                refuse_here(key)?;
            }
        }
        "object" => {
            for key in ["format", "items", "enum", "enumDescriptions", "default"] {
                refuse_here(key)?;
            }
            out.insert("type".into(), json!("object"));
            if let Some(properties) = map.get("properties") {
                let at = child(pointer, "properties");
                let mut projected = Map::new();
                for (name, property) in object(properties, &at)? {
                    projected.insert(
                        name.clone(),
                        schema(property, &child(&at, name), names, None)?,
                    );
                }
                out.insert("properties".into(), Value::Object(projected));
            }
            if let Some(additional) = map.get("additionalProperties") {
                let at = child(pointer, "additionalProperties");
                out.insert(
                    "additionalProperties".into(),
                    schema(additional, &at, names, None)?,
                );
            }
        }
        "array" => {
            for key in [
                "format",
                "properties",
                "additionalProperties",
                "enum",
                "enumDescriptions",
                "default",
            ] {
                refuse_here(key)?;
            }
            let Some(items) = map.get("items") else {
                return invalid(pointer, "an array without `items`");
            };
            out.insert("type".into(), json!("array"));
            out.insert(
                "items".into(),
                schema(items, &child(pointer, "items"), names, None)?,
            );
        }
        _ => {
            for key in ["properties", "items", "additionalProperties"] {
                refuse_here(key)?;
            }
            let format = string(map, "format", pointer)?;
            out.extend(scalar(kind, format, pointer)?);
            enumeration(map, kind, pointer, &mut out)?;
            if let Some(text) = string(map, "default", pointer)? {
                out.insert(
                    "default".into(),
                    typed(kind, format, text, &child(pointer, "default"))?,
                );
            }
        }
    }
    schema_notes(map, pointer, &mut out)?;
    Ok(Value::Object(out))
}

/// `host[:port]`: one or more dot-separated labels, each non-empty, of ASCII
/// letters, digits and inner hyphens, then optionally `:` and one or more digits.
fn authority(text: &str) -> bool {
    let (host, port) = match text.split_once(':') {
        Some((host, port)) => (host, Some(port)),
        None => (text, None),
    };
    let label = |label: &str| {
        !label.is_empty()
            && !label.starts_with('-')
            && !label.ends_with('-')
            && label
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'-')
    };
    let port = port.is_none_or(|port| !port.is_empty() && port.bytes().all(|b| b.is_ascii_digit()));
    host.split('.').all(label) && port
}

/// `rootUrl` + `servicePath` as one server URL without its final `/`.
fn server(document: &Map<String, Value>) -> Result<String> {
    let root = required_string(document, "rootUrl", "")?;
    let authority = root
        .strip_prefix("https://")
        .and_then(|rest| rest.strip_suffix('/'))
        .filter(|host| authority(host));
    if authority.is_none() {
        return refuse("/rootUrl", Reason::NotAbsoluteHttps);
    }
    let service = required_string(document, "servicePath", "")?;
    let clean = service
        .bytes()
        .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'/' | b'-' | b'_' | b'.' | b'~'));
    if service.starts_with('/')
        || (!service.is_empty() && !service.ends_with('/'))
        || !clean
        || service
            .split('/')
            .any(|segment| segment == "." || segment == "..")
        || service.contains("//")
    {
        return invalid(
            "/servicePath",
            "must be empty or a relative path ending in `/`",
        );
    }
    let base = format!("{root}{service}");
    if let Some(url) = string(document, "baseUrl", "")?
        && url != base
    {
        return invalid(
            "/baseUrl",
            format!("must equal rootUrl + servicePath, `{base}`"),
        );
    }
    // Exactly `/` + servicePath. With an empty servicePath the pinned Gmail and
    // Slides documents write an empty basePath, which names the same root.
    if let Some(path) = string(document, "basePath", "")?
        && path != format!("/{service}")
        && !(service.is_empty() && path.is_empty())
    {
        return invalid("/basePath", format!("must be `/{service}`"));
    }
    Ok(base.trim_end_matches('/').to_owned())
}

/// One path template segment list: literal text and parameter names, and
/// whether each name used reserved expansion.
struct Template {
    /// The OpenAPI path.
    path: String,
    /// Names in template order.
    names: Vec<String>,
    rewritten: bool,
}

fn template(path: &str, pointer: &str) -> Result<Template> {
    let bad = |what: &str| invalid::<Template>(pointer, format!("path `{path}`: {what}"));
    if path.is_empty() || path.starts_with('/') || path.contains("://") {
        return bad("must be a non-empty path relative to the service path");
    }
    let mut out = String::from("/");
    let mut names = Vec::new();
    let mut rewritten = false;
    let mut rest = path;
    while let Some(open) = rest.find('{') {
        let literal = &rest[..open];
        if literal.contains('}') {
            return bad("an unmatched `}`");
        }
        out.push_str(literal);
        let Some(close) = rest[open..].find('}') else {
            return bad("an unmatched `{`");
        };
        let expression = &rest[open + 1..open + close];
        let (name, reserved) = match expression.strip_prefix('+') {
            Some(name) => (name, true),
            None => (expression, false),
        };
        if name.is_empty() || !name.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_') {
            return bad(&format!(
                "expression `{{{expression}}}` is not in the table"
            ));
        }
        if names.iter().any(|seen| seen == name) {
            return bad(&format!("`{name}` appears twice"));
        }
        rewritten |= reserved;
        names.push(name.to_owned());
        out.push('{');
        out.push_str(name);
        out.push('}');
        rest = &rest[open + close + 1..];
    }
    if rest.contains('}') {
        return bad("an unmatched `}`");
    }
    out.push_str(rest);
    if out.contains(['?', '#', ' '])
        || out
            .split('/')
            .any(|segment| segment == "." || segment == "..")
        || out.contains("//")
    {
        return bad("carries a query, a fragment, a space or an empty or dot segment");
    }
    Ok(Template {
        path: out,
        names,
        rewritten,
    })
}

/// One projected method, before the spellings of its path are reconciled.
struct Candidate {
    /// The path with every template name blanked: `/things/{}`.
    shape: String,
    path: String,
    verb: &'static str,
    id: String,
    /// The pointer of the method's `path`.
    at: String,
    operation: Value,
    rewritten: Option<RewrittenPath>,
}

/// Everything collected while walking the methods.
struct Walk<'a> {
    names: &'a BTreeSet<String>,
    scopes: &'a BTreeSet<String>,
    fields: Option<&'a Value>,
    candidates: Vec<Candidate>,
    /// Operation id → the pointer of its method.
    ids: BTreeMap<String, String>,
    method_count: usize,
    uploads: Vec<UploadPath>,
    ignored: BTreeSet<String>,
}

/// What the candidates become: the OpenAPI paths and the methods excluded.
type Selected = (
    BTreeMap<String, Map<String, Value>>,
    Vec<String>,
    Vec<ExcludedMethod>,
    Vec<RewrittenPath>,
);

/// OpenAPI 3.0.3 admits one spelling of templated paths that differ only in
/// names. Of such spellings the one most methods use is projected, the one that
/// sorts first on a tie; methods on every other spelling are excluded and named.
/// The choice reads the whole set, so it does not depend on declaration order.
/// Two methods on one OpenAPI path and one HTTP method refuse the document,
/// whether they spell the names alike or not.
fn select(mut candidates: Vec<Candidate>) -> Result<Selected> {
    let mut spellings: BTreeMap<&str, BTreeMap<&str, usize>> = BTreeMap::new();
    for candidate in &candidates {
        *spellings
            .entry(&candidate.shape)
            .or_default()
            .entry(&candidate.path)
            .or_default() += 1;
    }
    let kept: BTreeMap<String, String> = spellings
        .into_iter()
        .map(|(shape, paths)| {
            let chosen = paths
                .iter()
                .max_by(|a, b| a.1.cmp(b.1).then_with(|| b.0.cmp(a.0)))
                .map(|(path, _)| (*path).to_owned())
                .unwrap_or_default();
            (shape.to_owned(), chosen)
        })
        .collect();
    let mut paths: BTreeMap<String, Map<String, Value>> = BTreeMap::new();
    let mut operations = Vec::new();
    let mut excluded = Vec::new();
    let mut rewritten = Vec::new();
    candidates.sort_by(|a, b| a.at.cmp(&b.at));
    // Every method on a kept spelling is placed first, so that a method on
    // another spelling can be judged against all of them.
    let (on_kept, elsewhere): (Vec<Candidate>, Vec<Candidate>) = candidates
        .into_iter()
        .partition(|candidate| candidate.path == kept[&candidate.shape]);
    for candidate in on_kept {
        let item = paths.entry(candidate.path.clone()).or_default();
        if item.contains_key(candidate.verb) {
            return invalid(
                &candidate.at,
                format!(
                    "a second {} method on `{}`",
                    candidate.verb.to_uppercase(),
                    candidate.path
                ),
            );
        }
        item.insert(candidate.verb.to_owned(), candidate.operation);
        operations.push(candidate.id);
        rewritten.extend(candidate.rewritten);
    }
    for candidate in elsewhere {
        let chosen = &kept[&candidate.shape];
        // One OpenAPI path and one HTTP method, whatever the names are spelled:
        // the same collision as two methods on one spelling, and refused alike
        // (adversary pass 1, F3). Only a method whose HTTP method the kept
        // spelling does not carry is excluded instead.
        if paths[chosen].contains_key(candidate.verb) {
            return invalid(
                &candidate.at,
                format!(
                    "a second {} method on `{chosen}`, spelled `{}`",
                    candidate.verb.to_uppercase(),
                    candidate.path
                ),
            );
        }
        excluded.push(ExcludedMethod {
            id: candidate.id,
            reason: format!(
                "path `{}` differs from the projected `{chosen}` only in parameter names, \
                 which OpenAPI 3.0.3 does not admit",
                candidate.path
            ),
        });
    }
    operations.sort();
    excluded.sort_by(|a, b| a.id.cmp(&b.id));
    rewritten.sort_by(|a, b| a.operation_id.cmp(&b.operation_id));
    Ok((paths, operations, excluded, rewritten))
}

fn resource(node: &Map<String, Value>, pointer: &str, walk: &mut Walk<'_>) -> Result<()> {
    if let Some(methods) = node.get("methods") {
        let at = child(pointer, "methods");
        for (name, method_value) in object(methods, &at)? {
            method(method_value, &child(&at, name), walk)?;
        }
    }
    if let Some(resources) = node.get("resources") {
        let at = child(pointer, "resources");
        for (name, value) in object(resources, &at)? {
            let here = child(&at, name);
            let map = object(value, &here)?;
            only(map, &here, &["methods", "resources"])?;
            resource(map, &here, walk)?;
        }
    }
    Ok(())
}

fn method(value: &Value, pointer: &str, walk: &mut Walk<'_>) -> Result<()> {
    let map = object(value, pointer)?;
    only(map, pointer, &METHOD_KEYS)?;
    walk.method_count += 1;
    let id = required_string(map, "id", pointer)?;
    if id.is_empty() {
        return invalid(&child(pointer, "id"), "an empty method id");
    }
    if let Some(first) = walk.ids.insert(id.to_owned(), pointer.to_owned()) {
        return invalid(
            &child(pointer, "id"),
            format!("method id `{id}` is also declared at `{first}`"),
        );
    }
    let verb = match required_string(map, "httpMethod", pointer)? {
        "GET" => "get",
        "POST" => "post",
        "PUT" => "put",
        "PATCH" => "patch",
        "DELETE" => "delete",
        other => {
            return invalid(
                &child(pointer, "httpMethod"),
                format!("HTTP method `{other}` is not in the table"),
            );
        }
    };
    let written = required_string(map, "path", pointer)?;
    let path_at = child(pointer, "path");
    let shape = template(written, &path_at)?;

    let mut declared: BTreeMap<String, Parameter> = BTreeMap::new();
    if let Some(parameters) = map.get("parameters") {
        let at = child(pointer, "parameters");
        for (name, value) in object(parameters, &at)? {
            let here = child(&at, name);
            if name == PROJECTED_PARAMETER || EXCLUDED_PARAMETERS.contains(&name.as_str()) {
                return invalid(
                    &here,
                    format!("method parameter `{name}` shadows a document parameter"),
                );
            }
            declared.insert(name.clone(), parameter(name, value, &here)?);
        }
    }
    for name in &shape.names {
        match declared.get(name) {
            Some(parameter) if parameter.location == Location::Path => {}
            _ => {
                return invalid(
                    &path_at,
                    format!("template name `{name}` is no declared path parameter"),
                );
            }
        }
    }
    for (name, parameter) in &declared {
        if parameter.location == Location::Path && !shape.names.contains(name) {
            return invalid(
                &child(&child(pointer, "parameters"), name),
                format!("path parameter `{name}` is not in the path `{written}`"),
            );
        }
    }
    let mut parameters: Vec<Value> = shape
        .names
        .iter()
        .map(|name| declared[name].value.clone())
        .collect();
    let mut query: Vec<(String, Value)> = declared
        .values()
        .filter(|parameter| parameter.location == Location::Query)
        .map(|parameter| (parameter.name.clone(), parameter.value.clone()))
        .collect();
    if let Some(fields) = walk.fields {
        query.push((PROJECTED_PARAMETER.to_owned(), fields.clone()));
    }
    query.sort_by(|a, b| a.0.cmp(&b.0));
    parameters.extend(query.into_iter().map(|(_, value)| value));

    let mut operation = Map::new();
    operation.insert("operationId".into(), json!(id));
    if let Some(description) = string(map, "description", pointer)? {
        operation.insert("description".into(), json!(description));
    }
    if !parameters.is_empty() {
        operation.insert("parameters".into(), Value::Array(parameters));
    }

    if let Some(request) = map.get("request") {
        let at = child(pointer, "request");
        let request = object(request, &at)?;
        only(request, &at, &["$ref", "parameterName"])?;
        let Some(target) = request.get("$ref") else {
            return invalid(&at, "a request without `$ref`");
        };
        let target = reference(target, &child(&at, "$ref"), walk.names)?;
        if string(request, "parameterName", &at)?.is_some() {
            walk.ignored.insert(child(&at, "parameterName"));
        }
        operation.insert(
            "requestBody".into(),
            json!({"required": true, "content": {"application/json": {"schema": {"$ref": target}}}}),
        );
    }

    let download = boolean(map, "supportsMediaDownload", pointer)?;
    let response = match map.get("response") {
        Some(response) => {
            let at = child(pointer, "response");
            let response = object(response, &at)?;
            only(response, &at, &["$ref"])?;
            let Some(target) = response.get("$ref") else {
                return invalid(&at, "a response without `$ref`");
            };
            let target = reference(target, &child(&at, "$ref"), walk.names)?;
            if download.is_some() {
                walk.ignored.insert(child(pointer, "supportsMediaDownload"));
            }
            json!({
                "description": RESPONSE_DESCRIPTION,
                "content": {"application/json": {"schema": {"$ref": target}}}
            })
        }
        None if download == Some(true) => json!({
            "description": RESPONSE_DESCRIPTION,
            "content": {"application/octet-stream": {"schema": {"type": "string", "format": "binary"}}}
        }),
        None => {
            if download.is_some() {
                walk.ignored.insert(child(pointer, "supportsMediaDownload"));
            }
            json!({"description": RESPONSE_DESCRIPTION})
        }
    };
    operation.insert("responses".into(), json!({"200": response}));

    if let Some(scopes) = strings(map, "scopes", pointer)? {
        let at = child(pointer, "scopes");
        for (index, scope) in scopes.iter().enumerate() {
            if !walk.scopes.contains(*scope) {
                return invalid(
                    &child(&at, &index.to_string()),
                    format!("scope `{scope}` is not declared in `auth`"),
                );
            }
        }
        operation.insert("x-google-scopes".into(), json!(scopes));
    }

    let upload = boolean(map, "supportsMediaUpload", pointer)?;
    match (upload, map.get("mediaUpload")) {
        (Some(true), Some(media)) => {
            let at = child(pointer, "mediaUpload");
            let media = object(media, &at)?;
            only(media, &at, &["accept", "maxSize", "protocols"])?;
            // The upload protocol is excluded, so what only it reads — the media
            // types it accepts, its size limit, whether it is multipart — is
            // carried nowhere; each is listed (adversary pass 1, F1).
            if strings(media, "accept", &at)?.is_some() {
                walk.ignored.insert(child(&at, "accept"));
            }
            if string(media, "maxSize", &at)?.is_some() {
                walk.ignored.insert(child(&at, "maxSize"));
            }
            let protocols_at = child(&at, "protocols");
            let Some(protocols) = media.get("protocols") else {
                return invalid(&at, "media upload without `protocols`");
            };
            let protocols = object(protocols, &protocols_at)?;
            only(protocols, &protocols_at, &["simple", "resumable"])?;
            for (protocol, value) in protocols {
                let here = child(&protocols_at, protocol);
                let entry = object(value, &here)?;
                only(entry, &here, &["multipart", "path"])?;
                if boolean(entry, "multipart", &here)?.is_some() {
                    walk.ignored.insert(child(&here, "multipart"));
                }
                let path = required_string(entry, "path", &here)?;
                if !path.starts_with('/') {
                    return invalid(&child(&here, "path"), "an upload path is absolute");
                }
                walk.uploads.push(UploadPath {
                    operation_id: id.to_owned(),
                    protocol: protocol.clone(),
                    path: path.to_owned(),
                });
            }
        }
        (Some(true), None) => {
            return invalid(
                &child(pointer, "supportsMediaUpload"),
                "media upload without `mediaUpload`",
            );
        }
        (_, Some(_)) => {
            return invalid(
                &child(pointer, "mediaUpload"),
                "`mediaUpload` without `supportsMediaUpload: true`",
            );
        }
        (Some(false), None) => {
            walk.ignored.insert(child(pointer, "supportsMediaUpload"));
        }
        (None, None) => {}
    }

    for key in ["useMediaDownloadService", "supportsSubscription"] {
        if boolean(map, key, pointer)?.is_some() {
            walk.ignored.insert(child(pointer, key));
        }
    }
    if strings(map, "parameterOrder", pointer)?.is_some() {
        walk.ignored.insert(child(pointer, "parameterOrder"));
    }
    if string(map, "flatPath", pointer)?.is_some() {
        walk.ignored.insert(child(pointer, "flatPath"));
    }

    let mut blank = shape.path.clone();
    for name in &shape.names {
        blank = blank.replacen(&format!("{{{name}}}"), "{}", 1);
    }
    walk.candidates.push(Candidate {
        shape: blank,
        verb,
        id: id.to_owned(),
        at: path_at,
        operation: Value::Object(operation),
        rewritten: shape.rewritten.then(|| RewrittenPath {
            operation_id: id.to_owned(),
            discovery_path: written.to_owned(),
            path: shape.path.clone(),
        }),
        path: shape.path,
    });
    Ok(())
}

/// The declared OAuth scope names. The descriptions are read for shape only.
fn scopes(document: &Map<String, Value>) -> Result<BTreeSet<String>> {
    let mut out = BTreeSet::new();
    let Some(auth) = document.get("auth") else {
        return Ok(out);
    };
    let auth = object(auth, "/auth")?;
    only(auth, "/auth", &["oauth2"])?;
    let Some(oauth2) = auth.get("oauth2") else {
        return Ok(out);
    };
    let oauth2 = object(oauth2, "/auth/oauth2")?;
    only(oauth2, "/auth/oauth2", &["scopes"])?;
    let Some(declared) = oauth2.get("scopes") else {
        return Ok(out);
    };
    let at = "/auth/oauth2/scopes";
    for (scope, value) in object(declared, at)? {
        let here = child(at, scope);
        let entry = object(value, &here)?;
        only(entry, &here, &["description"])?;
        string(entry, "description", &here)?;
        out.insert(scope.clone());
    }
    Ok(out)
}

/// Project a Discovery document's exact bytes.
pub fn project(bytes: &[u8]) -> std::result::Result<Projection, Refusal> {
    if bytes.len() > SOURCE_LIMIT {
        return refuse("", Reason::TooLarge);
    }
    let parsed: Value =
        connectors_core::read_json(bytes).or_else(|_| refuse("", Reason::Malformed))?;
    let document = object(&parsed, "")?;
    let allowed: Vec<&str> = DOCUMENT_KEYS
        .iter()
        .chain(IGNORED_DOCUMENT_KEYS.iter())
        .chain(["methods"].iter())
        .copied()
        .collect();
    only(document, "", &allowed)?;
    fixed(document, "kind", "discovery#restDescription")?;
    fixed(document, "discoveryVersion", "v1")?;
    fixed(document, "protocol", "rest")?;
    let revision = required_string(document, "revision", "")?;
    let mut info = Map::new();
    info.insert(
        "title".into(),
        json!(required_string(document, "title", "")?),
    );
    info.insert(
        "version".into(),
        json!(required_string(document, "version", "")?),
    );
    if let Some(description) = string(document, "description", "")? {
        info.insert("description".into(), json!(description));
    }
    let server = server(document)?;
    let scopes = scopes(document)?;

    let mut ignored = BTreeSet::new();
    for key in IGNORED_DOCUMENT_KEYS {
        if document.contains_key(key) {
            ignored.insert(child("", key));
        }
    }

    let mut names = BTreeSet::new();
    let none = Map::new();
    let schemas = match document.get("schemas") {
        Some(schemas) => object(schemas, "/schemas")?,
        None => &none,
    };
    for name in schemas.keys() {
        if name.is_empty()
            || !name
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'-' | b'_'))
        {
            return invalid(
                &child("/schemas", name),
                format!("schema name `{name}` is not an OpenAPI component name"),
            );
        }
        names.insert(name.clone());
    }
    let mut components = Map::new();
    for (name, value) in schemas {
        components.insert(
            name.clone(),
            schema(value, &child("/schemas", name), &names, Some(name))?,
        );
    }

    let mut excluded_parameters = Vec::new();
    let mut fields = None;
    if let Some(parameters) = document.get("parameters") {
        for (name, value) in object(parameters, "/parameters")? {
            let here = child("/parameters", name);
            let projected = parameter(name, value, &here)?;
            if projected.location != Location::Query {
                return invalid(&here, "a document parameter travels in the query");
            }
            if name == PROJECTED_PARAMETER {
                fields = Some(projected.value);
            } else if EXCLUDED_PARAMETERS.contains(&name.as_str()) {
                excluded_parameters.push(name.clone());
            } else {
                return invalid(
                    &here,
                    format!("document parameter `{name}` is not in the table"),
                );
            }
        }
    }
    excluded_parameters.sort();

    let mut walk = Walk {
        names: &names,
        scopes: &scopes,
        fields: fields.as_ref(),
        candidates: Vec::new(),
        ids: BTreeMap::new(),
        method_count: 0,
        uploads: Vec::new(),
        ignored,
    };
    resource(document, "", &mut walk)?;
    let (paths, operations, excluded_methods, rewritten_paths) = select(walk.candidates)?;

    let mut openapi = Map::new();
    openapi.insert("openapi".into(), json!(OPENAPI));
    openapi.insert("info".into(), Value::Object(info));
    if let Some(link) = string(document, "documentationLink", "")? {
        openapi.insert("externalDocs".into(), json!({"url": link}));
    }
    openapi.insert("servers".into(), json!([{"url": server}]));
    openapi.insert(
        "paths".into(),
        Value::Object(
            paths
                .into_iter()
                .map(|(path, item)| (path, Value::Object(item)))
                .collect(),
        ),
    );
    if !components.is_empty() {
        openapi.insert("components".into(), json!({"schemas": components}));
    }

    walk.uploads.sort_by(|a, b| {
        (&a.operation_id, &a.protocol, &a.path).cmp(&(&b.operation_id, &b.protocol, &b.path))
    });
    let record = ProjectionRecord {
        projector: PROJECTOR.to_owned(),
        source_sha256: hex::encode(Sha256::digest(bytes)),
        source_bytes: bytes.len(),
        discovery_revision: revision.to_owned(),
        method_count: walk.method_count,
        operations,
        excluded_methods,
        rewritten_paths,
        excluded_parameters,
        excluded_upload_paths: walk.uploads,
        ignored_keys: walk.ignored.into_iter().collect(),
    };
    Ok(Projection {
        openapi: canonical(&Value::Object(openapi)),
        record,
    })
}
