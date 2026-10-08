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

/// How a listing continues, sent as `parameter`: the provider's own continuation read back at
/// `next` in the answer, or the key at `last` in the last record of a full page, for a provider
/// that lists after a key it is given. Exactly one of the two.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CursorPaging {
    pub parameter: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub next: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last: Option<String>,
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

/// An inclusive lower bound on `updated_at`, answered oldest change first.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TimeFilter {
    pub parameter: String,
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
    pub container: String,
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
        if c.cursor.next.is_some() == c.cursor.last.is_some() {
            return Err(refuse(
                "feed listing continues by exactly one of `next` and `last`",
            ));
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
        let resume = match &i.position {
            Position::Time(time) => &time.parameter,
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
                &cursor.parameter
            }
        };
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
            &[(&i.container, true), (&i.limit, false)],
            &[resume],
            i.query.as_ref(),
        )?;
        let (containers_declaration, items_declaration) = declarations(d);
        Ok(Self {
            declaration: d.clone(),
            base_len: base.len(),
            list,
            lookup,
            items,
            containers_declaration,
            items_declaration,
        })
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
            (Some(next), _) => answer
                .pointer(next)
                .and_then(scalar)
                .filter(|next| !next.is_empty()),
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
        let seen: BTreeSet<(String, String)> = mark.seen.iter().cloned().collect();
        // Every item returned at the kept instant comes back with the filter; asking for that
        // many more keeps a page of unseen items within reach.
        let requested = limit + seen.len() as u64;
        if seen.len() as u64 >= self.declaration.max_limit {
            return Err(unavailable(
                "more items share one instant than one provider page holds",
            ));
        }
        let requested = requested.min(self.declaration.max_limit);
        let mut values = vec![
            (i.container.as_str(), container.to_owned()),
            (i.limit.as_str(), requested.to_string()),
        ];
        let since = match &mark.time {
            Some(text) => {
                values.push((time.parameter.as_str(), text.clone()));
                instant(text)
            }
            None => None,
        };
        let answer = self
            .get(http, &self.items, values, i.query.as_ref())
            .await?
            .map_err(refused)?;
        let records = Self::records(&answer, &i.records)?;
        let mut items = Vec::new();
        let mut last: Option<(i128, String)> = None;
        let mut at_last: Vec<(String, String)> = Vec::new();
        let mut previous = since;
        let mut unseen_left = false;
        for record in records {
            let mapped = self.item(record)?;
            // The watermark is only as sound as the order: an answer out of order, or before
            // the filter, could put it past an item not yet returned.
            if previous.is_some_and(|previous| mapped.updated < previous) {
                return Err(unavailable("provider answered out of updated_at order"));
            }
            previous = Some(mapped.updated);
            let pair = (mapped.id.clone(), mapped.revision.clone());
            if Some(mapped.updated) == since && seen.contains(&pair) {
                continue;
            }
            if items.len() as u64 == limit {
                unseen_left = true;
                break;
            }
            if last.as_ref().is_none_or(|(at, _)| *at != mapped.updated) {
                at_last.clear();
            }
            last = Some((mapped.updated, mapped.updated_text.clone()));
            at_last.push(pair);
            items.push(mapped.value);
        }
        if let Some((at, text)) = last {
            if Some(at) == since {
                at_last.extend(seen);
            }
            at_last.sort();
            at_last.dedup();
            mark.time = Some(text);
            mark.seen = at_last;
        }
        let complete = (records.len() as u64) < requested && !unseen_left;
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
        let mut values = vec![
            (i.container.as_str(), container.to_owned()),
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
        // The provider's cursor is past every record it answered, so none can be held back.
        if records.len() as u64 > limit {
            return Err(unavailable("provider answered more records than requested"));
        }
        let items = records
            .iter()
            .map(|record| self.item(record).map(|mapped| mapped.value))
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

    fn item(&self, record: &Value) -> Result<Mapped> {
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
            .filter(|url| url.starts_with("https://") || url.starts_with("http://"));
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

/// A required scalar field of a record, as text.
fn required(record: &Value, pointer: &str) -> Result<String> {
    record
        .pointer(pointer)
        .and_then(scalar)
        .filter(|text| !text.is_empty() && text.len() <= FIELD_BYTES)
        .ok_or_else(|| unavailable("provider answered a record the declaration cannot read"))
}

/// A required RFC 3339 instant: the provider's text and its position in time.
fn time(record: &Value, pointer: &str) -> Result<(String, i128)> {
    let text = required(record, pointer)?;
    let at = instant(&text)
        .ok_or_else(|| unavailable("provider answered an instant that is not RFC 3339"))?;
    Ok((text, at))
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

/// `ms` since the Unix epoch as an RFC 3339 instant in UTC, millisecond precision.
fn rfc3339(ms: u64) -> String {
    let secs = ms / 1000;
    let days = (secs / 86_400) as i64;
    let rem = secs % 86_400;
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    format!(
        "{year:04}-{month:02}-{day:02}T{:02}:{:02}:{:02}.{:03}Z",
        rem / 3600,
        rem % 3600 / 60,
        rem % 60,
        ms % 1000
    )
}

#[cfg(test)]
mod tests {
    use super::*;

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
