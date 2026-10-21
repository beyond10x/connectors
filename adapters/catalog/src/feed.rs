//! The `datasource.feed/v1alpha1` realization: a provider's [`Declaration`], read as data, binds
//! `feed.containers` and `feed.items` over list endpoints of its pinned bundle. No provider has
//! its own Rust here; `contracts/catalog/v1alpha1/semantics.md` section 3.3 is the normative
//! statement, and `spec/ess/domains/feed.yaml` beside this crate models the declaration.
//!
//! A watermark is formed in one of two ways, as the declaration states:
//! - **time**: the provider lists a container's items at or after an instant, oldest change
//!   first. The watermark is the last returned item's `updated_at`, sent back as that filter, and
//!   the `(id, revision)` pairs already returned at that instant, which are skipped on resume.
//! - **cursor**: the provider keeps a change cursor. The watermark carries it unchanged.
//!
//! A time filter is sent as a parameter or inside a query template (JQL, CQL) whose container
//! and time values are quoted literals; at minute precision the overlap minute is re-read and
//! what the watermark holds within it is dropped by `(id, revision)`, and a page continuation
//! reads further pages within one call. A container listing continues by the provider's token,
//! by one parameter of its next link (never requested), after a key, or by offset.
//!
//! Both are wrapped with the profile, the instance and the container, so a watermark issued for
//! any other is refused as `stale_cursor`, never answered as a first read. That refusal comes
//! after the container's admission: an unknown container and one whose items the provider
//! refuses answer `not_found` whatever the watermark.
use crate::{path_within, read_body, scalar};
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use connectors_catalog::{
    bundle::Bundle,
    inventory::{Location, Operation},
    template::{Supplied, Template},
};
use connectors_core::{Error, ErrorCode, Result};
use connectors_sdk::AuthenticatedHttp;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};

/// The family both operations carry in the descriptor.
pub const FEED_CONTRACT: &str = "datasource.feed/v1alpha1";
/// The family's fixed operation ids.
pub const CONTAINERS: &str = "feed.containers";
pub const ITEMS: &str = "feed.items";
/// A body longer than this is clipped on a UTF-8 boundary and reported in its envelope.
pub const BODY_BYTES: usize = 64 * 1024;
/// A serialized page longer than this is not returned; the answer is `unavailable`.
pub const RESULT_BYTES: usize = 3 * 1024 * 1024;
/// The longest watermark or listing cursor issued or read.
pub const WATERMARK_BYTES: usize = 16 * 1024;
/// The longest id, revision or instant read from a provider record.
const FIELD_BYTES: usize = 1024;
/// The longest query template.
const TEMPLATE_BYTES: usize = 1024;
/// The most listing requests one `feed.items` call with a page continuation makes.
pub const PAGES: usize = 10;
const MARK_FORMAT: &str = "catalog-feed/1";

// ---- the declaration -------------------------------------------------------------------------

/// The family's closed visibility vocabulary.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Visibility {
    Public,
    Private,
    Direct,
}

impl Visibility {
    fn as_str(self) -> &'static str {
        match self {
            Self::Public => "public",
            Self::Private => "private",
            Self::Direct => "direct",
        }
    }
}

/// True for a record or answer whose value at `pointer` equals `equals`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Condition {
    pub pointer: String,
    pub equals: Value,
}

impl Condition {
    fn holds(&self, value: &Value) -> bool {
        value.pointer(&self.pointer) == Some(&self.equals)
    }
}

/// The provider value at `pointer`, as text, selects the visibility; anything else is private.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VisibilityRule {
    pub pointer: String,
    pub map: BTreeMap<String, Visibility>,
}

/// Offset paging: the listing ends on the answer for which `complete` holds.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OffsetPaging {
    pub complete: Condition,
}

/// How a listing continues, sent as `parameter`: the provider's own continuation read back at
/// `next` in the answer (with `url_parameter`, that query parameter of the next link at `next`),
/// the key at `last` in the last record of a full page, for a provider that lists after a key it
/// is given, or the index of the next page's first record (`offset`). Exactly one of the three.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CursorPaging {
    pub parameter: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub next: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url_parameter: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub offset: Option<OffsetPaging>,
}

/// Reads one container by id; `record` points at it in the answer.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Lookup {
    pub operation_id: String,
    pub parameter: String,
    pub record: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Containers {
    pub operation_id: String,
    pub lookup: Lookup,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub query: Option<BTreeMap<String, String>>,
    pub limit: String,
    pub cursor: CursorPaging,
    pub records: String,
    pub id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// The provider's word for a container, read at this pointer in each record; or, for a
    /// provider whose records carry none, `kind_word`. Exactly one of the two.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kind: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kind_word: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub visibility: Option<VisibilityRule>,
}

/// The resolution of a time filter: the kept instant as it is, or its minute in UTC.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Precision {
    Instant,
    Minute,
}

/// The query languages a template is written in; both quote a value as `"..."`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum QueryLanguage {
    Jql,
    Cql,
}

/// A query sent as `parameter`: `first` with a `{container}` slot, `resume` with a
/// `{container}` and a `{since}` slot. Each value is a quoted literal, never spliced raw.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QueryTemplate {
    pub parameter: String,
    pub language: QueryLanguage,
    pub first: String,
    pub resume: String,
}

/// A continuation within one `feed.items` call, sent as `parameter`: the token at `next`, or,
/// with `url_parameter`, that query parameter of the next link at `next`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Continuation {
    pub parameter: String,
    pub next: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url_parameter: Option<String>,
}

/// An inclusive lower bound on `updated_at`, answered oldest change first: sent as `parameter`
/// or inside `query`, exactly one of the two, and read further by `page` within one call.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TimeFilter {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parameter: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub precision: Option<Precision>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub query: Option<QueryTemplate>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub page: Option<Continuation>,
}

/// The provider's change cursor, its end condition, and the statuses with which it refuses a
/// cursor it no longer resumes.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ChangeCursor {
    pub parameter: String,
    pub next: String,
    pub complete: Condition,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stale: Option<Vec<u16>>,
}

/// How `feed.items` resumes.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "value", rename_all = "snake_case")]
pub enum Position {
    Time(TimeFilter),
    Cursor(ChangeCursor),
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AuthorFields {
    pub id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BodyField {
    pub pointer: String,
    pub representation: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Items {
    pub operation_id: String,
    /// The parameter that takes the container id; absent exactly when a time `query` carries it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub container: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub query: Option<BTreeMap<String, String>>,
    pub limit: String,
    pub position: Position,
    pub records: String,
    pub id: String,
    pub revision: String,
    pub created_at: String,
    pub updated_at: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub author: Option<AuthorFields>,
    pub body: BodyField,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    /// Where the answer names the base its relative item links are under.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url_base: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub deleted: Option<Condition>,
}

/// Whether a removed item comes back as a tombstone.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum DeletionCapability {
    Observed,
    NotObserved,
}

/// Where a container's `kind` comes from.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum KindCapability {
    ProviderWord,
    FixedWord,
}

/// How a revision is formed.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum RevisionCapability {
    Opaque,
    UpdateTime,
}

/// How a container's visibility is stated.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum VisibilityCapability {
    Mapped,
    AllPrivate,
}

/// What the profile states its provider lets it observe: the family's profile capabilities. The
/// declaration must support each claim; [`Feed::new`] refuses one it does not.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Capabilities {
    pub deletions: DeletionCapability,
    pub kind: KindCapability,
    pub revision: RevisionCapability,
    pub visibility: VisibilityCapability,
}

/// One provider's feed, as data: the `feed` of a `connectors-catalog-operations/1` file.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Declaration {
    pub profile: String,
    pub capabilities: Capabilities,
    pub max_limit: u64,
    pub containers: Containers,
    pub items: Items,
}

// ---- the realization -------------------------------------------------------------------------

fn refuse(message: impl Into<String>) -> Error {
    Error::invalid(message)
}

fn unavailable(message: &str) -> Error {
    Error::new(ErrorCode::Unavailable, message)
}

/// The one answer for an unknown container and an excluded direct one alike.
fn not_found() -> Error {
    Error::new(ErrorCode::NotFound, "container not found")
}

fn stale() -> Error {
    Error::new(
        ErrorCode::StaleCursor,
        "watermark or cursor was not issued for this read",
    )
}

