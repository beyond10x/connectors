---
format: aep.planning-md/3
id: review-result:seven-cli-shorten-scope-reread-revision-10
kind: review-result
status: active
title: Scope critic, re-read of the seven-cli-shorten story at revision 10
relations:
- reviews: story:seven-cli-tests-shorten-their-waits-or-a-production-wait
revision: 1
---
Verdict: approve, for story:seven-cli-tests-shorten-their-waits-or-a-production-wait at
revision 10. A scope verdict only: notes 3 to 5 sit in the design and acceptance lanes and did not
set it.

Read-only re-read by the scope critic at revision 10, from main. Nothing was run. The round-3
record of this critic lists the story as `approve` with no note, so there was nothing to apply.

## Findings

1. Note. The `## Scope` bullet for `clock.rs` (story lines 294 to 297) is narrower than the Change
   (lines 178 to 185). The bullet lists the `#[cfg(not(test))]` `query_timeout()` and "one unit
   test of the constant `QUERY_TIMEOUT`". The Change also writes a new
   `overridable_query_timeout()` compiled in every build, a dead-code allowance for the unit-test
   build, and a test that asserts `overridable_query_timeout()` returns 2 s. The typed `scope:`
   path is right. Settles it: the bullet names the new function, the allowance and the test.
2. Note. `print_deadline()`, `drain_bound()` and `overridable_query_timeout()`, each with a unit
   test of its production value (story lines 167 to 189, acceptance 5), are not in the parent's
   "Decisions beyond the plan" (parent lines 128 to 146). They trace to the parent's "debug-only
   override" seam, so this is not reach beyond the parent. Settles it: one parent line, or one
   sentence in the story's "A decision beyond the plan".
3. Note, design lane. Three windows fall below the 2 s floor: the clock query timeout (a prompt
   source has 800 ms, story lines 154 to 157); the walk (`page_ms` 1,200 under
   `--deadline-seconds 3` leaves a page 1,800 ms, story line 91; today 5 s,
   `invoke_walks.rs:780`); the drain (2,000 ms around a 1,000 ms drain, story lines 134 to 135:
   about 0.75 s of slack; today `BOUND + 2 s`, `owner_tether_adversary.rs:181`). Possible: a drain
   window of 3,000 ms; `--deadline-seconds 4` with `page_ms` 1,400 (4.2 s total, 2.6 s slack); a
   clock value at or above 2 s, or a test a slow machine only makes slower.
4. Note, design lane. In
   `a_queued_read_another_session_cancels_runs_unsent_and_the_running_read_succeeds` the sleep
   before the cancel drops from 700 ms to 300 ms (`owner_cancel_adversary.rs:914`). The test needs
   the second session's read queued before the cancel arrives. The story does not say what 300 ms
   is a multiple of.
5. Note, design lane. `print_deadline()` changes the deadline of every `Executable::print`
   caller, including `--print-local-bootstrap` (`config.rs:1503`), whenever the variable is set.
   The story limits this to one CLI run. It does not say that the owner's `config.rs` calls also
   see the variable through the prefix forwarding.

## Held

- Plan rows: group M has 7 rows; this story claims 6 (11.5, 10.9, 10.2, 9.0, 7.0, 5.6 s) and
  story:where-do-the-10-s-of-the-identification-retry-loop-test-go claims row 46. No other story
  claims any of the six test names.
- Parent numbers: row S8c (6 tests, 54.0 s, 24.5 s, three overrides) matches; the 20 repeat runs
  match acceptance 4; the `transport.rs` write order matches.
- The exclusions match the parent's Limits and the neighbouring stories.
- The 8 typed `scope:` paths are the 8 files the Change writes. `support/standin.rs` is not
  written. `connectors-host` already depends on `connectors-sdk`: no manifest is touched.
- Other writers of the same files: `local_cli.rs` regions of the four-idle story and the
  capacity-stress story are other regions; `transport.rs` order helper, four-idle, slow-owner,
  this story, eleven-lease.
- All citations match main.
- Not checked: whether `setup check` runs `installed_provider_runs` in the CLI or the owner (the
  story says so itself).
