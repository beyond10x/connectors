---
format: aep.planning-md/3
id: story:the-sixteen-ess-model-tests-read-schemas-generated-once-per-run
kind: story
status: draft
title: 'Sixteen ess_model tests read schemas generated once per run: 2.18 s on the build host; on the forge at most 1.41 s, measured on 5 of the 16 tests'
summary: 16 tests of adapters/sql/tests/ess_model.rs each run ess generate for the same 4 source files; one directory per test run under the target directory, keyed by a digest of its sources and filled under a file lock, replaces 15 of the 16 calls
relations:
- decomposes: story:the-gates-test-step-ends-within-120-s-on-the-forge
- informed_by: story:where-do-the-82-s-of-the-eleven-store-tests-go-that-the-registry-story-leaves
- depends_on: story:the-mysql-test-fixture-sends-its-packets-without-the-nagle-hold
scope:
- confidence: cited
  path: adapters/sql/tests/ess_model.rs
revision: 4
---
## Defect

16 of the 17 tests of `adapters/sql/tests/ess_model.rs` (binary `connectors-sql::ess_model`)
run `ess generate --path spec/ess --kind schema` once each, through the helper `schema`
(line 24), into a temporary directory of their own. Every call makes the same files from the
same 4 source files under `adapters/sql/spec/ess`. The folder rule "an expensive product is
made once, never per test" names this shape.

Measured on the build host on 2026-11-25, on a head that holds
story:the-mysql-test-fixture-sends-its-packets-without-the-nagle-hold, the binary alone,
`cargo nextest run --locked -p connectors-sql --test ess_model`, the 17 durations nextest
printed summed per run, with the jobs running in the pool beside each run:

- as the code is: 3.295 s (5 jobs), 4.543 s (7 jobs), 3.424 s (6 jobs); median 3.424 s;
- with a temporary probe, taken back: `schema` reads one directory under the target directory
  that one test filled before the run: 0.909 s (5 jobs), 1.242 s (4 jobs), 1.349 s (5 jobs);
  median 1.242 s.

The medians differ by 2.18 s (arithmetic). Per test: a test that does little besides the call
took 0.155 to 0.300 s as the code is and 0.007 to 0.013 s with the probe, over the three runs
(`the_read_binding_and_the_mysql_cell_rules_match_the_model`,
`the_password_bound_counts_bytes_in_the_model_and_the_executable`,
`each_columns_masking_is_the_one_the_model_fixes`).

On the forge the call is cheaper, and it is measured on 5 of the 16 tests only. In the test
step of four "repository gate" jobs (pull requests 302, 303, 304 and 307) five of the 16 tests
ended in 0.061 to 0.088 s each, the call included. A call there takes at most 0.088 s; 16
calls at most 1.41 s (arithmetic, the other 11 tests not read). The forge figure with the calls
removed is not measured. The saving on the forge is inside what two gate jobs differ by, so
this story has no acceptance on time: it is built for the rule, not for seconds.

## Change

Test code only, one file, `adapters/sql/tests/ess_model.rs`.

1. `schema` reads from one directory per test run, under the target directory:
   `CARGO_TARGET_TMPDIR/connectors-sql-ess-model-schemas/<key>`. The key is a SHA-256 over the
   names and bytes of the 4 files under `adapters/sql/spec/ess`, in name order, and the
   version of the pinned `ess` (`connectors_spec::toolchain::version()`). The name of the
   parent directory holds the crate and the binary, because `target/tmp` is shared by every
   package of the tree.
2. Creation is single-flight across processes. A caller opens the file `<key>.lock` beside the
   directory and takes an exclusive lock on it (`std::fs::File::lock`), which the operating
   system releases when the process ends. Under the lock: when the directory is there, it is
   read; when not, the caller generates into `<key>.<process id>.tmp` beside it, renames that
   into place and appends one line to `generations.log` in the parent directory. A caller in
   another process therefore reads a whole directory or waits for one. cargo-nextest runs each
   test in its own process, so an in-process global is not used.