/// The declaration checked against the bundle; built once, before any request.
pub(crate) struct Feed {
    declaration: Declaration,
    base_len: usize,
    list: Template,
    lookup: Template,
    items: Template,
    /// The time query's two templates, parsed when the declaration loads.
    query: Option<Query>,
    /// The origin of the instance's API base, which a root-relative item link resolves under
    /// when the declaration names no `url_base`.
    links: Option<String>,
    containers_declaration: connectors_core::Operation,
    items_declaration: connectors_core::Operation,
}

fn pointer(name: &str, pointer: &str) -> Result<()> {
    if pointer.is_empty() || pointer.starts_with('/') {
        Ok(())
    } else {
        Err(refuse(format!("feed `{name}` is not a JSON pointer")))
    }
}

/// A query parameter name of a next link: 1 to 128 visible ASCII characters.
fn url_name(name: &str, value: &str) -> Result<()> {
    if value.is_empty() || value.len() > 128 || !value.bytes().all(|b| b.is_ascii_graphic()) {
        return Err(refuse(format!(
            "feed `{name}` is not 1 to 128 visible ASCII characters"
        )));
    }
    Ok(())
}

// ---- query templates -------------------------------------------------------------------------

/// One piece of a parsed query template.
#[derive(Clone, Debug, PartialEq, Eq)]
enum Part {
    Text(String),
    Container,
    Since,
}

/// A time query, its templates parsed and checked.
struct Query {
    parameter: String,
    first: Vec<Part>,
    resume: Vec<Part>,
}

/// `text`, the template `name`, as text and slots. Refused: a template longer than
/// [`TEMPLATE_BYTES`], a control character, a brace outside a slot, a slot other than
/// `{container}` and `{since}`, a slot beside a `"`, `'` or `\` or inside a literal the template
/// opens with `"` or `'` (where a value would be read as part of that literal, not as its own),
/// a literal left open, a `\` outside a literal, and slot counts other than `first`
/// (`{container}` once, no `{since}`) and `resume` (each once) take.
fn template(name: &str, text: &str, resume: bool) -> Result<Vec<Part>> {
    if text.len() > TEMPLATE_BYTES {
        return Err(refuse(format!(
            "feed `{name}` is longer than {TEMPLATE_BYTES} bytes"
        )));
    }
    if text.chars().any(char::is_control) {
        return Err(refuse(format!("feed `{name}` holds a control character")));
    }
    let stray = || refuse(format!("feed `{name}` has a stray brace"));
    let mut parts = Vec::new();
    let mut rest = text;
    while !rest.is_empty() {
        let at = rest.find(['{', '}']).unwrap_or(rest.len());
        if at > 0 {
            parts.push(Part::Text(rest[..at].to_owned()));
        }
        rest = &rest[at..];
        if rest.is_empty() {
            break;
        }
        if rest.starts_with('}') {
            return Err(stray());
        }
        let end = rest.find('}').ok_or_else(stray)?;
        let slot = &rest[1..end];
        if slot.contains('{') {
            return Err(stray());
        }
        parts.push(match slot {
            "container" => Part::Container,
            "since" => Part::Since,
            other => {
                return Err(refuse(format!(
                    "feed `{name}` names an unknown slot `{{{other}}}`"
                )));
            }
        });
        rest = &rest[end + 1..];
    }
    let quoting = |c: char| matches!(c, '"' | '\'' | '\\');
    for (index, part) in parts.iter().enumerate() {
        if matches!(part, Part::Text(_)) {
            continue;
        }
        let before = index
            .checked_sub(1)
            .and_then(|i| parts.get(i))
            .and_then(|p| match p {
                Part::Text(text) => text.chars().next_back(),
                _ => None,
            });
        let after = parts.get(index + 1).and_then(|p| match p {
            Part::Text(text) => text.chars().next(),
            _ => None,
        });
        if before.is_some_and(quoting) || after.is_some_and(quoting) {
            return Err(refuse(format!(
                "feed `{name}` puts a slot beside a quote or backslash; the engine quotes each value"
            )));
        }
    }
    // A slot inside a literal the template opens would let its value close that literal.
    let mut open: Option<char> = None;
    let mut escaped = false;
    for part in &parts {
        let Part::Text(text) = part else {
            if open.is_some() {
                return Err(refuse(format!(
                    "feed `{name}` puts a slot inside a literal; the engine quotes each value"
                )));
            }
            continue;
        };
        for c in text.chars() {
            match open {
                Some(_) if escaped => escaped = false,
                Some(_) if c == '\\' => escaped = true,
                Some(quote) if c == quote => open = None,
                Some(_) => {}
                None if matches!(c, '"' | '\'') => open = Some(c),
                // Outside a literal a provider may or may not read `\` as an escape, so where a
                // literal starts would be the provider's guess, not the template's.
                None if c == '\\' => {
                    return Err(refuse(format!(
                        "feed `{name}` has a backslash outside a literal"
                    )));
                }
                None => {}
            }
        }
    }
    if open.is_some() {
        return Err(refuse(format!("feed `{name}` leaves a literal open")));
    }
    let count = |slot: &Part| parts.iter().filter(|p| *p == slot).count();
    let (containers, sinces) = (count(&Part::Container), count(&Part::Since));
    if !resume && (containers, sinces) != (1, 0) {
        return Err(refuse(format!(
            "feed `{name}` names `{{container}}` once and `{{since}}` never"
        )));
    }
    if resume && (containers, sinces) != (1, 1) {
        return Err(refuse(format!(
            "feed `{name}` names `{{container}}` and `{{since}}` once each"
        )));
    }
    Ok(parts)
}

/// `value` as one quoted literal of the query languages: `"`, each `\` and `"` in it escaped
/// with a `\`, then `"`. A value holding a control character is refused, never sent.
fn quote(value: &str) -> Result<String> {
    if value.chars().any(char::is_control) {
        return Err(refuse("a query value holds a control character"));
    }
    let mut quoted = String::with_capacity(value.len() + 2);
    quoted.push('"');
    for c in value.chars() {
        if matches!(c, '"' | '\\') {
            quoted.push('\\');
        }
        quoted.push(c);
    }
    quoted.push('"');
    Ok(quoted)
}

/// The template filled in one pass: each slot's value is quoted, and no value is read again as
/// template text.
fn render(parts: &[Part], container: &str, since: Option<&str>) -> Result<String> {
    let container = quote(container)?;
    let since = since.map(quote).transpose()?;
    let mut text = String::new();
    for part in parts {
        match part {
            Part::Text(piece) => text.push_str(piece),
            Part::Container => text.push_str(&container),
            Part::Since => text.push_str(since.as_deref().ok_or_else(Error::internal)?),
        }
    }
    Ok(text)
}

// ---- next links ------------------------------------------------------------------------------

/// The one value of the query parameter `name` in the link `url`, form-decoded. Only the query
/// is read: the scheme, host and path are ignored and the link is never requested. `None` for a
/// link without a query, without the parameter or with it twice, with an empty value, or with
/// text that does not decode to UTF-8.
pub(crate) fn url_parameter(url: &str, name: &str) -> Option<String> {
    if url.len() > WATERMARK_BYTES {
        return None;
    }
    let url = url.split('#').next()?;
    let (_, query) = url.split_once('?')?;
    let mut found = None;
    for pair in query.split('&') {
        let (key, value) = pair.split_once('=').unwrap_or((pair, ""));
        if form_decode(key)? != name {
            continue;
        }
        if found.is_some() {
            return None;
        }
        found = Some(form_decode(value)?);
    }
    found.filter(|value| !value.is_empty())
}

/// `text` form-decoded: `+` is a space and `%XX` a byte; `None` for a malformed escape or bytes
/// that are not UTF-8.
fn form_decode(text: &str) -> Option<String> {
    let bytes = text.as_bytes();
    let mut decoded = Vec::with_capacity(bytes.len());
    let mut at = 0;
    while at < bytes.len() {
        match bytes[at] {
            b'+' => {
                decoded.push(b' ');
                at += 1;
            }
            b'%' => {
                let hex = bytes.get(at + 1..at + 3)?;
                if !hex.iter().all(u8::is_ascii_hexdigit) {
                    return None;
                }
                decoded.push(u8::from_str_radix(std::str::from_utf8(hex).ok()?, 16).ok()?);
                at += 3;
            }
            byte => {
                decoded.push(byte);
                at += 1;
            }
        }
    }
    String::from_utf8(decoded).ok()
}

