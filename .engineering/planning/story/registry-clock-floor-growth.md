---
format: aep.planning-md/3
id: story:registry-clock-floor-growth
kind: story
status: active
title: The registry clock floor no longer records an event on every command
relations:
- decomposes: epic:connector-probe-20261006
- depends_on: story:dependency-refresh-20261006
- serves: vision:independent-contract-adapters
scope:
- confidence: inferred
  path: contracts/service/clock.md
- confidence: cited
  path: crates/connectors-host/src/local/metadata/er.rs
- confidence: inferred
  path: crates/connectors-host/src/local/metadata/metamorphic_tests.rs
- confidence: cited
  path: crates/connectors-host/src/local/registry.rs
- confidence: cited
  path: crates/connectors-host/src/local/registry/store_cost_tests.rs
- confidence: inferred
  path: crates/connectors-host/src/local/registry/tests.rs
- confidence: inferred
  path: docs/design.md
- confidence: inferred
  path: docs/local-er-metadata.md
- confidence: cited
  path: ess/domains/clock.yaml
revision: 10
transitions:
- {from: "draft", to: "proposed", at: "2026-10-06T10:22:14Z", actor: "human:timo", revision: 7, decided_on: {"recorded":{"review_outcome":3}}}
- {from: "proposed", to: "active", at: "2026-10-06T10:22:15Z", actor: "human:timo", revision: 8, decided_on: {"recorded":{"review_outcome":3}}}
---
## Observed

2026-10-06, an operator store grown from 1,022 to 2,797 events in one day: 236 of the last 400 events
(59%) are the subject `connectors.clock.LocalClockFloor` / `s:registry`. Every registry transaction
writes `registry_clock.last_seen_ms = now` (`crates/connectors-host/src/local/registry.rs:313-325`)
and the projection records the advanced floor in Entity Runtime with its batch, record and request
blobs; `ess/domains/clock.yaml` notes the host re-records even an equal value. Analysis on
beyond10x/connectors#101.

## Constraint

The check the floor serves is strict: a transaction whose clock reading is below `last_seen_ms` is
refused (`registry.rs:318`). A lease-ahead floor (record now + delta, write again once the clock
passes it) was rejected on 2026-10-06 because it accepts a regression of up to delta.

## Acceptance

- A design record decides which of the SQLite `registry_clock` row and the Entity Runtime floor is
  authoritative after a rebuild, and how often the floor must reach Entity Runtime, without
  admitting a clock regression the current check refuses (`registry.rs:330`), including after a
  rebuild that refills the row from the Entity Runtime floor (`er.rs:2661-2668`).
- `read_invoke_cost_by_store_size` reports `connectors.clock.LocalClockFloor` events appended per
  read invoke as its own count. On the story's base commit the same command gives the baseline
  count; after the change the count is at most half of it, with
  `production_clock_samples_after_locking_and_still_rejects_regression` and
  `a_reopen_after_its_own_write_refuses_every_blob_altered_since` unchanged and green.

## Scope

Derived 2026-10-06 by `aep:story-scoper` at connectors `80bee2fb58`. Every line is **cited** (read
from the story or the tree) or **inferred** (a reading that could be wrong).

- **Primary surface:** `crates/connectors-host/src/local` (the registry transaction and the metadata
  Entity Runtime projection) — cited
