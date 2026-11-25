---
format: aep.planning-md/3
id: story:the-lane-slow-runs-the-ignored-timing-family
kind: story
status: implemented
title: The lane slow runs the ignored timing family
summary: The lane slow runs no test today; one naming rule in classify() and four steps make it run every slow_lane_ test, for changes under crates, apps and adapters; the eight old timing entries are deferred
relations:
- decomposes: story:the-gates-test-step-ends-within-120-s-on-the-forge
- informed_by: test-plan:where-the-gates-test-step-spends-its-time
- serves: vision:independent-contract-adapters
scope:
- confidence: cited
  path: .github/workflows/slow.yml
- confidence: cited
  path: crates/connectors-build/src/ignored.rs
- confidence: cited
  path: crates/connectors-sdk/src/lib.rs
- confidence: cited
  path: docs/development.md
revision: 16
transitions:
- {from: "draft", to: "proposed", at: "2026-11-25T02:53:29Z", actor: "human:operator", revision: 8}
- {from: "proposed", to: "active", at: "2026-11-25T02:53:29Z", actor: "human:operator", revision: 9}
- {from: "active", to: "implemented", at: "2026-11-25T05:49:06Z", actor: "human:operator", revision: 16, decided_on: {"recorded":{"test_result":1,"review_outcome":3}}}
---
## Defect

Eight stories of this set move a test with a real wait out of the gate and say it still passes "in
the lane `slow`". The lane cannot run a test today.

Read at 0.78.1; the lines below are those of 0.78.2:

- `.github/workflows/slow.yml` has one job with one check
  (`cargo run --locked --offline -p connectors-build -- metadata-conformance emissions --check`). It
  starts only for `ess/**`, `Cargo.lock`, `crates/connectors-spec/toolchain.json`, three paths under
  `crates/connectors-build/` and the workflow itself. It runs no test. A pull request that changes
  `crates/connectors-host/`, `apps/` or `adapters/` does not start it.
- `crates/connectors-build/src/ignored.rs`: `classify()` (line 144) is a table of exact
  `(package, target, test name)` entries. Its comment says: "New tests, even in an existing module,
  are unknown until someone reviews their authority and needs." `evaluate()` (line 587) executes
  nothing while one ignored test is unknown, and `Report::accepted` (line 112) refuses a report
  with an unknown test. So every new `#[ignore]` test needs an edit of this table, or the gate's
  ignored step (`.github/workflows/rust-gate.yml` lines 109 to 111,
  `connectors-build ignored --required --only-prerequisite provider`) fails.
- `Family::Timing` exists (line 46). Nine entries of the table carry it: seven of
  `local::registry` in `connectors-host` (lines 169 to 176), one of `cold_owner_start` in
  `connectors` (line 513) and one of `connectors-catalog-provider::main` that needs the Secret
  Service and the CLI (line 249). No job selects the family.
- `discover()` (line 568) lists tests with `--ignored --list --format terse`. That output holds the
  test name and not the ignore reason. So a rule can use the name. A rule on the ignore reason would
  need a scan of the sources.
- `execute_test()` (line 794) runs one test per process (`--exact`, `--test-threads=1`) and waits up
  to 600 s for it (line 842). The 600 s are a literal in its body: no function takes the limit as a
  parameter. A test that is still running at the limit is killed with its process group; its log
  then holds no result line, `execute_test` returns an error, and `evaluate()` gives the outcome
  `execution-error` and counts it in `failed` (lines 668 to 672).

Read at 0.78.2 for the second round of critics:

- `slow.yml` has no step that permits unprivileged user namespaces. The gate job has one
  (`rust-gate.yml` lines 100 to 105) and the journeys job has one (lines 155 to 159). The jail of
  `crates/connectors-custody/src/mounted/fixture.rs` needs them: its header says so (lines 11 to
  14), and its child calls `unshare(CLONE_NEWUSER | CLONE_NEWNS)` (line 249).
