use super::*;

/// A parsed and validated `providers/<name>.toml`.
///
/// The [`connector`](Self::connector) is complete and ready for codegen either way. A
/// hand-authored file describes it inline; a spec-backed one loaded through [`load_with_spec`] has
/// had ingest fill it in from the vendored document. Loaded through plain [`load`], a spec-backed
/// file is still the *skeleton* it always was — id, base URL, credentials, provenance, plus any
/// operations written inline — because no document was supplied to ingest.
#[derive(Debug, Clone, PartialEq)]
pub struct LoadedProvider {
    /// The connector this file describes.
    pub connector: Connector,
    /// The vendor documents the file points at, in the order it declares them — C-410.
    ///
    /// Empty for a fully hand-authored connector. A single `[spec]` block is one entry, which is why
    /// the plural costs the single-document form nothing: `[spec]` and `[[spec]]` are two spellings
    /// of one field, and the loader treats the first as the one-element case of the second.
    pub specs: Vec<SpecSource>,
    /// The patch set applied over the ingested specs. Empty for a hand-authored connector.
    pub patch: Patch,
    /// What each vendored document said, when documents were supplied to [`load_with_spec`] — C-4,
    /// widened to several by C-410.
    ///
    /// The **whole** ingest of each, not just the part that was published: every operation the
    /// document declares including the ones no patch selected, plus the servers it names and every
    /// [`Diagnostic`](crate::openapi::Diagnostic) it earned. That is what makes "ingest makes
    /// everything *available* to patch" inspectable rather than merely claimed — and it is what a
    /// future `connectors check` reads to tell an author which operations they could have
    /// selected.
    ///
    /// **One entry per document, never one merged whole.** Merging is exactly what this story
    /// exists to refuse: babelforce's manager document declares root `oauth2` with zero operation
    /// overrides while `task-automation` declares `bearerAuth`+`oauth2` on all 31 of its operations,
    /// and one field holding "the ingest" would have let whichever was read last describe both.
    ///
    /// Empty for a hand-authored connector, and also for a spec-backed one loaded through plain
    /// [`load`], which is given no document to ingest.
    pub ingested: Vec<IngestedDocument>,
    /// AsyncAPI component messages made available by event-source documents. Like OpenAPI ingest,
    /// this is the complete selectable set, not only the events a patch published.
    pub ingested_events: Vec<IngestedEventDocument>,
    /// Members whose TOML table omitted `service` before serde normalized that omission to
    /// [`DEFAULT_SERVICE`]. Needed only for C-458's mixed legacy-default shape, where explicit
    /// `service = "default"` and omission must remain different authoring decisions.
    pub(super) implicit_service_members: Vec<ImplicitServiceMember>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct ImplicitServiceMember {
    pub(super) kind: &'static str,
    pub(super) name: String,
}

impl LoadedProvider {
    /// Whether this file is a complete hand-authored definition — no spec, so nothing to ingest and
    /// nothing to overlay.
    pub fn is_hand_authored(&self) -> bool {
        self.specs.is_empty()
    }

    /// Everything wrong with the vendored documents that did not stop their ingest.
    ///
    /// Empty for a hand-authored connector. A real vendor document is never fully well-formed, so
    /// this being non-empty is the normal case, not a failure — see [`crate::openapi`].
    pub fn diagnostics(&self) -> Vec<&crate::openapi::Diagnostic> {
        self.ingested
            .iter()
            .flat_map(|document| document.ingested.diagnostics.iter())
            .collect()
    }

    /// The ingest of the document that joined `service`, if the file declared one.
    pub fn ingested_for(&self, service: &str) -> Option<&IngestedDocument> {
        self.ingested
            .iter()
            .find(|document| document.service == service)
    }

    /// The AsyncAPI ingest that joined `service`, if one was supplied.
    pub fn ingested_events_for(&self, service: &str) -> Option<&IngestedEventDocument> {
        self.ingested_events
            .iter()
            .find(|document| document.service == service)
    }
}

/// One vendored document, ingested, and the service its operations join — C-410.
#[derive(Debug, Clone, PartialEq)]
pub struct IngestedDocument {
    /// The repository-relative path the `[[spec]]` entry pinned.
    pub path: String,
    /// The service this document's selected operations belong to.
    ///
    /// [`DEFAULT_SERVICE`](crate::DEFAULT_SERVICE) when the entry names none, which is what keeps a
    /// single `[spec]` block meaning exactly what it meant before this field existed.
    pub service: String,
    /// Everything the document declares.
    pub ingested: crate::openapi::Ingested,
}

/// One AsyncAPI document and the service its selected messages join.
#[derive(Debug, Clone, PartialEq)]
pub struct IngestedEventDocument {
    pub path: String,
    pub service: String,
    pub ingested: crate::asyncapi::Ingested,
}

/// Which pure front-end parses a pinned source document.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SpecKind {
    #[default]
    Openapi,
    Asyncapi,
}

impl SpecKind {
    const fn is_openapi(&self) -> bool {
        matches!(self, Self::Openapi)
    }

