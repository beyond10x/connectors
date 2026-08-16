//! The normalized connector IR: the single shape both front-ends produce and codegen consumes.
//!
//! Everything here is plain data with a serde encoding. Two properties are load-bearing and are
//! asserted by the tests next door rather than left to good intentions:
//!
//! 1. **Types survive.** Every [`Param`] and the response carry a real [`JsonSchema`], not a
//!    stringly-typed shadow of one. The cautionary tale is action-proxy's YAML, where `type:
//!    string` stood in for dates and ids alike and no schema ever reached the caller.
//! 2. **Serialization is deterministic.** Identical values encode to identical bytes.
//!    `connectors.lock` (C-7) hashes this encoding and `connectors check` fails on a
//!    mismatch, so any leaked iteration order would surface as phantom drift on every build.
//!
//! # Why these types are strict on deserialization
//!
//! C-2 originally left them permissive, on the theory that "validation lives in the loader (C-3),
//! not here". C-2's review disproved it. The provider loader parses `providers/*.toml` straight into
//! these types, so a key the derived `Deserialize` does not recognize is discarded *before* any
//! loader-level check can see it — the strictness cannot be bolted on from outside. Two concrete
//! failures were demonstrated, and both fail in the dangerous direction:
//!
//! - a mistyped `authh` on an operation yields [`Operation::auth`] = `None`, which means *inherit
//!   the connector default*, so the operation authenticates with the connector's default
//!   credentials rather than the narrower set the author meant to name. The failure direction is
//!   credential-**sending**, not fail-closed;
//! - a mistyped `envv` on a credential yields an empty `env` list with no error at all.
//!
//! So every struct here carries `#[serde(deny_unknown_fields)]`, and
//! [`AuthMethod::scheme`](crate::AuthMethod::scheme) lost its `#[serde(default)]` for the same
//! reason [`Risk`] and [`Idempotency`] have no `Default`: how a secret reaches the wire is not a
//! decision to make by silence. `tests/strict_fields.rs` pins all of it.
//!
//! *Semantic* validation — a credential named by no declaration, a degenerate empty auth mechanism,
//! a `basic` scheme with no user half — still lives in the loader ([`crate::provider`]), because it
//! is cross-field reasoning serde cannot express.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::auth::{AuthMethod, AuthRequirement};
use crate::config::ConfigField;
use crate::graph::Graph;
use crate::inbound::{ChannelBinding, EventDecl};

/// A JSON Schema, carried verbatim.
///
/// JSON Schema *is* JSON, and the pipeline's job is to move a vendor's schema from a spec document
/// into an op contract without reinterpreting it — so a faithful [`serde_json::Value`] is both the
/// simplest and the most honest representation. Modelling a subset in Rust would silently drop
/// every keyword the subset missed, which is exactly the failure this field exists to prevent.
///
/// It is also deterministic: `serde_json::Map` is a `BTreeMap` unless `serde_json/preserve_order`
/// is enabled, so object keys serialize in sorted order regardless of how the document was parsed.
/// `tests/determinism.rs::serde_json_object_keys_stay_sorted` is the tripwire that fails if any
/// future dependency turns that feature on.
pub type JsonSchema = serde_json::Value;

/// **Whether a schema admits every document it could be checked against** — C-126, C-417.
///
/// An empty object, a bare `true`, or a schema stating a `type` and no members: each of these
/// counts as "declares a response schema" while telling a consumer exactly what absence tells them.
/// That makes a placeholder *worse* than nothing, because absence is reviewable and a placeholder
/// is indistinguishable from a real declaration.
///
/// # One predicate, two callers, and that is the point
///
/// `crates/connector-spec/tests/response_schema_coverage.rs` refuses an operation whose published
/// schema satisfies this, and [`openapi::ingest`](crate::openapi) refuses to *derive* one — a
/// vendor's placeholder must not become this repository's coverage any more than an author's may.
/// Those are the same judgement about the same shapes, and two copies of it would be two statements
/// of one fact with only one of them machine-checked, which is the duplication
/// [`MIN_REPEATABILITY_CONDITION`] documents this crate legislating against.
///
/// The keyword list is what *narrows* a document rather than what describes one: `properties`,
/// `items`, `required` and `$ref` name members; `oneOf`/`anyOf`/`allOf` compose schemas that do;
/// `const` pins a value. A schema carrying none of them has a name for its type and nothing else.
pub fn constrains_nothing(schema: &JsonSchema) -> bool {
    let Some(object) = schema.as_object() else {
        // A bare `true` is the JSON Schema spelling of "anything".
        return schema.as_bool() == Some(true);
    };
    if object.is_empty() {
        return true;
    }

    const INFORMATIVE: [&str; 8] = [
        "properties",
        "items",
        "required",
        "$ref",
        "oneOf",
        "anyOf",
        "allOf",
        "const",
    ];
    !INFORMATIVE.iter().any(|key| object.contains_key(*key))
}

/// The HTTP method an operation issues.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum HttpMethod {
    /// `GET`.
    Get,
    /// `POST`.
    Post,
    /// `PUT`.
    Put,
    /// `PATCH`.
    Patch,
    /// `DELETE`.
    Delete,
    /// `HEAD`.
    Head,
    /// `OPTIONS`.
    Options,
}

/// Whether executing an operation reads vendor state or may change it.
///
/// This is connector truth, not HTTP syntax. A vendor may expose a mutating `GET` or a proven
/// read over `POST`; neither [`HttpMethod`] nor any other metadata field supplies a default.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OperationDirection {
    /// The operation only observes vendor state.
    Read,
    /// The operation may change vendor state.
    Write,
}

impl OperationDirection {
    /// The stable spelling published by manifests and catalogues.
    pub const fn word(self) -> &'static str {
        match self {
            Self::Read => "read",
            Self::Write => "write",
        }
    }
}

/// How much damage an operation can do, in flux's own vocabulary (`flux_spec::Risk`).
///
/// There is deliberately **no `Default`**. flux's approval gate reads this, so letting the field be
/// omitted would mean a safety decision made by silence — an operation that forgot to declare
/// itself destructive would be waved through as low risk. Both front-ends must state it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Risk {
    /// Reads, and writes that cannot surprise anyone.
    Low,
    /// Writes with limited blast radius.
    Medium,
    /// Writes a reviewer would want to see first.
    High,
    /// Deletes or otherwise irreversible.
    Destructive,
}

/// Whether repeating the operation is safe, in flux's own vocabulary (`flux_spec::Idempotency`).
///
/// No `Default`, for the same reason as [`Risk`]: this is what tells flux whether a `retry` around
/// the request is sound, and guessing on the operation's behalf is how a retry turns one charge
/// into three.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Idempotency {
    /// Repeating the call has the same effect as making it once — **and the call may be skipped
    /// entirely in favour of a stored result.**
    ///
    /// That second clause is not a gloss; it is what the value licenses downstream, and this
    /// repository had it wrong. `flux_spec::coherence`'s **I3, the repeatability floor**, is
    /// explicit: `Idempotent` "is what licenses the dispatcher's op cache to serve a stored result
    /// *instead of executing*", and a consequence-bearing spec must not declare it. So `Idempotent`
    /// is a claim about a **read**, not a claim that a write is replay-safe. Reserve it for
    /// operations that change nothing.
    Idempotent,
    /// Repeating the call repeats its effect.
    NonIdempotent,
    /// **Safe to repeat under a stated condition** — flux's own escape hatch for a mutation that
    /// really is replay-safe.
    ///
    /// The wording is flux's: I3 says the honest declaration for a mutating operation is
    /// `NonIdempotent`, "or [`Conditional`](Self::Conditional) when it is genuinely safe to repeat
    /// under stated conditions. `Conditional` — not a loosened rule — is the escape hatch for
    /// 'safely repeatable'."
    ///
    /// **This doc comment used to read "idempotent only under a condition the caller supplies (e.g.
    /// an idempotency key)", and that narrowing cost three connectors their honest declaration**
    /// (C-186). Cloudflare's cache purge, LaunchDarkly's flag toggle and Miro's sticky-note update
    /// are each safely repeatable because of what the *endpoint* does — a target state rather than
    /// a delta — with no key for a caller to supply. Reading `Conditional` as "caller supplies a
    /// key" put all three outside a value that was always meant for them, and each shipped
    /// `NonIdempotent` with a comment saying the opposite. A condition the caller supplies is one
    /// kind of stated condition; it was never the only kind.
    ///
    /// The condition itself is [`Operation::repeatable_because`], and stating it is mandatory on an
    /// authored write — flux says *stated* conditions, and a condition recorded nowhere is not one.
    Conditional,
}

/// What executing an operation *means*, in Flux's semantic-effect vocabulary.
///
/// This is deliberately distinct from the host-resource effects emitted in the Flux declaration.
/// Every connector operation reaches the network, but only some move money, delete irreversibly,
/// or send something to a third party. The compiler crates do not depend on `flux-spec`, so this
/// enum mirrors the non-deprecated `flux_spec::FlowEffect` wire tags and `connector-pack` checks the
/// tags against Flux again at the runtime seam.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SemanticEffect {
    /// Deterministic and side-effect free. Refused on connector operations, which make HTTP calls.
    Pure,
    /// Reads external state.
    Read,
    /// Invokes a model.
    Model,
    /// General network egress.
    Network,
    /// Writes a file.
    WriteFile,
    /// Writes persistent state.
    WriteDb,
    /// Sends something to a third party.
    SendExternal,
    /// Irreversibly deletes.
    Delete,
    /// Moves money.
    Money,
    /// Produces output a human will see.
    HumanVisible,
}

impl SemanticEffect {
    /// The stable Flux tag written to manifests and catalogues.
    pub const fn tag(self) -> &'static str {
        match self {
            Self::Pure => "pure",
            Self::Read => "read",
            Self::Model => "model",
            Self::Network => "network",
            Self::WriteFile => "write_file",
            Self::WriteDb => "write_db",
            Self::SendExternal => "send_external",
            Self::Delete => "delete",
            Self::Money => "money",
            Self::HumanVisible => "human_visible",
        }
    }

    /// Whether Flux treats this as a consequence that outlives the call.
    pub const fn is_consequential(self) -> bool {
        matches!(
            self,
            Self::Model
                | Self::WriteFile
                | Self::WriteDb
                | Self::SendExternal
                | Self::Delete
                | Self::Money
        )
    }
}

/// Host-resource consequences that admission evaluates.
///
/// This is deliberately separate from [`SemanticEffect`]: `network` says which host resource a
/// call touches, while `money` or `send_external` says what the call means. The list is authored on
/// an exact operation or reviewed selector and resolved before publication; no consumer derives it
/// from direction, HTTP method, host, risk, or protocol driver.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HostEffect {
    Read,
    Write,
    Network,
    Process,
    Browser,
    Filesystem,
    LocalSystem,
}

impl HostEffect {
    pub const fn tag(self) -> &'static str {
        match self {
            Self::Read => "read",
            Self::Write => "write",
            Self::Network => "network",
            Self::Process => "process",
            Self::Browser => "browser",
            Self::Filesystem => "filesystem",
            Self::LocalSystem => "local_system",
        }
    }
}

/// Lifecycle exposed by one connector member.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InteractionShape {
    Unary,
    Stream,
    Subscription,
    LeasedSession,
    SessionEstablishment,
}

/// Closed, versioned protocol implementation used to speak to the external system.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProtocolDriver {
    HttpV1,
    SipV1,
    /// Local audio devices. Closed, built into connectors, and reachable only where a person is —
    /// a speaker exists at a satellite placement, never in a hosted deployment.
    AudioV1,
    /// A browser, spoken to over the Chrome `DevTools` Protocol. Closed and built into connectors.
    /// The external system is the browser process the driver launched on its own dedicated
    /// profile; page, element and navigation meaning stay here rather than moving to Substrate.
    CdpV1,
}

/// Driver-specific request facts for one operation.
///
/// This is flattened into the canonical IR so existing HTTP operations retain their wire spelling
/// (`protocol_driver`, `method`, `path`) while the Rust type makes an incoherent combination such
/// as `sip_v1` plus an HTTP method impossible to construct. A SIP operation deliberately has no
/// HTTP request fields: its bounded destination and call parameters live in [`Operation::params`]
/// and are interpreted only by the closed SIP driver after admission.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "protocol_driver", rename_all = "snake_case")]
pub enum OperationRequest {
    /// One HTTP request template.
    HttpV1 {
        /// The HTTP method.
        method: HttpMethod,
        /// The path template relative to the service base URL.
        path: String,
    },
    /// One admitted SIP call-establishment request.
    SipV1,
    /// One admitted local-audio request. Like SIP it carries no HTTP-shaped fields: the bounded
    /// utterance is an ordinary declared parameter, and every device fact — synthesizer, voice,
    /// sink — is deployment-owned and resolved only after admission.
    AudioV1,
    /// One admitted browser request on a leased session. It carries no HTTP-shaped fields either:
    /// the browser's own HTTP traffic is the driver's, never this operation's request template, and
    /// the executable, dedicated profile directory and artifact directory are deployment-owned
    /// facts resolved only after admission.
    CdpV1,
}

impl OperationRequest {
    /// The closed driver selected by this request shape.
    pub const fn driver(&self) -> ProtocolDriver {
        match self {
            Self::HttpV1 { .. } => ProtocolDriver::HttpV1,
            Self::SipV1 => ProtocolDriver::SipV1,
            Self::AudioV1 => ProtocolDriver::AudioV1,
            Self::CdpV1 => ProtocolDriver::CdpV1,
        }
    }

    /// The HTTP method when this is an HTTP request.
    pub const fn http_method(&self) -> Option<HttpMethod> {
        match self {
            Self::HttpV1 { method, .. } => Some(*method),
            Self::SipV1 | Self::AudioV1 | Self::CdpV1 => None,
        }
    }

    /// The HTTP path when this is an HTTP request.
    pub fn http_path(&self) -> Option<&str> {
        match self {
            Self::HttpV1 { path, .. } => Some(path),
            Self::SipV1 | Self::AudioV1 | Self::CdpV1 => None,
        }
    }
}

/// Where an operation must be served; deployment policy chooses a matching instance later.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PlacementRequirement {
    ConnectorsDeployment,
    SubstrateWorkload,
    FederatedSatellite,
}

/// How the protocol implementation is supplied.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ImplementationForm {
    BuiltIn,
}

/// Host authority an operation needs before any credential is materialized.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RequiredCapability {
    PublicNetwork,
    PrivateNetwork,
    UnixSocket,
    FileSecret,
    Process,
    Container,
    Device,
}

