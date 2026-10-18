//! The importer: one shipped `connectors-catalog-operations/1` selection, with
//! the bundle it selects from and the pinned source that bundle was compiled
//! from, written as a `connectors-definition/1` definition into a definitions
//! checkout, with its sources vendored beside it.
//!
//! Everything is read from a Connectors checkout; nothing reaches the network.
//! The output is a function of those files: importing twice writes the same
//! bytes. Before anything is written, the definition is lowered in memory
//! ([`definition::select`]) and must give back the shipped selection as a JSON
//! value; a selection the definition notation cannot say is refused by name
//! ([`Failure::Unexpressible`]) rather than written approximately.
//!
//! What the selection does not say is taken from the Connectors files that do:
//! - the source is the file under `adapters/` whose name and SHA-256 the
//!   bundle's source record (or its derivation) names, and the amendments
//!   likewise;
//! - `base_url` and the auth profiles are the documented configuration, the
//!   `connectors-catalog-local/2` blocks in `docs/*.md` whose `operations_file`
//!   is this selection; the first block per profile id. In a URL a host label
//!   `your-<name>` becomes the variable `{<name>}` and a host under `.example`
//!   becomes `{hostname}`. The bundle's `auth_profile` is the default profile,
//!   taken from another selection of the same catalog provider when this one's
//!   documentation does not configure it;
//! - a profile id configured for more than one definition is shared: what all
//!   of them agree on goes into `auth/<prefix>.yaml` (the id up to its first
//!   `.`), and each definition `use`s it and gives the rest;
//! - URL, retrieval date and license come from the README beside the source.
//!
//! Choices the notation leaves open are made the same way every time: a paging
//! style (`limit` and `max`, no `request`) for a page-size bound that every
//! operation with that parameter carries, when at least two do or the feed
//! needs it; a field moved into `defaults` when every operation it applies to
//! carries the same value; a guard read by the first operation key that names
//! it; `effect` only where the lowerer cannot infer it; `description` only
//! where it is not the source's summary.

use crate::definition::{
    self, Auth, AuthAccess, AuthIdentity, AuthLibrary, AuthLibraryId, AuthScheme, AuthScopes,
    Binary, Bound, ConfigField, ConfigType, Custody, Datasource, DatasourceKind, Defaults,
    Download, Effect, Envelope, FeedAuthor, FeedBody, FeedContainers, FeedCursor, FeedItems,
    FeedPosition, Guard, GuardAfter, GuardBefore, GuardRead, GuardRefusal, IdentitySource,
    Operation, PagingStyle, ProviderDefinition, ResponseKind, SchemaId, Source, TextRefusal,
};
use crate::inventory::{Inventory, Location};
use indexmap::IndexMap;
use serde::Deserialize;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::io::Write as _;
use std::path::{Path, PathBuf};

/// What to import, from where, into where.
#[derive(Clone, Copy, Debug)]
pub struct Request<'a> {
    /// The Connectors checkout: `adapters/`, `docs/`.
    pub connectors: &'a Path,
    /// The catalog provider, the bundle's name.
    pub catalog: &'a str,
    /// The selection file; `adapters/catalog/providers/<catalog>/operations.json`
    /// when absent.
    pub selection: Option<&'a Path>,
    /// The definitions checkout written into.
    pub definitions: &'a Path,
    /// The definition's file stem; derived from the selection's file name when
    /// absent: `<catalog>` for `operations.json`, `<catalog>-<x>` for
    /// `<x>-operations.json`.
    pub name: Option<&'a str>,
}

/// What an import wrote.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Imported {
    pub name: String,
    /// `connectors/<name>.yaml` under the definitions root.
    pub definition: PathBuf,
    /// Every file written, the definition first, in the order written.
    pub written: Vec<PathBuf>,
}

/// Why an import was refused. Nothing is written when one is.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Failure {
    /// A Connectors file cannot be read.
    Unreadable(String),
    /// A Connectors file is not in the shape its format names.
    Malformed(String),
    /// What the import needs is not in the Connectors checkout: the source, the
    /// amendments, the documented configuration or a profile.
    NotFound(String),
    /// The selection says something the definition notation cannot.
    Unexpressible {
        operation: String,
        field: String,
        detail: String,
    },
    /// The imported definition does not lower; the lowerer's refusal.
    Lowering(String),
    /// The definitions checkout cannot be written.
    Write(String),
}

impl Failure {
    pub fn name(&self) -> &'static str {
        match self {
            Self::Unreadable(_) => "unreadable",
            Self::Malformed(_) => "malformed",
            Self::NotFound(_) => "not_found",
            Self::Unexpressible { .. } => "unexpressible",
            Self::Lowering(_) => "lowering",
            Self::Write(_) => "write",
        }
    }
}

impl std::fmt::Display for Failure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name = self.name();
        match self {
            Self::Unreadable(detail)
            | Self::Malformed(detail)
            | Self::NotFound(detail)
            | Self::Lowering(detail)
            | Self::Write(detail) => write!(f, "{name}: {detail}"),
            Self::Unexpressible {
                operation,
                field,
                detail,
            } => write!(f, "{name}: `{operation}` {field}: {detail}"),
        }
    }
}

impl std::error::Error for Failure {}

fn unexpressible(operation: &str, field: &str, detail: impl Into<String>) -> Failure {
    Failure::Unexpressible {
        operation: operation.to_owned(),
        field: field.to_owned(),
        detail: detail.into(),
    }
}

fn read(path: &Path) -> Result<Vec<u8>, Failure> {
    std::fs::read(path).map_err(|error| Failure::Unreadable(format!("{}: {error}", path.display())))
}

fn sha256(bytes: &[u8]) -> String {
    hex::encode(Sha256::digest(bytes))
}

// ---- the shipped selection, read as the engine reads it -------------------------------------

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ShippedFile {
    format: String,
    provider: String,
    operations: Vec<ShippedOperation>,
    #[serde(default)]
    feed: Option<ShippedFeed>,
}

#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct ShippedOperation {
    id: String,
    #[serde(default)]
    operation_id: Option<String>,
    effect: Effect,
    #[serde(default)]
    description: Option<String>,
    #[serde(default)]
    guard: Option<ShippedGuard>,
    #[serde(default)]
    response: Option<ResponseKind>,
    #[serde(default)]
    binary: Option<Binary>,
    #[serde(default)]
    download: Option<Download>,
    #[serde(default)]
    bounds: Option<IndexMap<String, Bound>>,
    #[serde(default)]
    required: Option<Vec<String>>,
    #[serde(default)]
    withhold: Option<Vec<String>>,
    #[serde(default)]
    credential: Option<Vec<String>>,
    #[serde(default)]
    rate_limit_reasons: Option<Vec<String>>,
    #[serde(default)]
    refusal_texts: Option<Vec<TextRefusal>>,
    #[serde(default)]
    custody: Option<Custody>,
    #[serde(default)]
    envelope: Option<Envelope>,
    #[serde(default)]
    body_keys: Vec<String>,
    #[serde(default)]
    body_types: BTreeMap<String, String>,
    #[serde(default)]
    body_required: Vec<String>,
    #[serde(default)]
    body_nonempty: Vec<String>,
    #[serde(default)]
    body_fixed: BTreeMap<String, Value>,
}

#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct ShippedGuard {
    preflight: ShippedPreflight,
    #[serde(default)]
    further_preflights: Vec<ShippedPreflight>,
    postflight: ShippedPostflight,
}

#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct ShippedPreflight {
    operation_id: String,
    values: IndexMap<String, String>,
    checks: Vec<ShippedCheck>,
}

#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct ShippedRead {
    operation_id: String,
    values: IndexMap<String, String>,
}

#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct ShippedPostflight {
    checks: Vec<ShippedCheck>,
    #[serde(default)]
    read: Option<ShippedRead>,
    #[serde(default)]
    any_of: Option<Vec<ShippedCheck>>,
    #[serde(default)]
    absent: Option<bool>,
    #[serde(default)]
    refusals: Option<Vec<GuardRefusal>>,
}

#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct ShippedCheck {
    pointer: String,
    expect: ShippedExpectation,
}

#[derive(Clone, Deserialize)]
#[serde(rename_all = "snake_case")]
enum ShippedExpectation {
    Input(String),
    Literal(String),
    Boolean(bool),
}

#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct ShippedFeed {
    profile: String,
    capabilities: BTreeMap<String, String>,
    #[allow(dead_code)]
    max_limit: u64,
    containers: ShippedContainers,
    items: ShippedItems,
}

#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct ShippedLookup {
    operation_id: String,
    #[allow(dead_code)]
    parameter: String,
    #[allow(dead_code)]
    record: String,
}

