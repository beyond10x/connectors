# Design 02: architecture

**Status:** draft for review · **Date:** 2026-08-13
**Inputs:** [01-domain-model.md](01-domain-model.md) ·
[../research/catalog-precedents.md](../research/catalog-precedents.md) · the measured predecessor
inventory (what migrates wholesale, what is redesigned, what is left behind).

Private predecessor decisions are provenance only. Every rule this design carries from them is
restated here or in an ADR/story so a reader never needs an unavailable record to know the
current contract.

## 2026-09-02 amendment: catalog browsing belongs to products

The in-repository `web/` explorer and its generated `web/public/catalog.json` projection were never
shipped and had remained incompatible with the M1 document shape. They are retired together. The
hosted, authenticated Catalog protocol is now the sole interactive catalog source, and DevCenter is
the first product UI that presents it. Canonical per-provider documents and `catalog.pack` remain
the build artifacts; `catalog-build` no longer contains a site projection.

This amendment supersedes the `web/` layout row, the site-projection responsibilities attributed to
`catalog-build` below, the maintenance-tool mention of site projection, and open question 2. It does
not turn catalog membership into invocation authority: products still use the Operation contract
to discover effective callable capabilities.

This document fixes the physical shape: repository layout, crate boundaries, storage, the one
invocation path, the fence regime, and the build order. Wire schemas and endpoint lists come
later and cite this.

## 1. Repository layout

```text
providers/            reviewed projection overlays/recipes (TOML; transitional declarations in M1)
specs/                official or repository-authored API specs + provenance sidecars
catalog/              canonical documents (compiled) + connector-document.schema.json
connectors.lock       per-provider input/artifact hashes + the pack digest
crates/               one Rust workspace (catalog family + platform family + the binary)
docs/                 VISION, design/NN-*, research/, stories/ (track framework)
web/                  public catalog explorer (sources; generated output not committed twice)
scripts/              spec vendoring + scrub + release
sdks/go  sdks/ts      client SDKs (later)
console/              operator UI (later; the API and CLI come first)
```

The catalog dirs (`providers/`, `specs/`, `catalog/`, `scripts/`, `connectors.lock`) migrate
from the predecessor **as text, largely unmodified**. M1 therefore contains transitional
hand-authored provider declarations. The target invariant is stricter: every canonical connector
document is deterministically reproducible from a provenance-bearing source spec plus reviewed
overlays. An official machine-readable spec is preferred; where none exists, the component owns
and marks the authored spec instead of hiding the missing source behind a hand-written output.

## 2. The workspace

Nothing publishes to crates.io. Every crate is an internal workspace member; names are
path-local. Two families and one binary:

### Catalog family (migrating, engine-free by construction)

| Crate | Role | Provenance |
|---|---|---|
| `crates/connector-address` | addressing vocabulary (zero internal deps) | copy as-is |
| `crates/connector-spec` | IR + provider-TOML/OpenAPI front-ends + lockfile writer | copy as-is |
| `crates/catalog-reader` | pack reader, zero non-optional deps, `Pack::load` | copy as-is |
| `crates/connector-resolve` | canonical document → `RequestPlan` (data) | copy as-is |
| `crates/connector-secrets` | owner-bound secret store (+ optional Vault backend) | copy as-is |
| `crates/catalog-build` | the build orchestrator: pipeline (plan/apply), document lowering, pack writer, site projection, diff | extracted from the predecessor's CLI crate **minus** the Flux emitter and legacy artifact writers |
| `crates/catalog` | typed views (`Provider`, `Operation`, `Credential`, …) **derived from the pack** for `connector-resolve` | rebuilt over `catalog-reader` in M1; the predecessor's 17k-line generated tables are gone |

Day-one changes, from the precedents analysis and the predecessor's own stories:
1. The document schema gains the caller-contract fields (predecessor C-552: caller-facing
   symbol, error-envelope-extended description, contract `input_schema`) and **per-operation
   `effects` read from the document, never derived**.
2. `connectors.lock` gets the verifier it never had: `catalog check` recomputes every hash and
   exits non-zero on drift.
3. Adopt from the wild (ordered by cost/benefit): `token_response_metadata`, header-name
   rate-limit retry, per-service verification probes.
4. Retire the `quirks` umbrella: `pagination`, `rate_limit`, `error_envelope` become first-class
   fields; a rare `workarounds` category (each entry naming the vendor defect it compensates)
   exists only if a concrete deviation needs it. Strictly after the M1 differential.

