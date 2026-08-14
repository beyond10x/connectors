//! The connector catalogue as typed `&'static` data, served from the embedded catalog pack.
//!
//! One deterministic JSON document per provider (`catalog/<id>.catalog.json`) is compiled into one
//! versioned, digest-carrying pack that [`reader`] embeds. This crate is the **shim** over that
//! pack: it parses the documents once, on first use, and hands out the typed, `&'static` view a
//! consumer decides with — risk, idempotency, credential placement, endpoint configuration,
//! channel bindings.
//!
//! ```
//! use catalog::{OperationKey, ProviderKey, Risk};
//!
//! let show = catalog::operation(OperationKey::id("zendesk-ticket-show")).unwrap();
//! assert_eq!(show.risk, Risk::Low);
//! assert_eq!(show.provider, "zendesk");
//!
//! // "every operation in this provider" is one call.
//! let zendesk = catalog::operations_of(ProviderKey::id("zendesk"));
//! assert!(zendesk.iter().all(|operation| operation.provider == "zendesk"));
//! ```
//!
//! # What this crate is *not* any more
//!
//! The predecessor stored this surface as **generated Rust**: 16,909 lines of `&'static` tables
//! under `src/generated/`, plus 835 `ops/**/*.flux` renderings embedded with `include_str!`. The
//! tables existed for one field the documents deliberately do not carry — `Operation::flux`, the
//! emitted Flux text — and that field is gone with the emitter (design 02 §2). Everything else the
//! tables held is in the canonical documents already, so the storage is now the pack and the
//! tables are not regenerated, not committed and not reviewed.
//!
//! The observable consequences, stated rather than left to be discovered:
//!
//! - **`Operation::flux` no longer exists.** Nothing in this workspace emits Flux; a request is
//!   derived from the document's request template by `connector-resolve`.
//! - **The table is built on first use**, behind a `OnceLock`, and lives for the process. It is
//!   `&'static` for the same reason it always was — a resolved operation is held by every
//!   projection of it — but the cost moved from compile time to one lazy parse of the pack.
//! - **`OAuth2` carries no `client_id`.** The document has no field for a registration value by
//!   design, and the predecessor's table could only ever emit an empty string there.
//!
//! It stays what it always was in every other respect: data plus lookups over it, with nothing to
//! parse at query time and no network, filesystem or engine anywhere in its dependency graph.

/// The catalog pack and its dependency-free reader, re-exported whole (C-537).
///
/// `catalog::reader::operation("zendesk-ticket-show")` serves the operation's canonical JSON
/// record; `catalog::reader::Pack::load(path)` serves a newer pack than this crate was built
/// with, refusing a wrong schema version or digest before serving any record. See
/// `catalog-reader` for the format.
pub use catalog_reader as reader;

mod table;

/// How much damage an operation can do, in flux's own vocabulary.
///
/// A mirror of `connector_spec::Risk`, not a re-export: this crate resolves no machinery — its
/// storage is the pack behind [`reader`] — which is what makes it cheap to add. The document
/// spells the level as one of the four words below and [`table`] matches them exhaustively, so a
/// level the document grows is a refusal naming it rather than a silent misclassification.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
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

impl Risk {
    /// The stable published spelling — the word the canonical document carries.
    pub const fn as_str(self) -> &'static str {
        match self {
            Risk::Low => "low",
            Risk::Medium => "medium",
            Risk::High => "high",
            Risk::Destructive => "destructive",
        }
    }
}

/// Whether repeating an operation is safe, in flux's own vocabulary.
///
/// Mirrors `connector_spec::Idempotency` for the same reason [`Risk`] does.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Idempotency {
    /// Repeating the call has the same effect as making it once.
    Idempotent,
    /// Repeating the call repeats its effect.
    NonIdempotent,
    /// Idempotent only under a condition the caller supplies (e.g. an idempotency key).
    Conditional,
}

impl Idempotency {
    /// The stable published spelling — the word the canonical document carries.
    pub const fn as_str(self) -> &'static str {
        match self {
            Idempotency::Idempotent => "idempotent",
            Idempotency::NonIdempotent => "non_idempotent",
            Idempotency::Conditional => "conditional",
        }
    }
}

/// **Why an operation names the credentials it does** — and, when it names none, which of the two
/// opposite reasons that is (C-235).
///
/// [`Operation::credentials`] is an empty slice in two situations that mean the reverse of each
/// other, and a consumer reading only that slice gets one value for both. This field is what a
/// connector *declares*, carried across rather than left to be inferred from an absence — the trap
/// C-206 closed in the published catalogue and this closes in the embedded one.
///
/// # The vocabulary is C-206's
///
/// The two named states are the two codes `web/public/catalog.json` already publishes for the same
/// distinction (`predecessor:docs/designs/catalog-json.md`), and [`as_str`](Self::as_str) returns those tokens
/// character for character. A host restating them in words of its own is how two surfaces
/// describing one fact come to disagree, so this crate restates nothing:
/// `connectors_api::api::Wiring` serializes these tokens directly.
///
/// [`Declared`](Self::Declared) is the ordinary case and has no published *status* code, because
/// there is nothing to report about it — the operation names its mechanisms and the slice beside
/// this field is them. It is given a token anyway so that every state has one name rather than two
/// having names and one being "the other one".
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CredentialRequirement {
    /// The operation authenticates: [`Operation::credentials`] names at least one mechanism.
    Declared,
    /// **The vendor requires no credential**, declared positively by the connector (`auth = []` on
    /// the operation). Nothing is withheld and nothing is missing: the unauthenticated call is the
    /// correct working call.
    NoneRequired,
    /// **A credential is withheld.** Nothing is declared and nothing declares that nothing is
    /// needed, so the request would go out unauthenticated — the fail-closed outcome, not a working
    /// one. `freshdesk` is the shipped case: its API key occupies the Basic *username* position,
    /// which this repository's model treats as non-secret config, so declaring it would route a
    /// live key outside the secret gate (`AGENTS.md`, Intentional gaps).
    ///
    /// The operator-facing consequence is the part worth carrying: there is nothing for an operator
    /// to supply *and* the operation does not work, which is neither of the other two states.
    Withheld,
}