    pub(super) const fn label(self) -> &'static str {
        match self {
            Self::Openapi => "OpenAPI",
            Self::Asyncapi => "AsyncAPI",
        }
    }
}

/// Where one vendor document for this connector lives, and which service it becomes.
///
/// The path is into the **vendored, committed** cache under `specs/`, never a URL to fetch at build
/// time: builds are hermetic and offline (AGENTS.md). `source_url` records where the bytes came
/// from so C-14 can re-fetch and diff, and `sha256` is what makes that diff a fact rather than a
/// guess.
///
/// # Provenance is per document, not per connector — C-410
///
/// A connector may declare several documents, and each carries **its own** `sha256`, `fetched_at`
/// and `upstream_version`. babelforce's five documents were pulled on two different dates and three
/// of them publish `info.version = "0.0.0-dev"`; one hash for the connector could not say which of
/// them moved, which is the only question a drift check is asked. So this whole struct is what
/// reaches [`Provenance::specs`](crate::Provenance::specs), one entry per document.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SpecSource {
    /// The vendored spec file, relative to the repository root
    /// (`specs/babelforce/manager-2026-07-10.openapi.yaml`).
    pub path: String,
    /// The source grammar. Omitted means OpenAPI for every previously published provider.
    #[serde(default, skip_serializing_if = "SpecKind::is_openapi")]
    pub kind: SpecKind,
    /// The [`Service`] this document's selected operations join — C-410.
    ///
    /// Absent means the reserved [`DEFAULT_SERVICE`](crate::DEFAULT_SERVICE), which is what a single
    /// `[spec]` block meant before this key existed and must keep meaning. A named value must be one
    /// a `[[services]]` entry declares, checked by the same pass that checks an inline operation's
    /// `service` — a document is not a declaration of a service, it joins one.
    ///
    /// **This is what makes several documents a partition rather than a pile.** Two documents may
    /// declare the same `operationId` — `getUser` genuinely exists in babelforce's
    /// `manager-2026-07-10` and `user-2026-06-25`, and they are different calls — so an id is only
    /// unambiguous inside one document's service. Every [`OperationPatch`] therefore resolves
    /// against exactly one of these.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub service: Option<String>,
    /// The URL the spec was fetched from, recorded for drift-check.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_url: Option<String>,
    /// The upstream version string the vendor published (`info.version`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub upstream_version: Option<String>,
    /// SHA-256 of the vendored bytes.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sha256: Option<String>,
    /// When the spec was fetched, RFC 3339.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fetched_at: Option<String>,
}

impl SpecSource {
    /// The service this document's operations join — [`DEFAULT_SERVICE`](crate::DEFAULT_SERVICE)
    /// when the entry names none.
    pub fn service(&self) -> &str {
        self.service.as_deref().unwrap_or(DEFAULT_SERVICE)
    }
}

/// The patch set applied over an ingested spec — C-6's input, widened to statements about sets by
/// C-411, C-412 and C-414.
///
/// **Selection is opt-in**, which is why there is no `hide`. A 163-operation spec must not become
/// 163 LLM tools (`predecessor:docs/designs/provider-operation-inventory.md` §5.2 selects 9 of them), and an
/// opt-out list would make every new upstream operation a new tool by default. Only operations a
/// [`OperationPatch::select`] names or an [`OperationSelector`] matches reach the connector, and a
/// selector widens what one *statement* selects without making anything default-selected.
///
/// # The merge order, stated once
///
/// **spec → select → per-operation patch → validate**, and it is total:
///
/// 1. ingest turns each document into every operation the vendor declares;
/// 2. every [`OperationSelector`] states what it states about the set it matched, and two selectors
///    that state different values for one operation are refused rather than ordered;
/// 3. identity-stable maps state reviewed fields by service and vendor operation id;
/// 4. the [`OperationPatch`] that names an operation overrides the selector **field by field** —
///    where the block is silent the identity-stable map or selector's statement stands, and where
///    none speaks the rules on each field decide;
/// 5. the result is validated by exactly the pass a hand-authored operation goes through.
///
/// The published order follows from the same sentence: operations a `[[patch.operations]]` block
/// names publish in file order, then everything a selector matched publishes in document order, per
/// `[[spec]]` entry. Fixed, so identical inputs produce byte-identical IR — and so a file that
/// declares no selector publishes exactly what it published before selectors existed.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Patch {
    /// The statements that select **sets** of operations — C-411.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub select: Vec<OperationSelector>,
    /// How an `operationId` becomes an op id, declared once — C-412.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub naming: Option<Naming>,
    /// Reviewed direction keyed by stable spec identity: service, then vendor `operationId`.
    ///
    /// Unlike a selector this map cannot change membership when an upstream method, path, name or
    /// description changes. Quoted operation ids are ordinary TOML keys:
    /// `[patch.directions.manager]` followed by `flushDialer = "write"`.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub directions: BTreeMap<String, BTreeMap<String, OperationDirection>>,
    /// Source-grounded description corrections keyed by stable service and vendor `operationId`.
    /// This map exists for bulk-selected documents whose source omits descriptions. Unlike an exact
    /// operation patch it changes no selection or publication order.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub descriptions: BTreeMap<String, BTreeMap<String, String>>,
    /// The operations selected one at a time, each with its corrections.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub operations: Vec<OperationPatch>,
    /// AsyncAPI component messages selected one at a time. There is deliberately no bulk event
    /// selector: an inbound firehose deserves an exact reviewed subscription inventory.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub events: Vec<EventPatch>,
}

