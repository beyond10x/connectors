//! One `connectors-definition/1` connector definition, read and lowered into
//! what the catalog provider already runs: a bundle with its index, compiled by
//! [`crate::pipeline`] from the vendored source the definition pins, a
//! `connectors-catalog-operations/1` selection of the operations it exposes, and
//! the `connectors-catalog-connector/1` declaration of the connector and its
//! auth profiles, byte for byte as the catalog commits it.
//!
//! The types mirror the schema (`spec/domains/definition.yaml` in the
//! definitions repository) field for field, and every struct refuses a field
//! the schema does not name. A definition the lowering cannot honour is refused
//! by name ([`Refusal`]) before anything is written to the output directory.
//!
//! The maps a file lists (operations, body keys, guard comparisons, auth
//! profiles) keep the file's order: the selection lists its operations, a closed
//! body its keys and a guard its checks in the order the definition writes them.

use crate::{bundle, pipeline};
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::io::Read as _;
use std::path::{Component, Path, PathBuf};

/// The one schema identifier this reader accepts.
pub const SCHEMA: &str = "urn:beyond10x:connectors-definition:1";
/// The longest `evidence_lifetime_ms` the catalog provider takes.
pub const EVIDENCE_LIFETIME_MS: u64 = 300_000;
/// The selection format the catalog provider's `operations_file` reads.
pub const OPERATIONS_FORMAT: &str = "connectors-catalog-operations/1";
/// The optional definition members this lowerer reads that a connectors-definitions
/// release names in its index's `features`, sorted: the members connectors-definitions
/// 0.3.0 and later declare. `connectors definitions install` admits exactly these
/// (D247); a test that links both crates holds the two lists equal.
pub const READ_FEATURES: &[&str] = &[
    "embedded_text",
    "query_fixed",
    "query_folds",
    "rate_limit_headers",
];

/// `connectors_definitions.definition.SchemaId`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum SchemaId {
    #[serde(rename = "urn:beyond10x:connectors-definition:1")]
    V1,
}

/// `connectors_definitions.definition.Source`: the vendored document, by its
/// path under the definitions root, and the SHA-256 of its decompressed bytes.
/// Exactly one of `openapi`, `swagger` (Swagger 2.0) and `discovery` (Google
/// Discovery) names it. A Swagger or Discovery document is projected to
/// OpenAPI as `connectors-build catalog --derived-from` projects it, and
/// `projection` is the file name the projected document is recorded under.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Source {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub openapi: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub swagger: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub discovery: Option<String>,
    pub sha256: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub projection: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub amendments: Option<String>,
}

/// Which kind of document a source names.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceKind {
    OpenApi,
    Swagger,
    Discovery,
}

impl Source {
    /// The one document this source names, and its kind.
    pub fn document(&self) -> Result<(SourceKind, &str), Refusal> {
        let named: Vec<(SourceKind, &str)> = [
            (SourceKind::OpenApi, &self.openapi),
            (SourceKind::Swagger, &self.swagger),
            (SourceKind::Discovery, &self.discovery),
        ]
        .into_iter()
        .filter_map(|(kind, path)| path.as_deref().map(|path| (kind, path)))
        .collect();
        let [(kind, path)] = named[..] else {
            return Err(Refusal::SourceKind(format!(
                "a source names exactly one of openapi, swagger and discovery; this one names {}",
                named.len()
            )));
        };
        match (kind, &self.projection) {
            (SourceKind::OpenApi, Some(_)) => Err(Refusal::SourceKind(
                "an openapi source is not projected and names no projection".to_owned(),
            )),
            (SourceKind::Swagger | SourceKind::Discovery, None) => Err(Refusal::SourceKind(
                "a swagger or discovery source names the projection's file name".to_owned(),
            )),
            _ => Ok((kind, path)),
        }
    }
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub values: Option<Vec<String>>,
}

/// The schema identifier of a shared auth file, `auth/<name>.yaml`.
pub const AUTH_SCHEMA: &str = "urn:beyond10x:connectors-auth:1";
/// The connector declaration the lowering writes: the catalog's
/// `providers/<name>/connector.json`.
pub const CONNECTOR_FORMAT: &str = "connectors-catalog-connector/1";

/// `connectors_definitions.definition.AuthScheme`: `basic` sends
/// `Authorization: Basic`, `bearer` `Authorization: Bearer`, `header` the token
/// in `header`, behind `prefix` when one is given; the two OAuth schemes
/// exchange the stored entry at `token_url` for a bearer access token.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AuthScheme {
    Basic,
    Bearer,
    Header,
    #[serde(rename = "oauth2_refresh")]
    OAuth2Refresh,
    #[serde(rename = "oauth2_client_credentials")]
    OAuth2ClientCredentials,
}

impl AuthScheme {
    fn is_oauth(self) -> bool {
        matches!(self, Self::OAuth2Refresh | Self::OAuth2ClientCredentials)
    }
}

/// `connectors_definitions.definition.IdentitySource`: where the subject of a
/// credential comes from. `api` (the default) reads `path`; `id_token` the
/// OAuth token response; `configuration` names the connection itself after a
/// read of `path` proves the credential.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IdentitySource {
    Api,
    IdToken,
    Configuration,
}

/// `connectors_definitions.definition.AuthIdentity`: the read that names who
/// holds the credential.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AuthIdentity {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<IdentitySource>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kind: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subject: Option<String>,
}

/// `connectors_definitions.definition.AuthScopes`: the read that names the
/// granted scopes (`path` and `pointer`, both or neither), the scopes a
/// connection must hold, and the scopes an OAuth consent requests.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AuthScopes {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pointer: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub minimum: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub requested: Option<Vec<String>>,
}

/// `connectors_definitions.definition.AuthAccess`: a read whose `200` proves
/// the credential reaches what the selection needs.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AuthAccess {
    pub path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub query: Option<BTreeMap<String, String>>,
}

/// `connectors_definitions.definition.Auth`: one profile. With `use`, the
/// profile of the same id in `auth/<use>.yaml` is the base, and every field
/// given here replaces its counterpart; `identity` field by field.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Auth {
    #[serde(rename = "use", skip_serializing_if = "Option::is_none")]
    pub library: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scheme: Option<AuthScheme>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub header: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prefix: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub account_label: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub identity: Option<AuthIdentity>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scopes: Option<AuthScopes>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub access: Option<AuthAccess>,
    /// How long a validation of the credential stays current, in
    /// milliseconds; the catalog provider takes 1 to 300000 and holds 60000
    /// when the declaration states none.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub evidence_lifetime_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub token_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub authorize_url: Option<String>,
}

/// `connectors_definitions.definition.AuthLibraryId`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum AuthLibraryId {
    #[serde(rename = "urn:beyond10x:connectors-auth:1")]
    V1,
}

/// `connectors_definitions.definition.AuthLibrary`: profiles several connectors
/// share, `auth/<name>.yaml`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AuthLibrary {
    pub schema: AuthLibraryId,
    pub profiles: IndexMap<String, Auth>,
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
/// walk itself. A style matches an operation that has its `request` query
/// parameter, or, without `request`, its `limit` one, unless the operation
/// withholds that parameter.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PagingStyle {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last: Option<String>,
}

/// `connectors_definitions.definition.MismatchCode`, the engine's
/// `connectors_catalog.guard.MismatchCode`: what a differing `expect` of a
/// read before the write refuses as, instead of `forbidden`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MismatchCode {
    /// The target changed since the caller read it.
    Conflict,
}

/// `connectors_definitions.definition.GuardBefore`: a read before a write.
/// `read` is an operation key of the definition, else an `operationId` of the
/// source; its path parameters take the input of the same name unless `with`
/// binds them. `expect` maps a JSON pointer to an input reference (`sha`,
/// `body.sha`), a literal (`=opened`) or a boolean (`=true`), in file order;
/// `mismatch` names what a differing one refuses as.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GuardBefore {
    pub read: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub with: Option<IndexMap<String, String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expect: Option<IndexMap<String, String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mismatch: Option<MismatchCode>,
}

/// `connectors_definitions.definition.GuardRead`: a read after the write, its
/// parameters bound as a [`GuardBefore`]'s are.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GuardRead {
    pub read: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub with: Option<IndexMap<String, String>>,
}

/// `connectors_definitions.definition.RefusalCode`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RefusalCode {
    InvalidInput,
    Forbidden,
    NotFound,
}

/// `connectors_definitions.definition.GuardRefusal`, the engine's
/// `connectors_catalog.guard.Refusal`, carried as written.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GuardRefusal {
    pub pointer: String,
    pub values: Vec<String>,
    pub code: RefusalCode,
}

/// `connectors_definitions.definition.GuardAfter`: comparisons against the
/// write's answer, or with `read` against that read's; `any` holds when at
/// least one of its comparisons holds; `absent: true` wants the read to answer
/// 404; `refusals` classify a 2xx answer that did not apply the write.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GuardAfter {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expect: Option<IndexMap<String, String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub any: Option<IndexMap<String, String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub read: Option<GuardRead>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub absent: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub refusals: Option<Vec<GuardRefusal>>,
}

/// `connectors_definitions.definition.Guard`: `before`, then each of `also`,
/// in order, before the write; `after` once it answered, no comparison when
/// absent.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Guard {
    pub before: GuardBefore,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub also: Option<Vec<GuardBefore>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub after: Option<GuardAfter>,
}

/// `connectors_definitions.definition.ResponseKind`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResponseKind {
    Json,
    Text,
    Binary,
}

/// `connectors_definitions.definition.Binary`, the engine's
/// `connectors_catalog.binary.Binary`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Binary {
    pub max_bytes: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hosts: Option<Vec<String>>,
}

/// `connectors_definitions.definition.Download`, the engine's
/// `connectors_catalog.binary.Download`: a read of a provider-issued URL, which
/// names no `op`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Download {
    pub path_prefix: String,
}

/// `connectors_definitions.definition.EnvelopeCode`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EnvelopeCode {
    InvalidInput,
    Unauthorized,
    Forbidden,
    NotFound,
    RateLimited,
    Internal,
}

/// `connectors_definitions.definition.EnvelopeRefusal`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EnvelopeRefusal {
    pub values: Vec<String>,
    pub code: EnvelopeCode,
}

/// `connectors_definitions.definition.Envelope`, the engine's
/// `connectors_catalog.selection.Envelope`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Envelope {
    pub ok: String,
    pub error: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub refusals: Option<Vec<EnvelopeRefusal>>,
    pub otherwise: EnvelopeCode,
}

/// `connectors_definitions.definition.Bound`, the engine's bound of one query
/// parameter: a range or the allowed values.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Bound {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub minimum: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub maximum: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub values: Option<Vec<String>>,
}

/// `connectors_definitions.definition.TextRefusalCode`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TextRefusalCode {
    Capacity,
}

/// `connectors_definitions.definition.TextRefusal`, the engine's
/// `connectors_catalog.selection.TextRefusal`, carried as written: a definite
/// 4xx refusal of a write whose body holds, at one of `pointers`, a string
/// containing one of `contains` is refused with `code`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TextRefusal {
    pub pointers: Vec<String>,
    pub contains: Vec<String>,
    pub code: TextRefusalCode,
}

/// `connectors_definitions.definition.Custody`, the engine's
/// `connectors_catalog.selection.Custody`, carried as written: the pointer of
/// the credential a write's answer carries that the write created, and the
/// public members the caller receives in its place.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Custody {
    pub secret: String,
    pub returns: BTreeMap<String, String>,
}

/// `connectors_definitions.definition.FoldFormat`, the engine's
/// `connectors_catalog.selection.FoldFormat`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FoldFormat {
    Timestamp,
}

/// `connectors_definitions.definition.QueryFold`, the engine's
/// `connectors_catalog.selection.QueryFold`, carried as written: a caller
/// input appended to the fixed query value of `into` as one space, `prefix`
/// and the input in `format`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QueryFold {
    pub into: String,
    pub prefix: String,
    pub format: FoldFormat,
}

