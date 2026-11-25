---
format: aep.planning-md/3
id: story:six-cli-tests-take-jail-binaries-built-once-per-run
kind: story
status: draft
title: 'Six CLI tests link their binaries into the jail instead of copying them: 50.7 s, no saving estimated'
summary: Each mounted-container test copies about 133 MB of binaries into its jail; Jail::install hard-links them and copies only where it cannot link
relations:
- decomposes: story:the-gates-test-step-ends-within-120-s-on-the-forge
- informed_by: test-plan:where-the-gates-test-step-spends-its-time
- serves: vision:independent-contract-adapters
scope:
- confidence: cited
  path: crates/connectors-custody/src/mounted/fixture.rs
- confidence: cited
  path: crates/connectors-custody/src/mounted/tests.rs
revision: 7
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
`Container::new` or one CLI run. So the plan claims no saving for these six, and this story
estimates none. What this story changes is the copy per test.

This story was drafted with a product "built once per run" by a setup step. The second round of
critics found that the product had no producer: the CLI binary is built by cargo and named at
compile time. The setup product is dropped. The id keeps the old name; an id cannot be changed.

Read at 0.78.2:

- `Container::new` in `apps/connectors/tests/support/mounted.rs` (lines 264 to 296) calls
  `jail.install` for `env!("CARGO_BIN_EXE_connectors")` (lines 279 to 282) and for the running
  test binary (line 283), then reads the installed test binary to take its sha256 (line 284).
- `Jail::install` in `crates/connectors-custody/src/mounted/fixture.rs` (lines 119 to 127) creates
  the parent directory, copies the source with `fs::copy` and sets the mode of the copy to 0700.
- The jail root is a directory `mounted-jail-…` in the temporary directory (`Jail::new`, lines 57
  to 61). In the gate the temporary directory of a test is `<checkout>/.local/tmp/gate-…`
  (`crates/connectors-build/src/gate.rs`, `cargo()` lines 24 to 46 and lines 222 to 224, repeated
  for nextest in `test_step`, lines 527 to 547). So in the gate the jail root and `target/` are
  both under the checkout. Under plain `cargo test` the jail root is in the caller's temporary
  directory.
- `Jail::install` has callers beside `Container::new`: `setup_migrate_records.rs` (lines 380 to
  385; the 12.5 s test builds its own jail), `crates/connectors-custody/src/mounted/tests.rs`
  (line 40) and tests of `adapters/catalog/tests/`. A search for the installed paths
  (`/bin-test/…`) found them run and read, and found no write into an installed file.
- The product checks an adapter executable in `crates/connectors-host/src/local/config.rs` (lines
  2429 to 2434): a regular file, owned by the user or root, not writable by group or others,
  executable, at most 512 MiB. It does not read the link count. The check that does
  (`check_private_file`, `crates/connectors-host/src/local/filesystem.rs` line 278) is for private
  files.

## Change

1. `Jail::install` links the source into the jail with a hard link and copies as today when it
   does not link. It links when the source is not writable by group or others and the link call
   succeeds. In every other case it copies and sets 0700, as today: a source on another file
   system, a file system or a host setting that refuses the link, a source with a group or other
   write bit.
   The reason for the mode rule: a link is the same file as the source, so it keeps the mode cargo
   gave it, and `install` must not change the mode of a file under `target/`. The copy's 0700
   passes the product's check of an adapter executable on every host; a linked file passes it only
   without the two write bits.
2. Every caller gets the link without an edit, `Container::new` and the jail of
   `setup_migrate_records.rs` among them. `install` keeps its signature.
3. No setup step, no product under `target/test-fixtures/`, no variable and no digest: the source
   is the file cargo built, and a link to it is always the current one.
4. The sha256 of the test binary stays as it is, once per test (`mounted.rs` line 284). A kept hash
   would have to be keyed by the content of the binary, and reading the content is the cost.

Bound rule: bound = estimate × 1.5, rounded up to a whole second.

