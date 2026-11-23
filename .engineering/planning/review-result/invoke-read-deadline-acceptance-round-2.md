---
format: aep.planning-md/3
id: review-result:invoke-read-deadline-acceptance-round-2
kind: review-result
status: active
title: Acceptance critic, round 2, story:invoke-admission-queues-under-load
relations:
- reviews: story:invoke-admission-queues-under-load
revision: 1
---
verdict: needs-revision

1. Line 5: "an owner start delayed 20 s, the call exits non-zero with `timeout` before that owner greets."
   - No named observable says the exit came before the greeting.
   - Fix: "an owner start delayed 600 s (it never greets within the test), the call exits non-zero with `timeout`". That is event-based. The test runner's 60 s kill makes a hang a failure.

2. Line 9: "read deadline 60 s, lease 30 s, validation slower than the lease ... the read is refused once, by the validation bound".
   - The lease is `deadline.min(now + 30_000)` on the real clock (`crates/connectors-host/src/local/owner.rs:1135-1141`).
   - A validation slower than the lease therefore means a test of 30 s or more, which is a heavy share of the 120 s step.
   - "Refused once, by the validation bound" names no code.
   - Fix: "the lease length is a parameter of `follow_executable` (30 s in production). The test uses a 1 s lease, a 5 s read deadline and a validation that pauses 2 s. The read is refused `<code>` once, and the registry holds no validation record dated after the lease expiry." Name the code.

3. Line 7: "asserts that the cutoff ... is 30,000 ms after its instant, and that the owner start and `session.deadline` are derived from that one value."
   - It holds two outcomes. The second reaches `session.rs:600`, which is set in the generated runner and is likely out of reach of a unit test in `invoke.rs`.
   - Line 4 already checks the one-cutoff behaviour by observation.
   - Fix: keep only "the cutoff built from arguments without the flag is 30,000 ms after its instant". Leave the one-value promise to line 4.

4. Line 11: "Pass: no read is refused `timeout` before 120 s of its own wall time, and every other read answers or is refused `capacity`."
   - A run where all 16 reads are refused `timeout` at 120 s passes. It is worse than the 5 of 16 baseline.
   - Fix: add "and more than 5 of the 16 answer (baseline 5 of 16)".

5. Design: "With `--definition` the flag sets the read's deadline as for a named instance."
   - No line checks this.
   - Fix: add to line 3 "and the same with `--definition`". Or drop the sentence from Design.

Notes (not blockers)
- Line 8: the fixture in `apps/connectors/tests/invoke_walks.rs` has no delay knob (no `sleep` or `knob` in the file). The test needs a 1.5 s per-page delay added there.
- Line 4: it says "refused `timeout`" without a stage. Add "at `admission`" if that is the stage that holds.
- Line 10: it holds five documents and a gate. Its checks are mechanical, so it can stay. "The steps after an edit to `ess/domains`" should name the task, for example `task <name>`.