impl Patch {
    /// Whether the file carries no patches at all.
    pub fn is_empty(&self) -> bool {
        self.select.is_empty()
            && self.naming.is_none()
            && self.directions.is_empty()
            && self.descriptions.is_empty()
            && self.operations.is_empty()
            && self.events.is_empty()
    }

    /// How to spell the block an author would go and edit, for a refusal about a patch set with no
    /// `[spec]` to apply to.
    ///
    /// Names what the file actually wrote rather than the commonest key: a message about
    /// `[[patch.operations]]` sends someone who only wrote a selector looking for a block they never
    /// authored.
    pub(super) fn declared(&self) -> &'static str {
        if !self.directions.is_empty() {
            "[patch.directions]"
        } else if !self.descriptions.is_empty() {
            "[patch.descriptions]"
        } else if !self.operations.is_empty() {
            "[[patch.operations]]"
        } else if !self.events.is_empty() {
            "[[patch.events]]"
        } else if !self.select.is_empty() {
            "[[patch.select]]"
        } else {
            "[patch.naming]"
        }
    }
}

/// One exact event selected from an AsyncAPI component message.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EventPatch {
    /// Service of the AsyncAPI source document. Required when more than one source is pinned.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub service: Option<String>,
    /// Component-message key in `components.messages`.
    pub select: String,
    /// Stable catalog event name. Absent keeps the message's declared `name`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rename: Option<String>,
    /// Exact transport discriminator value when it differs from the stable event name.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub wire_value: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub group: Option<String>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub when: BTreeMap<String, JsonSchema>,
    /// Credential capability requirements, independent of channel transport auth/verification.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub auth: Option<Vec<AuthRequirement>>,
}

