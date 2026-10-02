unit: ignored-suites-have-a-runner final attack; cb26b-runner on b3ddded7618b5a4a23fe74dbec739e0938ecc296; ignored.rs SHA-256 6ca3b95861a3f74f93527357fbd6ded828e8fbeee9fcf6f9db9bc58e6139b9e4
verdict: INFEASIBLE
cases: executed 150→152, red 1
origin: introduced 1 / pre-existing 0 / undecided 0
wrote-outside-worktree: reused approved temporary root; no retained new files
needs-coordinator: retain isolated autoreap probe as diagnostic evidence rather than unsupported release acceptance

1. git --no-pager diff --stat

```text
 Cargo.lock                          |  1 +
 crates/connectors-build/Cargo.toml  |  1 +
 crates/connectors-build/src/main.rs |  6 ++++
 docs/development.md                 | 59 +++++++++++++++++++++++++++++++++++++
 4 files changed, 67 insertions(+)
```

That whole-worktree diff is inherited implementation work; untracked ignored.rs is omitted by ordinary git diff. This pass changed only its explicitly authorized cfg(test) section. Reviewed production snapshot: ignored.rs SHA-256 da00b9cd54f23d7332b94f6643b6e0963324e296496991a0d0e6f1dddaf7ae17. Actual adversary delta:

```text
 .../connectors-build/src/ignored.rs                | 115 +++++++++++++++++++++
 1 file changed, 115 insertions(+)
```

`tests.patch` retains the first 54-line, green guard-drop addition. `tests-final.patch` retains the complete 115-line delta, including the isolated autoreap probe. Both tests remain in the assigned tree for coordinator disposition; no test was deleted or weakened. Cargo.lock, Cargo.toml, main.rs and development.md were unchanged by this pass. No production changes, store writes or commits were made.

The earlier provisional report and its publication copy were preserved as `provisional-report.md` and `provisional-publication-report.md`, not overwritten. They document the state before the author raised the autoreap concern and are superseded by this report within the same second attack. No third attack was opened.

2. Tests authored before execution

At `ignored.rs:965`, `adversary_error_drops_live_group_without_reaping_another_fixture` creates two real child process groups. An injected Result error drops the first FixtureGroup while its leader is live. The test requires bounded cleanup, observes ECHILD for the already-reaped exact leader and verifies that the unrelated group stayed alive. It cleans exact children before assertions. First run green; the two original child-leak cases also remained green.

At `ignored.rs:1018`, `adversary_inherited_autoreap_refuses_before_fixture_dispatch` uses an isolated exact-test subprocess, temporarily sets SIGCHLD=SIG_IGN, calls the private execute_test path through execute_one, and checks that fixture dispatch did not occur. The marker proves dispatch did occur. This is red, but its prerequisite is not reached by the current public command: the separate production probe below exits during the earlier Cargo metadata wait. This distinction limits the finding to INFEASIBLE/warning, not a required production fix.

All Cargo test commands ran from `$HOME/.local/state/worktree/trees/b10x/connectors/cb26b-runner` with this prefix:

```sh
RUSTC_WRAPPER=/usr/bin/sccache CARGO_BUILD_JOBS=2 TMPDIR=$HOME/.cache/cb26b-runner-adversary/tmp CONNECTORS_ESS=$HOME/.cache/ess/toolchains/0.45.0/ess CONNECTORS_AEP=$HOME/.cache/aep/toolchains/0.65.0/aep-0.65.0-x86_64-unknown-linux-gnu/aep
```

First targeted command, before any suite execution in this pass:

```sh
cargo test --locked -p connectors-build --bin connectors-build ignored::tests::adversary_error_drops_live_group_without_reaping_another_fixture -- --exact --nocapture
```

Exit 0; verbatim:

```text
   Compiling ring v0.17.14
   Compiling rustls v0.23.45
   Compiling rustls-webpki v0.103.15
   Compiling tokio-rustls v0.26.6
   Compiling rustls-platform-verifier v0.7.1
   Compiling hyper-rustls v0.27.10
   Compiling reqwest v0.13.5
   Compiling connectors-client v0.24.0 ($HOME/.local/state/worktree/trees/b10x/connectors/cb26b-runner/crates/connectors-client)
   Compiling connectors-host v0.24.0 ($HOME/.local/state/worktree/trees/b10x/connectors/cb26b-runner/crates/connectors-host)
   Compiling connectors-spec v0.24.0 ($HOME/.local/state/worktree/trees/b10x/connectors/cb26b-runner/crates/connectors-spec)
   Compiling connectors-build v0.24.0 ($HOME/.local/state/worktree/trees/b10x/connectors/cb26b-runner/crates/connectors-build)
    Finished `test` profile [optimized] target(s) in 21.06s
     Running unittests src/main.rs (target/debug/deps/connectors_build-4e0acae3b18f6d3b)

running 1 test
test ignored::tests::adversary_error_drops_live_group_without_reaping_another_fixture ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 57 filtered out; finished in 0.02s

```

Then both original cases and the new guard-drop case:

```sh
cargo test --locked -p connectors-build --bin connectors-build ignored::tests::adversary_ -- --nocapture
```

Exit 0; verbatim:

```text
    Blocking waiting for file lock on package cache
    Blocking waiting for file lock on package cache
    Blocking waiting for file lock on package cache
    Blocking waiting for file lock on package cache
    Finished `test` profile [optimized] target(s) in 1.65s
     Running unittests src/main.rs (target/debug/deps/connectors_build-4e0acae3b18f6d3b)

running 3 tests
test ignored::tests::adversary_error_drops_live_group_without_reaping_another_fixture ... ok
test ignored::tests::adversary_passing_fixture_does_not_leave_running_descendants ... ok
test ignored::tests::adversary_failing_fixture_does_not_leave_running_descendants ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 55 filtered out; finished in 0.50s

```

The interim package run executed 151 cases, all passing, and is retained verbatim in package.log and provisional-report.md. After the author raised automatic reaping, the isolated case was written before its first execution:

```sh
cargo test --locked -p connectors-build --bin connectors-build ignored::tests::adversary_inherited_autoreap_refuses_before_fixture_dispatch -- --exact --nocapture
```

Exit 101; verbatim focused red:

```text
    Blocking waiting for file lock on package cache
   Compiling connectors-build v0.24.0 ($HOME/.local/state/worktree/trees/b10x/connectors/cb26b-runner/crates/connectors-build)
    Finished `test` profile [optimized] target(s) in 55.48s
     Running unittests src/main.rs (target/debug/deps/connectors_build-4e0acae3b18f6d3b)

running 1 test

thread 'ignored::tests::adversary_inherited_autoreap_refuses_before_fixture_dispatch' (2169625) panicked at crates/connectors-build/src/ignored.rs:1071:9:

running 1 test
test ignored::tests::adversary_inherited_autoreap_refuses_before_fixture_dispatch ... FAILED

failures:

failures:
    ignored::tests::adversary_inherited_autoreap_refuses_before_fixture_dispatch

test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 58 filtered out; finished in 0.02s


thread 'ignored::tests::adversary_inherited_autoreap_refuses_before_fixture_dispatch' (2169901) panicked at crates/connectors-build/src/ignored.rs:1035:13:
runner dispatched the fixture while SIGCHLD=SIG_IGN invalidated its retained-leader ownership premise
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace

note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace
test ignored::tests::adversary_inherited_autoreap_refuses_before_fixture_dispatch ... FAILED

failures:

failures:
    ignored::tests::adversary_inherited_autoreap_refuses_before_fixture_dispatch

test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 58 filtered out; finished in 0.18s

error: test failed, to rerun pass `-p connectors-build --bin connectors-build`
```

Reachability probe: the retained Rust source inherited-sigchld.rs sets SIGCHLD=SIG_IGN and execs the production binary, preserving the inherited disposition through exec. Exact commands:

```sh
rustc --edition=2024 .local/wave-20261002b/adversary-2/inherited-sigchld.rs -o .local/wave-20261002b/adversary-2/inherited-sigchld
.local/wave-20261002b/adversary-2/inherited-sigchld target/debug/connectors-build --root $HOME/.local/state/worktree/trees/b10x/connectors/cb26b-runner ignored --inventory --tmpdir $HOME/.cache/cb26b-runner-adversary/tmp --report .local/wave-20261002b/adversary-2/autoreap-inventory.json
```

Exit 1 before producing an inventory report; verbatim:

```text
Error: Os { code: 10, kind: Uncategorized, message: "No child processes" }
```

