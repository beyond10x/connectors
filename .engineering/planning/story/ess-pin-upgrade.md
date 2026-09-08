---
format: aep.planning-md/1
id: story:ess-pin-upgrade
kind: story
status: implemented
title: Decide and, if approved, upgrade the ESS pin from 0.9.2 to 0.18.0
summary: Trial regeneration with 0.18.0, bundle review and live GitLab re-acceptance; or record that 0.9.2 stays.
relations:
- derived_from: specification:contract-driven-connectors-design
- informed_by: story:gitlab-spec-service
- depends_on: story:ess-executable-pin
scope:
- confidence: cited
  path: README.md
- confidence: cited
  path: adapters/gitlab/generated
- confidence: cited
  path: crates/connectors-build
- confidence: cited
  path: crates/connectors-spec
- confidence: cited
  path: docs
- confidence: cited
  path: spec-kinds/adapter/v2/semantics.md
revision: 8
---
## Context

The repository pins ESS 0.9.2 for GitLab generation (`crates/connectors-spec/src/v2.rs:8`; `docs/gitlab-generation.md:12`). The newer installed ESS on the operator's machine is 0.18.0 (`~/.cargo/bin/ess`, observed 2026-09-08). `docs/design.md:64` and `:915` record 0.9.2 as the version observed during the design handoff. `crates/connectors-build/src/main.rs` carries at least one adaptation specific to 0.9.2 (the empty-`secrets` default on build IR input, commented at the `build-ir.projectable.json` step).

Whether 0.18.0 accepts the same `import openapi`, `build compile`, `project buildkit`, `realization validate` and `realization compile` invocations, and produces a byte-identical or reviewable bundle, is not known. It has not been tried.

The initial scheduling blocker was cleared by the operator decision below. The implemented target is 0.20.0, superseding the original 0.18.0 candidate named in the historical title.

## Specification

`spec-kinds/adapter/v2/semantics.md` and `docs/gitlab-generation.md` govern generation and its reproducibility evidence. No new domain entity.

## Acceptance

Either outcome is acceptable and must be recorded:

- Upgrade approved: the pin record from `story:ess-executable-pin` reads the new version; `adapters/gitlab/generated/` is regenerated with it; the bundle diff is reviewed and summarised in `docs/gitlab-generation.md`; `generation` and `obligations` tests pass; `--check` reports no drift; the direct and federated live GitLab acceptance in `docs/gitlab-generation.md` is re-run and its evidence directory recorded; 0.9.2-specific adaptations in `crates/connectors-build/src/main.rs` are removed or re-justified.
- Upgrade declined: the blocker is cleared with the reason, this story moves to `rejected`, and `docs/gitlab-generation.md` states that 0.9.2 is the supported version and why.

## Scope

- `crates/connectors-spec/toolchain.json` (after `story:ess-executable-pin`), `crates/connectors-spec/src/v2.rs`, `crates/connectors-build/src/main.rs`.
- `adapters/gitlab/generated/` (all files, including `manifest.json`).
- `docs/gitlab-generation.md`, `docs/verification.md`, `README.md:19-20`.
- `spec-kinds/adapter/v2/semantics.md`: replace the stale literal version with a reference to the pin; supported semantics are unchanged.
- Not in scope: Kubernetes and SQL adapters (v1 specifications, no ESS generation), `ess/` declarations.

## Verification

A trial regeneration with the approved 0.20.0 release into a task-owned directory under `.local/`, comparing against the committed bundle, is the first upgrade step before replacing any committed generated file. Then the gate, then the live GitLab run. Record commands and output.

## Progress

Implemented 2026-09-08 after story:ess-executable-pin. Verified official 0.20.0 archive and installed only in the checkout-local versioned cache. Trial generation changed only the ESS import report and manifest; the other 20 generated files are byte-identical. Adopted the grouped CLI and removed the empty-secrets build-IR adaptation after direct projection succeeded. Full gate with Rust 1.88: 39 tests, zero failures/ignored, all checks passed. Generation 5/5, obligations 4/4 and explicit bundle drift check pass. Local image packaging and ESS physical realization validation/compilation pass. All three direct/federated public GitLab live acceptance groups pass; explicit continuation checks return distinct second issues. Readback binary hash and all 81 code/Cargo inputs match the package evidence. Both services exited 0; the task container, credential and copied probe binary were removed. Evidence and commands: docs/verification.md, ESS pin upgrade section, and docs/evidence/ess-toolchain-2026-09-08/. The image remains local; no publication or global installation change.

## Decision

The operator authorized completing both ESS stories after the current release was verified. Upgrade to ESS 0.20.0, superseding the original 0.18.0 candidate in this artifact's historical title and acceptance examples. Use official release tag commit c90ca1b2a3a5db02d7580dab63be6cbc56679e0b and verify the downloaded platform archive against release metadata. Trial generation remains the first upgrade check. Adapt to the current grouped CLI, regenerate and review GitLab outputs, reassess the old build-IR shim, run the gate and direct/federated live GitLab image acceptance. Preserve historical evidence. The resolver story is implemented first on the existing 0.9.2 pin; final resolver acceptance applies to whichever version the single repository record declares.

The primary checkout is used for these explicitly requested single-agent prerequisite fixes under AGENTS.md. The Kubernetes managed checkout and its draft driven task remain separate and untouched. No paid model run is launched, and no global binary or PATH configuration is changed.
