---
format: aep.planning-md/1
id: review-result:cli-pipe-adversary-2-20260906
kind: review-result
status: active
title: CLI closed-pipe adversary pass 2
refs:
- provider: git
  reference: 68b30016aafb3a2bf5912f2d606c2a3a6413e100
relations:
- reviews: story:emit-treats-a-closed-pipe-as-failure
revision: 1
---
unit: story:emit-treats-a-closed-pipe-as-failure at 68b30016aafb3a2bf5912f2d606c2a3a6413e100 plus test-only additions
verdict: nothing found
cases: executed 101→104, red 0
origin: introduced 0 / pre-existing 0 / undecided 0
wrote-outside-worktree: 137 retained files; transient fixture and tool-cache classes inventoried below
needs-coordinator: none

```text
 crates/connectors-cli/tests/closed_pipe.rs | 147 +++++++++++++++++++++++++++++
 1 file changed, 147 insertions(+)
```

1. Scope proof

Only `crates/connectors-cli/tests/closed_pipe.rs` changed: three appended cases and their helper. A byte-prefix comparison against HEAD confirms all 627 retained lines are unchanged. No production file, Git index/ref, planning artifact, operator configuration, credentials or live service was mutated. The assigned managed worktree was reused under the worktree skill; no lifecycle or cleanup command was run. `git diff --check` exited 0. The exact test-only diff is retained as test-only.patch in the assigned scratch.

Read the whole unit diff against base `76f3fef9ce53a92d54d5e1c8147c5943315d423f`, the current log, acceptance, previous findings, correction account and retained tests. Traced run_from from main, all new emit/emit_targeted callers, complete_output, both administrative leaves, protocol-envelope reduction and renderer write/flush boundaries. No base execution was assigned or performed, and no finding origin is inferred from inspection.

2. Cases written before execution

All three additions existed before any test execution. Before-count 101 comes from the correction's reported unfiltered package run, not from an early suite run by this pass. The deciding protocol-refusal case ran alone first; each of the other two cases then ran alone before the full suite. All three were green on their first execution, so this pass has no red deciding output.

- `crates/connectors-cli/tests/closed_pipe.rs:693`: protocol refusals remain unsuccessful for connection, event and operation commands in every format with stdout already closed; open-reader controls prove the server refusal reaches the CLI. Green.
- `crates/connectors-cli/tests/closed_pipe.rs:715`: successful searches in the three protocol groups accept an already-closed reader, while a real non-BrokenPipe stdout error remains unsuccessful whenever that renderer produces bytes. Empty compact listings intentionally emit no bytes and cannot exercise a write failure. Green.
- `crates/connectors-cli/tests/closed_pipe.rs:738`: setup init finishes an owner-only fixture configuration before accepting output closure, and a repeated invocation still refuses without changing the existing bytes. Green.

The synthetic Unix peers require the actual search method and preserve request protocol/request_id. The closed stdout helper asserts an actual BrokenPipe before spawning each CLI process; the other-error sink asserts its real write failure is not BrokenPipe. The init case supplies only its own inert kubeconfig; no Kubernetes API is contacted.

Command: `env -u CARGO_TARGET_DIR TMPDIR=~/.cache/cw6/e RUSTC_WRAPPER=/usr/bin/sccache CARGO_INCREMENTAL=0 CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 CARGO_BUILD_JOBS=3 cargo test --manifest-path crates/connectors-cli/Cargo.toml --locked --test closed_pipe protocol_refusals_keep_their_failure_when_a_result_reader_closes -- --exact --nocapture`.

