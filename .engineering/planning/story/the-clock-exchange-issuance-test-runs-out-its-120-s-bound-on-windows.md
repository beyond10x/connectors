---
format: aep.planning-md/3
id: story:the-clock-exchange-issuance-test-runs-out-its-120-s-bound-on-windows
kind: story
status: implemented
title: The clock-exchange issuance test ran 120 s on Windows for one silent source and did not name it
summary: One test of approval issuance waited 120 s for a clock source and answered a bare Unavailable. It gets bounds of 10 s, 15 s and 41 s, survives one lost datagram and names its facts when red.
relations:
- serves: vision:independent-contract-adapters
scope:
- confidence: cited
  path: crates/connectors-host/src/local/owner/approval_issuance/tests.rs
revision: 8
transitions:
- {from: "draft", to: "proposed", at: "2026-11-25T06:48:02Z", actor: "human:operator", revision: 5}
- {from: "proposed", to: "active", at: "2026-11-25T06:48:02Z", actor: "human:operator", revision: 6}
- {from: "active", to: "implemented", at: "2026-11-25T07:46:15Z", actor: "human:operator", revision: 8, decided_on: {"recorded":{"test_result":2}}}
---
## Defect

On 2026-11-25 the forge's job "host and CLI on windows-latest" of pull request 303 ended red in
one test that the pull request does not touch:

`local::owner::approval_issuance::tests::clock_exchange_precedes_leases_and_policy_is_rechecked_after_network`
(`crates/connectors-host/src/local/owner/approval_issuance/tests.rs`, the assertion at line 559).

Read from the log of job 139007679698 (run 45555866013):

- `issue` answered `Error { code: Unavailable, acquisition: None, cause: None, .. }`. The test
  expects `Forbidden`: the policy is changed while the clock is asked, and `issue` must see the
  change after the network step.
- The step ran `cargo test -p connectors-host --lib`: 724 tests in one process, 707 passed, 1
  failed, 16 ignored, 292.99 s.
- The test ran about 121.6 s: libtest printed "has been running for over 60 seconds" at 06:17:30
  UTC and `FAILED` at 06:18:32 UTC.
- The thread of the test that plays the first clock source did not panic: `exchange.join()` at
  line 558 passed. So it received the clock request, its `policy_set` returned, and its reply was
  sent.

In the two other Windows logs of the same day that were read (the first head of pull request 303
and pull request 304) the test ended within seconds, before its neighbours of the same module.

The test has one bound for every wait: `CONTENDED_LOCK_WAIT`, 120 s
(`crates/connectors-host/src/local/metadata.rs` line 240). It is the deadline of `issue`, the
lifecycle lock wait of both threads, the clock query timeout, and the read timeout of the first
source's socket. A test that runs out this bound takes 120 s. The rule of this repository's parent
is 60 s for one test.

## Cause

Read in the code and the log. No run reproduced it.

Read:

- One of the two clock exchanges ran its whole query deadline of 120 s: the test took 121.6 s,
  and a green run takes about 2 s.
- Only `Error::clock` (`crates/connectors-host/src/local/owner.rs` lines 261 to 272) gives
  `Unavailable` with neither `acquisition` nor `cause`. It drops the refusal and the report of
  each source, so the log does not say which source stayed silent.
- A second attempt was not possible. `issue` asks again when the first attempt ends below quorum
  and one query deadline plus the reserve is left (`clock.rs`, `second_attempt_fits`). The
  deadline of `issue` was the same 120 s as the query deadline.
- A source that did answer is refused when more than 30 s passed since its exchange started
  (`bounds_ns`, `clock.rs` lines 879 to 882). Production's query deadline is 2 s and cannot reach
  that limit. The test's 120 s can: the test's comment "a loaded scheduler cannot spend it" is
  wrong for every wait over 30 s.
- 175 other tests of the process reported `ok` during the 120 s. No lock of the whole process was
  held that long.

Concluded from that, not shown:

- The second source (the fixture server, seed 8) or one datagram between it and `issue` did not
  arrive.
