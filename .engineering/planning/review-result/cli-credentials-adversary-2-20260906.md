---
format: aep.planning-md/1
id: review-result:cli-credentials-adversary-2-20260906
kind: review-result
status: active
title: 'Final credential isolation review: inaccessible unrelated binding blocks healthy bot'
relations:
- reviews: story:one-placement-several-credentials
revision: 1
---
unit: story:one-placement-several-credentials; source 9f15c108fbd18cf6f079f0b04c0a9bb746b1ac0d plus retained test-only diff
verdict: CONFIRMED
cases: executed 229→234, red 1
origin: introduced 0 / pre-existing 0 / undecided 1
wrote-outside-worktree: 27 paths, listed below
needs-coordinator: record the final second-pass result and route the retained failure under the wave's two-pass bound

`git --no-pager diff --stat` (exit 0):

```text
 .../src/enrol_adversary_tests.rs                   | 218 +++++++++++++++++++++
 .../src/personal_connections_tests.rs              | 105 ++++++++++
 2 files changed, 323 insertions(+)
```

`git status --short` (exit 0):

```text
 M crates/connectors-console/src/enrol_adversary_tests.rs
 M crates/integration-catalog/src/personal_connections_tests.rs
```

Only these two existing test files were edited, by appending five deciding cases and their test-only fixture. No existing case was rewritten, weakened, skipped or deleted. No implementation file, including inline implementation-module tests, was edited. No Git or AEP mutation, agent dispatch, provider call, daemon action, operator configuration or operator secret access occurred.

## 2. Deciding cases, written before any test execution

All five cases were present and formatted before the first test execution. Each ran alone with an exact filter before any full suite. Runtime and console lanes ran independently; the order within each lane is retained in the logs. No failed case was repaired. An enum-variant spelling in a new test was corrected by reading the source before compilation; every deciding command compiled and executed exactly one case on its first run.

### locked-unrelated

crates/integration-catalog/src/personal_connections_tests.rs:921. Red. The Connection describe result says the writable bot is Callable. Operation describe for its admitted Slack post returns Unavailable because the separate read-only user slot is denied. This is the deciding failure.

Command (exit 101):

```text
env -u CARGO_TARGET_DIR TMPDIR=~/.cache/cw6/m RUSTC_WRAPPER=/usr/bin/sccache CARGO_INCREMENTAL=0 CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 CARGO_BUILD_JOBS=3 cargo test --manifest-path crates/connectors-runtime/Cargo.toml --locked -p integration-catalog personal_connections_tests::adversary2_an_unadmitted_locked_user_does_not_hide_the_callable_bot_post -- --exact
```

Verbatim output:

```text
    Blocking waiting for file lock on package cache
    Blocking waiting for file lock on package cache
   Compiling integration-catalog v0.6.5 (~/.local/state/worktree/trees/b10x/connectors/wt-03d120cf8736/crates/integration-catalog)
    Finished `test` profile [unoptimized] target(s) in 1.45s
     Running unittests src/lib.rs (crates/connectors-runtime/target/debug/deps/integration_catalog-f55a18277ee5bce7)

running 1 test
test personal_connections_tests::adversary2_an_unadmitted_locked_user_does_not_hide_the_callable_bot_post ... FAILED

failures:

---- personal_connections_tests::adversary2_an_unadmitted_locked_user_does_not_hide_the_callable_bot_post stdout ----

thread 'personal_connections_tests::adversary2_an_unadmitted_locked_user_does_not_hide_the_callable_bot_post' (2036619) panicked at ~/.local/state/worktree/trees/b10x/connectors/wt-03d120cf8736/crates/integration-catalog/src/personal_connections_tests.rs:964:5:
the read-only user's denied slot cannot admit this post and must not suppress the callable bot: Err(OperationError { code: Unavailable, message: "credential presence could not be checked", retriable: false })
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace


failures:
    personal_connections_tests::adversary2_an_unadmitted_locked_user_does_not_hide_the_callable_bot_post

test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 53 filtered out; finished in 1.12s

error: test failed, to rerun pass `-p integration-catalog --lib`
```

### removed-after-describe

crates/integration-catalog/src/personal_connections_tests.rs:979. Green. Removing the selected user credential after describe refuses invocation with NotGranted and zero egress; a stored bot is never substituted.

Command (exit 0):

```text
env -u CARGO_TARGET_DIR TMPDIR=~/.cache/cw6/m RUSTC_WRAPPER=/usr/bin/sccache CARGO_INCREMENTAL=0 CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 CARGO_BUILD_JOBS=3 cargo test --manifest-path crates/connectors-runtime/Cargo.toml --locked -p integration-catalog personal_connections_tests::adversary2_removing_a_described_user_credential_never_invokes_the_bot_instead -- --exact
```

Verbatim output:

```text
    Finished `test` profile [unoptimized] target(s) in 0.15s
     Running unittests src/lib.rs (crates/connectors-runtime/target/debug/deps/integration_catalog-f55a18277ee5bce7)

running 1 test
test personal_connections_tests::adversary2_removing_a_described_user_credential_never_invokes_the_bot_instead ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 53 filtered out; finished in 1.03s

```

### cancel-before-publication

crates/connectors-console/src/enrol_adversary_tests.rs:428. Green. A paused readiness check holds the writer guard; a competing enrollment refuses. Cancellation before publication preserves config bytes and the live value, and removes only the owned lock/staged files.

Command (exit 0):

```text
env -u CARGO_TARGET_DIR TMPDIR=~/.cache/cw6/m RUSTC_WRAPPER=/usr/bin/sccache CARGO_INCREMENTAL=0 CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 CARGO_BUILD_JOBS=3 cargo test --manifest-path crates/connectors-console/Cargo.toml --locked enrol::enrol_adversary_tests::adversary2_cancellation_before_publication_releases_only_owned_files_and_keeps_live_value -- --exact
```

