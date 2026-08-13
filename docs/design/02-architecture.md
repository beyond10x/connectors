# Design 02: architecture

**Status:** draft for review · **Date:** 2026-08-13
**Inputs:** [01-domain-model.md](01-domain-model.md) ·
[../research/catalog-precedents.md](../research/catalog-precedents.md) · the measured predecessor
inventory (what migrates wholesale, what is redesigned, what is left behind).

This document fixes the physical shape: repository layout, crate boundaries, storage, the one
invocation path, the fence regime, and the build order. Wire schemas and endpoint lists come
later and cite this.

## 1. Repository layout

```
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
overlays. An official machine-readable spec is preferred; where none exists, the repository owns
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
| `crates/server` | composition: axum transport with routes-as-data + `Access` on the route, posture-selected identity (local owner / OIDC / hosted), SQLite + secret-store bindings, the egress module, channel supervisor, WS subscriptions, the binary's serve path. |

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
| `connectors` | `crates/connectors-cli` — the product, arrives with M2 | end users and operators, against any deployment and entirely without flux: `connectors serve`, `connectors admin org \| user \| integration \| grant \| …`, and the client verbs (login, connect, invoke, events) |

The maintenance tool links the compiler family (`connector-spec` ingest, site projection,
`catalog-build`); the product CLI links `protocol`/`service`/`server` and **never the
compiler**.

Both binaries parse their command line with **clap (derive API)** — hand-rolled argv parsing is
banned in this repository. The predecessor hand-rolled its connector CLI to avoid a dependency
mid-flight and it went sideways; the parser is not where this project spends its novelty budget.

`connectors serve` with no config is the **personal posture**: loopback bind, local-owner
identity, one implicit organization, state under an owner-only user-state root. Zero
configuration is the personal tier's contract — it is what a supervising client (flux) starts.

## 3. Postures are configuration, not builds

One config document (`platform.toml`), fail-closed (unknown field = refusal by name):

```toml
posture = "personal" | "org" | "saas"

[identity]        # org: OIDC issuer, client, hosted-domain claim; personal: absent
[organization]    # org: the one tenant's name; operator subject allowlist
[storage]         # state root; refuses working-tree paths
[catalog]         # pack path override (default: embedded), later: additional sources
[egress]          # org: the deployment-declared destination allowlist (value-free)
```

Same binary, same features, every posture. A posture only selects the identity chain, tenancy
mode, and bind policy (loopback-only while no real identity is armed — carried refusal).

## 4. Storage

- **Relational state** — organizations, users, service-account verifiers, integrations,
  connection registry, grants (CAS-revisioned), channels, events, deliveries, audit — in **one
  SQLite database** under the state root. WAL mode, one writer, migrations embedded.
- **Credentials** — never in the database. `connector-secrets` owner-bound file store (personal/
  org), envelope-encrypted per-org for saas later; the port stays, the backend swaps.
- **Catalog** — the pack, embedded in the binary and overridable by verified `Pack::load`; a
  pack that fails verification refuses startup.

The predecessor scattered state across seven owner-only JSON files plus two SQLite databases —
each individually justified, collectively unqueryable. One database + one secret store + one
pack is the whole inventory here. SaaS-scale Postgres is a port swap decided when saas is real,
not before.

## 5. The one invocation path

```
token ─▶ principal (org inside) ─▶ effective catalogue (sealed generation)
      ─▶ grant admission (proof types; deny>allow>predicate)
      ─▶ connection resolution (grant names the connection, never the credential)
      ─▶ connector-resolve: document ─▶ RequestPlan {request, subjects, redactions}
      ─▶ credential placement (subjects computed BEFORE placement)
      ─▶ egress (destination policy; the only module that dials)
      ─▶ audit (closed vocabulary)
```

Structural rules, each with a fence or a type making it non-optional:
- `crates/server`'s egress module is the **only** place a vendor socket is opened; a dependency
  fence classifies every crate as network/no-network and fails on drift.
- The proxy rides the same path with fixed worst-case facts; it is a granted capability, not a
  bypass.
- No runtime parsing of any source form, ever — the plan is derived from document data only.

## 6. Eventing

- **Channel supervisor** (server): owns provider-side transports per the catalog's channel
  bindings; opaque host-minted ids; restart-safe.
- **Webhook terminator**: one inbound endpoint; per-provider verification + attribution from
  declarative catalog rules (the new grammar — designed against the five existing bindings and
  the Nango corpus before generalizing; no script escape hatch in v1).
- **Event store**: SQLite, append-only, provenance (`native`/`polled`) on every row, dedup by
  delivery id, the data/operational family split from design 01.
- **Deliveries**: durable per-endpoint queues; Svix envelope (`id`, `timestamp`, HMAC over
  `{id}.{timestamp}.{body}`, dedicated key); retries with backoff; **replay-by-id API**.
- **Subscriptions**: one authenticated WS per client, multiplexed, gated by inbound grants.

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

## 8. Releases and automation identity

Pre-v1 there are no release artifacts; the repo is the product. When releases start: a Linux
server/CLI binary, the pack as both embedded default and standalone asset, and a signed release
manifest under a single-owner trust bootstrap (the 0011 pattern), with the update channel
arriving when flux's managed personal posture needs it — not before.

Automation (catalog rebuild commits, lock bumps, release cuts) authenticates as an
**org-owned GitHub App** (`b10x-bot`) minting short-lived installation tokens — never a
personal account, never a long-lived PAT. In-workflow commits may alternatively use the Actions
`GITHUB_TOKEN` only for read-only workflow operations. Every automated commit and push uses
`b10x-bot[bot]`. Human pushes stay human.

## 9. Build order

| Milestone | Content | Exit |
|---|---|---|
| **M1 catalog** | copy catalog dirs + family crates; `catalog-build` extracted minus emitters; schema gains C-552 fields + per-op effects; lock verifier | `catalog build/diff/check` green; one-time pack differential vs predecessor passes |
| **M2 skeleton** | `domain`/`protocol`/`service`/`server` scaffolds; postures + identity (personal, org-OIDC); organizations, service accounts, audit | `connectors serve` healthy in both postures; routes fence green |
| **M3 connections** | integrations, connect sessions, acquisition (OAuth + API key), connections lifecycle, grants, invoke + proxy | end-to-end: sign in → connect a real provider → grant → invoke, all audited |
| **M4 events** | channels, webhook terminator, event store, deliveries + replay, subscriptions | a provider event reaches a client by push and by pull, with provenance |
| **M5 clients** | flux re-point (embedded client + local supervise), plugin-retirement wave 1 (gitlab) per flux-roadmap 0024 | flux invokes gitlab through the platform; the gitlab plugin deleted |

## Open questions

1. Binary/product naming: `connectors` (as here) vs a shorter brand (`sdc`). Cheap to change
   until M2, expensive after.
2. Whether `web/` (public explorer) stays in-repo or becomes the org's site repo once the
   catalog is public.
3. Console timing and shape (operator SPA served by the host, per the predecessor's
   same-origin lesson) — after M3 at the earliest.
4. SQLite encryption-at-rest for the saas posture (per-org envelope keys) — decide with the
   saas design, shaped for by keeping all secret material out of the database now.
