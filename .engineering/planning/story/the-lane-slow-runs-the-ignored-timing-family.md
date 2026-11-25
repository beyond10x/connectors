---
format: aep.planning-md/3
id: story:the-lane-slow-runs-the-ignored-timing-family
kind: story
status: draft
title: The lane slow runs the ignored timing family
summary: The lane slow runs no test today; one naming rule in classify() and one step make it run every slow_lane_ test, for changes under crates, apps and adapters
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
  path: docs/development.md
revision: 2
---
## Defect

Eight stories of this set move a test with a real wait out of the gate and say it still passes "in
the lane `slow`". The lane cannot run a test today.

Read at 0.78.1:

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
  ignored step (`.github/workflows/rust-gate.yml` line 109,
  `connectors-build ignored --required --only-prerequisite provider`) fails.
- `Family::Timing` exists (line 46). Nine entries of the table carry it: seven of
  `local::registry` in `connectors-host` (lines 169 to 176), one of `cold_owner_start` in
  `connectors` (line 513) and one of `connectors-catalog-provider::main` that needs the Secret
  Service and the CLI (line 249). No job selects the family.
- `discover()` (line 568) lists tests with `--ignored --list --format terse`. That output holds the
  test name and not the ignore reason. So a rule can use the name. A rule on the ignore reason would
  need a scan of the sources.
- `execute_test()` (line 794) runs one test per process (`--exact`, `--test-threads=1`) and waits up
  to 600 s for it (line 842).

## Change

1. One rule in `classify()`, written once: an ignored test whose function name (the part after the
   last `::`) starts with `slow_lane_` belongs to `Family::Timing` and has no prerequisite. The
   exact entries of the table are matched first. A story that moves a test out of the gate adds the
   test under such a name with `#[ignore]`. It edits neither `ignored.rs` nor `slow.yml`.
2. `slow.yml` gets two steps:
   - `cargo run --locked -p connectors-build -- ignored --family timing --required --only-prerequisite provider`.
     It runs every test of the rule, and the eight entries of the table that are `Timing` and need
     no prerequisite.
   - `cargo test --release --locked -p connectors-sdk release_build_`. It runs, in a build without
     `debug_assertions`, the tests of `connectors-sdk` whose name starts with `release_build_`. It
     runs no test until
     story:one-debug-only-override-helper-is-forwarded-to-the-owner-and-off-in-a-release-build adds
     one.
3. Both `paths:` lists of `slow.yml` (pull request and push to `main`) keep their seven entries and
   gain `crates/**`, `apps/**` and `adapters/**`. `adapters/**` is one more than the revision named:
   three stories of the set add a lane test under `adapters/`. No story of the set adds a path.
4. `execute_test()` waits 60 s in place of 600 s for a test of `Family::Timing`. A test over 60 s
   counts as failed (folder rule "Tests stay fast": no single test runs longer than 60 s).
5. The section "Ignored tests" of `docs/development.md` (line 278) states the rule, the two
   commands and the paths.

The lane then starts for nearly every pull request that changes code. How long it runs was not
measured.

## Acceptance

1. A unit test of `ignored.rs` holds the rule both ways. `classify()` gives a test named
   `tests::slow_lane_example` the family `Timing` and no prerequisite; under the options of the
   gate's ignored step (`--required --only-prerequisite provider`, family `disposable`) its outcome
   is `not-selected` and the report counts no unknown test. A test named `tests::example` that is
   in no entry is unknown, and nothing executes (today's test
   `unknown_inventory_refuses_before_any_execution`, line 1771, keeps passing).
2. A pull request that changes one file under `crates/`, one under `apps/` or one under
   `adapters/` starts the workflow `slow`. A test of `connectors-build` reads `slow.yml` and holds
   the three paths in both lists.
3. In the lane, the first command above exits 0 and its report lists each `slow_lane_` test with
   the outcome `passed`.
4. A test of `Family::Timing` that runs longer than 60 s is ended and reported as failed. A unit
   test holds this with a fixture test that sleeps.
5. The eight `Timing` entries without a prerequisite that the table holds today run in the lane for
   the first time. Each passes within 60 s in the job of this story's pull request, or this story
   moves it to `Deferred` and the pull request states its measured time.

The lane's step budget: the folder rule gives the test step of the gate 120 s and names no budget
for the lane `slow`. The lane has none yet. The job has `timeout-minutes: 60` (`slow.yml`).

## Scope

- `.github/workflows/slow.yml`: the two steps and both `paths:` lists.
- `crates/connectors-build/src/ignored.rs`: `classify()`, `execute_test()` and the tests.
- `docs/development.md`: the section "Ignored tests".

No other story of this set writes these three files.
story:test-binary-per-package-rest (a draft outside this set) changes target names in the table of
`ignored.rs`; the rule reads the function name only, so it holds before and after that story.

## Not in this story

- The tests the eight stories move. Each story adds its own test under a `slow_lane_` name.
- A run of several suites in one process.
  `adapters/catalog/tests/adversary_one_binary.rs::no_suite_sees_the_installation_another_suite_sets`
  can fail only when other suites of its binary run in the same process. Both commands above run
  one test per process, as nextest does. Whether the lane gets a third step for that test is open:
  story:eight-adapter-and-catalog-tests-drop-waits-that-prove-nothing.
- The Windows and macOS jobs. `slow.yml` runs on one Linux runner.
- The gate failing when its test step runs over 120 s:
  story:the-gates-test-step-ends-within-120-s-on-the-forge.
- Not established: how long the eight existing `Timing` entries run in the lane; how long the lane
  runs with the wide paths. Nothing was run for this draft.