/// One statement that selects a **set** of operations — C-411.
///
/// ```toml
/// [[patch.select]]
/// service = "manager"
/// path_prefix = "/api/v2/agents"
/// methods = ["GET"]
/// risk = "low"
/// idempotency = "idempotent"
/// expose = false
/// ```
///
/// # Why this exists
///
/// [`OperationPatch`] selects exactly one `operationId`. For babelforce's canonical surface that is
/// **397** blocks, each carrying a `select`, a `rename`, a `risk` and an `idempotency` before any
/// real correction — a file nobody reviews, which means a file in which nobody notices a wrong
/// safety claim. A selector is the same statements at the grain they are actually true at: one risk
/// for 50 DELETEs, one exposure decision for the 388 operations that are callable without being
/// tools.
///
/// # What it does *not* do
///
/// It does not make anything default-selected. A file with no selector and no
/// `[[patch.operations]]` publishes nothing, and there is no `hide`: an opt-out list would make
/// every operation a vendor adds upstream a tool by default, learned about from a model's behaviour
/// rather than from a diff.
///
/// A selector that matches nothing is a **loud error**, for the same reason
/// [`OperationPatch::select`] naming an absent `operationId` is: a prefix that stops matching after
/// an upstream reshuffle would quietly empty the connector and the build would stay green.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OperationSelector {
    /// **Which document this selector reads** — the `service` of one `[[spec]]` entry (C-410).
    ///
    /// Absent is legal only when the file declares exactly one document, and means that one. The
    /// rule is [`OperationPatch::service`]'s and for the same reason: a path prefix is no more
    /// unique across documents than an `operationId` is.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub service: Option<String>,
    /// The path prefix an operation's path must carry, matched on **whole segments**.
    ///
    /// `/api/v2/agents` reaches `/api/v2/agents` and `/api/v2/agents/{id}` and does not reach
    /// `/api/v2/agentsummary` — a prefix that matched half a segment would select by spelling
    /// accident, which is the opposite of a statement.
    ///
    /// Absent means every path in the document. That is a real case (a document that *is* one
    /// resource namespace) and still an explicit statement, so it stays legal.
    ///
    /// **Path prefix rather than tag**: `Manager` tags 309 of the manager document's 356
    /// operations, while 47 distinct three-segment prefixes reproduce the SDK's 36 resource
    /// namespaces almost exactly. The vendor's tags describe the docs site; its paths describe the
    /// API.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path_prefix: Option<String>,
    /// The HTTP methods to match. Empty means every method.
    ///
    /// Splitting a prefix by method is how one `risk` covers a set honestly: the reads and the
    /// deletes under one prefix are not one damage claim.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub methods: Vec<HttpMethod>,
    /// The [`Risk`] every matched operation carries — C-414.
    ///
    /// **Silence on an authored write refuses the build.** See [`Self::idempotency`] for the whole
    /// rule, which is one rule for both fields.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub risk: Option<Risk>,
    /// The [`Idempotency`] every matched operation carries — C-414.
    ///
    /// # Silence refuses on a write and is answered on a read, and that asymmetry is the point
    ///
    /// No OpenAPI document publishes either field, so 214 of babelforce's 398 operations need both
    /// stated by someone. Deriving them from the HTTP method is the failure mode this repository has
    /// legislated against twice ([`Risk`] has no `Default`; C-186 made `conditional` state its
    /// condition or not build), because a default that *flatters* turns 214 unmade decisions into
    /// 214 claims a host reads as a licence.
    ///
    /// So: a matched operation whose identity-stable direction is `write` and about which neither
    /// this selector nor a `[[patch.operations]]` block says anything is **refused, by name**. A
    /// matched operation authored as `read` takes `low` and `idempotent` — not a method-derived
    /// direction or a flattering write default, but the only absent safety values a reviewed read
    /// can receive without widening its authority.
    ///
    /// The asymmetry belongs to **selection**, which is a statement about a set that may mix
    /// methods. A `[[patch.operations]]` block is a statement about one operation, and it still
    /// states both — one line, on the operation an author is already looking at.
    ///
    /// # `conditional` is not made bulk by this
    ///
    /// A selector may state `idempotency = "conditional"`, and every matched mutating operation
    /// then still owes the stated `repeatable_because` C-186 requires — which no selector can
    /// supply for many operations at once, because one sentence about 54 endpoints is not a
    /// condition. So the build refuses, per operation. A bulk escape hatch around C-186 is the one
    /// thing this field must not become.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub idempotency: Option<Idempotency>,
    /// Host-resource consequences shared by the reviewed selection.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub effects: Option<Vec<HostEffect>>,
    /// Lifecycle shared by the reviewed selection.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub interaction_shape: Option<InteractionShape>,
    /// Closed protocol implementation shared by the reviewed selection.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub protocol_driver: Option<ProtocolDriver>,
    /// Placement requirement shared by the reviewed selection.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub placement_requirement: Option<PlacementRequirement>,
    /// Implementation form shared by the reviewed selection.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub implementation_form: Option<ImplementationForm>,
    /// Required capabilities shared by the reviewed selection.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub required_capabilities: Option<Vec<RequiredCapability>>,
    /// Whether every matched operation reaches a model as a tool — C-413's [`Operation::expose`],
    /// declared for a set.
    ///
    /// Absent means the field's own default, which is **exposed**: silence here decides nothing, and
    /// nothing-decided must keep meaning what the repository already does. Declaring the inverse
    /// per operation is 388 lines for babelforce, which is the whole reason this key is here.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expose: Option<bool>,
}

/// How an `operationId` becomes an op id, declared once — C-412.
///
/// ```toml
/// [patch.naming]
/// rule = "kebab"                     # listReportingCalls -> babelforce-list-reporting-calls
/// prefix = "babelforce"
/// [patch.naming.pin]                 # the escape hatch, and the only per-op naming cost
/// listAgents = "babelforce-agent-list"
/// ```
///
/// # Why a rule is allowed to exist beside "op naming is a public contract"
///
/// It is not allowed to exist *instead* of it. `predecessor:docs/designs/connector-pipeline.md` refuses ids
/// "derived from volatile spec fields like `operationId` without a pinned override" — and this is
/// the pinned override, made bulk. Three properties are what make it safe, and all three are
/// enforced rather than intended:
///
/// - the rule is **declared**, so it is reviewable as one line rather than inferred per operation;
/// - **collisions refuse** — two `operationId`s deriving one op id is an error, never
///   last-write-wins, because the loser would silently become unreachable under a name a user or a
///   model still calls;
/// - a derived id that is not a legal flux `decl_name` is **reported, naming the operation**, never
///   mangled into something that happens to parse.
///
/// The remaining half is a test, not a type: `tests/operation_selection.rs` pins the full derived
/// id set for a fixture, so an upstream `operationId` rename moves an op id **loudly**.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Naming {
    /// The derivation to apply. Required: a rule that could be omitted would be a rule decided by
    /// silence, and silence must not name a public contract.
    pub rule: NamingRule,
    /// Prepended to every derived id, joined with `-`. Absent means no prefix.
    ///
    /// In practice this is the connector id, because an op id is global: `babelforce` +
    /// `listReportingCalls` is `babelforce-list-reporting-calls`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prefix: Option<String>,
    /// `operationId` → the op id to publish it as, overriding [`Self::rule`].
    ///
    /// This is where the ids a connector already ships are held still while everything around them
    /// is derived — the nine `providers/babelforce.toml` publishes today are exactly that case.
    ///
    /// **Keyed by `operationId` alone**, which is unique inside one document and nowhere else. A
    /// key two of the connector's documents both declare is refused rather than applied twice; the
    /// way to name one of them is a `[[patch.operations]]` block with a `rename`, which is
    /// service-qualified and outranks a pin.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub pin: BTreeMap<String, String>,
}

