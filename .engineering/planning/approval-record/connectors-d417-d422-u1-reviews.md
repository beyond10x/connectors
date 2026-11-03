---
format: aep.planning-md/3
id: approval-record:connectors-d417-d422-u1-reviews
kind: approval-record
status: approved
title: 'D422: the U1 review findings are fixed in the unit'
relations:
- decides: story:cold-owner-start-verifies-once
revision: 2
transitions:
- {from: "draft", to: "approved", at: "2026-11-03T16:52:34Z", actor: "human:operator", revision: 2}
---
## Decision D422 (2026-11-03, orchestrator)

On the review passes of wave D417 unit U1: the adversary pass (commit `23247269c`, 3 red cases) and the
security-reviewer pass (commit `2b6e32c15`, 0 findings, 4 green cases).

The orchestrator, 2026-11-03 (decisions.md row 16:5xZ), verbatim:

- "F1 blocker (er.rs:579). take_or_wait waits while the caller holds the lifecycle flock, so the
  observation's relock fails MetadataUnavailable. Fix it in the unit. The adversary suggested: never wait
  when the caller holds the lifecycle lock (wait only from unlocked opens). Its red case
  checkpoint_tests.rs::a_locked_open_never_waits_for_a_handle_whose_user_waits_for_its_lock must turn
  green."
- "F2 (owner.rs:668, uncached description) and F3 (owner.rs:567, bound description): both are in scope.
  The acceptance says one cold start verifies once, and both cold reads verify the store twice: (2,2,2)
  instead of (1,1,1). Fix both in the unit. For F2 the suggestion is that the owner's `cached` request
  with launch:true answers the launch. For F3, route the bound describe through the owner. Both red cases
  in cold_owner_start_adversary.rs must turn green."
- "F4: owner.md and invoke.rs:1001 disagree on whether `invoke --describe` starts the owner. Align them in
  the unit and pick the smaller change."
- "Merge both review commits (23247269c, 2b6e32c15) into the unit branch."
- "After the fix, re-run only the targeted suites (connectors-host --lib checkpoint_tests; connectors
  --test cold_owner_start, _adversary, _security). One short adversary re-check of F1-F3 is enough."
- "Stop criterion as in the brief: if the same cause fails twice, report and stop. A finding outside the
  acceptance becomes a from-finding draft."

Applied: `2b6e32c15` merged into `unit/d417-cold-owner-start` at `6a72056d7` (`23247269c` was committed on
that branch by the adversary); the four findings went back to the U1 implementor as correction round 2.
