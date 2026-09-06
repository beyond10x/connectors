---
format: aep.planning-md/1
id: review-result:cli-pipe-adversary-1-20260906
kind: review-result
status: active
title: Closed-pipe adversary pass 1
refs:
- provider: git
  reference: f4ff534681eae7c1f9989154491b2c4f8e616c7d
relations:
- reviews: story:emit-treats-a-closed-pipe-as-failure
revision: 1
---
unit: story:emit-treats-a-closed-pipe-as-failure at f4ff534681eae7c1f9989154491b2c4f8e616c7d plus retained tests
verdict: NEEDS-CHANGE
cases: executed 92→96, red 2
origin: introduced 0 / pre-existing 0 / undecided 2
wrote-outside-worktree: 175 retained files; transient fixtures under the assigned TMPDIR inventoried below
needs-coordinator: route two measured failures and classify origin without an assigned base execution

```text
 crates/connectors-cli/tests/closed_pipe.rs | 211 +++++++++++++++++++++++++++++
 1 file changed, 211 insertions(+)
```

1. Scope proof

The diff above is `git --no-pager diff --stat` from `~/.local/state/worktree/trees/b10x/connectors/wt-55b6348b9982`, exit 0. Only the existing test file was extended; its prior 256 lines are unchanged. No production file, planning artifact, Git index/ref, operator configuration, credential store, daemon or live provider was mutated. No subagents were used. Read the full unit diff, the acceptance statement, prior tests and implementation report, run_from and its callers, all result-output call sites, MainError conversions, doctor report semantics, and hosted admin response decoding. The branch remains at the assigned commit. `git diff --check` exited 0.

2. Cases written before execution

All four cases and their test fixtures were written before the first test execution. Before-count 92 is from the implementor's reported full CLI run, not a pre-attack test execution.

- `crates/connectors-cli/tests/closed_pipe.rs:274`: an unhealthy doctor remains unsuccessful with a closed report reader. Red in every format. The same missing configuration exits 1 with an open reader and 0 with a closed reader.
- `crates/connectors-cli/tests/closed_pipe.rs:388`: a successful synthetic admin readiness response accepts a closed result reader. Red in every format. Open-reader controls exit 0 and contain the fixture integration; closed-reader JSON/YAML exit 101 and text/compact exit 1.
- `crates/connectors-cli/tests/closed_pipe.rs:416`: an actual stdin directory-read failure still exits nonzero when stdout is also closed. Green in every format.
- `crates/connectors-cli/tests/closed_pipe.rs:450`: an actual non-BrokenPipe completion write failure still exits nonzero. Green.

The test-only stdout peer is shut down before spawning the CLI and directly asserts a BrokenPipe error; this removes the race between a small output and the reader close. Existing large anonymous-pipe cases continue to exercise actual pipe output above 64 KiB. The admin server is a synthetic loopback HTTP peer; its only accepted request is the administrative GET with a fixture-only access token, and the response includes the required cache headers. No real service or token was used.

First isolated doctor execution:

Command: `env -u CARGO_TARGET_DIR TMPDIR=~/.cache/cw6/e RUSTC_WRAPPER=/usr/bin/sccache CARGO_INCREMENTAL=0 CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 CARGO_BUILD_JOBS=3 cargo test --manifest-path crates/connectors-cli/Cargo.toml --locked --test closed_pipe an_unhealthy_doctor_stays_unsuccessful_when_its_report_reader_closes -- --exact --nocapture`.

```text
warning: connector-secrets@0.6.5: no live Vault was offered (CONNECTOR_SECRETS_VAULT_ADDR and CONNECTOR_SECRETS_VAULT_TOKEN are unset), so the reqwest HttpTransport is UNEXERCISED by this build: tests/vault_live.rs is compiled #[ignore]d
   Compiling connectors-cli v0.6.5 (~/.local/state/worktree/trees/b10x/connectors/wt-55b6348b9982/crates/connectors-cli)
    Finished `test` profile [unoptimized] target(s) in 7.46s
     Running tests/closed_pipe.rs (crates/connectors-cli/target/debug/deps/closed_pipe-2a9d7878c97aca5d)

running 1 test
doctor json: open reader exit status: 1, closed reader exit status: 0
doctor yaml: open reader exit status: 1, closed reader exit status: 0
doctor text: open reader exit status: 1, closed reader exit status: 0
doctor compact: open reader exit status: 1, closed reader exit status: 0

thread 'an_unhealthy_doctor_stays_unsuccessful_when_its_report_reader_closes' (1259854) panicked at tests/closed_pipe.rs:299:5:
a closed report reader erased the unhealthy status in ["json", "yaml", "text", "compact"]
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace
test an_unhealthy_doctor_stays_unsuccessful_when_its_report_reader_closes ... FAILED

failures:

failures:
    an_unhealthy_doctor_stays_unsuccessful_when_its_report_reader_closes

test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 11 filtered out; finished in 0.05s

error: test failed, to rerun pass `--test closed_pipe`
```

Exit status: 101. Log: `~/.cache/connectors-cli-wave-20260906/emit-treats-a-closed-pipe-as-failure/adversary-1/doctor.log`.


The first admin fixture omitted the required cache-control headers. Its control failed before the output scenario. Adding Cache-Control alone was insufficient; the unchanged client also requires Pragma. These were fixture mistakes, not product findings. Their complete chronological outputs are retained:

```text
warning: connector-secrets@0.6.5: no live Vault was offered (CONNECTOR_SECRETS_VAULT_ADDR and CONNECTOR_SECRETS_VAULT_TOKEN are unset), so the reqwest HttpTransport is UNEXERCISED by this build: tests/vault_live.rs is compiled #[ignore]d
    Finished `test` profile [unoptimized] target(s) in 0.26s
     Running tests/closed_pipe.rs (crates/connectors-cli/target/debug/deps/closed_pipe-2a9d7878c97aca5d)

running 1 test

thread 'successful_admin_results_accept_a_closed_reader_in_every_format' (1262809) panicked at tests/closed_pipe.rs:139:5:
status: exit status: 1; stderr: 
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace
test successful_admin_results_accept_a_closed_reader_in_every_format ... FAILED

failures:

failures:
    successful_admin_results_accept_a_closed_reader_in_every_format

test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 11 filtered out; finished in 0.01s

error: test failed, to rerun pass `--test closed_pipe`
```

Exit status: 101. Log: `~/.cache/connectors-cli-wave-20260906/emit-treats-a-closed-pipe-as-failure/adversary-1/admin.log`.

```text
warning: connector-secrets@0.6.5: no live Vault was offered (CONNECTOR_SECRETS_VAULT_ADDR and CONNECTOR_SECRETS_VAULT_TOKEN are unset), so the reqwest HttpTransport is UNEXERCISED by this build: tests/vault_live.rs is compiled #[ignore]d
   Compiling connectors-cli v0.6.5 (~/.local/state/worktree/trees/b10x/connectors/wt-55b6348b9982/crates/connectors-cli)
    Finished `test` profile [unoptimized] target(s) in 0.45s
     Running tests/closed_pipe.rs (crates/connectors-cli/target/debug/deps/closed_pipe-2a9d7878c97aca5d)

running 1 test

thread 'successful_admin_results_accept_a_closed_reader_in_every_format' (1269109) panicked at tests/closed_pipe.rs:139:5:
status: exit status: 1; stderr: 
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace
test successful_admin_results_accept_a_closed_reader_in_every_format ... FAILED

failures:

failures:
    successful_admin_results_accept_a_closed_reader_in_every_format

test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 11 filtered out; finished in 0.02s

error: test failed, to rerun pass `--test closed_pipe`
```

Exit status: 101. Log: `~/.cache/connectors-cli-wave-20260906/emit-treats-a-closed-pipe-as-failure/adversary-1/admin-valid-fixture.log`.

