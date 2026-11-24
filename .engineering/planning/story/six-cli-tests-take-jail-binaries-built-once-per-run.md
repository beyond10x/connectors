---
format: aep.planning-md/3
id: story:six-cli-tests-take-jail-binaries-built-once-per-run
kind: story
status: draft
title: 'Six CLI tests take jail binaries built once per run: 50.7 s, estimated unchanged at 50.6 s until the copy is measured'
summary: Each mounted-container test copies about 133 MB of binaries into its jail and hashes one; a link from a product built once per run replaces the copy
relations:
- decomposes: story:the-gates-test-step-ends-within-120-s-on-the-forge
- informed_by: test-plan:where-the-gates-test-step-spends-its-time
- serves: vision:independent-contract-adapters
- depends_on: story:eighteen-tests-take-a-template-store-or-jail-built-once-per-run
scope:
- confidence: cited
  path: apps/connectors/tests/support/mounted.rs
- confidence: inferred
  path: crates/connectors-build/src/test_fixtures.rs
- confidence: cited
  path: crates/connectors-custody/src/mounted/fixture.rs
revision: 3
---
## Defect

Every CLI test that uses the mounted container copies two binaries into its jail and takes the
sha256 of one of them. The plan gives the two binaries as about 133 MB. Six such tests are among
the 81 tests of 5 s or more. Together they take 50.7 s of the test step. This is the mistake the
folder rule "Tests stay fast" names: an expensive product made per test.

Source of the durations: the ones nextest printed in the forge's gate job 138270416028 of pull
request 291, 2026-11-24, as listed in test-plan:where-the-gates-test-step-spends-its-time (six rows
of group K). The plan read the sources at 0.77.2. The function names below were read in the tree of
0.78.1. All six are under `apps/connectors/tests/`. The last figure of each row is the plan's
estimate after.

- 12.5 s, binary `connectors::setup_migrate_records`, `setup_migrate_records.rs`,
  `migrated_entries_keep_their_connections_under_mounted_custody`. After: 12.5 s.
- 12.5 s, binary `connectors::connector_instance_selectors`, `connector_instance_selectors.rs`,
  `both_selectors_reach_one_instance_and_only_the_alias_is_deprecated`: 28 commands for 2
  selectors. After: 12.5 s.
- 8.2 s, binary `connectors::renamed_commands_adversary`, `renamed_commands_adversary.rs`,
  `reconnect_and_repair_answer_alike_on_every_argument_combination`. After: 8.2 s.
- 6.0 s, binary `connectors::owner_slow_start`, `owner_slow_start.rs`,
  `reads_that_name_a_longer_deadline_wait_for_their_turn`. After: 5.9 s.
- 5.8 s, binary `connectors::owner_recovers_after_failed_acquisition`,
  `owner_recovers_after_failed_acquisition.rs`,
  `a_connect_that_failed_publication_leaves_the_owner_admitting_work`. After: 5.8 s.
- 5.7 s, binary `connectors::security_mcp_deadline`, `security_mcp_deadline.rs`,
  `security_requests_past_their_deadline_each_get_one_reply_on_their_own_id`. After: 5.7 s.

Sums, computed from the rows: before 50.7 s, after 50.6 s. The plan keeps the seconds of these six
almost unchanged: it names no seam for the three CLI process loops and did not measure
`Container::new` or one CLI run. So the plan claims no saving for these six. What this story
changes is the copy per test; what that saves is its acceptance 3.

Read at 0.78.1: `Container::new` in `apps/connectors/tests/support/mounted.rs` (lines 264 to 292)
calls `jail.install` for `CARGO_BIN_EXE_connectors` and for the running test binary, then reads the
installed test binary to take its sha256. `Jail::install` is in
`crates/connectors-custody/src/mounted/fixture.rs` (line 119). Its body was not read; the plan says
it copies.

## Change

