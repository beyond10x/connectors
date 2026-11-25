---
format: aep.planning-md/3
id: story:where-do-the-10-s-of-the-identification-retry-loop-test-go
kind: story
status: draft
title: Where do the 10 s of the identification retry loop test go?
summary: 'A question, not a change: which wait the 10.2 s test lives through and what seam would shorten it'
relations:
- decomposes: story:the-gates-test-step-ends-within-120-s-on-the-forge
- informed_by: test-plan:where-the-gates-test-step-spends-its-time
- serves: vision:independent-contract-adapters
revision: 3
---
## Defect

One CLI test takes 10.2 s of the test step and no story names a change for it:
`adversary2_the_identification_retry_loop_ends_at_its_deadline_without_flooding_the_owner`, binary
`connectors::local_cli`, `apps/connectors/tests/local_cli.rs` (line 1769 at 0.78.1 and at 0.78.2).
The test plan says it needs a new seam and did not locate the bound of the loop. The test left
story:seven-cli-tests-shorten-their-waits-or-a-production-wait. It stays counted among the 81 tests
of the plan as not yet claimed by a change.

This story is a question. It changes no product code. The store has no kind for a spike, so it is
a story with one acceptance: a written answer.

Read at 0.78.1, not run; the lines are those of 0.78.2:

- The test starts an owner behind a relay that never answers a greeting carrying `build`, runs
  `instances status --adapter forge`, and asserts that the command ends in under 12 s, does not
  report another build, and opened at most 100 owner connections.
- Its comment (lines 1762 to 1766) says: "The CLI must stop at its 10-second connect deadline".
- `Client::connect` in `crates/connectors-host/src/local/owner/transport.rs` (lines 254 to 262)
  waits `Instant::now() + Duration::from_secs(10)`, a literal (line 259).
  story:seven-cli-tests-of-a-slow-owner-start-no-longer-wait-out-the-10-s-connect-wait puts that
  literal behind the override `CONNECTORS_TEST_OVERRIDE_OWNER_CONNECT_MS`. This story does not
  wait for that override and reads nothing of that story: it has no edge to it.

## Change

None in product code.

Questions:

1. Where do the 10.2 s go? Is it the 10 s of `Client::connect`, as the comment says? Proven by one
   run of the test with the time of each round of the loop printed.
2. If yes: does a shorter connect wait shorten the test, and what do the two bounds of the test
   become (under 12 s, at most 100 connections) so that it still shows a loop that ends at its
   deadline and does not flood the owner? Answered with a throwaway probe: the literal 10 s of
   `Client::connect` (line 259) is changed for one run of the test and is not committed. The
   probe runs in a worktree made for the probe and removed after it, because `transport.rs` is a
   file five stories of this set write in sequence. The probe needs no override and no other
   story.
3. If no: which wait it is, with `file:line`, and what seam would shorten it.

## Acceptance

1. A written answer to each question is added to this story under a heading `## Answer`, with the
   `file:line` of the wait and the output of the run. The answer names the story that then claims
   the test, or says that the test stays at 10.2 s and why.

## Scope

No change is committed. Recorded scope: none. The files read are
`apps/connectors/tests/local_cli.rs` and `crates/connectors-host/src/local/owner/transport.rs`.
The probe of question 2 edits `transport.rs` in its own worktree, which is removed after the run.

## Not in this story

- Any committed change of the test or of the connect wait. The probe of question 2 changes the
  literal in a worktree made for the probe, for one run, and the worktree is removed.
- The override of the connect wait:
  story:seven-cli-tests-of-a-slow-owner-start-no-longer-wait-out-the-10-s-connect-wait.
- The other tests of `local_cli.rs`.
- The runs of questions 1 and 2 follow the folder rules: one test per run, bound to 60 s by the
  runner.
