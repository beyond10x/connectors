---
format: aep.planning-md/1
id: review-result:cli-target-adversary-2-20260906
kind: review-result
status: active
title: Target selection adversary pass 2
refs:
- provider: git
  reference: 58cc5d22d8215712d2408b8971c7b605f4b6a9a2
relations:
- reviews: story:explicit-target-never-implicit
revision: 1
---
unit: story:explicit-target-never-implicit at 58cc5d22d8215712d2408b8971c7b605f4b6a9a2
verdict: nothing found
cases: executed 168→171, red 0
origin: introduced 0 / pre-existing 0 / undecided 0
wrote-outside-worktree: 12 report/log files; 408 retained fixture paths; tool-managed temporary files and sccache
needs-coordinator: none

```text
 crates/connectors-cli/tests/cli_surface.rs | 255 +++++++++++++++++++++++++++++
 1 file changed, 255 insertions(+)
```

Public report privacy: report.md mechanically replaces only the local home prefix with ~. raw-report.md retains the original paths and verbatim output. Commands, outcomes, counts, findings, and repository-relative paths are otherwise unchanged.

## 2. Cases written before execution

Case: `crates/connectors-cli/tests/cli_surface.rs::all_target_conflicts_precede_local_and_hosted_state_access`. Every one of the 12 dual-target leaves refuses both local-only options, with --target=hosted before or after the leaf, before opening poisoned hosted/local state or creating the selected state path. Green.

Command: `env -u CARGO_TARGET_DIR TMPDIR=~/.cache/cw6/t RUSTC_WRAPPER=/usr/bin/sccache CARGO_INCREMENTAL=0 CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 CARGO_BUILD_JOBS=3 cargo test --manifest-path crates/connectors-cli/Cargo.toml -p connectors-cli --locked --test cli_surface all_target_conflicts_precede_local_and_hosted_state_access -- --exact`

```text
warning: connector-secrets@0.6.5: no live Vault was offered (CONNECTOR_SECRETS_VAULT_ADDR and CONNECTOR_SECRETS_VAULT_TOKEN are unset), so the reqwest HttpTransport is UNEXERCISED by this build: tests/vault_live.rs is compiled #[ignore]d
   Compiling connectors-cli v0.6.5 (~/.local/state/worktree/trees/b10x/connectors/wt-09950110d866/crates/connectors-cli)
    Finished `test` profile [unoptimized] target(s) in 7.24s
     Running tests/cli_surface.rs (crates/connectors-cli/target/debug/deps/cli_surface-a182753279fee22f)

running 1 test
test all_target_conflicts_precede_local_and_hosted_state_access ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 33 filtered out; finished in 0.30s

```

Exit status: 0.

Case: `crates/connectors-cli/tests/cli_surface.rs::broken_explicit_hosted_selection_never_falls_back_to_a_local_listener`. Every explicit hosted leaf reports the hosted identity-state error when a valid local socket listener is available. The listener receives zero requests. Green.

Command: `env -u CARGO_TARGET_DIR TMPDIR=~/.cache/cw6/t RUSTC_WRAPPER=/usr/bin/sccache CARGO_INCREMENTAL=0 CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 CARGO_BUILD_JOBS=3 cargo test --manifest-path crates/connectors-cli/Cargo.toml -p connectors-cli --locked --test cli_surface broken_explicit_hosted_selection_never_falls_back_to_a_local_listener -- --exact`

```text
warning: connector-secrets@0.6.5: no live Vault was offered (CONNECTOR_SECRETS_VAULT_ADDR and CONNECTOR_SECRETS_VAULT_TOKEN are unset), so the reqwest HttpTransport is UNEXERCISED by this build: tests/vault_live.rs is compiled #[ignore]d
    Finished `test` profile [unoptimized] target(s) in 0.26s
     Running tests/cli_surface.rs (crates/connectors-cli/target/debug/deps/cli_surface-a182753279fee22f)

running 1 test
test broken_explicit_hosted_selection_never_falls_back_to_a_local_listener ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 33 filtered out; finished in 0.08s

```

Exit status: 0.

Case: `crates/connectors-cli/tests/cli_surface.rs::selected_target_preserves_provider_owned_target_fields_in_every_renderer`. Successful operation output preserves nested provider-owned target fields and its audit reference while the outer result names local; JSON, YAML, text and compact are exercised. Green.

Command: `env -u CARGO_TARGET_DIR TMPDIR=~/.cache/cw6/t RUSTC_WRAPPER=/usr/bin/sccache CARGO_INCREMENTAL=0 CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 CARGO_BUILD_JOBS=3 cargo test --manifest-path crates/connectors-cli/Cargo.toml -p connectors-cli --locked --test cli_surface selected_target_preserves_provider_owned_target_fields_in_every_renderer -- --exact`

```text
    Blocking waiting for file lock on package cache
    Blocking waiting for file lock on package cache
warning: connector-secrets@0.6.5: no live Vault was offered (CONNECTOR_SECRETS_VAULT_ADDR and CONNECTOR_SECRETS_VAULT_TOKEN are unset), so the reqwest HttpTransport is UNEXERCISED by this build: tests/vault_live.rs is compiled #[ignore]d
    Finished `test` profile [unoptimized] target(s) in 0.32s
     Running tests/cli_surface.rs (crates/connectors-cli/target/debug/deps/cli_surface-a182753279fee22f)

running 1 test
test selected_target_preserves_provider_owned_target_fields_in_every_renderer ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 33 filtered out; finished in 0.03s

```

Exit status: 0.

## 3. Affected suites after the deciding cases

Before count 168 comes from the correction report (81 CLI, 87 console). After count is 171 (84 CLI, 87 console). All three new cases were selected alone before either whole package suite ran. The retained first-pass file/stdin failures and correction inline/missing-input matrix also passed in this run.

Command: `env -u CARGO_TARGET_DIR TMPDIR=~/.cache/cw6/t RUSTC_WRAPPER=/usr/bin/sccache CARGO_INCREMENTAL=0 CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 CARGO_BUILD_JOBS=3 cargo test --manifest-path crates/connectors-cli/Cargo.toml -p connectors-cli --locked --no-fail-fast`