- **Files:** `crates/connectors-host/src/local/registry.rs:316-336` (floor read at 323-329, strict
  refusal `now_sql < previous` at 330, unconditional `UPDATE registry_clock` at 332-336; the
  Observed section's `:313-325` and `:318` predate it) — cited
- **Files:** `crates/connectors-host/src/local/metadata/er.rs:3313-3343`, where the registry floor
  gets an `AdvanceLocalClockFloor` even when unchanged (`ess/domains/clock.yaml:75-76`) — cited
- **Files:** `crates/connectors-host/src/local/registry/store_cost_tests.rs:174`
  (`read_invoke_cost_by_store_size`) — cited
- **Files:** `ess/domains/clock.yaml:69-84` — cited
- **Also likely:** `er.rs:2387-2393, 2517-2520, 2661-2668` (a rebuild deletes the SQLite
  `registry_clock` row and refills it from the Entity Runtime floor); `metadata/metamorphic_tests.rs:830,878`;
  `registry/tests.rs:395` `production_clock_samples_after_locking_and_still_rejects_regression` and
  `store_cost_tests.rs:262` `a_reopen_after_its_own_write_refuses_every_blob_altered_since` (must
  stay green); `docs/local-er-metadata.md:127-129`, `docs/design.md` §31, `contracts/service/clock.md:36-38`
  — inferred
- **Confidence:** high
- **Would collide with:** any unit touching `local/metadata/er.rs`, `local/registry.rs`,
  `local/registry/store_cost_tests.rs` or `ess/domains/clock.yaml`
- **Safety fact:** the strict refusal reads only the SQLite row inside one `IMMEDIATE` transaction
  (`registry.rs:323-331`). Recording the floor less often in Entity Runtime weakens the check only
  on a rebuild, where `er.rs:2392` deletes the row and `er.rs:2661-2668` refills it from the Entity
  Runtime floor, which could then admit a regression of up to the lag — unproven

## Design record (decided 2026-10-06)

Facts (Scope): the strict refusal reads only the SQLite `registry_clock.last_seen_ms` row inside one
`IMMEDIATE` transaction (`registry.rs:323-331`); the Entity Runtime `LocalClockFloor` matters only
on a rebuild, which deletes the row and refills it from the Entity Runtime floor
(`er.rs:2392`, `er.rs:2661-2668`).

Decision:

- `registry_clock` gains `recorded_ms`, the value the Entity Runtime floor holds; the
  `LocalClockFloor` / `s:registry` projection reads `recorded_ms`, not `last_seen_ms`.
- Every registry transaction still writes `last_seen_ms = now` and still refuses `now <
  last_seen_ms` (unchanged check, no Entity Runtime event). It sets `recorded_ms = now` only when
  `now > recorded_ms + Δ`, Δ = 60 s; only then does the projection record a floor advance.
- Invariant after every transaction: `last_seen_ms <= recorded_ms + Δ`.
- A rebuild refills `recorded_ms` from the Entity Runtime floor and `last_seen_ms = recorded_ms + Δ`,
  which is at least every `last_seen_ms` ever committed, so no regression the current check refuses
  is admitted after a rebuild either. Cost: after a rebuild, transactions whose clock reads below
  the refilled `last_seen_ms` are refused for at most Δ.
- Authority: the SQLite row is authoritative while it exists; the Entity Runtime floor is
  authoritative for a rebuild, read through the Δ bound.

Rejected: a lease-ahead floor written only when the clock passes it (2026-10-06): it accepts a
regression of up to the lease between commands, which the current check refuses.

## Design amendment (decided 2026-10-06, supersedes the "Design record" above)

Finding (implementor, 2026-10-06): the SQLite `registry_clock` row is not durable. The projection is
an in-memory connection (`er.rs:2359` `Connection::open_in_memory`) built again from Entity Runtime
by `build_projection` on every registry transaction (a probe of two transactions counted two
builds). Every command is therefore "after a rebuild", and the refill rule above refuses every
command within Δ of the last recorded floor: emulated, `cargo test -p connectors-host --lib
local::registry` gave 0 passed, 43 failed. Between processes Entity Runtime is today the only
durable home of the floor, so a strict cross-process check needs a record on every command unless
the floor gets another durable home.

Decision: the floor gets a durable home outside Entity Runtime.

- A floor file beside the metadata lock in the connectors state directory holds `last_seen_ms`.
  It is read and written only while the metadata lifecycle lock is held, written by write-to-temp,
  fsync, rename.
- Every registry transaction refuses `now < max(file floor, Entity Runtime floor)` (the strict
  check, unchanged in meaning) and writes the file floor `= now` before its Entity Runtime batch
  commits. A failed commit leaves the file higher than needed, which only refuses more.
- The Entity Runtime `LocalClockFloor` / `s:registry` floor advances only when `now > recorded + Δ`,
  Δ = 60 s.
- Missing or unreadable floor file (first run, deleted, corrupt): the floor is the Entity Runtime
  floor `+ Δ`, which is at least every `now` ever committed. Cost: refusals for at most Δ after the
  file is lost.
- A floor file rolled back by hand can admit a regression of at most Δ below the last committed
  `now`, because the Entity Runtime floor still bounds it.
- The same-millisecond fence (`er.rs:3313-3343`) keeps its concurrency guarantee; whether it can
  stop emitting `AdvanceLocalClockFloor` for an unchanged floor is the implementor's call, proven by
  `prepared_same_millisecond_observation_refuses_stale_registry_state` staying green.

Rejected 2026-10-06: the `recorded_ms` refill design above (the row it relies on is not durable);
a bounded regression (lease-ahead, rejected earlier); not advancing on read invokes (a read invoke
appends 7 events and is not read-only at the store).
