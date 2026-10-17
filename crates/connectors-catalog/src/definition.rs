//! One `connectors-definition/1` provider definition, read and lowered into what
//! the catalog provider already runs: a bundle with its index, compiled by
//! [`crate::pipeline`] from the vendored source the definition pins, and a
//! `connectors-catalog-operations/1` selection of the operations it exposes.
//!
//! The types mirror schema v1 (`spec/domains/definition.yaml` in the
//! definitions repository) field for field, and every struct refuses a field
//! the schema does not name. A definition the lowering cannot honour is refused
//! by name ([`Refusal`]) before anything is written to the output directory.

use crate::{bundle, pipeline};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::io::Read as _;
use std::path::{Component, Path, PathBuf};

/// The one schema identifier this reader accepts.
pub const SCHEMA: &str = "urn:beyond10x:connectors-definition:1";
/// The selection format the catalog provider's `operations_file` reads.
pub const OPERATIONS_FORMAT: &str = "connectors-catalog-operations/1";

/// `connectors_definitions.definition.SchemaId`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum SchemaId {
    #[serde(rename = "urn:beyond10x:connectors-definition:1")]
    V1,
}

/// `connectors_definitions.definition.Source`: the vendored document, by its
/// path under the definitions root, and the SHA-256 of its decompressed bytes.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Source {
    pub openapi: String,
    pub sha256: String,
    pub amendments: Option<String>,
}

/// `connectors_definitions.definition.ConfigType`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ConfigType {
    String,
}

/// `connectors_definitions.definition.ConfigField`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConfigField {
    #[serde(rename = "type")]
    pub value_type: ConfigType,
    pub default: Option<String>,
    pub values: Option<Vec<String>>,
}

/// The schema identifier of a shared auth file, `auth/<name>.yaml`.
pub const AUTH_SCHEMA: &str = "urn:beyond10x:connectors-auth:1";
/// The file the lowering writes every auth profile into.
pub const AUTH_FORMAT: &str = "connectors-catalog-auth/1";

/// `connectors_definitions.definition.AuthScheme`: `basic` sends
/// `Authorization: Basic`, `bearer` `Authorization: Bearer`, `header` the token
/// in `header`, behind `prefix` when one is given.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AuthScheme {
    Basic,
    Bearer,
    Header,
}

/// `connectors_definitions.definition.AuthIdentity`: the read that names who
/// holds the credential.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AuthIdentity {
    pub path: Option<String>,
    pub kind: Option<String>,
    pub subject: Option<String>,
}

/// `connectors_definitions.definition.AuthScopes`: the read that names the
/// granted scopes, and those a connection must hold.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AuthScopes {
    pub path: String,
    pub pointer: String,
    pub minimum: Option<Vec<String>>,
}

/// `connectors_definitions.definition.AuthAccess`: a read whose `200` proves
/// the credential reaches what the selection needs.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AuthAccess {
    pub path: String,
    pub query: Option<BTreeMap<String, String>>,
}

/// `connectors_definitions.definition.Auth`: one profile. With `use`, the
/// profile of the same id in `auth/<use>.yaml` is the base, and every field
/// given here replaces its counterpart; `identity` field by field.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Auth {
    #[serde(rename = "use")]
    pub library: Option<String>,
    pub default: Option<bool>,
    pub scheme: Option<AuthScheme>,
    pub header: Option<String>,
    pub prefix: Option<String>,
    pub label: Option<String>,
    pub account_label: Option<String>,
    pub identity: Option<AuthIdentity>,
    pub scopes: Option<AuthScopes>,
    pub access: Option<AuthAccess>,
}

/// `connectors_definitions.definition.AuthLibraryId`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum AuthLibraryId {
    #[serde(rename = "urn:beyond10x:connectors-auth:1")]
    V1,
}

/// `connectors_definitions.definition.AuthLibrary`: profiles several providers
/// share, `auth/<name>.yaml`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AuthLibrary {
    pub schema: AuthLibraryId,
    pub profiles: BTreeMap<String, Auth>,
}

/// `connectors_definitions.definition.Effect`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Effect {
    Read,
    Write,
}

/// `connectors_definitions.definition.PagingStyle`: how one family of list
/// operations pages. `request` is the query parameter that carries the position;
/// `limit` the page-size parameter, capped at `max`. `next`, `total` and `last`
/// say where a walk reads its next position and its end; the runtime owns the
/// walk itself.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PagingStyle {
    pub request: Option<String>,
    pub next: Option<String>,
    pub limit: Option<String>,
    pub max: Option<u64>,
    pub total: Option<String>,
    pub last: Option<String>,
}

