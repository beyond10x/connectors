---
format: aep.planning-md/1
id: review-result:cli-credentials-adversary-1-20260906
kind: review-result
status: active
title: CLI credential selection adversary pass 1
refs:
- provider: git
  reference: 66c62dea36120bf1bd267b0863f2f3e2bc4b9d8b
relations:
- reviews: story:one-placement-several-credentials
revision: 1
---
unit: story:one-placement-several-credentials; source cacff114c10a6429fdd6e3aa27f2142f1c68aa83; coordinator test seam 66c62dea36120bf1bd267b0863f2f3e2bc4b9d8b plus retained test diff
verdict: CONFIRMED
cases: executed 165→169, red 2
origin: introduced 2 / pre-existing 0 / undecided 0
wrote-outside-worktree: 11 paths, listed below
needs-coordinator: route both retained failures for correction; no missing fixture seam

`git --no-pager diff --stat` (exit 0):

```text
 .../src/personal_connections_tests.rs              | 194 +++++++++++++++++++++
 1 file changed, 194 insertions(+)
```

The adversary changed only tests. The coordinator supplied and committed the requested cfg(test) registration in enrol.rs as 66c62dea; reviewed production remains cacff114. No production file, including its inline tests, was edited by this adversary. The new untracked test file is absent from the ordinary diff stat, so its no-index stat follows (exit 1 means the new file differs):

```text
 .../src/enrol_adversary_tests.rs                   | 87 ++++++++++++++++++++++
 1 file changed, 87 insertions(+)
```

`git status --short` (exit 0):

```text
 M crates/integration-catalog/src/personal_connections_tests.rs
?? crates/connectors-console/src/enrol_adversary_tests.rs
```

## 2. Deciding cases, written before the first test run

All four cases existed before any test executed. Each was first run alone before either full package command. The enrollment case waited for the coordinator's registration seam. No case was removed, weakened, deselected or repaired after it failed. No production mutation was used to obtain a failure.

### Datadog conjunction — red

crates/integration-catalog/src/personal_connections_tests.rs:421. The real compiled datadog-monitor-list declares the API-key/application-key conjunction. Both declared credential addresses are populated in MemoryStore under one legacy placement, and the unchanged resolver successfully assembles two credentials. Operation search then returns zero operations instead of the one admitted read. This is a published provider mechanism, not a fabricated consumer or a request to mix Slack identities.

Command (exit 101):

```text
env TMPDIR=~/.cache/cw6/m RUSTC_WRAPPER=/usr/bin/sccache CARGO_INCREMENTAL=0 CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 CARGO_BUILD_JOBS=3 cargo test --manifest-path crates/connectors-runtime/Cargo.toml --locked -p integration-catalog personal_connections_tests::adversary_legacy_compound_auth_keeps_the_catalogued_datadog_read_callable -- --exact
```

Verbatim output:

```text
   Compiling integration-catalog v0.6.5 (~/.local/state/worktree/trees/b10x/connectors/wt-03d120cf8736/crates/integration-catalog)
    Finished `test` profile [unoptimized] target(s) in 1.28s
     Running unittests src/lib.rs (crates/connectors-runtime/target/debug/deps/integration_catalog-f55a18277ee5bce7)

running 1 test
test personal_connections_tests::adversary_legacy_compound_auth_keeps_the_catalogued_datadog_read_callable ... FAILED

failures:

---- personal_connections_tests::adversary_legacy_compound_auth_keeps_the_catalogued_datadog_read_callable stdout ----

thread 'personal_connections_tests::adversary_legacy_compound_auth_keeps_the_catalogued_datadog_read_callable' (1388575) panicked at ~/.local/state/worktree/trees/b10x/connectors/wt-03d120cf8736/crates/integration-catalog/src/personal_connections_tests.rs:481:5:
assertion `left == right` failed: legacy Datadog credentials form one conjunctive mechanism; selecting its API key must not erase every operation
  left: 0
 right: 1
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace


failures:
    personal_connections_tests::adversary_legacy_compound_auth_keeps_the_catalogued_datadog_read_callable

test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 46 filtered out; finished in 1.01s

error: test failed, to rerun pass `-p integration-catalog --lib`
```

### Same selector across placements — green

crates/integration-catalog/src/personal_connections_tests.rs:485. Two placements select slack.user_token; only the second has that stored credential. Describe lists only the second Connection, and invocation through the first refuses without egress.