```text
    Blocking waiting for file lock on package cache
warning: connector-secrets@0.6.5: no live Vault was offered (CONNECTOR_SECRETS_VAULT_ADDR and CONNECTOR_SECRETS_VAULT_TOKEN are unset), so the reqwest HttpTransport is UNEXERCISED by this build: tests/vault_live.rs is compiled #[ignore]d
    Finished `test` profile [unoptimized] target(s) in 0.46s
     Running unittests src/lib.rs (crates/connectors-cli/target/debug/deps/connectors_cli-38a26c90d743ef42)

running 5 tests
test tests::slack_connect_needs_no_internal_reference_or_path_argument ... ok
test tests::kubernetes_connect_accepts_an_exact_context_selection ... ok
test tests::grafana_connect_uses_the_same_guided_surface ... ok
test tests::normal_help_exposes_the_guided_flow_and_hides_acquisition_plumbing ... ok
test tests::every_supported_shell_gets_a_script_naming_the_whole_surface ... ok

test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s

     Running unittests src/main.rs (crates/connectors-cli/target/debug/deps/connectors-23e728bf6e359b3b)

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests/adversary_cli_cap_pass3.rs (crates/connectors-cli/target/debug/deps/adversary_cli_cap_pass3-728995953b17e671)

running 1 test
test the_cap_the_design_page_says_is_measured_is_declared_and_asserted ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests/adversary_fence_probe.rs (crates/connectors-cli/target/debug/deps/adversary_fence_probe-7703694c004a0637)

running 6 tests
test the_wire_name_rule_citation_in_the_design_document_points_at_the_rule ... ok
test the_wire_name_rule_citation_in_the_specification_points_at_the_rule ... ok
test the_typeable_words_are_the_words_the_design_document_names ... ok
test the_copies_this_probe_carries_are_still_copies ... ok
test a_forwarding_reason_that_names_no_command_is_refused_whatever_kind_it_carries ... ok
test every_entry_that_is_not_a_lifecycle_step_is_refused_when_it_claims_to_be_one ... ok

test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.12s

     Running tests/adversary_fence_probe_pass2.rs (crates/connectors-cli/target/debug/deps/adversary_fence_probe_pass2-329b7d26d642c510)

running 3 tests
test every_file_the_committed_contract_opens_is_a_file_a_clone_has ... ok
test the_kinds_the_design_document_says_rest_on_no_sentence_rest_on_no_sentence ... ok
test the_paths_the_design_document_says_send_no_protocol_request_send_none ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests/adversary_shim_pass3.rs (crates/connectors-cli/target/debug/deps/adversary_shim_pass3-ef2c4ea286dd4433)

running 6 tests
test the_serve_group_advertises_a_help_subcommand ... ok
test a_help_path_of_the_new_tree_under_serve_is_left_alone ... ok
test connectors_help_still_answers_for_a_path_that_moved ... ok
test a_moved_path_typed_with_the_global_output_flag_still_works ... ok
test the_group_word_whose_only_command_moved_still_points_somewhere ... ok
test nothing_this_product_prints_names_a_moved_path_behind_a_global_flag ... ok

test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.17s

     Running tests/adversary_shim_pass4.rs (crates/connectors-cli/target/debug/deps/adversary_shim_pass4-e90fa4ddec4cdde1)

running 3 tests
test the_table_this_suite_copies_by_hand_is_the_table_the_binary_ships ... ok
test the_auth_group_still_answers_the_help_subcommand_it_advertised ... ok
test a_two_word_path_that_moved_works_with_the_global_flag_between_its_words ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.05s

     Running tests/adversary_shim_pass5.rs (crates/connectors-cli/target/debug/deps/adversary_shim_pass5-d7babecf7c496bb5)

running 4 tests
test a_positional_value_spelled_help_is_a_value_not_a_help_request ... ok
test an_argument_neither_the_group_nor_the_leaf_declares_is_not_the_old_leaf ... ok
test the_double_dash_escape_is_not_a_word_that_moved ... ok
test serve_with_only_global_options_is_the_group_in_every_spelling_and_position ... ok

test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.05s

     Running tests/cli_surface.rs (crates/connectors-cli/target/debug/deps/cli_surface-a182753279fee22f)

running 34 tests
test every_kind_of_exception_is_used_and_every_entry_gives_a_reason ... ok
test every_named_exception_is_still_a_path_of_the_parser ... ok
test no_word_of_the_parser_answers_to_a_name_the_specification_cannot_declare ... ok
test every_declared_group_is_a_group_of_the_parser ... ok
test every_declared_group_help_line_is_the_summary_the_specification_declares ... ok
test no_path_is_both_declared_and_excepted ... ok
test every_path_of_the_parser_is_declared_or_a_named_exception ... ok
test the_committed_generated_tree_is_the_specification_word_for_word ... ok
test every_declaration_the_adversary_probe_copies_is_still_a_copy ... ok
test the_old_login_selected_target_guard_is_absent ... ok
test every_citation_this_unit_wrote_resolves ... ok
test the_read_verb_enumeration_partitions_the_protocols_it_names ... ok
test the_parser_accepts_target_before_and_after_each_dual_target_leaf ... ok
test the_regeneration_command_the_documents_name_is_the_one_the_gate_runs ... ok
test a_read_stops_being_an_exception_once_the_specification_declares_a_view ... ok
test the_specification_names_the_binary_the_parser_builds ... ok
test the_target_countdown_is_exactly_what_the_parser_still_owes ... ok
test a_command_absorbed_into_the_exception_list_alone_is_refused ... ok
test the_exception_list_is_the_set_the_specification_enumerates ... ok
test every_citation_that_names_a_symbol_lands_on_its_declaration ... ok
test target_conflict_does_not_wait_for_open_stdin ... ok
test target_conflict_precedes_invoke_payload_loading ... ok
test targeted_errors_keep_the_target_in_yaml_and_text ... ok
test an_explicit_hosted_target_requires_a_login_by_name_for_every_group ... ok
test an_omitted_target_ignores_a_saved_login_for_every_dual_target_group ... ok
test target_conflict_precedes_missing_or_malformed_inline_input ... ok
test an_exception_whose_kind_the_tree_contradicts_is_refused ... ok
test hosted_refuses_each_local_only_option_for_every_group ... ok
test the_kinds_the_tree_derives_are_the_kinds_the_list_carries ... ok
test selected_target_preserves_provider_owned_target_fields_in_every_renderer ... ok
test broken_explicit_hosted_selection_never_falls_back_to_a_local_listener ... ok
test local_success_and_protocol_refusals_report_the_selected_target ... ok
test all_target_conflicts_precede_local_and_hosted_state_access ... ok
test every_local_leaf_ignores_broken_login_metadata_and_preserves_its_request ... ok

test result: ok. 34 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.32s

     Running tests/cli_surface_drift.rs (crates/connectors-cli/target/debug/deps/cli_surface_drift-54c6716a31d65891)

running 10 tests
test the_copied_declarations_are_still_copies ... ok
test the_thin_frontend_citation_points_at_the_thin_frontend_test ... ok
test a_command_added_under_a_declared_group_is_refused ... ok
test a_committed_tree_whose_group_about_no_longer_matches_the_specification_is_refused ... ok
test a_committed_tree_that_swaps_completions_for_an_undeclared_word_is_refused ... ok
test a_command_added_under_the_wrong_declared_group_is_refused ... ok
test cutting_the_admin_group_over_to_the_generated_tree_is_refused ... ok
test the_restated_contract_is_green_against_the_unchanged_tree ... ok
test a_target_flag_removed_from_a_group_is_refused_by_the_countdown ... ok
test cargo_can_read_the_committed_emitted_manifest ... ok

test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s

     Running tests/cli_surface_pass_two.rs (crates/connectors-cli/target/debug/deps/cli_surface_pass_two-e5ac027a941a00de)

running 6 tests
test the_design_document_names_only_constants_that_exist ... ok
test the_design_document_describes_the_countdown_assertion_the_contract_makes ... ok
test the_design_document_states_the_shape_of_the_exception_list ... ok
test the_target_countdown_candidates_are_derived_from_every_protocol_a_deployment_answers ... ok
test the_drift_suites_copies_are_checked_rather_than_cited ... ok
test the_drift_suite_attributes_nothing_to_the_contract_that_is_not_there ... ok

test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests/first_level_groups.rs (crates/connectors-cli/target/debug/deps/first_level_groups-3efdb248e70da056)

running 5 tests
test the_first_level_is_eight_words ... ok
test doctor_reports_the_same_installation_at_both_paths ... ok
test a_path_of_the_new_tree_is_left_alone ... ok
test the_serve_group_answers_bare_and_with_help_like_the_other_groups ... ok
test every_moved_path_still_works_and_names_where_it_went ... ok

test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.14s

     Running tests/moved_paths_are_not_taught.rs (crates/connectors-cli/target/debug/deps/moved_paths_are_not_taught-d9a0438e91f74c90)

running 1 test
test nothing_this_product_prints_names_a_path_that_moved ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.12s

   Doc-tests connectors_cli

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

```