/// `connectors_definitions.definition.GuardBefore`: the read before a write.
/// `read` is an operation key of the definition, else an `operationId` of the
/// source; its path parameters take the input of the same name unless `with`
/// binds them. `expect` maps a JSON pointer to an input reference (`sha`,
/// `body.sha`), a literal (`=opened`) or a boolean (`=true`), in file order.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GuardBefore {
    pub read: String,
    pub with: Option<BTreeMap<String, String>>,
    pub expect: serde_yaml_ng::Mapping,
}

/// `connectors_definitions.definition.GuardAfter`: comparisons against the
/// write's answer; `any` holds when at least one of its comparisons holds.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GuardAfter {
    pub expect: serde_yaml_ng::Mapping,
    pub any: Option<serde_yaml_ng::Mapping>,
}

/// `connectors_definitions.definition.Guard`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Guard {
    pub before: GuardBefore,
    pub after: GuardAfter,
}

/// `connectors_definitions.definition.Operation`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Operation {
    pub op: String,
    pub effect: Option<Effect>,
    pub description: Option<String>,
    /// A style from `paging`, for an operation more than one style matches.
    pub paging: Option<String>,
    /// The closed body of a write, by key: `string`, `integer`, `boolean` or
    /// `any`, required with a trailing `!`; or `=<value>`, sent fixed.
    pub body: Option<BTreeMap<String, String>>,
    pub guard: Option<Guard>,
}

/// `connectors_definitions.definition.ProviderDefinition`. The maps are ordered
/// by key, so a lowering is a function of the definition's content, not of the
/// order its file happens to list them in.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProviderDefinition {
    pub schema: SchemaId,
    pub source: Source,
    pub base_url: String,
    pub config: Option<BTreeMap<String, ConfigField>>,
    pub auth: BTreeMap<String, Auth>,
    pub paging: Option<BTreeMap<String, PagingStyle>>,
    pub operations: BTreeMap<String, Operation>,
}

/// Why a definition was not lowered. Each case is distinct, and [`Refusal::name`]
/// names it, so a caller reports which rule refused rather than that one did.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Refusal {
    /// The definition file, the vendored source or its amendments cannot be read.
    Unreadable(String),
    /// Not YAML, a missing field, or a value outside the schema's vocabulary.
    Malformed(String),
    /// A field schema v1 does not name, anywhere in the definition.
    UnknownField(String),
    /// A `schema` other than [`SCHEMA`].
    WrongSchema(String),
    /// No auth profile, or more than one and other than exactly one marked
    /// `default: true`; the count is how many are marked.
    AuthDefault(usize),
    /// A shared auth file that cannot be read, or is not a valid
    /// `urn:beyond10x:connectors-auth:1` file, or a `use` that is not a plain
    /// file name.
    AuthLibrary(String),
    /// A profile `use`s a shared file that has no profile of the same id.
    AuthProfileMissing { key: String, library: String },
    /// A profile, after `use`, lacks a field its scheme needs.
    AuthIncomplete { key: String, field: String },
    /// A `{variable}` in `base_url` that `config` does not declare.
    UndeclaredVariable(String),
    /// A `source` path that is absolute or leaves the definitions root.
    SourceOutsideRoot(String),
    /// A vendored source that is not a gzip file named `<name>.gz`.
    SourceNotGzip(String),
    /// A vendored source that decompresses past [`crate::SOURCE_LIMIT`].
    SourceTooLarge(String),
    /// The decompressed source does not hash to `source.sha256`.
    SourceHashMismatch { expected: String, actual: String },
    /// An `op` the bundle inventory carries no operation for.
    UnknownOperation { key: String, op: String },
    /// An operation whose method is not `get` and that declares no `effect`.
    EffectRequired {
        key: String,
        op: String,
        method: String,
    },
    /// An operation names a `paging` style the definition does not declare.
    UnknownPaging { key: String, style: String },
    /// More than one style's `request` parameter is a query parameter of the
    /// operation, and the operation names none of them.
    AmbiguousPaging { key: String, styles: Vec<String> },
    /// An operation names a style whose `request` or `limit` parameter it lacks.
    PagingParameter {
        key: String,
        style: String,
        parameter: String,
    },
    /// A guard reads something that is neither an operation key of the
    /// definition nor an `operationId` of the source.
    UnknownGuardRead { key: String, read: String },
    /// A guard binds, with `with`, a parameter its read does not have.
    GuardParameter { key: String, parameter: String },
    /// A body key whose spec is not `string`, `integer`, `boolean` or `any`
    /// (each optionally ending in `!`), nor `=<value>`.
    BodyType {
        key: String,
        field: String,
        spec: String,
    },
    /// The catalog pipeline refused the source; the message names its step.
    Pipeline(String),
    /// The output directory, the selection or the scratch copy cannot be written.
    Write(String),
}