/// The declared derivation from `operationId` to op id.
///
/// A closed enum with one variant today, so a second rule is a deliberate addition with its own
/// review rather than a string the loader interprets — and so a typo is refused by serde naming
/// every rule that exists.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NamingRule {
    /// `listReportingCalls` → `list-reporting-calls`, with case boundaries becoming `-`.
    ///
    /// Acronyms keep their shape: `listHTTPCalls` → `list-http-calls`, because the boundary is read
    /// at the *end* of a run of capitals rather than at every capital.
    Kebab,
}

impl Naming {
    /// The op id this declaration gives `operation_id`, or the reason it gives none.
    ///
    /// A pin answers directly; otherwise the rule derives one and the result is held to the same
    /// grammar an authored `rename` is. The `Err` is the *reason*, phrased to be pasted into a
    /// refusal that has already named the operation.
    pub(super) fn derive(&self, operation_id: &str) -> std::result::Result<String, String> {
        if let Some(pinned) = self.pin.get(operation_id) {
            let pinned = pinned.trim();
            return legal_op_id(pinned).map(|()| pinned.to_owned());
        }

        let stem = match self.rule {
            NamingRule::Kebab => kebab(operation_id),
        };
        let derived = match self
            .prefix
            .as_deref()
            .map(str::trim)
            .filter(|p| !p.is_empty())
        {
            Some(prefix) => format!("{prefix}-{stem}"),
            None => stem,
        };
        legal_op_id(&derived).map(|()| derived)
    }
}

impl OperationSelector {
    /// Whether this selector matches one of a document's operations.
    ///
    /// The `internal` guard is **not** here: matching and eligibility are different questions, and
    /// a selector that matched only internal paths must still be reported as matching nothing
    /// rather than as matching something it then dropped.
    pub(super) fn matches(&self, operation: &crate::openapi::SpecOperation) -> bool {
        if !self.methods.is_empty() && !self.methods.contains(&operation.method) {
            return false;
        }
        match self.path_prefix.as_deref().map(str::trim) {
            Some(prefix) => path_has_prefix(&operation.path, prefix),
            None => true,
        }
    }

    /// How the selector reads back in a refusal — the statement, not an index nobody can find.
    pub(super) fn describe(&self) -> String {
        let mut parts = Vec::new();
        if let Some(service) = self.service.as_deref() {
            parts.push(format!("service = {service:?}"));
        }
        if let Some(prefix) = self.path_prefix.as_deref() {
            parts.push(format!("path_prefix = {prefix:?}"));
        }
        if !self.methods.is_empty() {
            let methods: Vec<&str> = self.methods.iter().copied().map(method_word).collect();
            parts.push(format!("methods = {methods:?}"));
        }
        if parts.is_empty() {
            "`[[patch.select]]` (stating nothing)".to_owned()
        } else {
            format!("`[[patch.select]] {}`", parts.join(", "))
        }
    }
}

/// Whether `path` lies under `prefix`, matched on **whole segments**.
///
/// `/api/v2/agents` covers `/api/v2/agents/{id}` and not `/api/v2/agentsummary`. Without the
/// boundary a prefix would select by spelling accident, and the accident would be invisible: the
/// extra operations arrive silently and correctly-shaped.
fn path_has_prefix(path: &str, prefix: &str) -> bool {
    let prefix = prefix.trim_end_matches('/');
    if prefix.is_empty() {
        return true;
    }
    path == prefix
        || path
            .strip_prefix(prefix)
            .is_some_and(|rest| rest.starts_with('/'))
}

/// The path segment no selection may ever reach.
///
/// Zero of the 398 operations babelforce's five documents declare carry it, which is exactly why it
/// is here: this is a guard against a *future* pull, and the moment a vendor publishes an internal
/// endpoint a broad selector would otherwise catalogue it as a supported call. Costing one check to
/// keep that impossible is the trade.
const INTERNAL_SEGMENT: &str = "internal";

/// Whether a path names the vendor's own internals.
pub(super) fn is_internal(path: &str) -> bool {
    path.split('/').any(|segment| segment == INTERNAL_SEGMENT)
}

/// `listReportingCalls` → `list-reporting-calls`.
///
/// The boundary is read at the end of a run of capitals rather than at every capital, so
/// `listHTTPCalls` is `list-http-calls` and not `list-h-t-t-p-calls`. Characters that are neither
/// letters nor digits are **passed through unchanged** rather than substituted: the result is then
/// held to the `decl_name` grammar, so an `operationId` that cannot produce a legal id is reported
/// as itself instead of being silently mangled into something that parses.
fn kebab(operation_id: &str) -> String {
    let chars: Vec<char> = operation_id.chars().collect();
    let mut out = String::with_capacity(operation_id.len() + 8);
    for (index, ch) in chars.iter().copied().enumerate() {
        if !ch.is_ascii_uppercase() {
            out.push(ch);
            continue;
        }
        let follows_a_word = index > 0
            && (chars[index - 1].is_ascii_lowercase() || chars[index - 1].is_ascii_digit());
        let ends_an_acronym = index > 0
            && chars[index - 1].is_ascii_uppercase()
            && chars.get(index + 1).is_some_and(char::is_ascii_lowercase);
        if follows_a_word || ends_an_acronym {
            out.push('-');
        }
        out.push(ch.to_ascii_lowercase());
    }
    out
}

