---
format: aep.planning-md/3
id: story:registry-replay-test-flaky-under-load
kind: story
status: archived
title: Grown-store replay test saw two replays under load
relations:
- serves: vision:independent-contract-adapters
revision: 5
transitions:
- {from: "draft", to: "archived", at: "2026-09-30T16:59:08Z", actor: "human:timo", revision: 3}
---
## Defect

`local::registry::tests::a_read_invoke_against_a_grown_store_replays_it_at_most_once`
(`crates/connectors-host/src/local/registry/tests.rs:1645`) failed once in a full
`connectors-build gate --msrv` run on `0667e89ad` with "the next read invoke replayed the whole
store 2 times", at load average 73. The same binary passed it 20 of 20 times run alone, and the
next full gate on the same commit passed (99 suites, 731 passed, 0 failed).

The replay counter is thread-local (`crates/connectors-host/src/local/metadata/er.rs:2130`), so
other tests cannot inflate it. What made the second replay happen under load has not been
established.

## Acceptance

- The cause of the second replay is named with a reproduction, or the test's bound is shown to be
  independent of scheduling.
- The test passes 100 of 100 runs under a parallel full workspace test run.

## Resolution (2026-09-30)

Duplicate of story:grown-store-replay-test-flake, implemented in wave 20260930b (unit commit on
`impl/grown-store-replay-test-flake`, merged at `5a438dc0e`). Cause named there: the idle-handle pool in
`metadata/er.rs` was shared by every test in the binary, and other tests' releases evicted the owner's handle;
the limit now applies per process. 10 of 10 full `--lib` runs after the fix. The 100-run acceptance above was not run.

## Closed

Archived 2026-09-30 (commit `7e5015cf1`, "plan: archive the duplicate replay-flake story") as a duplicate of `story:grown-store-replay-test-flake`. `story:planning-store-hygiene-20261001` listed it before that archive reached its reading of the store.
