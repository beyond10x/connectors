---
format: aep.planning-md/1
id: story:independent-review-remediation
kind: story
status: implemented
title: Resolve independent review defects and semantic documentation drift
tags:
- P1
relations:
- derived_from: specification:contract-driven-connectors-design
- informed_by: verification-report:independent-repository-followup-20260908
revision: 5
---
## Context

The operator requested “good, solve them” after verification-report:independent-repository-followup-20260908 reproduced both major findings and reconciled all 24 independent-review rows at 492c20c. This interactive, local-only story owns remediation of those confirmed defects and documentation inconsistencies, preserving qualified findings and existing versioning decisions. It does not add private-CA federation, public-ingress hosting, new adapters or an auth runtime.

## Acceptance

The two reproduced major failures and confirmed smaller defects have regression evidence for their corrected behavior, the normal pinned-ESS gate passes, and a source-ID disposition accounts for all 24 findings without claiming unsupported capabilities or settling the wire-version question.

## Implementation and verification

- B-F01: serialize executable fixture writes and probes within the affected test harness; retain ordinary default-thread tests and resolver refusal behavior; run a fixed 100-run stress cohort.
- B-F02: bound connection establishment, query execution and cleanup; send PostgreSQL cancellation with the captured endpoint/TLS binding on timeout or caller-future drop; retain a bounded driver lifetime. Test cancellation framing and setup/parameter bytes; rerun the live timeout bypass and confirm server cleanup. Document cancellation uncertainty during partitions or process loss.
- A-M7/B-F03: respect the caller target base for the MSRV build and verify isolation through the full gate.
- B-F07: derive the federation descriptor schema from the existing Rust configuration owner and check admission against it.
- B-F12: malformed public Spec values return a refusal instead of panicking; exercise missing operation and field cases.
- A-N1/B-F09: strengthen outcome, SQL protocol, host-bound, credential, federation and discovery tests where the named behavior is present. Cover SQL TLS and schema discovery with focused fixtures/live checks; keep limits and optional capabilities explicit.
- A-M1–M5/M8, A-N2–N4, B-F04/F05/F08/F10/F11: correct current documentation/planning drift, reconcile the inventory, explain limits and trusted plaintext access, and preserve already-strict error decoding. G-source portability (A-M6/B-F06) was fixed in 492c20c.
- Preserve historical raw reviews. Record final per-finding outcomes and evidence in a new response; do not rewrite the investigation into a claim its old baseline already passed.

## Scope

Cited: adapters/sql/src/lib.rs; adapters/sql/tests/protocol.rs; crates/connectors-spec/src/toolchain.rs; crates/connectors-spec/src/v2.rs; crates/connectors-spec/tests/generation.rs; crates/connectors-build/src/gate.rs; crates/connectors-host/src/federation.rs; crates/connectors-host/src/schema.rs; crates/connectors-host/src/server.rs; crates/connectors-host/src/credentials.rs; crates/connectors-host/tests/service.rs; crates/connectors-core/tests/wire.rs; adapters/kubernetes/tests/provider.rs; contracts/service/v1alpha1/semantics.md; contracts/service/v1alpha2/semantics.md; contracts/operations/v1alpha1/semantics.md; contracts/README.md; docs/design.md; docs/cli-migration-v1-to-v2.md; adapters/gitlab/upstream/README.md.

Inferred support: affected Cargo manifests/lockfile for runtime schema derivation and test utilities; a focused Rust live SQL regression harness; docs/evidence/independent-review-fixes-20260908; AEP story/completion bodies through the CLI only.

## Boundaries and coordination

Single-agent changes take place directly in primary main, per AGENTS.md; no managed trees are created or adopted. Existing unrelated trees stay untouched. This is one story, with no decomposition or concurrent scheduling; a planning-critic panel is not applicable. The user’s implementation request authorizes advancing this story through the ordinary proposed/active states; implementation status requires real completion evidence.

These changes repair existing Rust behavior and schema reflection. Existing typed configuration owners are ServiceConfig, CredentialRef and FederationConfig; no new domain identity or relation is introduced. The existing ESS model at ess/system.yaml and authored scenarios remain the specification baseline, validated with the pinned 0.20.0 binary. A proposed wire label is not decided here. The existing contracts-documentation-index story retains ownership of broader prerequisite-dependent adapter coverage; record the corrected inventory overlap against it without falsely completing its other work.

## Completion evidence — 2026-09-08

The implementation and documentation corrections are complete. [The final response](../../../docs/independent-review-response-2026-09-08.md) gives one disposition for each of the 24 source findings: 21 fixed rows (two already fixed in 492c20c), two explicit documented limits, and one verified existing decoder behavior. SQL deadline/drop cancellation passes wire and live PostgreSQL checks, including TLS cancellation after CA-file replacement and bounded cleanup when the server does not respond. The fixture stress cohort passes 100/100 with default threads.

The full gate passed on its first run (exit 0): 50 tests, 0 failures/ignored, formatting, Clippy with warnings denied, generation drift, dependency boundaries and Rust 1.88 all targets. ESS 0.20.0 validates seven files and compiles 92 declarations/169 scenarios (34 authored), zero refusals. All 1,991 default MSRV files remain untouched; the caller-selected target contains its own 1,997 MSRV files. Gate logs, live/stress/inventory results and verified source digests are retained under docs/evidence/independent-review-fixes-20260908.

AEP validates 60 artifacts with 18 findings-block warnings; source reviews remain verbatim. Original E13 is closed by the inventory correction, while the broader documentation-index story stays draft for E22 and its prerequisite-dependent coverage. No new entity, wire version, private-CA federation capability, public-ingress implementation, adapter or Atlas/consumer integration was introduced. This was one single-agent story; no decomposition or reviewer dispatch occurred.
