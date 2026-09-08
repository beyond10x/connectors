---
format: aep.planning-md/1
id: review-result:concept-stack-integration-20260908
kind: review-result
status: active
title: Historical concept and stack integration review with G1–G10
relations:
- reviews: specification:contract-driven-connectors-design
revision: 1
---
# Connectors v2: concept review and stack-integration check — 2026-09-08

Baseline: `connectors_v2` working tree at `57be07c` plus 7 modified / 2 untracked files
(`git status`, `git diff --stat`: 127 insertions, 33 deletions; uncommitted edits to
`contracts/operations/v1alpha1/semantics.md`, `crates/connectors-build/src/gate.rs`, planning
store). Atlas read at `../atlas` working tree (README/ROADMAP dated 2026-09-04, ADRs 0001–0031).
Consumers read from `../devcenter`, `../workspace`, `../agent-platform`, `../service-sdk`,
`../zwirn`, `../org-brain` working trees. No gate was run in this review; the last recorded gate
is `docs/evidence/review-2026-09-08/gate.log` (35 tests, 0 failed, per
`docs/review-response-2026-09-08.md`).

## 1. Verdict

The concept is sound and the implemented slice matches it. It is **not** a drop-in for any
current consumer of `../connectors`, because the v2 wire and host carry no verified caller
context, no Identity audience, no grants and no approval evidence. Nothing in v2's structure
blocks adding those; the host crate is where admission lives and the design already reserves
the "Beyond10x service binding" (`docs/design.md` § 7). The missing piece is one planned story
and one wire version, not a redesign.

## 2. What holds (verified)

| Claim | Evidence |
|---|---|
| Adapters have no host/client/sibling dependency | `adapters/*/Cargo.toml`: deps are `connectors-core`, `-contracts`, `-sdk`; `connectors-host` optional behind `service` feature |
| Client depends on core only; host on core+sdk+client; no host→adapter edge | `crates/*/Cargo.toml` `[dependencies]` sections |
| Kernel is small | `wc -l`: core 258, sdk 210, client 187, host 798, contracts 52 lines |
| Old system scale for comparison | `docs/design.md` § 2.2: 44 crate manifests, ~186,000 Rust lines, 1,017 catalog operations |
| Provider trait is minimal | `crates/connectors-sdk/src/lib.rs:18-31`: `descriptor`, `invoke`, `invoke_at` (default) |
| Old backend trait breadth | `../connectors/crates/service/src/runtime.rs:531-652`: 18 methods on `ConnectorBackend` |
| Live evidence exists for all 8 operations, direct and federated | `docs/verification.md`, `docs/evidence/2026-09-08-live-acceptance.json` |
| Contract inventory is explicit about status | `contracts/README.md`: 5 implemented, 14 proposed, 4 deferred families |
| Design names the ADR 0026 rule without citing it | `docs/design.md` § 7 paragraph "For the Beyond10x service binding…" |
| `## Serves` present (Atlas `check-map.sh` grounding fence) | `AGENTS.md` |

## 3. Gaps against the organization seams (verified)

