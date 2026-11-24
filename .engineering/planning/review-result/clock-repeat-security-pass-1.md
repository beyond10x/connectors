---
format: aep.planning-md/3
id: review-result:clock-repeat-security-pass-1
kind: review-result
status: active
title: 'Security review pass 1: an approved write survives one refused clock acquisition'
relations:
- reviews: story:an-approved-write-survives-one-refused-clock-acquisition
revision: 1
---
```
unit: story:an-approved-write-survives-one-refused-clock-acquisition, working tree clock-security (base faa134f0d + implementor's unstaged change)
verdict: CONFIRMED (3 notes, no blocker, no warning)
cases: executed 93→105 on the brief's filter (12 of mine selected, before by subtraction), +6 owner cases outside it; red 0
origin: introduced 2 / pre-existing 1 / undecided 0
wrote-outside-worktree: 5 paths
needs-coordinator: story statement 26 and the brief's filter (below)
```

**1. What I touched.** Charter note, named by me: two non-test files each got a 2-line test-module declaration, as the brief directs; no other production line changed.
```
crates/connectors-host/src/local/clock.rs                                        | +2 (#[cfg(test)] mod, :9-10; lines below shift by 2)
crates/connectors-host/src/local/owner/mutation.rs                               | +2 (#[cfg(all(test, unix))] mod, :12-13)
crates/connectors-host/src/local/clock/second_attempt_security_tests.rs          | new, 594 lines, 12 tests
crates/connectors-host/src/local/owner/mutation/second_attempt_security_tests.rs | new, 636 lines, 6 tests
```

**2. My cases alone** (08:56 CET): `Summary [ 4.448s] 18 tests run: 18 passed, 1031 skipped`, exit 0. All green now; none was ever red.

**3. Suite** (08:57 CET), `cargo nextest run --locked -p connectors-host -E 'test(/clock|recheck|approval_issuance/)'`: `Summary [ 7.013s] 105 tests run: 105 passed, 944 skipped`, exit 0. Clippy `--all-targets -D warnings` exit 0; `cargo fmt --check` exit 0.

**Per invariant** (code in `crates/connectors-host/src/local/`; clock tests = C, owner tests = O)
- **I1 holds.** `clock.rs:410-429`: the second attempt is a whole `acquire_explained()` and the first refusal is dropped. Nonce and Merkle root bind the reply to its request (`clock/protocol.rs:148`, `:192-201`).
  - C `answers_of_the_two_attempts_are_never_added_up` (1.1 s)
  - C `a_signed_reply_to_the_first_request_is_refused_by_the_second_attempt` (0.6 s): the source's own valid reply to request 0, sent to the second socket, is `unverified`.
  - The implementor's late-reply test (`clock/tests.rs:1233`) sends the reply to the closed first port, so it measures socket reuse and not nonce matching.
- **I2 holds.** `clock.rs:419-423`.
  - C `a_reply_that_does_not_verify_ends_the_acquisition` (1.3 s): 12 forgeries, each with a second round that would answer; requests stay [1,1] and no time is spent waiting.
  - C `no_third_attempt_is_made_where_it_would_answer` (1.1 s)
  - C `the_other_refusals_are_answered_after_one_attempt` (0.2 s)
  - Dropping attacker, new after the change:
    - holds a caller 4 s per acquisition and 8 s per write (before: 2 and 4);
    - at the second reading the worker, the captured connection, the native preparation and the taken grant use are held 2 s longer;
    - picks which of two fresh readings is used;
    - must drop two rounds to refuse.
  - Forging attacker: nothing new. A forgery in round 1 refuses as before; one in round 2 can only refuse. This matches the row at `docs/design/approval-clock-quorum.md:346`, except F3.
- **I3 holds** on the standing-grant route. `owner/mutation/execution.rs:306`, `:442`, `:534-543`.
  - C `the_reading_is_the_second_attempts_alone` (1.1 s)
  - O `a_grant_expired_at_the_second_attempts_reading_admits_nothing` (2.1 s)
  - O `a_grant_that_expires_during_the_wait_of_the_second_reading_sends_nothing` (2.7 s): prepare 1, send 0, use released, no attempt.
