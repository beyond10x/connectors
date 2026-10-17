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

/// `connectors_definitions.definition.AuthScheme`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AuthScheme {
    Basic,
    Bearer,
}

/// `connectors_definitions.definition.Auth`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Auth {
    pub scheme: AuthScheme,
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

/// `connectors_definitions.definition.Operation`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Operation {
    pub op: String,
    pub effect: Option<Effect>,
    pub description: Option<String>,
    /// A style from `paging`, for an operation more than one style matches.
    pub paging: Option<String>,
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
    /// `auth` with other than exactly one entry; v1 lowers one profile.
    AuthCount(usize),
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
            Self::AuthCount(_) => "auth_count",
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
            | Self::Pipeline(detail)
            | Self::Write(detail) => write!(f, "{name}: {detail}"),
            Self::WrongSchema(schema) => write!(f, "{name}: `{schema}`, expected `{SCHEMA}`"),
            Self::AuthCount(count) => {
                write!(f, "{name}: {count} auth entries; v1 lowers exactly one")
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

    let [auth_profile] = definition.auth.keys().collect::<Vec<_>>()[..] else {
        return Err(Refusal::AuthCount(definition.auth.len()));
    };
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
        operations.push(SelectionEntry {
            id: key.clone(),
            operation_id: operation.op.clone(),
            effect,
            description: operation
                .description
                .clone()
                .or_else(|| summary(&document, &operation.op)),
            bounds,
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
    Ok(Lowered {
        provider: provider.to_owned(),
        bundle: request.out.join(&entry.file_name),
        entry,
        operations: operations_path,
    })
}