#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct ShippedContainers {
    operation_id: String,
    lookup: ShippedLookup,
    #[serde(default)]
    query: Option<BTreeMap<String, String>>,
    limit: String,
    cursor: FeedCursor,
    records: String,
    id: String,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    kind_word: Option<String>,
}

#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct ShippedPositionValue {
    parameter: String,
}

#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct ShippedPosition {
    kind: String,
    value: ShippedPositionValue,
}

#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct ShippedItems {
    operation_id: String,
    #[allow(dead_code)]
    container: String,
    #[serde(default)]
    query: Option<BTreeMap<String, String>>,
    limit: String,
    position: ShippedPosition,
    records: String,
    id: String,
    revision: String,
    created_at: String,
    updated_at: String,
    #[serde(default)]
    author: Option<FeedAuthor>,
    body: FeedBody,
}

// ---- the documented configuration -----------------------------------------------------------

/// A `connectors-catalog-local/2` block of a guide.
#[derive(Clone, Deserialize)]
struct DocumentedConfiguration {
    api_base: String,
    auth: DocumentedAuth,
    operations_file: Option<String>,
}

/// The `auth` block, as the guides write it.
#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct DocumentedAuth {
    profile: String,
    #[serde(default)]
    scheme: Option<String>,
    header: String,
    bearer: bool,
    #[serde(default)]
    prefix: Option<String>,
    #[serde(default)]
    account_label: Option<String>,
    label: String,
    identity: DocumentedIdentity,
    #[serde(default)]
    scopes: Option<DocumentedScopes>,
    #[serde(default)]
    access: Option<DocumentedAccess>,
    #[serde(default)]
    minimum_scopes: Vec<String>,
    #[serde(default)]
    evidence_lifetime_ms: Option<u64>,
    #[serde(default)]
    token_url: Option<String>,
    #[serde(default)]
    authorize_url: Option<String>,
    #[serde(default)]
    requested_scopes: Vec<String>,
}

#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct DocumentedIdentity {
    #[serde(default)]
    path: Option<String>,
    kind: String,
    #[serde(default)]
    subject_pointer: Option<String>,
    #[serde(default)]
    source: Option<IdentitySource>,
}

#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct DocumentedScopes {
    path: String,
    pointer: String,
}

#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct DocumentedAccess {
    path: String,
    #[serde(default)]
    query: BTreeMap<String, String>,
}

/// A connector declaration, `adapters/catalog/providers/<catalog>/connector.json`.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Declared {
    format: String,
    connector_id: String,
    #[allow(dead_code)]
    provider_id: String,
    #[allow(dead_code)]
    adapter_id: String,
    catalog_provider: String,
    #[serde(default)]
    default_endpoint: Option<String>,
    #[serde(default)]
    api_path: Option<String>,
    selection: String,
    profiles: Vec<DocumentedAuth>,
}

/// The scheme and authority of `url`, or all of it when it has no path.
fn origin(url: &str) -> &str {
    let after_scheme = url.find("://").map_or(0, |at| at + 3);
    url[after_scheme..]
        .find('/')
        .map_or(url, |at| &url[..after_scheme + at])
}

/// A URL of a guide with its placeholders as definition variables.
fn templated(url: &str) -> String {
    let Some(rest) = url.strip_prefix("https://") else {
        return url.to_owned();
    };
    let end = rest.find(['/', '?', '#']).unwrap_or(rest.len());
    let (host, tail) = rest.split_at(end);
    let host = if host.ends_with(".example") || host == "example" {
        "{hostname}".to_owned()
    } else {
        host.split('.')
            .map(|label| match label.strip_prefix("your-") {
                Some(name) if !name.is_empty() => format!("{{{name}}}"),
                _ => label.to_owned(),
            })
            .collect::<Vec<_>>()
            .join(".")
    };
    format!("https://{host}{tail}")
}

/// A guide's auth block as a definition profile.
fn documented_profile(block: &DocumentedAuth) -> Result<Auth, Failure> {
    let refused = |detail: &str| {
        Failure::Malformed(format!(
            "the documented `{}` profile {detail}",
            block.profile
        ))
    };
    let scheme = match (block.scheme.as_deref(), block.bearer) {
        (Some("basic"), false) => AuthScheme::Basic,
        (Some("oauth2_refresh"), true) => AuthScheme::OAuth2Refresh,
        (Some("oauth2_client_credentials"), true) => AuthScheme::OAuth2ClientCredentials,
        (None | Some("token"), true) => AuthScheme::Bearer,
        (None | Some("token"), false) => AuthScheme::Header,
        _ => return Err(refused("names a scheme the definition notation lacks")),
    };
    let header = (scheme == AuthScheme::Header || block.header != "Authorization")
        .then(|| block.header.clone());
    let scopes = AuthScopes {
        path: block.scopes.as_ref().map(|scopes| scopes.path.clone()),
        pointer: block.scopes.as_ref().map(|scopes| scopes.pointer.clone()),
        minimum: (!block.minimum_scopes.is_empty()).then(|| block.minimum_scopes.clone()),
        requested: (!block.requested_scopes.is_empty()).then(|| block.requested_scopes.clone()),
    };
    let has_scopes =
        scopes.path.is_some() || scopes.minimum.is_some() || scopes.requested.is_some();
    Ok(Auth {
        library: None,
        default: None,
        scheme: Some(scheme),
        header,
        prefix: block.prefix.clone(),
        label: Some(block.label.clone()),
        account_label: block.account_label.clone(),
        identity: Some(AuthIdentity {
            source: block
                .identity
                .source
                .filter(|source| *source != IdentitySource::Api),
            path: block.identity.path.clone(),
            kind: Some(block.identity.kind.clone()),
            subject: block.identity.subject_pointer.clone(),
        }),
        scopes: has_scopes.then_some(scopes),
        access: block.access.as_ref().map(|access| AuthAccess {
            path: access.path.clone(),
            query: (!access.query.is_empty()).then(|| access.query.clone()),
        }),
        evidence_lifetime_ms: block.evidence_lifetime_ms,
        token_url: block.token_url.as_deref().map(templated),
        authorize_url: block.authorize_url.as_deref().map(templated),
    })
}

/// The fenced code blocks of a Markdown file.
fn fenced_blocks(text: &str) -> Vec<String> {
    let mut blocks = Vec::new();
    let mut current: Option<String> = None;
    for line in text.lines() {
        if line.trim_start().starts_with("```") {
            match current.take() {
                Some(block) => blocks.push(block),
                None => current = Some(String::new()),
            }
        } else if let Some(block) = &mut current {
            block.push_str(line);
            block.push('\n');
        }
    }
    blocks
}

/// One shipped selection file and the definition it imports into.
#[derive(Clone, Debug)]
struct SelectionFile {
    name: String,
    catalog: String,
    /// `<catalog>/<file>`, relative to `adapters/catalog/providers`.
    relative: String,
}

fn definition_name(catalog: &str, file: &str) -> Option<String> {
    if file == "operations.json" {
        return Some(catalog.to_owned());
    }
    file.strip_suffix("-operations.json")
        .filter(|stem| !stem.is_empty())
        .map(|stem| format!("{catalog}-{stem}"))
}

fn sorted_entries(directory: &Path) -> Result<Vec<std::fs::DirEntry>, Failure> {
    let mut entries: Vec<std::fs::DirEntry> = std::fs::read_dir(directory)
        .map_err(|error| Failure::Unreadable(format!("{}: {error}", directory.display())))?
        .collect::<Result<_, _>>()
        .map_err(|error| Failure::Unreadable(format!("{}: {error}", directory.display())))?;
    entries.sort_by_key(std::fs::DirEntry::file_name);
    Ok(entries)
}

/// Every shipped selection file of the checkout.
fn shipped_selections(connectors: &Path) -> Result<Vec<SelectionFile>, Failure> {
    let providers = connectors.join("adapters/catalog/providers");
    let mut files = Vec::new();
    for provider in sorted_entries(&providers)? {
        if !provider.path().is_dir() {
            continue;
        }
        let catalog = provider.file_name().to_string_lossy().into_owned();
        for file in sorted_entries(&provider.path())? {
            let file = file.file_name().to_string_lossy().into_owned();
            if let Some(name) = definition_name(&catalog, &file) {
                files.push(SelectionFile {
                    name,
                    relative: format!("{catalog}/{file}"),
                    catalog: catalog.clone(),
                });
            }
        }
    }
    Ok(files)
}

