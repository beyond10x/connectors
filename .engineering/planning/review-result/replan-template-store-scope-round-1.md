---
format: aep.planning-md/3
id: review-result:replan-template-store-scope-round-1
kind: review-result
status: active
title: Scope critic, round 1, the re-plan of the template-store story
relations:
- reviews: story:the-gates-test-step-ends-within-120-s-on-the-forge
revision: 1
---
Verdict: needs-revision. Read at commit 198fc306b. S9a is the archived template-store story,
S9d the registry draft, S9e the MySQL fixture draft, N6 the question story of the re-plan, N5 the
question story of the tests under 5 s.

## Findings

1. Blocking. N6, the items and the paragraph "Closed by the measurement". Nothing owns "Rest,
   27.47 s". The archived story says N6 "holds the rest as questions" and lists nine costs; the
   rest is not one of them. N6 closes only the 5.55 s of `initialize` out of it. The other
   21.9 s (27.47 − 5.55) have no item and no stated drop. Timers of `run5-table.txt` in no item,
   summed over the eleven tests: `host.feed` 3,467.8 ms, `host.capture_sql` 1,694.2 ms,
   `m.open_connection` 1,463.9 ms, `m.floor_record` 909.9 ms, `er.write_checkpoint` plus
   `er.enable_checkpoints` 698.0 ms. Item 8 covers 8.32 thread-seconds of `er.start`,
   `resynchronize` and `complete_snapshot` in one test; the three sum to 12.33 s over the
   eleven, so about 4.0 s of fresh-open cost sit in other tests. How the timers nest was not
   checked.
2. Blocking. `four-host-tests-no-longer-wait-out-the-30-s-lock-wait` line 118 (draft) says S9a
   "reads `metadata.rs` as an input of a digest and does not write it". S9a is archived and has
   no digest.
3. Blocking. `one-debug-only-override-helper-is-forwarded-to-the-owner-and-off-in-a-release-build`
   lines 124 to 128 (draft) name the setup script of S9a. No such script exists any more.
4. Note. N5 has two sentences the re-plan made false: line 16, "The eighteen stories"; line 50,
   "Not measured: the cost of `Metadata::initialize`".
5. Note. The hand-off of N6 item 11 to N5: N5 question 3 says it "reads no run made after a
   story of the set landed", and nothing in N5's questions or acceptance takes the figure.
6. Note. S9a's setup step is not named as dropped in S9d, N6 or the parent's list of dropped
   decisions. Its figure (0.27 s per run, the setting `experimental = ["setup-scripts"]`) is
   held only by the archived section.
7. Note. `store-tests-end-within-60s-under-load` still says the set "owns the change" for its
   tests. S9d removes 30.6 % on average and 7.7 % in `a_read_invoke…`; its acceptance is a sum
   of forge durations, not 60 s at load. The "shared fixture" route is now measured at 33 to
   38 ms per store. A sentence that the story keeps its own change if S9d does not bring the
   tests under 60 s at load would settle it.
8. Note. The archived story lists nine costs for N6; N6 has twelve. N6's title says "of the
   eleven store tests", and items 9 and 10 concern the SQL test.
9. Note. Parent, the list "Not claimed by a change … Each test below keeps its time in the sum
   after" now carries the bullets of S9d (76.0 s after) and S9e (0.6 s after). Both are claimed
   changes.

## Checked and found correct

- The twelve tests of S9a are each claimed once: eleven in S9d, one in S9e, with the "before"
  seconds of S9a's table. 108.0 + 7.0 = 115.0 s.
- The parent: 81 tests in 17 rows, 1,117.8 before, 361.2 after, 368.6, 268.6, 375.0, 1,671.6,
  417.9; "twenty-one drafts" matches the `decomposes` edges without the archived story.
- The cost table against `run5-table.txt`, timer by timer: BATCH 30,286.9 ms in 1,293 calls,
  REG 29,004.2 ms in 2,729, PARSE2 6,974.3 ms in 2,293, READBACK 6,995.8 ms, CATCHUP 5,745.3 ms
  in 1,972, PROJ 5,673.9 ms in 2,973, VALID 5,532.2 ms, `initialize` 5,546 ms in 101, the lock
  poll 170.55 s in 2,345. Named 90.21 s, rest 27.47 s. N6 item 6: 420 calls, 8,068.1 ms.
- S9d against the source: the cited lines of `er.rs` match; the five non-test callers; the
  order of checks; one file; the parent admits product code.
- S9e against the source: no `nodelay` in `fixture.rs`; the two `write_all` calls at lines 239
  to 245.
- Nothing is dropped without a word except findings 1 and 6. No person's name, machine path or
  estimate of working time in the three drafts.

## Not established

- The dependency's source and the `mysql_async` lines.
- The forge figures of job 138270416028.
- How the timers nest: the figure of finding 1 is a lower bound of what is unowned.

## For another critic

- S9d's scope paragraph names three drafts outside the set by reading
  (`connections-list-answers-through-the-owner`, `post-commit-metadata-failure-is-outcome-unknown`,
  `spec-models-operation-admission`) and two by the tool (`mcp-outbound-stdio-runtime`,
  `spec-models-operation-admission`). The two lists do not agree.
