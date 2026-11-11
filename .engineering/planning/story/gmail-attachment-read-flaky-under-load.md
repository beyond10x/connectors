---
format: aep.planning-md/3
id: story:gmail-attachment-read-flaky-under-load
kind: story
status: active
title: A 3 MiB Gmail attachment read answered Unavailable under gate load
relations:
- decomposes: epic:tech-debt-review-20260930
- serves: vision:independent-contract-adapters
- decomposes: epic:zero-open-issues-and-store-defects
scope:
- confidence: cited
  path: adapters/catalog/tests/google_calendar_gmail_adversary_pass2.rs
revision: 5
transitions:
- {from: "draft", to: "proposed", at: "2026-11-09T21:58:29Z", actor: "human:operator", revision: 3, executor: "agent:connectors-zero-issues", correlation: "connectors-zero-issues"}
- {from: "proposed", to: "active", at: "2026-11-09T21:58:30Z", actor: "human:operator", revision: 4, executor: "agent:connectors-zero-issues", correlation: "connectors-zero-issues"}
---
## Defect

`an_attachment_under_the_named_limit_is_refused_and_the_texts_state_the_real_ceiling`
(`adapters/catalog/tests/google_calendar_gmail_adversary_pass2.rs:132`) failed once in a full
`connectors-build gate --msrv` run on `82658e5e5` (wave `20261001-techdebt`, which changes no Rust):

```
a 3144704-byte part failed: Unavailable
```

The same binary then passed it 10 of 10 times run alone, at load average 19.6. The read moves a
3,144,704-byte part through the provider child; whether `Unavailable` comes from an invocation
deadline, the fixture server or the child under parallel test load has not been established.

## Acceptance

- The cause of `Unavailable` for the 3 MiB read under load is named with a reproduction, or the test's
  outcome is shown to be independent of scheduling.
- The test passes 100 of 100 runs inside a parallel full workspace test run.

## Wave status

Status on 2026-11-10 (single defect wave): not reproduced; no change made.

The built test binary was run under CPU starvation (`taskset`, `nice 19`, busy workers):

| Setup | Passed |
|---|---|
| 2 CPUs, 4 workers | 30 of 30 |
| 1 CPU, 10 workers | 27 of 27 |
| 1 CPU, 8 workers | 18 of 20 |
| 1 CPU, 30 workers | 7 of 10 |

Every failure was `Child::spawn` returning `Timeout`, at its fixed 10 s deadline
(`process.rs:331`). None was the `Unavailable` this story reports.

Timestamps from those runs:

- The 3 MiB read itself took 0.15 to 0.6 s, against the 30 s invocation deadline.
- The fixture server wrote all 4,192,965 bytes.
- An HTTP timeout maps to `Timeout`, not `Unavailable` (`crates/connectors-host/src/http.rs:1054`).

So the original `Unavailable` needs a transport error that is not a timeout: a reset, a truncated
body, or the child dying. The test does not capture the provider child's stderr or the error kind.

Next step: make the failure message carry the child's stderr and the underlying error kind. Then
measure the 30 runs inside a loaded full-workspace run, the condition of the original failure.
