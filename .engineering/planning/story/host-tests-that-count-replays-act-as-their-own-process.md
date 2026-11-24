---
format: aep.planning-md/3
id: story:host-tests-that-count-replays-act-as-their-own-process
kind: story
status: active
title: Host tests that count store replays act as their own process
relations:
- serves: vision:independent-contract-adapters
scope:
- confidence: cited
  path: crates/connectors-host/src/local/metadata.rs
- confidence: cited
  path: crates/connectors-host/src/local/metadata/checkpoint_tests.rs
- confidence: cited
  path: crates/connectors-host/src/local/metadata/er.rs
- confidence: cited
  path: crates/connectors-host/src/local/registry/store_cost_tests.rs
- confidence: cited
  path: crates/connectors-host/src/local/registry/tests.rs
- confidence: cited
  path: crates/connectors-host/src/local/security_replay_tests.rs
revision: 9
transitions:
- {from: "draft", to: "proposed", at: "2026-11-24T01:50:55Z", actor: "human:timo", revision: 8}
- {from: "proposed", to: "active", at: "2026-11-24T01:50:56Z", actor: "human:timo", revision: 9}
---
## What happened

Pull request 289 (release 0.76.2), check "host and CLI on windows-latest", run 45300932153,
attempt 1, 2026-11-24 02:43 CET: step "Test the host and the CLI" (`cargo test --locked
--no-fail-fast -p connectors-host -p connectors`) ended with 652 passed, 1 failed.

```
local::registry::tests::a_read_invoke_against_a_grown_store_replays_it_at_most_once
panicked at crates\connectors-host\src\local\registry\tests.rs:2287:5:
the next read invoke replayed the whole store 2 times
```

The same step passed on macOS for the same commit, and on Windows for the two commits before
(pull request 288 and its merge to main).

## Cause (read from the code, not yet reproduced)

- A test acts as a process through a thread-local id (`simulate_process`,
  `crates/connectors-host/src/local/metadata/er.rs:2997`). The ids are literals that many tests
  share: `0` (the default), `1` (the owner, 13 call sites), `NOW + 1` and so on.
- The idle handles of all tests of one test binary are in one static pool (`IDLE`, `er.rs:698`),
  bounded per process id: the fifth idle handle of a process id evicts the oldest
  (`IDLE_LIMIT = 4`, `er.rs:699`, `er.rs:812`).
- `cargo test` runs the tests of a binary as threads of one process. The macOS and Windows jobs
  run it that way (`.github/workflows/windows-host.yml:68`, `macos-host.yml:60`). The local gate
  runs cargo-nextest, one process per test, so no test evicts another test's handle there.
- `read_invoke` (`registry/tests.rs:2248`) counts complete replays on its own thread. It expects
  one: the fresh open of the process that acts as the CLI. When other tests pool handles of their
  own stores under the same process id between two steps of this test, this test's idle handle
  is evicted, the next open is fresh, and the count is 2.

## What changes

1. A test that asserts on `full_replays()`, on `fresh_opens()` or on the reuse of a handle acts
   under process ids no other test uses. One helper in the test support of
   `crates/connectors-host/src/local/metadata/er.rs` gives a test its own namespace of process
   ids; a helper thread of the same test that must be the same process takes the namespace from
   its parent explicitly.
2. No change outside `#[cfg(test)]` code. The pool, its bound and `process()` of a release build
   stay as they are.
3. The local gate catches the same failure: one new test, run by nextest, makes other threads
   pool five handles of their own stores as process `1` and as the CLI's process id between the
   two invokes of `a_read_invoke_against_a_grown_store_replays_it_at_most_once`'s helper, and
   asserts that the count stays at most 1.

## Acceptance

1. The new test of point 3 fails on the tree before the change with "replayed the whole store 2
   times" (or its own message naming 2) and passes after it.
2. Every test of `crates/connectors-host` that asserts on `full_replays()` or `fresh_opens()`
   acts in its own namespace: the list of those tests, by file and name, is in the pull request,
   each with the line that sets the namespace.
3. `cargo test --locked -p connectors-host --lib local::registry::tests -- --test-threads 8`
   (the threaded form the macOS and Windows jobs use) passes in 5 of 5 runs.
4. `cargo nextest run --locked -p connectors-host -E 'test(local::metadata) | test(local::registry)
   | test(security_replay) | test(approval_issuance)'`: no test that passed before fails.
5. No test added or changed runs over 5 s; none is over the 60 s bound.

## Out of scope

- Moving the macOS and Windows jobs to cargo-nextest.
- The three load-sensitive tests of `story/connections-through-owner`.