impl CredentialRequirement {
    /// The token this state travels as — C-206's published spellings for the two it named.
    ///
    /// `no-credential-required` is `connector_cli::status::NO_CREDENTIAL_REQUIRED` and
    /// `no-credential` is `connector_cli::status::NO_CREDENTIAL`, both of which are published
    /// contract tokens that are extended but never renamed once shipped.
    pub const fn as_str(self) -> &'static str {
        match self {
            CredentialRequirement::Declared => "declared",
            CredentialRequirement::NoneRequired => "no-credential-required",
            CredentialRequirement::Withheld => "no-credential",
        }
    }
}

/// The connector-authored vendor-state direction of an operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OperationDirection {
    /// The operation observes vendor state without changing it.
    Read,
    /// The operation changes vendor state.
    Write,
}

impl OperationDirection {
    /// The provider/public spelling shared by every artifact.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Read => "read",
            Self::Write => "write",
        }
    }
}

/// Declared host-resource consequence; distinct from semantic-effect tags.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HostEffect {
    Read,
    Write,
    Network,
    Process,
    Browser,
    Filesystem,
    LocalSystem,
}

/// Connector member lifecycle.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InteractionShape {
    Unary,
    Stream,
    Subscription,
    LeasedSession,
    SessionEstablishment,
}

/// Closed, versioned protocol implementation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProtocolDriver {
    HttpV1,
    SipV1,
}

/// Placement requirement before deployment selection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlacementRequirement {
    ConnectorsDeployment,
    SubstrateWorkload,
    FederatedSatellite,
}

/// How the implementation is supplied.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImplementationForm {
    BuiltIn,
}

/// Capability admission must prove before dispatch.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RequiredCapability {
    PublicNetwork,
    PrivateNetwork,
    UnixSocket,
    FileSecret,
    Process,
    Container,
    Device,
}

/// One operation: its Flux source, and what a caller needs in order to decide whether to use it.
///
/// `#[non_exhaustive]` so that the global address and the resolved endpoint spec can land
/// additively: a consumer outside this crate can neither construct one with a struct literal nor
/// destructure one exhaustively, so a field arriving is additive for them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct Operation {
    /// The Flux symbol the operation is declared and called by, e.g. `zendesk-ticket-show`. Unique
    /// across the whole catalog, and the key [`OperationKey::id`] matches on.
    pub id: &'static str,
    /// The [`Provider::id`] this operation belongs to.
    pub provider: &'static str,
    /// The **service** this operation belongs to — `delivery`, `s3`, or the reserved `default` for
    /// a provider with a single API surface. Never empty: the IR always names a service
    /// (`connector_spec::ir::DEFAULT_SERVICE`), and the reserved name is elided from *addresses*
    /// only, never from this field.
    ///
    /// # Why it is here, and what was wrong without it (C-197)
    ///
    /// Services partition the operation set, and a service owns its own `base_url` — so
    /// [`Operation::hosts`] is already per service. What was missing was the *name*, and a consumer
    /// that cannot read it cannot key anything by service.
    ///
    /// `connector-pack`'s configuration port is where that bit. It keyed a tenant's endpoint values
    /// on `(tenant, provider, kind, name)`, because there was no service here to key on, so two
    /// services of one connector that spell the same `{variable}` in their own `base_url` collapsed
    /// to a single value. `contentful` is the shipped case: `delivery_space_id` and
    /// `management_space_id` are two declared configuration fields, both binding `endpoint.space_id`
    /// under different services, and `contentful-entry-create` on `api.contentful.com` resolved
    /// whatever `contentful-entry-get` on `cdn.contentful.com` had. A tenant whose two environments
    /// differ got a `200` from the wrong one rather than a refusal.
    ///
    /// The published catalogue carried the service all along; the embedded one caught up in the
    /// predecessor's C-197.
    pub service: &'static str,
    /// Whether this operation reads or writes vendor state. Authored in the connector declaration;
    /// never inferred from its HTTP method or any policy metadata.
    pub direction: OperationDirection,
    /// What the operation does, in one line — the same text the model sees as the tool description.
    pub description: &'static str,
    /// How much damage it can do.
    pub risk: Risk,
    /// Whether repeating it is safe.
    pub idempotency: Idempotency,
    /// Host-resource consequences read from the document, never derived.
    pub effects: &'static [HostEffect],
    /// Semantic-effect tags (`money`, `delete`, `send_external`, …) as the connector declares
    /// them. Empty means no semantic consequence is declared.
    pub semantic_effects: &'static [&'static str],
    /// Lifecycle, independently of protocol and placement.
    pub interaction_shape: InteractionShape,
    /// Closed, versioned protocol implementation.
    pub protocol_driver: ProtocolDriver,
    /// Placement requirement, not a selected worker.
    pub placement_requirement: PlacementRequirement,
    /// How the implementation is supplied.
    pub implementation_form: ImplementationForm,
    /// Capabilities required before dispatch.
    pub required_capabilities: &'static [RequiredCapability],
    /// The credentials the operation needs, as **alternatives of mechanisms**: the outer slice is
    /// an OR over ways to authenticate, and each inner slice is the set of credentials that must
    /// all be satisfied on the same request (AND).
    ///
    /// `&[&["babelforce.access_id", "babelforce.access_token"]]` is one mechanism needing two
    /// headers together, not two ways to authenticate — the distinction babelforce forces and that
    /// a flat list of credential names cannot express. An empty outer slice means the operation
    /// needs no credential at all.
    ///
    /// The names are credential *references*. No secret, and no environment variable's value, is
    /// ever in this crate — a host resolves the reference and applies the scheme.
    ///
    /// **An empty outer slice does not say why**, which is what [`credential_requirement`] is for.
    ///
    /// [`credential_requirement`]: Self::credential_requirement
    pub credentials: &'static [&'static [&'static str]],
    /// **Why [`credentials`](Self::credentials) is what it is** — and, when it is empty, which of
    /// the two opposite reasons that is (C-235). See [`CredentialRequirement`].
    ///
    /// It is a separate field rather than a richer `credentials` because the type of that field is
    /// this crate's published API: a consumer already matching on the mechanism list keeps working,
    /// and one that needs the reason reads a field that was not there before. `Operation` is
    /// `#[non_exhaustive]` precisely so that is additive.
    ///
    /// The invariant, held at emission and pinned by
    /// `tests/embedded_operations.rs::the_declared_requirement_agrees_with_the_mechanism_list`:
    /// this is [`Declared`](CredentialRequirement::Declared) exactly when `credentials` is
    /// non-empty.
    pub credential_requirement: CredentialRequirement,
    /// The hosts a call reaches, as the connector's base URL spells them — templating included, so
    /// Zendesk is `{subdomain}.zendesk.com` rather than a tenant nobody has chosen yet. What a
    /// caller does with this is decide whether their egress policy admits the call.
    pub hosts: &'static [&'static str],
    /// **The description a model receives** (S-001) — the document's stored contract projection,
    /// error-envelope-extended where the vendor declares an envelope. Not
    /// [`description`](Self::description), the one-line summary.
    pub contract_description: &'static str,
    /// **The lowered, caller-typed input schema a model receives** (S-001), as canonical JSON
    /// text: one object keyed by caller-facing symbols, every declared parameter required. Read
    /// from the document's stored contract, never derived — together with
    /// [`contract_description`](Self::contract_description), [`id`](Self::id) and
    /// [`expose`](Self::expose) this is the complete caller-facing contract, built from document
    /// data alone.
    pub input_schema: &'static str,
    /// Whether the operation is published to callers. An unexposed operation exists (its document
    /// entry is the audit trail) but is not offered.
    pub expose: bool,
}