### Platform family (fresh)

| Crate | Role |
|---|---|
| `crates/domain` | the nouns of design 01 as types: entities, closed vocabularies (risk, effects, audit actions…), ports (traits) for every store, and the **proof-type gates** (admission → grant → dispatch). No IO, no HTTP, no persistence. |
| `crates/protocol` | versioned wire contracts: protocol identities (`connectors.api.v1`, `connectors.invoke-request.v1`, …), request/response DTOs, strict conformance (`deny_unknown_fields`, bounded diagnostics). The single source for SDK generation later. |
| `crates/service` | use-cases over ports: connection lifecycle, connect sessions, acquisition, grant admission and CAS mutation, invocation assembly (document → plan → placed request), event routing, delivery queues. Pure logic; testable without a socket. |
| `crates/server` | composition: axum transport with routes-as-data + `Access` on the route, personal-local authentication or the released platform Identity verifier adapter, injected state + secret-store bindings, the closed protocol-driver registry, egress, channel supervision, WS subscriptions, the binary's serve path. It never implements OIDC login or Identity session/service-credential storage. |

The predecessor's two-crate split (host/server) was right; its failure mode was god modules
(one 10.7k-line route file). The four-crate split above moves the pressure points (`service`,
`protocol`) out of the transport crate, and a fence asserts a size discipline per module
(soft cap 1,500 lines; breach requires a named waiver in the fence test, not silence).

### The binary

Two binaries, deliberately — the repo-maintenance tool and the product are different programs
for different audiences, and conflating them would ship the catalog compiler to every end user
and spend the product's name on a dev tool:

| Binary | Crate | Audience & verbs |
|---|---|---|
| `catalog` | `crates/catalog-cli` — internal, **never a release artifact** | this repo's maintainers, agents and CI: `catalog build \| diff \| check \| scaffold`, `catalog sources check \| refresh \| diff \| mint` (S-016/S-017) |
| `connectors` | `crates/connectors-cli` — the product, arrives with M2 | end users and operators, against any deployment and entirely without flux: `connectors serve`, connector-owned administration (`integration \| connection \| grant \| channel \| delivery`), and the client verbs (`connect`, `invoke`, `events`). Hosted login, organizations, memberships, sessions, and service credentials remain Identity surfaces. |

The maintenance tool links the compiler family (`connector-spec` ingest, site projection,
`catalog-build`); the product CLI links `protocol`/`service`/`server` and **never the
compiler**.

Both binaries parse their command line with **clap (derive API)** — hand-rolled argv parsing is
banned in this repository. The predecessor hand-rolled its connector CLI to avoid a dependency
mid-flight and it went sideways; the parser is not where this project spends its novelty budget.

`connectors serve` with no config is the **personal posture**: prefer an owner-permissioned Unix
socket; otherwise use a loopback listener plus a generated high-entropy token stored under the
owner-only state root. Local reachability alone is never identity. The posture has one implicit
deployment-local tenant namespace and refuses a working-tree state path. Zero manual configuration
remains the personal tier's contract — secure local material is generated automatically.

**2026-08-14 identity-boundary amendment.** The personal posture above remains Connectors-owned.
Organization and hosted postures do not terminate OIDC or mint/store login or service credentials.
They consume the released platform Identity validated-envelope/verifier contract and apply a
second, Connectors-owned audience-scope and Grant decision. This amendment supersedes the founding
`local owner / OIDC / hosted` server split and every M2 reference to Connectors-owned hosted login.

**2026-08-15 composition-boundary amendment.** The platform family and binary descriptions above
name responsibilities correctly but put too much composition in `server` and the product package.
The following physical boundary supersedes those parts of the table:

- `service` owns the transport-neutral `ConnectorBackend` port and its request/result/error
  contract. `server` implements only personal-local and hosted transports around that port; it may
  not define the port, select an Integration, open a credential source, or contain a provider
  adapter.
- `connectors-client` is the reusable typed control client used by the CLI and future products. It
  owns transport selection, bounded request/response exchange, protocol-version refusal, and
  provider-neutral Connection workflows such as candidate activation and Connect Session
  completion. It owns no command-line parsing, runtime composition, admission policy, provider
  adapter, or credential custody.
- `connectors-runtime` is the reusable composition package. It validates deployment configuration,
  constructs the transport, credential-source, and focused Integration adapters, and installs an
  exact dispatch registry. The product binary delegates `serve` to this package instead of being a
  second composition root.
