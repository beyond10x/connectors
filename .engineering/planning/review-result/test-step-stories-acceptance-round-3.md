---
format: aep.planning-md/3
id: review-result:test-step-stories-acceptance-round-3
kind: review-result
status: active
title: Acceptance critic, round 3, the test-step story set
relations:
- reviews: story:the-gates-test-step-ends-within-120-s-on-the-forge
revision: 1
---
Third read, acceptance critic, 2026-11-25. Read-only; nothing was run.

## Verdict

needs-revision for the set. Two stories block:
story:seven-cli-tests-shorten-their-waits-or-a-production-wait and
story:eleven-connection-lease-tests-advance-a-clock-instead-of-sleeping. The other 18 are approved
on their own.

## Verdict per story

- `the-lane-slow-runs-the-ignored-timing-family`: approve
- `one-debug-only-override-helper-is-forwarded-to-the-owner-and-off-in-a-release-build`: approve
- `eight-adapter-and-catalog-tests-drop-waits-that-prove-nothing`: approve
- `eighteen-tests-take-a-template-store-or-jail-built-once-per-run`: approve
- `six-cli-tests-take-jail-binaries-built-once-per-run`: approve
- `seven-mcp-tests-use-a-4-s-request-budget-and-shorter-holds`: approve
- `the-catalog-provider-tests-read-a-committed-lowering-of-the-released-definition`: approve
- `the-catalog-crate-tests-ingest-the-released-definition-once-per-process`: approve
- `four-idle-exit-tests-no-longer-span-several-5-s-recovery-sweeps`: approve
- `seven-cli-tests-of-a-slow-owner-start-no-longer-wait-out-the-10-s-connect-wait`: approve
- `five-tests-no-longer-wait-out-the-http-and-sql-client-timeouts`: approve
- `seven-cli-tests-shorten-their-waits-or-a-production-wait`: needs-revision (new finding 1)
- `eleven-connection-lease-tests-advance-a-clock-instead-of-sleeping`: needs-revision (round-2
  finding 1)
- `where-do-the-connection-lease-tests-clock-and-the-30-s-admission-bound-live`: approve
- `where-do-the-10-s-of-the-identification-retry-loop-test-go`: approve
- `where-do-the-1298-s-of-the-tests-under-5-s-go`: approve
- `store-tests-end-within-60s-under-load`: approve (note 6)
- `the-gates-test-step-ends-within-120-s-on-the-forge` (parent): approve (note 8)
- `four-host-tests-no-longer-wait-out-the-30-s-lock-wait`: approve
- `the-100-run-capacity-stress-keeps-10-runs-in-the-gate`: approve

## Round-2 findings

All paths are under `.engineering/planning/story/`.

1. Not resolved, by design. `eleven…` lines 88 to 96 say the story stays draft and is revised from
   the question story's `## Answer`. Its Change names no seam, and acceptance 1 (72 s) and the lane
   test at 30 s (lines 144 to 158) still rest on that open seam. The edge it records is correct.
   needs-revision until the answer exists and the story names the 30,000 ms site and the route for
   group B.
2. Resolved. `seven-mcp…` lines 28 to 31, 100 to 102, 104 to 108 take the 16.5 s test out of the
   sum and the title. Acceptance 1 is 85.3 s before and a 42 s bound.
3. Resolved. `eight…` lines 100 to 116 remove the test, and acceptance 4 (lines 148 to 149) is a
   `grep` that prints nothing. `adversary_one_binary.rs:159` still holds it today.
4. Resolved. `eight…` lines 117 to 123, 145 to 147: the test fails when `GITHUB_ACTIONS` is set and
   the path came from the walk, so a pass is observable without stdout.
5. Resolved. `eighteen…` lines 175 to 180, 202 to 205: the gate prints `gate: test-fixtures: …`
   lines, so the check reads the gate's own output.
6. Resolved. `eighteen…` lines 211 to 212: acceptance 3 is a sum of durations. The duty to record a
   figure in the pull request is gone.
7. Resolved. `six…` lines 99 to 115: acceptance 1 is three unit tests that compare inodes, so no
   setup-script output is read.
8. Resolved. `six…` lines 116 to 122: acceptance 3 is a `git diff --name-only` and has no record
   duty.
9. Resolved. `catalog-crate…` lines 82 to 91, 93 to 101: the time bound is dropped, the duration is
   "reported, not accepted", and acceptance 1 is a `sed | grep -c` that prints 0.
10. Resolved. `lane…` lines 119 to 122, 157 to 161: `execute_test()` takes the limit as a parameter
    and the test uses 1 s on a 30 s sleeper.
11. Resolved. `provider…` lines 151 to 177: acceptances 3 to 7 are split. Acceptance 7 mutates a
    copy of the fixture and names the test.
