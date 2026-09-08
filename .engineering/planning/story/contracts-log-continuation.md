---
format: aep.planning-md/1
id: story:contracts-log-continuation
kind: story
status: active
title: Make bounded log continuation truthful at timestamp ties
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
  path: adapters/docker/contracts/logs
- confidence: cited
  path: adapters/kubernetes/contracts/logs
- confidence: cited
  path: adapters/loki
- confidence: cited
  path: contracts/auth/capability/v1alpha1/http-headers.md
- confidence: cited
  path: contracts/datasources/logs/v1alpha1/semantics.md
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
revision: 11
---
## Context

Priority: **P2**. Sources: `F12` in `specification:contract-review-intake-20260908`.

Timestamp-only continuation can omit entries or repeat forever when a tie exceeds the page limit.

Baseline for all file:line citations is local commit `db1c329` unless an old-repository path is named. Source findings F01–F15 are preserved in `review-result:contract-semantics-20260908`; E01–E33 are preserved in `review-result:contract-docs-external-20260908`.

## Required revision

Specify exact per-profile time selectors, native LogQL scope admission, occurrence-preserving bounded continuation, full immutable cursor context and current authorization, ordering, exhaustion and every truncation cause. Keep useful native query and pagination capability without promising recovery of an unobservable timestamp tie.

## Acceptance

The selected initial/resume protocol makes bounded progress through every retained occurrence, including identical lines, and reports a truthful terminal partial result when the provider limit prevents exhaustive continuation. Provider scope admission, source work, malformed input/output and current cache/cursor authority have explicit decisions.

## Verification scenarios

- 1,001 entries at one timestamp with page size 1,000, including identical lines across streams.
- Inclusive and exclusive edge behavior are explicit.
- Provider without reliable tie continuation returns a truthful partial/non-resumable result.

Verification is a textual scenario audit: record the chosen rule and expected observation/refusal for every scenario, cross-check all cited documents, and link the resulting review evidence before claiming the story complete. Runtime conformance implementation is subsequent work.

## Scope

- cited: `contracts/datasources/logs/v1alpha1/semantics.md`
- cited: `adapters/loki`
- cited: `adapters/kubernetes/contracts/logs`
- cited: `adapters/docker/contracts/logs`
- cited: `contracts/auth/capability/v1alpha1/http-headers.md`

Historical initial-review source locations: `contracts/datasources/logs/v1alpha1/semantics.md:67`; `contracts/datasources/logs/v1alpha1/semantics.md:75`; `contracts/datasources/logs/v1alpha1/semantics.md:94`.

## Dependencies and edit coordination

No semantic prerequisite is declared within this draft set.

Shared edit surfaces with `story:contracts-wire-compatibility`, `story:contracts-anonymous-auth`, `story:contracts-discovery-coverage`, `story:contracts-host-composition`, `story:contracts-discovery-profiles`, `story:contracts-acquisition-profiles`, `story:contracts-evidence-precision`, `story:contracts-tenant-header`. These collisions require serialized edits or a coordinated integration owner; the stories are not a pre-approved concurrent wave. Semantic dependencies do not stand in for edit-collision records. Each story owns only its listed findings, not sibling outcomes on the same file.

## Boundary and modeling

The operator authorized semantic hardening with ESS and local checkpoints. This supersedes the original text-only/no-ESS/no-commit boundary. This interactive run changes specifications, evidence and planning only; no runtime, public codec, adapter-kind schema implementation, extra adapter implementation or external publication is authorized. Existing declarations remain in ess/domains/declarations.yaml. Generic log facts/decisions live in ess/domains/datasource_reads.yaml. Native LogQL selection/equalities live in adapters/loki/spec/ess/domains/reads.yaml; pod/container selectors and decoders remain explicit adapter binding obligations. Models use pinned ESS 0.20.0; schema compilation does not execute native behavior. Provider content and bounded ephemeral read observations are values, not invented persistent entities. Unsupported predicates and unresolved persistent relations stay explicitly UNMAPPED.

Root is the sole tracked writer in the primary checkout under the repository's single-agent rule. The two already authorized independent reviewers write only their separate ignored evidence. Shared ESS, design and evidence surfaces are serialized between these two stories and the previously named siblings; this is not a new concurrent implementation wave.

## Current hardening intake — 2026-09-08

The dd08cfd checkpoint is the current normative baseline. Both independent initial reviews require revision; their exact bodies are review-result:datasource-semantics-a-initial-20260908 and review-result:datasource-semantics-b-initial-20260908. This story exclusively owns LD-A-01–03 and LD-B-01–04 in that round, in addition to its original source finding. Initial provider supplements add evidence, not another verdict or a fixed outcome. Complete frozen inputs, raw reports and provider byte/hash provenance are retained in docs/evidence/datasource-semantics-20260908/.

Current priority is P1 because the deeper review found scope/correctness failures with that severity. The original intake priority remains historical P2. No review finding is closed at activation. Completion requires revised normative semantics, honest ESS/verification limits and independent final review; schema acceptance alone does not execute authority, ordering, provider or storage behavior.

## Adapter ownership correction — 2026-09-09

story:extractable-adapter-contract-ownership relocates native authority under adapters/<owner>/contracts and authored spec/ess. Shared root contracts retain envelopes and common obligations. Initial review snapshots retain original paths and bytes. This relocation does not close any datasource finding or claim final semantic approval; this story remains active pending its own scenario/disposition evidence and independent review.
