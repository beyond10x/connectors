//! The generic HTTP provider realization: one verified bundle, a declarative
//! selection of its operations, and one engine that binds, sends and classifies.
//! No operation here has its own Rust. Adding a supported endpoint changes the
//! selection and the pinned source; it changes nothing in this crate.
//!
//! Authority stays where it is: the host admits the connection, spends the
//! approval, records the attempt and supplies the one-use write capability. The
//! engine turns a declared operation plus validated input into exactly one
//! request and turns the response into an observation.
//!
//! A provider may also declare a `datasource.feed/v1alpha1` binding as data; [`feed`] realizes
//! it over the same bundle, and the engine answers `feed.containers` and `feed.items` from it.
pub mod feed;

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
use std::collections::{BTreeMap, BTreeSet};

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

/// A read a guard issues after the write: a GET of the bundle, its parameter
/// keys bound to input references as a preflight's `values` are.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Read {
    pub operation_id: String,
    pub values: BTreeMap<String, String>,
}

/// Comparisons after dispatch: against the write's own response body, or,
/// when `read` is declared, against the answer of that read, issued once after
/// a 2xx, for a write that answers without a body.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Postflight {
    pub checks: Vec<Check>,
    /// Omitted when absent, so a guard without it keeps its bytes. An explicit
    /// `null` is refused, as the model refuses it.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "present_read"
    )]
    pub read: Option<Read>,
    /// One check that accepts one of several observations: it holds when at
    /// least one of these comparisons holds, beside every check in `checks`,
    /// for a write whose success has more than one shape. At least two when
    /// declared; omitted when empty, so a guard without it keeps its bytes.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub any_of: Vec<Check>,
    /// Only `true`, beside `read`: the read after the write must answer 404,
    /// proving a delete, and nothing else is compared. Omitted when absent, so
    /// a guard without it keeps its bytes; an explicit `null` is refused, as
    /// the model refuses it, and `false` is refused when the selection loads.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "present_flag"
    )]
    pub absent: Option<bool>,
}

/// How a 2xx body is read. The bundle's declared media types decide by default;
/// `text` is the reviewed exception for a source that declares JSON where the
/// provider answers with plain text. `binary` reads the body as bytes and
/// answers with their media type, length, SHA-256 and base64, bounded by the
/// selection's [`Binary`] (`connectors_catalog.binary.ResponseKind`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResponseKind {
    Json,
    Text,
    Binary,
}

/// The largest `max_bytes` a binary read may declare: its base64 body and the
/// result envelope stay under the served result limit (4 MiB less 1 KiB) and
/// the provider transport limit (4 MiB).
pub const BINARY_LIMIT: u64 = 2 * 1024 * 1024;
/// The most redirects one binary read follows.
pub const REDIRECT_LIMIT: usize = 3;
/// The most hosts one binary read may reach besides the API base.
const BINARY_HOSTS: usize = 4;

/// The bound and the reach of a binary read (`connectors_catalog.binary.Binary`).
/// A body longer than `max_bytes` is refused as `capacity`, never truncated.
/// `hosts` are the origins, besides the API base, that the read may reach: a
/// `download` URL's origin and a redirect's target. A redirect is followed only
/// to one of them that the connection also admits ([`Hosts`]); with none, a
/// redirect is refused. Omitted when empty, as the model omits it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Binary {
    pub max_bytes: u64,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub hosts: Vec<String>,
}

/// A read of a URL the provider handed out, such as Slack's `url_private`
/// (`connectors_catalog.binary.Download`): the caller gives it as `url`, and it
/// must be an `https` URL on an origin in the selection's `binary.hosts` whose
/// path starts with `path_prefix`. Such a selection names no bundle operation.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Download {
    pub path_prefix: String,
}

/// The ports for the origins a connection admits besides its API base, each
/// carrying the connection's credential or none, as the connection declares.
/// Trusted composition builds them; the engine only looks one up by origin.
pub trait Hosts: Send + Sync {
    fn port(&self, origin: &str) -> Option<&dyn AuthenticatedHttp>;
}

/// A connection that admits no host besides its API base.
pub struct NoHosts;
impl Hosts for NoHosts {
    fn port(&self, _origin: &str) -> Option<&dyn AuthenticatedHttp> {
        None
    }
}

impl Hosts for BTreeMap<String, std::sync::Arc<dyn AuthenticatedHttp>> {
    fn port(&self, origin: &str) -> Option<&dyn AuthenticatedHttp> {
        self.get(origin).map(|port| port.as_ref())
    }
}

/// The canonical origin of an `https` origin or URL: `https://<host>` or
/// `https://<host>:<port>`, the host in lower case, the default port dropped.
/// `None` for anything else: another scheme, user information, an empty or
/// bracketed host, a host with characters outside letters, digits, `-` and
/// `.`, or a port that is not a decimal number in range.
pub fn origin(text: &str) -> Option<String> {
    if !text.get(..8)?.eq_ignore_ascii_case("https://") {
        return None;
    }
    let rest = &text[8..];
    let end = rest.find(['/', '?', '#']).unwrap_or(rest.len());
    let authority = &rest[..end];
    let (host, port) = match authority.rsplit_once(':') {
        Some((host, port)) => (host, Some(port)),
        None => (authority, None),
    };
    if host.is_empty()
        || !host
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'.')
    {
        return None;
    }
    let host = host.to_ascii_lowercase();
    match port {
        None => Some(format!("https://{host}")),
        Some(port) => {
            if port.is_empty() || !port.bytes().all(|b| b.is_ascii_digit()) {
                return None;
            }
            match port.parse::<u16>().ok()? {
                0 => None,
                443 => Some(format!("https://{host}")),
                number => Some(format!("https://{host}:{number}")),
            }
        }
    }
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
    /// Up to three more reads before the write, run in order after
    /// `preflight`, each with its own checks, for preconditions that live in
    /// more than one answer. Omitted when empty, so a guard without them keeps
    /// its bytes.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub further_preflights: Vec<Preflight>,
    pub postflight: Postflight,
}

impl Guard {
    /// Every read before the write, in the order it is issued.
    fn preflights(&self) -> impl Iterator<Item = &Preflight> {
        std::iter::once(&self.preflight).chain(&self.further_preflights)
    }

    /// Every check, before the write and after it.
    fn checks(&self) -> impl Iterator<Item = &Check> {
        self.preflights()
            .flat_map(|preflight| &preflight.checks)
            .chain(&self.postflight.checks)
            .chain(&self.postflight.any_of)
    }

    /// Every parameter a read binds, with the input reference it binds.
    fn bound(&self) -> impl Iterator<Item = (&String, &String)> {
        self.preflights()
            .flat_map(|preflight| &preflight.values)
            .chain(self.postflight.read.iter().flat_map(|read| &read.values))
    }

    /// Every input reference the guard reads: each read's values, then each
    /// check's expected input.
    fn references(&self) -> impl Iterator<Item = &String> {
        self.bound()
            .map(|(_, reference)| reference)
            .chain(self.checks().filter_map(|check| match &check.expect {
                Expectation::Input(path) => Some(path),
                Expectation::Literal(_) => None,
            }))
    }
}

