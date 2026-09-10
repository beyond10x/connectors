---
format: aep.planning-md/1
id: story:kubernetes-spec-service
kind: story
status: draft
title: Generate and package the Kubernetes adapter from the current verified baseline
relations:
- derived_from: specification:contract-driven-connectors-design
- informed_by: story:gitlab-spec-service
- informed_by: story:full-review-remediation
- informed_by: specification:repository-integration-20260910
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
  path: adapters/kubernetes
- confidence: cited
  path: contracts/service/v1alpha1
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
revision: 2
---
## Context and current status

This draft integrates the unfinished Kubernetes generation preparation preserved in commit cf24afc86195c834634ae5d630a3e9517696c6eb into the current main backlog. The original story, eight journal entries and driver task remain readable in that commit. Its former worktree has been retired. Integration does not claim Kubernetes generation or packaging has been implemented and does not authorize a paid governed run.

The integration baseline is e4b4b9817ee65cd859baef960299d56cce7e2063. README.md and docs/development.md distinguish the working Kubernetes read/discovery adapter from GitLab's implemented v2 generation and packaging pipeline. Kubernetes and SQL still use v1 descriptor generation. The September 8 baseline 801b8d5 and the old ESS release migration are historical; they are not instructions to downgrade current main.

## Specification and typed ownership

Use docs/design.md sections 18, 21, 22 and 27–29, contracts/service/v1alpha1/semantics.md, spec-kinds/adapter/v1/semantics.md, spec-kinds/adapter/v2/semantics.md and docs/gitlab-generation.md. Existing AdapterSpecification, OperationDeclaration, ServiceConfiguration, UpstreamSource and RequestMapping declarations in ess/domains/declarations.yaml own the shared types. Provider models and upstream mappings belong to adapters/kubernetes. Model and validate any necessary typed extension before implementation; keep unknown mapping semantics explicit.

The single ESS selection is crates/connectors-spec/toolchain.json, currently source 6f7ef46163e758f3401945d1a946e0fc80ebc003 at the integration baseline. Resolve it through the repository's exact-source receipt verification. Read the current pin when work starts; do not restore the old 0.9.2 binary or the superseded 0.20.0 release commit. The GitLab generator migration and removal of the empty-secrets adaptation have already landed, as documented in docs/gitlab-generation.md.

## Acceptance

From a recorded proof of the selected current baseline, the Kubernetes adapter executes its three existing read/discovery operations through deterministic pinned specification-derived bindings and an independently runnable local image, preserving scope, paging, optional host discovery and provenance directly and through federation, with GitLab/SQL regressions and the repository gate passing.

## Work sequence and evidence

1. Review the current baseline and retained verification evidence. Reuse earlier checks only for their exact unchanged inputs; obtain missing live baseline/image evidence under the selected implementation run and keep it distinct from final evidence. Keep credentials private and fixtures owned by the run.
2. Pin official Kubernetes specification bytes with immutable revision, digest and license provenance. Preserve resources.list for pods, services, deployments and EndpointSlices; endpoints.discover for EndpointSlices; and hosts.discover for nodes only when configured. Resolve the current v2 one-mapping-per-operation limitation explicitly before modeling Kubernetes selection. Do not add an unrestricted endpoint escape hatch or omit a supported resource kind.
3. Extend the Connectors frontend and supported ESS lowering so generated request construction and dispatch execute in the real adapter. Keep admission, signed cursor binding, completeness, response interpretation and endpoint/host provenance in their explicit semantic bindings. Missing mappings must refuse generation or advertisement. Preserve GitLab compatibility.
4. Add an adapter-owned Kubernetes local realization and extend the existing Rust packaging executor. Verify the exact image directly and through federation, including namespace/kind refusal, continuation, optional-host behavior and explicit discovery-to-SQL activation. Configuration and credentials remain separate from the image.
5. Run applicable deterministic generation/refusal, packaging and three-adapter checks, plus cargo run --locked -p connectors-build -- gate --msrv. Record source, binary and image digests and real command results. Stop owned services and remove only owned fixtures.

## Boundaries

The existing eight operations across GitLab, Kubernetes and SQL define the runtime surface. No new provider, write, watch, automatic discovery activation, OAuth acquisition, tenant system, external publication, Atlas enrollment or consumer migration is included. SQL remains an explicit native-protocol implementation. Do not change AEP, ESS, Atlas or the original Connectors repository as an incidental part of this story.

Use connectors for integrations and report gaps before an alternative client. Follow AGENTS.md for bot commits, current Atlas authority, managed checkout leases, two Cargo jobs and task-owned TMPDIR. Retain historical evidence. No additional bare recovery repository is required by this draft.

## Governed launch

.engineering/tasks/kubernetes-spec-service.yaml is the reconciled draft input for one future governed run under the project's configured adp/1 and development.standard profile. No launch, map selection, budget or approval is supplied by this integration. The historical resolver attempt reported unknown_protocol with no protocol documents loaded; its empty output file is not successful resolution evidence. Recheck the current project-loading preflight when a run is selected, preserve any refusal, and use operator-supplied budget and per-run cost assumptions before paid execution.

## Scope

Cited from the original draft and current owners: Cargo.toml, Cargo.lock, README.md, adapters/kubernetes, adapters/gitlab, crates/connectors-spec, crates/connectors-build, crates/connectors-conformance, contracts/service/v1alpha1, spec-kinds, ess, examples and docs. This retained scope belongs to future implementation; the present integration changes only planning prose and the draft task.