```text
warning: connector-secrets@0.6.5: no live Vault was offered (CONNECTOR_SECRETS_VAULT_ADDR and CONNECTOR_SECRETS_VAULT_TOKEN are unset), so the reqwest HttpTransport is UNEXERCISED by this build: tests/vault_live.rs is compiled #[ignore]d
   Compiling connectors-cli v0.6.5 (~/.local/state/worktree/trees/b10x/connectors/wt-55b6348b9982/crates/connectors-cli)
    Finished `test` profile [unoptimized] target(s) in 1.22s
     Running tests/closed_pipe.rs (crates/connectors-cli/target/debug/deps/closed_pipe-2a9d7878c97aca5d)

running 1 test
protocol refusal connection json: open exit status: 1, closed exit status: 101
protocol refusal connection yaml: open exit status: 1, closed exit status: 101
protocol refusal connection text: open exit status: 1, closed exit status: 1
protocol refusal connection compact: open exit status: 1, closed exit status: 1
protocol refusal event json: open exit status: 1, closed exit status: 101
protocol refusal event yaml: open exit status: 1, closed exit status: 101
protocol refusal event text: open exit status: 1, closed exit status: 1
protocol refusal event compact: open exit status: 1, closed exit status: 1
protocol refusal operation json: open exit status: 1, closed exit status: 101
protocol refusal operation yaml: open exit status: 1, closed exit status: 101
protocol refusal operation text: open exit status: 1, closed exit status: 1
protocol refusal operation compact: open exit status: 1, closed exit status: 1
test protocol_refusals_keep_their_failure_when_a_result_reader_closes ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 19 filtered out; finished in 0.30s

```

Exit status: 0.

Command: `env -u CARGO_TARGET_DIR TMPDIR=~/.cache/cw6/e RUSTC_WRAPPER=/usr/bin/sccache CARGO_INCREMENTAL=0 CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 CARGO_BUILD_JOBS=3 cargo test --manifest-path crates/connectors-cli/Cargo.toml --locked --test closed_pipe each_protocol_search_distinguishes_closed_readers_from_other_write_failures -- --exact --nocapture`.

```text
    Blocking waiting for file lock on package cache
    Blocking waiting for file lock on package cache
    Blocking waiting for file lock on package cache
    Blocking waiting for file lock on package cache
    Blocking waiting for file lock on build directory
warning: connector-secrets@0.6.5: no live Vault was offered (CONNECTOR_SECRETS_VAULT_ADDR and CONNECTOR_SECRETS_VAULT_TOKEN are unset), so the reqwest HttpTransport is UNEXERCISED by this build: tests/vault_live.rs is compiled #[ignore]d
    Finished `test` profile [unoptimized] target(s) in 0.50s
     Running tests/closed_pipe.rs (crates/connectors-cli/target/debug/deps/closed_pipe-2a9d7878c97aca5d)

running 1 test
protocol search connection json: closed exit status: 0
protocol search connection yaml: closed exit status: 0
protocol search connection text: closed exit status: 0
protocol search connection compact: closed exit status: 0
protocol search event json: closed exit status: 0
protocol search event yaml: closed exit status: 0
protocol search event text: closed exit status: 0
protocol search event compact: closed exit status: 0
protocol search operation json: closed exit status: 0
protocol search operation yaml: closed exit status: 0
protocol search operation text: closed exit status: 0
protocol search operation compact: closed exit status: 0
test each_protocol_search_distinguishes_closed_readers_from_other_write_failures ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 19 filtered out; finished in 0.39s

```

Exit status: 0.

Command: `env -u CARGO_TARGET_DIR TMPDIR=~/.cache/cw6/e RUSTC_WRAPPER=/usr/bin/sccache CARGO_INCREMENTAL=0 CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 CARGO_BUILD_JOBS=3 cargo test --manifest-path crates/connectors-cli/Cargo.toml --locked --test closed_pipe setup_init_commits_its_result_before_a_reader_close_but_keeps_repeat_refusal -- --exact --nocapture`.