```text
warning: connector-secrets@0.6.5: no live Vault was offered (CONNECTOR_SECRETS_VAULT_ADDR and CONNECTOR_SECRETS_VAULT_TOKEN are unset), so the reqwest HttpTransport is UNEXERCISED by this build: tests/vault_live.rs is compiled #[ignore]d
   Compiling connectors-cli v0.6.5 (~/.local/state/worktree/trees/b10x/connectors/wt-55b6348b9982/crates/connectors-cli)
    Finished `test` profile [unoptimized] target(s) in 0.44s
     Running tests/closed_pipe.rs (crates/connectors-cli/target/debug/deps/closed_pipe-2a9d7878c97aca5d)

running 1 test

thread 'successful_admin_results_accept_a_closed_reader_in_every_format' (1275867) panicked at tests/closed_pipe.rs:372:9:
admin control json: Output { status: ExitStatus(unix_wait_status(256)), stdout: "{\n  \"error\": {\n    \"code\": \"admin\",\n    \"message\": \"hosted Connector returned a cacheable credential response\"\n  }\n}\n", stderr: "" }
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace
test successful_admin_results_accept_a_closed_reader_in_every_format ... FAILED

failures:

failures:
    successful_admin_results_accept_a_closed_reader_in_every_format

test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 11 filtered out; finished in 0.01s

error: test failed, to rerun pass `--test closed_pipe`
```

Exit status: 101. Log: `~/.cache/connectors-cli-wave-20260906/emit-treats-a-closed-pipe-as-failure/adversary-1/admin-fixture-diagnostic.log`.


The corrected fixture retained all assertions and passed its open-reader controls. Its deciding execution, before any full suite:

Command: `env -u CARGO_TARGET_DIR TMPDIR=~/.cache/cw6/e RUSTC_WRAPPER=/usr/bin/sccache CARGO_INCREMENTAL=0 CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 CARGO_BUILD_JOBS=3 cargo test --manifest-path crates/connectors-cli/Cargo.toml --locked --test closed_pipe successful_admin_results_accept_a_closed_reader_in_every_format -- --exact --nocapture`.

```text
warning: connector-secrets@0.6.5: no live Vault was offered (CONNECTOR_SECRETS_VAULT_ADDR and CONNECTOR_SECRETS_VAULT_TOKEN are unset), so the reqwest HttpTransport is UNEXERCISED by this build: tests/vault_live.rs is compiled #[ignore]d
   Compiling connectors-cli v0.6.5 (~/.local/state/worktree/trees/b10x/connectors/wt-55b6348b9982/crates/connectors-cli)
    Finished `test` profile [unoptimized] target(s) in 0.55s
     Running tests/closed_pipe.rs (crates/connectors-cli/target/debug/deps/closed_pipe-2a9d7878c97aca5d)

running 1 test
admin json: open reader exit status: 0, closed reader exit status: 101; stderr: 
thread 'main' (1278546) panicked at /rustc/88d9e12ae178fab0fb5cc050a94da85685d449ea/library/std/src/io/stdio.rs:1166:9:
failed printing to stdout: Broken pipe (os error 32)
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace

admin yaml: open reader exit status: 0, closed reader exit status: 101; stderr: 
thread 'main' (1278601) panicked at /rustc/88d9e12ae178fab0fb5cc050a94da85685d449ea/library/std/src/io/stdio.rs:1166:9:
failed printing to stdout: Broken pipe (os error 32)
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace

admin text: open reader exit status: 0, closed reader exit status: 1; stderr: admin: the result could not be written: Broken pipe (os error 32)

admin compact: open reader exit status: 0, closed reader exit status: 1; stderr: admin: the result could not be written: Broken pipe (os error 32)


thread 'successful_admin_results_accept_a_closed_reader_in_every_format' (1278519) panicked at tests/closed_pipe.rs:384:5:
successful admin output rejected a closed reader in ["json", "yaml", "text", "compact"]
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace
test successful_admin_results_accept_a_closed_reader_in_every_format ... FAILED

failures:

failures:
    successful_admin_results_accept_a_closed_reader_in_every_format

test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 11 filtered out; finished in 0.10s

error: test failed, to rerun pass `--test closed_pipe`
```

Exit status: 101. Log: `~/.cache/connectors-cli-wave-20260906/emit-treats-a-closed-pipe-as-failure/adversary-1/admin-correct-fixture.log`.


The two additional cases were each run alone before the suite:

Command: `env -u CARGO_TARGET_DIR TMPDIR=~/.cache/cw6/e RUSTC_WRAPPER=/usr/bin/sccache CARGO_INCREMENTAL=0 CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 CARGO_BUILD_JOBS=3 cargo test --manifest-path crates/connectors-cli/Cargo.toml --locked --test closed_pipe stdin_read_errors_stay_unsuccessful_with_a_closed_output_reader -- --exact --nocapture`.

```text
warning: connector-secrets@0.6.5: no live Vault was offered (CONNECTOR_SECRETS_VAULT_ADDR and CONNECTOR_SECRETS_VAULT_TOKEN are unset), so the reqwest HttpTransport is UNEXERCISED by this build: tests/vault_live.rs is compiled #[ignore]d
    Finished `test` profile [unoptimized] target(s) in 0.26s
     Running tests/closed_pipe.rs (crates/connectors-cli/target/debug/deps/closed_pipe-2a9d7878c97aca5d)

running 1 test
stdin failure json: exit status: 101
stdin failure yaml: exit status: 101
stdin failure text: exit status: 1
stdin failure compact: exit status: 1
test stdin_read_errors_stay_unsuccessful_with_a_closed_output_reader ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 11 filtered out; finished in 0.02s

```

Exit status: 0. Log: `~/.cache/connectors-cli-wave-20260906/emit-treats-a-closed-pipe-as-failure/adversary-1/stdin.log`.


Command: `env -u CARGO_TARGET_DIR TMPDIR=~/.cache/cw6/e RUSTC_WRAPPER=/usr/bin/sccache CARGO_INCREMENTAL=0 CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 CARGO_BUILD_JOBS=3 cargo test --manifest-path crates/connectors-cli/Cargo.toml --locked --test closed_pipe completion_scripts_keep_other_output_write_failures_unsuccessful -- --exact --nocapture`.

```text
warning: connector-secrets@0.6.5: no live Vault was offered (CONNECTOR_SECRETS_VAULT_ADDR and CONNECTOR_SECRETS_VAULT_TOKEN are unset), so the reqwest HttpTransport is UNEXERCISED by this build: tests/vault_live.rs is compiled #[ignore]d
    Finished `test` profile [unoptimized] target(s) in 0.25s
     Running tests/closed_pipe.rs (crates/connectors-cli/target/debug/deps/closed_pipe-2a9d7878c97aca5d)

running 1 test
test completion_scripts_keep_other_output_write_failures_unsuccessful ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 11 filtered out; finished in 0.01s

```

Exit status: 0. Log: `~/.cache/connectors-cli-wave-20260906/emit-treats-a-closed-pipe-as-failure/adversary-1/completion.log`.


3. Full CLI package runs and test hygiene

Command for both suite executions: `env -u CARGO_TARGET_DIR TMPDIR=~/.cache/cw6/e RUSTC_WRAPPER=/usr/bin/sccache CARGO_INCREMENTAL=0 CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 CARGO_BUILD_JOBS=3 cargo test --manifest-path crates/connectors-cli/Cargo.toml --locked --no-fail-fast`. The no-fail-fast option ensures later integration targets still run after a red target.

The first full run also exposed a race in the new stdout fixture: another concurrent subprocess fork can briefly inherit the peer descriptor until exec closes it, so dropping the original peer alone did not guarantee that the probe immediately returned BrokenPipe. This was a test-fixture failure, not a third product finding. Its full output is preserved:

```text
warning: connector-secrets@0.6.5: no live Vault was offered (CONNECTOR_SECRETS_VAULT_ADDR and CONNECTOR_SECRETS_VAULT_TOKEN are unset), so the reqwest HttpTransport is UNEXERCISED by this build: tests/vault_live.rs is compiled #[ignore]d
   Compiling connectors-cli v0.6.5 (~/.local/state/worktree/trees/b10x/connectors/wt-55b6348b9982/crates/connectors-cli)
    Finished `test` profile [unoptimized] target(s) in 4.07s
     Running unittests src/lib.rs (crates/connectors-cli/target/debug/deps/connectors_cli-57384b8f68913e31)

running 5 tests
test tests::grafana_connect_uses_the_same_guided_surface ... ok
test tests::kubernetes_connect_accepts_an_exact_context_selection ... ok
test tests::slack_connect_needs_no_internal_reference_or_path_argument ... ok
test tests::normal_help_exposes_the_guided_flow_and_hides_acquisition_plumbing ... ok
test tests::every_supported_shell_gets_a_script_naming_the_whole_surface ... ok

test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s

     Running unittests src/main.rs (crates/connectors-cli/target/debug/deps/connectors-345457defe935fe9)

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests/adversary_cli_cap_pass3.rs (crates/connectors-cli/target/debug/deps/adversary_cli_cap_pass3-70cb79e6da31936e)

running 1 test
test the_cap_the_design_page_says_is_measured_is_declared_and_asserted ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests/adversary_fence_probe.rs (crates/connectors-cli/target/debug/deps/adversary_fence_probe-6389f5548bc21248)

running 6 tests
test the_wire_name_rule_citation_in_the_design_document_points_at_the_rule ... ok
test the_wire_name_rule_citation_in_the_specification_points_at_the_rule ... ok
test the_typeable_words_are_the_words_the_design_document_names ... ok
test the_copies_this_probe_carries_are_still_copies ... ok
test a_forwarding_reason_that_names_no_command_is_refused_whatever_kind_it_carries ... ok
test every_entry_that_is_not_a_lifecycle_step_is_refused_when_it_claims_to_be_one ... ok

test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.12s

     Running tests/adversary_fence_probe_pass2.rs (crates/connectors-cli/target/debug/deps/adversary_fence_probe_pass2-1d36e62dc0fb94dd)

running 3 tests
test every_file_the_committed_contract_opens_is_a_file_a_clone_has ... ok
test the_kinds_the_design_document_says_rest_on_no_sentence_rest_on_no_sentence ... ok
test the_paths_the_design_document_says_send_no_protocol_request_send_none ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests/adversary_shim_pass3.rs (crates/connectors-cli/target/debug/deps/adversary_shim_pass3-a9d55d257d709356)

running 6 tests
test the_serve_group_advertises_a_help_subcommand ... ok
test a_help_path_of_the_new_tree_under_serve_is_left_alone ... ok
test connectors_help_still_answers_for_a_path_that_moved ... ok
test a_moved_path_typed_with_the_global_output_flag_still_works ... ok
test the_group_word_whose_only_command_moved_still_points_somewhere ... ok
test nothing_this_product_prints_names_a_moved_path_behind_a_global_flag ... ok

test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.12s

     Running tests/adversary_shim_pass4.rs (crates/connectors-cli/target/debug/deps/adversary_shim_pass4-9047fd66bf9f43ea)

running 3 tests
test the_table_this_suite_copies_by_hand_is_the_table_the_binary_ships ... ok
test the_auth_group_still_answers_the_help_subcommand_it_advertised ... ok
test a_two_word_path_that_moved_works_with_the_global_flag_between_its_words ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.05s

     Running tests/adversary_shim_pass5.rs (crates/connectors-cli/target/debug/deps/adversary_shim_pass5-fbcea861a3d6c3b8)

running 4 tests
test a_positional_value_spelled_help_is_a_value_not_a_help_request ... ok
test an_argument_neither_the_group_nor_the_leaf_declares_is_not_the_old_leaf ... ok
test the_double_dash_escape_is_not_a_word_that_moved ... ok
test serve_with_only_global_options_is_the_group_in_every_spelling_and_position ... ok

test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.06s

     Running tests/cli_surface.rs (crates/connectors-cli/target/debug/deps/cli_surface-8ea8fa8a79700c16)

running 34 tests
test every_kind_of_exception_is_used_and_every_entry_gives_a_reason ... ok
test every_named_exception_is_still_a_path_of_the_parser ... ok
test no_word_of_the_parser_answers_to_a_name_the_specification_cannot_declare ... ok
test every_declared_group_is_a_group_of_the_parser ... ok
test every_declared_group_help_line_is_the_summary_the_specification_declares ... ok
test every_path_of_the_parser_is_declared_or_a_named_exception ... ok
test no_path_is_both_declared_and_excepted ... ok
test every_declaration_the_adversary_probe_copies_is_still_a_copy ... ok
test the_committed_generated_tree_is_the_specification_word_for_word ... ok
test every_citation_this_unit_wrote_resolves ... ok
test the_old_login_selected_target_guard_is_absent ... ok
test the_read_verb_enumeration_partitions_the_protocols_it_names ... ok
test a_read_stops_being_an_exception_once_the_specification_declares_a_view ... ok
test a_command_absorbed_into_the_exception_list_alone_is_refused ... ok
test the_regeneration_command_the_documents_name_is_the_one_the_gate_runs ... ok
test every_citation_that_names_a_symbol_lands_on_its_declaration ... ok
test the_target_countdown_is_exactly_what_the_parser_still_owes ... ok
test the_parser_accepts_target_before_and_after_each_dual_target_leaf ... ok
test the_specification_names_the_binary_the_parser_builds ... ok
test the_kinds_the_tree_derives_are_the_kinds_the_list_carries ... ok
test the_exception_list_is_the_set_the_specification_enumerates ... ok
test targeted_errors_keep_the_target_in_yaml_and_text ... ok
test target_conflict_precedes_invoke_payload_loading ... ok
test target_conflict_does_not_wait_for_open_stdin ... ok
test an_explicit_hosted_target_requires_a_login_by_name_for_every_group ... ok
test an_omitted_target_ignores_a_saved_login_for_every_dual_target_group ... ok
test target_conflict_precedes_missing_or_malformed_inline_input ... ok
test an_exception_whose_kind_the_tree_contradicts_is_refused ... ok
test hosted_refuses_each_local_only_option_for_every_group ... ok
test selected_target_preserves_provider_owned_target_fields_in_every_renderer ... ok
test local_success_and_protocol_refusals_report_the_selected_target ... ok
test broken_explicit_hosted_selection_never_falls_back_to_a_local_listener ... ok
test all_target_conflicts_precede_local_and_hosted_state_access ... ok
test every_local_leaf_ignores_broken_login_metadata_and_preserves_its_request ... ok

test result: ok. 34 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.40s

     Running tests/cli_surface_drift.rs (crates/connectors-cli/target/debug/deps/cli_surface_drift-a9239116bea3e52d)

running 10 tests
test the_copied_declarations_are_still_copies ... ok
test the_thin_frontend_citation_points_at_the_thin_frontend_test ... ok
test a_committed_tree_whose_group_about_no_longer_matches_the_specification_is_refused ... ok
test a_command_added_under_a_declared_group_is_refused ... ok
test a_committed_tree_that_swaps_completions_for_an_undeclared_word_is_refused ... ok
test a_command_added_under_the_wrong_declared_group_is_refused ... ok
test cutting_the_admin_group_over_to_the_generated_tree_is_refused ... ok
test the_restated_contract_is_green_against_the_unchanged_tree ... ok
test a_target_flag_removed_from_a_group_is_refused_by_the_countdown ... ok
test cargo_can_read_the_committed_emitted_manifest ... ok

test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s

     Running tests/cli_surface_pass_two.rs (crates/connectors-cli/target/debug/deps/cli_surface_pass_two-0a78eebb598fc922)

running 6 tests
test the_design_document_names_only_constants_that_exist ... ok
test the_design_document_states_the_shape_of_the_exception_list ... ok
test the_design_document_describes_the_countdown_assertion_the_contract_makes ... ok
test the_target_countdown_candidates_are_derived_from_every_protocol_a_deployment_answers ... ok
test the_drift_suites_copies_are_checked_rather_than_cited ... ok
test the_drift_suite_attributes_nothing_to_the_contract_that_is_not_there ... ok

test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests/closed_pipe.rs (crates/connectors-cli/target/debug/deps/closed_pipe-2a9d7878c97aca5d)

running 12 tests
test stdin_read_errors_stay_unsuccessful_with_a_closed_output_reader ... FAILED
test completion_scripts_still_accept_a_closed_reader ... ok
test completion_scripts_keep_other_output_write_failures_unsuccessful ... ok
test a_closed_transport_stays_unsuccessful ... ok
test an_unhealthy_doctor_stays_unsuccessful_when_its_report_reader_closes ... FAILED
test compact_consumer_closes_early ... ok
test json_consumer_closes_early ... ok
test text_consumer_closes_early ... ok
test yaml_consumer_closes_early ... ok
test successful_admin_results_accept_a_closed_reader_in_every_format ... FAILED
test every_format_really_emits_more_than_a_64_kib_pipe_buffer ... ok
test a_real_non_broken_pipe_output_failure_stays_unsuccessful ... ok

failures:

---- stdin_read_errors_stay_unsuccessful_with_a_closed_output_reader stdout ----

thread 'stdin_read_errors_stay_unsuccessful_with_a_closed_output_reader' (1292881) panicked at tests/closed_pipe.rs:264:32:
called `Result::unwrap_err()` on an `Ok` value: 5
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace

---- an_unhealthy_doctor_stays_unsuccessful_when_its_report_reader_closes stdout ----
doctor json: open reader exit status: 1, closed reader exit status: 0
doctor yaml: open reader exit status: 1, closed reader exit status: 0
doctor text: open reader exit status: 1, closed reader exit status: 0
doctor compact: open reader exit status: 1, closed reader exit status: 0

thread 'an_unhealthy_doctor_stays_unsuccessful_when_its_report_reader_closes' (1292874) panicked at tests/closed_pipe.rs:299:5:
a closed report reader erased the unhealthy status in ["json", "yaml", "text", "compact"]

---- successful_admin_results_accept_a_closed_reader_in_every_format stdout ----
admin json: open reader exit status: 0, closed reader exit status: 101; stderr: 
thread 'main' (1293145) panicked at /rustc/88d9e12ae178fab0fb5cc050a94da85685d449ea/library/std/src/io/stdio.rs:1166:9:
failed printing to stdout: Broken pipe (os error 32)
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace

admin yaml: open reader exit status: 0, closed reader exit status: 101; stderr: 
thread 'main' (1293428) panicked at /rustc/88d9e12ae178fab0fb5cc050a94da85685d449ea/library/std/src/io/stdio.rs:1166:9:
failed printing to stdout: Broken pipe (os error 32)
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace

admin text: open reader exit status: 0, closed reader exit status: 1; stderr: admin: the result could not be written: Broken pipe (os error 32)

admin compact: open reader exit status: 0, closed reader exit status: 1; stderr: admin: the result could not be written: Broken pipe (os error 32)


thread 'successful_admin_results_accept_a_closed_reader_in_every_format' (1292884) panicked at tests/closed_pipe.rs:406:5:
successful admin output rejected a closed reader in ["json", "yaml", "text", "compact"]


failures:
    an_unhealthy_doctor_stays_unsuccessful_when_its_report_reader_closes
    stdin_read_errors_stay_unsuccessful_with_a_closed_output_reader
    successful_admin_results_accept_a_closed_reader_in_every_format

test result: FAILED. 9 passed; 3 failed; 0 ignored; 0 measured; 0 filtered out; finished in 4.58s

error: test failed, to rerun pass `--test closed_pipe`
     Running tests/first_level_groups.rs (crates/connectors-cli/target/debug/deps/first_level_groups-6a4964cb963d131a)

running 5 tests
test the_first_level_is_eight_words ... ok
test doctor_reports_the_same_installation_at_both_paths ... ok
test a_path_of_the_new_tree_is_left_alone ... ok
test the_serve_group_answers_bare_and_with_help_like_the_other_groups ... ok
test every_moved_path_still_works_and_names_where_it_went ... ok

test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.14s

     Running tests/moved_paths_are_not_taught.rs (crates/connectors-cli/target/debug/deps/moved_paths_are_not_taught-97aa8a1f37a00951)

running 1 test
test nothing_this_product_prints_names_a_path_that_moved ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.12s

   Doc-tests connectors_cli

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

error: 1 target failed:
    `--test closed_pipe`
```