- `slow.yml` installs no cargo-nextest. The gate job installs 0.9.143 (`rust-gate.yml` lines 87 to
  96).
- The gate job starts `connectors-build` with `cargo run --locked --bin connectors-build`
  (`rust-gate.yml` lines 108 and 111) since 0.78.2: with `-p` cargo resolves another feature set
  than the build of the workspace and compiles crates a second time. `slow.yml` kept `-p` because
  its job builds one program and no workspace. A step that runs `ignored` builds the test binaries
  of the workspace.

## Change

1. One rule in `classify()`, written once: an ignored test whose function name (the part after the
   last `::`) starts with `slow_lane_` belongs to `Family::Timing` and has no prerequisite. The
   exact entries of the table are matched first. A story that moves a test out of the gate adds the
   test under such a name with `#[ignore]`. It edits neither `ignored.rs` nor `slow.yml`.
2. `slow.yml` gets four steps, in this order, after "Fetch locked dependencies":
   - "Permit unprivileged user namespaces", copied from the gate job (`rust-gate.yml` lines 100
     to 105, the form with `sudo`: both jobs run on `ubuntu-24.04` without a container). Lane tests
     that need it, because they start the CLI in the jail through `Container::new`
     (`apps/connectors/tests/support/mounted.rs` line 264):
     `connectors::connection_lease::slow_lane_concurrent_revalidations_both_answer_ready_within_the_admission_bound`
     and
     `connectors::connection_lease_adversary::slow_lane_adversary_a_hanging_early_recollection_does_not_cost_a_read_its_current_evidence`
     (story:eleven-connection-lease-tests-advance-a-clock-instead-of-sleeping),
     `connectors::owner_slow_start::slow_lane_a_read_waits_for_a_slow_owner_start_within_its_deadline`
     (story:seven-cli-tests-of-a-slow-owner-start-no-longer-wait-out-the-10-s-connect-wait),
     `connectors::security_mcp_deadline::slow_lane_security_an_on_time_reply_written_across_its_deadline_keeps_the_session`
     (story:seven-mcp-tests-use-a-4-s-request-budget-and-shorter-holds). Of the entries the
     table holds today
     `connectors::cold_owner_start::a_cold_describe_and_read_of_a_large_store_answer_within_60_seconds`
     needs it too; the lane does not select it (point 7).
     Read for this: each of the five test files builds its container with `Container::new`
     (`connection_lease.rs` line 27, `connection_lease_adversary.rs` line 23, `owner_slow_start.rs`
     line 48, `security_mcp_deadline.rs` line 363, `cold_owner_start.rs` line 43). The other lane
     tests the stories name build no jail (inferred from their files' imports; not followed call
     by call).
   - "Install the pinned cargo-nextest release", copied from the gate job (`rust-gate.yml` lines
     87 to 96, version 0.9.143 with its checksum).
   - `cargo run --locked --bin connectors-build -- ignored --family timing --required --only-prerequisite provider`.
     It runs every test of the rule and no entry of the table: the eight entries that were `Timing`
     without a prerequisite are `Deferred` (point 7), and the ninth needs the Secret Service and
     the CLI, which the lane does not supply. It uses `--bin` as the gate job does, for the reason
     read above.
   - `cargo nextest run --release --locked -p connectors-sdk --no-tests fail release_build_`. It
     runs, in a build without `debug_assertions`, the tests of `connectors-sdk` whose name holds
     `release_build_`. With `--no-tests fail` cargo-nextest exits with code 4 when the filter
     matches no test (read in `cargo nextest run --help` of 0.9.143, the pinned version: the values
     are `auto`, `pass`, `warn` and `fail`, and `auto` defaults to fail). The flag is written out so
     that the step does not rest on a default. `cargo test` has no such flag: it prints
     `running 0 tests` and exits 0.