Verbatim output:

```text
    Blocking waiting for file lock on package cache
    Blocking waiting for file lock on package cache
   Compiling connectors-console v0.6.5 (~/.local/state/worktree/trees/b10x/connectors/wt-03d120cf8736/crates/connectors-console)
    Finished `test` profile [unoptimized] target(s) in 2.25s
     Running unittests src/lib.rs (crates/connectors-console/target/debug/deps/connectors_console-fbbac75288b56ca5)

running 1 test
test enrol::enrol_adversary_tests::adversary2_cancellation_before_publication_releases_only_owned_files_and_keeps_live_value ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 88 filtered out; finished in 1.05s

     Running tests/adversary_budget_prose.rs (crates/connectors-console/target/debug/deps/adversary_budget_prose-b9e90466491d04c5)

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 3 filtered out; finished in 0.00s

     Running tests/adversary_readability.rs (crates/connectors-console/target/debug/deps/adversary_readability-e11396cb0c03f757)

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 7 filtered out; finished in 0.00s

     Running tests/adversary_readability_pass2.rs (crates/connectors-console/target/debug/deps/adversary_readability_pass2-4a6dea2d8dbb1d3d)

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 5 filtered out; finished in 0.00s

```

### cancel-during-custody

crates/connectors-console/src/enrol_adversary_tests.rs:498. Green. Cancellation after config publication but during custody leaves the new read-only user binding with the previous stored value, releases the lock, and performs no unsafe rollback. This tests the explicitly accepted partial-outcome limit, not a global transaction guarantee.

Command (exit 0):

```text
env -u CARGO_TARGET_DIR TMPDIR=~/.cache/cw6/m RUSTC_WRAPPER=/usr/bin/sccache CARGO_INCREMENTAL=0 CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 CARGO_BUILD_JOBS=3 cargo test --manifest-path crates/connectors-console/Cargo.toml --locked enrol::enrol_adversary_tests::adversary2_cancellation_during_custody_keeps_the_declared_partial_outcome -- --exact
```

Verbatim output:

```text
    Finished `test` profile [unoptimized] target(s) in 0.11s
     Running unittests src/lib.rs (crates/connectors-console/target/debug/deps/connectors_console-fbbac75288b56ca5)

running 1 test
test enrol::enrol_adversary_tests::adversary2_cancellation_during_custody_keeps_the_declared_partial_outcome ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 88 filtered out; finished in 1.09s

     Running tests/adversary_budget_prose.rs (crates/connectors-console/target/debug/deps/adversary_budget_prose-b9e90466491d04c5)

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 3 filtered out; finished in 0.00s

     Running tests/adversary_readability.rs (crates/connectors-console/target/debug/deps/adversary_readability-e11396cb0c03f757)

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 7 filtered out; finished in 0.00s

     Running tests/adversary_readability_pass2.rs (crates/connectors-console/target/debug/deps/adversary_readability_pass2-4a6dea2d8dbb1d3d)

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 5 filtered out; finished in 0.00s

```

### legacy-default-selector

crates/connectors-console/src/enrol_adversary_tests.rs:557. Green. Re-enrollment without --as preserves an existing legacy user selector, its single-binding shape, stable Connection reference and write policy, and stores no bot credential.

Command (exit 0):

```text
env -u CARGO_TARGET_DIR TMPDIR=~/.cache/cw6/m RUSTC_WRAPPER=/usr/bin/sccache CARGO_INCREMENTAL=0 CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 CARGO_BUILD_JOBS=3 cargo test --manifest-path crates/connectors-console/Cargo.toml --locked enrol::enrol_adversary_tests::adversary2_reenrollment_without_as_keeps_a_legacy_user_selector_and_address -- --exact
```

Verbatim output:

```text
    Finished `test` profile [unoptimized] target(s) in 0.12s
     Running unittests src/lib.rs (crates/connectors-console/target/debug/deps/connectors_console-fbbac75288b56ca5)

running 1 test
test enrol::enrol_adversary_tests::adversary2_reenrollment_without_as_keeps_a_legacy_user_selector_and_address ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 88 filtered out; finished in 1.04s

     Running tests/adversary_budget_prose.rs (crates/connectors-console/target/debug/deps/adversary_budget_prose-b9e90466491d04c5)

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 3 filtered out; finished in 0.00s

     Running tests/adversary_readability.rs (crates/connectors-console/target/debug/deps/adversary_readability-e11396cb0c03f757)

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 7 filtered out; finished in 0.00s

     Running tests/adversary_readability_pass2.rs (crates/connectors-console/target/debug/deps/adversary_readability_pass2-4a6dea2d8dbb1d3d)

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 5 filtered out; finished in 0.00s

```

## 3. Full package suites after the deciding runs

The before count of 229 is the corrected implementor report's measured aggregate: config 27, catalog 52, console 101, resolver 49. It was supplied in this pass's brief and was not obtained by running a suite before adding cases. The current full commands ran 234 cases: config 27, catalog 54, console 104, resolver 49. Exactly 233 passed and one failed; none were ignored. The unchanged resolver suite includes 48 unit tests and one doc test. The console count includes 89 unit and 15 integration-test executions. This is package-level verification, not the repository's full integration gate.

Command (exit 101):

```text
env -u CARGO_TARGET_DIR TMPDIR=~/.cache/cw6/m RUSTC_WRAPPER=/usr/bin/sccache CARGO_INCREMENTAL=0 CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 CARGO_BUILD_JOBS=3 cargo test --manifest-path crates/connectors-runtime/Cargo.toml --locked --no-fail-fast -p connectors-config -p integration-catalog
```

