---
format: aep.planning-md/3
id: story:where-do-the-1298-s-of-the-tests-under-5-s-go
kind: story
status: draft
title: Where do the 1,298 s of the tests under 5 s go?
summary: 'A question, not a change: what the uncovered tests pay per test, and whether the 120 s budget is reachable by changes to tests'
relations:
- decomposes: story:the-gates-test-step-ends-within-120-s-on-the-forge
- informed_by: test-plan:where-the-gates-test-step-spends-its-time
- serves: vision:independent-contract-adapters
- depends_on: story:the-entity-runtime-registry-and-the-definition-bundle-are-built-once-per-process
revision: 6
---
## Defect

The stories that decompose story:the-gates-test-step-ends-within-120-s-on-the-forge and change
tests or code do not reach its budget. By the parent's arithmetic the test step sums to 1,643.2 s after all of
them, 410.8 s of wall time at four tests at a time, against 120 s. About 1,298 s of that sum are in
tests no story covers: about 755 s in the band of 1 s to under 5 s and 543.1 s in the 3,831 tests
under 1 s (test-plan:where-the-gates-test-step-spends-its-time, the table of bands and section 3).
The scope critic of round 2 (review-result:test-step-stories-scope-round-2, finding 1) found that
no story, question or follow-up owns that gap.

Since the re-plan of 2026-11-25 the parent's arithmetic gives 1,671.6 s, 417.9 s of wall time at
four tests at a time. Before it the parent gave 1,650.9 s; the 1,643.2 s above predate the
eleventh revision of story:seven-cli-tests-shorten-their-waits-or-a-production-wait. The 1,298 s
are unchanged.

This story is a question. It changes no product code and no test. The store has no kind for a
spike, so it is a story with one acceptance: a written answer.

What the test plan has, and what it says it has not:

- Per binary, under 1 s: `connectors-host` 659 tests 161.7 s, `connectors-catalog-provider::main`
  1,038 tests 119.5 s, `connectors::*` 450 tests 95.3 s, `connectors-catalog::main` 413 tests
  62.0 s, `connectors-sql`, `connectors-kubernetes` and `connectors-build` 577 tests 58.0 s.
- The `connectors::*` binaries hold 635 tests and 1,026.6 s over all bands.
- One candidate, inferred and not measured: the template store and the jail of
  story:eighteen-tests-take-a-template-store-or-jail-built-once-per-run for every test that builds
  one. The plan's own estimate with the fixture cost halved is about 1,155 s summed, 289 s of wall
  time: still over 120 s.
