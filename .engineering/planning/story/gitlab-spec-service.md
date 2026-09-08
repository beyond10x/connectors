---
format: aep.planning-md/1
id: story:gitlab-spec-service
kind: story
status: active
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
revision: 4
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

Run reproducibility/drift/refusal tests; fixture request, policy, paging, response and auth tests; Cargo formatting/build/test/Clippy and independent adapter/host dependency checks; ESS input/build/realization validation; a local Docker build and live GitLab direct/federated acceptance; Kubernetes/SQL regressions; artifact identity and clean shutdown checks. Record exact results and limitations in docs before marking implemented.

## Progress

Plan approved. Existing v1 implementation and live acceptance evidence are the baseline. Source/toolchain discovery is in progress.