/// **Where a credential goes on the way out** — the *placement* axis of
/// `predecessor:docs/designs/unified-auth.md`.
///
/// The design's central claim is carried by one field: `prefix` on [`Header`](Self::Header) is what
/// turns `Bearer` from an enum variant into data, so `Bearer `, `Basic `, `Token `, `GenieKey ` and
/// the empty prefix are one code path rather than five variants. A flat scheme enum conflates
/// *where* a secret goes with *what it is prefixed by*, and needs a new variant the moment a vendor
/// wants `Token <t>` in `Authorization`.
///
/// Deliberately **not** `#[non_exhaustive]`, unlike [`Provider`] and [`Operation`]. A consumer
/// placing a credential must match every variant, and a new placement must be a compile error at
/// every such site rather than a silently skipped credential — a request that quietly went out
/// without one is exactly the failure this type exists to prevent.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Placement {
    /// `<name>: <prefix><value>`. `Authorization: Bearer <t>` is
    /// `Header { name: "Authorization", prefix: "Bearer " }`; a raw-value header such as
    /// `X-Shopify-Access-Token` is the same variant with an empty prefix.
    Header {
        /// The header name.
        name: &'static str,
        /// The literal text the value is prefixed with, **trailing space included**. Empty for a
        /// raw-value header.
        prefix: &'static str,
    },
    /// `?<name>=<value>` — a query-parameter key.
    Query {
        /// The parameter name.
        name: &'static str,
    },
    /// **Never placed on an outgoing request.** A webhook signing secret verifies bytes that
    /// arrived, so it has no answer to "where does this go on the way out" — the one deliberate
    /// divergence from flux's vocabulary, recorded in `AGENTS.md`'s authentication contract. It is
    /// here so that one credential namespace covers both directions.
    Inbound,
}

/// **How stored material becomes the value that is placed** — the *acquisition* axis.
///
/// Two of the three are pure: the stored secret is placed unchanged, or it is joined into a Basic
/// pair. The third, [`Minted`](Self::Minted), is the one effectful acquisition this repository
/// models, and it is modelled as *provenance* rather than as a step a connector performs — see its
/// own documentation. The remaining effectful ones — token refresh, expiry, `session` — are still
/// the host's, by the line `predecessor:docs/designs/unified-auth.md` draws: they need a cache and a
/// refresh-on-401, and C-90 put both out of scope.
///
/// Not `#[non_exhaustive]`, for the reason [`Placement`] is not.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Acquisition {
    /// The stored secret, unchanged.
    Static,
    /// The stored secret, unchanged, entered by a human through a single-use Connector Connect
    /// Session completion endpoint. No client or runtime may resolve it from ambient process state.
    ConnectSession,
    /// **The stored secret, unchanged — and one of this connector's own operations put it there**
    /// (C-136).
    ///
    /// Read as a *placement* instruction this is [`Static`](Self::Static): a minted credential goes
    /// onto a request exactly as a statically provisioned one does, because by the time anything
    /// places it, it is a value in the store like any other. What the variant adds is the fact the
    /// diversion needs, and which nothing else in the catalogue carries: **which call mints it, and
    /// where in that call's answer the value arrives**.
    ///
    /// `connector-pack` reads it from the minting side. Projecting an operation, it looks for the
    /// declared credential whose [`by`](Self::Minted::by) is that operation's own id; finding one
    /// makes the operation credential-producing, and its result stops being the vendor's body and
    /// becomes the handle `{ "credential": "tenants/…" }`. A downstream operation that *uses* the
    /// credential names it in `credentials` like any other and never sees a value — which is the
    /// property C-136 exists for: **a caller can use a credential it can never read.**
    ///
    /// # Why the fact lives on the credential rather than on the operation
    ///
    /// It reads as though it belongs on [`Operation`], and semantically it is the operation's
    /// declaration — `produces_credential` in the provider file sits in the `[[operations]]` block.
    /// It is carried here because it is also, exactly, an answer on this axis, and because a field
    /// on `Operation` is a field every operation of every provider must then state, for a fact no
    /// shipped connector declares. A variant costs nothing until something uses it, which is the
    /// same reason `skip_serializing_if` is on the IR field.
    ///
    /// **Reachable from the canonical document since S-001.** The document carries the minting
    /// join (`produces_credential` on the minting operation: which credential the value is stored
    /// as, and where in the response the secret arrives), and [`table`] constructs this variant
    /// from it. No shipped connector declares one yet —
    /// `tests/main/pack_table.rs` pins that — so the shipped catalogue still never constructs it;
    /// the fixture-backed unit test in [`table`] is what proves the path.
    Minted {
        /// The [`Operation::id`] whose call mints this credential.
        by: &'static str,
        /// Where the secret arrives in that operation's response body — **one** JSON Pointer into
        /// the response value, `/access_token`.
        ///
        /// Exactly one location, and the loader refuses anything else. `credential_response`'s
        /// sibling vocabulary admits `*` for every element of an array because it describes where
        /// credentials *appear*; a mint stores one value at one address, so there would be nothing
        /// to say which element it was. That is also what keeps this field's documented behaviour
        /// and the runtime's the same thing: the diversion resolves it with
        /// `serde_json::Value::pointer`, which has no wildcard.
        from: &'static str,
    },
    /// `base64(<user><user_suffix>:<secret>)` — HTTP Basic, composed rather than pre-composed.
    ///
    /// The user half is **config, not a gated secret**: it is an email address or an account name,
    /// and it resolves from the declared environment variables rather than from the secret store.
    /// `user_suffix` is Zendesk's `/token` marker — public API syntax, and the reason a bare env
    /// value is not enough (`predecessor:docs/designs/auth-seam.md` §7.5).
    ///
    /// The secret occupies the **password** position. Freshdesk's shape, where the API key occupies
    /// the *username* position, is not expressible here because the IR cannot yet say so either
    /// (C-16 owns that decision); Freshdesk therefore declares no credential at all, which
    /// `AGENTS.md` records as an intentional gap rather than an oversight.
    BasicJoin {
        /// Environment-variable **keys** holding the user half, tried in order. Never a value.
        user_env: &'static [&'static str],
        /// A literal appended to the resolved user half before the `user:secret` join.
        user_suffix: &'static str,
    },
    /// **The host runs an OAuth2 grant and places the resulting access token** (C-525).
    ///
    /// Read as a placement instruction this is [`Static`](Self::Static) too — a freshly granted
    /// access token goes onto the request exactly as a pasted one does. What the variant adds is
    /// the only thing a host cannot derive from anything else in the catalogue: **which endpoints
    /// the grant runs against, which grants this credential allows, and what it asks for.**
    ///
    /// It is on this axis rather than on [`Credential`] because that is what this axis *is* — how
    /// stored material becomes the value that is placed — and because [`Minted`](Self::Minted)
    /// settled the same question the same way: a variant costs nothing until something uses it,
    /// whereas a field on `Credential` would be one every connector must state for a fact most of
    /// them never declare.
    ///
    /// **Nothing in this repository performs the grant.** An authorize endpoint is a browser
    /// redirect and a token endpoint's response body *is* a credential, so neither is a connector
    /// operation. This variant is the declaration a host reads before doing the grant itself.
    OAuth2(&'static OAuth2),
}

