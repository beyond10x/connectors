---
format: aep.planning-md/1
id: story:contracts-read-refresh-retry
kind: story
status: implemented
title: Version and bound read redispatch after refresh
tags:
- P2
- contract-review
relations:
- decomposes: epic:contract-semantics-remediation
- informed_by: specification:contract-review-intake-20260908
- depends_on: story:contracts-wire-compatibility
- depends_on: story:contracts-refresh-coordination
- depends_on: story:contracts-credential-evidence
scope:
- confidence: cited
  path: contracts/auth/acquisition/v1alpha1/semantics.md
- confidence: cited
  path: contracts/auth/capability/v1alpha1/read-refresh-once.md
- confidence: cited
  path: contracts/auth/capability/v1alpha1/semantics.md
- confidence: cited
  path: contracts/service/compatibility.md
- confidence: cited
  path: contracts/service/v1alpha1/semantics.md
- confidence: cited
  path: contracts/service/v1alpha2/semantics.md
revision: 10
---
## Context

Priority: **P2**. Sources: `F15`, `E05` in `specification:contract-review-intake-20260908`.

The inherited service prohibits automatic retries while the capability mandates one redispatch after a read 401.

Baseline for all file:line citations is local commit `db1c329` unless an old-repository path is named. Source findings F01–F15 are preserved in `review-result:contract-semantics-20260908`; E01–E33 are preserved in `review-result:contract-docs-external-20260908`.

## Required revision

Record the disposition change and selected compatible profile; define shared deadline, maximum exchange count, refresh errors and admission/evidence checks on redispatch.

## Acceptance

After revision, each negotiated read profile determines one bounded 401-response sequence without silently changing existing service semantics.

## Verification scenarios

- Old read profile produces the documented single-call error.
- Explicit retry profile allows only its declared refresh/redispatch sequence within one budget.
- Mutation 401 never redispatches the business effect.

Verification is a textual scenario audit: record the chosen rule and expected observation/refusal for every scenario, cross-check all cited documents, and link the resulting review evidence before claiming the story complete. Runtime conformance implementation is subsequent work.

## Scope

- cited: `contracts/auth/acquisition/v1alpha1/semantics.md`
- cited: `contracts/auth/capability/v1alpha1/read-refresh-once.md`
- cited: `contracts/auth/capability/v1alpha1/semantics.md`
- cited: `contracts/service/compatibility.md`
- cited: `contracts/service/v1alpha1/semantics.md`
- cited: `contracts/service/v1alpha2/semantics.md`

Current scope follows the adapter-owned layout and specification:spec-completion-parallel-20260909. Shared files are coordinator-integrated or assigned to one worker; stories are serialized where required.

Historical Source locations: `contracts/service/v1alpha1/semantics.md:43`; `contracts/auth/capability/v1alpha1/semantics.md:70`; `contracts/auth/capability/v1alpha1/semantics.md:91`; `contracts/auth/acquisition/v1alpha1/semantics.md:77`.

## Dependencies and edit coordination

Semantic prerequisites: `story:contracts-wire-compatibility`, `story:contracts-refresh-coordination`, `story:contracts-credential-evidence`.

Shared edit surfaces with `story:contracts-federated-approval`, `story:contracts-refresh-coordination`, `story:contracts-credential-evidence`, `story:contracts-wire-compatibility`, `story:contracts-connection-readiness`, `story:contracts-anonymous-auth`, `story:contracts-management-boundary`, `story:contracts-acquisition-profiles`, `story:contracts-media-controls`, `story:contracts-supported-vocabulary`, `story:contracts-persistence-ownership`, `story:contracts-mutation-visibility`. These collisions require serialized edits or a coordinated integration owner; the stories are not a pre-approved concurrent wave. Semantic dependencies do not stand in for edit-collision records. Each story owns only its listed findings, not sibling outcomes on the same file.

## Boundary and modeling

This local-only story completes textual semantics, adapter-owned specifications, conformance scenarios and relevant ESS shape validation under the operator's current specification-completion authorization. No runtime/provider/codec implementation or adapter runtime expansion is authorized. Local coordinator commits and managed worktrees are authorized; external publication is not. New typed entities or relations must use the ESS workflow, preserving UNMAPPED implementation obligations explicitly. Existing review/source snapshots remain immutable.

The approved parallel assignment is specification:spec-completion-parallel-20260909. Root alone writes planning records and integrates shared-file patches. Original file:line citations above retain their historical db1c329 baseline; current writable ownership follows the dispatch brief and current machine scope, including the adapter-owned layout checkpoint 8e1836c.

## Completion evidence — 2026-09-09

F15/E05 are fixed by the explicitly selected native combined read-refresh-once profile: one definitive complete first 401, one coordinated refresh participation, the exact acknowledged current successor and at most one new dispatch under the original deadline and consumed permission budget. No current adapter selects it. Fourteen manual cases are recorded in docs/evidence/read-refresh-retry-20260909/baseline-cases.md.

Both independent final reviews spec-completion-a-final-20260909 and spec-completion-b-final-20260909 approve the selected specification scope at d1dc83f5d600816c699db07dd843f079ded0e72b. The complete checkpoint, unchanged reports and exact source archive are retained in [checkpoint](../../../docs/evidence/spec-completion-20260909/checkpoint.md). The full repository gate including MSRV 1.88 passed with 58 Rust tests; ESS synthesis/compilation and textual scenarios do not execute runtime conformance. No runtime implementation or public codec change was added in this wave.

The approved parallel plan supersedes earlier single-primary-writer dispatch wording: isolated workers changed only their assigned sources, coordinator alone wrote planning/shared joins. This story is complete for its enumerated specification findings; the overall goal remains active for the separately scoped persistent ESS model closure. Original immutable review bodies and historical source references remain unchanged.