/// One request parameter, carrying its JSON Schema.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Param {
    /// The **caller-facing** name: what the generated op declares, and what a model passes.
    ///
    /// It is also the wire name unless [`wire`](Self::wire) says otherwise, which is the common
    /// case — a vendor that names a field the same thing its callers do needs no second name.
    pub name: String,
    /// The spelling the **vendor** sees, when it differs from [`name`](Self::name).
    ///
    /// Two shapes, one field, because they are the same fact — *where this value goes on the
    /// request* — asked of two different request positions:
    ///
    /// - a **body** field: the dot-separated JSON path it occupies in the request body.
    ///   Zendesk's comment text is `ticket.comment.body`, not `body`
    ///   (`predecessor:docs/designs/provider-operation-inventory.md` §3.3.1); babelforce's agent-status update
    ///   writes `presence.name`. Without this the field is emitted at the root of the body, which
    ///   Zendesk **accepts and ignores** — a wrong result that answers 200, which is why this is a
    ///   correctness field and not a convenience one.
    /// - a **path, query or header** parameter: a plain alias, the vendor's own name for a
    ///   parameter whose caller-facing name reads better. Freshdesk's requester filter is
    ///   `req_id` to a caller and `requester_id` on the wire (inventory §4.2 op 2).
    ///
    /// A single field rather than a `body_path` and an `alias`: a JSON path with one segment *is*
    /// an alias, so two fields would be two spellings of one idea, and codegen would have to decide
    /// what a parameter carrying both meant.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub wire: Option<String>,
    /// Human-readable description, surfaced to the model as part of the op's tool contract.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub description: String,
    /// Whether the vendor requires the parameter.
    #[serde(default)]
    pub required: bool,
    /// The parameter's JSON Schema.
    ///
    /// Mandatory, with no default: a parameter whose type is unknown is a parameter that has
    /// already collapsed to a string, and that is the failure this whole crate is arranged around.
    pub schema: JsonSchema,
}

/// The caller-facing name of the one parameter a free-form body travels in.
///
/// A convention rather than an IR field: [`ParamSet::body_schema`] says the body *is* a schema and
/// names nothing, so the parameter needs a name from somewhere. It lives here, on the IR, because
/// two surfaces now have to agree on it — `connector-flux` declares the emitted `op`'s body
/// parameter under this name, and [`Operation::input_schema`] composes the property under it. A
/// second spelling of the constant would be the two surfaces asking for one body by two names.
///
/// It is a *starting* name on the Flux side: the emitter allocates symbols through one allocator,
/// so an operation that already binds `body` elsewhere gets a disambiguated symbol rather than a
/// silently shadowed one.
pub const FREE_FORM_BODY: &str = "body";

/// How an operation's request body is encoded on the wire — C-144.
///
/// # Why it exists
///
/// Stripe parses `application/x-www-form-urlencoded` and nothing else, and so do Twilio, Mailgun,
/// PayPal classic, and **every OAuth2 token endpoint by specification** (RFC 6749 §4.3.2). Before
/// this existed the pipeline bound one media type unconditionally, which is why
/// `providers/stripe.toml` selects only operations that address everything they need in the path:
/// a Stripe write with a body field would have sent a JSON document Stripe does not parse.
///
/// # Why it is a closed enum
///
/// An open media-type string is a header nobody validates, so a typo ships a body the vendor
/// accepts and silently ignores — the failure mode this whole module is arranged against. A closed
/// set makes an unknown spelling a **load error**, and it also means the value can never carry
/// anything but one of these two words: a declared encoding cannot become a second, ungated route
/// for a credential into a request header, which is what a free-text `content_type` key would have
/// been.
///
/// # Why it sits on [`ParamSet`] rather than on a service or a provider
///
/// The axis is per-request, not per-vendor: Stripe's API is form-encoded while its *webhook*
/// payloads are JSON, and an OAuth2 token grant is form-encoded on a vendor whose whole business
/// API is JSON. It lives next to [`body`](ParamSet::body) and
/// [`body_schema`](ParamSet::body_schema) because it describes exactly what those declare and
/// nothing else.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BodyEncoding {
    /// A JSON document — `application/json`. The default, so no shipped provider had to change.
    #[default]
    Json,
    /// `key=value&key=value` — `application/x-www-form-urlencoded`.
    ///
    /// It has **no agreed nesting convention** (Stripe writes `metadata[key]`, PHP writes `a[b]`,
    /// Rails writes `a[b][]`, and a token grant nests nothing), so `connector-flux` refuses a nested
    /// body under this encoding rather than picking one of them silently.
    Form,
}

impl BodyEncoding {
    /// The spelling a provider file's `body_encoding` key carries.
    ///
    /// The serde tag, restated as a value so an error message can name what the author wrote.
    /// `tests/ir_roundtrip.rs` holds the two together, so a variant added with a mismatched word
    /// fails rather than producing an error message that names a key nobody can find.
    pub fn tag(&self) -> &'static str {
        match self {
            Self::Json => "json",
            Self::Form => "form",
        }
    }

    /// The media type this encoding travels under, which the emitter puts in `content-type`.
    ///
    /// A method on the enum rather than a table in the emitter: the encoding and the media type are
    /// one fact, and separating them would let a future variant exist with no answer to "what does
    /// the vendor see".
    pub fn media_type(&self) -> &'static str {
        match self {
            Self::Json => "application/json",
            Self::Form => "application/x-www-form-urlencoded",
        }
    }

    /// Whether this is the default, in which case it is omitted from every serialization.
    ///
    /// That omission is what keeps every committed artifact byte-identical across C-144: a
    /// `body_encoding` nobody declared does not appear in a manifest, in the lockfile's hash domain,
    /// or in the published catalogue.
    pub fn is_json(&self) -> bool {
        matches!(self, Self::Json)
    }
}

/// An operation's parameters, grouped by where they travel on the request.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ParamSet {
    /// Parameters interpolated into the path template (`/v2/calls/{call_id}`).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub path: Vec<Param>,
    /// Query-string parameters.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub query: Vec<Param>,
    /// Request headers the caller supplies. Auth headers are **not** here — they are injected by
    /// the host from an [`AuthMethod`], so no credential passes through the parameter surface.
    ///
    /// A header the *vendor* fixes is [`const_headers`](Self::const_headers), not one of these with
    /// a `const` in its schema: this list means caller-supplied, and reinterpreting one entry of it
    /// by keyword would make a single declaration mean two things. `connector-flux` refuses the
    /// pinned spelling rather than honouring it (C-55).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub header: Vec<Param>,
    /// Request headers the **vendor** fixes: `Accept: application/vnd.github+json`,
    /// `Notion-Version: 2022-06-28`, an API version, a `User-Agent`.
    ///
    /// Not parameters, which is why they are a map of literals rather than [`Param`]s and why
    /// [`iter`](Self::iter) does not yield them: nothing is caller-supplied here, nothing reaches
    /// the emitted `op`'s signature, and a model is never asked to guess a value the vendor has
    /// already decided. The emitter binds each value as a literal and puts it in the request's
    /// header record (C-55).
    ///
    /// **Declared at two levels, resolved to one.** A provider states a header once — the file's
    /// top-level `[const_headers]` — and the loader distributes it onto every operation, an
    /// operation's own entry replacing the provider's when they name the same header (HTTP field
    /// names are case-insensitive, so `Notion-Version` and `notion-version` are one header and never
    /// two). So this map is always the *complete* set the operation sends, and no consumer has to
    /// resolve an inheritance to know what travels.
    ///
    /// **It can never carry a credential.** A value here is a literal in a committed artifact, so a
    /// secret placed in one would be a secret in the repository — the one thing generated data must
    /// never hold (AGENTS.md). The loader refuses a value that resolves from an environment variable
    /// or names a declared credential, and refuses a header name the `$auth` seam owns; `Authorization`
    /// is C-10's business and this field must not become a second, ungated path to it.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub const_headers: BTreeMap<String, String>,
    /// Fields assembled into the JSON request body, each at the JSON path its
    /// [`Param::wire`] names (or at the root of the body when it names none).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub body: Vec<Param>,
    /// The body **is** this schema, rather than being assembled from named fields.
    ///
    /// A free-form object body: `babelforce-call-session-set` and `babelforce-session-update` both
    /// take `{"type": "object"}` with no properties — a map of caller-chosen keys
    /// (`predecessor:docs/designs/provider-operation-inventory.md` §6.5). A field list cannot describe that, and
    /// an operation that declared no body field at all emitted a `PUT` with **no body**, which is
    /// indistinguishable from a legitimately bodiless write.
    ///
    /// Mutually exclusive with [`body`](Self::body): "the body is these fields" and "the body is
    /// this schema" are two answers to one question, and nothing states how to merge them. Codegen
    /// refuses an operation that declares both rather than picking one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub body_schema: Option<JsonSchema>,
    /// How the body above is encoded on the wire. See [`BodyEncoding`].
    ///
    /// Defaulted rather than mandatory, unlike [`Risk`] and [`Idempotency`]: silence here is not a
    /// safety decision, it is the answer fifty-three of fifty-three shipped providers already give
    /// (no shipped operation declares a non-default encoding), and making it explicit everywhere
    /// would be fifty-three files restating one fact. What silence must
    /// not do is *change* — so `json` is the default and the emitter's output for an operation that
    /// declares nothing is byte-for-byte what it was before this field existed.
    #[serde(default, skip_serializing_if = "BodyEncoding::is_json")]
    pub body_encoding: BodyEncoding,
}

impl ParamSet {
    /// Whether the operation takes no parameters at all.
    ///
    /// `body_schema` counts: an operation whose only body declaration is a schema would otherwise
    /// encode as an absent `params` and lose the whole body on the way back. So does
    /// [`const_headers`](Self::const_headers), for the same reason — an operation whose only request
    /// declaration is a pinned header would lose it on the way back, and the emitted module would
    /// stop sending the header the vendor requires. And so does a non-default
    /// [`body_encoding`](Self::body_encoding): the emitter refuses that shape rather than emitting
    /// it, so it cannot ship — but a round trip that dropped the declaration would turn a loud
    /// refusal into a silent JSON body, which is the wrong direction to fail in.
    pub fn is_empty(&self) -> bool {
        self.path.is_empty()
            && self.query.is_empty()
            && self.header.is_empty()
            && self.body.is_empty()
            && self.body_schema.is_none()
            && self.const_headers.is_empty()
            && self.body_encoding.is_json()
    }

    /// Every parameter, in request-position order: path, query, header, body.
    ///
    /// [`body_schema`](Self::body_schema) is deliberately absent — it is a schema, not a [`Param`],
    /// and a caller iterating parameters is asking about named ones.
    /// [`const_headers`](Self::const_headers) is absent for the stronger version of that reason:
    /// nothing about it is caller-supplied.
    pub fn iter(&self) -> impl Iterator<Item = &Param> {
        self.path
            .iter()
            .chain(&self.query)
            .chain(&self.header)
            .chain(&self.body)
    }
}

/// How a vendor paginates a collection endpoint.
///
/// `max_pages` is mandatory on every variant because flux's analyzer rejects unbounded loops — a
/// constraint worth honoring rather than working around. Compiling this into Flux control flow is
/// C-12; this is only its declaration.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum Pagination {
    /// Page-number pagination: `?page=2&per_page=100`.
    Page {
        /// The query parameter carrying the page number.
        page_param: String,
        /// The query parameter carrying the page size, when the vendor allows one.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        size_param: Option<String>,
        /// The page size to request.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        page_size: Option<u32>,
        /// The hard cap on pages fetched.
        max_pages: u32,
    },
    /// Cursor pagination: the response carries the cursor for the next request.
    Cursor {
        /// The query parameter carrying the cursor.
        cursor_param: String,
        /// A JSON Pointer (RFC 6901) into the response body locating the next cursor.
        next_cursor_pointer: String,
        /// The hard cap on pages fetched.
        max_pages: u32,
    },
}

/// A vendor's published rate limit, compiled into a Flux `throttle` by C-12.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RateLimit {
    /// Requests allowed per window.
    pub requests: u32,
    /// The window, in seconds.
    pub per_seconds: u32,
    /// The throttle bucket name. Buckets collide if they are not unique within a session, so when
    /// this is `None` codegen derives one from the connector and operation ids.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bucket: Option<String>,
}

/// Where a vendor hides the real error inside a non-2xx response body.
///
/// `http.request` treats a non-2xx as a *result* rather than an op failure, so the generated op has
/// to dig the message out itself. Both fields are JSON Pointers (RFC 6901) into the response body.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ErrorEnvelope {
    /// Pointer to the human-readable message.
    pub message_pointer: String,
    /// Pointer to the vendor's error code, when it publishes one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub code_pointer: Option<String>,
}

/// The exact vendor document selection that produced one public operation — C-481.
///
/// This type deliberately has no repository-local path or fetch timestamp. A catalogue consumer
/// needs the vendor operation identity and the public source contract, while local refresh
/// mechanics remain in [`crate::provider::SpecSource`]. `source_url` alone is nullable: private
/// vendoring may publish the bytes and hashes without publishing the pull location.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OperationSpecSource {
    /// The vendor's exact `operationId`, before the connector gives it a stable public id.
    pub operation_id: String,
    /// The public URL the vendored document came from, when one may be published.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_url: Option<String>,
    /// The upstream version the ingested document declares.
    pub upstream_version: String,
    /// Measured SHA-256 of the committed document bytes the compiler ingested.
    pub sha256: String,
}

