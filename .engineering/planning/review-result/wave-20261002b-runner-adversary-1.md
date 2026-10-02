---
format: aep.planning-md/3
id: review-result:wave-20261002b-runner-adversary-1
kind: review-result
status: active
title: Ignored-suite runner adversary, pass 1
relations:
- reviews: story:ignored-suites-have-a-runner
revision: 1
---
unit: ignored-suites-have-a-runner; cb26b-runner working tree on b3ddded7618b5a4a23fe74dbec739e0938ecc296; ignored.rs sha256 fdce9d4b16462fcfa6be086f76e8c3b7c646298bc6dd3fff1a78e1fec9a3bb1f
verdict: NEEDS-CHANGE
cases: executed 148→150, red 2
origin: introduced 1 / pre-existing 0 / undecided 0
wrote-outside-worktree: 2 retained directories; task temporary children removed
needs-coordinator: record this report before routing the descendant-cleanup fix

1. git --no-pager diff --stat

```text
 crates/connectors-build/src/main.rs |  6 ++++
 docs/development.md                 | 55 +++++++++++++++++++++++++++++++++++++
 2 files changed, 61 insertions(+)
```

That whole-worktree diff is inherited implementor work; ignored.rs is still untracked, so ordinary git diff omits it. This pass changed only the authorized cfg(test) section of ignored.rs. The baseline saved after the implementor's last fixes is `.local/wave-20261002b/adversary-1/ignored-final-author.rs`, SHA-256 2eeec19ec84ffa7ee6b6e954df5bcf5671c11d1436f1b65becf741ca19a97bcf. The actual adversary delta, retained in `tests.patch`, is:

```text
 .../connectors-build/src/ignored.rs                | 74 ++++++++++++++++++++++
 1 file changed, 74 insertions(+)
```

All 74 added lines are test-only at lines 804–877. `cmp` confirmed main.rs and development.md unchanged by this pass. No production mutation, test weakening, AEP write or commit was performed. Embedded test additions were explicitly authorized in the coordinator brief.

2. Cases written before any test execution

- `crates/connectors-build/src/ignored.rs:869`, `adversary_passing_fixture_does_not_leave_running_descendants`: compile and execute a real Rust libtest fixture, spawn a child which would sleep 900 seconds, let the selected fixture pass, and assert the runner does not return with that descendant running. Red.
- `crates/connectors-build/src/ignored.rs:874`, `adversary_failing_fixture_does_not_leave_running_descendants`: same real libtest path, but selected fixture panics after launching the child. Red.

Both cases inspect the exact PID written by their fixture and kill that exact child before the test assertion. They neither use a shell fixture nor leave the sleeping children running. The measured fact is survival when the runner returns; the test does not spend 600 seconds measuring the eventual lifetime.

First focused command (before the suite):

```sh
RUSTC_WRAPPER=/usr/bin/sccache CARGO_BUILD_JOBS=2 TMPDIR=$HOME/.cache/cb26b-runner-adversary/tmp CONNECTORS_ESS=$HOME/.cache/ess/toolchains/0.45.0/ess CONNECTORS_AEP=$HOME/.cache/aep/toolchains/0.65.0/aep-0.65.0-x86_64-unknown-linux-gnu/aep cargo test --locked -p connectors-build --bin connectors-build ignored::tests::adversary_passing_fixture_does_not_leave_running_descendants -- --exact --nocapture
```

Exit 101; verbatim output:

```text
   Compiling connectors-build v0.24.0 ($HOME/.local/state/worktree/trees/b10x/connectors/cb26b-runner/crates/connectors-build)
    Finished `test` profile [optimized] target(s) in 11.41s
     Running unittests src/main.rs (target/debug/deps/connectors_build-b336cbc3f2901527)

running 1 test

thread 'ignored::tests::adversary_passing_fixture_does_not_leave_running_descendants' (1642571) panicked at crates/connectors-build/src/ignored.rs:862:9:
runner returned while fixture descendant 1642796 was still running; child planned to outlive the 600-second fixture deadline
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace
test ignored::tests::adversary_passing_fixture_does_not_leave_running_descendants ... FAILED

failures:

failures:
    ignored::tests::adversary_passing_fixture_does_not_leave_running_descendants

test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 56 filtered out; finished in 0.24s

error: test failed, to rerun pass `-p connectors-build --bin connectors-build`
```