/// How a host obtains an OAuth2-backed credential. Mirrors `connector_spec::OAuth2Spec`.
///
/// **Carries no registration value and has no field one could occupy.** The client id *and* the
/// client secret are per-deployment registration values: they are `[[config]]` bindings a host
/// resolves from its own store, and the canonical document closes its OAuth2 object against them
/// (`additionalProperties: false`), so a future document cannot carry one for a consumer to
/// mistakenly trust.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct OAuth2 {
    /// The declared endpoint name whose base URL [`authorize_path`](Self::authorize_path) and
    /// [`token_path`](Self::token_path) resolve against. Empty means the connector's own base URL.
    ///
    /// It is a name rather than a URL because the endpoint's host allow-list is what admits the
    /// token exchange through the egress gate; a bare URL would name a host nothing had admitted.
    pub endpoint: &'static str,
    /// The declared service name whose base URL [`token_path`](Self::token_path) resolves against
    /// when the token endpoint lives on a **different host** from the authorize endpoint (C-556).
    /// Empty — every shipped declaration's value — means the token exchange resolves against
    /// [`endpoint`](Self::endpoint), which is a connector's single-host behaviour unchanged.
    ///
    /// A host joins [`token_path`](Self::token_path) onto this service's base URL when it is set, and
    /// onto [`endpoint`](Self::endpoint)'s otherwise. It is a name and never a URL, for the same
    /// reason [`endpoint`](Self::endpoint) is: the host set is derived from declared services.
    pub token_endpoint: &'static str,
    /// The authorize endpoint path, joined onto the endpoint base URL.
    pub authorize_path: &'static str,
    /// The token endpoint path. Every grant and every refresh POSTs here. It resolves against
    /// [`token_endpoint`](Self::token_endpoint)'s base URL when that is set, and
    /// [`endpoint`](Self::endpoint)'s otherwise.
    pub token_path: &'static str,
    /// The scopes the grant requests.
    pub scopes: &'static [&'static str],
    /// The grants a host may run for this credential. A grant absent from this list is one the
    /// connector does not allow, not one the host may try anyway.
    pub grants: &'static [OAuthGrant],
    /// The loopback redirect an [`OAuthGrant::AuthorizationCode`] login binds.
    pub redirect: Option<OAuthRedirect>,
    /// Whether this is a public PKCE client that issues and uses no client secret (C-556). `false`
    /// — every shipped declaration's value — is a confidential client, which registers a client
    /// secret an operator supplies. A public client needs a client id but no secret, so a host
    /// must not require one.
    pub public_client: bool,
}

/// One token grant an [`OAuth2`] credential allows. Mirrors `connector_spec::OAuthGrant`.
///
/// Closed, for the reason [`Placement`] is: a grant this did not recognise would have to become
/// *some* grant, and every wrong answer either sends a credential to an endpoint that does not
/// expect it or runs a flow the connector never allowed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum OAuthGrant {
    /// Browser redirect plus loopback callback, with PKCE.
    AuthorizationCode,
    /// The resource-owner password grant. RFC 6749 §4.3.2 makes discarding the password a MUST for
    /// the client, and a host running this one is that client.
    Password,
    /// Exchange a stored refresh token for a fresh access token.
    RefreshToken,
    /// The two-legged client-credentials grant, with no user.
    ClientCredentials,
}

/// The loopback redirect an `authorization_code` login binds. Mirrors
/// `connector_spec::OAuthRedirect`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct OAuthRedirect {
    /// The loopback port to bind.
    pub port: u16,
    /// The callback path the browser is redirected to.
    pub path: &'static str,
}