Command (exit 0):

```text
env TMPDIR=~/.cache/cw6/m RUSTC_WRAPPER=/usr/bin/sccache CARGO_INCREMENTAL=0 CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 CARGO_BUILD_JOBS=3 cargo test --manifest-path crates/connectors-runtime/Cargo.toml --locked -p integration-catalog personal_connections_tests::adversary_the_same_selector_in_another_placement_cannot_supply_this_connection -- --exact
```

Verbatim output:

```text
    Finished `test` profile [unoptimized] target(s) in 0.12s
     Running unittests src/lib.rs (crates/connectors-runtime/target/debug/deps/integration_catalog-f55a18277ee5bce7)

running 1 test
test personal_connections_tests::adversary_the_same_selector_in_another_placement_cannot_supply_this_connection ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 46 filtered out; finished in 1.06s

```

### Locked selected slot — green

crates/integration-catalog/src/personal_connections_tests.rs:574. The selected user slot refuses access while a bot sibling is stored. Search returns Unavailable, invocation refuses, only the selected address is read, and no egress occurs.

Command (exit 0):

```text
env TMPDIR=~/.cache/cw6/m RUSTC_WRAPPER=/usr/bin/sccache CARGO_INCREMENTAL=0 CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 CARGO_BUILD_JOBS=3 cargo test --manifest-path crates/connectors-runtime/Cargo.toml --locked -p integration-catalog personal_connections_tests::adversary_a_locked_selected_slot_never_reads_a_callable_sibling -- --exact
```

Verbatim output:

```text
    Blocking waiting for file lock on package cache
    Blocking waiting for file lock on package cache
    Blocking waiting for file lock on package cache
    Blocking waiting for file lock on package cache
    Finished `test` profile [unoptimized] target(s) in 2.44s
     Running unittests src/lib.rs (crates/connectors-runtime/target/debug/deps/integration_catalog-f55a18277ee5bce7)

running 1 test
test personal_connections_tests::adversary_a_locked_selected_slot_never_reads_a_callable_sibling ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 46 filtered out; finished in 1.11s

```

### Rejected writer overwrites a callable slot — red

crates/connectors-console/src/enrol_adversary_tests.rs:40. An existing enrollment lock refuses re-entry of a bot credential. The configuration bytes and the other writer's lock remain unchanged, but the stored credential for the already configured writable Connection has been replaced by the rejected candidate. This is an existing live slot, not merely a newly sealed orphan slot. The fixture uses private run_with_store with MemoryStore and a 0600 candidate file; the public run is a direct wrapper around this function.

Command (exit 101):

```text
env TMPDIR=~/.cache/cw6/m RUSTC_WRAPPER=/usr/bin/sccache CARGO_INCREMENTAL=0 CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 CARGO_BUILD_JOBS=3 cargo test --manifest-path crates/connectors-console/Cargo.toml --locked enrol::enrol_adversary_tests::adversary_refused_writer_lock_must_not_replace_the_live_selected_credential -- --exact
```

Verbatim output:

```text
    Blocking waiting for file lock on package cache
   Compiling integration-catalog v0.6.5 (~/.local/state/worktree/trees/b10x/connectors/wt-03d120cf8736/crates/integration-catalog)
   Compiling connectors-console v0.6.5 (~/.local/state/worktree/trees/b10x/connectors/wt-03d120cf8736/crates/connectors-console)
    Finished `test` profile [unoptimized] target(s) in 5.29s
     Running unittests src/lib.rs (crates/connectors-console/target/debug/deps/connectors_console-8bcc5a4ea9745940)

running 1 test
test enrol::enrol_adversary_tests::adversary_refused_writer_lock_must_not_replace_the_live_selected_credential ... FAILED

failures:

---- enrol::enrol_adversary_tests::adversary_refused_writer_lock_must_not_replace_the_live_selected_credential stdout ----
Connect Slack (slack)
Credential: slack.bot_token
  acts as an application identity, bounded by its own memberships and scopes
Input is hidden and goes straight to the credential store.

thread 'enrol::enrol_adversary_tests::adversary_refused_writer_lock_must_not_replace_the_live_selected_credential' (1572877) panicked at src/enrol_adversary_tests.rs:82:5:
assertion `left == right` failed: a rejected writer must not change the credential used by the already callable Connection
  left: "fixture-rejected-replacement"
 right: "fixture-live-original"
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace


failures:
    enrol::enrol_adversary_tests::adversary_refused_writer_lock_must_not_replace_the_live_selected_credential

test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 79 filtered out; finished in 1.00s

error: test failed, to rerun pass `--lib`
```

