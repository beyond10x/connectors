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

```
CATALOG  (shared text, versioned, signed)      Provider — the template
   │  enabled + configured by a deployment
DEPLOYMENT (one posture: personal/org/saas)    Integration — the org-configured provider
   │  authorized by / for people
RUNTIME  (per-user state and traffic)          Connection — the authorized instance
                                               Grant · Invocation · Event · …
```

Industry mapping: Provider/Integration/Connection ≈ Nango provider/integration/connection ≈
Composio toolkit/auth-config/connected-account ≈ Merge integration/dashboard-config/linked-account.
We use the majority nouns; every divergence below says why.

---

## Identity & tenancy

### Organization

The tenant. Every principal, connection, credential, grant, event and audit record belongs to
exactly one organization.

- **personal**: exactly one, implicit, owned by the machine owner. **org**: exactly one,
  explicit, bound to the deployment's IdP. **saas**: many.
- *Invariant:* the organization is part of every `Principal` **structurally** — no constructor
  takes an organization separately from an identity, and no runtime operation accepts one as a
  parameter that could stand in for the principal's. Admission compares and refuses on mismatch;
  it never rewrites. (Proven in the predecessor; the single rule that stops a config change
  turning one org's token into another org's authority.)
- *Invariant:* organization ids are validated at construction (bounded length, closed character
  set) because they lead storage paths.

### User

A human principal. Identity arrives from the posture's source: local owner (personal), OIDC with
PKCE + signature-verified claims (org), hosted accounts (saas). Sign-in yields a session;
sessions are process-local and cookie-bound (`__Host-`, `SameSite=Strict`).

- *Invariant:* nothing about authority is derived from sign-in alone (see Operator).
- *Invariant:* identity resolution distinguishes "nothing presented" from "presented and bad";
  the latter is never anonymous.

### Operator

A **role axis, not a principal kind**: deployment-declared authority (an allowlist of immutable
IdP subjects, or the local owner in personal posture) over management surfaces — integrations,
grants, service accounts, channels. Being human is an identity fact; being an operator is
deployment policy.

- *Invariant:* a missing or malformed operator policy admits **nobody** (fail closed).
- Industry note: no researched platform separates this axis; dashboard access is all-or-nothing.
  Kept from the predecessor — it is what makes org posture safe to open to every employee.

### Service Account

The one token a client holds (vision: authenticate once). Minted by a signed-in human, bounded
lifetime, revocable, auditable; the store keeps a verifier, never the token.

- *Invariant:* a Service Account token grants access to **operations, never to credentials**.
- *Invariant:* resolution takes one explicit clock reading.
- **org** posture: members mint their own (bounded count, per-principal, audited); operator
  authority is not required for self-service tokens.
- Industry mapping: ≈ Merge `account_token`, Paragon user JWT — but scoped by grants, not by
  linked-account.

### Connect Session

A short-lived, server-created, single-purpose token that authorizes exactly one thing: completing
(or repairing) one connection's authorization, optionally bound to a set of allowed integrations
and carrying reconciliation tags (end-user id, external ids). Surfaced as a hosted URL, an
embeddable component, or headlessly (an agent hands the URL to a human — auth-as-tool-result).

- *Invariant:* a connect session never carries or returns credential material to its creator;
  its terminal event names the connection id, nothing else.
- Industry mapping: ≈ Nango connect session, Merge link token / Magic Link, Apideck Vault
  session. Adopted wholesale; it is the pattern that keeps org tokens out of browsers.

---

## Catalog side

### Provider

The catalog template for one vendor surface: reverse-DNS **authority** (e.g. `com.gitlab.api`),
services with base-URL templates, **operations**, the complete credential surface (schemes,
acquisition, placement, subject, hazard, full OAuth2 spec), configuration fields, events, channel
bindings, verification probes, and per-operation runtime traits — pagination, rate limits, error
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
  the served catalog with the organization's integrations and the principal's grants, sealed
  under a content-digest generation so clients can diff cheaply.
- Later (out of v1 scope, shaped for from day one): **composition** — an ordered set of catalog
  sources (official + organization overlays, same schema, own signing trust), collisions refused
  unless explicitly declared, audit recording `(source, generation)` per served operation.

---

## Deployment side

### Integration

An organization's configured enablement of a provider — the middle layer the predecessor lacked
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
(`organization`-shared or `user`-owned), connection config values, and a lifecycle:

