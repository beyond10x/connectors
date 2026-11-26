---
format: aep.planning-md/3
id: story:the-sixteen-ess-model-tests-read-schemas-generated-once-per-run
kind: story
status: draft
title: 'Sixteen ess_model tests read schemas generated once per run: 2.18 s on the build host, at most 1.41 s on the forge'
summary: 16 tests of adapters/sql/tests/ess_model.rs each run ess generate for the same 4 source files; one directory per test run, keyed by a digest of its sources, replaces 15 of the 16 calls
relations:
- decomposes: story:the-gates-test-step-ends-within-120-s-on-the-forge
- informed_by: story:where-do-the-82-s-of-the-eleven-store-tests-go-that-the-registry-story-leaves
scope:
- confidence: cited
  path: adapters/sql/tests/ess_model.rs
revision: 2
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

On the forge the call is cheaper. In the test step of four "repository gate" jobs (pull
requests 302, 303, 304 and 307) five of the 16 tests ended in 0.061 to 0.088 s each, the call
included. A call there takes at most 0.088 s; 16 calls at most 1.41 s (arithmetic). The forge
figure with the calls removed is not measured.

## Change

Test code only, one file, `adapters/sql/tests/ess_model.rs`.

1. `schema` reads from one directory per test run: a directory under the target directory
   (`CARGO_TARGET_TMPDIR`) whose name holds a digest of its sources, the 4 files under
   `adapters/sql/spec/ess` and the version of the pinned `ess`. A test that finds the directory
   reads it. A test that does not find it generates into a temporary directory beside it and
   renames that into place, so that a test in another process reads a whole directory or none.
   When the rename finds the directory there, the loser's copy is removed.
2. The `out` argument and the per-test temporary directory go where a test used them only for
   the schemas.

Not decided in this draft: whether a committed generated file with a digest of its sources is
the better form. It would remove the 16th call too and add a regeneration step for every change
of `adapters/sql/spec/ess`.

## Acceptance

1. A red-capable check that the product is made once: after
   `cargo nextest run --locked -p connectors-sql --test ess_model` on an empty target temporary
   directory, a second run of the binary starts no `ess` process. How the test observes that
   (a wrapper around the pinned `ess` that counts its starts, as other tests of the repository
   use, or a count kept by `schema`) is settled before implementation.
2. A changed source makes a new product: with one byte of a file under `adapters/sql/spec/ess`
   changed, the next run generates again. Shown once; the changed byte is not committed.
3. Behaviour: the 17 tests pass, and no assertion of a test changes.
   `git diff --stat` of the pull request names one file outside the planning store.
4. Time: the 17 durations of the binary in the test step of the forge's "repository gate" job
   of the pull request sum to less than in the newest gate job on main at or before its base
   commit. The bound and its margin are set before implementation, from a forge job that holds
   the fixture story.

## Scope

- `adapters/sql/tests/ess_model.rs`: the helper `schema` and its call sites.

Stories of the set that name files of the same crate's tests:
story:the-mysql-test-fixture-sends-its-packets-without-the-nagle-hold writes
`adapters/sql/tests/mysql/fixture.rs`, and
story:five-tests-no-longer-wait-out-the-http-and-sql-client-timeouts writes
`adapters/sql/tests/mysql_protocol.rs`. Neither writes this file.

## Not in this story

- The fixture's hold: story:the-mysql-test-fixture-sends-its-packets-without-the-nagle-hold.
- `the_model_admits_exactly_the_read_profiles_the_executable_admits`, 0.559 to 0.700 s as the
  code is and 0.557 to 0.800 s with the probe: its time is not in the call.
- The other callers of `ess generate` in tests of the repository: not counted here.