## 3. Complete affected-package runs

Before counts are taken from the implementor's green report, not from a preliminary suite run: config 27, catalog 44, console 94, total 165. Both commands below began only after all four isolated deciding runs above completed. --no-fail-fast retains the complete case count when the library test executable fails.

| Lane | Before | Executed now | Passed | Failed | Exit |
| --- | ---: | ---: | ---: | ---: | ---: |
| connectors-config | 27 | 27 (24 unit + 3 integration) | 27 | 0 | shared runtime command 101 |
| integration-catalog | 44 | 47 | 46 | 1 | shared runtime command 101 |
| connectors-console | 94 | 95 (80 unit + 3 + 7 + 5 integration) | 94 | 1 | 101 |
| Total | 165 | 169 | 167 | 2 | — |

All doc-test targets ran zero cases. Every inherited case passes. The only failures are the two newly retained deciding cases.

Command (exit 101):

```text
env TMPDIR=~/.cache/cw6/m RUSTC_WRAPPER=/usr/bin/sccache CARGO_INCREMENTAL=0 CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 CARGO_BUILD_JOBS=3 cargo test --manifest-path crates/connectors-runtime/Cargo.toml --locked --no-fail-fast -p connectors-config -p integration-catalog
```

Verbatim output:

```text
    Blocking waiting for file lock on package cache
    Blocking waiting for file lock on package cache
    Finished `test` profile [unoptimized] target(s) in 6.43s
     Running unittests src/lib.rs (crates/connectors-runtime/target/debug/deps/connectors_config-f982a143bd730f2e)

running 24 tests
test hosted::tests::claude_code_custody_is_explicit_and_requires_the_hosted_secret_store ... ok
test hosted::tests::hosted_configuration_uses_same_handle_and_refuses_mutable_or_symlinked_files ... ok
test personal::tests::a_trunk_address_may_be_a_literal_or_a_name_and_nothing_else ... ok
test hosted::tests::an_existing_hosted_config_with_the_old_b10x_section_parses_unchanged ... ok
test personal::catalog::tests::explicit_policy_per_credential_is_a_valid_value_free_configuration ... ok
test hosted::tests::hosted_integrations_are_explicit_and_fail_closed ... ok
test hosted::tests::kubernetes_namespace_groups_are_exact_sorted_and_restart_is_a_read_subset ... ok
test personal::catalog::tests::credential_policy_is_explicit_and_unknown_binding_fields_are_refused ... ok
test hosted::tests::hosted_vault_requires_a_valid_distinct_sip_digest_pair ... ok
test hosted::tests::hosted_vault_is_all_or_nothing ... ok
test hosted::tests::hosted_grafana_requires_vault_exact_groups_and_digest_bound_targets ... ok
test personal::catalog::tests::legacy_single_selection_retains_its_shape_and_write_policy ... ok
test personal::tests::an_existing_personal_config_with_the_old_b10x_section_parses_unchanged ... ok
test hosted::tests::hosted_jira_service_api_token_excludes_a_service_oauth_client_id ... ok
test hosted::tests::hosted_gitlab_requires_vault_and_a_same_origin_callback ... ok
test personal::tests::grafana_configuration_names_origin_and_independent_target_grants_only ... ok
test hosted::tests::hosted_jira_separates_organization_and_user_authority ... ok
test personal::tests::kubernetes_configuration_is_policy_only ... ok
test hosted::tests::hosted_deployment_accepts_planner_integration ... ok
test personal::catalog::tests::duplicate_empty_and_ambiguous_selections_are_refused ... ok
test personal::tests::slack_only_configuration_contains_policy_but_no_secret_source ... ok
test personal::tests::documented_development_configuration_stays_strict_and_valid ... ok
test personal::tests::deployment_configuration_cannot_be_symlinked_or_group_writable ... ok
test personal::tests::the_named_default_trunk_example_parses_and_dials_by_number ... ok

test result: ok. 24 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests/catalog_usernames.rs (crates/connectors-runtime/target/debug/deps/catalog_usernames-d53b5bdd7fa1175f)

running 3 tests
test an_entry_with_no_user_half_still_reads_and_reports_none ... ok
test a_basic_credential_carries_its_user_half_beside_its_endpoints ... ok
test a_user_half_is_refused_when_it_is_empty_or_could_not_travel ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running unittests src/lib.rs (crates/connectors-runtime/target/debug/deps/integration_catalog-f55a18277ee5bce7)

running 47 tests
test argocd::tests::a_rejected_sign_in_says_so_rather_than_reporting_a_missing_project ... ok
test argocd::tests::a_missing_project_is_not_reported_as_a_permission_problem ... ok
test argocd::tests::a_login_without_projects_update_is_told_which_grant_it_lacks ... ok
test argocd::tests::a_project_or_role_name_that_could_change_the_path_is_refused ... ok
test argocd::tests::a_zero_lifetime_is_refused_rather_than_meaning_forever ... ok
test argocd::tests::an_unreachable_origin_names_the_aperture_as_a_possibility ... ok
test argocd::tests::every_generated_policy_is_one_argo_cd_will_accept ... ok
test argocd::tests::read_only_acquisition_carries_no_sync_policy ... ok
test argocd::tests::an_existing_role_is_reused_and_its_policies_are_left_alone ... ok
test argocd::tests::the_password_appears_once_and_the_session_token_never_persists ... ok
test config::tests::a_value_without_operator_approval_says_so_rather_than_claiming_it ... ok
test argocd::tests::the_four_calls_happen_in_order_and_the_token_comes_back ... ok
test config::tests::an_unsupplied_variable_is_absent_rather_than_empty ... ok
test config::tests::an_endpoint_value_reaches_the_resolver_under_its_declared_name ... ok
test argocd::tests::the_project_is_written_back_whole ... ok
test tests::an_entry_stating_no_user_half_answers_none_rather_than_an_empty_string ... ok
test tests::an_unnamed_entry_keeps_the_address_it_had_before_instances_existed ... ok
test tests::a_basic_credentials_user_half_reaches_the_resolver_from_the_entry ... ok
test tests::a_network_effect_is_not_an_escalation_but_a_filesystem_one_would_be ... ok
test personal_connections_tests::unknown_selectors_are_refused_even_without_a_bootstrap_file ... ok
test personal_connections_tests::duplicate_placement_references_are_refused_at_composition ... ok
test tests::a_read_only_first_connection_does_not_hide_an_admitted_write ... ok
test tests::effect_class_comes_from_the_declaration_not_the_method ... ok
test tests::a_provider_with_a_fixed_base_url_needs_no_configuration ... ok
test personal_connections_tests::selecting_one_credential_hides_other_mechanisms_and_keeps_legacy_addresses ... ok
test tests::every_catalogued_provider_can_address_a_credential ... ok
test tests::every_declared_user_half_field_names_a_credential_that_actually_wants_one ... ok
test personal_connections_tests::a_missing_selected_credential_is_absent_from_callable_discovery ... ok
test tests::the_aperture_is_derived_from_the_same_declaration_the_request_is ... ok
test personal_connections_tests::primary_and_additional_references_do_not_depend_on_binding_order ... ok
test personal_connections_tests::discovery_and_connection_metadata_refuse_another_owner ... ok
test tests::the_ceiling_reads_declared_facts_rather_than_an_operation_list ... ok
test tests::the_default_ceiling_admits_reads_and_refuses_writes ... ok
test tests::a_request_template_exists_for_every_operation_this_backend_would_offer ... ok
test tests::the_limit_drops_operations_rather_than_the_identities_that_serve_one ... ok
test personal_connections_tests::adversary_legacy_compound_auth_keeps_the_catalogued_datadog_read_callable ... FAILED
test tests::the_instance_derivation_puts_the_provider_in_the_namespace ... ok
test tests::two_named_instances_of_one_provider_get_different_addresses ... ok
test tests::search_and_describe_require_approval_for_every_admitted_write ... ok
test tests::an_unsupplied_url_variable_refuses_rather_than_reaching_a_literal_placeholder ... ok
test personal_connections_tests::every_catalog_connection_summary_obeys_the_frozen_wire_contract ... ok
test personal_connections_tests::a_selected_user_never_falls_back_to_a_stored_bot ... ok
test personal_connections_tests::adversary_the_same_selector_in_another_placement_cannot_supply_this_connection ... ok
test personal_connections_tests::adversary_a_locked_selected_slot_never_reads_a_callable_sibling ... ok
test hosted::tests::completion_does_not_overwrite_a_terminal_session_after_verification_awaits ... ok
test personal_connections_tests::two_selected_credentials_have_distinct_connections_and_exact_egress ... ok
test hosted::tests::two_people_receive_isolated_connections_and_credential_addresses ... ok

failures:

---- personal_connections_tests::adversary_legacy_compound_auth_keeps_the_catalogued_datadog_read_callable stdout ----

thread 'personal_connections_tests::adversary_legacy_compound_auth_keeps_the_catalogued_datadog_read_callable' (1603658) panicked at ~/.local/state/worktree/trees/b10x/connectors/wt-03d120cf8736/crates/integration-catalog/src/personal_connections_tests.rs:481:5:
assertion `left == right` failed: legacy Datadog credentials form one conjunctive mechanism; selecting its API key must not erase every operation
  left: 0
 right: 1
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace


failures:
    personal_connections_tests::adversary_legacy_compound_auth_keeps_the_catalogued_datadog_read_callable

test result: FAILED. 46 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.05s

error: test failed, to rerun pass `-p integration-catalog --lib`
   Doc-tests connectors_config

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

   Doc-tests integration_catalog

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

error: 1 target failed:
    `-p integration-catalog --lib`
```

