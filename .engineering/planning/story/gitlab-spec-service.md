---
format: aep.planning-md/1
id: story:gitlab-spec-service
kind: story
status: implemented
title: Generate the GitLab adapter from its specification and prove the local service artifact
relations:
- derived_from: specification:contract-driven-connectors-design
scope:
- confidence: cited
  path: Cargo.lock
- confidence: cited
  path: Cargo.toml
- confidence: cited
  path: README.md
- confidence: cited
  path: adapters/gitlab
- confidence: cited
  path: crates/connectors-build
- confidence: cited
  path: crates/connectors-conformance
- confidence: cited
  path: crates/connectors-spec
- confidence: cited
  path: docs
- confidence: cited
  path: ess
- confidence: cited
  path: examples
- confidence: cited
  path: spec-kinds
revision: 8
---
## Context and decision

The operator approved the GitLab specification-to-service plan and selected all three existing reads: project.get, issues.list and file.get. Implement locally in connectors_v2; do not change ESS, Atlas, the old Connectors implementation or remote state. Single-agent work uses this checkout. This is one integrated story, so no multi-item decomposition or critic panel is required.

## Specification

docs/design.md sections 18 and 27 and contracts/service/v1alpha1/semantics.md govern ownership and preserved runtime behavior. ess/system.yaml and ess/domains/declarations.yaml already declare AdapterSpecification, OperationDeclaration and ServiceConfiguration. Extend the authored specification with strict connectors.adapter/v2 source references, mechanical request mappings and explicit implementation obligations; preserve strict v1. Generate supported ESS request types and a selected component without inventing external GitLab entity lifecycles.

## Acceptance

Pin the official GitLab OpenAPI bytes and ESS 0.9.2; produce deterministic import/coverage, ESS input/synthesis, typed inputs used in real execution, request construction, dispatch and descriptors. Retain explicit handwritten admission, pagination, response interpretation and provenance ports. Missing implementations and unsupported required semantics cannot produce an advertised runnable operation. Regeneration preserves handwritten files and detects drift. Cargo-only builds work from repository-generated files. All three GitLab reads pass directly and through federation from a locally built container, with configuration and credentials mounted separately. Existing Kubernetes/SQL behavior and dependency boundaries remain verified.

## Scope

Cited: crates/connectors-spec, adapters/gitlab, spec-kinds/adapter, ess, Cargo.toml/Cargo.lock, README.md and docs/design.md contain the existing authoring/compiler/runtime boundaries. Inferred additions: Rust build tooling, generation tests, local build/realization descriptions, source provenance, generated artifacts, fixture and live evidence. The host remains the asynchronous serving runtime. No OAuth lifecycle, tenant management, new provider operations, registry publishing or Atlas integration is included.

## Verification

Verification is recorded in docs/verification.md and docs/evidence/gitlab-spec-service-2026-09-08/. The full workspace suite passed 24 tests with no failures or ignored tests. After normalizing ESS-generated Rust through rustfmt, all 13 affected generation/GitLab tests and warning-denying workspace Clippy passed again. Formatting, offline workspace builds, three independent adapter library builds and normal dependency boundaries passed. Strict v1 remains supported.

Generation pins official GitLab commit 2ff8d865e5016b14b724d1c2ce745f8300696192 and its original OpenAPI SHA-256; the bundle records ESS 0.9.2 and rustfmt. Reproducibility, committed-output drift, source/mapping refusals, missing binding implementations, unowned files and output symlinks are tested. ESS validates and synthesizes the typed local request inputs; the vendor OpenAPI import refusal remains intact in generated/ess-import.json. The v2 value declarations and existing entity relationships are in ess/domains/declarations.yaml; no external GitLab lifecycle was invented.

The local Rust executor passed bundle checks, ESS build compilation, BuildKit projection, Docker build, and ESS realization validation/compilation. It preserves the original build IR and supplies only an explicit empty secrets list to the pinned ESS reader's separate projection input. Configuration and credentials are absent from the image and mounted separately. The final image connectors-v2-gitlab:spec-local is sha256:220b27a8a4f91cd69618c0324dc7243cd446cd7f97454f8cd35bb4cfe8b9b247; its runtime executable was read back and matched the build digest.

The full live run passed all nine groups against public GitLab, fresh scoped Kubernetes, and PostgreSQL 17.11, including the explicit discovery-to-SQL configuration chain. After final generated-code formatting and repackaging, all three GitLab groups passed again against the exact final image. The original container evidence is retained under prior-build/. All four service roles exited 0 on SIGTERM; final container/gateway shutdown also exited 0. Owned containers and disposable rootfs copies were removed; the final image remains local. Private GitLab credentials use HTTP fixtures; the local PostgreSQL fixture uses explicit plaintext. The k3s fixture exited 2 on Docker stop after acceptance; that is recorded separately from adapter service exits.

## Progress

Implemented and verified locally. The operator-requested pre-implementation snapshot is commit 75f1c7275d7b227d5a2e4a3e95bfc5c9b2de262a, with both author and committer verified as b10x-bot[bot]. GitLab now executes ESS-generated request types and generated request/dispatch code, with explicit handwritten admission, pagination, response and provenance bindings. The local build executor produces an independently runnable image and exact ESS build/realization evidence. Reproduction and limitations are in docs/gitlab-generation.md and docs/verification.md.

This was one integrated story in an interactive, single-agent session. No decomposition or critic panel was needed, no approval bypass records were created, and no new worktree was created under the project's direct-checkout rule. Atlas, the original Connectors repository, registries, consumers and remote state remain unchanged. Publication is outside this task; the final image and source changes remain local.
