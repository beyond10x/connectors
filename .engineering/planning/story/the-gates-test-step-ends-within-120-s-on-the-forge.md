---
format: aep.planning-md/3
id: story:the-gates-test-step-ends-within-120-s-on-the-forge
kind: story
status: draft
title: The gate's test step ends within 120 s on the forge, and fails when it does not
relations:
- serves: vision:independent-contract-adapters
- informed_by: test-plan:where-the-gates-test-step-spends-its-time
- depends_on: story:where-do-the-1298-s-of-the-tests-under-5-s-go
scope:
- confidence: cited
  path: crates/connectors-build/src/gate.rs
revision: 13
---
## Defect

The folder rule "Tests stay fast" gives the test step of a repository's gate 120 s of wall time and
says the step itself fails when it runs over that, locally and in CI. The gate prints the time and
does not fail on it (`crates/connectors-build/src/gate.rs`, `TEST_STEP_BUDGET` and the line
`gate: test step ran … s (budget 120 s)`).

Measured on the forge, pull request 291, job 138270416028, 2026-11-24 08:06 to 08:17 CET:

- `Starting 4317 tests across 240 binaries (223 tests skipped)`
- `Summary [ 620.625s] 4317 tests run: 4316 passed (9 slow), 1 failed, 223 skipped`
- `gate: test step ran 621.8 s (budget 120 s)`

The durations nextest printed sum to 2,475 s, 3.99 times the wall time: the runner ran four tests
at a time. A step of 120 s on that runner holds 480 s of tests.

| band | tests | sum |
|---|---|---|
| 20 s and more | 14 | 412 s |
| 5 s and more (holds the row above) | 81 | 1,118 s |
| 1 s to under 5 s | 406 | 814 s |
| under 1 s | 3,831 | 543 s |

The tests of 5 s and more by binary: `connectors-host` (unit tests) 19 tests 265 s,
`connectors::connection_lease_adversary` 6 tests 143 s, `connectors-catalog-provider::main` 9 tests
77 s, `connectors::connection_lease` 4 tests 68 s, `connectors::owner_slow_start_adversary` 3 tests
66 s, `connectors::local_cli` 4 tests 57 s, `connectors::security_mcp_deadline` 3 tests 49 s,
`connectors::owner_slow_start` 4 tests 47 s, `connectors::adversary_local_mcp_server` 4 tests 41 s.

All test time by binary: `connectors-host` 781 tests 616 s, `connectors-catalog-provider::main`
1,122 tests 364 s (median 0.049 s, 90th percentile 0.582 s), `connectors-catalog::main` 438 tests
135 s.

Six of the ten slowest tests ran 30.2 s to 31.9 s. Inferred, not read from their code: they wait
out a production wait of 30 s.

Second measurement, pull request 292, job 138329455972, 2026-11-24 10:30 to 10:44 CET:

- `Starting 4379 tests across 240 binaries (223 tests skipped)`
- `Summary [ 840.388s] 4379 tests run: 4379 passed (9 slow), 223 skipped`
- `gate: test step ran 842.2 s (budget 120 s)`

The two runs differ by 62 tests and 220 s. Why the second run was slower is not known: the
durations of its tests were not compared with the first run's. N5 (below) owns that comparison.

## Change

Decomposed on 2026-11-24 in test-plan:where-the-gates-test-step-spends-its-time, from the
durations of the first run and the sources. It names, for each of the 81 tests of 5 s and
more, the line the time is spent at, and groups them by cause. Nineteen draft stories follow
from it: fourteen change tests, two (N1, N2) build what the others use, and three (N3, N4, N5) are
questions. The first draft of twelve was revised twice, each time after four critics read it
(acceptance, design, scope, parallel safety; all four asked for a revision in both rounds).

Re-planned on 2026-11-25: S9a was measured before it was built and is not built (the operator's
decision; the section "Measured (2026-11-25): not built as written" of that story). A store from
nothing costs 51 to 58 ms on the build host and a template saves 33 to 38 ms per store: about
1.5 s of the 31 s its acceptance asked for. S9a is archived. Three drafts take its place: S9d and
S9e each change code for the tests S9a named, and N6 is a question. The critics of the re-plan
(round 1, all four asked for a revision) found that a comment of the specification cites
`er.rs` by line and a test holds those lines, so S9d could not change the file alone; N7 was
added for it. The set is twenty-two drafts now: fifteen change tests or the product code their
time is spent in, three build what the others use (N1, N2, N7), and four are questions.