/// Where a connector came from, so drift against upstream can be detected rather than absorbed.
///
/// **`ir_sha256` is deliberately absent.** The pipeline design lists it under provenance, but it is
/// computed *from* the serialized IR — storing it inside the value being hashed would make the hash
/// depend on itself. It belongs in `connectors.lock` alongside the generated-artifact hashes and
/// the generator version, and that is where [`LockEntry::ir_sha256`](crate::lock::LockEntry) keeps
/// it.
///
/// Note that **none of these fields is in the IR hash domain** — see [`Connector::hash_domain`].
/// They are inputs to the build, recorded and verified one by one, not part of what was compiled.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Provenance {
    /// The URL the vendor spec was fetched from. `None` for a fully hand-authored connector — the
    /// Ollama case, where no vendor OpenAPI document exists at all.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_url: Option<String>,
    /// The upstream version string the vendor published for that spec.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub upstream_version: Option<String>,
    /// When the spec was fetched, as an RFC 3339 timestamp. A string, not a date type: this crate
    /// takes no new dependency for a field nothing here does arithmetic on.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fetched_at: Option<String>,
    /// SHA-256 of the vendored spec bytes under `specs/`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub spec_sha256: Option<String>,
    /// **Every vendored document this connector compiles, one entry each** — C-410.
    ///
    /// The four fields above describe *a* spec, which was adequate while a connector compiled from
    /// exactly one. babelforce compiles from five, pulled on two different dates, three of which
    /// publish `info.version = "0.0.0-dev"`; one hash for the connector cannot answer the only
    /// question a drift check asks, which is **which document moved**. So the per-document record is
    /// this list, and it is filled for a single-document connector too — the singular fields are the
    /// projection of the one-element case, not a separate truth. With several documents they are
    /// `None`, because no single value of them is true and filling them from the first would record
    /// one document's provenance as the whole connector's.
    ///
    /// Empty for a hand-authored connector, and elided from the encoded IR when empty, so landing
    /// this field moves no `ir_sha256` in the repository.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub specs: Vec<crate::provider::SpecSource>,
    /// Stable public operation id → the exact vendor selection that produced it.
    ///
    /// Filled only by the patch application path. [`crate::provider::ProviderFile`] has no
    /// provenance field, so an inline operation cannot author or forge this map. A `BTreeMap`
    /// makes its canonical encoding independent of selection traversal order.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub operation_specs: BTreeMap<String, OperationSpecSource>,
    /// SHA-256 of the provider TOML bytes.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub toml_sha256: Option<String>,
}

// The reserved service name, defined in `connector-address` and re-exported here so that
// `crate::ir::DEFAULT_SERVICE` — which most of this crate spells — keeps resolving (C-407).
//
// It went down with the two address modules rather than staying with the IR because the rule it
// carries is an *address* rule: it is elided from a gid and from a credential path, both grammars
// are written against it, and a copy on this side of the crate boundary is a copy that can drift
// from the one the elision is implemented against.
pub use connector_address::DEFAULT_SERVICE;

/// A named capability shape a [`Service`] claims to implement, checked at load — C-120.
///
/// Seventeen connectors share structure that nothing named. `openai` and `openrouter` both list
/// models; `zendesk`, `freshdesk`, `intercom` and `jira` all show a ticket. Nothing in the IR said
/// so, so nothing could act on it: a UI could not group them, and a second vendor filling the same
/// shape was a coincidence rather than a contract. A role is that missing declaration. See
/// `predecessor:docs/designs/provider-roles.md` (provenance only).
///
/// # Why it is closed
///
/// An open string set is a tag system, and a tag system cannot be checked. A closed enum can, and
/// the checking is the point: `llm_catalogue` is a *promise the loader enforces*, so a consumer
/// reading the catalogue can rely on it without reading the provider's TOML. It also makes an
/// unknown name a **load error** rather than a silent no-op, which is the failure mode this whole
/// mechanism exists to prevent — a typo'd capability that quietly means "no capability" is invisible
/// to everyone downstream.
///
/// # Why it attaches to a service
///
/// `openai` is not "an LLM provider": it exposes a management surface (`models`) and an inference
/// surface (`chat`), which are different capabilities with different risk and different idempotency.
/// C-49's service level is exactly that granularity. A role declared on the *provider* would smear
/// the two together, so a provider's roles are **derived** ([`Connector::roles`]) and never authored
/// — the same rule [`Level`](crate::config::Level) follows against `binds`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Role {
    /// A live list of the models a vendor serves — the half of the LLM story a connector may own.
    ///
    /// flux's own model metadata lives in static tables that go stale the moment a vendor ships a
    /// model; `openai-models-list` is live. So a connector *informs* the pool of `(provider, model)`
    /// tuples while `flux-providers` keeps *serving* it. Serving inference from a connector is a
    /// stated non-goal (C-123 is the charter decision), which is why there is no `llm_inference`
    /// variant here.
    LlmCatalogue,
}

impl Role {
    /// Every role there is, in the order an error message lists them.
    ///
    /// A `const` rather than a derived iterator so that the "known set" a refusal prints and the set
    /// the loader accepts are the same value, and a variant added without extending this fails the
    /// exhaustive `match` below rather than silently going unlisted.
    pub const ALL: [Self; 1] = [Self::LlmCatalogue];

    /// The token this role serializes as, for error text.
    pub fn word(self) -> &'static str {
        match self {
            Self::LlmCatalogue => "llm_catalogue",
        }
    }

    /// The **operations** a service must have to claim this role.
    ///
    /// Named by **member name within the service** (`list`), never by full operation id
    /// (`openai-models-list`) — see [`fills_slot`] for what "within the service" means and why it is
    /// what makes a role vendor-independent. Only an operation fills a slot; see
    /// [`Connector::missing_role_members`] for why an event does not.
    pub fn required_members(self) -> &'static [&'static str] {
        match self {
            Self::LlmCatalogue => &["list"],
        }
    }
}

/// Whether an operation's name fills one of a role's required-member slots.
///
/// A role requires `list`, not `openai-models-list`. The slot is matched against the member's
/// **trailing name segments**, so `openai-models-list` and `openrouter-models-list` fill the same
/// slot and the vendor prefix nobody agreed on stays out of the contract — which is the whole reason
/// a role is worth declaring rather than inferring.
///
/// Segments split on `-`, `_` and `.`: those are the three separators a member name admits (see
/// [`Connector::member_names_of`]), so a role naming `comment.list` and a member named
/// `zendesk-ticket-comment-list` are the same slot. The match is on whole segments and never on a
/// substring, or `acme-blocklist-get` would satisfy a role it has nothing to do with.
///
/// **A one-segment slot is loose**, and known to be: a bare `list` is the trailing segment of **83
/// shipped operations across 42 of the 53 shipped providers** — a majority, and growing with every
/// fleet wave, so the looseness is worse now than when this was first written (it then read "nine of
/// seventeen"). That is a weakness of how a role spells its slots, not of the match, and C-121
/// addresses it by giving a slot a tighter spelling (`models.list`) and a set of accepted ones.
/// Multi-segment slots are already tight.
fn fills_slot(member: &str, slot: &str) -> bool {
    const SEPARATORS: [char; 3] = ['-', '_', '.'];
    let member: Vec<&str> = member.split(SEPARATORS).collect();
    let slot: Vec<&str> = slot.split(SEPARATORS).collect();
    member.len() >= slot.len() && member[member.len() - slot.len()..] == slot[..]
}

/// **What kind of thing a [`Service`] is**, so a catalogue can be filtered by domain — C-153.
///
/// Fifty-four providers and sixty-three services, and nothing said which of them are telephony and
/// which are payments. A tag is that missing label.
///
/// # A tag is not a [`Role`], and the distinction is the point
///
/// | | [`Role`] | `Tag` |
/// |---|---|---|
/// | answers | "can this service **do** X, checkably?" | "what **kind** of thing is this?" |
/// | carries | required members, verified at load | nothing |
/// | refuses | an unknown name **and** a claim the members do not satisfy | an unknown name only |
///
/// They are two fields on purpose. Giving `storage` a required-member list would be meaningless — no
/// operation makes a service storage — and letting a role carry no members would turn every role into
/// an unchecked assertion, which is exactly what [`Role`]'s closed set exists to prevent.
///
/// **The misread to design against:** a UI filtering by tag invites the inference "this category
/// means these capabilities". It does not. A tag is evidence of *kind*, never of what is callable.
///
/// # Why it is closed anyway
///
/// A tag carries no members, so nothing downstream can notice that `telephny` is not `telephony` —
/// it simply never matches a filter, and a service that looks tagged is invisible to the one query it
/// was tagged for. That silent-nothing failure is cheaper to be wrong about than a typo'd role, but
/// it is not free, so the vocabulary is closed and an unknown word is refused at the parse.
///
/// # The vocabulary is derived, not invented
///
/// Every variant below is populated by at least one shipped service; the set was read off all 54
/// providers rather than designed ahead of them. Seven are **singletons** today — [`Search`](Self::Search),
/// [`ESignature`](Self::ESignature), [`Ecommerce`](Self::Ecommerce), [`Payments`](Self::Payments),
/// [`Identity`](Self::Identity), [`Forms`](Self::Forms) and
/// [`VideoConferencing`](Self::VideoConferencing) — kept rather than folded because each names a
/// domain a reader would actually filter for, and folding them would put `algolia` under a label that
/// describes it worse. The clustering is recorded in
/// `predecessor:docs/designs/provider-roles.md` (provenance only).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Tag {
    /// Model catalogues and inference APIs — `anthropic`, `openai`, `openrouter`.
    Ai,
    /// Headless content management — `contentful`, `webflow`.
    Cms,
    /// Customer relationship management — `hubspot`, `salesforce`.
    Crm,
    /// Structured data stores addressed as a product — `airtable`, `supabase`.
    Database,
    /// Design and whiteboard surfaces — `figma`, `miro`.
    Design,
    /// Tooling aimed at engineers that is not source control — `launchdarkly`, `vercel`.
    DeveloperTools,
    /// Electronic signature — `docusign`.
    ESignature,
    /// Online storefronts and their orders — `shopify`.
    Ecommerce,
    /// Sending and reading mail — `postmark`, `resend`, `sendgrid`, `google`'s `gmail`.
    Email,
    /// Form and survey collection — `typeform`.
    Forms,
    /// Directory, identity and access management — `okta`.
    Identity,
    /// On-call, incident response and status communication — `pagerduty`, `statuspage`.
    IncidentResponse,
    /// Compute, networking and hosting — `cloudflare`, `fly`, `vercel`, `supabase`.
    Infrastructure,
    /// Issue and bug tracking — `jira`, `github`, `gitlab`.
    IssueTracking,
    /// Wikis and long-form internal knowledge — `confluence`, `notion`.
    KnowledgeBase,
    /// Campaigns, audiences and lifecycle marketing — `klaviyo`, `mailchimp`.
    Marketing,
    /// Human-to-human chat — `slack`, `discord`, `twilio`.
    Messaging,
    /// Metrics, traces and error tracking — `datadog`, `newrelic`, `sentry`.
    Observability,
    /// Money movement — `stripe`.
    Payments,
    /// Tasks, boards and project planning — `asana`, `clickup`, `trello`, `jira`.
    ProjectManagement,
    /// Calendars and booking — `calendly`, `google`'s `calendar`, `zoom`.
    Scheduling,
    /// Search as a product — `algolia`.
    Search,
    /// Repositories, branches and pull requests — `github`, `gitlab`, `bitbucket`.
    SourceControl,
    /// Files and objects — `box`, `dropbox`, `google`'s `drive`.
    Storage,
    /// Customer support and shared inboxes — `zendesk`, `freshdesk`, `intercom`, `front`.
    Support,
    /// Voice, calls and contact centres — `babelforce`, `twilio`.
    Telephony,
    /// Meetings and video — `zoom`.
    VideoConferencing,
}

impl Tag {
    /// Every tag there is, in the order an error message lists them.
    ///
    /// A `const` rather than a derived iterator, for the reason [`Role::ALL`] is one: the set a
    /// refusal prints and the set the loader accepts are the same value, and a variant added without
    /// extending this fails the exhaustive `match` in [`word`](Self::word) rather than silently going
    /// unlisted.
    pub const ALL: [Self; 27] = [
        Self::Ai,
        Self::Cms,
        Self::Crm,
        Self::Database,
        Self::Design,
        Self::DeveloperTools,
        Self::ESignature,
        Self::Ecommerce,
        Self::Email,
        Self::Forms,
        Self::Identity,
        Self::IncidentResponse,
        Self::Infrastructure,
        Self::IssueTracking,
        Self::KnowledgeBase,
        Self::Marketing,
        Self::Messaging,
        Self::Observability,
        Self::Payments,
        Self::ProjectManagement,
        Self::Scheduling,
        Self::Search,
        Self::SourceControl,
        Self::Storage,
        Self::Support,
        Self::Telephony,
        Self::VideoConferencing,
    ];

    /// The token this tag serializes as, for error text.
    pub fn word(self) -> &'static str {
        match self {
            Self::Ai => "ai",
            Self::Cms => "cms",
            Self::Crm => "crm",
            Self::Database => "database",
            Self::Design => "design",
            Self::DeveloperTools => "developer-tools",
            Self::ESignature => "e-signature",
            Self::Ecommerce => "ecommerce",
            Self::Email => "email",
            Self::Forms => "forms",
            Self::Identity => "identity",
            Self::IncidentResponse => "incident-response",
            Self::Infrastructure => "infrastructure",
            Self::IssueTracking => "issue-tracking",
            Self::KnowledgeBase => "knowledge-base",
            Self::Marketing => "marketing",
            Self::Messaging => "messaging",
            Self::Observability => "observability",
            Self::Payments => "payments",
            Self::ProjectManagement => "project-management",
            Self::Scheduling => "scheduling",
            Self::Search => "search",
            Self::SourceControl => "source-control",
            Self::Storage => "storage",
            Self::Support => "support",
            Self::Telephony => "telephony",
            Self::VideoConferencing => "video-conferencing",
        }
    }

    /// Every tag's word, comma-separated — the "known set" a refusal names.
    pub fn known_set() -> String {
        Self::ALL
            .iter()
            .map(|tag| tag.word())
            .collect::<Vec<_>>()
            .join(", ")
    }
}

/// The people a service is ordinarily useful to, for catalogue discovery only.
///
/// An audience is deliberately neither a permission nor a visibility rule. `Sre` means “an SRE
/// browsing the catalogue is likely to find this useful”; it does not grant that person an
/// operation, hide the service from anybody else, or participate in policy admission. Those remain
/// Connection, Grant and deployment-policy decisions.
///
/// This lives on [`Service`] rather than [`Connector`], because one provider may expose surfaces
/// aimed at different kinds of work. A provider-level audience is derived as the union via
/// [`Connector::audiences`], exactly as tags are.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Audience {
    /// People building and operating software.
    Developer,
    /// Site reliability and production operations practitioners.
    Sre,
    /// People designing and reviewing security controls.
    SecurityEngineer,
    /// People exploring and interpreting operational or business data.
    DataAnalyst,
    /// People shaping product outcomes and roadmaps.
    ProductManager,
    /// People coordinating projects and delivery work.
    ProjectManager,
    /// People designing product and visual experiences.
    Designer,
    /// People working with prospects and customer accounts.
    SalesRep,
    /// People supporting customers and resolving their requests.
    SupportAgent,
    /// People running campaigns, audiences and lifecycle communication.
    Marketer,
    /// People responsible for payments and financial operations.
    Finance,
    /// People managing online storefronts, products and orders.
    EcommerceManager,
    /// People managing published content and editorial workflows.
    ContentManager,
}