Command (exit 101):

```text
env TMPDIR=~/.cache/cw6/m RUSTC_WRAPPER=/usr/bin/sccache CARGO_INCREMENTAL=0 CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 CARGO_BUILD_JOBS=3 cargo test --manifest-path crates/connectors-console/Cargo.toml --locked --no-fail-fast
```

Verbatim output:

```text
    Blocking waiting for file lock on package cache
    Blocking waiting for file lock on package cache
    Finished `test` profile [unoptimized] target(s) in 3.22s
     Running unittests src/lib.rs (crates/connectors-console/target/debug/deps/connectors_console-8bcc5a4ea9745940)

running 80 tests
test auth::tests::a_basic_credential_row_reports_whether_its_user_half_is_configured_and_never_the_value ... ok
test auth::tests::nothing_in_the_result_can_carry_a_secret ... ok
test admin::tests::explicit_secret_file_must_be_owner_only ... ok
test doctor::tests::a_missing_configuration_is_fatal_and_names_the_command_that_fixes_it ... ok
test connect::tests::the_error_for_an_unknown_provider_names_it ... ok
test doctor::tests::a_report_is_unhealthy_only_when_something_cannot_work ... ok
test auth::tests::the_store_preference_matches_what_the_runtime_composes ... ok
test doctor::tests::doctor_names_the_default_local_target_and_its_socket ... ok
test doctor::tests::a_short_state_root_passes_both_budgets ... ok
test connect::tests::a_catalogued_provider_whose_curated_backend_is_absent_takes_the_catalogue_path ... ok
test doctor::tests::every_state_a_check_can_report_reaches_the_reader_as_its_own_marker ... ok
test admin::tests::command_shape_accepts_secret_stdin_without_a_secret_argument ... ok
test connect::tests::a_provider_outside_the_guided_set_is_refused_by_name ... ok
test doctor::tests::the_budget_is_measured_against_the_deepest_path_the_daemon_binds ... ok
test doctor::tests::the_report_renders_every_check_as_data ... ok
test enrol::tests::a_provider_outside_the_catalogue_is_named_rather_than_guessed_at ... ok
test envelope::tests::an_envelope_carrying_neither_is_a_named_failure_not_an_empty_success ... ok
test init::tests::admitting_a_credential_plugin_is_a_choice_and_its_absence_is_explained ... ok
test envelope::tests::a_result_loses_its_envelope_and_its_discriminant ... ok
test envelope::tests::a_refusal_becomes_an_error_rather_than_a_result ... ok
test init::tests::the_separator_keeps_a_concatenation_from_colliding ... ok
test init::tests::the_snapshot_digest_is_stable_and_moves_with_the_admitted_set ... ok
test init::tests::an_agent_id_is_stable_across_calls ... ok
test input::tests::an_inline_object_is_parsed ... ok
test input::tests::input_accepts_only_the_stdin_marker ... ok
test input::tests::no_source_names_all_three_rather_than_defaulting_to_empty ... ok
test enrol::binding_update_tests::an_existing_writer_lock_is_refused_and_never_removed ... ok
test init::tests::an_existing_configuration_is_never_replaced_silently ... ok
test output::tests::a_payload_carrying_its_own_value_field_is_left_alone ... ok
test input::tests::a_file_is_read_from_its_path ... ok
test output::tests::a_field_a_record_does_not_carry_reads_as_absent_rather_than_blank ... ok
test output::tests::a_structured_format_carries_its_failure_on_stdout ... ok
test output::tests::a_record_that_is_not_an_object_keeps_the_name_the_report_gave_it ... ok
test output::tests::a_table_reads_left_to_right_with_the_column_that_runs_long_last ... ok
test output::tests::a_row_shows_its_severity_before_anybody_reads_it ... ok
test output::tests::a_wide_character_cell_keeps_the_column_after_it_aligned ... ok
test output::tests::a_word_the_renderer_cannot_rank_is_marked_unknown_rather_than_good ... ok
test output::tests::an_empty_listing_is_an_empty_stream_rather_than_a_line_shaped_like_a_record ... ok
test output::tests::an_object_with_two_arrays_is_not_unwrapped ... ok
test output::tests::an_unranked_table_still_keeps_the_marker_column ... ok
test output::tests::columns_of_equal_width_keep_the_order_the_record_carries ... ok
test output::tests::compact_keeps_a_field_a_record_carries_below_its_top_level ... ok
test output::tests::compact_leaves_a_single_record_as_one_line ... ok
test output::tests::compact_keeps_the_scalar_a_list_response_carries_beside_its_records ... ok
test output::tests::compact_unwraps_the_one_array_a_list_response_carries ... ok
test output::tests::every_protocol_state_this_renderer_can_be_handed_has_a_rank ... ok
test output::tests::no_cell_is_ever_empty_so_no_row_can_end_in_whitespace ... ok
test output::tests::severity_survives_a_pipe_because_it_is_not_carried_by_colour ... ok
test output::tests::each_output_format_preserves_the_binding_selector_reference_and_grant ... ok
test output::tests::text_does_not_quote_a_string_a_person_is_reading ... ok
test output::tests::text_says_none_rather_than_printing_an_empty_bracket ... ok
test output::tests::text_keeps_every_field_a_record_carries_including_a_nested_list ... ok
test output::tests::text_spends_one_aligned_row_on_each_record ... ok
test output::tests::the_result_discriminant_is_stripped_so_compact_can_see_the_records ... ok
test output::tests::the_structured_formats_render_the_bytes_they_rendered_before ... ok
test output::tests::the_widest_column_moves_last_even_when_the_record_puts_it_first ... ok
test output::tests::yaml_renders_through_the_maintained_crate ... ok
test output::tests::a_cell_never_carries_a_character_that_breaks_the_row ... ok
test output::tests::every_status_word_this_package_emits_is_one_the_renderer_can_rank ... ok
test init::tests::a_configuration_the_daemon_would_refuse_is_not_left_on_disk ... ok
test init::tests::what_init_writes_is_what_the_daemon_can_read ... ok
test enrol::tests::a_self_hosted_origin_is_the_case_operator_approval_exists_for ... ok
test auth::tests::the_catalogue_is_what_says_a_credential_has_a_user_half ... ok
test enrol::tests::slack_declares_a_bot_and_a_user_credential_which_one_identity_may_both_hold ... ok
test enrol::tests::gitlab_asks_for_nothing_when_its_default_origin_is_wanted ... ok
test enrol::tests::most_of_the_catalogue_asks_no_configuration_question_at_all ... ok
test providers::tests::an_unmatched_query_is_an_empty_listing_rather_than_the_whole_catalogue ... ok
test providers::tests::a_provider_without_a_probe_is_not_ready_and_says_why_by_omission ... ok
test enrol::binding_update_tests::a_rejected_configuration_update_never_replaces_previous_bytes ... ok
test auth::selected_status_tests::an_unselected_stored_sibling_does_not_make_a_connection_callable ... ok
test providers::tests::a_query_narrows_to_one_provider_and_its_summary_follows ... ok
test enrol::enrol_adversary_tests::adversary_refused_writer_lock_must_not_replace_the_live_selected_credential ... FAILED
test providers::tests::the_shipped_catalogue_is_reported_rather_than_asserted ... ok
test output::tests::two_providers_that_differ_in_their_id_differ_on_screen ... ok
test output::tests::a_table_too_wide_for_a_terminal_starts_its_last_column_inside_the_budget ... ok
test output::tests::the_budget_is_documented_as_what_it_is_and_a_real_row_is_wider_than_it ... ok
test output::tests::a_cell_the_budget_cut_says_so_and_the_column_names_are_cut_last ... ok
test enrol::binding_update_tests::an_explicit_write_update_changes_only_the_selected_binding ... ok
test enrol::binding_update_tests::adding_an_identity_preserves_the_existing_placement_and_its_policy ... ok
test enrol::binding_update_tests::enrollment_persists_the_new_selected_identity_and_explicit_write_updates ... ok

failures:

---- enrol::enrol_adversary_tests::adversary_refused_writer_lock_must_not_replace_the_live_selected_credential stdout ----
Connect Slack (slack)
Credential: slack.bot_token
  acts as an application identity, bounded by its own memberships and scopes
Input is hidden and goes straight to the credential store.

thread 'enrol::enrol_adversary_tests::adversary_refused_writer_lock_must_not_replace_the_live_selected_credential' (1602634) panicked at src/enrol_adversary_tests.rs:82:5:
assertion `left == right` failed: a rejected writer must not change the credential used by the already callable Connection
  left: "fixture-rejected-replacement"
 right: "fixture-live-original"
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace


failures:
    enrol::enrol_adversary_tests::adversary_refused_writer_lock_must_not_replace_the_live_selected_credential

test result: FAILED. 79 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.64s

error: test failed, to rerun pass `--lib`
     Running tests/adversary_budget_prose.rs (crates/connectors-console/target/debug/deps/adversary_budget_prose-00086c4b97ccd587)

running 3 tests
test pass3_render_helper_child ... ok
test the_quoted_module_header_sentence_is_at_the_line_the_pass_two_suite_cites ... ok
test the_widths_the_documents_state_are_the_widths_the_renderer_prints ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.12s

     Running tests/adversary_readability.rs (crates/connectors-console/target/debug/deps/adversary_readability-f75e840efc05f3e0)

running 7 tests
test render_helper_child ... ok
test a_record_whose_cells_are_all_empty_is_rendered_as_a_blank_line ... ok
test an_unranked_table_lets_a_cell_sit_where_the_severity_marker_sits ... ok
test a_wide_character_cell_leaves_the_column_after_it_ragged ... ok
test doctor_spreads_one_check_over_several_unmarked_lines_when_the_configuration_is_malformed ... ok
test compact_no_longer_puts_one_record_on_every_line ... ok
test providers_starts_its_last_column_past_the_width_of_any_terminal ... ok

test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.04s

     Running tests/adversary_readability_pass2.rs (crates/connectors-console/target/debug/deps/adversary_readability_pass2-9ec7479a47680986)

running 5 tests
test pass2_render_helper_child ... ok
test compact_drops_the_name_of_the_array_a_report_carries ... ok
test a_column_the_budget_squeezes_to_nothing_pushes_every_later_column_out_of_line ... ok
test compact_answers_an_empty_listing_with_a_line_that_is_not_a_record ... ok
test the_last_column_of_providers_begins_one_column_past_the_terminal_it_is_laid_out_for ... ok

test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.04s

   Doc-tests connectors_console

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

error: 1 target failed:
    `--lib`
```

