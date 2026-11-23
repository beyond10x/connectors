---
format: aep.planning-md/3
id: review-result:invoke-read-deadline-scope-round-2
kind: review-result
status: active
title: Scope critic, round 2, story:invoke-admission-queues-under-load
relations:
- reviews: story:invoke-admission-queues-under-load
revision: 1
---
verdict: needs-revision

1. Acceptance 11: "Pass: no read is refused `timeout` before 120 s of its own wall time, and every other read answers or is refused `capacity`."
   - This contradicts the Design line that "some are refused even with the flag at 120" on a large store. A read refused `timeout` at 120 s is neither "answers" nor `capacity`, so the measurement would fail by its own rule on exactly the case the Design predicts.
   - Smallest change: "...and every read answers, is refused `capacity`, or is refused `timeout` at `admission` at 120 s; the count of each is recorded."
   - Consider adding "Pass is not 16 of 16 on a large store".
   - This criterion is the only one that checks the consumer's burst, so it needs to be satisfiable.

All four questions otherwise hold:
- Retry delay, `metadata_unavailable` under load, and workers/queue are each named in Out of scope, either dropped explicitly or assumed to be the store.
- `waited_ms` has its own story, `story:timeout-refusal-says-how-long-it-waited`, which depends on this one.
- The flag is justified by the one-cutoff and `--walk` lines.
- What the consumer gets past 120 s is stated, with "split the burst".
- The fixed 30 s and 40 s owner exchanges are bootstrap requests and are separate from the greeting wait that Acceptance 5 relies on. I checked `transport.rs:1037`, `:1052` and `:1068` and `invoke.rs:52`.

Notes:
- Outcome: a consumer that does not change its call, such as the dev-operator, still gets the 14-of-16 refusals. The Outcome now says so. Consider naming the consumer's required change in one line ("pass `--deadline-seconds 120`") so the hand-off is explicit.
- At 650 MB the consumer's one read may take more than 120 s alone, so the flag may not help it. Out of scope assumes that store is the cause, so this is covered. Re-check with the consumer after #267.
- Acceptance 4 depends on `CONNECTORS_TEST_OWNER_START_DELAY_MS` existing. I did not check that.
