---
format: aep.planning-md/3
id: story:the-forges-gate-job-compiles-once-and-prints-its-time-per-step
kind: story
status: active
title: The forge's gate job compiles its test binaries once and prints its time per step
relations:
- serves: vision:independent-contract-adapters
- informed_by: story:the-gates-test-step-ends-within-120-s-on-the-forge
revision: 10
transitions:
- {from: "draft", to: "proposed", at: "2026-11-24T15:26:22Z", actor: "human:timo", revision: 3}
- {from: "proposed", to: "active", at: "2026-11-24T15:26:23Z", actor: "human:timo", revision: 4}
---
## Defect

The forge's "repository gate" job of a pull request runs 61 to 69 minutes. The operator's word of
2026-11-24 11:41 CET: "A gate run of 61 to 69 minutes is a defect: the test step ends within 2
minutes, no test over 60 s. File the story now with the measured split (build against test, slowest
ten) … No raised limit."

`story:the-gates-test-step-ends-within-120-s-on-the-forge` holds the test step. This story holds the
rest of the job: what it compiles, the step that runs the ignored tests, and a printed time per
step.

Measured on pull request 292, job 138329455972 of run 45360621432, 2026-11-24 10:04:38 to 11:08:06
CET, 63 min 28 s (3,808 s). Each row is the time between two lines of the job's log.

| part | seconds | share | read from |
|---|---|---|---|
| test step (`cargo nextest run`) | 842 | 22 % | `gate: test step ran 842.2 s (budget 120 s)` |
| step "Run the ignored tests a built catalog provider suffices for" | 752 | 20 % | 10:55:34 to 11:08:06 CET |
| compile of the test binaries | 750 | 20 % | ``Finished `test` profile [optimized] target(s) in 12m 28s`` |
| first build of the gate (`cargo run -p connectors-build -- gate`) | 449 | 12 % | ``Finished `dev` profile [optimized] target(s) in 7m 29s`` |
| second build inside the gate | 261 | 7 % | ``Finished `dev` profile [optimized] target(s) in 4m 21s`` |
| three `cargo clippy` runs (workspace, one more, Windows target) | 270 | 7 % | 1m 05s, 2m 10s, 1m 14s |
| nine library-boundary checks | 199 | 5 % | 10:52:07 to 10:55:26 CET |
| `connectors-docs generate --check` | 168 | 4 % | 10:49:19 to 10:52:07 CET |
| checkout, toolchain, `cargo fetch` | 91 | 2 % | 10:04:38 to 10:06:09 CET |
| generation checks, scenarios, conformance, `aep plan artifact validate` | 26 | 1 % | the remaining lines |

Build against test:

- compile, four builds: 449 + 261 + 750 + 62 = 1,522 s (40 %);
- tests that ran: 842 s in the test step and 2 s in the ignored step (22 %);
- the ignored step without a printed line: 688 s (18 %), see below;
- lint and checks: 663 s (17 %); setup 91 s (2 %).

The ten slowest tests of that run (`gate: ten slowest tests`):

| seconds | binary | test |
|---|---|---|
| 47.7 | `connectors::connection_lease_adversary` | `adversary_a_hanging_early_recollection_does_not_cost_a_read_its_current_evidence` |
| 36.5 | `connectors::owner_slow_start_adversary` | `a_read_after_one_that_outlasted_its_deadline_reaches_the_owner_it_left` |
| 34.9 | `connectors::local_cli` | `adversary2_a_same_build_owner_at_capacity_is_never_another_build_over_100_runs` |
| 33.9 | `connectors::connection_lease_adversary` | `adversary_two_revalidations_with_a_slow_probe_both_answer_ready_within_the_bound` |
| 31.5 | `connectors-host` | `local::approval_policy::tests::child_process_waits_on_the_lock_wait_its_parent_hands_it` |
| 31.5 | `connectors::owner_idle_exit_adversary` | `idle_owners_exit_within_their_bound_when_it_spans_several_recovery_sweeps` |
| 30.5 | `connectors-host` | `local::adversary_w2_tests::blocked_policy_child_keeps_production_wait_when_the_variable_is_inherited` |
| 30.5 | `connectors-host` | `local::approval_policy::tests::held_use_blocks_cross_process_revocation_until_release` |
| 30.2 | `connectors-host::local_foundation` | `contending_metadata_open_refuses_at_its_bound_and_recovers_after_release` |
| 28.6 | `connectors-host` | `local::owner::governed::tests::bounded_realization_security::bounded_owner_serves_exactly_the_realizations_…` (name cut by the log) |

