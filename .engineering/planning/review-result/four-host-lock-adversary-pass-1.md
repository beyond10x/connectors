---
format: aep.planning-md/3
id: review-result:four-host-lock-adversary-pass-1
kind: review-result
status: active
title: Adversary read of the four host lock-wait tests, first commit
relations:
- reviews: story:four-host-tests-no-longer-wait-out-the-30-s-lock-wait
revision: 1
---
Verdict: needs-change, for the first commit of the change of
story:four-host-tests-no-longer-wait-out-the-30-s-lock-wait. One introduced finding and two
pre-existing ones are fixed in the second commit of the same pull request; one is not changed and
says why; one is the merge order the story names.

Adversary read of the committed change. The adversary added two cases to the tests of
`crates/connectors-host/src/local`, uncommitted, and ran them with the four tests of the change on
the build host of the shared pool: 6 tests run, 5 passed, 1 failed. It edited no product line and
ran no mutation in place. It did not run the `slow_lane_` test.

## Findings

1. Warning, introduced. `approval_policy/tests.rs`,
   `child_process_waits_on_the_lock_wait_its_parent_hands_it`. With the hold of 200 ms a child
   whose lease waits production's 30 s passes every assertion of the test; with the hold of 31 s
   before the change that child answered Conflict. So no test showed any more that the policy
   lease uses the wait handed to the child: a lease on `WAIT_BOUND` kept all four tests green
   (the adversary's case was red; the mutant itself was read, not run).
   Fixed: `held_use_blocks_cross_process_revocation_until_release` times its `blocked` child,
   which is handed 1 s, and asserts that it ended before `WAIT_BOUND / 2` (15 s). A lease on
   30 s keeps the child 30 s. The adversary's case compared two rounds of the other test and is
   not in the tree.
2. Note, pre-existing. `adversary_w2_tests.rs`,
   `blocked_policy_child_keeps_production_wait_when_the_variable_is_inherited`. On Unix the test
   stays green when the removal of the variable or its binding to the handing process is dropped;
   no test pinned the binding across a process boundary. The adversary's case (green) shows it:
   a child of a shell that was handed 600 s reports 30 s, and a process that replaces the shell
   reports 600 s.
   Fixed: the case stays as
   `a_wait_handed_to_a_parent_is_not_its_childs_but_that_of_a_process_replacing_the_parent`
   (Unix only, `/bin/sh`). The doc comment of the other test said the child keeps production's
   wait only when the variable is absent from the parent's environment; it now names both things
   that hold it on Unix and the one on Windows.
3. Note, pre-existing. `approval_policy/tests.rs`,
   `child_process_waits_on_the_lock_wait_its_parent_hands_it`. The child's admission was asserted
   in a way that also passes when the child never contended for the lease.
   Fixed: before the lease is released the test asserts that the child is still running and the
   revision is still 1. The child was handed 120 s and can only end by getting the lease, so the
   assertion cannot fail on a slow runner.
4. Note, pre-existing. `metadata.rs`,
   `contending_metadata_open_refuses_at_a_short_bound_and_recovers_after_release`. The refusal
   is accepted between 1 s and 11 s, so the test cannot tell a wait of 1 s from one of 10 s. The
   test before the change had the same 10 s of slack (30 s to 40 s). Not changed: the adversary
   found nothing that reaches it; the lock returns at its deadline in steps of 5 ms.
5. Note, introduced. `crates/connectors-host/tests/local_foundation.rs`. The new ignored
   `slow_lane_` test is unknown to the gate's ignored step until the rule of
   story:the-lane-slow-runs-the-ignored-timing-family is on the base. The story says so: its pull
   request opens after that story is merged.

## Attacked and held

- The `blocked` child handed 1 s: no deadline sits on the test's side, so a slow machine makes
  the test slower and not red (read, and run green). The 15 s bound of finding 1 is new; the
  whole test took 1.36 s to 1.57 s on the build host.
- A Conflict for the wrong reason: before the lease the child only inspects metadata nobody
  holds; without the lease it gets revision 2 and fails (read).
- The library metadata test holds every assertion of the test it replaces; both run in one
  process with two open file descriptions (read).
- A production wait under 3 s is still caught in the gate by
  `contending_metadata_open_waits_for_a_live_holder_instead_of_reporting_it_unavailable`;
  between 3 s and 30 s only the lane `slow` catches it (read).
- The `slow_lane_` test against the 60 s limit: one wait of 30 s and about 0.2 s of work (read,
  not run).
- A worker that does not set its wait waits 30 s, and the 11 s receive bound turns the metadata
  test red (read, not run).
- Windows: the removal alone holds the 30 s there (read, not run).

```findings
- file: crates/connectors-host/src/local/approval_policy/tests.rs
  line: 440
  category: mutant
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: with the 200 ms hold no test shows that the policy lease uses the handed wait, so a lease on production's 30 s passes all four tests
- file: crates/connectors-host/src/local/adversary_w2_tests.rs
  line: 82
  category: mutant
  severity: note
  verdict: CONFIRMED
  origin: pre-existing
  message: on Unix the test stays green when either the removal of the variable or its pid binding is dropped, and no test pins the binding across a process boundary
- file: crates/connectors-host/src/local/approval_policy/tests.rs
  line: 442
  category: concurrency
  severity: note
  verdict: CONFIRMED
  origin: pre-existing
  message: the waits child's admission is asserted in a way that also passes when the child never contended for the lease
- file: crates/connectors-host/src/local/metadata.rs
  line: 1783
  category: boundary
  severity: note
  verdict: INFEASIBLE
  origin: pre-existing
  message: the refusal is accepted anywhere between 1 s and 11 s, so the test cannot tell a 1 s wait from a 10 s one, and no reachable defect was found
- file: crates/connectors-host/tests/local_foundation.rs
  line: 309
  category: acceptance
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: the new ignored test is unknown to the gate's ignored step at this commit, so acceptance 3 holds only after the lane story's rule is on the base
```