impl Refusal {
    pub fn name(&self) -> &'static str {
        match self {
            Self::Unreadable(_) => "unreadable",
            Self::Malformed(_) => "malformed",
            Self::UnknownField(_) => "unknown_field",
            Self::WrongSchema(_) => "wrong_schema",
            Self::AuthDefault(_) => "auth_default",
            Self::AuthLibrary(_) => "auth_library",
            Self::AuthProfileMissing { .. } => "auth_profile_missing",
            Self::AuthIncomplete { .. } => "auth_incomplete",
            Self::UndeclaredVariable(_) => "undeclared_variable",
            Self::SourceOutsideRoot(_) => "source_outside_root",
            Self::SourceNotGzip(_) => "source_not_gzip",
            Self::SourceTooLarge(_) => "source_too_large",
            Self::SourceHashMismatch { .. } => "source_hash_mismatch",
            Self::UnknownOperation { .. } => "unknown_operation",
            Self::EffectRequired { .. } => "effect_required",
            Self::UnknownPaging { .. } => "unknown_paging",
            Self::AmbiguousPaging { .. } => "ambiguous_paging",
            Self::PagingParameter { .. } => "paging_parameter",
            Self::UnknownGuardRead { .. } => "unknown_guard_read",
            Self::GuardParameter { .. } => "guard_parameter",
            Self::BodyType { .. } => "body_type",
            Self::Pipeline(_) => "pipeline",
            Self::Write(_) => "write",
        }
    }
}

impl std::fmt::Display for Refusal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name = self.name();
        match self {
            Self::Unreadable(detail)
            | Self::Malformed(detail)
            | Self::UnknownField(detail)
            | Self::SourceOutsideRoot(detail)
            | Self::SourceNotGzip(detail)
            | Self::SourceTooLarge(detail)
            | Self::AuthLibrary(detail)
            | Self::Pipeline(detail)
            | Self::Write(detail) => write!(f, "{name}: {detail}"),
            Self::WrongSchema(schema) => write!(f, "{name}: `{schema}`, expected `{SCHEMA}`"),
            Self::AuthDefault(count) => write!(
                f,
                "{name}: {count} auth profiles marked default; with more than one profile, exactly one is"
            ),
            Self::AuthProfileMissing { key, library } => write!(
                f,
                "{name}: auth profile `{key}` uses `auth/{library}.yaml`, which has no profile `{key}`"
            ),
            Self::AuthIncomplete { key, field } => {
                write!(f, "{name}: auth profile `{key}` has no `{field}`")
            }
            Self::UndeclaredVariable(variable) => write!(
                f,
                "{name}: base_url variable `{variable}` is not declared in config"
            ),
            Self::SourceHashMismatch { expected, actual } => write!(
                f,
                "{name}: source.sha256 is {expected}, the decompressed source hashes to {actual}"
            ),
            Self::UnknownOperation { key, op } => write!(
                f,
                "{name}: operation `{key}` names op `{op}`, which the bundle inventory lacks"
            ),
            Self::EffectRequired { key, op, method } => write!(
                f,
                "{name}: operation `{key}` (`{op}`) is {} and declares no effect",
                method.to_uppercase()
            ),
            Self::UnknownPaging { key, style } => write!(
                f,
                "{name}: operation `{key}` names paging style `{style}`, which paging does not declare"
            ),
            Self::AmbiguousPaging { key, styles } => write!(
                f,
                "{name}: operation `{key}` matches paging styles {}; name one",
                styles.join(", ")
            ),
            Self::PagingParameter {
                key,
                style,
                parameter,
            } => write!(
                f,
                "{name}: operation `{key}` names paging style `{style}`, but has no query parameter `{parameter}`"
            ),
            Self::UnknownGuardRead { key, read } => write!(
                f,
                "{name}: operation `{key}`'s guard reads `{read}`, which is neither an operation key nor a source operationId"
            ),
            Self::GuardParameter { key, parameter } => write!(
                f,
                "{name}: operation `{key}`'s guard binds `{parameter}`, which its read does not have"
            ),
            Self::BodyType { key, field, spec } => write!(
                f,
                "{name}: operation `{key}` body `{field}` is `{spec}`; expected string, integer, boolean or any, optionally with `!`, or `=<value>`"
            ),
        }
    }
}

impl std::error::Error for Refusal {}