| # | Gap | v2 today | What the org requires | Source |
|---|---|---|---|---|
| G1 | Caller context on the wire | `Invocation { version, request_id, operation, revision, input }` — no tenant, principal, realm, authority (`crates/connectors-core/src/lib.rs:92-99`) | `OwnerContext { tenant_id, agent_id, agent_revision, authority_snapshot_id, authority_snapshot_sha256 }` on every hosted request (`../connectors/crates/protocol/src/operation.rs:23-29`); verified tenant/realm/authority/executor before decoding, `None` ≠ `Some("default")` | ADR 0026 |
| G2 | Admission | One shared static bearer, constant-time compare (`crates/connectors-host/src/server.rs:57-70`); no Identity dependency (`Cargo.lock`: 0 hits for `identity-client`) | Connectors owns and verifies an exact Identity audience (`../connectors/crates/server/src/hosted/routing.rs:31-41`); trusted access exchange into Connector scopes | ADR 0021, 0031; ROADMAP row `identity → connectors` (deployed) |
| G3 | Grants / approvals | `ErrorCode` has no grant code (`connectors-core/src/lib.rs:13-27`); approval binding exists only in the *proposed* mutation profile (`contracts/operations/v1alpha1/semantics.md:43-55, 100-109`, currently uncommitted) | Grant evaluation + one-time approval redemption are what O1 means by "governed reach" | ROADMAP O1; old `docs/design/13-grant-evaluation-and-approval-redemption.md` |
| G4 | Secrets custody | `SecretStore::read(reference)` only (`connectors-sdk/src/lib.rs:42-44`); no `secrets-client` dependency | Connectors reaches Secrets through the released client and a workload identity; Vault path to be removed | ADR 0023; ROADMAP `connectors → secrets` (intended) |
| G5 | service-sdk factory seam | v2 design, contracts and adapter docs contain 0 mentions of `service-sdk`, `ConnectorServiceFactory` or any ADR (`grep -c` over `docs/design.md`, `contracts/`, `docs/adapters/`) | `service-connectors` implements old `ConnectorBackend` + `ConnectorServiceFactory` with `PrincipalContext`, `ServiceDeployment`, `ServiceDispatch`, `VerifiedAuthContext`, `GrantId`, `RealmId` (`../service-sdk/crates/service-connectors/src/lib.rs:13-33`); Todo → connectors → devcenter arrow depends on it | ADR 0027, 0029 |
| G6 | Agent-platform projection | `Operation { id, description, contract, profile, input_schema, output_schema }` — no effect class, risk or approval posture (`connectors-core/src/lib.rs:61-69`) | `agent-platform-connectors` compiles `b10x.connector-operation.v0alpha1` `OperationDescription` with `EffectClass` and `ApprovalPosture` into a Harness toolset (`../agent-platform/crates/agent-platform-connectors/src/lib.rs:10-17`) | ROADMAP `agent-platform → connectors (projection)` |
| G7 | Datasource seam | `datasource.records/v1alpha1` is pages + cursors + provenance; no binding, no `NotGranted`/`StaleAuthority` | Workspace imports `DatasourceBinding`, `DatasourceErrorCode::{NotGranted, StaleAuthority}` (`../workspace/crates/workspace-service/src/main.rs:24, 1112-1115`); org-brain intends `b10x.connector-datasource.v0alpha1` (`../org-brain/docs/design/org-brain-v0.1.md:76`) | ROADMAP `workspace → connectors`, `connectors → org-brain` |
| G8 | MCP | Design mentions MCP only as "can be a client-facing binding and/or an adapter" (`docs/design.md` § 19); no `b10x-mcp-*` dependency | Connectors exposes a governed outbound MCP adapter and the hosted inbound `/mcp` transport | ADR 0028; ROADMAP `mcp → connectors` (pinned) |
| G9 | Client API surface | `Client`: `describe`, `invoke`, `send` (`crates/connectors-client/src/lib.rs:71-111`) | Old `HostedClient`: 12 async methods (`../connectors/crates/connectors-client/src/lib.rs:463-737`) incl. `issue_approval`, `connection`, `event`, `datasource` | consumer Cargo pins below |
| G10 | Crate-name collision | v2 workspace crate is named `connectors-client` (`Cargo.toml` workspace deps) | Atlas component id `connectors/connectors-client` already exists (`../atlas/docs/catalog.md:221`); 3 consumers pin that name by git | ADR 0017 (typed catalog) |

Gaps G1–G3 are one problem: there is no verified-context input to the host, so nothing
downstream of admission can be governed per caller. G5–G7 are consequences of G1–G3 plus the
narrower operation metadata. G4 and G8 are bindings the design defers explicitly.

## 4. Current consumer pins (what would need to move)