impl Audience {
    /// Every admitted catalogue token, in the order diagnostics present them.
    pub const ALL: [Self; 13] = [
        Self::Developer,
        Self::Sre,
        Self::SecurityEngineer,
        Self::DataAnalyst,
        Self::ProductManager,
        Self::ProjectManager,
        Self::Designer,
        Self::SalesRep,
        Self::SupportAgent,
        Self::Marketer,
        Self::Finance,
        Self::EcommerceManager,
        Self::ContentManager,
    ];

    /// The stable public token used by catalogues and explorer filters.
    pub const fn word(self) -> &'static str {
        match self {
            Self::Developer => "developer",
            Self::Sre => "sre",
            Self::SecurityEngineer => "security-engineer",
            Self::DataAnalyst => "data-analyst",
            Self::ProductManager => "product-manager",
            Self::ProjectManager => "project-manager",
            Self::Designer => "designer",
            Self::SalesRep => "sales-rep",
            Self::SupportAgent => "support-agent",
            Self::Marketer => "marketer",
            Self::Finance => "finance",
            Self::EcommerceManager => "ecommerce-manager",
            Self::ContentManager => "content-manager",
        }
    }

    /// Every audience token, comma-separated for actionable loader errors.
    pub fn known_set() -> String {
        Self::ALL
            .iter()
            .map(|audience| audience.word())
            .collect::<Vec<_>>()
            .join(", ")
    }
}

/// The closed parser that turns one reviewed provider response into discovery observations.
///
/// A discovery driver may interpret only the response of [`Discovery::operation`]. It does not
/// scan a network, follow arbitrary links, accept a caller-selected path, or create authority.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DiscoveryDriver {
    /// Grafana's `GET /api/datasources` response, normalized without publishing data-source URLs,
    /// UIDs, credentials, or arbitrary proxy paths.
    GrafanaDatasourceV1,
}

impl DiscoveryDriver {
    /// The stable catalog token.
    pub const fn word(self) -> &'static str {
        match self {
            Self::GrafanaDatasourceV1 => "grafana_datasource_v1",
        }
    }
}

/// The closed adapter that may execute a target Provider operation through another Connection.
///
/// Route adapters are implementation identities, not caller-selectable strings. Unknown adapters
/// are refused rather than treated as direct HTTP.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RouteAdapter {
    /// Prefix a native backend request with Grafana's reviewed data-source proxy route, using an
    /// opaque Connector-owned resource binding rather than a caller-provided Grafana UID.
    GrafanaDatasourceProxyV1,
}

impl RouteAdapter {
    /// The stable catalog and Connection token.
    pub const fn word(self) -> &'static str {
        match self {
            Self::GrafanaDatasourceProxyV1 => "grafana_datasource_proxy_v1",
        }
    }
}

/// One closed normalization from a vendor-observed type to a native B10x Provider.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DiscoveryMapping {
    /// The exact normalized vendor type accepted by this mapping.
    pub observed_type: String,
    /// The target Provider contract whose semantics and operation catalog the candidate uses.
    pub target_provider: String,
    /// The reviewed adapter used if the candidate is materialized as a mediated Connection.
    pub route_adapter: RouteAdapter,
}

/// A bounded, catalog-declared observation source.
///
/// Running the named operation can produce observations and Connection candidates. It can never
/// create a Connection, inherit a Grant, or authorize an operation by itself.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Discovery {
    /// Stable declaration identity within the Provider.
    pub id: String,
    /// Service containing the observation operation.
    #[serde(
        default = "default_service",
        skip_serializing_if = "is_default_service"
    )]
    pub service: String,
    /// A bounded read Operation in this same Provider.
    pub operation: String,
    /// Closed response normalizer.
    pub driver: DiscoveryDriver,
    /// Closed vendor-type to target-Provider mappings.
    pub mappings: Vec<DiscoveryMapping>,
}

/// One API surface of a provider: the unit you address, version, select and install.
///
/// `s3` and `bedrock-runtime` under AWS; `support` under Zendesk. A service is the middle level of
/// C-37's address scheme, promoted from an anonymous path segment to a named thing with an owner —
/// see `predecessor:docs/designs/provider-services.md` (provenance only).
///
/// **This is not a tag.** A free-form label cannot do the three things this level exists for: it
/// cannot *partition* (so "install the whole s3 service" would not denote a set), it cannot carry a
/// *version* (so `s3:2006-03-01` would have to be repeated on every operation and could disagree with
/// itself), and it cannot carry a *host* (so a service could not have its own egress surface).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Service {
    /// The service name, e.g. `s3`. Names the emitted `<provider>-<name>.flux`, is the first path
    /// segment of the service's gid, and is what [`Operation::service`] references.
    pub name: String,
    /// This is the connector's already-published, address-elided `default` service, preserved while
    /// named siblings are added — C-458.
    ///
    /// The marker is admitted only on a `default` entry beside at least one named service. It is an
    /// address-migration capability, not shorthand for a new connector: without it `default` beside
    /// a named service remains refused, and with it every member must state its service explicitly.
    #[serde(default, skip_serializing_if = "is_false")]
    pub legacy: bool,
    /// What the service is for, in one line.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub description: String,
    /// The service's own API base URL, when it differs from the connector's.
    ///
    /// AWS is the motivating case: `s3.amazonaws.com` and `bedrock-runtime.<region>.amazonaws.com`
    /// share an authority and not a host. Resolve it with [`Connector::base_url_of`] rather than
    /// reading this directly — the connector's value is the default.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub base_url: Option<String>,
    /// The vendor's API version for *this* service, when it differs from the connector's.
    ///
    /// AWS versions each service on its own date (`s3:2006-03-01`,
    /// `bedrock-runtime:2023-09-30`), which is why a single connector-level version cannot describe a
    /// multi-service provider. Resolve it with [`Connector::api_version_of`].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub api_version: Option<String>,
    /// The capability shapes this service claims to implement — see [`Role`].
    ///
    /// The loader checks every claim: a service declaring a role it does not satisfy is refused,
    /// naming the member that is missing. That check is what makes the declaration worth anything to
    /// a consumer.
    ///
    /// A **provider's** roles are the union of its services' ([`Connector::roles`]) and are never
    /// authored — a provider-level `roles` key is a load error, because a value that is both derived
    /// and writable is two sources of truth waiting to disagree.
    ///
    /// `skip_serializing_if` for the same reason the three fields above carry it: a service that
    /// claims no role must hash to what it hashed before roles existed, or landing C-120 moves every
    /// `ir_sha256` in the repository and churns `connectors.lock` for a provider nobody edited.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub roles: Vec<Role>,
    /// What **kind** of thing this service is — see [`Tag`].
    ///
    /// Unlike [`roles`](Self::roles) this carries no required members and the loader verifies no
    /// claim beyond the name being one that exists: a tag answers "what kind of thing is this?", not
    /// "what can it do?". The two are deliberately separate fields with separate guarantees.
    ///
    /// A **provider's** tags are the union of its services' ([`Connector::tags`]) and are never
    /// authored, for the reason `roles` follows the same rule — and because it is the wrong *level*:
    /// `google` is not one kind of thing, its `gmail` is email and its `drive` is storage.
    ///
    /// `skip_serializing_if` for the reason every optional field on this struct carries it: a service
    /// declaring no tag must hash to what it hashed before tags existed, or landing C-153 moves every
    /// `ir_sha256` in the repository and churns `connectors.lock` for a provider nobody edited.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<Tag>,
    /// Who is likely to find this service useful — see [`Audience`].
    ///
    /// Discovery metadata only. It never affects operation exposure, credential resolution,
    /// Connection ownership, Grant admission or runtime authorization.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub audiences: Vec<Audience>,
}

fn is_false(value: &bool) -> bool {
    !*value
}

/// **The shortest stated condition that counts as one**, in characters after trimming.
///
/// Not a measure of truth — nothing here can check whether a vendor really deduplicates — but a
/// floor on effort, and the number is calibrated rather than invented: it is the length of
/// `"purging twice is a no-op"`, the shortest honest reason anyone working on C-186 actually wrote.
/// Below it live `"yes"`, `"idempotent"` and `"see above"`, which unlock the claim while telling a
/// reviewer nothing, and an escape hatch that costs nothing is a deleted guard.
///
/// `crates/connector-spec/schema/provider-toml.schema.json` publishes this as the `minLength` of
/// `repeatable_because`, and `tests/provider_schema.rs` reads *this constant* to check it rather
/// than a second copy of the number — two statements of one fact with only one machine-checked is
/// the defect class C-186 was filed for, and duplicating it here would have re-enacted it.
pub const MIN_REPEATABILITY_CONDITION: usize = 24;