/// Read one definition. A wrong `schema` is told apart before the typed read,
/// so it is refused as itself rather than as an unknown enum variant.
pub fn parse(bytes: &[u8]) -> Result<ProviderDefinition, Refusal> {
    let value: serde_yaml_ng::Value =
        serde_yaml_ng::from_slice(bytes).map_err(|error| Refusal::Malformed(error.to_string()))?;
    if let Some(schema) = value.get("schema") {
        match schema.as_str() {
            Some(SCHEMA) => {}
            Some(other) => return Err(Refusal::WrongSchema(other.to_owned())),
            None => return Err(Refusal::Malformed("schema is not a string".to_owned())),
        }
    }
    let definition: ProviderDefinition = serde_yaml_ng::from_value(value).map_err(|error| {
        let message = error.to_string();
        // `deny_unknown_fields` reports through serde's `unknown_field`, whose
        // message is the only place the case is distinguishable.
        if message.starts_with("unknown field `") || message.contains(": unknown field `") {
            Refusal::UnknownField(message)
        } else {
            Refusal::Malformed(message)
        }
    })?;
    let digest = &definition.source.sha256;
    if digest.is_empty() || !digest.bytes().all(|b| b"0123456789abcdef".contains(&b)) {
        return Err(Refusal::Malformed(format!(
            "source.sha256 `{digest}` is not lowercase hexadecimal"
        )));
    }
    Ok(definition)
}

/// What to lower, from where, into where.
#[derive(Clone, Copy, Debug)]
pub struct Request<'a> {
    /// `providers/<id>.yaml`; the file stem is the provider id.
    pub definition: &'a Path,
    /// The definitions checkout `source.openapi` and `source.amendments` are under.
    pub root: &'a Path,
    /// The bundle directory the bundle, its index entry and the selection go to.
    pub out: &'a Path,
    /// An existing directory owned by the caller; the decompressed source and
    /// the staged bundle live in a fresh directory under it, removed on return.
    pub scratch: &'a Path,
}

/// What a completed lowering wrote.
#[derive(Clone, Debug)]
pub struct Lowered {
    pub provider: String,
    pub entry: bundle::Entry,
    pub bundle: PathBuf,
    pub operations: PathBuf,
    /// `<provider>.auth.json`: every profile as the provider's `auth` block.
    pub auth: PathBuf,
}

/// One row of the selection, in the shape the catalog provider's `Selection`
/// reads.
#[derive(Clone, Debug, Serialize)]
struct SelectionEntry {
    id: String,
    operation_id: String,
    effect: Effect,
    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<String>,
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    bounds: BTreeMap<String, Bound>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    body_keys: Vec<String>,
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    body_types: BTreeMap<String, String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    body_required: Vec<String>,
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    body_fixed: BTreeMap<String, Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    guard: Option<Value>,
}

/// The closed body a definition declares, in the selection's `body_*` fields.
#[derive(Default)]
struct Body {
    keys: Vec<String>,
    types: BTreeMap<String, String>,
    required: Vec<String>,
    fixed: BTreeMap<String, Value>,
}

fn body(key: &str, declared: Option<&BTreeMap<String, String>>) -> Result<Body, Refusal> {
    let mut body = Body::default();
    for (field, spec) in declared.into_iter().flatten() {
        body.keys.push(field.clone());
        if let Some(value) = spec.strip_prefix('=') {
            let fixed = match value {
                "true" => Value::Bool(true),
                "false" => Value::Bool(false),
                _ => value
                    .parse::<i64>()
                    .map_or_else(|_| Value::String(value.to_owned()), Value::from),
            };
            body.fixed.insert(field.clone(), fixed);
            continue;
        }
        let (kind, required) = match spec.strip_suffix('!') {
            Some(kind) => (kind, true),
            None => (spec.as_str(), false),
        };
        match kind {
            "string" | "integer" | "boolean" => {
                body.types.insert(field.clone(), kind.to_owned());
            }
            "any" => {}
            _ => {
                return Err(Refusal::BodyType {
                    key: key.to_owned(),
                    field: field.clone(),
                    spec: spec.clone(),
                });
            }
        }
        if required {
            body.required.push(field.clone());
        }
    }
    Ok(body)
}

/// One `expect` entry as the engine's check: `=true`/`=false` a boolean, any
/// other `=…` a literal, else an input reference.
fn checks(mapping: &serde_yaml_ng::Mapping) -> Result<Vec<Value>, Refusal> {
    mapping
        .iter()
        .map(|(pointer, expected)| {
            let (Some(pointer), Some(expected)) = (pointer.as_str(), expected.as_str()) else {
                return Err(Refusal::Malformed(
                    "a guard's expect maps a pointer string to a string".to_owned(),
                ));
            };
            let expect = match expected.strip_prefix('=') {
                Some("true") => serde_json::json!({"boolean": true}),
                Some("false") => serde_json::json!({"boolean": false}),
                Some(literal) => serde_json::json!({"literal": literal}),
                None => serde_json::json!({"input": expected}),
            };
            Ok(serde_json::json!({"pointer": pointer, "expect": expect}))
        })
        .collect()
}

