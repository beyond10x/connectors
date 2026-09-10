---
format: aep.planning-md/1
id: story:kubernetes-spec-service
kind: story
status: draft
title: Generate and package the Kubernetes adapter after proving the reviewed baseline
relations:
- derived_from: specification:contract-driven-connectors-design
- informed_by: story:full-review-remediation
- informed_by: story:gitlab-spec-service
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
revision: 5
---
## Context

The operator accepted the next sequence: first rebuild and prove reviewed commit 801b8d5aad9166f3a378b9471e7d733cafa95f50 against live configured services; then extend the proven GitLab generation and local packaging pipeline to Kubernetes. The operator selected aep-drive:drive, aep-plan:planning and workspace-hygiene:worktree. This is one integrated story for one governed run, prepared in a managed checkout. No implementation or lifecycle move is performed by the preparing session.

## Specification

Read docs/design.md sections 18, 21, 22 and 27–29, contracts/service/v1alpha1/semantics.md, spec-kinds/adapter/v1/semantics.md, spec-kinds/adapter/v2/semantics.md, and docs/gitlab-generation.md. Existing typed declarations in ess/system.yaml and ess/domains/declarations.yaml cover AdapterSpecification, OperationDeclaration, ServiceConfiguration, UpstreamSource and RequestMapping. Preserve their ownership relations. Model and validate any necessary typed declaration extension before implementing it; keep unsupported upstream semantics explicit. No external Kubernetes resource lifecycle or tenant relation is to be invented.

## Acceptance

Starting from a recorded live proof of reviewed baseline 801b8d5, the Kubernetes adapter executes its three existing read/discovery operations through deterministic, pinned specification-derived bindings and an independently runnable local image, preserving scope, paging, optional host discovery and provenance directly and through federation, while GitLab/SQL regressions and the repository gate remain green.

## Work sequence and evidence

1. Before implementation changes, rebuild the baseline GitLab image and run the existing public GitLab, scoped Kubernetes and PostgreSQL acceptance chain, including discovery followed by an explicit SQL binding. Bind source, binaries and image to their digests; retain sanitized results separately from the later implementation evidence. Use owned fixtures and loopback exposure. Existing .local directories from other sessions are not inputs or disposable resources.
2. Pin official Kubernetes specification bytes with immutable revision, digest and license provenance. Account for all selected source operations and required semantics. The current resources.list handler selects among pods, services, deployments and EndpointSlices; endpoints.discover lists EndpointSlices; hosts.discover lists nodes only when configured. The current v2 generator assumes one mapping per advertised operation and needs a reviewed, bounded solution for this existing Kubernetes selection behavior. Do not silently drop a resource kind, insert an arbitrary endpoint escape hatch or handwrite a second editable generated representation.
3. Reuse and extend the Connectors frontend and ESS lowering where supported. Generated request types/construction/dispatch must execute in the real adapter. Keep explicit handwritten bindings for admission, signed cursor context, completeness, response interpretation and endpoint/host provenance. Unknown mappings or missing bindings must refuse advertisement or generation. Preserve GitLab generation and strict existing document compatibility.
4. Add a Kubernetes local realization and reuse the Rust packaging executor. Verify the generated bundle and exact local image; mount configuration and credentials separately. Run Kubernetes directly and through federation from that image, with namespace/kind refusal, continuation, optional-host behavior, endpoint observations and explicit discovery-to-SQL activation. Re-run the complete three-adapter acceptance against the final revision, verify service shutdown and remove only owned fixtures.
5. Run cargo run --locked -p connectors-build -- gate --msrv, generator refusal/reproducibility tests and packaging checks. Extend focused fixtures for any new mapping semantics. Record exact versions, commands, hashes and limits in docs/evidence/ and the operating guide. Evidence from baseline and final code must be distinguishable.

## Boundaries

Implementation is local Rust tooling in this repository. Existing eight read/discovery operations define the surface; no new provider, write operation, watch, automatic discovery activation, OAuth flow, tenant system, external registry publication, Atlas integration or consumer migration is included. SQL's native protocol remains an explicit implementation. Keep the pinned ESS version unless a concrete refusal requires an operator-visible compatibility decision; never weaken a contract to make generation pass.

