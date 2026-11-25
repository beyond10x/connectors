---
format: aep.planning-md/3
id: review-result:test-step-stories-acceptance-round-2
kind: review-result
status: active
title: Acceptance critic, round 2, the test-step story set
relations:
- reviews: story:the-gates-test-step-ends-within-120-s-on-the-forge
revision: 1
---
Verdict: needs-revision.

Round 2 of the acceptance critic on the 18 drafts of
`story:the-gates-test-step-ends-within-120-s-on-the-forge`, read on 2026-11-25 on the branch
`plan/test-step-stories`. Read-only, nothing was run. "Verified" was read in the tree, "inferred"
was not run. Nine of the sixteen round-1 findings and both set-wide defects are resolved; three
findings are still open and one new defect appeared.

## Findings, most severe first

1. `story:eleven-connection-lease-tests-advance-a-clock-instead-of-sleeping` (blocker): acceptance
   1's bound (72 s, from the untimed 48.0 s) and the lane test at the real 30 s admission bound rest
   on a seam that is open until
   `story:where-do-the-connection-lease-tests-clock-and-the-30-s-admission-bound-live` answers.
   Round-1 finding 5 repeated. Change: keep it in draft until the question story's `## Answer`
   exists, then name the 30,000 ms site and the route that gets group B under the bound.
2. `story:seven-mcp-tests-use-a-4-s-request-budget-and-shorter-holds` (blocker): test 7,
   `adversary_a_store_replaced_under_the_owner_ends_the_session`, is in acceptance 1's sum with no
   change named. Inferred from the story's figures: unchanged at 16.5 s the sum is 50.5 s against a
   bound of 51 s. Change: take it out of the sum and the title, or name a change for it.
3. `story:eight-adapter-and-catalog-tests-drop-waits-that-prove-nothing` (blocker): acceptance 4
   requires `slow_lane_no_suite_sees_the_installation_another_suite_sets` to pass in the lane, and
   the story says the test cannot fail there. Change: delete the test, or add a lane step that runs
   the whole binary in one process and name its command.
4. Same story (warning): acceptance 3 reads the stdout of a passing test. Verified:
   `.config/nextest.toml` sets no `success-output`. Change: the test fails when `GITHUB_ACTIONS` is
   set and the path came from the walk.
5. `story:eighteen-tests-take-a-template-store-or-jail-built-once-per-run` (warning): acceptance 1
   reads setup-script output that nextest may not print on success. Change: say where the line is
   read, for example a gate-printed line (`crates/connectors-build/src/gate.rs` prints `gate:`
   lines).
6. Same story (warning): acceptance 3 is a duty to record a figure in the pull request description.
   Round-1 finding 3, partly resolved. Change: drop it or replace it with something a command reads.
7. `story:six-cli-tests-take-jail-binaries-built-once-per-run` (warning): acceptance 1 has the
   visibility gap of finding 5.
8. Same story (warning): acceptance 3 has the visibility gap and the record duty.
9. `story:the-catalog-crate-tests-ingest-the-released-definition-once-per-process` (warning):
   acceptance 1's bound of 1 s rests on 0.14 s, the mean of another test. Verified:
   `the_released_github_definition_lowers` still reads the file, hashes it and runs
   `definition::parse` before it lowers; that cost was never timed. Change: time `parse` first, or
   rest the story on acceptance 2 and widen or drop the time bound.
10. `story:the-lane-slow-runs-the-ignored-timing-family` (warning): acceptance 4 holds the 60 s end
    with a fixture test that sleeps, which would break the 60 s test limit in the gate. Change: the
    limit is a parameter of `execute_test` and the test passes a limit of a second or two.
11. `story:the-catalog-provider-tests-read-a-committed-lowering-of-the-released-definition`
    (warning): acceptance 3 joins two outcomes; acceptance 4's "fails when one byte differs" names
    no run that shows it. Change: split acceptance 3 and name a test that mutates a copy of the
    fixture.