/// A range bound, in the shape the catalog provider's `Bound` reads.
#[derive(Clone, Debug, Serialize)]
struct Bound {
    minimum: u64,
    maximum: u64,
}

/// The paging style an operation uses, by name, given its query parameters: the
/// one it names, else the one style whose `request` parameter it has, else none.
fn paging_style<'a>(
    key: &str,
    operation: &Operation,
    styles: Option<&'a BTreeMap<String, PagingStyle>>,
    query: &[&str],
) -> Result<Option<(&'a str, &'a PagingStyle)>, Refusal> {
    let Some(styles) = styles else {
        return match &operation.paging {
            Some(named) => Err(Refusal::UnknownPaging {
                key: key.to_owned(),
                style: named.clone(),
            }),
            None => Ok(None),
        };
    };
    if let Some(named) = &operation.paging {
        let (name, style) = styles
            .get_key_value(named)
            .ok_or_else(|| Refusal::UnknownPaging {
                key: key.to_owned(),
                style: named.clone(),
            })?;
        for parameter in [&style.request, &style.limit].into_iter().flatten() {
            if !query.contains(&parameter.as_str()) {
                return Err(Refusal::PagingParameter {
                    key: key.to_owned(),
                    style: name.clone(),
                    parameter: parameter.clone(),
                });
            }
        }
        return Ok(Some((name.as_str(), style)));
    }
    let matching: Vec<(&String, &PagingStyle)> = styles
        .iter()
        .filter(|(_, style)| {
            style
                .request
                .as_deref()
                .is_some_and(|request| query.contains(&request))
        })
        .collect();
    match matching[..] {
        [] => Ok(None),
        [(name, style)] => Ok(Some((name.as_str(), style))),
        _ => Err(Refusal::AmbiguousPaging {
            key: key.to_owned(),
            styles: matching.iter().map(|(name, _)| (*name).clone()).collect(),
        }),
    }
}

#[derive(Serialize)]
struct OperationsFile<'a> {
    format: &'a str,
    provider: &'a str,
    operations: Vec<SelectionEntry>,
}

/// The `{name}` variables of a URL template, in order.
fn variables(template: &str) -> Result<Vec<&str>, Refusal> {
    let malformed = || Refusal::Malformed(format!("base_url `{template}` is not a URL template"));
    let mut names = Vec::new();
    let mut rest = template;
    while let Some(open) = rest.find(['{', '}']) {
        if rest.as_bytes()[open] == b'}' {
            return Err(malformed());
        }
        let after = &rest[open + 1..];
        let close = after.find('}').ok_or_else(malformed)?;
        let name = &after[..close];
        if name.is_empty() || name.contains('{') {
            return Err(malformed());
        }
        names.push(name);
        rest = &after[close + 1..];
    }
    Ok(names)
}

/// A path the definition names, resolved under the definitions root. Only plain
/// relative components are accepted, and the resolved file must still lie under
/// the root once links are followed.
fn under_root(root: &Path, relative: &str) -> Result<PathBuf, Refusal> {
    let outside = || Refusal::SourceOutsideRoot(relative.to_owned());
    let path = Path::new(relative);
    if relative.is_empty()
        || !path
            .components()
            .all(|component| matches!(component, Component::Normal(_)))
    {
        return Err(outside());
    }
    let unreadable = |error: std::io::Error| Refusal::Unreadable(format!("{relative}: {error}"));
    let root = root.canonicalize().map_err(unreadable)?;
    let resolved = root.join(path).canonicalize().map_err(unreadable)?;
    if !resolved.starts_with(&root) {
        return Err(outside());
    }
    Ok(resolved)
}

fn gunzip(name: &str, bytes: &[u8]) -> Result<Vec<u8>, Refusal> {
    let mut out = Vec::new();
    flate2::read::MultiGzDecoder::new(bytes)
        .take(crate::SOURCE_LIMIT as u64 + 1)
        .read_to_end(&mut out)
        .map_err(|error| Refusal::SourceNotGzip(format!("{name}: {error}")))?;
    if out.len() > crate::SOURCE_LIMIT {
        return Err(Refusal::SourceTooLarge(format!(
            "{name} decompresses past {} bytes",
            crate::SOURCE_LIMIT
        )));
    }
    Ok(out)
}

