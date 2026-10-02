unit:                   story:sql-fixture-accepts-stray-connections — SQL fixture distinguishes cancellation connections
verdict:                blocked — partial verification; 50 repetitions under workspace load pending
cases:                  executed 13→17, red 4
origin:                 n/a
wrote-outside-worktree: none (tool-managed worktree lease and compiler caches only)
needs-coordinator:      yes — execute 50 repetitions during workspace tests and record measured cause/case rename through AEP; no source patch needed

## 1. Unit and acceptance

Keep admitted SQL-session counts exact while recognizing valid PostgreSQL CancelRequest traffic separately; reject malformed cancellation, unexpected startup and a second SUT session. Complete 50 repetitions during an overlapping full-workspace test run.

The story's only scope entry is cited (`adapters/sql/tests/local_runtime.rs`); no inferred scope lines or dependency edges were present. Read the complete story, repository AGENTS.md, affected fixture/protocol tests, runtime cancellation code and the artifact graph. Graph evidence is `graph.json` in this directory. No out-of-scope source edit was needed.

The original unrelated-traffic mechanism was explicitly a hypothesis. Before the fix, deliberate foreign startup reproduced the fixture assertion and wrong session count (`red.log`). The extra-session control independently produced a packet with bytes `[4,210,22,46,0,0,0,42,0,0,4,210]` (`startup-probe.log`). That is the PostgreSQL CancelRequest code 80877102 and the fixture's backend PID 42/key 1234. `adapters/sql/src/lib.rs` sends this control connection when a query fails; dependency `postgres-protocol` encodes the same bytes. A cancellation-specific case was then run red before implementing classification (`cancel-red.log`). Coordinator approved the evidence-led refinement and new name `sql_fixture_counts_cancel_requests_separately`.

The fixture verifies cancellation packet length, type, backend PID and secret key. It counts those packets independently, counts normal startup sessions exactly, and surfaces fixture-worker panics through Drop. The read assertion now requires exactly one session and exactly one additional cancellation. Foreign startup remains a failure; no endpoint relocation or arbitrary-packet filtering was introduced. The fixture is loopback test infrastructure, not a hardened PostgreSQL server.

## 2. Actual change shape

 adapters/sql/tests/local_runtime.rs | 131 ++++++++++++++++++++++++++++++++----
 1 file changed, 119 insertions(+), 12 deletions(-)

## 3. Red before implementation

All test commands ran inside the assigned worktree, with `CARGO_BUILD_JOBS=2`, `RUSTC_WRAPPER=/usr/bin/sccache`, and Cargo's x86_64-unknown-linux-gnu runner setting only test processes' TMPDIR to this tree's `.local/tmp`. Target output stayed in this tree's `target/`. No CARGO_TARGET_DIR was set.

Command: `cargo test -p connectors-sql --test local_runtime sql_fixture -- --nocapture` (with the runner/environment above). Exit 101. Initial unexpected-startup reproduction:

```text
   Compiling connectors-sql v0.24.0 ($HOME/.local/state/worktree/trees/b10x/connectors/cb26b-sql/adapters/sql)
    Finished `test` profile [optimized] target(s) in 1.56s
     Running tests/local_runtime.rs (target/debug/deps/local_runtime-b43f87152464020b)

running 2 tests

thread '<unnamed>' (894142) panicked at adapters/sql/tests/local_runtime.rs:83:21:
assertion failed: startup.windows(7).any(|w| w == b"reader\0")
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace

thread 'sql_fixture_isolates_unrelated_connection' (894140) panicked at adapters/sql/tests/local_runtime.rs:277:5:
assertion `left == right` failed: unrelated traffic became a SUT session
  left: 1
 right: 0
test sql_fixture_isolates_unrelated_connection ... FAILED

thread '<unnamed>' (894143) panicked at adapters/sql/tests/local_runtime.rs:83:21:
assertion failed: startup.windows(7).any(|w| w == b"reader\0")
test sql_fixture_rejects_an_extra_sut_session - should panic ... FAILED

failures:

---- sql_fixture_rejects_an_extra_sut_session stdout ----
note: test did not panic as expected at adapters/sql/tests/local_runtime.rs:283:4

failures:
    sql_fixture_isolates_unrelated_connection
    sql_fixture_rejects_an_extra_sut_session

test result: FAILED. 0 passed; 2 failed; 0 ignored; 0 measured; 5 filtered out; finished in 0.49s

error: test failed, to rerun pass `-p connectors-sql --test local_runtime`
```

