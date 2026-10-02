---
format: aep.planning-md/3
id: story:metadata-invoke-cost-flat-in-store-size
kind: story
status: active
title: Per-invoke metadata cost does not grow with the store
relations:
- serves: vision:independent-contract-adapters
scope:
- confidence: inferred
  path: crates/connectors-host/src/local/metadata.rs
- confidence: inferred
  path: crates/connectors-host/src/local/metadata/er.rs
- confidence: inferred
  path: crates/connectors-host/src/local/registry.rs
- confidence: inferred
  path: crates/connectors-host/src/local/registry/tests.rs
revision: 7
transitions:
- {from: "draft", to: "proposed", at: "2026-10-01T22:09:40Z", actor: "human:timo", revision: 4}
- {from: "proposed", to: "active", at: "2026-10-01T22:09:41Z", actor: "human:timo", revision: 5}
---
## Source

Split from story:metadata-open-is-not-a-full-replay on 2026-09-29. That unit cut a read invoke from 11 full
replays to 1 (600-event store: 26.1 s → 5.5 s median per invoke, release build, measured by its implementor
with `read_invoke_metadata_time`). What still grows with the store, measured by the same implementor:

- Entity Runtime batch execution: a 20-member batch took 1.5 s at 120 events and 14.5 s at 920, the same for
  a reused and a fresh handle, so the cost is in the library (entity-runtime 0.25.1), not in the host.
- `RecordedProviderFacade::read_history` goes through `capture_model`, a full native capture: 190–250 ms per
  verified read at 600 events against 280–310 ms for a full replay.
- About 5 verifying reads per invoke and the owner catching up after the CLI's clock write (1.6–1.7 s at 600).
- A store of 6,000 events could not be built on base or unit: batch execution passes the 30 s bridge deadline
  (`OutcomeUnknown`) between about 920 and 1,200 events.

So the consumer's ~700-invoke workload will still reach the admission bound, later than on 0.15.1.

## Acceptance

- Entity Runtime exposes subject-scoped reads and batch execution whose cost does not depend on the number of
  unrelated recorded events (an upstream change; the host adopts the release that carries it).
- Per-invoke metadata time is measured at 600 and 6,000 events with the same command and stays within a stated
  bound; the ~700-invoke workload completes with no admission `timeout`.

## Not taken

A fingerprint over event rows in place of the verifying read on an unchanged store would drop about 5 captures
per invoke, but corruption inside blob bytes would no longer be caught on a read-only reopen. Declined by the
coordinator on 2026-09-29: integrity checks stay as they are.

## Observed on an operator store (2026-09-30)

The default local store (`~/.local/state/connectors`, 837 events, 29.5 MB, 24 MB of it in
`connectors_er_blobs`) with 0.20.0: `connections describe` took 6.9 s and `connections revalidate` 30.09 s,
answering `outcome_unknown` at `publication`; new connections stayed `pending`. The same adapter
configuration revalidated in a 3.7 MB store. The store was moved to `connectors.grown-20260930` and
re-initialised to restore use.

## Observed on a fresh store (2026-10-01, 0.22.0)

The default store was reset at 22:49. One consumer run of paged Jira reads failed after 313 s with `timeout` on an
invoke and `outcome_unknown` on a revalidate. Measured in the same store:

| events | `issues.search` (maxResults 1) |
|---|---|
| about 50 (after reconnect) | 1.75 s |
| 577 (8 minutes later) | 13.5 s |

One invoke appended 10 events (567 to 577). At about 22 ms per recorded event, a paged source (one search per 100
issues plus one comment read per issue) reaches the 30 s bridge deadline after roughly 100 invokes, so no paged
source can complete. The consumer has parked its Jira run until this story ships.

## Diagnosed (2026-10-02, wave 20261002a)

Release build, stores grown by real read invokes (`read_invoke_cost_by_store_size` in
`crates/connectors-host/src/local/registry/store_cost_tests.rs`): 578 ms per invoke at 55 events, 12.4 s at 601,
40.2 s at 1,203 (69x; target 2x). `execute_batch` is 62-87% of it, facade reads (each a full capture) most of the
rest, host-only work 0.4%. The cause is inside Entity Runtime 0.25.1: upstream-blocker:entity-runtime-batch-closure-cost
(beyond10x/entity-runtime#51). Host follow-ups: story:registry-clock-outside-shared-batches (a mitigation to test),
story:bridge-drop-waits-for-dispatched-batch. A host change that skipped the reopen verification saved 1.3% and
regressed recovery; it was reverted.