3. One test in `crates/connectors-sdk/src/lib.rs`, compiled only without `debug_assertions`:
   `release_build_has_no_debug_assertions`. It asserts nothing but that it runs. The reason: the
   step of point 2 fails on zero tests, and
   story:one-debug-only-override-helper-is-forwarded-to-the-owner-and-off-in-a-release-build adds
   its release test only after this story. With this test the step runs one test from this
   story's merge on, and zero tests means that `--release` resolved to a profile with
   `debug_assertions`.
4. Both `paths:` lists of `slow.yml` (pull request and push to `main`) keep their seven entries and
   gain `crates/**`, `apps/**` and `adapters/**`. `adapters/**` is one more than the revision named:
   three stories of the set add a lane test under `adapters/`. No story of the set adds a path.
5. `execute_test()` takes the limit as a parameter in place of the literal 600 s. `run()` (line
   1289) passes 60 s for a test of `Family::Timing` and 600 s for every other family; the test
   helper `execute_one()` (line 866) passes 600 s. A test over its limit is ended and counts as
   failed, as read above (folder rule "Tests stay fast": no single test runs longer than 60 s).
6. The section "Ignored tests" of `docs/development.md` (line 307) states the rule, the two
   commands and the paths. Its paragraph on the `deferred` family says today that each deferred
   test's ignore reason names its limitation or follow-up story, and names
   story:main-failing-ci-journeys for eleven journeys. It gains one sentence: the eight timing
   measurements of point 7 are `deferred` too, held by
   story:eight-timing-tests-have-no-job-and-none-ends-within-60-s, and their ignore reasons say
   "run explicitly". The page does not say that the lane runs `timing` entries of the table.
7. The eight `Timing` entries without a prerequisite that the table holds today move to
   `Deferred`. A comment at the entries has one line per test: the test's function name, the id
   story:eight-timing-tests-have-no-job-and-none-ends-within-60-s and the time or the failure of
   "Measured" below. No other line of `ignored.rs` holds that id. The eight get match arms of
   their own under that comment; they do not join the `Deferred` arm of `connectors-host` whose
   comment names story:main-failing-ci-journeys. Seven are tests of
   `connectors-host` (`local::registry::tests::read_invoke_metadata_time` and six of
   `local::registry::store_cost_tests`); one is
   `connectors::cold_owner_start::a_cold_describe_and_read_of_a_large_store_answer_within_60_seconds`.

The lane then starts for nearly every pull request that changes code. How long it runs on the
forge was not measured.

## Measured (2026-11-25, revision 10)

Revision 9 let the lane run the eight entries of point 7 and said in acceptance 5: "if none passes
within 60 s, this story goes back to revision." None passed.

- One run of the lane's first command with the implementation of revision 9, on the build host of
  the shared pool, before any push: exit 1. The build took 297.2 s and the tests 360.4 s. Six of
  the eight were ended at the limit of 60 s. Two failed at once:
  `first_owner_open_of_a_grown_store` and `owner_start_of_a_grown_store` unwrap the variables
  `CONNECTORS_STORE_COST_STORES` and `CONNECTORS_STORE_COST_EVENTS`, which no job sets.
- The six, each run alone with a limit of 600 s, on a workstation with a load average of 14 to 27:
  `two_revalidations_of_a_grown_store` 133.1 s, `batch_cost_by_subject` 178.7 s,
  `read_invoke_cost_by_store_size` 186.6 s, `fresh_open_passes_of_a_blob_heavy_store` 266.9 s,
  `read_invoke_metadata_time` 468.8 s, and
  `a_cold_describe_and_read_of_a_large_store_answer_within_60_seconds` not ended at 600 s.
- Seven of the eight print costs and assert no time. The eighth asserts 60 s.

No job ran these eight before, and none runs them after this story.

## Decision beyond the plan

