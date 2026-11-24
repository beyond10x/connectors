---
format: aep.planning-md/3
id: story:one-debug-only-override-helper-is-forwarded-to-the-owner-and-off-in-a-release-build
kind: story
status: draft
title: One debug-only override helper is forwarded to the owner and off in a release build
summary: A copied cfg(debug_assertions) pattern becomes one helper in connectors-sdk; the owner spawn forwards by prefix; one release-build test proves an override is off
relations:
- decomposes: story:the-gates-test-step-ends-within-120-s-on-the-forge
- informed_by: test-plan:where-the-gates-test-step-spends-its-time
- serves: vision:independent-contract-adapters
- depends_on: story:the-lane-slow-runs-the-ignored-timing-family
scope:
- confidence: cited
  path: apps/connectors/src/local/owner_stop.rs
- confidence: cited
  path: crates/connectors-host/src/local/owner/transport.rs
- confidence: cited
  path: crates/connectors-host/src/local/registry/lifecycle.rs
- confidence: cited
  path: crates/connectors-sdk/src/lib.rs
revision: 2
---
## Defect

Five stories of this set add a debug-only override of a production wait. The repository has no
helper for that. It has a pattern that is copied. Read at 0.78.1:

- `crates/connectors-host/src/local/owner/transport.rs` lines 1969 to 1979: `idle_bound()` has one
  definition under `#[cfg(debug_assertions)]` that reads `CONNECTORS_TEST_OWNER_IDLE_MS` and one
  under `#[cfg(not(debug_assertions))]` that returns `IDLE_EXIT` (600 s).
- `apps/connectors/src/local/owner_stop.rs` lines 18 to 29: `bound()` reads
  `CONNECTORS_TEST_OWNER_STOP_MS` the same way; production is `BOUND` (30 s).
- `crates/connectors-host/src/local/registry/lifecycle.rs` lines 695 to 704: `acquisition_ms()`
  reads `CONNECTORS_TEST_ACQUISITION_MS`; production is `ENTRY_MS`.
- `transport.rs` lines 1590 to 1597: the names `CONNECTORS_TEST_OWNER_START_DELAY_MS` and
  `CONNECTORS_TEST_OWNER_LAUNCH_DELAY_MS`. Their readers were not opened for this draft.

The stories would add about eight more copies.

A second gap: the CLI starts the owner with `.env_clear()` (`transport.rs` line 1670) and passes,
in a debug build, a fixed list of seven names (lines 1686 to 1698). An override that the owner reads
does not reach an owner the CLI started unless its name is added to that list.

A third gap: no test shows that a build without `debug_assertions` ignores an override. A debug
build cannot show it.

## Change

1. One helper in `crates/connectors-sdk/src/lib.rs`, hidden from the documentation:
   `debug_override_ms(name, production)`. In a build with `debug_assertions` it returns the
   milliseconds the environment variable `name` holds, or `production` when the variable is absent
   or no number. In a build without `debug_assertions` it returns `production` and reads no
   variable.
   The crate: `connectors-sdk` is the smallest crate that both `crates/connectors-host` and
   `apps/connectors` depend on (counted at 0.78.1: 493 lines under `src/`, against 1,794 in
   `connectors-oauth2` and 1,888 in `connectors-core`). It is also the crate the adapters build
   against. The other home would be `connectors-host` itself: `apps/connectors` depends on it
   (`apps/connectors/Cargo.toml`), and three of the four sites are there. The release test of
   point 4 would then need a release build of that crate's test binary.
2. The four sites above call the helper. Their variable names stay, so no test changes.
3. The owner spawn in `transport.rs` forwards by one rule, in a debug build only: every variable of
   the CLI's environment whose name starts with `CONNECTORS_TEST_OVERRIDE_`. The fixed list of
   seven names stays. Every override a story of this set adds carries that prefix, so no story
   edits the spawn.
4. One release-build test in `connectors-sdk`, compiled only without `debug_assertions`:
   `release_build_ignores_a_debug_override`. It runs itself as a child process with a variable set
   and asserts that the helper returns the production value. The lane `slow` runs it with
   `cargo test --release --locked -p connectors-sdk release_build_`, the second step of
   story:the-lane-slow-runs-the-ignored-timing-family.

## Decision beyond the plan

The plan says "debug-only" and asks for no proof. This story adds the proof of point 4. The reason:
an override that worked in a release build would be a production knob that shortens a safety wait
(the connect wait, the client drain, the recovery sweep). One test of the one helper covers every
override that goes through it. The five stories that carried this acceptance each (four-idle,
slow-owner, eleven, five-tests, seven-cli-shorten) no longer carry it.

## Acceptance

1. A test of `connectors-sdk` in the gate's test step: a child process with the variable set gets
   the variable's value from the helper; without the variable it gets the production value.
2. `cargo test --release --locked -p connectors-sdk release_build_` runs
   `release_build_ignores_a_debug_override` and it passes: with the variable set, the helper
   returns the production value. The lane `slow` runs this command.
3. A unit test of `transport.rs` on the function that picks the owner's environment: from an
   environment that holds `CONNECTORS_TEST_OVERRIDE_EXAMPLE_MS` and `CONNECTORS_TEST_EXAMPLE`, the
   owner gets the first and not the second. The seven names of the fixed list still pass.
4. `grep` finds no `#[cfg(not(debug_assertions))]` function that returns a wait constant in
   `transport.rs`, `owner_stop.rs` and `registry/lifecycle.rs`: the four sites call the helper.
5. The tests that set `CONNECTORS_TEST_OWNER_IDLE_MS`, `CONNECTORS_TEST_OWNER_STOP_MS`,
   `CONNECTORS_TEST_ACQUISITION_MS`, `CONNECTORS_TEST_OWNER_START_DELAY_MS` and
   `CONNECTORS_TEST_OWNER_LAUNCH_DELAY_MS` pass unchanged in the gate's test step.

## Scope

- `crates/connectors-sdk/src/lib.rs`: the helper and its two tests.
- `crates/connectors-host/src/local/owner/transport.rs`: `idle_bound()`, the readers of the two
  delay names, and the owner spawn (lines 1663 to 1698). Four other stories of this set write this
  file after this one, in this order by their `depends_on` edges:
  story:four-idle-exit-tests-no-longer-span-several-5-s-recovery-sweeps,
  story:seven-cli-tests-of-a-slow-owner-start-no-longer-wait-out-the-10-s-connect-wait,
  story:seven-cli-tests-shorten-their-waits-or-a-production-wait,
  story:eleven-connection-lease-tests-advance-a-clock-instead-of-sleeping.
- `apps/connectors/src/local/owner_stop.rs`: `bound()`.
- `crates/connectors-host/src/local/registry/lifecycle.rs`: `acquisition_ms()`.

## Not in this story

- The overrides themselves. Each story adds its call of the helper and names its variable.
- The SQL query timeout. It arrives by a constructor parameter:
  story:five-tests-no-longer-wait-out-the-http-and-sql-client-timeouts.
- A clock offset on `connectors_sdk::now_ms()`:
  story:where-do-the-connection-lease-tests-clock-and-the-30-s-admission-bound-live.
- The `#[cfg(test)]` seams of the library (`set_lifecycle_lock_wait`, `set_test_query_timeout`).
  They reach unit tests only and stay.
- Not established: where `CONNECTORS_TEST_OWNER_START_DELAY_MS` and
  `CONNECTORS_TEST_OWNER_LAUNCH_DELAY_MS` are read (`transport.rs` lines 1590 to 1597 hold the
  names only); whether a hidden public function in `connectors-sdk` is acceptable in the crate the
  adapters build against.
