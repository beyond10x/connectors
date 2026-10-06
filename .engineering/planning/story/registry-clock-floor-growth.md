---
format: aep.planning-md/3
id: story:registry-clock-floor-growth
kind: story
status: draft
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
revision: 6
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
