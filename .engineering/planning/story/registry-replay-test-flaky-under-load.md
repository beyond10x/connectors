---
format: aep.planning-md/3
id: story:registry-replay-test-flaky-under-load
kind: story
status: draft
title: Grown-store replay test saw two replays under load
relations:
- serves: vision:independent-contract-adapters
revision: 1
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