Two findings of that round are open: S1 (in the paragraph on `metadata.rs`) and N2 (in the
paragraph on the setup script) still describe S9a as it was planned. Their story files are
changed by their own open pull requests, so the two sentences are corrected in the planning
store after those pull requests merge. The stories that are implemented keep their sentences
about S9a as written; S9a's section names what took its place.

"Before" and "after" are seconds summed over the tests of 5 s and more that a story covers,
from the first run; every "after" is an estimate and none was timed.

| story | tests | before | after | seam |
|---|---|---|---|---|
| S1 `story:four-host-tests-no-longer-wait-out-the-30-s-lock-wait` | 4 | 122.0 | 5.5 | `child_lock_wait` and `set_lifecycle_lock_wait` exist; the 30 s test goes to the lane `slow`, a library test replaces it in the gate |
| S2 `story:seven-cli-tests-of-a-slow-owner-start-no-longer-wait-out-the-10-s-connect-wait` | 7 | 120.8 | 36.0 | one debug-only override of the 10 s connect wait |
| S3 `story:eleven-connection-lease-tests-advance-a-clock-instead-of-sleeping` | 11 | 222.9 | 48.0 | open until N3 answers; a clock offset in `connectors-sdk` needs the operator's word |
| S4 `story:four-idle-exit-tests-no-longer-span-several-5-s-recovery-sweeps` | 4 | 67.4 | 9.5 | a debug-only sweep interval |
| S5 `story:seven-mcp-tests-use-a-4-s-request-budget-and-shorter-holds` | 6 | 85.3 | 28.0 | the tests' own values; the story had seven tests and now names six |
| no story: `adversary_a_store_replaced_under_the_owner_ends_the_session` | 1 | 16.5 | 16.5 | none; the test left S5, which found no value of the test's own to shorten |
| S6 `story:five-tests-no-longer-wait-out-the-http-and-sql-client-timeouts` | 5 | 56.2 | 5.4 | two debug-only HTTP overrides; the SQL timeout by a constructor parameter |
| S7a `story:the-catalog-provider-tests-read-a-committed-lowering-of-the-released-definition` | 4 | 35.9 | 4.7 | a committed lowered file with a digest of its sources; it reaches 40 tests in all, about 94.9 s to 9.7 s |
| S7b `story:the-catalog-crate-tests-ingest-the-released-definition-once-per-process` | 1 | 7.4 | none stated | none for this test (`embedded_text_where_the_engine_refuses_it_is_refused`); the story reads S7a's file in one test outside the 81 and states no time after |
| S8a `story:eight-adapter-and-catalog-tests-drop-waits-that-prove-nothing` | 7 | 73.0 | 16.6 | the tests' own values; one test is removed; one stays at 8.3 s |
| S8b `story:five-host-unit-tests-shorten-the-waits-they-chose` | 5 | 48.6 | 18.2 | the tests' own values |
| S8c `story:seven-cli-tests-shorten-their-waits-or-a-production-wait` | 6 | 54.0 | 32.2 | two debug-only overrides: print deadline, client drain bound; the clock test runs its three rows at once; one test keeps its 7.0 s |
| S9d `story:the-entity-runtime-registry-and-the-definition-bundle-are-built-once-per-process` | 11 | 108.0 | 76.0 | product code: the Entity Runtime registry and the map of commands are built once per process, where every persist and every fresh open built them; the eleven store tests of S9a |
| S9e `story:the-mysql-test-fixture-sends-its-packets-without-the-nagle-hold` | 1 | 7.0 | 0.6 | the test fixture sets `TCP_NODELAY` on the accepted socket; the cause is a reading that the story's first acceptance statement confirms or refutes; the SQL test of S9a |
| S9b `story:six-cli-tests-take-jail-binaries-built-once-per-run` | 6 | 50.7 | 50.6 | `Jail::install` links the binaries into the jail and copies where it cannot link; no saving is estimated |
| S9c `story:the-100-run-capacity-stress-keeps-10-runs-in-the-gate` | 1 | 31.9 | 3.2 | 10 runs in the gate, 100 in the lane `slow` |
| N4 `story:where-do-the-10-s-of-the-identification-retry-loop-test-go` | 1 | 10.2 | 10.2 | a question; no change claimed |
| sum | 81 | 1,117.8 | 361.2 | the "after" sum leaves out S7b's test, which has no figure |