```text
    Blocking waiting for file lock on package cache
    Blocking waiting for file lock on package cache
    Blocking waiting for file lock on package cache
warning: connector-secrets@0.6.5: no live Vault was offered (CONNECTOR_SECRETS_VAULT_ADDR and CONNECTOR_SECRETS_VAULT_TOKEN are unset), so the reqwest HttpTransport is UNEXERCISED by this build: tests/vault_live.rs is compiled #[ignore]d
    Finished `test` profile [unoptimized] target(s) in 0.46s
     Running tests/closed_pipe.rs (crates/connectors-cli/target/debug/deps/closed_pipe-2a9d7878c97aca5d)

running 1 test
setup init json: created exit status: 0, repeated exit status: 101
setup init yaml: created exit status: 0, repeated exit status: 101
setup init text: created exit status: 0, repeated exit status: 1
setup init compact: created exit status: 0, repeated exit status: 1
test setup_init_commits_its_result_before_a_reader_close_but_keeps_repeat_refusal ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 19 filtered out; finished in 0.07s

```

Exit status: 0.

3. Full package execution and test hygiene

Command: `env -u CARGO_TARGET_DIR TMPDIR=~/.cache/cw6/e RUSTC_WRAPPER=/usr/bin/sccache CARGO_INCREMENTAL=0 CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 CARGO_BUILD_JOBS=3 cargo test --manifest-path crates/connectors-cli/Cargo.toml --locked --no-fail-fast`.