- Each Integration family is a focused adapter package (for example Slack, monitoring, Kubernetes,
  or SIP). It implements `service::ConnectorBackend` and translates only its owned provider or
  protocol family. Adapter implementation modules do not live in `connectors-cli`, `server`, or
  the composition root.
- `connectors-cli` is a thin clap-and-presentation frontend. It may map typed arguments to protocol
  DTOs and render typed results; it does not implement backends, supervise services, load Vault,
  initialize Kubernetes or voice transports, or hold business rules. Its direct runtime edges are
  the reusable client and runtime packages, never `server`, `service`, voice drivers, secret-store
  implementations, or Integration adapters.

Dispatch is exact, not an ordered search. `service::ConnectorBackend` exposes explicit
`owns_operation`, `owns_connection`, and `owns_event` claims, and on startup the runtime constructs a
`BackendRegistry` from configured Integration identity and the immutable Connection/provider
ownership it loads. Discovery/search requests may deliberately fan out and deterministically merge
results. A Describe may merge compatible contributions when multiple route adapters deliberately
expose the same provider operation; incompatible contributions refuse as a protocol error. Every
other targeted request (`operation_ref` plus `connection_ref`, `integration_ref`, `candidate_ref`,
`observation_ref`, `connect_session_ref`, `channel_ref`, or `execution_ref`) resolves its
method-specific ownership claim to exactly one registered adapter before invocation. Zero matches is
`not_found`; more than one is a protocol/configuration error; an adapter's `not_found` never means
“try the next backend.” The selected Connection, catalog operation provider, and registered adapter
identity must agree before credential placement. This prevents backend ordering from becoming an
authorization or routing mechanism.

Credential and session ownership remains split along protocol authority. Only credential-source
implementations inside the runtime custody vendor credentials; CLI arguments, client DTOs,
transport servers, Integration metadata, and logs remain value-free. During local one-use
completion, the CLI may read hidden terminal input and `connectors-client` may carry those bytes to
the validated owner-only completion endpoint; neither persists, interprets, logs, or reuses them. A
connector **Connect Session** is a short-lived flow for creating or repairing a Connection, while an
operation's `execution_ref` identifies a direct byte/session operation. Neither is an Identity login
session, and Connectors never accepts, stores, refreshes, or exposes Identity login-session cookies
or service credentials. Hosted transport accepts only the released exact-audience validated
authority described by the identity-boundary amendment.

The nested workspaces are intentional architectural isolation, not Cargo-layout accidents.
`connectors-runtime` owns the feature-unified adapter/composition graph; `connectors-cli` owns the
release binary closure; `voice-runtime` is the sole SIP/RTVBP composition leaf; `driver-sip` owns the
reviewed `sipx` socket closure; and `rtvbp-voice-endpoint` owns RTVBP's
`serde_json/preserve_order` closure. Each has an independently reviewed lockfile and a one-way edge
toward the narrower packages it composes. None is admitted to the canonical catalog workspace,
whose feature set and artifact bytes must remain unchanged by runtime dependencies. The dependency
and module-size fences in `catalog-build` mechanically assert these boundaries, including the
1,500-line module cap and every temporary named waiver.

**2026-09-02 outbound-MCP amendment.** A remote MCP server is one more provider-shaped service,
not a second runtime or admission path. `integration-mcp` consumes the shared, exactly pinned MCP
client foundation through its injected Streamable HTTP boundary; every exchange still crosses
`service::EgressTransport`, and every bearer is fetched through `SecretStore` for that exchange.
The server's complete tool snapshot is immutable reviewed input. A local mapping owns operation
identity, prose and effect, while the existing `ServiceDeployment` owns permanent provider
identity, exposure, risk, approval, Grants and opaque endpoint/credential bindings. MCP annotations
are retained in the snapshot but grant nothing. The detailed contract is
[design 18](18-governed-outbound-mcp-services.md).

## 3. Postures are configuration, not builds

One config document (`platform.toml`), fail-closed (unknown field = refusal by name):

```toml
posture = "personal" | "org" | "saas"

[identity_verifier] # org/saas: pinned Identity owner bundle, audience, issuer/trust roots;
                    # personal: absent
[tenant_binding]    # org/saas: receiver-configured expected tenant/trust domain/deployment;
                    # never request selected and not an Organization record
[storage]         # state root; refuses working-tree paths
[catalog]         # pack path override (default: embedded), later: additional sources
[egress]          # org: the deployment-declared destination allowlist (value-free)
[[credential_sources]] # closed driver + allowed placement/scope; values are forbidden
```