/// The continuation an answer carries at `next`: the token itself, or with `url_parameter` that
/// parameter of the link. An absent, null or empty `next` ends the list; a link that lost its
/// parameter is `unavailable`, never the end.
fn continuation(answer: &Value, next: &str, url_parameter: Option<&str>) -> Result<Option<String>> {
    let Some(found) = answer
        .pointer(next)
        .and_then(scalar)
        .filter(|next| !next.is_empty())
    else {
        return Ok(None);
    };
    match url_parameter {
        None => Ok(Some(found)),
        Some(name) => self::url_parameter(&found, name).map(Some).ok_or_else(|| {
            unavailable("provider answered a next link without the continuation it names")
        }),
    }
}

/// The operation `id` names, a GET under the base path, its template, and every parameter it
/// is bound with checked: `fixed` and `constants` are sent on every request and must cover each
/// required parameter; `sometimes` must not be required.
fn bind(
    bundle: &Bundle,
    base: &[String],
    id: &str,
    fixed: &[(&str, bool)],
    sometimes: &[&str],
    constants: Option<&BTreeMap<String, String>>,
) -> Result<Template> {
    // As for a selection: an `operationId` the source declares twice names no
    // single request and is refused.
    let mut matching = bundle
        .inventory
        .operations
        .iter()
        .filter(|o| o.operation_id.as_deref() == Some(id));
    let operation: &Operation = matching
        .next()
        .ok_or_else(|| refuse(format!("bundle carries no operation `{id}`")))?;
    if matching.next().is_some() {
        return Err(refuse(format!(
            "bundle carries more than one operation `{id}`"
        )));
    }
    if operation.method != "get" || !path_within(&operation.path, base) {
        return Err(refuse(format!(
            "feed operation `{id}` must be a GET under the base path"
        )));
    }
    let constants: Vec<&str> = constants
        .into_iter()
        .flat_map(|c| c.keys().map(String::as_str))
        .collect();
    let mut bound = BTreeSet::new();
    let named = fixed
        .iter()
        .copied()
        .chain(sometimes.iter().map(|name| (*name, false)))
        .chain(constants.iter().map(|name| (*name, false)));
    for (name, path_allowed) in named {
        let parameter = operation
            .parameters
            .iter()
            .find(|p| p.name == name)
            .filter(|p| {
                p.location == Location::Query || (path_allowed && p.location == Location::Path)
            })
            .ok_or_else(|| {
                refuse(format!(
                    "feed binds `{name}`, which `{id}` does not take there"
                ))
            })?;
        if !bound.insert(name) {
            return Err(refuse(format!("feed binds `{name}` of `{id}` twice")));
        }
        if sometimes.contains(&name) && parameter.required {
            return Err(refuse(format!(
                "feed sends `{name}` of `{id}` only on resume, but it is required"
            )));
        }
    }
    let always: BTreeSet<&str> = fixed
        .iter()
        .map(|(name, _)| *name)
        .chain(constants.iter().copied())
        .collect();
    if let Some(unbound) = operation
        .parameters
        .iter()
        .find(|p| p.required && !always.contains(p.name.as_str()))
    {
        return Err(refuse(format!(
            "feed leaves `{}` of `{id}` unbound, which it requires",
            unbound.name
        )));
    }
    Template::from_operation(operation).map_err(|refusal| refuse(refusal.reason()))
}

/// Each capability the declaration claims is the one the rest of it supports: the suite holds the
/// binding to its claim, so a claim stronger than the declaration cannot be honoured, and one
/// weaker would leave out scenarios the binding is able to pass.
fn supports(d: &Declaration) -> Result<()> {
    let claims = &d.capabilities;
    let public = d
        .containers
        .visibility
        .as_ref()
        .is_some_and(|rule| rule.map.values().any(|v| *v == Visibility::Public));
    let checks = [
        (
            claims.deletions == DeletionCapability::Observed,
            d.items.deleted.is_some(),
            "deletions: observed",
            "deletions: not-observed",
            "a tombstone is reported exactly when `items.deleted` is declared",
        ),
        (
            claims.kind == KindCapability::ProviderWord,
            d.containers.kind.is_some(),
            "kind: provider-word",
            "kind: fixed-word",
            "the kind is the provider's word exactly when `containers.kind` reads it, and a fixed word exactly when `containers.kind_word` states it",
        ),
        (
            claims.revision == RevisionCapability::UpdateTime,
            d.items.revision == d.items.updated_at,
            "revision: update-time",
            "revision: opaque",
            "the revision is the update time exactly when `items.revision` is `items.updated_at`",
        ),
        (
            claims.visibility == VisibilityCapability::Mapped,
            public,
            "visibility: mapped",
            "visibility: all-private",
            "visibility is mapped exactly when `containers.visibility` maps a value to `public`",
        ),
    ];
    for (claimed, supported, strong, weak, rule) in checks {
        if claimed != supported {
            let claim = if claimed { strong } else { weak };
            return Err(refuse(format!(
                "feed capabilities claim `{claim}`, which the declaration does not support: {rule}"
            )));
        }
    }
    Ok(())
}

impl Feed {
    pub(crate) fn new(bundle: &Bundle, base: &[String], declaration: &Declaration) -> Result<Self> {
        let d = declaration;
        if d.profile.is_empty()
            || d.profile.len() > 128
            || !d.profile.bytes().all(|b| b.is_ascii_graphic())
        {
            return Err(refuse("feed profile must be 1 to 128 visible ASCII bytes"));
        }
        if !(1..=100).contains(&d.max_limit) {
            return Err(refuse("feed max_limit must be between 1 and 100"));
        }
        let c = &d.containers;
        let i = &d.items;
        let ways = [
            c.cursor.next.is_some(),
            c.cursor.last.is_some(),
            c.cursor.offset.is_some(),
        ];
        if ways.iter().filter(|way| **way).count() != 1 {
            return Err(refuse(
                "feed listing continues by exactly one of `next` and `last`, or by `offset` alone",
            ));
        }
        if let Some(name) = &c.cursor.url_parameter {
            if c.cursor.next.is_none() {
                return Err(refuse(
                    "feed `containers.cursor.url_parameter` reads a `next` link, and the listing has none",
                ));
            }
            url_name("containers.cursor.url_parameter", name)?;
        }
        if c.kind.is_some() == c.kind_word.is_some() {
            return Err(refuse(
                "feed containers take exactly one of `kind` and `kind_word`",
            ));
        }
        if c.kind_word.as_ref().is_some_and(|word| {
            word.is_empty() || word.len() > 128 || !word.bytes().all(|b| b.is_ascii_graphic())
        }) {
            return Err(refuse(
                "feed kind_word must be 1 to 128 visible ASCII bytes",
            ));
        }
        for (name, value) in [
            ("containers.lookup.record", &c.lookup.record),
            ("containers.records", &c.records),
            ("containers.id", &c.id),
            ("items.records", &i.records),
            ("items.id", &i.id),
            ("items.revision", &i.revision),
            ("items.created_at", &i.created_at),
            ("items.updated_at", &i.updated_at),
            ("items.body.pointer", &i.body.pointer),
        ] {
            pointer(name, value)?;
        }
        for (name, value) in [
            ("containers.cursor.next", c.cursor.next.as_ref()),
            ("containers.cursor.last", c.cursor.last.as_ref()),
            (
                "containers.cursor.offset.complete.pointer",
                c.cursor.offset.as_ref().map(|o| &o.complete.pointer),
            ),
            ("containers.kind", c.kind.as_ref()),
            ("containers.name", c.name.as_ref()),
            (
                "containers.visibility",
                c.visibility.as_ref().map(|v| &v.pointer),
            ),
            ("items.author.id", i.author.as_ref().map(|a| &a.id)),
            (
                "items.author.display_name",
                i.author.as_ref().and_then(|a| a.display_name.as_ref()),
            ),
            ("items.url", i.url.as_ref()),
            ("items.url_base", i.url_base.as_ref()),
            ("items.parent", i.parent.as_ref()),
            ("items.deleted", i.deleted.as_ref().map(|c| &c.pointer)),
        ] {
            if let Some(value) = value {
                pointer(name, value)?;
            }
        }
        if i.body.representation.is_empty() {
            return Err(refuse("feed body names no representation"));
        }
        supports(d)?;
        let time = match &i.position {
            Position::Time(time) => Some(time),
            Position::Cursor(_) => None,
        };
        let query = time.and_then(|time| time.query.as_ref());
        if let Some(time) = time {
            if time.parameter.is_some() == time.query.is_some() {
                return Err(refuse(
                    "feed time position takes exactly one of `parameter` and `query`",
                ));
            }
            if time.query.is_some() && time.precision != Some(Precision::Minute) {
                return Err(refuse(
                    "feed time position: a query reads at `precision: minute`",
                ));
            }
            if let Some(page) = &time.page {
                pointer("items.position.page.next", &page.next)?;
                if let Some(name) = &page.url_parameter {
                    url_name("items.position.page.url_parameter", name)?;
                }
            }
        }
        if i.container.is_some() == query.is_some() {
            return Err(refuse(
                "feed `items.container` is absent exactly when a query selects the container",
            ));
        }
        let parsed = query
            .map(|query| -> Result<Query> {
                Ok(Query {
                    parameter: query.parameter.clone(),
                    first: template("items.position.query.first", &query.first, false)?,
                    resume: template("items.position.query.resume", &query.resume, true)?,
                })
            })
            .transpose()?;
        let mut sometimes: Vec<&str> = Vec::new();
        match &i.position {
            Position::Time(time) => {
                sometimes.extend(time.parameter.as_deref());
                sometimes.extend(time.page.as_ref().map(|page| page.parameter.as_str()));
            }
            Position::Cursor(cursor) => {
                pointer("items.position.next", &cursor.next)?;
                pointer("items.position.complete", &cursor.complete.pointer)?;
                if cursor
                    .stale
                    .iter()
                    .flatten()
                    .any(|status| !(400..500).contains(status))
                {
                    return Err(refuse("feed stale statuses must be refusals (4xx)"));
                }
                sometimes.push(&cursor.parameter);
            }
        }
        // The container is a path or query parameter; a query is a query parameter only.
        let mut always: Vec<(&str, bool)> = Vec::new();
        always.extend(i.container.as_deref().map(|name| (name, true)));
        always.push((i.limit.as_str(), false));
        always.extend(
            parsed
                .as_ref()
                .map(|query| (query.parameter.as_str(), false)),
        );
        let list = bind(
            bundle,
            base,
            &c.operation_id,
            &[(&c.limit, false)],
            &[&c.cursor.parameter],
            c.query.as_ref(),
        )?;
        let lookup = bind(
            bundle,
            base,
            &c.lookup.operation_id,
            &[(&c.lookup.parameter, true)],
            &[],
            None,
        )?;
        let items = bind(
            bundle,
            base,
            &i.operation_id,
            &always,
            &sometimes,
            i.query.as_ref(),
        )?;
        let (containers_declaration, items_declaration) = declarations(d);
        Ok(Self {
            declaration: d.clone(),
            base_len: base.len(),
            list,
            lookup,
            items,
            query: parsed,
            links: None,
            containers_declaration,
            items_declaration,
        })
    }