/// The `summary` of the operation `operation_id` names in the source document.
fn summary(document: &Value, operation_id: &str) -> Option<String> {
    document
        .get("paths")?
        .as_object()?
        .values()
        .filter_map(Value::as_object)
        .flat_map(|item| {
            crate::inventory::METHODS
                .iter()
                .filter_map(move |method| item.get(*method))
        })
        .find(|operation| {
            operation.get("operationId").and_then(Value::as_str) == Some(operation_id)
        })?
        .get("summary")?
        .as_str()
        .map(str::to_owned)
}

/// `base` with every field `over` gives replacing its counterpart, `identity`
/// field by field.
fn merged(base: &Auth, over: &Auth) -> Auth {
    let identity = match (&base.identity, &over.identity) {
        (Some(base), Some(over)) => Some(AuthIdentity {
            path: over.path.clone().or_else(|| base.path.clone()),
            kind: over.kind.clone().or_else(|| base.kind.clone()),
            subject: over.subject.clone().or_else(|| base.subject.clone()),
        }),
        (base, over) => over.clone().or_else(|| base.clone()),
    };
    Auth {
        library: None,
        default: over.default.or(base.default),
        scheme: over.scheme.or(base.scheme),
        header: over.header.clone().or_else(|| base.header.clone()),
        prefix: over.prefix.clone().or_else(|| base.prefix.clone()),
        label: over.label.clone().or_else(|| base.label.clone()),
        account_label: over
            .account_label
            .clone()
            .or_else(|| base.account_label.clone()),
        identity,
        scopes: over.scopes.clone().or_else(|| base.scopes.clone()),
        access: over.access.clone().or_else(|| base.access.clone()),
    }
}

/// Every profile of the definition with its `use` resolved, and the one the
/// bundle records: the only one, or the one marked `default: true`.
fn resolve_auth(
    definition: &ProviderDefinition,
    root: &Path,
) -> Result<(String, BTreeMap<String, Auth>), Refusal> {
    let mut resolved = BTreeMap::new();
    for (id, entry) in &definition.auth {
        let profile = match &entry.library {
            None => entry.clone(),
            Some(library) => {
                if library.is_empty()
                    || !library
                        .bytes()
                        .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
                {
                    return Err(Refusal::AuthLibrary(format!(
                        "`use: {library}` is not a plain lowercase name"
                    )));
                }
                let path = under_root(root, &format!("auth/{library}.yaml"))
                    .map_err(|refusal| Refusal::AuthLibrary(refusal.to_string()))?;
                let bytes = std::fs::read(&path).map_err(|error| {
                    Refusal::AuthLibrary(format!("auth/{library}.yaml: {error}"))
                })?;
                let shared: AuthLibrary = serde_yaml_ng::from_slice(&bytes).map_err(|error| {
                    Refusal::AuthLibrary(format!("auth/{library}.yaml: {error}"))
                })?;
                let base = shared
                    .profiles
                    .get(id)
                    .ok_or_else(|| Refusal::AuthProfileMissing {
                        key: id.clone(),
                        library: library.clone(),
                    })?;
                if base.library.is_some() || base.default.is_some() {
                    return Err(Refusal::AuthLibrary(format!(
                        "auth/{library}.yaml profile `{id}` carries `use` or `default`, which only a provider's profile may"
                    )));
                }
                merged(base, entry)
            }
        };
        resolved.insert(id.clone(), profile);
    }
    let marked: Vec<&String> = resolved
        .iter()
        .filter(|(_, profile)| profile.default == Some(true))
        .map(|(id, _)| id)
        .collect();
    let default = match (resolved.len(), &marked[..]) {
        (1, []) => resolved.keys().next().cloned(),
        (_, [only]) => Some((*only).clone()),
        _ => None,
    }
    .ok_or(Refusal::AuthDefault(marked.len()))?;
    Ok((default, resolved))
}

