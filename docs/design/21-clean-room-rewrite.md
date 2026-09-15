# 21 — Clean-room rewrite by spec extraction

**Status: accepted 2026-09-15. Drafted 2026-09-08; accepted by the operator through the org-state
review decision sheet of 2026-09-15, items 1 and 15. The body below is the draft as written, except that two
upstream-repository references are redacted under atlas ADR 0001 and §3's consumer table is
corrected in place by the dated note it carries (2026-09-15); only §7 has been answered.** The lineage the acceptance rests on is recorded in the atlas ADR 0051
(2026-09-15).

The operator's judgment: this repository is heavily over-engineered. The proposal: extract one
authoritative specification of the semantics worth keeping, with the bad parts named and excluded,
then implement that specification clean-room in a fresh repository — the same move this repository
itself made against `the former-org upstream repository named in `AGENTS.md` § Boundaries/flux-connectors` (AGENTS.md "Boundaries"), now applied to itself.

## 1. What is here (measured 2026-09-08)

| Measure | Value | Source |
|---|---|---|
| Repository age | first commit 2026-08-13, 387 commits | `git log` |
| Tracked Rust | 186,207 lines (132,182 src, 54,025 test-path) | `wc -l` over `git ls-files '*.rs'` |
| Crates | 44 | `ls crates/` |
| Cargo workspaces | 12, ~41 GB of target dirs between them | `gate.sh --list-workspaces`; AGENTS.md gate section |
| Design docs | 23 files, 4,210 lines (numbering already collides: two 09s, two 13s, two 19s) | `docs/design/` |
| All docs | 11,580 markdown lines | `git ls-files 'docs/*.md'` |
| Planning store | 163 files in `.engineering/planning`, 74 more in `docs/stories` | `git ls-files` |
| ESS spec | 2,083 yaml lines, 7 domains, 62 `UNMAPPED` gap markers | `ess/` |
| Provider specs | 19 vendors with provenance under `specs/` | `ls specs/` |
| Fence-mentioning test/src files | 46 | grep |

Two structural observations that drive the plan:

1. **The over-engineering is more structural than scoped.** The strippable capability families
   (§4) total roughly 25k of 186k lines. The remaining weight is layering: 44 crates where ~10
   would do, a 32k-line CLI crate, a 38k-line spec-ingest crate, proof-type ceremony on every
   internal boundary, and a fence/meta-test regime (46 files) that guards the structure rather
   than the behavior.
2. **There are already four overlapping specification layers** — the design-doc series, the
   partial ESS model, `contracts/` fixtures, and the generated artifacts (catalog pack/lock,
   served OpenAPI, generated clap tree). None is complete and none is authoritative
   (`docs/architecture/specification.md` says so explicitly). The extraction phase is therefore
   largely *consolidation into one authority*, not writing from nothing.

## 2. The semantics to preserve (the keep list)

The product, per `docs/design/01-domain-model.md`, is the three-layer shape — and that shape is
sound; the rewrite keeps it:

```
CATALOG    Provider (versioned, deterministic, provenance-pinned)
DEPLOYMENT Integration (a tenant enables + configures a provider)
RUNTIME    Connection · Grant · Invocation · Event · Audit
```

