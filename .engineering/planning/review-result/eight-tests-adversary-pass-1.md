---
format: aep.planning-md/3
id: review-result:eight-tests-adversary-pass-1
kind: review-result
status: active
title: Adversary read, pass 1, seven adapter and catalog tests with dropped waits
relations:
- reviews: story:eight-adapter-and-catalog-tests-drop-waits-that-prove-nothing
revision: 1
---
Verdict: needs-change, for the first commit of the change of
story:eight-adapter-and-catalog-tests-drop-waits-that-prove-nothing. Both findings that name a
change are fixed in the second commit of the same pull request.

Adversary read of the committed change (seven test files). The adversary added four cases to the
test files, uncommitted, and ran them with the seven committed tests on the build host of the
shared pool: 11 tests run, 9 passed, 2 failed. It edited no product file.

## Findings

1. Blocker, introduced. `crates/connectors-catalog/tests/bundle_adversary_2.rs`, the swap race.
   The loop of 1,000 calls of `load` and 100 swaps, 2 s at most, misses the defect the file was
   written against. The adversary wrote that defect inside a test (a `load` that checks
   `symlink_metadata` and then reads by path) and ran the committed loop against it: missed in 8
   of 240 loops on any CPU, and in 210 of 240 with both threads on one CPU. `swaps >= 100` passed
   while as few as 1 call of `load` overlapped a swap. The loop of 20 s before the change made 0.5
   to 2.6 million calls and stopped at the first follow.
   Fixed: the loop counts the calls of `load` during which the swap counter advanced, ends at the
   first follow, at 500 such calls or after 20 s, and the test asserts the 500. Against the
   defective `load`: caught in 40 of 40 loops on any CPU and 40 of 40 on one CPU.
2. Warning, introduced. `adapters/kubernetes/tests/exec_credential_security.rs`. The assertion on
   the candidate list exists only when `GITHUB_ACTIONS` is set, and the list is tied to the image
   behind the runner label of the gate job. When that image changes and no listed file is left,
   every gate job fails at this test with no commit in this repository, and the local gate cannot
   show it. Read, not run.
   Changed: in the failing case the walk collects up to 3 fitting files and the failure names all
   of them, so that the list has 3 entries from the first gate job. The design itself is the
   story's and stays; the story's "As built" section states the cost.
3. Note, pre-existing. `adapters/catalog/tests/security_git_relay.rs`. A process the timed-out
   pre-push check starts in a new session (`setsid`) outlives the refusal: the relay kills the
   check's process group only (`adapters/catalog/src/served/git.rs`). The adversary's case was red
   in 12 of 12 runs: the survivor wrote 999 ms after the refusal reached the test. No configured
   check was found that starts a new session. Not changed by this story.

## Attacked and held

- The push after a hang-up: the hang-up came 33 µs to 4.6 ms after the test saw the forge's read,
  and the forge applied the push 432 ms to 581 ms later. A late hang-up gives a weaker test, not
  a red one. The log poll has the predicate and the 3 s bound of the sleep it replaces; only one
  line of the relay writes `outcome=`.
- The timed-out check: timeout at 1 s, kill at the timeout, a survivor's write at 2 s, the look at
  3 s. The look came 995 ms to 1,018 ms after a survivor's write.
- `guarded_merge.rs`: no pair of timers has to end in a given order.
- `exec_credential_security.rs`: the product assertion comes before the list assertion; the
  condition `fits` is the old condition word for word.
- `adversary_one_binary.rs`: nothing is lost beyond what the story's Change 3 names.
- No other test reads a value this change moved.
- Inferred, not run: 100 swaps took at most 107 ms; at the forge's measured slowdown of 7.5 that
  is about 0.8 s.

```findings
- file: crates/connectors-catalog/tests/bundle_adversary_2.rs
  line: 103
  category: concurrency
  severity: blocker
  verdict: NEEDS-CHANGE
  origin: introduced
  message: the loop of 1,000 calls and 100 swaps misses a load that checks and then reads by path in 8 of 240 runs on the busy build host and 210 of 240 on one CPU
- file: adapters/kubernetes/tests/exec_credential_security.rs
  line: 530
  category: judgement
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: the list assertion exists only under GITHUB_ACTIONS and is tied to the image behind the runner label, so an image change turns every gate job red with no commit and the local gate cannot reproduce it
- file: adapters/catalog/tests/security_git_relay.rs
  line: 851
  category: boundary
  severity: note
  verdict: INFEASIBLE
  origin: pre-existing
  message: a process the timed-out check starts in a new session outlives the refusal because the relay kills the check's process group only
```
