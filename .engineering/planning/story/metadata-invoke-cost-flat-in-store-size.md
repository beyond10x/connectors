---
format: aep.planning-md/3
id: story:metadata-invoke-cost-flat-in-store-size
kind: story
status: implemented
title: Per-invoke metadata cost does not grow with the store
relations:
- serves: vision:independent-contract-adapters
- decomposes: epic:connector-probe-20261006
- informed_by: upstream-blocker:er-batch-cost-superlinear
scope:
- confidence: cited
  path: Cargo.lock
- confidence: cited
  path: Cargo.toml
- confidence: inferred
  path: crates/connectors-host/src/local/metadata.rs
- confidence: inferred
  path: crates/connectors-host/src/local/metadata/er.rs
- confidence: inferred
  path: crates/connectors-host/src/local/registry.rs
- confidence: inferred
  path: crates/connectors-host/src/local/registry/tests.rs
revision: 17
transitions:
- {from: "draft", to: "proposed", at: "2026-10-01T22:09:40Z", actor: "human:timo", revision: 4}
- {from: "proposed", to: "active", at: "2026-10-01T22:09:41Z", actor: "human:timo", revision: 5}
- {from: "active", to: "implemented", at: "2026-10-08T06:29:54Z", actor: "human:timo", revision: 17, decided_on: {"recorded":{"test_result":1,"review_outcome":3,"verification":1}}}
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

Revised 2026-10-07: Entity Runtime 0.29.0 is pinned (`story:entity-runtime-029-pin`); its durable
open checkpoints are adopted as decided in `decision-blocker:store-checkpoints-one-way` (option C).

- Spec first: the store-creation behaviour and the new command are modelled in the CLI
  specification (`ess/domains/cli.yaml` and `apps/connectors/spec/cli.yaml`) and validated before
  the handler exists.
- `setup init` (and any path that creates a new metadata store) creates it with durable open
  checkpoints enabled.
- An explicit command enables checkpoints on an existing store. It states that the change is
  one-way (connectors 0.32.0 and earlier can no longer open the store) and refuses to run
  without a confirming flag; run twice it answers that the store is already enabled.
- Tests: a new store is created with checkpoints; an existing store (the 0.26.0 fixture of
  `tests/metadata_store_previous_pin.rs` restored) is enabled by the command and then opens from
  its checkpoint; a store without the flag is refused unchanged.
- `read_invoke_cost_by_store_size`, release build, on stores with checkpoints: the median
  per-invoke time at 6,000 events is at most twice the median at 600; a 700-invoke run
  (`CONNECTORS_STORE_COST_INVOKES=700`) completes with no admission `timeout` and no
  `outcome_unknown`.
- `CHANGELOG.md` names the command under Migration and the one-way change under Breaking.

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

## Adopted Entity Runtime 0.26.0 (2026-10-05)

beyond10x/entity-runtime#51 closed with Entity Runtime 0.26.0 (2026-10-03): opt-in
`CapturePolicy::ProviderTracked` reuses a fully verified observation while SQLite attests it
unchanged, or verifies only an appended suffix; every open still verifies the whole store. The host
now pins 0.26.0 (eventlog `6983cc2`, the revision 0.26.0 builds on) and opens the recorded store
with `RecordedProviderFacade::start_with_read_policy(.., CapturePolicy::ProviderTracked)`.

`read_invoke_cost_by_store_size`, release build, same command as the 2026-10-02 diagnosis, 5
measured invokes per store:

| events | 0.25.1 (2026-10-02) | 0.26.0 + ProviderTracked |
|---|---|---|
| 55 | 578 ms | 302 ms |
| 601 | 12.4 s | 1,255 ms |
| 1,203 | 40.2 s | 2,332 ms |

`er.execute_batch` is flat (77 ms at 55 events, 112 ms at 601, 114 ms at 1,203). What still grows is
`er.start` (75, 492, 963 ms), the complete verification every open performs, and `er.read_history`
(80, 504, 976 ms for 6 calls). Switching `read_history` to the scoped facade read was measured
slower (2,003 ms median at 601 events, 1 of 5 invokes failed at 1,203) and was not kept.

Integrity: with `ProviderTracked`, a raw edit of `metadata.sqlite3` that bypasses SQLite while an
owner holds the store is not seen until the next open; an open, and any SQL write from another
connection, still verifies.

Operator observation the same day: on the default store (1,022 events, 29 MB) and on a fresh
Zendesk store grown to 489 events, `connections revalidate` answered `outcome_unknown` at
`publication` after 30 s with the 0.27.0 and 0.28.0 CLIs.

## Integrity

Added 2026-10-07 (`decision-blocker:checkpoint-offline-edit-detection`, option B); revised
2026-10-08 after adversary pass 1 (`review-result:adversary-store-checkpoints-pass-1`, J1, R1) to
state what is built:

- Only the owner, holding a handle for its whole run, reads from a checkpoint without a complete
  read; it does one complete read at its start (`Metadata::start_owner`). Every other open (a
  command run without the owner, a second process, recovery) is a provider open followed by a
  complete read (`complete_snapshot`), which Entity Runtime 0.29.0 treats as complete
  verification (Entity Runtime 0.29.0 CHANGELOG and the `CapturePolicy::ProviderTracked` doc comment, `entity-eventlog` `adapter/tracked.rs:19-20`). A command run with the owner reads through
  it (`LocalOwnerCachedRequest`).
- Tests: an offline raw edit of the SQLite file is refused before the first command after it, on
  the owner path (edited while the owner is stopped, then started) and on the direct path.
- Accepted: inside a process that already holds a verified handle, a later open of the same file
  skips `PRAGMA quick_check`, so page damage written by a non-SQLite writer while that process
  runs is seen at the next fresh open, not before; damage in the `connectors_er_*` tables is still refused by the catch-up read (`crates/connectors-host/tests/pooled_open_scan_adversary.rs`). This matches the provider-tracked posture
  recorded on 2026-10-05.
- `CHANGELOG.md` and the store contract (`docs/local-er-metadata.md`,
  `contracts/cli/v1alpha1/semantics.md` §3) say which open verifies what.
- No disable command is offered.
