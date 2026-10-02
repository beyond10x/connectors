---
format: aep.planning-md/3
id: review-result:wave-20261002b-sql-adversary-1
kind: review-result
status: active
title: SQL cancellation fixture adversary, pass 1
relations:
- reviews: story:sql-fixture-accepts-stray-connections
revision: 1
---
unit: story:sql-fixture-accepts-stray-connections — dirty cb26b-sql at b3ddded7618b5a4a23fe74dbec739e0938ecc296
verdict: nothing found
cases: executed 17→19, red 0
origin: introduced 0 / pre-existing 0 / undecided 0
wrote-outside-worktree: none
needs-coordinator: 50 repetitions during overlapping workspace tests remain pending

## 1. Diff proof

`git --no-pager diff --stat` against HEAD (includes the implementor's supplied dirty change):

```text
 adapters/sql/tests/local_runtime.rs | 189 +++++++++++++++++++++++++++++++++---
 1 file changed, 177 insertions(+), 12 deletions(-)
```

Diff relative to the exact supplied source snapshot (`input.rs`, SHA256 2090429b13dcd83f006efd94b3e34415bea131563ddc579b89e41d2ee5619676):

```text
 .../sql/tests/local_runtime.rs                     | 58 ++++++++++++++++++++++
 1 file changed, 58 insertions(+)
```

Only two cases were added to adapters/sql/tests/local_runtime.rs. No existing case or fixture implementation was modified. The addition-only patch is retained as test-additions.patch. Final source SHA256: 094565444a701a196221cd902fc5ce50c447774ff31c1a6f1357b22e0ff5d23d.

## 2. Cases written before any test execution

At adapters/sql/tests/local_runtime.rs:318, sql_fixture_rejects_cancellation_length_and_pid_boundaries injects cancellation bodies of length 4, 8 and 13 and one wrong backend PID. Every input must leave both counters at zero and propagate the fixture worker's rejection through Drop. This tests the acceptance's malformed-cancellation negative control; it does not claim the actual adapter emits malformed cancellation.

At adapters/sql/tests/local_runtime.rs:341, sql_fixture_repeated_control_packets_preserve_provider_refusal sends three valid cancellation controls, then invokes the actual adapter. It requires the exact Forbidden result, exactly one session and four cancellation packets, and the actual expected password. The actual read/refusal cleanup reaches this control connection through adapters/sql/src/lib.rs query supervision and Connection::close. The repeated controls are deliberately injected to test accounting rather than claimed as normal single-invocation behavior.

Both cases passed on their first execution; no red case or defect was produced. Exact first runs follow. All Cargo test commands used CARGO_BUILD_JOBS=2, RUSTC_WRAPPER=/usr/bin/sccache and CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_RUNNER='env TMPDIR=$HOME/.local/state/worktree/trees/b10x/connectors/cb26b-sql/.local/tmp'. No CARGO_TARGET_DIR override. Each new case ran alone before the package suite.

Command: cargo test -p connectors-sql --test local_runtime sql_fixture_rejects_cancellation_length_and_pid_boundaries -- --exact --nocapture
Exit: 0. The expected caught panics below are passing negative controls, not red findings.

```text
   Compiling connectors-sql v0.24.0 ($HOME/.local/state/worktree/trees/b10x/connectors/cb26b-sql/adapters/sql)
    Finished `test` profile [optimized] target(s) in 4.64s
     Running tests/local_runtime.rs (target/debug/deps/local_runtime-b43f87152464020b)

running 1 test

thread '<unnamed>' (1252225) panicked at adapters/sql/tests/local_runtime.rs:88:25:
assertion `left == right` failed: invalid cancellation length
  left: 4
 right: 12
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace

thread 'sql_fixture_rejects_cancellation_length_and_pid_boundaries' (1252221) panicked at adapters/sql/tests/local_runtime.rs:263:20:
fixture worker failed: Any { .. }

thread '<unnamed>' (1252227) panicked at adapters/sql/tests/local_runtime.rs:88:25:
assertion `left == right` failed: invalid cancellation length
  left: 8
 right: 12

thread 'sql_fixture_rejects_cancellation_length_and_pid_boundaries' (1252221) panicked at adapters/sql/tests/local_runtime.rs:263:20:
fixture worker failed: Any { .. }

thread '<unnamed>' (1252228) panicked at adapters/sql/tests/local_runtime.rs:89:25:
assertion `left == right` failed: invalid backend pid
  left: [0, 0, 0, 43]
 right: [0, 0, 0, 42]

thread 'sql_fixture_rejects_cancellation_length_and_pid_boundaries' (1252221) panicked at adapters/sql/tests/local_runtime.rs:263:20:
fixture worker failed: Any { .. }

thread '<unnamed>' (1252245) panicked at adapters/sql/tests/local_runtime.rs:88:25:
assertion `left == right` failed: invalid cancellation length
  left: 13
 right: 12

thread 'sql_fixture_rejects_cancellation_length_and_pid_boundaries' (1252221) panicked at adapters/sql/tests/local_runtime.rs:263:20:
fixture worker failed: Any { .. }
test sql_fixture_rejects_cancellation_length_and_pid_boundaries ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 10 filtered out; finished in 0.03s

```

Command: cargo test -p connectors-sql --test local_runtime sql_fixture_repeated_control_packets_preserve_provider_refusal -- --exact --nocapture
Exit: 0.

```text
    Finished `test` profile [optimized] target(s) in 0.89s
     Running tests/local_runtime.rs (target/debug/deps/local_runtime-b43f87152464020b)

running 1 test
test sql_fixture_repeated_control_packets_preserve_provider_refusal ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 10 filtered out; finished in 0.52s

```

## 3. Package suite after the cases existed and ran alone

Command: cargo test -p connectors-sql
Exit: 0.

```text
    Finished `test` profile [optimized] target(s) in 0.62s
     Running unittests src/lib.rs (target/debug/deps/connectors_sql-7f7352538509c72d)

running 2 tests
test error_tests::the_databases_statement_timeout_and_capacity_answers_are_marked_and_own_deadlines_are_not ... ok
test tls_tests::configured_ca_replaces_public_roots_and_requires_certificates ... ok

test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running unittests src/main.rs (target/debug/deps/connectors_sql-44f221ff0550c362)

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests/local_runtime.rs (target/debug/deps/local_runtime-b43f87152464020b)

running 11 tests
test cli_journey::a_real_postgres_session_persists_across_cli_and_owner_restart ... ignored, requires CONNECTORS_PG_SANDBOX, a built production CLI and qualified disposable Secret Service
test sql_fixture_rejects_an_unexpected_startup - should panic ... ok
test sql_fixture_rejects_a_malformed_cancellation - should panic ... ok
test sql_fixture_rejects_cancellation_length_and_pid_boundaries ... ok
test sql_fixture_repeated_control_packets_preserve_provider_refusal ... ok
test a_rejected_password_and_a_malformed_entry_are_distinguishable ... ok
test a_dispatched_read_reaches_the_database_and_returns_its_refusal ... ok
test the_session_is_the_credential_check_and_its_identity_is_the_role_and_database ... ok
test sql_fixture_counts_cancel_requests_separately ... ok
test bootstrap_mismatches_and_changed_artifacts_refuse_before_any_session ... ok
test sql_fixture_rejects_an_extra_sut_session - should panic ... ok

test result: ok. 10 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 1.05s

     Running tests/protocol.rs (target/debug/deps/protocol-86f9b7cdcec5ad66)

running 7 tests
test parsing_and_limits_refuse_before_credentials_or_connection ... ok
test dropping_the_invocation_still_cancels_the_database ... ok
test text_null_and_empty_parameters_reach_bind_without_interpolation ... ok
test rows_nulls_native_types_and_truncation_are_preserved ... ok
test authentication_database_errors_and_row_capacity_are_sanitized ... ok
test unresponsive_database_cannot_extend_the_cleanup_budget ... ok
test deadline_sends_the_backend_cancel_key_and_drains_the_connection ... ok

test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 10.10s

   Doc-tests connectors_sql

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

```

The before count 17 is the implementor's declared package count; no suite ran before the new cases were written. After: library 2, binary 0, local_runtime 10, protocol 7, doctests 0 = 19 executed, with one live-provider case still ignored. A subsequent formatter check identified layout in two added assertions only; those new lines were formatted and cargo fmt -p connectors-sql --check plus git diff --check then passed. No behavior changed after the suite. Clippy checked final formatted source:

Command: CARGO_BUILD_JOBS=2 RUSTC_WRAPPER=/usr/bin/sccache cargo clippy -p connectors-sql --all-targets -- -D warnings
Exit: 0.

```text
    Blocking waiting for file lock on package cache
    Blocking waiting for file lock on package cache
    Checking connectors-sql v0.24.0 ($HOME/.local/state/worktree/trees/b10x/connectors/cb26b-sql/adapters/sql)
    Finished `dev` profile [optimized + debuginfo] target(s) in 3.14s
```

## 4. Findings

Nothing found in this bounded attack. No approval or independent-verification claim. The 50-repeat acceptance requirement has not been measured by this pass and remains the coordinator's responsibility.

## 5. Attacks that did not break the change

Malformed cancellation body length and backend PID rejected without changing counters.
Repeated valid cancellation controls did not inflate session counts or hide the actual adapter's refusal.
Actual provider password/session/refusal behavior survived the control traffic, and existing extra-session/startup/key negative controls passed in the package suite.

## 6. Outside-worktree writes and handoff

None, excluding the worktree tool's managed lease state and ordinary shared compiler cache activity explicitly allowed by the brief. Source additions, snapshot, patch, logs and report are inside the assigned tree/scratch. Target output remained tree-local; no output was removed. Free disk was 20 GiB before the bounded package work. Own lease codex-cb26b-sql-adversary is released at report handoff; the coordinator owns the active tree and cleanup.

```findings
[]
```