/// One resolved profile as the catalog provider's `auth` block.
fn auth_block(id: &str, profile: &Auth) -> Result<Value, Refusal> {
    let missing = |field: &str| Refusal::AuthIncomplete {
        key: id.to_owned(),
        field: field.to_owned(),
    };
    let scheme = profile.scheme.ok_or_else(|| missing("scheme"))?;
    let label = profile.label.as_ref().ok_or_else(|| missing("label"))?;
    let identity = profile
        .identity
        .as_ref()
        .ok_or_else(|| missing("identity"))?;
    let mut block = serde_json::json!({
        "profile": id,
        "label": label,
        "identity": {
            "path": identity.path.as_ref().ok_or_else(|| missing("identity.path"))?,
            "kind": identity.kind.as_ref().ok_or_else(|| missing("identity.kind"))?,
        },
    });
    if let Some(subject) = &identity.subject {
        block["identity"]["subject_pointer"] = Value::from(subject.as_str());
    }
    let header = profile.header.clone();
    match scheme {
        AuthScheme::Basic => {
            block["scheme"] = Value::from("basic");
            block["header"] = Value::from(header.unwrap_or_else(|| "Authorization".to_owned()));
            block["bearer"] = Value::Bool(false);
            block["account_label"] = Value::from(
                profile
                    .account_label
                    .as_ref()
                    .ok_or_else(|| missing("account_label"))?
                    .as_str(),
            );
        }
        AuthScheme::Bearer => {
            block["header"] = Value::from(header.unwrap_or_else(|| "Authorization".to_owned()));
            block["bearer"] = Value::Bool(true);
        }
        AuthScheme::Header => {
            block["header"] = Value::from(header.ok_or_else(|| missing("header"))?);
            block["bearer"] = Value::Bool(false);
            if let Some(prefix) = &profile.prefix {
                block["prefix"] = Value::from(prefix.as_str());
            }
        }
    }
    if let Some(scopes) = &profile.scopes {
        block["scopes"] = serde_json::json!({"path": scopes.path, "pointer": scopes.pointer});
        if let Some(minimum) = &scopes.minimum {
            block["minimum_scopes"] = serde_json::json!(minimum);
        }
    }
    if let Some(access) = &profile.access {
        block["access"] = serde_json::json!({"path": access.path});
        if let Some(query) = &access.query {
            block["access"]["query"] = serde_json::json!(query);
        }
    }
    Ok(block)
}