12. `story:seven-cli-tests-shorten-their-waits-or-a-production-wait` (warning): acceptance 3 joins a
    stated comment and "20 runs in a row" on an unspecified machine, recorded in the pull request
    description. Change: split it and name the command.
13. `story:four-idle-exit-tests-no-longer-span-several-5-s-recovery-sweeps` (warning): acceptance 2
    asserts a sweep counter read from `idle_exit`; no Change point adds one. Verified: `idle_exit`
    returns `Option<Duration>`. Change: name where the count comes from.
14. `story:five-tests-no-longer-wait-out-the-http-and-sql-client-timeouts` (warning): the lane
    command of the two host tests carries the placeholder `--exact <name>`. Change: write both
    commands.
15. `story:where-do-the-connection-lease-tests-clock-and-the-30-s-admission-bound-live` (warning):
    question 2 has no stopping rule for "shortest lifetime", question 1 proves four tests by one run
    of one test, and acceptance 1 ends with a consequence for another story. Change: state the pass
    criterion and ask for each of the four tests.

## Round-1 findings

1 resolved. 2 resolved. 3 partly (finding 6). 4 resolved (modulo finding 5). 5 not resolved, by
design (finding 1). 6 resolved. 7 resolved (the counter is finding 13). 8 resolved (new bound
problem: finding 9). 9 mostly (finding 11). 10 partly (finding 3). 11 resolved. 12 resolved
(visibility: finding 4). 13 resolved. 14 resolved (the unchanged test 7 is finding 2). 15 mostly
(finding 14). 16 resolved. Set-wide 1 (no trigger or test for the lane) resolved by the lane story.
Set-wide 2 (no job for the release-build check) resolved by the helper story and the lane's second
step.

## Approve as they stand

- `story:five-host-unit-tests-shorten-the-waits-they-chose`
- `story:four-host-tests-no-longer-wait-out-the-30-s-lock-wait`
- `story:one-debug-only-override-helper-is-forwarded-to-the-owner-and-off-in-a-release-build`
- `story:seven-cli-tests-of-a-slow-owner-start-no-longer-wait-out-the-10-s-connect-wait`
- `story:the-100-run-capacity-stress-keeps-10-runs-in-the-gate`
- `story:where-do-the-10-s-of-the-identification-retry-loop-test-go`

One edit each to the acceptance, then they can start: the lane story (finding 10), the http and sql
story (finding 14), the idle-exit story (finding 13), the catalog-provider story (finding 11).

## Does any acceptance depend on an estimate being right

Inferred: yes, in every change story. Acceptance 1 is a sum of durations with a bound of 1.5 times
an estimate nobody timed. For most a wrong estimate sends the story back for measurement and never
gives a false pass. Exposed: the seven-mcp story (finding 2), the catalog-crate story (finding 9),
the eleven-tests story (finding 1). The six-cli story's bound is 76 s against 50.7 s before, so
acceptance 1 passes with no change; its real check is the inode test in acceptance 2.

## Verified in source

- All 86 test names the stories cite exist as functions under `crates/`, `apps/` or `adapters/`.
- The seams exist: `child_lock_wait`, `set_lifecycle_lock_wait`, `Background::start`,
  `Client::connect`, `QUERY_TIMEOUT`, the `idle_bound` cfg pair.
- The `ignored` command accepts `--family timing` and prints its report to stdout.
- The tree is at 0.78.2; some cited lines drifted from the stories' 0.78.1.

## Not established

- Whether nextest 0.9.143 prints the output of a passing setup script.
- No "after" figure was timed, and neither was `definition::parse`.
- Whether acceptance 4 of the catalog-provider story fails on one changed byte, and whether the
  two-read test reaches 6 s.

## Outside this critic's role

- The set leaves about 408 s of wall time against 120 s by the parent's arithmetic.
- Five stories write `transport.rs` and `slow.yml`.
- Several ids no longer match their titles.
- The helper's home in `connectors-sdk`, which the adapters build against.
