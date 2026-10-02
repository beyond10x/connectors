unit: story:postgres-real-provider-acceptance + story:postgres-unsupported-feature-classification; cb26d-pg at f3fb222b7edc7fc29520dd30effdb58bfdec5274 plus supplied frozen five-file patch c583e40f88483081519eb9c192c159807341b0030f38c227b3ef027f585a9665
verdict: nothing found
cases: executed 19→20 ordinary, red 0; six author-run live cases retained, not rerun
origin: introduced 0 / pre-existing 0 / undecided 0
wrote-outside-worktree: task-assigned TMPDIR/cache and own worktree lease metadata only
needs-coordinator: retain or integrate test-only delta; root owns AEP/Git/cleanup
 .../sql/tests/protocol.rs                          | 26 ++++++++++++++++++++++
 1 file changed, 26 insertions(+)

The stat immediately above is this review's delta against the frozen author source: only adapters/sql/tests/protocol.rs changed, adding 26 lines. protocol-before.rs is the exact saved author file. The inherited five-file subject includes production/docs/contract changes; those were not written by this reviewer. test-addition.patch SHA256 fd85498bbb888c8688280ea1d261a01c9be9dc0b843e2300f7b0b1032a295033 records the review-only patch. No implementation, contract, guide, planning-store or Git lifecycle changes were made.

## 1. Subject and read order

Read the complete supplied five-file diff and latest commit, then active acceptance story revision 10 and exact-mapping story revision 9 from root's planning tree, the author report, all added tests, native read/cancellation contract, and the changed mapper's callers before constructing a case. Verified initial diff SHA256 c583e40f88483081519eb9c192c159807341b0030f38c227b3ef027f585a9665 and author report SHA256 5a674325a4631a77577e7f4538decd33879d0fbe412d43f8348f27ccda05f60b. Both author source and executable/lock checksum manifests verified before review mutation.

The only production change is exact Some("0A000") => Unsupported in adapters/sql/src/lib.rs:411. database_error at :406 is reached from startup and transaction/prepare/portal/rollback paths; the existing sanitized message remains shared. runtime Failure::from_service maps Unsupported unchanged (crates/connectors-host/src/local/runtime.rs:207–211), and owner conversion preserves that public code. No deadlines, transaction setup, credential binding or upstream_answer rule changes.

## 2. Added attack case, before execution

Added adversary_only_exact_feature_not_supported_changes_classification at adapters/sql/tests/protocol.rs:357. Through public Sql::invoke and the existing actual PostgreSQL wire fixture it sends exact 0A000 and neighboring custom 0A001/0A999, requiring Unsupported only for 0A000 and unchanged Unavailable for the neighbors. All three require upstream_answer=false and sanitized output without provider details/password. This tests the accepted exact-code-only boundary, not a claim that official PostgreSQL currently emits those custom neighboring codes in this business workflow. No production mutation was made and no suite pre-run occurred.

The first focused execution was green. There is no red finding to report. A future accidental starts_with("0A") broadening would fail the neighbor assertions, but this review did not run such a mutant and does not claim measured mutation evidence.

## 3. Actual execution, in order

The case existed before the first command below. Catalog live activity had released the serialized window; root explicitly granted this fixture-only build/test window. All commands used the PG tree's default target, CARGO_BUILD_JOBS=2, RUSTC_WRAPPER=/usr/bin/sccache and TMPDIR=$HOME/.cache/c26d/pg; no CARGO_TARGET_DIR. No Docker, PostgreSQL business call or other live integration was invoked by this review.

Focused command, exit 0:
`cargo test --locked --offline --release -p connectors-sql --test protocol adversary_only_exact_feature_not_supported_changes_classification -- --exact --nocapture`

```text
   Compiling connectors-sql v0.25.0 ($HOME/.local/state/worktree/trees/b10x/connectors/cb26d-pg/adapters/sql)
    Finished `release` profile [optimized] target(s) in 1.81s
     Running tests/protocol.rs (target/release/deps/protocol-ac09d42148fc3a16)

running 1 test
test adversary_only_exact_feature_not_supported_changes_classification ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 7 filtered out; finished in 0.62s

```