/// `connectors_definitions.definition.EmbeddedText`, the engine's
/// `connectors_catalog.selection.EmbeddedText`, carried as written: the file a
/// read's JSON answer carries as base64, answered as its text.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EmbeddedText {
    pub content: String,
    pub encoding: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size: Option<String>,
    pub max_bytes: u64,
}

/// `connectors_definitions.definition.RateLimitHeaders`, the engine's
/// `connectors_catalog.selection.RateLimitHeaders`, carried as written: the
/// headers by which a `403` or `429` says the rate limit is spent.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RateLimitHeaders {
    pub remaining: String,
    pub reset: String,
}

/// `connectors_definitions.definition.Defaults`: the fields an operation
/// carries to the engine as written, given once for every operation that
/// does not set the field itself. `envelope` applies only to the reads that
/// answer JSON (no `response` other than `json`, no `download`, no
/// `embedded_text`), the only operations the engine admits one on;
/// `refusal_texts` and `custody` apply only to writes, the only operations the
/// engine admits them on; every other field, `rate_limit_headers` included,
/// applies to every operation.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Defaults {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub required: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub withhold: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub credential: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bounds: Option<IndexMap<String, Bound>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response: Option<ResponseKind>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub binary: Option<Binary>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub download: Option<Download>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub envelope: Option<Envelope>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rate_limit_reasons: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rate_limit_headers: Option<RateLimitHeaders>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub refusal_texts: Option<Vec<TextRefusal>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub custody: Option<Custody>,
}

/// `connectors_definitions.definition.DatasourceKind`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DatasourceKind {
    Feed,
}

/// `connectors_definitions.definition.FeedCondition`: the value at `pointer`
/// in a page equals `equals`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FeedCondition {
    pub pointer: String,
    pub equals: Value,
}

/// `connectors_definitions.definition.FeedOffset`: the cursor parameter takes
/// the count of records walked so far, and the walk ends on the page where
/// `complete` holds.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FeedOffset {
    pub complete: FeedCondition,
}

/// `connectors_definitions.definition.FeedCursor`: exactly one of `next`,
/// `last` and `offset` (rule `feed_cursor_continuation`), and `url_parameter`
/// only beside `next` (rule `feed_cursor_url_parameter`).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FeedCursor {
    pub parameter: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url_parameter: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub offset: Option<FeedOffset>,
}

/// `connectors_definitions.definition.FeedContainers`: `list` and `lookup`
/// are operation keys of the definition.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FeedContainers {
    pub list: String,
    pub lookup: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub query: Option<BTreeMap<String, String>>,
    pub cursor: FeedCursor,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub records: Option<String>,
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kind_word: Option<String>,
}

/// `connectors_definitions.definition.FeedPrecision`: how finely the provider
/// compares a "changed since" time.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FeedPrecision {
    Instant,
    Minute,
}

impl FeedPrecision {
    fn word(self) -> &'static str {
        match self {
            Self::Instant => "instant",
            Self::Minute => "minute",
        }
    }
}

/// `connectors_definitions.definition.FeedQueryLanguage`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FeedQueryLanguage {
    Jql,
    Cql,
}

/// `connectors_definitions.definition.FeedUtcOffsets`: the least and greatest
/// offset from UTC, in minutes, a provider may compare `{since}` in.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FeedUtcOffsets {
    pub least: i64,
    pub greatest: i64,
}

/// `connectors_definitions.definition.FeedQuery`: the query parameter that
/// takes a whole query, `first` for a walk from the start and `resume` for
/// one from a time.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FeedQuery {
    pub parameter: String,
    pub language: FeedQueryLanguage,
    pub first: String,
    pub resume: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub utc_offsets: Option<FeedUtcOffsets>,
}

/// `connectors_definitions.definition.FeedPage`: the parameter that takes the
/// token at `next` for the following page of items.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FeedPage {
    pub parameter: String,
    pub next: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url_parameter: Option<String>,
}

/// `connectors_definitions.definition.FeedPosition`: "changed since" as the
/// query parameter `time` takes, or as a whole `query`; exactly one of the two
/// (rule `feed_position_filter`).
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FeedPosition {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub time: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub query: Option<FeedQuery>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub precision: Option<FeedPrecision>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub page: Option<FeedPage>,
}

/// `connectors_definitions.definition.FeedAuthor`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FeedAuthor {
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
}

/// `connectors_definitions.definition.FeedBody`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FeedBody {
    pub pointer: String,
    pub representation: String,
}

/// `connectors_definitions.definition.FeedItems`: `list` is an operation key.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FeedItems {
    pub list: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub query: Option<BTreeMap<String, String>>,
    pub position: FeedPosition,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub records: Option<String>,
    pub id: String,
    pub revision: String,
    pub created_at: String,
    pub updated_at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub author: Option<FeedAuthor>,
    pub body: FeedBody,
    /// A JSON pointer to the item's link (rule `feed_item_pointer`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    /// A JSON pointer to the base `url` is relative to (rule `feed_item_pointer`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url_base: Option<String>,
    /// A JSON pointer to the item's parent (rule `feed_item_pointer`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent: Option<String>,
}

/// `connectors_definitions.definition.Datasource`. `filters` name preset item
/// queries, such as "mine and open"; each key is a query parameter of
/// `items.list`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Datasource {
    pub kind: DatasourceKind,
    pub profile: String,
    pub capabilities: BTreeMap<String, String>,
    pub containers: FeedContainers,
    pub items: FeedItems,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filters: Option<BTreeMap<String, BTreeMap<String, String>>>,
}

/// `connectors_definitions.definition.Operation`. `op` is absent only for a
/// `download`. The fields from `required` to `custody`, and
/// `rate_limit_headers`, reach the engine as written, and each takes its value
/// from `defaults` when not set. `query_fixed`, `query_folds` and
/// `embedded_text` reach it as written and have no default.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Operation {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub op: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub effect: Option<Effect>,
    /// A style from `paging`, for an operation more than one style matches.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub paging: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub required: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub withhold: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub credential: Option<Vec<String>>,
    /// Bounds beside those the paging style gives, each replacing the style's
    /// bound of the same parameter.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bounds: Option<IndexMap<String, Bound>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response: Option<ResponseKind>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub binary: Option<Binary>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub download: Option<Download>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub envelope: Option<Envelope>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rate_limit_reasons: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub refusal_texts: Option<Vec<TextRefusal>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub custody: Option<Custody>,
    /// The closed body of a write, by key, a top-level name or a JSON
    /// pointer: `string`, `integer`, `boolean`, `string_list` or `any`; a
    /// `string` followed by `+` must not be empty, and a trailing `!` requires
    /// the key; or `=<value>`, sent fixed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub body: Option<IndexMap<String, String>>,
    /// The values a `string` key of `body` is held to, the engine's
    /// `body_enums`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub body_enums: Option<IndexMap<String, Vec<String>>>,
    /// Keys of `body` under one parent of which the caller sends at least
    /// one, the engine's `body_any_of`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub body_any_of: Option<Vec<String>>,
    /// The text sent before the caller's value of a `string` key of `body`,
    /// the engine's `body_prefixes`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub body_prefixes: Option<IndexMap<String, String>>,
    /// The status, 409 or 412, by which the provider answers a write whose
    /// target changed since it was read, the engine's `conflict_status`.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "status_or_beyond"
    )]
    pub conflict_status: Option<u16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub guard: Option<Guard>,
    /// Query parameters of a read sent with these values on every request,
    /// which a caller cannot send: the engine's `query_fixed`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub query_fixed: Option<IndexMap<String, String>>,
    /// Caller inputs, by name, folded into a fixed query value: the engine's
    /// `query_folds`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub query_folds: Option<IndexMap<String, QueryFold>>,
    /// A read answering JSON whose answer carries a file as base64, answered
    /// as its text: the engine's `embedded_text`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub embedded_text: Option<EmbeddedText>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rate_limit_headers: Option<RateLimitHeaders>,
    /// Written only where it differs from the source operation's summary,
    /// which is the description otherwise.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

/// `connectors_definitions.definition.ConnectorDefinition`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProviderDefinition {
    pub schema: SchemaId,
    /// The connector id, when it is not the file stem: `calendar` for
    /// `google-calendar.yaml`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    /// The catalog provider, the bundle's name, when it is not the file stem:
    /// a second selection over one bundle.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub catalog: Option<String>,
    pub source: Source,
    pub base_url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub config: Option<IndexMap<String, ConfigField>>,
    pub auth: IndexMap<String, Auth>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub paging: Option<IndexMap<String, PagingStyle>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub defaults: Option<Defaults>,
    pub operations: IndexMap<String, Operation>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub datasources: Option<IndexMap<String, Datasource>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub references: Option<IndexMap<String, Reference>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub views: Option<IndexMap<String, View>>,
}

/// `connectors_definitions.references.Match`: exactly one of `url`, an RFC 6570
/// URI template matched in reverse (`{+name}` may span `/`), or `pattern`, a
/// regular expression whose named groups are the reference's values.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReferenceMatch {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pattern: Option<String>,
}

/// `connectors_definitions.references.Resolve`: the read operation, by key, and
/// the template each of its parameters takes.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReferenceResolve {
    pub operation: String,
    pub with: IndexMap<String, String>,
}

/// `connectors_definitions.references.Reference` (`references/v1alpha1`).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Reference {
    #[serde(rename = "match")]
    pub matches: Vec<ReferenceMatch>,
    pub resolve: ReferenceResolve,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub outputs: Option<IndexMap<String, String>>,
}

/// `connectors_definitions.references.ViewField`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ViewField {
    pub label: String,
    pub pointer: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub renderer: Option<String>,
}

/// `connectors_definitions.references.View`: a field selection of the item a
/// reference kind resolves to.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct View {
    pub of: String,
    pub fields: Vec<ViewField>,
}

