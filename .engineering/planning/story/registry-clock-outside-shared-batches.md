---
format: aep.planning-md/3
id: story:registry-clock-outside-shared-batches
kind: story
status: implemented
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
revision: 11
transitions:
- {from: "draft", to: "proposed", at: "2026-10-02T13:56:03Z", actor: "human:timo", revision: 8, decided_on: {"recorded":{"review_outcome":1}}}
- {from: "proposed", to: "active", at: "2026-10-02T13:56:03Z", actor: "human:timo", revision: 9, decided_on: {"recorded":{"review_outcome":1}}}
- {from: "active", to: "implemented", at: "2026-10-02T15:16:40Z", actor: "human:timo", revision: 10, decided_on: {"recorded":{"test_result":1,"review_outcome":1,"verification":1}}, correlation: "wave-20261002c-clock-experiment"}
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

## Completed rejection experiment — 2026-10-02

The single candidate was rejected after all required measurements completed.
Verification-report:registry-clock-mitigation-decision-20261002 and
review-result:registry-clock-experiment-20261002 retain the decision and independent
review. Public logs, exact candidate patch, fixture identities and numeric matrix
are in docs/evidence/clock-experiment-20261002/. Implemented means this bounded
investigation delivered its required rejection decision; it does not mean the
candidate was adopted or the upstream performance milestone completed.

Baseline 9/9 invariants; candidate 8/9, with the same-millisecond revision fence
returning MetadataUnavailable instead of ConcurrentRevision. All 15 measured
invokes and 3 warmups per profile passed. Candidate large/small median ratio
25.9185 exceeds the accepted maximum2. Production source was restored exactly.
No second candidate, deadline extension or verification weakening occurred.
Entity Runtime issue51 remains OPEN; M1 remains outstanding.

Scope confirmation from the worker: metadata.rs owns admission/lifecycle/ER
activation; registry/tests.rs contains the exact deciding clock/fence/race tests.
These previously inferred reading owners are now source-checked. The sole actual
source edit was the one-condition candidate in metadata/er.rs, then reversed;
public evidence and AEP records are the only retained repository changes.

Both worker and independent reviewer released their leases after all subprocesses
exited. The task-owned Cargo target was cleaned through cargo clean. Recovery
archive cb26c-clock was verified at
$HOME/.local/state/worktree/archives/connectors/cb26c-clock, holding the original
frozen report, logs, hashes, retained patch and saved task fixtures. No candidate
binary is retained after reproducible build-output cleanup; its measured hash is
in the archive. The experiment produced no production commit to integrate.
