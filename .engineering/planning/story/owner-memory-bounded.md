---
format: aep.planning-md/3
id: story:owner-memory-bounded
kind: story
status: draft
title: The owner's memory is bounded by the store it holds
relations:
- decomposes: epic:connector-probe-20261006
scope:
- confidence: inferred
  path: Cargo.lock
- confidence: inferred
  path: Cargo.toml
- confidence: inferred
  path: crates/connectors-host/src/local/metadata/er.rs
- confidence: inferred
  path: crates/connectors-host/src/local/owner/transport.rs
- confidence: cited
  path: crates/connectors-host/src/local/registry/store_cost_tests.rs
revision: 10
---
## Observed

beyond10x/connectors#103: an operator's owner held 0.8 GB after one revalidate on a 29 MB store and
rose to 4.0 GB over 8 small reads. Measured 2026-10-06 (peak RSS of
`read_invoke_cost_by_store_size` at 601 events, release build): 1,991 MB on v0.28.0 (Entity
Runtime 0.25.1), 1,012 MB on v0.30.0 (0.26.0, provider-tracked capture). The CLI strips `MALLOC_*`
from the owner's environment.

## Acceptance

- A heap profile of one open and one read invoke at 600 events names the allocations that hold the
  peak, recorded on beyond10x/connectors#103.
- `read_invoke_cost_by_store_size` reports peak RSS per store size, each size measured in its own
  process (so `ru_maxrss` does not carry over from a larger size).
- Peak RSS at 600 events is at most half of v0.30.0's 1,012 MB (2026-10-06, `peak.sh` over
  `read_invoke_cost_by_store_size`, 601 events), and peak RSS at 1,200 events is at most twice the
  peak at 600, both measured by that command in one release build; the base commit's numbers at
  600 and 1,200 are recorded first.

## Scope

Derived 2026-10-06 by `aep:story-scoper`. **Cited** = read from the story or the tree; **inferred** =
a reading that could be wrong.

- **Files:** `crates/connectors-host/src/local/registry/store_cost_tests.rs:174`
  (`read_invoke_cost_by_store_size`, gains a peak-RSS column beside `process_cpu` at :79) — cited
- **Also likely:** `crates/connectors-host/src/local/metadata/er.rs:444` `ErAuthority` (`baseline`,
  `streams`), `:486` `IDLE` / `IDLE_LIMIT` (up to 4 kept handles per process), `:1813` `resynchronize`
  (clones every `complete_snapshot` history into `Observed`) — inferred, these hold whole-store state
- **Also likely:** `crates/connectors-host/src/local/owner/transport.rs:706` (`.env_clear()` on the
  owner spawn, where `MALLOC_*` is dropped), `Cargo.toml` / `Cargo.lock` (Entity Runtime pin) —
  inferred, only if allocator tuning or an upstream fix is adopted
- **Confidence:** medium; the fix site depends on the heap profile the first acceptance item asks for
- **Would collide with:** `story:metadata-invoke-cost-flat-in-store-size` and
  `story:registry-clock-floor-growth` (`metadata/er.rs`, `store_cost_tests.rs`)
- **Safety fact:** `grown_stores` grows every size in one test process and `ru_maxrss` only rises,
  so a per-size peak column is honest only if each size runs in its own process
  (`store_cost_tests.rs:88-125`, `:174-190`) — unproven