The second accepted form of the folder rule: one fixture built once per test run, kept on disk
under `target/` and keyed by the digest of its sources. An in-process global is not this form.

1. The setup step of
   story:eighteen-tests-take-a-template-store-or-jail-built-once-per-run (`connectors-build
   test-fixtures`) gets a third product: the CLI binary, kept in
   `target/test-fixtures/jail-binaries/<digest>/connectors`, keyed by the sha256 of the binary. It
   prints `test-fixtures: jail-binaries built in <ms> ms, digest <hex>` or `… reused …` and hands
   the directory to the tests as `CONNECTORS_TEST_JAIL_BINARIES`.
2. `Jail::install` links the file into the jail (a hard link) when the variable is set and the
   file system allows it, and copies as today when not. It reports which of the two it did.
3. The sha256 of the test binary is taken once per run per test binary and kept beside the
   product, in place of once per test. Inferred: the key is the binary's path, length and
   modification time, because a key that reads the whole file is the cost this story removes.
4. `Container::new` uses both. A run without the setup step behaves as today.

Bound rule: bound = estimate × 1.5, rounded up to a whole second.

## Acceptance

1. Built once: in the test step of the forge's "repository gate" job of this story's pull request,
   the line `test-fixtures: jail-binaries …` appears once. A second run of the setup step with an
   unchanged binary prints `reused`.
2. Linked, not copied: a test of the support code asserts that after `Container::new` the file
   `/bin-test/connectors` of the jail and the file of the product are one inode. Where the file
   system refuses a link the test asserts the fallback copy and the report of it.
3. The measurement: one test prints the time of `Container::new` in milliseconds. The pull
   request's description records it from the gate job of the base and of the pull request, and
   for each of the six tests the duration before (above) and after.
4. The six tests pass, and their durations in the test step of that job sum to at most 76 s.
   Before: 50.7 s. Estimate: 50.6 s. Bound: 1.5 × 50.6 = 75.9, rounded up to 76 s. The bound by the
   rule is above the before, because the plan estimates no saving here; it guards against a slower
   result only.
5. Kept runs: the six tests keep their commands and loops (28 commands for 2 selectors, every
   argument combination, three queued reads). The pull request changes no loop count in them.

## Scope

- `apps/connectors/tests/support/mounted.rs`: `Container::new` (lines 264 to 292).
  story:eleven-connection-lease-tests-advance-a-clock-instead-of-sleeping touches another region
  of this file: the knobs of the adapter fixture (lines 180 to 230).
- `crates/connectors-custody/src/mounted/fixture.rs`: `Jail::install`.
- `crates/connectors-build/src/test_fixtures.rs`: the third product. The file is created by
  story:eighteen-tests-take-a-template-store-or-jail-built-once-per-run; this story `depends_on`
  it. `.config/nextest.toml` is not edited again.

The six test files are not edited. Other stories of this set edit other tests in two of them:
`owner_slow_start.rs`
(story:seven-cli-tests-of-a-slow-owner-start-no-longer-wait-out-the-10-s-connect-wait) and
`security_mcp_deadline.rs` (story:seven-mcp-tests-use-a-4-s-request-budget-and-shorter-holds).

## Not in this story

- A seam for the three CLI process loops (12.5 s, 12.5 s, 8.2 s). The plan names none.
- The other tests that use the mounted container: among them the 14 tests of
  `connectors::adversary_local_mcp_server` and the 15 of `connectors::local_mcp_server` in the
  band of 1 s to under 5 s. They get the link too; their seconds are not counted here.
- The gate failing when its test step runs over 120 s:
  story:the-gates-test-step-ends-within-120-s-on-the-forge.
- Not established: whether the jail allows a hard link of a binary from `target/` (the plan did
  not check; the test's directory and `target/` can be on two file systems); what `Jail::install`
  does today beyond what the plan says; the cost of `Container::new` and of one CLI run; whether a
  key of path, length and modification time is accepted as "the digest of its sources".