`rustfmt --edition 2021 --check crates/integration-catalog/src/personal_connections_tests.rs crates/connectors-console/src/enrol_adversary_tests.rs` exited 0 with empty output. `git diff --check` exited 0 with empty output. No integrated repository gate or approval is claimed. Compiler output remained in the assigned worktree's runtime and console targets; CARGO_TARGET_DIR was unset. The observed filesystem had 46 GB free, above the 20 GB floor.

## 4. Findings and reachable callers

These findings cover source cacff114, with test registration 66c62dea and the retained test diff. Both origins are introduced based on the exact unit diff: the singleton-mechanism predicate is new; the writer lock and the existing-entry path through append_entry are new. Base 76f3fef9 was read with git show, never checked out or executed. No claim of a base suite run is made.

| File:line | Verdict / origin / severity | Finding | What was measured | What reaches it |
| --- | --- | --- | --- | --- |
| crates/integration-catalog/src/personal_connections.rs:33 | CONFIRMED / introduced / blocker | Singleton-only credential admission removes the existing Datadog conjunctive mechanism from callable discovery even when both keys are stored in one legacy placement. | personal_connections_tests.rs:481 asserts one operation after the actual compiled mechanism assembled two credentials; actual zero; isolated exit 101 and full catalog exit 101. | CLI operation search at connectors-cli/src/lib.rs:1208 reaches the registry and CatalogBackend::handle; personal composition opens this backend from catalog entries at connectors-runtime/src/composition.rs:422. providers/datadog.toml:116 and catalog/datadog.catalog.json declare the exact conjunction, and the resolver already supports it. |
| crates/connectors-console/src/enrol.rs:322 | CONFIRMED / introduced / blocker | Enrollment replaces an already callable Connection's selected credential before the new writer lock can reject the update. | enrol_adversary_tests.rs:82 observes fixture-rejected-replacement instead of fixture-live-original after run_with_store returns an error; unchanged configuration and foreign lock asserted first; isolated exit 101 and full console exit 101. | setup connect --instance fixture-app --as slack.bot_token --credential-file <owner-only-file> maps through connectors-cli/src/lib.rs:867, connect::dispatch at connect.rs:75 and public enrol::run at enrol.rs:130. A competing or interrupted enrollment leaves the exact enrol.lock append_entry creates at enrol.rs:468. |

