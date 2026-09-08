---
format: aep.planning-md/1
id: story:contracts-evidence-precision
kind: story
status: active
title: Correct citations and distinguish examples from verified provider facts
tags:
- P3
- contract-review
relations:
- decomposes: epic:contract-semantics-remediation
- informed_by: specification:contract-review-intake-20260908
scope:
- confidence: cited
  path: adapters/atlassian/design.md
- confidence: cited
  path: adapters/docker/design.md
- confidence: cited
  path: adapters/grafana/design.md
- confidence: cited
  path: adapters/kubernetes/design.md
- confidence: cited
  path: adapters/sip/design.md
- confidence: cited
  path: contracts/media/v1alpha1/semantics.md
- confidence: cited
  path: contracts/operations/v1alpha1/semantics.md
- confidence: cited
  path: contracts/sessions/v1alpha1/semantics.md
- confidence: cited
  path: docs/compositions/media-session.md
revision: 7
---
## Context

Priority: **P3**. Sources: `E15`, `E16`, `E17`, `E18`, `E25`, `E26`, `E31`, `E33` in `specification:contract-review-intake-20260908`.

Cited source locations, Grafana GET coverage and Kubernetes status claims overstate their evidence; unverified Docker facts and unlabeled configuration numbers appear normative.

Baseline for all file:line citations is local commit `db1c329` unless an old-repository path is named. Source findings F01–F15 are preserved in `review-result:contract-semantics-20260908`; E01–E33 are preserved in `review-result:contract-docs-external-20260908`.

## Required revision

Correct the specified citations against local sources; distinguish selected Grafana GET operations from its unselected POST; qualify Docker assertions until pinning and Kubernetes projection until declared; label example values or trace intentionally normative defaults. Do not perform a vendor implementation or blindly relabel actual contract bounds as illustrative.

## Acceptance

After revision, every listed evidence defect has a checked citation or an explicit evidence limitation in the affected document.

## Verification scenarios

- Correct media old-design path, SIP response range, Jira effect line and core struct ranges.
- Grafana source POST remains acknowledged as unselected.
- Docker paging/no-op assertions are marked for verification unless independently verified later.
- Configuration outlines distinguish illustrative values from selected profile bounds; Kubernetes status projection is not inferred from an untyped item schema.

Verification is a textual scenario audit: record the chosen rule and expected observation/refusal for every scenario, cross-check all cited documents, and link the resulting review evidence before claiming the story complete. Runtime conformance implementation is subsequent work.

## Scope

- cited: `adapters/atlassian/design.md`
- cited: `adapters/docker/design.md`
- cited: `adapters/grafana/design.md`
- cited: `adapters/kubernetes/design.md`
- cited: `adapters/sip/design.md`
- cited: `contracts/media/v1alpha1/semantics.md`
- cited: `contracts/operations/v1alpha1/semantics.md`
- cited: `contracts/sessions/v1alpha1/semantics.md`
- cited: `docs/compositions/media-session.md`

Current scope follows the adapter-owned layout and specification:spec-completion-parallel-20260909. Shared files are coordinator-integrated or assigned to one worker; stories are serialized where required.

Historical Source locations: `contracts/media/v1alpha1/semantics.md:72`; `contracts/sessions/v1alpha1/semantics.md:21`; `docs/adapters/media-session.md:17`; `contracts/operations/v1alpha1/semantics.md:22`; `contracts/operations/v1alpha1/semantics.md:32`; `contracts/operations/v1alpha1/semantics.md:57`; `docs/adapters/grafana.md:110`; `docs/adapters/docker.md:22`; `docs/adapters/docker.md:25`; `docs/adapters/media-session.md:86`; `docs/adapters/kubernetes.md:17`.

## Dependencies and edit coordination

No semantic prerequisite is declared within this draft set.

Shared edit surfaces with `story:contracts-mutation-outcomes`, `story:contracts-idempotency-scope`, `story:contracts-federated-approval`, `story:contracts-refresh-coordination`, `story:contracts-credential-evidence`, `story:contracts-session-revocation`, `story:contracts-wire-compatibility`, `story:contracts-permission-budgets`, `story:contracts-anonymous-auth`, `story:contracts-mutation-classification`, `story:contracts-restart-idempotency`, `story:contracts-log-continuation`, `story:contracts-discovery-coverage`, `story:contracts-document-admission`, `story:contracts-host-composition`, `story:contracts-discovery-profiles`, `story:contracts-acquisition-profiles`, `story:contracts-media-controls`, `story:contracts-supported-vocabulary`, `story:contracts-persistence-ownership`, `story:contracts-tenant-header`, `story:contracts-mutation-visibility`. These collisions require serialized edits or a coordinated integration owner; the stories are not a pre-approved concurrent wave. Semantic dependencies do not stand in for edit-collision records. Each story owns only its listed findings, not sibling outcomes on the same file.

## Boundary and modeling

This local-only story completes textual semantics, adapter-owned specifications, conformance scenarios and relevant ESS shape validation under the operator's current specification-completion authorization. No runtime/provider/codec implementation or adapter runtime expansion is authorized. Local coordinator commits and managed worktrees are authorized; external publication is not. New typed entities or relations must use the ESS workflow, preserving UNMAPPED implementation obligations explicitly. Existing review/source snapshots remain immutable.

The approved parallel assignment is specification:spec-completion-parallel-20260909. Root alone writes planning records and integrates shared-file patches. Original file:line citations above retain their historical db1c329 baseline; current writable ownership follows the dispatch brief and current machine scope, including the adapter-owned layout checkpoint 8e1836c.