Exit status: 101. Log: `~/.cache/connectors-cli-wave-20260906/emit-treats-a-closed-pipe-as-failure/adversary-1/full-suite.log`.


The fixture was corrected by calling `shutdown(Both)` on the peer before dropping it. This closes its read direction even while a fork briefly holds another descriptor. No production or existing test was changed. The affected stdin control was rerun alone before the final suite:

Command: `env -u CARGO_TARGET_DIR TMPDIR=~/.cache/cw6/e RUSTC_WRAPPER=/usr/bin/sccache CARGO_INCREMENTAL=0 CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 CARGO_BUILD_JOBS=3 cargo test --manifest-path crates/connectors-cli/Cargo.toml --locked --test closed_pipe stdin_read_errors_stay_unsuccessful_with_a_closed_output_reader -- --exact --nocapture`.

```text
warning: connector-secrets@0.6.5: no live Vault was offered (CONNECTOR_SECRETS_VAULT_ADDR and CONNECTOR_SECRETS_VAULT_TOKEN are unset), so the reqwest HttpTransport is UNEXERCISED by this build: tests/vault_live.rs is compiled #[ignore]d
   Compiling connectors-cli v0.6.5 (~/.local/state/worktree/trees/b10x/connectors/wt-55b6348b9982/crates/connectors-cli)
    Finished `test` profile [unoptimized] target(s) in 0.49s
     Running tests/closed_pipe.rs (crates/connectors-cli/target/debug/deps/closed_pipe-2a9d7878c97aca5d)

running 1 test
stdin failure json: exit status: 101
stdin failure yaml: exit status: 101
stdin failure text: exit status: 1
stdin failure compact: exit status: 1
test stdin_read_errors_stay_unsuccessful_with_a_closed_output_reader ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 11 filtered out; finished in 0.03s

```

Exit status: 0. Log: `~/.cache/connectors-cli-wave-20260906/emit-treats-a-closed-pipe-as-failure/adversary-1/stdin-shutdown-fixture.log`.


Final full CLI package output:

```text
warning: connector-secrets@0.6.5: no live Vault was offered (CONNECTOR_SECRETS_VAULT_ADDR and CONNECTOR_SECRETS_VAULT_TOKEN are unset), so the reqwest HttpTransport is UNEXERCISED by this build: tests/vault_live.rs is compiled #[ignore]d
    Finished `test` profile [unoptimized] target(s) in 0.31s
     Running unittests src/lib.rs (crates/connectors-cli/target/debug/deps/connectors_cli-57384b8f68913e31)

running 5 tests
test tests::grafana_connect_uses_the_same_guided_surface ... ok
test tests::slack_connect_needs_no_internal_reference_or_path_argument ... ok
test tests::kubernetes_connect_accepts_an_exact_context_selection ... ok
test tests::normal_help_exposes_the_guided_flow_and_hides_acquisition_plumbing ... ok
test tests::every_supported_shell_gets_a_script_naming_the_whole_surface ... ok

test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s

     Running unittests src/main.rs (crates/connectors-cli/target/debug/deps/connectors-345457defe935fe9)

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests/adversary_cli_cap_pass3.rs (crates/connectors-cli/target/debug/deps/adversary_cli_cap_pass3-70cb79e6da31936e)

running 1 test
test the_cap_the_design_page_says_is_measured_is_declared_and_asserted ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests/adversary_fence_probe.rs (crates/connectors-cli/target/debug/deps/adversary_fence_probe-6389f5548bc21248)

running 6 tests
test the_wire_name_rule_citation_in_the_specification_points_at_the_rule ... ok
test the_wire_name_rule_citation_in_the_design_document_points_at_the_rule ... ok
test the_typeable_words_are_the_words_the_design_document_names ... ok
test the_copies_this_probe_carries_are_still_copies ... ok
test a_forwarding_reason_that_names_no_command_is_refused_whatever_kind_it_carries ... ok
test every_entry_that_is_not_a_lifecycle_step_is_refused_when_it_claims_to_be_one ... ok

test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.13s

     Running tests/adversary_fence_probe_pass2.rs (crates/connectors-cli/target/debug/deps/adversary_fence_probe_pass2-1d36e62dc0fb94dd)

running 3 tests
test every_file_the_committed_contract_opens_is_a_file_a_clone_has ... ok
test the_kinds_the_design_document_says_rest_on_no_sentence_rest_on_no_sentence ... ok
test the_paths_the_design_document_says_send_no_protocol_request_send_none ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests/adversary_shim_pass3.rs (crates/connectors-cli/target/debug/deps/adversary_shim_pass3-a9d55d257d709356)

running 6 tests
test the_serve_group_advertises_a_help_subcommand ... ok
test a_help_path_of_the_new_tree_under_serve_is_left_alone ... ok
test connectors_help_still_answers_for_a_path_that_moved ... ok
test a_moved_path_typed_with_the_global_output_flag_still_works ... ok
test the_group_word_whose_only_command_moved_still_points_somewhere ... ok
test nothing_this_product_prints_names_a_moved_path_behind_a_global_flag ... ok

test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.11s

     Running tests/adversary_shim_pass4.rs (crates/connectors-cli/target/debug/deps/adversary_shim_pass4-9047fd66bf9f43ea)

running 3 tests
test the_table_this_suite_copies_by_hand_is_the_table_the_binary_ships ... ok
test the_auth_group_still_answers_the_help_subcommand_it_advertised ... ok
test a_two_word_path_that_moved_works_with_the_global_flag_between_its_words ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.05s

     Running tests/adversary_shim_pass5.rs (crates/connectors-cli/target/debug/deps/adversary_shim_pass5-fbcea861a3d6c3b8)

running 4 tests
test a_positional_value_spelled_help_is_a_value_not_a_help_request ... ok
test the_double_dash_escape_is_not_a_word_that_moved ... ok
test an_argument_neither_the_group_nor_the_leaf_declares_is_not_the_old_leaf ... ok
test serve_with_only_global_options_is_the_group_in_every_spelling_and_position ... ok

test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.05s

     Running tests/cli_surface.rs (crates/connectors-cli/target/debug/deps/cli_surface-8ea8fa8a79700c16)

running 34 tests
test every_kind_of_exception_is_used_and_every_entry_gives_a_reason ... ok
test every_named_exception_is_still_a_path_of_the_parser ... ok
test every_declared_group_is_a_group_of_the_parser ... ok
test every_declared_group_help_line_is_the_summary_the_specification_declares ... ok
test every_path_of_the_parser_is_declared_or_a_named_exception ... ok
test no_path_is_both_declared_and_excepted ... ok
test every_declaration_the_adversary_probe_copies_is_still_a_copy ... ok
test no_word_of_the_parser_answers_to_a_name_the_specification_cannot_declare ... ok
test every_citation_this_unit_wrote_resolves ... ok
test the_old_login_selected_target_guard_is_absent ... ok
test the_committed_generated_tree_is_the_specification_word_for_word ... ok
test the_read_verb_enumeration_partitions_the_protocols_it_names ... ok
test a_read_stops_being_an_exception_once_the_specification_declares_a_view ... ok
test the_regeneration_command_the_documents_name_is_the_one_the_gate_runs ... ok
test a_command_absorbed_into_the_exception_list_alone_is_refused ... ok
test the_specification_names_the_binary_the_parser_builds ... ok
test the_parser_accepts_target_before_and_after_each_dual_target_leaf ... ok
test the_target_countdown_is_exactly_what_the_parser_still_owes ... ok
test every_citation_that_names_a_symbol_lands_on_its_declaration ... ok
test target_conflict_does_not_wait_for_open_stdin ... ok
test the_exception_list_is_the_set_the_specification_enumerates ... ok
test the_kinds_the_tree_derives_are_the_kinds_the_list_carries ... ok
test target_conflict_precedes_invoke_payload_loading ... ok
test targeted_errors_keep_the_target_in_yaml_and_text ... ok
test an_explicit_hosted_target_requires_a_login_by_name_for_every_group ... ok
test an_omitted_target_ignores_a_saved_login_for_every_dual_target_group ... ok
test target_conflict_precedes_missing_or_malformed_inline_input ... ok
test an_exception_whose_kind_the_tree_contradicts_is_refused ... ok
test hosted_refuses_each_local_only_option_for_every_group ... ok
test selected_target_preserves_provider_owned_target_fields_in_every_renderer ... ok
test local_success_and_protocol_refusals_report_the_selected_target ... ok
test broken_explicit_hosted_selection_never_falls_back_to_a_local_listener ... ok
test all_target_conflicts_precede_local_and_hosted_state_access ... ok
test every_local_leaf_ignores_broken_login_metadata_and_preserves_its_request ... ok

test result: ok. 34 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.28s

     Running tests/cli_surface_drift.rs (crates/connectors-cli/target/debug/deps/cli_surface_drift-a9239116bea3e52d)

running 10 tests
test the_thin_frontend_citation_points_at_the_thin_frontend_test ... ok
test the_copied_declarations_are_still_copies ... ok
test a_command_added_under_the_wrong_declared_group_is_refused ... ok
test a_command_added_under_a_declared_group_is_refused ... ok
test a_committed_tree_that_swaps_completions_for_an_undeclared_word_is_refused ... ok
test a_committed_tree_whose_group_about_no_longer_matches_the_specification_is_refused ... ok
test cutting_the_admin_group_over_to_the_generated_tree_is_refused ... ok
test the_restated_contract_is_green_against_the_unchanged_tree ... ok
test a_target_flag_removed_from_a_group_is_refused_by_the_countdown ... ok
test cargo_can_read_the_committed_emitted_manifest ... ok

test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s

     Running tests/cli_surface_pass_two.rs (crates/connectors-cli/target/debug/deps/cli_surface_pass_two-0a78eebb598fc922)

running 6 tests
test the_design_document_names_only_constants_that_exist ... ok
test the_design_document_states_the_shape_of_the_exception_list ... ok
test the_design_document_describes_the_countdown_assertion_the_contract_makes ... ok
test the_target_countdown_candidates_are_derived_from_every_protocol_a_deployment_answers ... ok
test the_drift_suites_copies_are_checked_rather_than_cited ... ok
test the_drift_suite_attributes_nothing_to_the_contract_that_is_not_there ... ok

test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests/closed_pipe.rs (crates/connectors-cli/target/debug/deps/closed_pipe-2a9d7878c97aca5d)

running 12 tests
test completion_scripts_keep_other_output_write_failures_unsuccessful ... ok
test completion_scripts_still_accept_a_closed_reader ... ok
test stdin_read_errors_stay_unsuccessful_with_a_closed_output_reader ... ok
test a_closed_transport_stays_unsuccessful ... ok
test an_unhealthy_doctor_stays_unsuccessful_when_its_report_reader_closes ... FAILED
test compact_consumer_closes_early ... ok
test text_consumer_closes_early ... ok
test json_consumer_closes_early ... ok
test yaml_consumer_closes_early ... ok
test successful_admin_results_accept_a_closed_reader_in_every_format ... FAILED
test every_format_really_emits_more_than_a_64_kib_pipe_buffer ... ok
test a_real_non_broken_pipe_output_failure_stays_unsuccessful ... ok

failures:

---- an_unhealthy_doctor_stays_unsuccessful_when_its_report_reader_closes stdout ----
doctor json: open reader exit status: 1, closed reader exit status: 0
doctor yaml: open reader exit status: 1, closed reader exit status: 0
doctor text: open reader exit status: 1, closed reader exit status: 0
doctor compact: open reader exit status: 1, closed reader exit status: 0

thread 'an_unhealthy_doctor_stays_unsuccessful_when_its_report_reader_closes' (1309260) panicked at tests/closed_pipe.rs:302:5:
a closed report reader erased the unhealthy status in ["json", "yaml", "text", "compact"]
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace

---- successful_admin_results_accept_a_closed_reader_in_every_format stdout ----
admin json: open reader exit status: 0, closed reader exit status: 101; stderr: 
thread 'main' (1309569) panicked at /rustc/88d9e12ae178fab0fb5cc050a94da85685d449ea/library/std/src/io/stdio.rs:1166:9:
failed printing to stdout: Broken pipe (os error 32)
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace

admin yaml: open reader exit status: 0, closed reader exit status: 101; stderr: 
thread 'main' (1309849) panicked at /rustc/88d9e12ae178fab0fb5cc050a94da85685d449ea/library/std/src/io/stdio.rs:1166:9:
failed printing to stdout: Broken pipe (os error 32)
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace

admin text: open reader exit status: 0, closed reader exit status: 1; stderr: admin: the result could not be written: Broken pipe (os error 32)

admin compact: open reader exit status: 0, closed reader exit status: 1; stderr: admin: the result could not be written: Broken pipe (os error 32)


thread 'successful_admin_results_accept_a_closed_reader_in_every_format' (1309268) panicked at tests/closed_pipe.rs:409:5:
successful admin output rejected a closed reader in ["json", "yaml", "text", "compact"]


failures:
    an_unhealthy_doctor_stays_unsuccessful_when_its_report_reader_closes
    successful_admin_results_accept_a_closed_reader_in_every_format

test result: FAILED. 10 passed; 2 failed; 0 ignored; 0 measured; 0 filtered out; finished in 4.43s

error: test failed, to rerun pass `--test closed_pipe`
     Running tests/first_level_groups.rs (crates/connectors-cli/target/debug/deps/first_level_groups-6a4964cb963d131a)

running 5 tests
test the_first_level_is_eight_words ... ok
test doctor_reports_the_same_installation_at_both_paths ... ok
test a_path_of_the_new_tree_is_left_alone ... ok
test the_serve_group_answers_bare_and_with_help_like_the_other_groups ... ok
test every_moved_path_still_works_and_names_where_it_went ... ok

test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.14s

     Running tests/moved_paths_are_not_taught.rs (crates/connectors-cli/target/debug/deps/moved_paths_are_not_taught-97aa8a1f37a00951)

running 1 test
test nothing_this_product_prints_names_a_path_that_moved ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.12s

   Doc-tests connectors_cli

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

error: 1 target failed:
    `--test closed_pipe`
```