/// A narrower bound for one query parameter than the pinned source declares,
/// in exactly one of two forms: a range, `maximum` and an optional `minimum`,
/// within which the value must be a decimal integer, such as a provider's
/// page-size cap; or `values`, the allowed strings, one of which the value's
/// sent text must be, such as GitLab's project search held to `scope` `blobs`.
/// A bound naming neither `maximum` nor `values` is not read; the engine
/// refuses one with both forms, or neither, when it loads.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, try_from = "WrittenBound")]
pub struct Bound {
    /// Omitted when absent, so a maximum-only bound serialises unchanged.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub minimum: Option<u64>,
    /// Omitted when absent, so a `values` bound carries no range.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub maximum: Option<u64>,
    /// Omitted when empty, so a range bound serialises as it did before
    /// `values` existed.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub values: Vec<String>,
}

/// A bound as written, before it is read: `values` absent is told apart from
/// `values` empty, so `{}` and a lone `minimum` are refused as they were before
/// `values` existed, while an empty set reaches the engine's load check.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct WrittenBound {
    #[serde(default)]
    minimum: Option<u64>,
    #[serde(default)]
    maximum: Option<u64>,
    #[serde(default)]
    values: Option<Vec<String>>,
}

impl TryFrom<WrittenBound> for Bound {
    type Error = String;

    fn try_from(written: WrittenBound) -> std::result::Result<Self, String> {
        if written.maximum.is_none() && written.values.is_none() {
            return Err("a bound names a `maximum` or `values`".to_owned());
        }
        Ok(Bound {
            minimum: written.minimum,
            maximum: written.maximum,
            values: written.values.unwrap_or_default(),
        })
    }
}

/// One operation exposed from the bundle under a local id.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Selection {
    pub id: String,
    /// The bundle operation this selection exposes. Empty, and omitted, only
    /// for a `download`, which names none; any other selection without one is
    /// refused when it loads.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub operation_id: String,
    pub effect: Effect,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub guard: Option<Guard>,
    #[serde(default)]
    pub response: Option<ResponseKind>,
    /// Declared exactly when `response` is `binary`. Omitted when absent, so a
    /// selection without it keeps its bytes.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub binary: Option<Binary>,
    /// A read of a provider-issued URL instead of a bundle operation. Omitted
    /// when absent.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub download: Option<Download>,
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
    /// Parameters through which the pinned document passes the credential
    /// that the connection already sends in its authentication header, such
    /// as Slack's `token`. Each must be a parameter of the operation, in the
    /// query or a header, that is not bounded, not listed in `required` or
    /// `withhold`, that no guard reads as an input, that `body_keys` does not
    /// admit and that no guard's preflight sends as a probe parameter, the
    /// last two compared without regard to ASCII case. It is left out of the
    /// declaration whether or not the document requires it, before the
    /// required-header check, and out of a guard's probe operation, so an
    /// input carrying it is refused before any request, a write's body
    /// carrying it as a top-level key under any ASCII case is refused before
    /// any request, and the credential travels in the header alone. Omitted
    /// when empty.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub credential: Vec<String>,
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
    /// The JSON type of closed body keys, keyed by a name `body_keys` admits:
    /// `string`, `integer` or `boolean`, as the provider's request body schema
    /// types it, which the bundle does not record. A typed key present in a
    /// body must be a JSON value of exactly that type (no string spelling of
    /// a boolean or an integer, and no `null`), or the write is refused before
    /// any request; the declaration types it the same, except that an
    /// `integer` key admits only an integer literal in the i64 or u64 range,
    /// where JSON Schema also counts `3.0` or `1e2`. Omitted when empty.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub body_types: BTreeMap<String, ValueType>,
    /// Closed body keys the provider's request body schema requires, each
    /// admitted by `body_keys`: a body without one is refused before any
    /// request, and the declaration requires it. Keys a guard reads are
    /// required already. Omitted when empty.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub body_required: Vec<String>,
    /// Closed body keys fixed to one value each: a string, an integer or a
    /// boolean, of the key's `body_types` type when it has one. The engine
    /// sends exactly that value under that key on every write, and a caller's
    /// body carrying the key at all is refused before any request; the key is
    /// not declared in the input schema. When every key `body_keys` admits is
    /// fixed, the caller's `body` may be omitted. A fixed key must be admitted
    /// by `body_keys`, and no guard may read it or `body_required` name it.
    /// Omitted when empty.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub body_fixed: BTreeMap<String, Value>,
}

impl Selection {
    /// Whether the caller's `body` may be omitted: every key the closed body
    /// admits is fixed, so a caller has nothing to send in it.
    fn body_fixed_whole(&self) -> bool {
        !self.body_fixed.is_empty()
            && self
                .body_keys
                .iter()
                .all(|key| self.body_fixed.contains_key(key))
    }
}

/// The declared schema of a body key typed by `body_types`: exactly that JSON
/// type, since the body is sent as given.
fn body_type_schema(value_type: ValueType) -> Value {
    match value_type {
        ValueType::String => json!({"type": "string"}),
        ValueType::Integer => json!({"type": "integer"}),
        ValueType::Boolean => json!({"type": "boolean"}),
    }
}

/// Whether a body value is a JSON value of exactly the declared type.
fn fits_body_type(value_type: ValueType, value: &Value) -> bool {
    match value_type {
        ValueType::String => value.is_string(),
        ValueType::Integer => value.is_i64() || value.is_u64(),
        ValueType::Boolean => value.is_boolean(),
    }
}

/// The body paths a guard reads, each the part after `body.` of a reference.
fn guarded_body_paths(guard: &Guard) -> impl Iterator<Item = &str> {
    guard
        .references()
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

fn present_read<'de, D>(deserializer: D) -> std::result::Result<Option<Read>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    Read::deserialize(deserializer).map(Some)
}

fn present_flag<'de, D>(deserializer: D) -> std::result::Result<Option<bool>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    bool::deserialize(deserializer).map(Some)
}

fn refuse(message: impl Into<String>) -> Error {
    Error::invalid(message)
}