    /// The feed with root-relative item links resolved under the origin of `api_base`, the
    /// instance's `https` API base. A base with no such origin (a host that is not a hostname,
    /// such as a bracketed IP literal) leaves those links `null`; it never refuses the feed.
    pub(crate) fn with_link_origin(mut self, api_base: &str) -> Self {
        self.links = crate::origin(api_base);
        self
    }

    /// The base an answer's relative item links resolve under: the `http(s)` URL at `url_base`
    /// when declared (none when the answer carries no usable one: no host, user information, a
    /// query or fragment), otherwise the origin.
    fn link_base(&self, answer: &Value) -> Option<String> {
        let Some(pointer) = &self.declaration.items.url_base else {
            return self.links.clone();
        };
        let base = answer.pointer(pointer).and_then(Value::as_str)?;
        let authority = base
            .strip_prefix("https://")
            .or_else(|| base.strip_prefix("http://"))?;
        // A base without a host would let the link's own path name one.
        let authority = authority.split('/').next().unwrap_or_default();
        let host = match authority.strip_prefix('[') {
            Some(literal) => literal.split(']').next().unwrap_or_default(),
            None => authority.split(':').next().unwrap_or_default(),
        };
        let usable = !host.is_empty()
            && !authority.contains('@')
            && base.len() <= FIELD_BYTES
            && !base.contains(['?', '#', '\\'])
            && !base.chars().any(|c| c.is_whitespace() || c.is_control());
        usable.then(|| base.trim_end_matches('/').to_owned())
    }

    pub(crate) fn declarations(&self) -> [connectors_core::Operation; 2] {
        [
            self.containers_declaration.clone(),
            self.items_declaration.clone(),
        ]
    }

    /// One GET of a bound template; the answer as JSON, or the status that refused it.
    async fn get(
        &self,
        http: &dyn AuthenticatedHttp,
        template: &Template,
        values: Vec<(&str, String)>,
        constants: Option<&BTreeMap<String, String>>,
    ) -> Result<std::result::Result<Value, u16>> {
        let mut supplied: BTreeMap<String, Supplied> = BTreeMap::new();
        for (name, value) in constants.into_iter().flatten() {
            supplied.insert(name.clone(), Supplied::One(value.clone()));
        }
        for (name, value) in values {
            supplied.insert(name.to_owned(), Supplied::One(value));
        }
        let resolved = template
            .resolve_raw_values(&supplied)
            .map_err(|refusal| refuse(refusal.reason()))?;
        let segments: Vec<&str> = resolved.segments[self.base_len..]
            .iter()
            .map(String::as_str)
            .collect();
        let query: Vec<(&str, String)> = resolved
            .query
            .iter()
            .map(|(k, v)| (k.as_str(), v.clone()))
            .collect();
        let response = http.get(&segments, &query).await?;
        // A refusal about the request itself is the caller's to name; every other answer is
        // classified as the generic engine classifies it, and a malformed or unexpected answer
        // is the family's `unavailable`, never an empty page.
        if matches!(response.status, 400 | 404 | 409 | 410 | 412 | 422) {
            return Ok(Err(response.status));
        }
        match read_body(&response, false, &[]) {
            Ok(body) => Ok(Ok(body)),
            Err(error) if error.code == ErrorCode::UpstreamProtocol => Err(unavailable(
                "provider answered with a body this engine cannot read",
            )),
            Err(error) => Err(error),
        }
    }

    fn provenance(&self, instance: &str) -> Value {
        json!({
            "instance": instance,
            "profile": self.declaration.profile,
            "received_at": rfc3339(connectors_sdk::now_ms()),
        })
    }

    fn mark(&self, instance: &str, container: Option<&str>) -> Mark {
        Mark {
            format: MARK_FORMAT.into(),
            profile: self.declaration.profile.clone(),
            instance: instance.into(),
            container: container.map(str::to_owned),
            cursor: None,
            time: None,
            seen: Vec::new(),
        }
    }

    /// A mark this binding issued for this instance and container; anything else is stale.
    fn open(&self, token: &str, instance: &str, container: Option<&str>) -> Result<Mark> {
        if token.len() > WATERMARK_BYTES {
            return Err(stale());
        }
        let bytes = URL_SAFE_NO_PAD.decode(token).map_err(|_| stale())?;
        let mark: Mark = connectors_core::read_json(&bytes).map_err(|_| stale())?;
        let expected = self.mark(instance, container);
        let time = matches!(self.declaration.items.position, Position::Time(_));
        if mark.format != expected.format
            || mark.profile != expected.profile
            || mark.instance != expected.instance
            || mark.container != expected.container
            || (container.is_none() && (mark.time.is_some() || !mark.seen.is_empty()))
            || (container.is_some() && time && mark.cursor.is_some())
            || (container.is_some() && !time && (mark.time.is_some() || !mark.seen.is_empty()))
            || (mark.time.is_none() && !mark.seen.is_empty())
            || mark.time.as_deref().is_some_and(|t| instant(t).is_none())
        {
            return Err(stale());
        }
        Ok(mark)
    }

    fn issue(mark: &Mark) -> Result<String> {
        let token =
            URL_SAFE_NO_PAD.encode(serde_json::to_vec(mark).map_err(|_| Error::internal())?);
        if token.len() > WATERMARK_BYTES {
            return Err(unavailable(
                "the watermark would exceed its ceiling; too many items share one instant",
            ));
        }
        Ok(token)
    }

    fn bounded(page: Value) -> Result<Value> {
        let bytes = serde_json::to_vec(&page).map_err(|_| Error::internal())?;
        if bytes.len() > RESULT_BYTES {
            return Err(unavailable(
                "the page exceeds the result ceiling; read with a smaller limit",
            ));
        }
        Ok(page)
    }