## Acceptance

1. Linked where linking works: four unit tests in
   `crates/connectors-custody/src/mounted/tests.rs`, run by
   `cargo nextest run --locked -p connectors-custody -E 'test(/mounted::tests::install_/)'` and by
   the gate's test step.
   - `install_links_a_source_on_the_jails_file_system`: the test writes a file with mode 0755 into
     the temporary directory and installs it. The installed file and the source are one inode, and
     the source still has mode 0755.
   - `install_links_the_running_test_binary_on_one_device`: it installs the running test binary, a
     file under `target/`. When that file and the jail root are on one device and the file has no
     group or other write bit, the two are one inode. Otherwise the installed file is a copy with
     mode 0700.
   - `install_copies_a_source_with_a_group_or_other_write_bit`: it installs a source with mode
     0775 from the temporary directory. The installed file is another inode than its source, has
     the source's length and has mode 0700.
   - `install_copies_a_source_on_another_file_system`: it installs a source on another file system
     (candidate: `/proc/self/exe`, inferred, not run). The installed file is another inode than
     its source, has the source's length and has mode 0700.
   The two copy cases are two tests, so that one failing does not hide the other.
2. The six tests pass, and their durations in the test step of the forge's "repository gate" job
   of this story's pull request sum to at most 76 s. Before: 50.7 s. Estimate: 50.6 s. Bound:
   1.5 × 50.6 = 75.9, rounded up to 76 s. The bound by the rule is above the before, because the
   plan estimates no saving here; it guards against a slower result only.
3. Kept runs: the six tests keep their commands and loops (28 commands for 2 selectors, every
   argument combination, three queued reads). `git diff --name-only` of this story's branch
   against its base lists none of the six test files.

## Scope

- `crates/connectors-custody/src/mounted/fixture.rs`: `Jail::install` (lines 118 to 127).
- `crates/connectors-custody/src/mounted/tests.rs`: the four unit tests.

No other story of this set writes these two files. `apps/connectors/tests/support/mounted.rs` is
no longer edited by this story, so it shares no file with
story:eleven-connection-lease-tests-advance-a-clock-instead-of-sleeping.
`crates/connectors-build/src/test_fixtures.rs` is not edited either. This story reads nothing of
story:eighteen-tests-take-a-template-store-or-jail-built-once-per-run; its `depends_on` edge to
that story is removed.

The six test files are not edited. Other stories of this set edit other tests in two of them:
`owner_slow_start.rs`
(story:seven-cli-tests-of-a-slow-owner-start-no-longer-wait-out-the-10-s-connect-wait) and
`security_mcp_deadline.rs` (story:seven-mcp-tests-use-a-4-s-request-budget-and-shorter-holds).

## Not in this story

- A seam for the three CLI process loops (12.5 s, 12.5 s, 8.2 s). The plan names none.
- The other tests that use the mounted container: among them the 14 tests of
  `connectors::adversary_local_mcp_server` and the 15 of `connectors::local_mcp_server` in the
  band of 1 s to under 5 s. They get the link too; their seconds are not counted here.
- A test that shows the link for `/bin-test/connectors` after `Container::new`. It would sit in a
  test file of `apps/connectors/tests/`; the second unit test above holds the same pair of places
  (a binary under `target/` and a jail root in the temporary directory).
- The gate failing when its test step runs over 120 s:
  story:the-gates-test-step-ends-within-120-s-on-the-forge.
- Not established:
  - Whether a link from `target/` into the jail root works in the forge's gate job. Read: both
    are under the checkout there. Not read or run: whether they are one file system on the
    runner, whether the runner allows the link, and which mode cargo gives a binary there. Where
    one of the three fails, the copy runs as today and this story saves nothing.
  - Whether a test needs the installed binary to be owner-only. The comment of `install` says
    "owner-only"; no check of it was found for an executable.
  - The cost of the copy, of the sha256, of `Container::new` and of one CLI run. None was
    measured, so no "after" figure is claimed.