Only after that focused result, package command, exit 0:
`cargo test --locked --offline --release -p connectors-sql`

```text
    Finished `release` profile [optimized] target(s) in 0.28s
     Running unittests src/lib.rs (target/release/deps/connectors_sql-ae8de4e0264c5706)

running 2 tests
test error_tests::the_databases_statement_timeout_and_capacity_answers_are_marked_and_own_deadlines_are_not ... ok
test tls_tests::configured_ca_replaces_public_roots_and_requires_certificates ... ok

test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running unittests src/main.rs (target/release/deps/connectors_sql-79e02e23017a24d4)

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests/local_runtime.rs (target/release/deps/local_runtime-a24700cdb1049868)

running 16 tests
test cli_journey::a_real_postgres_session_persists_across_cli_and_owner_restart ... ignored, requires CONNECTORS_PG_SANDBOX, CONNECTORS_PG_CONTAINER, a built production CLI and qualified disposable Secret Service
test cli_journey::postgres_cli_cannot_escape_read_only_transaction ... ignored, requires CONNECTORS_PG_SANDBOX, CONNECTORS_PG_CONTAINER, built CONNECTORS_TEST_CLI and qualified disposable Secret Service
test cli_journey::postgres_cli_distinguishes_empty_truncated_capacity_and_timeout ... ignored, requires CONNECTORS_PG_SANDBOX, CONNECTORS_PG_CONTAINER, built CONNECTORS_TEST_CLI and qualified disposable Secret Service
test cli_journey::postgres_cli_preserves_join_group_utc_and_quoted_parameters ... ignored, requires CONNECTORS_PG_SANDBOX, CONNECTORS_PG_CONTAINER, built CONNECTORS_TEST_CLI and qualified disposable Secret Service
test cli_journey::postgres_dropped_invocation_cancels_its_backend ... ignored, requires CONNECTORS_PG_SANDBOX and CONNECTORS_PG_CONTAINER for owned PostgreSQL observation
test cli_journey::postgres_repair_revoke_and_busy_stop_preserve_authority ... ignored, requires CONNECTORS_PG_SANDBOX, CONNECTORS_PG_CONTAINER, built CONNECTORS_TEST_CLI and qualified disposable Secret Service
test sql_fixture_rejects_a_malformed_cancellation - should panic ... ok
test sql_fixture_rejects_an_unexpected_startup - should panic ... ok
test sql_fixture_rejects_cancellation_length_and_pid_boundaries ... ok
test a_rejected_password_and_a_malformed_entry_are_distinguishable ... ok
test sql_fixture_repeated_control_packets_preserve_provider_refusal ... ok
test a_dispatched_read_reaches_the_database_and_returns_its_refusal ... ok
test sql_fixture_counts_cancel_requests_separately ... ok
test the_session_is_the_credential_check_and_its_identity_is_the_role_and_database ... ok
test bootstrap_mismatches_and_changed_artifacts_refuse_before_any_session ... ok
test sql_fixture_rejects_an_extra_sut_session - should panic ... ok

test result: ok. 10 passed; 0 failed; 6 ignored; 0 measured; 0 filtered out; finished in 0.51s

     Running tests/protocol.rs (target/release/deps/protocol-ac09d42148fc3a16)

running 8 tests
test parsing_and_limits_refuse_before_credentials_or_connection ... ok
test dropping_the_invocation_still_cancels_the_database ... ok
test text_null_and_empty_parameters_reach_bind_without_interpolation ... ok
test adversary_only_exact_feature_not_supported_changes_classification ... ok
test rows_nulls_native_types_and_truncation_are_preserved ... ok
test authentication_database_errors_and_row_capacity_are_sanitized ... ok
test unresponsive_database_cannot_extend_the_cleanup_budget ... ok
test deadline_sends_the_backend_cancel_key_and_drains_the_connection ... ok

test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 10.08s

   Doc-tests connectors_sql

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

```