/// Why a definition was not lowered. Each case is distinct, and [`Refusal::name`]
/// names it, so a caller reports which rule refused rather than that one did.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Refusal {
    /// The definition file, the vendored source or its amendments cannot be read.
    Unreadable(String),
    /// Not YAML, a missing field, or a value outside the schema's vocabulary.
    Malformed(String),
    /// A field the schema does not name, anywhere in the definition.
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
    /// A profile, after `use`, gives a field its scheme does not take.
    AuthMisplaced { key: String, field: String },
    /// A profile, after `use`, gives an `evidence_lifetime_ms` outside the
    /// catalog provider's 1 to [`EVIDENCE_LIFETIME_MS`].
    AuthLifetime { key: String, value: u64 },
    /// A `{variable}` in `base_url` or an OAuth URL that `config` does not declare.
    UndeclaredVariable(String),
    /// Profile ids that do not all start with one provider id and a `.`.
    ProviderId(String),
    /// A source that names other than one document, or a projection where
    /// none is taken, or none where one is; or a document of another kind.
    SourceKind(String),
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
    /// An operation with neither an `op` nor a `download`.
    NoOperation { key: String },
    /// An operation whose method is not `get` and that declares no `effect`.
    EffectRequired {
        key: String,
        op: String,
        method: String,
    },
    /// An operation that declares `effect: write` but is not a POST, PUT,
    /// PATCH or DELETE of the source, or has no `op` at all (`method` is empty).
    WriteMethod {
        key: String,
        op: String,
        method: String,
    },
    /// An operation names a `paging` style the definition does not declare.
    UnknownPaging { key: String, style: String },
    /// More than one style matches the operation, and the operation names
    /// none of them.
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
    /// A body key whose spec is not `string`, `integer`, `boolean`,
    /// `string_list` or `any` (a `string` optionally followed by `+`, each
    /// optionally ending in `!`), nor `=<value>`.
    BodyType {
        key: String,
        field: String,
        spec: String,
    },
    /// A `body_enums` entry other than distinct non-empty values of a key
    /// `body` declares as a `string` the caller sends.
    BodyEnum { key: String, field: String },
    /// A `body_any_of` other than at least two distinct keys under one parent
    /// that `body` declares and the caller sends.
    BodyAnyOf { key: String, detail: String },
    /// A `body_prefixes` entry other than a non-empty prefix of a key `body`
    /// declares as a `string` the caller sends.
    BodyPrefix { key: String, field: String },
    /// A `conflict_status` on a read, or other than 409 or 412.
    ConflictStatus { key: String, status: u16 },
    /// A datasource names an operation key the definition does not declare.
    UnknownDatasourceOperation { datasource: String, key: String },
    /// A datasource operation lacks what the feed derives from it: a single
    /// path parameter, or a paging style with `limit`.
    DatasourceOperation {
        datasource: String,
        key: String,
        detail: String,
    },
    /// A feed datasource breaks a rule the definitions schema states only in
    /// its domain summary; [`Refusal::name`] is the rule's name, the detail
    /// what broke it.
    Feed {
        datasource: String,
        rule: FeedRule,
        detail: String,
    },
    /// More feed datasources than the catalog provider's selection carries (one).
    FeedCount(usize),
    /// A filter sets a parameter `items.list` does not take as a query parameter.
    FilterParameter {
        datasource: String,
        filter: String,
        parameter: String,
    },
    /// A reference kind breaks a `references/v1alpha1` rule; the detail names it.
    Reference { reference: String, detail: String },
    /// A view names a reference kind the definition does not declare.
    ViewReference { view: String, of: String },
    /// A `catalog` or file stem that is not a plain name: empty, `.` or `..`,
    /// or holding `/`, `\` or NUL. It names the directory under the output
    /// directory the declaration and its selection are written to.
    NotPlainName { field: String, value: String },
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
            Self::AuthMisplaced { .. } => "auth_misplaced",
            Self::AuthLifetime { .. } => "auth_lifetime",
            Self::UndeclaredVariable(_) => "undeclared_variable",
            Self::ProviderId(_) => "provider_id",
            Self::SourceKind(_) => "source_kind",
            Self::SourceOutsideRoot(_) => "source_outside_root",
            Self::SourceNotGzip(_) => "source_not_gzip",
            Self::SourceTooLarge(_) => "source_too_large",
            Self::SourceHashMismatch { .. } => "source_hash_mismatch",
            Self::UnknownOperation { .. } => "unknown_operation",
            Self::NoOperation { .. } => "no_operation",
            Self::EffectRequired { .. } => "effect_required",
            Self::WriteMethod { .. } => "write_method",
            Self::UnknownPaging { .. } => "unknown_paging",
            Self::AmbiguousPaging { .. } => "ambiguous_paging",
            Self::PagingParameter { .. } => "paging_parameter",
            Self::UnknownGuardRead { .. } => "unknown_guard_read",
            Self::GuardParameter { .. } => "guard_parameter",
            Self::BodyType { .. } => "body_type",
            Self::BodyEnum { .. } => "body_enum",
            Self::BodyAnyOf { .. } => "body_any_of",
            Self::BodyPrefix { .. } => "body_prefix",
            Self::ConflictStatus { .. } => "conflict_status",
            Self::UnknownDatasourceOperation { .. } => "unknown_datasource_operation",
            Self::DatasourceOperation { .. } => "datasource_operation",
            Self::Feed { rule, .. } => rule.name(),
            Self::FeedCount(_) => "feed_count",
            Self::FilterParameter { .. } => "filter_parameter",
            Self::Reference { .. } => "reference",
            Self::ViewReference { .. } => "view_reference",
            Self::NotPlainName { .. } => "not_plain_name",
            Self::Pipeline(_) => "pipeline",
            Self::Write(_) => "write",
        }
    }
}

/// A feed datasource rule the definitions schema states only in prose (its
/// domain summary, and for [`FeedRule::PositionParameter`] the descriptions of
/// the fields), so the reader enforces it. [`FeedRule::name`] is the refusal's
/// name.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FeedRule {
    /// A cursor names exactly one of `next`, `last` and `offset`.
    CursorContinuation,
    /// `url_parameter` appears only beside `next`.
    CursorUrlParameter,
    /// A position names exactly one of `time` and `query`.
    PositionFilter,
    /// A query's `first` holds `{container}` once and `{since}` never: the
    /// first read has no time to fill in, and the engine loads no such feed.
    QueryFirst,
    /// A query's `resume` holds `{container}` and `{since}` exactly once each.
    QueryResume,
    /// A query position is at `minute` precision.
    QueryPrecision,
    /// Items with a query position name no container parameter: their list
    /// operation has no path parameter.
    QueryContainer,
    /// `utc_offsets` lies within -1439..1439, and `least` is not above `greatest`.
    UtcOffsets,
    /// `utc_offsets` needs `page`.
    UtcOffsetsPage,
    /// `url`, `url_base` and `parent` are JSON pointers.
    ItemPointer,
    /// The parameters a position names, `time`, `query.parameter` and
    /// `page.parameter`, are query parameters of the items' list operation,
    /// which the engine binds them to.
    PositionParameter,
}

impl FeedRule {
    pub fn name(self) -> &'static str {
        match self {
            Self::CursorContinuation => "feed_cursor_continuation",
            Self::CursorUrlParameter => "feed_cursor_url_parameter",
            Self::PositionFilter => "feed_position_filter",
            Self::QueryFirst => "feed_query_first",
            Self::QueryResume => "feed_query_resume",
            Self::QueryPrecision => "feed_query_precision",
            Self::QueryContainer => "feed_query_container",
            Self::UtcOffsets => "feed_utc_offsets",
            Self::UtcOffsetsPage => "feed_utc_offsets_page",
            Self::ItemPointer => "feed_item_pointer",
            Self::PositionParameter => "feed_position_parameter",
        }
    }

    /// The rule as the refusal states it.
    pub fn statement(self) -> &'static str {
        match self {
            Self::CursorContinuation => "a cursor names exactly one of next, last and offset",
            Self::CursorUrlParameter => "url_parameter appears only beside next",
            Self::PositionFilter => "a position names exactly one of time and query",
            Self::QueryFirst => "first holds {container} once and {since} never",
            Self::QueryResume => "resume holds {container} and {since} exactly once each",
            Self::QueryPrecision => "a query position is at minute precision",
            Self::QueryContainer => "items with a query position name no container parameter",
            Self::UtcOffsets => {
                "utc_offsets lies within -1439..1439 and least is not above greatest"
            }
            Self::UtcOffsetsPage => "utc_offsets needs page",
            Self::ItemPointer => "url, url_base and parent are JSON pointers",
            Self::PositionParameter => {
                "the parameters a position names are query parameters of items.list"
            }
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
            | Self::SourceKind(detail)
            | Self::SourceOutsideRoot(detail)
            | Self::SourceNotGzip(detail)
            | Self::SourceTooLarge(detail)
            | Self::AuthLibrary(detail)
            | Self::ProviderId(detail)
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
            Self::AuthMisplaced { key, field } => {
                write!(
                    f,
                    "{name}: auth profile `{key}` gives `{field}`, which its scheme does not take"
                )
            }
            Self::AuthLifetime { key, value } => write!(
                f,
                "{name}: auth profile `{key}` gives evidence_lifetime_ms {value}; the catalog provider takes 1 to {EVIDENCE_LIFETIME_MS}"
            ),
            Self::UndeclaredVariable(variable) => write!(
                f,
                "{name}: URL variable `{variable}` is not declared in config"
            ),
            Self::SourceHashMismatch { expected, actual } => write!(
                f,
                "{name}: source.sha256 is {expected}, the decompressed source hashes to {actual}"
            ),
            Self::UnknownOperation { key, op } => write!(
                f,
                "{name}: operation `{key}` names op `{op}`, which the bundle inventory lacks"
            ),
            Self::NoOperation { key } => write!(
                f,
                "{name}: operation `{key}` names no op and declares no download"
            ),
            Self::EffectRequired { key, op, method } => write!(
                f,
                "{name}: operation `{key}` (`{op}`) is {} and declares no effect",
                method.to_uppercase()
            ),
            Self::WriteMethod { key, method, .. } if method.is_empty() => write!(
                f,
                "{name}: operation `{key}` declares effect write but names no op, so it sends no write request"
            ),
            Self::WriteMethod { key, op, method } => write!(
                f,
                "{name}: operation `{key}` (`{op}`) declares effect write but is {}, not POST, PUT, PATCH or DELETE",
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
                "{name}: operation `{key}` body `{field}` is `{spec}`; expected string, integer, boolean, string_list or any, a string optionally with `+`, each optionally with `!`, or `=<value>`"
            ),
            Self::BodyEnum { key, field } => write!(
                f,
                "{name}: operation `{key}` body_enums `{field}` is not distinct non-empty values of a string key its body declares and the caller sends"
            ),
            Self::BodyAnyOf { key, detail } => write!(
                f,
                "{name}: operation `{key}` body_any_of {detail}; expected at least two distinct keys under one parent that its body declares and the caller sends"
            ),
            Self::BodyPrefix { key, field } => write!(
                f,
                "{name}: operation `{key}` body_prefixes `{field}` is not a non-empty prefix of a string key its body declares and the caller sends"
            ),
            Self::ConflictStatus { key, status } => write!(
                f,
                "{name}: operation `{key}` declares conflict_status {status}; a write declares 409 or 412, a read none"
            ),
            Self::UnknownDatasourceOperation { datasource, key } => write!(
                f,
                "{name}: datasource `{datasource}` names operation `{key}`, which operations does not declare"
            ),
            Self::DatasourceOperation {
                datasource,
                key,
                detail,
            } => write!(
                f,
                "{name}: datasource `{datasource}` operation `{key}`: {detail}"
            ),
            Self::Feed {
                datasource,
                rule,
                detail,
            } => write!(
                f,
                "{name}: datasource `{datasource}`: {detail}; {}",
                rule.statement()
            ),
            Self::FeedCount(count) => write!(
                f,
                "{name}: {count} feed datasources; the catalog provider's selection carries one"
            ),
            Self::FilterParameter {
                datasource,
                filter,
                parameter,
            } => write!(
                f,
                "{name}: datasource `{datasource}` filter `{filter}` sets `{parameter}`, which items.list does not take"
            ),
            Self::Reference { reference, detail } => {
                write!(f, "{name}: reference `{reference}`: {detail}")
            }
            Self::ViewReference { view, of } => write!(
                f,
                "{name}: view `{view}` is of `{of}`, which references does not declare"
            ),
            Self::NotPlainName { field, value } => write!(
                f,
                "{name}: {field} `{}` is not a plain name; it names a directory under the output directory",
                value.escape_debug()
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
    if let Some(catalog) = &definition.catalog {
        plain_name("catalog", catalog)?;
        if !connectors_core::valid_id(catalog) {
            return Err(Refusal::Malformed(format!(
                "catalog `{catalog}` is not a provider id"
            )));
        }
    }
    Ok(definition)
}

/// What to lower, from where, into where.
#[derive(Clone, Copy, Debug)]
pub struct Request<'a> {
    /// `connectors/<id>.yaml`; the file stem names the selection and the auth
    /// file, and the catalog provider unless `catalog` names another.
    pub definition: &'a Path,
    /// The definitions checkout the source paths and `auth/` are under.
    pub root: &'a Path,
    /// The bundle directory the bundle and its index entry go to; the
    /// declaration and its selection go to `<catalog>/` under it, as the
    /// catalog keeps them in `providers/<catalog>/`.
    pub out: &'a Path,
    /// An existing directory owned by the caller; the decompressed source and
    /// the staged bundle live in a fresh directory under it, removed on return.
    pub scratch: &'a Path,
}

/// What a completed lowering wrote.
#[derive(Clone, Debug)]
pub struct Lowered {
    /// The catalog provider: the bundle's name and the selection's `provider`.
    pub provider: String,
    pub entry: bundle::Entry,
    pub bundle: PathBuf,
    /// `<catalog>/operations.json`, or `<catalog>/<x>-operations.json` for a
    /// definition `<catalog>-<x>`: the file the declaration names.
    pub operations: PathBuf,
    /// `<catalog>/connector.json`: the connector declaration.
    pub connector: PathBuf,
}

