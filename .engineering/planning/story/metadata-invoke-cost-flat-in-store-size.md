---
format: aep.planning-md/3
id: story:metadata-invoke-cost-flat-in-store-size
kind: story
status: draft
title: Per-invoke metadata cost does not grow with the store
relations:
- serves: vision:independent-contract-adapters
revision: 2
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