/// Every documented configuration, keyed by the selection it names: the guides'
/// blocks in guide order, then the hand-written configurations the catalog tests
/// hold (`adapters/catalog/tests/hand_written/*.json`), one per file.
fn documented(connectors: &Path) -> Result<Vec<(String, DocumentedConfiguration)>, Failure> {
    let mut sources = Vec::new();
    for entry in sorted_entries(&connectors.join("docs"))? {
        let path = entry.path();
        if path.extension().is_none_or(|extension| extension != "md") {
            continue;
        }
        let text = String::from_utf8_lossy(&read(&path)?).into_owned();
        sources.push((path, fenced_blocks(&text)));
    }
    let hand_written = connectors.join("adapters/catalog/tests/hand_written");
    if hand_written.is_dir() {
        for entry in sorted_entries(&hand_written)? {
            let path = entry.path();
            if path.extension().is_none_or(|extension| extension != "json") {
                continue;
            }
            let text = String::from_utf8_lossy(&read(&path)?).into_owned();
            sources.push((path, vec![text]));
        }
    }
    let mut found = Vec::new();
    for (path, blocks) in sources {
        for block in blocks {
            let Ok(probe) = serde_json::from_str::<Value>(&block) else {
                continue;
            };
            if probe.get("format").and_then(Value::as_str) != Some("connectors-catalog-local/2") {
                continue;
            }
            let configuration: DocumentedConfiguration =
                serde_json::from_str(&block).map_err(|error| {
                    Failure::Malformed(format!(
                        "{}: a connectors-catalog-local/2 block: {error}",
                        path.display()
                    ))
                })?;
            let Some(file) = &configuration.operations_file else {
                continue;
            };
            let parts: Vec<&str> = file.rsplitn(4, '/').collect();
            if let [file, catalog, "providers", ..] = parts[..] {
                found.push((format!("{catalog}/{file}"), configuration.clone()));
            }
        }
    }
    Ok(found)
}

/// The profiles and base URL of every definition, as the guides configure them.
struct Configured {
    /// Definition name → profile id → profile, in guide order, the bundle's
    /// default profile last when the guide does not configure it.
    profiles: BTreeMap<String, IndexMap<String, Auth>>,
    /// Definition name → the first documented `api_base`.
    base: BTreeMap<String, String>,
    /// Definition name → the profile the bundle records.
    default: BTreeMap<String, String>,
    /// Definition name → its connector id, where the declaration names one
    /// other than the definition name.
    id: BTreeMap<String, String>,
    /// Definition name → the config variable the declaration's default
    /// endpoint defaults, and its value.
    defaults: BTreeMap<String, (String, String)>,
}

/// Take a definition's connector id, base URL and profiles from its declaration.
/// A declaration with `api_path` has a host the user names: the base URL is that
/// host as one variable and the path, and the default endpoint's host is the
/// variable's default. Without `api_path` a default endpoint is the base URL;
/// without either, the guides' `api_base` stays. A `token_url` path is on the
/// base URL's origin.
fn declare(configured: &mut Configured, name: &str, declared: Declared) -> Result<(), Failure> {
    let guide = configured.base.get(name).cloned();
    let base = match (&declared.api_path, &declared.default_endpoint) {
        (Some(path), endpoint) => {
            let documented = guide.as_deref().and_then(|base| {
                let rest = base.strip_prefix("https://{")?;
                let (variable, tail) = rest.split_once('}')?;
                (tail == path && !variable.contains(['{', '/'])).then(|| variable.to_owned())
            });
            let variable = documented.unwrap_or_else(|| "hostname".to_owned());
            if let Some(endpoint) = endpoint {
                let host = endpoint
                    .strip_prefix("https://")
                    .filter(|host| !host.is_empty() && !host.contains('/'))
                    .ok_or_else(|| {
                        Failure::Malformed(format!(
                            "the `{name}` declaration's default endpoint `{endpoint}` is not an https origin"
                        ))
                    })?;
                configured
                    .defaults
                    .insert(name.to_owned(), (variable.clone(), host.to_owned()));
            }
            format!("https://{{{variable}}}{path}")
        }
        (None, Some(endpoint)) => endpoint.clone(),
        (None, None) => guide.ok_or_else(|| {
            Failure::NotFound(format!("no guide documents a configuration of `{name}`"))
        })?,
    };
    let mut profiles = IndexMap::new();
    for block in &declared.profiles {
        let mut profile = documented_profile(block)?;
        if let Some(path) = profile
            .token_url
            .as_deref()
            .filter(|url| url.starts_with('/'))
        {
            profile.token_url = Some(format!("{}{path}", origin(&base)));
        }
        profiles.insert(block.profile.clone(), profile);
    }
    if declared.connector_id != name {
        configured.id.insert(name.to_owned(), declared.connector_id);
    }
    configured.base.insert(name.to_owned(), base);
    configured.profiles.insert(name.to_owned(), profiles);
    Ok(())
}

/// The declaration beside `selection`, when one names it.
fn declared(connectors: &Path, selection: &SelectionFile) -> Result<Option<Declared>, Failure> {
    let path = connectors
        .join("adapters/catalog/providers")
        .join(&selection.catalog)
        .join("connector.json");
    let bytes = match std::fs::read(&path) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(Failure::Unreadable(format!("{}: {error}", path.display()))),
    };
    let declared: Declared = serde_json::from_slice(&bytes)
        .map_err(|error| Failure::Malformed(format!("{}: {error}", path.display())))?;
    if declared.format != definition::CONNECTOR_FORMAT
        || declared.catalog_provider != selection.catalog
    {
        return Err(Failure::Malformed(format!(
            "{}: not a `{}` declaration of `{}`",
            path.display(),
            definition::CONNECTOR_FORMAT,
            selection.catalog
        )));
    }
    let file = selection.relative.rsplit('/').next().unwrap_or_default();
    Ok((declared.selection == file).then_some(declared))
}

fn configured(connectors: &Path) -> Result<Configured, Failure> {
    let selections = shipped_selections(connectors)?;
    let blocks = documented(connectors)?;
    let bundles = connectors.join("adapters/catalog/generated/bundles");
    let mut configured = Configured {
        profiles: BTreeMap::new(),
        base: BTreeMap::new(),
        default: BTreeMap::new(),
        id: BTreeMap::new(),
        defaults: BTreeMap::new(),
    };
    for selection in &selections {
        let mut profiles = IndexMap::new();
        for (relative, block) in &blocks {
            if *relative != selection.relative {
                continue;
            }
            configured
                .base
                .entry(selection.name.clone())
                .or_insert_with(|| templated(&block.api_base));
            if !profiles.contains_key(&block.auth.profile) {
                profiles.insert(block.auth.profile.clone(), documented_profile(&block.auth)?);
            }
        }
        if let Some(declared) = declared(connectors, selection)? {
            declare(&mut configured, &selection.name, declared)?;
            profiles = configured
                .profiles
                .remove(&selection.name)
                .unwrap_or_default();
        }
        let bundle = crate::bundle::load(&bundles, &selection.catalog).map_err(|error| {
            Failure::Unreadable(format!("bundle `{}`: {error}", selection.catalog))
        })?;
        let default = bundle.auth_profile.clone();
        if !profiles.contains_key(&default) {
            let elsewhere = blocks.iter().find(|(relative, block)| {
                relative.starts_with(&format!("{}/", selection.catalog))
                    && block.auth.profile == default
            });
            let (_, block) = elsewhere.ok_or_else(|| {
                Failure::NotFound(format!(
                    "no guide configures the profile `{default}` the `{}` bundle records",
                    selection.catalog
                ))
            })?;
            profiles.insert(default.clone(), documented_profile(&block.auth)?);
        }
        configured.default.insert(selection.name.clone(), default);
        configured.profiles.insert(selection.name.clone(), profiles);
    }
    Ok(configured)
}

// ---- shared profiles ------------------------------------------------------------------------

fn same<T: Clone + PartialEq>(a: &Option<T>, b: &Option<T>) -> Option<T> {
    match (a, b) {
        (Some(a), Some(b)) if a == b => Some(a.clone()),
        _ => None,
    }
}

/// What two profiles agree on, `identity` field by field.
fn common(a: &Auth, b: &Auth) -> Auth {
    let identity = match (&a.identity, &b.identity) {
        (Some(a), Some(b)) => {
            let identity = AuthIdentity {
                source: same(&a.source, &b.source),
                path: same(&a.path, &b.path),
                kind: same(&a.kind, &b.kind),
                subject: same(&a.subject, &b.subject),
            };
            let empty = identity.source.is_none()
                && identity.path.is_none()
                && identity.kind.is_none()
                && identity.subject.is_none();
            (!empty).then_some(identity)
        }
        _ => None,
    };
    Auth {
        library: None,
        default: None,
        scheme: same(&a.scheme, &b.scheme),
        header: same(&a.header, &b.header),
        prefix: same(&a.prefix, &b.prefix),
        label: same(&a.label, &b.label),
        account_label: same(&a.account_label, &b.account_label),
        identity,
        scopes: same(&a.scopes, &b.scopes),
        access: same(&a.access, &b.access),
        evidence_lifetime_ms: same(&a.evidence_lifetime_ms, &b.evidence_lifetime_ms),
        token_url: same(&a.token_url, &b.token_url),
        authorize_url: same(&a.authorize_url, &b.authorize_url),
    }
}