/// **Whose authority a credential carries when it is used** (C-528). Mirrors
/// `connector_spec::Subject`.
///
/// The "on behalf of" axis, independent of every other one: [`Placement`] says where the value goes,
/// [`Acquisition`] says how it is obtained, and this says *who the vendor thinks is acting* once it
/// arrives. Slack is the case that forces it — one OAuth v2 grant returns a workspace bot token and
/// a signed-in user's token in one response, placed identically and acquired identically, differing
/// only in who they can act as.
///
/// A host reads this to bound a credential's reach and to decide where it may be stored: an
/// app-subject credential is provisioned once for a tenant, a user-subject one once per person, and
/// keeping a user token at a tenant-wide address would let one member act as another.
///
/// # `Unstated` is a real answer and must be handled
///
/// It means *"nobody has reviewed this credential for its subject"* — not "app". Every connector
/// shipped before C-528 carries it, including genuinely ambiguous ones: GitHub's single
/// `github.token` covers both an App installation token and a personal access token, which are
/// opposite answers. **A consumer that needs the distinction refuses on `Unstated`** rather than
/// assuming; assuming `App` over-grants, and assuming `User` silently fails.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum Subject {
    /// Not reviewed. Carries no claim in either direction — refuse rather than choose one.
    #[default]
    Unstated,
    /// The integration itself: Slack's `xoxb-` bot token, a GitHub App installation token.
    App,
    /// The person who granted it — "on behalf of": Slack's `xoxp-` user token, an Atlassian 3LO
    /// token, a personal access token.
    User,
}

/// **A named weakness in how a credential is obtained** (C-440). Mirrors
/// `connector_spec::AuthHazard`.
///
/// The axis a host refuses on, and it is deliberately a *declared property* rather than a connector
/// name: an operator who wants no password-grant authentication anywhere says that once, and the
/// fifty-sixth connector — added next month by somebody who never read their policy — is covered by
/// the same sentence. A list of names is correct on the day it is written and silently wrong
/// afterwards.
///
/// # This is a *kind*. [`Risk`] is a *level*
///
/// Not interchangeable, and the difference is load-bearing because [`Risk`] is **ordered** and a
/// selector compares against that ordering. A hazard has no position on that ladder: a password
/// grant that buys a **read-only** token is [`Risk::Low`] *and* hazardous. A fifth rung would be
/// wrong in one direction or the other — high enough to catch it and every destructive operation
/// inherits a weakness it does not have; low enough not to and `at_most(Risk::High)` silently
/// admits password-grant authentication to every rule an operator has already written.
///
/// # The absence is not a clean bill of health
///
/// `None` on a [`Credential`] means *no weakness is declared*, which is the state every connector
/// but babelforce is in and which nobody has reviewed. It does not mean *reviewed and found safe*.
/// A consumer's fail-closed default is written against the presence of a value.
///
/// Not `#[non_exhaustive]`, for the reason [`Placement`] is not: a consumer matching exhaustively
/// *should* be told when the set grows, because a new hazard is a new weakness they have not decided
/// about — and defaulting it into a catch-all arm is how a deployment admits one it never saw.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AuthHazard {
    /// The resource owner's own password is presented to **the host** rather than to the
    /// authorization server — the OAuth 2.0 resource owner password credentials grant.
    ///
    /// **RFC 9700 §2.4** says the grant MUST NOT be used: it exposes the resource owner's
    /// credentials to the client, widens where they can leak beyond the authorization server, and
    /// cannot carry two-factor authentication. **RFC 6749 §4.3** makes discarding them once a token
    /// is obtained a MUST for the client, and a host running the grant is that client. **CWE-522**
    /// is the nearest weakness-catalogue entry. OAuth 2.1 drops the grant entirely.
    ResourceOwnerSecretShared,
}

impl AuthHazard {
    /// The stable published spelling a host's deployment filter matches as an exact word.
    ///
    /// Matched exhaustively rather than derived, for the reason [`Subject`] is: the string is
    /// configuration an operator audits in another repository, and it must not move because a
    /// derive attribute did.
    pub const fn word(self) -> &'static str {
        match self {
            Self::ResourceOwnerSecretShared => "resource_owner_secret_shared",
        }
    }
}

/// One credential a connector declares: what it is called, where its value is kept, and how it
/// reaches the wire.
///
/// **No value, ever**, and no field it could occupy: [`Credential::leaf`] is the last segment of a
/// *path*, [`Acquisition`] names environment-variable *keys*, and [`Placement`] names a header, a
/// prefix or a query key. This crate is `&'static` data compiled from the IR, so "it holds no secret"
/// is a property of the shape rather than a habit of whoever last edited the emitter.
///
/// Not `#[non_exhaustive]`: a consumer constructs one in a test to prove its own placement code, and
/// the type is small, declarative and complete.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Credential {
    /// The flat-namespace name an operation references, e.g. `zendesk.api_token`. Vendor-prefixed,
    /// because credentials share one namespace across the connector.
    pub name: &'static str,
    /// The last segment of the credential's address — `api_token`, not `zendesk.api_token`. The
    /// path already carries the authority, so the vendor prefix would be said twice.
    pub leaf: &'static str,
    /// How stored material becomes the value that is placed.
    pub acquire: Acquisition,
    /// Where that value goes on the request.
    pub place: Placement,
    /// Whose authority the value carries once it arrives — see [`Subject`].
    ///
    /// Unlike the three fields above, this one has a meaningful *unreviewed* state, and every
    /// connector shipped before C-528 is in it. Read [`Subject::Unstated`] before branching on this.
    pub subject: Subject,
    /// The declared weakness in **obtaining** this credential, when it has one — see [`AuthHazard`].
    ///
    /// `None` means no weakness is declared, not that one was looked for and not found. A host
    /// refuses a hazardous acquisition unless its deployment explicitly opted into that hazard,
    /// which is what makes the refusal a property rather than a list of connector names.
    pub hazard: Option<AuthHazard>,
}

/// One string pair in a declaration (query, header or payload mapping).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Pair {
    pub name: &'static str,
    pub value: &'static str,
}

/// How an event reaches a connector binding.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChannelTransport {
    Webhook,
    Socket,
    Poll,
    Session,
}

/// One event declaration, including the local/wire identity split.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Event {
    pub name: &'static str,
    pub service: &'static str,
    pub description: &'static str,
    pub wire_value: Option<&'static str>,
    pub default: bool,
    pub group: &'static str,
    /// Canonical JSON Schema text, or `None` when the vendor publishes none.
    pub schema: Option<&'static str>,
    /// Canonical JSON for the complete source declaration, including narrowing rules.
    pub declaration_json: &'static str,
}