None is over 60 s. `Summary [ 840.388s] 4379 tests run: 4379 passed (9 slow), 223 skipped`,
`Starting 4379 tests across 240 binaries`.

### The ignored step: 688 s for 4 tests

The step runs `cargo run --locked -p connectors-build -- ignored --required --only-prerequisite
provider` after the gate. Its log:

- 10:55:34 CET the gate ends; 10:56:36 ``Finished `dev` profile [optimized] target(s) in 1m 01s``
  and ``Running `target/debug/connectors-build ignored …` ``;
- no line until 11:08:04 CET (688 s);
- 4 tests between 11:08:04 and 11:08:06 CET.

Not measured, read from the source: `crates/connectors-build/src/ignored.rs` starts `cargo test
--workspace --no-run --locked --offline --message-format=json` with `CARGO_BUILD_JOBS=2` and keeps
its output, then starts each test executable with `--list`. The gate compiled the test binaries
12 minutes earlier with another command line (`cargo nextest`, a `--config` for the target's
runner). Whether the 688 s are a second compile of the 240 test binaries with 2 jobs, or the
listing, is the first measurement of this story.

### Measured 2026-11-24: a build script runs again at each change of cargo's caller

Job 138385972600 of run 45377042383 (pull request 293, 52 min 55 s, 3,175 s). Each line is read
from the job's log; times in CET.

- 11:54:56 `Compiling ring v0.17.14`: the first build, `cargo run --locked -p connectors-build --
  gate` started by the workflow's shell (``Finished `dev` profile`` in 5 min 43 s).
- 12:00:34 `Compiling ring v0.17.14` again, 1 s after the gate started its own `cargo build
  --workspace`, then `rustls`, `reqwest`, `connectors-client`, `connectors-host` and what depends
  on them (``Finished `dev` profile`` in 3 min 29 s, 209 s).