- `issue` did not block the policy update. Between its first `resolve` and the end of the clock
  exchange it holds plain data and a directory handle. A blocked `policy_set` runs out its own
  bound and fails, which panics the first source's thread at line 558. The log shows the assertion
  at line 559.

No defect of the product was found. The defects are the test's: one silent source costs 120 s,
the red run does not say which source was silent, and its bound is over 60 s.

## Change

In the one test, nothing else:

- Three bounds in place of one. The policy update and every lifecycle lock wait: 10 s. One clock
  attempt: 15 s, which outlasts the update and stays under the 30 s of `bounds_ns`. The deadline
  of `issue`: two attempts, the reserve after the clock and one more 10 s, 41 s in sum. A right
  run waits none of them out.
- The first source is a `clock_fixture::Server` whose answer makes the policy update before its
  first reply. It answers a second attempt's request without a second update. It records what
  `policy_set` answered and how long it took.
- One silent source or one lost datagram costs one attempt of 15 s: `issue` asks again and the
  test ends `Forbidden`.
- The update is asserted to have answered `Ok` once. An update blocked by a policy lease or by
  the metadata lifecycle lock of `issue` fails inside 10 s, before the query ends, and the
  assertion names its error code and its time.
- A red run prints the error of `issue`, the number of requests each source received, and the
  update's result and time.
- Duration of a red run, read from the bounds and not timed: about 11 s when the update is
  blocked, about 30 s plus the work before the clock when a source never answers. The deadline
  of `issue` is 41 s and is read between its steps: a lock wait that is running then adds up to
  10 s, and nothing in the test cuts a longer hang.

## Acceptance

1. The forge's job "host and CLI on windows-latest" of the pull request reports the test `ok`,
   read from the job's log.
2. Red reproduction: with the second source replaced by `clock_fixture::Server::dropping` and one
   drop (an edit that is not committed), the test of the parent commit ends red after about
   121 s with `Unavailable`. Read from the runner's lines of one run on the build node.
3. The changed test ends green on the build node, as committed. With the same edit as in
   statement 2 it ends green too, with 2 requests at each source.
4. Mutant: with the policy recheck after the network step removed from `issue` (an edit that is
   not committed), the changed test ends red. Read from the runner's lines on the build node.
   Read in the code before the run: two layers read the policy again after the network step,
   the second `resolve` and `Store::acquire` of the policy lease, and each answers `Forbidden`.
   The run without the second `resolve` alone is made and recorded as it ends; the mutant of
   this statement removes both.
5. With two drops at the second source (an edit that is not committed) the changed test ends red
   in under 60 s, and its message names 2 requests at each source and the update's `Ok`. Read
   from the runner's line of one run on the build node.
6. `git diff --stat origin/main...HEAD -- crates` names one file,
   `crates/connectors-host/src/local/owner/approval_issuance/tests.rs`.

Statements 2 to 5 need the build node, which did not answer when the change was written. The
operator's decision: the pull request is pushed once and is not merged before the runs of
statements 2, 3 and 4 are recorded on this story.

## Scope

- `crates/connectors-host/src/local/owner/approval_issuance/tests.rs`: the one test.

## Not in this story

- The other tests that use `CONTENDED_LOCK_WAIT` as their bound (`approval_policy/tests.rs`,
  `approvals/tests.rs`, `mutations/tests.rs`), and `until()` of this test file.
- `Error::clock` dropping the refusal and the source reports. A caller of `issue` that gets
  `Unavailable` cannot tell a silent source from a timer refusal; whether it should is a question
  of the product's contract.
- The fixture server's receive loop (50 ms poll).
- A key lease that `issue` held during the clock exchange. The policy update does not take the
  key's lock, so the test does not see it, before this change or after it.
- A test that tells the second `resolve` from the policy lease's own re-read. Both answer
  `Forbidden`, before this change and after it.
- The steps of the test before `issue`, which keep `until()` of 120 s.
- The clock test of pull request 303
  (`a_passed_deadline_and_an_unbounded_reserve_leave_one_attempt`), which that pull request
  changes.