3. The helper takes the parent directory and the source directory as arguments, so that a
   test can point it at directories of its own. The 16 tests pass the two defaults.
4. The `out` argument and the per-test temporary directory go where a test used them only for
   the schemas.

Decided: the form is the fixture on disk keyed by the digest of its sources. The other
accepted form, a committed generated file with a digest of its sources, is not taken: it adds
a regeneration step and a drift check for every change of `adapters/sql/spec/ess`, for a
product that takes at most 0.3 s to make.

## Acceptance

1. Made once on a cold directory, across processes: a new test
   `four_processes_on_an_empty_directory_generate_once` starts 4 children of its own test
   binary at once (each runs the one child test, which calls the helper with the pinned `ess`
   on a parent directory the test made empty) and then reads `generations.log`: 1 line. The 4
   children exit 0 and each printed the same directory.
2. Acceptance 1 is red-capable: with the lock removed (not committed), one run of the test is
   recorded in "As built" with its line count. If that run still shows 1 line, "As built" says
   so and names what the test does not see.
3. A changed source makes a new product: a new test
   `a_changed_source_byte_makes_a_new_directory` copies the 4 files to a temporary directory,
   calls the helper, changes one byte of one copy and calls it again: the two directories
   differ and `generations.log` holds 2 lines. No file under `adapters/sql/spec/ess` changes.
4. The 17 tests that exist today pass in the test step of the forge's "repository gate" job of
   the pull request.
5. No assertion of an existing test changes: the pull request's diff of
   `adapters/sql/tests/ess_model.rs` removes no line that contains `assert`.
6. `git diff --stat` of the pull request names one file outside the planning store.
7. The two new tests end within 5 s each in the same job; neither is a `slow_lane_` test.

"As built" records the 17 durations of the binary in the pull request's gate job beside those
of the newest gate job on main at or before its base commit. That is a record, not an
acceptance.

## Scope

- `adapters/sql/tests/ess_model.rs`: the helper `schema`, its call sites, the two new tests
  and the child test.

No `Cargo.toml` changes: `sha2`, `hex`, `tempfile` and `connectors-spec` are dev-dependencies
of `connectors-sql` already. Nothing but the tests writes under
`CARGO_TARGET_TMPDIR/connectors-sql-ess-model-schemas`.

Other stories near this file:

- story:the-mysql-test-fixture-sends-its-packets-without-the-nagle-hold writes
  `adapters/sql/tests/mysql/fixture.rs`, which `ess_model.rs` includes by `#[path]`, and
  changes the duration of `every_page_is_one_the_model_admits`. This story depends on it.
- story:five-tests-no-longer-wait-out-the-http-and-sql-client-timeouts writes
  `adapters/sql/tests/mysql_protocol.rs`. It does not write this file.
- story:test-binary-per-package-rest (draft, no scope) would move this file under one test
  binary and change the name `--test ess_model`. It is not planned beside this story.

## Not in this story

- The fixture's hold: story:the-mysql-test-fixture-sends-its-packets-without-the-nagle-hold.
- `the_model_admits_exactly_the_read_profiles_the_executable_admits`, 0.559 to 0.700 s as the
  code is and 0.557 to 0.800 s with the probe: its time is not in the call.
- The other tests that call `ess generate`, one call each per test that uses it:
  `adapters/sql/tests/mysql_adversary.rs`, `adapters/sql/tests/mysql_adversary_2.rs`,
  `adapters/grafana/tests/ess_model.rs`, `adapters/loki/tests/ess_model.rs`,
  `adapters/loki/tests/adversary_pass1.rs`, `adapters/prometheus/tests/ess_model.rs`. Not
  counted and not measured here; they belong to
  story:where-do-the-1298-s-of-the-tests-under-5-s-go.
