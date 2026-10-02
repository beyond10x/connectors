---
format: aep.planning-md/3
id: story:bridge-drop-waits-for-dispatched-batch
kind: story
status: active
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
- confidence: inferred
  path: crates/connectors-host/src/local/metadata/metamorphic_tests.rs
revision: 10
transitions:
- {from: "draft", to: "proposed", at: "2026-10-02T11:48:32Z", actor: "human:timo", revision: 9, decided_on: {"recorded":{"review_outcome":1}}}
- {from: "proposed", to: "active", at: "2026-10-02T11:48:32Z", actor: "human:timo", revision: 10, decided_on: {"recorded":{"review_outcome":1}}}
---
## Observed
The 2026-10-02 investigation saw an OutcomeUnknown batch continue after its handle released the lifecycle lock. Skipping reopen verification then regressed recovery. See story:metadata-invoke-cost-flat-in-store-size and crates/connectors-host/src/local/metadata.rs:206.

## Outcome
Preserve exclusive metadata ownership until dispatched work finishes or is proven unable to commit. Timeout does not prove cancellation. Verify upstream shutdown guarantees before choosing its API.

## Acceptance

A metadata-timeout-safety conformance result passes all three Required conformance cases using the fault injection in Verification and model, demonstrating safe recovery across the timed-out handle's ownership boundary.

## Verification and model
These are proposed runtime conformance case names, not existing-test claims. Bind them to contracts/cli/v1alpha1/semantics.md and existing ess/domains/mutations.yaml, idempotency.yaml and clock.yaml. Reproduce on base with synchronized fault injection: hold a worker past deadline, attempt a second owner, release the worker, inspect commits and provider request counts. Retain corruption detection and reopen checks.
If upstream cannot prove completion safely, record that missing guarantee; do not implement an unbounded Drop wait or mark this story done.

## Scope
Cited: crates/connectors-host/src/local/metadata.rs and crates/connectors-host/src/local/metadata/er.rs. Inferred tests: crates/connectors-host/src/local/metadata/metamorphic_tests.rs.
Serialize with story:registry-clock-outside-shared-batches and story:metadata-invoke-cost-flat-in-store-size. No upstream edits in this unit. Changed model semantics require ESS authoring first.

## Required conformance cases

The named conformance cases batch-timeout-retains-ownership, queued-batch-cancelled-before-release and next-invoke-after-unknown-outcome prove no dispatched batch commits after ownership release, a subsequent invoke recovers without a duplicate provider effect, and uncertainty remains until observed.