- 12:35:35 `Compiling ring v0.17.14` again, when the workflow's shell starts `cargo run --locked
  -p connectors-build -- ignored …`, then `connectors-host`, `connectors-spec` and
  `connectors-build` (48 s).
- 12:36:23 to 12:45:50: that program's `cargo test --workspace --no-run`, 567 s without a line,
  then 4 tests in 2 s.
- Two more compiles of `ring` belong to steps that build another unit: 12:29:27 the Windows
  lint (`--target x86_64-pc-windows-gnu`), 12:33:20 `cargo build -p connectors-sql --lib
  --no-default-features`.

Cause, measured on the workstation in the tree of 0.78.0 with cargo 1.98.1:

- `ring` 0.17.14 reads its environment through a wrapper (`build.rs`, `mod env`) that prints
  `cargo:rerun-if-env-changed=<name>` for every variable it reads, among them
  `CARGO_MANIFEST_DIR`, `CARGO_PKG_NAME` and `CARGO_PKG_VERSION_MAJOR`, `_MINOR`, `_PATCH` and
  `_PRE`.
- cargo compares a watched variable with its own environment. `cargo run` hands the program it
  starts the package's `CARGO_MANIFEST_DIR` and `CARGO_PKG_*`, so a cargo that `connectors-build`
  starts sees them set, and a cargo the shell starts sees them unset.
- `cargo build --locked --offline -p ring -v`, twice from the shell: `Fresh ring v0.17.14`.
- The same command with `CARGO_MANIFEST_DIR`, `CARGO_PKG_NAME` and the four version variables
  set: `Dirty ring v0.17.14: the env variable CARGO_MANIFEST_DIR changed`, and `ring` is
  compiled.
- The same command from the shell again: the same `Dirty` line, and `ring` is compiled again.

So each change between a cargo of the shell and a cargo of `connectors-build` compiles `ring`
and everything that depends on it. In the job above that is the second build inside the gate
(209 s) and the build at the start of the ignored step (48 s). Inferred, not read from a line:
the 567 s without a line are the same thing once more, the test binaries of the workspace built
again with 2 jobs after the shell's `cargo run` had made `ring` new (the gate's own build of the
test binaries took 615 s). Together 824 s of the job's 3,175 s (26 %).

Change made with this measurement: every cargo that `connectors-build` starts for a build
(`gate`, `ignored`, the documentation build) is started without the `CARGO_MANIFEST_*` and
`CARGO_PKG_*` variables it inherited (`cargo_program` in `crates/connectors-build/src/main.rs`;
two tests there). What the forge's job takes after it is read from the first run that has it.

### Measured 2026-11-24: the first forge run without the package variables

Job 138569201465 of run 45429122108 (pull request 294, release 0.78.1), 16:46:03 to 17:37:21
CET, 51 min 18 s (3,078 s). Each line is read from the job's log.

| part | 292 | 293 | 294 |
|---|---|---|---|
| whole job | 3,808 s | 3,175 s | 3,078 s |
| first build (`cargo run … gate` from the shell) | 449 s | 343 s | 449 s |
| second build inside the gate (`cargo build --workspace`) | 261 s | 209 s | 250 s |
| build of the test binaries | 750 s | 615 s | 758 s |
| test step | 842.2 s | 735.4 s | 856.7 s |
| step of the ignored tests | 752 s | 617 s | 6 s |
| `Compiling ring` lines | not counted | 5 | 3 |

What the change did:

- The step of the ignored tests: ``Finished `dev` profile`` in 0.26 s, the first test 4 s after
  the program started, the report 6 s after the step began. The 48 s build at its start and the
  567 s without a line are gone. The inference above (the 567 s were the test binaries built
  again) is confirmed by this: nothing else changed in that step.
- `ring` is compiled 3 times: by the first build (16:47:58 CET), by the Windows lint
  (17:29:52, another target) and by `cargo build -p connectors-sql --lib --no-default-features`
  (17:34:29, another feature set). The compile 1 s after the gate's own build began is gone.

What it did not do, against the expectation written above:

- The second build inside the gate still takes 250 s. Its log has 100 `Compiling` lines (104 on
  293): without `ring` and what was built again with it, the build compiles 100 crates the
  first build did not make in that form, among them `connectors-sdk`, `connectors-client`,
  `connectors-host`, `connectors-spec`, `connectors-build` and `connectors-docs`. So the 209 s
  of pull request 293 were not the cost of `ring`. Not measured yet: why a build of the
  workspace compiles `connectors-spec` and `connectors-build` again after `cargo run -p
  connectors-build` built them. Inferred: `--workspace` selects more features of shared
  dependencies (`tokio`, `hyper`, `reqwest` are in the list), so the crates above them are
  built a second time. That is change 3.
- Of the 824 s named above, 611 s are confirmed (the ignored step, 617 s to 6 s) and 209 s are
  not.
- The job is 97 s shorter than on 293 and 730 s shorter than on 292. The runner of 294 was as
  slow as the one of 292 in every other row (first build 449 s on both, test binaries 750 s
  and 758 s, tests 842 s and 857 s), and the one of 293 faster, so 292 is the fairer
  comparison; that is a reading of three runs, not a measurement of the runners.

Also seen in that run: 4,483 tests across 241 binaries, `Summary [ 854.839s] 4483 tests run:
4483 passed (9 slow), 223 skipped`; the three slowest are the same as on 292 (47.8 s, 36.5 s,
34.9 s).

### Measured 2026-11-24: what each build of the job compiles again, and why

Read from the same log (pull request 294) and from cargo's unit graph of that tree (`cargo build
… --unit-graph -Z unstable-options`, which prints the units a call would compile and compiles
nothing). A unit is a package with one feature set.

| call of the job | `Compiling` lines | of them compiled before in another form | time |
|---|---|---|---|
| shell: `cargo run -p connectors-build -- gate` | 314 | first build | 449 s |
| gate: `cargo build --workspace` | 100 | 40 | 250 s |
| gate: the test binaries (`cargo nextest run --workspace`) | 43 | 39 | 758 s |
| gate: `cargo run -p connectors-tool-contracts` | 51 | not counted | 65 s |
| gate: `cargo run -q -p connectors-docs` | no line, `-q` | not known | 167 s for the step |

The cause is one rule of cargo: features are resolved for the packages a call selects.

- `-p connectors-build` selects one package. Against `--workspace`, 10 packages get another
  feature set: `clap` and `clap_builder` (`string`), `crossbeam-utils` and `crc32fast`
  (`std`), `getrandom` 0.4 (`std`, `sys_rng`), `futures-channel` (`sink`), `tokio-util`
  (`codec`), `tokio-rustls` (`ring`), `toml_writer` (`default`) and
  `connectors-catalog-provider` (`service`). Everything above them is compiled again by the
  workspace build: the 40, with `hyper`, `reqwest`, `zbus`, `connectors-sdk`,
  `connectors-host`, `connectors-catalog`, `connectors-spec` and `connectors-build` itself.
- `-p connectors-tool-contracts` has 19 units the workspace build lacks (14 packages with
  another feature set, among them `libc`, `memchr`, `serde_json`, `syn`, `uuid`), and `-p
  connectors-docs` has 11 (7 packages). Each is one more feature set of the same crates.
- The test build adds what only tests ask for: `tokio` with `test-util`, `connectors-host` and
  `connectors-custody` with `test-support`, and four crates only tests use (`rcgen`, `pem`,
  `yasna`, `openapiv3`). Everything above `tokio` is compiled again, which is every crate of
  the workspace. This one is not removed by a selection: a build without `test-util` is the
  program a user gets.

The same unit graph for `cargo build --bin connectors-build`, `--bin connectors-tool-contracts`
and `--bin connectors-docs`: 404, 156 and 342 units, each with 0 units that the workspace build
lacks. The workspace has no root package, so a call without `-p` selects every member and
resolves the features of the workspace, and `--bin` then builds the one program.

Also read: the test build makes 241 test binaries: 106 of `apps/connectors`, 34 of
`crates/connectors-host`, 19 of `adapters/kubernetes`, 17 of `adapters/sql`, 65 of the 19
other packages. The folder rule names one integration-test binary per crate. That is the
test-step story's, not this one's.

### Measured 2026-11-24: the first forge run that prints its steps

Job 138646969126 of run 45450782257 (pull request 295), 18:39:42 to 19:19:43 CET, 40 min 1 s
(2,401 s). The table is the job's own (`gate: time per step`); the rows around it are read from
the job's step times and cargo's `Finished` line.

| part | s |
|---|---|
| the job's steps before the gate (toolchain, ESS, AEP, nextest, fetch) | 99 |
| build of `connectors-build` by `cargo run -p` (``Finished `dev` profile … in 5m 44s``) | 344 |
| build of the workspace | 203.2 |
| build of the test binaries | 599.3 |
| tests (`Summary [ 637.457s] 4495 tests run: 4495 passed (9 slow), 223 skipped`) | 638.6 |
| documentation drift check | 134.9 |
| clippy of the workspace | 91.8 |
| library boundary of `connectors-sql` | 67.3 |
| windows lint | 53.2 |
| tool contracts check | 50.7 |
| library boundary of `connectors-catalog-provider` | 44.4 |
| library boundary of `connectors-grafana` | 25.6 |
| library boundary of `connectors-mcp` | 13.8 |
| the 16 other steps, each under 5 s | 25.3 |
| sum of the gate's 27 steps, as printed | 1,948.1 |
| the step of the ignored tests (its table: 0.4, 2.2, 1.3, 0.0) | 4.0 |

344 s and 1,948.1 s are 2,292 s; the workflow step of the gate ran 2,293 s.

- Compiling takes 1,147 s of the 2,401 s before one test runs: 344 + 203.2 + 599.3.
- The four steps change 3 is about (the first build, the build of the workspace, the tool
  contracts, the documentation drift check) take 733 s. How much of the 134.9 s of the
  documentation check is a compile is not in the log (`-q`).
- The lints and the boundary builds of change 4 take 304.2 s: clippy 91.8, the Windows lint 53.2,
  the ten boundary steps 159.2. Three boundary builds hold 137.3 s of those: `connectors-sql`,
  `connectors-catalog-provider`, `connectors-grafana`.
- The step of the ignored tests spends 2.6 s before its first test. Acceptance 2 holds.
- This runner was faster than the one of pull request 294 in every row (first build 344 s
  against 449 s, tests 638.6 s against 856.7 s). A comparison of two pull requests needs the
  same rows from both, not the job's total.

### Measured 2026-11-24: the forge run with `--bin`

Job 138682816066 of run 45460889072 (pull request 296, change 3), 19:42:10 to 20:17:04 CET,
34 min 54 s (2,094 s), against 2,401 s on pull request 295. Both tables are the jobs' own; the
crate counts are the `Compiling` lines of each step in the two logs.

| part | 295: s | 296: s | 295: crates | 296: crates |
|---|---|---|---|---|
| the job's steps before the gate | 99 | 97 | | |
| build of `connectors-build` by `cargo run` | 344 | 337 | 314 | 315 |
| build of the workspace | 203.2 | 80.2 | 100 | 61 |
| build of the test binaries | 599.3 | 597.8 | 43 | 43 |
| tests | 638.6 | 641.3 | | |
| tool contracts check | 50.7 | 0.2 | 51 | 0 |
| documentation drift check | 134.9 | 1.0 | not in the log (`-q`) | not in the log (`-q`) |
| clippy of the workspace | 91.8 | 93.1 | 1, and 342 checked | 1, and 342 checked |
| windows lint | 53.2 | 52.4 | 32, and 244 checked | 32, and 244 checked |
| library boundary of `connectors-grafana` | 25.6 | 37.9 | 9 | 37 |
| library boundary of `connectors-kubernetes` | 3.4 | 4.8 | 1 | 5 |
| library boundary of `connectors-loki` | 2.5 | 2.4 | 1 | 1 |
| library boundary of `connectors-prometheus` | 0.9 | 0.9 | 1 | 1 |
| library boundary of `connectors-sql` | 67.3 | 66.4 | 53 | 57 |
| library boundary of `connectors-tavily` | 1.0 | 0.9 | 1 | 1 |
| library boundary of `connectors-catalog-provider` | 44.4 | 43.8 | 17 | 18 |
| library boundary of `connectors-mcp` | 13.8 | 13.5 | 24 | 24 |
| sum of the gate's 27 steps, as printed | 1,948.1 | 1,653.4 | | |
| the step of the ignored tests | 4.0 | 3.9 | | |

337 s and 1,653.4 s are 1,990 s; the workflow step of the gate ran 1,991 s (18:43:47 to 19:16:58
UTC). The tests' line: `Summary [ 640.117s] 4497 tests run: 4497 passed (9 slow), 223 skipped`.

- The four steps of change 3 take 418.4 s, against 732.8 s: the first build 337, the build of
  the workspace 80.2, the tool contracts 0.2, the documentation check 1.0. The job is 307 s
  shorter, the sum of the 27 steps 294.7 s.
- The build of the workspace compiles 61 crates. Two names are in the first build's 315 too:
  `connectors-catalog-provider` and `toml_edit`. Why is not read. On pull request 294 it compiled
  100, 40 of them a second time.
- The tool contracts check compiles nothing. The documentation check ends in 1.0 s.
- The eight boundary builds compile 144 crates in 170.6 s, against 107 in 158.9 s: 28 more for
  `connectors-grafana`, 4 for `connectors-kubernetes`, 4 for `connectors-sql`, 1 for
  `connectors-catalog-provider`. Not established: whether the builds of the tool contracts and
  of the documentation program by `-p` made those units on pull request 295.
- The test build compiles 43 crates, 39 of them compiled by a build before it in the same job.
  That is the part change 3 names as not changed.
- Compiling takes 1,015 s before one test runs (337 + 80.2 + 597.8), against 1,147 s.
- Both runs are one runner each. The rows that did not change (test binaries 599.3 and 597.8,
  clippy 91.8 and 93.1, windows lint 53.2 and 52.4) differ by under 2 s, so the two runners
  were about equally fast.

For change 4: the lints and the boundary builds take 316.4 s on pull request 296 (clippy 93.1,
the Windows lint 52.4, the ten boundary steps 170.9). Each boundary step runs `cargo build
--locked --offline -p <package> --lib --no-default-features` and compiles the package's
dependencies with that feature set; clippy and the Windows lint check 342 and 244 crates.

## Change

1. The gate and the ignored step print one line per step with its wall time, and a closing table
   of the steps, so the split above is read from one place and not from timestamps.
   Made 2026-11-24 (`crates/connectors-build/src/steps.rs`): `gate: step ran <seconds> s: <name>`
   when a step ends, then `gate: time per step` with one row per step and `sum of <n> steps`;
   27 steps without `--msrv`, 8 of them the library-boundary builds, 4 the test step's. A run an
   error ended prints the table too and names the step that did not end. `connectors-build
   ignored` prints the same on standard error with four rows: the build of the test binaries, the
   listing of the ignored tests, the ignored tests, the report. The row "build of the test
   binaries" of the ignored step is the measurement change 2 asks for. Not in a row: the build
   of `connectors-build` itself by `cargo run`, which cargo prints as ``Finished `dev` profile``.
   Read from the forge the same day (pull request 295): 27 rows, 1,948.1 s, in the section on
   the first forge run that prints its steps.
2. Measure the 688 s of the ignored step (a run with the build's output shown). Then remove them:
   the step reads the test binaries the gate built, or the gate hands their list over, so that the
   job compiles the test binaries once.
3. Measure why the gate builds three times before its tests (449 s, 261 s, 750 s) and what each
   build makes that the one before did not. One build of the workspace and one of the test
   binaries is the target.
   Measured 2026-11-24 (the section on what each build compiles again): each `cargo run -p`
   of the job is a feature set of its own. Made the same day: the forge's three calls of the
   build program and the gate's calls of `connectors-docs` and `connectors-tool-contracts`
   name the program with `--bin`, which selects the workspace; two tests of
   `crates/connectors-build` fail when one of them selects a package. Expected from the unit
   graph: the gate's build of the workspace compiles the 60 crates the first build did not
   need and none a second time, and the tool contracts and the documentation check compile
   nothing. Read from the forge the same day (pull request 296, the section on the forge run
   with `--bin`): the build of the workspace compiles 61 crates in 80.2 s (before: 100 in
   203.2 s), two of them named in the first build too; the tool contracts check takes 0.2 s
   (50.7 s) and the documentation check 1.0 s (134.9 s); the job ran 2,094 s (2,401 s). Not
   changed: the test build compiles the crates above `tokio` again, because tests ask for
   `test-util`.
4. The three `cargo clippy` runs and the nine boundary checks: time each, and name which of them
   compile again what a step before compiled.
   Timed and counted 2026-11-24 from the two forge runs (the same section): 316.4 s on pull
   request 296. Not done: which crates of a boundary build a step before it compiled with
   another feature set.

Each of 2 to 4 is a measurement first. The stories that change the job are drafted from the
numbers, not from this list.

## Acceptance

- A "repository gate" job of a pull request prints the wall time of each step and their sum.
- The step that runs the ignored tests spends under 60 s before its first test when the gate ran
  before it in the same job.
- The job compiles the test binaries once: its log has one ``Finished `test` profile`` line that
  names more than 60 s.
- The job's time on a pull request that changes one crate is reported before and after, from two
  forge runs, in this story's evidence.

## Limits

- No limit of the forge or of the gate is raised, and no test is skipped or sharded to reach a
  number (the operator, 2026-11-24: "No raised limit").
- The bound for the whole job is not set here. The folder rule names 120 s for the test step and
  60 s for a test; a number for the compile is the operator's to name.
- Order: first story after release 0.78.0 and the move of the side installs (the operator,
  2026-11-24 11:41 CET).

## As built

Read from the forge on 2026-11-26: "repository gate" job 139714078337 (run 45748988866, attempt
2) of pull request 313, head 606a611d5, 06:01:26 to 06:33:18 UTC. Attempt 1 (job 139671926182)
was cancelled after 90 minutes in `apt-get update`, before the gate started.

- Acceptance 1 holds: the job printed `gate: time per step` with 27 rows and `gate: 1480.4 s sum
  of 27 steps`, and the step of the ignored tests printed `ignored: time per step` with 4 rows
  and `ignored: 3.9 s sum of 4 steps`.
- Acceptance 2 holds: the step of the ignored tests began at 06:33:12.038 (`##[group]Run cargo
  run --locked --bin connectors-build -- ignored …`) and printed its first `ignored: running`
  line at 06:33:14.909, 2.9 s later (arithmetic); its rows `0.4 s build of the test binaries`
  and `2.2 s listing of the ignored tests`.
- Acceptance 3 holds: the log has three ``Finished `test` profile`` lines, `in 9m 59s` (the
  gate's step "build of the test binaries", 599.9 s), `in 0.29s` (the test step) and `in 0.20s`
  (the doctests). One names more than 60 s.
- Acceptance 4 is open: this story has no evidence record. The body holds pull request 295
  (job 138646969126, 2,401 s) against pull request 296 (job 138682816066, 2,094 s), but pull
  request 296 changes two crates (`connectors-build`, `connectors-host`) and the workflow.
  Pull requests that change one crate outside the planning store and documents, with their gate
  job's time from the jobs' start and end (UTC): 295 (`connectors-build`, before change 3) job
  138646969126, 2,401 s; 300 (`connectors-host`, after change 3) job 138945630062, 2,736 s; 307
  (`connectors-host`) job 139129733813, 2,076 s. No pull request before change 2 (pull request
  294) changes one crate. Which two runs are "before" and "after" is not named by the statement;
  the record waits for that choice.

The story stays active and is not moved to implemented: acceptance 4 has no record.
