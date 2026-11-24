---
format: aep.planning-md/3
id: story:the-forges-gate-job-compiles-once-and-prints-its-time-per-step
kind: story
status: active
title: The forge's gate job compiles its test binaries once and prints its time per step
relations:
- serves: vision:independent-contract-adapters
- informed_by: story:the-gates-test-step-ends-within-120-s-on-the-forge
revision: 4
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

## Change

1. The gate and the ignored step print one line per step with its wall time, and a closing table
   of the steps, so the split above is read from one place and not from timestamps.
2. Measure the 688 s of the ignored step (a run with the build's output shown). Then remove them:
   the step reads the test binaries the gate built, or the gate hands their list over, so that the
   job compiles the test binaries once.
3. Measure why the gate builds three times before its tests (449 s, 261 s, 750 s) and what each
   build makes that the one before did not. One build of the workspace and one of the test
   binaries is the target.
4. The three `cargo clippy` runs and the nine boundary checks: time each, and name which of them
   compile again what a step before compiled.

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
