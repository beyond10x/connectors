---
format: aep.planning-md/3
id: review-result:wave-20261002b-bridge-adversary-1
kind: review-result
status: active
title: Metadata timeout cleanup adversary, pass 1
relations:
- reviews: story:bridge-drop-waits-for-dispatched-batch
revision: 1
---
unit: story:bridge-drop-waits-for-dispatched-batch — cb26b-bridge working tree over b3ddded7618b5a4a23fe74dbec739e0938ecc296
verdict: nothing found
cases: executed 333→335, red 0
origin: introduced 0 / pre-existing 0 / undecided 0
wrote-outside-worktree: none
needs-coordinator: none

## 1. Diff boundary

Full working-tree `git --no-pager diff --stat` (includes the implementor's preexisting dirty production diff):

```text
 crates/connectors-host/src/local/metadata.rs       |  46 +++-
 crates/connectors-host/src/local/metadata/er.rs    | 126 ++++++++-
 .../src/local/metadata/metamorphic_tests.rs        | 300 +++++++++++++++++++++
 3 files changed, 450 insertions(+), 22 deletions(-)
```

Adversary-only delta against the saved input is 73 added lines in `crates/connectors-host/src/local/metadata/metamorphic_tests.rs`, two tests. No production code or existing case changed. The two production paths above were dirty on receipt and remain byte-identical to the implementor's input. Input diff, exact test-only patch, and final source digests are retained as `input.patch`, `adversary-only.patch`, and `source-sha256.txt` in this report's directory. No charter violation.

## 2. Cases authored before execution

- `metadata/metamorphic_tests.rs:1370`, `adversary_timeout_unwind_keeps_ownership_until_dispatched_work_finishes`: real SQLite writer contention makes a dispatched call return OutcomeUnknown; caller unwinding must return promptly, keep lifecycle ownership, and allow the original row to appear once contention ends. Green on first execution. The test was subsequently strengthened to inspect the exact injected panic payload, preventing a swallowed assertion failure from satisfying catch_unwind; its final exact run is reported below.
- `metadata/metamorphic_tests.rs:1403`, `adversary_timeout_before_dispatch_releases_ownership_without_a_commit`: an expired deadline rejects before dispatch; retirement must release its lock promptly and reopen must observe exactly zero attempted rows. Green on first execution.

There was no red output. Both tests were written before any test command. Before count 333 is from the implementor's report, not a preliminary suite execution. Suite counts exclude child-process helper summaries with nonzero filtered counts.

All commands used `CARGO_BUILD_JOBS=2`, `RUSTC_WRAPPER=/usr/bin/sccache`, `CONNECTORS_ESS=$HOME/.cache/ess/toolchains/0.45.0/ess`, and `CONNECTORS_AEP=$HOME/.cache/aep/toolchains/0.65.0/aep-0.65.0-x86_64-unknown-linux-gnu/aep`. Cargo used the following test runner; TMPDIR was not changed for Cargo/sccache itself:

```text
--config 'target.x86_64-unknown-linux-gnu.runner=["sh","-c","export TMPDIR=\"$0\"; exec \"$@\"","$HOME/.local/state/worktree/trees/b10x/connectors/cb26b-bridge/.local/tmp-adversary"]'
```

First command: `cargo <runner-config> test -p connectors-host --lib local::metadata::metamorphic_tests::adversary_timeout_unwind_keeps_ownership_until_dispatched_work_finishes -- --exact`, exit 0:

```text
   Compiling connectors-host v0.24.0 ($HOME/.local/state/worktree/trees/b10x/connectors/cb26b-bridge/crates/connectors-host)
    Finished `test` profile [optimized] target(s) in 36.73s
     Running unittests src/lib.rs (target/debug/deps/connectors_host-ed89bb8c0002e8a9)

running 1 test
test local::metadata::metamorphic_tests::adversary_timeout_unwind_keeps_ownership_until_dispatched_work_finishes ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 319 filtered out; finished in 0.72s

```

Second command: `cargo <runner-config> test -p connectors-host --lib local::metadata::metamorphic_tests::adversary_timeout_before_dispatch_releases_ownership_without_a_commit -- --exact`, exit 0:

```text
    Blocking waiting for file lock on package cache
    Blocking waiting for file lock on package cache
    Blocking waiting for file lock on package cache
    Finished `test` profile [optimized] target(s) in 0.81s
     Running unittests src/lib.rs (target/debug/deps/connectors_host-ed89bb8c0002e8a9)

running 1 test
test local::metadata::metamorphic_tests::adversary_timeout_before_dispatch_releases_ownership_without_a_commit ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 319 filtered out; finished in 0.49s

```

## 3. Suite after authored cases

`cargo <runner-config> test -p connectors-host`, exit 0. 294 library cases plus 41 integration/doc cases = 335 executed, 26 ignored unchanged. The suite compiled before the unwind panic-payload assertion was strengthened; the final exact run after this output verifies that stronger assertion. No production input changed between these runs.

```text
    Finished `test` profile [optimized] target(s) in 0.36s
     Running unittests src/lib.rs (target/debug/deps/connectors_host-ed89bb8c0002e8a9)

running 320 tests
test local::approval_keys::tests::durable_key_management_with_qualified_custody ... ignored, requires qualified disposable GNOME Secret Service; run explicitly
test local::approval_keys::tests::key_crash_child ... ignored, auxiliary child entry; parent invokes exact phases
test local::approval_keys::tests::key_use_excludes_rotation_and_concurrent_init_has_one_winner ... ignored, requires qualified disposable GNOME Secret Service; run explicitly
test local::approval_keys::tests::process_exits_keep_pre_and_post_publication_distinct ... ignored, requires qualified disposable GNOME Secret Service; run explicitly
test local::approval_keys::tests::production_cli_key_journey ... ignored, requires built production CLI and qualified disposable GNOME Secret Service
test local::approval_keys::tests::purpose_isolation_for_identical_private_uuid_tuples ... ignored, requires qualified disposable GNOME Secret Service; run explicitly

running 1 test
test local::approval_keys::tests::restart_failed_rotation_recovery_and_revocation_fences ... ignored, requires qualified disposable GNOME Secret Service; run explicitly
test local::approval_keys::tests::uncertain_deletion_keeps_retirement_fence_and_exact_retry ... ignored, requires qualified disposable GNOME Secret Service; run explicitly
test local::approval_keys::tests::unknown_storage_acknowledgement_requires_exact_reconfirmation ... ignored, requires qualified disposable GNOME Secret Service; run explicitly
test local::approval_keys::tests::wrong_recovery_material_never_publishes ... ignored, requires qualified disposable GNOME Secret Service; run explicitly
test local::adversary_w3_tests::publish_binding_allowed_on_a_revoked_connection_refuses_with_connection_state_conflict ... ok
test local::adversary_w3_tests::expiring_an_expired_cursor_refuses_with_cursor_state_conflict ... ok
test http::tests::a_form_post_sends_the_encoded_form_and_never_the_credential_header ... ok
test local::approval_policy::tests::policy_child ... ignored, child fixture; launched by process serialization tests
test http::tests::a_probe_endpoint_is_one_bounded_fixed_path ... ok
test http::tests::an_oversized_probe_document_is_refused_without_any_request ... ok
test http::tests::an_oversized_or_unaddressed_form_is_refused_without_any_request ... ok
test local::approvals::tests::all_subject_coordinates_and_unicode_identity_are_bound ... ok
test http::tests::a_sent_request_whose_deadline_passes_is_marked_as_the_providers_timeout ... ok
test local::approvals::tests::canonical_framing_rejects_omission_duplicates_unknown_fields_and_alternate_spellings ... ok
test local::approval_keys::tests::passive_status_does_not_migrate_or_touch_custody ... ok

running 1 test

running 1 test
test local::approval_policy::tests::passive_status_and_missing_issuer_never_install_policy ... ok
test local::approvals::tests::crash_child ... ignored, subprocess entry point exercised by abrupt_process_exits
test local::adversary_w2_tests::preparation_whose_deadline_passes_in_preflight_times_out_and_terminates_the_child ... ok

running 1 test

running 1 test

running 1 test
test local::approval_policy::tests::status_and_snapshot_exclude_private_authority_and_custody_coordinates ... ok
test local::approvals::tests::issuer_audience_time_key_and_clock_refusals_are_current ... ok

running 1 test
test local::approval_policy::tests::inherited_policy_use_is_not_valid_in_a_forked_process ... ok
test local::approval_policy::tests::concurrent_initial_publication_has_one_winner ... ok
test local::approval_policy::tests::invalid_publication_cannot_replace_a_valid_policy ... ok
test local::approvals::tests::protected_document_decoding_is_closed_bounded_and_verifiable ... ok
test local::approvals::tests::rfc8032_vector_and_exact_fixed_algorithm_round_trip ... ok
test local::approval_keys::tests::unavailable_write_leaves_exact_candidate_and_no_duplicate_allocation ... ok
test local::approval_policy::tests::exhausted_revision_never_wraps_or_reuses_identity ... ok

running 1 test
test local::approval_policy::tests::durable_guards_retain_identity_revision_and_instance_binding ... ok
test local::approval_policy::tests::exact_selections_and_operations_are_required_for_use ... ok
test local::approval_policy::tests::child_exit_releases_lease_and_committed_unacknowledged_revision_survives ... ok
test local::approvals::tests::concurrent_substituted_reference_cannot_rebind_one_attempt ... ok
test local::approval_policy::tests::failed_sql_write_preserves_policy_but_lost_ack_requires_observation ... ok
test local::approvals::tests::expiry_during_acknowledgement_retains_spend_without_granting_receipt ... ok
test local::approval_policy::tests::restart_preserves_identity_revision_principal_and_snapshot ... ok

running 1 test
test local::approvals::tests::policy_guard_survives_commit_and_clock_is_rechecked_after_storage ... ok
test local::approvals::tests::unavailable_live_binding_is_storage_failure_without_receipt_or_retry ... ok
test local::audit::tests::crash_child ... ignored, subprocess entry point exercised by abrupt_process_exits
test local::approvals::tests::tombstone_capacity_and_migrations_survive_restart_without_secret_storage ... ok
test local::audit::tests::exact_private_identity_is_injective_bounded_and_strictly_decoded ... ok
test local::approvals::tests::unavailable_redemption_evidence_cannot_open_dispatch ... ok

running 1 test
test local::approvals::tests::approved_gate_requires_original_spend_and_abort_never_refunds ... ok
test local::approvals::tests::policy_schema_supports_audit_spend_dispatch_and_restart_observation ... ok
test local::audit::tests::anchor_failure_or_lost_ack_never_returns_an_admission_receipt ... ok
test local::approvals::tests::unkeyed_preparation_retains_authority_and_old_records_cannot_gain_it ... ok
test local::audit::tests::byte_bounds_safe_shapes_and_aggregate_budget_are_enforced ... ok

running 1 test
test local::audit::tests::corrupted_or_unavailable_metadata_is_not_absence_or_recovered_acknowledgement ... ok

running 1 test
test local::audit::tests::concurrent_finalization_has_one_immutable_winner_and_exact_retry ... ok
test local::audit::tests::failed_final_append_preserves_live_answer_and_unknown_ack_recovers_exactly ... ok
test local::approvals::tests::rollback_or_lost_ack_never_reconstructs_receipt ... ok
test local::clock::protocol::tests::authentic_unknown_fields_versions_and_both_merkle_directions_work ... ok
test local::clock::protocol::tests::duplicate_tags_bad_offsets_nested_lengths_and_signature_substitutions_refuse ... ok
test local::clock::protocol::tests::every_truncation_and_single_byte_change_in_known_reply_refuses ... ok
test local::clock::protocol::tests::independent_signed_fixture_verifies_without_a_wall_clock ... ok
test local::clock::protocol::tests::signed_semantic_errors_do_not_supply_time ... ok
test local::clock::tests::authenticated_acquisition_and_wrong_key_refusal ... ok
test local::clock::tests::configuration_never_discovers_or_defaults_trust ... ok
test local::clock::tests::elapsed_bounds_cover_both_rate_extremes_with_outward_rounding ... ok
test local::clock::tests::independent_live_source ... ignored, Explicit public Roughtime network interoperability; no physical clock guarantee
test local::clock::tests::inherited_clock_refuses_in_an_actual_fork ... ok
test local::clock::tests::interval_includes_full_network_delay_and_ages_without_sliding ... ok
test local::clock::tests::no_estimate_extrapolates_through_a_possible_utc_leap ... ok
test local::clock::tests::oversized_and_unavailable_peers_refuse ... ok
test local::clock::tests::process_suspend_backward_time_and_unsafe_bounds_refuse ... ok
test local::approvals::tests::mismatched_receipt_missing_subject_and_existing_key_never_open_new_gate ... ok
test local::keyring::custody::gnome::tests::daemon_environment_mapping_refuses_ambiguous_or_injected_layouts ... ok
test local::keyring::custody::tests::custody_locks_wait_as_long_as_the_test_sets ... ok
test local::keyring::custody::tests::custody_neither_reads_nor_changes_the_default_alias ... ignored, requires qualified GNOME Keyring 50.0, dbus-daemon and task-owned TMPDIR
test local::keyring::custody::tests::disposable_changed_keyring_format_is_refused_before_transfer ... ignored, requires qualified GNOME Keyring 50.0, dbus-daemon and task-owned TMPDIR
test local::keyring::custody::tests::disposable_registry_publication_cli_and_retirement_restart ... ignored, requires qualified GNOME, dbus-daemon, task-owned TMPDIR and CONNECTORS_TEST_CLI
test local::keyring::custody::tests::disposable_secret_service_restart_and_failures ... ignored, requires qualified GNOME Keyring 50.0, dbus-daemon and task-owned TMPDIR
test local::keyring::custody::tests::private_custody_identities_are_non_nil ... ok
test local::approvals::tests::abrupt_process_exits_recover_without_refunding_or_resending ... ok
test local::audit::tests::recovering_append_stops_after_one_failed_retry ... ok
test local::audit::tests::passive_old_schema_inspection_never_installs_audit_and_migration_preserves_history ... ok
test local::audit::tests::recovering_append_requires_a_readable_exact_original_anchor ... ok

running 1 test
test local::metadata::er::tests::approval_policy_exhausted_revision_is_refused ... ok
test local::metadata::er::tests::approval_policy_identity_is_not_nil ... ok
test local::metadata::er::tests::acquisition_completion_refuses_a_result_other_than_its_target ... ok
test local::audit::tests::capacity_is_atomic_per_instance_without_eviction_or_append_restriction ... ok
test local::audit::tests::recovery_budget_expiring_after_first_read_prevents_retry ... ok
test local::metadata::er::tests::approval_redemption_subject_carries_the_fixed_format ... ok
test local::metadata::er::tests::approval_policy_revision_must_advance ... ok
test local::metadata::er::tests::approval_redemption_subject_couples_origin_and_route ... ok
test local::metadata::er::tests::audit_anchor_fields_are_set_from_the_acknowledged_input ... ok
test local::clock::tests::timeout_and_excessive_signed_radius_supply_no_capability ... ok
test local::metadata::er::tests::approval_policy_revision_is_bounded ... ok
test local::metadata::er::tests::auth_binding_commands_store_the_facts_they_carry ... ok
test local::metadata::er::tests::approval_policy_lists_at_most_256_operations ... ok
test local::metadata::er::tests::attempt_settlement_exists_only_for_a_known_terminal_outcome ... ok
test local::audit::tests::expired_recovery_budget_performs_no_extra_storage_calls ... ok
test local::metadata::er::tests::audit_anchor_co_presence_is_enforced_at_acknowledgement ... ok
test local::metadata::er::tests::attempt_transitions_are_decided_recorded_replayed_and_rebuilt_by_er ... ok
test local::metadata::er::tests::clock_floor_advance_refuses_a_lower_floor_and_accepts_the_same_one ... ok
test local::metadata::er::tests::final_projection_row_cannot_invent_missing_lifecycle_history ... ok
test local::metadata::er::tests::binding_publication_refuses_a_stale_publication_fence ... ok
test local::metadata::er::tests::cursor_expiry_is_recorded_in_the_expired_state ... ok
test local::metadata::er::tests::connection_list_cursor_page_limit_is_bounded ... ok
test local::metadata::er::tests::inspected_admission_sets_its_capture_from_the_input ... ok
test local::metadata::er::tests::approval_policy_revise_at_the_maximum_answers_revision_exhausted ... ok
test local::metadata::er::tests::auth_binding_invariants_refuse_custody_without_generation_and_oversized_material ... ok
test local::approvals::tests::concurrent_reference_and_attempt_spend_produce_one_receipt ... ok
test local::metadata::er::tests::key_reservation_commands_store_their_inputs ... ok
test local::metadata::er::tests::final_append_stores_the_observation_and_emits_the_owner_id ... ok

running 1 test
test local::metadata::er::tests::dispatch_refuses_a_generation_other_than_the_admitted_one ... ok
test local::metadata::er::tests::clock_floor_is_never_negative ... ok
test local::audit::tests::same_public_ref_is_independent_per_instance_and_gateway_and_leaf ... ok
test local::metadata::er::tests::postcommit_business_projection_requires_its_own_clock_state ... ok
test local::metadata::er::tests::postcommit_prepared_observation_ignores_only_unread_runtime_records ... ok
test local::metadata::er::tests::postcommit_runtime_state_accepts_only_an_authored_registry_clock_advance ... ok
test local::metadata::er::tests::read_use_capture_sets_its_bindings_from_the_input ... ok
test local::metadata::er::tests::read_use_creation_requires_the_admitted_registry_operation ... ok
test local::metadata::er::tests::fingerprints_record_the_adapter_v1_canonicalization ... ok
test local::approvals::tests::fresh_spend_rechecks_policy_key_time_and_exact_initial_proof ... ok
test local::metadata::er::tests::key_publication_is_recorded_in_the_active_state ... ok
test local::metadata::er::tests::host_derived_commands_without_a_suite_caller_are_accepted ... ok
test local::metadata::er::tests::issuer_revision_advance_is_a_cas_to_a_fresh_revision ... ok
test local::audit::tests::early_refusal_and_malformed_or_mismatched_facts_cannot_satisfy_the_gate ... ok
test local::metadata::er::tests::key_reservation_refuses_an_empty_caller_key ... ok
test local::metadata::er::tests::key_reservation_couples_origin_and_route ... ok
test local::audit::tests::abrupt_process_exits_preserve_acknowledgement_boundaries_without_resend ... ok
test local::metadata::er::tests::registry_epoch_advance_refuses_a_lower_epoch_and_accepts_the_same_one ... ok
test local::metadata::er::tests::key_reservation_settlement_exists_only_for_a_settled_known_result ... ok
test local::metadata::er::tests::required_approval_dispatch_needs_the_captured_subject ... ok
test local::metadata::er::tests::issuer_and_audience_carry_their_prefixes ... ok
test local::metadata::metamorphic_tests::adversary_timeout_before_dispatch_releases_ownership_without_a_commit ... ok
test local::audit::tests::retained_cross_owner_references_are_checked_and_never_cascade_deleted ... ok
test local::metadata::er::tests::signing_key_validity_bounds_are_enforced ... ok
test local::metadata::metamorphic_tests::adversary_timeout_unwind_keeps_ownership_until_dispatched_work_finishes ... ok
test local::metadata::metamorphic_tests::metadata_timeout_safety_batch_timeout_retains_ownership ... ok
test local::audit::tests::recovering_append_reuses_exact_fields_and_reads_a_lost_acknowledgement ... ok
test local::metadata::er::tests::one_sequence_records_the_same_decisions_twice ... ok
test local::metadata::metamorphic_tests::metadata_timeout_safety_queued_batch_cancelled_before_release ... ok
test local::metadata::er::tests::handles_another_process_releases_never_evict_this_process_held_handle ... ok
test local::metadata::metamorphic_tests::metadata_timeout_safety_legacy_import_retains_ownership ... ok
test local::metadata::tests::a_sidecar_retired_during_admission_is_absent_not_unavailable ... ok
test local::audit::tests::recovering_append_never_substitutes_a_different_final_observation ... ok
test local::metadata::metamorphic_tests::mr11_same_millisecond_and_regressed_observations_leave_views_unchanged ... ok
test local::metadata::tests::a_business_batch_cannot_commit_behind_an_unrecorded_clock_advance ... ok
test local::metadata::metamorphic_tests::mr10_repeated_final_audit_observation_is_a_pure_replay ... ok
test local::metadata::tests::an_unmodeled_sql_deletion_cannot_erase_recorded_metadata ... ok
test local::metadata::metamorphic_tests::metadata_timeout_safety_next_invoke_after_unknown_outcome ... ok
test local::metadata::metamorphic_tests::mr6_repeated_revoke_is_a_pure_replay ... ok
test local::metadata::tests::metadata_file_verification_preserves_an_existing_sqlite_wal_lock ... ok
test local::metadata::tests::only_the_handing_parent_binds_a_lock_wait ... ok
test local::metadata::tests::altered_legacy_and_retained_recovery_schemas_are_refused ... ok
test local::metadata::tests::fresh_setup_selects_the_recorded_authority_and_reopens_after_a_real_mutation ... ok
test local::metadata::tests::initialize_resumes_a_committed_switch_before_installing_its_required_projection ... ok
test local::metadata::metamorphic_tests::mr4_a_settled_attempt_ignores_every_later_settlement ... ok
test local::metadata::metamorphic_tests::mr3_replayed_keyed_mutation_has_one_attempt_and_the_original_result ... ok
test local::metadata::metamorphic_tests::mr13_refused_mutation_commands_leave_every_view_unchanged ... ok
test local::metadata::tests::passive_v1_inspection_cannot_migrate_or_claim_an_empty_registry ... ok
test local::metadata::tests::prepared_observation_commits_across_an_interleaved_runtime_record_write ... ok
test local::metadata::tests::runtime_state_persist_refuses_a_changed_registry_clock ... ok
test local::metadata::tests::unchanged_clock_business_batch_succeeds_without_advancing_time ... ok
test local::metadata::tests::runtime_migration_is_admitted_and_retains_the_v2_authority ... ok
test local::metadata::tests::interrupted_import_and_committed_switch_resume_without_resetting_authority ... ok
test local::metadata::metamorphic_tests::mr7_refused_registry_actions_change_only_the_clock ... ok
test local::mutations::tests::crash_child ... ignored, subprocess entry point, exercised by abrupt_process_exit
test local::metadata::tests::revoked_legacy_connection_retains_both_read_use_histories_without_invented_admission ... ok
test local::metadata::metamorphic_tests::mr5_revoke_changes_only_the_revoked_connection ... ok
test local::mutations::tests::ledger_limits_default_and_configured_bounds ... ok
test local::metadata::tests::nonempty_level_eight_migrates_without_changing_identity_or_retained_facts ... ok
test local::mutations::tests::capacity_and_corrupt_or_unavailable_reads_do_not_become_absence ... ok
test local::metadata::metamorphic_tests::mr8_refused_key_and_policy_commands_leave_every_view_unchanged ... ok
test local::mutations::tests::pending_scan_does_not_install_mutation_state ... ok
test local::mutations::tests::instance_recovery_preserves_fault_uncertainty_and_ignores_revoked_business_grants ... ok
test local::mutations::tests::concurrent_duplicate_prepare_has_one_live_receipt_and_original_correlation ... ok
test local::mutations::tests::every_fingerprint_coordinate_conflicts_including_exact_route_revisions ... ok
test local::mutations::tests::atomic_prepare_faults_never_return_a_handle_or_leave_a_half_reservation ... ok
test local::oauth::tests::a_head_over_the_limit_is_not_a_request ... ok
test local::oauth::tests::a_request_off_the_redirect_path_does_not_decide ... ok
test local::oauth::tests::cancellation_fixture ... ok
test local::oauth::tests::cancellation_stores_nothing ... ok
test local::oauth::tests::client_file_uri_mismatch_refused ... ok
test local::oauth::tests::consent_submits_the_acquired_triple ... ok
test local::oauth::tests::documents_other_than_a_google_client_pass_through ... ok
test local::oauth::tests::error_redirect_refused ... ok
test local::oauth::tests::pkce_challenge_matches_rfc7636_appendix_b ... ok
test local::oauth::tests::profile_field_mismatch_refused ... ok
test local::oauth::tests::second_request_refused ... ok
test local::oauth::tests::state_mismatch_refused ... ok
test local::oauth::tests::the_consent_address_without_a_terminal_is_one_marked_line ... ok
test local::oauth::tests::timeout_stores_nothing ... ok
test local::mutations::tests::retention_uses_trusted_bounds_and_cas_cannot_retire_a_replacement ... ok
test local::mutations::tests::terminal_first_fact_is_immutable_and_unknown_never_expires ... ok
test local::metadata::tests::every_recognized_legacy_level_uses_the_same_explicit_cutover ... ok
test local::oauth_adversary_pass2_tests::a_redirect_carrying_the_browsers_loopback_cookies_is_accepted ... ok
test local::oauth_adversary_pass2_tests::chrome_with_loopback_cookies_following_the_redirect_is_accepted ... ignored, requires google-chrome-stable
test local::mutations::tests::namespace_is_injective_with_explicit_nulls_origin_and_opaque_keys ... ok
test local::oauth_adversary_pass2_tests::pass2_flow_fixture ... ok
test local::oauth_adversary_pass2_tests::a_connection_flood_during_the_exchange_does_not_hold_the_flow_open ... ok
test local::oauth_adversary_pass2_tests::standard_error_carries_only_the_consent_line ... ok
test local::mutations::tests::unkeyed_attempts_still_require_the_gate_and_retain_outcome ... ok
test local::oauth_adversary_pass2_tests::the_listener_is_closed_once_the_flow_returns ... ok
test local::oauth_adversary_tests::adversary_flow_fixture ... ok
test local::oauth_adversary_pass2_tests::sixty_four_idle_connections_delay_the_redirect_until_a_slot_frees ... ok
test local::oauth_adversary_tests::chrome_following_the_redirect_is_accepted ... ignored, requires google-chrome-stable
test local::oauth_adversary_tests::chrome_opens_one_connection_per_navigation ... ignored, requires google-chrome-stable
test local::mutations::tests::version_three_inspection_is_read_only_and_admitted_upgrade_preserves_authority ... ok
test local::oauth_adversary_tests::a_queued_connection_without_a_request_is_not_a_second_request ... ok
test local::oauth_adversary_tests::an_idle_connection_first_does_not_hide_the_redirect ... ok
test local::metadata::metamorphic_tests::mr9_independent_namespaces_and_runtime_records_commute ... ok
test local::oauth_adversary_pass2_tests::a_connection_flood_does_not_hold_the_flow_past_its_deadline ... ok
test local::owner::approval_issuance::tests::production_cli_approval_issuance_and_restart ... ignored, requires built production CLI and qualified disposable GNOME Secret Service
test local::mutations::tests::ambiguous_gate_is_unknown_and_ambiguous_terminal_preserves_known_live_answer ... ok
test local::owner::mutation::tests::a_write_failure_names_its_stage_and_never_suggests_retrying_a_possible_effect ... ok
test local::owner::approval_issuance::tests::publication_refusal_and_unknown_acknowledgement_preserve_the_protected_file ... ok
test local::approval_policy::tests::policy_child ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 319 filtered out; finished in 30.07s

.
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 319 filtered out; finished in 31.87s

test local::adversary_w2_tests::blocked_policy_child_keeps_production_wait_when_the_variable_is_inherited ... ok
test local::mutations::tests::abrupt_process_exit_preserves_each_durable_boundary_without_resend ... ok
test local::metadata::metamorphic_tests::mr1_restart_and_every_open_path_yield_identical_views ... ok
test local::metadata::metamorphic_tests::mr12_guard_refused_projection_changes_leave_recorded_views_unchanged ... ok
test local::approval_policy::tests::policy_child ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 319 filtered out; finished in 30.06s

test local::oauth_adversary_pass2_tests::a_head_completing_at_the_idle_limit ... ok
test local::owner::supervisor::maintenance_tests::a_live_worker_with_an_idle_hint_queues_one_recovery_instead_of_taking_ownership ... ok
test local::owner::mutation::tests::production_finish_audit_recovers_a_lost_acknowledgement_on_the_real_clock ... ok
test local::owner::supervisor::suppression_tests::only_explicit_resume_actions_clear_stop_suppression ... ok
test local::approval_policy::tests::held_use_blocks_cross_process_revocation_until_release ... ok
test local::approval_policy::tests::policy_child ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 319 filtered out; finished in 31.29s

test local::owner::supervisor::revalidate_wait_tests::a_revalidation_the_pool_stopped_waiting_for_is_outcome_unknown ... ok
test local::owner::supervisor::idle_tests::a_live_idle_child_leaves_the_pool_quiet_and_a_request_in_flight_does_not ... ok
test local::approval_policy::tests::child_process_waits_on_the_lock_wait_its_parent_hands_it ... ok
test local::owner::mutation::tests::replay::a_host_refusal_is_stored_without_an_origin_and_keeps_retry_status ... ok
test local::protected::tests::adversary_empty_read_then_writer_closes_reports_end_of_input ... ok
test local::protected::tests::adversary_empty_read_without_interrupt_waits_for_the_next_input ... ok
test local::protected::tests::bounded_document_uses_original_deadline_and_rejects_links_and_excess_input ... ok
test local::protected::tests::adversary_empty_read_with_nothing_further_blocks_in_poll_until_the_deadline ... ok
test local::protected::tests::flush_fixture ... ok
test local::protected::tests::interrupt_that_flushes_polled_input_before_the_read_is_reported_as_interrupted ... ok
test local::protected::tests::source_admission_rejects_links_modes_and_oversized_files ... ok
test local::protected::tests::terminal_fixture ... ok
test local::mutations::tests::malformed_or_missing_clock_never_settles_a_known_result_or_shortens_retention ... ok
test local::registry::store_cost_tests::batch_cost_by_subject ... ignored, timing measurement; run explicitly
test local::registry::store_cost_tests::read_invoke_cost_by_store_size ... ignored, timing measurement; run explicitly
test local::metadata::metamorphic_tests::mr2_fresh_and_migrated_installations_record_the_same_subjects ... ok
test local::owner::transport::tests::a_connection_closed_by_a_retiring_owner_reaches_the_next_owner ... ok
test local::owner::transport::tests::an_owner_from_an_earlier_build_is_refused_by_name_and_left_running ... ok
test local::oauth_adversary_pass2_tests::how_long_sixty_four_idle_connections_delay_the_redirect ... ok
test local::owner::approval_issuance::tests::preparation_bounds_the_target_envelope_and_keeps_original_deadline ... ok
test local::owner::mutation::tests::replay::a_stored_provider_refusal_replays_at_dispatch_with_its_next_action_after_a_restart ... ok
test local::owner::approval_issuance::tests::prepare_is_passive_and_digests_the_strict_input_and_current_policy ... ok
test local::mutations::tests::connection_fences_authority_and_process_identity_refuse_stale_handles ... ok
test local::mutations::tests::pending_scan_pages_existing_instances_and_keyed_or_unkeyed_attempts ... ok
test local::registry::tests::acknowledged_deletion_cannot_keep_the_byte_size ... ok
test local::owner::approval_issuance::tests::changed_clock_or_connection_refuses_without_issuing_or_contacting_services ... ok
test local::registry::tests::abandoned_candidates_require_acknowledged_retirement_and_full_retention ... ok
test local::owner::approval_issuance::tests::clock_exchange_precedes_leases_and_policy_is_rechecked_after_network ... ok
test local::registry::tests::acquisition_deadline_closes_consumption_and_publication_at_expiry ... ok
test local::registry::tests::a_second_instance_of_the_same_adapter_connects ... ok
test local::owner::mutation::tests::replay::a_ledger_entry_written_before_the_origin_existed_still_replays ... ok
test local::owner::approval_issuance::tests::policy_and_prepare_refuse_wrong_permissions_pins_input_and_revision ... ok
test local::registry::tests::allocation_custody_and_publication_are_distinct_and_restartable ... ok
test local::registry::tests::an_observation_raced_on_every_unlocked_replay_still_completes ... ok
test local::protected::tests::controlling_terminal_hides_input_and_restores_echo_after_sigint ... ok
test local::registry::tests::approval_target_is_passive_but_requires_exact_retained_admission ... ok
test local::registry::tests::read_invoke_metadata_time ... ignored, timing measurement; run explicitly
test local::registry::tests::expiry_sweep_retires_a_pending_acquisition_that_was_never_consumed ... ok
test local::registry::tests::concurrent_observations_replay_current_clock_without_losing_a_floor ... ok
test local::registry::tests::lost_publication_acknowledgement_is_resolved_without_repeating_capture ... ok
test local::registry::tests::observation_after_a_recorded_read_use_expiry_is_admitted ... ok
test local::registry::tests::production_clock_samples_after_locking_and_still_rejects_regression ... ok
test local::registry::tests::identity_scope_and_expiry_failures_preserve_the_usable_generation ... ok
test local::registry::tests::prepared_same_millisecond_observation_refuses_stale_registry_state ... ok
test local::registry::tests::competing_repairs_have_one_winner_and_old_dispatch_is_fenced ... ok
test local::registry::tests::refused_business_action_still_advances_the_restart_clock_floor ... ok
test local::registry::tests::retained_scopes_are_nonempty ... ok
test local::registry::tests::repeated_binding_after_reopen_keeps_the_original_profile_declaration ... ok
test local::registry::tests::retained_scopes_are_at_most_sixty_four ... ok
test local::registry::tests::publication_preserves_unobserved_scope_and_credential_expiry ... ok
test local::runtime::artifact::tests::admitted_snapshot_survives_source_changes_and_refuses_mutation ... ok
test local::runtime::artifact::tests::configured_capture_requires_both_path_admission_and_matching_content ... ok
test local::registry::tests::cursors_are_scoped_bounded_and_invalidated_by_metadata_change ... ok
test local::runtime::channel::tests::malformed_frames_are_refused_without_unbounded_reads ... ok
test local::runtime::channel::tests::separate_protected_section_and_bounded_document_depth ... ok
test local::runtime::channel::tests::stalled_partial_frame_uses_the_original_deadline ... ok
test local::runtime::failure_tests::a_provider_refusal_keeps_its_code_and_names_the_provider_as_origin ... ok
test local::runtime::failure_tests::a_provider_timeout_or_capacity_answer_is_the_providers_and_the_hosts_own_is_not ... ok
test local::runtime::failure_tests::provider_codes_survive_the_private_boundary_without_raw_error_data ... ok
test local::registry::tests::cursors_expire_at_the_original_five_minute_deadline ... ok
test local::runtime::process::tests::protocol_fixture ... ok
test local::runtime::channel::tests::cancellation_observes_a_stalled_partial_reply_without_resetting_the_deadline ... ok
test local::registry::tests::approval_target_refuses_known_invalid_material_without_custody_access ... ok
test local::owner::transport::idle_sweep_tests::an_idle_owner_exits_at_a_bound_spanning_several_empty_recovery_sweeps ... ok
test local::runtime::process::write_tests::a_provider_timeout_is_the_providers_and_the_host_deadline_stays_the_hosts ... ok
test local::runtime::process::write_tests::fixture ... ok
test local::runtime::process::tests::malformed_replies_terminate_and_reap_only_the_owned_child ... ok
test local::owner::transport::idle_sweep_tests::an_unsettleable_pending_attempt_does_not_keep_an_owner_alive ... ok
test local::runtime::process::write_tests::a_provider_refusal_of_a_write_names_the_provider_and_a_fitting_next_action ... ok
test local::runtime::process::write_tests::duplicate_commit_refuses_and_legacy_selection_never_exposes_a_write ... ok
test local::runtime::process::write_tests::prepared_request_does_not_send_until_commit_and_cancel_destroys_before_ack ... ok
test local::runtime::process::write_tests::mismatched_readiness_refuses_before_any_credential_frame ... ok
test local::runtime::process::write_tests::schema_preflight_and_read_path_refusals_have_no_write ... ok
test local::runtime::writes::tests::old_codecs_refuse_new_variants_and_result_documents_remain_closed_and_bounded ... ok
test local::registry::tests::revalidation_profile_discovers_identity_without_persisting_clock_or_granting_admission ... ok
test local::registry::tests::receipt_identity_and_known_missing_material_cannot_be_substituted ... ok
test server::tests::oversized_adapter_result_is_refused_after_schema_validation ... ok
test server::tests::service_deadline_cancels_the_adapter_and_releases_capacity ... ok
test server::tests::thirty_two_inflight_calls_refuse_the_next_until_one_is_dropped ... ok
test local::registry::tests::revalidation_capture_and_completion_are_one_use_and_keep_original_deadline ... ok
test local::registry::tests::revocation_clears_the_baseline_with_the_active_material ... ok
test local::runtime::process::write_tests::malformed_preparation_cancel_and_result_replies_reap_the_exact_child ... ok
test local::registry::tests::revalidation_after_expiry_preserves_material_and_recovers_unknown_acknowledgement ... ok
test local::runtime::state::tests::remembered_runtime_state_survives_an_interleaved_registry_clock_advance ... ok
test local::runtime::process::write_tests::replacement_material_wrong_ids_and_other_requests_close_pending_without_send ... ok
test local::security_replay_tests::an_owner_batch_over_a_subject_another_process_wrote_equals_a_fresh_replay ... ok
test local::registry::tests::the_same_identity_connects_again_after_revoke ... ok
test local::registry::tests::terminal_revoke_invalidates_repair_and_dispatch_without_custody ... ok
test local::security_replay_tests::a_held_authority_refuses_every_in_place_alteration_a_fresh_open_refuses ... ok
test local::runtime::process::write_tests::native_outcomes_and_lost_replies_never_repeat_a_send ... ok
test local::registry::tests::revalidation_orders_competing_recollection_repair_revoke_and_pending_reads ... ok
test local::registry::tests::revalidation_preserves_transient_failures_but_cannot_resurrect_known_invalidity ... ok
test local::runtime::process::write_tests::drop_eof_and_original_deadline_destroy_pending_without_a_write ... ok
test local::registry::tests::a_read_invoke_against_a_grown_store_replays_it_at_most_once ... ok
test local::mutations::tests::abort_and_gate_compete_and_recovery_never_reconstructs_a_send_receipt has been running for over 60 seconds
test local::owner::mutation::tests::final_audit_recovery_preserves_every_live_business_result has been running for over 60 seconds
test local::registry::tests::concurrent_revoke_repair_publication_and_read_dispatch_share_one_fence ... ok
test local::registry::store_cost_tests::a_reopen_after_its_own_write_refuses_every_blob_altered_since has been running for over 60 seconds
test local::registry::store_cost_tests::a_reopen_after_its_own_write_refuses_every_blob_altered_since ... ok
test local::mutations::tests::abort_and_gate_compete_and_recovery_never_reconstructs_a_send_receipt ... ok
test local::owner::mutation::tests::final_audit_recovery_preserves_every_live_business_result ... ok

test result: ok. 294 passed; 0 failed; 26 ignored; 0 measured; 0 filtered out; finished in 109.82s

     Running tests/configuration_refusal_adversary.rs (target/debug/deps/configuration_refusal_adversary-d8ead58f020fc84a)

running 1 test
test a_file_invalid_for_another_reason_names_no_mismatched_entry ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.50s

     Running tests/http.rs (target/debug/deps/http-ef18003d50a11ef9)

running 3 tests
test custom_ca_accepts_its_endpoint_and_refuses_other_roots ... ok
test upstream_redirects_cannot_exfiltrate_bound_credentials ... ok
test error_bodies_are_not_exposed_and_large_responses_are_bounded ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.07s

     Running tests/http_prefix.rs (target/debug/deps/http_prefix-970880c0e3c7351b)

running 6 tests
test invalid_limits_and_unsupported_ports_refuse_before_work ... ok
test prefix_header_bytes_are_bounded_and_errors_do_not_include_provider_secrets ... ok
test excess_bytes_return_without_waiting_for_a_stalled_remainder ... ok
test premature_fixed_or_chunked_closure_is_an_error_even_after_exact_prefix ... ok
test exact_prefix_still_obeys_original_deadline_and_caller_cancellation ... ok
test prefix_distinguishes_eof_from_omitted_bytes_for_fixed_and_chunked_bodies ... ok

test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.33s

     Running tests/http_write.rs (target/debug/deps/http_write-b3e1f58de9a0732b)

running 5 tests
test put_preserves_exact_segments_body_and_native_response_without_redirect ... ok
test post_sends_one_json_document_with_the_selected_method ... ok
test native_failures_are_returned_once_and_body_limits_remain_errors ... ok
test invalid_target_segments_refuse_before_credential_resolution ... ok
test lost_reply_after_complete_put_is_never_retried ... ok

test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.34s

     Running tests/local_foundation.rs (target/debug/deps/local_foundation-46ecee937a296db4)

running 10 tests
test sidecar_symlink_is_refused_without_touching_its_target ... ok
test symlinks_hardlinks_and_broad_permissions_are_refused ... ok
test private_protocol_requires_explicit_v2_configuration_without_rewriting_v1 ... ok
test a_format_and_private_protocol_mismatch_names_the_format_and_entry ... ok
test exclusive_setup_persists_private_metadata_and_refuses_overwrite ... ok
test unavailable_or_unrecognized_metadata_is_never_reinitialized ... ok
test owner_and_migration_tampering_refuse_inspection ... ok
test contending_metadata_open_waits_for_a_live_holder_instead_of_reporting_it_unavailable ... ok
test concurrent_initialization_has_one_authority_and_one_configuration ... ok
test contending_metadata_open_refuses_at_its_bound_and_recovers_after_release ... ok

test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 32.13s

     Running tests/metadata_reopen_adversary.rs (target/debug/deps/metadata_reopen_adversary-06109d2409af17bb)

running 4 tests
test a_write_after_the_store_is_replaced_at_its_path_lands_in_the_new_store ... ok
test a_reopen_refuses_every_altered_event_a_full_replay_refuses ... ok
test concurrent_writers_leave_a_reopen_equal_to_a_full_replay ... ok
test a_reopen_refuses_every_altered_blob_a_full_replay_refuses has been running for over 60 seconds
test a_reopen_refuses_every_altered_blob_a_full_replay_refuses ... ok

test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 72.50s

     Running tests/provider_timeout_adversary.rs (target/debug/deps/provider_timeout_adversary-93d6859421d28275)

running 2 tests
test a_tls_handshake_that_never_completes_sent_no_request_and_is_not_marked ... ok
test a_body_that_stalls_after_the_provider_answered_is_the_providers_timeout ... ok

test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 15.05s

     Running tests/service.rs (target/debug/deps/service-227ce54b0e7a41be)

running 7 tests
test credential_stores_are_substitutable_and_file_permissions_are_enforced ... ok
test configuration_yaml_and_json_remain_strict ... ok
test environment_credentials_apply_the_same_value_bounds_as_files ... ok
test federation_refreshes_atomically_without_replaying_and_rotates_credentials ... ok
test wire_admission_and_freshness_refuse_before_dispatch ... ok
test invalid_identifiers_cannot_inject_log_lines_or_terminal_controls ... ok
test federation_preserves_result_and_reports_downstream_failure ... ok

test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.13s

   Doc-tests connectors_host

running 3 tests
test crates/connectors-host/src/http.rs - http::ScopedHttp::into_write (line 162) - compile fail ... ok
test crates/connectors-host/src/http.rs - http::ScopedHttp::probe_capability (line 179) - compile fail ... ok
test crates/connectors-host/src/local/runtime/process.rs - local::runtime::process::PreparedInvocation (line 456) - compile fail ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.44s

```

Final strengthened-case command: `cargo <runner-config> test -p connectors-host --lib local::metadata::metamorphic_tests::adversary_timeout_unwind_keeps_ownership_until_dispatched_work_finishes -- --exact`, exit 0:

```text
   Compiling connectors-host v0.24.0 ($HOME/.local/state/worktree/trees/b10x/connectors/cb26b-bridge/crates/connectors-host)
    Finished `test` profile [optimized] target(s) in 1m 01s
     Running unittests src/lib.rs (target/debug/deps/connectors_host-ed89bb8c0002e8a9)

running 1 test
test local::metadata::metamorphic_tests::adversary_timeout_unwind_keeps_ownership_until_dispatched_work_finishes ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 319 filtered out; finished in 3.28s

```

`cargo fmt -p connectors-host --check` and `git diff --check` exited 0 with no output.

## 4. Findings

Nothing found.

## 5. Attack boundaries

- Caller unwinding after real dispatched uncertainty could not release ownership early or strand subsequent recovery.
- Rejection before dispatch could not invent a row or permanently retain an idle lifecycle lock.
- The implementation's four timeout cases ran in the package suite, including independent-process idle recovery, queued cancellation, keyed recovery without another dispatch, and legacy import ownership retention.
- Read actual Entity Runtime 0.25.1 `sync.rs` shutdown/Drop/startup/wait-cell paths. Joined follows the actual thread join; uncertain dispatch is distinct from queued cancellation; startup errors join before returning. Callers of changed provisioning and persist paths were traced, including registry observation relocking.
- Thread-spawn failure and panic inside the reaper were inspected only: ManuallyDrop retains the entire authority/lock tuple if it cannot reach Joined. Neither branch was fault-injected. This report makes no execution claim for them.
- A fabricated retry on an errored Metadata handle was excluded: inspected production callers return on these failures, so no independently reachable defect was established.

## 6. Paths outside the worktree

None. Worktree-managed lease/cache activity used the existing tools; no agent-authored files, logs, code or custom build directory were written outside this tree. Task-owned `.local/tmp-adversary` was removed after both test processes exited. All retained report material is under `.local/wave-20261002b/adversary-1/`. Coordinator retains ownership of commits, planning artifacts and tree cleanup.

```findings
[]
```