fn unless_same<T: Clone + PartialEq>(full: &Option<T>, base: &Option<T>) -> Option<T> {
    if full == base { None } else { full.clone() }
}

/// What `full` adds to `base`, so that `merged(base, over)` is `full` again.
fn over(full: &Auth, base: &Auth) -> Auth {
    let identity = match (&full.identity, &base.identity) {
        (Some(full), Some(base)) => {
            let identity = AuthIdentity {
                source: unless_same(&full.source, &base.source),
                path: unless_same(&full.path, &base.path),
                kind: unless_same(&full.kind, &base.kind),
                subject: unless_same(&full.subject, &base.subject),
            };
            let empty = identity.source.is_none()
                && identity.path.is_none()
                && identity.kind.is_none()
                && identity.subject.is_none();
            (!empty).then_some(identity)
        }
        (full, _) => full.clone(),
    };
    Auth {
        library: None,
        default: None,
        scheme: unless_same(&full.scheme, &base.scheme),
        header: unless_same(&full.header, &base.header),
        prefix: unless_same(&full.prefix, &base.prefix),
        label: unless_same(&full.label, &base.label),
        account_label: unless_same(&full.account_label, &base.account_label),
        identity,
        scopes: unless_same(&full.scopes, &base.scopes),
        access: unless_same(&full.access, &base.access),
        evidence_lifetime_ms: unless_same(&full.evidence_lifetime_ms, &base.evidence_lifetime_ms),
        token_url: unless_same(&full.token_url, &base.token_url),
        authorize_url: unless_same(&full.authorize_url, &base.authorize_url),
    }
}

/// The shared file a profile id belongs to: its prefix, when that is a plain
/// lowercase name.
fn library_of(id: &str) -> Option<&str> {
    let prefix = id.split('.').next()?;
    (prefix != id
        && !prefix.is_empty()
        && prefix
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-'))
    .then_some(prefix)
}

/// Shared file name → profile id → what every definition configuring it agrees on.
fn libraries(configured: &Configured) -> BTreeMap<String, BTreeMap<String, Auth>> {
    let mut uses: BTreeMap<&str, Vec<&Auth>> = BTreeMap::new();
    for profiles in configured.profiles.values() {
        for (id, profile) in profiles {
            uses.entry(id.as_str()).or_default().push(profile);
        }
    }
    let mut libraries: BTreeMap<String, BTreeMap<String, Auth>> = BTreeMap::new();
    for (id, profiles) in uses {
        let Some(library) = library_of(id) else {
            continue;
        };
        if profiles.len() < 2 {
            continue;
        }
        let base = profiles[1..]
            .iter()
            .fold(profiles[0].clone(), |base, profile| common(&base, profile));
        libraries
            .entry(library.to_owned())
            .or_default()
            .insert(id.to_owned(), base);
    }
    libraries
}

// ---- sources --------------------------------------------------------------------------------

/// The file under `adapters/` with this name and SHA-256; the first in path
/// order when several are.
fn locate(connectors: &Path, name: &str, digest: &str) -> Result<PathBuf, Failure> {
    fn walk(directory: &Path, name: &str, digest: &str) -> Result<Option<PathBuf>, Failure> {
        for entry in sorted_entries(directory)? {
            let path = entry.path();
            let file_name = entry.file_name();
            if path.is_dir() {
                if matches!(file_name.to_str(), Some("target" | "node_modules" | ".git")) {
                    continue;
                }
                if let Some(found) = walk(&path, name, digest)? {
                    return Ok(Some(found));
                }
            } else if file_name.to_str() == Some(name) && sha256(&read(&path)?) == digest {
                return Ok(Some(path));
            }
        }
        Ok(None)
    }
    walk(&connectors.join("adapters"), name, digest)?.ok_or_else(|| {
        Failure::NotFound(format!(
            "no file `{name}` with SHA-256 {digest} under adapters/"
        ))
    })
}

/// A vendored file: its name in `upstream/<name>/`, its bytes as written, the
/// Connectors file it came from, and that file's digest and size.
struct Vendored {
    file_name: String,
    bytes: Vec<u8>,
    from: PathBuf,
    sha256: String,
    size: usize,
    url: Option<String>,
    retrieved: Option<String>,
    license: String,
}

fn gzip(bytes: &[u8]) -> Result<Vec<u8>, Failure> {
    let mut encoder = flate2::GzBuilder::new().write(Vec::new(), flate2::Compression::best());
    encoder
        .write_all(bytes)
        .map_err(|error| Failure::Write(error.to_string()))?;
    encoder
        .finish()
        .map_err(|error| Failure::Write(error.to_string()))
}

/// The first `[Exact source](<url>)` and `retrieved on <date>` of a README.
fn provenance(readme: &str) -> (Option<String>, Option<String>) {
    let url = readme.split("[Exact source](").nth(1).and_then(|rest| {
        let end = rest.find(')')?;
        Some(rest[..end].to_owned())
    });
    let flat = readme.split_whitespace().collect::<Vec<_>>().join(" ");
    let retrieved = flat.split("retrieved on ").nth(1).and_then(|rest| {
        let date: String = rest.chars().take(10).collect();
        let shaped = date.len() == 10
            && date.bytes().enumerate().all(|(i, b)| {
                if i == 4 || i == 7 {
                    b == b'-'
                } else {
                    b.is_ascii_digit()
                }
            });
        shaped.then_some(date)
    });
    (url, retrieved)
}

/// The license file that goes with a source: `LICENSE` beside it, or the one
/// its README names as `../LICENSE`.
fn license_file(source: &Path, readme: Option<&str>) -> Option<PathBuf> {
    let directory = source.parent()?;
    let beside = directory.join("LICENSE");
    if beside.is_file() {
        return Some(beside);
    }
    let parent = directory.parent()?.join("LICENSE");
    (readme.is_some_and(|text| text.contains("`../LICENSE`")) && parent.is_file()).then_some(parent)
}

fn relative_to(path: &Path, root: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .into_owned()
}

fn thousands(value: usize) -> String {
    let digits = value.to_string();
    let mut out = String::new();
    for (i, digit) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i).is_multiple_of(3) {
            out.push(',');
        }
        out.push(digit);
    }
    out
}

fn readme(name: &str, connectors: &Path, vendored: &[Vendored], notes: Option<&Path>) -> String {
    let mut text = format!(
        "# {name} upstream sources\n\n\
         Written by `connectors-build definition import` from\n\
         [`beyond10x/connectors`](https://github.com/beyond10x/connectors). Each `.gz` file is a\n\
         gzip copy of the unmodified file; SHA-256 and size are of the uncompressed bytes.\n\n\
         | File | Taken from | URL | Retrieved | SHA-256 | Size | License |\n\
         |---|---|---|---|---|---|---|\n"
    );
    for file in vendored {
        text.push_str(&format!(
            "| `{}` | `{}` | {} | {} | `{}` | {} bytes | {} |\n",
            file.file_name,
            relative_to(&file.from, connectors),
            file.url.as_deref().unwrap_or("none"),
            file.retrieved.as_deref().unwrap_or("none"),
            file.sha256,
            thousands(file.size),
            file.license,
        ));
    }
    if let Some(notes) = notes {
        text.push_str(&format!(
            "\nProvenance, review and refresh notes: `{}` in `beyond10x/connectors`.\n",
            relative_to(notes, connectors)
        ));
    }
    text
}

// ---- the definition -------------------------------------------------------------------------

/// A shipped check as a guard `expect` value.
fn expectation(operation: &str, check: &ShippedCheck) -> Result<String, Failure> {
    match &check.expect {
        ShippedExpectation::Input(input) if !input.starts_with('=') => Ok(input.clone()),
        ShippedExpectation::Input(input) => Err(unexpressible(
            operation,
            "guard",
            format!("the input reference `{input}` begins with `=`, which marks a literal"),
        )),
        ShippedExpectation::Literal(literal) if literal == "true" || literal == "false" => {
            Err(unexpressible(
                operation,
                "guard",
                format!("the literal `{literal}` reads as a boolean"),
            ))
        }
        ShippedExpectation::Literal(literal) => Ok(format!("={literal}")),
        ShippedExpectation::Boolean(flag) => Ok(format!("={flag}")),
    }
}

fn expectations(
    operation: &str,
    checks: &[ShippedCheck],
) -> Result<Option<IndexMap<String, String>>, Failure> {
    let mut mapped = IndexMap::new();
    for check in checks {
        if mapped
            .insert(check.pointer.clone(), expectation(operation, check)?)
            .is_some()
        {
            return Err(unexpressible(
                operation,
                "guard",
                format!("two checks compare `{}`", check.pointer),
            ));
        }
    }
    Ok((!mapped.is_empty()).then_some(mapped))
}