    fn records<'a>(answer: &'a Value, pointer: &str) -> Result<&'a Vec<Value>> {
        answer
            .pointer(pointer)
            .and_then(Value::as_array)
            .ok_or_else(|| unavailable("provider answered without its list of records"))
    }

    fn visibility(&self, record: &Value) -> Visibility {
        self.declaration
            .containers
            .visibility
            .as_ref()
            .and_then(|rule| {
                record
                    .pointer(&rule.pointer)
                    .and_then(scalar)
                    .and_then(|value| rule.map.get(&value).copied())
            })
            .unwrap_or(Visibility::Private)
    }

    /// `feed.containers`: one provider page, direct conversations left out.
    pub(crate) async fn containers(
        &self,
        http: &dyn AuthenticatedHttp,
        instance: &str,
        input: Value,
    ) -> Result<Value> {
        connectors_sdk::validate(&self.containers_declaration.input_schema, &input)?;
        let c = &self.declaration.containers;
        let limit = input
            .get("limit")
            .and_then(Value::as_u64)
            .unwrap_or(self.declaration.max_limit);
        let cursor = match input.get("cursor").and_then(Value::as_str) {
            None => None,
            Some(token) => Some(self.open(token, instance, None)?.cursor.ok_or_else(stale)?),
        };
        // An offset cursor is the index of the page's first record, which this binding issued.
        // One whose next page could not be counted was not issued by this binding.
        let start = match (&c.cursor.offset, &cursor) {
            (Some(_), Some(cursor)) => cursor
                .parse::<u64>()
                .ok()
                .filter(|start| start.checked_add(limit).is_some())
                .ok_or_else(stale)?,
            _ => 0,
        };
        let mut values = vec![(c.limit.as_str(), limit.to_string())];
        if let Some(cursor) = cursor {
            values.push((c.cursor.parameter.as_str(), cursor));
        }
        let answer = self
            .get(http, &self.list, values, c.query.as_ref())
            .await?
            .map_err(refused)?;
        let records = Self::records(&answer, &c.records)?;
        let mut containers = Vec::new();
        for record in records {
            let visibility = self.visibility(record);
            if visibility == Visibility::Direct {
                continue;
            }
            let kind = match (&c.kind, &c.kind_word) {
                (Some(pointer), _) => required(record, pointer)?,
                (None, Some(word)) => word.clone(),
                (None, None) => return Err(Error::internal()),
            };
            containers.push(json!({
                "id": required(record, &c.id)?,
                "name": c.name.as_ref().and_then(|p| record.pointer(p)).and_then(Value::as_str),
                "kind": kind,
                "visibility": visibility.as_str(),
            }));
        }
        let next = match (&c.cursor.next, &c.cursor.last) {
            (Some(next), _) => continuation(&answer, next, c.cursor.url_parameter.as_deref())?,
            // The next page starts after every record this one carried, a direct one included;
            // the listing ends where the declared flag says, and an empty page the provider does
            // not call the last cannot be continued.
            (None, None) => match &c.cursor.offset {
                Some(offset) if offset.complete.holds(&answer) => None,
                Some(_) if records.is_empty() => {
                    return Err(unavailable(
                        "provider answered an empty page it does not call the last",
                    ));
                }
                Some(_) => Some(
                    start
                        .checked_add(records.len() as u64)
                        .ok_or_else(Error::internal)?
                        .to_string(),
                ),
                None => return Err(Error::internal()),
            },
            // A page shorter than the limit is the end. A full one continues after the key of
            // its last record, a direct one included; one without a key cannot be continued
            // and is never reported complete.
            (None, Some(last)) if records.len() as u64 >= limit => Some(
                records
                    .last()
                    .and_then(|record| record.pointer(last))
                    .and_then(scalar)
                    .filter(|key| !key.is_empty())
                    .ok_or_else(|| {
                        unavailable("provider answered a full page whose last record has no key")
                    })?,
            ),
            (None, _) => None,
        };
        let next_cursor = match next {
            None => None,
            Some(next) => {
                let mut mark = self.mark(instance, None);
                mark.cursor = Some(next);
                Some(Self::issue(&mark)?)
            }
        };
        Self::bounded(json!({
            "containers": containers,
            "complete": next_cursor.is_none(),
            "next_cursor": next_cursor,
            "provenance": self.provenance(instance),
        }))
    }

    /// `feed.items`: the container is admitted first (an unknown and an excluded direct one
    /// answer alike), then its watermark is read, then one provider list.
    pub(crate) async fn items(
        &self,
        http: &dyn AuthenticatedHttp,
        instance: &str,
        input: Value,
    ) -> Result<Value> {
        connectors_sdk::validate(&self.items_declaration.input_schema, &input)?;
        let container = input["container"].as_str().ok_or_else(Error::internal)?;
        let limit = input["limit"].as_u64().ok_or_else(Error::internal)?;
        // A container id a query cannot carry as a literal is refused before any request.
        if self.query.is_some() {
            quote(container)?;
        }
        let lookup = &self.declaration.containers.lookup;
        let answer = self
            .get(
                http,
                &self.lookup,
                vec![(lookup.parameter.as_str(), container.to_owned())],
                None,
            )
            .await?
            .map_err(|status| match status {
                404 | 410 => not_found(),
                status => refused(status),
            })?;
        let record = answer
            .pointer(&lookup.record)
            .ok_or_else(|| unavailable("provider answered without the container"))?;
        if self.visibility(record) == Visibility::Direct {
            return Err(not_found());
        }
        // Admission of the container precedes any disclosure, whether the watermark is valid
        // included: a watermark this read cannot resume is answered `stale_cursor` only once the
        // listing shows the connection can read the container. Until then it reads as a first
        // read, whose records are discarded, never returned.
        let (mark, resumable) = match input.get("watermark").and_then(Value::as_str) {
            None => (self.mark(instance, Some(container)), true),
            Some(token) => match self.open(token, instance, Some(container)) {
                Ok(mark) => (mark, true),
                Err(error) if error.code == ErrorCode::StaleCursor => {
                    (self.mark(instance, Some(container)), false)
                }
                Err(error) => return Err(error),
            },
        };
        let page = match &self.declaration.items.position {
            Position::Time(time) => self.by_time(http, container, limit, mark, time).await,
            Position::Cursor(cursor) => self.by_cursor(http, container, limit, mark, cursor).await,
        }
        // The lookup answered, so the connection sees the container; the provider's own refusal
        // of its items that is not a quota refusal (classified `rate_limited`) is a container the
        // connection cannot read, which the family answers exactly as an unknown one. A refusal
        // the host raised, not the provider, keeps its code.
        .map_err(|error| {
            if error.code == ErrorCode::Forbidden && error.upstream_answer {
                not_found()
            } else {
                error
            }
        })?;
        if !resumable {
            return Err(stale());
        }
        let (items, next, complete) = page;
        Self::bounded(json!({
            "items": items,
            "next_watermark": Self::issue(&next)?,
            "complete": complete,
            "provenance": self.provenance(instance),
        }))
    }

    async fn by_time(
        &self,
        http: &dyn AuthenticatedHttp,
        container: &str,
        limit: u64,
        mut mark: Mark,
        time: &TimeFilter,
    ) -> Result<(Vec<Value>, Mark, bool)> {
        let i = &self.declaration.items;
        let minute = time.precision == Some(Precision::Minute);
        // At minute precision the provider filters by the kept instant's minute and answers
        // that whole minute again; the watermark holds what was returned within it.
        let bucket = |at: i128| if minute { floor_minute(at) } else { at };
        let seen: BTreeSet<(String, String)> = mark.seen.iter().cloned().collect();
        let since = mark.time.as_deref().and_then(instant);
        let floor = since.map(bucket);
        // Every item returned in the kept instant or minute comes back with the filter; asking
        // for that many more keeps a page of unseen items within reach. A list read further by
        // its continuation reaches them on later pages.
        if time.page.is_none() && seen.len() as u64 >= self.declaration.max_limit {
            return Err(unavailable(
                "more items share one instant than one provider page holds",
            ));
        }
        let requested = (limit + seen.len() as u64).min(self.declaration.max_limit);
        let mut values = vec![(i.limit.as_str(), requested.to_string())];
        if let Some(name) = &i.container {
            values.push((name.as_str(), container.to_owned()));
        }
        match (&time.parameter, &self.query) {
            (Some(parameter), _) => {
                if let (Some(at), Some(text)) = (since, &mark.time) {
                    let bound = if minute {
                        format!("{}:00Z", civil_minute(at, 'T')?)
                    } else {
                        text.clone()
                    };
                    values.push((parameter.as_str(), bound));
                }
            }
            (None, Some(query)) => {
                let text = match since {
                    None => render(&query.first, container, None)?,
                    Some(at) => render(&query.resume, container, Some(&civil_minute(at, ' ')?))?,
                };
                values.push((query.parameter.as_str(), text));
            }
            (None, None) => return Err(Error::internal()),
        }
        let mut items = Vec::new();
        let mut last: Option<(i128, String)> = None;
        let mut in_bucket: Vec<(String, String)> = Vec::new();
        // The order is checked from the start of what the filter re-reads.
        let mut previous = floor;
        let mut unseen_left = false;
        let mut token: Option<String> = None;
        let mut pages = 0;
        let complete = loop {
            let mut sent = values.clone();
            if let (Some(page), Some(token)) = (&time.page, &token) {
                sent.push((page.parameter.as_str(), token.clone()));
            }
            let answer = self
                .get(http, &self.items, sent, i.query.as_ref())
                .await?
                .map_err(refused)?;
            pages += 1;
            let records = Self::records(&answer, &i.records)?;
            let base = self.link_base(&answer);
            for record in records {
                let mapped = self.item(record, base.as_deref())?;
                // The watermark is only as sound as the order: an answer out of order, or
                // before the filter, could put it past an item not yet returned.
                if previous.is_some_and(|previous| mapped.updated < previous) {
                    return Err(unavailable("provider answered out of updated_at order"));
                }
                previous = Some(mapped.updated);
                let pair = (mapped.id.clone(), mapped.revision.clone());
                let at = bucket(mapped.updated);
                if Some(at) == floor && seen.contains(&pair) {
                    continue;
                }
                if items.len() as u64 == limit {
                    unseen_left = true;
                    break;
                }
                if last.as_ref().is_none_or(|(kept, _)| *kept != at) {
                    in_bucket.clear();
                }
                last = Some((at, mapped.updated_text.clone()));
                in_bucket.push(pair);
                items.push(mapped.value);
            }
            let Some(page) = &time.page else {
                break (records.len() as u64) < requested && !unseen_left;
            };
            if unseen_left {
                break false;
            }
            let Some(next) = continuation(&answer, &page.next, page.url_parameter.as_deref())?
            else {
                break true;
            };
            if items.len() as u64 == limit {
                break false;
            }
            if pages == PAGES {
                if items.is_empty() {
                    return Err(unavailable(
                        "more items share one minute than the pages one read may request hold",
                    ));
                }
                break false;
            }
            token = Some(next);
        };
        if let Some((at, text)) = last {
            if Some(at) == floor {
                in_bucket.extend(seen);
            }
            in_bucket.sort();
            in_bucket.dedup();
            mark.time = Some(text);
            mark.seen = in_bucket;
        }
        Ok((items, mark, complete))
    }

    async fn by_cursor(
        &self,
        http: &dyn AuthenticatedHttp,
        container: &str,
        limit: u64,
        mut mark: Mark,
        cursor: &ChangeCursor,
    ) -> Result<(Vec<Value>, Mark, bool)> {
        let i = &self.declaration.items;
        // A change cursor has no query, so the load check left a container parameter.
        let name = i.container.as_deref().ok_or_else(Error::internal)?;
        let mut values = vec![
            (name, container.to_owned()),
            (i.limit.as_str(), limit.to_string()),
        ];
        if let Some(position) = &mark.cursor {
            values.push((cursor.parameter.as_str(), position.clone()));
        }
        let resumed = mark.cursor.is_some();
        let answer = self
            .get(http, &self.items, values, i.query.as_ref())
            .await?
            .map_err(|status| {
                if resumed && cursor.stale.iter().flatten().any(|s| *s == status) {
                    stale()
                } else {
                    refused(status)
                }
            })?;
        let records = Self::records(&answer, &i.records)?;
        let base = self.link_base(&answer);
        // The provider's cursor is past every record it answered, so none can be held back.
        if records.len() as u64 > limit {
            return Err(unavailable("provider answered more records than requested"));
        }
        let items = records
            .iter()
            .map(|record| {
                self.item(record, base.as_deref())
                    .map(|mapped| mapped.value)
            })
            .collect::<Result<Vec<_>>>()?;
        if let Some(next) = answer
            .pointer(&cursor.next)
            .and_then(scalar)
            .filter(|next| !next.is_empty())
        {
            mark.cursor = Some(next);
        }
        let complete = cursor.complete.holds(&answer);
        Ok((items, mark, complete))
    }

    fn item(&self, record: &Value, base: Option<&str>) -> Result<Mapped> {
        let i = &self.declaration.items;
        let id = required(record, &i.id)?;
        let revision = required(record, &i.revision)?;
        let created_at = time(record, &i.created_at)?.0;
        let (updated_text, updated) = time(record, &i.updated_at)?;
        let deleted = i.deleted.as_ref().is_some_and(|c| c.holds(record));
        let author = i.author.as_ref().and_then(|author| {
            let id = record.pointer(&author.id).and_then(scalar)?;
            let name = author
                .display_name
                .as_ref()
                .and_then(|p| record.pointer(p))
                .and_then(Value::as_str);
            Some(json!({"id": id, "display_name": name}))
        });
        let body = if deleted {
            Value::Null
        } else {
            match record.pointer(&i.body.pointer) {
                None | Some(Value::Null) => envelope(&i.body.representation, ""),
                Some(Value::String(text)) => envelope(&i.body.representation, text),
                Some(_) => return Err(unavailable("provider answered a body that is not text")),
            }
        };
        let url = i
            .url
            .as_ref()
            .and_then(|p| record.pointer(p))
            .and_then(Value::as_str)
            .and_then(|url| link(url, base));
        let parent = i
            .parent
            .as_ref()
            .and_then(|p| record.pointer(p))
            .and_then(scalar);
        Ok(Mapped {
            value: json!({
                "id": id,
                "revision": revision,
                "created_at": created_at,
                "updated_at": updated_text,
                "author": author,
                "body": body,
                "url": url,
                "parent": parent,
                "deleted": deleted,
            }),
            id,
            revision,
            updated,
            updated_text,
        })
    }
}