The plan asks for a lane that runs the tests the stories move. Revision 9 also ran the eight
`Timing` entries of the table, because the family is selected as a whole. With the figures above
that lane takes at least 1,234 s for five of them and up to 600 s more for the sixth, at a limit
of 600 s per test, on every pull request that changes code, and the folder rule lets no single
test run over 60 s. The operator's decision of 2026-11-25 (D24,
option A): the eight move to `Deferred`, the lane keeps 60 s per test and runs the `slow_lane_`
tests only, and one draft story holds the eight with their times
(story:eight-timing-tests-have-no-job-and-none-ends-within-60-s).

`--required` refuses an empty selection (`Report::accepted`, lines 112 to 117), and this story
adds no `slow_lane_` test. So this story and
story:the-100-run-capacity-stress-keeps-10-runs-in-the-gate, which adds the first one and edits one
test file, go in one pull request. No `depends_on` runs from this story to that one: that story
`depends_on` this one, and the edge back would close a cycle. The coupling is the one pull request.

From that merge on the lane fails when no `slow_lane_` test exists. Removing the last one, or
taking its `#[ignore]` away, is a change to the lane's first step as well.

## Acceptance

1. A unit test of `ignored.rs` holds the rule both ways. `classify()` gives a test named
   `tests::slow_lane_example` the family `Timing` and no prerequisite; under the options of the
   gate's ignored step (`--required --only-prerequisite provider`, family `disposable`) its outcome
   is `not-selected` and the report counts no unknown test. A test named `tests::example` that is
   in no entry is unknown, and nothing executes (today's test
   `unknown_inventory_refuses_before_any_execution`, line 1813, keeps passing).
2. A pull request that changes one file under `crates/`, one under `apps/` or one under
   `adapters/` starts the workflow `slow`. A unit test in the tests module of
   `crates/connectors-build/src/ignored.rs` reads `slow.yml` and holds the three paths in both
   lists, the step that permits user namespaces before the two commands, `--bin connectors-build`
   in the first command and `--release` with `--no-tests fail` in the second. It stands beside the
   two modules of that file that read `rust-gate.yml` the same way (`journeys_lane` and
   `adversary`). It is no module of `crates/connectors-build/tests/main.rs`.
3. In the lane job of the one pull request that holds this story and
   story:the-100-run-capacity-stress-keeps-10-runs-in-the-gate, the first command above exits 0.
   Its report is printed as JSON to standard output. In it `result.unknown` is 0, `result.failed`
   is 0, `result.selected` is 1 and `result.executed` is 1, and the one outcome in
   `result.outcomes` whose `selected` is true has the `name`
   `connectors::local_cli::slow_lane_adversary2_a_same_build_owner_at_capacity_is_never_another_build_over_100_runs`
   (the report's form: package, target and test joined by `::`) and the `status` `passed`. That
   test is the one the other story adds. A later `slow_lane_` test is in the report from the pull
   request that adds it; that it is listed as `passed` is the acceptance of the story that adds
   it.
4. A test over its limit is ended and reported as failed. A unit test of `ignored.rs` calls
   `execute_test()` with a limit of 1 s on a fixture test that sleeps 30 s: the call returns an
   error within 5 s and no process of the fixture is left running. A second assertion holds the
   limits `run()` passes: 60 s for `Family::Timing`, 600 s for the other families. No test of the
   gate sleeps 60 s for this.
5. The eight entries of point 7 are `Deferred` and the lane does not select them. A unit test of
   `ignored.rs` names the eight and holds for each that `classify()` gives the family `Deferred`
   and no prerequisite, and that `evaluate()` under the lane's options (family `timing`,
   `--required --only-prerequisite provider`) gives it `selected` false and the status
   `not-selected`. The report of acceptance 3 shows the same for each of the eight.
6. In the lane job of that pull request the second command above exits 0 and its summary names
   1 test run, `release_build_has_no_debug_assertions`.
7. The comment of point 7 is there:
   `grep -c 'story:eight-timing-tests-have-no-job-and-none-ends-within-60-s' crates/connectors-build/src/ignored.rs`
   prints 8. The count shows that the eight lines exist. That each line holds the figure of
   "Measured" is read in the pull request's diff.