| Consumer | Pin | Crates used | Source |
|---|---|---|---|
| devcenter | `e80b7ae…` `=0.7.0` | `connectors-client`, `protocol` | `../devcenter/Cargo.toml:37-38` |
| devcenter-connectors (excluded member) | `097b1c5…` | `connectors-runtime`, `service` | `../devcenter/crates/devcenter-connectors/Cargo.toml:22-23` |
| workspace | `b4f6d65…` | `connectors-client` (`HostedClient`, `OwnerContext`, datasource types) | `../workspace/Cargo.toml:21`; `crates/workspace-service/src/main.rs:23-28` |
| agent-platform | tag `v0.3.1` | `connectors-client` | `../agent-platform/Cargo.toml:44` |
| service-sdk | `235558c…` | `protocol`, `service` | `../service-sdk/crates/service-connectors/Cargo.toml:25-26` |
| zwirn | `1e0eb9f` | `connectors-cli` | `../zwirn/Cargo.toml:25` |
| org-brain | none (intended) | operations + datasources via `connectors serve` | `../org-brain/docs/design/org-brain-v0.1.md:39, 70` |

Atlas's projection is behind for one row: `docs/catalog.md:595` records devcenter-http at
`v0.5.3`; `../devcenter/Cargo.toml:37` pins `e80b7ae…` `=0.7.0`. Not a v2 issue; noted so it is not
mistaken for a v2 discrepancy.

## 5. Atlas registration path (verified, mechanical)

| Step | Mechanism | Status for v2 |
|---|---|---|
| Repository record | `atlas.repository` requires `layer`, `visibility`, `default_branch`, `url`, `primary_release_unit`, `gate_program` (`../atlas/catalog/definitions/repository.yaml`) | No remote, no URL; gate is `cargo run -p connectors-build -- gate --msrv`, not `scripts/gate.sh` |
| Succession | `atlas.repository-lineage` kinds: rename, split, merge, extract, archive, split-and-rename, extract-and-retire (`catalog/definitions/repository-lineage.yaml`); 3 precedents recorded 2026-09-01 | Not recorded (deferred by operator, `AGENTS.md`) |
| Consumer pins | `catalog refresh` refuses a new cross-repo pin until its typed relation exists (`../atlas/AGENTS.md`, "refuses a newly discovered cross-repository dependency") | Each of the 6 consumer rows above needs a `component-dependency` record |
| Public docs | `b10x.docs.yaml` per repository, reconciled by `atlas docs reconcile` | Absent in v2 |
| Grounding fence | `## Serves` in `AGENTS.md` | Present |

Nothing here is hard. It is 1 repository, 1 release unit, 1 lineage, N components and 6–9
dependency records, all through the existing `atlas` CLI.

## 6. Recommendation

1. Add one story now, before the remaining 20 contract-doc stories: **Beyond10x host binding**.
   Scope: (a) wire `v1alpha2` envelope carrying a receiver-verified context (tenant, principal,
   optional realm, authority ref, executor) — `story:contracts-wire-compatibility` already
   proposes a v1alpha2 envelope and is P1; (b) an Identity-audience `Credential`/admission
   implementation in `connectors-host` behind a feature, keeping the static-bearer path for
   local; (c) a `SecretStore` implementation over `secrets-client`; (d) operation metadata
   `effects`/`risk`/`approval` promoted from the proposed mutation profile into the descriptor so
   the agent-platform projection has something to compile.
2. Decide the service-sdk target: generate a v2 `Adapter` (3 methods) instead of the old
   `ConnectorBackend` (18 methods). This is the cheapest seam to move and the one with a
   released consumer (Todo 0.1.0).
3. Cite ADRs 0021, 0023, 0026, 0027, 0028, 0029, 0031 in `docs/design.md` § 7 and § 12; today
   the design cites old Connectors source paths but no organization decision.
4. Rename v2 crates or plan the lineage as `rename`/`split` before any consumer pins, so
   `connectors-client` does not mean two APIs under one Atlas component id.

Not recommended: a compatibility facade that re-implements the 12-method `HostedClient` on top
of v2. `docs/design.md` § 23.2 already says consumers embedding old runtime crates need source
changes; the consumers are 4 repos with exact pins, and moving them per repo is bounded.
