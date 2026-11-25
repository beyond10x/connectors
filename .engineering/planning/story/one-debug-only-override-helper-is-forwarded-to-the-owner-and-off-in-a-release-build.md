---
format: aep.planning-md/3
id: story:one-debug-only-override-helper-is-forwarded-to-the-owner-and-off-in-a-release-build
kind: story
status: active
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
  path: crates/connectors-build/tests/debug_override_reads.rs
- confidence: cited
  path: crates/connectors-build/tests/main.rs
- confidence: cited
  path: crates/connectors-host/src/local/owner/transport.rs
- confidence: cited
  path: crates/connectors-host/src/local/registry/lifecycle.rs
- confidence: cited
  path: crates/connectors-sdk/src/lib.rs
revision: 8
transitions:
- {from: "draft", to: "proposed", at: "2026-11-25T05:07:24Z", actor: "human:operator", revision: 7}
- {from: "proposed", to: "active", at: "2026-11-25T05:07:25Z", actor: "human:operator", revision: 8}
---
## Defect

Five stories of this set add a debug-only override of a production wait. The repository has no
helper for that. It has a pattern that is copied. Read at 0.78.1; the lines are those of 0.78.2:

- `crates/connectors-host/src/local/owner/transport.rs` lines 1969 to 1980: `idle_bound()` has one
  definition under `#[cfg(debug_assertions)]` that reads `CONNECTORS_TEST_OWNER_IDLE_MS` and one
  under `#[cfg(not(debug_assertions))]` that returns `IDLE_EXIT` (600 s).
- `apps/connectors/src/local/owner_stop.rs` lines 18 to 29: `bound()` reads
  `CONNECTORS_TEST_OWNER_STOP_MS` the same way; production is `BOUND` (30 s).
- `crates/connectors-host/src/local/registry/lifecycle.rs` lines 695 to 704: `acquisition_ms()`
  reads `CONNECTORS_TEST_ACQUISITION_MS`; production is `ENTRY_MS`.
- `transport.rs` lines 1590 to 1597: the names `CONNECTORS_TEST_OWNER_START_DELAY_MS` and
  `CONNECTORS_TEST_OWNER_LAUNCH_DELAY_MS`. Their readers, read at 0.78.2: the CLI's `spawn` sleeps
  for the launch delay when the variable is set (lines 1630 to 1636), and the owner's start sleeps
  for the start delay when the variable is set (lines 1780 to 1786). Both are sleeps a test adds.
  Neither replaces a production wait, so neither is a site of this pattern.

The stories would add about eight more copies.

A second gap: the CLI starts the owner with `.env_clear()` (`transport.rs` line 1670) and passes,
in a debug build, a fixed list of seven names (lines 1686 to 1698). An override that the owner reads
does not reach an owner the CLI started unless its name is added to that list. The list is a loop
in the body of `spawn` (line 1612): no function picks the owner's environment today.

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
   (`apps/connectors/Cargo.toml`), and two of the three sites are there. The release test of
   point 4 would then need a release build of that crate's test binary.
2. The three sites above (`idle_bound()`, `bound()`, `acquisition_ms()`) call the helper. Their
   variable names stay, so no test changes. The two delay sleeps stay as they are.
3. This story extracts one function from `spawn` in `transport.rs`: it takes the CLI's environment
   and returns the variables the owner gets. `spawn` applies its result after `.env_clear()`. The
   function returns the installation directory and the seven names of the fixed list as today, and
   in a debug build also every variable whose name starts with `CONNECTORS_TEST_OVERRIDE_`. A
   release build compiles neither the list nor the prefix rule, as today. Every override a story of
   this set adds carries that prefix, so no story edits the spawn.
4. One release-build test in `connectors-sdk`, compiled only without `debug_assertions`:
   `release_build_ignores_a_debug_override`. It runs itself as a child process with a variable set
   and asserts that the helper returns the production value. The lane `slow` runs it with
   `cargo nextest run --release --locked -p connectors-sdk --no-tests fail release_build_`, the
   fourth step of story:the-lane-slow-runs-the-ignored-timing-family. That story adds the first
   test with this name prefix; this story adds the second.
5. One test in the gate's test step that finds a direct read of an override:
   `no_debug_override_is_read_outside_the_helper`, a new module
   `crates/connectors-build/tests/debug_override_reads.rs` declared in
   `crates/connectors-build/tests/main.rs`. It reads every `.rs` file under the `src/` directories
   of `crates/`, `apps/` and `adapters/`. Outside `crates/connectors-sdk/src/lib.rs` it fails when
   a file passes to `std::env::var` or `std::env::var_os` a name that starts with
   `CONNECTORS_TEST_OVERRIDE_`, one of the three older names of point 2, or a constant the same
   file defines from such a name. It is a scan of the text: it does not follow a name through a
   variable or across files.

## Decisions beyond the plan