| # | Capability | Current evidence | Spec artifact it becomes |
|---|---|---|---|
| K1 | Catalog pipeline: vendor/authored spec + patch rules → canonical document → pack → lock, byte-deterministic, provenance-pinned, expose-curated | `connector-spec`, `catalog-build`, AGENTS.md "Adding a connector" | Pipeline spec + golden fixtures (input spec → expected pack/lock bytes) |
| K2 | Domain nouns, lifecycles, closed vocabularies (risk, effects, idempotency, audit actions) | `crates/domain`, design 01 | Completed ESS core (resolve the 62 UNMAPPED, add typed relations) |
| K3 | Authority: admitted principal, Grant evaluation (deny > allow > predicate), approval issuance / exact-input binding / one-time redemption | `crates/domain`, `crates/service`, design 13 | Decision tables as executable fixtures + prose contract |
| K4 | Invocation path: document → RequestPlan → credential placement (subjects before placement) → egress → audit | design 02 §5, `connector-resolve` | RequestPlan golden fixtures + path contract |
| K5 | Egress policy: post-DNS pinning, scheme/host/port match, redirect/proxy refusal, per-provider origin rules | design 02 §5 amendment 2026-08-17 | Decision table (input → admit/refuse) |
| K6 | Eventing: channel supervision, intake, ack/replay, delivery/subscription | design 02 §6, `crates/server` | Event-flow contract + fixtures |
| K7 | Credential custody: owner-bound secret store, requirements-not-values in catalog | `connector-secrets`, design 07 | Custody contract |
| K8 | Wire protocol + HTTP surface | `crates/protocol`, served `openapi.json` | The served OpenAPI document, frozen as fixture |
| K9 | CLI surface (`serve`, `connect`, `invoke`, `events`, admin verbs) | `connectors-cli`, ESS clap tree | ESS-generated surface + verb inventory |
| K10 | Postures: personal-local and hosted as configuration, not builds | design 02 §3 | Deployment contract |
| K11 | Shipped integrations: Slack, GitLab, Jira/Confluence, Kubernetes, Grafana/monitoring, catalog, platform | `crates/integration-*` | One behavior spec per provider, evidence-cited |

Data that survives verbatim, no rewrite: `specs/` (19 vendors + provenance), `providers/*.toml`,
`SOURCES.toml`. They are already the spec-driven, deterministic part of the system.

## 3. The consumers the rewrite must not break

**Corrected 2026-09-15** (org-state review run 2, lane 08 F2/F3/F8; ledger ORG-0082, ORG-0083,
ORG-0088). As drafted on 2026-09-08 this section listed four manifests in four repositories and
called the cutover "a coordinated repin of these four manifests". At `origin/main` of each consumer
there are **nine manifests in five repositories**, and three of the four pins the draft did list had
moved. The table below is re-derived from
`git -C <repo> grep -nE 'beyond10x/connectors(\.git)?"' origin/main -- '*Cargo.toml'`, run over the
five repositories on 2026-09-15; the draft rows it replaces are named under it.