The ids of S5, S7b, S8a, S8c and S9b keep the words of their first titles; the store does not
change an id. S9a `story:eighteen-tests-take-a-template-store-or-jail-built-once-per-run` has no
row since 2026-11-25: its row was 12 tests, 115.0 s before and 55.9 s after, and the "after" sum
was 340.5 s (340.5 − 55.9 + 76.0 + 0.6 = 361.2).

Without a row, because they cover no test of their own:

- N1 `story:the-lane-slow-runs-the-ignored-timing-family`: an ignored test whose function name
  starts with `slow_lane_` runs in the lane `slow` and is ended at 60 s. It owns
  `.github/workflows/slow.yml` and the table of `crates/connectors-build/src/ignored.rs`. The lane
  gets the step that permits unprivileged user namespaces, the pinned cargo-nextest and a
  release-build step that fails when it runs no test. With its paths the lane starts for nearly
  every pull request that changes code; the lane has no step budget yet.
- N2 `story:one-debug-only-override-helper-is-forwarded-to-the-owner-and-off-in-a-release-build`:
  one helper for every debug-only override, forwarded to the owner by one prefix, and a test of
  the gate that refuses a direct read of an override outside the helper.
- N3 `story:where-do-the-connection-lease-tests-clock-and-the-30-s-admission-bound-live`: the
  question S3 waits for.
- N5 `story:where-do-the-1298-s-of-the-tests-under-5-s-go`: the question that owns the gap to the
  budget, the unexplained second run of 842.2 s, and what acceptance 1 needs beyond the other
  stories of the set (below).
- N6 `story:where-do-the-82-s-of-the-eleven-store-tests-go-that-the-registry-story-leaves`:
  thirteen items that the measurement of 2026-11-25 and the critics' reading left open in the
  tests of S9d and S9e, none attributed far enough for a story. The largest is the write of the
  Entity Runtime batch, 30.29 s of 117.685 s on the build host.
- N7 `story:the-specification-cites-er-rs-by-function-name-not-by-line`: two comments under
  `ess/domains/` cite `er.rs` by line and a test of `connectors-build` holds one of them, so a
  change above line 5095 of `er.rs` makes that test red. N7 makes the citations name a
  function. It claims no test time. S9d has a `depends_on` edge to it.

