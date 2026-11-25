---
format: aep.planning-md/3
id: review-result:test-step-stories-scope-round-2
kind: review-result
status: active
title: Scope critic, round 2, the test-step story set
relations:
- reviews: story:the-gates-test-step-ends-within-120-s-on-the-forge
revision: 1
---
Verdict: needs-revision.

Round 2 of the scope critic on the 18 drafts of
`story:the-gates-test-step-ends-within-120-s-on-the-forge`, read on 2026-11-25 on the branch
`plan/test-step-stories`. Read-only, nothing was run. The set claims every one of the 81 tests
once. The three round-1 findings are resolved. What fails is the parent: it states the gap to 120 s
and gives it no owner, and it understates which tests have no change behind them.

Short names below: S1 four-host, S2 seven-cli of a slow owner, S3 eleven connection-lease, S4
four-idle-exit, S5 seven-mcp, S6 http and sql timeouts, S7a catalog-provider, S7b catalog-crate,
S8a eight adapter and catalog, S8b five-host, S8c seven-cli shorten, S9a eighteen template store,
S9b six-cli jail binaries, S9c the 100-run stress, N1 the lane, N2 the helper, N3 the
connection-lease question, N4 the identification question.

## Findings, most severe first

1. Parent (blocker): the gap of 408 s of wall time against 120 s is stated ("What closes that gap is
   not known") and no story, question or named follow-up owns it. The test plan says a step that
   stays over budget after the seams is "a story of its own". Change: name an owner for the gap and
   for acceptance 1.
2. Parent (warning): acceptance 3 (a gate run whose test step takes longer than 120 s exits non-zero
   and names the time) is claimed by no child, and the parent does not say it carries it; acceptance
   2 holds today (maximum 47.2 s, nextest ends a test at 60 s) and the parent does not say so.
3. Parent (warning): "Not claimed by a change, among the 81: 2 tests" understates. Counted as
   claimed with no change behind them: `adversary_a_store_replaced_under_the_owner_ends_the_session`
   (16.5 s, S5, estimate 6 s with no seam), `a_rate_limited_read_waits_once_…` (8.3 s, S8a), the six
   jail tests (50.7 s, S9b, no saving estimated). S7b's row counts a test its body disclaims; S9a's
   two `http_standing_grant` tests have an unproven reach. Change: list them under "what no story
   claims" and give the S5 row an "after" that has a change behind it.
4. Parent (warning, pre-existing): the overlap with `story:store-tests-end-within-60s-under-load`
   (the same shared-store form for eight S9a tests, S9c's stress test and one S2 test) is named and
   not decided. Change: say which story owns each test, or narrow the older one.
5. Parent (warning, pre-existing): the second run's 842.2 s against 621.8 s (62 more tests) is
   unexplained and owned by nothing. Change: give it an owner, for instance the measuring story
   below.
6. N2 (warning): the story moves three existing production sites (`idle_bound`,
   `owner_stop::bound`, `acquisition_ms`) onto the helper, which neither the parent nor the plan
   asked for. Change: drop it, or record it in N2 and in the parent as a decision.
7. N1 (warning): acceptance 5 runs the eight existing Timing entries in the lane for the first time
   and allows moving them to `Deferred`; the parent states only the widened paths. Change: say both
   in the parent, or limit the lane to entries this set adds.
8. S6 (warning): adds a lane test at the 5 s connect timeout and an acceptance for it; the plan
   names runs at 15 s and 10 s only. Change: record it as a decision beyond the plan or drop it.
9. S8c (warning): acceptance 3 asks for 20 runs in a row of one test recorded in the pull request,
   which neither the parent nor the plan asked for. Change: drop it or mark it as a decision.
10. Parent (warning): "seven waves (N1 and S8b first, S3 last)" is stale: the set spans waves 1 to
    8, the draft store has 11 waves, and N3 and N4 have no scope and appear in no wave.

## Round-1 findings

- Catalog-crate seam: resolved (no in-process memo; one test, 1.583 s to about 0.14 s).
- Release-build acceptance in five stories: resolved (each points to the helper story).
- Three overrides against the parent's "two": resolved.

## Coverage count

- Tests per story: S1 4, S2 7, S3 11, S4 4, S5 7, S6 5, S7a 4, S7b 1, S8a 7, S8b 5, S8c 6, S9a 12,
  S9b 6, S9c 1, N4 1. Sum 81. No test claimed twice, none by nothing.
- 1,117.8 s before and 329.7 s after recompute from the table.
- 4 tests have no change behind them (rows 63, 59, 20, 46), not 2. S3's 11 tests are 222.9 s of the
  1,117.8 s and rest on an open design.

## The gap

Promises of the parent: acceptance 1, 2, 3, and the line where each of the 81 tests spends its
time. Traced: the coverage fully, acceptance 1 partly, acceptances 2 and 3 to nothing. Smallest
addition, not drafted: one question story in the style of N3 and N4, "where do the 1,298 s of the
tests under 5 s go", over the 406 tests of 1 s to under 5 s (814 s) and the 3,831 under 1 s (543 s),
with a written answer that names the stories which then claim them; it could take the unexplained
842 s run too. Then one sentence in the parent naming that story as owner of the gap and of
acceptances 1 and 3.

## Approve as they stand (scope only)

S1, S2, S4, S5, S7a, S7b, S8a, S8b, S9a, S9c, N3, N4. Not yet: S6 (finding 8), S8c (finding 9), N1
(finding 7), N2 (finding 6). S3 is scope-sound but open on N3; S9b carries no estimated saving.

## Not established

- Whether the 120 s budget is reachable at all.
- Whether S9a's setup script runs under the pinned nextest.
- The cost of `Metadata::initialize`.
- Whether the two `http_standing_grant` tests are reached by the template.
- Whether `story:test-binary-per-package-rest` changes any claim of the set.

## Outside this critic's role

- N1 collides with `claude-getting-started-tutorial` and `mounted-custody-kind-secret-volume` on
  `ignored.rs`, and with the lane's `docs/development.md`.
