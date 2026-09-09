---
format: aep.planning-md/1
id: story:local-cli-ess-surface
kind: story
status: implemented
title: Generate and verify the declared local CLI surface through ESS
refs:
- provider: local
  reference: ess:story:cli-presentation-binding
relations:
- decomposes: epic:local-cli-contracts
- serves: vision:independent-contract-adapters
- depends_on: story:local-cli-binding-semantics
scope:
- confidence: inferred
  path: .gitignore
- confidence: inferred
  path: Cargo.lock
- confidence: inferred
  path: Cargo.toml
- confidence: inferred
  path: README.md
- confidence: cited
  path: adapters/gitlab/generated
- confidence: cited
  path: apps/connectors-cli-contract
- confidence: inferred
  path: apps/connectors/spec/README.md
- confidence: cited
  path: apps/connectors/spec/cli.yaml
- confidence: inferred
  path: apps/connectors/spec/compatibility.json
- confidence: inferred
  path: apps/connectors/tests/compatibility.rs
- confidence: inferred
  path: contracts/README.md
- confidence: inferred
  path: crates/connectors-build/src/cli.rs
- confidence: inferred
  path: crates/connectors-build/src/docs.rs
- confidence: cited
  path: crates/connectors-build/src/gate.rs
- confidence: cited
  path: crates/connectors-build/src/main.rs
- confidence: inferred
  path: crates/connectors-conformance/Cargo.toml
- confidence: inferred
  path: crates/connectors-conformance/tests/cli_surface.rs
- confidence: cited
  path: crates/connectors-spec/src/toolchain.rs
- confidence: cited
  path: crates/connectors-spec/src/v2.rs
- confidence: cited
  path: crates/connectors-spec/tests/generation.rs
- confidence: cited
  path: crates/connectors-spec/toolchain.json
- confidence: inferred
  path: docs/design.md
- confidence: inferred
  path: docs/development.md
- confidence: cited
  path: docs/gitlab-generation.md
- confidence: inferred
  path: ess/system.yaml
- confidence: inferred
  path: website/publication.json
revision: 32
---
## Context
ESS release 0.20.0 currently cannot place service-forwarding CLI calls while preserving component ownership; parameterized view projection and process contracts are incomplete. The approved sibling ESS story cli-presentation-binding supplies an additive, versioned presentation binding. Existing apps/connectors/src/main.rs remains the runtime baseline until a separately scoped runtime story.
## Acceptance
The selected CLI inventory compiles through the reviewed upstream CLI binding into deterministic parser/reference artifacts, with Rust conformance and drift gates proving argument/source/type/process behavior and explicit deferred-runtime accounting, while preserving existing runtime commands.
## Scope and dependencies
Depends on story:local-cli-binding-semantics for the authoritative values and behavior. Also requires the reviewed exact source/build of sibling ESS story:cli-presentation-binding; the cross-repository ref is evidence, not a fabricated local depends_on id.
Author apps/connectors/spec/cli.yaml and generated contract fixture artifacts; add Rust checks under crates/connectors-conformance and integrate via crates/connectors-build. Root owns Cargo manifests/lock, the toolchain resolver/pin, ess/system.yaml, contracts/README.md, docs/design.md, docs/cli-migration-v1-to-v2.md and docs/stack-integration-proposal.md.
## Checks
Compile generated parser/help/completions; red-first malformed/unknown/collision/ref/source controls; valid structured process cases with recording handlers; dynamic operation input schema selection and stale descriptor refusal. Keyring and launch lifecycle are specification/model fixtures, never real custody/provider success. Preserve release-version pin behavior; any candidate executable is exact-source/digest identified and explicitly development evidence. Do not replace production handlers or alter the website unless a documented input-registration change requires regeneration.

## Scope

Derived 2026-09-09 by aep-drive:story-scoper. apps/connectors/spec/cli.yaml — cited. New generated destination apps/connectors/generated/cli/ — inferred and adopted by coordinator. Offline test crates/connectors-conformance/tests/cli_surface.rs and build module crates/connectors-build/src/cli.rs — inferred and adopted. Action/dispatch in crates/connectors-build/src/main.rs and full gate in crates/connectors-build/src/gate.rs — cited. Existing runtime CLI and input tests remain regression inputs — cited. Shared manifests/pin/registration/indexes are coordinator-owned — cited. Confidence medium — inferred; final generator API must come from upstream. Collides with other edits to these source/gate surfaces — inferred. Dependency local-cli-binding-semantics is not terminal; this unit is not ready merely because source paths differ.

## Exact-source toolchain and regeneration
Operator selection supersedes the old release-only pin: adopt current verified ESS main as an exact Git commit, then the reviewed local main containing the required CLI addition. Record source commit and executable digest/build evidence through the single toolchain owner; reject a same-version executable with a different pinned identity. Preserve a documented legacy version-only reader for old release records, without selecting it for this task.
This story owns revalidation of all registered shared/native ESS roots and regeneration/diff review of affected GitLab bundle, schemas, scenarios, documentation reference and example outputs. Historical evidence stays immutable. Compatibility failures are fixed against current source and recorded, never hidden by returning to 0.20.0.

## Generated fixture integration

The exact-source candidate fbd9b7a3ce751c8077a862f05e00c20751b08b80 built
successfully from a clean independent Git clone, with the source/executable receipt
under .local/toolchains/ess. The builder's initial clone-root refusal identified a
derived cache path containing parent components; the cloned path is now
canonicalized before the unchanged exact-root admission check. ess-source-build-01
records the expected lockfile refusal after adding the conformance dependency;
ess-source-build-02 records the path defect, and ess-source-build-03 passes.