The first production child wait is workspace()'s cargo metadata through crate::run(Command::output). Thus this inherited mode is refused before fixture discovery or dispatch. No PID reuse, wrong-process signal, or current public-command leak was measured.

3. Final suite after the new focused red case

```sh
cargo test --locked -p connectors-build --no-fail-fast
```

Exit 101. Eighteen top-level suites: 151 passed, 1 failed, 0 ignored, 152 executed. The failing assertion prints a nested subprocess summary; it is diagnostic output from the same isolated case and is not counted as another top-level test. The before count 150 is the implementor's corrected package result supplied before this attack. Verbatim:

```text
    Blocking waiting for file lock on package cache
    Blocking waiting for file lock on package cache
   Compiling connectors-build v0.24.0 ($HOME/.local/state/worktree/trees/b10x/connectors/cb26b-runner/crates/connectors-build)
    Finished `test` profile [optimized] target(s) in 10.52s
     Running unittests src/main.rs (target/debug/deps/connectors_build-4e0acae3b18f6d3b)

running 59 tests
test cli::tests::cache_labels_and_page_deadlines_cannot_assert_unobserved_freshness ... ok
test docs::tests::refuses_unknown_projection_nodes ... ok
test aep_toolchain::tests::the_pin_is_one_exact_release ... ok
test docs::tests::public_audit_checks_embedded_binary_paths ... ok
test cli::tests::public_acquisition_examples_cannot_expose_internal_states_or_wrong_terminal_fields ... ok
test docs::tests::cannot_publish_a_source_outside_its_owner ... ok
test docs::tests::imported_markdown_cannot_execute_html_or_script_links ... ok
test docs::tests::resolves_only_selected_local_links ... ok
test docs::tests::preserves_rules_and_stable_heading_anchors ... ok
test docs::tests::reference_title_keeps_anchor_and_text_without_a_second_h1 ... ok
test ess_boundary::tests::matches_case_and_separator_variants_without_substring_false_positives ... ok
test docs::tests::rejects_duplicate_and_escaping_routes ... ok
test ess_boundary::tests::admits_a_root_component_source_that_owns_only_declared_domains ... ok
test ess_boundary::tests::allows_shared_protocols_and_separate_provider_models ... ok
test ess_boundary::tests::refuses_linked_source_files_and_directories ... ok
test ess_boundary::tests::protocol_adapter_does_not_ban_shared_protocol_or_suppress_native_leaks ... ok
test gate::tests::gate_cargo_keeps_the_toolchain_selector_first ... ok
test gate::tests::gate_cargo_leaves_the_compiler_wrapper_on_the_callers_tmpdir ... ok
test gate::adversary_tests::runner_hands_the_root_to_a_binary_whose_path_contains_an_equals_sign ... ok
test ignored::tests::adversary_error_drops_live_group_without_reaping_another_fixture ... ok
test gate::adversary_tests::runner_executes_a_failing_test_binary_whose_path_contains_an_equals_sign ... ok
test gate::adversary_pass2_tests::runner_passes_test_arguments_byte_for_byte ... ok
test gate::tests::gate_cargo_keys_the_runner_to_the_host_triple ... ok
test ignored::tests::ignored_runner_has_an_explicit_cli_and_defaults_to_disposable ... ok
test ess_boundary::tests::rejects_malformed_optional_alias_inputs_and_model_sources ... ok
test docs::tests::example_model_carries_the_example_domains_and_their_references_only ... ok
test docs::tests::walkthrough_fixtures_follow_the_shipped_selection ... ok
test gate::adversary_pass2_tests::runner_hands_over_roots_with_a_leading_dash_or_newline ... ok
test gate::adversary_tests::runner_preserves_awkward_roots ... ok
test ess_boundary::tests::rejects_names_comments_fields_encoded_values_and_new_adapter_ids ... ok
test metadata_conformance::tests::kernel_invariant_violation_is_a_failed_command_not_unsupported ... ok
test metadata_conformance::tests::named_optional_members_round_trip_through_the_runtime_form ... ok
test metadata_conformance::tests::other_kernel_errors_stay_unsupported ... ok
test metadata_conformance::tests::unknown_fixture_is_an_error_not_a_default ... ok
test source_hashes::tests::a_manifest_without_retained_archives_is_not_this_check ... ok
test source_hashes::tests::recorded_digests_are_rederived_from_the_archived_bytes ... ok
test ess_boundary::tests::discovers_design_only_and_spec_only_owners_without_runtime_declarations ... ok
test ess_boundary::tests::checks_all_paths_and_requires_complete_local_domain_inventory ... ok
test ess_boundary::tests::refuses_a_component_source_owning_undeclared_domains_or_outside_the_root ... ok
test gate::tests::gate_cargo_hands_the_temporary_root_to_test_binaries_through_a_runner ... ok
test gate::adversary_pass2_tests::runner_preserves_exit_codes_above_one ... ok
test gate::adversary_pass2_tests::runner_executes_awkward_binary_paths ... ok
test ignored::tests::adversary_inherited_autoreap_refuses_before_fixture_dispatch ... FAILED
test ignored::tests::ignored_inventory_accounted ... ok
test ignored::tests::missing_prerequisite_is_explicit ... ok
test ignored::tests::zero_test_success_is_not_execution ... ok
test tests::catalog_derived_from_mismatch_refused ... ok
test ignored::tests::selected_test_failure_is_nonzero ... ok
test tests::catalog_without_derived_from_records_none ... ok
test ignored::tests::unknown_inventory_refuses_before_any_execution ... ok
test ignored::tests::inventory_mode_never_executes_selected_cases ... ok
test ignored::tests::adversary_passing_fixture_does_not_leave_running_descendants ... ok
test tests::discovery_writes_the_projection_and_its_record ... ok
test tests::catalog_records_derivation ... ok
test ignored::tests::subprocess_helper_never_top_level ... ok
test ignored::tests::adversary_failing_fixture_does_not_leave_running_descendants ... ok
test gate::adversary_pass2_tests::runner_preserves_a_killing_signal ... ok
test docs::tests::example_model_synthesizes_for_every_example_target_with_the_pinned_ess ... ok
test metadata_conformance::tests::emitted_manifest_is_admitted_by_the_pinned_ess ... ok

failures:

---- ignored::tests::adversary_inherited_autoreap_refuses_before_fixture_dispatch stdout ----

thread 'ignored::tests::adversary_inherited_autoreap_refuses_before_fixture_dispatch' (2205463) panicked at crates/connectors-build/src/ignored.rs:1071:9:

running 1 test
test ignored::tests::adversary_inherited_autoreap_refuses_before_fixture_dispatch ... FAILED

failures:

failures:
    ignored::tests::adversary_inherited_autoreap_refuses_before_fixture_dispatch

test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 58 filtered out; finished in 0.05s


thread 'ignored::tests::adversary_inherited_autoreap_refuses_before_fixture_dispatch' (2205864) panicked at crates/connectors-build/src/ignored.rs:1035:13:
runner dispatched the fixture while SIGCHLD=SIG_IGN invalidated its retained-leader ownership premise
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace

note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace


failures:
    ignored::tests::adversary_inherited_autoreap_refuses_before_fixture_dispatch

test result: FAILED. 58 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 19.22s

error: test failed, to rerun pass `-p connectors-build --bin connectors-build`
     Running tests/cli_spec_mapping_adversary.rs (target/debug/deps/cli_spec_mapping_adversary-144de18fb6ce964b)

running 5 tests
test the_corrected_semantics_citations_land_on_their_rules ... ok
test the_er_rs_citation_lands_on_the_cursor_expiry_path ... ok
test the_owner_greeting_reply_is_declared_with_the_hosts_fields ... ok
test the_owner_greeting_request_the_cli_sends_is_declared ... ok
test the_build_digest_format_is_declared ... ok

test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s

     Running tests/cli_spec_mapping_adversary_pass2.rs (target/debug/deps/cli_spec_mapping_adversary_pass2-0ead9d1ec130e4b1)

running 5 tests
test the_owner_greeting_types_its_uuid_fields ... ok
test the_build_digest_comment_cites_the_file_that_encodes_it ... ok
test the_owner_hello_types_its_uuid_fields ... ok
test the_build_digest_admits_exactly_lowercase_64_hex ... ok
test the_owner_frame_tag_is_modelled_the_same_way_on_every_frame ... ok

test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s

     Running tests/mcp_domain_model_adversary.rs (target/debug/deps/mcp_domain_model_adversary-50919bcc7e7813ce)

running 3 tests
test the_one_stated_census_edge_agrees_with_the_field_that_realises_it ... ok
test the_adapter_owner_document_does_not_deny_the_model_this_unit_added ... ok
test a_binding_can_select_every_version_a_server_can_report_as_supported ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests/mcp_domain_model_adversary_pass2.rs (target/debug/deps/mcp_domain_model_adversary_pass2-214fe694069a776b)

running 3 tests
test a_census_rows_verdict_cannot_be_supplied_by_the_row_beside_it ... ok
test every_relation_no_source_answers_carries_the_marker_the_acceptance_names ... ok
test no_state_carrier_closes_a_set_the_pinned_schema_leaves_open ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests/mcp_domain_model_census.rs (target/debug/deps/mcp_domain_model_census-3bfc3659d9861070)

running 4 tests
test the_protocol_domain_still_disclaims_making_a_selection ... ok
test the_protocol_revision_values_are_the_negotiated_strings_not_the_trap_constants ... ok
test the_three_credential_kinds_are_three_distinct_declared_types ... ok
test every_census_edge_is_marked_and_no_unreadable_one_is_realised ... ok

test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s

     Running tests/mcp_inbound_local_binding.rs (target/debug/deps/mcp_inbound_local_binding-fbc738e613f1ddf6)

running 16 tests
test the_binding_names_no_cloud_identity_or_network_dependency_it_does_not_refuse ... ok
test the_directory_ships_what_the_story_acceptance_states ... ok
test the_acceptance_rule_is_read_from_the_statement_in_either_form_it_can_take ... ok
test the_document_states_the_lease_obligation_its_traces_inherit ... ok
test no_scenario_this_story_owns_names_an_outcome_the_document_does_not ... ok
test the_single_principal_boundary_is_stated_and_no_scenario_crosses_it ... ok
test the_document_specifies_the_transport_the_matrix_selected_and_selects_nothing ... ok
test every_named_transition_is_one_the_sessions_lifecycle_declares ... ok
test every_behaviour_states_what_the_selected_transport_does_not_offer ... ok
test no_trace_issues_a_lease_longer_than_the_contract_ceiling ... ok
test each_scenario_performs_the_transition_its_behaviour_names ... ok
test every_behaviour_the_acceptance_names_has_an_outcome_a_transition_and_a_scenario ... ok
test every_transition_a_behaviour_section_asserts_is_bound_to_one_of_its_traces ... ok
test an_acceptance_that_states_both_a_count_and_a_rule_is_refused_rather_than_resolved - should panic ... ok
test every_close_deadline_a_trace_records_is_bounded_by_the_contract_and_the_live_lease ... ok
test no_trace_admits_data_or_renews_after_its_live_lease_deadline ... ok

test result: ok. 16 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s

     Running tests/mcp_inbound_local_binding_adversary.rs (target/debug/deps/mcp_inbound_local_binding_adversary-0eaf0b757a9878fb)

running 3 tests
test no_lease_a_trace_issues_outlives_the_ceiling_the_sessions_contract_sets ... ok
test no_close_records_a_cutoff_later_than_the_lease_it_ends ... ok
test no_data_act_is_admitted_after_the_lease_that_admits_it_can_legally_be_live ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s

     Running tests/mcp_inbound_local_binding_adversary_pass2.rs (target/debug/deps/mcp_inbound_local_binding_adversary_pass2-f7e00cfa879ea3e0)

running 2 tests
test the_story_ships_the_number_of_scenario_files_its_acceptance_states ... ok
test no_act_that_needs_a_live_lease_is_admitted_at_or_after_its_effective_expiry ... ok

test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s

     Running tests/mcp_outbound_connection_lifecycle.rs (target/debug/deps/mcp_outbound_connection_lifecycle-fad87bc9b8c3c1dc)

running 11 tests
test every_refusal_names_a_declared_wire_code_and_loss_is_not_failure ... ok
test the_document_does_not_close_the_revision_set_the_model_leaves_open ... ok
test every_state_the_story_names_has_a_named_outcome_and_a_scenario_file ... ok
test every_refusal_names_whose_act_it_reports ... ok
test outbound_stdio_is_held_by_its_blocker_and_nothing_else_is ... ok
test no_citation_into_an_editable_document_carries_a_line_number ... ok
test no_state_is_answered_by_sending_the_request_again ... ok
test every_scenario_this_story_owns_names_a_row_of_the_register ... ok
test every_citation_resolves_into_an_archived_file_of_the_pin ... ok
test every_section_and_scenario_resolves_which_revision_it_holds_for ... ok
test no_sentence_writes_a_field_of_the_binding_entity ... ok

test result: ok. 11 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.37s

     Running tests/mcp_outbound_connection_lifecycle_adversary.rs (target/debug/deps/mcp_outbound_connection_lifecycle_adversary-6159e1bccc3da86a)

running 6 tests
test a_selection_names_as_many_revisions_as_the_outcome_and_the_model_can_hold ... ok
test a_request_the_interoperability_revision_makes_this_client_answer_has_an_outcome ... ok
test cancellation_says_which_revision_its_signal_holds_for ... ok
test the_document_does_not_call_a_four_family_transport_space_a_pair ... ok
test an_answer_that_was_never_streamed_can_also_be_lost ... ok
test no_binding_field_is_both_unchanged_by_an_observation_and_written_from_one ... ok

test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s

     Running tests/mcp_outbound_connection_lifecycle_adversary_pass2.rs (target/debug/deps/mcp_outbound_connection_lifecycle_adversary_pass2-344a9ab73be884bf)

running 5 tests
test a_state_does_not_carry_two_outcomes_that_deny_each_other_s_precondition ... ok
test a_refusal_does_not_name_the_peer_for_a_reason_in_which_no_peer_was_seen ... ok
test the_advertised_capability_scenario_reads_a_result_the_interoperability_revision_has_not ... ok
test the_selection_scenario_states_a_framing_the_interoperability_revision_has_not ... ok
test no_sentence_of_this_contract_writes_a_field_of_the_binding_entity ... ok

test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.26s

     Running tests/mcp_profile_selection_matrix.rs (target/debug/deps/mcp_profile_selection_matrix-fbf40d70ffaa53bb)

running 12 tests
test no_row_resting_on_an_open_decision_blocker_claims_support_or_refusal ... ok
test no_revision_outside_the_pin_is_dispositioned_supported ... ok
test the_matrix_states_that_a_supported_row_is_a_selection_not_implemented_support ... ok
test every_deferred_row_names_a_record_and_every_quoted_marker_exists_in_the_model ... ok
test every_deferred_row_inheriting_the_other_sides_marker_is_stated_as_inheriting_it ... ok
test every_nested_capability_setting_is_named_in_the_section_of_the_side_that_declares_it ... ok
test each_pinned_schema_declares_exactly_two_capability_interfaces_with_optional_fields ... ok
test an_inbound_header_refusal_excepts_every_supported_revision_whose_opener_carries_none ... ok
test each_row_carries_a_named_disposition_a_reason_and_a_resolvable_source_line ... ok
test every_extension_the_pinned_revisions_identify_is_named_by_the_matrix ... ok
test every_revision_exclusion_is_stated_by_the_matrix_and_still_removes_a_token ... ok
test every_feature_the_pinned_specification_names_is_dispositioned_once_per_direction ... ok

test result: ok. 12 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.06s

     Running tests/mcp_profile_selection_matrix_adversary.rs (target/debug/deps/mcp_profile_selection_matrix_adversary-4647c5a48416dc42)

running 3 tests
test every_deferred_row_rests_on_a_blocker_whose_own_record_names_that_feature ... ok
test every_nested_setting_is_named_in_the_section_of_the_side_whose_schema_declares_it ... ok
test every_protocol_version_the_archives_tell_a_peer_to_assume_is_dispositioned ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.04s

     Running tests/mcp_profile_selection_matrix_adversary_pass2.rs (target/debug/deps/mcp_profile_selection_matrix_adversary_pass2-08892ff74f99b045)

running 2 tests
test the_inbound_refusal_of_a_header_less_request_excepts_the_legacy_initialize_it_supports ... ok
test every_extension_the_pinned_revisions_identify_is_named_by_the_matrix ... ok

test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.04s

     Running tests/mcp_specification_pin_adversary.rs (target/debug/deps/mcp_specification_pin_adversary-a105d50d7a2877c2)

running 4 tests
test manifest_provenance_fields_agree_with_the_revision_each_entry_claims ... ok
test version_negotiation_row_cites_the_interop_revisions_negotiation_section ... ok
test interop_revision_names_its_own_version_string_outside_a_documentation_path ... ok
test every_recorded_digest_and_byte_length_rederives_from_the_archived_bytes ... ok

test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.33s

     Running tests/mcp_specification_pin_adversary_pass2.rs (target/debug/deps/mcp_specification_pin_adversary_pass2-93e6fffabbad4cca)

running 4 tests
test record_does_not_count_a_banner_document_twice_when_naming_running_text_mentions ... ok
test documented_provenance_check_refuses_a_wholesale_revision_to_commit_swap ... ok
test documented_provenance_check_refuses_a_path_repointed_within_its_own_revision ... ok
test design_document_accounts_for_every_transport_binding_the_pinned_revisions_specify ... ok

test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.04s

     Running tests/metadata_conformance_fixture_adversary.rs (target/debug/deps/metadata_conformance_fixture_adversary-4583e554883aaf53)

running 5 tests
test an_unknown_fixture_name_errors_the_run ... ok
test a_planted_wrong_outcome_fails_the_run ... ok
test an_invariant_violation_is_failed_through_the_runner ... ok
test an_unobservable_invariant_is_failed_as_a_violation_is ... ok
test a_well_formed_anchor_passes ... ok

test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.36s

error: 1 target failed:
    `-p connectors-build --bin connectors-build`
```

