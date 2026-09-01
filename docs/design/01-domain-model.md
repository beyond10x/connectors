# Design 01: the domain model

**Status:** draft for review · **Date:** 2026-08-13
**Inputs:** [VISION.md](../VISION.md) ·
[research/unified-api-platforms.md](../research/unified-api-platforms.md) ·
[research/catalog-precedents.md](../research/catalog-precedents.md) · the measured entity
inventory of the predecessor codebases (flux-exchange, flux-connectors).

Nouns are forever (vision principle 10). This document picks them, states each entity's
invariants, and records the naming rationale against the industry so a future reader knows which
divergences are deliberate. Wire schemas, storage layouts and API shapes are *not* this document;
they cite it.

## The three layers

Every platform in the researched category converges on the same three-layer shape, and it maps
exactly onto our postures:

```text
CATALOG  (shared text, versioned, signed)      Provider — the template
   │  enabled + configured by a deployment
DEPLOYMENT (one posture: personal/org/saas)    Integration — the tenant-configured provider
   │  authorized by / for people
RUNTIME  (per-user state and traffic)          Connection — the authorized instance
                                               Grant · Invocation · Event · …
```

Industry mapping: Provider/Integration/Connection ≈ Nango provider/integration/connection ≈
Composio toolkit/auth-config/connected-account ≈ Merge integration/dashboard-config/linked-account.
We use the majority nouns; every divergence below says why.

---

## Identity, tenant binding, and connector authority

**2026-08-14 identity-boundary amendment.** This section supersedes the founding vision's wording
that assigned identity, organizations, login sessions, and one reusable client token to this
platform. General identity belongs to `beyond10x/identity`; Connectors owns only personal-local
authentication plus receiver-side admission, connector capabilities, and connector Grants.

### Admitted principal and tenant projection

Every connection, credential, grant, event, and audit record belongs to exactly one tenant binding.
That binding is not a Connectors-owned Organization record:

- **personal:** one implicit deployment-local tenant and one local-owner subject, derived from an
  owner-permissioned Unix peer or generated owner-held bearer;
- **organization/hosted:** a closed validated principal from the platform Identity verifier,
  matched against the receiver-configured audience, tenant, deployment posture, token lifetime,
  key generation, and revocation freshness.

The initial hosted deployment is bound to one trust domain and tenant. A future multi-tenant
Connectors process requires a separate threat model and design; a request field never selects a
tenant or identity verifier.

- *Invariant:* the accepted tenant is part of the admitted `Principal` **structurally**. No domain
  constructor or runtime operation accepts a second tenant that could override it; admission
  compares and refuses on mismatch rather than rewriting.
- *Invariant:* tenant ids are validated at construction because they partition persistence, but
  Connectors stores only the stable receiver projection needed by its own records. It does not own
  organization membership, profiles, login identities, sessions, or general roles.
- *Invariant:* identity resolution distinguishes "nothing presented" from "presented and bad";
  the latter is never anonymous.

### Identity principals

In hosted posture, human, service, and deployment principals are Identity-owned facts. Connectors
accepts only the released validated-envelope/verifier contract; it never terminates upstream OIDC,
stores an Identity login session, accepts an upstream-provider token, validates a client assertion
as resource authority, or mints a hosted principal.

An Identity service principal is therefore not a Connectors-owned credential class. It authenticates
with Identity using Identity-owned asymmetric credentials and presents short-lived, exact-audience
authority to Connectors. Connectors stores neither its private key nor a reusable bearer verifier.

### Connector operator capability

Operator is a connector-specific capability axis, not a principal kind or general Identity role. A
personal local owner receives it from the personal posture. A hosted principal receives only the
closed connector audience scopes carried by a valid Identity result and is then narrowed by
receiver-owned connector policy and Grants. Being authenticated, human, or an organization member
does not itself confer connector management authority.

- *Invariant:* missing or malformed connector-management policy admits **nobody**.
- *Invariant:* Identity never evaluates a Connection or Grant and Connectors never promotes an
  Identity role into ambient operator authority.

### Connect Session

A short-lived, server-created, single-purpose token that authorizes exactly one thing: completing
(or repairing) one connection's authorization, optionally bound to a set of allowed integrations
and carrying reconciliation tags (end-user id, external ids). Surfaced as a hosted URL, an
embeddable component, or through a trusted product/control-plane surface that hands it to the
human. It is never a model or inner-harness tool result. In personal-local operator-entry flows it
may expose a short-lived owner-only completion socket. That completion endpoint is called once; it
is neither the durable Connection nor an Agent Endpoint.