Exit status: 0.

Command: `env -u CARGO_TARGET_DIR TMPDIR=~/.cache/cw6/t RUSTC_WRAPPER=/usr/bin/sccache CARGO_INCREMENTAL=0 CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 CARGO_BUILD_JOBS=3 cargo test --manifest-path crates/connectors-console/Cargo.toml -p connectors-console --locked --no-fail-fast`

```text
    Blocking waiting for file lock on package cache
    Finished `test` profile [unoptimized] target(s) in 0.58s
     Running unittests src/lib.rs (crates/connectors-console/target/debug/deps/connectors_console-04d869bc2d5c0fc0)

running 72 tests
test auth::tests::a_basic_credential_row_reports_whether_its_user_half_is_configured_and_never_the_value ... ok
test auth::tests::nothing_in_the_result_can_carry_a_secret ... ok
test admin::tests::explicit_secret_file_must_be_owner_only ... ok
test connect::tests::a_provider_outside_the_guided_set_is_refused_by_name ... ok
test doctor::tests::a_missing_configuration_is_fatal_and_names_the_command_that_fixes_it ... ok
test connect::tests::the_error_for_an_unknown_provider_names_it ... ok
test auth::tests::the_store_preference_matches_what_the_runtime_composes ... ok
test doctor::tests::a_report_is_unhealthy_only_when_something_cannot_work ... ok
test doctor::tests::a_short_state_root_passes_both_budgets ... ok
test doctor::tests::the_report_renders_every_check_as_data ... ok
test enrol::tests::a_provider_outside_the_catalogue_is_named_rather_than_guessed_at ... ok
test doctor::tests::doctor_names_the_default_local_target_and_its_socket ... ok
test doctor::tests::the_budget_is_measured_against_the_deepest_path_the_daemon_binds ... ok
test doctor::tests::every_state_a_check_can_report_reaches_the_reader_as_its_own_marker ... ok
test connect::tests::a_catalogued_provider_whose_curated_backend_is_absent_takes_the_catalogue_path ... ok
test envelope::tests::a_refusal_becomes_an_error_rather_than_a_result ... ok
test envelope::tests::an_envelope_carrying_neither_is_a_named_failure_not_an_empty_success ... ok
test envelope::tests::a_result_loses_its_envelope_and_its_discriminant ... ok
test init::tests::an_agent_id_is_stable_across_calls ... ok
test init::tests::admitting_a_credential_plugin_is_a_choice_and_its_absence_is_explained ... ok
test init::tests::the_separator_keeps_a_concatenation_from_colliding ... ok
test admin::tests::command_shape_accepts_secret_stdin_without_a_secret_argument ... ok
test input::tests::an_inline_object_is_parsed ... ok
test input::tests::no_source_names_all_three_rather_than_defaulting_to_empty ... ok
test init::tests::the_snapshot_digest_is_stable_and_moves_with_the_admitted_set ... ok
test input::tests::input_accepts_only_the_stdin_marker ... ok
test output::tests::a_payload_carrying_its_own_value_field_is_left_alone ... ok
test init::tests::an_existing_configuration_is_never_replaced_silently ... ok
test output::tests::a_record_that_is_not_an_object_keeps_the_name_the_report_gave_it ... ok
test output::tests::a_field_a_record_does_not_carry_reads_as_absent_rather_than_blank ... ok
test input::tests::a_file_is_read_from_its_path ... ok
test output::tests::a_table_reads_left_to_right_with_the_column_that_runs_long_last ... ok
test output::tests::a_structured_format_carries_its_failure_on_stdout ... ok
test output::tests::a_word_the_renderer_cannot_rank_is_marked_unknown_rather_than_good ... ok
test output::tests::an_empty_listing_is_an_empty_stream_rather_than_a_line_shaped_like_a_record ... ok
test output::tests::an_object_with_two_arrays_is_not_unwrapped ... ok
test output::tests::a_row_shows_its_severity_before_anybody_reads_it ... ok
test output::tests::a_wide_character_cell_keeps_the_column_after_it_aligned ... ok
test output::tests::columns_of_equal_width_keep_the_order_the_record_carries ... ok
test output::tests::compact_leaves_a_single_record_as_one_line ... ok
test output::tests::an_unranked_table_still_keeps_the_marker_column ... ok
test output::tests::compact_keeps_the_scalar_a_list_response_carries_beside_its_records ... ok
test output::tests::compact_keeps_a_field_a_record_carries_below_its_top_level ... ok
test output::tests::compact_unwraps_the_one_array_a_list_response_carries ... ok
test output::tests::every_protocol_state_this_renderer_can_be_handed_has_a_rank ... ok
test output::tests::no_cell_is_ever_empty_so_no_row_can_end_in_whitespace ... ok
test output::tests::text_does_not_quote_a_string_a_person_is_reading ... ok
test output::tests::severity_survives_a_pipe_because_it_is_not_carried_by_colour ... ok
test output::tests::text_says_none_rather_than_printing_an_empty_bracket ... ok
test output::tests::the_result_discriminant_is_stripped_so_compact_can_see_the_records ... ok
test output::tests::text_spends_one_aligned_row_on_each_record ... ok
test output::tests::text_keeps_every_field_a_record_carries_including_a_nested_list ... ok
test output::tests::the_structured_formats_render_the_bytes_they_rendered_before ... ok
test output::tests::the_widest_column_moves_last_even_when_the_record_puts_it_first ... ok
test output::tests::yaml_renders_through_the_maintained_crate ... ok
test output::tests::a_cell_never_carries_a_character_that_breaks_the_row ... ok
test init::tests::a_configuration_the_daemon_would_refuse_is_not_left_on_disk ... ok
test output::tests::every_status_word_this_package_emits_is_one_the_renderer_can_rank ... ok
test init::tests::what_init_writes_is_what_the_daemon_can_read ... ok
test auth::tests::the_catalogue_is_what_says_a_credential_has_a_user_half ... ok
test enrol::tests::a_self_hosted_origin_is_the_case_operator_approval_exists_for ... ok
test enrol::tests::gitlab_asks_for_nothing_when_its_default_origin_is_wanted ... ok
test enrol::tests::most_of_the_catalogue_asks_no_configuration_question_at_all ... ok
test enrol::tests::slack_declares_a_bot_and_a_user_credential_which_one_identity_may_both_hold ... ok
test providers::tests::a_provider_without_a_probe_is_not_ready_and_says_why_by_omission ... ok
test providers::tests::an_unmatched_query_is_an_empty_listing_rather_than_the_whole_catalogue ... ok
test providers::tests::a_query_narrows_to_one_provider_and_its_summary_follows ... ok
test providers::tests::the_shipped_catalogue_is_reported_rather_than_asserted ... ok
test output::tests::a_table_too_wide_for_a_terminal_starts_its_last_column_inside_the_budget ... ok
test output::tests::the_budget_is_documented_as_what_it_is_and_a_real_row_is_wider_than_it ... ok
test output::tests::two_providers_that_differ_in_their_id_differ_on_screen ... ok
test output::tests::a_cell_the_budget_cut_says_so_and_the_column_names_are_cut_last ... ok

test result: ok. 72 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.01s

     Running tests/adversary_budget_prose.rs (crates/connectors-console/target/debug/deps/adversary_budget_prose-e698524ec7985705)

running 3 tests
test pass3_render_helper_child ... ok
test the_quoted_module_header_sentence_is_at_the_line_the_pass_two_suite_cites ... ok
test the_widths_the_documents_state_are_the_widths_the_renderer_prints ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.05s

     Running tests/adversary_readability.rs (crates/connectors-console/target/debug/deps/adversary_readability-5df21c78ad04b16c)

running 7 tests
test render_helper_child ... ok
test a_record_whose_cells_are_all_empty_is_rendered_as_a_blank_line ... ok
test an_unranked_table_lets_a_cell_sit_where_the_severity_marker_sits ... ok
test a_wide_character_cell_leaves_the_column_after_it_ragged ... ok
test doctor_spreads_one_check_over_several_unmarked_lines_when_the_configuration_is_malformed ... ok
test compact_no_longer_puts_one_record_on_every_line ... ok
test providers_starts_its_last_column_past_the_width_of_any_terminal ... ok

test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.02s

     Running tests/adversary_readability_pass2.rs (crates/connectors-console/target/debug/deps/adversary_readability_pass2-558df7c3d207b050)

running 5 tests
test pass2_render_helper_child ... ok
test compact_drops_the_name_of_the_array_a_report_carries ... ok
test a_column_the_budget_squeezes_to_nothing_pushes_every_later_column_out_of_line ... ok
test compact_answers_an_empty_listing_with_a_line_that_is_not_a_record ... ok
test the_last_column_of_providers_begins_one_column_past_the_terminal_it_is_laid_out_for ... ok

test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.05s

   Doc-tests connectors_console

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

```

