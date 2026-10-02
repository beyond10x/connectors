---
format: aep.planning-md/3
id: story:registry-clock-outside-shared-batches
kind: story
status: draft
title: Investigate keeping the registry clock out of every registry batch
relations:
- serves: vision:independent-contract-adapters
- informed_by: story:metadata-invoke-cost-flat-in-store-size
- depends_on: story:bridge-drop-waits-for-dispatched-batch
- informed_by: specification:milestone-acceleration-20261002
scope:
- confidence: inferred
  path: crates/connectors-host/src/local/metadata.rs
- confidence: cited
  path: crates/connectors-host/src/local/metadata/er.rs
- confidence: cited
  path: crates/connectors-host/src/local/registry.rs
- confidence: cited
  path: crates/connectors-host/src/local/registry/store_cost_tests.rs
- confidence: inferred
  path: crates/connectors-host/src/local/registry/tests.rs
revision: 7
---
## Hypothesis
Entity Runtime 0.25.1 follows co-batched subjects through the registry clock, growing its closure across the store. Isolating that clock might bound the work, but must preserve the atomic revision guard in crates/connectors-host/src/local/metadata/er.rs:3215.

## Acceptance

A registry-clock-mitigation-decision record is valid only when it includes every numeric and invariant check below and selects adoption exactly when all adoption thresholds hold, otherwise rejection.

## Bounded experiment
One baseline, one candidate, one decision; reassess after one working day. Use release builds, identical starting fixtures and five measured invokes after warmup; report failures as well as timing. Retain the 30-second bridge bound and blob verification.
Adopt only if the 1200-event median is at most twice the 50-event median, all five attempts succeed and invariants hold. Otherwise reject, retain evidence and keep upstream critical. Even success does not complete story:metadata-invoke-cost-flat-in-store-size: 600/6000 events and the 700-invoke workload remain required.

## Model and scope
Existing owners: ess/domains/clock.yaml, auth_bindings.yaml and docs/design.md section 31. Proposed scenario names above need executable runtime bindings; the existing a_reopen_after_its_own_write_refuses_every_blob_altered_since anchors corruption verification. No new entity.
Cited: crates/connectors-host/src/local/registry/store_cost_tests.rs, crates/connectors-host/src/local/metadata/er.rs, crates/connectors-host/src/local/registry.rs. Inferred: crates/connectors-host/src/local/metadata.rs and crates/connectors-host/src/local/registry/tests.rs.
Depends on story:bridge-drop-waits-for-dispatched-batch for safe unknown-outcome cleanup. Serialize with upstream adoption and other metadata changes.

## Required conformance cases

The named conformance case registry-clock-mitigation-decision records adoption or rejection against the existing batch_cost_by_subject and read_invoke_cost_by_store_size probes at requested event counts 50,600,1200 (actual counts recorded), while clock-floor-monotonic, registry-observation-revision-fenced and reopen-corruption-refused preserve current semantics.