Verbatim output:

```text
    Blocking waiting for file lock on package cache
    Blocking waiting for file lock on package cache
    Blocking waiting for file lock on package cache
    Finished `test` profile [unoptimized] target(s) in 0.25s
     Running unittests src/lib.rs (crates/connectors-runtime/target/debug/deps/connectors_config-f982a143bd730f2e)

running 24 tests
test hosted::tests::claude_code_custody_is_explicit_and_requires_the_hosted_secret_store ... ok
test hosted::tests::hosted_configuration_uses_same_handle_and_refuses_mutable_or_symlinked_files ... ok
test hosted::tests::an_existing_hosted_config_with_the_old_b10x_section_parses_unchanged ... ok
test personal::catalog::tests::explicit_policy_per_credential_is_a_valid_value_free_configuration ... ok
test hosted::tests::kubernetes_namespace_groups_are_exact_sorted_and_restart_is_a_read_subset ... ok
test personal::tests::a_trunk_address_may_be_a_literal_or_a_name_and_nothing_else ... ok
test hosted::tests::hosted_integrations_are_explicit_and_fail_closed ... ok
test hosted::tests::hosted_vault_is_all_or_nothing ... ok
test personal::catalog::tests::credential_policy_is_explicit_and_unknown_binding_fields_are_refused ... ok
test hosted::tests::hosted_jira_service_api_token_excludes_a_service_oauth_client_id ... ok
test hosted::tests::hosted_vault_requires_a_valid_distinct_sip_digest_pair ... ok
test hosted::tests::hosted_gitlab_requires_vault_and_a_same_origin_callback ... ok
test personal::catalog::tests::legacy_single_selection_retains_its_shape_and_write_policy ... ok
test hosted::tests::hosted_deployment_accepts_planner_integration ... ok
test hosted::tests::hosted_grafana_requires_vault_exact_groups_and_digest_bound_targets ... ok
test hosted::tests::hosted_jira_separates_organization_and_user_authority ... ok
test personal::tests::slack_only_configuration_contains_policy_but_no_secret_source ... ok
test personal::tests::an_existing_personal_config_with_the_old_b10x_section_parses_unchanged ... ok
test personal::catalog::tests::duplicate_empty_and_ambiguous_selections_are_refused ... ok
test personal::tests::grafana_configuration_names_origin_and_independent_target_grants_only ... ok
test personal::tests::kubernetes_configuration_is_policy_only ... ok
test personal::tests::the_named_default_trunk_example_parses_and_dials_by_number ... ok
test personal::tests::deployment_configuration_cannot_be_symlinked_or_group_writable ... ok
test personal::tests::documented_development_configuration_stays_strict_and_valid ... ok

test result: ok. 24 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests/catalog_usernames.rs (crates/connectors-runtime/target/debug/deps/catalog_usernames-d53b5bdd7fa1175f)

running 3 tests
test an_entry_with_no_user_half_still_reads_and_reports_none ... ok
test a_basic_credential_carries_its_user_half_beside_its_endpoints ... ok
test a_user_half_is_refused_when_it_is_empty_or_could_not_travel ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running unittests src/lib.rs (crates/connectors-runtime/target/debug/deps/integration_catalog-f55a18277ee5bce7)

running 54 tests
test argocd::tests::a_project_or_role_name_that_could_change_the_path_is_refused ... ok
test argocd::tests::a_zero_lifetime_is_refused_rather_than_meaning_forever ... ok
test argocd::tests::a_rejected_sign_in_says_so_rather_than_reporting_a_missing_project ... ok
test argocd::tests::a_missing_project_is_not_reported_as_a_permission_problem ... ok
test argocd::tests::every_generated_policy_is_one_argo_cd_will_accept ... ok
test argocd::tests::a_login_without_projects_update_is_told_which_grant_it_lacks ... ok
test argocd::tests::an_unreachable_origin_names_the_aperture_as_a_possibility ... ok
test argocd::tests::read_only_acquisition_carries_no_sync_policy ... ok
test argocd::tests::an_existing_role_is_reused_and_its_policies_are_left_alone ... ok
test argocd::tests::the_four_calls_happen_in_order_and_the_token_comes_back ... ok
test config::tests::a_value_without_operator_approval_says_so_rather_than_claiming_it ... ok
test config::tests::an_endpoint_value_reaches_the_resolver_under_its_declared_name ... ok
test config::tests::an_unsupplied_variable_is_absent_rather_than_empty ... ok
test argocd::tests::the_password_appears_once_and_the_session_token_never_persists ... ok
test argocd::tests::the_project_is_written_back_whole ... ok
test personal_connections_tests::duplicate_placement_references_are_refused_at_composition ... ok
test personal_connections_tests::correction_selection_assembler_never_uses_an_unselected_subset_alternative ... ok
test personal_connections_tests::adversary_legacy_compound_auth_keeps_the_catalogued_datadog_read_callable ... ok
test personal_connections_tests::a_missing_selected_credential_is_absent_from_callable_discovery ... ok
test personal_connections_tests::selecting_one_credential_hides_other_mechanisms_and_keeps_legacy_addresses ... ok
test personal_connections_tests::primary_and_additional_references_do_not_depend_on_binding_order ... ok
test personal_connections_tests::discovery_and_connection_metadata_refuse_another_owner ... ok
test personal_connections_tests::unknown_selectors_are_refused_even_without_a_bootstrap_file ... ok
test personal_connections_tests::correction_every_catalogued_mechanism_preserves_selected_members_and_write_ceilings ... ok
test personal_connections_tests::correction_compound_write_ceiling_and_other_placement_never_supply_authority ... ok
test tests::a_provider_with_a_fixed_base_url_needs_no_configuration ... ok
test personal_connections_tests::adversary2_an_unadmitted_locked_user_does_not_hide_the_callable_bot_post ... FAILED
test tests::a_network_effect_is_not_an_escalation_but_a_filesystem_one_would_be ... ok
test tests::effect_class_comes_from_the_declaration_not_the_method ... ok
test tests::a_basic_credentials_user_half_reaches_the_resolver_from_the_entry ... ok
test tests::an_unnamed_entry_keeps_the_address_it_had_before_instances_existed ... ok
test tests::every_catalogued_provider_can_address_a_credential ... ok
test tests::an_unsupplied_url_variable_refuses_rather_than_reaching_a_literal_placeholder ... ok
test tests::a_read_only_first_connection_does_not_hide_an_admitted_write ... ok
test tests::the_aperture_is_derived_from_the_same_declaration_the_request_is ... ok
test tests::the_ceiling_reads_declared_facts_rather_than_an_operation_list ... ok
test tests::every_declared_user_half_field_names_a_credential_that_actually_wants_one ... ok
test tests::an_entry_stating_no_user_half_answers_none_rather_than_an_empty_string ... ok
test tests::the_instance_derivation_puts_the_provider_in_the_namespace ... ok
test tests::the_default_ceiling_admits_reads_and_refuses_writes ... ok
test tests::the_limit_drops_operations_rather_than_the_identities_that_serve_one ... ok
test tests::two_named_instances_of_one_provider_get_different_addresses ... ok
test tests::search_and_describe_require_approval_for_every_admitted_write ... ok
test personal_connections_tests::every_catalog_connection_summary_obeys_the_frozen_wire_contract ... ok
test personal_connections_tests::a_selected_user_never_falls_back_to_a_stored_bot ... ok
test personal_connections_tests::adversary_a_locked_selected_slot_never_reads_a_callable_sibling ... ok
test hosted::tests::completion_does_not_overwrite_a_terminal_session_after_verification_awaits ... ok
test personal_connections_tests::adversary_the_same_selector_in_another_placement_cannot_supply_this_connection ... ok
test personal_connections_tests::adversary2_removing_a_described_user_credential_never_invokes_the_bot_instead ... ok
test tests::a_request_template_exists_for_every_operation_this_backend_would_offer ... ok
test personal_connections_tests::correction_complete_compound_binding_invokes_both_declared_keys_for_either_selector ... ok
test personal_connections_tests::two_selected_credentials_have_distinct_connections_and_exact_egress ... ok
test hosted::tests::two_people_receive_isolated_connections_and_credential_addresses ... ok
test personal_connections_tests::correction_incomplete_compound_bindings_are_never_callable_or_invoked ... ok

failures:

---- personal_connections_tests::adversary2_an_unadmitted_locked_user_does_not_hide_the_callable_bot_post stdout ----

thread 'personal_connections_tests::adversary2_an_unadmitted_locked_user_does_not_hide_the_callable_bot_post' (2046791) panicked at ~/.local/state/worktree/trees/b10x/connectors/wt-03d120cf8736/crates/integration-catalog/src/personal_connections_tests.rs:964:5:
the read-only user's denied slot cannot admit this post and must not suppress the callable bot: Err(OperationError { code: Unavailable, message: "credential presence could not be checked", retriable: false })
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace


failures:
    personal_connections_tests::adversary2_an_unadmitted_locked_user_does_not_hide_the_callable_bot_post

test result: FAILED. 53 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.03s

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

Command (exit 0):

```text
env -u CARGO_TARGET_DIR TMPDIR=~/.cache/cw6/m RUSTC_WRAPPER=/usr/bin/sccache CARGO_INCREMENTAL=0 CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 CARGO_BUILD_JOBS=3 cargo test --manifest-path crates/connectors-console/Cargo.toml --locked --no-fail-fast
```

Verbatim output:

```text
    Blocking waiting for file lock on package cache
    Blocking waiting for file lock on package cache
    Blocking waiting for file lock on package cache
    Blocking waiting for file lock on package cache
    Finished `test` profile [unoptimized] target(s) in 0.26s
     Running unittests src/lib.rs (crates/connectors-console/target/debug/deps/connectors_console-fbbac75288b56ca5)