Exit status: 101. Log: `~/.cache/connectors-cli-wave-20260906/emit-treats-a-closed-pipe-as-failure/adversary-1/full-suite-final.log`.


Actual runner arithmetic: 5 + 0 + 1 + 6 + 3 + 6 + 3 + 4 + 34 + 10 + 6 + 12 + 5 + 1 + 0 = 96 executed cases; 94 passed, 2 failed, 0 ignored and 0 filtered. The focused closed_pipe target moved 8→12, with 10 passed and the two measured product failures. The four additional cases account for the full package moving 92→96.

`rustfmt --edition 2021 --check crates/connectors-cli/tests/closed_pipe.rs` exited 0 with empty output (`~/.cache/connectors-cli-wave-20260906/emit-treats-a-closed-pipe-as-failure/adversary-1/fmt.log`). Strict test-target clippy:

Command: `env -u CARGO_TARGET_DIR TMPDIR=~/.cache/cw6/e RUSTC_WRAPPER=/usr/bin/sccache CARGO_INCREMENTAL=0 CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 CARGO_BUILD_JOBS=3 cargo clippy --manifest-path crates/connectors-cli/Cargo.toml --locked --tests -- -D warnings`.

```text
warning: connector-secrets@0.6.5: no live Vault was offered (CONNECTOR_SECRETS_VAULT_ADDR and CONNECTOR_SECRETS_VAULT_TOKEN are unset), so the reqwest HttpTransport is UNEXERCISED by this build: tests/vault_live.rs is compiled #[ignore]d
    Checking connectors-cli v0.6.5 (~/.local/state/worktree/trees/b10x/connectors/wt-55b6348b9982/crates/connectors-cli)
    Finished `dev` profile [unoptimized] target(s) in 0.44s
```

Exit status: 0. Log: `~/.cache/connectors-cli-wave-20260906/emit-treats-a-closed-pipe-as-failure/adversary-1/clippy.log`.


4. Findings

The findings cover the assigned commit plus the test-only working tree. Base `76f3fef9ce53a92d54d5e1c8147c5943315d423f` was read through Git, not checked out or executed. No base worktree was assigned, so both origins remain undecided for coordinator classification. The new guard is the path that converts doctor's output error to success; the admin conversion and nested Output variant already exist in the inspected base.

| File:line | Verdict | Origin | Severity | Finding | What was measured | What reaches it |
| --- | --- | --- | --- | --- | --- | --- |
| crates/connectors-cli/src/lib.rs:805 | NEEDS-CHANGE | undecided | blocker | A closed stdout turns an unhealthy doctor report into exit 0 because rendering returns before the unhealthy status is applied. | Retained case at closed_pipe.rs:274/302; each format exits 1 with an open reader and 0 with a closed reader; deciding test and final suite exit 101. | `connectors inspect doctor --config <missing-file> --state-root <fixture>` calls diagnose at lib.rs:956, which emits at :964 before checking healthy at :965; run_from swallows the resulting Output BrokenPipe. |
| crates/connectors-cli/src/lib.rs:951 | NEEDS-CHANGE | undecided | blocker | Successful admin result output still fails on a closed reader because its nested AdminError::Output bypasses the BrokenPipe success guard. | Retained case at closed_pipe.rs:388/409; all open-reader controls exit 0, while closed-reader JSON/YAML exit 101 and text/compact exit 1; deciding test and final suite exit 101. | `connectors admin integrations status --endpoint <synthetic-loopback> --access-token-file <fixture-file>` reaches console admin.rs:153 output, AdminError::Output at :31, then MainError::Admin via lib.rs:951. |

The doctor failure must retain its semantic exit status even if report delivery fails. The admin path needs output-error handling that distinguishes its output variant from its transport and authentication failures. These are suggested correction boundaries; no fix was applied.

5. Attacks that remained green

- All four existing output formats accept early closure of successful operation output both before reading and after reading one byte, with full output above 64 KiB.
- An actual closed Connector transport remains unsuccessful in all four formats.
- A real non-BrokenPipe result writer failure remains unsuccessful in all four formats.
- A real stdin read failure remains unsuccessful even when its error output also meets a closed reader.
- Completion output accepts early reader closure and retains non-BrokenPipe writer failures.
- Existing CLI contract, drift, moved-path and target-selection tests continue to pass.

6. Complete external-path inventory

The 15 persistent files written in the assigned scratch are:

- `~/.cache/connectors-cli-wave-20260906/emit-treats-a-closed-pipe-as-failure/adversary-1/doctor.log`
- `~/.cache/connectors-cli-wave-20260906/emit-treats-a-closed-pipe-as-failure/adversary-1/admin.log`
- `~/.cache/connectors-cli-wave-20260906/emit-treats-a-closed-pipe-as-failure/adversary-1/admin-valid-fixture.log`
- `~/.cache/connectors-cli-wave-20260906/emit-treats-a-closed-pipe-as-failure/adversary-1/admin-fixture-diagnostic.log`
- `~/.cache/connectors-cli-wave-20260906/emit-treats-a-closed-pipe-as-failure/adversary-1/admin-correct-fixture.log`
- `~/.cache/connectors-cli-wave-20260906/emit-treats-a-closed-pipe-as-failure/adversary-1/stdin.log`
- `~/.cache/connectors-cli-wave-20260906/emit-treats-a-closed-pipe-as-failure/adversary-1/completion.log`
- `~/.cache/connectors-cli-wave-20260906/emit-treats-a-closed-pipe-as-failure/adversary-1/full-suite.log`
- `~/.cache/connectors-cli-wave-20260906/emit-treats-a-closed-pipe-as-failure/adversary-1/stdin-shutdown-fixture.log`
- `~/.cache/connectors-cli-wave-20260906/emit-treats-a-closed-pipe-as-failure/adversary-1/full-suite-final.log`
- `~/.cache/connectors-cli-wave-20260906/emit-treats-a-closed-pipe-as-failure/adversary-1/fmt.log`
- `~/.cache/connectors-cli-wave-20260906/emit-treats-a-closed-pipe-as-failure/adversary-1/clippy.log`
- `~/.cache/connectors-cli-wave-20260906/emit-treats-a-closed-pipe-as-failure/adversary-1/external-inventory.txt`
- `~/.cache/connectors-cli-wave-20260906/emit-treats-a-closed-pipe-as-failure/adversary-1/raw-report.md`
- `~/.cache/connectors-cli-wave-20260906/emit-treats-a-closed-pipe-as-failure/adversary-1/report.md`