struct Context<'a> {
    inventory: &'a Inventory,
    shipped: &'a [ShippedOperation],
}

impl Context<'_> {
    fn found(&self, operation_id: &str) -> Option<&crate::inventory::Operation> {
        self.inventory
            .operations
            .iter()
            .find(|candidate| candidate.operation_id.as_deref() == Some(operation_id))
    }

    /// The key a guard reads `operation_id` by: the first operation naming it,
    /// else the operation id.
    fn read_key(&self, operation_id: &str) -> String {
        self.shipped
            .iter()
            .find(|operation| operation.operation_id.as_deref() == Some(operation_id))
            .map_or_else(|| operation_id.to_owned(), |operation| operation.id.clone())
    }

    /// `with`: the values a read binds other than its path parameters' own names.
    fn with(
        &self,
        operation: &str,
        read: &str,
        values: &IndexMap<String, String>,
    ) -> Result<Option<IndexMap<String, String>>, Failure> {
        let found = self.found(read).ok_or_else(|| {
            unexpressible(
                operation,
                "guard",
                format!("reads `{read}`, which the bundle lacks"),
            )
        })?;
        let mut with = IndexMap::new();
        for parameter in &found.parameters {
            if parameter.location == Location::Path && !values.contains_key(&parameter.name) {
                return Err(unexpressible(
                    operation,
                    "guard",
                    format!("the read of `{read}` binds no `{}`", parameter.name),
                ));
            }
        }
        for (parameter, input) in values {
            let path = found
                .parameters
                .iter()
                .any(|p| p.location == Location::Path && &p.name == parameter);
            if !(path && parameter == input) {
                with.insert(parameter.clone(), input.clone());
            }
        }
        Ok((!with.is_empty()).then_some(with))
    }

    fn before(
        &self,
        operation: &str,
        preflight: &ShippedPreflight,
    ) -> Result<GuardBefore, Failure> {
        Ok(GuardBefore {
            read: self.read_key(&preflight.operation_id),
            with: self.with(operation, &preflight.operation_id, &preflight.values)?,
            expect: expectations(operation, &preflight.checks)?,
        })
    }

    fn guard(&self, operation: &str, guard: &ShippedGuard) -> Result<Guard, Failure> {
        let also = guard
            .further_preflights
            .iter()
            .map(|preflight| self.before(operation, preflight))
            .collect::<Result<Vec<_>, _>>()?;
        let postflight = &guard.postflight;
        let read = postflight
            .read
            .as_ref()
            .map(|read| {
                Ok::<_, Failure>(GuardRead {
                    read: self.read_key(&read.operation_id),
                    with: self.with(operation, &read.operation_id, &read.values)?,
                })
            })
            .transpose()?;
        let any = match &postflight.any_of {
            Some(checks) => Some(
                expectations(operation, checks)?
                    .ok_or_else(|| unexpressible(operation, "guard", "an empty any_of"))?,
            ),
            None => None,
        };
        let after = GuardAfter {
            expect: expectations(operation, &postflight.checks)?,
            any,
            read,
            absent: postflight.absent,
            refusals: postflight.refusals.clone(),
        };
        let empty = after.expect.is_none()
            && after.any.is_none()
            && after.read.is_none()
            && after.absent.is_none()
            && after.refusals.is_none();
        Ok(Guard {
            before: self.before(operation, &guard.preflight)?,
            also: (!also.is_empty()).then_some(also),
            after: (!empty).then_some(after),
        })
    }
}

/// A closed body in the definition notation.
fn body_notation(shipped: &ShippedOperation) -> Result<Option<IndexMap<String, String>>, Failure> {
    let id = &shipped.id;
    let refused = |detail: String| unexpressible(id, "body", detail);
    if shipped.body_keys.is_empty() {
        let any = !shipped.body_types.is_empty()
            || !shipped.body_required.is_empty()
            || !shipped.body_nonempty.is_empty()
            || !shipped.body_fixed.is_empty();
        return if any {
            Err(refused("body fields without body_keys".to_owned()))
        } else {
            Ok(None)
        };
    }
    for key in shipped
        .body_types
        .keys()
        .chain(&shipped.body_required)
        .chain(&shipped.body_nonempty)
        .chain(shipped.body_fixed.keys())
    {
        if !shipped.body_keys.contains(key) {
            return Err(refused(format!("`{key}` is not a body key")));
        }
    }
    let in_key_order = |listed: &[String]| {
        let ordered: Vec<&String> = shipped
            .body_keys
            .iter()
            .filter(|key| listed.contains(key))
            .collect();
        ordered.len() == listed.len() && ordered.iter().zip(listed).all(|(a, b)| *a == b)
    };
    if !in_key_order(&shipped.body_required) || !in_key_order(&shipped.body_nonempty) {
        return Err(refused(
            "body_required or body_nonempty is not in body_keys order".to_owned(),
        ));
    }
    let mut body = IndexMap::new();
    for key in &shipped.body_keys {
        if let Some(value) = shipped.body_fixed.get(key) {
            if shipped.body_types.contains_key(key)
                || shipped.body_required.contains(key)
                || shipped.body_nonempty.contains(key)
            {
                return Err(refused(format!(
                    "the fixed `{key}` is also typed or required"
                )));
            }
            let spec = match value {
                Value::Bool(flag) => format!("={flag}"),
                Value::Number(number) if number.is_i64() => format!("={number}"),
                Value::String(text)
                    if text != "true" && text != "false" && text.parse::<i64>().is_err() =>
                {
                    format!("={text}")
                }
                _ => {
                    return Err(refused(format!(
                        "the fixed value of `{key}` reads as another type"
                    )));
                }
            };
            body.insert(key.clone(), spec);
            continue;
        }
        let kind = shipped.body_types.get(key).map_or("any", String::as_str);
        if !matches!(kind, "string" | "integer" | "boolean" | "any") {
            return Err(refused(format!("`{key}` has the type `{kind}`")));
        }
        let mut spec = kind.to_owned();
        if shipped.body_nonempty.contains(key) {
            if kind != "string" {
                return Err(refused(format!("the non-empty `{key}` is not a string")));
            }
            spec.push('+');
        }
        if shipped.body_required.contains(key) {
            spec.push('!');
        }
        body.insert(key.clone(), spec);
    }
    Ok(Some(body))
}

fn is_page_bound(bound: &Bound) -> Option<u64> {
    (bound.minimum == Some(1) && bound.values.is_none())
        .then_some(bound.maximum)
        .flatten()
}

/// The paging styles: one per page-size parameter that every operation having
/// it (and not withholding it) bounds the same, when at least two do or the
/// feed pages by it.
fn styles(
    context: &Context<'_>,
    operations: &IndexMap<String, Operation>,
    feed_limits: &[&str],
) -> IndexMap<String, PagingStyle> {
    let query = |operation: &Operation| -> Vec<String> {
        operation
            .op
            .as_deref()
            .and_then(|op| context.found(op))
            .map(|found| {
                found
                    .parameters
                    .iter()
                    .filter(|p| p.location == Location::Query)
                    .map(|p| p.name.clone())
                    .collect()
            })
            .unwrap_or_default()
    };
    let mut candidates: Vec<String> = Vec::new();
    for operation in operations.values() {
        let query = query(operation);
        for (parameter, bound) in operation.bounds.iter().flatten() {
            if is_page_bound(bound).is_some()
                && query.contains(parameter)
                && !candidates.contains(parameter)
            {
                candidates.push(parameter.clone());
            }
        }
    }
    let mut styles = IndexMap::new();
    for parameter in candidates {
        let having: Vec<&Operation> = operations
            .values()
            .filter(|operation| {
                query(operation).contains(&parameter)
                    && !operation.withhold.iter().flatten().any(|w| *w == parameter)
            })
            .collect();
        let maxima: Vec<Option<u64>> = having
            .iter()
            .map(|operation| {
                operation
                    .bounds
                    .as_ref()
                    .and_then(|bounds| bounds.get(&parameter))
                    .and_then(is_page_bound)
            })
            .collect();
        let Some(Some(max)) = maxima.first().copied() else {
            continue;
        };
        if maxima.iter().any(|m| *m != Some(max)) {
            continue;
        }
        if having.len() < 2 && !feed_limits.contains(&parameter.as_str()) {
            continue;
        }
        styles.insert(
            parameter.clone(),
            PagingStyle {
                request: None,
                next: None,
                limit: Some(parameter.clone()),
                max: Some(max),
                total: None,
                last: None,
            },
        );
    }
    styles
}