Second focused command used the identical environment and Cargo arguments, with exact filter `ignored::tests::adversary_failing_fixture_does_not_leave_running_descendants`. Exit 101; verbatim output:

```text
    Finished `test` profile [optimized] target(s) in 0.20s
     Running unittests src/main.rs (target/debug/deps/connectors_build-b336cbc3f2901527)

running 1 test

thread 'ignored::tests::adversary_failing_fixture_does_not_leave_running_descendants' (1651255) panicked at crates/connectors-build/src/ignored.rs:862:9:
runner returned while fixture descendant 1651296 was still running; child planned to outlive the 600-second fixture deadline
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace
test ignored::tests::adversary_failing_fixture_does_not_leave_running_descendants ... FAILED

failures:

failures:
    ignored::tests::adversary_failing_fixture_does_not_leave_running_descendants

test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 56 filtered out; finished in 0.11s

error: test failed, to rerun pass `-p connectors-build --bin connectors-build`
```

3. Suite run after both focused red cases

```sh
RUSTC_WRAPPER=/usr/bin/sccache CARGO_BUILD_JOBS=2 TMPDIR=$HOME/.cache/cb26b-runner-adversary/tmp CONNECTORS_ESS=$HOME/.cache/ess/toolchains/0.45.0/ess CONNECTORS_AEP=$HOME/.cache/aep/toolchains/0.65.0/aep-0.65.0-x86_64-unknown-linux-gnu/aep cargo test --locked -p connectors-build --no-fail-fast
```

Exit 101. Eighteen suite summaries total 148 passed, 2 failed, 0 ignored: 150 executed. Before=148 is the implementor's stated final package result, supplied before adversarial execution. No pre-attack suite was run. Verbatim output:

```text
   Compiling connectors-build v0.24.0 ($HOME/.local/state/worktree/trees/b10x/connectors/cb26b-runner/crates/connectors-build)
    Finished `test` profile [optimized] target(s) in 5.38s
     Running unittests src/main.rs (target/debug/deps/connectors_build-b336cbc3f2901527)

running 57 tests
test aep_toolchain::tests::the_pin_is_one_exact_release ... ok
test cli::tests::public_acquisition_examples_cannot_expose_internal_states_or_wrong_terminal_fields ... ok
test cli::tests::cache_labels_and_page_deadlines_cannot_assert_unobserved_freshness ... ok
test docs::tests::cannot_publish_a_source_outside_its_owner ... ok
test docs::tests::refuses_unknown_projection_nodes ... ok
test docs::tests::public_audit_checks_embedded_binary_paths ... ok
test docs::tests::resolves_only_selected_local_links ... ok
test ess_boundary::tests::matches_case_and_separator_variants_without_substring_false_positives ... ok
test docs::tests::preserves_rules_and_stable_heading_anchors ... ok
test docs::tests::imported_markdown_cannot_execute_html_or_script_links ... ok
test ess_boundary::tests::allows_shared_protocols_and_separate_provider_models ... ok
test ess_boundary::tests::admits_a_root_component_source_that_owns_only_declared_domains ... ok
test docs::tests::rejects_duplicate_and_escaping_routes ... ok
test ess_boundary::tests::refuses_linked_source_files_and_directories ... ok
test docs::tests::reference_title_keeps_anchor_and_text_without_a_second_h1 ... ok
test ess_boundary::tests::discovers_design_only_and_spec_only_owners_without_runtime_declarations ... ok
test ess_boundary::tests::checks_all_paths_and_requires_complete_local_domain_inventory ... ok
test ess_boundary::tests::protocol_adapter_does_not_ban_shared_protocol_or_suppress_native_leaks ... ok
test ess_boundary::tests::refuses_a_component_source_owning_undeclared_domains_or_outside_the_root ... ok
test gate::tests::gate_cargo_keeps_the_toolchain_selector_first ... ok
test gate::tests::gate_cargo_keys_the_runner_to_the_host_triple ... ok
test ess_boundary::tests::rejects_malformed_optional_alias_inputs_and_model_sources ... ok
test gate::tests::gate_cargo_leaves_the_compiler_wrapper_on_the_callers_tmpdir ... ok
test ignored::tests::ignored_runner_has_an_explicit_cli_and_defaults_to_disposable ... ok
test gate::adversary_tests::runner_executes_a_failing_test_binary_whose_path_contains_an_equals_sign ... ok
test gate::adversary_tests::runner_hands_the_root_to_a_binary_whose_path_contains_an_equals_sign ... ok
test gate::adversary_pass2_tests::runner_passes_test_arguments_byte_for_byte ... ok
test docs::tests::example_model_carries_the_example_domains_and_their_references_only ... ok
test metadata_conformance::tests::kernel_invariant_violation_is_a_failed_command_not_unsupported ... ok
test metadata_conformance::tests::named_optional_members_round_trip_through_the_runtime_form ... ok
test metadata_conformance::tests::other_kernel_errors_stay_unsupported ... ok
test metadata_conformance::tests::unknown_fixture_is_an_error_not_a_default ... ok
test source_hashes::tests::a_manifest_without_retained_archives_is_not_this_check ... ok
test gate::adversary_pass2_tests::runner_preserves_exit_codes_above_one ... ok
test gate::tests::gate_cargo_hands_the_temporary_root_to_test_binaries_through_a_runner ... ok
test gate::adversary_pass2_tests::runner_executes_awkward_binary_paths ... ok
test gate::adversary_tests::runner_preserves_awkward_roots ... ok
test ess_boundary::tests::rejects_names_comments_fields_encoded_values_and_new_adapter_ids ... ok
test gate::adversary_pass2_tests::runner_hands_over_roots_with_a_leading_dash_or_newline ... ok
test docs::tests::walkthrough_fixtures_follow_the_shipped_selection ... ok
test tests::catalog_derived_from_mismatch_refused ... ok
test source_hashes::tests::recorded_digests_are_rederived_from_the_archived_bytes ... ok
test tests::discovery_writes_the_projection_and_its_record ... ok
test tests::catalog_without_derived_from_records_none ... ok
test tests::catalog_records_derivation ... ok
test ignored::tests::adversary_failing_fixture_does_not_leave_running_descendants ... FAILED
test ignored::tests::missing_prerequisite_is_explicit ... ok
test ignored::tests::unknown_inventory_refuses_before_any_execution ... ok
test ignored::tests::ignored_inventory_accounted ... ok
test ignored::tests::selected_test_failure_is_nonzero ... ok
test ignored::tests::inventory_mode_never_executes_selected_cases ... ok
test ignored::tests::adversary_passing_fixture_does_not_leave_running_descendants ... FAILED
test ignored::tests::zero_test_success_is_not_execution ... ok
test ignored::tests::subprocess_helper_never_top_level ... ok
test gate::adversary_pass2_tests::runner_preserves_a_killing_signal ... ok
test docs::tests::example_model_synthesizes_for_every_example_target_with_the_pinned_ess ... ok
test metadata_conformance::tests::emitted_manifest_is_admitted_by_the_pinned_ess ... ok

failures:

---- ignored::tests::adversary_failing_fixture_does_not_leave_running_descendants stdout ----

thread 'ignored::tests::adversary_failing_fixture_does_not_leave_running_descendants' (1657731) panicked at crates/connectors-build/src/ignored.rs:862:9:
runner returned while fixture descendant 1658227 was still running; child planned to outlive the 600-second fixture deadline
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace

---- ignored::tests::adversary_passing_fixture_does_not_leave_running_descendants stdout ----

thread 'ignored::tests::adversary_passing_fixture_does_not_leave_running_descendants' (1657733) panicked at crates/connectors-build/src/ignored.rs:862:9:
runner returned while fixture descendant 1658287 was still running; child planned to outlive the 600-second fixture deadline


failures:
    ignored::tests::adversary_failing_fixture_does_not_leave_running_descendants
    ignored::tests::adversary_passing_fixture_does_not_leave_running_descendants

test result: FAILED. 55 passed; 2 failed; 0 ignored; 0 measured; 0 filtered out; finished in 15.91s

error: test failed, to rerun pass `-p connectors-build --bin connectors-build`
     Running tests/cli_spec_mapping_adversary.rs (target/debug/deps/cli_spec_mapping_adversary-71fc452c77a216ec)

running 5 tests
test the_corrected_semantics_citations_land_on_their_rules ... ok
test the_er_rs_citation_lands_on_the_cursor_expiry_path ... ok
test the_owner_greeting_request_the_cli_sends_is_declared ... ok
test the_owner_greeting_reply_is_declared_with_the_hosts_fields ... ok
test the_build_digest_format_is_declared ... ok

test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s

     Running tests/cli_spec_mapping_adversary_pass2.rs (target/debug/deps/cli_spec_mapping_adversary_pass2-080cb476107a0c8e)

running 5 tests
test the_build_digest_comment_cites_the_file_that_encodes_it ... ok
test the_build_digest_admits_exactly_lowercase_64_hex ... ok
test the_owner_greeting_types_its_uuid_fields ... ok
test the_owner_hello_types_its_uuid_fields ... ok
test the_owner_frame_tag_is_modelled_the_same_way_on_every_frame ... ok

test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s

     Running tests/mcp_domain_model_adversary.rs (target/debug/deps/mcp_domain_model_adversary-228c2493728793c6)

running 3 tests
test the_one_stated_census_edge_agrees_with_the_field_that_realises_it ... ok
test a_binding_can_select_every_version_a_server_can_report_as_supported ... ok
test the_adapter_owner_document_does_not_deny_the_model_this_unit_added ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests/mcp_domain_model_adversary_pass2.rs (target/debug/deps/mcp_domain_model_adversary_pass2-f815742d91ef3b9a)

running 3 tests
test a_census_rows_verdict_cannot_be_supplied_by_the_row_beside_it ... ok
test no_state_carrier_closes_a_set_the_pinned_schema_leaves_open ... ok
test every_relation_no_source_answers_carries_the_marker_the_acceptance_names ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests/mcp_domain_model_census.rs (target/debug/deps/mcp_domain_model_census-5a568b97cacee13f)

running 4 tests
test the_protocol_domain_still_disclaims_making_a_selection ... ok
test the_protocol_revision_values_are_the_negotiated_strings_not_the_trap_constants ... ok
test the_three_credential_kinds_are_three_distinct_declared_types ... ok
test every_census_edge_is_marked_and_no_unreadable_one_is_realised ... ok

test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s

     Running tests/mcp_inbound_local_binding.rs (target/debug/deps/mcp_inbound_local_binding-6629082810c174ab)

running 16 tests
test no_scenario_this_story_owns_names_an_outcome_the_document_does_not ... ok
test the_binding_names_no_cloud_identity_or_network_dependency_it_does_not_refuse ... ok
test the_directory_ships_what_the_story_acceptance_states ... ok
test the_document_specifies_the_transport_the_matrix_selected_and_selects_nothing ... ok
test no_trace_issues_a_lease_longer_than_the_contract_ceiling ... ok
test every_close_deadline_a_trace_records_is_bounded_by_the_contract_and_the_live_lease ... ok
test the_acceptance_rule_is_read_from_the_statement_in_either_form_it_can_take ... ok
test every_behaviour_states_what_the_selected_transport_does_not_offer ... ok
test the_document_states_the_lease_obligation_its_traces_inherit ... ok
test the_single_principal_boundary_is_stated_and_no_scenario_crosses_it ... ok
test an_acceptance_that_states_both_a_count_and_a_rule_is_refused_rather_than_resolved - should panic ... ok
test every_behaviour_the_acceptance_names_has_an_outcome_a_transition_and_a_scenario ... ok
test every_named_transition_is_one_the_sessions_lifecycle_declares ... ok
test each_scenario_performs_the_transition_its_behaviour_names ... ok
test every_transition_a_behaviour_section_asserts_is_bound_to_one_of_its_traces ... ok
test no_trace_admits_data_or_renews_after_its_live_lease_deadline ... ok

test result: ok. 16 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s

     Running tests/mcp_inbound_local_binding_adversary.rs (target/debug/deps/mcp_inbound_local_binding_adversary-8dd79b368b0eca9d)

running 3 tests
test no_close_records_a_cutoff_later_than_the_lease_it_ends ... ok
test no_lease_a_trace_issues_outlives_the_ceiling_the_sessions_contract_sets ... ok
test no_data_act_is_admitted_after_the_lease_that_admits_it_can_legally_be_live ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s

     Running tests/mcp_inbound_local_binding_adversary_pass2.rs (target/debug/deps/mcp_inbound_local_binding_adversary_pass2-51f04ec67164740a)

running 2 tests
test the_story_ships_the_number_of_scenario_files_its_acceptance_states ... ok
test no_act_that_needs_a_live_lease_is_admitted_at_or_after_its_effective_expiry ... ok

test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s

     Running tests/mcp_outbound_connection_lifecycle.rs (target/debug/deps/mcp_outbound_connection_lifecycle-d9e65e6d6210ddba)

running 11 tests
test every_refusal_names_a_declared_wire_code_and_loss_is_not_failure ... ok
test every_scenario_this_story_owns_names_a_row_of_the_register ... ok
test the_document_does_not_close_the_revision_set_the_model_leaves_open ... ok
test every_state_the_story_names_has_a_named_outcome_and_a_scenario_file ... ok
test every_refusal_names_whose_act_it_reports ... ok
test no_citation_into_an_editable_document_carries_a_line_number ... ok
test outbound_stdio_is_held_by_its_blocker_and_nothing_else_is ... ok
test no_state_is_answered_by_sending_the_request_again ... ok
test every_citation_resolves_into_an_archived_file_of_the_pin ... ok
test every_section_and_scenario_resolves_which_revision_it_holds_for ... ok
test no_sentence_writes_a_field_of_the_binding_entity ... ok

test result: ok. 11 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.00s

     Running tests/mcp_outbound_connection_lifecycle_adversary.rs (target/debug/deps/mcp_outbound_connection_lifecycle_adversary-e4cd3504b394bb25)

running 6 tests
test a_selection_names_as_many_revisions_as_the_outcome_and_the_model_can_hold ... ok
test cancellation_says_which_revision_its_signal_holds_for ... ok
test a_request_the_interoperability_revision_makes_this_client_answer_has_an_outcome ... ok
test the_document_does_not_call_a_four_family_transport_space_a_pair ... ok
test an_answer_that_was_never_streamed_can_also_be_lost ... ok
test no_binding_field_is_both_unchanged_by_an_observation_and_written_from_one ... ok

test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s

     Running tests/mcp_outbound_connection_lifecycle_adversary_pass2.rs (target/debug/deps/mcp_outbound_connection_lifecycle_adversary_pass2-577e4a4ed4bcca90)

running 5 tests
test a_state_does_not_carry_two_outcomes_that_deny_each_other_s_precondition ... ok
test a_refusal_does_not_name_the_peer_for_a_reason_in_which_no_peer_was_seen ... ok
test the_advertised_capability_scenario_reads_a_result_the_interoperability_revision_has_not ... ok
test the_selection_scenario_states_a_framing_the_interoperability_revision_has_not ... ok
test no_sentence_of_this_contract_writes_a_field_of_the_binding_entity ... ok

test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.42s

     Running tests/mcp_profile_selection_matrix.rs (target/debug/deps/mcp_profile_selection_matrix-e8611fd8dd9d768a)

running 12 tests
test the_matrix_states_that_a_supported_row_is_a_selection_not_implemented_support ... ok
test no_row_resting_on_an_open_decision_blocker_claims_support_or_refusal ... ok
test no_revision_outside_the_pin_is_dispositioned_supported ... ok
test every_deferred_row_inheriting_the_other_sides_marker_is_stated_as_inheriting_it ... ok
test every_deferred_row_names_a_record_and_every_quoted_marker_exists_in_the_model ... ok
test each_pinned_schema_declares_exactly_two_capability_interfaces_with_optional_fields ... ok
test every_nested_capability_setting_is_named_in_the_section_of_the_side_that_declares_it ... ok
test an_inbound_header_refusal_excepts_every_supported_revision_whose_opener_carries_none ... ok
test each_row_carries_a_named_disposition_a_reason_and_a_resolvable_source_line ... ok
test every_extension_the_pinned_revisions_identify_is_named_by_the_matrix ... ok
test every_feature_the_pinned_specification_names_is_dispositioned_once_per_direction ... ok
test every_revision_exclusion_is_stated_by_the_matrix_and_still_removes_a_token ... ok

test result: ok. 12 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.14s

     Running tests/mcp_profile_selection_matrix_adversary.rs (target/debug/deps/mcp_profile_selection_matrix_adversary-7743f34177ec5c00)

running 3 tests
test every_deferred_row_rests_on_a_blocker_whose_own_record_names_that_feature ... ok
test every_nested_setting_is_named_in_the_section_of_the_side_whose_schema_declares_it ... ok
test every_protocol_version_the_archives_tell_a_peer_to_assume_is_dispositioned ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.10s

     Running tests/mcp_profile_selection_matrix_adversary_pass2.rs (target/debug/deps/mcp_profile_selection_matrix_adversary_pass2-7d4efded747be96c)

running 2 tests
test the_inbound_refusal_of_a_header_less_request_excepts_the_legacy_initialize_it_supports ... ok
test every_extension_the_pinned_revisions_identify_is_named_by_the_matrix ... ok

test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.10s

     Running tests/mcp_specification_pin_adversary.rs (target/debug/deps/mcp_specification_pin_adversary-cdc829c145204b4b)

running 4 tests
test manifest_provenance_fields_agree_with_the_revision_each_entry_claims ... ok
test version_negotiation_row_cites_the_interop_revisions_negotiation_section ... ok
test interop_revision_names_its_own_version_string_outside_a_documentation_path ... ok
test every_recorded_digest_and_byte_length_rederives_from_the_archived_bytes ... ok

test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.13s

     Running tests/mcp_specification_pin_adversary_pass2.rs (target/debug/deps/mcp_specification_pin_adversary_pass2-80a67822c2d7c384)

running 4 tests
test record_does_not_count_a_banner_document_twice_when_naming_running_text_mentions ... ok
test documented_provenance_check_refuses_a_path_repointed_within_its_own_revision ... ok
test documented_provenance_check_refuses_a_wholesale_revision_to_commit_swap ... ok
test design_document_accounts_for_every_transport_binding_the_pinned_revisions_specify ... ok

test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.17s

     Running tests/metadata_conformance_fixture_adversary.rs (target/debug/deps/metadata_conformance_fixture_adversary-6592e74e0ce5f82f)

running 5 tests
test an_unobservable_invariant_is_failed_as_a_violation_is ... ok
test a_well_formed_anchor_passes ... ok
test a_planted_wrong_outcome_fails_the_run ... ok
test an_invariant_violation_is_failed_through_the_runner ... ok
test an_unknown_fixture_name_errors_the_run ... ok

test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.89s

error: 1 target failed:
    `-p connectors-build --bin connectors-build`
```

