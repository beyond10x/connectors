---
format: aep.planning-md/1
id: story:extractable-adapter-contract-ownership
kind: story
status: implemented
title: Co-locate native contracts with independently extractable adapters
tags:
- P1
relations:
- decomposes: epic:contract-semantics-remediation
- informed_by: specification:contract-driven-connectors-design
- informed_by: story:shared-ess-provider-boundary
scope:
- confidence: cited
  path: Cargo.toml
- confidence: cited
  path: README.md
- confidence: cited
  path: adapters
- confidence: cited
  path: contracts
- confidence: cited
  path: crates/connectors-build
- confidence: cited
  path: docs/adapters
- confidence: cited
  path: docs/design.md
- confidence: cited
  path: docs/evidence/adapter-ownership-20260909
revision: 7
---
## Context

The operator wants to extract an adapter such as Loki into its own repository. The completed shared-ESS boundary correction separated namespaces but put authored native models in contracts/adapters and left provider-specific normative prose in root contracts and docs/adapters. File ownership still crosses the proposed extraction boundary.

## Acceptance

An adapter's native semantic authority lives under adapters/<owner>, including authored ESS and profile contracts; root contracts retain shared protocols. The design identifies the versioned shared dependency and honest remaining extraction prerequisites. Repository validation discovers spec-only adapter directories and independently validates their authored models without adding runtime implementations.

## Work

Move native authored models to adapters/<owner>/spec/ess and provider design to its owning adapter. Split provider-specific normative profiles out of shared contract documents, keeping each selected behavior and explicit unresolved obligation. Document ownership, versioning and how independently packaged adapters consume shared protocols. Move the repository boundary policy to connectors-build, update Rust discovery and tests, and make Cargo membership explicit for the three implemented adapters. Validate current links and ESS roots and independently review ownership and gate regressions.

## Scope

- cited: adapters
- cited: contracts
- cited: crates/connectors-build
- cited: Cargo.toml
- cited: README.md
- cited: docs/design.md
- cited: docs/adapters
- inferred: docs/evidence/adapter-ownership-20260909

## Coordination

This continues the operator-authorized specification hardening. Root is the sole tracked writer; independent reviewers are read-only. No new runtime, public codec, remote publication, distribution package or adapter implementation is authorized by this correction. No entity is introduced: native types already exist in the authored ESS roots and move without semantic invention. Existing datasource drafts and unresolved findings remain owned by contracts-log-continuation and contracts-document-admission. Historical evidence snapshots and immutable review reports retain their original paths and bytes; new evidence records the relocation. No multi-story decomposition is proposed.

## Completion evidence — 2026-09-09

Native contracts, designs and authored ESS now travel under adapters/<owner>; shared contracts retain generic authority. Rust discovery validates spec-only adapter roots and the provider-boundary gate. Final reviewers explicitly close ACO-S-04. Packaging, source/license pinning and standalone extraction builds remain declared subsequent prerequisites, not completed by relocation.

Both independent final reviews spec-completion-a-final-20260909 and spec-completion-b-final-20260909 approve the selected specification scope at d1dc83f5d600816c699db07dd843f079ded0e72b. The complete checkpoint, unchanged reports and exact source archive are retained in [checkpoint](../../../docs/evidence/spec-completion-20260909/checkpoint.md). The full repository gate including MSRV 1.88 passed with 58 Rust tests; ESS synthesis/compilation and textual scenarios do not execute runtime conformance. No runtime implementation or public codec change was added in this wave.

The approved parallel plan supersedes earlier single-primary-writer dispatch wording: isolated workers changed only their assigned sources, coordinator alone wrote planning/shared joins. This story is complete for its enumerated specification findings; the overall goal remains active for the separately scoped persistent ESS model closure. Original immutable review bodies and historical source references remain unchanged.