running 89 tests
test auth::tests::nothing_in_the_result_can_carry_a_secret ... ok
test admin::tests::explicit_secret_file_must_be_owner_only ... ok
test auth::tests::the_store_preference_matches_what_the_runtime_composes ... ok
test auth::tests::a_basic_credential_row_reports_whether_its_user_half_is_configured_and_never_the_value ... ok
test connect::tests::a_provider_outside_the_guided_set_is_refused_by_name ... ok
test connect::tests::the_error_for_an_unknown_provider_names_it ... ok
test doctor::tests::a_missing_configuration_is_fatal_and_names_the_command_that_fixes_it ... ok
test doctor::tests::a_report_is_unhealthy_only_when_something_cannot_work ... ok
test doctor::tests::a_short_state_root_passes_both_budgets ... ok
test admin::tests::command_shape_accepts_secret_stdin_without_a_secret_argument ... ok
test doctor::tests::the_budget_is_measured_against_the_deepest_path_the_daemon_binds ... ok
test doctor::tests::the_report_renders_every_check_as_data ... ok
test doctor::tests::every_state_a_check_can_report_reaches_the_reader_as_its_own_marker ... ok
test doctor::tests::doctor_names_the_default_local_target_and_its_socket ... ok
test connect::tests::a_catalogued_provider_whose_curated_backend_is_absent_takes_the_catalogue_path ... ok
test enrol::tests::a_provider_outside_the_catalogue_is_named_rather_than_guessed_at ... ok
test enrol::binding_update_tests::an_existing_writer_lock_is_refused_and_never_removed ... ok
test enrol::tests::a_self_hosted_origin_is_the_case_operator_approval_exists_for ... ok
test enrol::tests::most_of_the_catalogue_asks_no_configuration_question_at_all ... ok
test auth::tests::the_catalogue_is_what_says_a_credential_has_a_user_half ... ok
test enrol::tests::gitlab_asks_for_nothing_when_its_default_origin_is_wanted ... ok
test envelope::tests::a_refusal_becomes_an_error_rather_than_a_result ... ok
test envelope::tests::a_result_loses_its_envelope_and_its_discriminant ... ok
test envelope::tests::an_envelope_carrying_neither_is_a_named_failure_not_an_empty_success ... ok
test enrol::binding_update_tests::a_rejected_configuration_update_never_replaces_previous_bytes ... ok
test enrol::enrol_adversary_tests::adversary_refused_writer_lock_must_not_replace_the_live_selected_credential ... ok
test init::tests::an_agent_id_is_stable_across_calls ... ok
test enrol::enrol_adversary_tests::correction_malformed_existing_config_does_not_write_any_secret ... ok
test init::tests::admitting_a_credential_plugin_is_a_choice_and_its_absence_is_explained ... ok
test init::tests::the_separator_keeps_a_concatenation_from_colliding ... ok
test auth::selected_status_tests::an_unselected_stored_sibling_does_not_make_a_connection_callable ... ok
test input::tests::input_accepts_only_the_stdin_marker ... ok
test input::tests::an_inline_object_is_parsed ... ok
test init::tests::the_snapshot_digest_is_stable_and_moves_with_the_admitted_set ... ok
test input::tests::no_source_names_all_three_rather_than_defaulting_to_empty ... ok
test init::tests::an_existing_configuration_is_never_replaced_silently ... ok
test output::tests::a_payload_carrying_its_own_value_field_is_left_alone ... ok
test output::tests::a_field_a_record_does_not_carry_reads_as_absent_rather_than_blank ... ok
test output::tests::a_record_that_is_not_an_object_keeps_the_name_the_report_gave_it ... ok
test input::tests::a_file_is_read_from_its_path ... ok
test output::tests::a_structured_format_carries_its_failure_on_stdout ... ok
test output::tests::a_row_shows_its_severity_before_anybody_reads_it ... ok
test output::tests::a_table_reads_left_to_right_with_the_column_that_runs_long_last ... ok
test output::tests::a_wide_character_cell_keeps_the_column_after_it_aligned ... ok
test output::tests::a_word_the_renderer_cannot_rank_is_marked_unknown_rather_than_good ... ok
test output::tests::an_empty_listing_is_an_empty_stream_rather_than_a_line_shaped_like_a_record ... ok
test output::tests::an_object_with_two_arrays_is_not_unwrapped ... ok
test output::tests::an_unranked_table_still_keeps_the_marker_column ... ok
test output::tests::columns_of_equal_width_keep_the_order_the_record_carries ... ok
test output::tests::compact_leaves_a_single_record_as_one_line ... ok
test output::tests::compact_keeps_a_field_a_record_carries_below_its_top_level ... ok
test output::tests::compact_keeps_the_scalar_a_list_response_carries_beside_its_records ... ok
test output::tests::compact_unwraps_the_one_array_a_list_response_carries ... ok
test output::tests::every_protocol_state_this_renderer_can_be_handed_has_a_rank ... ok
test output::tests::no_cell_is_ever_empty_so_no_row_can_end_in_whitespace ... ok
test output::tests::severity_survives_a_pipe_because_it_is_not_carried_by_colour ... ok
test output::tests::each_output_format_preserves_the_binding_selector_reference_and_grant ... ok
test enrol::enrol_adversary_tests::correction_invalid_intended_config_preserves_live_credential ... ok
test output::tests::text_does_not_quote_a_string_a_person_is_reading ... ok
test output::tests::text_says_none_rather_than_printing_an_empty_bracket ... ok
test output::tests::text_keeps_every_field_a_record_carries_including_a_nested_list ... ok
test output::tests::text_spends_one_aligned_row_on_each_record ... ok
test output::tests::the_result_discriminant_is_stripped_so_compact_can_see_the_records ... ok
test output::tests::the_widest_column_moves_last_even_when_the_record_puts_it_first ... ok
test output::tests::the_structured_formats_render_the_bytes_they_rendered_before ... ok
test enrol::tests::slack_declares_a_bot_and_a_user_credential_which_one_identity_may_both_hold ... ok
test auth::selected_status_tests::correction_compound_auth_diagnostics_require_every_member ... ok
test output::tests::yaml_renders_through_the_maintained_crate ... ok
test providers::tests::a_provider_without_a_probe_is_not_ready_and_says_why_by_omission ... ok
test providers::tests::an_unmatched_query_is_an_empty_listing_rather_than_the_whole_catalogue ... ok
test providers::tests::a_query_narrows_to_one_provider_and_its_summary_follows ... ok
test output::tests::a_cell_never_carries_a_character_that_breaks_the_row ... ok
test providers::tests::the_shipped_catalogue_is_reported_rather_than_asserted ... ok
test output::tests::a_table_too_wide_for_a_terminal_starts_its_last_column_inside_the_budget ... ok
test output::tests::the_budget_is_documented_as_what_it_is_and_a_real_row_is_wider_than_it ... ok
test output::tests::two_providers_that_differ_in_their_id_differ_on_screen ... ok
test output::tests::every_status_word_this_package_emits_is_one_the_renderer_can_rank ... ok
test init::tests::a_configuration_the_daemon_would_refuse_is_not_left_on_disk ... ok
test output::tests::a_cell_the_budget_cut_says_so_and_the_column_names_are_cut_last ... ok
test enrol::enrol_adversary_tests::adversary2_cancellation_before_publication_releases_only_owned_files_and_keeps_live_value ... ok
test init::tests::what_init_writes_is_what_the_daemon_can_read ... ok
test enrol::enrol_adversary_tests::correction_an_externally_replaced_writer_lock_is_never_removed ... ok
test enrol::binding_update_tests::adding_an_identity_preserves_the_existing_placement_and_its_policy ... ok
test enrol::binding_update_tests::an_explicit_write_update_changes_only_the_selected_binding ... ok
test enrol::enrol_adversary_tests::adversary2_reenrollment_without_as_keeps_a_legacy_user_selector_and_address ... ok
test enrol::enrol_adversary_tests::adversary2_cancellation_during_custody_keeps_the_declared_partial_outcome ... ok
test enrol::enrol_adversary_tests::correction_staging_or_concurrent_config_failure_never_replaces_live_slot ... ok
test enrol::binding_update_tests::enrollment_persists_the_new_selected_identity_and_explicit_write_updates ... ok
test enrol::enrol_adversary_tests::correction_store_failure_names_committed_configuration_without_unsafe_rollback ... ok