Same binary, same connector feature set. A posture selects local authentication versus the Identity
verifier, the fixed tenant binding, and bind policy. A hosted listener refuses startup when its
verifier contract, trust roots, expected audience/tenant, or connected revocation posture is absent
or invalid. Upstream OIDC issuer/client configuration and Identity login-session storage never
enter this document or process; hosted requests carry only exact-audience short-lived access
authority.

Credential-source configuration is Connectors-owned even when `platform.toml` is embedded as a
subtree of a wider product configuration. It declares only closed backend drivers and non-secret
placement/scope policy. Integration policy narrows which named sources may establish a Connection;
the harness, web product, task, and invocation cannot supply or override a backend. The exact shape,
including external-provider references and satellite generation binding, is defined by
[Design 07](07-credential-custody-topologies.md#7-deployment-configuration-owns-available-stores).

## 4. Storage

- **Relational state** — the stable admitted tenant/principal references required for receiver-owned
  records, integrations, connection registry, grants (CAS-revisioned), channels, events,
  deliveries, and connector audit. Personal placement uses the owner-only local state backends;
  hosted placement requires one service-owned **PostgreSQL database**. There is no
  Organization, membership, Identity login-session, upstream-token, service-principal credential,
  or reusable service-bearer verifier store. WAL mode, one writer, migrations embedded.
- **Credentials** — never in the database. `connector-secrets` owner-bound file store (personal/
  org), envelope-encrypted per-tenant for saas later; the port stays, the backend swaps.
- **Catalog** — the pack, embedded in the binary and overridable by verified `Pack::load`; a
  pack that fails verification refuses startup.

**2026-08-14 credential-custody amendment.** The owner-bound file store is a development backend,
not the release default. Personal release posture binds `connector-secrets` to the OS keychain or
an operator-selected external secret provider. Organization and satellite placements bind it to a
deployment-managed encrypted store, an external secret provider, or a workload identity that needs
no reusable vendor secret. [Design 07](07-credential-custody-topologies.md) defines how one-use
completion reaches the execution placement without exposing plaintext to a caller or central relay.

**2026-08-15 hosted binding.** The first organization deployment binds the existing
`SecretStore` port to Vault KV v2. The hosted process authenticates over internal TLS with its
projected Kubernetes identity, retains only a short-lived Vault token in memory, refreshes it before
expiry, and fails startup if the store is unusable. The default `TenantLayout` keeps every
`CredentialRef` below `tenants/<tenant>/`; Vault policy independently restricts the workload to the
installation tenant. This is the managed-store topology, not a model-visible Vault Integration or
a generic secret-reading operation. The first hosted Kubernetes Integration itself continues to
use workload identity and therefore does not manufacture a reusable credential merely because a
store is present.

Hosted liveness remains process-local. Hosted readiness additionally requires the Identity
verifier and every configured backend's mandatory dependencies. A credential-backed Integration
therefore checks its `SecretStore` through a value-free readiness port; Vault proves service health
and a usable workload-authenticated session without reading a credential address or value. An
upstream provider outage degrades that Integration and is not, by itself, global process
readiness. Both `ConnectorBackend` and `SecretStore` require every implementation to state its
readiness posture explicitly; neither port supplies a fail-open success default.

**2026-08-15 hosted-consumer correction.** The Vault capability and its deployment configuration
are implemented, but the current hosted adapter set contains no provider-credential consumer:
Kubernetes status uses workload identity and SIP resolves operation-scoped deployment material.
`connectors-runtime` therefore refuses `vault.enabled = true` instead of initializing and parking
an inert store. The binding described above becomes active only when a configured hosted
Integration explicitly receives and consumes the `SecretStore` capability; readiness may not
advertise it before then.

**2026-08-15 SIP credential activation.** Hosted SIP digest authentication now consumes that
capability. Configuration contains only the authority, service, and distinct username/password
credential leaf names. The runtime constructs tenant-pinned `CredentialRef`s, initializes Vault
before readiness, and injects the resulting `CredentialSource` into the SIP session launcher;
unauthenticated SIP remains possible only when both the credential binding and Vault are disabled.

The predecessor scattered connector state across seven owner-only JSON files plus two SQLite
databases — each individually justified, collectively unqueryable. One connector state authority +
one credential-store port + one pack is the whole Connectors-owned inventory here; the hosted
authority is PostgreSQL and the credential port may retain sealed values itself or resolve an
opaque external-provider binding. Identity persistence is not another store hidden in this
process. PostgreSQL adoption does not itself claim SaaS isolation, HA, or restore readiness.

## 5. The one invocation path

```text
presented authority ─▶ personal-local auth OR Identity verifier
      ─▶ admitted principal (tenant inside; exact Connectors audience scopes)
      ─▶ connector Grant admission (proof types; deny>allow>predicate)
      ─▶ Connection resolution (Grant names the Connection, never the credential)
      ─▶ connector-resolve: document ─▶ RequestPlan {request, subjects, redactions}
      ─▶ credential placement (subjects computed BEFORE placement)
      ─▶ egress (destination policy; the only module that dials)
      ─▶ connector audit (closed vocabulary)
```

Identity scopes and connector Grants remain distinct. The closed scope strings are owned by
[Design 01](01-domain-model.md#grant); no token claim is proof of a receiver-owned Connection or
Grant. A future first-party substrate provider uses a separate substrate-audience authority when it
calls substrate. Its owner-defined scopes are exactly `observe`, `workspaces`, and `exec`;
Connectors does not rename `exec` to `execs`, alias any of those terms into a Connectors scope, or
treat connector admission as substrate admission.

Structural rules, each with a fence or a type making it non-optional:
- `crates/server`'s egress module is the **only** place a vendor socket is opened; a dependency
  fence classifies every crate as network/no-network and fails on drift.
- **Post-DNS egress amendment (2026-08-17):** `service` owns one transport-neutral egress port;
  released HTTP integrations receive that capability from `connectors-runtime` and cannot depend
  on `server`. The sole `server::egress` implementation matches deployment-owned scheme, host, and
  port before resolution; rejects an entire empty, mixed, local, reserved, or out-of-scope DNS
  answer; disables redirects and ambient proxies; bounds selected response headers and bodies; and
  pins HTTP and WebSocket sockets to the admitted answer while retaining the hostname for TLS.
  Slack admits only public `slack.com` HTTP and Slack-owned Socket Mode hosts. GitLab and Grafana
  each admit one exact operator-selected origin, including a private routed address but never
  process-local, link-local, multicast, or documentation space. The exhaustive gate scans every
  released HTTP integration for a direct `server` dependency and raw HTTP client, DNS, TCP, or
  WebSocket calls, and proves `server` remains the physical port implementation. Hosted
  configuration and Helm must
  carry `connection_bound_post_dns_v1`; omission fails closed. This releases Slack, GitLab, and
  Grafana transport. SIP, Jira, browser navigation, and effect-bearing hosted operations retain
  their independent fences.
- **Native-voice amendment (2026-08-14):** the selected `sipx-transport` API owns its sockets and
  performs its own `bind(Config)`. `server`/`service` remains the only admission, destination-policy,
  credential and composition path, but the closed `driver-sip` crate is the one named exception
  allowed to perform physical SIP/RTP binds from a non-serializable admitted plan. Every configured,
  resolved or protocol-learned target is checked against that plan; the fence rejects any other
  network-capable driver or direct `sipx` bind.
- Generic v1 has no raw proxy. If S-030 later enables operator-only break-glass access, it rides the
  same path with destructive/max-effect facts plus separate method/path and destination apertures;
  it is never model-exposed or admitted by an ordinary catalog grant.
- No runtime parsing of any source form, ever — the plan is derived from document data only.

The invocation path is protocol-neutral at the admission boundary. Per
[Design 03](03-beyond-http.md), the canonical document fixes interaction shape, protocol driver and
required capabilities; deployment policy resolves placement. HTTP egress is the first registry
binding. A missing driver or capability refuses before credential access and never falls back to
HTTP, Flux, an ambient executable or another placement.

## 6. Eventing

- **Channel supervisor** (server): owns provider-side transports per the catalog's channel
  bindings; opaque host-minted ids; restart-safe.
- **Webhook terminator**: one inbound endpoint; per-provider verification + attribution from
  declarative catalog rules (the new grammar — designed against the five existing bindings and
  the Nango corpus before generalizing; no script escape hatch in v1).
- **Event store**: append-only durable state (local backend in personal placement, PostgreSQL in
  hosted placement), provenance (`native`/`polled`) on every row, dedup by
  delivery id, the data/operational family split from design 01.
- **Deliveries**: durable per-endpoint queues; Svix envelope (`id`, `timestamp`, HMAC over
  `{id}.{timestamp}.{body}`, dedicated key); retries with backoff; **replay-by-id API**.
- **Subscriptions**: one authenticated WS per client, multiplexed, gated by inbound grants.

**2026-08-14 substrate-ingestion amendment.** The substrate adapter supervises one Channel per
`(Connection, source_scope)` and commits the native identity
`(deployment, source_scope, generation, seq)` with its delivery/high-water update. It bootstraps by
creating and completely consuming a stable snapshot, then resumes from the snapshot's opaque
inclusive-barrier cursor. Retention, source-scope, and generation mismatches share one
non-oracular gap posture. Snapshot “complete” means a quota-bounded complete current set for its
current workspace and exec kinds; operation-ledger rows and deletion-tombstone tables are excluded,
its event-provenance window is separately bounded and may be truncated, and an empty current set is
valid. No-cursor pull is diagnostic, not a durable bootstrap shortcut.

The tuple's `deployment` is never event-selected. It comes from the Connection's authenticated,
out-of-band substrate peer binding. Any deployment assertion carried by a frame, page, or snapshot
must match that binding exactly. Mismatch refuses before deduplication, delivery creation, or
high-water advancement, leaves the Channel degraded, and requires authenticated operator rebind;
it is not routed through ordinary gap recovery.

## 7. Fence and test regime (carried as mechanism)

1. **Dependency fence** — every workspace member classified (catalog / platform / network);
   an unclassified member fails the build's test run.
2. **Determinism** — two independent builds produce byte-identical canonical documents and pack.
3. **Routes enumeration** — the published HTTP surface compared against a hand-declared list
   with an argument per entry; `Access` is on the route, not in the handler.
4. **Proof-type chain compile checks** — gate-skipping is unrepresentable; where the predecessor
   used source scanners, we use types and delete the scanner generation entirely.
5. **Protocol conformance** — positive + adversarial fixtures per protocol identity, shared
   verbatim with future SDK test suites.
6. **One-time migration differential** — during M1, our pack vs the predecessor's pack at the
   same inputs, byte-identical; retired once the catalog builds green here.
7. **MSRV** — resolver v3 plus the predecessor's msrv fence (declared-vs-tested gap named, not
   implied).

## 8. Releases and automation

Pre-v1 there are no release artifacts; the repo is the product. When releases start: a Linux
server/CLI binary, the pack as both embedded default and standalone asset, and a signed release
manifest under a single-owner trust bootstrap (the 0011 pattern), with the update channel
arriving when flux's managed personal posture needs it — not before.

Organization delivery identity and credentials are centrally managed outside component source.
This repository carries the release workflow and verification contract, but no token-minting or
credential-path machinery.

## 9. Build order

| Milestone | Content | Exit |
|---|---|---|
| **M1 catalog** | copy catalog dirs + family crates; `catalog-build` extracted minus emitters; schema gains C-552 fields + per-op effects; lock verifier | `catalog build/diff/check` green; one-time pack differential vs predecessor passes |
| **M2 skeleton** | `domain`/`protocol`/`service`/`server` scaffolds; personal-local authentication; hosted Identity verifier port; fixed tenant/principal projection; closed connector audience scopes, Grants, and connector audit; no Identity-owned store | personal posture is healthy; hosted conformance passes the pinned Identity owner bundle; routes/dependency fences prove Identity implementation and persistence stay absent |
| **M3 connections** | integrations, connect sessions, acquisition (OAuth + API key), connections lifecycle, grants, declared-operation invoke; raw proxy remains deferred to S-030 | end-to-end: admit local or Identity authority → connect a real provider → grant → invoke, all audited |
| **M4 events** | channels, webhook terminator, event store, deliveries + replay, subscriptions | a provider event reaches a client by push and by pull, with provenance |
| **M5 clients** | externally gated until Flux records platform adoption; then flux re-point (embedded client + local supervise) and the first measured plugin-retirement wave (gitlab), as recorded in S-010 | downstream adoption record exists; flux invokes gitlab through the platform; the gitlab plugin is deleted |

## Open questions

1. Binary/product naming: `connectors` (as here) vs a shorter brand (`sdc`). Cheap to change
   until M2, expensive after.
2. Whether `web/` (public explorer) stays in-repo or becomes the org's site repo once the
   catalog is public.
3. Console timing and shape (operator SPA served by the host, per the predecessor's
   same-origin lesson) — after M3 at the earliest.
4. Encryption-at-rest and tenant-isolation evidence for the hosted PostgreSQL posture — decide
   with the SaaS design while keeping all secret material out of the database.