/// One row of the selection, in the shape the catalog provider's `Selection`
/// reads.
#[derive(Clone, Debug, Serialize)]
pub struct SelectionEntry {
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub operation_id: Option<String>,
    pub effect: Effect,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response: Option<ResponseKind>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub binary: Option<Binary>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub download: Option<Download>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub envelope: Option<Envelope>,
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub bounds: BTreeMap<String, Bound>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub required: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub withhold: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub credential: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub rate_limit_reasons: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub refusal_texts: Vec<TextRefusal>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub body_keys: Vec<String>,
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub body_types: BTreeMap<String, String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub body_required: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub body_nonempty: Vec<String>,
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub body_fixed: BTreeMap<String, Value>,
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub body_enums: BTreeMap<String, Vec<String>>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub body_any_of: Vec<String>,
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub body_prefixes: BTreeMap<String, String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub conflict_status: Option<u16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub guard: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub custody: Option<Custody>,
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub query_fixed: BTreeMap<String, String>,
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub query_folds: BTreeMap<String, QueryFold>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub embedded_text: Option<EmbeddedText>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rate_limit_headers: Option<RateLimitHeaders>,
}

/// The `connectors-catalog-operations/1` file a definition lowers to.
#[derive(Clone, Debug, Serialize)]
pub struct OperationsFile {
    pub format: &'static str,
    pub provider: String,
    pub operations: Vec<SelectionEntry>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub feed: Option<Value>,
}

/// The closed body a definition declares, in the selection's `body_*` fields.
#[derive(Default)]
struct Body {
    keys: Vec<String>,
    types: BTreeMap<String, String>,
    required: Vec<String>,
    nonempty: Vec<String>,
    fixed: BTreeMap<String, Value>,
    enums: BTreeMap<String, Vec<String>>,
    any_of: Vec<String>,
    prefixes: BTreeMap<String, String>,
}

impl Body {
    /// Whether `field` is a `string` the caller sends: declared so, not fixed.
    fn sent_string(&self, field: &str) -> bool {
        let Some(path) = path(field) else {
            return false;
        };
        self.keys.iter().any(|declared| {
            path_eq(declared, &path)
                && self
                    .types
                    .get(declared)
                    .is_some_and(|kind| kind == "string")
                && !self.fixed.contains_key(declared)
        })
    }
}

/// The segments a body key names, by the engine's rules: a plain name is one
/// segment, a pointer its segments, none empty and none holding `~` or `.`.
fn path(key: &str) -> Option<Vec<&str>> {
    match key.strip_prefix('/') {
        None => (!key.is_empty() && !key.contains('.')).then(|| vec![key]),
        Some(rest) => {
            let segments: Vec<&str> = rest.split('/').collect();
            segments
                .iter()
                .all(|segment| !segment.is_empty() && !segment.contains(['~', '.']))
                .then_some(segments)
        }
    }
}

fn path_eq(key: &str, path: &[&str]) -> bool {
    self::path(key).is_some_and(|own| own == path)
}

/// The parent of a body key: everything before a pointer's last `/`, the top
/// level for a plain name.
fn parent(field: &str) -> &str {
    field.rfind('/').map_or("", |at| &field[..at])
}

fn body(key: &str, operation: &Operation) -> Result<Body, Refusal> {
    let mut body = body_keys(key, operation.body.as_ref())?;
    if let Some(field) = body.keys.iter().find(|field| path(field).is_none()) {
        return Err(Refusal::BodyType {
            key: key.to_owned(),
            field: field.clone(),
            spec: "a key the engine reads as a path: a plain name without `.`, or a pointer"
                .to_owned(),
        });
    }
    let mut paths = BTreeSet::new();
    for field in &body.keys {
        if !paths.insert(path(field)) {
            return Err(Refusal::BodyType {
                key: key.to_owned(),
                field: field.clone(),
                spec: "a path no other body key names (`x` and `/x` are one path)".to_owned(),
            });
        }
    }
    let all: Vec<_> = body.keys.iter().filter_map(|field| path(field)).collect();
    for field in &body.keys {
        let own = path(field).unwrap_or_default();
        if all
            .iter()
            .any(|other| other.len() < own.len() && own.starts_with(other))
        {
            return Err(Refusal::BodyType {
                key: key.to_owned(),
                field: field.clone(),
                spec: "a path no other body key names or lies inside".to_owned(),
            });
        }
    }
    for (field, values) in operation.body_enums.iter().flatten() {
        let mut seen = BTreeSet::new();
        if !body.sent_string(field)
            || values.is_empty()
            || !values
                .iter()
                .all(|value| !value.is_empty() && seen.insert(value))
        {
            return Err(Refusal::BodyEnum {
                key: key.to_owned(),
                field: field.clone(),
            });
        }
        body.enums.insert(field.clone(), values.clone());
    }
    if let Some(any_of) = &operation.body_any_of {
        let refuse = |detail: String| Refusal::BodyAnyOf {
            key: key.to_owned(),
            detail,
        };
        let mut seen = BTreeSet::new();
        for field in any_of {
            if !body.keys.contains(field) || body.fixed.contains_key(field) {
                return Err(refuse(format!(
                    "names `{field}`, which body does not declare for the caller"
                )));
            }
            if !seen.insert(field) {
                return Err(refuse(format!("names `{field}` twice")));
            }
        }
        match any_of.len() {
            0 => return Err(refuse("names no key".to_owned())),
            1 => return Err(refuse("names one key".to_owned())),
            _ => {}
        }
        if any_of
            .iter()
            .any(|field| parent(field) != parent(&any_of[0]))
        {
            return Err(refuse("names keys under more than one parent".to_owned()));
        }
        body.any_of = any_of.clone();
    }
    for (field, prefix) in operation.body_prefixes.iter().flatten() {
        if !body.sent_string(field) || prefix.is_empty() {
            return Err(Refusal::BodyPrefix {
                key: key.to_owned(),
                field: field.clone(),
            });
        }
        body.prefixes.insert(field.clone(), prefix.clone());
    }
    Ok(body)
}

fn body_keys(key: &str, declared: Option<&IndexMap<String, String>>) -> Result<Body, Refusal> {
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
        let refused = || Refusal::BodyType {
            key: key.to_owned(),
            field: field.clone(),
            spec: spec.clone(),
        };
        let (rest, required) = match spec.strip_suffix('!') {
            Some(rest) => (rest, true),
            None => (spec.as_str(), false),
        };
        let (kind, nonempty) = match rest.strip_suffix('+') {
            Some(kind) => (kind, true),
            None => (rest, false),
        };
        match kind {
            "string" | "integer" | "boolean" | "string_list" => {
                body.types.insert(field.clone(), kind.to_owned());
            }
            "any" => {}
            _ => return Err(refused()),
        }
        if nonempty {
            if kind != "string" {
                return Err(refused());
            }
            body.nonempty.push(field.clone());
        }
        if required {
            body.required.push(field.clone());
        }
    }
    Ok(body)
}

/// One `expect` entry as the engine's check: `=true`/`=false` a boolean, any
/// other `=…` a literal, `?` a presence (the member exists and is not `null`),
/// else an input reference.
fn checks(mapping: Option<&IndexMap<String, String>>) -> Vec<Value> {
    mapping
        .into_iter()
        .flatten()
        .map(|(pointer, expected)| {
            let expect = match expected.strip_prefix('=') {
                Some("true") => serde_json::json!({"boolean": true}),
                Some("false") => serde_json::json!({"boolean": false}),
                Some(literal) => serde_json::json!({"literal": literal}),
                None if expected == "?" => serde_json::json!({"present": true}),
                None => serde_json::json!({"input": expected}),
            };
            serde_json::json!({"pointer": pointer, "expect": expect})
        })
        .collect()
}

/// The query parameter names of a source operation.
fn query_of(found: &crate::inventory::Operation) -> Vec<&str> {
    found
        .parameters
        .iter()
        .filter(|parameter| parameter.location == crate::inventory::Location::Query)
        .map(|parameter| parameter.name.as_str())
        .collect()
}

/// The paging style an operation uses, by name, given its query parameters and
/// what it withholds: the one it names, else the one style that matches it,
/// else none.
fn paging_style<'a>(
    key: &str,
    operation: &Operation,
    styles: Option<&'a IndexMap<String, PagingStyle>>,
    query: &[&str],
    withheld: &[String],
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
            let parameter = style.request.as_deref().or(style.limit.as_deref());
            parameter.is_some_and(|parameter| {
                query.contains(&parameter) && !withheld.iter().any(|w| w == parameter)
            })
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

