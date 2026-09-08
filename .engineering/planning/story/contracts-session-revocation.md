---
format: aep.planning-md/1
id: story:contracts-session-revocation
kind: story
status: implemented
title: Define bounded traffic cessation and terminal media reasons
tags:
- P1
- contract-review
relations:
- decomposes: epic:contract-semantics-remediation
- informed_by: specification:contract-review-intake-20260908
scope:
- confidence: cited
  path: contracts/auth/capability/v1alpha1/semantics.md
- confidence: cited
  path: contracts/media/v1alpha1/semantics.md
- confidence: inferred
  path: contracts/sessions/v1alpha1/scenarios/
- confidence: cited
  path: contracts/sessions/v1alpha1/semantics.md
- confidence: inferred
  path: contracts/sessions/v1alpha1/verification.md
- confidence: cited
  path: docs/adapters/media-session.md
- confidence: cited
  path: docs/evidence/spec-stabilization-20260908/
- confidence: inferred
  path: ess/domains/sessions.yaml
- confidence: cited
  path: ess/system.yaml
revision: 11
---
## Context

Priority: **P1**. Sources: `F06`, `E20` in `specification:contract-review-intake-20260908`.

Entering closing does not bound continuing media access; media_incompatible is absent from the sessions reason vocabulary while media_overload is already present.

Baseline for all file:line citations is local commit `db1c329` unless an old-repository path is named. Source findings F01–F15 are preserved in `review-result:contract-semantics-20260908`; E01–E33 are preserved in `review-result:contract-docs-external-20260908`.

## Required revision

Define a single revocation/closing traffic policy with bounded queue drain and teardown, covering direct paths and unresponsive peers; align shared terminal reasons. Numeric bounds must be selected or explicitly classified, not inferred from maximum call duration.

## Acceptance

After revision, the session termination table defines a bounded data cutoff and a valid terminal reason for every listed revocation or media-failure scenario.

## Verification scenarios

- Peer ignores close → data cessation within the declared bound independent of peer cooperation.
- Queued input/output at revocation has explicit treatment; direct paths enforce the same authority cutoff.
- Incompatible negotiation and overload use reasons admitted by the shared sessions vocabulary.

Verification is a textual scenario audit: record the chosen rule and expected observation/refusal for every scenario, cross-check all cited documents, and link the resulting review evidence before claiming the story complete. Runtime conformance implementation is subsequent work.

## Scope

- cited: `contracts/sessions/v1alpha1/semantics.md`
- cited: `contracts/media/v1alpha1/semantics.md`
- cited: `docs/adapters/media-session.md`

Source locations: `contracts/sessions/v1alpha1/semantics.md:84`; `contracts/sessions/v1alpha1/semantics.md:101`; `contracts/media/v1alpha1/semantics.md:62`; `contracts/media/v1alpha1/semantics.md:67`; `docs/adapters/media-session.md:89`.

## Dependencies and edit coordination

No semantic prerequisite is declared within this draft set.

Shared edit surfaces with `story:contracts-wire-compatibility`, `story:contracts-mutation-classification`, `story:contracts-evidence-precision`, `story:contracts-media-controls`, `story:contracts-supported-vocabulary`. These collisions require serialized edits or a coordinated integration owner; the stories are not a pre-approved concurrent wave. Semantic dependencies do not stand in for edit-collision records. Each story owns only its listed findings, not sibling outcomes on the same file.

## Boundary and modeling

This interactive, local-only hardening story revises the session/media semantics and validates their ESS model under the operator's specification-hardening authorization. It writes no runtime implementation, wire codec, generated adapter schema, provider adapter or external integration. Single-agent edits use the primary checkout, following the repository's explicit worktree override. Existing managed trees remain untouched.

The new session lifecycle has its typed home in `ess/domains/sessions.yaml`, registered in `ess/system.yaml`. The model describes supervisor decisions and refusal states. Connection and establishment-authority references remain opaque with UNMAPPED relation markers until those owners are modeled. Enforcement clocks, queue/device cutoff, trusted lease issuance, cross-endpoint fan-out, immutable first-terminal assignment and resource accounting remain named executable obligations rather than claims made by scenario compilation.

Additional scope: `ess/system.yaml`, `ess/domains/sessions.yaml`, `contracts/sessions/v1alpha1/scenarios/`, `contracts/sessions/v1alpha1/verification.md`, and the session-authority cross-reference in `contracts/auth/capability/v1alpha1/semantics.md`. No new decomposition or concurrent implementation wave is being created. The existing four-critic decomposition remains the governing plan.

## Completion evidence — 2026-09-08

F06 and E20 are fixed at the semantic-contract/model level. `contracts/sessions/v1alpha1/semantics.md` §4.1 defines the cutoff, live-lease, zero-data-drain and teardown policy, covering direct/relay/device paths and peer failure. `media_incompatible` joins `media_overload` in the shared terminal vocabulary. The model and 13 authored obligations are in `ess/domains/sessions.yaml` and `contracts/sessions/v1alpha1/scenarios/`.

`contracts/sessions/v1alpha1/verification.md` records the full scenario audit, structural ESS results and explicit runtime obligations. Both final independent reviewers approve with no residual findings (`review-result:sessions-semantics-r2-20260908`, `review-result:sessions-model-r2-20260908`). First-pass findings and counterexamples are retained; terminal-denial latching, Closing-to-Lost continuity, deadline clamping and compilation-limit wording were corrected. A separate declaration/trace/deadline audit passes 13 scenarios / 77 acts; ESS author/synthesize does not execute these traces. Full gate exits 0 with 50 existing Rust tests and MSRV 1.88; explicit session author/synthesis exits 0 with 13 authored / 201 compiled, 0 refusals. The normal gate does not yet collect the new authored directory, so those explicit commands remain required.

Tracked evidence: `docs/evidence/spec-stabilization-20260908/`. Runtime session tests executed: 0. No runtime implementation, public wire expansion, external publication or managed-tree mutation was made. New session model/scenario paths were confirmed when authored; their initial inferred scope was accurate. Connection/SessionAuthority owner relations and real timing, authority, field assignment, queue/device and resource algorithms remain explicitly UNMAPPED for later binding work.
