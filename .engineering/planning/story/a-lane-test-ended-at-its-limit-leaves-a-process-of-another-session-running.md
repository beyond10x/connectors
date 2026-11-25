---
format: aep.planning-md/3
id: story:a-lane-test-ended-at-its-limit-leaves-a-process-of-another-session-running
kind: story
status: draft
title: A lane test ended at its limit leaves a process of another session running
relations:
- informed_by: story:the-lane-slow-runs-the-ignored-timing-family
- serves: vision:independent-contract-adapters
revision: 1
---
## Defect

`connectors-build ignored` ends a test that runs over its limit by killing the test's process
group (`execute_test()` in `crates/connectors-build/src/ignored.rs`). A process the test started
in a session or group of its own is not in that group. It keeps running, and nothing reaps it.

Found by the adversary read of story:the-lane-slow-runs-the-ignored-timing-family on 2026-11-25.
Its case: a fixture test starts `/usr/bin/setsid /bin/sleep 4` and sleeps 4 s, under a limit of
1 s. `execute_test()` returned the limit error in time, and the child was still running after the
call returned. The kill of the group only is unchanged from before that story.

What reaches it:

- Today: nothing. The one test the lane `slow` selects starts its owner in its own group.
- Later, inferred from the sources and not run: the lane tests that start the CLI in the jail
  (four are named in the lane story's Change 2). The jail runs the CLI with a cleared environment
  (`apps/connectors/tests/support/mounted.rs`), so the variable that ties an owner to its parent
  is not set, and the CLI starts the owner with `setsid`
  (`crates/connectors-host/src/local/owner/transport.rs`). A jail test ended at 60 s would leave
  its owner running on the runner.

The lane story's acceptance 4 ("no process of the fixture is left running") holds for the
process group only.

## Change

Not decided. Two ways the adversary named:

- After the group kill, the runner also ends what it adopted: it makes itself a subreaper before
  it starts the test and kills every process it adopted.
- The jail fixture passes the variable that ties the owner to its parent.

## Acceptance

Draft. For either way: a unit test of `ignored.rs` in which a fixture test starts a child in a
session of its own and runs over a limit of 1 s; after `execute_test()` returns, the child is not
running. No case waits more than 5 s.

## Scope

- `crates/connectors-build/src/ignored.rs`: `execute_test()` and its tests; or
- `apps/connectors/tests/support/mounted.rs`, by the choice.

## Not in this story

- The limit itself and the name rule: story:the-lane-slow-runs-the-ignored-timing-family.
- Not established: whether an owner left by a jail test on a runner of the forge harms a later
  step of the same job, and whether the runner ends such a process when the job ends.