Use connectors for engineering integrations and report missing operations before an alternative client. Credentials stay out of task documents, source and evidence. Direct commits must use the verified Atlas as-bot wrapper as required by AGENTS.md. Do not modify AEP, ESS, Atlas or the original Connectors repository. Inspect disk space before builds, bound jobs, use checkout-owned TMPDIR, and preserve managed-worktree leases and local-only retention.

## Governed launch

The reviewable driver task is .engineering/tasks/kubernetes-spec-service.yaml, using the project's existing adp/1 protocol and development.standard profile. Let the driver select its map; do not supply a guessed map or suppress refusals. The operator must review that task and supply budget-usd and assume-usd-per-run before the free preflight and one paid launch. No amount or approval is inferred by this draft. This interactive preparation creates no bypass record and performs no artifact move. There is no multi-story decomposition, so the decomposition critic panel does not apply.

## Toolchain correction

The operator identified the stale ESS pin during preparation. Verified on 2026-09-08: the login shell selects /home/timo/.local/bin/ess, reporting 0.9.2; /home/timo/.cargo/bin/ess reports 0.18.0. Native read-only Git inspection of the official ESS origin lists newest tag 0.20.0, whose peeled commit is c90ca1b2a3a5db02d7580dab63be6cbc56679e0b. The local ESS changelog dates that release 2026-09-06. The local source checkout has later unreleased commits and is not the release pin. Connectors returned no admitted release lookup operation before native Git was used. The existing domain validates with the installed 0.18.0 binary; full generation/package compatibility with 0.20.0 has not yet been tested.

The work sequence is amended: retain the exact 801b8d5 baseline proof as historical evidence using its declared inputs, then pin and verify the 0.20.0 release toolchain, migrate the existing GitLab generator/build executor/gate and their command spellings, regenerate and review the bundle and import coverage/refusals, and re-prove the exact image before extending Kubernetes. Reassess the old build-IR empty-secrets compatibility adaptation against the current tool instead of carrying it forward unexamined. Preserve original baseline evidence and distinguish it from migrated evidence. An explicit 0.20.0 executable path belongs in the local build/run instructions; do not alter global PATH or overwrite either installed binary. The user correction supersedes the earlier preference to retain 0.9.2 for new implementation. Final GitLab/Kubernetes generation and packaging acceptance must use the verified current release pin.

This is a preparation correction inside the existing single story, not a second implementation run. The driver remains unlaunched, with no budget assigned or lifecycle move made. The separate read-only AEP resolution attempt refused: error: the task cannot be resolved: [unknown_protocol] protocol adp/1: no protocol document declares `adp/1` (hint: no protocol documents are loaded at all). Fixing ESS does not establish that this AEP project-loading issue is resolved; it remains a preflight concern, not evidence of implementation failure.

## Recovery handoff 2026-09-10

The operator authorized preserving this unfinished preparation, committing it to local recovery, and retiring the managed checkout. This is an interactive cleanup, not authorization to launch the governed run or implement the story. The story remains draft; no runtime, live-provider, or completion evidence is asserted.

The retained source baseline is 801b8d5aad9166f3a378b9471e7d733cafa95f50 on work/kubernetes-spec-service. At cleanup, primary main is e4b4b9817ee65cd859baef960299d56cce7e2063, 56 commits ahead. The current primary selects ESS source 6f7ef46163e758f3401945d1a946e0fc80ebc003. The earlier release-pin migration instructions and task file are preserved as historical preparation; reconcile them against current contracts and the current single toolchain pin before a future run. Do not downgrade current main to execute this draft unchanged.

The recovery checkpoint contains the story, its AEP journal history, and .engineering/tasks/kubernetes-spec-service.yaml. Preserve it under local-recovery refs/heads/work/kubernetes-spec-service before managed finish and exact-id GC. The original ignored .local/task-resolution.json is an empty file, not successful resolution evidence; an identical copy is retained in the primary checkout at .local/cleanup-20260910/kubernetes-draft/task-resolution.json. Cleanup receipts and the final commit/ref belong under that primary .local/cleanup-20260910 directory, outside the retired tree.

Next owner: the operator or a newly assigned implementor. Next action: recover this draft from its local recovery branch, reconcile its historical baseline and toolchain instructions with current main through AEP, and resolve the recorded driver preflight and budget requirements before launch. No expensive gate is rerun for this planning-only checkpoint; prior CLI implementation checks remain evidence of their original revisions only.