- *Invariant:* a connect session never carries or returns credential material to its creator;
  its terminal event names the connection id, nothing else.
- *Invariant:* a Connect Session is vendor-credential acquisition state, not an Identity login
  session, Foundation Trust Envelope, service credential, or general client authority.
- Industry mapping: ≈ Nango connect session, Merge link token / Magic Link, Apideck Vault
  session. Adopted wholesale; it is the pattern that keeps vendor tokens out of clients.

---

## Catalog side

### Provider

The catalog template for one vendor surface: reverse-DNS **authority** (e.g. `com.gitlab.api`),
services with base-URL templates, **operations**, the complete credential surface (schemes,
acquisition, placement, subject, hazard, full OAuth2 spec), configuration fields, events, channel
bindings, verification probes, curated service audiences, and per-operation runtime traits — pagination, rate limits, error
envelopes — as **first-class named fields**. (The predecessor bagged these under `quirks`; that
umbrella retires. A rare, precisely-scoped `workarounds` category may exist for genuine vendor
spec deviations, each entry naming the defect it compensates — its rarity is the signal.)

Authored from an authoritative **source specification** plus explicit reviewed overlays. The source
is either an official machine-readable vendor artifact or, when none exists, a
repository-authored specification that says so and cites the official documentation it models.
The deterministic projection (currently provider TOML/OpenAPI ingest) compiles those inputs to one
canonical JSON document, hashes it in the lockfile and packs it into an offset-indexed artifact.
"Connector" names the *source specification + overlay declaration*; "Provider" names the
*compiled catalog entry*. One noun pair, fixed here.

- *Invariant:* review-equals-execution — every derived form is byte-identical to the committed
  canonical documents; unchanged inputs reproduce every artifact byte for byte.
- *Invariant:* every provider is reproducible from a declared spec origin plus reviewed overlays;
  the current imported hand-authored declarations are migration debt until that derivation is
  explicit, not an exemption from the model.
- *Invariant:* the document carries the registration **requirement** (which OAuth fields a
  deployment must supply), never a value. No `client_id` ever appears in the catalog.
- *Invariant:* the request-template vocabulary is closed and total; anything outside it is a
  build error, never a degraded document.
- *Invariant:* an audience (`sre`, `developer`, `sales-rep`, …) is service-level discovery metadata
  only. Provider audiences are the derived union used by catalog explorers; Connection ownership,
  Grant admission, visibility and runtime authorization never consume it.
- *Invariant:* protocol driver, interaction shape and required capabilities are declared facts,
  never caller choices. HTTP is the first driver, not an implicit default for an unknown one.
- *Invariant:* the authority is never repointed once published; it leads every credential path.

### Operation

One declared, invocable unit of a provider: id (stable, public), service, direction, **risk**
(ordered), **idempotency**, **effects**, `expose` (callable vs projected-to-models), parameter
schemas, an explicit request template, response schema, and the runtime traits above.

Beyond HTTP, an Operation also names its closed protocol driver, interaction shape and capability
requirements as defined by [Design 03](03-beyond-http.md). Process/container placement is not a
protocol and remains a separate deployment decision.

- *Invariant:* the facts the authorization gate decides on are the facts the catalogue publishes
  — a client can always predict admission from what it can read.
- *Invariant:* effects are **read from the document, not derived** — the predecessor derived
  them (every operation got `Network`, nothing else was ever emitted), which made effect
  selectors silently over-admit the day a new effect shipped. The document schema carries
  effects per operation from day one.