/// One operation and the closed driver request it dispatches.
///
/// `description`, `risk` and `idempotency` map straight onto the metadata a Flux composite op
/// declares (`op … description "…" risk "low" idempotency "idempotent"`), which is also the
/// `ToolSpec` surface flux exposes to a model.
#[derive(Debug, Clone, PartialEq)]
pub struct Operation {
    /// The op name, e.g. `babelforce.call.list`. This is a **stable public contract**: users and
    /// models call it by name, so it must survive regeneration and must not be derived from a
    /// volatile spec field like `operationId` without a pinned override.
    pub id: String,
    /// The [`Service`] this operation belongs to — **exactly one**, always a concrete name.
    ///
    /// A `String` rather than an `Option<String>`: `None` and `Some("default")` would be two
    /// spellings of one meaning, which is the objection [`AuthRequirement`] already records against an
    /// empty mechanism inside a non-empty alternatives list. "Unset" exists in the *file*, where it
    /// deserializes to [`DEFAULT_SERVICE`]; the IR always names a service.
    ///
    /// Not a set, either. A set would leave the gid ambiguous (which segment renders?) and would make
    /// selection non-partitioning, so "the s3 service" would stop being answerable by set membership.
    /// An operation that genuinely serves two services is duplicated deliberately with two ids —
    /// which is visible in a diff — rather than resolved by a rule nobody can see.
    pub service: String,
    /// Driver-specific request facts. Flattened so `protocol_driver` remains a first-class public
    /// field while impossible cross-driver field combinations stay unrepresentable.
    pub request: OperationRequest,
    /// Whether the operation reads or may change vendor state. Required and never inferred from
    /// [`Self::method`], its name, risk, idempotency, semantic effects, or exposure.
    pub direction: OperationDirection,
    /// What the operation does, in one line. Reaches the model as the tool description.
    pub description: String,
    /// How much damage the operation can do. See [`Risk`].
    pub risk: Risk,
    /// Whether repeating it is safe. See [`Idempotency`].
    pub idempotency: Idempotency,
    /// Declared host-resource consequences, resolved from an exact operation or reviewed selector.
    pub effects: Vec<HostEffect>,
    /// What executing this operation means to Flux policy, independently of the host resources it
    /// touches. Empty means no semantic consequence has been declared.
    pub semantic_effects: Vec<SemanticEffect>,
    /// The operation's lifecycle, independently of its protocol and placement.
    pub interaction_shape: InteractionShape,
    /// A placement requirement, not a deployment-selected destination.
    pub placement_requirement: PlacementRequirement,
    /// How the protocol implementation is supplied.
    pub implementation_form: ImplementationForm,
    /// Host capabilities required before credential access or dispatch.
    pub required_capabilities: Vec<RequiredCapability>,
    /// **The condition under which repeating this write is safe** — mandatory on an authored write
    /// declaring [`Idempotency::Conditional`], and meaningless anywhere else.
    ///
    /// flux's I3 (`flux_spec::coherence`) says a mutating operation that really is replay-safe
    /// declares `Conditional` "when it is genuinely safe to repeat under **stated** conditions".
    /// Nothing was making anyone state them. Six operations declared `Conditional` before C-186 —
    /// three of them Stripe money movements — with the condition recorded in no field, no artifact
    /// and, for four of the six, no comment either. A host reading `Conditional` learned that some
    /// condition existed and nothing about what it was, which is barely more than `NonIdempotent`.
    ///
    /// This field is that condition, and stating it is the whole cost of the claim:
    ///
    /// - **Silence refuses.** An authored write declaring `Conditional` without one does not
    ///   build. That is a rule this repository did not have before C-186 and is a tightening, not a
    ///   loosening — the escape hatch was already wide open and unguarded.
    /// - **The condition must be one.** Blank, whitespace, or shorter than
    ///   [`MIN_REPEATABILITY_CONDITION`] is refused: an escape hatch anyone can take without saying
    ///   anything is a deleted guard wearing the guard's clothes. What no compiler can check is
    ///   whether the sentence is *true* — that is what publishing it into
    ///   `web/public/catalog.json`, beside the claim it licenses, is for.
    /// - **It is refused where it means nothing.** On an authored read there is no repeat
    ///   hazard to condition; and on an operation not declaring `Conditional` the prose asserts what
    ///   its own field denies, which is C-186's defect arriving backwards.
    ///
    /// It deliberately does **not** unlock [`Idempotency::Idempotent`] on an authored write. The
    /// first landing of C-186 made it do exactly that, and it was wrong for a reason worth keeping
    /// here: `Idempotent` licenses flux's op cache to serve a stored result *instead of executing*,
    /// so "safe to repeat" and "safe to skip" are different claims and only the second is what that
    /// value means. Purging a cache is the first and not the second.
    ///
    /// `Option`, not a defaulted `String`: "stated nothing" and "stated the empty string" must not
    /// be two spellings of one thing, and `skip_serializing_if` keeps every `ir_sha256` in the
    /// repository exactly where it was for the operations that do not use it — asserted by
    /// `tests/ir_roundtrip.rs`, not merely claimed here.
    pub repeatable_because: Option<String>,
    /// **Whether this operation reaches a model as an LLM tool.**
    ///
    /// Two claims were fused before C-413, because the emitter hard-coded `expose: true`: that an
    /// operation **exists and can be called**, and that it **reaches a model as a tool**. They are not
    /// the same claim, and conflating them prices coverage in context. A connector covering the
    /// babelforce manager document is 397 operations; 397 tools is not a catalogue but a denial of
    /// service against the model's context window, and it is the entire reason
    /// `predecessor:docs/designs/provider-operation-inventory.md` §5.2 shipped 9 of 163 rather than all of them.
    ///
    /// Curation was the right answer while the connector was the only surface. It is the wrong answer
    /// for a connector that must serve every caller a vendor SDK served. So the claims separate:
    ///
    /// - **catalogued and callable** — the operation exists, `connectors-api` will run it, it appears
    ///   in the manifest's `operations` list, in `catalog.json` and in the embedded catalogue, and
    ///   `connector-pack` will build a request for it. True of *every* operation, unconditionally.
    /// - **exposed** — it additionally reaches a model as a tool. That is this field, and it is the
    ///   only thing withheld when it is `false`.
    ///
    /// Keeping the first of those true takes a deliberate seam, because a `ToolRegistry` is *both*
    /// what a host advertises to a model and what an execute route resolves through: filtering it
    /// withholds the **call** as a side effect of withholding the **tool**. So `connector_pack::pack`
    /// is model-facing and withholds an unexposed operation, while `connector_pack::resolve` is
    /// caller-facing and withholds nothing — and `connectors-api`'s execute route goes through the
    /// second. Without that split this field would make several hundred operations catalogued,
    /// documented, manifest-listed and unreachable, which is the feature inverted.
    ///
    /// # Why the default is `true`
    ///
    /// This is a **widening, not a loosening**: `false` is a state no author could express before, and
    /// the default keeps today's behaviour exactly. That is not only a compatibility courtesy — an
    /// operation whose exposure was decided by a default that *hides* it would be a safety-shaped
    /// decision made by silence, which is the failure mode [`Risk`] refuses a `Default` for. Silence
    /// here means "nothing was decided", and nothing-decided must mean what the repository already
    /// does.
    ///
    /// `skip_serializing_if` is what makes the default cost nothing: an operation that says nothing
    /// hashes exactly as it did before this field existed, so landing C-413 moved no `ir_sha256`, no
    /// `connectors.lock` entry, and none of the 557 generated artifacts. Asserted by
    /// `tests/ir_roundtrip.rs`, not merely claimed here — the same precedent
    /// [`Operation::repeatable_because`] and [`Service::roles`] record, and for the same reason.
    ///
    /// # What this is not
    ///
    /// It is **not a curation mechanism in the loader**. Nothing here decides *which* operations a
    /// build compiles; that is selection, it stays opt-in, and it belongs to C-411. This field says
    /// what happens to an operation that was already selected.
    pub expose: bool,
    /// Which auth this operation requires, as a set of **alternatives** (OR); each alternative is
    /// an [`AuthRequirement`] — one mechanism — whose credentials must all be satisfied together
    /// (AND).
    ///
    /// The `Option` is the whole point, and it is OpenAPI's operation-level `security` semantics
    /// exactly:
    ///
    /// - `None` — **unset**. The operation inherits [`Connector::default_auth`]. Encodes by being
    ///   omitted entirely.
    /// - `Some(vec![])` — **explicitly none**. The operation needs no auth at all: a health or ping
    ///   endpoint. It does *not* inherit the connector default. Encodes as `[]`.
    /// - `Some(vec![a, b])` — either `a` or `b` authenticates the request.
    ///
    /// Collapsing the two empty cases into one would make an unauthenticated endpoint
    /// inexpressible on a connector that has a default, which is most of them. Use
    /// [`Connector::effective_auth`] to resolve the inheritance rather than reading this directly.
    pub auth: Option<Vec<AuthRequirement>>,
    /// The request parameters, grouped by position.
    pub params: ParamSet,
    /// The JSON Schema of a successful response body, when the spec publishes one.
    pub response_schema: Option<JsonSchema>,
    /// **Where in this operation's own response a credential arrives** — one location per entry,
    /// each a [`response_location_exists`] pointer into [`response_schema`](Self::response_schema).
    ///
    /// C-430's declaration, and the *only* thing the gate that story built reads. A connector says
    /// "the value at this location is a credential"; nothing infers it from a field name, because a
    /// catalogue-wide scan for token-shaped names returned 31 hits of which **28 were correct as
    /// they stood** — Klaviyo's `public_api_key` is public by design, Typeform's `token` is a
    /// response's own opaque id, Anthropic's `max_input_tokens` is a limit. A gate wrong nine times
    /// in ten does not get obeyed, it gets routed around. So the claim is the connector's to make.
    ///
    /// # Declaring it withholds the operation
    ///
    /// Today this field has exactly one consequence and it is refusal: the loader will not accept an
    /// operation that declares one, and `AGENTS.md` § Authentication contract is why — an operation
    /// whose declared response carries a token is withheld until C-136's diversion lands, because
    /// the host's redactor holds only values the host itself resolved and cannot know a secret
    /// minted by the very call returning it. So the field is not "mark it and ship it"; it is the
    /// author's statement of the fact that withholds the operation, and it survives in a rejection
    /// fixture and in the provider file's own accounting rather than in a shipped connector.
    ///
    /// **That is a floor, not the ceiling.** [C-79] designs the marker that lets an operation ship
    /// *without* the field — the host redacting the location before the value reaches a
    /// model-visible symbol — and C-136 designs the answer for an operation whose whole purpose is
    /// to mint a credential: return a handle, not the secret. When either lands, this field is what
    /// they are declared through; the refusal below is what changes, not the declaration.
    ///
    /// # This field or [`produces_credential`](Self::produces_credential), never both (C-432)
    ///
    /// The two state one fact and prescribe opposite dispositions, so declaring both is refused —
    /// see `validate_one_credential_disposition`. **The discriminator is purpose, not shape:** this
    /// field is for a credential that arrives *incidentally*, beside the result the operation exists
    /// to deliver, where diverting the whole result would delete the answer. When the credential
    /// **is** the answer, the operation is a mint and declares `produces_credential` instead.
    ///
    /// # Empty for every shipped connector, deliberately
    ///
    /// Because an operation carrying one cannot be selected. `skip_serializing_if` keeps the encoded
    /// IR of every connector shipped before C-430 byte-identical, so landing the field moved no
    /// `ir_sha256` and churned `connectors.lock` for nobody — the same property [`Service::roles`]
    /// documents for the same reason.
    ///
    /// [C-79]: ../../../docs/stories/C-79-sensitive-response-fields.md
    pub credential_response: Vec<String>,
    /// **This operation's whole purpose is to mint a credential**, and this says which one and
    /// where in the vendor's answer it arrives — C-136's declaration.
    ///
    /// The sibling of [`credential_response`](Self::credential_response) and the inverse of it.
    /// That field says *"a credential arrives here and I have no way to keep it out of the
    /// session"*, and the only thing the loader can do with the claim is withhold the operation.
    /// This one says *"a credential arrives here and it is to be diverted into the store"*, which is
    /// an operation that can ship: the value travels from the response into the bound
    /// `CredentialStore` and the caller receives the **handle**.
    ///
    /// **An operation declares one or the other, never both** (C-432), and the loader refuses the
    /// pair — see `validate_one_credential_disposition`. Nothing about the pointer, the schema or
    /// the field's name distinguishes them, because the discriminator is **purpose**: declare this
    /// field when the credential *is* the answer, and `credential_response` when it arrives
    /// *incidentally* beside the answer, where diverting the result would delete what the caller
    /// asked for.
    ///
    /// # The declared output is the handle, not the vendor's body
    ///
    /// [`Operation::effective_response_schema`] is what a consumer reads, and for an operation
    /// declaring this it is [`credential_handle_schema`] — `{ "credential": "tenants/…" }` — whatever
    /// else the author wrote. A `response_schema` beside this field documents the *vendor's wire
    /// body*, which the loader cross-checks does not carry the secret; it is never the operation's
    /// output. C-430's finding is why the distinction is spelled out rather than left implicit:
    /// deleting a location from a published schema removes the **disclosure** and leaves the
    /// **exposure**, because nothing between the vendor and a model-visible symbol projects a
    /// response. Only removing the value from what a caller receives removes the exposure, and that
    /// is what the diversion does.
    ///
    /// # Empty for every shipped connector today
    ///
    /// The four operations v0.9.0 and v0.9.1 withheld under `AGENTS.md` § Authentication contract
    /// are withheld for `credential_response`, and reinstating one is a separate change to that
    /// provider file. `skip_serializing_if` keeps the encoded IR of every connector shipped before
    /// C-136 byte-identical, so landing the field moves no `ir_sha256`, no `connectors.lock` entry
    /// and none of the generated artifacts — the same property
    /// [`credential_response`](Self::credential_response) and [`Operation::expose`] document, for
    /// the same reason.
    pub produces_credential: Option<ProducedCredential>,
    /// How this endpoint paginates, when it returns a collection.
    pub pagination: Option<Pagination>,
    /// The endpoint's published rate limit.
    pub rate_limit: Option<RateLimit>,
    /// Where structured vendor errors carry their code and message.
    pub error_envelope: Option<ErrorEnvelope>,
}

impl Serialize for Operation {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeMap as _;

        // Keep the established HTTP key order byte-for-byte: connector lock hashes cover encoded
        // IR, and adding a second driver must not churn every existing connector. SIP omits the two
        // HTTP-only entries while retaining the shared fields in the same relative order.
        let mut map = serializer.serialize_map(None)?;
        map.serialize_entry("id", &self.id)?;
        if !is_default_service(&self.service) {
            map.serialize_entry("service", &self.service)?;
        }
        if let OperationRequest::HttpV1 { method, .. } = &self.request {
            map.serialize_entry("method", method)?;
        }
        map.serialize_entry("direction", &self.direction)?;
        if let OperationRequest::HttpV1 { path, .. } = &self.request {
            map.serialize_entry("path", path)?;
        }
        if !self.description.is_empty() {
            map.serialize_entry("description", &self.description)?;
        }
        map.serialize_entry("risk", &self.risk)?;
        map.serialize_entry("idempotency", &self.idempotency)?;
        map.serialize_entry("effects", &self.effects)?;
        if !self.semantic_effects.is_empty() {
            map.serialize_entry("semantic_effects", &self.semantic_effects)?;
        }
        map.serialize_entry("interaction_shape", &self.interaction_shape)?;
        map.serialize_entry("protocol_driver", &self.request.driver())?;
        map.serialize_entry("placement_requirement", &self.placement_requirement)?;
        map.serialize_entry("implementation_form", &self.implementation_form)?;
        map.serialize_entry("required_capabilities", &self.required_capabilities)?;
        if let Some(value) = &self.repeatable_because {
            map.serialize_entry("repeatable_because", value)?;
        }
        if !is_exposed(&self.expose) {
            map.serialize_entry("expose", &self.expose)?;
        }
        if let Some(value) = &self.auth {
            map.serialize_entry("auth", value)?;
        }
        if !self.params.is_empty() {
            map.serialize_entry("params", &self.params)?;
        }
        if let Some(value) = &self.response_schema {
            map.serialize_entry("response_schema", value)?;
        }
        if !self.credential_response.is_empty() {
            map.serialize_entry("credential_response", &self.credential_response)?;
        }
        if let Some(value) = &self.produces_credential {
            map.serialize_entry("produces_credential", value)?;
        }
        if let Some(value) = &self.pagination {
            map.serialize_entry("pagination", value)?;
        }
        if let Some(value) = &self.rate_limit {
            map.serialize_entry("rate_limit", value)?;
        }
        if let Some(value) = &self.error_envelope {
            map.serialize_entry("error_envelope", value)?;
        }
        map.end()
    }
}

/// Deserialization spelling for [`Operation`]. Serde cannot combine `deny_unknown_fields` with a
/// flattened internally-tagged enum reliably, so this explicit wire shape keeps typo refusal while
/// constructing the closed driver union only after all top-level fields have been read.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct OperationWire {
    id: String,
    #[serde(default = "default_service")]
    service: String,
    #[serde(default)]
    method: Option<HttpMethod>,
    direction: OperationDirection,
    #[serde(default)]
    path: Option<String>,
    #[serde(default)]
    description: String,
    risk: Risk,
    idempotency: Idempotency,
    effects: Vec<HostEffect>,
    #[serde(default)]
    semantic_effects: Vec<SemanticEffect>,
    interaction_shape: InteractionShape,
    protocol_driver: ProtocolDriver,
    placement_requirement: PlacementRequirement,
    implementation_form: ImplementationForm,
    required_capabilities: Vec<RequiredCapability>,
    #[serde(default)]
    repeatable_because: Option<String>,
    #[serde(default = "exposed")]
    expose: bool,
    #[serde(default)]
    auth: Option<Vec<AuthRequirement>>,
    #[serde(default)]
    params: ParamSet,
    #[serde(default)]
    response_schema: Option<JsonSchema>,
    #[serde(default)]
    credential_response: Vec<String>,
    #[serde(default)]
    produces_credential: Option<ProducedCredential>,
    #[serde(default)]
    pagination: Option<Pagination>,
    #[serde(default)]
    rate_limit: Option<RateLimit>,
    #[serde(default)]
    error_envelope: Option<ErrorEnvelope>,
}

impl<'de> Deserialize<'de> for Operation {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let wire = OperationWire::deserialize(deserializer)?;
        let request = match (wire.protocol_driver, wire.method, wire.path) {
            (ProtocolDriver::HttpV1, Some(method), Some(path)) => {
                OperationRequest::HttpV1 { method, path }
            }
            (ProtocolDriver::HttpV1, None, _) => {
                return Err(serde::de::Error::missing_field("method"));
            }
            (ProtocolDriver::HttpV1, Some(_), None) => {
                return Err(serde::de::Error::missing_field("path"));
            }
            (ProtocolDriver::SipV1, None, None) => OperationRequest::SipV1,
            (ProtocolDriver::SipV1, _, _) => {
                return Err(serde::de::Error::custom(
                    "sip_v1 operation refuses HTTP-only `method` and `path`",
                ));
            }
            (ProtocolDriver::AudioV1, None, None) => OperationRequest::AudioV1,
            (ProtocolDriver::AudioV1, _, _) => {
                return Err(serde::de::Error::custom(
                    "audio_v1 operation refuses HTTP-only `method` and `path`",
                ));
            }
            (ProtocolDriver::CdpV1, None, None) => OperationRequest::CdpV1,
            (ProtocolDriver::CdpV1, _, _) => {
                return Err(serde::de::Error::custom(
                    "cdp_v1 operation refuses HTTP-only `method` and `path`",
                ));
            }
        };
        Ok(Self {
            id: wire.id,
            service: wire.service,
            request,
            direction: wire.direction,
            description: wire.description,
            risk: wire.risk,
            idempotency: wire.idempotency,
            effects: wire.effects,
            semantic_effects: wire.semantic_effects,
            interaction_shape: wire.interaction_shape,
            placement_requirement: wire.placement_requirement,
            implementation_form: wire.implementation_form,
            required_capabilities: wire.required_capabilities,
            repeatable_because: wire.repeatable_because,
            expose: wire.expose,
            auth: wire.auth,
            params: wire.params,
            response_schema: wire.response_schema,
            credential_response: wire.credential_response,
            produces_credential: wire.produces_credential,
            pagination: wire.pagination,
            rate_limit: wire.rate_limit,
            error_envelope: wire.error_envelope,
        })
    }
}