Correction must preserve the selected identity boundary: an explicit Slack user binding must not borrow an alternative bot mechanism. Supporting the vendor's conjunctive requirement does not authorize unioning alternative identities. The enrollment issue can be reproduced without a concurrent timing race: an already existing writer lock suffices. The report's stated orphan-slot limitation does not describe this loss of an existing selected value.

No additional unmeasured judgement finding is returned.

## 5. Attacked without a break

- The same credential selector in another placement does not supply a missing selected credential; describe agrees with stored-slot ownership, and the refused invocation makes no egress.
- A locked selected user slot does not read or substitute a stored bot sibling; search exposes Unavailable and invocation refuses.
- Existing cases remain green for per-binding read/write ceilings, exact user versus bot egress, primary/additional reference reordering, missing/unknown/duplicate selectors, owner isolation, Connection-list/operation-reference consistency, and the frozen Connection envelope across real catalog providers.
- The LocalCredentialSelection ESS value and local config type both require credential:String and allow_writes:Boolean; the existing parser cases continue to reject missing policy, unknown binding fields, shorthand, duplicates and ambiguous combinations. No new global Credential cardinality was inferred.

## 6. External path inventory

Only the following persistent files and tool-managed/transient roots were written outside the assigned worktree. Tempfile fixture children under the assigned TMPDIR are removed by their owners. The sccache location is tool-managed and was not redirected or cleaned. No production provider, operator configuration, operator secret store, daemon, AEP artifact or Git mutation was touched by this adversary.

