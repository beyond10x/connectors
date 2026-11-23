---
format: aep.planning-md/3
id: review-result:invoke-read-deadline-acceptance-round-1
kind: review-result
status: active
title: Acceptance critic, round 1, story:invoke-admission-queues-under-load
relations:
- reviews: story:invoke-admission-queues-under-load
revision: 1
---
verdict: needs-revision

1. Quoted: "`--deadline-seconds 2` ... refused `timeout` at `admission` with `waited_ms` of 2000 or more and the provider did not receive it; with `--deadline-seconds 10` all 3 answer."
   - This line holds three outcomes: the refusal with its `waited_ms`, the provider not receiving the call, and all 3 answering at 10.
   - Checked `apps/connectors/tests/support/mounted.rs:147-232`. The fixture has an `invoke_ms` knob, and `/work/invocations` has one line per provider call, so the line can be checked.
   - Rewording: split it into three outcomes in `owner_slow_start.rs`.
     - (a) At 2 s, at least one read exits with `timeout` at `admission` and `waited_ms >= 2000`.
     - (b) At 2 s, the `/work/invocations` line count equals the number of answered reads.
     - (c) At 10 s, all 3 answer.

2. Quoted: Design "One cutoff: ... ends within n seconds plus the answer grace, not 2n."
   - No acceptance line checks this. The never-greets line cannot catch it: at n=2, 2n is 4 s, which is inside the 5 s bound, and the owner start fails before any read begins.
   - "Answer grace" has no value anywhere in the story.
   - Rewording: add "With `--deadline-seconds 2`, an owner start delayed 1.5 s and a read of 1.5 s, the call is refused `timeout` and exits within 2 s plus G", and give G a number. `CONNECTORS_TEST_OWNER_START_DELAY_MS` and `invoke_ms` already exist.

3. Quoted: Design "With `--walk`, the flag is each page's deadline."
   - No acceptance line names `--walk`.
   - Rewording: add "A `--walk` with `--deadline-seconds 2` over pages of 1.5 s each answers every page" in `invoke_walks.rs`. Otherwise drop the promise.

4. Quoted: "`--deadline-seconds 0`, `121` and a non-number are refused `invalid_input`, `field = deadline_seconds`, at `arguments`; so is the flag beside `--idempotency-key`."
   - The Design also refuses the flag beside `--approval-file`, and promises refusal "before anything is sent". Neither is checked.
   - The accepted edges 1 and 120 are not checked.
   - Rewording: add `--approval-file` to the refused cases and "and no owner is started and `/work/invocations` is empty". Add "1 and 120 are accepted".

5. Quoted: "A read without the flag sends the same deadline as before the change (30 s): a test reads it."
   - It does not say what is read or where. The deadline lives in two places: `apps/connectors/src/local/invoke.rs:52` (`READ_DEADLINE`) and `apps/connectors/src/local/session.rs:600` (`now_ms() + 30_000`). The owner's wire is said not to change.
   - Rewording: name the observable. For example: "a unit test asserts that the cutoff built from arguments without the flag is 30 000 ms, for both the owner start and `session.deadline`".

6. Quoted: "A call with `--deadline-seconds 2` against an owner that never greets exits within 5 s."
   - It names no refusal and no exit status.
   - Rewording: "... exits non-zero with `timeout` and `waited_ms` between 2000 and 5000, within 5 s of wall time". The test uses the `START_DELAY_MS` knob set far above 5 s. The bound should not be tighter than 5 s, because the host load seen was 126 to 154.

7. Quoted: "`--help`, the generated CLI reference, the bash completion and `semantics.md` name the flag, its default and its bounds; `owner.md:175` no longer states "30 seconds" as the only value."
   - One line holds six artifacts, and "name ... as the only value" cannot be checked as written.
   - Checked `contracts/cli/v1alpha1/owner.md:175`: "waits for its greeting up to the read's own 30 seconds" is there. `semantics.md:1493` already says "30-second default/120-second maximum", so the grep must look for the flag, not for the bounds.
   - Rewording: "`connectors invoke --help` and the bash completion output contain `--deadline-seconds`, and `1` and `120` in the help. The generated reference pages and the repository's generated-file check pass. `semantics.md` contains `--deadline-seconds`. `owner.md` no longer contains 'the read's own 30 seconds'."

8. Quoted: Outcome and Design "a `timeout` refusal says how long the call waited" and "`waited_ms`".
   - No acceptance line says `waited_ms` is documented in `semantics.md`, the CLI spec (`apps/connectors/spec/cli.yaml`) or the reference, though the scope lists them.
   - Rewording: add `waited_ms` to line 7's "contains" list.

9. Quoted (last line): "16 parallel reads with `--deadline-seconds 120` all answer, or the ones that do not are refused `capacity`; the count and times are recorded as evidence."
   - A reviewer cannot tell pass from fail reliably, for three reasons.
     - The queue is 16 (`supervisor.rs:703`), so `capacity` is unlikely at 16 reads. The clause reads like a fallback but changes nothing.
     - The story's own numbers make an all-answer result borderline. At 9.9 s per read, 16 serial reads need about 158 s, over the 120 s deadline. A last read may time out legitimately, and that would fail the line as written.
     - "Recorded as evidence" names no place, and the baseline conditions (store size, host load, version) are not tied to it.
   - Rewording: "On the instance of the 2026-11-23 comment, record on issue #267 the version, store size, host load, and per read the outcome, time and `waited_ms`. Pass: no read is refused `timeout` with `waited_ms` under 120000, and the other reads answer or are refused `capacity`. The baseline is 5 of 16 answered at 0.74.0."

10. Quoted: Design "Unchanged: the queue of 16 and its `capacity` refusal, the 30 s clamp of a recollection (`supervisor.rs:1584`), the store lock's 30 s (`metadata.rs:51`)".
    - No acceptance line checks this. The existing gate may cover it.
    - Rewording: add "the existing tests for these pass unchanged". Or move the item to "Out of scope".

Out of my lane: none seen.
Read: the story (revision 20), the cited lines in `invoke.rs`, `session.rs`, `owner.md` and `semantics.md`, `owner_slow_start.rs` and `tests/support/mounted.rs`. Not checked: `supervisor.rs:698-725`, `:1584`, `metadata.rs:51`, the `transport.rs`, `registry.rs` and `process.rs` lines, and the generated-file checks.