/// The `{name}` variables of a URL template, in order.
fn variables(template: &str) -> Result<Vec<&str>, Refusal> {
    let malformed = || Refusal::Malformed(format!("`{template}` is not a URL template"));
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
pub fn summary(document: &Value, operation_id: &str) -> Option<String> {
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
pub fn merged(base: &Auth, over: &Auth) -> Auth {
    let identity = match (&base.identity, &over.identity) {
        (Some(base), Some(over)) => Some(AuthIdentity {
            source: over.source.or(base.source),
            path: over.path.clone().or_else(|| base.path.clone()),
            kind: over.kind.clone().or_else(|| base.kind.clone()),
            subject: over.subject.clone().or_else(|| base.subject.clone()),
        }),
        (base, over) => over.clone().or_else(|| base.clone()),
    };
    let pick = |over: &Option<String>, base: &Option<String>| over.clone().or_else(|| base.clone());
    Auth {
        library: None,
        default: over.default.or(base.default),
        scheme: over.scheme.or(base.scheme),
        header: pick(&over.header, &base.header),
        prefix: pick(&over.prefix, &base.prefix),
        label: pick(&over.label, &base.label),
        account_label: pick(&over.account_label, &base.account_label),
        identity,
        scopes: over.scopes.clone().or_else(|| base.scopes.clone()),
        access: over.access.clone().or_else(|| base.access.clone()),
        evidence_lifetime_ms: over.evidence_lifetime_ms.or(base.evidence_lifetime_ms),
        token_url: pick(&over.token_url, &base.token_url),
        authorize_url: pick(&over.authorize_url, &base.authorize_url),
    }
}

/// Every profile of the definition with its `use` resolved, and the one the
/// bundle records: the only one, or the one marked `default: true`.
fn resolve_auth(
    definition: &ProviderDefinition,
    root: &Path,
) -> Result<(String, IndexMap<String, Auth>), Refusal> {
    let mut resolved = IndexMap::new();
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
                        "auth/{library}.yaml profile `{id}` carries `use` or `default`, which only a connector's profile may"
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

/// A value of a connector declaration, written as the committed declarations
/// write it: an object on one line, a list of more than one string one string
/// per line.
enum Node {
    Text(String),
    Flag(bool),
    Number(u64),
    Inline(Fields),
    List(Vec<String>),
}

type Fields = Vec<(String, Node)>;

fn text(value: &str) -> Node {
    Node::Text(value.to_owned())
}

fn quoted(value: &str) -> String {
    Value::from(value).to_string()
}

impl Node {
    fn inline(&self) -> String {
        match self {
            Self::Text(value) => quoted(value),
            Self::Flag(value) => value.to_string(),
            Self::Number(value) => value.to_string(),
            Self::Inline(fields) => {
                let pairs: Vec<String> = fields
                    .iter()
                    .map(|(key, node)| format!("{}: {}", quoted(key), node.inline()))
                    .collect();
                format!("{{{}}}", pairs.join(", "))
            }
            Self::List(items) => {
                let items: Vec<String> = items.iter().map(|item| quoted(item)).collect();
                format!("[{}]", items.join(", "))
            }
        }
    }

    /// The value as it stands in an object indented by `indent`.
    fn written(&self, indent: usize) -> String {
        match self {
            Self::List(items) if items.len() > 1 => {
                let pad = " ".repeat(indent + 2);
                let lines: Vec<String> = items
                    .iter()
                    .map(|item| format!("{pad}{}", quoted(item)))
                    .collect();
                format!("[\n{}\n{}]", lines.join(",\n"), " ".repeat(indent))
            }
            _ => self.inline(),
        }
    }
}

/// `fields` as an object whose closing brace is indented by `indent`.
fn object(fields: &Fields, indent: usize) -> String {
    let pad = " ".repeat(indent + 2);
    let lines: Vec<String> = fields
        .iter()
        .map(|(key, node)| format!("{pad}{}: {}", quoted(key), node.written(indent + 2)))
        .collect();
    format!("{{\n{}\n{}}}", lines.join(",\n"), " ".repeat(indent))
}

fn field(key: &str, node: Node) -> (String, Node) {
    (key.to_owned(), node)
}

/// `url` as its path when it lies on `base_url`'s origin, which the catalog
/// provider resolves against the instance endpoint; otherwise as written.
fn on_origin(url: &str, base_url: &str) -> String {
    let own = origin(url);
    url.strip_prefix(own)
        .filter(|path| own == origin(base_url) && path.starts_with('/'))
        .unwrap_or(url)
        .to_owned()
}

/// One resolved profile as a declaration's profile block, its fields in the
/// order of the catalog provider's `AuthConfig`, the identity's `source` first.
fn profile_block(id: &str, profile: &Auth, base_url: &str) -> Result<Fields, Refusal> {
    let missing = |field: &str| Refusal::AuthIncomplete {
        key: id.to_owned(),
        field: field.to_owned(),
    };
    let misplaced = |field: &str| Refusal::AuthMisplaced {
        key: id.to_owned(),
        field: field.to_owned(),
    };
    let scheme = profile.scheme.ok_or_else(|| missing("scheme"))?;
    let label = profile.label.as_ref().ok_or_else(|| missing("label"))?;
    let identity = profile
        .identity
        .as_ref()
        .ok_or_else(|| missing("identity"))?;
    let source = identity.source.unwrap_or(IdentitySource::Api);
    let mut probe = Fields::new();
    match source {
        IdentitySource::Api => {}
        IdentitySource::IdToken => probe.push(field("source", text("id_token"))),
        IdentitySource::Configuration => probe.push(field("source", text("configuration"))),
    }
    match (&identity.path, source) {
        (Some(path), IdentitySource::Api | IdentitySource::Configuration) => {
            probe.push(field("path", text(path)));
        }
        (None, IdentitySource::Api | IdentitySource::Configuration) => {
            return Err(missing("identity.path"));
        }
        (Some(_), IdentitySource::IdToken) => return Err(misplaced("identity.path")),
        (None, IdentitySource::IdToken) => {}
    }
    let kind = identity
        .kind
        .as_ref()
        .ok_or_else(|| missing("identity.kind"))?;
    probe.push(field("kind", text(kind)));
    if let Some(subject) = &identity.subject {
        probe.push(field("subject_pointer", text(subject)));
    }

    let mut block = vec![field("profile", text(id))];
    let word = match scheme {
        AuthScheme::Basic => Some("basic"),
        AuthScheme::OAuth2Refresh => Some("oauth2_refresh"),
        AuthScheme::OAuth2ClientCredentials => Some("oauth2_client_credentials"),
        AuthScheme::Bearer | AuthScheme::Header => None,
    };
    if let Some(word) = word {
        block.push(field("scheme", text(word)));
    }
    let header = match (&profile.header, scheme) {
        (Some(header), _) => header.clone(),
        (None, AuthScheme::Header) => return Err(missing("header")),
        (None, _) => "Authorization".to_owned(),
    };
    block.push(field("header", Node::Text(header)));
    block.push(field(
        "bearer",
        Node::Flag(!matches!(scheme, AuthScheme::Basic | AuthScheme::Header)),
    ));
    match (&profile.prefix, scheme) {
        (Some(prefix), AuthScheme::Header) => block.push(field("prefix", text(prefix))),
        (Some(_), _) => return Err(misplaced("prefix")),
        (None, _) => {}
    }
    match (&profile.account_label, scheme) {
        (Some(account), AuthScheme::Basic) => block.push(field("account_label", text(account))),
        (None, AuthScheme::Basic) => return Err(missing("account_label")),
        (Some(_), _) => return Err(misplaced("account_label")),
        (None, _) => {}
    }
    block.push(field("label", text(label)));
    block.push(field("identity", Node::Inline(probe)));
    let scopes = profile.scopes.as_ref();
    if let Some(scopes) = scopes {
        match (&scopes.path, &scopes.pointer) {
            (Some(path), Some(pointer)) => block.push(field(
                "scopes",
                Node::Inline(vec![
                    field("path", text(path)),
                    field("pointer", text(pointer)),
                ]),
            )),
            (None, None) => {}
            (Some(_), None) => return Err(missing("scopes.pointer")),
            (None, Some(_)) => return Err(missing("scopes.path")),
        }
    }
    if let Some(access) = &profile.access {
        let mut read = vec![field("path", text(&access.path))];
        if let Some(query) = &access.query {
            read.push(field(
                "query",
                Node::Inline(
                    query
                        .iter()
                        .map(|(key, value)| field(key, text(value)))
                        .collect(),
                ),
            ));
        }
        block.push(field("access", Node::Inline(read)));
    }
    if let Some(minimum) = scopes.and_then(|scopes| scopes.minimum.as_ref()) {
        block.push(field("minimum_scopes", Node::List(minimum.clone())));
    }
    if let Some(lifetime) = profile.evidence_lifetime_ms {
        if !(1..=EVIDENCE_LIFETIME_MS).contains(&lifetime) {
            return Err(Refusal::AuthLifetime {
                key: id.to_owned(),
                value: lifetime,
            });
        }
        block.push(field("evidence_lifetime_ms", Node::Number(lifetime)));
    }
    if scheme.is_oauth() {
        let token_url = profile
            .token_url
            .as_ref()
            .ok_or_else(|| missing("token_url"))?;
        block.push(field(
            "token_url",
            Node::Text(on_origin(token_url, base_url)),
        ));
        match (&profile.authorize_url, scheme) {
            (Some(url), AuthScheme::OAuth2Refresh) => block.push(field("authorize_url", text(url))),
            (None, AuthScheme::OAuth2Refresh) => return Err(missing("authorize_url")),
            (Some(_), _) => return Err(misplaced("authorize_url")),
            (None, _) => {}
        }
    } else {
        for (name, given) in [
            ("token_url", &profile.token_url),
            ("authorize_url", &profile.authorize_url),
        ] {
            if given.is_some() {
                return Err(misplaced(name));
            }
        }
    }
    if let Some(requested) = scopes.and_then(|scopes| scopes.requested.as_ref()) {
        if !scheme.is_oauth() {
            return Err(misplaced("scopes.requested"));
        }
        block.push(field("requested_scopes", Node::List(requested.clone())));
    }
    Ok(block)
}

/// The declaration's `default_endpoint` and `api_path`. When the host of
/// `base_url` is one variable, users know the host and not the API under it, so
/// the endpoint is the origin and the path under it is `api_path`. The endpoint
/// is a default only when `config` defaults every variable in it.
fn endpoint(definition: &ProviderDefinition) -> Result<(Option<String>, Option<String>), Refusal> {
    let base = definition.base_url.as_str();
    let (scheme, rest) = base
        .split_once("://")
        .ok_or_else(|| Refusal::Malformed(format!("`{base}` is not an absolute URL")))?;
    let (authority, path) = rest.find('/').map_or((rest, ""), |at| rest.split_at(at));
    let one_variable =
        authority.starts_with('{') && authority.ends_with('}') && variables(authority)?.len() == 1;
    let api_path = (one_variable && !path.is_empty()).then(|| path.to_owned());
    let mut resolved = match api_path {
        Some(_) => format!("{scheme}://{authority}"),
        None => base.to_owned(),
    };
    for variable in variables(&resolved.clone())? {
        let Some(default) = definition
            .config
            .as_ref()
            .and_then(|config| config.get(variable))
            .and_then(|field| field.default.as_ref())
        else {
            return Ok((None, api_path));
        };
        resolved = resolved.replace(&format!("{{{variable}}}"), default);
    }
    Ok((Some(resolved), api_path))
}

/// The name of the selection file in the catalog's provider directory, which
/// the declaration names and the lowering writes beside it: `operations.json`,
/// or `<x>-operations.json` for a definition `<catalog>-<x>` over another
/// catalog, as Slack's `user-operations.json`.
fn selection_file(definition: &ProviderDefinition, name: &str) -> String {
    match &definition.catalog {
        None => "operations.json".to_owned(),
        Some(catalog) => {
            let suffix = name.strip_prefix(&format!("{catalog}-")).unwrap_or(name);
            format!("{suffix}-operations.json")
        }
    }
}

/// The connector declaration a definition lowers to, `use` resolved against
/// `root`, and the profile its bundle records. The provider id is the part of
/// the profile ids before their first `.`, which every profile shares.
fn declaration(
    definition: &ProviderDefinition,
    name: &str,
    provider: &str,
    root: &Path,
) -> Result<(String, String), Refusal> {
    let (default_profile, profiles) = resolve_auth(definition, root)?;
    let mut urls = vec![definition.base_url.as_str()];
    for profile in profiles.values() {
        urls.extend(profile.token_url.as_deref());
        urls.extend(profile.authorize_url.as_deref());
    }
    for url in urls {
        for variable in variables(url)? {
            if !definition
                .config
                .as_ref()
                .is_some_and(|config| config.contains_key(variable))
            {
                return Err(Refusal::UndeclaredVariable(variable.to_owned()));
            }
        }
    }
    let prefixes: std::collections::BTreeSet<&str> = profiles
        .keys()
        .map(|id| id.split_once('.').map_or("", |(prefix, _)| prefix))
        .collect();
    let provider_id = match prefixes.iter().collect::<Vec<_>>()[..] {
        [only] if !only.is_empty() => *only,
        _ => {
            let ids: Vec<&str> = profiles.keys().map(String::as_str).collect();
            return Err(Refusal::ProviderId(format!(
                "profiles {} do not share one `<provider>.` prefix",
                ids.join(", ")
            )));
        }
    };
    let (default_endpoint, api_path) = endpoint(definition)?;
    let selection = selection_file(definition, name);
    let mut head = vec![
        field("format", text(CONNECTOR_FORMAT)),
        field(
            "connector_id",
            text(definition.id.as_deref().unwrap_or(name)),
        ),
        field("provider_id", text(provider_id)),
        field("adapter_id", text("catalog")),
        field("catalog_provider", text(provider)),
    ];
    if let Some(endpoint) = default_endpoint {
        head.push(field("default_endpoint", Node::Text(endpoint)));
    }
    if let Some(path) = api_path {
        head.push(field("api_path", Node::Text(path)));
    }
    head.push(field("selection", Node::Text(selection)));
    let mut blocks = Vec::new();
    for (id, profile) in &profiles {
        blocks.push(format!(
            "    {}",
            object(&profile_block(id, profile, &definition.base_url)?, 4)
        ));
    }
    let mut lines: Vec<String> = head
        .iter()
        .map(|(key, node)| format!("  {}: {}", quoted(key), node.written(2)))
        .collect();
    lines.push(format!("  \"profiles\": [\n{}\n  ]", blocks.join(",\n")));
    Ok((format!("{{\n{}\n}}\n", lines.join(",\n")), default_profile))
}

/// The connector declaration the definition at `file` lowers to, `use`
/// resolved against the definitions `root`. Reads no source.
pub fn connector(file: &Path, root: &Path) -> Result<String, Refusal> {
    let shown = file.display();
    let bytes =
        std::fs::read(file).map_err(|error| Refusal::Unreadable(format!("{shown}: {error}")))?;
    let definition = parse(&bytes)?;
    let name = stem(file)?;
    let provider = definition.catalog.as_deref().unwrap_or(name);
    Ok(declaration(&definition, name, provider, root)?.0)
}

/// The file stem, which names the selection and, without a `catalog`, the
/// directory under the output directory; so it is a plain name.
fn stem(file: &Path) -> Result<&str, Refusal> {
    let stem = file
        .file_stem()
        .and_then(|stem| stem.to_str())
        .ok_or_else(|| Refusal::Unreadable(format!("{}: no file stem", file.display())))?;
    plain_name("file stem", stem)?;
    Ok(stem)
}

/// The source operation an operation key names, with its query parameters.
fn keyed<'a>(
    definition: &'a ProviderDefinition,
    inventory: &'a crate::inventory::Inventory,
    datasource: &str,
    key: &str,
) -> Result<(&'a Operation, &'a crate::inventory::Operation, Vec<&'a str>), Refusal> {
    let unknown = || Refusal::UnknownDatasourceOperation {
        datasource: datasource.to_owned(),
        key: key.to_owned(),
    };
    let operation = definition.operations.get(key).ok_or_else(unknown)?;
    let op = operation.op.as_deref().ok_or_else(unknown)?;
    let found = inventory
        .operations
        .iter()
        .find(|candidate| candidate.operation_id.as_deref() == Some(op))
        .ok_or_else(unknown)?;
    Ok((operation, found, query_of(found)))
}

/// The one path parameter of a datasource operation.
fn sole_path_parameter(
    datasource: &str,
    key: &str,
    found: &crate::inventory::Operation,
) -> Result<String, Refusal> {
    let path: Vec<&str> = found
        .parameters
        .iter()
        .filter(|parameter| parameter.location == crate::inventory::Location::Path)
        .map(|parameter| parameter.name.as_str())
        .collect();
    match path[..] {
        [only] => Ok(only.to_owned()),
        _ => Err(Refusal::DatasourceOperation {
            datasource: datasource.to_owned(),
            key: key.to_owned(),
            detail: format!(
                "the feed needs exactly one path parameter, the operation has {}",
                path.len()
            ),
        }),
    }
}

/// The paging style's page-size parameter and cap for a datasource operation.
/// A style without `max` leaves the cap to the feed's.
fn page_size(
    definition: &ProviderDefinition,
    datasource: &str,
    key: &str,
    operation: &Operation,
    query: &[&str],
) -> Result<(String, Option<u64>), Refusal> {
    let lacking = || Refusal::DatasourceOperation {
        datasource: datasource.to_owned(),
        key: key.to_owned(),
        detail: "the feed needs a paging style with limit".to_owned(),
    };
    let withheld = carried(definition, operation).withhold;
    let (_, style) = paging_style(key, operation, definition.paging.as_ref(), query, &withheld)?
        .ok_or_else(lacking)?;
    match &style.limit {
        Some(limit) => Ok((limit.clone(), style.max)),
        None => Err(lacking()),
    }
}

/// The page-size cap of a feed whose items' paging style names no `max`: the
/// most the catalog provider's feed declaration admits
/// (`adapters/catalog/src/feed.rs`, `max_limit` 1 to 100).
pub const FEED_MAX_LIMIT: u64 = 100;

/// How many times `needle` occurs in `text`, in words.
fn times(text: &str, needle: &str) -> String {
    match text.matches(needle).count() {
        1 => "once".to_owned(),
        count => format!("{count} times"),
    }
}

/// Whether `text` is a JSON pointer (RFC 6901): empty, or `/` and reference
/// tokens in which `~` is followed by `0` or `1`.
fn is_pointer(text: &str) -> bool {
    if text.is_empty() {
        return true;
    }
    let Some(tokens) = text.strip_prefix('/') else {
        return false;
    };
    let mut bytes = tokens.bytes();
    while let Some(byte) = bytes.next() {
        if byte == b'~' && !matches!(bytes.next(), Some(b'0' | b'1')) {
            return false;
        }
    }
    true
}

/// The rules of one feed datasource that its fields alone decide, in the
/// order the schema's domain summary states them; the ones that need the
/// items' operation ([`FeedRule::QueryContainer`] and
/// [`FeedRule::PositionParameter`]) are in [`feed`].
fn feed_rules(name: &str, datasource: &Datasource) -> Result<(), Refusal> {
    let refuse = |rule: FeedRule, detail: String| Refusal::Feed {
        datasource: name.to_owned(),
        rule,
        detail,
    };
    let cursor = &datasource.containers.cursor;
    let named: Vec<&str> = [
        ("next", cursor.next.is_some()),
        ("last", cursor.last.is_some()),
        ("offset", cursor.offset.is_some()),
    ]
    .into_iter()
    .filter_map(|(field, present)| present.then_some(field))
    .collect();
    match named.as_slice() {
        [_] => {}
        [] => {
            return Err(refuse(
                FeedRule::CursorContinuation,
                "containers.cursor names none of next, last and offset".to_owned(),
            ));
        }
        [head @ .., tail] => {
            return Err(refuse(
                FeedRule::CursorContinuation,
                format!("containers.cursor names {} and {tail}", head.join(", ")),
            ));
        }
    }
    if let Some(parameter) = &cursor.url_parameter
        && cursor.next.is_none()
    {
        return Err(refuse(
            FeedRule::CursorUrlParameter,
            format!(
                "containers.cursor gives url_parameter `{parameter}` beside {}, not next",
                named[0]
            ),
        ));
    }
    let position = &datasource.items.position;
    let query = match (&position.time, &position.query) {
        (Some(_), None) => None,
        (None, Some(query)) => Some(query),
        (None, None) => {
            return Err(refuse(
                FeedRule::PositionFilter,
                "items.position names neither time nor query".to_owned(),
            ));
        }
        (Some(_), Some(_)) => {
            return Err(refuse(
                FeedRule::PositionFilter,
                "items.position names both time and query".to_owned(),
            ));
        }
    };
    if let Some(query) = query {
        if query.first.matches("{container}").count() != 1 || query.first.contains("{since}") {
            return Err(refuse(
                FeedRule::QueryFirst,
                format!(
                    "items.position.query.first holds {{container}} {} and {{since}} {}",
                    times(&query.first, "{container}"),
                    times(&query.first, "{since}")
                ),
            ));
        }
        if query.resume.matches("{container}").count() != 1
            || query.resume.matches("{since}").count() != 1
        {
            return Err(refuse(
                FeedRule::QueryResume,
                format!(
                    "items.position.query.resume holds {{container}} {} and {{since}} {}",
                    times(&query.resume, "{container}"),
                    times(&query.resume, "{since}")
                ),
            ));
        }
        match position.precision {
            Some(FeedPrecision::Minute) => {}
            Some(other) => {
                return Err(refuse(
                    FeedRule::QueryPrecision,
                    format!("items.position has a query at precision {}", other.word()),
                ));
            }
            None => {
                return Err(refuse(
                    FeedRule::QueryPrecision,
                    "items.position has a query and no precision".to_owned(),
                ));
            }
        }
        if let Some(offsets) = &query.utc_offsets {
            for (bound, value) in [("least", offsets.least), ("greatest", offsets.greatest)] {
                if !(-1439..=1439).contains(&value) {
                    return Err(refuse(
                        FeedRule::UtcOffsets,
                        format!(
                            "items.position.query.utc_offsets {bound} {value} is outside -1439..1439"
                        ),
                    ));
                }
            }
            if offsets.least > offsets.greatest {
                return Err(refuse(
                    FeedRule::UtcOffsets,
                    format!(
                        "items.position.query.utc_offsets least {} is above greatest {}",
                        offsets.least, offsets.greatest
                    ),
                ));
            }
            if position.page.is_none() {
                return Err(refuse(
                    FeedRule::UtcOffsetsPage,
                    "items.position gives query.utc_offsets and no page".to_owned(),
                ));
            }
        }
    }
    let items = &datasource.items;
    for (field, value) in [
        ("url", &items.url),
        ("url_base", &items.url_base),
        ("parent", &items.parent),
    ] {
        if let Some(value) = value
            && !is_pointer(value)
        {
            return Err(refuse(
                FeedRule::ItemPointer,
                format!("items.{field} `{value}` is not a JSON pointer"),
            ));
        }
    }
    Ok(())
}

/// The feed a definition's datasource lowers to, in the catalog provider's
/// `datasource.feed/v1alpha1` declaration shape. Operation ids, page-size
/// parameters, the cap (the items' style's `max`, else [`FEED_MAX_LIMIT`]),
/// the lookup parameter and, for a `time` position, the items' container
/// parameter are derived; the rest is carried as written. The rules the schema
/// states only in prose are enforced first ([`FeedRule`]).
fn feed(
    definition: &ProviderDefinition,
    inventory: &crate::inventory::Inventory,
) -> Result<Option<Value>, Refusal> {
    let Some(datasources) = &definition.datasources else {
        return Ok(None);
    };
    let [(name, datasource)] = datasources.iter().collect::<Vec<_>>()[..] else {
        return if datasources.is_empty() {
            Ok(None)
        } else {
            Err(Refusal::FeedCount(datasources.len()))
        };
    };
    feed_rules(name, datasource)?;
    let containers = &datasource.containers;
    let items = &datasource.items;
    let (list_operation, list, list_query) = keyed(definition, inventory, name, &containers.list)?;
    let (_, lookup, _) = keyed(definition, inventory, name, &containers.lookup)?;
    let (items_operation, items_found, items_query) =
        keyed(definition, inventory, name, &items.list)?;
    if items.position.query.is_some() {
        let path: Vec<&str> = items_found
            .parameters
            .iter()
            .filter(|parameter| parameter.location == crate::inventory::Location::Path)
            .map(|parameter| parameter.name.as_str())
            .collect();
        if !path.is_empty() {
            return Err(Refusal::Feed {
                datasource: name.clone(),
                rule: FeedRule::QueryContainer,
                detail: format!(
                    "items.list `{}` has the path parameter{} `{}`",
                    items.list,
                    if path.len() == 1 { "" } else { "s" },
                    path.join("`, `")
                ),
            });
        }
    }
    let position = &items.position;
    for (field, parameter) in [
        ("time", position.time.as_deref()),
        (
            "query.parameter",
            position.query.as_ref().map(|q| q.parameter.as_str()),
        ),
        (
            "page.parameter",
            position.page.as_ref().map(|p| p.parameter.as_str()),
        ),
    ] {
        if let Some(parameter) = parameter
            && !items_query.contains(&parameter)
        {
            return Err(Refusal::Feed {
                datasource: name.clone(),
                rule: FeedRule::PositionParameter,
                detail: format!(
                    "items.position.{field} `{parameter}` is not a query parameter of items.list `{}`",
                    items.list
                ),
            });
        }
    }
    let (containers_limit, _) = page_size(
        definition,
        name,
        &containers.list,
        list_operation,
        &list_query,
    )?;
    let (items_limit, max_limit) =
        page_size(definition, name, &items.list, items_operation, &items_query)?;
    for (filter, values) in datasource.filters.iter().flatten() {
        for parameter in values.keys() {
            if !items_query.contains(&parameter.as_str()) {
                return Err(Refusal::FilterParameter {
                    datasource: name.clone(),
                    filter: filter.clone(),
                    parameter: parameter.clone(),
                });
            }
        }
    }
    let mut containers_value = serde_json::json!({
        "operation_id": list.operation_id,
        "lookup": {
            "operation_id": lookup.operation_id,
            "parameter": sole_path_parameter(name, &containers.lookup, lookup)?,
            "record": "",
        },
        "limit": containers_limit,
        "cursor": containers.cursor,
        "records": containers.records.clone().unwrap_or_default(),
        "id": containers.id,
    });
    if let Some(query) = &containers.query {
        containers_value["query"] = serde_json::json!(query);
    }
    if let Some(name) = &containers.name {
        containers_value["name"] = Value::from(name.as_str());
    }
    if let Some(word) = &containers.kind_word {
        containers_value["kind_word"] = Value::from(word.as_str());
    }
    let mut time = serde_json::Map::new();
    if let Some(parameter) = &position.time {
        time.insert("parameter".to_owned(), Value::from(parameter.as_str()));
    }
    if let Some(precision) = position.precision {
        time.insert("precision".to_owned(), Value::from(precision.word()));
    }
    if let Some(query) = &position.query {
        time.insert("query".to_owned(), serde_json::json!(query));
    }
    if let Some(page) = &position.page {
        time.insert("page".to_owned(), serde_json::json!(page));
    }
    let mut items_value = serde_json::json!({
        "operation_id": items_found.operation_id,
        "limit": items_limit,
        "position": {"kind": "time", "value": time},
        "records": items.records.clone().unwrap_or_default(),
        "id": items.id,
        "revision": items.revision,
        "created_at": items.created_at,
        "updated_at": items.updated_at,
        "body": items.body,
    });
    if position.query.is_none() {
        items_value["container"] =
            Value::from(sole_path_parameter(name, &items.list, items_found)?);
    }
    if let Some(query) = &items.query {
        items_value["query"] = serde_json::json!(query);
    }
    if let Some(author) = &items.author {
        items_value["author"] = serde_json::json!(author);
    }
    for (field, value) in [
        ("url", &items.url),
        ("url_base", &items.url_base),
        ("parent", &items.parent),
    ] {
        if let Some(value) = value {
            items_value[field] = Value::from(value.as_str());
        }
    }
    Ok(Some(serde_json::json!({
        "profile": datasource.profile,
        "capabilities": datasource.capabilities,
        "max_limit": max_limit.unwrap_or(FEED_MAX_LIMIT),
        "containers": containers_value,
        "items": items_value,
    })))
}

/// The fields an operation carries to the engine as written, each from the
/// operation or else from `defaults` ([`Defaults`] says where each applies).
#[derive(Clone, Debug, Default)]
struct Carried {
    required: Vec<String>,
    withhold: Vec<String>,
    credential: Vec<String>,
    bounds: Option<IndexMap<String, Bound>>,
    response: Option<ResponseKind>,
    binary: Option<Binary>,
    download: Option<Download>,
    envelope: Option<Envelope>,
    rate_limit_reasons: Vec<String>,
    rate_limit_headers: Option<RateLimitHeaders>,
    refusal_texts: Vec<TextRefusal>,
    custody: Option<Custody>,
}

fn carried(definition: &ProviderDefinition, operation: &Operation) -> Carried {
    let none = Defaults::default();
    let defaults = definition.defaults.as_ref().unwrap_or(&none);
    let list = |own: &Option<Vec<String>>, default: &Option<Vec<String>>| {
        own.clone().or_else(|| default.clone()).unwrap_or_default()
    };
    let response = operation.response.or(defaults.response);
    let download = operation
        .download
        .clone()
        .or_else(|| defaults.download.clone());
    let effect = operation.effect.unwrap_or(Effect::Read);
    let writes = effect == Effect::Write;
    let answers_json = effect == Effect::Read
        && matches!(response, None | Some(ResponseKind::Json))
        && download.is_none()
        && operation.embedded_text.is_none();
    Carried {
        required: list(&operation.required, &defaults.required),
        withhold: list(&operation.withhold, &defaults.withhold),
        credential: list(&operation.credential, &defaults.credential),
        bounds: operation.bounds.clone().or_else(|| defaults.bounds.clone()),
        response,
        binary: operation.binary.clone().or_else(|| defaults.binary.clone()),
        download,
        envelope: operation
            .envelope
            .clone()
            .or_else(|| answers_json.then(|| defaults.envelope.clone()).flatten()),
        rate_limit_reasons: list(&operation.rate_limit_reasons, &defaults.rate_limit_reasons),
        rate_limit_headers: operation
            .rate_limit_headers
            .clone()
            .or_else(|| defaults.rate_limit_headers.clone()),
        refusal_texts: operation
            .refusal_texts
            .clone()
            .or_else(|| writes.then(|| defaults.refusal_texts.clone()).flatten())
            .unwrap_or_default(),
        custody: operation
            .custody
            .clone()
            .or_else(|| writes.then(|| defaults.custody.clone()).flatten()),
    }
}

/// A guard read's parameter values: each path parameter of the read takes the
/// input of its own name, and `with` binds any parameter of the read.
fn bound_values(
    key: &str,
    probe: &crate::inventory::Operation,
    with: Option<&IndexMap<String, String>>,
) -> Result<BTreeMap<String, String>, Refusal> {
    let mut values: BTreeMap<String, String> = probe
        .parameters
        .iter()
        .filter(|parameter| parameter.location == crate::inventory::Location::Path)
        .map(|parameter| (parameter.name.clone(), parameter.name.clone()))
        .collect();
    for (parameter, input) in with.into_iter().flatten() {
        if !probe.parameters.iter().any(|p| &p.name == parameter) {
            return Err(Refusal::GuardParameter {
                key: key.to_owned(),
                parameter: parameter.clone(),
            });
        }
        values.insert(parameter.clone(), input.clone());
    }
    Ok(values)
}

/// The engine's guard for one operation's `guard`.
fn guard(
    definition: &ProviderDefinition,
    inventory: &crate::inventory::Inventory,
    key: &str,
    guard: &Guard,
) -> Result<Value, Refusal> {
    let probe = |read: &str| -> Result<(String, &crate::inventory::Operation), Refusal> {
        let operation_id = definition
            .operations
            .get(read)
            .and_then(|local| local.op.as_deref())
            .unwrap_or(read);
        let found = inventory
            .operations
            .iter()
            .find(|candidate| candidate.operation_id.as_deref() == Some(operation_id))
            .ok_or_else(|| Refusal::UnknownGuardRead {
                key: key.to_owned(),
                read: read.to_owned(),
            })?;
        Ok((operation_id.to_owned(), found))
    };
    let preflight = |before: &GuardBefore| -> Result<Value, Refusal> {
        let (operation_id, found) = probe(&before.read)?;
        let mut preflight = serde_json::json!({
            "operation_id": operation_id,
            "values": bound_values(key, found, before.with.as_ref())?,
            "checks": checks(before.expect.as_ref()),
        });
        if let Some(mismatch) = before.mismatch {
            preflight["mismatch"] = serde_json::json!(mismatch);
        }
        Ok(preflight)
    };
    let mut value = serde_json::json!({"preflight": preflight(&guard.before)?});
    if let Some(also) = &guard.also {
        value["further_preflights"] =
            Value::Array(also.iter().map(&preflight).collect::<Result<_, _>>()?);
    }
    let after = guard.after.as_ref();
    let mut postflight = serde_json::json!({
        "checks": checks(after.and_then(|after| after.expect.as_ref())),
    });
    if let Some(after) = after {
        if let Some(read) = &after.read {
            let (operation_id, found) = probe(&read.read)?;
            postflight["read"] = serde_json::json!({
                "operation_id": operation_id,
                "values": bound_values(key, found, read.with.as_ref())?,
            });
        }
        if after.any.is_some() {
            postflight["any_of"] = Value::Array(checks(after.any.as_ref()));
        }
        if let Some(absent) = after.absent {
            postflight["absent"] = Value::Bool(absent);
        }
        if let Some(refusals) = &after.refusals {
            postflight["refusals"] = serde_json::json!(refusals);
        }
    }
    value["postflight"] = postflight;
    Ok(value)
}

/// Every input a lowered guard reads: a check's `input` and each bound value.
fn guard_inputs<'a>(value: &'a Value, found: &mut Vec<&'a str>) {
    match value {
        Value::Object(map) => {
            for (name, inner) in map {
                match (name.as_str(), inner) {
                    ("input", Value::String(input)) => found.push(input),
                    ("values", Value::Object(values)) => {
                        found.extend(values.values().filter_map(Value::as_str));
                    }
                    _ => guard_inputs(inner, found),
                }
            }
        }
        Value::Array(items) => items.iter().for_each(|item| guard_inputs(item, found)),
        _ => {}
    }
}

/// The engine's load check of a guard against a closed body: a guard reading
/// `body.<dotted>` needs a body key the body admits and the caller sends.
fn guard_body_reads(key: &str, body: &Body, guard: &Value) -> Result<(), Refusal> {
    if body.keys.is_empty() {
        return Ok(());
    }
    let mut inputs = Vec::new();
    guard_inputs(guard, &mut inputs);
    for dotted in inputs
        .into_iter()
        .filter_map(|input| input.strip_prefix("body."))
    {
        let read: Vec<&str> = dotted.split('.').collect();
        let refuse = |spec: &str| Refusal::BodyType {
            key: key.to_owned(),
            field: format!("body.{dotted}"),
            spec: spec.to_owned(),
        };
        let Some(field) = body
            .keys
            .iter()
            .find(|field| path(field).is_some_and(|own| read.starts_with(&own)))
        else {
            return Err(refuse(
                "a body key its guard reads, which its body does not admit",
            ));
        };
        if body.fixed.contains_key(field) {
            return Err(refuse(
                "a body key its guard reads as an input, which its body fixes",
            ));
        }
    }
    Ok(())
}

/// The selection a definition lowers to, against the inventory of its bundle
/// and the (projected) source document whose summaries are the default
/// descriptions. Pure: it reads and writes nothing.
pub fn select(
    definition: &ProviderDefinition,
    provider: &str,
    inventory: &crate::inventory::Inventory,
    document: &Value,
) -> Result<OperationsFile, Refusal> {
    let mut operations = Vec::new();
    for (key, operation) in &definition.operations {
        let carried = carried(definition, operation);
        let found = match &operation.op {
            Some(op) => Some(
                inventory
                    .operations
                    .iter()
                    .find(|candidate| candidate.operation_id.as_deref() == Some(op.as_str()))
                    .ok_or_else(|| Refusal::UnknownOperation {
                        key: key.clone(),
                        op: op.clone(),
                    })?,
            ),
            None if carried.download.is_some() => None,
            None => return Err(Refusal::NoOperation { key: key.clone() }),
        };
        let effect = match (operation.effect, found) {
            (Some(Effect::Write), found)
                if !found.is_some_and(|found| {
                    matches!(found.method.as_str(), "post" | "put" | "patch" | "delete")
                }) =>
            {
                return Err(Refusal::WriteMethod {
                    key: key.clone(),
                    op: operation.op.clone().unwrap_or_default(),
                    method: found.map(|found| found.method.clone()).unwrap_or_default(),
                });
            }
            (Some(effect), _) => effect,
            (None, None) => Effect::Read,
            (None, Some(found)) if found.method == "get" => Effect::Read,
            (None, Some(found)) => {
                return Err(Refusal::EffectRequired {
                    key: key.clone(),
                    op: operation.op.clone().unwrap_or_default(),
                    method: found.method.clone(),
                });
            }
        };
        let query = found.map(query_of).unwrap_or_default();
        let mut bounds = BTreeMap::new();
        if let Some((_, style)) = paging_style(
            key,
            operation,
            definition.paging.as_ref(),
            &query,
            &carried.withhold,
        )? && let (Some(limit), Some(max)) = (&style.limit, style.max)
            && query.contains(&limit.as_str())
        {
            bounds.insert(
                limit.clone(),
                Bound {
                    minimum: Some(1),
                    maximum: Some(max),
                    values: None,
                },
            );
        }
        for (parameter, bound) in carried.bounds.iter().flatten() {
            bounds.insert(parameter.clone(), bound.clone());
        }
        if let Some(status) = operation.conflict_status
            && !(effect == Effect::Write && matches!(status, 409 | 412))
        {
            return Err(Refusal::ConflictStatus {
                key: key.clone(),
                status,
            });
        }
        let declared_body = body(key, operation)?;
        if effect != Effect::Write
            && let Some(member) = [
                ("body", operation.body.is_some()),
                ("body_enums", operation.body_enums.is_some()),
                ("body_any_of", operation.body_any_of.is_some()),
                ("body_prefixes", operation.body_prefixes.is_some()),
            ]
            .iter()
            .find_map(|(member, declared)| declared.then_some(member))
        {
            return Err(Refusal::Malformed(format!(
                "operation `{key}` declares {member}; only a write declares a body"
            )));
        }
        // The engine fixes query values only on a read, and reads an embedded
        // text only from a read answering JSON that reads its answer no other way.
        if effect != Effect::Read
            && let Some(member) = [
                ("query_fixed", operation.query_fixed.is_some()),
                ("query_folds", operation.query_folds.is_some()),
            ]
            .iter()
            .find_map(|(member, declared)| declared.then_some(member))
        {
            return Err(Refusal::Malformed(format!(
                "operation `{key}` declares {member}; only a read fixes query values"
            )));
        }
        if operation.embedded_text.is_some()
            && let Some(member) = [
                ("a write", effect != Effect::Read),
                ("response", carried.response.is_some()),
                ("envelope", carried.envelope.is_some()),
                ("binary", carried.binary.is_some()),
                ("download", carried.download.is_some()),
            ]
            .iter()
            .find_map(|(member, declared)| declared.then_some(member))
        {
            return Err(Refusal::Malformed(format!(
                "operation `{key}` declares embedded_text beside {member}; only a read answering JSON that it reads no other way carries one"
            )));
        }
        let guard = operation
            .guard
            .as_ref()
            .map(|declared| guard(definition, inventory, key, declared))
            .transpose()?;
        if let Some(guard) = &guard {
            guard_body_reads(key, &declared_body, guard)?;
        }
        operations.push(SelectionEntry {
            id: key.clone(),
            operation_id: operation.op.clone(),
            effect,
            description: operation
                .description
                .clone()
                .or_else(|| operation.op.as_deref().and_then(|op| summary(document, op))),
            response: carried.response,
            binary: carried.binary,
            download: carried.download,
            envelope: carried.envelope,
            bounds,
            required: carried.required,
            withhold: carried.withhold,
            credential: carried.credential,
            rate_limit_reasons: carried.rate_limit_reasons,
            refusal_texts: carried.refusal_texts,
            body_keys: declared_body.keys,
            body_types: declared_body.types,
            body_required: declared_body.required,
            body_nonempty: declared_body.nonempty,
            body_fixed: declared_body.fixed,
            body_enums: declared_body.enums,
            body_any_of: declared_body.any_of,
            body_prefixes: declared_body.prefixes,
            conflict_status: operation.conflict_status,
            guard,
            custody: carried.custody,
            query_fixed: operation
                .query_fixed
                .iter()
                .flatten()
                .map(|(name, value)| (name.clone(), value.clone()))
                .collect(),
            query_folds: operation
                .query_folds
                .iter()
                .flatten()
                .map(|(name, fold)| (name.clone(), fold.clone()))
                .collect(),
            embedded_text: operation.embedded_text.clone(),
            rate_limit_headers: carried.rate_limit_headers,
        });
    }
    check_references(definition, inventory)?;
    Ok(OperationsFile {
        format: OPERATIONS_FORMAT,
        provider: provider.to_owned(),
        operations,
        feed: feed(definition, inventory)?,
    })
}

/// `scheme://host` of a URL or URL template: everything before the third `/`.
fn origin(template: &str) -> &str {
    let after_scheme = template.find("://").map_or(0, |at| at + 3);
    template[after_scheme..]
        .find('/')
        .map_or(template, |at| &template[..after_scheme + at])
}

/// The rules of `references/v1alpha1` a definition can be held to before any
/// runtime serves it. References and views are validated here and lowered into
/// nothing: no runtime serves them yet.
fn check_references(
    definition: &ProviderDefinition,
    inventory: &crate::inventory::Inventory,
) -> Result<(), Refusal> {
    let empty = IndexMap::new();
    let references = definition.references.as_ref().unwrap_or(&empty);
    let base = origin(&definition.base_url);
    for (name, reference) in references {
        let refuse = |detail: String| Refusal::Reference {
            reference: name.clone(),
            detail,
        };
        if reference.matches.is_empty() {
            return Err(refuse("declares no match".to_owned()));
        }
        let mut bound: Vec<Vec<String>> = Vec::new();
        for entry in &reference.matches {
            let names: Vec<String> = match (&entry.url, &entry.pattern) {
                (Some(url), None) => {
                    if origin(url) != base {
                        return Err(refuse(format!(
                            "url `{url}` is not on base_url's origin `{base}`"
                        )));
                    }
                    variables(url)?
                        .into_iter()
                        .map(|name| name.trim_start_matches('+').to_owned())
                        .collect()
                }
                (None, Some(pattern)) => {
                    let compiled = regex::Regex::new(pattern)
                        .map_err(|error| refuse(format!("pattern `{pattern}`: {error}")))?;
                    compiled
                        .capture_names()
                        .flatten()
                        .map(str::to_owned)
                        .collect()
                }
                _ => {
                    return Err(refuse(
                        "a match is exactly one of url or pattern".to_owned(),
                    ));
                }
            };
            if names.is_empty() {
                return Err(refuse("a match binds no value".to_owned()));
            }
            bound.push(names);
        }
        let unbound = |variable: &str| {
            bound
                .iter()
                .any(|names| !names.iter().any(|n| n == variable))
        };
        let operation = definition
            .operations
            .get(&reference.resolve.operation)
            .ok_or_else(|| {
                refuse(format!(
                    "resolves through `{}`, which operations does not declare",
                    reference.resolve.operation
                ))
            })?;
        let found = inventory
            .operations
            .iter()
            .find(|candidate| {
                operation.op.is_some()
                    && candidate.operation_id.as_deref() == operation.op.as_deref()
            })
            .ok_or_else(|| {
                refuse(format!(
                    "`{}` names no source operation",
                    reference.resolve.operation
                ))
            })?;
        if found.method != "get" {
            return Err(refuse(format!(
                "resolves through `{}`, which is {}, not a read",
                reference.resolve.operation,
                found.method.to_uppercase()
            )));
        }
        for parameter in found
            .parameters
            .iter()
            .filter(|parameter| parameter.location == crate::inventory::Location::Path)
        {
            if !reference.resolve.with.contains_key(&parameter.name) {
                return Err(refuse(format!(
                    "path parameter `{}` of `{}` is not bound by with",
                    parameter.name, reference.resolve.operation
                )));
            }
        }
        for (parameter, value) in &reference.resolve.with {
            if !found.parameters.iter().any(|p| &p.name == parameter) {
                return Err(refuse(format!(
                    "with binds `{parameter}`, which `{}` does not take",
                    reference.resolve.operation
                )));
            }
            for variable in variables(value)? {
                if unbound(variable) {
                    return Err(refuse(format!("`{variable}` is not bound by every match")));
                }
            }
        }
        for template in reference.outputs.iter().flat_map(IndexMap::values) {
            for variable in variables(template)? {
                if !variable.starts_with('/') && unbound(variable) {
                    return Err(refuse(format!(
                        "output uses `{variable}`, which not every match binds"
                    )));
                }
            }
        }
    }
    for (view, declared) in definition.views.iter().flatten() {
        if !references.contains_key(&declared.of) {
            return Err(Refusal::ViewReference {
                view: view.clone(),
                of: declared.of.clone(),
            });
        }
    }
    Ok(())
}

/// A plain file name: one normal path component.
fn plain_file_name(name: &str) -> bool {
    !name.is_empty()
        && Path::new(name).components().collect::<Vec<_>>().as_slice()
            == [Component::Normal(name.as_ref())]
}

/// Refuse `value` of `field` unless it is a plain file name on every platform
/// the output may be read on: no `\` (a separator on Windows) and no NUL.
fn plain_name(field: &str, value: &str) -> Result<(), Refusal> {
    if plain_file_name(value) && !value.contains(['\\', '\0']) {
        return Ok(());
    }
    Err(Refusal::NotPlainName {
        field: field.to_owned(),
        value: value.to_owned(),
    })
}

/// Lower one definition. Every check that can refuse runs before the output
/// directory is touched: the bundle is compiled into a staging directory under
/// `scratch`, the selection is resolved against it, and only then are the bundle
/// (with its index entry) written to `out` and the declaration and the selection
/// it names to `out/<catalog>/`.
pub fn lower(request: &Request<'_>) -> Result<Lowered, Refusal> {
    let shown = request.definition.display();
    let bytes = std::fs::read(request.definition)
        .map_err(|error| Refusal::Unreadable(format!("{shown}: {error}")))?;
    let definition = parse(&bytes)?;
    let name = stem(request.definition)?;
    let provider = definition.catalog.as_deref().unwrap_or(name);

    let (declared, auth_profile) = declaration(&definition, name, provider, request.root)?;

    let (kind, document_path) = definition.source.document()?;
    let source_path = under_root(request.root, document_path)?;
    let file_name = source_path
        .file_name()
        .and_then(|name| name.to_str())
        .and_then(|name| name.strip_suffix(".gz"))
        .filter(|name| !name.is_empty())
        .ok_or_else(|| {
            Refusal::SourceNotGzip(format!("{document_path} is not named `<name>.gz`"))
        })?;
    let compressed = std::fs::read(&source_path)
        .map_err(|error| Refusal::Unreadable(format!("{document_path}: {error}")))?;
    let source = gunzip(document_path, &compressed)?;
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
    let bundles = staging.path().join("bundles");
    let base = pipeline::Request {
        provider,
        source: Path::new(""),
        directory: &bundles,
        auth_profile: &auth_profile,
        replace: false,
        amendments: amendments.as_deref(),
    };
    let openapi = match kind {
        SourceKind::OpenApi => {
            let source_file = staging.path().join(file_name);
            std::fs::write(&source_file, &source).map_err(write)?;
            pipeline::run(&pipeline::Request {
                source: &source_file,
                ..base
            })
            .map_err(|failure| Refusal::Pipeline(failure.to_string()))?;
            source
        }
        SourceKind::Swagger | SourceKind::Discovery => {
            let projection_name = definition.source.projection.as_deref().unwrap_or_default();
            if !plain_file_name(projection_name) || projection_name == file_name {
                return Err(Refusal::SourceKind(format!(
                    "projection `{projection_name}` is not a plain file name beside the source"
                )));
            }
            let projection = crate::derived::project(&source)
                .map_err(|refusal| Refusal::Pipeline(refusal.to_string()))?;
            let expected = match kind {
                SourceKind::Swagger => crate::swagger::FORMAT,
                _ => crate::discovery::FORMAT,
            };
            if projection.format() != expected {
                return Err(Refusal::SourceKind(format!(
                    "{document_path} is a {} document, not the {expected} the source names",
                    projection.format()
                )));
            }
            let original = staging.path().join(file_name);
            std::fs::write(&original, &source).map_err(write)?;
            let projected = staging.path().join(projection_name);
            std::fs::write(&projected, projection.openapi()).map_err(write)?;
            pipeline::run_derived(
                &pipeline::Request {
                    source: &projected,
                    ..base
                },
                &original,
            )
            .map_err(|failure| Refusal::Pipeline(failure.to_string()))?;
            projection.openapi().to_vec()
        }
    };
    let bundle =
        bundle::load(&bundles, provider).map_err(|error| Refusal::Pipeline(error.to_string()))?;
    let document = crate::parse_document(&openapi)
        .map_err(|refusal| Refusal::Pipeline(refusal.reason().to_owned()))?;

    let selection = select(&definition, provider, &bundle.inventory, &document)?;
    let mut selection =
        serde_json::to_vec_pretty(&selection).map_err(|error| Refusal::Write(error.to_string()))?;
    selection.push(b'\n');

    let entry = bundle::write(request.out, &bundle, true)
        .map_err(|error| Refusal::Write(error.to_string()))?;
    let directory = request.out.join(provider);
    std::fs::create_dir_all(&directory).map_err(write)?;
    let operations_path = directory.join(selection_file(&definition, name));
    std::fs::write(&operations_path, selection).map_err(write)?;
    let connector_path = directory.join("connector.json");
    std::fs::write(&connector_path, declared).map_err(write)?;
    Ok(Lowered {
        provider: provider.to_owned(),
        bundle: request.out.join(&entry.file_name),
        entry,
        operations: operations_path,
        connector: connector_path,
    })
}

/// A `conflict_status` read wide: a number beyond a status becomes `u16::MAX`,
/// which the lowering refuses as a `ConflictStatus` naming the operation.
fn status_or_beyond<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<Option<u16>, D::Error> {
    Ok(Option::<i64>::deserialize(deserializer)?
        .map(|status| u16::try_from(status).unwrap_or(u16::MAX)))
}
