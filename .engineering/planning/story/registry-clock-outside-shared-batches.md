---
format: aep.planning-md/3
id: story:registry-clock-outside-shared-batches
kind: story
status: draft
title: Investigate keeping the registry clock out of every registry batch
relations:
- serves: vision:independent-contract-adapters
- informed_by: story:metadata-invoke-cost-flat-in-store-size
revision: 1
---
## Hypothesis (not verified)

Entity Runtime 0.25.1's scoped read follows every subject that shares a batch with a record it reads
(upstream-blocker:entity-runtime-batch-closure-cost). The host puts the registry clock subject in every registry
batch, so the clock's batch closure is the whole store, and every batch that names the clock reads everything. If
the clock were written in its own batch, or its role were held by a field on the subjects a batch already names,
the closure of an ordinary batch would stay near its own subjects.

## Acceptance

- The probe `batch_cost_by_subject` (`crates/connectors-host/src/local/registry/store_cost_tests.rs`) is run against a
  host variant that does not put the clock in shared batches, at 55, 601 and 1,203 events, and the numbers are
  recorded here.
- A decision is recorded: adopt (with the invariants the clock currently provides restated and held by tests), or
  reject with the reason.