- ~/.cache/connectors-cli-wave-20260906/one-placement-several-credentials/adversary-1/datadog-case.log
- ~/.cache/connectors-cli-wave-20260906/one-placement-several-credentials/adversary-1/placement-case.log
- ~/.cache/connectors-cli-wave-20260906/one-placement-several-credentials/adversary-1/locked-slot-case.log
- ~/.cache/connectors-cli-wave-20260906/one-placement-several-credentials/adversary-1/enrollment-lock-case.log
- ~/.cache/connectors-cli-wave-20260906/one-placement-several-credentials/adversary-1/runtime-suites.log
- ~/.cache/connectors-cli-wave-20260906/one-placement-several-credentials/adversary-1/console-suite.log
- ~/.cache/connectors-cli-wave-20260906/one-placement-several-credentials/adversary-1/enrol-test-registration.patch
- ~/.cache/connectors-cli-wave-20260906/one-placement-several-credentials/adversary-1/raw-report.md
- ~/.cache/connectors-cli-wave-20260906/one-placement-several-credentials/adversary-1/report.md
- ~/.cache/cw6/m
- ~/.cache/sccache

Public report transformation replaces the local absolute home-directory prefix with ~; all other bytes, measured outputs, counts, findings and ordering are preserved.

```findings
- file: crates/integration-catalog/src/personal_connections.rs
  line: 33
  category: acceptance
  severity: blocker
  verdict: CONFIRMED
  origin: introduced
  message: Singleton-only credential admission removes the existing Datadog conjunctive mechanism from callable discovery even when both keys are stored in one legacy placement.
- file: crates/connectors-console/src/enrol.rs
  line: 322
  category: concurrency
  severity: blocker
  verdict: CONFIRMED
  origin: introduced
  message: Enrollment replaces an already callable Connection's selected credential before the new writer lock can reject the update.
```