test result: ok. 89 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.10s

     Running tests/adversary_budget_prose.rs (crates/connectors-console/target/debug/deps/adversary_budget_prose-b9e90466491d04c5)

running 3 tests
test pass3_render_helper_child ... ok
test the_quoted_module_header_sentence_is_at_the_line_the_pass_two_suite_cites ... ok
test the_widths_the_documents_state_are_the_widths_the_renderer_prints ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.99s

     Running tests/adversary_readability.rs (crates/connectors-console/target/debug/deps/adversary_readability-e11396cb0c03f757)

running 7 tests
test render_helper_child ... ok
test a_wide_character_cell_leaves_the_column_after_it_ragged ... ok
test a_record_whose_cells_are_all_empty_is_rendered_as_a_blank_line ... ok
test an_unranked_table_lets_a_cell_sit_where_the_severity_marker_sits ... ok
test doctor_spreads_one_check_over_several_unmarked_lines_when_the_configuration_is_malformed ... ok
test compact_no_longer_puts_one_record_on_every_line ... ok
test providers_starts_its_last_column_past_the_width_of_any_terminal ... ok

test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.00s

     Running tests/adversary_readability_pass2.rs (crates/connectors-console/target/debug/deps/adversary_readability_pass2-4a6dea2d8dbb1d3d)