- Industry note: no platform has this vocabulary (closest: Pipedream's three MCP boolean hints).
  It is the core differentiator; the grant system consumes it.

### Catalog (deployed)

The sealed, versioned set of providers a deployment serves: the pack, its schema version and its
content **generation** (monotonic). One value per process; origin is embedded-or-loaded, never
"loaded, then silently fell back".

- *Invariant:* a pack that fails verification (format, schema, digest) refuses **startup**.
- *Invariant:* the **effective catalogue** — what one principal sees — is the intersection of
  the served catalog with the admitted tenant's integrations and the principal's Grants, sealed
  under a content-digest generation so clients can diff cheaply.
- Later (out of v1 scope, shaped for from day one): **composition** — an ordered set of catalog
  sources (official + tenant overlays, same schema, own signing trust), collisions refused
  unless explicitly declared, audit recording `(source, generation)` per served operation.

---

## Deployment side

### Integration

A tenant's configured enablement of a provider — the middle layer the predecessor lacked
as a first-class noun (it existed as scattered per-connector settings). Holds: enabled state,
the deployment's OAuth registration (BYO client_id/secret as file-shaped secrets), allowed
scopes, settings defaults, and the **destination policy** for private-host providers
(deployment-operator authority; a value-free allowlist — no request field, catalog entry or
grant may name a destination).

- *Invariant:* an integration configures a provider; it never extends one. The catalog is the
  only source of surface.
- Industry mapping: ≈ Nango Integration, Composio Auth Config. The majority noun, adopted.

### Connection

An authorized instance of an integration: stable id, human **label**, scope
(`tenant`-shared or `principal`-owned), connection config values, and a lifecycle:

```text
created ──authorize──▶ authorized ──verify──▶ callable
                            ▲                    │ credential expiry / revocation upstream
                            └── reauthorize ── degraded ──revoke──▶ revoked
```

- *Invariant:* connection ids are stable across reauthorization — repair in place, never
  delete-and-recreate; everything that references a connection (grants, subscriptions, event
  attribution) survives a token refresh or re-auth.
- *Invariant:* labels are an overlay for humans and disambiguation; credential existence is
  authoritative. Multiple connections per integration are first-class (labels distinguish them).
- *Invariant:* every callable Connection has a non-empty initiation policy containing
  `b10x`, `provider`, or both. `b10x` allows an operation caller (including the coding
  harness) to start work through the Connection; `provider` allows the configured external system
  to start an admitted inbound session or event path. Inactivity is a lifecycle state, never an
  empty initiation policy.
- *Invariant:* initiation policy and Grant authority are independent gates. Allowing
  `b10x` does not grant `sip.dial` or any other operation; allowing `provider` does not grant
  every inbound channel. The Connection answers *who may start*, then the Grant answers *which
  reviewed member may run*.
- *Invariant:* initiation is not the catalog operation's `direction`. `direction = write` describes
  vendor-state effect; it cannot say which side is allowed to begin a connection.

**2026-08-15 delegated-actor amendment.** Three identities remain separate on every invocation:

1. the authenticated caller (a person, harness, or agent such as Zwirn);
2. the Connection owner (`tenant`-shared or `principal`-owned); and
3. the credential subject the vendor observes (`app` or `user`).

An agent may be granted use of a principal-owned user Connection, but that does not turn the app's
bot credential into a user credential. The host selects the user Connection, evaluates its Grant,
and places only that Connection's user-subject credential; the vendor therefore attributes the
operation to the consenting user. A tenant-shared app Connection remains a different Connection and
never serves as fallback. Audit records identify caller, Connection, and declared credential
subject without containing the credential. Slack and GitLab prove the two paths in
[Design 09](09-curation-and-credential-capability-admission.md).

**2026-08-14 route amendment.** Every Connection has one immutable route: `direct`, or
`via_connection(parent Connection, opaque resource binding, closed route adapter)`. The mediated
form remains governed by its target Provider contract; the parent supplies transport only. It
does not imply direct backend access or another credential. It
does not inherit the parent's Provider semantics, credentials, Connector Grant, or Agent Endpoint
Grant. Discovery may propose such a Connection but never creates or authorizes one. See
[Design 08](08-discovery-observations-and-mediated-connections.md).

- Scope rationale: agent automation needs both the team's shared GitLab connection and a user's
  personal calendar; the industry models only end-user-owned connections, the predecessor only
  org-owned ones. We carry both, and grants decide who may exercise which.
- Industry mapping: ≈ Nango/Apideck Connection, Merge Linked Account, Composio Connected
  Account. States adopted from Apideck's ladder + Nango's reconnect-in-place.

### Credential

Vendor secret material backing a connection: acquired through a connect session (OAuth
authorization code, API key entry, …) or operator-entered; refreshed by the platform; stored
owner-bound (owner-only files in personal/org; per-tenant envelope encryption in saas) with
prepared, atomic multi-step mutations.

**2026-08-14 credential-custody amendment.** Owner-bound describes an authority boundary, not a
license to keep release credentials in plaintext files. A released personal deployment stores an
entered value in the operating-system keychain or binds the credential slot to an operator-approved
external secret provider. An organization deployment uses its configured encrypted credential
store, an external secret provider, or workload identity. A development-only owner-permissioned
file backend may prove transaction and recovery behavior, but is not a releasable credential
posture. [Design 07](07-credential-custody-topologies.md) proposes the placement and custody matrix
for cross-component decision.

- *Invariant:* a credential never crosses to a client, appears in a log, an audit record, an
  error, or a catalog artifact. Placement (where it goes in a request) and subject/hazard (what
  it touches, what may see it) come from the catalog.
- *Invariant:* no memory fallback, no path inside a working tree, refusal names the address,
  never the value.
- *Invariant:* `token_response_metadata`-style extraction (extra OAuth response fields such as a
  webhook URL or bot user id) lands in **connection metadata**, never in the credential store.

**2026-08-15 credential-capability amendment.** Credential presence is not operation authority.
Each scope-bearing credential generation has value-free capability evidence on its Connection:
subject, proven granted scopes, required provider installation context, observation source, and
freshness. Catalog requirements associate scopes with their exact credential purpose; discovery
and invocation apply the same fail-closed predicate. Requested scopes, configuration, token
prefixes, and caller assertions are not evidence. See
[Design 09](09-curation-and-credential-capability-admission.md).

- *Invariant:* scope sets from different credentials on one Connection are never unioned.
- *Invariant:* missing or stale capability evidence hides the member and refuses dispatch; every
  cache is bound to the credential generation that produced the evidence.

---

## Runtime side

### Grant

The unit of authority: tenant-scoped, per-connector, admitting operations by **selector**
over declared facts — risk ceiling, effects subset, idempotency — plus explicit allow/deny
exceptions, where **deny beats allow beats predicate**. Inbound grants admit provider events as
a **closed** event set (no wildcards). Grants bind to connections (which one a principal may
exercise), never to credentials.

- *Invariant:* admission chains through unconstructible proof types (admitted → granted → the
  only route to dispatch); skipping a gate is a compile error.
- *Invariant:* no store bound is an outage (503), an empty store is a refusal (403) — fail
  closed, and the refusal never names the axis that refused (no policy enumeration oracle).
- *Invariant:* grant mutation is CAS-revisioned with previewable proposals and receipts.

Identity access authority and connector Grants are two different gates. The closed initial
Connectors audience-scope vocabulary is:

- `connectors.catalog.read`
- `connectors.invoke`
- `connectors.events.read`
- `connectors.events.self`
- `connectors.audit.read`
- `connectors.integrations.manage`
- `connectors.connections.manage`
- `connectors.connections.self`
- `connectors.grants.manage`
- `connectors.channels.manage`
- `connectors.deliveries.manage`

Identity carries these exact audience-owner strings after its own exchange; it does not define or
interpret their connector meaning. They admit route families only. Connectors still resolves its
own Connection and Grant and applies the fine-grained operation/effect/risk decision. An
An Identity-carried `connection_id` or `grant_id` value, if present for correlation, is never proof that
the referenced Connectors record exists or grants an operation.

### Invocation

One granted execution of one operation over one connection: parameters in → request plan
(**data**: method, url, headers, body, permission subjects, redaction set) → credential placed →
dispatched → response or refusal out, audited.

- *Invariant:* permission subjects are computed **before** credential placement, so a
  query-placed secret can never enter an approval prompt or an evidence record.
- *Invariant:* the request plan is derived only from the canonical document — nothing at runtime
  parses source of any kind (the predecessor's parse-back mistake, closed by construction).
- *Invariant:* there is exactly one request-composition path; a consumer that edits a plan has
  become a second one, and that is refused by design (and by fence tests).
- *Invariant:* caller-initiated planning additionally proves that the selected Connection admits
  the `b10x` initiator. Provider-started session/channel admission proves `provider` on its
  own path; neither path infers the other.

### Proxy

Raw authenticated passthrough is **not part of the generic v1 or model-callable surface**. If an
operator deployment enables it later, it is a distinct break-glass capability with fixed maximum
facts: `risk = destructive`, non-idempotent, and the full conservative semantic-effect set,
including external send, write, delete, money movement, and network access. It has a separately
granted method/path aperture in addition to the Integration destination policy, and every use is
audited as raw authority rather than disguised as a catalog operation.

- *Invariant:* a model, ordinary Identity service principal, or catalog grant cannot obtain raw proxy
  authority. Credential-bearing model calls require a declared operation whose reviewed facts are
  the execution facts. Raw access is never a premium feature (vision principle 8).

### Channel

A supervised instance of a provider's declared channel binding for one connection — the
provider-side transport (websocket, webhook registration) the platform owns so that events flow
without any client running. Opaque host-minted id; lifecycle owned by the platform.

**2026-08-14 source-partition amendment.** A channel binding may expose an opaque source partition
whose identity is owned by the upstream system. For substrate ingestion, the concrete Channel key
is `(Connection, source_scope)`, not Connection alone. `source_scope` is substrate-minted and is
stored/compared as opaque data; connectors never constructs it from an organization, principal, or
tenant. Native deduplication uses `(deployment, source_scope, generation, seq)`. This partitions
supervision and high-water marks without changing Connection credential custody.

### Event

One normalized unit of something happening, in two disjoint families:

- **data events** — from providers, through channels/webhooks, attributed to a connection,
  labeled by the catalog's closed event set, deduplicated by delivery id;
- **operational events** — from the platform itself: credential degraded, delivery failing,
  channel down. Never mixed into the data stream (separate family, separate subscriptions).

- *Invariant:* every event carries provenance — `native` (pushed by the provider) vs `polled`
  (synthesized) — honestly. No reliability theater.

### Webhook (inbound)

The webhook transport of channels: one platform endpoint; per-provider **verification and
attribution declared in the catalog** (signature matrix, discriminator, delivery-id selector,
payload map — declarative rules, no script escape hatch until a real provider defeats the
grammar; industry data point: 94% of Nango's 957 providers never needed one).

### Delivery & Subscription (outbound)

How events reach clients. **Subscription**: authenticated pull (one multiplexed stream per
client, gated by inbound grants). **Delivery**: durable per-endpoint push queues with the
Svix-style envelope — `id` + `timestamp` + HMAC-SHA256 over `{id}.{timestamp}.{body}` with a
dedicated signing key — retries with backoff, and **replay-by-id** as a first-class client API
(the researched category's most conspicuous gap).

- *Invariant:* never sign with an API key; never canonicalize the payload before signing.
- *Invariant:* only an authenticated tenant principal with `connectors.deliveries.manage` may
  register or change a delivery endpoint. The normalized endpoint and its post-resolution
  destination aperture are
  stored as governed deployment configuration. Event payloads, catalog data, connection values,
  grants, and models cannot select or widen the destination. Both registration validation and the
  delivery worker apply the shared aperture immediately before opening a socket.

### Audit

Append-only record of every security-relevant action: closed action vocabulary, closed target
vocabulary, outcome, principal, request id, retention window.

- *Invariant:* **no generic metadata field** — tokens, bodies, OIDC material and credential
  values are unrepresentable in the record type, not merely forbidden.

---

## Structural invariants (cross-cutting)

1. **Proof-type chains, not scanners.** Every multi-gate path (identity verification → audience
   scope → grant → dispatch; connect session → vendor credential acquisition; proposal → apply) is
   chained by values with private fields and no public
   constructor, `Default`, or `Clone`.
2. **Tenant-in-admitted-principal.** No API, port or constructor accepts a tenant beside an
   admitted identity.
3. **Closed vocabularies everywhere** — risk, effects, idempotency, event sets, audit actions,
   template grammar, auth schemes. Extension is a schema version, not a wildcard.
4. **Routes as data with access on the route.** The HTTP surface is an enumerable value; guard
   tests compare against a declared list. Domain route modules stay small (the predecessor's
   10k-line route file is a named anti-goal).
5. **Fail closed, refuse by name, one source of truth, no foreign engine in dispatch** — vision
   principles 3, 6, 7 applied to every entity above.

## Out of scope (v1)

- **Workflows, managed apps, behaviour of any kind.** Behaviour is a client. The reintroduction
  path, if ever, is a supervised client runtime over the same contract, never an embedded engine.
  Private predecessor research is provenance for that rule, not required authority.
- **Unified data models** — declared operations are the generic invocation layer; operator-only raw
  proxy is deferred behind S-030 and never substitutes for a reviewed operation.
- **Leases / long-running operation scopes** — designed in the predecessor, never used in anger;
  reintroduce only against a real consumer.
- **Signed catalog-source composition/overlays** — shaped for (source identity in locks and audit
  from day one), not built. This is distinct from the runtime's generated-service bundle seam:
  that seam composes reviewed manifests and deployment bindings into `ConnectorBackend` instances,
  but does not merge or mint canonical catalog packs.

## Open questions

1. Multi-tenant process topology and SaaS organization lifecycle remain Identity/Cloud plus later
   Connectors composition work; M2 does not create an Organization store.
2. Whether org-posture destination policy (private-host allowlist) belongs on Integration (as
   modeled here) or as a deployment-global document with per-integration references.
3. The exact connect-session ↔ OAuth-callback custody chain in personal posture, where there is
   no public callback origin (loopback redirect vs device-code-style flows per provider).
4. Naming of the client-facing effective-catalogue "generation" vs the catalog artifact's
   "content generation" — same word, two seals today; consider distinct nouns.