/// **A credential an operation mints**: where the secret arrives, and which declared credential it
/// is stored as.
///
/// Two facts, and the diversion needs both. Without [`secret`](Self::secret) the extractor would not
/// know what to divert; without [`credential`](Self::credential) there is no address to divert it
/// *to*, because a [`CredentialRef`](connector_address::CredentialRef) is composed from the
/// connector's `authority` and the credential's own leaf. The loader refuses a declaration missing
/// either, rather than inferring one — inferring "the field called `access_token`" is exactly the
/// name-shaped guessing C-430 measured to be wrong nine times in ten.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProducedCredential {
    /// **Where the secret arrives in the vendor's response body** — one
    /// [`response_location_exists`] pointer, `/access_token`.
    ///
    /// The same vocabulary as [`Operation::credential_response`], deliberately: two spellings of
    /// "a location in a response" would be two resolvers to keep in step, and the pair of fields
    /// describes one thing from two sides.
    ///
    /// **Minus the `*`.** That extension names every element of an array, which
    /// `credential_response` needs because it describes where credentials *appear* — postmark's
    /// `Servers[].ApiTokens` is a real array of them. A **mint** is one call storing one value at
    /// one address, so `*` would name several secrets with nothing to say which is the credential.
    /// The loader refuses it, which also keeps this field's documented behaviour and the runtime's
    /// the same thing: `connector_pack::mint` resolves the location with
    /// `serde_json::Value::pointer`, which has no wildcard.
    pub secret: String,
    /// **Which declared credential the value is stored as** — an [`AuthMethod::name`], e.g.
    /// `babelforce.access_token`.
    ///
    /// A name rather than an address: the address is the host's to render, from the tenant it
    /// authenticated, the connector's `authority` and this credential's leaf. This repository owns
    /// the address and never the store, and a field holding a rendered path here would be a second
    /// spelling of one C-90 already owns.
    pub credential: String,
}

/// **The property name a diverted credential's handle is returned under.**
///
/// One spelling, shared by the schema below and by `connector-pack`'s runtime, so the declared
/// output and the effective output cannot drift into two different words.
pub const CREDENTIAL_HANDLE_FIELD: &str = "credential";

/// **The declared output of every credential-producing operation**: the handle, and nothing else.
///
/// `{ "credential": "tenants/<tenant>/<authority>/…" }`. It is derived rather than authored, for the
/// reason `Level` is in `predecessor:docs/designs/connector-configuration.md`: a value a author could state
/// differently per operation is a value some operation will eventually state wrongly, and here
/// "wrongly" means a published schema that promises a caller the secret.
pub fn credential_handle_schema() -> JsonSchema {
    serde_json::json!({
        "type": "object",
        "description": "A handle to the credential this call minted. The secret itself is never \
                        returned: it travels from the vendor's response straight into the host's \
                        credential store, and downstream operations name this address rather than \
                        reading a value.",
        "properties": {
            CREDENTIAL_HANDLE_FIELD: {
                "type": "string",
                "description": "The credential's address, e.g. \
                                `tenants/<tenant>/<authority>/<credential>`.",
            }
        },
        "required": [CREDENTIAL_HANDLE_FIELD],
        "additionalProperties": false,
    })
}

/// **Whether a response location resolves against a response schema**, walking nested schemas.
///
/// A location is a JSON Pointer into the *response value* — `/start_url`, `/Servers/*/ApiTokens` —
/// with one extension: `*` stands for "every element of an array". It must start with `/`, so there
/// is exactly one spelling of a location and not two.
///
/// # Why the nesting is the whole of it
///
/// The scan that found C-430's violations was run twice. The first pass walked `properties` one
/// level deep and found three; the second walked nested schemas and found a fourth, Postmark's
/// `ApiTokens` — an array of live tokens in plaintext, sitting under `Servers[]`, and the worst of
/// the set. A resolver that stops at the root would have let exactly that declaration pass as
/// "matches nothing", which is why the walk descends through `items` as well as `properties` and why
/// `tests/credential_response.rs` pins the nested case in both directions.
///
/// # Resolving to nothing is never silence
///
/// The caller's contract is that `false` is a **loud** answer: `validate_operations` turns it into a
/// refusal naming the location. A pointer that matches nothing protects nothing, and it is the shape
/// a vendor rename takes — a declaration that quietly stopped applying reads as protection while
/// being none, which is worse than never having been written.
pub fn response_location_exists(schema: &JsonSchema, location: &str) -> bool {
    let Some(path) = location.strip_prefix('/') else {
        return false;
    };

    let mut current = schema;
    for segment in path.split('/') {
        let Some(object) = current.as_object() else {
            return false;
        };
        current = if segment == "*" {
            // The one extension to JSON Pointer: an array is walked by element, never by index. An
            // index would name whichever element happened to be at it, and a credential in the
            // second entry is the same credential as one in the first.
            match object.get("items") {
                Some(items) => items,
                None => return false,
            }
        } else {
            // `~1` and `~0` are JSON Pointer's own escapes for `/` and `~`, unescaped in that order
            // per RFC 6901 — the reverse order turns a literal `~1` into a separator.
            let name = segment.replace("~1", "/").replace("~0", "~");
            match object.get("properties").and_then(|properties| {
                properties
                    .as_object()
                    .and_then(|properties| properties.get(&name))
            }) {
                Some(member) => member,
                None => return false,
            }
        };
    }

    true
}

impl Operation {
    /// **The stated condition that licenses `conditional` on this operation**, or `None`.
    ///
    /// One reading of [`Operation::repeatable_because`], shared by the loader (which refuses a file
    /// stating a condition that says nothing) and by `connector-flux`'s `check_write_metadata`
    /// (which must not trust an IR assembled in memory rather than loaded). Two implementations of
    /// "is this a real condition" would be two places for the floor to drift apart, and the emitter
    /// is the one that has to be right.
    ///
    /// **The `trim` is load-bearing, not cosmetic**: without it
    /// [`MIN_REPEATABILITY_CONDITION`] spaces would satisfy the floor and unlock the claim while
    /// saying literally nothing. `tests/repeatability_condition.rs` asserts that case directly.
    pub fn repeatability_condition(&self) -> Option<&str> {
        let condition = self.repeatable_because.as_deref()?.trim();
        (condition.chars().count() >= MIN_REPEATABILITY_CONDITION).then_some(condition)
    }

    /// Whether this operation *states* a condition at all, however poor.
    ///
    /// The difference from [`Operation::repeatability_condition`] is what the loader needs to tell
    /// an author apart from silence: "you wrote nothing" and "you wrote `yes`" are different
    /// mistakes and deserve different refusals, and a field present-but-rejected must never read as
    /// a field absent.
    pub fn states_repeatability_condition(&self) -> bool {
        self.repeatable_because.is_some()
    }

    /// **What a caller of this operation actually receives**, as opposed to what the vendor sends.
    ///
    /// The two are the same for every operation except a credential-producing one, where the
    /// difference is the whole of C-136: the secret travels into the store and the caller receives
    /// [`credential_handle_schema`]. Read this rather than [`response_schema`](Self::response_schema)
    /// anywhere a consumer is being told what the operation returns — the public catalogue does
    /// (`connector-cli`'s `site`), because a published schema promising the secret is precisely the
    /// disclosure C-430 refused to fix by deletion.
    ///
    /// Derived, never authored. An author who could spell the output would eventually spell one
    /// that contains the secret, and a schema is not the kind of thing a reviewer re-reads.
    pub fn effective_response_schema(&self) -> Option<JsonSchema> {
        if self.produces_credential.is_some() {
            return Some(credential_handle_schema());
        }
        self.response_schema.clone()
    }

    /// **One JSON Schema describing everything the operation receives.**
    ///
    /// Derived, never authored: there is no `input_schema` key in a provider file, and one is a
    /// load error — `Operation` is `deny_unknown_fields` and the editor-facing
    /// [`crate::PROVIDER_TOML_JSON_SCHEMA`] closes the operation object as well. The same rule
    /// `Level` follows in `predecessor:docs/designs/connector-configuration.md`, for the same reason: a value
    /// that is both derived and writable is two sources of truth waiting to disagree.
    ///
    /// # Why this exists at all
    ///
    /// The data was already here — [`Param::schema`] on every parameter, plus
    /// [`ParamSet::body_schema`] — but split across four request positions with nothing saying what
    /// the operation *receives*. Every consumer therefore composed its own, and each disagreed at
    /// the corners: whether a path parameter belongs in the same object as a body field, what
    /// `required` means, how a free-form body merges with named ones. One composition settles it.
    ///
    /// # The rules, each of which was a corner someone got wrong
    ///
    /// - **Keyed by the caller-facing [`Param::name`], never the wire name.** The vendor's own
    ///   spelling stays exactly where it already lives, in [`Param::wire`]; a schema keyed by it
    ///   would name arguments no caller passes. Where the caller-facing name is not a spellable
    ///   symbol — babelforce's `time.start` — the document's contract declares a normalized symbol
    ///   instead, and that mapping lives in this crate's [`crate::names`] module, which owns it
    ///   (S-001; the predecessor's emitter owned it before).
    /// - **Each property is the declared schema, verbatim.** This composes; it does not enrich.
    ///   Folding [`Param::description`] in would be this function improving a vendor's declaration,
    ///   and raising declaration quality is its own work.
    /// - **A free-form [`ParamSet::body_schema`] is one property, [`FREE_FORM_BODY`]** — the same
    ///   name the emitted `op` declares it under, so both surfaces ask for the body by one name.
    ///   It is required: the body *is* the request for the operations that declare one.
    /// - **`required` is exactly the parameters marked [`Param::required`].** Not everything (which
    ///   is what a consumer reading the emitted Flux must say, since flux has no optional
    ///   composite-op parameter) and not nothing.
    /// - **[`ParamSet::const_headers`] are absent.** They are not parameters and nothing about them
    ///   is caller-supplied, exactly as [`ParamSet::iter`] already has it.
    /// - **An operation with no parameters composes an empty object schema, not absence.** "This
    ///   takes nothing" is a derived answer and deserves to be stated; the output side publishes
    ///   absence instead precisely because "we do not know" is *not* an answer
    ///   (`predecessor:docs/designs/member-io-schemas.md`).
    ///
    /// # The merge rule for a body declared twice
    ///
    /// Named [`ParamSet::body`] fields and a free-form [`ParamSet::body_schema`] are two answers to
    /// one question, and **the answer is that an operation declaring both is refused at load**
    /// (`provider::load`; `connector-flux` refuses it a second time at emission). So the case does
    /// not arise for any connector that came through the loader, and this function does not invent
    /// a merge for it. An IR assembled in memory can still carry both, and then both appear here —
    /// stated so it is not a surprise, and deliberately *not* a rule that drops one silently, which
    /// would make the refusal look optional.
    pub fn input_schema(&self) -> JsonSchema {
        let mut properties = serde_json::Map::new();
        let mut required: Vec<JsonSchema> = Vec::new();

        for param in self.params.iter() {
            properties.insert(param.name.clone(), param.schema.clone());
            if param.required {
                required.push(JsonSchema::String(param.name.clone()));
            }
        }
        if let Some(schema) = &self.params.body_schema {
            properties.insert(FREE_FORM_BODY.to_owned(), schema.clone());
            required.push(JsonSchema::String(FREE_FORM_BODY.to_owned()));
        }

        serde_json::json!({
            "type": "object",
            "properties": JsonSchema::Object(properties),
            "required": JsonSchema::Array(required),
        })
    }
}

/// A whole connector: the normalized form of one provider, whether it came from a vendor OpenAPI
/// document or was hand-authored in TOML.
///
/// The two front-ends produce this same shape — spec ingest merely *pre-fills* it — which is what
/// lets a vendor with no usable spec travel the identical codegen path.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Connector {
    /// The connector id, e.g. `babelforce`. Prefixes every operation id and names the generated
    /// `<id>.flux` and `<id>.connector.toml`.
    pub id: String,
    /// The reverse-DNS authority this provider publishes under (`com.amazonaws`) — the `pid` of
    /// C-37's address scheme, and the leading component of every service gid.
    ///
    /// Optional: a connector that declares none has no rendered address at all. Every shipped
    /// provider was in that state until `slack` needed one — a channel binding's reply renders as an
    /// oip, and an address missing its authority is not an address. See [`crate::address`].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub authority: Option<String>,
    /// The vendor's API version for this connector, as the *default* for its services.
    ///
    /// A multi-service provider overrides it per [`Service`]; a single-surface provider states it
    /// once here. It is the vendor's version, never ours — our own connector version lives in
    /// `connectors.lock` — so an address stays stable across our regenerations.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub api_version: Option<String>,
    /// The API surfaces this provider exposes, each owning its operations.
    ///
    /// Empty means the provider has exactly one surface, the reserved [`DEFAULT_SERVICE`], which no
    /// entry declares and which is elided from every rendered address. A provider that declares any
    /// service must place **every** operation in one of them — an operation that omits `service` in
    /// such a file is a loader error, not an implicit `default` sitting beside the declared ones.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub services: Vec<Service>,
    /// The vendor's display name.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub vendor: String,
    /// The API base URL, which may carry tenant templating (`https://{tenant}.babelforce.com`).
    pub base_url: String,
    /// What the connector is for, in one line.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub description: String,
    /// Every credential this connector declares, each keyed by its [`AuthMethod::name`].
    ///
    /// A `Vec` rather than a map, mirroring the plugin manifest's `Vec<AuthMethod>`: the name
    /// already lives inside the method, so a keyed map would store it twice and invite the two
    /// copies to disagree. Look one up with [`auth_method`](Self::auth_method).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub auth: Vec<AuthMethod>,
    /// The connector-wide default requirement list — OpenAPI's document-level `security`.
    ///
    /// Every operation that does not declare its own [`Operation::auth`] inherits this. An empty
    /// list means the connector is unauthenticated by default.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub default_auth: Vec<AuthRequirement>,
    /// The operations this connector exposes — the **outbound** call direction.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub operations: Vec<Operation>,
    /// The events this connector receives — the **inbound** call direction.
    ///
    /// An operation is flux calling the vendor; an [`EventDecl`] is the vendor calling flux. Both are
    /// members of a [`Service`], and both share one name namespace within it — see
    /// [`member_names_of`](Self::member_names_of).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub events: Vec<EventDecl>,
    /// The ingress surfaces this connector describes, each composing events with a reply operation.
    ///
    /// The third member kind, and the one that is not a primitive: see [`crate::inbound`] for why a
    /// [`ChannelBinding`] is a composition of the two above rather than a thing of its own.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub channels: Vec<ChannelBinding>,
    /// Bounded observations this Provider can produce about target Provider instances.
    ///
    /// Discovery is catalog metadata over an ordinary read Operation. It is neither a callable
    /// member kind nor authority, and therefore does not share the member namespace.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub discoveries: Vec<Discovery>,
    /// What a **human** must supply before any of the above can run — see [`crate::config`].
    ///
    /// Not a member kind in the addressing sense: a configuration field is not something a program
    /// calls or a trigger matches, it is a question a settings page asks. It shares the member
    /// namespace anyway, because it shares the *host's* namespace — a config value and an operation
    /// resolving to one name would be ambiguous wherever a host looks either up.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub config: Vec<ConfigField>,
    /// The flows this connector composes from its own members — see [`crate::graph`].
    ///
    /// A graph is the only member kind that is itself a *composition of members*: its nodes name
    /// operations, events and channel bindings this connector already declares. It lowers to one Flux
    /// `op`, which is the only declaration flux lifts from a module.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub graphs: Vec<Graph>,
    /// The operation a host calls to prove the configuration works — a "Test connection" button.
    ///
    /// Read-shaped and cheap by construction: `freshdesk-test` is a bounded contact read,
    /// `babelforce-agent-list` doubles as one. Both already existed as a convention nothing declared,
    /// so nothing could use them.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub verify: Option<String>,
    /// Where this connector came from.
    #[serde(default, skip_serializing_if = "provenance_is_empty")]
    pub provenance: Provenance,
}