/// Take each operation's bound that its paging style gives out of its explicit
/// bounds, naming the style where more than one matches.
fn apply_styles(
    context: &Context<'_>,
    operations: &mut IndexMap<String, Operation>,
    styles: &IndexMap<String, PagingStyle>,
) {
    for operation in operations.values_mut() {
        let query: Vec<String> = operation
            .op
            .as_deref()
            .and_then(|op| context.found(op))
            .map(|found| {
                found
                    .parameters
                    .iter()
                    .filter(|p| p.location == Location::Query)
                    .map(|p| p.name.clone())
                    .collect()
            })
            .unwrap_or_default();
        let withheld: Vec<String> = operation.withhold.clone().unwrap_or_default();
        let matching: Vec<(&String, &PagingStyle)> = styles
            .iter()
            .filter(|(_, style)| {
                style
                    .limit
                    .as_ref()
                    .is_some_and(|limit| query.contains(limit) && !withheld.contains(limit))
            })
            .collect();
        let Some((name, style)) = matching.first() else {
            continue;
        };
        if matching.len() > 1 {
            operation.paging = Some((*name).clone());
        }
        let (Some(limit), Some(max)) = (&style.limit, style.max) else {
            continue;
        };
        if let Some(bounds) = &mut operation.bounds {
            if bounds
                .get(limit)
                .is_some_and(|bound| is_page_bound(bound) == Some(max))
            {
                bounds.shift_remove(limit);
            }
            if bounds.is_empty() {
                operation.bounds = None;
            }
        }
    }
}

/// Move into `defaults` each carried field every operation it applies to
/// carries alike.
fn defaults(operations: &mut IndexMap<String, Operation>) -> Option<Defaults> {
    let mut defaults = Defaults::default();
    fn shared<T: Clone + PartialEq>(
        operations: &mut IndexMap<String, Operation>,
        applies: impl Fn(&Operation) -> bool,
        field: fn(&mut Operation) -> &mut Option<T>,
    ) -> Option<T> {
        let mut values = Vec::new();
        for operation in operations.values_mut() {
            if applies(operation) {
                values.push(field(operation).clone());
            }
        }
        if values.len() < 2 {
            return None;
        }
        let first = values[0].clone()?;
        if values.iter().any(|value| value.as_ref() != Some(&first)) {
            return None;
        }
        for operation in operations.values_mut() {
            if applies(operation) {
                *field(operation) = None;
            }
        }
        Some(first)
    }
    let all = |_: &Operation| true;
    defaults.required = shared(operations, all, |o| &mut o.required);
    defaults.withhold = shared(operations, all, |o| &mut o.withhold);
    defaults.credential = shared(operations, all, |o| &mut o.credential);
    defaults.response = shared(operations, all, |o| &mut o.response);
    defaults.binary = shared(operations, all, |o| &mut o.binary);
    defaults.download = shared(operations, all, |o| &mut o.download);
    defaults.rate_limit_reasons = shared(operations, all, |o| &mut o.rate_limit_reasons);
    let response = defaults.response;
    defaults.envelope = shared(
        operations,
        |o| {
            o.effect.unwrap_or(Effect::Read) == Effect::Read
                && matches!(o.response.or(response), None | Some(ResponseKind::Json))
                && o.download.is_none()
        },
        |o| &mut o.envelope,
    );
    let writes = |o: &Operation| o.effect == Some(Effect::Write);
    defaults.refusal_texts = shared(operations, writes, |o| &mut o.refusal_texts);
    defaults.custody = shared(operations, writes, |o| &mut o.custody);
    (defaults != Defaults::default()).then_some(defaults)
}

/// The datasource a shipped feed lowers from.
fn datasource(context: &Context<'_>, feed: &ShippedFeed) -> Result<(String, Datasource), Failure> {
    let key = |operation_id: &str, role: &str| {
        context
            .shipped
            .iter()
            .find(|operation| operation.operation_id.as_deref() == Some(operation_id))
            .map(|operation| operation.id.clone())
            .ok_or_else(|| {
                unexpressible(
                    "feed",
                    role,
                    format!("`{operation_id}` is not a selected operation"),
                )
            })
    };
    if feed.items.position.kind != "time" {
        return Err(unexpressible(
            "feed",
            "items.position",
            format!("the position kind `{}`", feed.items.position.kind),
        ));
    }
    let containers = &feed.containers;
    let items = &feed.items;
    let list = key(&items.operation_id, "items.list")?;
    let name = list
        .rsplit_once('.')
        .map_or(list.as_str(), |(head, _)| head)
        .to_owned();
    let non_empty = |text: &String| (!text.is_empty()).then(|| text.clone());
    Ok((
        name,
        Datasource {
            kind: DatasourceKind::Feed,
            profile: feed.profile.clone(),
            capabilities: feed.capabilities.clone(),
            containers: FeedContainers {
                list: key(&containers.operation_id, "containers.list")?,
                lookup: key(&containers.lookup.operation_id, "containers.lookup")?,
                query: containers.query.clone(),
                cursor: containers.cursor.clone(),
                records: non_empty(&containers.records),
                id: containers.id.clone(),
                name: containers.name.clone(),
                kind_word: containers.kind_word.clone(),
            },
            items: FeedItems {
                list,
                query: items.query.clone(),
                position: FeedPosition {
                    time: items.position.value.parameter.clone(),
                },
                records: non_empty(&items.records),
                id: items.id.clone(),
                revision: items.revision.clone(),
                created_at: items.created_at.clone(),
                updated_at: items.updated_at.clone(),
                author: items.author.clone(),
                body: items.body.clone(),
            },
            filters: None,
        },
    ))
}

/// The first place two selections differ, as the field it is in.
fn first_difference(written: &Value, shipped: &Value) -> Failure {
    let empty = Vec::new();
    let ours = written["operations"].as_array().unwrap_or(&empty);
    let theirs = shipped["operations"].as_array().unwrap_or(&empty);
    for (index, theirs) in theirs.iter().enumerate() {
        let id = theirs["id"].as_str().unwrap_or_default().to_owned();
        let Some(ours) = ours.get(index) else {
            return unexpressible(&id, "operations", "the definition lowers fewer operations");
        };
        let fields: BTreeSet<&String> = theirs
            .as_object()
            .into_iter()
            .chain(ours.as_object())
            .flat_map(|object| object.keys())
            .collect();
        for field in fields {
            if ours.get(field) != theirs.get(field) {
                return unexpressible(
                    &id,
                    field,
                    format!(
                        "lowers to {}, the shipped file has {}",
                        ours.get(field)
                            .map_or("nothing".to_owned(), Value::to_string),
                        theirs
                            .get(field)
                            .map_or("nothing".to_owned(), Value::to_string)
                    ),
                );
            }
        }
    }
    for field in ["format", "provider", "feed", "operations"] {
        if written.get(field) != shipped.get(field) {
            return unexpressible("selection", field, "lowers to another value");
        }
    }
    unexpressible("selection", "", "lowers to another value")
}

// ---- writing YAML ---------------------------------------------------------------------------

/// How wide a line may grow before a mapping or a sequence is written in block
/// style rather than flow style.
const WIDTH: usize = 100;

enum Node {
    Scalar(String),
    /// A string, as written inside a flow collection and as a block value.
    Text {
        flow: String,
        block: String,
    },
    Map(Vec<(String, Node)>),
    Seq(Vec<Node>),
}

/// A string as a YAML scalar: plain when it reads back as that string and
/// carries nothing a flow collection or a reader could take for syntax,
/// otherwise double-quoted (JSON's escapes are YAML's).
fn scalar(text: &str) -> String {
    let plain = !text.is_empty()
        && text.trim() == text
        && !text.starts_with(['-', '?', '!'])
        && !text.chars().any(|c| {
            c.is_control()
                || matches!(
                    c,
                    '{' | '}'
                        | '['
                        | ']'
                        | ','
                        | '"'
                        | '\''
                        | '#'
                        | '`'
                        | '|'
                        | '>'
                        | '&'
                        | '*'
                        | '%'
                        | '@'
                        | ':'
                        | '\\'
                )
        })
        && serde_yaml_ng::from_str::<serde_yaml_ng::Value>(text).ok()
            == Some(serde_yaml_ng::Value::String(text.to_owned()));
    if plain {
        text.to_owned()
    } else {
        serde_json::to_string(text).unwrap_or_default()
    }
}

fn node(value: &serde_yaml_ng::Value) -> Result<Node, Failure> {
    Ok(match value {
        serde_yaml_ng::Value::Null => Node::Scalar("null".to_owned()),
        serde_yaml_ng::Value::Bool(flag) => Node::Scalar(flag.to_string()),
        serde_yaml_ng::Value::Number(number) => Node::Scalar(number.to_string()),
        serde_yaml_ng::Value::String(text) => Node::Text {
            flow: scalar(text),
            block: block_scalar(text),
        },
        serde_yaml_ng::Value::Sequence(items) => {
            Node::Seq(items.iter().map(node).collect::<Result<_, _>>()?)
        }
        serde_yaml_ng::Value::Mapping(mapping) => Node::Map(
            mapping
                .iter()
                .map(|(key, value)| {
                    let key = key.as_str().ok_or_else(|| {
                        Failure::Write("a mapping key is not a string".to_owned())
                    })?;
                    Ok((scalar(key), node(value)?))
                })
                .collect::<Result<_, Failure>>()?,
        ),
        serde_yaml_ng::Value::Tagged(_) => {
            return Err(Failure::Write("a tagged YAML value".to_owned()));
        }
    })
}