4. Findings

| File:line | Verdict | Origin | What was measured | What reaches it |
| --- | --- | --- | --- | --- |
| crates/connectors-build/src/ignored.rs:349 | INFEASIBLE | introduced | With SIGCHLD=SIG_IGN, an isolated private execute_test call dispatches a fixture without a waitable leader, invalidating the premise used for its later group signal; the current public command refuses that disposition during its earlier Cargo metadata wait. The isolated marker assertion is red (focused and package exit 101). | Only the constructed private-function call reaches fixture dispatch in this state. The actual production CLI probe exits 1 with ECHILD before fixture dispatch; no current public caller reaching this state was found. |

Severity warning. This is not a demanded production change or release acceptance obligation. The coordinator should preserve the diagnostic probe and reachability result without imposing its unsupported private-function precondition as a release gate. Origin is introduced: this runner and its guard are absent from the unit base.

5. Attacks without another finding

- The original success and panic descendant-leak regressions both pass after the implementation correction.
- Early error/drop cleanup reaps the live owned leader within the test ceiling and leaves a different fixture group alive.
- Under ordinary child-reaping disposition, WNOWAIT retains the leader until the group signal; waits are restricted to that group, signalling does not resume after reaping, and cleanup has bounded waits. No process scan is used.
- Existing inventory, classification, prerequisites, helper exclusion, failure propagation, zero-case refusal and private TMPDIR cases remain green.
- The initial 54-line guard test passed formatting. The later isolated probe has only a rustfmt line-wrap difference (recorded in fmt-autoreap.log); it is diagnostic source pending coordinator disposition. No production formatting defect is claimed.
- No full workspace, live sandbox or full ten-minute timeout run was performed by this pass. Deliberately detached sessions remain the documented fixture shutdown responsibility.

6. Outside-worktree paths and publication handling

`$HOME/.cache/cb26b-runner-adversary/tmp` was reused as approved TMPDIR. It and its existing parent `$HOME/.cache/cb26b-runner-adversary` remain owned by the session user, mode 700. Dynamic test children were removed by their TempDir owners; no new retained outside-worktree files were created. Compiler output stayed in the assigned target. Tool-managed lease and cache writes were used; coordinator owns final cleanup.

The separate publication-report.md applies only a literal home-directory-prefix replacement with `$HOME`; all test output, counts, findings and command content otherwise remain unchanged. This disclosed path-only publication redaction leaves report.md as the private raw source. No results were removed or rewritten.

```findings
- file: crates/connectors-build/src/ignored.rs
  line: 349
  category: boundary
  severity: warning
  verdict: INFEASIBLE
  origin: introduced
  message: With SIGCHLD=SIG_IGN, an isolated private execute_test call dispatches a fixture without a waitable leader, invalidating the premise used for its later group signal; the current public command refuses that disposition during its earlier Cargo metadata wait.
```
