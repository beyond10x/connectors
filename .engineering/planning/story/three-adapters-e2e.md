---
format: aep.planning-md/1
id: story:three-adapters-e2e
kind: story
status: implemented
title: Implement Kubernetes discovery, GitLab, and SQL end to end
relations:
- derived_from: specification:contract-driven-connectors-design
scope:
- confidence: cited
  path: .gitignore
- confidence: cited
  path: AGENTS.md
- confidence: cited
  path: Cargo.lock
- confidence: cited
  path: Cargo.toml
- confidence: cited
  path: README.md
- confidence: cited
  path: adapters
- confidence: cited
  path: apps
- confidence: cited
  path: contracts
- confidence: cited
  path: crates
- confidence: cited
  path: docs
- confidence: cited
  path: ess
- confidence: cited
  path: examples
- confidence: cited
  path: spec-kinds
revision: 7
---
## Context

The operator's goal is "3 adapters implemented e2e", selecting Kubernetes including discovery, GitLab, and SQL. Work remains local, with Atlas integration explicitly deferred. The operator permits single-agent edits directly in the checkout.

## Specification

The typed declaration domain is ess/system.yaml and ess/domains/declarations.yaml. ESS validation reports `connectors v1 — 2 file(s), valid`. The executable scope is contracts/service/v1alpha1/semantics.md. The larger architectural direction remains docs/design.md, with the first implementation recorded in section 27. Multi-tenant assignment cardinality remains explicitly UNMAPPED; the first slice does not invent a tenant control plane.

## Acceptance

Kubernetes resource/endpoint/host discovery, GitLab project/issue/file reads, and PostgreSQL schema/query access work through independently configured adapter executables and the provider-independent client and federation host. Automated wire/failure tests and separate live upstream evidence prove all three. Discovery requires a separate explicit configuration/credential binding before SQL access.

## Scope

- Root Cargo workspace, lockfile, README, AGENTS.md and ignore rules.
- Shared crates for core/contract types, SDK ports, client, host, specification compiler and live conformance runner.
- adapters/gitlab, adapters/kubernetes and adapters/sql, each with authored specification, deterministic generated descriptor, independent library and executable.
- apps/connectors supplies generic client commands and federation serving.
- contracts, spec-kinds, ess, examples and docs hold the selected behavior, declarations, reproducible fixtures, operating instructions and evidence.
- One writer and one integrated end-to-end outcome; no concurrent multi-item implementation was scheduled.

## Verification

Recorded commands, limitations and evidence are in docs/verification.md. The workspace builds; formatting and warning-denying Clippy pass. All 14 workspace tests pass with no failed or ignored tests. All three generated descriptors match their sources (3 GitLab, 3 Kubernetes, 2 SQL handler obligations). ESS validation passes. Each adapter library builds without default features and without host/client/sibling dependencies; the generic CLI/host dependency tree contains no provider implementation.

The final live Rust acceptance runner passed all nine scenario groups against public GitLab over verified HTTPS, an actual disposable k3s API server using a scoped bearer credential and verified CA, and actual PostgreSQL 17.11 using a restricted reader role. All eight supported operations and selected refusal cases worked directly and through federation. SQL evidence includes exact numeric values, NULLs, arrays/JSON, parameter binding, truncation, oversized-row refusal, mutation/multiple-statement refusal and database deadlines. The explicit Kubernetes discovery-to-SQL binding chain was executed. Each final service process exited 0 on SIGTERM; the two owned Docker fixtures were stopped and removed.

Evidence files: docs/evidence/2026-09-08-live-acceptance.json, docs/evidence/2026-09-08-workspace-tests.log, docs/evidence/2026-09-08-build.sha256 and docs/evidence/2026-09-08-source.sha256. Reproduction: docs/live-e2e.md.

## Progress

The selected first slice is implemented and verified locally. GitLab private credentials are covered by HTTP fixtures rather than a private live account; PostgreSQL TLS is implemented but the disposable live database used explicit plaintext. The production tenant/auth control plane, caching, event/media contracts, other providers, automatic OpenAPI-to-executable ESS lowering, OCI synthesis and registry deployment remain outside this goal. Atlas and the original Connectors checkout remain unchanged. No publication or remote integration was performed.