```
created ──authorize──▶ authorized ──verify──▶ callable
                            ▲                    │ credential expiry / revocation upstream
                            └── reauthorize ── degraded ──revoke──▶ revoked
```

- *Invariant:* connection ids are stable across reauthorization — repair in place, never
  delete-and-recreate; everything that references a connection (grants, subscriptions, event
  attribution) survives a token refresh or re-auth.
- *Invariant:* labels are an overlay for humans and disambiguation; credential existence is
  authoritative. Multiple connections per integration are first-class (labels distinguish them).
- Scope rationale: agent automation needs both the team's shared GitLab connection and a user's
  personal calendar; the industry models only end-user-owned connections, the predecessor only
  org-owned ones. We carry both, and grants decide who may exercise which.
- Industry mapping: ≈ Nango/Apideck Connection, Merge Linked Account, Composio Connected
  Account. States adopted from Apideck's ladder + Nango's reconnect-in-place.

### Credential

Vendor secret material backing a connection: acquired through a connect session (OAuth
authorization code, API key entry, …) or operator-entered; refreshed by the platform; stored
owner-bound (owner-only files in personal/org; per-org envelope encryption in saas) with
prepared, atomic multi-step mutations.

- *Invariant:* a credential never crosses to a client, appears in a log, an audit record, an
  error, or a catalog artifact. Placement (where it goes in a request) and subject/hazard (what
  it touches, what may see it) come from the catalog.
- *Invariant:* no memory fallback, no path inside a working tree, refusal names the address,
  never the value.
- *Invariant:* `token_response_metadata`-style extraction (extra OAuth response fields such as a
  webhook URL or bot user id) lands in **connection metadata**, never in the credential store.

---

## Runtime side

### Grant

The unit of authority: organization-scoped, per-connector, admitting operations by **selector**
over declared facts — risk ceiling, effects subset, idempotency — plus explicit allow/deny
exceptions, where **deny beats allow beats predicate**. Inbound grants admit provider events as
a **closed** event set (no wildcards). Grants bind to connections (which one a principal may
exercise), never to credentials.

- *Invariant:* admission chains through unconstructible proof types (admitted → granted → the
  only route to dispatch); skipping a gate is a compile error.
- *Invariant:* no store bound is an outage (503), an empty store is a refusal (403) — fail
  closed, and the refusal never names the axis that refused (no policy enumeration oracle).
- *Invariant:* grant mutation is CAS-revisioned with previewable proposals and receipts.

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

### Proxy

Raw authenticated passthrough is **not part of the generic v1 or model-callable surface**. If an
operator deployment enables it later, it is a distinct break-glass capability with fixed maximum
facts: `risk = destructive`, non-idempotent, and the full conservative semantic-effect set,
including external send, write, delete, money movement, and network access. It has a separately
granted method/path aperture in addition to the Integration destination policy, and every use is
audited as raw authority rather than disguised as a catalog operation.

- *Invariant:* a model, ordinary Service Account, or catalog grant cannot obtain raw proxy
  authority. Credential-bearing model calls require a declared operation whose reviewed facts are
  the execution facts. Raw access is never a premium feature (vision principle 8).

### Channel

A supervised instance of a provider's declared channel binding for one connection — the
provider-side transport (websocket, webhook registration) the platform owns so that events flow
without any client running. Opaque host-minted id; lifecycle owned by the platform.

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
- *Invariant:* only an authenticated tenant principal with `delivery.manage` may register or change
  a delivery endpoint. The normalized endpoint and its post-resolution destination aperture are
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

1. **Proof-type chains, not scanners.** Every multi-gate path (admission → grant → dispatch;
   session → mint; proposal → apply) is chained by values with private fields and no public
   constructor, `Default`, or `Clone`.
2. **Organization-in-principal.** No API, port or constructor accepts a tenant beside an
   identity.
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
- **Catalog composition/overlays** — shaped for (source identity in locks and audit from day
  one), not built.

## Open questions

1. SaaS org lifecycle (creation, billing identity, deletion semantics) — needs its own design.
2. Whether org-posture destination policy (private-host allowlist) belongs on Integration (as
   modeled here) or as a deployment-global document with per-integration references.
3. The exact connect-session ↔ OAuth-callback custody chain in personal posture, where there is
   no public callback origin (loopback redirect vs device-code-style flows per provider).
4. Naming of the client-facing effective-catalogue "generation" vs the catalog artifact's
   "content generation" — same word, two seals today; consider distinct nouns.
