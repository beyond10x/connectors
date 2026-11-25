---
format: aep.planning-md/3
id: review-result:test-step-stories-scope-round-3
kind: review-result
status: active
title: Scope critic, round 3, the test-step story set
relations:
- reviews: story:the-gates-test-step-ends-within-120-s-on-the-forge
revision: 1
---
Third read, scope critic, 2026-11-25. Read-only; nothing was run.

## Verdict

needs-revision for the set. One story blocks:
story:eleven-connection-lease-tests-advance-a-clock-instead-of-sleeping, whose scope is open by its
own text. The other 19 are approved on scope. The new question story closes the gap the parent
left; the parent has two stale or incomplete lines (notes only).

## Verdict per story

- `the-gates-test-step-ends-within-120-s-on-the-forge` (the parent): approve (notes 1, 3)
- `the-lane-slow-runs-the-ignored-timing-family`: approve (note 6)
- `one-debug-only-override-helper-is-forwarded-to-the-owner-and-off-in-a-release-build`: approve
- `eight-adapter-and-catalog-tests-drop-waits-that-prove-nothing`: approve
- `eighteen-tests-take-a-template-store-or-jail-built-once-per-run`: approve (note 4)
- `six-cli-tests-take-jail-binaries-built-once-per-run`: approve
- `seven-mcp-tests-use-a-4-s-request-budget-and-shorter-holds`: approve
- `the-catalog-provider-tests-read-a-committed-lowering-of-the-released-definition`: approve
- `the-catalog-crate-tests-ingest-the-released-definition-once-per-process`: approve
- `four-idle-exit-tests-no-longer-span-several-5-s-recovery-sweeps`: approve
- `seven-cli-tests-of-a-slow-owner-start-no-longer-wait-out-the-10-s-connect-wait`: approve
- `five-tests-no-longer-wait-out-the-http-and-sql-client-timeouts`: approve
- `seven-cli-tests-shorten-their-waits-or-a-production-wait`: approve
- `eleven-connection-lease-tests-advance-a-clock-instead-of-sleeping`: needs-revision (finding 1)
- `where-do-the-connection-lease-tests-clock-and-the-30-s-admission-bound-live`: approve (note 5)
- `where-do-the-10-s-of-the-identification-retry-loop-test-go`: approve; its answer must name the
  story that claims the test, and it does (lines 55 to 57)
- `where-do-the-1298-s-of-the-tests-under-5-s-go`: approve (note 2); its answer is checkable (four
  figures, each with a source) and is read by the operator, since it names the stories that follow
- `store-tests-end-within-60s-under-load`: approve; the new section is at lines 123 to 131 and the
  `depends_on` edge to the eighteen-tests story at line 10
- `four-host-tests-no-longer-wait-out-the-30-s-lock-wait`: approve; the diff against HEAD is the
  `--bin` change only
- `the-100-run-capacity-stress-keeps-10-runs-in-the-gate`: approve; the `--bin` change only

## Round-2 findings

7 promises were extracted from the parent and 7 traced to an item: acceptance 1, acceptance 2,
acceptance 3, one claimant for each of the 81 tests, an owner for the 1,298 s gap, an owner for the
unexplained second run, a decision on the two older overlapping drafts. Acceptance 1 is traced to
the question story on the tests under 5 s as owner, not to a change, and the parent says so (lines
168 to 170).

The set claims 81 tests once each (4+7+11+4+6+1+5+4+1+7+5+6+12+6+1+1). The parent's sums
recompute: 1,117.8 s before, 332.8 s after, 1,643.2 s, 410.8 s, and a rest of 1,298.0 s.

1. Gap to 120 s has no owner: resolved. The question story on the tests under 5 s is the owner
   (parent lines 166 to 172; its `Defect` and acceptances 1 to 3).
2. Acceptances 2 and 3 not claimed: resolved. Parent lines 185 to 188 and 196 to 200: acceptance 3
   is carried by the parent itself, and acceptance 2 is shown to hold today with
   `.config/nextest.toml` line 5.
3. "Not claimed by a change" understated: resolved. Parent lines 132 to 144 list 4 tests with no
   change, 6 with no saving and 3 with unproven reach.
4. Overlap with `store-tests-end-within-60s-under-load`: resolved. Parent lines 174 to 179 state
   each story owns its tests; the older story carries a sentence (lines 123 to 131) and an edge to
   the eighteen-tests story (line 10).