The lane's step budget: the folder rule gives the test step of the gate 120 s and names no budget
for the lane `slow`. The lane has none yet. The job has `timeout-minutes: 60` (`slow.yml`).

## Scope

- `.github/workflows/slow.yml`: the four steps and both `paths:` lists.
- `crates/connectors-build/src/ignored.rs`: `classify()`, `execute_test()`, `execute_one()`,
  `run()` and the tests.
- `crates/connectors-sdk/src/lib.rs`: the one test of point 3.
  story:one-debug-only-override-helper-is-forwarded-to-the-owner-and-off-in-a-release-build writes
  this file after this story and `depends_on` it.
- `docs/development.md`: the section "Ignored tests".

No other story of this set writes `slow.yml`, `ignored.rs` or `docs/development.md`.

story:test-binary-per-package-rest (a draft outside this set) builds the integration tests of each
remaining package into one binary. The rule of this story reads the function name only, so it
holds before and after that story. The commands and the nextest ids the stories of this set cite
do not: they name a test binary by its file. Whichever of the two lands later, this story or that
one, rewrites the `--test <binary>` commands and the nextest ids in the stories of this set that
are still open at that time. The eight commands are:

- `-p connectors --test connection_lease_adversary` and `-p connectors --test connection_lease`
  (story:eleven-connection-lease-tests-advance-a-clock-instead-of-sleeping);
- `-p connectors --test owner_slow_start`
  (story:seven-cli-tests-of-a-slow-owner-start-no-longer-wait-out-the-10-s-connect-wait);
- `-p connectors --test local_cli` (story:the-100-run-capacity-stress-keeps-10-runs-in-the-gate);
- `-p connectors --test security_mcp_deadline`
  (story:seven-mcp-tests-use-a-4-s-request-budget-and-shorter-holds);
- `-p connectors-host --test local_foundation`
  (story:four-host-tests-no-longer-wait-out-the-30-s-lock-wait);
- `-p connectors-host --test provider_timeout_adversary` and `-p connectors-sql --test protocol`
  (story:five-tests-no-longer-wait-out-the-http-and-sql-client-timeouts).

The nextest ids of the same six stories carry the binary's name too
(`connectors::owner_slow_start::…`) and are rewritten with the commands.

## Not in this story

- The tests the eight stories move. Each story adds its own test under a `slow_lane_` name.
- A run of several suites in one process. The lane gets no third step for it.
  `adapters/catalog/tests/adversary_one_binary.rs::no_suite_sees_the_installation_another_suite_sets`
  can fail only when a module of its binary sets `CONNECTORS_INSTALLATION` in the same process.
  Both commands above run one test per process, as nextest does, and no module of that binary
  sets the variable today. story:eight-adapter-and-catalog-tests-drop-waits-that-prove-nothing
  removes the test and gives the reason.
- The Windows and macOS jobs. `slow.yml` runs on one Linux runner.
- The gate failing when its test step runs over 120 s:
  story:the-gates-test-step-ends-within-120-s-on-the-forge.
- A job for the eight entries of point 7, or their removal:
  story:eight-timing-tests-have-no-job-and-none-ends-within-60-s.
- Not established: whether
  `connectors::local_cli::slow_lane_adversary2_a_same_build_owner_at_capacity_is_never_another_build_over_100_runs`
  ends within the lane's 60 s. Its 100 rounds ran 31.9 s in a gate job of the forge and were
  killed at 60 s on a loaded workstation. If it runs over 60 s in the lane, the first command
  exits 1 with the outcome `execution-error` for it, acceptance 3 fails and this story is not
  done; the fix is then in that story (fewer rounds), not a larger limit in this one.
- Not established: how long the lane runs with the wide paths; whether the lane needs more setup
  than the step for user namespaces
  (the gate job also checks for `gzip`, `sha256sum`, `sqlite3` and `jq`, `rust-gate.yml` lines 43 to
  45);
  that `cargo nextest run --release` resolves to a profile without `debug_assertions`
  (`Cargo.toml` sets `debug-assertions = true` for `[profile.dev]` and `[profile.test]` only,
  lines 64 to 74; not confirmed by a run).