4. Findings

| File:line | Verdict | Origin | Finding and measurement | What reaches it |
| --- | --- | --- | --- | --- |
| crates/connectors-build/src/ignored.rs:348 | NEEDS-CHANGE | introduced | A selected fixture which passes or panics can leave its process-group descendants running after the runner returns because group cleanup is only performed on timeout. Two real libtest cases fail at line 862, each exit 101, and the package repeats both failures. | Every selected case runs through `run -> evaluate -> execute_test`; normal success and libtest panic both take this early exit. The default family includes browser and owner subprocess fixtures; e.g. oauth_adversary_tests.rs:265 spawns Chrome before fallible operations and cleanup at :288. The new test uses the same ordinary child-spawn and libtest completion mechanics. No existing provider-specific leak is claimed. |

Severity: blocker for this runner unit's claimed bounded fixture lifecycle. The fix should retire the owned process group on every completion and error path, not only after ten minutes, with bounded cleanup before dropping the fixture directory. A subprocess fixture should not outlive its owner after either a green or red result. The underlying source module is new in this unit; the base commit has no ignored runner, establishing introduced origin without moving the shared worktree.

5. Attacks that did not produce another finding

- The existing real-Rust fixture cases stayed green for unknown inventory refusing all execution, helpers excluded from top-level runs, default family selection, explicit missing prerequisites, inventory-only execution, failure propagation, and zero-test false-green refusal.
- The implementor's corrected physical owner-private TMPDIR checks, symlink refusal and explicit mode 700 task directory passed in the full package.
- No claim is made about a full ten-minute timeout run, sandbox acceptance, or live provider execution; this pass did not execute them.

6. Outside-worktree paths

- `$HOME/.cache/cb26b-runner-adversary` — created task-owned parent, user timo, mode 700; retained.
- `$HOME/.cache/cb26b-runner-adversary/tmp` — approved task-owned TMPDIR, user timo, mode 700; retained and empty after all test runs. Dynamically named tempfile children and their compiled Rust fixtures/logs were removed by TempDir drop. Exact ephemeral basenames were not retained; the report does not claim otherwise.

Compiler output stays in this worktree's existing target directory; no shared CARGO_TARGET_DIR was set. Tool-managed worktree lease and sccache state were used. The coordinator owns retained directory cleanup.

```findings
- file: crates/connectors-build/src/ignored.rs
  line: 348
  category: concurrency
  severity: blocker
  verdict: NEEDS-CHANGE
  origin: introduced
  message: A selected fixture which passes or panics can leave its process-group descendants running after the runner returns because group cleanup is only performed on timeout.
```