/// Whether `id` is a name flux can declare and `connector-pack` can project onto a tool name.
///
/// The charset is `flux_lang`'s `decl_name` grammar (C-8) — ASCII alphanumerics, `_` and `-` — and
/// the empty-level rule is `connector_pack::dotted_name`'s, because an id with a `--` in it becomes
/// a dotted tool name with an empty level. Re-stated here rather than imported: `connector-spec`
/// takes neither dependency, and this crate is where a bad id must be refused, since by the time
/// the emitter sees one the file that produced it is three layers away.
pub(super) fn legal_op_id(id: &str) -> std::result::Result<(), String> {
    if id.is_empty() {
        return Err("it is empty".to_owned());
    }
    if let Some(offender) = id
        .chars()
        .find(|ch| !ch.is_ascii_alphanumeric() && *ch != '_' && *ch != '-')
    {
        return Err(format!(
            "it holds {offender:?}, and flux-lang's `decl_name` grammar admits ASCII \
             alphanumerics, `_` and `-` only"
        ));
    }
    if id.starts_with('-') || id.ends_with('-') || id.contains("--") {
        return Err(
            "it has an empty `-`-separated level, so `connector-pack` cannot project it onto a \
             dotted tool name"
                .to_owned(),
        );
    }
    Ok(())
}

/// One operation selected from the vendor spec, and everything the author corrects about it.
///
/// Every override is an `Option` so that "not stated" stays distinguishable from "stated as the
/// value that happens to equal the spec's" — the overlay must be able to tell whether the author
/// made a decision, because a spec that later changes underneath an unstated field should follow
/// the spec, while a stated one must not move.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OperationPatch {
    /// **Which document this patch reads** — the `service` of one `[[spec]]` entry (C-410).
    ///
    /// Absent is legal only when the file declares exactly one document, where it means that one.
    /// The moment a second is declared, every patch states this, and the reason is that `select`
    /// stops being a unique key: `getUser` is declared by babelforce's `manager-2026-07-10` **and**
    /// by its `user-2026-06-25`, as two different requests. Resolving an unqualified `select`
    /// against whichever document declared it would compile one of the two by accident and emit
    /// plausible, wrong Flux — so the loader refuses instead of choosing.
    ///
    /// It is also the [`Service`] the published operation lands in, because the two are the same
    /// statement: a document becomes a service, and a patch selects out of a document.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub service: Option<String>,
    /// The spec's `operationId` this patch selects, e.g. `listReportingCalls`.
    pub select: String,
    /// Withhold this exact operation from a set selected in bulk, with the reason review needs.
    ///
    /// This is deliberately **not** operation selection by exclusion: it is legal only when a
    /// `[[patch.select]]` already matched the operation. The selector remains the positive review
    /// boundary; this field records why one member of that stated set cannot publish yet. Nothing
    /// else may be corrected on a deferred operation because no corrected operation would exist.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub defer: Option<String>,
    /// The stable op id to publish it as, e.g. `babelforce.call.list`.
    ///
    /// Almost always set: `operationId` is a volatile vendor field and the op name is a public
    /// contract users and models call by name
    /// (`predecessor:docs/designs/connector-pipeline.md`, "Op naming is a public contract").
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rename: Option<String>,
    /// Replaces the spec's `summary`/`description` as the model-facing tool description.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// States vendor-state direction on this exact stable operation identity. If the directions map
    /// also states it, the two values must agree or loading refuses.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub direction: Option<OperationDirection>,
    /// Overrides the risk the spec implies. Specs do not carry risk, so in practice this is where
    /// risk is *stated*, not overridden.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub risk: Option<Risk>,
    /// Overrides idempotency. As with `risk`, specs do not publish it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub idempotency: Option<Idempotency>,
    /// Exact host-resource consequences. Overrides a selector declaration.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub effects: Option<Vec<HostEffect>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub interaction_shape: Option<InteractionShape>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub protocol_driver: Option<ProtocolDriver>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub placement_requirement: Option<PlacementRequirement>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub implementation_form: Option<ImplementationForm>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub required_capabilities: Option<Vec<RequiredCapability>>,
    /// Semantic consequences stated by the author who reviewed this operation. A vendor document
    /// cannot infer business meaning, and selectors cannot state one value for a heterogeneous set.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub semantic_effects: Option<Vec<SemanticEffect>>,
    /// Overrides the operation's auth alternatives.
    ///
    /// The `Option` carries the same three-way meaning as [`Operation::auth`]: absent means "leave
    /// whatever ingest extracted", `[]` means "this operation needs no auth", and a non-empty list
    /// replaces the extracted set. Babelforce's excluded header pair
    /// (`provider-operation-inventory.md` §5.1.3) is exactly this: ingest must keep seeing it, and
    /// the overlay is the only place it may be removed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub auth: Option<Vec<AuthRequirement>>,
    /// Pagination to attach when the vendor document does not state it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pagination: Option<Pagination>,
    /// Published rate limit to attach when the vendor document does not state it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rate_limit: Option<RateLimit>,
    /// Structured vendor error envelope to attach when the vendor document does not state it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error_envelope: Option<ErrorEnvelope>,
    /// Parameter-level corrections: a wrong type, a false `required`, a missing description.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub params: Vec<ParamPatch>,
    /// Parameters the connector **drops** from what the document declares — C-422.
    ///
    /// Applied after [`params`](Self::params), so requiredness is judged as the connector states it
    /// rather than as the vendor guessed it: an author may correct a wrong `required` flag and then
    /// omit the parameter, and the two statements read in that order.
    #[serde(default, skip_serializing_if = "ParamOmission::is_empty")]
    pub omit: ParamOmission,
    /// Overrides whether this operation reaches a model as a tool — C-413's [`Operation::expose`].
    ///
    /// The counterpart of [`OperationSelector::expose`], and the reason both exist: a selector
    /// states the rule for a set (`expose = false` over 388 operations) and a block states the
    /// exception (`expose = true` on the curated nine). Absent means whatever the selector that
    /// matched this operation said, and exposed if none did.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expose: Option<bool>,
}

