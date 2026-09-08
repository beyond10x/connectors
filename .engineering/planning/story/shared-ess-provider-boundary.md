---
format: aep.planning-md/1
id: story:shared-ess-provider-boundary
kind: story
status: implemented
title: Keep provider semantics out of shared ESS and enforce the boundary
tags:
- P1
relations:
- decomposes: epic:contract-semantics-remediation
- informed_by: specification:contract-driven-connectors-design
scope:
- confidence: cited
  path: Cargo.lock
- confidence: cited
  path: README.md
- confidence: cited
  path: contracts/adapters
- confidence: cited
  path: contracts/auth/evidence/v1alpha1/semantics.md
- confidence: cited
  path: contracts/datasources/logs/v1alpha1/semantics.md
- confidence: cited
  path: contracts/datasources/records/v1alpha1/semantics.md
- confidence: cited
  path: contracts/discovery/resources/v1alpha1/semantics.md
- confidence: cited
  path: contracts/ess-boundary.json
- confidence: cited
  path: contracts/operations/v1alpha1/semantics.md
- confidence: cited
  path: crates/connectors-build
- confidence: cited
  path: docs/adapters
- confidence: cited
  path: docs/design.md
- confidence: cited
  path: docs/evidence/ess-boundary-20260908
- confidence: cited
  path: ess
revision: 7
---
## Context

The operator requested a wider audit and a regression gate after provider-specific document enums were added to shared ESS during datasource hardening. Existing shared ESS also contains closed provider discovery enums, provider lifecycle/restart values and a concrete permission tuple (ess/domains/discovery.yaml:8, ess/domains/mutations.yaml:56, ess/domains/auth_access.yaml:47). This contradicts the ownership direction in docs/design.md §3.2: provider-specific mapping and protocol behavior stay with adapters.

## Acceptance

The repository gate rejects provider terminology and adapter dependencies anywhere in the shared ESS source tree, while shared and separately owned adapter models validate and compile and the relocated typed semantics remain documented.

## Work

Audit every shared domain, move provider-specific types to adapter-owned ESS roots under contracts/adapters, retain only provider-independent facts and opaque adapter identifiers in shared values, and update current ownership references. Add the check to existing Rust connectors-build tooling with negative tests for comments, names, encoded YAML, paths and forbidden dependencies; derive adapter names from repository declarations as well as a reviewed alias/native-term policy. Validate every adapter model through the pinned ESS compiler. Document limits: the lexical policy does not infer the semantics of novel terminology, and protocol names are not a provider catalog.

## Scope

- cited: ess
- cited: crates/connectors-build
- inferred: contracts/adapters
- inferred: contracts/ess-boundary.json
- cited: contracts/operations/v1alpha1/semantics.md
- cited: docs/adapters
- cited: docs/design.md
- cited: README.md
- inferred: docs/evidence/ess-boundary-20260908

## Coordination and boundary

Root is the sole tracked writer in the primary checkout under the repository's single-agent exception. This is an explicitly requested specification and Rust validation-tooling correction, not provider runtime implementation or public codec generation. Datasource stories remain in progress and retain their own findings. Existing review snapshots are historical and will not be rewritten. All work remains local. No decomposition is proposed, so the multi-story critic panel does not apply.

## Completion evidence

The [boundary checkpoint](../../../docs/evidence/ess-boundary-20260908/checkpoint.md) records the final ownership audit, exact source archive, independent initial/final reviews and per-finding dispositions. Shared ESS is provider-independent; native semantics remain typed in five independently validated adapter roots. The existing Rust gate and focused ess-boundary command share terminology, source-layout and namespace checks followed by the pinned compiler. New aliases/native concepts still need semantic review.

ESB-S-01, ESB-S-02 and ESS-GATE-01 are fixed. Both independent final reviewers approve this boundary scope with no residual findings. Their exact returned records are ess-boundary-semantics-final-20260908 and ess-boundary-gate-final-20260908; original reports and reviewer-authored native-format reissues are retained unchanged. No broader datasource completion is implied.

The full local command cargo run -p connectors-build --locked --offline -- gate --msrv exited 0: 55 Rust tests, Clippy, Rust 1.88.0, all six ESS roots, 222 compiled scenarios (34 authored, zero refusals), descriptor/library boundaries and planning validation. Eleven independent injected lexical leaks were refused and the shared protocol positive fixture passed. The shell link audit checked 182 targets with zero missing; git diff --check passed. No provider runtime, public codec, Python helper or external publication was introduced. Existing datasource drafts remain active under their original stories.
