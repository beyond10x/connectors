---
format: aep.planning-md/3
id: story:bridge-drop-waits-for-dispatched-batch
kind: story
status: implemented
title: A refused metadata handle does not leave a batch running after its lock is released
relations:
- serves: vision:independent-contract-adapters
- informed_by: story:metadata-invoke-cost-flat-in-store-size
- informed_by: specification:milestone-acceleration-20261002
scope:
- confidence: cited
  path: crates/connectors-host/src/local/metadata.rs
- confidence: cited
  path: crates/connectors-host/src/local/metadata/er.rs
- confidence: cited
  path: crates/connectors-host/src/local/metadata/metamorphic_tests.rs
- confidence: cited
  path: ess/domains/cli.yaml
revision: 16
transitions:
- {from: "draft", to: "proposed", at: "2026-10-02T11:48:32Z", actor: "human:timo", revision: 9, decided_on: {"recorded":{"review_outcome":1}}}
- {from: "proposed", to: "active", at: "2026-10-02T11:48:32Z", actor: "human:timo", revision: 10, decided_on: {"recorded":{"review_outcome":1}}}
- {from: "active", to: "implemented", at: "2026-10-02T13:37:11Z", actor: "human:timo", revision: 16, decided_on: {"recorded":{"test_result":3,"review_outcome":1}}}
---
## Observed
The 2026-10-02 investigation saw an OutcomeUnknown batch continue after its handle released the lifecycle lock. Skipping reopen verification then regressed recovery. See story:metadata-invoke-cost-flat-in-store-size and crates/connectors-host/src/local/metadata.rs:206.

## Outcome
Preserve exclusive metadata ownership until dispatched work finishes or is proven unable to commit. Timeout does not prove cancellation. Verify upstream shutdown guarantees before choosing its API.

## Acceptance

A metadata-timeout-safety conformance result passes every Required conformance case using the fault injection in Verification and model, demonstrating safe recovery across the timed-out handle's ownership boundary, including legacy import before the authority is installed in the handle.

## Verification and model
These are proposed runtime conformance case names, not existing-test claims. Bind them to contracts/cli/v1alpha1/semantics.md and existing ess/domains/mutations.yaml, idempotency.yaml and clock.yaml. Reproduce on base with synchronized fault injection: hold a worker past deadline, attempt a second owner, release the worker, inspect commits and provider request counts. Retain corruption detection and reopen checks.
If upstream cannot prove completion safely, record that missing guarantee; do not implement an unbounded Drop wait or mark this story done.

## Scope

Confirmed by the implementor and unit commit 91511f4379579d0605af80420d5c34ea4874d162:
crates/connectors-host/src/local/metadata.rs,
crates/connectors-host/src/local/metadata/er.rs, and
crates/connectors-host/src/local/metadata/metamorphic_tests.rs (previously inferred).
The integrated gate additionally required a comment-only source-line citation
correction in ess/domains/cli.yaml; compiled ESS JSON is byte-identical.
Serialize with the clock experiment and upstream metadata adoption. No upstream
code or model semantics changed.

## Required conformance cases

The named conformance cases batch-timeout-retains-ownership, queued-batch-cancelled-before-release and next-invoke-after-unknown-outcome prove no dispatched batch commits after ownership release, a subsequent invoke recovers without a duplicate provider effect, and uncertainty remains until observed. The additional metadata_timeout_safety_legacy_import_retains_ownership case covers the same boundary while Metadata::adopt_er imports legacy state before installing its authority. The implementation's exact test names and outputs are retained in its report; all four must pass. This closes a reachable import path discovered during review rather than excluding it as migration-only.

## Integrated gate citation correction

The first integrated gate at 05f9ce537f7b8e0de3064a82d1ec5f985a62a473 exited 1
because its workspace-test step exited 101. The existing
`the_er_rs_citation_lands_on_the_cursor_expiry_path` assertion found that
`ess/domains/cli.yaml` still cited er.rs:3610-3612 after this unit moved the
cursor-removal branch to 3760-3762. The correction updates that comment only.
ESS 0.45.0 validates all 24 files and compiled JSON before/after is byte-identical.
No model semantics, test assertion or generated source is changed.
The retained first-run log is integration-gate-1.log in coordinator scratch.