running 5 tests
test pass2_render_helper_child ... ok
test compact_drops_the_name_of_the_array_a_report_carries ... ok
test a_column_the_budget_squeezes_to_nothing_pushes_every_later_column_out_of_line ... ok
test compact_answers_an_empty_listing_with_a_line_that_is_not_a_record ... ok
test the_last_column_of_providers_begins_one_column_past_the_terminal_it_is_laid_out_for ... ok

test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.00s

   Doc-tests connectors_console

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

```

Command (exit 0):

```text
env -u CARGO_TARGET_DIR TMPDIR=~/.cache/cw6/m RUSTC_WRAPPER=/usr/bin/sccache CARGO_INCREMENTAL=0 CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 CARGO_BUILD_JOBS=3 cargo test --manifest-path Cargo.toml --locked --no-fail-fast -p connector-resolve
```

Verbatim output:

```text
    Blocking waiting for file lock on package cache
    Blocking waiting for file lock on package cache
    Blocking waiting for file lock on package cache
    Finished `test` profile [unoptimized] target(s) in 0.23s
     Running unittests src/lib.rs (target/debug/deps/connector_resolve-34efe3bf274bb692)

running 48 tests
test auth::tests::a_header_the_template_already_sets_is_refused_rather_than_overwritten ... ok
test auth::tests::a_basic_join_composes_the_pair_the_vendor_expects ... ok
test auth::tests::a_prefixed_header_carries_the_bare_value_inside_it ... ok
test auth::tests::a_query_placement_registers_the_encoded_form_it_sends ... ok
test auth::tests::an_inbound_signing_secret_never_leaves ... ok
test auth::tests::base64_matches_rfc_4648s_own_vectors ... ok
test auth::tests::the_assembled_debug_prints_no_value ... ok
test config::tests::a_config_value_debug_carries_no_value ... ok
test config::tests::a_username_prefix_is_the_one_reserved_qualifier ... ok
test document::tests::a_pre_c552_document_without_symbols_falls_back_to_the_allocation ... ok
test document::tests::a_sip_session_driver_survives_the_canonical_document ... ok
test document::tests::a_stated_symbol_is_honored_over_the_naive_allocation ... ok
test document::tests::the_emitters_own_symbols_are_reserved ... ok
test plan::tests::sensitive_text_never_prints ... ok
test plan::tests::a_plans_debug_carries_no_credential ... ok
test document::tests::the_symbol_allocation_reproduces_the_emitters ... ok
test request::tests::a_duplicate_query_key_is_refused_rather_than_sent_twice ... ok
test request::tests::a_fragment_survives_the_appended_query ... ok
test request::tests::debug_prints_shape_and_never_a_value ... ok
test request::tests::the_default_identity_names_this_software_and_this_repository ... ok
test request::tests::the_params_omit_what_is_absent ... ok
test slot::tests::a_document_position_maps_onto_a_slot_and_anything_else_fails_closed ... ok
test slot::tests::a_host_rule_refuses_what_moves_the_authority ... ok
test slot::tests::a_value_that_moves_the_authority_is_refused_in_context ... ok
test slot::tests::an_unplaced_value_is_held_to_every_rule_including_the_hosts ... ok
test template::tests::a_doubled_brace_is_an_escape_and_an_unterminated_one_is_text ... ok
test template::tests::an_unfilled_placeholder_stays_verbatim ... ok
test template::tests::markers_are_located_and_filled_by_offset ... ok
test template::tests::truthiness_is_flux_langs ... ok
test template::tests::value_to_text_is_flux_langs ... ok
test document::tests::a_shipped_document_parses_into_its_services_and_operations ... ok
test document::tests::an_unknown_provider_or_operation_is_absent_rather_than_a_panic ... ok
test resolve::tests::a_caller_parameter_that_leaves_its_path_segment_is_refused ... ok
test resolve::tests::a_caller_value_spelling_a_configuration_variable_does_not_reach_the_wire ... ok
test resolve::tests::a_configuration_value_that_moves_the_authority_is_refused ... ok
test resolve::tests::an_omitted_required_parameter_is_refused_and_names_itself ... ok
test resolve::tests::a_null_query_field_is_omitted_rather_than_sent_empty ... ok
test resolve::tests::an_optional_query_filter_may_simply_be_left_out ... ok
test resolve::tests::every_request_carries_this_softwares_identity ... ok
test resolve::tests::an_omitted_optional_body_field_is_not_a_missing_parameter ... ok
test resolve::tests::gitlab_publication_keeps_the_reviewed_actions_in_one_json_request ... ok
test resolve::tests::the_plan_carries_the_placed_credential_and_the_set_to_redact ... ok
test resolve::tests::the_request_is_the_documents_request ... ok
test document::tests::an_operation_resolves_through_the_embedded_pack_without_naming_its_provider ... ok
test credentials::tests::a_basic_join_reads_its_user_half_from_the_config_port ... ok
test credentials::tests::a_bearer_credential_assembles_and_lists_its_redaction ... ok
test credentials::tests::a_basic_join_with_no_user_half_refuses_by_name ... ok
test credentials::tests::an_unstored_credential_refuses_and_names_the_alternatives ... ok