/// The generic RFC 6455 handshake facts for one socket binding.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SocketConnect {
    pub path: &'static str,
    pub query: &'static [Pair],
    pub headers: &'static [Pair],
    pub auth: &'static [&'static [&'static str]],
    pub subprotocols: &'static [&'static str],
}

/// One selector into an inbound envelope.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Selector {
    pub source: &'static str,
    pub name: &'static str,
}

/// Driver and capability facts for an inbound direct-byte session.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChannelSession {
    pub interaction_shape: InteractionShape,
    pub protocol_driver: ProtocolDriver,
    pub placement_requirement: PlacementRequirement,
    pub implementation_form: ImplementationForm,
    pub required_capabilities: &'static [RequiredCapability],
}

/// One generated channel binding. Verification/reply/setup remain available in the manifest and
/// public catalogue; this embedded shape carries every fact the zero-I/O socket planner and generic
/// event router require.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Channel {
    pub name: &'static str,
    pub service: &'static str,
    /// The service base URL against which [`SocketConnect::path`] is derived.
    pub base_url: &'static str,
    pub description: &'static str,
    pub transport: ChannelTransport,
    pub session: Option<ChannelSession>,
    pub events: &'static [&'static str],
    pub connect: Option<SocketConnect>,
    /// Credential-name alternatives required to establish or supervise the channel. Values stay in
    /// Connector custody and never enter this catalog projection.
    pub auth: &'static [&'static [&'static str]],
    pub discriminator: Option<Selector>,
    /// Stable vendor delivery identity used for deduplication, when declared.
    pub delivery_id: Option<Selector>,
    pub payload: &'static [Pair],
    pub payload_root: bool,
    /// Canonical JSON for the complete binding declaration, including verification and reply.
    pub declaration_json: &'static str,
}

/// One complete configuration field, with no stored value.
///
/// Activation policy is carried as [`Approval`] rather than left inside [`Self::declaration_json`]:
/// a host enforcing whether a configured value may become active must match a closed vocabulary,
/// not search serialized source text for a token.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ConfigField {
    pub name: &'static str,
    pub service: &'static str,
    pub label: &'static str,
    pub help: &'static str,
    pub example: Option<&'static str>,
    pub format: &'static str,
    pub required: bool,
    pub default: Option<&'static str>,
    /// Extra authorization required before the configured value becomes active.
    pub approval: Approval,
    pub secret: bool,
    pub docs_url: Option<&'static str>,
    pub binds: &'static str,
    pub also_binds: &'static [&'static str],
    /// The further services whose base URL this one value also fills (C-529).
    ///
    /// Empty for the ordinary field. When it is not, [`service`](Self::service) remains the address
    /// the value is stored under and the services named here resolve their `{variable}` from that
    /// same address — one question, one value, one [`approval`](Self::approval). A host composing a
    /// URL for a sibling service must consult this or it will report an unbound placeholder for a
    /// value the operator has already supplied.
    pub also_services: &'static [&'static str],
    /// Canonical JSON for the complete form declaration, including choices.
    pub declaration_json: &'static str,
}

/// Extra authorization required before a supplied configuration value becomes active.
///
/// Mirrors `connector_spec::Approval`. Deliberately closed: a new activation policy must produce a
/// compile error in every consumer that decides whether a value may influence a request.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum Approval {
    /// The ordinary configuration flow may activate the value.
    #[default]
    None,
    /// A deployment operator must approve and pin a non-default value for this connection.
    Operator,
}

impl Approval {
    /// The stable token used by the connector declaration and public catalogue.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Operator => "operator",
        }
    }
}

/// Who is likely to find a service useful, for catalogue discovery only.
///
/// This is never an authorization, visibility, entitlement or credential-policy input.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Audience {
    Developer,
    Sre,
    SecurityEngineer,
    DataAnalyst,
    ProductManager,
    ProjectManager,
    Designer,
    SalesRep,
    SupportAgent,
    Marketer,
    Finance,
    EcommerceManager,
    ContentManager,
}

impl Audience {
    /// The stable public token used by source declarations and explorer filters.
    pub const fn as_str(self) -> &'static str {
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
}

/// Closed parser for one Provider-declared discovery observation source.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DiscoveryDriver {
    /// Grafana's bounded data-source listing response.
    GrafanaDatasourceV1,
}

impl DiscoveryDriver {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::GrafanaDatasourceV1 => "grafana_datasource_v1",
        }
    }
}

/// Closed implementation identity for a mediated Connection route.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RouteAdapter {
    GrafanaDatasourceProxyV1,
}

impl RouteAdapter {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::GrafanaDatasourceProxyV1 => "grafana_datasource_proxy_v1",
        }
    }
}

/// One exact observed-type to target-Provider normalization.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DiscoveryMapping {
    pub observed_type: &'static str,
    pub target_provider: &'static str,
    pub route_adapter: RouteAdapter,
}

/// One bounded observation declaration from a Provider's canonical document.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Discovery {
    pub id: &'static str,
    pub service: &'static str,
    pub operation: &'static str,
    pub driver: DiscoveryDriver,
    pub mappings: &'static [DiscoveryMapping],
}

/// One API surface a connector declares, with its base URL as the document resolves it.
///
/// Not `#[non_exhaustive]`: it is two facts, and a consumer constructs one in a test of its own
/// URL composition.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Service {
    /// The service name — `delivery`, `s3`, or the reserved [`DEFAULT`](Self::DEFAULT) for a
    /// connector with a single API surface. This is the name [`Operation::service`] carries.
    pub name: &'static str,
    /// The base URL, templating included: `https://{subdomain}.zendesk.com`. The tenant is the
    /// operator's to choose, and a resolved value here would invent one.
    pub base_url: &'static str,
    /// Curated discovery audiences for this service. Never a runtime-policy input.
    pub audiences: &'static [Audience],
}

impl Service {
    /// The reserved name of a connector's single API surface.
    pub const DEFAULT: &'static str = "default";
}

