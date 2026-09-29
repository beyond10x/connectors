---
format: aep.planning-md/3
id: story:metadata-open-is-not-a-full-replay
kind: story
status: implemented
title: A metadata open or write does not replay and re-verify the whole store
relations:
- serves: vision:independent-contract-adapters
scope:
- confidence: cited
  path: crates/connectors-host/src/local/metadata.rs
- confidence: cited
  path: crates/connectors-host/src/local/metadata/er.rs
- confidence: cited
  path: crates/connectors-host/src/local/mod.rs
- confidence: cited
  path: crates/connectors-host/src/local/registry/tests.rs
- confidence: cited
  path: crates/connectors-host/src/local/security_replay_tests.rs
- confidence: cited
  path: crates/connectors-host/tests/metadata_reopen_adversary.rs
- confidence: cited
  path: docs/development.md
revision: 11
transitions:
- {from: "draft", to: "proposed", at: "2026-09-29T15:46:15Z", actor: "human:timo", revision: 6}
- {from: "proposed", to: "active", at: "2026-09-29T15:46:15Z", actor: "human:timo", revision: 7}
- {from: "active", to: "implemented", at: "2026-09-29T20:39:43Z", actor: "human:timo", revision: 11, decided_on: {"recorded":{"test_result":2,"review_outcome":2}}}
---
## Observed (2026-09-29, v0.15.1, the knowledge-ingest consumer's store)

- Store: `metadata.sqlite3` 21 MB; 667 `connectors_er_events`, 2,377 blobs totalling 16,245,658 bytes,
  0 rows in `connectors_er_snapshots`.
- Each `operations invoke` takes 26–31 s against GitLab calls of 3.6–4.5 s; the admission wait is
  `WAIT_BOUND` = 30 s (`crates/connectors-host/src/local/metadata.rs:35`), so about half the calls fail
  with `timeout` at admission. Earlier the same day, with a smaller store, 40+ invokes took 259 s in total.
- The owner held 39–70 % CPU and 2.6 GB RSS.
- `perf record` of the owner (PID 2179360, 60 s, 1,825 samples; then 20 s): 84 % of samples on the
  `entity-eventlog` thread; `sha2::sha256::compress256` 16–18 %; the rest mostly `serde_json::Value`
  clone/serialize/deserialize, `entity_core::definition::FieldDefinition` deserialize and BTreeMap
  clone/drop; `eventlog_core::capture::validate_captured_digest` present.

## Code path (read, not measured per call)

- Every registry operation opens `Metadata` from the path (`Metadata::update`, `update_observation`,
  `inspect` in `registry.rs:287-289`, `registry/observation.rs:60`, `registry/revalidation.rs:112`).
- Each open runs `er::open` (`er.rs:1526`), which takes `complete_snapshot` of the whole store
  (`er.rs:1544`).
- Each `er::persist` takes another `complete_snapshot` after its append (`er.rs:2691`) to check the
  receipt and rebuild the baseline.

Inferred, not yet measured: one invoke performs several opens and persists, each costing a full
replay with digest verification of every stored record, so per-call cost grows linearly with the
store and total cost quadratically with its history.

## Acceptance

- A test builds a store with at least 600 recorded events and asserts the number of full-store
  replays per `operations invoke` (a counter on `complete_snapshot`), red on the current code.
- After the change, one invoke against that store performs at most one full replay, or none when
  the owner already holds the authority open; per-invoke metadata time stays within a stated bound
  independent of the event count (measured at 600 and 6,000 events).
- The receipt check in `er::persist` reads only the subjects the batch wrote.
- The consumer's workload (about 700 invokes) completes with no `timeout` at admission.

## Second observation: a fresh store reaches the same limit (2026-09-29, 15:17–15:32)

- The consumer started from an empty state directory and ran sequential invokes (describe per
  operation, then tags, releases and events per project, paged); after 916 s and an estimated
  110–150 invokes (not counted per call), one invoke failed `timeout` at admission.
- That store then held 2,725 blobs, 14,646,120 bytes (read here from a backup copy at about 15:35);
  `metadata.sqlite3` was 20,029,440 bytes, about the size of the first store when it failed.
- So a store grows by roughly 20 records and 100 KB per invoke (derived from the estimate above), and
  the admission bound is reached at about the same store size both times. This supports the
  store-size dependence; per-call times were not logged.

## Decided for the wave (coordinator, 2026-09-29)

- Every full replay goes through `er::complete_snapshot` (`er.rs:1668`); the replay counter sits
  there. The count test is in-crate (`registry/tests.rs` or `er.rs` tests) and drives the registry
  transaction sequence one invoke performs; it does not need the CLI process. A read of the call
  paths estimates about 12 full replays per invoke today (about 7 opens, 5 persists), not measured.
- Correctness first: the CLI process writes to the same store outside the owner
  (`apps/connectors/src/local/operations.rs:72`), so any state kept across opens must catch up on
  what other handles appended before it is used, and the digest checks at `metadata.rs:602` and
  `:630` must keep holding.
- The receipt check in `er::persist` reads the batch's subjects with the per-subject history read
  (`RecordedProviderFacade::read_history`) instead of a full snapshot.
- The timing bound is recorded as measured numbers at 600 and 6,000 events, base against unit, with
  the same command.

## Split (coordinator, 2026-09-29)

The unit met the replay-count acceptance: one read invoke does 1 full replay instead of 11 (red test
`a_read_invoke_against_a_grown_store`: 11), and the 600-event store went from 26.1 s to 5.5 s median per invoke.
The lines on a bound independent of event count, the 6,000-event measurement and the ~700-invoke workload
move to story:metadata-invoke-cost-flat-in-store-size: the remaining growth is in Entity Runtime batch
execution and `read_history`, measured there. This story is done when the replay count holds and its reviews
are routed.