test result: ok. 48 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.83s

   Doc-tests connector_resolve

running 1 test
test crates/connector-resolve/src/lib.rs - (line 3) - compile ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s

```

Strict clippy and formatting were run after these suites, without changing any case:

Command (exit 0):

```text
env -u CARGO_TARGET_DIR TMPDIR=~/.cache/cw6/m RUSTC_WRAPPER=/usr/bin/sccache CARGO_INCREMENTAL=0 CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 CARGO_BUILD_JOBS=3 cargo clippy --manifest-path crates/connectors-runtime/Cargo.toml --locked -p connectors-config -p integration-catalog --all-targets -- -D warnings
```

Verbatim output:

```text
    Blocking waiting for file lock on package cache
    Blocking waiting for file lock on package cache
    Checking integration-catalog v0.6.5 (~/.local/state/worktree/trees/b10x/connectors/wt-03d120cf8736/crates/integration-catalog)
    Finished `dev` profile [unoptimized] target(s) in 1.06s
```

Command (exit 0):

```text
env -u CARGO_TARGET_DIR TMPDIR=~/.cache/cw6/m RUSTC_WRAPPER=/usr/bin/sccache CARGO_INCREMENTAL=0 CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 CARGO_BUILD_JOBS=3 cargo clippy --manifest-path crates/connectors-console/Cargo.toml --locked --all-targets -- -D warnings
```

Verbatim output:

```text
    Blocking waiting for file lock on package cache
    Blocking waiting for file lock on package cache
    Checking connectors-console v0.6.5 (~/.local/state/worktree/trees/b10x/connectors/wt-03d120cf8736/crates/connectors-console)
    Finished `dev` profile [unoptimized] target(s) in 1.11s