5. Second run of 842.2 s unowned: resolved. Question 4 of the question story on the tests under
   5 s (lines 64 to 66); parent lines 168 to 169.
6. The helper story moves three existing sites: resolved. Recorded as a decision at its lines 101
   to 104 and in the parent at lines 121 to 122.
7. The lane runs the eight existing `Timing` entries: resolved. Lane story lines 129 to 136 and
   acceptance 5; parent lines 123 to 125. `ignored.rs` has 9 `Timing` entries, 8 without a
   prerequisite (lines 169 to 176 and 513).
8. The http-and-sql story's lane test at 5 s: resolved. Its "A decision beyond the plan"; parent
   line 125.
9. The seven-cli-shorten story's 20 repeat runs: resolved. Its "A decision beyond the plan",
   acceptance 4; parent lines 126 to 127.
10. Stale wave count: resolved. Parent lines 109 to 114 match `aep plan artifact waves`: 11 waves,
    the set in waves 1 to 8, none in 5, and the questions unassessed.

## New findings

1. blocker. `eleven-connection-lease-tests-advance-a-clock-instead-of-sleeping`, lines 88 to 96
   and 180. The story says it "stays `draft`" and "is revised … before it is proposed", and "Which
   of the last four files this story writes is settled by the answer". Its route 3 puts a clock
   offset in `connectors-sdk` into scope, which needs the operator's word. Neither scope nor the
   72 s bound can be judged. The lease question records no scope, so the waves tool places this
   story in wave 8 without regard to it (inferred, not run). Approve once the lease question has
   its `## Answer` and the story is revised with a settled scope.
2. note. The question story on the tests under 5 s, lines 61 to 63 and acceptance 3 (lines 75 to
   77): the options the answer may offer include "more tests at a time on the forge's runner" and
   "a split of the step". Parent line 204 and the folder rule exclude a larger runner and more
   shards as a fix. The answer can then end with acceptance 1 of the parent unmet. Say that
   acceptance 1 stays open in that case, and who owns it.
3. note. The parent, lines 169 to 170: the sentence that the question story "was drafted with the
   figures of the first revision (1,632.7 s and 408 s)" is stale; its revision 2 uses 1,643.2 s and
   410.8 s. Delete it. Also the catalog-crate table row (line 79) counts the 7.4 s test, which that
   story's body disclaims (its lines 115 to 117). The parent lists it as "no change named" at line
   134. The row should read "0 of the 81".
4. note. `eighteen-tests-take-a-template-store-or-jail-built-once-per-run`, lines 143 to 195,
   against the parent's decision list (lines 118 to 130). Three choices are not in the parent's
   list: a new direct dependency of `connectors-build` on `connectors-host`, which `slow.yml` and
   `pages.yml` also build; a digest over all of `crates/connectors-host/src/local/` (the plan keys
   it on `metadata.rs` and `metadata/`); a `gate.rs` edit that prints the fixture lines. The
   parent's own change is in `test_step` and its recorded scope is none, so the two edits to one
   function (eighteen-tests lines 224 to 226) are invisible to the waves tool.
5. note. The lease question, lines 86 to 90, does not say who reads the answer. The eleven-tests
   story does, and question 3 needs the operator. One sentence would do.
6. note. Lane story, point 3: the `connectors-sdk` test `release_build_has_no_debug_assertions`
   and the new limit parameter of `execute_test` are not in the parent's decision list. The
   existing entry
   `cold_owner_start::a_cold_describe_and_read_of_a_large_store_answer_within_60_seconds` now falls
   under the 60 s lane limit; `store-tests-end-within-60s-under-load` lines 83 to 85 record it
   killed at 60.496 s. Acceptance 5 covers this with `Deferred`.

## Not checked

- Whether the lane runs inside any time bound: the lane story says it was not measured.
- Whether the setup script runs under cargo-nextest 0.9.143.
- Whether hard links work on the forge runner.
- Whether the logs of jobs 138270416028 and 138329455972 are still retrievable for question 4 of
  the question story on the tests under 5 s.
- Spot-checked and found right: `http.rs` 238 to 239, `maintenance.rs` 48, `fixture.rs` 118 to 127,
  `transport.rs` 259 and 1923, `config.rs` 2296, `clock.rs` 30, `ignored.rs` 112, 144, 568, 794,
  842, 866, 1289, `gate.rs` 513 to 580. The remaining cited lines were not checked.
- The test sources of the 3,831 tests under 1 s.
- The nine `depends_on` ordering edges and the acceptance commands (other roles).