The assigned temporary root is `~/.cache/cw6/e`. New fixtures create `~/.cache/cw6/e/pipe-<test-process-id-in-hex>-<counter-in-hex>/`, with `connectors.toml`, and, for admin cases, `synthetic-access-token`; existing operation fixtures additionally bind `connectors.sock`. Their Drop removes their own temporary directory. The doctor missing.toml path is intentionally never created. No managed worktree or build directory was removed.

The two complete package runs also executed existing tests which retain files under the same assigned temporary root. The 160 retained paths created after this review's first deciding run are listed verbatim below and in external-inventory.txt:

```text
~/.cache/cw6/e/t13f08126/s/b10x/connectors/identity-sessions.json
~/.cache/cw6/e/t13f08122/s/b10x/connectors/identity-sessions.json
~/.cache/cw6/e/t13f08126/c/b10x/connectors.toml
~/.cache/cw6/e/t13f08122/c/b10x/connectors.toml
~/.cache/cw6/e/t13f0811c/s/b10x/connectors/identity-sessions.json
~/.cache/cw6/e/t13f0811c/c/b10x/connectors.toml
~/.cache/cw6/e/t13f08120/s/b10x/connectors/identity-sessions.json
~/.cache/cw6/e/t13f08120/c/b10x/connectors.toml
~/.cache/cw6/e/t13f08112/s/b10x/connectors/identity-sessions.json
~/.cache/cw6/e/t13b07b17/s/b10x/connectors/identity-sessions.json
~/.cache/cw6/e/t13f08112/c/b10x/connectors.toml
~/.cache/cw6/e/t13b07b11/s/b10x/connectors/identity-sessions.json
~/.cache/cw6/e/t13f0811f/s/b10x/connectors/identity-sessions.json
~/.cache/cw6/e/t13b07b17/c/b10x/connectors.toml
~/.cache/cw6/e/t13b07b11/c/b10x/connectors.toml
~/.cache/cw6/e/t13f0811f/c/b10x/connectors.toml
~/.cache/cw6/e/t13b07b22/s/b10x/connectors/identity-sessions.json
~/.cache/cw6/e/t13b07b19/s/b10x/connectors/identity-sessions.json
~/.cache/cw6/e/t13b07b22/c/b10x/connectors.toml
~/.cache/cw6/e/t13f08111/s/b10x/connectors/identity-sessions.json
~/.cache/cw6/e/t13b07b19/c/b10x/connectors.toml
~/.cache/cw6/e/t13f08111/c/b10x/connectors.toml
~/.cache/cw6/e/t13f0811e/s/b10x/connectors/identity-sessions.json
~/.cache/cw6/e/t13f0811a/s/b10x/connectors/identity-sessions.json
~/.cache/cw6/e/t13f0811e/c/b10x/connectors.toml
~/.cache/cw6/e/t13f0811a/c/b10x/connectors.toml
~/.cache/cw6/e/t13b07b1/c/b10x/connectors.toml
~/.cache/cw6/e/t13f08117/s/b10x/connectors/identity-sessions.json
~/.cache/cw6/e/t13b07b21/s/b10x/connectors/identity-sessions.json
~/.cache/cw6/e/t13f0811/c/b10x/connectors.toml
~/.cache/cw6/e/t13f08117/c/b10x/connectors.toml
~/.cache/cw6/e/t13b07b21/c/b10x/connectors.toml
~/.cache/cw6/e/t13b07b27/s/b10x/connectors/identity-sessions.json
~/.cache/cw6/e/t13b07b1c/s/b10x/connectors/identity-sessions.json
~/.cache/cw6/e/t13b07b1e/s/b10x/connectors/identity-sessions.json
~/.cache/cw6/e/t13b07b27/c/b10x/connectors.toml
~/.cache/cw6/e/t13f081b/s/b10x/connectors/identity-sessions.json
~/.cache/cw6/e/t13b07b1c/c/b10x/connectors.toml
~/.cache/cw6/e/t13b07b1e/c/b10x/connectors.toml
~/.cache/cw6/e/t13f081b/c/b10x/connectors.toml
~/.cache/cw6/e/t13f08119/s/b10x/connectors/identity-sessions.json
~/.cache/cw6/e/t13b07bb/c/b10x/connectors.toml
~/.cache/cw6/e/t13f0813/c/b10x/connectors.toml
~/.cache/cw6/e/t13f0813/s/b10x/connectors/identity-sessions.json
~/.cache/cw6/e/t13f08119/c/b10x/connectors.toml
~/.cache/cw6/e/t13b07b2a/s/b10x/connectors/identity-sessions.json
~/.cache/cw6/e/t13b07bd/s/b10x/connectors/identity-sessions.json
~/.cache/cw6/e/t13b07b2a/c/b10x/connectors.toml
~/.cache/cw6/e/t13b07bd/c/b10x/connectors.toml
~/.cache/cw6/e/t13b07b24/s/b10x/connectors/identity-sessions.json
~/.cache/cw6/e/t13b07b16/s/b10x/connectors/identity-sessions.json
~/.cache/cw6/e/t13b07b24/c/b10x/connectors.toml
~/.cache/cw6/e/t13b07b16/c/b10x/connectors.toml
~/.cache/cw6/e/t13b07bc/s/b10x/connectors/identity-sessions.json
~/.cache/cw6/e/t13b07bc/c/b10x/connectors.toml
~/.cache/cw6/e/t13f08121/s/b10x/connectors/identity-sessions.json
~/.cache/cw6/e/t13b07be/s/b10x/connectors/identity-sessions.json
~/.cache/cw6/e/t13f08121/c/b10x/connectors.toml
~/.cache/cw6/e/t13b07be/c/b10x/connectors.toml
~/.cache/cw6/e/t13f08128/s/b10x/connectors/identity-sessions.json
~/.cache/cw6/e/t13b07b2/s/b10x/connectors/identity-sessions.json
~/.cache/cw6/e/t13f08128/c/b10x/connectors.toml
~/.cache/cw6/e/t13b07b2/c/b10x/connectors.toml
~/.cache/cw6/e/t13b07b14/s/b10x/connectors/identity-sessions.json
~/.cache/cw6/e/t13b07b14/c/b10x/connectors.toml
~/.cache/cw6/e/t13b07b7/s/b10x/connectors/identity-sessions.json
~/.cache/cw6/e/t13f0815/c/b10x/connectors.toml
~/.cache/cw6/e/t13b07b15/s/b10x/connectors/identity-sessions.json
~/.cache/cw6/e/t13b07b23/s/b10x/connectors/identity-sessions.json
~/.cache/cw6/e/t13b07b15/c/b10x/connectors.toml
~/.cache/cw6/e/t13b07b23/c/b10x/connectors.toml
~/.cache/cw6/e/t13b07b25/s/b10x/connectors/identity-sessions.json
~/.cache/cw6/e/t13b07b25/c/b10x/connectors.toml
~/.cache/cw6/e/t13b07b28/s/b10x/connectors/identity-sessions.json
~/.cache/cw6/e/t13b07b6/c/b10x/connectors.toml
~/.cache/cw6/e/t13b07b7/c/b10x/connectors.toml
~/.cache/cw6/e/t13b07b1d/s/b10x/connectors/identity-sessions.json
~/.cache/cw6/e/t13b07b28/c/b10x/connectors.toml
~/.cache/cw6/e/t13f08114/s/b10x/connectors/identity-sessions.json
~/.cache/cw6/e/t13b07b5/s/b10x/connectors/identity-sessions.json
~/.cache/cw6/e/t13f08114/c/b10x/connectors.toml
~/.cache/cw6/e/t13b07bf/s/b10x/connectors/identity-sessions.json
~/.cache/cw6/e/t13b07b5/c/b10x/connectors.toml
~/.cache/cw6/e/t13f0818/c/b10x/connectors.toml
~/.cache/cw6/e/t13b07b1d/c/b10x/connectors.toml
~/.cache/cw6/e/t13b07bf/c/b10x/connectors.toml
~/.cache/cw6/e/t13f081d/c/b10x/connectors.toml
~/.cache/cw6/e/t13b07b4/s/b10x/connectors/identity-sessions.json
~/.cache/cw6/e/t13f0812/s/b10x/connectors/identity-sessions.json
~/.cache/cw6/e/t13f081d/s/b10x/connectors/identity-sessions.json
~/.cache/cw6/e/t13b07b4/c/b10x/connectors.toml
~/.cache/cw6/e/t13f0814/c/b10x/connectors.toml
~/.cache/cw6/e/t13f0814/s/b10x/connectors/identity-sessions.json
~/.cache/cw6/e/t13f0812/c/b10x/connectors.toml
~/.cache/cw6/e/t13f081f/s/b10x/connectors/identity-sessions.json
~/.cache/cw6/e/t13f081f/c/b10x/connectors.toml
~/.cache/cw6/e/t13f0811b/s/b10x/connectors/identity-sessions.json
~/.cache/cw6/e/t13b07b18/s/b10x/connectors/identity-sessions.json
~/.cache/cw6/e/t13f0811b/c/b10x/connectors.toml
~/.cache/cw6/e/t13b07b18/c/b10x/connectors.toml
~/.cache/cw6/e/t13f08124/s/b10x/connectors/identity-sessions.json
~/.cache/cw6/e/t13b07b0/s/b10x/connectors/identity-sessions.json
~/.cache/cw6/e/t13b07b12/c/b10x/connectors.toml
~/.cache/cw6/e/t13b07b0/c/b10x/connectors.toml
~/.cache/cw6/e/t13f0819/c/b10x/connectors.toml
~/.cache/cw6/e/t13f08124/c/b10x/connectors.toml
~/.cache/cw6/e/t13f08113/c/b10x/connectors.toml
~/.cache/cw6/e/t13b07b12/s/b10x/connectors/identity-sessions.json
~/.cache/cw6/e/t13f08127/s/b10x/connectors/identity-sessions.json
~/.cache/cw6/e/t13f08113/s/b10x/connectors/identity-sessions.json
~/.cache/cw6/e/t13f08129/s/b10x/connectors/identity-sessions.json
~/.cache/cw6/e/t13f0817/s/b10x/connectors/identity-sessions.json
~/.cache/cw6/e/t13f0817/c/b10x/connectors.toml
~/.cache/cw6/e/t13f08129/c/b10x/connectors.toml
~/.cache/cw6/e/t13f08127/c/b10x/connectors.toml
~/.cache/cw6/e/t13f08125/s/b10x/connectors/identity-sessions.json
~/.cache/cw6/e/t13b07b1b/c/b10x/connectors.toml
~/.cache/cw6/e/t13b07b20/s/b10x/connectors/identity-sessions.json
~/.cache/cw6/e/t13b07b1b/s/b10x/connectors/identity-sessions.json
~/.cache/cw6/e/t13b07b20/c/b10x/connectors.toml
~/.cache/cw6/e/t13f08123/s/b10x/connectors/identity-sessions.json
~/.cache/cw6/e/t13f08123/c/b10x/connectors.toml
~/.cache/cw6/e/t13f08125/c/b10x/connectors.toml
~/.cache/cw6/e/t13f08118/s/b10x/connectors/identity-sessions.json
~/.cache/cw6/e/t13b07b1f/s/b10x/connectors/identity-sessions.json
~/.cache/cw6/e/t13f08118/c/b10x/connectors.toml
~/.cache/cw6/e/t13f08115/s/b10x/connectors/identity-sessions.json
~/.cache/cw6/e/t13b07b1f/c/b10x/connectors.toml
~/.cache/cw6/e/t13f081c/s/b10x/connectors/identity-sessions.json
~/.cache/cw6/e/t13f08115/c/b10x/connectors.toml
~/.cache/cw6/e/t13f081c/c/b10x/connectors.toml
~/.cache/cw6/e/t13f0811d/s/b10x/connectors/identity-sessions.json
~/.cache/cw6/e/t13b07b3/s/b10x/connectors/identity-sessions.json
~/.cache/cw6/e/t13f081a/c/b10x/connectors.toml
~/.cache/cw6/e/t13b07b26/s/b10x/connectors/identity-sessions.json
~/.cache/cw6/e/t13f0811d/c/b10x/connectors.toml
~/.cache/cw6/e/t13b07b3/c/b10x/connectors.toml
~/.cache/cw6/e/t13b07b26/c/b10x/connectors.toml
~/.cache/cw6/e/t13b07b8/c/b10x/connectors.toml
~/.cache/cw6/e/t13b07b9/c/b10x/connectors.toml
~/.cache/cw6/e/t13f081e/s/b10x/connectors/identity-sessions.json
~/.cache/cw6/e/t13f081e/c/b10x/connectors.toml
~/.cache/cw6/e/t13b07b10/s/b10x/connectors/identity-sessions.json
~/.cache/cw6/e/t13b07b13/s/b10x/connectors/identity-sessions.json
~/.cache/cw6/e/t13b07b10/c/b10x/connectors.toml
~/.cache/cw6/e/t13f0812a/s/b10x/connectors/identity-sessions.json
~/.cache/cw6/e/t13b07b29/s/b10x/connectors/identity-sessions.json
~/.cache/cw6/e/t13b07b13/c/b10x/connectors.toml
~/.cache/cw6/e/t13f0812a/c/b10x/connectors.toml
~/.cache/cw6/e/t13b07b29/c/b10x/connectors.toml
~/.cache/cw6/e/t13b07ba/c/b10x/connectors.toml
~/.cache/cw6/e/t13b07b1a/s/b10x/connectors/identity-sessions.json
~/.cache/cw6/e/t13f08116/s/b10x/connectors/identity-sessions.json
~/.cache/cw6/e/t13f0810/s/b10x/connectors/identity-sessions.json
~/.cache/cw6/e/t13f0816/c/b10x/connectors.toml
~/.cache/cw6/e/t13b07b1a/c/b10x/connectors.toml
~/.cache/cw6/e/t13f0810/c/b10x/connectors.toml
~/.cache/cw6/e/t13f08116/c/b10x/connectors.toml
~/.cache/cw6/e/t13f08110/c/b10x/connectors.toml
~/.cache/cw6/e/t13f08110/s/b10x/connectors/identity-sessions.json
```

Cargo output stayed in `~/.local/state/worktree/trees/b10x/connectors/wt-55b6348b9982/crates/connectors-cli/target`, inside the assigned worktree. CARGO_TARGET_DIR was unset. Cargo transient files used the assigned TMPDIR; shared Cargo and sccache caches were tool-managed and were not cleaned or redirected.

The companion report.md mechanically replaces only absolute local-home prefixes with ~. No other text, measured output, finding, count or status is changed.

```findings
- file: crates/connectors-cli/src/lib.rs
  line: 805
  category: boundary
  severity: blocker
  verdict: NEEDS-CHANGE
  origin: undecided
  message: A closed stdout turns an unhealthy doctor report into exit 0 because rendering returns before the unhealthy status is applied.
- file: crates/connectors-cli/src/lib.rs
  line: 951
  category: acceptance
  severity: blocker
  verdict: NEEDS-CHANGE
  origin: undecided
  message: Successful admin result output still fails on a closed reader because its nested AdminError::Output bypasses the BrokenPipe success guard.
```