/// A provider refusal of a request as the family's code; a 404 or 410 is the container gone.
fn refused(status: u16) -> Error {
    let code = match status {
        400 | 409 | 412 | 422 => ErrorCode::InvalidInput,
        404 | 410 => ErrorCode::NotFound,
        _ => ErrorCode::Unavailable,
    };
    Error::new(code, "provider refused the request").answered()
}

struct Mapped {
    value: Value,
    id: String,
    revision: String,
    updated: i128,
    updated_text: String,
}

/// An item link: an `http(s)` URL as it is, or a path rooted at one `/` appended to `base`.
/// Anything else, and a path without a base, is no link.
fn link(url: &str, base: Option<&str>) -> Option<String> {
    if url.starts_with("https://") || url.starts_with("http://") {
        return Some(url.to_owned());
    }
    let base = base?;
    let rooted = url.starts_with('/')
        && !url.starts_with("//")
        && !url.contains('\\')
        && !url.chars().any(|c| c.is_whitespace() || c.is_control());
    rooted.then(|| format!("{base}{url}"))
}

/// A required scalar field of a record, as text.
fn required(record: &Value, pointer: &str) -> Result<String> {
    record
        .pointer(pointer)
        .and_then(scalar)
        .filter(|text| !text.is_empty() && text.len() <= FIELD_BYTES)
        .ok_or_else(|| unavailable("provider answered a record the declaration cannot read"))
}

/// A required instant and its position in time: RFC 3339 text as the provider wrote it, or an
/// instant with a `±hhmm` offset rewritten in UTC ([`compact_offset`]).
fn time(record: &Value, pointer: &str) -> Result<(String, i128)> {
    let text = required(record, pointer)?;
    let refused = || unavailable("provider answered an instant that is not RFC 3339");
    if let Some(at) = instant(&text) {
        return Ok((text, at));
    }
    let utc = compact_offset(&text).ok_or_else(refused)?;
    let at = instant(&utc).ok_or_else(refused)?;
    Ok((utc, at))
}

