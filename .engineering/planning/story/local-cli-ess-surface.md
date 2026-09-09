---
format: aep.planning-md/1
id: story:local-cli-ess-surface
kind: story
status: draft
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
  path: apps/connectors/generated/cli
- confidence: cited
  path: apps/connectors/spec/cli.yaml
- confidence: inferred
  path: crates/connectors-build/src/cli.rs
- confidence: cited
  path: crates/connectors-build/src/gate.rs
- confidence: cited
  path: crates/connectors-build/src/main.rs
- confidence: inferred
  path: crates/connectors-conformance/tests/cli_surface.rs
revision: 9
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