```

Command (exit 0):

```text
rustfmt --edition 2021 --check crates/integration-catalog/src/personal_connections_tests.rs crates/connectors-console/src/enrol_adversary_tests.rs
```

Verbatim output (empty):

```text
```

`git --no-pager diff --check` exited 0 with empty output. Disk at completion was 34 GB free, above the 20 GB floor. All builds stayed in the assigned worktree's root, runtime and console targets, with CARGO_TARGET_DIR unset.

## 4. Finding

| File:line | Category / severity | Verdict / origin | What was measured | What reaches it |
| --- | --- | --- | --- | --- |
| crates/integration-catalog/src/personal_connections_tests.rs:964 | contract-drift / blocker | CONFIRMED / undecided | A denied credential slot on a read-only user binding aborts description of the separately callable bot's admitted Slack post. The exact deciding case exits 101; its Connection describe check first succeeds with Callable, then operation describe returns Unavailable. | The documented dual-credential Slack configuration and ordinary `connectors operation describe slack-chat-post-message` reach this branch through the local catalog backend. The fixture injects a per-address custody refusal at the actual SecretStore port while the bot slot remains accessible; it does not posit an invalid configuration or change provider facts. |

The source path is `crates/connectors-cli/src/lib.rs:1217` → the catalog backend composed at `crates/connectors-runtime/src/composition.rs:422` → `crates/integration-catalog/src/lib.rs:645`. The latter calls `available_connections` for every binding. `crates/integration-catalog/src/personal_connections.rs:326` checks the unrelated user's slot and propagates its error before the description filters its admitted Connections. That user binding is explicitly read-only and cannot be offered for the requested post. Connection describe checks the selected bot alone, which explains the contradictory externally returned state measured in the case. Operation search uses the same global helper at lib.rs:624; that analogous path was read but was not a second deciding execution.

The authority and credential isolation fix need not be weakened: availability checks can be scoped to bindings which actually admit the requested operation. A missing or denied selected credential must remain a refusal, and the retained first-pass selected-slot case must continue to pass. This report changes no implementation and requests no third adversarial pass.

Reachability limit: the custody failure is deterministic fault injection, not a reproduced incident in the operator's OS keyring. The local composition selects KeyringStore when available, whose individual lookups can return Unreachable at keyring.rs:142–164; FileStore failures usually affect its whole backing file. The same `SecretStore::exists` contract preserves non-NotFound errors instead of claiming absence (connector-secrets/src/lib.rs:215). No claim is made that a real user's current read-only slot is locked. The measured bug is that a failure outside the requested operation's admitted bindings invalidates that operation's healthy Connection.

Origin is undecided because no base worktree was assigned and the base was not executed. The whole-unit diff was read against 76f3fef9ce53a92d54d5e1c8147c5943315d423f, but source inspection is not represented as a base execution.

No separate judgement-only finding is returned.

## 5. Boundaries not broken

- Retained corrected Datadog conjunctions, absent companions, exact headers and selected subset alternatives pass; the resolver's pre-existing public API tests also pass.
- Removing a selected credential after a successful describe refuses without reading through a stored alternative or making an egress call.
- Cancellation before config publication leaves original bytes and the live credential intact, excludes a cooperative writer, and releases owned staging and lock files.
- Cancellation during the custody write leaves the explicitly documented partial outcome, with no rollback over the earlier stored value.
- Re-enrollment without --as preserves a legacy user binding, reference, grant and selected storage address.
- Existing malformed/invalid-config, staging failure, concurrent edit, store refusal and foreign-lock tests remain green.

## 6. External write inventory

The coordinator's brief.md was read, not written. Persistent pass outputs are listed individually. Temporary test children under the assigned TMPDIR are managed by their tempfile owners. Compiler cache and ordinary Cargo cache/lock activity are tool-managed; neither cache was redirected or cleaned. There are no other intentional external artifacts or live integration writes.

- ~/.cache/connectors-cli-wave-20260906/one-placement-several-credentials/adversary-2/cancel-before-publication.exit
- ~/.cache/connectors-cli-wave-20260906/one-placement-several-credentials/adversary-2/cancel-before-publication.log
- ~/.cache/connectors-cli-wave-20260906/one-placement-several-credentials/adversary-2/cancel-during-custody.exit
- ~/.cache/connectors-cli-wave-20260906/one-placement-several-credentials/adversary-2/cancel-during-custody.log
- ~/.cache/connectors-cli-wave-20260906/one-placement-several-credentials/adversary-2/console-clippy.exit
- ~/.cache/connectors-cli-wave-20260906/one-placement-several-credentials/adversary-2/console-clippy.log
- ~/.cache/connectors-cli-wave-20260906/one-placement-several-credentials/adversary-2/console-suite.exit
- ~/.cache/connectors-cli-wave-20260906/one-placement-several-credentials/adversary-2/console-suite.log
- ~/.cache/connectors-cli-wave-20260906/one-placement-several-credentials/adversary-2/legacy-default-selector.exit
- ~/.cache/connectors-cli-wave-20260906/one-placement-several-credentials/adversary-2/legacy-default-selector.log
- ~/.cache/connectors-cli-wave-20260906/one-placement-several-credentials/adversary-2/locked-unrelated.exit
- ~/.cache/connectors-cli-wave-20260906/one-placement-several-credentials/adversary-2/locked-unrelated.log
- ~/.cache/connectors-cli-wave-20260906/one-placement-several-credentials/adversary-2/raw-report.md
- ~/.cache/connectors-cli-wave-20260906/one-placement-several-credentials/adversary-2/removed-after-describe.exit
- ~/.cache/connectors-cli-wave-20260906/one-placement-several-credentials/adversary-2/removed-after-describe.log
- ~/.cache/connectors-cli-wave-20260906/one-placement-several-credentials/adversary-2/report.md
- ~/.cache/connectors-cli-wave-20260906/one-placement-several-credentials/adversary-2/resolver-suite.exit
- ~/.cache/connectors-cli-wave-20260906/one-placement-several-credentials/adversary-2/resolver-suite.log
- ~/.cache/connectors-cli-wave-20260906/one-placement-several-credentials/adversary-2/runtime-clippy.exit
- ~/.cache/connectors-cli-wave-20260906/one-placement-several-credentials/adversary-2/runtime-clippy.log
- ~/.cache/connectors-cli-wave-20260906/one-placement-several-credentials/adversary-2/runtime-suites.exit
- ~/.cache/connectors-cli-wave-20260906/one-placement-several-credentials/adversary-2/runtime-suites.log
- ~/.cache/connectors-cli-wave-20260906/one-placement-several-credentials/adversary-2/test-format.exit
- ~/.cache/connectors-cli-wave-20260906/one-placement-several-credentials/adversary-2/test-format.log
- ~/.cache/cw6/m (assigned fixture temporary root; transient test children)
- ~/.cache/sccache (tool-managed compiler cache)
- ~/.cargo (tool-managed Cargo cache and package locks)

The public report is produced by mechanically replacing the local absolute home-directory prefix with a tilde. All other bytes, including counts, outcomes, deciding output, finding severity and origin, remain unchanged.

```findings
- file: crates/integration-catalog/src/personal_connections_tests.rs
  line: 964
  category: contract-drift
  severity: blocker
  verdict: CONFIRMED
  origin: undecided
  message: A denied credential slot on a read-only user binding aborts description of the separately callable bot's admitted Slack post.
```