The generated fixture now lives at apps/connectors-cli-contract. This supersedes
the earlier apps/connectors/generated/cli placement: nesting an independent
workspace below the existing runtime member conflicts with Cargo's workspace
rules. The sibling package is excluded from ordinary workspace builds, and only
the conformance dev-dependency consumes its generated library. The production
runtime commands remain unchanged. The first generated candidate is retained at
.local/tmp/cli-wave/generated-cli-first-candidate, not committed as a duplicate.

CLI generation and drift pass: 10 artifacts, 64 structural values, 2 cached
expectations, 5 acquisition and 3 page-consistency checks. All 7 CLI conformance
tests pass against the generated library, including all 15 command mappings,
protected input refusal, single source acquisition, native dynamic schema checks,
typed output and usage/operational/interruption exits. The actual root Cargo
metadata keeps 13 workspace members and excludes the fixture. Logs under
.local/tmp/cli-wave: cli-generate-01.log, cli-generate-sibling-01.log,
cli-conformance-01.log and -02.log (Cargo layout refusals), cli-conformance-03.log
(pass), cli-check-01.log, and cargo-cli-membership.json.

ESS still requires its full gate after a correction to an existing two-writer
authority-provisioning fixture race. The final source pin, full downstream gate,
all registered ESS roots, GitLab regeneration and website regeneration/acceptance
remain outstanding. This is executable contract-fixture evidence, not production
keyring, supervisor, provider or MCP implementation.

## Final source integration

The selected ESS source is e414b493e1d07b08a6dc5a9a2e769b527cbdb9d1,
including the reviewed CLI addition and test-only fixture follow-up. The clean
source build passes with an adjacent executable receipt (ess-source-build-04.log).
The shared model and five independently owned adapter models validate and compile
against this source. CLI generation still produces 10 artifacts and its selected
64 structural, 2 cache, 5 acquisition and 3 page-consistency checks pass.

Downstream integration found two additional current-ESS details. First, Cargo's
fmt --all traverses excluded path dependencies. The gate now reads actual Cargo
workspace membership and passes every authored member explicitly to rustfmt,
preserving generated fixture bytes without omitting any workspace member.
Second, ESS now writes native per-anchor recovery metadata under .ess-output.
The portable GitLab bundle must not claim that temporary anchor's random identity.
The existing two-destination reproducibility regression failed before correction
(ess-metadata-repro-red-01.log). The bundle collector now excludes only the known
temporary rust/.ess-output subtree; the general filesystem/public-output auditor
still reads complete trees. The regression also asserts that neither output nor
its manifest publishes that metadata. Regeneration retires the accidentally
collected file through existing manifest ownership, with no hand-edited output.
The generated CLI's actual sibling .ess-output directory is ignored in Git.

The first full Connectors gate records the formatter refusal. The corrected gate
and reproducibility checks are rerun before claiming completion. Website reference,
WASM generation, public-path audit, search and production build passed on the exact
ESS source (website-build-final-01.log); browser and UI checks target the isolated
local production preview on port 3101. No external deployment is involved.

An additional integration reviewer could not start because its account usage limit
was exhausted. Earlier independent semantic/toolchain and ESS adversarial reviews
remain preserved; the coordinator reviews these downstream deltas directly.

## Completed wave validation

The selected CLI contract fixture and both wave stories are implemented. ESS
implementation source 6f7ef46163e758f3401945d1a946e0fc80ebc003 passes its full
required task check, including all 152 freshly qualified consumer cases and the
six-row metadata guard. The separately required ESS website check also passed.
Historical partial attempts remain evidence of those attempts only.

Connectors implementation tree b6573f9a3c7936f7f7b3dde93e600bc27d17d03a selects
that exact clean source through a verified local build receipt. The final
connectors-full-gate-04.log records exit zero with Rust 1.88 compatibility,
all workspace tests and strict Clippy, shared plus five independent native ESS
roots, deterministic generation, adapter boundaries, the 15-command fixture,
64 structural CLI fixtures and 315 scenarios including 34 authored traces.
GitLab regeneration changes only ESS source provenance; generated API/schema
bytes remain unchanged.

website-build-final-03.log and website-reference-check-final-04.log pass on
the same pin. Earlier typecheck, 15 Rust example tests, browser walkthroughs and
UI checks passed; the final upstream follow-up changes only a test expectation
and planning evidence. No presentation or browser-example implementation changed
after those checks.

The 152 ESS cases comprise 130 new fine-grained CLI consumer checks, 20 existing
change-detection checks and two existing relation projections. They are separate
from Connectors' 315 scenarios. The accepted ESS unknown baseline remains
157,677 unproven cells; metadata bookkeeping is not runtime behavioral coverage.

Durable evidence is in the primary checkout under
.local/cli-contract-wave-20260909/integration-evidence/: connectors contains the
gate, build and website logs; ess contains the upstream gate, qualified summary
and qualified-cases.md. The ESS primary checkout retains the complete compressed
qualification and earlier checkpoints under its corresponding local evidence
directory. Final integration heads and managed retirement are recorded in the
local integration receipt.

This completes the approved contracts/ESS fixture milestone. Production OS keyring
custody, metadata persistence, adapter supervision and provider handlers remain
future implementation; MCP, cloud/federation expansion and external publication
remain deferred. Local integration and retirement follow the completed checks.