/// `skip_serializing_if` helper — a fully empty [`Provenance`] adds nothing to the encoding.
fn provenance_is_empty(provenance: &Provenance) -> bool {
    *provenance == Provenance::default()
}

/// `serde(default)` for [`Operation::service`]: an operation that names none belongs to the reserved
/// [`DEFAULT_SERVICE`]. Shared with the two other member kinds in [`crate::inbound`], which carry the
/// same field for the same reason.
pub(crate) fn default_service() -> String {
    DEFAULT_SERVICE.to_owned()
}

/// `skip_serializing_if` for [`Operation::service`], so that omitting the key and writing
/// `service = "default"` produce **identical bytes**. One meaning, one encoding — and the encoding of
/// every operation shipped before services existed is unchanged, which is what keeps
/// `connectors.lock` from churning for a connector nobody edited.
pub(crate) fn is_default_service(service: &str) -> bool {
    service == DEFAULT_SERVICE
}

/// `serde(default)` for [`Operation::expose`] and [`Graph::expose`](crate::Graph::expose).
///
/// A free function rather than `bool::default`, which is `false` — the wrong way round here, and
/// wrong in the direction that matters: a field defaulting to unexposed would hide every operation
/// that said nothing, which is a decision made by silence rather than by an author.
pub(crate) fn exposed() -> bool {
    true
}

/// `skip_serializing_if` for the two `expose` fields, so that omitting the key and writing
/// `expose = true` produce **identical bytes**.
///
/// One meaning, one encoding — the rule [`is_default_service`] already follows, and the reason
/// landing C-413 moved no `ir_sha256` and regenerated none of the 557 committed artifacts.
#[expect(
    clippy::trivially_copy_pass_by_ref,
    reason = "serde's `skip_serializing_if` calls this with a reference to the field"
)]
pub(crate) fn is_exposed(expose: &bool) -> bool {
    *expose
}

impl Connector {
    /// The auth alternatives that actually apply to an operation, resolving the inheritance rule.
    ///
    /// An operation that declares nothing inherits [`default_auth`](Self::default_auth); one that
    /// declares an explicit empty list gets an empty slice and inherits nothing. This is the only
    /// correct way to read [`Operation::auth`], and C-10 resolves credentials through it.
    pub fn effective_auth<'a>(&'a self, operation: &'a Operation) -> &'a [AuthRequirement] {
        match operation.auth.as_deref() {
            Some(declared) => declared,
            None => &self.default_auth,
        }
    }

    /// The auth alternatives that admit an inbound event for a Connection.
    ///
    /// Unlike operation auth, event auth does not inherit `default_auth`: that default describes
    /// outbound requests, while an inbound subscription is an independent vendor capability. A
    /// channel's auth and verification establish how bytes arrive; an event's declared auth proves
    /// that the Connection credential was granted the capability required to subscribe to it.
    pub fn effective_event_auth<'a>(&'a self, event: &'a EventDecl) -> &'a [AuthRequirement] {
        match event.auth.as_deref() {
            Some(declared) => declared,
            None => &[],
        }
    }

    /// The declared credential of that name, or `None` when nothing declares it.
    ///
    /// A requirement naming an undeclared credential is an authoring error the loader rejects
    /// (C-3); codegen must never invent one, because that would be connectors deciding on its
    /// own how to spend a credential.
    pub fn auth_method(&self, name: &str) -> Option<&AuthMethod> {
        self.auth.iter().find(|method| method.name == name)
    }

    /// An operation by id.
    pub fn operation(&self, id: &str) -> Option<&Operation> {
        self.operations.iter().find(|op| op.id == id)
    }

    /// Every service name this connector has, in declaration order.
    ///
    /// A connector that declares none has exactly one service, the reserved [`DEFAULT_SERVICE`]. This
    /// is the set an operation's [`service`](Operation::service) must be a member of — the loader
    /// enforces it — and therefore the set that makes the partition below total.
    pub fn service_names(&self) -> Vec<&str> {
        if self.services.is_empty() {
            vec![DEFAULT_SERVICE]
        } else {
            self.services
                .iter()
                .map(|service| service.name.as_str())
                .collect()
        }
    }

    /// The declared [`Service`] of that name, or `None`.
    ///
    /// `None` for [`DEFAULT_SERVICE`] on a connector that declares no services: the implicit service
    /// has no declaration by construction. Use [`service_names`](Self::service_names) to ask *which*
    /// services exist and the resolvers below to ask about their fields.
    pub fn service(&self, name: &str) -> Option<&Service> {
        self.services.iter().find(|service| service.name == name)
    }

    /// Whether this connector has exactly one service and it is the reserved [`DEFAULT_SERVICE`].
    ///
    /// The shape of every provider shipped today, and the shape whose emitted artifacts are named
    /// `<provider>.flux` rather than `<provider>-<service>.flux`.
    ///
    /// Declaring no services at all is the usual spelling, but not the only one: C-120 lets a
    /// single-surface provider write a `[[services]]` entry named [`DEFAULT_SERVICE`] **to carry
    /// roles**, because a role attaches to a service and there is no other service to attach it to.
    /// Such an entry names the service that already exists rather than adding one, so it must not
    /// rename a single artifact — which is what this predicate decides.
    pub fn is_default_only(&self) -> bool {
        self.services
            .iter()
            .all(|service| service.name == DEFAULT_SERVICE)
    }

    /// The provider's roles: the union of its services', deduplicated, in declaration order.
    ///
    /// **Derived, never authored.** A provider-level `roles` key is a load error, for the reason
    /// [`Level`](crate::config::Level) is derived from `binds`: an author who could state it could
    /// state it wrongly, and then two sources of truth would disagree about what the connector can
    /// do. It is also the wrong *level* — `openai`'s model-listing surface and its chat surface are
    /// different capabilities of one vendor, and a provider-level role would smear them together.
    ///
    /// A `Vec` rather than a set type: the order is the declaration order, which is what a catalogue
    /// and a UI want, and the loader has already refused a repeat within one service.
    pub fn roles(&self) -> Vec<Role> {
        let mut union: Vec<Role> = Vec::new();
        for role in self.services.iter().flat_map(|service| &service.roles) {
            if !union.contains(role) {
                union.push(*role);
            }
        }
        union
    }

    /// The provider's tags: the union of its services', deduplicated, in declaration order — C-153.
    ///
    /// **Derived, never authored**, exactly as [`roles`](Self::roles) is, and for the same two
    /// reasons. An author who could state it could state it wrongly, leaving two sources of truth to
    /// disagree; and it is the wrong level, because a multi-service provider is usually several kinds
    /// of thing at once — `google`'s `gmail` is email while its `drive` is storage.
    ///
    /// A `Vec` rather than a set type: the order is declaration order, which is what a catalogue and
    /// a UI want, and the loader has already refused a repeat within one service.
    pub fn tags(&self) -> Vec<Tag> {
        let mut union: Vec<Tag> = Vec::new();
        for tag in self.services.iter().flat_map(|service| &service.tags) {
            if !union.contains(tag) {
                union.push(*tag);
            }
        }
        union
    }

    /// The provider's audiences: the union of its services', deduplicated in declaration order.
    ///
    /// Derived rather than authored at provider level so a multi-service provider cannot contradict
    /// its own service metadata. Consumers may use this for discovery and presentation only.
    pub fn audiences(&self) -> Vec<Audience> {
        let mut union: Vec<Audience> = Vec::new();
        for audience in self.services.iter().flat_map(|service| &service.audiences) {
            if !union.contains(audience) {
                union.push(*audience);
            }
        }
        union
    }

