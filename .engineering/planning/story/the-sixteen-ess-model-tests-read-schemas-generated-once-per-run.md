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
- serves: vision:independent-contract-adapters
scope:
- confidence: cited
  path: adapters/sql/tests/ess_model.rs
revision: 6
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
   directory and takes an exclusive lock on it (`std::fs::File::lock`). The helper drops the
   file, and with it the lock, when it returns, so that the threads of one `cargo test`
   process, each with an open file of its own, take it in turn; the operating system
   releases it when a process is killed. Under the lock: when the directory is there, it is
   read; when not, the caller generates into `<key>.<process id>.tmp` beside it, renames that
   into place and appends one line to `generations.log` in the parent directory. A caller in
   another process therefore reads a whole directory or waits for one. cargo-nextest runs each
   test in its own process, so an in-process global is not used.
   Under the lock a directory `<key>.*.tmp` beside it is stale, left by a process that was
   killed; the caller removes its own `<key>.<process id>.tmp` before it generates.
3. The function that takes the lock carries `#[allow(clippy::incompatible_msrv)]` with the
   reason in a comment: `File::lock` is stable since Rust 1.89, `connectors-sql` inherits the
   workspace's `rust-version` of 1.88, and the gate's clippy runs with `-D warnings` over all
   targets. The file is a test binary compiled with the pinned toolchain alone; the gate's
   check at 1.88 compiles the library of `connectors-sql`, not its tests. The file carries
   `#![cfg(unix)]`, so the lock is `flock` on Linux and macOS.
4. Two functions. `schemas_in(parent, sources)` does items 1 and 2 for a parent directory and
   a source directory and returns the directory of the key. `schema(name)` loses its `out`
   argument, calls `schemas_in` with the two defaults and reads one schema from the result.
   The 16 tests call `schema(name)`. The two new tests and the child test call `schemas_in`
   with directories of their own.
5. The per-test temporary directory goes where a test used it only for the schemas. The
   executable `ess` is started in one place of the file, inside `schemas_in` under the lock.
6. The child test of acceptance 1 reads the parent directory from an environment variable and
   returns at once when the variable is absent, as an ordinary run has it. It is not an
   ignored test.
7. The two new tests of acceptances 1 and 3 each make a parent directory of their own with
   `tempfile::tempdir_in(CARGO_TARGET_TMPDIR)`, and the children get that one. No new test
   writes the default parent directory, so its `generations.log` holds lines of the 16 tests
   alone, and two runs in one tree do not empty each other's directory.

Decided: the form is the fixture on disk keyed by the digest of its sources. The other
accepted form, a committed generated file with a digest of its sources, is not taken: it adds
a regeneration step and a drift check for every change of `adapters/sql/spec/ess`, for a
product that takes at most 0.3 s to make.

## Acceptance

1. Made once on a cold directory, across processes: a new test
   `four_processes_on_an_empty_directory_generate_once` starts 4 children of its own test
   binary at once (each runs the one child test, which calls `schemas_in` with the pinned
   `ess` on the new, empty parent directory of the test) and then reads `generations.log`: 1
   line.
   The 4 children exit 0 and each printed the same directory.
2. Acceptance 1 is red-capable: with the lock removed (not committed), one run of the test is
   recorded in "As built" with its line count. If that run still shows 1 line, "As built" says
   so and names what the test does not see.
3. A changed source makes a new product: a new test
   `a_changed_source_byte_makes_a_new_directory` copies the 4 files to a temporary directory,
   calls `schemas_in` on a parent directory of its own, changes one byte inside a comment
   line of the copy of `domains/profiles.yaml` and calls it again: the two directories differ
   and that parent's `generations.log` holds 2 lines. No file under `adapters/sql/spec/ess`
   changes.
4. The 17 tests that exist today pass in the test step of the forge's "repository gate" job of
   the pull request.
5. No assertion of an existing test changes: the pull request's diff of
   `adapters/sql/tests/ess_model.rs` removes no line that contains `assert` from a function
   marked `#[test]` or `#[tokio::test]`. The helper's own `assert!(status.success(), …)`
   moves into `schemas_in` and is not such a line.
6. The 16 tests use the shared directory: in `adapters/sql/tests/ess_model.rs` as the pull
   request leaves it, `Command::new` of the `ess` executable appears once, inside
   `schemas_in`; `schema` takes the name alone; and `schemas_in` is called from `schema`,
   from the two new tests and from the child test, and from nowhere else.
7. `git diff --stat` of the pull request names one file outside the planning store.
8. The three new tests, the child test among them, end within 5 s each in the same job; none
   is a `slow_lane_` test.
9. The gate's clippy step passes in the same job.

"As built" records the 17 durations of the binary in the pull request's gate job beside those
of the newest gate job on main at or before its base commit. That is a record, not an
acceptance.

## Scope

- `adapters/sql/tests/ess_model.rs`: `schema`, the new `schemas_in`, the call sites of
  `schema`, the two new tests and the child test.

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
  code is and 0.557 to 0.800 s with the probe: its time is not in the call. No story.
- The other test files that start `ess generate` (found with a search for `"generate"` under
  `adapters/*/tests` and `crates/connectors-build/tests`):
  `adapters/sql/tests/mysql_adversary.rs`, `adapters/sql/tests/mysql_adversary_2.rs`,
  `adapters/grafana/tests/ess_model.rs`, `adapters/loki/tests/ess_model.rs`,
  `adapters/loki/tests/adversary_pass1.rs`, `adapters/prometheus/tests/ess_model.rs`,
  `crates/connectors-build/tests/catalog_guard_model.rs`, `catalog_walk_model.rs`,
  `catalog_binary_model.rs`, `catalog_amendment_model.rs`, `catalog_identity_model.rs` and
  `fresh_checkout_regeneration.rs` of the same directory. Each file starts it in one place,
  and the 12 files hold 42 test functions, so a run makes at most 42 such calls. Their calls
  per run and their times are not measured. No story claims them:
  story:where-do-the-1298-s-of-the-tests-under-5-s-go reads four other binaries. At the 0.088 s
  that a call takes at most on the forge for this story's sources, 42 calls are at most 3.7 s
  (arithmetic; their sources differ). No story.