/// A correction to one parameter of a selected operation.
///
/// Identified by `name` **and** `position`, because a vendor may bind the same name in two places
/// and because the position is what decides where the value travels on the request.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ParamPatch {
    /// The parameter name as the vendor spec declares it.
    pub name: String,
    /// Where on the request it travels.
    pub position: ParamPosition,
    /// Corrects the vendor's `required` flag. Freshdesk's collection marks a path parameter
    /// optional, which produces `PUT /tickets/` when it is omitted
    /// (`provider-operation-inventory.md` §6.4).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub required: Option<bool>,
    /// Replaces the vendor's description.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Replaces the vendor's JSON Schema for this parameter — the pressure valve for a spec that
    /// types a date as a bare string.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub schema: Option<JsonSchema>,
}

/// The parameters a selected operation **drops**, named by position and then by name — C-422.
///
/// # Why this exists at all
///
/// A vendor document is written to describe an API, not to be a tool contract. babelforce's
/// `listReportingCalls` declares 38 query parameters, of which the vendor's own prose marks most as
/// aliases of the others (`fromNumber` *of* `from`, and a whole `filters.`-prefixed restatement of
/// the set). A model choosing arguments out of 38 synonyms chooses worse than one choosing out of
/// 14, and before this existed the only way back to 14 was to abandon the document and hand-write
/// the operation — which C-416 measured as the single place where hand-authoring beat patching
/// across an entire converted provider.
///
/// # Why this is not a contradiction of `Patch` having no `hide`
///
/// [`Patch`] refuses an operation-level opt-out because **selection is opt-in**: a `hide` list would
/// make every operation a vendor adds upstream a new tool by default, and the author would learn
/// about it from a model's behaviour rather than from a diff. That argument does not reach one level
/// down, and lands the opposite way, *because the operation is already selected*. An author writing
/// here has stated intent about this endpoint and is **narrowing** it — not opting out of reviewing
/// it — and a new upstream parameter still arrives in the tool by default, exactly as an operation
/// does.
///
/// # Why it is a list of names rather than a flag on `ParamPatch`
///
/// Dropping is not correcting: there is nothing else to say about a parameter that is going away, so
/// a three-line block per name would cost 51 lines to remove babelforce's 17 synonyms and hand a
/// reviewer 51 lines that all say the same thing. Grouping by position keeps the identity
/// [`ParamPatch`] uses — name **and** position, because a vendor may bind one name in two places —
/// and costs one line per group plus the names.
///
/// **Every omission is written down**, which is the property that survives regeneration: nothing
/// here is inferred from a description, a naming convention or a similarity between two parameters,
/// because none of those is a decision anybody made.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ParamOmission {
    /// Path parameters to drop. Permitted only when the same service declares an exact
    /// operator-pinned `path.<name>` configuration value; otherwise see [`omit`] for the refusal.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub path: Vec<String>,
    /// Query-string parameters to drop. The synonym flood lives here.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub query: Vec<String>,
    /// Caller-supplied headers to drop.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub header: Vec<String>,
    /// Named request-body fields to drop.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub body: Vec<String>,
}

impl ParamOmission {
    /// Whether the patch drops nothing.
    pub fn is_empty(&self) -> bool {
        self.path.is_empty()
            && self.query.is_empty()
            && self.header.is_empty()
            && self.body.is_empty()
    }

