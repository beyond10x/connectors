---
format: aep.planning-md/1
id: story:contracts-wire-compatibility
kind: story
status: implemented
title: Define versioned compatibility for proposed contract extensions
tags:
- P1
- contract-review
relations:
- decomposes: epic:contract-semantics-remediation
- informed_by: specification:contract-review-intake-20260908
- depends_on: story:contracts-mutation-outcomes
scope:
- confidence: cited
  path: contracts/README.md
- confidence: cited
  path: contracts/auth/acquisition/v1alpha1/semantics.md
- confidence: cited
  path: contracts/auth/capability/v1alpha1/semantics.md
- confidence: cited
  path: contracts/auth/connection/v1alpha1/semantics.md
- confidence: cited
  path: contracts/auth/custody/v1alpha1/semantics.md
- confidence: cited
  path: contracts/auth/evidence/v1alpha1/semantics.md
- confidence: cited
  path: contracts/auth/profile/v1alpha1/semantics.md
- confidence: cited
  path: contracts/catalog/v1alpha1/semantics.md
- confidence: cited
  path: contracts/datasources/logs/v1alpha1/semantics.md
- confidence: cited
  path: contracts/datasources/records/v1alpha1/semantics.md
- confidence: cited
  path: contracts/datasources/series/v1alpha1/semantics.md
- confidence: cited
  path: contracts/discovery/mediated_route/v1alpha1/semantics.md
- confidence: cited
  path: contracts/discovery/resources/v1alpha1/semantics.md
- confidence: cited
  path: contracts/media/v1alpha1/semantics.md
- confidence: cited
  path: contracts/operations/v1alpha1/semantics.md
- confidence: inferred
  path: contracts/service/compatibility.md
- confidence: cited
  path: contracts/service/v1alpha1/semantics.md
- confidence: cited
  path: contracts/service/v1alpha2/semantics.md
- confidence: cited
  path: contracts/sessions/v1alpha1/semantics.md
- confidence: cited
  path: docs/cli-migration-v1-to-v2.md
- confidence: inferred
  path: docs/evidence/wire-compatibility-20260908/
- confidence: cited
  path: docs/stack-integration-proposal.md
- confidence: inferred
  path: ess/domains/service_wire.yaml
- confidence: cited
  path: ess/system.yaml
revision: 9
---
## Context

Priority: **P1**. Sources: `E02` in `specification:contract-review-intake-20260908`.

New descriptor fields and closed error codes are described beside no-wire-change claims; strict v1alpha1 readers reject them.

Baseline for all file:line citations is local commit `db1c329` unless an old-repository path is named. Source findings F01–F15 are preserved in `review-result:contract-semantics-20260908`; E01–E33 are preserved in `review-result:contract-docs-external-20260908`.

## Required revision

Produce one compatibility matrix covering the proposed families and the already proposed v1alpha2 envelope; distinguish contract/profile versions from wire versions, define old-reader refusal and negotiation, and assign binding obligations. Preserve valid no-change claims only for explicitly unchanged configured/read profiles. Other stories own behavior; this story owns its transport/version disposition.

## Acceptance

After revision, every proposed public field, error and profile extension has an explicit wire/version disposition in the compatibility matrix.

## Verification scenarios

- Old reader/new descriptor and new error → declared refusal or explicitly supported representation.
- New reader/old profile retains old semantics; unsupported profile is not silently downgraded.
- Outcome classifications from contracts-mutation-outcomes have an unambiguous representable encoding.

Verification is a textual scenario audit: record the chosen rule and expected observation/refusal for every scenario, cross-check all cited documents, and link the resulting review evidence before claiming the story complete. Runtime conformance implementation is subsequent work.

## Scope

- cited: `contracts/service/v1alpha1/semantics.md`
- cited: `contracts/operations/v1alpha1/semantics.md`
- cited: `contracts/auth/connection/v1alpha1/semantics.md`
- cited: `contracts/auth/profile/v1alpha1/semantics.md`
- cited: `contracts/auth/acquisition/v1alpha1/semantics.md`
- cited: `contracts/auth/capability/v1alpha1/semantics.md`
- cited: `contracts/auth/evidence/v1alpha1/semantics.md`
- cited: `contracts/sessions/v1alpha1/semantics.md`
- cited: `contracts/media/v1alpha1/semantics.md`
- cited: `contracts/discovery/resources/v1alpha1/semantics.md`
- cited: `contracts/discovery/mediated_route/v1alpha1/semantics.md`
- cited: `contracts/datasources/records/v1alpha1/semantics.md`
- cited: `contracts/datasources/logs/v1alpha1/semantics.md`
- cited: `contracts/datasources/series/v1alpha1/semantics.md`