    /// The required members of `role` that `service` does not have, in the role's own order.
    ///
    /// Empty when the service satisfies the claim. The loader calls this to refuse a claim that does
    /// not hold; a consumer can call it to explain one. Matching is by member name *within the
    /// service* — see [`fills_slot`].
    ///
    /// # Only an operation fills a slot
    ///
    /// Deliberately [`operations_of`](Self::operations_of) and **not**
    /// [`member_names_of`](Self::member_names_of), even though the latter is the per-service
    /// namespace every other rule here uses. A role is a claim that something is *callable*: a
    /// consumer resolving `llm_catalogue` intends to call the listing.
    ///
    /// The other member kinds cannot answer that. An [`EventDecl`] and a [`ChannelBinding`] reach the
    /// manifest and the catalogue and are emitted into no module at all — flux lifts `op`
    /// declarations only — so a service whose only `list` is an event named `models.list` would
    /// publish a live-listing capability that nothing can call. That is "an event dressed up as a
    /// pollable op", which this repository refuses on sight. A [`ConfigField`](crate::ConfigField) is
    /// not addressable in the calling sense either.
    ///
    /// [`Graph`](crate::graph::Graph) is the one arguable exclusion: it *does* lower to an emitted
    /// `op`. It stays out because a role is a vendor's API shape and a graph is a composition this
    /// repository wrote over it — admitting one would let a provider satisfy a role by wrapping
    /// members that do not implement it. Nothing needs it today; widening later is cheap.
    pub fn missing_role_members(&self, service: &str, role: Role) -> Vec<&'static str> {
        let callable: Vec<&str> = self
            .operations_of(service)
            .map(|operation| operation.id.as_str())
            .collect();
        role.required_members()
            .iter()
            .copied()
            .filter(|slot| !callable.iter().any(|member| fills_slot(member, slot)))
            .collect()
    }

    /// The operations belonging to one service, in IR order.
    ///
    /// Together with [`service_names`](Self::service_names) this is the partition: the sets are
    /// pairwise disjoint because [`Operation::service`] holds one name, and their union is every
    /// operation because the loader refuses an operation naming a service that does not exist.
    pub fn operations_of<'a>(&'a self, service: &'a str) -> impl Iterator<Item = &'a Operation> {
        self.operations
            .iter()
            .filter(move |operation| operation.service == service)
    }

    /// The events belonging to one service, in IR order. The inbound half of the partition
    /// [`operations_of`](Self::operations_of) documents.
    pub fn events_of<'a>(&'a self, service: &'a str) -> impl Iterator<Item = &'a EventDecl> {
        self.events
            .iter()
            .filter(move |event| event.service == service)
    }

    /// The channel bindings belonging to one service, in IR order.
    pub fn channels_of<'a>(&'a self, service: &'a str) -> impl Iterator<Item = &'a ChannelBinding> {
        self.channels
            .iter()
            .filter(move |channel| channel.service == service)
    }

    /// The configuration fields belonging to one service, in IR order.
    pub fn config_of<'a>(&'a self, service: &'a str) -> impl Iterator<Item = &'a ConfigField> {
        self.config
            .iter()
            .filter(move |field| field.service == service)
    }

    /// The configuration fields that **fill** one service's base-URL placeholders, in IR order.
    ///
    /// Wider than [`config_of`](Self::config_of), and the difference is C-529's: a field may name
    /// sibling services in `also_services`, filling their `{variable}` from its own single address.
    /// GitLab's API surface and its OAuth surface are one deployment, so one operator-approved
    /// origin fills both.
    ///
    /// Use this wherever the question is *"can this service's URL be composed"*; use `config_of`
    /// wherever the question is *"which values does this service own"*, since the head service is
    /// still the one the value is stored under.
    pub fn config_filling<'a>(&'a self, service: &'a str) -> impl Iterator<Item = &'a ConfigField> {
        self.config.iter().filter(move |field| {
            field.service == service || field.also_services.iter().any(|extra| extra == service)
        })
    }

    /// A configuration field by name.
    pub fn config_field(&self, name: &str) -> Option<&ConfigField> {
        self.config.iter().find(|field| field.name == name)
    }

    /// The graphs belonging to one service, in IR order.
    pub fn graphs_of<'a>(&'a self, service: &'a str) -> impl Iterator<Item = &'a Graph> {
        self.graphs.iter().filter(move |g| g.service == service)
    }

    /// A graph by name.
    pub fn graph(&self, name: &str) -> Option<&Graph> {
        self.graphs.iter().find(|g| g.name == name)
    }

    /// Every member name of one service — operations, then events, then channels, then config.
    ///
    /// **The three kinds share one namespace**, which is why this returns them together rather than
    /// leaving a caller to concatenate three lists and get the rule wrong. Two reasons it has to be
    /// one namespace, and the second is the load-bearing one:
    ///
    /// 1. All three project into the same address space. `com.slack.api:v1#slack` must denote one
    ///    thing, and an [`Oip`](crate::Oip) carries no kind discriminator to tell two apart.
    /// 2. All three project into flux's declaration namespace. An operation is an `op` a model calls
    ///    by name and an event is a trigger label; a collision would resolve differently depending on
    ///    which surface asked, which is the kind of ambiguity that is cheap to forbid now and
    ///    impossible to fix after publication.
    ///
    /// The loader refuses a duplicate, so on a loaded connector this list has no repeats.
    pub fn member_names_of<'a>(&'a self, service: &'a str) -> Vec<&'a str> {
        self.operations_of(service)
            .map(|operation| operation.id.as_str())
            .chain(self.events_of(service).map(|event| event.name.as_str()))
            .chain(
                self.channels_of(service)
                    .map(|channel| channel.name.as_str()),
            )
            .chain(self.config_of(service).map(|field| field.name.as_str()))
            .chain(self.graphs_of(service).map(|graph| graph.name.as_str()))
            .collect()
    }

    /// An event by name.
    pub fn event(&self, name: &str) -> Option<&EventDecl> {
        self.events.iter().find(|event| event.name == name)
    }

    /// A channel binding by name.
    pub fn channel(&self, name: &str) -> Option<&ChannelBinding> {
        self.channels.iter().find(|channel| channel.name == name)
    }

    /// The base URL a call to that service reaches: the service's own override, else the connector's.
    pub fn base_url_of(&self, service: &str) -> &str {
        self.service(service)
            .and_then(|service| service.base_url.as_deref())
            .unwrap_or(&self.base_url)
    }

    /// The vendor API version of that service: the service's own, else the connector's.
    ///
    /// `None` when neither states one, in which case the service has no renderable
    /// [`gid`](Self::gid_of) — stated at the accessor rather than papered over with a placeholder
    /// version that would become part of a published address.
    pub fn api_version_of(&self, service: &str) -> Option<&str> {
        self.service(service)
            .and_then(|service| service.api_version.as_deref())
            .or(self.api_version.as_deref())
    }

    /// The provider's `pid` — C-37's top address level — when it declares an authority.
    pub fn pid(&self) -> Option<crate::address::Pid> {
        Some(crate::address::Pid::new(self.authority.as_deref()?))
    }

    /// One service's `gid`: `com.amazonaws/s3:2006-03-01`, with [`DEFAULT_SERVICE`] elided.
    ///
    /// `None` unless the connector declares an authority *and* the service resolves an API version:
    /// an address missing either half is not an address, and inventing one is how a wrong identifier
    /// gets published under a stability contract that forbids reusing it.
    pub fn gid_of(&self, service: &str) -> Option<crate::address::Gid> {
        Some(crate::address::Gid::new(
            self.authority.as_deref()?,
            service,
            self.api_version_of(service)?,
        ))
    }

    /// One operation's `oip`: `com.amazonaws/s3:2006-03-01#object-get`.
    ///
    /// The fragment is [`Operation::id`], which stays the declarable Flux symbol — C-37's design
    /// records why the two identifiers are separate rather than one richer one.
    pub fn oip_of(&self, operation: &Operation) -> Option<crate::address::Oip> {
        self.oip_of_member(&operation.service, &operation.id)
    }

    /// One member's `oip`, whatever its kind: `com.slack.api:v1#app-mention`.
    ///
    /// Operations, events and channel bindings share one namespace per service
    /// ([`member_names_of`](Self::member_names_of)), so they share one address form and the `#`
    /// fragment needs no kind discriminator. `name` is assumed to be a member of `service` — the
    /// loader has already refused one that is not.
    pub fn oip_of_member(&self, service: &str, name: &str) -> Option<crate::address::Oip> {
        Some(crate::address::Oip {
            gid: self.gid_of(service)?,
            operation: name.to_owned(),
        })
    }

    /// Where one tenant's value for one of this connector's credentials is kept.
    ///
    /// ```text
    /// tenants/<tenant>/com.zendesk.api/api_token
    /// ```
    ///
    /// The three outcomes are distinct on purpose, because they have different owners:
    ///
    /// - `Err` — **the caller's tenant id is unusable.** It is untrusted input, so this is the
    ///   expected failure and it names what is wrong with it.
    /// - `Ok(None)` — **this connector has no `authority`**, so no address renders. The same answer
    ///   [`gid_of`](Self::gid_of) gives, for the same reason: inventing one publishes a wrong
    ///   identifier under a contract that forbids reusing it. Fifteen of the sixteen shipped
    ///   providers are in this state.
    /// - `Ok(Some)` — the address.
    ///
    /// # The service segment
    ///
    /// A credential is declared at **provider** level ([`Connector::auth`]), so this always renders
    /// the reserved [`DEFAULT_SERVICE`] — which is elided, giving the three-segment form above. The
    /// path *can* carry a service ([`CredentialRef::new`]), and that is deliberate headroom: a vendor
    /// whose surfaces authenticate separately is expressible the day one appears, without moving
    /// every path that already exists. Nothing today needs it, because credential names are unique
    /// within a provider and already distinguish the case.
    ///
    /// # The instance segment
    ///
    /// A connector is declared once; a *tenant* may connect it twice — two Zendesk subdomains, a
    /// sandbox and a production Jira. Nothing this type knows varies per connection, so `instances`
    /// carries the fact from the only place that holds it (C-406). Pass
    /// [`TenantInstances::sole`](crate::TenantInstances::sole) for the single-connection case, which
    /// is every address that exists today and renders byte-identically to what it always has.
    ///
    /// ```text
    /// tenants/<tenant>/com.zendesk.api/@instances/<uuid>/api_token
    /// ```
    ///
    /// # Errors
    ///
    /// A reason string when the tenant id is unusable, when `credential` is not one this connector
    /// declares, or when the tenant holds several connections and the reference names none — that
    /// last one lists the uuids that would have worked rather than picking one.
    pub fn credential_ref_for(
        &self,
        tenant: &str,
        credential: &str,
        instances: crate::credential::TenantInstances<'_>,
    ) -> crate::Result<Option<crate::credential::CredentialRef>> {
        let leaf = self.local_credential_name(credential)?;
        // The authority is checked first, so "this connector has no address at all" stays a single
        // answer rather than one that depends on how many times a tenant connected it.
        let Some(authority) = self.authority.as_deref() else {
            return Ok(None);
        };
        let instance = instances.resolve().map_err(crate::Error::Invalid)?;
        match instance {
            Some(instance) => crate::credential::CredentialRef::for_instance(
                tenant,
                authority,
                instance.as_str(),
                DEFAULT_SERVICE,
                leaf,
            ),
            None => crate::credential::CredentialRef::new(tenant, authority, DEFAULT_SERVICE, leaf),
        }
        .map(Some)
        .map_err(crate::Error::Invalid)
    }

    /// The local part of a declared credential's name — `api_token` from `zendesk.api_token`.
    ///
    /// Credential names are vendor-prefixed because they live in **one flat namespace** across the
    /// connector. A path does not need the prefix: the authority above it already says which vendor,
    /// so carrying it would say the same thing twice and put a `.` inside what must be one segment.
    ///
    /// # Errors
    ///
    /// When nothing declares `credential`, or when its prefix disagrees with the connector id — the
    /// latter is an authoring slip that would otherwise produce a plausible path pointing at the
    /// wrong vendor's namespace.
    pub fn local_credential_name<'a>(&self, credential: &'a str) -> crate::Result<&'a str> {
        if self.auth_method(credential).is_none() {
            return Err(crate::Error::Invalid(format!(
                "credential {credential:?} is not declared by connector {:?}",
                self.id
            )));
        }
        let prefix = format!("{}.", self.id);
        credential.strip_prefix(&prefix).ok_or_else(|| {
            crate::Error::Invalid(format!(
                "credential {credential:?} is not prefixed {prefix:?}, so its local name cannot be \
                 derived. A credential name is `<connector>.<name>` because credentials share one \
                 flat namespace; a disagreeing prefix would render a path under the wrong vendor"
            ))
        })
    }

    /// The connector's canonical JSON encoding: compact, and byte-identical for equal values.
    ///
    /// This is the connector's **complete** encoding, provenance included, and it is what a round
    /// trip through disk must preserve. It is deliberately *not* what `connectors.lock` hashes —
    /// see [`hash_domain`](Self::hash_domain) for why. Every ordering decision in this module
    /// exists to make this function total in the mathematical sense: the same value in, the same
    /// bytes out, on every machine and every run.
    pub fn canonical_json(&self) -> crate::Result<String> {
        Ok(serde_json::to_string(self)?)
    }

    /// The exact bytes [`ir_sha256`](Self::ir_sha256) hashes — the **hash domain**, stated as a
    /// value rather than left implicit in whatever `Serialize` happens to emit.
    ///
    /// # What is in it
    ///
    /// The connector's *compiled meaning*: `id`, `authority`, `runtime`, `api_version`, `services`, `vendor`,
    /// `base_url`, `description`, `auth`, `default_auth`, `operations`, `events`, `channels` — and,
    /// inside each member, its `service`. Change any of those and a generated artifact changes, so
    /// the hash must move. C-49's service fields are in for exactly this reason: a service owns the
    /// base URL a call reaches and names the module it is emitted into.
    ///
    /// The inbound members are in even though **nothing about them reaches the `.flux` module** — a
    /// channel binding is declared into the manifest and the catalogue, never emitted as code. The
    /// hash domain is the connector's compiled meaning, not the module's bytes, and a binding whose
    /// reply operation or verification scheme moved is a connector that changed. Leaving them out
    /// would make `connectors check` blind to a drifting event schema, which is the one thing
    /// the lockfile exists to catch.
    ///
    /// # What is out of it, and why
    ///
    /// **All of [`Provenance`]** — including `fetched_at`, which C-2's review caught leaking into
    /// [`canonical_json`](Self::canonical_json). Provenance records *where the bytes came from*;
    /// this hash records *what was compiled from them*. Two reasons, and the first is the one the
    /// story exists for:
    ///
    /// 1. `fetched_at` moves on every re-fetch of a byte-identical document, so hashing it would
    ///    report drift where nothing drifted — phantom drift on every build, which is precisely
    ///    what `connectors.lock` is supposed to rule out.
    /// 2. `source_url`, `upstream_version`, `spec_sha256` and `toml_sha256` are recorded verbatim
    ///    as their own fields of the lockfile entry
    ///    ([`LockEntry`](crate::lock::LockEntry)), and `check` (C-14) verifies each of them
    ///    directly. Folding them in here would hash the same facts twice and would make a
    ///    comment-only edit to a provider TOML — which changes `toml_sha256` and not one generated
    ///    byte — move the IR hash as well. Each recorded hash covers exactly one input; that is
    ///    what lets `check` name *which* input moved.
    ///
    /// A [`Connector`] field added later is a compile error inside this function until someone
    /// decides which side of that line it falls on.
    pub fn hash_domain(&self) -> crate::Result<String> {
        Ok(serde_json::to_string(&HashDomain::of(self))?)
    }

    /// Lowercase-hex SHA-256 of [`hash_domain`](Self::hash_domain) — the `ir_sha256` of a
    /// [`LockEntry`](crate::lock::LockEntry).
    pub fn ir_sha256(&self) -> crate::Result<String> {
        Ok(crate::lock::sha256_hex(self.hash_domain()?.as_bytes()))
    }
}

/// The serialized shape of the hash domain: an explicit projection of [`Connector`], not the
/// connector itself. See [`Connector::hash_domain`] for what it includes and why.
///
/// Borrowed rather than cloned so that hashing is allocation-cheap, and with no
/// `skip_serializing_if` of its own — the fields it names are always present, and the types it
/// borrows already encode canonically.
/// The three service fields carry `skip_serializing_if`, which is the one exception to the rule
/// above, and it exists for the same reason [`Provenance`] is excluded altogether: a connector that
/// declares no service, no authority and no version must hash to what it hashed *before* services
/// existed. Otherwise landing C-49 moves every `ir_sha256` in the repository and every
/// `connectors.lock` entry churns for a provider nobody edited — phantom drift, which is precisely
/// what the lockfile exists to rule out. [`Operation::service`] holds the same property through its
/// own `skip_serializing_if`.
#[derive(Serialize)]
struct HashDomain<'a> {
    id: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    authority: &'a Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    api_version: &'a Option<String>,
    #[serde(skip_serializing_if = "<[Service]>::is_empty")]
    services: &'a [Service],
    vendor: &'a str,
    base_url: &'a str,
    description: &'a str,
    auth: &'a [AuthMethod],
    default_auth: &'a [AuthRequirement],
    operations: &'a [Operation],
    /// Inbound members carry `skip_serializing_if` for the same reason the service fields do: a
    /// connector that declares none must hash to what it hashed *before* they existed, or landing
    /// them moves every `ir_sha256` in the repository and churns `connectors.lock` for 14 providers
    /// nobody edited.
    #[serde(skip_serializing_if = "<[EventDecl]>::is_empty")]
    events: &'a [EventDecl],
    #[serde(skip_serializing_if = "<[ChannelBinding]>::is_empty")]
    channels: &'a [ChannelBinding],
    /// Discovery declarations affect which target Provider candidate and route adapter a response
    /// may produce, so changing one is compiled meaning and must move the IR hash.
    #[serde(skip_serializing_if = "<[Discovery]>::is_empty")]
    discoveries: &'a [Discovery],
    /// In the domain, and the reasoning is worth stating because it is not obvious: a configuration
    /// field is presentation, and presentation usually is not compiled meaning. But a field's `binds`
    /// decides *where a value goes* — a changed binding sends a token to a different place — and its
    /// `secret` flag decides whether a value is masked. Both are behaviour. Splitting the struct so
    /// that only those two hashed would make `connectors check` silent about a label that had
    /// drifted from the vendor's own name for the field, which is a change a user sees.
    #[serde(skip_serializing_if = "<[ConfigField]>::is_empty")]
    config: &'a [ConfigField],
    #[serde(skip_serializing_if = "Option::is_none")]
    verify: &'a Option<String>,
    /// In the domain without argument: a graph *is* compiled meaning — it becomes an emitted `op`,
    /// so a changed edge is a changed module.
    #[serde(skip_serializing_if = "<[Graph]>::is_empty")]
    graphs: &'a [Graph],
}

impl<'a> HashDomain<'a> {
    fn of(connector: &'a Connector) -> Self {
        // The exhaustive destructuring is the tripwire, and it is the whole reason this is a
        // separate type instead of a `serde` attribute: a field added to `Connector` fails to
        // compile here until someone states whether it belongs in the hash domain. Silently
        // inheriting the decision is how `fetched_at` got into the hash in the first place.
        let Connector {
            id,
            authority,
            api_version,
            services,
            vendor,
            base_url,
            description,
            auth,
            default_auth,
            operations,
            events,
            channels,
            discoveries,
            config,
            verify,
            graphs,
            provenance,
        } = connector;

        // Deliberately excluded — see `Connector::hash_domain`.
        let _excluded = provenance;

        Self {
            id,
            authority,
            api_version,
            services,
            vendor,
            base_url,
            description,
            auth,
            default_auth,
            operations,
            events,
            channels,
            discoveries,
            config,
            verify,
            graphs,
        }
    }
}