fn flow(node: &Node) -> String {
    match node {
        Node::Scalar(text) => text.clone(),
        Node::Text { flow, .. } => flow.clone(),
        Node::Map(entries) => format!(
            "{{{}}}",
            entries
                .iter()
                .map(|(key, value)| format!("{key}: {}", flow(value)))
                .collect::<Vec<_>>()
                .join(", ")
        ),
        Node::Seq(items) => format!(
            "[{}]",
            items.iter().map(flow).collect::<Vec<_>>().join(", ")
        ),
    }
}

fn entries(entries: &[(String, Node)], indent: usize, out: &mut Vec<String>) {
    for (key, value) in entries {
        let prefix = format!("{}{key}:", " ".repeat(indent));
        match value {
            Node::Scalar(text) => out.push(format!("{prefix} {text}")),
            Node::Text { block, .. } => out.push(format!("{prefix} {block}")),
            Node::Map(children) => {
                let inline = flow(value);
                if children.is_empty() || prefix.len() + 1 + inline.len() <= WIDTH {
                    out.push(format!("{prefix} {inline}"));
                } else {
                    out.push(prefix);
                    self::entries(children, indent + 2, out);
                }
            }
            Node::Seq(items) => {
                let inline = flow(value);
                if items.is_empty() || prefix.len() + 1 + inline.len() <= WIDTH {
                    out.push(format!("{prefix} {inline}"));
                } else {
                    out.push(prefix);
                    for item in items {
                        sequence_item(item, indent + 2, out);
                    }
                }
            }
        }
    }
}

fn sequence_item(item: &Node, indent: usize, out: &mut Vec<String>) {
    if let Node::Text { block, .. } = item {
        out.push(format!("{}- {block}", " ".repeat(indent)));
        return;
    }
    let inline = flow(item);
    if matches!(item, Node::Scalar(_)) || indent + 2 + inline.len() <= WIDTH {
        out.push(format!("{}- {inline}", " ".repeat(indent)));
        return;
    }
    let mut lines = Vec::new();
    match item {
        Node::Map(children) => entries(children, indent + 2, &mut lines),
        Node::Seq(items) => {
            for nested in items {
                sequence_item(nested, indent + 2, &mut lines);
            }
        }
        Node::Scalar(_) | Node::Text { .. } => {}
    }
    if let Some(first) = lines.first_mut() {
        *first = format!("{}- {}", " ".repeat(indent), &first[indent + 2..]);
    }
    out.extend(lines);
}

/// A value as YAML in this repository's style: block mappings at the top,
/// flow style wherever it fits in [`WIDTH`].
fn yaml(value: &impl serde::Serialize) -> Result<String, Failure> {
    let value =
        serde_yaml_ng::to_value(value).map_err(|error| Failure::Write(error.to_string()))?;
    let Node::Map(top) = node(&value)? else {
        return Err(Failure::Write("the document is not a mapping".to_owned()));
    };
    let mut lines = Vec::new();
    entries(&top, 0, &mut lines);
    let mut text = lines.join("\n");
    text.push('\n');
    Ok(text)
}

// ---- the import -----------------------------------------------------------------------------

fn write_file(path: &Path, bytes: &[u8], written: &mut Vec<PathBuf>) -> Result<(), Failure> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|error| Failure::Write(format!("{}: {error}", parent.display())))?;
    }
    std::fs::write(path, bytes)
        .map_err(|error| Failure::Write(format!("{}: {error}", path.display())))?;
    written.push(path.to_owned());
    Ok(())
}

