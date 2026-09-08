---
format: aep.planning-md/1
id: story:contracts-supported-vocabulary
kind: story
status: implemented
title: Record support and disposition for reserved vocabulary
tags:
- P3
- contract-review
relations:
- decomposes: epic:contract-semantics-remediation
- informed_by: specification:contract-review-intake-20260908
scope:
- confidence: cited
  path: adapters/prometheus/contracts/series/v1alpha1/semantics.md
- confidence: cited
  path: adapters/sip/contracts/dial/v1alpha1/semantics.md
- confidence: cited
  path: adapters/sip/design.md
- confidence: cited
  path: contracts/auth/capability/v1alpha1/semantics.md
- confidence: cited
  path: contracts/auth/profile/v1alpha1/semantics.md
- confidence: cited
  path: contracts/datasources/series/v1alpha1/semantics.md
- confidence: cited
  path: contracts/discovery/resources/v1alpha1/semantics.md
- confidence: cited
  path: contracts/media/v1alpha1/semantics.md
revision: 10
---
## Context

Priority: **P3**. Sources: `E27` in `specification:contract-review-intake-20260908`.

Reserved names may imply support or create unnecessary abstractions, but reservation alone does not prove an architectural violation.

Baseline for all file:line citations is local commit `db1c329` unless an old-repository path is named. Source findings F01–F15 are preserved in `review-result:contract-semantics-20260908`; E01–E33 are preserved in `review-result:contract-docs-external-20260908`.

## Required revision

Inventory the cited reserved names and record supported, explicitly reserved/refused, or removed disposition with a requirement/source rationale. Do not delete a name needed by a selected adapter solely because the reviewer proposed blanket removal.

## Acceptance

After revision, each cited reserved name has an explicit support disposition that cannot be mistaken for advertised implementation.

## Verification scenarios

- http_signing, oauth2_password, hold/transfer, instant/labels and address locator each have an explicit outcome.
- Selected provider requirements justify retention; reserved-only names remain unadvertised/refused.
- Unsupported semantics are not modeled as implemented.

Verification is a textual scenario audit: record the chosen rule and expected observation/refusal for every scenario, cross-check all cited documents, and link the resulting review evidence before claiming the story complete. Runtime conformance implementation is subsequent work.

## Scope

- cited: `adapters/prometheus/contracts/series/v1alpha1/semantics.md`
- cited: `adapters/sip/contracts/dial/v1alpha1/semantics.md`
- cited: `adapters/sip/design.md`
- cited: `contracts/auth/capability/v1alpha1/semantics.md`
- cited: `contracts/auth/profile/v1alpha1/semantics.md`
- cited: `contracts/datasources/series/v1alpha1/semantics.md`
- cited: `contracts/discovery/resources/v1alpha1/semantics.md`
- cited: `contracts/media/v1alpha1/semantics.md`

Current scope follows the adapter-owned layout and specification:spec-completion-parallel-20260909. Shared files are coordinator-integrated or assigned to one worker; stories are serialized where required.

Historical Source locations: `contracts/auth/profile/v1alpha1/semantics.md:64`; `contracts/auth/profile/v1alpha1/semantics.md:65`; `contracts/auth/profile/v1alpha1/semantics.md:103`; `contracts/auth/capability/v1alpha1/semantics.md:12`; `contracts/media/v1alpha1/semantics.md:13`; `contracts/datasources/series/v1alpha1/semantics.md:12`; `contracts/discovery/resources/v1alpha1/semantics.md:62`; `docs/design.md:49`.

## Dependencies and edit coordination

No semantic prerequisite is declared within this draft set.

Shared edit surfaces with `story:contracts-credential-evidence`, `story:contracts-session-revocation`, `story:contracts-wire-compatibility`, `story:contracts-anonymous-auth`, `story:contracts-discovery-coverage`, `story:contracts-read-refresh-retry`, `story:contracts-discovery-profiles`, `story:contracts-acquisition-profiles`, `story:contracts-evidence-precision`, `story:contracts-media-controls`, `story:contracts-persistence-ownership`. These collisions require serialized edits or a coordinated integration owner; the stories are not a pre-approved concurrent wave. Semantic dependencies do not stand in for edit-collision records. Each story owns only its listed findings, not sibling outcomes on the same file.

## Boundary and modeling

This local-only story completes textual semantics, adapter-owned specifications, conformance scenarios and relevant ESS shape validation under the operator's current specification-completion authorization. No runtime/provider/codec implementation or adapter runtime expansion is authorized. Local coordinator commits and managed worktrees are authorized; external publication is not. New typed entities or relations must use the ESS workflow, preserving UNMAPPED implementation obligations explicitly. Existing review/source snapshots remain immutable.

The approved parallel assignment is specification:spec-completion-parallel-20260909. Root alone writes planning records and integrates shared-file patches. Original file:line citations above retain their historical db1c329 baseline; current writable ownership follows the dispatch brief and current machine scope, including the adapter-owned layout checkpoint 8e1836c.

## Completion evidence — 2026-09-09

E27 is fixed through explicit reserved/refused auth/media/series names and removal/refusal of address from resource locator vocabulary. Reserved enum/name presence cannot advertise support. See docs/evidence/supported-vocabulary-20260909/verification.md.

Both independent final reviews spec-completion-a-final-20260909 and spec-completion-b-final-20260909 approve the selected specification scope at d1dc83f5d600816c699db07dd843f079ded0e72b. The complete checkpoint, unchanged reports and exact source archive are retained in [checkpoint](../../../docs/evidence/spec-completion-20260909/checkpoint.md). The full repository gate including MSRV 1.88 passed with 58 Rust tests; ESS synthesis/compilation and textual scenarios do not execute runtime conformance. No runtime implementation or public codec change was added in this wave.

The approved parallel plan supersedes earlier single-primary-writer dispatch wording: isolated workers changed only their assigned sources, coordinator alone wrote planning/shared joins. This story is complete for its enumerated specification findings; the overall goal remains active for the separately scoped persistent ESS model closure. Original immutable review bodies and historical source references remain unchanged.