Exit status: 0.

Command: `env -u CARGO_TARGET_DIR TMPDIR=~/.cache/cw6/t RUSTC_WRAPPER=/usr/bin/sccache CARGO_INCREMENTAL=0 CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 CARGO_BUILD_JOBS=3 cargo clippy --manifest-path crates/connectors-cli/Cargo.toml -p connectors-cli --all-targets --locked -- -D warnings`

```text
warning: connector-secrets@0.6.5: no live Vault was offered (CONNECTOR_SECRETS_VAULT_ADDR and CONNECTOR_SECRETS_VAULT_TOKEN are unset), so the reqwest HttpTransport is UNEXERCISED by this build: tests/vault_live.rs is compiled #[ignore]d
    Checking connectors-cli v0.6.5 (~/.local/state/worktree/trees/b10x/connectors/wt-09950110d866/crates/connectors-cli)
    Finished `dev` profile [unoptimized] target(s) in 4.12s
```

Exit status: 0.

Command: `cargo fmt --manifest-path crates/connectors-cli/Cargo.toml -p connectors-cli -- --check`

```text
```

Exit status: 0.

## 4. Findings

Nothing found against the corrected reviewed commit and the test-only working-tree additions.

## 5. Attack coverage

- Correction class: every typed leaf, both local-only options, both target positions, poisoned configuration/login, and absence of unwanted state creation.
- Target routing: explicit hosted metadata failure never falls back to an available local listener; inherited cases cover omitted/explicit local requests despite poisoned hosted metadata.
- Result rendering: provider-owned target fields survive all four renderers; outer target remains local and structured output preserves the audit reference.
- Boundary: no real Identity/keyring or hosted endpoint was exercised; this pass used isolated account/config fixtures and local Unix sockets only.

## 6. Paths written outside the worktree

Build products remain in this managed tree's package target directories. Test-created temporary files use ~/.cache/cw6/t; tempfile-owned transient paths are removed by those existing tests. Compiler cache files are tool-managed below ~/.cache/sccache. No operator login or configuration was read.

Report/log paths:

```text
~/.cache/connectors-cli-wave-20260906/explicit-target-never-implicit/adversary-2/fixture-before.txt
~/.cache/connectors-cli-wave-20260906/explicit-target-never-implicit/adversary-2/conflict-case.log
~/.cache/connectors-cli-wave-20260906/explicit-target-never-implicit/adversary-2/hosted-case.log
~/.cache/connectors-cli-wave-20260906/explicit-target-never-implicit/adversary-2/renderer-case.log
~/.cache/connectors-cli-wave-20260906/explicit-target-never-implicit/adversary-2/cli-suite.log
~/.cache/connectors-cli-wave-20260906/explicit-target-never-implicit/adversary-2/console-suite.log
~/.cache/connectors-cli-wave-20260906/explicit-target-never-implicit/adversary-2/clippy.log
~/.cache/connectors-cli-wave-20260906/explicit-target-never-implicit/adversary-2/fmt.log
~/.cache/connectors-cli-wave-20260906/explicit-target-never-implicit/adversary-2/fixture-after.txt
~/.cache/connectors-cli-wave-20260906/explicit-target-never-implicit/adversary-2/diff-stat.txt
~/.cache/connectors-cli-wave-20260906/explicit-target-never-implicit/adversary-2/raw-report.md
~/.cache/connectors-cli-wave-20260906/explicit-target-never-implicit/adversary-2/report.md
```

Retained fixture paths first appearing during this pass:

```text
~/.cache/cw6/t/tc8aa60
~/.cache/cw6/t/tc8aa60/c
~/.cache/cw6/t/tc8aa60/c/b10x
~/.cache/cw6/t/tc8aa60/c/b10x/connectors.toml
~/.cache/cw6/t/tc8aa60/s
~/.cache/cw6/t/tc8aa60/s/b10x
~/.cache/cw6/t/tc8aa60/s/b10x/connectors
~/.cache/cw6/t/tc8aa60/s/b10x/connectors/identity-sessions.json
~/.cache/cw6/t/tc99420
~/.cache/cw6/t/tc99420/c
~/.cache/cw6/t/tc99420/c/b10x
~/.cache/cw6/t/tc99420/c/b10x/connectors.toml
~/.cache/cw6/t/tc99420/s
~/.cache/cw6/t/tc99420/s/b10x
~/.cache/cw6/t/tc99420/s/b10x/connectors
~/.cache/cw6/t/tc99420/s/b10x/connectors/connectors.sock
~/.cache/cw6/t/tc99420/s/b10x/connectors/identity-sessions.json
~/.cache/cw6/t/tc9b3b0
~/.cache/cw6/t/tc9b3b0/c
~/.cache/cw6/t/tc9b3b0/c/b10x
~/.cache/cw6/t/tc9b3b0/c/b10x/connectors.toml
~/.cache/cw6/t/tc9b3b0/s
~/.cache/cw6/t/tc9b3b0/s/b10x
~/.cache/cw6/t/tc9b3b0/s/b10x/connectors
~/.cache/cw6/t/tc9b3b0/s/b10x/connectors/connectors.sock
~/.cache/cw6/t/tc9b3b0/s/b10x/connectors/identity-sessions.json
~/.cache/cw6/t/tc9b3b1
~/.cache/cw6/t/tc9b3b1/c
~/.cache/cw6/t/tc9b3b1/c/b10x
~/.cache/cw6/t/tc9b3b1/c/b10x/connectors.toml
~/.cache/cw6/t/tc9b3b1/s
~/.cache/cw6/t/tc9b3b1/s/b10x
~/.cache/cw6/t/tc9b3b1/s/b10x/connectors
~/.cache/cw6/t/tc9b3b1/s/b10x/connectors/connectors.sock
~/.cache/cw6/t/tc9b3b1/s/b10x/connectors/identity-sessions.json
~/.cache/cw6/t/tc9b3b2
~/.cache/cw6/t/tc9b3b2/c
~/.cache/cw6/t/tc9b3b2/c/b10x
~/.cache/cw6/t/tc9b3b2/c/b10x/connectors.toml
~/.cache/cw6/t/tc9b3b2/s
~/.cache/cw6/t/tc9b3b2/s/b10x
~/.cache/cw6/t/tc9b3b2/s/b10x/connectors
~/.cache/cw6/t/tc9b3b2/s/b10x/connectors/connectors.sock
~/.cache/cw6/t/tc9b3b2/s/b10x/connectors/identity-sessions.json
~/.cache/cw6/t/tc9b3b3
~/.cache/cw6/t/tc9b3b3/c
~/.cache/cw6/t/tc9b3b3/c/b10x
~/.cache/cw6/t/tc9b3b3/c/b10x/connectors.toml
~/.cache/cw6/t/tc9b3b3/s
~/.cache/cw6/t/tc9b3b3/s/b10x
~/.cache/cw6/t/tc9b3b3/s/b10x/connectors
~/.cache/cw6/t/tc9b3b3/s/b10x/connectors/connectors.sock
~/.cache/cw6/t/tc9b3b3/s/b10x/connectors/identity-sessions.json
~/.cache/cw6/t/tcb6db0
~/.cache/cw6/t/tcb6db0/c
~/.cache/cw6/t/tcb6db0/c/b10x
~/.cache/cw6/t/tcb6db0/c/b10x/connectors.toml
~/.cache/cw6/t/tcb6db0/s
~/.cache/cw6/t/tcb6db0/s/b10x
~/.cache/cw6/t/tcb6db0/s/b10x/connectors
~/.cache/cw6/t/tcb6db0/s/b10x/connectors/identity-sessions.json
~/.cache/cw6/t/tcb6db1
~/.cache/cw6/t/tcb6db1/c
~/.cache/cw6/t/tcb6db1/c/b10x
~/.cache/cw6/t/tcb6db1/c/b10x/connectors.toml
~/.cache/cw6/t/tcb6db10
~/.cache/cw6/t/tcb6db10/c
~/.cache/cw6/t/tcb6db10/c/b10x
~/.cache/cw6/t/tcb6db10/c/b10x/connectors.toml
~/.cache/cw6/t/tcb6db10/connectors.sock
~/.cache/cw6/t/tcb6db10/s
~/.cache/cw6/t/tcb6db10/s/b10x
~/.cache/cw6/t/tcb6db10/s/b10x/connectors
~/.cache/cw6/t/tcb6db10/s/b10x/connectors/identity-sessions.json
~/.cache/cw6/t/tcb6db11
~/.cache/cw6/t/tcb6db11/c
~/.cache/cw6/t/tcb6db11/c/b10x
~/.cache/cw6/t/tcb6db11/c/b10x/connectors.toml
~/.cache/cw6/t/tcb6db11/s
~/.cache/cw6/t/tcb6db11/s/b10x
~/.cache/cw6/t/tcb6db11/s/b10x/connectors
~/.cache/cw6/t/tcb6db11/s/b10x/connectors/connectors.sock
~/.cache/cw6/t/tcb6db11/s/b10x/connectors/identity-sessions.json
~/.cache/cw6/t/tcb6db12
~/.cache/cw6/t/tcb6db12/c
~/.cache/cw6/t/tcb6db12/c/b10x
~/.cache/cw6/t/tcb6db12/c/b10x/connectors.toml
~/.cache/cw6/t/tcb6db12/s
~/.cache/cw6/t/tcb6db12/s/b10x
~/.cache/cw6/t/tcb6db12/s/b10x/connectors
~/.cache/cw6/t/tcb6db12/s/b10x/connectors/connectors.sock
~/.cache/cw6/t/tcb6db12/s/b10x/connectors/identity-sessions.json
~/.cache/cw6/t/tcb6db13
~/.cache/cw6/t/tcb6db13/c
~/.cache/cw6/t/tcb6db13/c/b10x
~/.cache/cw6/t/tcb6db13/c/b10x/connectors.toml
~/.cache/cw6/t/tcb6db13/s
~/.cache/cw6/t/tcb6db13/s/b10x
~/.cache/cw6/t/tcb6db13/s/b10x/connectors
~/.cache/cw6/t/tcb6db13/s/b10x/connectors/connectors.sock
~/.cache/cw6/t/tcb6db13/s/b10x/connectors/identity-sessions.json
~/.cache/cw6/t/tcb6db14
~/.cache/cw6/t/tcb6db14/c
~/.cache/cw6/t/tcb6db14/c/b10x
~/.cache/cw6/t/tcb6db14/c/b10x/connectors.toml
~/.cache/cw6/t/tcb6db14/connectors.sock
~/.cache/cw6/t/tcb6db14/s
~/.cache/cw6/t/tcb6db14/s/b10x
~/.cache/cw6/t/tcb6db14/s/b10x/connectors
~/.cache/cw6/t/tcb6db14/s/b10x/connectors/identity-sessions.json
~/.cache/cw6/t/tcb6db15
~/.cache/cw6/t/tcb6db15/c
~/.cache/cw6/t/tcb6db15/c/b10x
~/.cache/cw6/t/tcb6db15/c/b10x/connectors.toml
~/.cache/cw6/t/tcb6db15/s
~/.cache/cw6/t/tcb6db15/s/b10x
~/.cache/cw6/t/tcb6db15/s/b10x/connectors
~/.cache/cw6/t/tcb6db15/s/b10x/connectors/connectors.sock
~/.cache/cw6/t/tcb6db15/s/b10x/connectors/identity-sessions.json
~/.cache/cw6/t/tcb6db16
~/.cache/cw6/t/tcb6db16/c
~/.cache/cw6/t/tcb6db16/c/b10x
~/.cache/cw6/t/tcb6db16/c/b10x/connectors.toml
~/.cache/cw6/t/tcb6db16/connectors.sock
~/.cache/cw6/t/tcb6db16/s
~/.cache/cw6/t/tcb6db16/s/b10x
~/.cache/cw6/t/tcb6db16/s/b10x/connectors
~/.cache/cw6/t/tcb6db16/s/b10x/connectors/identity-sessions.json
~/.cache/cw6/t/tcb6db17
~/.cache/cw6/t/tcb6db17/c
~/.cache/cw6/t/tcb6db17/c/b10x
~/.cache/cw6/t/tcb6db17/c/b10x/connectors.toml
~/.cache/cw6/t/tcb6db17/s
~/.cache/cw6/t/tcb6db17/s/b10x
~/.cache/cw6/t/tcb6db17/s/b10x/connectors
~/.cache/cw6/t/tcb6db17/s/b10x/connectors/connectors.sock
~/.cache/cw6/t/tcb6db17/s/b10x/connectors/identity-sessions.json
~/.cache/cw6/t/tcb6db18
~/.cache/cw6/t/tcb6db18/c
~/.cache/cw6/t/tcb6db18/c/b10x
~/.cache/cw6/t/tcb6db18/c/b10x/connectors.toml
~/.cache/cw6/t/tcb6db18/connectors.sock
~/.cache/cw6/t/tcb6db18/s
~/.cache/cw6/t/tcb6db18/s/b10x
~/.cache/cw6/t/tcb6db18/s/b10x/connectors
~/.cache/cw6/t/tcb6db18/s/b10x/connectors/identity-sessions.json
~/.cache/cw6/t/tcb6db19
~/.cache/cw6/t/tcb6db19/c
~/.cache/cw6/t/tcb6db19/c/b10x
~/.cache/cw6/t/tcb6db19/c/b10x/connectors.toml
~/.cache/cw6/t/tcb6db19/s
~/.cache/cw6/t/tcb6db19/s/b10x
~/.cache/cw6/t/tcb6db19/s/b10x/connectors
~/.cache/cw6/t/tcb6db19/s/b10x/connectors/connectors.sock
~/.cache/cw6/t/tcb6db19/s/b10x/connectors/identity-sessions.json
~/.cache/cw6/t/tcb6db1a
~/.cache/cw6/t/tcb6db1a/c
~/.cache/cw6/t/tcb6db1a/c/b10x
~/.cache/cw6/t/tcb6db1a/c/b10x/connectors.toml
~/.cache/cw6/t/tcb6db1a/s
~/.cache/cw6/t/tcb6db1a/s/b10x
~/.cache/cw6/t/tcb6db1a/s/b10x/connectors
~/.cache/cw6/t/tcb6db1a/s/b10x/connectors/connectors.sock
~/.cache/cw6/t/tcb6db1a/s/b10x/connectors/identity-sessions.json
~/.cache/cw6/t/tcb6db1b
~/.cache/cw6/t/tcb6db1b/c
~/.cache/cw6/t/tcb6db1b/c/b10x
~/.cache/cw6/t/tcb6db1b/c/b10x/connectors.toml
~/.cache/cw6/t/tcb6db1b/s
~/.cache/cw6/t/tcb6db1b/s/b10x
~/.cache/cw6/t/tcb6db1b/s/b10x/connectors
~/.cache/cw6/t/tcb6db1b/s/b10x/connectors/connectors.sock
~/.cache/cw6/t/tcb6db1b/s/b10x/connectors/identity-sessions.json
~/.cache/cw6/t/tcb6db1c
~/.cache/cw6/t/tcb6db1c/c
~/.cache/cw6/t/tcb6db1c/c/b10x
~/.cache/cw6/t/tcb6db1c/c/b10x/connectors.toml
~/.cache/cw6/t/tcb6db1c/s
~/.cache/cw6/t/tcb6db1c/s/b10x
~/.cache/cw6/t/tcb6db1c/s/b10x/connectors
~/.cache/cw6/t/tcb6db1c/s/b10x/connectors/connectors.sock
~/.cache/cw6/t/tcb6db1c/s/b10x/connectors/identity-sessions.json
~/.cache/cw6/t/tcb6db1d
~/.cache/cw6/t/tcb6db1d/c
~/.cache/cw6/t/tcb6db1d/c/b10x
~/.cache/cw6/t/tcb6db1d/c/b10x/connectors.toml
~/.cache/cw6/t/tcb6db1d/s
~/.cache/cw6/t/tcb6db1d/s/b10x
~/.cache/cw6/t/tcb6db1d/s/b10x/connectors
~/.cache/cw6/t/tcb6db1d/s/b10x/connectors/connectors.sock
~/.cache/cw6/t/tcb6db1d/s/b10x/connectors/identity-sessions.json
~/.cache/cw6/t/tcb6db1e
~/.cache/cw6/t/tcb6db1e/c
~/.cache/cw6/t/tcb6db1e/c/b10x
~/.cache/cw6/t/tcb6db1e/c/b10x/connectors.toml
~/.cache/cw6/t/tcb6db1e/s
~/.cache/cw6/t/tcb6db1e/s/b10x
~/.cache/cw6/t/tcb6db1e/s/b10x/connectors
~/.cache/cw6/t/tcb6db1e/s/b10x/connectors/connectors.sock
~/.cache/cw6/t/tcb6db1e/s/b10x/connectors/identity-sessions.json
~/.cache/cw6/t/tcb6db1f
~/.cache/cw6/t/tcb6db1f/c
~/.cache/cw6/t/tcb6db1f/c/b10x
~/.cache/cw6/t/tcb6db1f/c/b10x/connectors.toml
~/.cache/cw6/t/tcb6db1f/s
~/.cache/cw6/t/tcb6db1f/s/b10x
~/.cache/cw6/t/tcb6db1f/s/b10x/connectors
~/.cache/cw6/t/tcb6db1f/s/b10x/connectors/connectors.sock
~/.cache/cw6/t/tcb6db1f/s/b10x/connectors/identity-sessions.json
~/.cache/cw6/t/tcb6db2
~/.cache/cw6/t/tcb6db2/c
~/.cache/cw6/t/tcb6db2/c/b10x
~/.cache/cw6/t/tcb6db2/c/b10x/connectors.toml
~/.cache/cw6/t/tcb6db2/s
~/.cache/cw6/t/tcb6db2/s/b10x
~/.cache/cw6/t/tcb6db2/s/b10x/connectors
~/.cache/cw6/t/tcb6db2/s/b10x/connectors/identity-sessions.json
~/.cache/cw6/t/tcb6db20
~/.cache/cw6/t/tcb6db20/c
~/.cache/cw6/t/tcb6db20/c/b10x
~/.cache/cw6/t/tcb6db20/c/b10x/connectors.toml
~/.cache/cw6/t/tcb6db20/s
~/.cache/cw6/t/tcb6db20/s/b10x
~/.cache/cw6/t/tcb6db20/s/b10x/connectors
~/.cache/cw6/t/tcb6db20/s/b10x/connectors/connectors.sock
~/.cache/cw6/t/tcb6db20/s/b10x/connectors/identity-sessions.json
~/.cache/cw6/t/tcb6db21
~/.cache/cw6/t/tcb6db21/c
~/.cache/cw6/t/tcb6db21/c/b10x
~/.cache/cw6/t/tcb6db21/c/b10x/connectors.toml
~/.cache/cw6/t/tcb6db21/s
~/.cache/cw6/t/tcb6db21/s/b10x
~/.cache/cw6/t/tcb6db21/s/b10x/connectors
~/.cache/cw6/t/tcb6db21/s/b10x/connectors/connectors.sock
~/.cache/cw6/t/tcb6db21/s/b10x/connectors/identity-sessions.json
~/.cache/cw6/t/tcb6db22
~/.cache/cw6/t/tcb6db22/c
~/.cache/cw6/t/tcb6db22/c/b10x
~/.cache/cw6/t/tcb6db22/c/b10x/connectors.toml
~/.cache/cw6/t/tcb6db22/s
~/.cache/cw6/t/tcb6db22/s/b10x
~/.cache/cw6/t/tcb6db22/s/b10x/connectors
~/.cache/cw6/t/tcb6db22/s/b10x/connectors/connectors.sock
~/.cache/cw6/t/tcb6db22/s/b10x/connectors/identity-sessions.json
~/.cache/cw6/t/tcb6db23
~/.cache/cw6/t/tcb6db23/c
~/.cache/cw6/t/tcb6db23/c/b10x
~/.cache/cw6/t/tcb6db23/c/b10x/connectors.toml
~/.cache/cw6/t/tcb6db23/s
~/.cache/cw6/t/tcb6db23/s/b10x
~/.cache/cw6/t/tcb6db23/s/b10x/connectors
~/.cache/cw6/t/tcb6db23/s/b10x/connectors/connectors.sock
~/.cache/cw6/t/tcb6db23/s/b10x/connectors/identity-sessions.json
~/.cache/cw6/t/tcb6db24
~/.cache/cw6/t/tcb6db24/c
~/.cache/cw6/t/tcb6db24/c/b10x
~/.cache/cw6/t/tcb6db24/c/b10x/connectors.toml
~/.cache/cw6/t/tcb6db24/s
~/.cache/cw6/t/tcb6db24/s/b10x
~/.cache/cw6/t/tcb6db24/s/b10x/connectors
~/.cache/cw6/t/tcb6db24/s/b10x/connectors/connectors.sock
~/.cache/cw6/t/tcb6db24/s/b10x/connectors/identity-sessions.json
~/.cache/cw6/t/tcb6db25
~/.cache/cw6/t/tcb6db25/c
~/.cache/cw6/t/tcb6db25/c/b10x
~/.cache/cw6/t/tcb6db25/c/b10x/connectors.toml
~/.cache/cw6/t/tcb6db25/s
~/.cache/cw6/t/tcb6db25/s/b10x
~/.cache/cw6/t/tcb6db25/s/b10x/connectors
~/.cache/cw6/t/tcb6db25/s/b10x/connectors/connectors.sock
~/.cache/cw6/t/tcb6db25/s/b10x/connectors/identity-sessions.json
~/.cache/cw6/t/tcb6db26
~/.cache/cw6/t/tcb6db26/c
~/.cache/cw6/t/tcb6db26/c/b10x
~/.cache/cw6/t/tcb6db26/c/b10x/connectors.toml
~/.cache/cw6/t/tcb6db26/s
~/.cache/cw6/t/tcb6db26/s/b10x
~/.cache/cw6/t/tcb6db26/s/b10x/connectors
~/.cache/cw6/t/tcb6db26/s/b10x/connectors/connectors.sock
~/.cache/cw6/t/tcb6db26/s/b10x/connectors/identity-sessions.json
~/.cache/cw6/t/tcb6db27
~/.cache/cw6/t/tcb6db27/c
~/.cache/cw6/t/tcb6db27/c/b10x
~/.cache/cw6/t/tcb6db27/c/b10x/connectors.toml
~/.cache/cw6/t/tcb6db27/s
~/.cache/cw6/t/tcb6db27/s/b10x
~/.cache/cw6/t/tcb6db27/s/b10x/connectors
~/.cache/cw6/t/tcb6db27/s/b10x/connectors/connectors.sock
~/.cache/cw6/t/tcb6db27/s/b10x/connectors/identity-sessions.json
~/.cache/cw6/t/tcb6db28
~/.cache/cw6/t/tcb6db28/c
~/.cache/cw6/t/tcb6db28/c/b10x
~/.cache/cw6/t/tcb6db28/c/b10x/connectors.toml
~/.cache/cw6/t/tcb6db28/s
~/.cache/cw6/t/tcb6db28/s/b10x
~/.cache/cw6/t/tcb6db28/s/b10x/connectors
~/.cache/cw6/t/tcb6db28/s/b10x/connectors/connectors.sock
~/.cache/cw6/t/tcb6db28/s/b10x/connectors/identity-sessions.json
~/.cache/cw6/t/tcb6db29
~/.cache/cw6/t/tcb6db29/c
~/.cache/cw6/t/tcb6db29/c/b10x
~/.cache/cw6/t/tcb6db29/c/b10x/connectors.toml
~/.cache/cw6/t/tcb6db29/s
~/.cache/cw6/t/tcb6db29/s/b10x
~/.cache/cw6/t/tcb6db29/s/b10x/connectors
~/.cache/cw6/t/tcb6db29/s/b10x/connectors/connectors.sock
~/.cache/cw6/t/tcb6db29/s/b10x/connectors/identity-sessions.json
~/.cache/cw6/t/tcb6db2a
~/.cache/cw6/t/tcb6db2a/c
~/.cache/cw6/t/tcb6db2a/c/b10x
~/.cache/cw6/t/tcb6db2a/c/b10x/connectors.toml
~/.cache/cw6/t/tcb6db2a/s
~/.cache/cw6/t/tcb6db2a/s/b10x
~/.cache/cw6/t/tcb6db2a/s/b10x/connectors
~/.cache/cw6/t/tcb6db2a/s/b10x/connectors/connectors.sock
~/.cache/cw6/t/tcb6db2a/s/b10x/connectors/identity-sessions.json
~/.cache/cw6/t/tcb6db3
~/.cache/cw6/t/tcb6db3/c
~/.cache/cw6/t/tcb6db3/c/b10x
~/.cache/cw6/t/tcb6db3/c/b10x/connectors.toml
~/.cache/cw6/t/tcb6db3/s
~/.cache/cw6/t/tcb6db3/s/b10x
~/.cache/cw6/t/tcb6db3/s/b10x/connectors
~/.cache/cw6/t/tcb6db3/s/b10x/connectors/connectors.sock
~/.cache/cw6/t/tcb6db3/s/b10x/connectors/identity-sessions.json
~/.cache/cw6/t/tcb6db4
~/.cache/cw6/t/tcb6db4/c
~/.cache/cw6/t/tcb6db4/c/b10x
~/.cache/cw6/t/tcb6db4/c/b10x/connectors.toml
~/.cache/cw6/t/tcb6db4/s
~/.cache/cw6/t/tcb6db4/s/b10x
~/.cache/cw6/t/tcb6db4/s/b10x/connectors
~/.cache/cw6/t/tcb6db4/s/b10x/connectors/connectors.sock
~/.cache/cw6/t/tcb6db4/s/b10x/connectors/identity-sessions.json
~/.cache/cw6/t/tcb6db5
~/.cache/cw6/t/tcb6db5/c
~/.cache/cw6/t/tcb6db5/c/b10x
~/.cache/cw6/t/tcb6db5/c/b10x/connectors.toml
~/.cache/cw6/t/tcb6db6
~/.cache/cw6/t/tcb6db6/c
~/.cache/cw6/t/tcb6db6/c/b10x
~/.cache/cw6/t/tcb6db6/c/b10x/connectors.toml
~/.cache/cw6/t/tcb6db6/connectors.sock
~/.cache/cw6/t/tcb6db6/s
~/.cache/cw6/t/tcb6db6/s/b10x
~/.cache/cw6/t/tcb6db6/s/b10x/connectors
~/.cache/cw6/t/tcb6db6/s/b10x/connectors/identity-sessions.json
~/.cache/cw6/t/tcb6db7
~/.cache/cw6/t/tcb6db7/c
~/.cache/cw6/t/tcb6db7/c/b10x
~/.cache/cw6/t/tcb6db7/c/b10x/connectors.toml
~/.cache/cw6/t/tcb6db7/s
~/.cache/cw6/t/tcb6db7/s/b10x
~/.cache/cw6/t/tcb6db7/s/b10x/connectors
~/.cache/cw6/t/tcb6db7/s/b10x/connectors/connectors.sock
~/.cache/cw6/t/tcb6db7/s/b10x/connectors/identity-sessions.json
~/.cache/cw6/t/tcb6db8
~/.cache/cw6/t/tcb6db8/c
~/.cache/cw6/t/tcb6db8/c/b10x
~/.cache/cw6/t/tcb6db8/c/b10x/connectors.toml
~/.cache/cw6/t/tcb6db9
~/.cache/cw6/t/tcb6db9/c
~/.cache/cw6/t/tcb6db9/c/b10x
~/.cache/cw6/t/tcb6db9/c/b10x/connectors.toml
~/.cache/cw6/t/tcb6dba
~/.cache/cw6/t/tcb6dba/c
~/.cache/cw6/t/tcb6dba/c/b10x
~/.cache/cw6/t/tcb6dba/c/b10x/connectors.toml
~/.cache/cw6/t/tcb6dbb
~/.cache/cw6/t/tcb6dbb/c
~/.cache/cw6/t/tcb6dbb/c/b10x
~/.cache/cw6/t/tcb6dbb/c/b10x/connectors.toml
~/.cache/cw6/t/tcb6dbc
~/.cache/cw6/t/tcb6dbc/c
~/.cache/cw6/t/tcb6dbc/c/b10x
~/.cache/cw6/t/tcb6dbc/c/b10x/connectors.toml
~/.cache/cw6/t/tcb6dbc/s
~/.cache/cw6/t/tcb6dbc/s/b10x
~/.cache/cw6/t/tcb6dbc/s/b10x/connectors
~/.cache/cw6/t/tcb6dbc/s/b10x/connectors/connectors.sock
~/.cache/cw6/t/tcb6dbc/s/b10x/connectors/identity-sessions.json
~/.cache/cw6/t/tcb6dbd
~/.cache/cw6/t/tcb6dbd/c
~/.cache/cw6/t/tcb6dbd/c/b10x
~/.cache/cw6/t/tcb6dbd/c/b10x/connectors.toml
~/.cache/cw6/t/tcb6dbd/connectors.sock
~/.cache/cw6/t/tcb6dbd/s
~/.cache/cw6/t/tcb6dbd/s/b10x
~/.cache/cw6/t/tcb6dbd/s/b10x/connectors
~/.cache/cw6/t/tcb6dbd/s/b10x/connectors/identity-sessions.json
~/.cache/cw6/t/tcb6dbe
~/.cache/cw6/t/tcb6dbe/c
~/.cache/cw6/t/tcb6dbe/c/b10x
~/.cache/cw6/t/tcb6dbe/c/b10x/connectors.toml
~/.cache/cw6/t/tcb6dbe/s
~/.cache/cw6/t/tcb6dbe/s/b10x
~/.cache/cw6/t/tcb6dbe/s/b10x/connectors
~/.cache/cw6/t/tcb6dbe/s/b10x/connectors/connectors.sock
~/.cache/cw6/t/tcb6dbe/s/b10x/connectors/identity-sessions.json
~/.cache/cw6/t/tcb6dbf
~/.cache/cw6/t/tcb6dbf/c
~/.cache/cw6/t/tcb6dbf/c/b10x
~/.cache/cw6/t/tcb6dbf/c/b10x/connectors.toml
~/.cache/cw6/t/tcb6dbf/s
~/.cache/cw6/t/tcb6dbf/s/b10x
~/.cache/cw6/t/tcb6dbf/s/b10x/connectors
~/.cache/cw6/t/tcb6dbf/s/b10x/connectors/connectors.sock
~/.cache/cw6/t/tcb6dbf/s/b10x/connectors/identity-sessions.json
```

```findings
[]
```
