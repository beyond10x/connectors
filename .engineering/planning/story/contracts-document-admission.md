---
format: aep.planning-md/1
id: story:contracts-document-admission
kind: story
status: active
title: Define trusted document membership before content access
tags:
- P1
- contract-review
relations:
- decomposes: epic:contract-semantics-remediation
- informed_by: specification:contract-review-intake-20260908
scope:
- confidence: cited
  path: adapters/README.md
- confidence: cited
  path: adapters/atlassian
- confidence: cited
  path: contracts/datasources/records/v1alpha1/semantics.md
- confidence: cited
  path: contracts/service/compatibility.md
- confidence: cited
  path: docs/design.md
- confidence: inferred
  path: docs/evidence/datasource-semantics-20260908
- confidence: inferred
  path: ess/domains/datasource_reads.yaml
- confidence: cited
  path: ess/system.yaml
revision: 12
---
## Context

Priority: **P2**. Sources: `F14` in `specification:contract-review-intake-20260908`.

An opaque unseen page ID does not locally prove space membership before the required no-provider-call admission boundary.

Baseline for all file:line citations is local commit `db1c329` unless an old-repository path is named. Source findings F01–F15 are preserved in `review-result:contract-semantics-20260908`; E01–E33 are preserved in `review-result:contract-docs-external-20260908`.

## Required revision

Select bounded provider-scoped retrieval for unseen IDs and moved objects, distinguish admitted metadata lookup from body access, and specify current membership/cache authority and the real observation boundary. Preserve Confluence storage and Jira native custom fields with coherent identity/version, exact representation-specific byte accounting and aggregate body-bearing list bounds.

## Acceptance

Every unseen, aliased, moved, cached, missing or malformed document follows an explicit bounded admission/retrieval/disclosure trace. No caller assertion, key prefix, old membership or cache TTL grants body access; the selected provider-native representation remains useful and truthful when bounded.

## Verification scenarios

- Unseen page ID in an allowed versus forbidden space.
- Page moves after cached membership; content retrieval rechecks the declared freshness/binding rule.
- Authorization lookup, if selected, has explicit admitted destination, data and work bounds.

Verification is a textual scenario audit: record the chosen rule and expected observation/refusal for every scenario, cross-check all cited documents, and link the resulting review evidence before claiming the story complete. Runtime conformance implementation is subsequent work.

## Scope

- cited: `contracts/datasources/records/v1alpha1/semantics.md`
- cited: `adapters/atlassian`

Historical initial-review source locations: `contracts/datasources/records/v1alpha1/semantics.md:34`; `contracts/datasources/records/v1alpha1/semantics.md:63`; `contracts/datasources/records/v1alpha1/semantics.md:83`; `docs/adapters/atlassian.md:106`.

## Dependencies and edit coordination

No semantic prerequisite is declared within this draft set.

Shared edit surfaces with `story:contracts-idempotency-scope`, `story:contracts-refresh-coordination`, `story:contracts-wire-compatibility`, `story:contracts-evidence-precision`, `story:contracts-mutation-visibility`. These collisions require serialized edits or a coordinated integration owner; the stories are not a pre-approved concurrent wave. Semantic dependencies do not stand in for edit-collision records. Each story owns only its listed findings, not sibling outcomes on the same file.

## Boundary and modeling

The operator authorized semantic hardening with ESS and local checkpoints. This supersedes the original text-only/no-ESS/no-commit boundary. This interactive run changes specifications, evidence and planning only; no runtime, public codec, adapter-kind schema implementation, extra adapter implementation or external publication is authorized. Existing declarations remain in ess/domains/declarations.yaml. Generic document body facts/decisions live in ess/domains/datasource_reads.yaml. Native representation/scope/CQL selection values live in adapters/atlassian/spec/ess/domains/documents.yaml; grammar and provider interpretation remain binding obligations. Models use pinned ESS 0.20.0; schema compilation does not execute native behavior. Provider content and bounded ephemeral read observations are values, not invented persistent entities. Unsupported predicates and unresolved persistent relations stay explicitly UNMAPPED.

Root is the sole tracked writer in the primary checkout under the repository's single-agent rule. The two already authorized independent reviewers write only their separate ignored evidence. Shared ESS, design and evidence surfaces are serialized between these two stories and the previously named siblings; this is not a new concurrent implementation wave.

## Current hardening intake — 2026-09-08

The dd08cfd checkpoint is the current normative baseline. Both independent initial reviews require revision; their exact bodies are review-result:datasource-semantics-a-initial-20260908 and review-result:datasource-semantics-b-initial-20260908. This story exclusively owns LD-A-04–08 and LD-B-05–08 in that round, in addition to its original source finding. Initial provider supplements add evidence, not another verdict or a fixed outcome. Complete frozen inputs, raw reports and provider byte/hash provenance are retained in docs/evidence/datasource-semantics-20260908/.

Current priority is P1 because the deeper review found scope/correctness failures with that severity. The original intake priority remains historical P2. No review finding is closed at activation. Completion requires revised normative semantics, honest ESS/verification limits and independent final review; schema acceptance alone does not execute authority, ordering, provider or storage behavior.

## Adapter ownership correction — 2026-09-09

story:extractable-adapter-contract-ownership relocates native authority under adapters/<owner>/contracts and authored spec/ess. Shared root contracts retain envelopes and common obligations. Initial review snapshots retain original paths and bytes. This relocation does not close any datasource finding or claim final semantic approval; this story remains active pending its own scenario/disposition evidence and independent review.