- That candidate was measured on 2026-11-25 and is closed: one `Metadata::initialize` costs 46 to
  67 ms on the build host and a template saves 33 to 38 ms per store (the section "Measured
  (2026-11-25): not built as written" of that story, which is archived). The measurement named
  another cost that every test with a persist pays, 13.67 ms per persist on the build host:
  story:the-entity-runtime-registry-and-the-definition-bundle-are-built-once-per-process removes
  it. Its share in the tests under 5 s is not measured; question 5 below reads it.
- Not measured: the cost of `Container::new`, of one run of the CLI and of `definition::lower`,
  each taken apart; no source of a test under 1 s was read. `Metadata::initialize` is measured
  on the build host (the line above) and not on the forge.

A second open point of the parent has no owner either (the same report, finding 5): the forge's
test step took 621.8 s on pull request 291 and 842.2 s on pull request 292, with 62 more tests. The
durations of the two runs were not compared.

## Change

None in product code.

Questions:

1. What do the tests under 5 s that no story claims pay per test? For each of the four binaries
   with the largest sums (`connectors-host`, `connectors-catalog-provider::main`, the
   `connectors::*` binaries, `connectors-catalog::main`): the time of the fixture steps taken apart
   (`Metadata::initialize`, `Container::new`, one run of the CLI, `definition::lower`), measured in
   one run of five tests at the binary's median, with the time of each step printed.
2. Which of those costs is an expensive product made per test, in the sense of the folder rule
   "Tests stay fast", and which accepted form removes it: a committed generated file with a digest
   of its sources, or one fixture built once per test run and read by every test.
3. With the costs of question 1 removed where question 2 names a form: what is the sum of the test
   step and its wall time at four tests at a time, computed from the durations of one forge run?
   If that is still over 120 s: what is left, per binary, and is the rest reachable by changes to
   tests, or does it need another answer (fewer tests in the gate's step, more tests at a time on
   the forge's runner, a split of the step)? Each such answer is named with its cost, not chosen
   here: the choice is the operator's.
   The run is the baseline run of the test plan: job 138270416028 of pull request 291. The
   computation is arithmetic on that run's durations: the tests the stories of the set cover are
   taken at the stories' estimates, as the parent sums them (1,671.6 s since the re-plan of 2026-11-25), and the costs of
   question 1 are taken off that sum. It reads no run made after a story of the set landed, so it
   needs no story of the set to have landed. Question 5 is the one exception.
   "More tests at a time on the forge's runner" and "a split of the step" are listed with their
   cost only. The folder rule "Tests stay fast" and the Limits of the parent exclude a larger
   runner and more shards as the fix, unless the operator lifts the rule. If the answer ends with
   only such options, acceptance 1 of the parent stays open, and the parent owns it.
4. Why did the test step take 842.2 s on pull request 292 against 621.8 s on pull request 291? The
   durations of the two jobs are compared per test: the 62 tests that are new, and the tests whose
   time changed by more than 1 s.
5. What does story:the-entity-runtime-registry-and-the-definition-bundle-are-built-once-per-process
   take from the tests under 5 s? That story changes product code that every test with a persist
   runs, and claims only eleven tests of 5 s and more. Read from two forge jobs: the "repository
   gate" job of that story's pull request and the job the pull request names as its base. The
   set of tests is fixed by the base job: the tests that took under 5 s there and that also
   ran in the pull request's job. A test that ran in only one of the two jobs is not in the
   set; the answer lists those tests by name. For each of the four binaries of question 1 and
   for the whole test step: the sum of the durations of the set in both jobs, and the
   difference. Each figure is one job. The spread is read the same way from two gate jobs of
   main that ran the test step with no change outside the planning store between their
   commits, and a difference inside that spread is reported as not shown. No job is run again
   to make such a pair. When no pair exists, the answer says so: the spread is not known and
   the difference is reported without a verdict.
   What the difference is: what that story took from the durations of these tests. It is not
   the share of the two costs in them, which only timers in the tests could give.
   This is the one question that reads a run made after a story of the set landed. This story
   has a `depends_on` edge to that story for it. Questions 1 to 4 do not wait for it; the sum of
   question 3 is corrected by the figure of question 5 once it exists.

## Acceptance

1. A written answer to each of the five questions is added to this story under a heading
   `## Answer`. Each figure in it names its source: the job of the forge run, or the command of the
   local run and its output. The answers to questions 1 to 4 may be written before the registry
   story has merged; the story is done when all five are written.
2. The answer names, for each cost of question 2, the draft story that then claims it (created as a
   draft that decomposes the parent), or says that no change removes it and why.
3. The answer to question 3 ends in one of two sentences: the sum after the named stories is at
   most 480 s (120 s of wall time at four tests at a time), with the arithmetic; or it is not, with
   the remaining sum per binary and the list of answers the operator chooses from.

## Scope

No file of the repository is written. Recorded scope: none. The files read are the test sources of
the four binaries and the logs of the forge jobs the questions name.

## Not in this story

- Any change of a test, a fixture or a production wait.
- The choice between the answers of question 3.
- The gate's failure on a test step over 120 s: it stays with the parent, after the step is under
  the budget.
- The runs of questions 1 and 4 follow the folder rules: no run of the whole workspace on the
  shared machine, each test bound to 60 s by the runner, a script that only measures starts without
  the build pool's prefix.