    /// Every omission as the pair that identifies it, in a fixed group order.
    ///
    /// The order is fixed rather than incidental because it is the order the refusals come out in,
    /// and a loader that reported the same file's problems in a different order on a different run
    /// would fail the determinism test this crate keeps.
    pub fn entries(&self) -> impl Iterator<Item = (ParamPosition, &str)> {
        [
            (ParamPosition::Path, &self.path),
            (ParamPosition::Query, &self.query),
            (ParamPosition::Header, &self.header),
            (ParamPosition::Body, &self.body),
        ]
        .into_iter()
        .flat_map(|(position, names)| names.iter().map(move |name| (position, name.as_str())))
    }
}

/// Where a parameter travels on the request. Mirrors the groups of [`ParamSet`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum ParamPosition {
    /// Interpolated into the path template.
    Path,
    /// A query-string parameter.
    Query,
    /// A caller-supplied request header.
    Header,
    /// A field of the JSON request body.
    Body,
}

/// The wire shape of `providers/<name>.toml`.
///
/// This is the *only* type that names the file's top-level keys, and it deserializes the connector
/// fields straight into the IR types rather than into shadows of them. Two consequences worth
/// stating:
///
/// - there is no translation layer to drift out of sync with the IR;
/// - `deny_unknown_fields` on the IR types is therefore what makes the *file* strict — the loader
///   could not add that from outside, which is C-2's review finding restated as a design constraint.
///
/// It is private: [`load`] is the entry point, and returning a validated [`LoadedProvider`] rather
/// than a raw parse is the point of having a loader at all.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct ProviderFile {
    pub(super) id: String,
    #[serde(default)]
    pub(super) authority: Option<String>,
    #[serde(default)]
    pub(super) api_version: Option<String>,
    #[serde(default)]
    pub(super) services: Vec<Service>,
    #[serde(default)]
    pub(super) vendor: String,
    pub(super) base_url: String,
    #[serde(default)]
    pub(super) description: String,
    #[serde(default)]
    pub(super) auth: Vec<AuthMethod>,
    #[serde(default)]
    pub(super) default_auth: Vec<AuthRequirement>,
    #[serde(default)]
    pub(super) const_headers: BTreeMap<String, String>,
    #[serde(default)]
    pub(super) operations: Vec<Operation>,
    #[serde(default)]
    pub(super) events: Vec<EventDecl>,
    #[serde(default)]
    pub(super) channels: Vec<ChannelBinding>,
    #[serde(default)]
    pub(super) discoveries: Vec<Discovery>,
    #[serde(default)]
    pub(super) config: Vec<ConfigField>,
    #[serde(default)]
    pub(super) verify: Option<String>,
    #[serde(default)]
    pub(super) graphs: Vec<Graph>,
    /// `[spec]` **or** `[[spec]]` — C-410.
    ///
    /// One key, two TOML spellings, because a connector with one vendor document and a connector
    /// with five are the same thing at different sizes. The single-table form is the one-element
    /// case and is spelled `[spec]` forever: converting the 53 shipped providers to array syntax to
    /// buy a plural nobody asked for would be churn, and the golden errors pin the single form's
    /// messages verbatim.
    #[serde(rename = "spec", default, deserialize_with = "one_or_many_specs")]
    pub(super) specs: Vec<SpecSource>,
    #[serde(default)]
    pub(super) patch: Patch,
}

/// Accepts `[spec]` as a table and `[[spec]]` as an array of them, into one `Vec`.
///
/// Written as a visitor rather than as `#[serde(untagged)]` on purpose. An untagged enum buffers the
/// input and reports `data did not match any variant of untagged enum`, which throws away both the
/// `deny_unknown_fields` key list and `toml`'s line, column and source snippet — and this loader's
/// error text is a deliverable pinned by golden files. Dispatching on the visited shape keeps the
/// inner type's own error, whichever form the author wrote.
fn one_or_many_specs<'de, D>(deserializer: D) -> std::result::Result<Vec<SpecSource>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::de::value::{MapAccessDeserializer, SeqAccessDeserializer};

    struct OneOrMany;

    impl<'de> serde::de::Visitor<'de> for OneOrMany {
        type Value = Vec<SpecSource>;

        fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            formatter.write_str("a `[spec]` table or a sequence of `[[spec]]` tables")
        }

        fn visit_map<A>(self, map: A) -> std::result::Result<Self::Value, A::Error>
        where
            A: serde::de::MapAccess<'de>,
        {
            SpecSource::deserialize(MapAccessDeserializer::new(map)).map(|spec| vec![spec])
        }

        fn visit_seq<A>(self, seq: A) -> std::result::Result<Self::Value, A::Error>
        where
            A: serde::de::SeqAccess<'de>,
        {
            Vec::deserialize(SeqAccessDeserializer::new(seq))
        }
    }

    deserializer.deserialize_any(OneOrMany)
}