Same command, after changing the probe to the observed CancelRequest and adding malformed/startup controls, still before implementation; exit 101. The positive case was subsequently renamed from the disproven unrelated-traffic hypothesis to `sql_fixture_counts_cancel_requests_separately` with coordinator approval.

```text
   Compiling connectors-sql v0.24.0 ($HOME/.local/state/worktree/trees/b10x/connectors/cb26b-sql/adapters/sql)
    Finished `test` profile [optimized] target(s) in 3.62s
     Running tests/local_runtime.rs (target/debug/deps/local_runtime-b43f87152464020b)

running 4 tests

thread '<unnamed>' (930399) panicked at adapters/sql/tests/local_runtime.rs:85:21:
unexpected startup: [0, 3, 0, 0, 117, 115, 101, 114, 0, 115, 116, 114, 97, 121, 0, 100, 97, 116, 97, 98, 97, 115, 101, 0, 117, 110, 114, 101, 108, 97, 116, 101, 100, 0, 0]
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace

thread '<unnamed>' (930401) panicked at adapters/sql/tests/local_runtime.rs:85:21:
unexpected startup: [4, 210, 22, 46, 0, 0, 0, 42, 0, 0, 4, 210]

thread '<unnamed>' (930400) panicked at adapters/sql/tests/local_runtime.rs:85:21:
unexpected startup: [4, 210, 22, 46, 0, 0, 0, 42, 0, 0, 4, 211]

thread 'sql_fixture_isolates_unrelated_connection' (930394) panicked at adapters/sql/tests/local_runtime.rs:281:5:
assertion `left == right` failed: control traffic became a SUT session
  left: 1
 right: 0
test sql_fixture_rejects_a_malformed_cancellation - should panic ... FAILED
test sql_fixture_rejects_an_unexpected_startup - should panic ... FAILED
test sql_fixture_isolates_unrelated_connection ... FAILED

thread '<unnamed>' (930402) panicked at adapters/sql/tests/local_runtime.rs:85:21:
unexpected startup: [4, 210, 22, 46, 0, 0, 0, 42, 0, 0, 4, 210]

thread 'sql_fixture_rejects_an_extra_sut_session' (930396) panicked at adapters/sql/tests/local_runtime.rs:433:5:
assertion `left == right` failed: an invalid query opened a session
  left: 3
 right: 2
test sql_fixture_rejects_an_extra_sut_session - should panic ... FAILED

failures:

---- sql_fixture_rejects_a_malformed_cancellation stdout ----
note: test did not panic as expected at adapters/sql/tests/local_runtime.rs:296:4
---- sql_fixture_rejects_an_unexpected_startup stdout ----
note: test did not panic as expected at adapters/sql/tests/local_runtime.rs:288:4
---- sql_fixture_rejects_an_extra_sut_session stdout ----
note: panic did not contain expected string
      panic message: "assertion `left == right` failed: an invalid query opened a session\n  left: 3\n right: 2"
 expected substring: "the read must open exactly one session"

failures:
    sql_fixture_isolates_unrelated_connection
    sql_fixture_rejects_a_malformed_cancellation
    sql_fixture_rejects_an_extra_sut_session
    sql_fixture_rejects_an_unexpected_startup

test result: FAILED. 0 passed; 4 failed; 0 ignored; 0 measured; 5 filtered out; finished in 0.68s

error: test failed, to rerun pass `-p connectors-sql --test local_runtime`
```

## 4. Green package run and verification

Command: `cargo test -p connectors-sql` with the runner/environment above and pinned CONNECTORS_ESS/CONNECTORS_AEP from the brief. Exit 0.