Edges (`depends_on`) order the stories that write one file.
`aep plan artifact waves --kind story --status draft`, run after the notes of the third round,
derives 11 waves for the drafts of the store and puts the set into waves 1 to 8: wave 1 N1, S8a,
S8b and S9b; wave 2 S1 and N2; wave 3 S5, S6, S7a, S9a and S9c; wave 4 S4 and S7b; wave 6 S2;
wave 7 S8c and this story; wave 8 S3. No story of the set is in wave 5. This story records
`crates/connectors-build/src/gate.rs` as scope and had `depends_on` edges to S9a and N5.
story:store-tests-end-within-60s-under-load records the files of its seven tests as scope and is
in wave 4. The three questions and story:test-binary-per-package-rest record no scope; the tool
lists them as unassessed and puts them in no wave. The wave of this story is not the place of its
change: the tool reads the file and the edge to S9a, N5 is in no wave, and wave 8 holds S3. The
change waits until acceptance 1 holds (below).
Since the re-plan of 2026-11-25 S9a is in no wave (archived). The same command, run that day,
puts S9d and S9e in wave 1, story:store-tests-end-within-60s-under-load in wave 2 behind S9d, and
this story in wave 5. N6 records no scope and is unassessed. This story's edge to S9a is removed:
it ordered two writes of `gate.rs`, and neither S9d nor S9e writes that file. Its edge to N5
stays. S9d writes `crates/connectors-host/src/local/metadata/er.rs`, which no other story of the
set writes; the tool reports two drafts outside the set that share it, and S9d names them.
After the second round of the re-plan (N7, S9d's edge to it, and the edges of N5 and N6 to S9d)
the same command derives 11 waves and puts S9e in wave 1, N7 in wave 4, S9d in wave 6, and
story:store-tests-end-within-60s-under-load and this story in wave 7. N7 has no `depends_on`
edge. Its wave is how the tool keeps apart the stories that record `ess/domains/cli.yaml`:
three drafts outside the set record it with inferred scope, in waves 1, 2 and 6, and none of
them is approved or started. N7 is the first story of the re-plan to be implemented; a draft
that changes `cli.yaml` after it starts from a head that holds it. N5 and N6 record no scope and
stay unassessed.
`crates/connectors-host/src/local/owner/transport.rs` is written by N2, S4, S2, S8c and S3, in
that order.

Decisions beyond the plan, each recorded in its story:

- N2: a test on a release build that an override is ignored there.
- N2: the three existing override sites (`idle_bound`, `owner_stop::bound`, `acquisition_ms`) move
  onto the helper, so that the gate's test of direct reads needs no exception.
- N1: the lane runs, for the first time in any job, the eight `Timing` entries without a
  prerequisite that the table of `ignored.rs` holds today, and may move one of them to `Deferred`.
- S6: one lane test at the 5 s connect timeout; the plan names runs at 15 s and at 10 s.
- S8c: `clock_check_below_quorum_refusal` gets no override of the clock query timeout. Its three
  rows run at once, each with a silent source of its own, at the production 2 s. The plan's
  override left a prompt source 800 ms.
- S8c: the functions `print_deadline()` and `drain_bound()`, each with a unit test of its
  production value.
- S8c: `a_queued_read_another_session_cancels_runs_unsent_and_the_running_read_succeeds` is not
  changed. The plan cut two of its sleeps; what they outlast was not established.
- S8a: the test `no_suite_sees_the_installation_another_suite_sets` is removed. The plan moved it
  to the lane, where it cannot fail either.
- S9b: no product built once per run; `Jail::install` links the file cargo built.
- S9a, not built since 2026-11-25; its decisions go with it: a direct dependency of
  `connectors-build` on `connectors-host`, a digest over all of
  `crates/connectors-host/src/local/`, an edit of `test_step` in `gate.rs` that prints the
  lines of the setup step, and the setup step itself (a nextest setup script, which needs the
  setting `experimental = ["setup-scripts"]` and costs about 0.27 s per run for a `cargo run`
  that builds nothing). No story of the set uses a setup script now.
- S9d: a change of product code, where the plan changed only tests and their fixtures. It keeps
  records, errors and their order, and its first acceptance statement is a test that counts the
  builds.
- S9d: its bound is "base" minus half the estimated saving (16.0 s), where "base" is the sum of
  the eleven tests in the newest gate job on main at or before the pull request's base commit:
  92.0 s for the 108.0 s of the plan. The set's rule, the estimate times 1.5, gives 114 s, above
  the 108.0 s before.
- S9d and S9e: a pull request whose gate job is over the time bound is not run again as a
  remedy. It may merge when the other acceptance statements hold; the story then stays active
  and the durations go to the operator.
- N7: the specification cites code by function name, and the test that held a citation by line
  finds the function.
- S9e: its bound is 2.0 s. The estimate times 1.5 gives 0.9 s, narrower than one `ess generate`
  call of the test varies on the build host (0.10 to 0.69 s).
- S9e: nothing is merged when its cause is refuted twice; N6 then holds the test.
- N1: the `connectors-sdk` test `release_build_has_no_debug_assertions`.
- N1: `execute_test` takes its limit as a parameter.

Not claimed by a change, among the 81. Each test below keeps its time in the sum after:

- No change is named, 5 tests, 49.4 s (10.2 + 7.4 + 16.5 + 8.3 + 7.0): N4's test (10.2 s); S7b's
  `embedded_text_where_the_engine_refuses_it_is_refused` (7.4 s, which the plan had at 2.0 s
  after); `adversary_a_store_replaced_under_the_owner_ends_the_session` (16.5 s, the row "no
  story"); `a_rate_limited_read_waits_once_for_a_delay_that_fits_and_names_any_other` (8.3 s,
  inside S8a's row: Retry-After is whole seconds);
  `a_queued_read_another_session_cancels_runs_unsent_and_the_running_read_succeeds` (7.0 s, inside
  S8c's row).
- A change with no estimated saving, 6 tests: the jail tests of S9b, 50.7 s before and 50.6 s
  after.

Claimed by a change whose estimate is not confirmed, 12 tests. Their "after" in the sum is the
estimate, not their time before:

- An estimate carried over from another machine, 11 tests of S9d: the 76.0 s after are each
  test's duration on the forge times the share of the two costs in that test on the build host.
  That the forge has the same shares is a reading. S9d's bound is 92.0 s for that reason.
- A cause not confirmed, 1 test of S9e: the 0.6 s after hold only when the whole wait of the test
  is the hold that `TCP_NODELAY` removes. When the cause is refuted twice the test keeps its
  7.0 s.

The sum after for the 81, computed:

- the table's column: 361.2 s for the sixteen rows with a figure;
- with S7b's test at its time before: 361.2 + 7.4 = 368.6 s;
- against the first revision's 329.7 s: 329.7 − 34.0 (S5 with seven tests) + 28.0 (S5 with six) +
  16.5 (the test that left S5) − 24.5 (S8c before its revision 11) + 32.2 (S8c now) = 347.9 s
  before the re-plan of S9a; 347.9 − 55.9 (S9a) + 76.0 (S9d) + 0.6 (S9e) = 368.6 s;
- of the 368.6 s, 49.4 s are the five tests with no change and 50.6 s the six with no estimated
  saving: 368.6 − 49.4 − 50.6 = 268.6 s rest on an estimated change;
- if S9e's cause is refuted twice: 368.6 + (7.0 − 0.6) = 375.0 s.

Outside the 81 and also unclaimed: 26 provider tests that lower a changed definition, about
80.5 s, and 9 catalog-crate tests of 1 s to under 5 s, about 22.6 s.

The stories do not reach the budget. Of the 2,474.8 s summed they cover 1,176.8 s (the 81 tests
and the 36 further tests of S7a, 59.0 s) and leave 373.6 s of it (368.6 s for the 81 and 5.0 s for
the 36); the sum after all of them is 2,474.8 − 1,176.8 + 373.6 = 1,671.6 s, 417.9 s of wall time
at four tests at a time, against 120 s. Before the re-plan of S9a these figures were 352.9 s,
1,650.9 s and 412.7 s. The rest, 1,298.0 s, is in tests the stories do not cover:
about 755 s in the band of 1 s to under 5 s and 543 s under 1 s. S9d changes product code that
every test with a persist runs; what it takes from the 1,298.0 s is not measured and not in these
sums. N5 reads it from the gate job of S9d's pull request (its question 5).

N5 owns that gap. It asks what the uncovered tests pay per test, which of those costs a fixture
built once removes, and whether changes to tests can bring the sum to 480 s; it names the draft
stories that follow, or the answers the operator chooses from. N5 also owns the comparison of the
two forge runs. Acceptance 1 of this story is not met by the other stories of the set: beyond them
it needs N5's answer and the stories drafted from it. The first plan's candidate, the store of S9a
for every test that builds one, is measured and closed: 33 to 38 ms per store. One candidate that
is not measured: the share of S9d's two costs in the tests under 5 s. N5's question 5 reads it
and has a `depends_on` edge to S9d.

This story's acceptance reads the forge's test step, which S9d and S9e change. It records no
edge to them: its own change waits until acceptance 1 holds, and that needs every story of the
set and N5's answer. The edge to N5 orders it behind all of them.

Two older drafts overlap the set.

- story:store-tests-end-within-60s-under-load names eight tests of S9d (S9a's until 2026-11-25),
  the stress test of S9c and one test of S2. The stories of this set own the change for the tests
  they name. The older story keeps its acceptance (60 s at host load) and is measured after S9d
  lands. It carries one sentence that says so and a `depends_on` edge to S9d.
- story:test-binary-per-package-rest renames the test binaries that the acceptance commands of
  the set name. Whichever of the two lands later, N1 or that story, rewrites the `--test <binary>`
  commands and the nextest ids in the stories of this set that are still open at that time. N1
  lists the eight commands.

The gate fails when the test step runs over `TEST_STEP_BUDGET` only after the step is under
the budget: before that it makes every pull request red. No child story carries that change. This
story carries it itself, as its last change, in `test_step` of
`crates/connectors-build/src/gate.rs` (acceptance 3).

## Acceptance

- The forge's "repository gate" job of a pull request prints `gate: test step ran <t> s (budget
  120 s)` with `<t>` at most 120.
- No test in that run is 60 s or longer, and the gate's list of the ten slowest starts under 60 s.
  This holds today: the slowest test of the first run took 47.2 s (row 1 of the test plan), and
  nextest ends a test at 60 s (`.config/nextest.toml` line 5: `slow-timeout` with a period of 30 s
  and `terminate-after = 2`).
- A gate run whose test step takes longer than 120 s exits non-zero and names the time. A unit
  test of `connectors-build` holds this with an injected duration. This story carries the change
  itself, after the first acceptance holds.

## Limits

- A larger runner, more shards and a higher timeout are not this story's fix (folder rule).
- The 223 skipped tests and the lane `slow` are not measured here.
- The time of the test step on the build server was not measured for this story.