| Consumer | Manifest (at that repository's `origin/main`) | Depends on | Pin |
|---|---|---|---|
| zwirn | `zwirn/Cargo.toml:25` | `connectors-cli` | rev `1e0eb9f` (ssh remote) |
| devcenter | `devcenter/Cargo.toml:37-38` | `connectors-client`, `protocol` | rev `e80b7ae1`, `version = "=0.7.0"` |
| devcenter | `devcenter/crates/devcenter-connectors/Cargo.toml:22-23`, with a `[patch."https://github.com/beyond10x/connectors.git"]` block at `:38-40` redirecting `protocol` and `service` | `connectors-runtime`, `service` | rev `097b1c58` (ssh remote) |
| service-sdk | `service-sdk/crates/service-catalog/Cargo.toml:20-21` | `protocol`, `service` | rev `9f2a361b` |
| service-sdk | `service-sdk/crates/service-conformance/Cargo.toml:23-24` | `protocol`, `service` | rev `9f2a361b` |
| service-sdk | `service-sdk/crates/service-connectors/Cargo.toml:25-26` | `protocol`, `service` | rev `9f2a361b` |
| service-sdk | `service-sdk/crates/service-host/Cargo.toml:33-34` | `protocol`, `service` | rev `9f2a361b` |
| agent-platform | `agent-platform/Cargo.toml:48` | `connectors-client` | tag `v0.5.6` |
| workspace | `workspace/Cargo.toml:21` | `connectors-client` | rev `dbdd285c`, `version = "=0.6.4"` |

Against the 2026-09-08 draft: service-sdk (four manifests at rev `9f2a361b`) and devcenter's second
manifest (`crates/devcenter-connectors/Cargo.toml`, rev `097b1c58`, plus its `[patch]` block) were
not listed at all; agent-platform is `tag = "v0.5.6"` at `Cargo.toml:48`, not `v0.3.1` at `:44`; and
workspace is `rev = "dbdd285c"`, `version = "=0.6.4"`, not `b4f6d655`. Only zwirn and
`devcenter/Cargo.toml` are unchanged.

zwirn is the one row whose pin cannot be fetched from this repository. `1e0eb9f` (2026-08-24) is
reachable from no ref here and survives only as an unreachable object on GitHub, which
[`AGENTS.md`](../../AGENTS.md) § Boundaries (`AGENTS.md:362-364`) names as the expected state until
GitHub garbage-collects — so the cutover has to repin `zwirn/Cargo.toml:25` to a commit on
`origin/main`, and has to do it before the pre-public garbage collection that paragraph asks for.

`connectors-client`, `protocol`, `service`, `connectors-runtime` and `connectors-cli` keep their
names and contract shape in the rewrite; the cutover (§6 phase 5) is a coordinated repin of these
nine manifests, and the `[patch]` block in `devcenter/crates/devcenter-connectors/Cargo.toml` moves
with them.

## 4. The strip list (proposed defaults; each is a named exclusion in the spec, re-addable later)

| Item | Crates / designs | ~LOC | Proposed disposition |
|---|---|---|---|
| Voice / SIP / RTVBP stack | `rtvbp-voice-endpoint`, `voice-runtime`, `voice-local-audio`, `driver-sip`, `driver-audio`, `driver-speech`, `integration-sip`; designs 05, 15 | 10,717 | **Relocate** to its own repository — it is a different product with a socket-owning driver exception carved through this one's fences (design 02 §5 native-voice amendment). Not deleted: operator decides (§7 D1) |
| Browser + SQL drivers | `driver-cdp`, `driver-sql` | 4,292 | Drop from v1 spec |
| Console | `connectors-console` | 5,310 | Drop from v1 spec |
| Subscription custody + leases + hosted vault family | `subscription-custody`, `hosted-vault`, `hosted-secrets`, `hosted-state`; designs 16, 17 | 3,366 | Drop — design 01 already lists leases as "designed in the predecessor, never used in anger" |
| Outbound MCP governance, MCP transport | `integration-mcp`; designs 14, 18 | 1,188 | Drop from v1 spec |
| Git fetch sessions | designs 19, 20; git ESS domain | — | Drop from v1 spec |
| 12-workspace split | satellite `Cargo.lock`s, 41 GB target, sharded CI | — | One workspace, ~10 crates (§5) |
| Fence/meta regime | 46 fence files, per-boundary proof-type ceremony | — | Keep three checks with teeth: catalog byte-determinism + lock verify, egress-is-the-only-dialer, no-credential-values. The rest is not re-derived |
| Spec-layer quadruplication | design docs + ESS + contracts/ + generated | — | One authority (§5); design docs shrink to decisions, not restated behavior |
| Planning/story sediment | 163 + 74 files | — | Not migrated; fresh store in the new repo seeded from the extraction epic only |

## 5. Target shape

One workspace, ~10 crates, one specification authority:

- **Spec authority**: completed ESS core (nouns, lifecycles, commands, events, relations) +
  per-subsystem behavior contracts (markdown, each claim citing old-repo `file:line` evidence) +
  executable conformance fixtures (golden bytes and decision tables). ESS limits are known
  (`docs/architecture/specification.md`); what ESS cannot express lives in the fixtures, not prose.
- **Crates**: `catalog` (lib: spec ingest + build + read, merging today's 6 catalog-family crates),
  `catalog-cli` (bin), `domain`, `service` (absorbing `connector-resolve`, `connector-oauth`,
  `connector-state`), `server`, `secrets`, `protocol`, `client`, `cli`, `integrations` (one crate,
  module per provider).
- **Clean-room rule**: the spec may cite old code; the new implementation is written from the spec
  and fixtures only. Verbatim reuse is allowed for *data* (`specs/`, `providers/`, `SOURCES.toml`)
  and nothing else. This repository becomes a read-only predecessor reference, exactly the status
  `the former-org upstream repository named in `AGENTS.md` § Boundaries/flux-connectors` holds here today.

## 6. Phases

| Phase | Work | Gate to pass |
|---|---|---|
| 0 | Operator decides §7; extraction epic drafted in AEP | Decisions recorded |
| 1 | Spec extraction, ~11 read-only stories, one per K-row of §2 (K11 splits per provider). Each story delivers: contract doc with evidence citations + conformance fixtures + ESS additions | Every story's claims cite `file:line` or a fixture; ESS validates |
| 2 | Adequacy gate: mechanically enumerate every HTTP route, CLI verb, protocol message, and event kind in the current code; each item is either cited by a spec or named on the strip list — no third state. AEP critic pass over the spec set | Zero unclassified surface items |
| 3 | Clean-room implementation in the new repo, waves of ≤5 parallel stories (AGENTS.md dispatching rule), each story owning one crate/module against its fixtures | Per-story: fixtures green |
| 4 | Parity: catalog build byte-parity old-vs-new on all 19 providers; grant-decision and RequestPlan fixture parity; the Slack mention→reply walkthrough end-to-end on personal-local | All parity checks green |
| 5 | Cutover: repin the 4 consumers (§3), release, archive this repo as predecessor | Consumers' gates green |

Phase 1 is where the leverage is: it is read-only, parallelizable, and cheap to review — and a
wrong strip decision surfaces there as a missing spec, not as a half-built runtime.

## 7. Decisions for the operator — answered 2026-09-15

Answered by the org-state review decision sheet of 2026-09-15 (items 1 and 15), except D1, which
that sheet does not reach and which stays **open**.

| # | Fork | Default on silence | Answer, 2026-09-15 |
|---|---|---|---|
| D1 | Voice/SIP/RTVBP: relocate to own repo, or keep in the rewrite scope, or delete | Relocate (kept buildable, out of connectors' spec) | **Open.** Item 15 accepts this document but names no destination, and no sibling checkout under `~/beyond10x` carries a voice, SIP or RTVBP repository (checked 2026-09-15). Out of the rewrite's scope under either remaining branch: the successor "does not advertise … media sessions" (`connectors_v2/README.md`). `epic:native-voice` is archived in this store on that exclusion; where the code lands is still the operator's call, and archiving is not deleting. |
| D2 | New repository vs in-place `v2` | New repository; this one becomes the read-only predecessor | **Answered — new repository, and it takes this one's name.** Sheet item 1 option A: the `beyond10x/connectors` name belongs to the successor lineage; this repository is the predecessor. Verified 2026-09-15: `git ls-remote --symref origin HEAD` → `ref: refs/heads/next`; the remote tag namespace carries v0.2.0–v0.7.2 (this line) and v0.8.0–v0.11.0 (the successor). v1's last tag is v0.7.2; `main` here is fast-forwarded to `origin/main` (eaa81228) and the 9 unpublished v1 commits are parked on local branch `v1-bounded-reads-20260906` (tip 81459ac4). |
| D3 | Strip-list confirmation for: cdp/sql drivers, console, custody/leases, MCP, git-fetch | Strip all six, each as a named exclusion in the spec | **Answered — confirmed as proposed**, by item 15 accepting this document with §4 unchanged. One note for phase 1: the successor has since shipped a PostgreSQL adapter (`schema.list`, `query.read`; `connectors_v2/README.md`), so §4's "drop" binds *this* repository's v1 spec and says nothing about what the successor may carry later. |

The epics this repository's planning store archives on that acceptance, and why, are recorded on
each epic under its `## Disposition` heading (`aep plan artifact list --kind epic --status archived`).
