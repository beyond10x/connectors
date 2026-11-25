---
format: aep.planning-md/3
id: review-result:five-host-waits-adversary-pass-1
kind: review-result
status: active
title: 'Adversary pass 1: five host unit tests with shorter waits'
relations:
- reviews: story:five-host-unit-tests-shorten-the-waits-they-chose
revision: 1
---
```
unit: story:five-host-unit-tests-shorten-the-waits-they-chose, commit 041e0c7eb (test code only) in the worktree `five-host-waits`
verdict: CONFIRMED
cases: executed 7→15, red 3
origin: introduced 3 / pre-existing 0 / undecided 0
needs-coordinator: the forge figure of acceptance 1 (sum at most 28 s in the gate job); whether three narrower margins are worth the seconds saved
```

All seven tests pass (20 of 20 repeat runs). Three of them went red on a stall the values before
the change survived. The eight added cases (names starting `adversary_w1_`) were not committed:
each red one builds its stall inside the test (a sleep, or SIGSTOP and SIGCONT on the child's
pidfd), and each has a green twin on the value before the change. No product code was edited.

## Cases added

`crates/connectors-host/src/local/owner/refresh/tests.rs`:

- `…ready_after_recovery_does_not_outlive_the_two_second_lifetime`: the committed test's steps at
  lifetime 2 s, with 2 s between `recover()` and `described()`. Red, 5.4 to 6.0 s:
  `left: (Published, Pending)`, `right: (Published, Ready)`.
- `…ready_after_recovery_outlives_two_seconds_on_the_ten_second_lifetime`: the same on 10 s. Green,
  13.9 s.
- `…measure_what_the_two_second_lifetime_has_to_cover`: prints the span from `recover()` to
  `described()`. Green; the span took 257 to 470 ms in 12 samples.

`crates/connectors-host/src/local/runtime/process/write_tests.rs`:

- `…a_second_spent_after_the_preparation_loses_the_send_on_the_committed_budget`: `expiring("timeout")`
  with 1 s between the preparation's answer and `commit()`. Red, 1.1 s: `count(send)` reads 0.
- `…a_second_spent_after_the_preparation_keeps_the_send_on_the_two_second_budget`: the same on a
  first budget of 2 s. Green, 2.2 s.
- `…a_child_first_scheduled_after_1100_ms_never_reaches_preflight_under_the_committed_deadline`
  (Linux): a `stalled-preflight` child stopped for 1.1 s, deadline 1 s. Red, 1.2 s:
  `count(prepare)` reads 0.
- `…a_child_first_scheduled_after_1100_ms_reaches_preflight_under_the_two_second_deadline` (Linux):
  the same on 2 s. Green, 2.2 s.
- `…measure_what_a_one_second_budget_has_to_cover`: prints preparation and commit latency in mode
  `ok`. Green; `send` is recorded 1 to 29 ms after the stamp.

## Findings

1. The refresh test's `Ready` read has a 2 s bound the story did not name. The lifetime is
   2,000 ms (was 10,000). Recovery dates the published evidence from its own `now()`
   (`owner/refresh.rs` lines 331 to 339) and `described()` computes the state at read time, so
   `recover()`, `attempt()` and `described()` must end inside the lifetime. A 4.3-fold slowdown of
   the slowest sample turns it red; on 10 s that took 21-fold. It then fails as
   `(Published, Pending)` under a message that says recovery refused. Not shown on the forge;
   inferred from the 20 repeat runs, where the test's time outside its sleep ranged 1.2 to 4.2 s.
2. `preparation_whose_deadline_passes_in_preflight_times_out_and_terminates_the_child` has no
   retry and its deadline was halved to 1,000 ms. Between the host's stamp and the fixture's
   `record("prepare")` sit host validation, the channel write, the child waking and a file write
   (0.5 to 28 ms locally). story:host-suite-load-sensitivity records this span overrunning the
   then 2 s deadline at load 8.51 and 13.20 on 2026-09-12.
3. `native_outcomes_and_lost_replies_never_repeat_a_send`, modes `timeout` and `peer-expired-eof`:
   `expiring` retries only a preparation that failed after its deadline. A first budget of 1,000 ms
   (was 2,000) that ends after the preparation answered and before the child records `send`
   returns with 0 sends. The story's "slower, not red" holds for the three modes of `drop_eof…`,
   not for these two.

## Attacked and held

- Each of the seven still fails on the behaviour it names; no shortened wait lets a deadline pass
  before the asserted step (read, not run).
- `drop_eof…`: the order 1 s, 1.2 s stall, 2 s second budget holds; a missed second attempt costs
  a retry, not a red. 21 runs green.
- Refresh, restart direction: the sleep is a lower bound against the lifetime, so a regression
  that dates evidence from the store acknowledgement is still caught.
- `late` terminal mode: 1.5 s still crosses the 1,000 ms poll slice once (`protected.rs` line 427).
- Clock test: other tests hold the same exchange at 500 ms; the override is thread-local;
  production's 2 s still elapses in two other tests of `clock/tests.rs`.
- OAuth test: the kept measurement is unchanged; the dropped result is in
  review-result:adversary-cli-oauth-acquisition-pass-2.
- No other test reads a value the change moved.
- Not checked: the macOS and Windows lanes, which run these tests with `cargo test`.

```findings
- file: crates/connectors-host/src/local/owner/refresh/tests.rs
  line: 997
  category: boundary
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: The final Ready read holds only while recover(), attempt() and described() end inside the 2 s lifetime (measured 257 to 470 ms; 10 s before), a span the story's As built does not name, and it fails as (Published, Pending) under a message that says recovery refused.
- file: crates/connectors-host/src/local/adversary_w2_tests.rs
  line: 63
  category: boundary
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: With a 1 s deadline and no retry, a child first scheduled 1.1 s after the stamp is terminated before the fixture records prepare, so the assert reads 0 where the 2 s deadline read 1.
- file: crates/connectors-host/src/local/runtime/process/write_tests.rs
  line: 535
  category: boundary
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: expiring retries only a failed preparation, so in the modes timeout and peer-expired-eof a first budget of 1 s that ends between the preparation's answer and the child's send record makes the assert read 0 where the 2 s budget read 1.
```