Ordinary package executed 19→20, exit 0→0; protocol target 7→8; ignored live count remains 6. Before counts come from the supplied author's final runner summaries, not a preemptive review suite. The newly selected case executed 1, not zero. Six author-run real cases remain distinct reused evidence: their final live output reports 6 passed, 0 failed in 58.17s. Review did not rerun them because the only review source change is a separate protocol test, and production/native live test/executable identities remain byte-identical.

`cargo fmt -p connectors-sql --check`, exit 0, emitted no output. `cargo clippy --locked --offline --release -p connectors-sql --all-targets -- -D warnings`, exit 0:

```text
    Checking connectors-sql v0.25.0 ($HOME/.local/state/worktree/trees/b10x/connectors/cb26d-pg/adapters/sql)
    Finished `release` profile [optimized] target(s) in 1.31s
```

## 4. Findings

Nothing found. No confirmed defect, unmet acceptance criterion or introduced contract mismatch was established in this bounded pass. This is not production approval or a claim that this review reran provider acceptance.

## 5. Attacks and limits

- Exact mapping/class boundary: the new public wire case passes for 0A000 versus custom neighbors; author matrix still covers prior SQLSTATE classifications and upstream_answer distinctions (adapters/sql/tests/protocol.rs:385 onward after insertion).
- Nonempty query behavior: exact join/group count, UTC boundaries, native column types and quote-as-data assertions at adapters/sql/tests/local_runtime/cli_journey.rs:671–706 would fail for default/empty results. Empty schema, truncation, capacity, timeout and transaction settings at :710–751 are separately asserted.
- Write protection: :756–807 requires specific refusal categories, independent admin snapshots after each attempt, exact reader/session role and read-only setting. Author retained a real misclassification red and corresponding exact wire red before changing only the mapper; arbitrary transport failures are not accepted as write protection.
- Native drop: :875–952 starts the real Sql invocation, observes before the strict two-second cutoff, aborts and awaits the invocation task, then requires marked execution gone before five seconds. The paired no-drop control remains running after five seconds before explicit exact-marker cleanup. :825–853 correlates one active reader backend by unique query marker, retains PID and prints the actual query/backend start. Recorded final observations are 44ms/42ms initial, 5045ms still-active control, and 44ms after dropped invocation. CLI kill is not substituted for future drop. This is author-run real evidence checked against source, not new review execution or a universal remote-termination guarantee.
- Repair/lifecycle: :958–1083 exercises reachable configuration-role mismatch and failed-password repair, verifies original authority through an exact reader query, stops an observed busy backend via current owner/child coordinates, resumes explicitly and checks revoked marked query absence. The accepted repair clarification matches password-only native binding in adapters/sql/src/local.rs:159–167,207–212.
- Identity/evidence: after the package run, production source, live test source, native contract/guide, CLI/SQL executables and live acceptance executable still match author manifests. Only protocol.rs and its test executable changed. Author cleanup log records zero remaining acceptance schemas/backends and reader authority false:false:false. No cleanup/live state was independently queried during this review.
- No exhaustive mutation campaign, property search, network-failure cancellation guarantee, Rust 1.88 check, full workspace gate, release or unrelated provider coverage is claimed. No prior critic report was read.

## 6. Outside-worktree writes and handoff

Cargo used the assigned $HOME/.cache/c26d/pg temporary root, $HOME/.cargo tool-managed inputs and $HOME/.cache/sccache compiler cache. Own lease metadata is maintained by the worktree CLI under $HOME/.local/state/worktree. All explicit review evidence is under cb26d-pg/.local/provider-wave/postgres/adversary-k8s; build output remains in the tree's existing target. No other external writes, provider calls or process manipulation occurred. Root owns artifact recording, integration and cleanup.

publication-report.md contains the identical report with only the local home-directory prefix replaced by literal $HOME; this is a declared path-only publication redaction, with all results and findings preserved. Own lease release and final digests are retained beside the reports, outside the self-hashed report text.

```findings
[]
```