/// One connector, and every operation the catalog carries for it.
///
/// `#[non_exhaustive]` for the same reason [`Operation`] is: C-37's `pid` lands here.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct Provider {
    /// The connector id, e.g. `zendesk`. Names `connectors/<id>.flux`, and is the key
    /// [`ProviderKey::id`] matches on.
    pub id: &'static str,
    /// The vendor's display name.
    pub vendor: &'static str,
    /// What the connector is for, in one line.
    pub description: &'static str,
    /// The reverse-DNS authority the connector publishes under (`com.slack.api`) — C-37's `pid`,
    /// and the second segment of every credential address this connector's secrets live at.
    ///
    /// `None` for a connector that has not declared one yet. That is not cosmetic: without an
    /// authority no credential address renders at all, so a consumer resolving credentials can only
    /// refuse. Refusing is the correct answer — a request sent without the credential it declares is
    /// the failure worth preventing — but the diagnostic must say *which* fact is missing.
    pub authority: Option<&'static str>,
    /// Every API surface this connector declares, in the document's order — `default` alone for a
    /// connector with a single one.
    ///
    /// **New here, and it closes a real gap.** The predecessor's table carried one connector-level
    /// `base_url` and nothing else, so a multi-service connector's actual hosts were unreachable:
    /// contentful reaches `cdn.contentful.com` for reads and `api.contentful.com` for writes, and
    /// google reaches three. The canonical document has carried the per-service base URL all
    /// along; this is the table catching up.
    pub services: &'static [Service],
    /// The deduplicated union of [`Service::audiences`], in service declaration order.
    pub audiences: &'static [Audience],
    /// The API base URL, templating included — the [`DEFAULT_SERVICE`](Service::DEFAULT) surface's
    /// when the connector declares one, otherwise the first declared service's.
    ///
    /// **Read [`services`](Self::services) instead when the connector has more than one.** A
    /// multi-service connector has no single base URL, and the predecessor's answer here was the
    /// connector-level default from its TOML — a value the canonical document does not carry and
    /// which, for contentful and google, names a host some of their operations never reach. This
    /// field is kept because it is exactly right for the 51 single-surface connectors and because
    /// it is what a one-line summary wants; it is not the value to build a request from.
    pub base_url: &'static str,
    /// Every credential the connector declares, in declaration order and keyed by
    /// [`Credential::name`] — the vocabulary [`Operation::credentials`] references by name.
    ///
    /// Declared at **provider** level, exactly as the IR declares them: a credential belongs to the
    /// connector, not to one of its services, which is why its address elides the service segment.
    pub auth: &'static [Credential],
    /// Every operation, in the order the provider declares them — which is also the order they
    /// appear in `connectors/<id>.flux`.
    pub operations: &'static [Operation],
    /// Complete configuration declarations, never values.
    pub config: &'static [ConfigField],
    /// The bounded, low-risk operation a host invokes to verify this connector's configuration.
    ///
    /// The operation id is guaranteed by the loader to name one of [`Self::operations`]. `None`
    /// means the connector declares no Test-connection read; it is not an invitation to guess one.
    pub verify: Option<&'static str>,
    /// Inbound events in declaration order.
    pub events: &'static [Event],
    /// Channel bindings in declaration order.
    pub channels: &'static [Channel],
    /// Bounded observation sources. An entry produces candidates only; it grants no authority and
    /// creates no Connection.
    pub discoveries: &'static [Discovery],
    /// **Every configuration field whose value comes from a closed set** — C-225.
    ///
    /// Empty for most connectors. Present for the ones whose value is a *choice* rather than a
    /// string the operator knows: New Relic's two region hosts, Intercom's three.
    ///
    /// The complete declaration is available through [`Self::config`] (C-87). This stays as the
    /// indexed compatibility view for consumers that address the set by `(service, kind, name)`.
    pub config_choices: &'static [ConfigChoices],
}

/// One configuration field that permits a closed set of values, and the set — C-225.
///
/// Addressed the way the runtime configuration port addresses a stored value: by
/// `(service, kind, name)`, where `kind` is `endpoint`, `path`, `query`, `header`, `username` or
/// `oauth` and `name` is the binding target within it. `field` is the declaration's own name, which
/// is what a form labels the row with and what an error message quotes.
///
/// Not `#[non_exhaustive]`: a consumer builds one in a test to prove its own rendering, exactly as
/// it does with [`Credential`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ConfigChoices {
    /// The service whose configuration this is. Written out, `default` included, because a consumer
    /// grouping by service needs a name in every row (C-197).
    pub service: &'static str,
    /// The declared field name — `host`. The key a host stores the collected value under.
    pub field: &'static str,
    /// The form label — `New Relic API host`.
    pub label: &'static str,
    /// Where the value goes: `endpoint`, `path`, `query`, `header`, `username` or `oauth`.
    pub kind: &'static str,
    /// The name within `kind` — the base-URL `{variable}`, the pinned wire name, the credential.
    pub name: &'static str,
    /// The permitted values, in the vendor's own order.
    pub choices: &'static [Choice],
}

/// One permitted value of a [`ConfigChoices`] set, and the text a renderer shows for it.
///
/// The label is what makes the set usable: an operator knows their account is in Frankfurt and does
/// not know that `api.eu.newrelic.com` is what that means.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Choice {
    /// The value itself — what a host stores and what substitution puts on the request.
    pub value: &'static str,
    /// The human name for it — `European Union`.
    pub label: &'static str,
}

impl Provider {
    /// One of this provider's operations, by key.
    pub fn operation(&self, key: OperationKey<'_>) -> Option<&'static Operation> {
        self.operations
            .iter()
            .find(|operation| key.matches(operation))
    }

    /// The base URL of one of this connector's services, or `None` when it declares no such
    /// service.
    ///
    /// The value a request is composed from. [`base_url`](Self::base_url) is the one-line summary;
    /// this is the answer per surface, and for a multi-service connector they differ.
    pub fn base_url_of(&self, service: &str) -> Option<&'static str> {
        self.services
            .iter()
            .find(|declared| declared.name == service)
            .map(|declared| declared.base_url)
    }