/// Every bounded value present must be a decimal integer within its range, or,
/// under a `values` bound, exactly one of the allowed strings; a repeated
/// parameter's bound holds for each element. Checked on the bound value
/// strings, the text each is sent as, before any request.
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
    if !bound.values.is_empty() {
        if bound.values.iter().any(|allowed| allowed == text) {
            return Ok(());
        }
        return Err(refuse(format!(
            "parameter `{name}` is not one of its allowed values"
        )));
    }
    let digits = text.strip_prefix('-').unwrap_or(text);
    let value = (!digits.is_empty() && digits.bytes().all(|b| b.is_ascii_digit()))
        .then(|| text.parse::<i128>().ok())
        .flatten()
        .ok_or_else(|| refuse(format!("parameter `{name}` is not an integer")))?;
    if let Some(maximum) = bound.maximum
        && value > i128::from(maximum)
    {
        return Err(refuse(format!(
            "parameter `{name}` is above its maximum of {maximum}"
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

/// A `download` selection: no bundle operation, one declared input `url`.
struct Downloaded {
    selection: Selection,
    declaration: connectors_core::Operation,
}

/// The templates of a guard's reads.
struct Probe {
    /// Each read before the write, in the order it is issued.
    preflights: Vec<Template>,
    /// The read after the write, when the guard declares one.
    postflight: Option<Template>,
}

/// The engine over one bundle. Built once per process; every selection is
/// checked against the inventory before any request can be made.
pub struct Engine {
    base_segments: Vec<String>,
    source_revision: String,
    exposed: Vec<Exposed>,
    /// The `download` selections, in the order they were selected.
    downloads: Vec<Downloaded>,
    /// Each guarded selection's probe, keyed by the selection id.
    probes: BTreeMap<String, Probe>,
    feed: Option<feed::Feed>,
}

/// One request, bound and ready to be sent exactly once.
pub struct Prepared {
    method: WriteMethod,
    segments: Vec<String>,
    query: Vec<(String, String)>,
    body: Value,
    postflight: Vec<(String, String)>,
    /// The postflight's `any_of`: at least one must hold, when declared.
    alternatives: Vec<(String, String)>,
    /// The bound read the postflight checks, when the guard declares one.
    reread: Option<Request>,
    /// Whether the read after the write must answer 404 (`postflight.absent`).
    absent: bool,
    resource: String,
    instance: String,
    source_revision: String,
    rate_limit_reasons: Vec<String>,
}

impl Engine {
    /// `base_path` is the path the provider authority already carries, such as
    /// `/api/v4`; every selected operation's declared path must start with it.
    pub fn new(bundle: &Bundle, base_path: &str, selections: &[Selection]) -> Result<Self> {
        Self::with_feed(bundle, base_path, selections, None)
    }

    /// [`Engine::new`], and the feed binding `feed` declares, if any. With a feed the
    /// selections may be empty; without one there must be at least one.
    pub fn with_feed(
        bundle: &Bundle,
        base_path: &str,
        selections: &[Selection],
        feed: Option<&feed::Declaration>,
    ) -> Result<Self> {
        let base_segments: Vec<String> = base_path
            .split('/')
            .filter(|s| !s.is_empty())
            .map(str::to_owned)
            .collect();
        if (selections.is_empty() && feed.is_none()) || selections.len() > 256 {
            return Err(refuse("select between one and 256 operations"));
        }
        // A source may declare one `operationId` on more than one operation
        // (Runpod's `UpdatePod` is both a PATCH and a POST). The id alone then
        // names no single request, so it is refused rather than resolved to
        // whichever the inventory lists first.
        let find = |operation_id: &str| -> Result<&Operation> {
            let mut matching = bundle
                .inventory
                .operations
                .iter()
                .filter(|o| o.operation_id.as_deref() == Some(operation_id));
            let operation = matching
                .next()
                .ok_or_else(|| refuse(format!("bundle carries no operation `{operation_id}`")))?;
            if matching.next().is_some() {
                return Err(refuse(format!(
                    "bundle carries more than one operation `{operation_id}`"
                )));
            }
            Ok(operation)
        };
        let mut exposed = Vec::new();
        let mut downloads = Vec::new();
        let mut probes = BTreeMap::new();
        let mut ids = std::collections::BTreeSet::new();
        for selection in selections {
            if !connectors_core::valid_id(&selection.id) || !ids.insert(selection.id.clone()) {
                return Err(refuse(format!(
                    "invalid or repeated selection id `{}`",
                    selection.id
                )));
            }
            // The family's ids mean the family's operations and nothing else.
            if [feed::CONTAINERS, feed::ITEMS].contains(&selection.id.as_str()) {
                return Err(refuse(format!(
                    "selection `{}` takes an id of the feed family",
                    selection.id
                )));
            }
            check_binary(selection)?;
            if let Some(download) = &selection.download {
                check_download(selection, download)?;
                downloads.push(Downloaded {
                    selection: selection.clone(),
                    declaration: declare_download(selection, download),
                });
                continue;
            }
            if selection.operation_id.is_empty() {
                return Err(refuse(format!(
                    "selection `{}` names no operation_id and declares no download",
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
            // The names a guard reads as input, through its preflight values
            // and the expectations of its checks.
            let guard_inputs: Vec<&str> = selection
                .guard
                .iter()
                .flat_map(Guard::references)
                .map(|path| path.split('.').next().unwrap_or(path))
                .collect();
            // A credential the document passes as a parameter travels in the
            // connection's authentication header instead: it leaves the
            // operation before anything reads its parameters, whether or not
            // the document requires it, so a required `token` header no longer
            // refuses the selection below.
            for name in &selection.credential {
                let declared: Vec<_> = operation
                    .parameters
                    .iter()
                    .filter(|p| &p.name == name)
                    .collect();
                let reason = if declared.is_empty()
                    || declared
                        .iter()
                        .any(|p| !matches!(p.location, Location::Query | Location::Header))
                {
                    Some("which is not only a query or header parameter of its operation")
                } else if selection.bounds.contains_key(name) {
                    Some("which it also bounds")
                } else if selection.required.contains(name) || selection.withhold.contains(name) {
                    Some("which it also marks required or withholds")
                } else if guard_inputs.contains(&name.as_str()) {
                    Some("which its guard reads as an input")
                } else if selection
                    .body_keys
                    .iter()
                    .any(|key| key.eq_ignore_ascii_case(name))
                {
                    Some("which its body_keys admit")
                } else if selection.guard.as_ref().is_some_and(|guard| {
                    guard.bound().any(|(key, _)| key.eq_ignore_ascii_case(name))
                }) {
                    Some("which its guard's preflight sends as a probe parameter")
                } else {
                    None
                };
                if let Some(reason) = reason {
                    return Err(refuse(format!(
                        "selection `{}` passes `{name}` as the credential, {reason}",
                        selection.id
                    )));
                }
            }
            operation
                .parameters
                .retain(|p| !selection.credential.contains(&p.name));
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
            for (name, bound) in &selection.bounds {
                let problem = match (bound.maximum, bound.values.is_empty()) {
                    (Some(_), false) => Some("with both a range and `values`"),
                    (None, true) => Some("with neither a `maximum` nor `values`"),
                    (Some(maximum), true) => bound
                        .minimum
                        .is_some_and(|minimum| minimum > maximum)
                        .then_some("with a minimum above its maximum"),
                    (None, false) => {
                        let string = operation.parameters.iter().any(|p| {
                            &p.name == name
                                && p.location == Location::Query
                                && p.value_type == Some(ValueType::String)
                        });
                        let mut seen = BTreeSet::new();
                        if bound.minimum.is_some() {
                            Some("with a `minimum` beside `values`")
                        } else if !string {
                            Some("with `values`, but the source does not type it as a string")
                        } else if bound
                            .values
                            .iter()
                            .any(|value| value.is_empty() || !seen.insert(value))
                        {
                            Some("with an empty or repeated value")
                        } else {
                            None
                        }
                    }
                };
                if let Some(problem) = problem {
                    return Err(refuse(format!(
                        "selection `{}` bounds `{name}` {problem}",
                        selection.id
                    )));
                }
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
                let probe_operation = |operation_id: &str| -> Result<Operation> {
                    let mut probe = find(operation_id)?.clone();
                    // The probe passes the credential the way the selected
                    // operation does: through the connection's header alone,
                    // never as a parameter bound from caller input.
                    probe
                        .parameters
                        .retain(|p| !selection.credential.contains(&p.name));
                    if probe.method != "get" || !path_within(&probe.path, &base_segments) {
                        return Err(refuse(format!(
                            "guard of `{}` must read through a GET under the base path",
                            selection.id
                        )));
                    }
                    Ok(probe)
                };
                let mut reads = vec![probe_operation(&guard.preflight.operation_id)?];
                let checks = guard.checks().collect::<Vec<_>>();
                if guard.preflight.checks.is_empty()
                    || checks.len() > 16
                    || checks.iter().any(|c| !c.pointer.starts_with('/'))
                {
                    return Err(refuse(format!(
                        "guard of `{}` needs one to sixteen checks with JSON pointers",
                        selection.id
                    )));
                }
                if guard.further_preflights.len() > 3
                    || guard
                        .further_preflights
                        .iter()
                        .any(|preflight| preflight.checks.is_empty())
                {
                    return Err(refuse(format!(
                        "guard of `{}` declares more than three further preflights, or one without checks",
                        selection.id
                    )));
                }
                match guard.postflight.absent {
                    None => {}
                    Some(true)
                        if guard.postflight.read.is_some()
                            && guard.postflight.checks.is_empty()
                            && guard.postflight.any_of.is_empty() => {}
                    Some(_) => {
                        return Err(refuse(format!(
                            "guard of `{}` declares absent other than true beside a read with no checks",
                            selection.id
                        )));
                    }
                }
                if guard.postflight.read.is_some()
                    && guard.postflight.checks.is_empty()
                    && guard.postflight.absent.is_none()
                {
                    return Err(refuse(format!(
                        "guard of `{}` declares a postflight read without checks",
                        selection.id
                    )));
                }
                if guard.postflight.any_of.len() == 1 {
                    return Err(refuse(format!(
                        "guard of `{}` declares an any_of of one comparison",
                        selection.id
                    )));
                }
                for preflight in &guard.further_preflights {
                    reads.push(probe_operation(&preflight.operation_id)?);
                }
                let template = |operation: &Operation| {
                    Template::from_operation(operation).map_err(|refusal| refuse(refusal.reason()))
                };
                let preflights = reads.iter().map(template).collect::<Result<Vec<_>>>()?;
                let postflight = guard
                    .postflight
                    .read
                    .as_ref()
                    .map(|read| template(&probe_operation(&read.operation_id)?))
                    .transpose()?;
                // Keyed by the selection: two selections may read one probe
                // operation with different credential parameters removed.
                probes.insert(
                    selection.id.clone(),
                    Probe {
                        preflights,
                        postflight,
                    },
                );
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
            // Types and requirements name keys of a closed body: a key the
            // set does not admit could never be sent, and a requirement named
            // twice is a mistake in the selection.
            let admitted = |key: &String| selection.body_keys.contains(key);
            let mut required = std::collections::BTreeSet::new();
            if !selection.body_types.keys().all(admitted) {
                return Err(refuse(format!(
                    "selection `{}` declares body_types for a key its body_keys do not admit",
                    selection.id
                )));
            }
            if !selection
                .body_required
                .iter()
                .all(|key| admitted(key) && required.insert(key))
            {
                return Err(refuse(format!(
                    "selection `{}` declares body_required that do not name distinct keys its body_keys admit",
                    selection.id
                )));
            }
            // A fixed key is a closed body key the caller never sends: its value
            // is a scalar of the key's type, and nothing may also require it
            // from the caller or compare it as caller input.
            for (key, value) in &selection.body_fixed {
                let scalar_value =
                    value.is_string() || value.is_boolean() || value.is_i64() || value.is_u64();
                let reason = if !admitted(key) {
                    Some("which its body_keys do not admit")
                } else if !scalar_value {
                    Some("to a value that is not a string, an integer or a boolean")
                } else if selection
                    .body_types
                    .get(key)
                    .is_some_and(|value_type| !fits_body_type(*value_type, value))
                {
                    Some("to a value that is not of its declared type")
                } else if selection.body_required.contains(key) {
                    Some("which its body_required also names")
                } else if selection
                    .guard
                    .as_ref()
                    .is_some_and(|guard| guarded_body_keys(guard).contains(key.as_str()))
                {
                    Some("which its guard reads as an input")
                } else {
                    None
                };
                if let Some(reason) = reason {
                    return Err(refuse(format!(
                        "selection `{}` fixes body key `{key}`, {reason}",
                        selection.id
                    )));
                }
            }
            // A guard path nested under a key typed as a scalar could never
            // resolve: refused here, not at every write.
            if let Some(path) = selection.guard.as_ref().and_then(|guard| {
                guarded_body_paths(guard).find(|rest| {
                    rest.split_once('.')
                        .is_some_and(|(key, _)| selection.body_types.contains_key(key))
                })
            }) {
                return Err(refuse(format!(
                    "guard of `{}` reads `body.{path}` under a key its body_types type as a scalar",
                    selection.id
                )));
            }
            if selection.response == Some(ResponseKind::Text) && selection.effect != Effect::Read {
                return Err(refuse(format!(
                    "selection `{}` declares a text response for a write",
                    selection.id
                )));
            }
            let mut declaration = declare(selection, &operation);
            if selection.binary.is_some() {
                declaration.output_schema["properties"]["body"] = binary_body_schema();
            }
            let text = expects_text(selection, &operation);
            exposed.push(Exposed {
                selection: selection.clone(),
                operation,
                template,
                declaration,
                text,
            });
        }
        let feed = feed
            .map(|declaration| feed::Feed::new(bundle, &base_segments, declaration))
            .transpose()?;
        Ok(Self {
            base_segments,
            source_revision: bundle.source.source_sha256.clone(),
            exposed,
            downloads,
            probes,
            feed,
        })
    }

    /// Every origin a selected binary read may reach besides the API base. A
    /// connection admits only these; a read reaching one it does not admit is
    /// refused before anything is sent there.
    pub fn reached_hosts(&self) -> BTreeSet<&str> {
        self.exposed
            .iter()
            .map(|e| &e.selection)
            .chain(self.downloads.iter().map(|d| &d.selection))
            .filter_map(|selection| selection.binary.as_ref())
            .flat_map(|binary| binary.hosts.iter().map(String::as_str))
            .collect()
    }

    /// The declarations for the selected operations with these effects, then the feed's two
    /// reads when a feed is declared.
    pub fn declarations(&self, effects: &[Effect]) -> Vec<connectors_core::Operation> {
        let mut declarations: Vec<connectors_core::Operation> = self
            .exposed
            .iter()
            .filter(|e| effects.contains(&e.selection.effect))
            .map(|e| e.declaration.clone())
            .collect();
        if effects.contains(&Effect::Read) {
            declarations.extend(self.downloads.iter().map(|d| d.declaration.clone()));
        }
        if let Some(feed) = self
            .feed
            .as_ref()
            .filter(|_| effects.contains(&Effect::Read))
        {
            declarations.extend(feed.declarations());
        }
        declarations
    }

    pub fn effect(&self, id: &str) -> Option<Effect> {
        if self.feed.is_some() && [feed::CONTAINERS, feed::ITEMS].contains(&id) {
            return Some(Effect::Read);
        }
        if self.download(id).is_some() {
            return Some(Effect::Read);
        }
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

    fn download(&self, id: &str) -> Option<&Downloaded> {
        self.downloads.iter().find(|d| d.selection.id == id)
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

    /// One GET, projected as an observation. A connection that admits no host
    /// besides its API base: [`Engine::read_reaching`] with [`NoHosts`].
    pub async fn read(
        &self,
        http: &dyn AuthenticatedHttp,
        instance: &str,
        id: &str,
        input: Value,
    ) -> Result<Value> {
        self.read_reaching(http, &NoHosts, instance, id, input)
            .await
    }

    /// One GET, projected as an observation, where a binary read may reach
    /// the origins `hosts` admits besides the API base `http`.
    pub async fn read_reaching(
        &self,
        http: &dyn AuthenticatedHttp,
        hosts: &dyn Hosts,
        instance: &str,
        id: &str,
        input: Value,
    ) -> Result<Value> {
        if let Some(feed) = &self.feed {
            match id {
                feed::CONTAINERS => return feed.containers(http, instance, input).await,
                feed::ITEMS => return feed.items(http, instance, input).await,
                _ => {}
            }
        }
        if let Some(download) = self.download(id) {
            return self.read_download(download, hosts, instance, input).await;
        }
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
        if let Some(binary) = &exposed.selection.binary {
            let (status, body) =
                read_binary(response, None, binary, hosts, &exposed.selection).await?;
            return Ok(json!({
                "status": status,
                "body": body,
                "provenance": provenance(instance, &exposed.operation.path, &self.source_revision),
            }));
        }
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

    /// A `download`: the caller's `url`, held to an origin the selection reaches
    /// and the connection admits and to the selection's path prefix, read once
    /// as a binary body through that origin's port.
    async fn read_download(
        &self,
        download: &Downloaded,
        hosts: &dyn Hosts,
        instance: &str,
        input: Value,
    ) -> Result<Value> {
        let selection = &download.selection;
        let (Some(binary), Some(declared)) = (&selection.binary, &selection.download) else {
            return Err(Error::internal());
        };
        connectors_sdk::validate(&download.declaration.input_schema, &input)?;
        let url = input
            .get("url")
            .and_then(Value::as_str)
            .ok_or_else(|| refuse("`url` is not a string"))?;
        let target = Target::absolute(url)
            .ok_or_else(|| refuse("`url` is not an https URL this engine can send"))?;
        if !binary.hosts.contains(&target.origin) {
            return Err(Error::new(
                ErrorCode::Forbidden,
                format!(
                    "`url` is on `{}`, which this download does not reach",
                    target.origin
                ),
            ));
        }
        if !target.raw_path.starts_with(&declared.path_prefix) {
            return Err(refuse(format!(
                "`url` is not under `{}`",
                declared.path_prefix
            )));
        }
        let port = hosts.port(&target.origin).ok_or_else(|| {
            Error::new(
                ErrorCode::Forbidden,
                format!(
                    "`url` is on `{}`, which the connection does not admit",
                    target.origin
                ),
            )
        })?;
        let response = target.get(port).await?;
        let (status, body) = read_binary(
            response,
            Some(target.origin.clone()),
            binary,
            hosts,
            selection,
        )
        .await?;
        Ok(json!({
            "status": status,
            "body": body,
            "provenance": provenance(
                instance,
                &format!("{}{}", target.origin, declared.path_prefix),
                &self.source_revision,
            ),
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
        if self.download(id).is_some() {
            return Err(Error::new(ErrorCode::Forbidden, "operation is a read"));
        }
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
        let omitted = input.get("body").is_none() && exposed.selection.body_fixed_whole();
        if !body_keys.is_empty()
            && !omitted
            && !input
                .get("body")
                .and_then(Value::as_object)
                .is_some_and(|body| body.keys().all(|key| body_keys.contains(key)))
        {
            return Err(refuse("body carries a key its selection does not admit"));
        }
        // A fixed key is never the caller's, whatever value it carries.
        if let Some(key) = input
            .get("body")
            .and_then(Value::as_object)
            .and_then(|body| {
                body.keys()
                    .find(|key| exposed.selection.body_fixed.contains_key(key.as_str()))
            })
        {
            return Err(refuse(format!(
                "body key `{key}` is fixed by its selection and is not caller input"
            )));
        }
        // So are the closed body's requirements and types.
        if let Some(body) = input.get("body").and_then(Value::as_object) {
            if let Some(key) = exposed
                .selection
                .body_required
                .iter()
                .find(|key| !body.contains_key(key.as_str()))
            {
                return Err(refuse(format!("body lacks the required key `{key}`")));
            }
            if let Some((key, _)) = exposed
                .selection
                .body_types
                .iter()
                .find(|(key, value_type)| {
                    body.get(key.as_str())
                        .is_some_and(|value| !fits_body_type(**value_type, value))
                })
            {
                return Err(refuse(format!(
                    "body key `{key}` is not of its declared type"
                )));
            }
        }
        // The credential is never sent from caller input, so a body naming
        // it under any ASCII case is refused here as well as in the
        // declaration.
        if let Some(body) = input.get("body").and_then(Value::as_object)
            && body.keys().any(|key| {
                exposed
                    .selection
                    .credential
                    .iter()
                    .any(|name| key.eq_ignore_ascii_case(name))
            })
        {
            return Err(refuse("body carries its selection's credential parameter"));
        }
        let values = Self::parameter_values(&exposed.operation, &input)?;
        check_bounds(&exposed.selection, &values)?;
        let (segments, query) = self.resolve(&exposed.template, values)?;
        let mut body = if exposed.operation.request_media_types.is_empty() {
            Value::Null
        } else {
            input.get("body").cloned().unwrap_or(Value::Null)
        };
        // The fixed values are always sent, into a body the caller may omit.
        if !exposed.selection.body_fixed.is_empty() {
            let mut members = body.as_object().cloned().unwrap_or_default();
            members.extend(exposed.selection.body_fixed.clone());
            body = Value::Object(members);
        }
        let mut postflight = Vec::new();
        let mut alternatives = Vec::new();
        let mut reread = None;
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
            let preflights = guard
                .preflights()
                .map(|preflight| expected(&preflight.checks))
                .collect::<Result<Vec<_>>>()?;
            postflight = expected(&guard.postflight.checks)?;
            alternatives = expected(&guard.postflight.any_of)?;
            let probe = self
                .probes
                .get(&exposed.selection.id)
                .ok_or_else(Error::internal)?;
            // So does every read's request, the one after the write included.
            let bind =
                |values: &BTreeMap<String, String>, template: &Template| -> Result<Request> {
                    let mut bound = BTreeMap::new();
                    for (parameter, path) in values {
                        let value = reference(&input, path)
                            .and_then(scalar)
                            .ok_or_else(|| refuse(format!("guard value `{path}` is absent")))?;
                        bound.insert(parameter.clone(), Supplied::One(value));
                    }
                    self.resolve(template, bound)
                };
            let requests = guard
                .preflights()
                .zip(&probe.preflights)
                .map(|(preflight, template)| bind(&preflight.values, template))
                .collect::<Result<Vec<_>>>()?;
            reread = match (&guard.postflight.read, &probe.postflight) {
                (Some(read), Some(template)) => Some(bind(&read.values, template)?),
                (None, None) => None,
                _ => return Err(Error::internal()),
            };
            for ((segments, query), preflight) in requests.iter().zip(&preflights) {
                let borrowed: Vec<&str> = segments.iter().map(String::as_str).collect();
                let query: Vec<(&str, String)> =
                    query.iter().map(|(k, v)| (k.as_str(), v.clone())).collect();
                let response = http.get(&borrowed, &query).await?;
                // A target the preflight cannot find is a definite refusal:
                // nothing has been sent.
                if response.status == 404 {
                    return Err(Error::new(
                        ErrorCode::Forbidden,
                        "guard target was not found before dispatch",
                    ));
                }
                let observed = read_body(&response, false, &exposed.selection.rate_limit_reasons)?;
                for (pointer, expected) in preflight {
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
                            format!(
                                "value at `{pointer}` differs from the pinned one before dispatch"
                            ),
                        ));
                    }
                }
            }
        }
        Ok(Prepared {
            method,
            segments,
            query,
            body,
            postflight,
            alternatives,
            reread,
            absent: exposed
                .selection
                .guard
                .as_ref()
                .is_some_and(|guard| guard.postflight.absent == Some(true)),
            resource: exposed.operation.path.clone(),
            instance: instance.to_owned(),
            source_revision: self.source_revision.clone(),
            rate_limit_reasons: exposed.selection.rate_limit_reasons.clone(),
        })
    }
}

impl Prepared {
    /// Whether the guard reads after the write, so [`Prepared::execute_reading`]
    /// needs a read capability.
    pub fn reads_after(&self) -> bool {
        self.reread.is_some()
    }

    /// Whether the observed answer satisfies the postflight's `any_of`: at
    /// least one of its comparisons holds, or it declares none.
    fn accepts_one(&self, observed: &Value) -> bool {
        self.alternatives.is_empty()
            || self.alternatives.iter().any(|(pointer, expected)| {
                observed.pointer(pointer).and_then(scalar).as_deref() == Some(expected.as_str())
            })
    }

    /// Send once and classify. Only documented definite refusals are refused;
    /// everything else that is not a success leaves the effect possible. A
    /// guard that reads after the write has no read capability here, so its
    /// success is unknown; use [`Prepared::execute_reading`].
    pub async fn execute(self, capability: Box<dyn AuthenticatedWrite>) -> WriteOutcome<Value> {
        self.execute_reading(capability, None).await
    }

    /// [`Prepared::execute`], with the capability the guard's read after the
    /// write is issued through, once, after a 2xx.
    pub async fn execute_reading(
        self,
        capability: Box<dyn AuthenticatedWrite>,
        reader: Option<&dyn AuthenticatedHttp>,
    ) -> WriteOutcome<Value> {
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
                Error::new(code, "provider refused the write")
                    .answered()
                    .with_upstream_reason(connectors_core::reason::from_body(&response.body)),
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
        let Some((segments, query)) = &self.reread else {
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
            if !self.accepts_one(&body) {
                return WriteOutcome::Unknown(Error::new(
                    ErrorCode::UpstreamProtocol,
                    "write acknowledged with none of the accepted values; the effect is possible",
                ));
            }
            return WriteOutcome::Applied(Ok(json!({
                "status": response.status,
                "body": body,
                "provenance": provenance(&self.instance, &self.resource, &self.source_revision),
            })));
        };
        // A write that answers without a body is proved by the read the guard
        // declares, issued once. The same boundary holds: a read that fails,
        // or answers with another value, leaves the effect possible.
        let unknown = |message: String| {
            WriteOutcome::Unknown(Error::new(ErrorCode::UpstreamProtocol, message))
        };
        let Some(reader) = reader else {
            return unknown(
                "the guard reads after the write and no read capability was given; the effect is possible"
                    .into(),
            );
        };
        let borrowed: Vec<&str> = segments.iter().map(String::as_str).collect();
        let query: Vec<(&str, String)> =
            query.iter().map(|(k, v)| (k.as_str(), v.clone())).collect();
        let answer = reader.get(&borrowed, &query).await;
        // A delete is proved by its target no longer being found: only a 404
        // to the read after it. The target still found, or a read that fails
        // otherwise, leaves the effect possible.
        if self.absent {
            return match answer {
                Ok(answer) if answer.status == 404 => WriteOutcome::Applied(Ok(json!({
                    "status": response.status,
                    "body": body,
                    "provenance": provenance(&self.instance, &self.resource, &self.source_revision),
                }))),
                Ok(answer) if (200..300).contains(&answer.status) => unknown(
                    "the guard's read after the write still found its target; the effect is possible"
                        .into(),
                ),
                _ => unknown(
                    "the guard's read after the write failed; the effect is possible".into(),
                ),
            };
        }
        let observed = match answer {
            Ok(answer) => read_body(&answer, false, &self.rate_limit_reasons),
            Err(error) => Err(error),
        };
        let Ok(observed) = observed else {
            return unknown(
                "the guard's read after the write failed; the effect is possible".into(),
            );
        };
        for (pointer, expected) in &self.postflight {
            if observed.pointer(pointer).and_then(scalar).as_deref() != Some(expected.as_str()) {
                return unknown(format!(
                    "the guard's read after the write answered with a value at `{pointer}` other than the pinned one; the effect is possible"
                ));
            }
        }
        if !self.accepts_one(&observed) {
            return unknown(
                "the guard's read after the write answered with none of the accepted values; the effect is possible"
                    .into(),
            );
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
        // Only the body's admitted reason travels on, beside the code, and on
        // a `429` the delay its `Retry-After` names; the body itself, and every
        // other header, stays here.
        let mut error = Error::new(code, "provider refused the request")
            .answered()
            .with_upstream_reason(connectors_core::reason::from_body(&response.body));
        if response.status == 429 {
            error.retry_after_seconds = response
                .headers
                .iter()
                .find(|(name, _)| name.eq_ignore_ascii_case("retry-after"))
                .and_then(|(_, value)| retry_after(value, std::time::SystemTime::now()));
        }
        return Err(error);
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

/// The delay a `Retry-After` value names, in whole seconds from `now`: either
/// form RFC 9110 §10.2.3 gives it, delta-seconds (ASCII digits only) or an
/// HTTP-date (IMF-fixdate, or the obsolete RFC 850 and asctime formats a
/// recipient must accept), a date rounded up to the next whole second and a
/// date already past naming no wait. Anything else, and a delay that does not
/// fit 32 bits, names none.
fn retry_after(value: &str, now: std::time::SystemTime) -> Option<u64> {
    let value = value.trim_matches([' ', '\t']);
    if !value.is_empty() && value.bytes().all(|byte| byte.is_ascii_digit()) {
        return value.parse::<u32>().ok().map(u64::from);
    }
    let date = httpdate::parse_http_date(value).ok()?;
    let ahead = date.duration_since(now).unwrap_or_default();
    let seconds = ahead.as_secs() + u64::from(ahead.subsec_nanos() > 0);
    u32::try_from(seconds).ok().map(u64::from)
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

/// The declared `enum` of a `values` bound: each allowed string, and beside one
/// that is the decimal text of an integer, that integer, which a string
/// parameter also takes and sends as exactly that text.
fn values_enum(values: &[String]) -> Value {
    let mut allowed = Vec::new();
    for value in values {
        allowed.push(json!(value));
        if let Ok(number) = value.parse::<i64>()
            && number.to_string() == *value
        {
            allowed.push(json!(number));
        } else if let Ok(number) = value.parse::<u64>()
            && number.to_string() == *value
        {
            allowed.push(json!(number));
        }
    }
    Value::Array(allowed)
}

/// A pattern matching exactly any of `names`, each ASCII letter in either case
/// and every other character literal, written without inline flags so every
/// JSON Schema reader of the declaration reads it the same way.
fn caseless_pattern(names: &[String]) -> String {
    let alternatives: Vec<String> = names
        .iter()
        .map(|name| {
            name.chars()
                .map(|c| {
                    if c.is_ascii_alphabetic() {
                        format!("[{}{}]", c.to_ascii_lowercase(), c.to_ascii_uppercase())
                    } else if "^$\\.*+?()[]{}|/".contains(c) {
                        // The syntax characters, the only identity escapes
                        // both ECMA-262 and Rust's regex accept.
                        format!("\\{c}")
                    } else {
                        c.to_string()
                    }
                })
                .collect()
        })
        .collect();
    format!("^(?:{})$", alternatives.join("|"))
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
        // Advisory for callers: the engine checks the bound itself on every
        // value. A range constrains only numbers, so it is declared on a
        // repeated parameter's array too; `values` is declared as the `enum` of
        // a single value, and of each element of a repeated one.
        let bounded = |schema: &mut Value, element: bool| {
            if let Some(bound) = selection.bounds.get(&parameter.name) {
                if let Some(maximum) = bound.maximum {
                    schema["maximum"] = json!(maximum);
                }
                if let Some(minimum) = bound.minimum {
                    schema["minimum"] = json!(minimum);
                }
                if element && !bound.values.is_empty() {
                    schema["enum"] = values_enum(&bound.values);
                }
            }
        };
        let schema = if parameter.repeated && parameter.location == Location::Query {
            // An array of the declared element type, sent one pair per
            // element; or one value typed like its elements, a comma-joined
            // list of them included, sent as it was given.
            let mut schema = repeated_schema(parameter.value_type, parameter.required);
            bounded(&mut schema["items"], true);
            bounded(&mut schema, false);
            schema
        } else {
            let mut schema = declared_type(parameter.value_type);
            bounded(&mut schema, true);
            schema
        };
        properties.insert(parameter.name.clone(), schema);
        if parameter.required {
            required.push(parameter.name.clone());
        }
    }
    if !operation.request_media_types.is_empty() {
        let body = if selection.body_keys.is_empty() {
            let mut body = json!({"type": "object"});
            if !selection.credential.is_empty() {
                // An open body still never names a credential parameter,
                // under any ASCII case. A closed body cannot: `body_keys`
                // admitting one is refused when the selection loads.
                body["propertyNames"] =
                    json!({"not": {"pattern": caseless_pattern(&selection.credential)}});
            }
            body
        } else {
            // A closed body: exactly these keys. A key `body_types` types is
            // declared as exactly that JSON type; otherwise a key a guard reads
            // directly is declared as the scalar the guard compares, and any
            // other key takes any JSON value. Every key a guard reads, and
            // every key `body_required` names, is required.
            let scalars = selection
                .guard
                .as_ref()
                .map(scalar_body_keys)
                .unwrap_or_default();
            // A fixed key is the selection's, never the caller's: undeclared.
            let keys: serde_json::Map<String, Value> = selection
                .body_keys
                .iter()
                .filter(|key| !selection.body_fixed.contains_key(*key))
                .map(|key| {
                    let schema = if let Some(value_type) = selection.body_types.get(key) {
                        body_type_schema(*value_type)
                    } else if scalars.contains(key.as_str()) {
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
            let mut required: std::collections::BTreeSet<&str> = selection
                .guard
                .as_ref()
                .map(guarded_body_keys)
                .unwrap_or_default();
            required.extend(selection.body_required.iter().map(String::as_str));
            if !required.is_empty() {
                body["required"] = json!(required);
            }
            body
        };
        properties.insert("body".into(), body);
        if !selection.body_fixed_whole() {
            required.push("body".into());
        }
    }
    if let Some(guard) = &selection.guard {
        for reference in guard.references() {
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
        output_schema: result_schema(),
    }
}

/// The output schema of every selected read and write: the status, the body
/// (any JSON value) and the provenance.
fn result_schema() -> Value {
    json!({
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
    })
}

/// The `body` of a binary read's result (`connectors_catalog.binary.Body`).
fn binary_body_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "media_type": {"type": "string", "minLength": 1},
            "length": {"type": "integer", "minimum": 0, "maximum": BINARY_LIMIT},
            "sha256": {"type": "string", "pattern": "^[0-9a-f]{64}$"},
            "content_base64": {"type": "string"}
        },
        "required": ["media_type", "length", "sha256", "content_base64"],
        "additionalProperties": false,
    })
}

/// The descriptor operation for a `download`: one required input, `url`.
fn declare_download(selection: &Selection, download: &Download) -> connectors_core::Operation {
    let mut output_schema = result_schema();
    output_schema["properties"]["body"] = binary_body_schema();
    let hosts = selection
        .binary
        .as_ref()
        .map(|binary| binary.hosts.join(", "))
        .unwrap_or_default();
    connectors_core::Operation {
        id: selection.id.clone(),
        description: selection.description.clone().unwrap_or_else(|| {
            format!(
                "GET a provider file URL under {} on {hosts}",
                download.path_prefix
            )
        }),
        contract: "operations/v1alpha1".into(),
        profile: "generic-http".into(),
        input_schema: json!({
            "type": "object",
            "properties": {"url": {"type": "string", "minLength": 1, "maxLength": 4096}},
            "required": ["url"],
            "additionalProperties": false,
        }),
        output_schema,
    }
}

/// What a selection's binary members must be, whatever it reads: `binary`
/// exactly when `response` is `binary`, on a read, with `max_bytes` from one
/// to [`BINARY_LIMIT`] and at most four distinct canonical `https` origins.
fn check_binary(selection: &Selection) -> Result<()> {
    let declared = selection.response == Some(ResponseKind::Binary);
    let Some(binary) = &selection.binary else {
        if declared || selection.download.is_some() {
            return Err(refuse(format!(
                "selection `{}` declares a binary response or a download without binary",
                selection.id
            )));
        }
        return Ok(());
    };
    let mut seen = BTreeSet::new();
    let problem = if !declared {
        Some("beside a response that is not binary")
    } else if selection.effect != Effect::Read {
        Some("for a write")
    } else if binary.max_bytes == 0 || binary.max_bytes > BINARY_LIMIT {
        Some("with max_bytes outside 1 to 2097152")
    } else if binary.hosts.len() > BINARY_HOSTS {
        Some("reaching more than four hosts")
    } else if binary
        .hosts
        .iter()
        .any(|host| origin(host).as_deref() != Some(host.as_str()) || !seen.insert(host))
    {
        Some("with a host that is not a distinct canonical https origin")
    } else {
        None
    };
    match problem {
        Some(problem) => Err(refuse(format!(
            "selection `{}` declares binary {problem}",
            selection.id
        ))),
        None => Ok(()),
    }
}

/// What a `download` selection must be beyond its binary members: no bundle
/// operation, at least one host, a path prefix bounded by `/`, and none of the
/// members that bind a bundle operation's parameters or body.
fn check_download(selection: &Selection, download: &Download) -> Result<()> {
    let prefix = &download.path_prefix;
    let problem = if !selection.operation_id.is_empty() {
        Some("beside an operation_id")
    } else if selection.binary.as_ref().is_none_or(|b| b.hosts.is_empty()) {
        Some("that reaches no host")
    } else if prefix.len() < 2
        || !prefix.starts_with('/')
        || !prefix.ends_with('/')
        || prefix.contains(['?', '#', '%', '\\'])
        || prefix
            .split('/')
            .any(|segment| segment == "." || segment == "..")
    {
        Some("whose path_prefix is not a plain path starting and ending with `/`")
    } else if selection.guard.is_some()
        || !selection.bounds.is_empty()
        || !selection.required.is_empty()
        || !selection.withhold.is_empty()
        || !selection.credential.is_empty()
        || !selection.body_keys.is_empty()
        || !selection.body_types.is_empty()
        || !selection.body_required.is_empty()
        || !selection.body_fixed.is_empty()
    {
        Some("beside members that bind a bundle operation")
    } else {
        None
    };
    match problem {
        Some(problem) => Err(refuse(format!(
            "selection `{}` declares a download {problem}",
            selection.id
        ))),
        None => Ok(()),
    }
}

/// One GET to an origin besides the API base: the canonical origin, the raw
/// path as written (for a prefix check), and the decoded segments and query
/// pairs the origin's port sends, each re-encoded by the port.
struct Target {
    origin: String,
    raw_path: String,
    segments: Vec<String>,
    query: Vec<(String, String)>,
}

impl Target {
    /// An absolute `https` URL; `None` for anything else.
    fn absolute(url: &str) -> Option<Self> {
        let origin = origin(url)?;
        let rest = &url["https://".len()..];
        let start = rest.find(['/', '?', '#']).unwrap_or(rest.len());
        Self::reference(origin, &rest[start..])
    }

    /// A redirect's `Location`, absolute or a path on `from`, the origin of the
    /// request it answered; `None` when it is neither, or when it is a path
    /// and `from` is the API base, whose origin the engine does not hold.
    fn location(location: &str, from: Option<&str>) -> Option<Self> {
        if location.starts_with('/') && !location.starts_with("//") {
            return Self::reference(from?.to_owned(), location);
        }
        Self::absolute(location)
    }

    /// The path and query after an origin, the fragment dropped. The path is
    /// at least one segment, none empty, `.` or `..` once decoded.
    fn reference(origin: String, reference: &str) -> Option<Self> {
        let reference = reference.split('#').next().unwrap_or_default();
        let (raw_path, raw_query) = reference.split_once('?').unwrap_or((reference, ""));
        let path = raw_path.strip_prefix('/')?;
        let segments = path
            .split('/')
            .map(|segment| percent_decode(segment, false))
            .collect::<Option<Vec<String>>>()?;
        if segments
            .iter()
            .any(|s| s.is_empty() || s == "." || s == "..")
        {
            return None;
        }
        let query = raw_query
            .split('&')
            .filter(|pair| !pair.is_empty())
            .map(|pair| {
                let (name, value) = pair.split_once('=').unwrap_or((pair, ""));
                Some((percent_decode(name, true)?, percent_decode(value, true)?))
            })
            .collect::<Option<Vec<_>>>()?;
        Some(Self {
            origin,
            raw_path: raw_path.to_owned(),
            segments,
            query,
        })
    }

    async fn get(&self, port: &dyn AuthenticatedHttp) -> Result<HttpResponse> {
        let segments: Vec<&str> = self.segments.iter().map(String::as_str).collect();
        let query: Vec<(&str, String)> = self
            .query
            .iter()
            .map(|(name, value)| (name.as_str(), value.clone()))
            .collect();
        port.get(&segments, &query).await
    }
}

/// `%XX` escapes decoded, and in a query `+` as a space; `None` for a
/// malformed escape or bytes that are not UTF-8.
fn percent_decode(text: &str, query: bool) -> Option<String> {
    let bytes = text.as_bytes();
    let mut decoded = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        match bytes[index] {
            b'%' => {
                let hex = text.get(index + 1..index + 3)?;
                if !hex.bytes().all(|b| b.is_ascii_hexdigit()) {
                    return None;
                }
                decoded.push(u8::from_str_radix(hex, 16).ok()?);
                index += 3;
            }
            b'+' if query => {
                decoded.push(b' ');
                index += 1;
            }
            byte => {
                decoded.push(byte);
                index += 1;
            }
        }
    }
    String::from_utf8(decoded).ok()
}

/// Whether a status is a redirect this engine may follow.
fn redirect(status: u16) -> bool {
    matches!(status, 301 | 302 | 303 | 307 | 308)
}

/// The answer of a binary read, after following at most [`REDIRECT_LIMIT`]
/// redirects, each only to an origin the selection reaches and the connection
/// admits: its status and its body. `from` is the origin of the first request,
/// `None` for the API base.
async fn read_binary(
    mut response: HttpResponse,
    mut from: Option<String>,
    binary: &Binary,
    hosts: &dyn Hosts,
    selection: &Selection,
) -> Result<(u16, Value)> {
    let mut followed = 0;
    while redirect(response.status) {
        let location = response
            .headers
            .iter()
            .find(|(name, _)| name.eq_ignore_ascii_case("location"))
            .map(|(_, value)| value.trim().to_owned())
            .ok_or_else(|| {
                Error::new(
                    ErrorCode::UpstreamProtocol,
                    "provider redirected without a location",
                )
            })?;
        if binary.hosts.is_empty() {
            return Err(Error::new(
                ErrorCode::Forbidden,
                "provider redirected, and this read reaches no other host",
            ));
        }
        followed += 1;
        if followed > REDIRECT_LIMIT {
            return Err(Error::new(
                ErrorCode::UpstreamProtocol,
                "provider redirected more than three times",
            ));
        }
        let target = Target::location(&location, from.as_deref()).ok_or_else(|| {
            Error::new(
                ErrorCode::Forbidden,
                "provider redirected to a location that is not an https URL this engine can send",
            )
        })?;
        if !binary.hosts.contains(&target.origin) {
            return Err(Error::new(
                ErrorCode::Forbidden,
                format!(
                    "provider redirected to `{}`, which this read does not reach",
                    target.origin
                ),
            ));
        }
        let port = hosts.port(&target.origin).ok_or_else(|| {
            Error::new(
                ErrorCode::Forbidden,
                format!(
                    "provider redirected to `{}`, which the connection does not admit",
                    target.origin
                ),
            )
        })?;
        response = target.get(port).await?;
        from = Some(target.origin);
    }
    if !(200..300).contains(&response.status) {
        // Classified exactly as any other read's refusal; never `Ok`.
        read_body(&response, false, &selection.rate_limit_reasons)?;
        return Err(Error::new(
            ErrorCode::UpstreamProtocol,
            "provider answered outside 2xx",
        ));
    }
    let length = response.body.len() as u64;
    if length > binary.max_bytes {
        return Err(Error::new(
            ErrorCode::Capacity,
            format!(
                "binary response of {length} bytes exceeds max_bytes of {}",
                binary.max_bytes
            ),
        ));
    }
    Ok((response.status, binary_body(&response)))
}

/// A binary body: its media type, length, SHA-256 and standard base64.
fn binary_body(response: &HttpResponse) -> Value {
    use base64::{Engine as _, engine::general_purpose::STANDARD};
    use sha2::{Digest as _, Sha256};
    let media_type = response
        .headers
        .iter()
        .find(|(name, _)| name.eq_ignore_ascii_case("content-type"))
        .and_then(|(_, value)| value.split(';').next())
        .map(|value| value.trim().to_ascii_lowercase())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "application/octet-stream".to_owned());
    let sha256: String = Sha256::digest(&response.body)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect();
    json!({
        "media_type": media_type,
        "length": response.body.len(),
        "sha256": sha256,
        "content_base64": STANDARD.encode(&response.body),
    })
}