The plan says "debug-only" and asks for no proof. This story adds the proof of point 4. The reason:
an override that worked in a release build would be a production knob that shortens a safety wait
(the connect wait, the client drain, the recovery sweep). One test of the one helper covers every
override that goes through it. The five stories that carried this acceptance each (four-idle,
slow-owner, eleven, five-tests, seven-cli-shorten) no longer carry it.

A second decision: the plan asks for no change of the three existing sites. This story moves them
onto the helper (point 2). The reason: the test of point 5 refuses every direct read of a wait
override, and the three sites are direct reads. Left as they are, they would need an exception in
that test, and the proof of point 4 would not cover them.

## Acceptance

1. A test of `connectors-sdk` in the gate's test step: a child process with the variable set gets
   the variable's value from the helper; without the variable it gets the production value.
2. `cargo nextest run --release --locked -p connectors-sdk --no-tests fail release_build_` runs
   `release_build_ignores_a_debug_override` and it passes: with the variable set, the helper
   returns the production value. The lane `slow` runs this command.
3. A unit test of `transport.rs` on the function of point 3: from an environment that holds
   `CONNECTORS_TEST_OVERRIDE_EXAMPLE_MS` and `CONNECTORS_TEST_EXAMPLE`, the owner gets the first
   and not the second. The seven names of the fixed list still pass.
4. `grep` finds no `#[cfg(not(debug_assertions))]` function that returns a wait constant in
   `transport.rs`, `owner_stop.rs` and `registry/lifecycle.rs`: the three sites call the helper.
5. The tests that set `CONNECTORS_TEST_OWNER_IDLE_MS`, `CONNECTORS_TEST_OWNER_STOP_MS`,
   `CONNECTORS_TEST_ACQUISITION_MS`, `CONNECTORS_TEST_OWNER_START_DELAY_MS` and
   `CONNECTORS_TEST_OWNER_LAUNCH_DELAY_MS` pass unchanged in the gate's test step.
6. `cargo nextest run --locked -p connectors-build --test main debug_override_reads` passes on the
   tree of this story. The same module holds both sides with two texts of its own: a text that
   calls `std::env::var("CONNECTORS_TEST_OVERRIDE_EXAMPLE_MS")` is reported with its line, and a
   text that passes the same name to `debug_override_ms` is not. The gate's test step runs the
   module. The command selects tests of `connectors-build` only, so the setup script of
   story:eighteen-tests-take-a-template-store-or-jail-built-once-per-run, whose filter names
   `connectors-host` and `connectors-sql`, does not run for it (that story lists the filter's
   support in cargo-nextest 0.9.143 as not established).

## Scope

- `crates/connectors-sdk/src/lib.rs`: the helper and its two tests.
  story:the-lane-slow-runs-the-ignored-timing-family writes this file before this story (one
  release-build test).
- `crates/connectors-host/src/local/owner/transport.rs`: `idle_bound()` and the owner spawn (lines
  1663 to 1698) with the extracted function and its test. Four other stories of this set write this
  file after this one, in this order by their `depends_on` edges:
  story:four-idle-exit-tests-no-longer-span-several-5-s-recovery-sweeps,
  story:seven-cli-tests-of-a-slow-owner-start-no-longer-wait-out-the-10-s-connect-wait,
  story:seven-cli-tests-shorten-their-waits-or-a-production-wait,
  story:eleven-connection-lease-tests-advance-a-clock-instead-of-sleeping.
- `apps/connectors/src/local/owner_stop.rs`: `bound()`.
- `crates/connectors-host/src/local/registry/lifecycle.rs`: `acquisition_ms()`.
- `crates/connectors-build/tests/debug_override_reads.rs` (new) and
  `crates/connectors-build/tests/main.rs` (one `mod` line). No other story of this set writes
  either.

## Not in this story

- The overrides themselves. Each story adds its call of the helper and names its variable.
- The SQL query timeout. It arrives by a constructor parameter:
  story:five-tests-no-longer-wait-out-the-http-and-sql-client-timeouts.
- A clock offset on `connectors_sdk::now_ms()`:
  story:where-do-the-connection-lease-tests-clock-and-the-30-s-admission-bound-live.
- The `#[cfg(test)]` seams of the library (`set_lifecycle_lock_wait`, `set_test_query_timeout`).
  They reach unit tests only and stay.
- The two delay sleeps (`CONNECTORS_TEST_OWNER_START_DELAY_MS`,
  `CONNECTORS_TEST_OWNER_LAUNCH_DELAY_MS`). The test of point 5 does not name them.
- The owner spawn on Windows (`transport/windows.rs`, `spawn` at line 558). It passes the owner
  `SystemRoot` (line 587) and no test variable, and is not changed.
- Not established: whether a hidden public function in `connectors-sdk` is acceptable in the crate
  the adapters build against; whether a read of an override through a name built at run time
  exists or will exist (the test of point 5 would not see it).