```text
   Compiling connectors-sql v0.24.0 ($HOME/.local/state/worktree/trees/b10x/connectors/cb26b-sql/adapters/sql)
    Finished `test` profile [optimized] target(s) in 1.99s
     Running unittests src/lib.rs (target/debug/deps/connectors_sql-7f7352538509c72d)

running 2 tests
test error_tests::the_databases_statement_timeout_and_capacity_answers_are_marked_and_own_deadlines_are_not ... ok
test tls_tests::configured_ca_replaces_public_roots_and_requires_certificates ... ok

test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s

     Running unittests src/main.rs (target/debug/deps/connectors_sql-44f221ff0550c362)

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests/local_runtime.rs (target/debug/deps/local_runtime-b43f87152464020b)

running 9 tests
test cli_journey::a_real_postgres_session_persists_across_cli_and_owner_restart ... ignored, requires CONNECTORS_PG_SANDBOX, a built production CLI and qualified disposable Secret Service
test sql_fixture_rejects_an_unexpected_startup - should panic ... ok
test sql_fixture_rejects_a_malformed_cancellation - should panic ... ok
test a_rejected_password_and_a_malformed_entry_are_distinguishable ... ok
test a_dispatched_read_reaches_the_database_and_returns_its_refusal ... ok
test sql_fixture_counts_cancel_requests_separately ... ok
test the_session_is_the_credential_check_and_its_identity_is_the_role_and_database ... ok
test bootstrap_mismatches_and_changed_artifacts_refuse_before_any_session ... ok
test sql_fixture_rejects_an_extra_sut_session - should panic ... ok

test result: ok. 8 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 0.81s

     Running tests/protocol.rs (target/debug/deps/protocol-86f9b7cdcec5ad66)

running 7 tests
test parsing_and_limits_refuse_before_credentials_or_connection ... ok
test dropping_the_invocation_still_cancels_the_database ... ok
test text_null_and_empty_parameters_reach_bind_without_interpolation ... ok
test rows_nulls_native_types_and_truncation_are_preserved ... ok
test authentication_database_errors_and_row_capacity_are_sanitized ... ok
test unresponsive_database_cannot_extend_the_cleanup_budget ... ok
test deadline_sends_the_backend_cancel_key_and_drains_the_connection ... ok

test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 10.11s

   Doc-tests connectors_sql

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

```

Counts come from the runner summaries in `baseline.log` and `green-package.log`:

- library unit tests: executed 2 → 2, exit 0.
- binary unit tests: executed 0 → 0, exit 0.
- local_runtime: executed 4 → 8, exit 0; 1 existing ignored live-database journey unchanged.
- protocol: executed 7 → 7, exit 0.
- doctests: executed 0 → 0, exit 0.
- package aggregate: executed 13 → 17, exit 0.

Baseline package build/test elapsed 163 seconds, target initially absent and 889092 KiB afterward. Free disk before build was 26 GiB. Baseline build and test measurement is in `baseline-time.txt`.

Formatter: `cargo fmt -p connectors-sql --check`, exit 0. Linter: `CARGO_BUILD_JOBS=2 RUSTC_WRAPPER=/usr/bin/sccache cargo clippy -p connectors-sql --all-targets -- -D warnings`, exit 0. Full linter output is `clippy.log`; its final line is:

```text
    Finished `dev` profile [optimized + debuginfo] target(s) in 3m 37s
```

To prove the negative controls really fail, temporarily removed only their three `#[should_panic]` attributes, then ran `cargo test -p connectors-sql --test local_runtime sql_fixture_rejects -- --nocapture` with the same environment/runner. Exit 101, 0 passed / 3 failed. Restored the file byte-for-byte from `final-local-runtime.rs` and reran the entire local_runtime suite, exit 0 (8 passed, 1 ignored), then formatter exit 0. Full restored result is `restored-green.log`. The three actual failures:

```text
   Compiling connectors-sql v0.24.0 ($HOME/.local/state/worktree/trees/b10x/connectors/cb26b-sql/adapters/sql)
    Finished `test` profile [optimized] target(s) in 5.76s
     Running tests/local_runtime.rs (target/debug/deps/local_runtime-b43f87152464020b)

running 3 tests

thread '<unnamed>' (1067581) panicked at adapters/sql/tests/local_runtime.rs:90:25:
assertion `left == right` failed: invalid backend key
  left: [0, 0, 4, 211]
 right: [0, 0, 4, 210]
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace

thread 'sql_fixture_rejects_a_malformed_cancellation' (1067540) panicked at adapters/sql/tests/local_runtime.rs:263:20:
fixture worker failed: Any { .. }

thread '<unnamed>' (1067582) panicked at adapters/sql/tests/local_runtime.rs:104:21:
assertion failed: startup.windows(7).any(|w| w == b"reader\0")

thread 'sql_fixture_rejects_an_unexpected_startup' (1067542) panicked at adapters/sql/tests/local_runtime.rs:263:20:
fixture worker failed: Any { .. }
test sql_fixture_rejects_a_malformed_cancellation ... FAILED
test sql_fixture_rejects_an_unexpected_startup ... FAILED

thread 'sql_fixture_rejects_an_extra_sut_session' (1067541) panicked at adapters/sql/tests/local_runtime.rs:445:5:
assertion `left == right` failed: the read must open exactly one session
  left: 2
 right: 1
test sql_fixture_rejects_an_extra_sut_session ... FAILED

failures:

failures:
    sql_fixture_rejects_a_malformed_cancellation
    sql_fixture_rejects_an_extra_sut_session
    sql_fixture_rejects_an_unexpected_startup

test result: FAILED. 0 passed; 3 failed; 0 ignored; 0 measured; 6 filtered out; finished in 0.84s

error: test failed, to rerun pass `-p connectors-sql --test local_runtime`
```

Required 50 repetitions under parallel workspace test execution are NOT complete. Coordinator explicitly took ownership of this remaining verification to free the implementation slot for independent review. Do not mark the story complete from this partial handoff.

The restored final test binary is `target/debug/deps/local_runtime-b43f87152464020b`. Each unfiltered execution currently reports `test result: ok. 8 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out`. It executes both `sql_fixture_counts_cancel_requests_separately` and `a_dispatched_read_reaches_the_database_and_returns_its_refusal`, plus the three negative controls. Run this binary 50 times with `TMPDIR` set to this worktree's `.local/tmp`, recording each exit, runner summary and UTC overlap timings. Running it directly uses the already tested build and avoids Cargo locking or additional compilation. Do not use the temporary unguarded-negative binary; the subsequent restored-green run rebuilt the final source.

Final source SHA256: `2090429b13dcd83f006efd94b3e34415bea131563ddc579b89e41d2ee5619676`, also verified equal to the scratch snapshot `final-local-runtime.rs`. Final target allocation: 1507964 KiB. `git diff --check` exits 0.

## 5. Deliberately not done and handoff

- No runtime code, shared gate, live CLI journey, generated output, AEP file, commit or external integration effect changed; assignment owns only the SQL fixture source.
- No arbitrary traffic is discarded. The unrelated-traffic explanation was disproven as the necessary mechanism for this reproduction, and foreign startup remains a failure. Coordinator owns the accepted case-name/acceptance refinement in AEP.
- The existing ignored real-PostgreSQL/Secret Service journey was not run: it is outside this loopback-fixture assignment and its prerequisites are unchanged.
- The full-workspace load and 50 repetitions are pending coordinator execution; this report claims only the observed package, formatter, linter and negative-control results.
- Tree remains active and source uncommitted, as instructed. Coordinator owns review, integration, publication and cleanup. No build cleanup or worktree removal performed.
- Own lease `codex-cb26b-sql` released at handoff. Tree id `cb26b-sql`, branch `impl/cb26b-sql`, base `b3ddded7618b5a4a23fe74dbec739e0938ecc296`.

## 6. Outside-worktree writes

None by this unit. Logs, probes, snapshots, graph output and this report all reside under the assigned `.local/wave-20261002b/`; test temporary files used this tree's `.local/tmp`. Only worktree CLI-managed lease state and normal compiler/tool caches were updated outside the tree, as permitted by the brief. No scratch source, log or patch was written outside the assigned worktree.