## As built (2026-11-25)

Where the implementation differs from the Change and Acceptance sections. Nothing of the timing
family was run after the runs of the section "Measured".

1. The limit of 60 s goes by the name rule, not by the family (Change 5 says "for a test of
   `Family::Timing`"). An entry has a field `slow_lane`, set only by the rule in `classify()`,
   and `time_limit(entry)` reads it: 60 s for a `slow_lane_` test, 600 s for every entry of the
   exact table. The adversary read showed why: the table's one `Timing` entry left
   (`connectors-catalog-provider::main`, `catalog_cli_read_recollects_expired_evidence_without_reentry`,
   which needs custody and the CLI) waits out 60,000 ms of evidence by design and had 600 s
   before this story. No job runs it. Tests:
   `a_slow_lane_test_gets_60_seconds_and_every_entry_of_the_table_600` and
   `the_timing_entry_that_waits_out_its_evidence_gets_longer_than_that_wait`.
2. A test ended at its limit returns "test was still running at its limit of N s and was ended
   with its process group; see log" in place of the generic message.
3. Both `paths:` lists of `slow.yml` hold 13 paths, not 10 (Change 4): `Cargo.toml`, `.cargo/**`
   and `rust-toolchain.toml` were added after the adversary read. They decide whether a release
   build has debug assertions, which the lane's second command exists to show. Test:
   `a_change_of_a_build_profile_or_of_the_toolchain_starts_the_lane`.
4. The lane has two steps the Change does not name, after the timing step, both with
   `if: always()`: "Name each lane test that did not pass" prints each selected test that did not
   pass with the end of its log, and "Keep the lane report and logs" uploads them as the artifact
   `ignored-slow-lane`. Their script and upload settings are those of the journeys job of
   `rust-gate.yml`. Without them a red lane test says "see log" about a file on the runner.
   Test: `the_lane_prints_and_keeps_the_logs_of_its_tests_and_still_checks_the_emissions`.
5. The four steps of Change 2 sit directly after "Fetch locked dependencies", so the emissions
   check is the last step of the job, and a test holds that it is still there. A red lane test
   ends the job before that check. The timing step is named "Run the ignored slow_lane_ tests,
   60 s each". The workflow file holds no comment line.
6. The comment of Change 7 is two blocks, because the eighth entry is in another arm of the table
   (`connectors`, `cold_owner_start`). Each block opens with a line that says where the times
   were measured and holds no story id. The lines of `first_owner_open_of_a_grown_store` and
   `owner_start_of_a_grown_store` say "fails at once" and name the two unset variables; they
   carry no time.
7. The unit test `the_lane_selects_a_slow_lane_test_and_no_entry_of_the_table` also reads the
   source of `classify()`: every `Timing` entry of the table has a prerequisite other than the
   provider. The same test holds that the lane's selection is empty without a `slow_lane_` test
   and that `--required` then refuses it.
8. The unit test of acceptance 2 also holds that the two setup steps equal the gate job's `run`
   text and that the guide shows both commands.
9. Acceptance 4, "no process of the fixture is left running", holds for the test's process group.
   A process the test started in a session of its own outlives the kill; the kill is unchanged
   from before this story. No lane test of this change starts one.
   story:a-lane-test-ended-at-its-limit-leaves-a-process-of-another-session-running holds it.

On the build host of the shared pool: `cargo nextest run --locked -p connectors-build -E
'test(ignored::)'` ran 39 tests, 39 passed; ten runs in a row of the ten tests of
`ignored::tests::slow_lane`: 10 of 10, the limit test 1.1 s. The count of acceptance 7 is 8.

Open until the lane job of the pull request: acceptance 3, acceptance 6, and the report's side of
acceptance 5.