/// An RFC 3339 date-time but for an offset written `±hhmm`, as some providers write it,
/// rewritten in UTC with `Z`, its fraction kept; `None` for anything else.
fn compact_offset(text: &str) -> Option<String> {
    let (head, offset) = text.split_at_checked(text.len().checked_sub(5)?)?;
    let [sign, h1, h2, m1, m2] = offset.as_bytes() else {
        return None;
    };
    let sign: i128 = match sign {
        b'+' => 1,
        b'-' => -1,
        _ => return None,
    };
    let digits = [*h1, *h2, *m1, *m2];
    if !digits.iter().all(u8::is_ascii_digit) {
        return None;
    }
    let value = |a: u8, b: u8| i128::from(a - b'0') * 10 + i128::from(b - b'0');
    let (hours, minutes) = (value(*h1, *h2), value(*m1, *m2));
    if hours > 23 || minutes > 59 {
        return None;
    }
    let local = instant(&format!("{head}Z"))?;
    let at = local - sign * (hours * 3600 + minutes * 60) * 1_000_000_000;
    let seconds = i64::try_from(at.div_euclid(1_000_000_000)).ok()?;
    let fraction = head.get(19..)?;
    Some(format!("{}{fraction}Z", civil(seconds, 'T')))
}

/// The UTC date and time of `seconds` since the Unix epoch, `YYYY-MM-DD<separator>HH:MM:SS`.
fn civil(seconds: i64, separator: char) -> String {
    let (year, month, day) = date(seconds.div_euclid(86_400));
    let rem = seconds.rem_euclid(86_400);
    format!(
        "{year:04}-{month:02}-{day:02}{separator}{:02}:{:02}:{:02}",
        rem / 3600,
        rem % 3600 / 60,
        rem % 60
    )
}

/// The start of the UTC minute holding `at`, in nanoseconds since the Unix epoch.
fn floor_minute(at: i128) -> i128 {
    at - at.rem_euclid(60_000_000_000)
}

/// The UTC minute holding `at`, `YYYY-MM-DD<separator>HH:MM`: the time literal a minute query
/// sends, formatted from the parsed instant, never from the provider's text. `at` is a
/// watermark's instant; one whose UTC year is outside 0000 to 9999 has no literal of that shape,
/// so the watermark is stale.
fn civil_minute(at: i128, separator: char) -> Result<String> {
    let seconds = i64::try_from(floor_minute(at).div_euclid(1_000_000_000)).map_err(|_| stale())?;
    let (year, _, _) = date(seconds.div_euclid(86_400));
    if !(0..=9999).contains(&year) {
        return Err(stale());
    }
    let mut text = civil(seconds, separator);
    text.truncate(16);
    Ok(text)
}

/// The records body envelope of a text, clipped on a UTF-8 boundary.
fn envelope(representation: &str, text: &str) -> Value {
    let mut end = text.len().min(BODY_BYTES);
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    let truncated = end < text.len();
    json!({
        "representation": representation,
        "bytes": text.len(),
        "content": &text[..end],
        "truncated": truncated,
        "truncation": if truncated { vec!["content_bytes"] } else { Vec::new() },
    })
}

/// What a watermark or listing cursor carries; opaque to the consumer.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Mark {
    format: String,
    profile: String,
    instance: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    container: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    cursor: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    time: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    seen: Vec<(String, String)>,
}

// ---- declarations ----------------------------------------------------------------------------

fn declarations(d: &Declaration) -> (connectors_core::Operation, connectors_core::Operation) {
    let limit = json!({"type": "integer", "minimum": 1, "maximum": d.max_limit});
    let nullable = |schema: Value| json!({"anyOf": [{"type": "null"}, schema]});
    let provenance = json!({
        "type": "object",
        "properties": {
            "instance": {"type": "string"},
            "profile": {"type": "string"},
            "received_at": {"type": "string"},
        },
        "required": ["instance", "profile", "received_at"],
        "additionalProperties": false,
    });
    let container = json!({
        "type": "object",
        "properties": {
            "id": {"type": "string"},
            "name": {"type": ["string", "null"]},
            "kind": {"type": "string"},
            "visibility": {"enum": ["public", "private"]},
        },
        "required": ["id", "name", "kind", "visibility"],
        "additionalProperties": false,
    });
    let body = json!({
        "type": "object",
        "properties": {
            "representation": {"type": "string"},
            "bytes": {"type": ["integer", "null"], "minimum": 0},
            "content": {"type": "string"},
            "truncated": {"type": "boolean"},
            "truncation": {"type": "array", "items": {"type": "string"}},
        },
        "required": ["representation", "bytes", "content", "truncated", "truncation"],
        "additionalProperties": false,
    });
    let author = json!({
        "type": "object",
        "properties": {
            "id": {"type": "string"},
            "display_name": {"type": ["string", "null"]},
        },
        "required": ["id", "display_name"],
        "additionalProperties": false,
    });
    let item = json!({
        "type": "object",
        "properties": {
            "id": {"type": "string"},
            "revision": {"type": "string"},
            "created_at": {"type": "string"},
            "updated_at": {"type": "string"},
            "author": nullable(author),
            "body": nullable(body),
            "url": {"type": ["string", "null"]},
            "parent": {"type": ["string", "null"]},
            "deleted": {"type": "boolean"},
        },
        "required": ["id", "revision", "created_at", "updated_at", "author", "body", "url", "parent", "deleted"],
        "additionalProperties": false,
    });
    let operation =
        |id: &str, description: String, input: Value, output: Value| connectors_core::Operation {
            id: id.into(),
            description,
            contract: FEED_CONTRACT.into(),
            profile: d.profile.clone(),
            input_schema: input,
            output_schema: output,
        };
    (
        operation(
            CONTAINERS,
            format!(
                "List the containers this connection can be read from, under profile {}",
                d.profile
            ),
            json!({
                "type": "object",
                "properties": {"limit": limit, "cursor": {"type": "string"}},
                "additionalProperties": false,
            }),
            json!({
                "type": "object",
                "properties": {
                    "containers": {"type": "array", "items": container},
                    "next_cursor": {"type": ["string", "null"]},
                    "complete": {"type": "boolean"},
                    "provenance": provenance,
                },
                "required": ["containers", "next_cursor", "complete", "provenance"],
                "additionalProperties": false,
            }),
        ),
        operation(
            ITEMS,
            format!(
                "Read one container's items changed since a watermark, under profile {}",
                d.profile
            ),
            json!({
                "type": "object",
                "properties": {
                    "container": {"type": "string", "minLength": 1},
                    "watermark": {"type": "string"},
                    "limit": limit,
                },
                "required": ["container", "limit"],
                "additionalProperties": false,
            }),
            json!({
                "type": "object",
                "properties": {
                    "items": {"type": "array", "items": item},
                    "next_watermark": {"type": "string"},
                    "complete": {"type": "boolean"},
                    "provenance": provenance,
                },
                "required": ["items", "next_watermark", "complete", "provenance"],
                "additionalProperties": false,
            }),
        ),
    )
}

// ---- instants --------------------------------------------------------------------------------