```text
    Blocking waiting for file lock on package cache
    Blocking waiting for file lock on package cache
warning: connector-secrets@0.6.5: no live Vault was offered (CONNECTOR_SECRETS_VAULT_ADDR and CONNECTOR_SECRETS_VAULT_TOKEN are unset), so the reqwest HttpTransport is UNEXERCISED by this build: tests/vault_live.rs is compiled #[ignore]d
    Finished `test` profile [unoptimized] target(s) in 0.32s
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
test the_wire_name_rule_citation_in_the_design_document_points_at_the_rule ... ok
test the_wire_name_rule_citation_in_the_specification_points_at_the_rule ... ok
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

test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.36s

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

test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.05s

     Running tests/cli_surface.rs (crates/connectors-cli/target/debug/deps/cli_surface-8ea8fa8a79700c16)

running 34 tests
test every_kind_of_exception_is_used_and_every_entry_gives_a_reason ... ok
test every_declared_group_is_a_group_of_the_parser ... ok
test every_named_exception_is_still_a_path_of_the_parser ... ok
test no_word_of_the_parser_answers_to_a_name_the_specification_cannot_declare ... ok
test every_path_of_the_parser_is_declared_or_a_named_exception ... ok
test every_declared_group_help_line_is_the_summary_the_specification_declares ... ok
test no_path_is_both_declared_and_excepted ... ok
test every_declaration_the_adversary_probe_copies_is_still_a_copy ... ok
test the_committed_generated_tree_is_the_specification_word_for_word ... ok
test the_old_login_selected_target_guard_is_absent ... ok
test every_citation_this_unit_wrote_resolves ... ok
test the_parser_accepts_target_before_and_after_each_dual_target_leaf ... ok
test the_read_verb_enumeration_partitions_the_protocols_it_names ... ok
test the_regeneration_command_the_documents_name_is_the_one_the_gate_runs ... ok
test the_specification_names_the_binary_the_parser_builds ... ok
test a_read_stops_being_an_exception_once_the_specification_declares_a_view ... ok
test a_command_absorbed_into_the_exception_list_alone_is_refused ... ok
test every_citation_that_names_a_symbol_lands_on_its_declaration ... ok
test the_target_countdown_is_exactly_what_the_parser_still_owes ... ok
test the_kinds_the_tree_derives_are_the_kinds_the_list_carries ... ok
test the_exception_list_is_the_set_the_specification_enumerates ... ok
test target_conflict_precedes_invoke_payload_loading ... ok
test targeted_errors_keep_the_target_in_yaml_and_text ... ok
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

test result: ok. 34 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.30s

     Running tests/cli_surface_drift.rs (crates/connectors-cli/target/debug/deps/cli_surface_drift-a9239116bea3e52d)

running 10 tests
test the_thin_frontend_citation_points_at_the_thin_frontend_test ... ok
test the_copied_declarations_are_still_copies ... ok
test a_command_added_under_the_wrong_declared_group_is_refused ... ok
test a_committed_tree_whose_group_about_no_longer_matches_the_specification_is_refused ... ok
test a_command_added_under_a_declared_group_is_refused ... ok
test a_committed_tree_that_swaps_completions_for_an_undeclared_word_is_refused ... ok
test cutting_the_admin_group_over_to_the_generated_tree_is_refused ... ok
test the_restated_contract_is_green_against_the_unchanged_tree ... ok
test a_target_flag_removed_from_a_group_is_refused_by_the_countdown ... ok
test cargo_can_read_the_committed_emitted_manifest ... ok

test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s

     Running tests/cli_surface_pass_two.rs (crates/connectors-cli/target/debug/deps/cli_surface_pass_two-0a78eebb598fc922)

running 6 tests
test the_design_document_states_the_shape_of_the_exception_list ... ok
test the_design_document_names_only_constants_that_exist ... ok
test the_design_document_describes_the_countdown_assertion_the_contract_makes ... ok
test the_target_countdown_candidates_are_derived_from_every_protocol_a_deployment_answers ... ok
test the_drift_suites_copies_are_checked_rather_than_cited ... ok
test the_drift_suite_attributes_nothing_to_the_contract_that_is_not_there ... ok

test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests/closed_pipe.rs (crates/connectors-cli/target/debug/deps/closed_pipe-2a9d7878c97aca5d)

running 20 tests
test completion_scripts_keep_other_output_write_failures_unsuccessful ... ok
test completion_scripts_still_accept_a_closed_reader ... ok
test stdin_read_errors_stay_unsuccessful_with_a_closed_output_reader ... ok
test admin_authentication_failures_remain_unsuccessful_with_a_closed_reader ... ok
test a_closed_transport_stays_unsuccessful ... ok
test compact_consumer_closes_early ... ok
test json_consumer_closes_early ... ok
test text_consumer_closes_early ... ok
test an_unhealthy_doctor_stays_unsuccessful_when_its_report_reader_closes ... ok
test a_healthy_doctor_accepts_a_closed_report_reader ... ok
test setup_init_commits_its_result_before_a_reader_close_but_keeps_repeat_refusal ... ok
test successful_admin_credential_write_accepts_a_closed_reader ... ok
test every_admin_leaf_preserves_non_broken_pipe_output_failures ... ok
test yaml_consumer_closes_early ... ok
test successful_admin_results_accept_a_closed_reader_in_every_format ... ok
test every_format_really_emits_more_than_a_64_kib_pipe_buffer ... ok
test each_unhealthy_report_class_keeps_its_failure_when_output_closes ... ok
test protocol_refusals_keep_their_failure_when_a_result_reader_closes ... ok
test each_protocol_search_distinguishes_closed_readers_from_other_write_failures ... ok
test a_real_non_broken_pipe_output_failure_stays_unsuccessful ... ok

test result: ok. 20 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 4.75s

     Running tests/first_level_groups.rs (crates/connectors-cli/target/debug/deps/first_level_groups-6a4964cb963d131a)

running 5 tests
test the_first_level_is_eight_words ... ok
test doctor_reports_the_same_installation_at_both_paths ... ok
test a_path_of_the_new_tree_is_left_alone ... ok
test the_serve_group_answers_bare_and_with_help_like_the_other_groups ... ok
test every_moved_path_still_works_and_names_where_it_went ... ok

test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.16s

     Running tests/moved_paths_are_not_taught.rs (crates/connectors-cli/target/debug/deps/moved_paths_are_not_taught-97aa8a1f37a00951)

running 1 test
test nothing_this_product_prints_names_a_path_that_moved ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.12s

   Doc-tests connectors_cli

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

```

Exit status: 0.

Runner arithmetic is 5 + 0 + 1 + 6 + 3 + 6 + 3 + 4 + 34 + 10 + 6 + 20 + 5 + 1 + 0 = 104 executed, all passed, zero failed or ignored. Closed-pipe cases increased 17→20; the whole package increased 101→104.

The first format check reported only two line-wrapping differences in the new init case. Those two new statements were formatted; no assertion or production byte changed. The final package fmt check exited 0 with empty output. Both fmt outputs and statuses are retained. No test was removed, skipped or weakened.