Source locations: `contracts/auth/capability/v1alpha1/semantics.md:99`; `contracts/auth/connection/v1alpha1/semantics.md:111`; `contracts/auth/profile/v1alpha1/semantics.md:110`; `contracts/sessions/v1alpha1/semantics.md:71`; `contracts/discovery/mediated_route/v1alpha1/semantics.md:55`; `contracts/operations/v1alpha1/semantics.md:144`; `crates/connectors-core/src/lib.rs:13`; `crates/connectors-core/src/lib.rs:31`; `crates/connectors-core/src/lib.rs:71`.

## Dependencies and edit coordination

Semantic prerequisites: `story:contracts-mutation-outcomes`.

Shared edit surfaces with `story:contracts-mutation-outcomes`, `story:contracts-idempotency-scope`, `story:contracts-federated-approval`, `story:contracts-refresh-coordination`, `story:contracts-credential-evidence`, `story:contracts-session-revocation`, `story:contracts-connection-readiness`, `story:contracts-permission-budgets`, `story:contracts-anonymous-auth`, `story:contracts-mutation-classification`, `story:contracts-restart-idempotency`, `story:contracts-log-continuation`, `story:contracts-discovery-coverage`, `story:contracts-document-admission`, `story:contracts-read-refresh-retry`, `story:contracts-host-composition`, `story:contracts-management-boundary`, `story:contracts-discovery-profiles`, `story:contracts-acquisition-profiles`, `story:contracts-evidence-precision`, `story:contracts-media-controls`, `story:contracts-supported-vocabulary`, `story:contracts-persistence-ownership`, `story:contracts-tenant-header`, `story:contracts-mutation-visibility`. These collisions require serialized edits or a coordinated integration owner; the stories are not a pre-approved concurrent wave. Semantic dependencies do not stand in for edit-collision records. Each story owns only its listed findings, not sibling outcomes on the same file.

## Boundary and modeling

The active specification-hardening goal authorizes this local semantic/ESS revision. It adds no runtime code, codec, adapter handler, generated adapter schema, external publication or consumer rollout. Single-agent edits use primary main under the repository override; independent reviewers remain read-only.

The compatibility owner is `contracts/service/compatibility.md`, with precise public observation values modeled in `ess/domains/service_wire.yaml` and registered in `ess/system.yaml`. This introduces values, not new persistent entities or guessed ownership. Existing effect knowledge comes from `ess/domains/mutations.yaml`; audit/grant persistence and exact delegated verification mechanics remain owned by their semantic dependencies, visibly unbound until settled.

Additional reviewed surfaces: `contracts/service/v1alpha2/semantics.md`, `contracts/catalog/v1alpha1/semantics.md`, `contracts/auth/custody/v1alpha1/semantics.md`, `contracts/README.md`, `docs/stack-integration-proposal.md`, `docs/cli-migration-v1-to-v2.md`, and verification evidence under `docs/evidence/wire-compatibility-20260908/`. All existing family documents are cross-checked against the same matrix. Service-wire version selection is separate from semantic-family and adapter-specification versions; no adapter/v1 or adapter/v2 file is changed. Other stories still own profile behavior and runtime bindings. This is not a new decomposition or implementation wave.

## Specification closure — E02

The authoritative owner is [service compatibility](../../../contracts/service/compatibility.md), covering all 15 proposed v1alpha1 family documents and the proposed governed service binding. It selects local v1alpha2 paths and strict fields independently of semantic-family and adapter-kind versions; preserves exact legacy-reader diagnostics; requires an off-by-default, invoke-enforced read projection; defines complete describe/invoke/error/audit/mutation observations; and assigns every extension to a selected payload, extended envelope, private port, independent reader or explicitly unbound transport. Generic read/mutation/page limits and safe catalog error semantics are reconciled across the owners.

Public values have a typed ESS home in `ess/domains/service_wire.yaml`, reusing the original mutation effect knowledge and AttemptId. No entity, persistence relation, decoder or provider runtime is invented. F03 delegation, protected callbacks, duplex/media codecs, real policy/audit persistence and dynamic enforcement remain explicit binding obligations with their own owners.

Two independent reviewers examined the initial draft, the corrected draft and final sources. All first/recheck findings were addressed, with original report bodies preserved in immutable review-result artifacts. [Verification](../../../docs/evidence/wire-compatibility-20260908/verification.md) records 68 executed current-decoder byte vectors, 27 proposed textual response vectors, the full gate (50 Rust tests, MSRV 1.88, format/lint/drift), ESS 0.20.0 validation of nine files, and separate session author/synthesis. No v1alpha2 codec or sequential runtime scenario execution is claimed. The original E02 can close at specification scope; sibling findings and the broader goal remain open.