    /// The credential this connector declares under `name`, or `None` when nothing declares it.
    ///
    /// A lookup rather than a map because the list is short, ordered and generated; the ordering is
    /// the connector's own, and an alternative-selection rule that depends on declaration order
    /// needs it preserved.
    pub fn credential(&self, name: &str) -> Option<&'static Credential> {
        self.auth.iter().find(|credential| credential.name == name)
    }

    /// One declared channel binding, by its service-local name.
    pub fn channel(&self, name: &str) -> Option<&'static Channel> {
        self.channels.iter().find(|channel| channel.name == name)
    }

    /// The closed set governing one configuration slot, addressed exactly as the runtime port
    /// addresses a stored value — `(service, kind, name)`.
    ///
    /// `None` means the slot is open, **not** that the value is unconstrained by anything: the full
    /// [`ConfigField`] still carries its format. A caller uses this indexed view to decide between a
    /// select and an input, and to refuse a value it was not offered.
    pub fn choices_for(
        &self,
        service: &str,
        kind: &str,
        name: &str,
    ) -> Option<&'static ConfigChoices> {
        self.config_choices
            .iter()
            .find(|entry| entry.service == service && entry.kind == kind && entry.name == name)
    }
}

/// How a caller names one operation.
///
/// A key type rather than a bare `&str` so that C-37's global address becomes a *constructor* —
/// `OperationKey::oip(…)` — instead of a reshape of every lookup in the crate. The field is
/// private and there is no `From<&str>`: a caller states which kind of name they hold.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct OperationKey<'a> {
    /// The Flux symbol. C-37 turns this field into an enum over `{ id, oip }`; because the field is
    /// private and every constructor is named, that is an additive change.
    id: &'a str,
}

impl<'a> OperationKey<'a> {
    /// Name an operation by its Flux symbol, e.g. `zendesk-ticket-show`.
    pub const fn id(id: &'a str) -> Self {
        Self { id }
    }

    /// Whether this key names `operation`.
    fn matches(&self, operation: &Operation) -> bool {
        operation.id == self.id
    }
}

/// How a caller names one provider.
///
/// The sibling of [`OperationKey`], and additive in the same way: C-37's `pid`
/// (`com.zendesk.api`) lands as `ProviderKey::pid`, and its middle level — the `gid`, a versioned
/// resource group — lands as a `GroupKey` beside it without disturbing anything here.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ProviderKey<'a> {
    /// The connector id. C-37 turns this field into an enum over `{ id, pid }`.
    id: &'a str,
}

impl<'a> ProviderKey<'a> {
    /// Name a provider by its connector id, e.g. `zendesk`.
    pub const fn id(id: &'a str) -> Self {
        Self { id }
    }

    /// Whether this key names `provider`.
    fn matches(&self, provider: &Provider) -> bool {
        provider.id == self.id
    }
}

/// Every provider in the catalog, ordered by [`Provider::id`].
///
/// The first call parses every canonical document out of the embedded pack; every call after it
/// reads the built table. See [`table`].
pub fn providers() -> &'static [&'static Provider] {
    table::providers()
}

/// One provider, by key.
pub fn provider(key: ProviderKey<'_>) -> Option<&'static Provider> {
    providers()
        .iter()
        .copied()
        .find(|provider| key.matches(provider))
}

/// **Every operation in one provider** — the listing that makes a provider addressable rather than
/// merely nameable.
///
/// An unknown provider yields an empty slice rather than an error; use [`provider`] when the
/// difference between "no such connector" and "a connector with nothing in it" matters.
pub fn operations_of(key: ProviderKey<'_>) -> &'static [Operation] {
    provider(key).map_or(&[], |provider| provider.operations)
}

/// Every operation in the catalog, provider by provider.
pub fn operations() -> impl Iterator<Item = &'static Operation> {
    providers()
        .iter()
        .flat_map(|provider| provider.operations.iter())
}

/// **The lookup.** One operation by key, with its Flux source and its metadata.
///
/// A linear scan: the catalog holds tens of operations today and low hundreds once spec ingest
/// selects more (a spec-ingested babelforce alone offers 163), which is well inside the range where
/// a scan over contiguous `&'static` data beats any index worth maintaining — and an index would be
/// a second generated artifact to keep in step with this one.
pub fn operation(key: OperationKey<'_>) -> Option<&'static Operation> {
    operations().find(|operation| key.matches(operation))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_operation_is_found_by_its_symbol() {
        let operation = operation(OperationKey::id("zendesk-ticket-show"))
            .expect("the shipped catalog carries zendesk-ticket-show");
        assert_eq!(operation.provider, "zendesk");
        assert_eq!(operation.service, "default");
    }

    #[test]
    fn an_unknown_key_is_none_rather_than_a_panic() {
        // A negative sentinel must not be a plausible vendor name. `salesforce` was used here, and
        // shipping the Salesforce connector (C-163) turned "definitely unknown" into "shipped" and broke
        // three tests across two crates at once — `AGENTS.md` had named Salesforce as a provider that
        // belongs here all along, so this was scheduled to happen. The sentinel below is not a company,
        // and each use is self-checking: the assertion *is* that the catalogue does not carry it, so this
        // cannot rot into a vacuous pass the way a stale hardcoded name can.
        const NO_SUCH_PROVIDER: &str = "no-such-vendor";

        assert!(operation(OperationKey::id("zendesk-ticket-obliterate")).is_none());
        assert!(provider(ProviderKey::id(NO_SUCH_PROVIDER)).is_none());
        assert!(operations_of(ProviderKey::id(NO_SUCH_PROVIDER)).is_empty());
    }

    /// Listing by provider is the whole point of the middle level, so it must agree with the flat
    /// listing rather than being a second, drifting source.
    #[test]
    fn listing_by_provider_partitions_the_catalog() {
        let listed: usize = providers()
            .iter()
            .map(|provider| operations_of(ProviderKey::id(provider.id)).len())
            .sum();
        assert_eq!(listed, operations().count());
        assert!(
            listed > 0,
            "an empty catalog would pass every other test here"
        );
    }

    /// Ids are what lookups key on, so a duplicate would make one of the two unreachable.
    #[test]
    fn operation_ids_are_unique_across_the_catalog() {
        let mut ids: Vec<&str> = operations().map(|operation| operation.id).collect();
        let total = ids.len();
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(ids.len(), total, "duplicate operation id in the catalog");
    }

    #[test]
    fn providers_are_listed_in_a_stable_order() {
        let ids: Vec<&str> = providers().iter().map(|provider| provider.id).collect();
        let mut sorted = ids.clone();
        sorted.sort_unstable();
        assert_eq!(ids, sorted);
    }
}
