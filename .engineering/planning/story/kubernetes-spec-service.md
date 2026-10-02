---
format: aep.planning-md/3
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
  path: adapters/kubernetes
- confidence: cited
  path: crates/connectors-build
- confidence: cited
  path: crates/connectors-conformance
- confidence: cited
  path: crates/connectors-spec
- confidence: cited
  path: docs/development.md
- confidence: cited
  path: docs/local-kubernetes-cli.md
- confidence: cited
  path: ess/domains/declarations.yaml
- confidence: cited
  path: examples
- confidence: cited
  path: spec-kinds
revision: 6
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

## Current preflight, 2026-10-02

The explicit configured protocol snapshot resolves with both AEP 0.65.0 and 0.68.0; tooling-blocker:kubernetes-driver-protocol-loading is cleared with retained output in docs/evidence/kubernetes-resolver-20261002/. Use that exact --root selection; implicit discovery remains broken. Runtime baseline is v0.24.0 at c7a9d5b1db1ff4e3b2af568db19c6eaa60d79d90; project tools are ESS 0.45.0 / AEP 0.65.0. The earlier source-pin, native-GitLab and local-only statements describe historical preparation, not the current implementation path. Native GitLab was retired; regressions now target the catalog provider. approval-record:milestones-delivery-20261002 authorizes the selected implementation and verified source release, not a paid drive or deployment. Re-scope actual generation ownership before scheduling this story alongside other source edits.

## Re-scoping conclusion — 2026-10-02

Read-only current-source preflight is retained in
`docs/evidence/milestone-preflight-20261002/kubernetes-generation.md`. The smallest
coherent unit remains actual specification-derived request construction/dispatch
for the three existing Kubernetes read/discovery operations, plus the independent
HTTP-service image, retaining authored admission, cursors, response/provenance and
four existing Helm operations. Descriptor generation alone cannot satisfy it.
Native GitLab restoration, catalog packaging repair, new logs/restart/execution
and broad shared service-contract changes are not part of this unit.

Five Connectors-owned design selections precede runtime dispatch: finite kind
mapping alternatives, /api versus /apis root ownership, closed typed selector
enum lowering, mixed three-generated/four-authored handler coverage, and an actual
immutable official Kubernetes OpenAPI pin with digest/license and operation IDs.
These are engineering design work for ESS/spec-kind ownership, not new requests
for operator permission. They remain unselected by this read-only audit. Do not
silently extend v2, downgrade pins or remove supported operations to fit it.

Required conformance names for the eventual selected implementation are:
`kubernetes_generation_is_deterministic_and_required_branches_are_complete`,
`kubernetes_generated_dispatch_serves_all_selected_kinds_from_exact_image`,
`kubernetes_generated_continuations_preserve_every_selection_partition`,
`kubernetes_generated_binding_preserves_helm_and_credential_authority`, and
`kubernetes_image_discovery_requires_explicit_sql_activation`. The source report
states their exact observable inputs and assertions. The same recorded image
must serve direct and federated cases; path/query evidence must establish actual
use of generated dispatch. Actual generation, image and runtime evidence is still
missing. No paid driver is authorized or needed for interactive work.

Scope is narrowed to Kubernetes-owned spec/source/generated/realization paths,
the shared declaration model, Connectors spec-kind/compiler and focused build/
conformance integration, plus the owning development/Kubernetes guides. Exact new
mapping-profile filenames remain inferred until the five selections are made.
Catalog GitLab, SQL and existing Helm code are regression owners, not migration
surfaces. Preserve all original three-operation/four-kind and explicit SQL
activation acceptance; do not treat this re-scope as completion.

## Source measurement and smaller composition option — 2026-10-02

The subsequent bounded source measurement resolves the unknown upstream bytes:
official Kubernetes tag v1.31.5 peels to
`af64d838aacd9173317b39cf273741816bd82377`. Its three official OpenAPI3.0 group
documents for core, apps and discovery are below the existing16MiB bound and name
all five required provider endpoints. Exact URLs, SHA256, sizes, Apache2.0 license,
parameter schemas and source-reference checks are retained in
`docs/evidence/milestone-preflight-20261002/kubernetes-upstream.md`. This is not
an importer or runtime pass; the K3s fixture's version is not independently proven
merely by verifying the upstream Kubernetes tag.

The smaller proposed design uses three private existing-v2 bundles with six fixed
internal mappings, explicit authored public composition and unchanged public seven
operations. It avoids a new generalized selector/mixed-coverage compiler profile.
`kubernetes-generation-options.md` beside the source report documents actual
parser/renderer/integration support and the required aggregate coverage. No new
public operation, provider instance or credential owner is introduced by a private
bundle. Public operation/cursor/connection identities remain authoritative.

The concrete mapped-parameter importer gap is `uniqueItems:true` on scalar namespace,
limit and continue schemas. Selected mapped parameters do not have the earlier
hypothesized int64/ref obstacles. A narrowly reviewed rule may recognize the
array-only keyword as inapplicable to admitted scalar types; no source rewrite or
arbitrary-unknown-keyword allowance is authorized by this observation. Exact
optional-continuation omission remains an explicit obligation, and packaging must
verify all private bundle manifests plus public aggregate coverage. No source/
model/compiler change is accepted by this planning preflight. Select and model
those bounded composition/import/serialization semantics before implementation.
