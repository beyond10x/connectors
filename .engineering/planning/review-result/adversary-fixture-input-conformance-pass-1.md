---
format: aep.planning-md/3
id: review-result:adversary-fixture-input-conformance-pass-1
kind: review-result
status: active
title: Adversary pass 1 on fixture-input conformance
relations:
- reviews: story:fixture-input-conformance
revision: 1
---
unit: story:fixture-input-conformance, uncommitted tree wave0929-fixture on base d215b3569
verdict: CONFIRMED (1 red case, warning)
cases: executed 106→111, red 1
origin: introduced 5 / pre-existing 0 / undecided 0
wrote-outside-worktree: scratch/adversary (7.7M)
needs-coordinator: no

Cases added in crates/connectors-build/tests/metadata_conformance_fixture_adversary.rs, each running a
planted one-scenario suite through `connectors-build metadata-conformance run`:
a_well_formed_anchor_passes, a_planted_wrong_outcome_fails_the_run, an_unknown_fixture_name_errors_the_run,
an_invariant_violation_is_failed_through_the_runner (green); an_unobservable_invariant_is_failed_as_a_violation_is
(red: `{"error":0,"failed":0,"passed":0,"skipped":0,"total":1,"unsupported":1}`).

Coordinator routing: finding 1 fixed (InvariantUnobservable scored failed with InvariantViolation);
finding 5 fixed (fingerprint fixture input_digest is 64 lowercase hex). Findings 2–4 are notes kept as
recorded; after the fix `cargo test -p connectors-build` exit 0, adversary file 5 passed.

```findings
[
  {"file": "crates/connectors-build/src/metadata_conformance.rs", "line": 240, "category": "acceptance", "severity": "warning", "verdict": "CONFIRMED", "origin": "introduced", "message": "kernel_refusal scores InvariantViolation as failed but leaves InvariantUnobservable, which the kernel also discards as a violation, as unsupported"},
  {"file": "crates/connectors-build/src/metadata_conformance.rs", "line": 240, "category": "judgement", "severity": "note", "verdict": "INFEASIBLE", "origin": "introduced", "message": "a scenario asserting only absence now passes when its command breaks an invariant (unsupported at base), but no synthesized scenario reaches this"},
  {"file": "crates/connectors-build/src/metadata_conformance.rs", "line": 105, "category": "mutant", "severity": "note", "verdict": "CONFIRMED", "origin": "introduced", "message": "the optional arm of restore_nulls is never exercised: no suite input and no unit case passes a null through an Optional-wrapped approval subject"},
  {"file": "ess/domains/execution_audit.yaml", "line": 137, "category": "judgement", "severity": "note", "verdict": "CONFIRMED", "origin": "introduced", "message": "fixtures pin every AcknowledgeAnchor run to admitted_execution/invoke/admission and every ReserveKey run to direct, so the describe, early-refusal and federated invariants are never exercised by the gate"},
  {"file": "crates/connectors-build/src/metadata_conformance.rs", "line": 316, "category": "judgement", "severity": "note", "verdict": "CONFIRMED", "origin": "introduced", "message": "the canonical-request-fingerprint fixture's input_digest is a value the host's production validator rejects, though no Entity Runtime rule reads it"}
]
```