- **I4 holds for the second attempt; the first attempt is not bounded by `until` (F2).**
  - C `the_decision_admits_only_what_ends_before_until` (10,000 seeded cases, 0.01 s)
  - C `a_refused_second_attempt_returns_with_the_reserve_left` (1.1 s)
  - C `a_reserve_the_deadline_cannot_hold_gets_one_attempt` (1.2 s)
  - O `a_write_whose_second_reading_takes_two_attempts_is_sent_once` (2.9 s): send 1.
  - O `a_reading_refused_after_two_attempts_leaves_what_one_refusal_leaves` (4.4 s; 5.7 s in the first run, two sites)
  - O `the_first_reading_makes_no_second_attempt_inside_its_reserve` (2.1 s)
  - O `the_second_reading_makes_no_second_attempt_inside_its_reserve` (2.5 s)
- **I5 holds.** `apps/connectors/src/local.rs:873` (`acquire_explained`), `owner/maintenance.rs:110` (`acquire`).
  - C `the_entry_points_of_clock_check_and_the_scan_make_one_attempt` (1.3 s), which also runs statement 19 for `maintenance.rs`.
- **I6 holds.** The production diff adds no log line; callers pass only `refused.failure` to `Error::clock`.
  - C `a_refusal_of_the_second_attempt_names_no_key_nonce_or_signature` (0.6 s)

**Findings**

| id | severity | file:line | what, evidence, smallest fix | verdict / origin |
|---|---|---|---|---|
| F1 | note | `owner/mutation/tests.rs:1176` | The implementor's suite does not measure which reserve a write's readings pass. With `until` 2 s, under 1.5 s is left after the first attempt; a reserve of 1 s needs 1.5 s, so swapping `execution.rs:306` to `RESERVE_AFTER_CLOCK` stays green. No test covered `:442` at all. Evidence is arithmetic; no mutant was run. Reached by every write. Fix: keep my two `..._inside_its_reserve` tests. | CONFIRMED / introduced |
| F2 | note | `clock.rs:415` | `acquire_within` makes its first attempt whatever `until` is, so a caller entered with less than one query deadline left answers up to one query deadline late. Measured by C `a_call_entered_at_its_deadline_ends_within_one_query_deadline` (0.6 s). The base `time()` has the same check, acquire, check. Reached by any reading started under 2 s before the deadline. Fix: none in this story. | CONFIRMED / pre-existing |
| F3 | note | `docs/design/approval-clock-quorum.md:346` | The row says a forged or replayed datagram ends the exchange `unverified`. `clock.rs:636` answers `unreachable`, before verifying, for a datagram stamped at or after the deadline, so that forgery gets a second round. No integrity effect. Not reproduced: the window is microseconds. Fix: add "that arrives inside the deadline" to the row. | INFEASIBLE / introduced |

**Needs coordinator**
- Statement 26's filter contains `second_attempt`, which matches my module path, so it now selects 33 tests, not 15.
- The brief's filter does not select my 6 owner cases; add `second_attempt_security_tests` to it.

**Not checked**
- The Windows lint (statement 27) with my clock module, which compiles on every target.
- `local_cli.rs` statement 18: read, not run.
- `connectors-docs generate --check`: not run; the website's "Second attempt" section is byte-equal to the contract's by `diff`.
- I3 on the per-input proof route, `issue` and the standing commands (signing needs custody).
- A late reply on a reused ephemeral port.
- Real sources.

**Outside the worktree**, all in `<session scratch>/clock/security/`: `run1-own.log`, `run2-own.log`, `run3-suite.log`, `run4-clippy.log`, `clock-fmt.out`.

```findings
- file: crates/connectors-host/src/local/owner/mutation/tests.rs
  line: 1176
  category: mutant
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: the implementor's reserve guard stays green when the first reading passes the one-second reserve, and nothing measured the second reading's reserve; two added owner tests close it
- file: crates/connectors-host/src/local/clock.rs
  line: 415
  category: boundary
  severity: note
  verdict: CONFIRMED
  origin: pre-existing
  message: the first attempt is made whatever until is, so a caller entered with less than one query deadline left answers up to one query deadline after its deadline
- file: docs/design/approval-clock-quorum.md
  line: 346
  category: contract-drift
  severity: note
  verdict: INFEASIBLE
  origin: introduced
  message: the row says every forged or replayed datagram ends the exchange unverified, while one stamped at the deadline is reported unreachable and gets a second attempt
```