/// Import one shipped selection. See the module documentation for what is
/// taken from where.
pub fn import(request: &Request<'_>) -> Result<Imported, Failure> {
    let connectors = request.connectors;
    let catalog = request.catalog;
    let default_selection = connectors
        .join("adapters/catalog/providers")
        .join(catalog)
        .join("operations.json");
    let selection_path = request.selection.unwrap_or(&default_selection);
    let selection_file = selection_path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| {
            Failure::Unreadable(format!("{}: no file name", selection_path.display()))
        })?;
    let name = match request.name {
        Some(name) => name.to_owned(),
        None => definition_name(catalog, selection_file).ok_or_else(|| {
            Failure::Malformed(format!(
                "`{selection_file}` is neither `operations.json` nor `<name>-operations.json`; give the name"
            ))
        })?,
    };
    if !connectors_core::valid_id(&name) || !connectors_core::valid_id(catalog) {
        return Err(Failure::Malformed(format!(
            "`{name}` or `{catalog}` is not an id"
        )));
    }

    let selection_bytes = read(selection_path)?;
    let shipped: ShippedFile = serde_json::from_slice(&selection_bytes)
        .map_err(|error| Failure::Malformed(format!("{}: {error}", selection_path.display())))?;
    if shipped.format != definition::OPERATIONS_FORMAT || shipped.provider != catalog {
        return Err(Failure::Malformed(format!(
            "{} is not a {} selection of `{catalog}`",
            selection_path.display(),
            definition::OPERATIONS_FORMAT
        )));
    }
    let bundle = crate::bundle::load(
        &connectors.join("adapters/catalog/generated/bundles"),
        catalog,
    )
    .map_err(|error| Failure::Unreadable(format!("bundle `{catalog}`: {error}")))?;
    let record = &bundle.source;

    // The source, as `connectors-build catalog` read it.
    let (kind_word, original_name, original_digest) = match &record.derivation {
        None => (
            "openapi",
            record.file_name.clone(),
            record.source_sha256.clone(),
        ),
        Some(derivation) => (
            match derivation.format.as_str() {
                crate::swagger::FORMAT => "swagger",
                crate::discovery::FORMAT => "discovery",
                other => {
                    return Err(Failure::Malformed(format!(
                        "the `{catalog}` bundle is derived from `{other}`"
                    )));
                }
            },
            derivation.from_file.clone(),
            derivation.from_sha256.clone(),
        ),
    };
    let original_path = locate(connectors, &original_name, &original_digest)?;
    let original = read(&original_path)?;
    let openapi = match &record.derivation {
        None => original.clone(),
        Some(_) => crate::derived::project(&original)
            .map_err(|refusal| Failure::Malformed(format!("{original_name}: {refusal}")))?
            .openapi()
            .to_vec(),
    };
    if sha256(&openapi) != record.source_sha256 {
        return Err(Failure::Malformed(format!(
            "the projection of `{original_name}` is not the source the `{catalog}` bundle records"
        )));
    }
    let document = crate::parse_document(&openapi)
        .map_err(|refusal| Failure::Malformed(refusal.reason().to_owned()))?;
    let amendments = record
        .amendments
        .as_ref()
        .map(|amendments| locate(connectors, &amendments.file_name, &amendments.sha256))
        .transpose()?;

    // The configuration the guides document.
    let configured = configured(connectors)?;
    let libraries = libraries(&configured);
    let profiles = configured.profiles.get(&name).ok_or_else(|| {
        Failure::NotFound(format!(
            "no shipped selection imports as `{name}`; the guides are read per shipped selection"
        ))
    })?;
    let base_url = configured.base.get(&name).cloned().ok_or_else(|| {
        Failure::NotFound(format!("no guide documents a configuration of `{name}`"))
    })?;
    let default_profile = &configured.default[&name];
    let mut auth = IndexMap::new();
    let mut used_libraries = BTreeSet::new();
    for (id, profile) in profiles {
        let shared = library_of(id).and_then(|library| {
            libraries
                .get(library)
                .and_then(|profiles| profiles.get(id))
                .map(|base| (library, base))
        });
        let mut entry = match shared {
            Some((library, base)) => {
                used_libraries.insert(library.to_owned());
                let mut entry = over(profile, base);
                entry.library = Some(library.to_owned());
                if definition::merged(base, &entry) != *profile {
                    return Err(Failure::Malformed(format!(
                        "profile `{id}` does not split into `auth/{library}.yaml` and an extension"
                    )));
                }
                entry
            }
            None => profile.clone(),
        };
        if profiles.len() > 1 && id == default_profile {
            entry.default = Some(true);
        }
        auth.insert(id.clone(), entry);
    }
    let mut config = IndexMap::new();
    let mut urls = vec![base_url.clone()];
    for profile in profiles.values() {
        urls.extend(profile.token_url.clone());
        urls.extend(profile.authorize_url.clone());
    }
    for url in &urls {
        let mut rest = url.as_str();
        while let Some(open) = rest.find('{') {
            let Some(close) = rest[open..].find('}') else {
                break;
            };
            config
                .entry(rest[open + 1..open + close].to_owned())
                .or_insert(ConfigField {
                    value_type: ConfigType::String,
                    default: None,
                    values: None,
                });
            rest = &rest[open + close + 1..];
        }
    }
    if let Some((variable, value)) = configured.defaults.get(&name)
        && let Some(field) = config.get_mut(variable)
    {
        field.default = Some(value.clone());
    }

    // The operations.
    let context = Context {
        inventory: &bundle.inventory,
        shipped: &shipped.operations,
    };
    let mut operations = IndexMap::new();
    for operation in &shipped.operations {
        let id = &operation.id;
        let found = match &operation.operation_id {
            Some(op) => Some(context.found(op).ok_or_else(|| {
                unexpressible(id, "operation_id", format!("`{op}` is not in the bundle"))
            })?),
            None => None,
        };
        let inferred = match found {
            Some(found) if found.method != "get" => None,
            _ => Some(Effect::Read),
        };
        let summary = operation
            .operation_id
            .as_deref()
            .and_then(|op| definition::summary(&document, op));
        if operation.description.is_none() && summary.is_some() {
            return Err(unexpressible(
                id,
                "description",
                "absent, where the lowerer gives the source's summary",
            ));
        }
        let guard = operation
            .guard
            .as_ref()
            .map(|guard| context.guard(id, guard))
            .transpose()?;
        let unless_empty = |list: &Option<Vec<String>>| list.clone().filter(|l| !l.is_empty());
        let entry = Operation {
            op: operation.operation_id.clone(),
            effect: (inferred != Some(operation.effect)).then_some(operation.effect),
            paging: None,
            required: unless_empty(&operation.required),
            withhold: unless_empty(&operation.withhold),
            credential: unless_empty(&operation.credential),
            bounds: operation.bounds.clone().filter(|bounds| !bounds.is_empty()),
            response: operation.response,
            binary: operation.binary.clone(),
            download: operation.download.clone(),
            envelope: operation.envelope.clone(),
            rate_limit_reasons: unless_empty(&operation.rate_limit_reasons),
            refusal_texts: operation
                .refusal_texts
                .clone()
                .filter(|texts| !texts.is_empty()),
            custody: operation.custody.clone(),
            body: body_notation(operation)?,
            guard,
            description: (operation.description != summary)
                .then(|| operation.description.clone())
                .flatten(),
        };
        if operations.insert(id.clone(), entry).is_some() {
            return Err(unexpressible(id, "id", "selected twice"));
        }
    }
    let datasources = shipped
        .feed
        .as_ref()
        .map(|feed| datasource(&context, feed))
        .transpose()?;
    let feed_limits: Vec<&str> = shipped
        .feed
        .iter()
        .flat_map(|feed| [feed.containers.limit.as_str(), feed.items.limit.as_str()])
        .collect();
    let paging = styles(&context, &operations, &feed_limits);
    apply_styles(&context, &mut operations, &paging);
    let defaults = defaults(&mut operations);

    let vendored_dir = format!("upstream/{name}");
    let source_file = format!("{original_name}.gz");
    let source_path = format!("{vendored_dir}/{source_file}");
    let definition = ProviderDefinition {
        schema: SchemaId::V1,
        id: configured.id.get(&name).cloned(),
        catalog: (catalog != name).then(|| catalog.to_owned()),
        source: Source {
            openapi: (kind_word == "openapi").then(|| source_path.clone()),
            swagger: (kind_word == "swagger").then(|| source_path.clone()),
            discovery: (kind_word == "discovery").then(|| source_path.clone()),
            sha256: original_digest.clone(),
            projection: record.derivation.as_ref().map(|_| record.file_name.clone()),
            amendments: record
                .amendments
                .as_ref()
                .map(|amendments| format!("{vendored_dir}/{}", amendments.file_name)),
        },
        base_url,
        config: (!config.is_empty()).then_some(config),
        auth,
        paging: (!paging.is_empty()).then_some(paging),
        defaults,
        operations,
        datasources: datasources.map(|(name, datasource)| {
            let mut map = IndexMap::new();
            map.insert(name, datasource);
            map
        }),
        references: None,
        views: None,
    };

    // The definition must lower back to the shipped selection.
    let lowered = definition::select(&definition, catalog, &bundle.inventory, &document)
        .map_err(|refusal| Failure::Lowering(refusal.to_string()))?;
    let lowered =
        serde_json::to_value(&lowered).map_err(|error| Failure::Write(error.to_string()))?;
    let expected: Value = serde_json::from_slice(&selection_bytes)
        .map_err(|error| Failure::Malformed(error.to_string()))?;
    if lowered != expected {
        return Err(first_difference(&lowered, &expected));
    }
    let text = yaml(&definition)?;
    if definition::parse(text.as_bytes()).map_err(|refusal| Failure::Write(refusal.to_string()))?
        != definition
    {
        return Err(Failure::Write(format!(
            "connectors/{name}.yaml does not read back as the definition"
        )));
    }

    // The vendored files and their README.
    let notes_path = original_path
        .parent()
        .map(|directory| directory.join("README.md"))
        .filter(|path| path.is_file());
    let notes = notes_path
        .as_deref()
        .map(read)
        .transpose()?
        .map(|bytes| String::from_utf8_lossy(&bytes).into_owned());
    let (url, retrieved) = notes.as_deref().map(provenance).unwrap_or_default();
    let license = license_file(&original_path, notes.as_deref());
    let declared = match &record.derivation {
        None => record.license.clone(),
        Some(_) => None,
    };
    let mut vendored = vec![Vendored {
        file_name: source_file,
        bytes: gzip(&original)?,
        from: original_path.clone(),
        sha256: original_digest.clone(),
        size: original.len(),
        url,
        retrieved,
        license: match (&declared, license.is_some()) {
            (Some(declared), true) => format!("{declared} (`info.license`); text in `LICENSE`"),
            (Some(declared), false) => format!("{declared} (`info.license`)"),
            (None, true) => "text in `LICENSE`".to_owned(),
            (None, false) => "none declared".to_owned(),
        },
    }];
    if let Some(path) = &amendments {
        let bytes = read(path)?;
        vendored.push(Vendored {
            file_name: path
                .file_name()
                .map(|name| name.to_string_lossy().into_owned())
                .unwrap_or_default(),
            sha256: sha256(&bytes),
            size: bytes.len(),
            bytes,
            from: path.clone(),
            url: None,
            retrieved: None,
            license: "cited corrections written in `beyond10x/connectors`".to_owned(),
        });
    }
    if let Some(path) = &license {
        let bytes = read(path)?;
        vendored.push(Vendored {
            file_name: "LICENSE".to_owned(),
            sha256: sha256(&bytes),
            size: bytes.len(),
            bytes,
            from: path.clone(),
            url: None,
            retrieved: None,
            license: "the license text".to_owned(),
        });
    }

    // Write.
    let root = request.definitions;
    let mut written = Vec::new();
    let definition_path = root.join("connectors").join(format!("{name}.yaml"));
    write_file(&definition_path, text.as_bytes(), &mut written)?;
    for library in &used_libraries {
        let profiles: IndexMap<String, Auth> = libraries[library]
            .iter()
            .map(|(id, profile)| (id.clone(), profile.clone()))
            .collect();
        let text = yaml(&AuthLibrary {
            schema: AuthLibraryId::V1,
            profiles,
        })?;
        write_file(
            &root.join("auth").join(format!("{library}.yaml")),
            text.as_bytes(),
            &mut written,
        )?;
    }
    for file in &vendored {
        write_file(
            &root.join(&vendored_dir).join(&file.file_name),
            &file.bytes,
            &mut written,
        )?;
    }
    write_file(
        &root.join(&vendored_dir).join("README.md"),
        readme(&name, connectors, &vendored, notes_path.as_deref()).as_bytes(),
        &mut written,
    )?;
    Ok(Imported {
        name,
        definition: definition_path,
        written,
    })
}

/// A string as a block value: plain wherever YAML reads it back as that string
/// (a URL, a sentence), otherwise as [`scalar`] writes it.
fn block_scalar(text: &str) -> String {
    let plain = !text.is_empty()
        && text.trim() == text
        && !text.starts_with([
            '-', '?', ':', ',', '[', ']', '{', '}', '#', '&', '*', '!', '|', '>', '\'', '"', '%',
            '@', '`',
        ])
        && !text.chars().any(char::is_control)
        && !text.contains(": ")
        && !text.contains(" #")
        && !text.ends_with(':')
        && serde_yaml_ng::from_str::<serde_yaml_ng::Value>(text).ok()
            == Some(serde_yaml_ng::Value::String(text.to_owned()));
    if plain { text.to_owned() } else { scalar(text) }
}