/// Days since 1970-01-01 of a civil date (Howard Hinnant's algorithm).
fn days(year: i64, month: i64, day: i64) -> i64 {
    let year = if month <= 2 { year - 1 } else { year };
    let era = year.div_euclid(400);
    let yoe = year.rem_euclid(400);
    let doy = (153 * ((month + 9) % 12) + 2) / 5 + day - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

/// An RFC 3339 date-time as nanoseconds since the Unix epoch, or `None` when it is not one.
pub(crate) fn instant(text: &str) -> Option<i128> {
    let bytes = text.as_bytes();
    let number = |range: std::ops::Range<usize>| -> Option<i64> {
        let digits = bytes.get(range)?;
        digits
            .iter()
            .all(u8::is_ascii_digit)
            .then(|| std::str::from_utf8(digits).ok()?.parse().ok())?
    };
    if bytes.len() < 20
        || bytes[4] != b'-'
        || bytes[7] != b'-'
        || !matches!(bytes[10], b'T' | b't')
        || bytes[13] != b':'
        || bytes[16] != b':'
    {
        return None;
    }
    let (year, month, day) = (number(0..4)?, number(5..7)?, number(8..10)?);
    let (hour, minute, second) = (number(11..13)?, number(14..16)?, number(17..19)?);
    let leap = (year % 4 == 0 && year % 100 != 0) || year % 400 == 0;
    let month_days = [
        31,
        if leap { 29 } else { 28 },
        31,
        30,
        31,
        30,
        31,
        31,
        30,
        31,
        30,
        31,
    ];
    if !(1..=12).contains(&month)
        || day < 1
        || day > month_days[(month - 1) as usize]
        || hour > 23
        || minute > 59
        || second > 60
    {
        return None;
    }
    let mut rest = &text[19..];
    let mut nanos: i128 = 0;
    if let Some(fraction) = rest.strip_prefix('.') {
        let digits = fraction.bytes().take_while(u8::is_ascii_digit).count();
        if digits == 0 {
            return None;
        }
        for (index, digit) in fraction.bytes().take(digits).enumerate() {
            if index < 9 {
                nanos += i128::from(digit - b'0') * 10i128.pow(8 - index as u32);
            }
        }
        rest = &fraction[digits..];
    }
    let offset = match rest.as_bytes() {
        [b'Z' | b'z'] => 0,
        [sign @ (b'+' | b'-'), h1, h2, b':', m1, m2] => {
            let pair = |a: u8, b: u8| -> Option<i64> {
                (a.is_ascii_digit() && b.is_ascii_digit())
                    .then(|| i64::from(a - b'0') * 10 + i64::from(b - b'0'))
            };
            let (hours, minutes) = (pair(*h1, *h2)?, pair(*m1, *m2)?);
            if hours > 23 || minutes > 59 {
                return None;
            }
            let seconds = hours * 3600 + minutes * 60;
            if *sign == b'+' { seconds } else { -seconds }
        }
        _ => return None,
    };
    let seconds = days(year, month, day) * 86_400 + hour * 3600 + minute * 60 + second - offset;
    Some(i128::from(seconds) * 1_000_000_000 + nanos)
}

/// The civil date of a day count since 1970-01-01 (Howard Hinnant's algorithm).
fn date(days: i64) -> (i64, i64, i64) {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    (year, month, day)
}

/// `ms` since the Unix epoch as an RFC 3339 instant in UTC, millisecond precision.
fn rfc3339(ms: u64) -> String {
    let seconds = i64::try_from(ms / 1000).unwrap_or(i64::MAX);
    format!("{}.{:03}Z", civil(seconds, 'T'), ms % 1000)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_offset_without_a_colon_is_read_and_rewritten_in_utc() {
        let read = |text: &str| time(&json!({"at": text}), "/at").ok();
        let utc = |text: &str| (text.to_owned(), instant(text).unwrap());
        assert_eq!(
            read("2026-10-06T09:00:00.000+0000"),
            Some(utc("2026-10-06T09:00:00.000Z"))
        );
        assert_eq!(
            read("2026-10-06T11:30:05+0230"),
            Some(utc("2026-10-06T09:00:05Z"))
        );
        assert_eq!(
            read("2026-10-05T23:59:59.5-0130"),
            Some(utc("2026-10-06T01:29:59.5Z"))
        );
        assert_eq!(
            read("2026-01-01T00:15:00+0100"),
            Some(utc("2025-12-31T23:15:00Z"))
        );
        // RFC 3339 text keeps its own form.
        assert_eq!(
            read("2026-10-06T11:00:00+02:00"),
            Some((
                "2026-10-06T11:00:00+02:00".to_owned(),
                instant("2026-10-06T09:00:00Z").unwrap()
            ))
        );
        for bad in [
            "2026-10-06T09:00:00+2400",
            "2026-10-06T09:00:00+0060",
            "2026-10-06T09:00:00+02",
            "2026-10-06T09:00:00 0200",
            "2026-10-06T09:00:00Z0200",
            "2026-10-06T09:00+0200",
            "2026-02-30T09:00:00+0200",
            "+0200",
        ] {
            assert_eq!(read(bad), None, "{bad}");
        }
    }

    #[test]
    fn a_minute_is_formatted_from_the_parsed_instant_in_utc() {
        let at = |text: &str| instant(text).unwrap();
        assert_eq!(
            civil_minute(at("2026-10-06T09:00:59.999Z"), ' ').unwrap(),
            "2026-10-06 09:00"
        );
        assert_eq!(
            civil_minute(at("2026-10-06T01:00:00+02:00"), ' ').unwrap(),
            "2026-10-05 23:00"
        );
        assert_eq!(
            civil_minute(at("1969-12-31T23:59:30Z"), 'T').unwrap(),
            "1969-12-31T23:59"
        );
        assert_eq!(
            civil_minute(at("9999-12-31T23:59:59Z"), ' ').unwrap(),
            "9999-12-31 23:59"
        );
        assert_eq!(
            civil_minute(at("0000-01-01T00:00:00Z"), ' ').unwrap(),
            "0000-01-01 00:00"
        );
        for outside in ["9999-12-31T23:59:00-01:00", "0000-01-01T00:30:00+01:00"] {
            assert_eq!(
                civil_minute(at(outside), ' ').unwrap_err().code,
                ErrorCode::StaleCursor,
                "{outside}"
            );
        }
        assert_eq!(
            floor_minute(at("2026-10-06T09:00:30Z")),
            at("2026-10-06T09:00:00Z")
        );
    }

    #[test]
    fn a_next_link_yields_one_decoded_parameter_or_none() {
        let read = |url: &str| url_parameter(url, "start");
        assert_eq!(
            read("https://h.test/x?limit=1&start=p%3A2"),
            Some("p:2".into())
        );
        assert_eq!(read("/x?start=a+b%2Bc"), Some("a b+c".into()));
        assert_eq!(read("x?st%61rt=v#start=w"), Some("v".into()));
        assert_eq!(read("?start=%E2%9C%93"), Some("\u{2713}".into()));
        for none in [
            "https://h.test/x",
            "https://h.test/x?limit=1",
            "https://h.test/x#start=1",
            "/x?start=",
            "/x?start",
            "/x?start=1&start=2",
            "/x?start=%zz",
            "/x?start=%2",
            "/x?start=%FF",
            "/x?start=%+1",
        ] {
            assert_eq!(read(none), None, "{none}");
        }
        assert_eq!(
            read(&format!("/x?start={}", "a".repeat(WATERMARK_BYTES))),
            None
        );
    }

    #[test]
    fn a_query_value_is_one_literal_and_a_template_fills_in_one_pass() {
        assert_eq!(quote("plain").unwrap(), "\"plain\"");
        assert_eq!(quote(r#"a"b\c"#).unwrap(), r#""a\"b\\c""#);
        assert_eq!(quote(r#"\""#).unwrap(), r#""\\\"""#);
        for control in ["a\nb", "a\u{0}b", "\u{7f}", "\u{85}"] {
            assert!(quote(control).is_err(), "{control:?}");
        }
        let parts = template("t", "a = {container} AND b >= {since}", true).unwrap();
        // A value that looks like a slot stays text.
        assert_eq!(
            render(&parts, "{since}", Some("2026-10-06 09:00")).unwrap(),
            r#"a = "{since}" AND b >= "2026-10-06 09:00""#
        );
    }

    #[test]
    fn instants_are_read_as_rfc_3339_or_not_at_all() {
        let at = |text| instant(text);
        assert_eq!(at("1970-01-01T00:00:00Z"), Some(0));
        assert_eq!(at("1970-01-01T00:00:01.5Z"), Some(1_500_000_000));
        assert_eq!(at("2026-10-06T11:00:00+02:00"), at("2026-10-06T09:00:00Z"));
        assert_eq!(at("2026-10-06t09:00:00z"), at("2026-10-06T09:00:00Z"));
        assert!(at("2026-10-06T09:00:00.000001Z") > at("2026-10-06T09:00:00Z"));
        for bad in [
            "2026-10-06",
            "2026-10-06 09:00:00Z",
            "2026-13-06T09:00:00Z",
            "2026-02-30T09:00:00Z",
            "2026-10-06T09:00:00",
            "2026-10-06T09:00:00.Z",
            "2026-10-06T09:00:00+0200",
            "2026-10-06T24:00:00Z",
        ] {
            assert_eq!(at(bad), None, "{bad}");
        }
        assert_eq!(rfc3339(0), "1970-01-01T00:00:00.000Z");
        assert_eq!(
            instant(&rfc3339(1_791_277_200_123)),
            Some(1_791_277_200_123_000_000)
        );
    }
}