12. Resolved. `seven-cli…` lines 196 to 204: the comment check (acceptance 3) and the 20 runs
    (acceptance 4) are split. Acceptance 4 names its command.
13. Resolved. `idle…` lines 76 to 85, 128 to 131: the count comes from a `#[cfg(test)]` counter on
    `Background`, returned by `idle_exit`. `idle_exit` returns `Option<Duration>` today
    (`transport.rs:4790`) and `Background::start` has three callers.
14. Resolved. `five-tests…` lines 158 to 163 write all three commands out with real names.
15. Resolved. `lease-question…` lines 61 to 80, 84 to 90: the pass criterion is 10 of 10 runs with a
    halving search, question 1 is asked per test, and acceptance 1 no longer ends on a consequence
    for another story.

## New findings

1. blocker. `seven-cli-tests-shorten-their-waits-or-a-production-wait`, lines 206 to 207, against
   `clock.rs:68-77`, `config.rs:2296` and `transport.rs:1923`. Acceptance 5 says "a unit test per
   site asserts the value the code passes without the variable".
   - Clock site: under `#[cfg(test)]` `query_timeout()` returns the per-thread value, and the helper
     version exists only under `#[cfg(not(test))]` (lines 161 to 167 of the story). A unit test of
     `connectors-host` can never run the definition it would assert on.
   - Print site and drain site: both are inline `Duration::from_secs(…)` literals in the middle of
     a function. No Change point extracts a function a test could call.
   - To approve: name for each site the function or constant a test reads. For the clock, use a
     pure function outside the `cfg(test)` split, or drop the unit test and say which integration
     run shows the 2 s. Or reword acceptance 5 to what can be observed.
2. note. `lane…` line 100 still says the dependent stories cite the command with
   `-p connectors-build`. They now cite `--bin connectors-build`. Delete the sentence.
3. note. `eight…` lines 143 to 144, acceptance 2: "the bundle test fails when the swapping thread
   completed fewer than 100 swaps". Nothing shows it fails; it can only be read in the source. A
   one-line check on the assertion would make it observable.
4. note. `catalog-crate…` lines 95 to 101: nothing shows that
   `the_released_github_definition_lowers` reads the committed directory. Acceptance 1 passes if
   the call is deleted, and only the clause "assertions it has today" in acceptance 2 guards that.
   Add a grep for the fixture path.
5. note. `six…` lines 112 to 115: `install_copies_a_source_it_cannot_link` joins two cases, a
   source with mode 0775 and a source on another file system. The second uses `/proc/self/exe`,
   marked inferred. If it fails, the first case goes unproved.
6. note. `store-tests…` lines 105 to 117: four independent outcomes, and "host load of at least 60"
   is not producible on demand. "The report" is not named. Name where the two times are written
   and have the run print `uptime`.
7. note. Wave order. The three question stories are `unassessed` and in no wave. `eleven…` is in
   wave 8 and nothing in the wave list holds it until the answer exists; only its prose and the
   `depends_on` edge do.
8. note. `parent…` lines 190 to 200: acceptance 1 is not met by the 19 stories and needs the answer
   to `where-do-the-1298-s-of-the-tests-under-5-s-go`. The parent says so at lines 160 to 171.
   Acceptance 2 holds today, so it is a guard and not a transition.
9. note. `seven-cli…` lines 200 to 204: the 20 runs are done on the implementor's host and leave no
   record. A reviewer can only re-run them.
10. note. `idle…` lines 133 to 134: acceptance 3 names "a unit test of `maintenance.rs` or
    `transport.rs`" and no function. It is buildable because the idle helper sits under
    `cfg(debug_assertions)`, not `cfg(test)`.

## Not checked

- Nothing was run: every "after" figure, cargo-nextest 0.9.143 behaviour (setup scripts without a
  switch, `--stress-count`, `--no-tests fail`) and the runner's file-system and hard-link behaviour
  are unread.
- The test plan's rows and sums were not read line by line. Recomputed: 108.0 and 53.9 s
  (eighteen), 85.3 and 28.0 s (seven-mcp), 1,643.2 s and 410.8 s (parent).
- Cited lines were spot-checked only in `ignored.rs`, `clock.rs`, `transport.rs`,
  `maintenance.rs`, `fixture.rs` and `definition_published_features.rs`.
- Whether the 8 `Timing` entries (including the 60 s `cold_owner_start` release measurement) pass
  in the lane was not followed. The lane story allows moving one to `Deferred`.
- Out of this role: the home of the `connectors-sdk` helper; the `connectors-build` dependency on
  `connectors-host`; the cargo-nextest setup script; five stories writing `transport.rs` and
  `slow.yml`; the scope gap of about 1,298 s.