/// Lower one definition. Every check that can refuse runs before the output
/// directory is touched: the bundle is compiled into a staging directory under
/// `scratch`, the selection is resolved against it, and only then are the bundle
/// (with its index entry) and the selection written to `out`.
pub fn lower(request: &Request<'_>) -> Result<Lowered, Refusal> {
    let shown = request.definition.display();
    let bytes = std::fs::read(request.definition)
        .map_err(|error| Refusal::Unreadable(format!("{shown}: {error}")))?;
    let definition = parse(&bytes)?;
    let provider = request
        .definition
        .file_stem()
        .and_then(|stem| stem.to_str())
        .ok_or_else(|| Refusal::Unreadable(format!("{shown}: no file stem")))?;

    let (default_profile, profiles) = resolve_auth(&definition, request.root)?;
    let auth_profile = default_profile.as_str();
    let mut blocks = serde_json::Map::new();
    for (id, profile) in &profiles {
        blocks.insert(id.clone(), auth_block(id, profile)?);
    }
    for variable in variables(&definition.base_url)? {
        if !definition
            .config
            .as_ref()
            .is_some_and(|config| config.contains_key(variable))
        {
            return Err(Refusal::UndeclaredVariable(variable.to_owned()));
        }
    }

    let openapi = &definition.source.openapi;
    let source_path = under_root(request.root, openapi)?;
    let file_name = source_path
        .file_name()
        .and_then(|name| name.to_str())
        .and_then(|name| name.strip_suffix(".gz"))
        .filter(|name| !name.is_empty())
        .ok_or_else(|| Refusal::SourceNotGzip(format!("{openapi} is not named `<name>.gz`")))?;
    let compressed = std::fs::read(&source_path)
        .map_err(|error| Refusal::Unreadable(format!("{openapi}: {error}")))?;
    let source = gunzip(openapi, &compressed)?;
    let actual = hex::encode(Sha256::digest(&source));
    if actual != definition.source.sha256 {
        return Err(Refusal::SourceHashMismatch {
            expected: definition.source.sha256.clone(),
            actual,
        });
    }
    let amendments = definition
        .source
        .amendments
        .as_deref()
        .map(|path| under_root(request.root, path))
        .transpose()?;

    let write = |error: std::io::Error| Refusal::Write(error.to_string());
    let staging = tempfile::Builder::new()
        .prefix("definition-")
        .tempdir_in(request.scratch)
        .map_err(write)?;
    let source_file = staging.path().join(file_name);
    std::fs::write(&source_file, &source).map_err(write)?;
    let bundles = staging.path().join("bundles");
    pipeline::run(&pipeline::Request {
        provider,
        source: &source_file,
        directory: &bundles,
        auth_profile,
        replace: false,
        amendments: amendments.as_deref(),
    })
    .map_err(|failure| Refusal::Pipeline(failure.to_string()))?;
    let bundle =
        bundle::load(&bundles, provider).map_err(|error| Refusal::Pipeline(error.to_string()))?;
    let document = crate::parse_document(&source)
        .map_err(|refusal| Refusal::Pipeline(refusal.reason().to_owned()))?;

    let mut operations = Vec::new();
    for (key, operation) in &definition.operations {
        let found = bundle
            .inventory
            .operations
            .iter()
            .find(|candidate| candidate.operation_id.as_deref() == Some(operation.op.as_str()))
            .ok_or_else(|| Refusal::UnknownOperation {
                key: key.clone(),
                op: operation.op.clone(),
            })?;
        let effect = match operation.effect {
            Some(effect) => effect,
            None if found.method == "get" => Effect::Read,
            None => {
                return Err(Refusal::EffectRequired {
                    key: key.clone(),
                    op: operation.op.clone(),
                    method: found.method.clone(),
                });
            }
        };
        let query: Vec<&str> = found
            .parameters
            .iter()
            .filter(|parameter| parameter.location == crate::inventory::Location::Query)
            .map(|parameter| parameter.name.as_str())
            .collect();
        let mut bounds = BTreeMap::new();
        if let Some((_, style)) = paging_style(key, operation, definition.paging.as_ref(), &query)?
            && let (Some(limit), Some(max)) = (&style.limit, style.max)
            && query.contains(&limit.as_str())
        {
            bounds.insert(
                limit.clone(),
                Bound {
                    minimum: 1,
                    maximum: max,
                },
            );
        }
        let declared_body = body(key, operation.body.as_ref())?;
        let guard = operation
            .guard
            .as_ref()
            .map(|guard| {
                let read = definition
                    .operations
                    .get(&guard.before.read)
                    .map_or(guard.before.read.as_str(), |local| local.op.as_str());
                let probe = bundle
                    .inventory
                    .operations
                    .iter()
                    .find(|candidate| candidate.operation_id.as_deref() == Some(read))
                    .ok_or_else(|| Refusal::UnknownGuardRead {
                        key: key.clone(),
                        read: guard.before.read.clone(),
                    })?;
                let mut values: BTreeMap<String, String> = probe
                    .parameters
                    .iter()
                    .filter(|parameter| parameter.location == crate::inventory::Location::Path)
                    .map(|parameter| (parameter.name.clone(), parameter.name.clone()))
                    .collect();
                for (parameter, input) in guard.before.with.iter().flatten() {
                    if !probe.parameters.iter().any(|p| &p.name == parameter) {
                        return Err(Refusal::GuardParameter {
                            key: key.clone(),
                            parameter: parameter.clone(),
                        });
                    }
                    values.insert(parameter.clone(), input.clone());
                }
                let mut postflight = serde_json::json!({"checks": checks(&guard.after.expect)?});
                if let Some(any) = &guard.after.any {
                    postflight["any_of"] = Value::Array(checks(any)?);
                }
                Ok(serde_json::json!({
                    "preflight": {
                        "operation_id": read,
                        "values": values,
                        "checks": checks(&guard.before.expect)?,
                    },
                    "postflight": postflight,
                }))
            })
            .transpose()?;
        operations.push(SelectionEntry {
            id: key.clone(),
            operation_id: operation.op.clone(),
            effect,
            description: operation
                .description
                .clone()
                .or_else(|| summary(&document, &operation.op)),
            bounds,
            body_keys: declared_body.keys,
            body_types: declared_body.types,
            body_required: declared_body.required,
            body_fixed: declared_body.fixed,
            guard,
        });
    }
    let mut selection = serde_json::to_vec_pretty(&OperationsFile {
        format: OPERATIONS_FORMAT,
        provider,
        operations,
    })
    .map_err(|error| Refusal::Write(error.to_string()))?;
    selection.push(b'\n');

    let entry = bundle::write(request.out, &bundle, true)
        .map_err(|error| Refusal::Write(error.to_string()))?;
    let operations_path = request.out.join(format!("{provider}.operations.json"));
    std::fs::write(&operations_path, selection).map_err(write)?;
    let mut auth = serde_json::to_vec_pretty(&serde_json::json!({
        "format": AUTH_FORMAT,
        "provider": provider,
        "default": auth_profile,
        "profiles": blocks,
    }))
    .map_err(|error| Refusal::Write(error.to_string()))?;
    auth.push(b'\n');
    let auth_path = request.out.join(format!("{provider}.auth.json"));
    std::fs::write(&auth_path, auth).map_err(write)?;
    Ok(Lowered {
        provider: provider.to_owned(),
        bundle: request.out.join(&entry.file_name),
        entry,
        operations: operations_path,
        auth: auth_path,
    })
}