Command: `env -u CARGO_TARGET_DIR TMPDIR=~/.cache/cw6/e RUSTC_WRAPPER=/usr/bin/sccache CARGO_INCREMENTAL=0 CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 CARGO_BUILD_JOBS=3 cargo clippy --manifest-path crates/connectors-cli/Cargo.toml --locked --tests -- -D warnings`.

```text
warning: connector-secrets@0.6.5: no live Vault was offered (CONNECTOR_SECRETS_VAULT_ADDR and CONNECTOR_SECRETS_VAULT_TOKEN are unset), so the reqwest HttpTransport is UNEXERCISED by this build: tests/vault_live.rs is compiled #[ignore]d
    Checking connectors-cli v0.6.5 (~/.local/state/worktree/trees/b10x/connectors/wt-55b6348b9982/crates/connectors-cli)
    Finished `dev` profile [unoptimized] target(s) in 1.39s
```

Exit status: 0.

4. Findings

Nothing found in this pass.

JSON/YAML semantic-failure commands still exit 101 when their unchanged error-envelope println meets closed stdout; text/compact exit 1. These measured statuses remain unsuccessful. The assigned brief explicitly identifies that existing error-rendering limitation; it is not reclassified as successful result output or presented as a newly attributed finding here.

5. Attacks that remained green

- Protocol refusals cannot be erased by closing stdout across all three protocol groups and all four formats.
- Successful small protocol results distinguish an already-closed reader from another real write error.
- Init's completed filesystem action survives result-output closure, while an existing-file refusal remains unsuccessful and preserves the bytes.
- Both retained first-pass failures now pass: unsuccessful doctor remains unsuccessful, and both successful administrative leaves accept an early-closing reader.
- Existing large-output tests still assert output above 64 KiB and exercise closure before reading and after one byte in every format.
- Existing transport, stdin, authentication, non-BrokenPipe writer and completions controls pass.

6. External writes

This pass retained 22 files in its assigned scratch and 115 newly retained files from the existing full-suite fixture class under the assigned TMPDIR. Exact paths:

```text
~/.cache/connectors-cli-wave-20260906/emit-treats-a-closed-pipe-as-failure/adversary-2/clippy.exit
~/.cache/connectors-cli-wave-20260906/emit-treats-a-closed-pipe-as-failure/adversary-2/clippy.log
~/.cache/connectors-cli-wave-20260906/emit-treats-a-closed-pipe-as-failure/adversary-2/counts.json
~/.cache/connectors-cli-wave-20260906/emit-treats-a-closed-pipe-as-failure/adversary-2/deciding-init.exit
~/.cache/connectors-cli-wave-20260906/emit-treats-a-closed-pipe-as-failure/adversary-2/deciding-init.log
~/.cache/connectors-cli-wave-20260906/emit-treats-a-closed-pipe-as-failure/adversary-2/deciding-protocol-refusal.exit
~/.cache/connectors-cli-wave-20260906/emit-treats-a-closed-pipe-as-failure/adversary-2/deciding-protocol-refusal.log
~/.cache/connectors-cli-wave-20260906/emit-treats-a-closed-pipe-as-failure/adversary-2/deciding-protocol-search.exit
~/.cache/connectors-cli-wave-20260906/emit-treats-a-closed-pipe-as-failure/adversary-2/deciding-protocol-search.log
~/.cache/connectors-cli-wave-20260906/emit-treats-a-closed-pipe-as-failure/adversary-2/diff-check.txt
~/.cache/connectors-cli-wave-20260906/emit-treats-a-closed-pipe-as-failure/adversary-2/diff-stat.txt
~/.cache/connectors-cli-wave-20260906/emit-treats-a-closed-pipe-as-failure/adversary-2/external-inventory.txt
~/.cache/connectors-cli-wave-20260906/emit-treats-a-closed-pipe-as-failure/adversary-2/fmt-final.exit
~/.cache/connectors-cli-wave-20260906/emit-treats-a-closed-pipe-as-failure/adversary-2/fmt-final.log
~/.cache/connectors-cli-wave-20260906/emit-treats-a-closed-pipe-as-failure/adversary-2/fmt.exit
~/.cache/connectors-cli-wave-20260906/emit-treats-a-closed-pipe-as-failure/adversary-2/fmt.log
~/.cache/connectors-cli-wave-20260906/emit-treats-a-closed-pipe-as-failure/adversary-2/outside-before.json
~/.cache/connectors-cli-wave-20260906/emit-treats-a-closed-pipe-as-failure/adversary-2/raw-report.md
~/.cache/connectors-cli-wave-20260906/emit-treats-a-closed-pipe-as-failure/adversary-2/report.md
~/.cache/connectors-cli-wave-20260906/emit-treats-a-closed-pipe-as-failure/adversary-2/suite.exit
~/.cache/connectors-cli-wave-20260906/emit-treats-a-closed-pipe-as-failure/adversary-2/suite.log
~/.cache/connectors-cli-wave-20260906/emit-treats-a-closed-pipe-as-failure/adversary-2/test-only.patch
~/.cache/cw6/e/t19b1710/c/b10x/connectors.toml
~/.cache/cw6/e/t19b1710/s/b10x/connectors/identity-sessions.json
~/.cache/cw6/e/t19b1711/c/b10x/connectors.toml
~/.cache/cw6/e/t19b17110/c/b10x/connectors.toml
~/.cache/cw6/e/t19b17110/s/b10x/connectors/connectors.sock
~/.cache/cw6/e/t19b17110/s/b10x/connectors/identity-sessions.json
~/.cache/cw6/e/t19b17111/c/b10x/connectors.toml
~/.cache/cw6/e/t19b17111/s/b10x/connectors/connectors.sock
~/.cache/cw6/e/t19b17111/s/b10x/connectors/identity-sessions.json
~/.cache/cw6/e/t19b17112/c/b10x/connectors.toml
~/.cache/cw6/e/t19b17112/s/b10x/connectors/connectors.sock
~/.cache/cw6/e/t19b17112/s/b10x/connectors/identity-sessions.json
~/.cache/cw6/e/t19b17113/c/b10x/connectors.toml
~/.cache/cw6/e/t19b17113/connectors.sock
~/.cache/cw6/e/t19b17113/s/b10x/connectors/identity-sessions.json
~/.cache/cw6/e/t19b17114/c/b10x/connectors.toml
~/.cache/cw6/e/t19b17114/s/b10x/connectors/connectors.sock
~/.cache/cw6/e/t19b17114/s/b10x/connectors/identity-sessions.json
~/.cache/cw6/e/t19b17115/c/b10x/connectors.toml
~/.cache/cw6/e/t19b17115/connectors.sock
~/.cache/cw6/e/t19b17115/s/b10x/connectors/identity-sessions.json
~/.cache/cw6/e/t19b17116/c/b10x/connectors.toml
~/.cache/cw6/e/t19b17116/s/b10x/connectors/connectors.sock
~/.cache/cw6/e/t19b17116/s/b10x/connectors/identity-sessions.json
~/.cache/cw6/e/t19b17117/c/b10x/connectors.toml
~/.cache/cw6/e/t19b17117/connectors.sock
~/.cache/cw6/e/t19b17117/s/b10x/connectors/identity-sessions.json
~/.cache/cw6/e/t19b17118/c/b10x/connectors.toml
~/.cache/cw6/e/t19b17118/s/b10x/connectors/connectors.sock
~/.cache/cw6/e/t19b17118/s/b10x/connectors/identity-sessions.json
~/.cache/cw6/e/t19b17119/c/b10x/connectors.toml
~/.cache/cw6/e/t19b17119/s/b10x/connectors/connectors.sock
~/.cache/cw6/e/t19b17119/s/b10x/connectors/identity-sessions.json
~/.cache/cw6/e/t19b1711a/c/b10x/connectors.toml
~/.cache/cw6/e/t19b1711a/s/b10x/connectors/connectors.sock
~/.cache/cw6/e/t19b1711a/s/b10x/connectors/identity-sessions.json
~/.cache/cw6/e/t19b1711b/c/b10x/connectors.toml
~/.cache/cw6/e/t19b1711b/s/b10x/connectors/connectors.sock
~/.cache/cw6/e/t19b1711b/s/b10x/connectors/identity-sessions.json
~/.cache/cw6/e/t19b1711c/c/b10x/connectors.toml
~/.cache/cw6/e/t19b1711c/s/b10x/connectors/connectors.sock
~/.cache/cw6/e/t19b1711c/s/b10x/connectors/identity-sessions.json
~/.cache/cw6/e/t19b1711d/c/b10x/connectors.toml
~/.cache/cw6/e/t19b1711d/s/b10x/connectors/connectors.sock
~/.cache/cw6/e/t19b1711d/s/b10x/connectors/identity-sessions.json
~/.cache/cw6/e/t19b1711e/c/b10x/connectors.toml
~/.cache/cw6/e/t19b1711e/s/b10x/connectors/connectors.sock
~/.cache/cw6/e/t19b1711e/s/b10x/connectors/identity-sessions.json
~/.cache/cw6/e/t19b1711f/c/b10x/connectors.toml
~/.cache/cw6/e/t19b1711f/s/b10x/connectors/connectors.sock
~/.cache/cw6/e/t19b1711f/s/b10x/connectors/identity-sessions.json
~/.cache/cw6/e/t19b1712/c/b10x/connectors.toml
~/.cache/cw6/e/t19b1712/s/b10x/connectors/identity-sessions.json
~/.cache/cw6/e/t19b17120/c/b10x/connectors.toml
~/.cache/cw6/e/t19b17120/s/b10x/connectors/connectors.sock
~/.cache/cw6/e/t19b17120/s/b10x/connectors/identity-sessions.json
~/.cache/cw6/e/t19b17121/c/b10x/connectors.toml
~/.cache/cw6/e/t19b17121/s/b10x/connectors/connectors.sock
~/.cache/cw6/e/t19b17121/s/b10x/connectors/identity-sessions.json
~/.cache/cw6/e/t19b17122/c/b10x/connectors.toml
~/.cache/cw6/e/t19b17122/s/b10x/connectors/connectors.sock
~/.cache/cw6/e/t19b17122/s/b10x/connectors/identity-sessions.json
~/.cache/cw6/e/t19b17123/c/b10x/connectors.toml
~/.cache/cw6/e/t19b17123/s/b10x/connectors/connectors.sock
~/.cache/cw6/e/t19b17123/s/b10x/connectors/identity-sessions.json
~/.cache/cw6/e/t19b17124/c/b10x/connectors.toml
~/.cache/cw6/e/t19b17124/s/b10x/connectors/connectors.sock
~/.cache/cw6/e/t19b17124/s/b10x/connectors/identity-sessions.json
~/.cache/cw6/e/t19b17125/c/b10x/connectors.toml
~/.cache/cw6/e/t19b17125/s/b10x/connectors/connectors.sock
~/.cache/cw6/e/t19b17125/s/b10x/connectors/identity-sessions.json
~/.cache/cw6/e/t19b17126/c/b10x/connectors.toml
~/.cache/cw6/e/t19b17126/s/b10x/connectors/connectors.sock
~/.cache/cw6/e/t19b17126/s/b10x/connectors/identity-sessions.json
~/.cache/cw6/e/t19b17127/c/b10x/connectors.toml
~/.cache/cw6/e/t19b17127/s/b10x/connectors/connectors.sock
~/.cache/cw6/e/t19b17127/s/b10x/connectors/identity-sessions.json
~/.cache/cw6/e/t19b17128/c/b10x/connectors.toml
~/.cache/cw6/e/t19b17128/s/b10x/connectors/connectors.sock
~/.cache/cw6/e/t19b17128/s/b10x/connectors/identity-sessions.json
~/.cache/cw6/e/t19b17129/c/b10x/connectors.toml
~/.cache/cw6/e/t19b17129/s/b10x/connectors/connectors.sock
~/.cache/cw6/e/t19b17129/s/b10x/connectors/identity-sessions.json
~/.cache/cw6/e/t19b1712a/c/b10x/connectors.toml
~/.cache/cw6/e/t19b1712a/s/b10x/connectors/connectors.sock
~/.cache/cw6/e/t19b1712a/s/b10x/connectors/identity-sessions.json
~/.cache/cw6/e/t19b1713/c/b10x/connectors.toml
~/.cache/cw6/e/t19b1713/s/b10x/connectors/connectors.sock
~/.cache/cw6/e/t19b1713/s/b10x/connectors/identity-sessions.json
~/.cache/cw6/e/t19b1714/c/b10x/connectors.toml
~/.cache/cw6/e/t19b1714/s/b10x/connectors/connectors.sock
~/.cache/cw6/e/t19b1714/s/b10x/connectors/identity-sessions.json
~/.cache/cw6/e/t19b1715/c/b10x/connectors.toml
~/.cache/cw6/e/t19b1716/c/b10x/connectors.toml
~/.cache/cw6/e/t19b1716/connectors.sock
~/.cache/cw6/e/t19b1716/s/b10x/connectors/identity-sessions.json
~/.cache/cw6/e/t19b1717/c/b10x/connectors.toml
~/.cache/cw6/e/t19b1717/s/b10x/connectors/connectors.sock
~/.cache/cw6/e/t19b1717/s/b10x/connectors/identity-sessions.json
~/.cache/cw6/e/t19b1718/c/b10x/connectors.toml
~/.cache/cw6/e/t19b1719/c/b10x/connectors.toml
~/.cache/cw6/e/t19b171a/c/b10x/connectors.toml
~/.cache/cw6/e/t19b171b/c/b10x/connectors.toml
~/.cache/cw6/e/t19b171c/c/b10x/connectors.toml
~/.cache/cw6/e/t19b171c/connectors.sock
~/.cache/cw6/e/t19b171c/s/b10x/connectors/identity-sessions.json
~/.cache/cw6/e/t19b171d/c/b10x/connectors.toml
~/.cache/cw6/e/t19b171d/s/b10x/connectors/connectors.sock
~/.cache/cw6/e/t19b171d/s/b10x/connectors/identity-sessions.json
~/.cache/cw6/e/t19b171e/c/b10x/connectors.toml
~/.cache/cw6/e/t19b171e/s/b10x/connectors/connectors.sock
~/.cache/cw6/e/t19b171e/s/b10x/connectors/identity-sessions.json
~/.cache/cw6/e/t19b171f/c/b10x/connectors.toml
~/.cache/cw6/e/t19b171f/connectors.sock
~/.cache/cw6/e/t19b171f/s/b10x/connectors/identity-sessions.json
```

Transient closed-pipe fixtures are created under `~/.cache/cw6/e/pipe-<test-process-id-in-hex>-<counter-in-hex>/`. Their existing Drop removes only that fixture directory. Depending on the case, they contain `connectors.toml`, `connectors.sock`, `synthetic-access-token`, `synthetic-secret`, `not-a-directory`, and the new init case's `synthetic-kubeconfig`, `initialized.toml.staged` then `initialized.toml`. Exact transient process/counter expansions were not persisted; the fixture class is declared rather than counted as retained paths. Other package tests use the same TMPDIR; all newly retained paths are listed above. No manual fixture, worktree or build cleanup occurred.

Cargo output remained under `~/.local/state/worktree/trees/b10x/connectors/wt-55b6348b9982/crates/connectors-cli/target`. CARGO_TARGET_DIR was unset. Cargo's shared cache under `~/.cargo` and the configured sccache service are tool-managed cache classes; they were not redirected or cleaned. Scratch report assembly used an inline Python command outside repository source. The last disk check reported 43 GB free, above the coordinator's 20 GB floor.

The companion public report replaces only `~` with `~` mechanically. No other text, measured output, count or status differs.

```findings
[]
```
