---
format: aep.planning-md/1
id: verification-report:cli-oauth-correction-one-20260906
kind: verification-report
status: draft
title: First OAuth authoring correction checkpoint
relations:
- verifies: story:connect-session-oauth-custody-in-personal-posture
revision: 1
---
## Coordinator verification and source commit

This is the first correction after review-result:cli-oauth-adversary-1-20260906, with the original report and origin unchanged. The exact retained regression now passes. The same ten-package root command changed from 1,103 passes and one failure to 1,104 passes and zero failures, with one preexisting reader ignore. Strict Clippy and root formatting pass. Other workspaces were not rerun by this correction; their first-pass results remain separate observations, and the second whole-unit review plus integration gate are outstanding.

Root verified all 53 new evidence members, all 1,193 source hashes and all 354 executable hashes, plus the separately sealed path erratum. Raw/public equality changes only the home-directory prefix. Public report SHA256 b28f54e097df89a3df284bd13bcfc86f25d16ec9e2b842671885aba4ee98e2b0; raw ce4c4c5e5124a024a59c36bfdb75ba3344b93b4691597898e50bc48db1b7192b; evidence manifest 3524c14842c80b5a9ebfd0c3b81d5476f8404ee38c7d7568d240234f7e81bdce; exact source patch a4b13d36288cba89e410fc6c6a3c3e3983a28af96c41f746f92e93b437cb686a. Root proof is oauth-correction1-root-verification.json under ~/.cache/cw6/p.

Root inspected and committed only crates/connector-spec/src/auth.rs at f9bf1d6a2e00b53711ef99473b7556b9e3743f25, parent 22e4d11ee1aad753d379e59f407cba3fc88bf6e2, tree 388b5da836da2f1748d16b9c5aa6038c79cbff3a. Both direct identities are the organization bot. All 1,193 source hashes matched after commit and the tree is clean. The correction adds a field deserializer; default-on-omission and serialization omission stay intact. Every test, schema, generated/frozen artifact and the other 1,192 source files are unchanged. The returned report's earlier uncommitted observation stays preserved below. Nothing is published by this checkpoint.

The owned private target remains preserved at 9,013,673,984 allocated bytes, with 17,779 files. The corrector released compilation before sealing. The first review outcome is fixed on this actual committed correction, not an assertion of whole-story completion.

## Separate path erratum

The original erratum SHA256 is bab5e59e661a7de6e7ebfb8db6bf3bcba68ba5c2fbae4ca03482378129dab2ee. The following copy changes only its home-directory prefix to ~; the original and sealed report remain unchanged.

Part 2 of the sealed raw/portable reports accidentally combines the before/after paths as preimage/auth-after.rs. The retained before file is preimage/crates/connector-spec/src/auth.rs; the after file is auth-after.rs. No source, result, report or manifest bytes changed. This separate post-seal wording erratum is the only additional outside write, at ~/.cache/connectors-cli-wave-20260906/connect-session-oauth-custody-in-personal-posture/whole-unit-correction-1/report-path-erratum.md.

## Complete frozen correction report

unit: connect-session-oauth-custody-in-personal-posture — whole-unit correction 1
verdict: green
cases: executed 1104→1104, red 0 (retained before: 1103 passed / 1 failed; after: 1104 passed / 0 failed; one old ignore unchanged)
origin: n/a
wrote-outside-worktree: ~/.cache/connectors-cli-wave-20260906/connect-session-oauth-custody-in-personal-posture/whole-unit-correction-1/outside-paths.json; exact persistent paths and assigned target/TMPDIR/Cargo bookkeeping roots below
needs-coordinator: yes — inspect/commit source.patch, independent second review and twelve-workspace integration gate remain

1. Unit and acceptance

Reject explicitly supplied personal_flows=[] at the provider authoring boundary while preserving omission, serialization omission and valid nonempty admissions. This is the bounded correction to the recorded first review, not a new attack/pass. No tests were added or altered; the identical executed count is expected for this correction. Managed tree ~/.local/state/worktree/trees/b10x/connectors/wt-4f1de73d0685, exact clean starting/current HEAD 22e4d11ee1aad753d379e59f407cba3fc88bf6e2, parent operational implementation 6fae9df000986f39d009a2e8503bdff03db41762. Root's authoritative review/scope is f5e4ec663fd20980673658a1756f46bed6307885, OAuth revision74. The brief SHA256 is42c22b862a9707ed8ecd3fb8e3bfa4ebcb2430b4df50256c8f3da8b0f8cac41b.

Only crates/connector-spec/src/auth.rs changes. The existing Vec default runs only when the field is absent; a field deserializer refuses a supplied empty Vec with a closed error. Serialization still omits the default empty Vec. The original schema minItems=1, every existing/new review test and all generated/frozen artifacts stay byte-exact. The exact cited production owner was confirmed before editing; no additional owner or model is needed.

Class: a nonempty-if-present authoring array must preserve the omission/presence distinction until it is validated. The mechanism is the common serde field deserializer, so the refusal does not depend on a particular provider or hand-written TOML search. Enumeration in class-enumeration.json covers every OAuth2Spec array-valued authoring property: personal_flows alone has minItems=1; scopes and grants have no such requirement and keep their existing empty behavior. Existing tests cover omission and serialization omission, nonempty PKCE/device serialization, malformed nonempty declarations and explicit-empty refusal. No broader repository-wide schema classification is claimed.

The first reviewer recorded origin undecided. Root separately routes the mismatch as introduced based on the published0c694509-to-6fae9df source diff. That is coordinator source inference, not an old-loader execution or measured runtime credential exposure; this implementor does not rewrite the original report or its origin field.

2. Actual diff shape

```text
 crates/connector-spec/src/auth.rs | 22 +++++++++++++++++++++-
 1 file changed, 21 insertions(+), 1 deletion(-)
```

Full exact patch is source.patch; preimage/auth-after.rs and the complete1193-source inventory are retained. Source-preservation.json proves the other1192 hashes are unchanged, including all tests, manifests, locks, schemas, canonical documents and packs. All227 first-review evidence hashes remain valid.

3. Retained actual red

Before-provenance.json verifies the first review's before inventory composed with its final test inventory against all1193 current handoff hashes. The exact final formatted test and unchanged production still fail in retained root-full-relocated at test line166. The original selected run also fails; it was not repeated to manufacture another red or added to the new execution count. The first read-only audit initially compared the original before-review inventory without its authorized test overlay; that audit assertion and correction are retained in provenance-audit-first-observation.json. No source had changed then.

Original cwd: ~/.local/state/worktree/trees/b10x/connectors/wt-4f1de73d0685
Original command, verbatim argv rendered with shell quoting:
```text
env -u RUSTC_WRAPPER CARGO_TARGET_DIR=/dev/shm/b10x-cli-wave-wt-4f1de73d0685-nr00xjox/runtime-target TMPDIR=~/.cache/cw6/a CARGO_BUILD_JOBS=1 CARGO_INCREMENTAL=0 CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 cargo test --locked --offline -p connector-spec oauth_pass1_explicit_empty_personal_admission_obeys_the_authoring_contract -- --nocapture
```
Original exit101 and complete selected output, verbatim:
```text
   Compiling syn v3.0.3
   Compiling serde_derive v1.0.229
   Compiling serde v1.0.229
   Compiling typenum v1.20.1
   Compiling generic-array v0.14.7
   Compiling indexmap v2.14.0
   Compiling crypto-common v0.1.7
   Compiling block-buffer v0.10.4
   Compiling toml_datetime v0.6.11
   Compiling serde_spanned v0.6.9
   Compiling ref-cast-impl v1.0.26
   Compiling thiserror-impl v2.0.20
   Compiling serde_json v1.0.151
   Compiling toml_edit v0.22.27
   Compiling thiserror v2.0.20
   Compiling ref-cast v1.0.26
   Compiling digest v0.10.7
   Compiling memchr v2.8.3
   Compiling sha2 v0.10.9
   Compiling serde_norway v0.9.42
   Compiling fluent-uri v0.4.1
   Compiling connector-address v0.6.5 (~/.local/state/worktree/trees/b10x/connectors/wt-4f1de73d0685/crates/connector-address)
   Compiling toml v0.8.23
   Compiling connector-spec v0.6.5 (~/.local/state/worktree/trees/b10x/connectors/wt-4f1de73d0685/crates/connector-spec)
    Finished `test` profile [unoptimized] target(s) in 27.19s
     Running unittests src/lib.rs (/dev/shm/b10x-cli-wave-wt-4f1de73d0685-nr00xjox/runtime-target/debug/deps/connector_spec-e6a0778184c9adb7)

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 28 filtered out; finished in 0.00s

     Running tests/main.rs (/dev/shm/b10x-cli-wave-wt-4f1de73d0685-nr00xjox/runtime-target/debug/deps/main-791231b9d8bface7)

running 1 test

thread 'personal_oauth::oauth_pass1_explicit_empty_personal_admission_obeys_the_authoring_contract' (4098534) panicked at crates/connector-spec/tests/main/personal_oauth.rs:159:5:
the published minItems=1 contract rejects explicit empty admission; the actual provider loader must retain that presence distinction
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace
test personal_oauth::oauth_pass1_explicit_empty_personal_admission_obeys_the_authoring_contract ... FAILED

failures:

failures:
    personal_oauth::oauth_pass1_explicit_empty_personal_admission_obeys_the_authoring_contract

test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 516 filtered out; finished in 0.00s

error: test failed, to rerun pass `-p connector-spec --test main`
```
The same full root selection before correction printed1103 passed,1 failed,1 ignored, exit101. Both earlier full failures and the first review's fixture/compiler/placement/harness observations remain intact in the original immutable report; none is relabeled as a correction failure or new attack.

4. Actual after checks

Selected lane: executed1→1, exit0; retained before0pass/1fail, after1pass/0fail. Root full lane: executed1104→1104, exit0; retained before1103pass/1fail, after1104pass/0fail. The one preexisting reader measurement remains ignored. Repeated selected cases are not added to the root count. The36 existing connector-oauth cases are included in the assigned ten-package root command; the other prior runtime/SQLite/console/CLI totals are not added or claimed freshly executed.

Every command below ran with the same private target, RUSTC_WRAPPER unset, jobs1, incremental0 and dev/test debug0. TMPDIR was exactly ~/.cache/cw6/oc1. Continuous guards preserved disk above12GiB, tmpfs free at least8GiB, MemAvailable at least16GiB and the assigned aggregate private target cap12GiB. All commands finished, no guard interruption or new compiler/fixture/test failure occurred. Minima: disk31028453376 bytes, tmpfs24586235904, MemAvailable36683894784; peak target9013673984. The slot was explicitly released immediately after the last check and before sealing. No compiler remains.

oauth-correction1-spec-empty-after, cwd ~/.local/state/worktree/trees/b10x/connectors/wt-4f1de73d0685, exit0:
```text
env -u RUSTC_WRAPPER CARGO_TARGET_DIR=/dev/shm/b10x-cli-wave-wt-4f1de73d0685-nr00xjox/runtime-target TMPDIR=~/.cache/cw6/oc1 CARGO_BUILD_JOBS=1 CARGO_INCREMENTAL=0 CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 cargo test --locked --offline -p connector-spec oauth_pass1_explicit_empty_personal_admission_obeys_the_authoring_contract -- --nocapture
```
Complete raw output:
```text
   Compiling connector-spec v0.6.5 (~/.local/state/worktree/trees/b10x/connectors/wt-4f1de73d0685/crates/connector-spec)
    Finished `test` profile [unoptimized] target(s) in 9.94s
     Running unittests src/lib.rs (/dev/shm/b10x-cli-wave-wt-4f1de73d0685-nr00xjox/runtime-target/debug/deps/connector_spec-e6a0778184c9adb7)

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 28 filtered out; finished in 0.00s

     Running tests/main.rs (/dev/shm/b10x-cli-wave-wt-4f1de73d0685-nr00xjox/runtime-target/debug/deps/main-791231b9d8bface7)

running 1 test
test personal_oauth::oauth_pass1_explicit_empty_personal_admission_obeys_the_authoring_contract ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 516 filtered out; finished in 0.00s
```

oauth-correction1-root-full, cwd ~/.local/state/worktree/trees/b10x/connectors/wt-4f1de73d0685, exit0:
```text
env -u RUSTC_WRAPPER CARGO_TARGET_DIR=/dev/shm/b10x-cli-wave-wt-4f1de73d0685-nr00xjox/runtime-target TMPDIR=~/.cache/cw6/oc1 CARGO_BUILD_JOBS=1 CARGO_INCREMENTAL=0 CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 cargo test --locked --offline -p connector-spec -p connector-resolve -p catalog -p catalog-build -p catalog-reader -p connectors-client -p protocol -p server -p service -p connector-oauth --no-fail-fast
```
Complete raw output:
```text
   Compiling connector-spec v0.6.5 (~/.local/state/worktree/trees/b10x/connectors/wt-4f1de73d0685/crates/connector-spec)
   Compiling catalog-build v0.6.5 (~/.local/state/worktree/trees/b10x/connectors/wt-4f1de73d0685/crates/catalog-build)
    Finished `test` profile [unoptimized] target(s) in 16.80s
     Running unittests src/lib.rs (/dev/shm/b10x-cli-wave-wt-4f1de73d0685-nr00xjox/runtime-target/debug/deps/catalog-bf05e0919bf12a7e)

running 9 tests
test table::tests::sip_v1_survives_the_generated_table ... ok
test table::tests::a_minting_join_in_the_document_reaches_acquisition_minted ... ok
test table::tests::cdp_v1_survives_the_generated_table ... ok
test table::tests::the_document_tells_apart_the_pair_the_derivation_could_not ... ok
test tests::an_operation_is_found_by_its_symbol ... ok
test tests::an_unknown_key_is_none_rather_than_a_panic ... ok
test tests::providers_are_listed_in_a_stable_order ... ok
test tests::listing_by_provider_partitions_the_catalog ... ok
test tests::operation_ids_are_unique_across_the_catalog ... ok

test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.89s

     Running tests/main.rs (/dev/shm/b10x-cli-wave-wt-4f1de73d0685-nr00xjox/runtime-target/debug/deps/main-d5244403d8e52e4e)

running 15 tests
test pack_table::grafana_discovery_is_available_as_closed_catalog_data ... ok
test consumer_api::the_inbound_and_configuration_surfaces_are_reachable ... ok
test consumer_api::the_caller_contract_is_document_data_alone ... ok
test consumer_api::rate_stage2_typed_history_advice_preserves_all_application_categories ... ok
test pack_table::every_credential_carries_a_leaf_and_a_placement ... ok
test consumer_api::the_lookup_surface_and_every_operation_field_are_reachable ... ok
test pack_table::the_table_covers_the_whole_pack ... ok
test pack_table::every_channel_binding_resolves_its_events ... ok
test consumer_api::the_closed_vocabularies_match_exhaustively ... ok
test pack_table::every_closed_configuration_set_is_addressable ... ok
test pack_table::the_derived_operation_facts_agree_with_the_documents ... ok
test source_fidelity_catalog_describes_translated_input_and_output_from_the_document ... ok
test consumer_api::the_reader_reexport_serves_the_same_catalogue ... ok
test pack_table::every_operation_is_reachable_by_key_exactly_once ... ok
test pack_table::rate_adversary_pack_and_table_preserve_every_declared_rate ... ok

test result: ok. 15 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.11s

     Running unittests src/lib.rs (/dev/shm/b10x-cli-wave-wt-4f1de73d0685-nr00xjox/runtime-target/debug/deps/catalog_build-b548eee29482c29e)

running 64 tests
test artifact::tests::fixtures_live_in_the_build_directory_and_never_repeat_a_path ... ok
test artifact::tests::read_if_exists_distinguishes_absent_from_present ... ok
test artifact::tests::write_atomic_creates_missing_directories ... ok
test artifact::tests::write_atomic_replaces_and_leaves_no_temporary ... ok
test artifact::tests::a_fixture_does_not_survive_a_panicking_test ... ok
test contract::tests::an_operation_with_no_parameters_states_the_empty_object ... ok
test contract::tests::arrays_carry_their_item_type_and_the_fallback_reaches_inside ... ok
test contract::tests::scalars_map_across_and_constraints_are_dropped ... ok
test contract::tests::a_const_pinned_body_field_reserves_the_symbol_a_later_field_must_shift_past ... ok
test contract::tests::shapes_the_contract_cannot_express_fall_back_to_the_top_type ... ok
test contract::tests::the_description_extends_with_the_error_envelope ... ok
test diff::tests::a_new_file_is_all_additions ... ok
test diff::tests::a_removed_line_is_a_single_removal ... ok
test diff::tests::a_replaced_middle_keeps_its_context ... ok
test diff::tests::a_wholly_different_file_shows_both_sides ... ok
test diff::tests::an_appended_line_is_a_single_addition ... ok
test diff::tests::an_unchanged_file_produces_only_context ... ok
test document::tests::an_approved_origin_may_be_the_whole_base_url ... ok
test document::tests::a_declared_graph_is_refused_rather_than_dropped ... ok
test document::tests::a_declared_registration_value_is_a_build_error ... ok
test document::tests::a_braced_constant_header_is_refused ... ok
test inbound::tests::an_unset_webhook_never_launders_itself_as_connection_authenticated ... ok
test inbound::tests::the_tri_state_publishes_as_three_distinct_kinds ... ok
test pack::tests::a_duplicate_operation_id_is_refused_by_name ... ok
test document::tests::an_unclassifiable_brace_literal_is_a_build_error_never_a_degraded_document ... ok
test pack::tests::disagreeing_schema_versions_are_refused ... ok
test pack::tests::spans_slice_each_operation_record_exactly ... ok
test pack::tests::the_compiled_pack_round_trips_its_own_spans ... ok
test contract::tests::source_fidelity_preserves_nullable_enum_union_defaults_and_body_constraints ... ok
test inbound::tests::only_a_deliberately_unverifiable_surface_reads_as_unverified ... ok
test pack::tests::a_nested_operations_key_is_not_the_array ... ok
test net::tests::a_denied_checkpoint_fails_and_is_counted ... ok
test document::tests::source_fidelity_schema_three_preserves_the_frozen_schema_two_identity_and_bytes ... ok
test scaffold::tests::a_service_name_is_cut_at_the_pull_date ... ok
test scaffold::tests::a_select_argument_drops_fields_from_the_right ... ok
test scaffold::tests::a_select_argument_refuses_what_it_cannot_mean ... ok
test seam::tests::a_pin_naming_an_absent_document_is_refused_and_lists_the_cache ... ok
test seam::tests::a_spec_backed_provider_ingests_its_operations ... ok
test seam::tests::a_hand_authored_definition_loads_into_the_ir ... ok
test seam::tests::an_empty_definition_is_rejected ... ok
test seam::tests::a_spec_backed_provider_with_no_patch_publishes_no_operations ... ok
test seam::tests::an_unknown_service_is_an_error_that_names_what_exists ... ok
test seam::tests::selecting_a_service_drops_every_other_operation ... ok
test seam::tests::selecting_a_service_carries_no_other_services_config_graphs_or_verify ... ok
test scaffold::tests::a_prefix_matches_whole_segments ... ok
test seam::tests::the_loaders_own_diagnosis_survives ... ok
test seam::tests::the_pinned_document_is_the_one_ingested_not_the_last_in_the_cache ... ok
test document::tests::personal_acquisition_schema_four_keeps_frozen_schema_three_bytes_and_closed_registration_boundary ... ok
test seam::tests::selecting_a_service_keeps_the_connector_level_surfaces ... ok
test document::tests::a_custody_only_connector_renders_a_document_with_no_surface ... ok
test document::tests::an_ordinary_connector_does_not_carry_the_flag ... ok
test document::tests::a_form_body_is_spelled_structurally ... ok
test document::tests::rendering_is_deterministic_and_validates ... ok
test check::tests::a_revendored_spec_is_named_as_spec_drift ... ok
test check::tests::a_wrong_lock_artifact_hash_is_named_as_lock_row_drift ... ok
test check::tests::a_mutated_artifact_is_named_as_artifact_drift ... ok
test check::tests::a_clean_tree_reports_the_provider_and_artifact_counts_without_writing ... ok
test check::tests::a_comment_only_provider_edit_is_named_as_declaration_drift ... ok
test check::tests::unsafe_and_symlinked_lock_paths_are_refused_without_reading_the_target ... ok
test check::tests::provider_coverage_is_checked_in_both_directions ... ok
test check::tests::artifact_coverage_is_checked_in_both_directions ... ok
test check::tests::spec_coverage_is_checked_in_both_directions ... ok
test pipeline::tests::a_refusal_is_the_first_one_in_provider_order_at_every_width ... ok
test pipeline::tests::the_plan_is_identical_at_every_compile_width ... ok

test result: ok. 64 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.05s

     Running tests/main.rs (/dev/shm/b10x-cli-wave-wt-4f1de73d0685-nr00xjox/runtime-target/debug/deps/main-4f9b671ea94aed75)

running 85 tests
test architecture_fence::reusable_client_has_no_runtime_or_backend_ownership ... ok
test architecture_fence::product_cli_is_a_thin_frontend ... ok
test catalog_invariants::a_session_signal_reaches_a_backend_only_through_the_admission_seam ... ok
test architecture_fence::every_runtime_isolation_boundary_is_explicit_and_locked ... ok
test catalog_invariants::adversary_gitlab_pass1_translation_preserves_composed_constraint_truth_tables ... ok
test architecture_fence::service_owns_the_backend_port_and_server_only_adapts_transport ... ok
test architecture_fence::runtime_is_the_only_adapter_composition_root ... ok
test catalog_invariants::rate_repair_source_uri_grammar_matches_authoring_and_both_schemas ... ok
test architecture_fence::production_modules_obey_the_named_size_fence ... ok
test catalog_invariants::no_effect_backend_is_reachable_without_an_admission_proof ... ok
test catalog_invariants::repository_authored_anthropic_sources_reproduce_only_the_api_connector ... ok
test catalog_invariants::personal_acquisition_schema_four_preserves_actual_provider_uri_vectors_and_contract ... ok
test catalog_invariants::rate_final_actual_provider_loading_preserves_uri_and_vendor_contract ... ok
test catalog_invariants::gitlab_official_source_inventory_accounts_for_every_operation ... ok
test catalog_invariants::substrate_axis_projection_is_pinned_total_and_non_mechanical ... ok
test catalog_invariants::adversary_gitlab_pass1_coverage_statuses_match_actual_importer_results ... ok
test catalog_invariants::source_fidelity_gitlab_preserves_every_selected_vendor_schema ... ok
test catalog_invariants::the_committed_tree_is_a_fixed_point_of_a_build ... ok
test catalog_invariants::a_full_build_leaves_no_orphaned_artifact ... ok
test catalog_invariants::a_custody_only_provider_publishes_no_surface_and_every_other_provider_does ... ok
test catalog_invariants::gitlab_schedule_slice_is_generated_and_preserves_legacy_contracts ... ok
test catalog_invariants::no_input_or_artifact_carries_a_credential_shaped_value ... ok
test catalog_invariants::the_browser_surface_is_read_only_and_carries_no_interaction_member ... ok
test catalog_invariants::slack_surface_is_curated_and_credential_scopes_never_cross_purposes ... ok
test catalog_invariants::personal_acquisition_schema_four_retains_the_rate_declaration_contract ... ok
test catalog_invariants::gitlab_user_and_automation_connections_are_distinct_and_scope_gated ... ok
test catalog_invariants::the_mcp_transport_reaches_a_backend_only_through_the_decided_seams ... ok
test dependency_fence::outbound_mcp_foundation_is_exactly_pinned ... ok
test dependency_fence::every_workspace_member_is_classified ... ok
test dependency_fence::the_build_path_does_not_depend_on_the_secret_store ... ok
test dependency_fence::a_compiler_crate_cannot_reach_a_network_crate ... ok
test catalog_invariants::promoted_operation_traits_equal_the_pre_migration_inventory ... ok
test catalog_invariants::every_format_origin_field_lowers_to_the_origin_slot ... ok
test dependency_fence::the_connectors_binary_is_an_isolated_locked_composition_leaf ... ok
test dependency_fence::the_gate_and_the_release_workflow_state_the_gates_own_workspace_count ... ok
test dependency_fence::the_walk_finds_an_edge_that_is_not_direct ... ok
test engine_free::no_manifest_in_this_workspace_requires_an_engine_crate ... ok
test dependency_fence::the_voice_runtime_is_the_only_production_composition_leaf ... ok
test engine_free::the_walk_finds_an_engine_two_edges_away_and_stops_at_a_severed_one ... ok
test catalog_invariants::rate_adversary_canonical_source_urls_match_authoring_reader ... ok
test catalog_invariants::the_lockfile_agrees_with_every_input_and_every_artifact ... ok
test engine_free::the_lockfile_names_no_engine_crate_at_all ... ok
test catalog_invariants::every_operations_required_parameters_are_the_ones_a_caller_must_send ... ok
test catalog_invariants::every_canonical_document_validates_against_the_committed_schema ... ok
test catalog_invariants::rate_stage2_conditional_history_advice_is_metadata_without_schema_edits ... ok
test ess_citation_fence::every_citation_of_the_specification_resolves ... ok
test dependency_fence::the_rtvbp_runtime_dependency_is_isolated_from_the_canonical_workspace ... ok
test dependency_fence::the_sipx_network_dependency_is_exactly_pinned_and_isolated ... ok
test ess_citation_fence::every_connect_session_state_write_is_cited ... ok
test ess_citation_fence::every_declared_error_and_event_is_cited_or_unmapped ... ok
test ess_citation_fence::every_published_event_is_an_event_some_domain_declares ... ok
test catalog_invariants::ids_are_unique_in_every_namespace_they_share ... ok
test ess_citation_fence::field_citations_span_the_struct_they_name ... ok
test dependency_fence::released_http_integrations_cannot_bypass_connection_bound_egress ... ok
test ess_citation_fence::the_channel_summary_that_carries_a_connection_ref_is_cited_and_modelled ... ok
test ess_claim_fence::deleting_session_terminates_declared_refusal_outcomes_is_refused ... ok
test ess_citation_fence::the_reobserve_site_leaves_a_connection_ref_and_the_specification_says_so ... ok
test catalog_invariants::the_contract_and_the_params_state_the_same_symbols ... ok
test ess_claim_fence::the_hosted_registry_never_reaches_the_state_its_marker_says_it_cannot ... ok
test ess_claim_fence::deleting_materializes_declared_refusal_outcomes_is_refused ... ok
test ess_claim_fence::session_terminate_declares_or_marks_every_refusal_its_cited_function_performs ... ok
test ess_claim_fence::every_declared_wire_name_is_a_method_the_protocol_accepts ... ok
test ess_claim_fence::deleting_the_sentence_that_counts_the_refusal_sites_is_refused ... ok
test ess_claim_fence::materialize_declares_or_marks_every_refusal_its_cited_function_performs ... ok
test ess_claim_fence::the_hosted_failed_claim_is_refused_from_either_side ... ok
test json_governance::a_json_schema_invalid_against_its_declared_meta_schema_fails ... ok
test json_governance::an_unclassified_json_file_fails_by_name ... ok
test json_governance::an_invalid_owned_document_fails_its_schema ... ok
test json_governance::malformed_vendored_json_fails_even_though_it_is_syntax_only ... ok
test msrv_fence::the_walk_finds_a_breach_that_is_not_direct ... ok
test msrv_fence::versions_compare_numerically_and_tolerate_both_spellings ... ok
test msrv_fence::the_running_toolchain_meets_the_declared_msrv ... ok
test no_network::build_records_no_network_attempt ... ok
test no_network::check_records_no_network_attempt ... ok
test no_network::the_network_seam_is_the_only_door ... ok
test catalog_invariants::the_credential_requirement_agrees_with_the_auth_list ... ok
test catalog_invariants::oauth_pass1_all_current_documents_retain_frozen_operation_meaning ... ok
test catalog_invariants::the_document_carries_the_callers_contract ... ok
test catalog_invariants::sip_catalog_surface_is_the_bounded_platform_dial_member ... ok
test engine_free::no_workspace_member_reaches_an_engine_crate ... ok
test msrv_fence::no_resolved_dependency_declares_a_rust_version_above_the_crate_that_reaches_it ... ok
test catalog_invariants::the_pack_serves_the_committed_documents_byte_for_byte ... ok
test json_governance::every_repository_json_is_classified_and_valid ... ok
test catalog_invariants::spec_backed_coverage_holds_in_both_directions ... ok
test catalog_invariants::two_plans_over_the_same_inputs_are_byte_identical ... ok

test result: ok. 85 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 34.39s

     Running unittests src/lib.rs (/dev/shm/b10x-cli-wave-wt-4f1de73d0685-nr00xjox/runtime-target/debug/deps/catalog_reader-44666dbfed70aba2)

running 2 tests
test sha256::tests::the_published_vectors_agree ... ok
test sha256::tests::the_million_a_vector_agrees ... ok

test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.04s

     Running tests/main.rs (/dev/shm/b10x-cli-wave-wt-4f1de73d0685-nr00xjox/runtime-target/debug/deps/main-787c16ebe6417ef9)

running 17 tests
test pack::a_newer_schema_version_is_refused_by_name ... ok
test pack::a_payload_length_disagreement_is_refused ... ok
test pack::a_span_outside_the_payload_is_refused ... ok
test pack::additive_growth_is_tolerated ... ok
test pack::measure_read_costs ... ignored, a measurement for predecessor:docs/designs/catalog-artifact.md, not an assertion
test pack::an_operation_naming_an_absent_provider_is_refused ... ok
test pack::something_that_is_not_a_pack_is_refused ... ok
test pack::personal_acquisition_does_not_reinterpret_schema_three_packs ... ok
test pack::personal_acquisition_schema_four_is_admitted_before_any_record_is_served ... ok
test pack::source_fidelity_does_not_reinterpret_schema_two_packs ... ok
test pack::the_vendored_sha256_agrees_with_sha2_across_padding_boundaries ... ok
test pack::a_newer_container_format_is_refused_by_name ... ok
test pack::the_embedded_pack_serves_the_shipped_catalogue ... ok
test pack::a_tampered_payload_is_refused_before_any_record ... ok
test pack::load_serves_the_committed_pack ... ok
test pack::every_embedded_record_agrees_with_its_canonical_document ... ok
test pack::oauth_pass1_full_published_pack_version_is_checked_before_nonempty_records ... ok

test result: ok. 16 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 3.51s

     Running unittests src/lib.rs (/dev/shm/b10x-cli-wave-wt-4f1de73d0685-nr00xjox/runtime-target/debug/deps/connector_oauth-9f92ae8f31968df6)

running 33 tests
test device::tests::caller_clock_rollback_never_shortens_the_polling_interval ... ok
test device::tests::device_instructions_do_not_invent_a_complete_uri_and_deadline_is_minimum ... ok
test device::tests::device_poll_expiry_cancel_and_unsolicited_response_are_terminal ... ok
test device::tests::device_response_refuses_the_entire_origin_bounds_and_expiry_class ... ok
test device::tests::device_poll_obeys_default_interval_pending_slow_down_and_timeout_backoff ... ok
test pkce::tests::a_random_token_is_unpadded_url_safe_and_the_requested_width ... ok
test pkce::tests::an_origin_carrying_a_query_or_fragment_contributes_neither ... ok
test pkce::tests::an_authorize_url_carries_the_pkce_pair_only_for_a_public_client ... ok
test device::tests::device_poll_terminal_outcomes_never_authorize_again_or_retain_a_code ... ok
test pkce::tests::authorize_url_percent_encodes_the_redirect_and_appends_extras_in_order ... ok
test pkce::tests::two_random_tokens_differ ... ok
test state::tests::a_state_is_redeemable_once ... ok
test pkce::tests::the_pkce_challenge_is_the_s256_of_the_verifier ... ok
test state::tests::an_expired_state_is_not_redeemable_and_does_not_linger ... ok
test pkce::tests::authorize_url_refuses_a_url_that_cannot_carry_a_path ... ok
test state::tests::clear_drops_live_entries_too ... ok
test state::tests::contains_reports_liveness_without_redeeming ... ok
test state::tests::contains_any_claims_an_expired_state_so_the_callback_is_refused_not_lost ... ok
test state::tests::expire_drops_only_what_is_past ... ok
test state::tests::insert_refuses_once_the_table_is_full_of_live_entries ... ok
test token::tests::a_conforming_response_validates_and_drops_unretained_scopes ... ok
test state::tests::insert_sweeps_expired_entries_before_refusing ... ok
test token::tests::refresh_is_due_inside_the_skew_and_whenever_the_clock_is_unavailable ... ok
test token::tests::personal_bearer_accepts_case_and_requires_bounded_finite_expiry ... ok
test token::tests::a_zero_expires_in_passes_only_where_the_caller_does_not_rely_on_it ... ok
test token::tests::comma_separated_scopes_are_split_and_trimmed ... ok
test token::tests::every_gitlab_condition_refuses ... ok
test state::tests::remove_returns_an_expired_entry_so_a_caller_can_refuse_rather_than_not_find ... ok
test token::tests::expiry_is_measured_from_our_clock ... ok
test token::tests::jira_tolerates_an_absent_created_at_and_an_absent_refresh_on_rotation ... ok
test state::tests::replacing_a_live_state_does_not_count_against_capacity ... ok
test token::tests::a_required_scope_must_survive_the_retain_filter ... ok
test token::tests::optional_refresh_is_bounded_even_when_issuance_is_not_required ... ok

test result: ok. 33 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running unittests src/lib.rs (/dev/shm/b10x-cli-wave-wt-4f1de73d0685-nr00xjox/runtime-target/debug/deps/connector_resolve-1c82d48a134d0594)

running 54 tests
test auth::tests::a_header_the_template_already_sets_is_refused_rather_than_overwritten ... ok
test auth::tests::a_basic_join_composes_the_pair_the_vendor_expects ... ok
test auth::tests::a_query_placement_registers_the_encoded_form_it_sends ... ok
test auth::tests::base64_matches_rfc_4648s_own_vectors ... ok
test auth::tests::a_prefixed_header_carries_the_bare_value_inside_it ... ok
test auth::tests::the_assembled_debug_prints_no_value ... ok
test auth::tests::an_inbound_signing_secret_never_leaves ... ok
test config::tests::a_username_prefix_is_the_one_reserved_qualifier ... ok
test config::tests::a_config_value_debug_carries_no_value ... ok
test document::tests::a_sip_session_driver_survives_the_canonical_document ... ok
test document::tests::a_pre_c552_document_without_symbols_falls_back_to_the_allocation ... ok
test document::tests::a_stated_symbol_is_honored_over_the_naive_allocation ... ok
test document::tests::the_emitters_own_symbols_are_reserved ... ok
test document::tests::the_symbol_allocation_reproduces_the_emitters ... ok
test plan::tests::sensitive_text_never_prints ... ok
test request::tests::a_duplicate_query_key_is_refused_rather_than_sent_twice ... ok
test request::tests::a_fragment_survives_the_appended_query ... ok
test document::tests::source_fidelity_refuses_missing_unknown_profiles_and_old_catalog_versions ... ok
test request::tests::the_default_identity_names_this_software_and_this_repository ... ok
test plan::tests::a_plans_debug_carries_no_credential ... ok
test request::tests::debug_prints_shape_and_never_a_value ... ok
test request::tests::the_params_omit_what_is_absent ... ok
test resolve::tests::source_fidelity_encodes_path_values_without_changing_route_or_authority ... ok
test resolve::tests::source_fidelity_keeps_json_string_values_and_explicit_null ... ok
test resolve::tests::source_fidelity_preserves_body_omission_null_and_inner_requiredness ... ok
test document::tests::a_shipped_document_parses_into_its_services_and_operations ... ok
test document::tests::an_unknown_provider_or_operation_is_absent_rather_than_a_panic ... ok
test resolve::tests::an_omitted_required_parameter_is_refused_and_names_itself ... ok
test resolve::tests::a_configuration_value_that_moves_the_authority_is_refused ... ok
test resolve::tests::a_caller_parameter_that_leaves_its_path_segment_is_refused ... ok
test slot::tests::a_host_rule_refuses_what_moves_the_authority ... ok
test slot::tests::a_document_position_maps_onto_a_slot_and_anything_else_fails_closed ... ok
test slot::tests::an_unplaced_value_is_held_to_every_rule_including_the_hosts ... ok
test resolve::tests::an_optional_query_filter_may_simply_be_left_out ... ok
test slot::tests::a_value_that_moves_the_authority_is_refused_in_context ... ok
test template::tests::markers_are_located_and_filled_by_offset ... ok
test resolve::tests::a_null_query_field_is_omitted_rather_than_sent_empty ... ok
test resolve::tests::a_caller_value_spelling_a_configuration_variable_does_not_reach_the_wire ... ok
test resolve::tests::an_omitted_optional_body_field_is_not_a_missing_parameter ... ok
test template::tests::an_unfilled_placeholder_stays_verbatim ... ok
test template::tests::a_doubled_brace_is_an_escape_and_an_unterminated_one_is_text ... ok
test template::tests::truthiness_is_flux_langs ... ok
test template::tests::value_to_text_is_flux_langs ... ok
test resolve::tests::every_request_carries_this_softwares_identity ... ok
test resolve::tests::gitlab_publication_keeps_the_reviewed_actions_in_one_json_request ... ok
test resolve::tests::the_plan_carries_the_placed_credential_and_the_set_to_redact ... ok
test resolve::tests::the_request_is_the_documents_request ... ok
test document::tests::an_operation_resolves_through_the_embedded_pack_without_naming_its_provider ... ok
test resolve::tests::source_fidelity_integer_parameters_accept_json_number_spellings_without_fractional_wire_text ... ok
test resolve::tests::source_fidelity_gitlab_requests_preserve_complete_body_values_and_defaults ... ok
test credentials::tests::a_basic_join_with_no_user_half_refuses_by_name ... ok
test credentials::tests::a_basic_join_reads_its_user_half_from_the_config_port ... ok
test credentials::tests::an_unstored_credential_refuses_and_names_the_alternatives ... ok
test credentials::tests::a_bearer_credential_assembles_and_lists_its_redaction ... ok

test result: ok. 54 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.88s

     Running tests/adversary_gitlab_pass1.rs (/dev/shm/b10x-cli-wave-wt-4f1de73d0685-nr00xjox/runtime-target/debug/deps/adversary_gitlab_pass1-82347af6da0fc8a7)

running 4 tests
test adversary_gitlab_pass1_schema_versions_and_profiles_fail_closed ... ok
test current_schema_four_and_reader_keep_all_version_and_profile_refusal_classes ... ok
test adversary_gitlab_pass1_body_literals_do_not_become_template_instructions ... ok
test adversary_gitlab_pass1_path_and_integer_boundaries_preserve_caller_values ... ok

test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.58s

     Running unittests src/lib.rs (/dev/shm/b10x-cli-wave-wt-4f1de73d0685-nr00xjox/runtime-target/debug/deps/connector_spec-d9099b8159241cd4)

running 28 tests
test config::tests::a_host_value_is_refused_for_what_no_request_position_would_catch ... ok
test config::tests::a_multi_destination_field_carries_one_slot_into_every_pin ... ok
test config::tests::a_pinned_request_value_parses_and_is_connection_level_configuration ... ok
test config::tests::a_username_head_qualifies_the_placeholder_of_its_request_pin ... ok
test config::tests::bindings_parse_and_carry_their_level_and_secrecy ... ok
test config::tests::every_template_variable_is_reported_not_only_the_first ... ok
test config::tests::a_pinned_value_cannot_reshape_the_request_it_lands_in ... ok
test config::tests::formats_validate_the_values_they_claim ... ok
test graph::tests::a_chain_orders_topologically ... ok
test graph::tests::a_cycle_has_no_order_at_all ... ok
test graph::tests::a_diamond_converges_because_data_edges_need_no_nesting ... ok
test graph::tests::comparisons_map_to_flux_operators ... ok
test graph::tests::enclosing_walks_outwards_and_detects_a_containment_cycle ... ok
test graph::tests::region_and_boundary_kinds_are_classified ... ok
test inbound::tests::a_count_too_large_to_scale_is_refused_rather_than_wrapped ... ok
test inbound::tests::a_window_is_a_whole_number_of_seconds_minutes_or_hours ... ok
test inbound::tests::a_window_no_host_could_apply_is_not_a_window ... ok
test inbound::tests::an_unterminated_placeholder_is_reported_as_one_no_host_can_fill ... ok
test inbound::tests::every_payload_placeholder_is_one_the_host_can_fill ... ok
test inbound::tests::paths_reject_empty_segments_and_whitespace ... ok
test inbound::tests::signed_templates_report_their_placeholders_in_order ... ok
test inbound::tests::symbols_are_snake_case_because_a_hyphen_reads_as_subtraction ... ok
test names::tests::a_dotted_vendor_name_becomes_an_identifier ... ok
test names::tests::a_leading_digit_is_prefixed ... ok
test names::tests::an_empty_or_brace_bearing_name_is_refused ... ok
test names::tests::an_identifier_safe_name_is_left_alone ... ok
test names::tests::the_reserved_symbols_are_not_handed_out ... ok
test names::tests::two_names_that_normalize_alike_stay_distinct ... ok

test result: ok. 28 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests/main.rs (/dev/shm/b10x-cli-wave-wt-4f1de73d0685-nr00xjox/runtime-target/debug/deps/main-3ba4a7286c43068f)

running 517 tests
test auth_archetypes::a_public_client_is_exempt_from_the_client_secret_a_confidential_one_owes ... ok
test auth_hazard::a_credential_declaring_no_hazard_carries_none ... ok
test auth_archetypes::the_operator_level_is_expressible ... ok
test auth_hazard::a_grant_list_without_the_password_grant_needs_no_hazard ... ok
test auth_hazard::a_password_grant_that_declares_no_hazard_is_refused ... ok
test auth_hazard::the_near_miss_hazard_spelling_is_refused_naming_the_value ... ok
test auth_hazard::the_declared_hazard_is_the_word_the_consuming_deployment_gate_reads ... ok
test auth_prefix::a_declared_prefix_round_trips_through_the_encoding ... ok
test auth_prefix::a_header_placement_carries_an_arbitrary_scheme_word ... ok
test auth_prefix::a_prefix_may_carry_punctuation_and_still_be_a_prefix ... ok
test auth_prefix::a_scheme_word_that_is_not_oauth2_is_still_just_a_prefix ... ok
test auth_prefix::a_prefix_missing_its_trailing_separator_is_refused ... ok
test auth_prefix::a_prefix_may_not_end_in_an_alphanumeric_character ... ok
test auth_prefix::a_prefix_may_not_break_out_of_the_header_value ... ok
test auth_prefix::a_prefix_may_not_carry_leading_or_doubled_whitespace ... ok
test auth_prefix::a_prefix_may_not_spell_a_resolution_marker ... ok
test auth_prefix::a_prefix_may_not_name_the_credential_or_its_env_var ... ok
test auth_prefix::repeated_punctuation_is_the_vendors_business_and_still_loads ... ok
test auth_prefix::an_omitted_prefix_is_empty_and_does_not_serialize ... ok
test auth_prefix::a_whitespace_only_prefix_is_refused ... ok
test auth_prefix::the_preset_schemes_carry_no_prefix_of_their_own ... ok
test auth_workarounds::a_token_endpoint_workaround_without_a_grant_is_refused ... ok
test auth_prefix::there_is_no_suffix_axis ... ok
test auth_workarounds::a_workaround_measured_on_a_non_date_is_refused ... ok
test auth_workarounds::a_workaround_does_not_reach_a_sibling_credential_in_the_same_connector ... ok
test auth_workarounds::a_workaround_without_attribution_is_refused ... ok
test auth_workarounds::two_workarounds_for_one_grant_are_refused ... ok
test channel_bindings::a_channel_and_an_operation_may_not_share_a_name ... ok
test channel_bindings::a_complete_binding_loads_and_composes_an_event_with_a_reply ... ok
test channel_bindings::a_binding_carrying_an_undeclared_event_is_refused ... ok
test channel_bindings::a_cursor_on_a_transport_that_is_not_polled_is_refused ... ok
test channel_bindings::a_parameter_cannot_be_both_bound_and_the_journey_result ... ok
test channel_bindings::a_payload_key_that_is_not_a_flux_symbol_is_refused ... ok
test channel_bindings::a_generic_socket_round_trips_every_connect_event_payload_and_config_fact ... ok
test channel_bindings::a_duplicate_operation_id_is_reported_once_and_not_also_as_a_namespace_collision ... ok
test channel_bindings::a_payload_source_path_with_an_empty_segment_is_refused ... ok
test channel_bindings::a_poll_binding_with_a_cursor_loads_and_may_omit_its_events ... ok
test channel_bindings::a_reply_binding_a_parameter_the_operation_does_not_declare_is_refused ... ok
test channel_bindings::a_poll_binding_without_a_cursor_is_refused ... ok
test channel_bindings::a_push_binding_carrying_no_events_is_refused ... ok
test channel_bindings::a_reply_leaving_a_required_parameter_unbound_is_refused ... ok
test channel_bindings::a_reply_naming_an_operation_nobody_declares_is_refused ... ok
test channel_bindings::a_reply_binding_from_a_symbol_the_payload_never_produces_is_refused ... ok
test channel_bindings::a_session_binding_loads_as_non_event_ingress_with_closed_admission_facts ... ok
test channel_bindings::a_reply_with_no_result_leaves_the_journey_output_parameter_unbound ... ok
test channel_bindings::a_signed_template_covering_only_the_url_is_refused ... ok
test channel_bindings::a_signed_template_that_never_interpolates_the_body_is_refused ... ok
test channel_bindings::a_timestamped_hmac_scheme_loads_with_its_window_and_its_selector ... ok
test channel_bindings::a_timestamped_scheme_without_a_timestamp_selector_is_refused ... ok
test channel_bindings::a_signed_template_the_host_cannot_fill_is_refused ... ok
test channel_bindings::a_socket_binding_may_declare_manual_vendor_side_setup ... ok
test auth_archetypes::raw_value_header_renders_the_same_form_as_a_bearer ... ok
test channel_bindings::a_timestamp_format_loads_beside_its_selector ... ok
test channel_bindings::a_timestamped_scheme_without_a_tolerance_is_refused ... ok
test channel_bindings::a_tolerance_too_large_to_scale_is_refused_by_the_loader ... ok
test channel_bindings::a_tolerance_that_is_not_a_duration_is_refused ... ok
test channel_bindings::a_webhook_binding_may_declare_itself_unverifiable_deliberately ... ok
test channel_bindings::an_event_name_may_carry_the_vendors_own_dots_and_underscores ... ok
test channel_bindings::an_event_and_an_operation_may_not_share_a_name ... ok
test channel_bindings::a_verification_timestamp_read_from_the_body_is_refused ... ok
test channel_bindings::a_webhook_binding_that_states_no_verification_is_refused ... ok
test channel_bindings::a_webhook_secret_declared_as_an_outbound_credential_is_refused ... ok
test channel_bindings::an_operation_cannot_authenticate_with_a_signing_credential ... ok
test channel_bindings::a_webhook_secret_that_no_credential_declares_is_refused ... ok
test channel_bindings::every_member_kind_addresses_and_round_trips_through_one_oip_form ... ok
test channel_bindings::an_event_name_that_could_not_travel_in_an_address_is_refused ... ok
test channel_bindings::session_ingress_refuses_missing_facts_and_event_only_fields ... ok
test config_choices::a_pinned_field_checks_every_choice_against_its_request_position ... ok
test channel_bindings::twilios_url_and_sorted_form_scheme_is_declarable ... ok
test config_choices::a_config_field_declares_a_closed_set_of_values_and_a_value_outside_it_is_refused ... ok
test config_choices::a_secret_field_cannot_declare_a_closed_set ... ok
test channel_bindings::verification_on_a_transport_that_cannot_use_it_is_refused ... ok
test config_choices::a_set_with_one_value_is_refused_and_an_empty_one_is_an_open_field ... ok
test config_choices::every_permitted_value_still_satisfies_the_fields_format ... ok
test config_choices::an_example_outside_the_closed_set_is_refused ... ok
test config_choices::a_choice_must_be_renderable_and_distinct ... ok
test config_fields::a_binding_that_is_not_a_binding_at_all_is_refused ... ok
test config_fields::a_complete_configuration_surface_loads_and_derives_its_levels ... ok
test config_fields::a_connector_with_no_configuration_at_all_is_refused_when_it_needs_some ... ok
test config_fields::a_config_field_can_pin_a_path_segment ... ok
test config_fields::a_connector_with_a_literal_base_url_needs_no_endpoint_field ... ok
test config_fields::a_credential_field_that_claims_not_to_be_secret_is_refused ... ok
test config_fields::a_destination_named_twice_is_refused ... ok
test config_fields::a_field_binding_a_credential_nobody_declares_is_refused ... ok
test config_fields::a_field_without_a_label_is_refused ... ok
test config_fields::a_field_without_help_is_refused ... ok
test config_fields::a_credential_cannot_also_reach_a_request_position ... ok
test channel_bindings::socket_connect_declarations_fail_closed_at_load ... ok
test config_fields::a_field_binding_a_template_variable_that_does_not_exist_is_refused ... ok
test config_fields::a_header_pin_on_an_auth_owned_header_is_refused ... ok
test config_fields::a_pin_parses_to_its_position_and_derives_its_level_and_secrecy ... ok
test config_fields::a_non_credential_field_that_claims_to_be_secret_is_refused ... ok
test config_fields::a_path_pin_no_operation_carries_is_refused ... ok
test config_fields::a_pin_that_claims_to_be_secret_is_refused ... ok
test config_fields::a_non_secret_field_may_still_declare_an_example ... ok
test config_fields::a_further_destination_that_is_not_a_request_position_is_refused ... ok
test config_fields::a_path_pin_whose_example_escapes_its_segment_is_refused ... ok
test config_fields::a_pinned_query_parameter_that_is_also_an_argument_is_refused ... ok
test config_fields::a_secret_field_that_declares_an_example_is_refused ... ok
test config_fields::a_query_parameter_and_a_header_can_be_pinned_too ... ok
test config_fields::a_template_variable_nothing_binds_is_refused ... ok
test config_fields::a_username_field_is_not_secret ... ok
test config_fields::a_username_field_on_a_non_basic_credential_is_refused ... ok
test config_fields::a_value_that_composes_a_host_is_refused_when_it_could_move_the_authority ... ok
test config_fields::a_verify_operation_loads_and_resolves ... ok
test config_fields::a_verify_operation_that_writes_is_refused ... ok
test config_fields::a_verify_operation_that_does_not_exist_is_refused ... ok
test config_fields::a_value_that_is_both_pinned_and_declared_as_a_parameter_is_refused ... ok
test config_fields::an_example_that_fails_its_own_format_is_refused ... ok
test config_fields::an_optional_pin_is_refused ... ok
test config_fields::every_permitted_choice_is_checked_against_every_destination ... ok
test config_fields::an_oauth_field_without_an_oauth_credential_is_refused ... ok
test config_fields::an_example_is_checked_against_every_destination_and_not_only_the_first ... ok
test config_fields::config_names_join_the_shared_member_namespace ... ok
test config_fields::one_field_declares_two_destinations_and_one_value_reaches_both ... ok
test config_fields::the_username_placeholder_prefix_is_reserved_from_endpoint_fields ... ok
test config_fields::two_fields_that_would_share_one_placeholder_are_refused ... ok
test auth_archetypes::a_connector_with_no_credential_still_has_a_form ... ok
test config_fields::two_fields_writing_one_header_are_refused ... ok
test auth_archetypes::basic_join_without_a_marker_is_a_distinct_form ... ok
test constant_headers::a_constant_header_must_state_a_value ... ok
test constant_headers::a_constant_header_survives_the_ir_round_trip ... ok
test constant_headers::a_provider_level_constant_header_is_distributed_onto_every_operation ... ok
test constant_headers::a_constant_header_value_may_not_carry_a_line_break ... ok
test constant_headers::a_constant_header_name_must_be_an_http_field_name ... ok
test constant_headers::an_operation_level_constant_header_may_not_carry_a_credential_either ... ok
test constant_headers::a_provider_level_refusal_is_reported_once ... ok
test constant_headers::an_operation_without_constant_headers_encodes_as_it_always_did ... ok
test credential_paths::a_path_from_another_convention_is_refused_rather_than_guessed_at ... ok
test credential_paths::an_explicitly_spelled_default_service_does_not_parse ... ok
test constant_headers::an_operations_own_constant_header_replaces_the_providers ... ok
test constant_headers::a_constant_header_may_not_carry_a_credential ... ok
test credential_paths::an_instance_that_is_not_a_uuid_is_refused_and_the_refusal_names_the_component ... ok
test credential_paths::an_instanced_path_cannot_be_confused_with_a_service ... ok
test config_fields::the_atlassian_connectors_address_the_cloud_gateway_by_id ... ok
test config_choices::the_shipped_connectors_that_have_regions_declare_them ... ok
test config_fields::the_atlassian_connectors_prefer_their_service_account ... ok
test credential_paths::the_default_service_is_elided_and_the_elision_stays_unambiguous ... ok
test credential_paths::the_tenant_validator_is_public_so_a_host_can_check_before_it_builds ... ok
test credential_paths::every_admissible_reference_round_trips_and_no_rejected_one_renders ... ok
test channel_bindings::the_shipped_slack_bindings_describe_both_of_slacks_real_transports ... ok
test credential_response::a_credential_at_the_response_root_is_reachable ... ok
test credential_response::a_credential_inside_an_array_of_objects_is_reachable ... ok
test auth_archetypes::slack_socket_mode_token_enters_through_a_connect_session_and_never_ambient_env ... ok
test credential_response::every_withheld_operation_is_recorded_in_a_provider_file ... ok
test credential_subject::a_connector_declares_a_bot_and_a_user_credential_side_by_side ... ok
test credential_subject::an_undeclared_subject_is_unstated ... ok
test credential_subject::an_unstated_subject_serializes_to_nothing ... ok
test custody_only::a_channel_binding_cannot_carry_the_credential_out ... ok
test custody_only::a_credential_and_nothing_else_loads ... ok
test custody_only::a_credential_that_asks_the_host_to_run_grants_is_refused ... ok
test custody_only::a_custody_only_provider_holding_nothing_is_refused ... ok
test custody_only::an_empty_auth_array_holds_nothing_too ... ok
test custody_only::an_ordinary_provider_still_needs_a_base_url_and_a_surface ... ok
test custody_only::every_key_that_would_describe_a_request_surface_is_refused_by_name ... ok
test custody_only::the_flag_is_in_the_hash_domain_but_costs_absent_declarations_nothing ... ok
test determinism::decode_then_encode_is_a_fixed_point ... ok
test determinism::identical_inputs_serialize_to_identical_bytes ... ok
test determinism::repeated_serialization_is_stable ... ok
test determinism::requirement_encoding_ignores_authoring_order ... ok
test determinism::serde_json_object_keys_stay_sorted ... ok
test discovery::discovery_cannot_name_a_nonexistent_operation ... ok
test discovery::duplicate_vendor_type_mapping_is_refused_instead_of_precedence_ordered ... ok
test discovery::one_read_can_declare_a_closed_native_provider_mapping ... ok
test execution_facts::a_seeded_write_remains_write_and_carries_write_effects ... ok
test execution_facts::audio_v1_is_a_closed_unary_device_driver ... ok
test execution_facts::cdp_v1_is_a_closed_leased_session_browser_driver ... ok
test execution_facts::effects_are_required_and_never_derived_from_http ... ok
test execution_facts::every_catalog_operation_requires_a_non_empty_description ... ok
test execution_facts::host_and_semantic_effects_remain_independent_axes ... ok
test execution_facts::predecessor_runtime_and_quirks_vocabularies_are_refused_by_name ... ok
test execution_facts::sip_v1_is_a_closed_session_establishment_driver ... ok
test execution_facts::sql_v1_is_a_closed_unary_database_driver ... ok
test execution_facts::unknown_effect_driver_and_capability_values_are_refused_by_name ... ok
test credential_paths::the_three_outcomes_are_distinguishable ... ok
test auth_archetypes::slack_bearers_keep_bot_user_admin_and_app_purposes_separate ... ok
test graphs::a_boundary_node_may_not_take_inputs_or_sit_in_a_region ... ok
test graphs::a_connector_without_graphs_encodes_as_it_did_before ... ok
test graphs::a_cycle_is_refused_because_flux_has_no_goto ... ok
test graphs::a_gate_cannot_export_a_value ... ok
test graphs::a_graph_is_in_the_hash_domain ... ok
test graphs::a_graph_name_that_could_not_be_a_flux_declaration_is_refused ... ok
test graphs::a_node_contained_in_itself_is_refused ... ok
test graphs::a_node_naming_an_operation_nobody_declares_is_refused ... ok
test graphs::a_region_naming_a_node_that_contains_nothing_is_refused ... ok
test graphs::a_retry_may_export_a_value ... ok
test graphs::a_trigger_naming_an_undeclared_event_is_refused ... ok
test graphs::a_value_may_not_leave_a_region_through_a_port_the_region_does_not_declare ... ok
test graphs::a_zero_bound_retry_is_refused ... ok
test graphs::an_edge_naming_a_port_that_does_not_exist_is_refused ... ok
test graphs::graph_names_join_the_shared_member_namespace ... ok
test graphs::no_node_kind_carries_a_formula ... ok
test graphs::the_comparison_vocabulary_is_closed ... ok
test auth_archetypes::slack_has_distinct_bot_install_and_delegated_user_oauth_flows ... ok
test graphs::the_worked_example_loads_and_composes_declared_members ... ok
test ir_roundtrip::an_exposed_operation_omits_only_the_default_exposure_field ... ok
test ir_roundtrip::a_hand_authored_toml_defines_a_complete_operation ... ok
test ir_roundtrip::an_unexposed_operation_reaches_the_hash_domain ... ok
test ir_roundtrip::auth_requirement_is_a_set ... ok
test ir_roundtrip::auth_scheme_matches_the_flux_plugin_protocol_vocabulary ... ok
test ir_roundtrip::body_encoding_is_closed_and_its_default_is_invisible ... ok
test ir_roundtrip::auth_requirement_cardinalities_round_trip ... ok
test ir_roundtrip::credentials_resolve_to_declared_auth_methods ... ok
test ir_roundtrip::operation_metadata_uses_the_flux_vocabulary ... ok
test ir_roundtrip::operation_traits_and_provenance_round_trip ... ok
test ir_roundtrip::parameter_and_response_schemas_survive_the_round_trip ... ok
test ir_roundtrip::rate_stage2_conditional_declarations_are_bounded_without_extending_fixed_rates ... ok
test ir_roundtrip::unset_and_explicit_empty_auth_differ_on_the_wire ... ok
test legacy_default_service::a_default_beside_a_named_service_without_the_legacy_marker_stays_refused ... ok
test legacy_default_service::a_legacy_marker_without_a_named_sibling_is_refused ... ok
test ir_roundtrip::rate_adversary_fixed_and_conditional_roundtrip_keep_distinct_meanings ... ok
test legacy_default_service::a_named_service_cannot_claim_the_legacy_default_marker ... ok
test legacy_default_service::an_explicit_legacy_default_can_coexist_with_a_named_service ... ok
test legacy_default_service::every_spec_document_of_a_mixed_connector_must_state_its_service ... ok
test lockfile::a_changed_generator_moves_the_artifact_hashes_alone ... ok
test lockfile::a_changed_spec_moves_the_spec_hash ... ok
test lockfile::a_changed_toml_moves_the_toml_hash_and_nothing_upstream_of_it ... ok
test legacy_default_service::every_member_of_a_mixed_connector_must_state_its_service ... ok
test lockfile::a_comment_only_edit_moves_the_toml_hash_alone ... ok
test lockfile::a_rebuild_replaces_a_row_rather_than_duplicating_it ... ok
test lockfile::a_lockfile_from_another_format_version_is_refused ... ok
test lockfile::a_second_spec_hash_is_additive ... ok
test lockfile::an_unknown_key_is_refused ... ok
test lockfile::the_hash_domain_covers_the_compiled_meaning ... ok
test lockfile::the_hash_domain_excludes_every_provenance_field ... ok
test lockfile::the_hash_domain_excludes_fetched_at ... ok
test lockfile::the_lockfile_carries_no_credential_and_no_endpoint ... ok
test lockfile::the_lockfile_records_no_timestamp ... ok
test lockfile::the_pack_section_renders_and_round_trips ... ok
test lockfile::the_rendered_shape_is_pinned ... ok
test lockfile::the_hash_is_stable_across_repeated_computation ... ok
test credential_paths::slack_derives_a_path_for_each_of_its_credentials ... ok
test oauth2_acquisition::a_plain_credential_declares_no_oauth2 ... ok
test oauth2_acquisition::a_redirect_uri_without_a_grant_is_refused ... ok
test oauth2_acquisition::a_redirect_uri_is_an_operator_level_registration_field ... ok
test oauth2_acquisition::a_scope_response_location_must_be_a_json_pointer ... ok
test oauth2_acquisition::a_scope_response_location_cannot_name_credential_material ... ok
test oauth2_acquisition::an_oauth2_credential_loads_with_every_field_intact ... ok
test oauth2_acquisition::declaring_both_an_oauth2_grant_and_a_minting_operation_is_refused ... ok
test oauth_token_endpoint::a_confidential_client_omits_the_discriminator ... ok
test oauth_token_endpoint::a_dangling_token_endpoint_is_refused_naming_it ... ok
test oauth_token_endpoint::a_public_client_loads_and_is_marked_public ... ok
test oauth_token_endpoint::a_two_host_declaration_loads_and_carries_both_services ... ok
test oauth_token_endpoint::an_absent_token_endpoint_is_skipped_entirely ... ok
test openapi_ingest::a_body_the_ir_cannot_express_skips_the_operation_rather_than_dropping_the_body ... ok
test openapi_ingest::a_cookie_parameter_skips_the_operation_rather_than_being_dropped_from_it ... ok
test openapi_ingest::a_document_this_ingest_cannot_read_is_a_whole_document_error ... ok
test openapi_ingest::a_duplicate_operation_id_is_reported_rather_than_resolved_by_position ... ok
test openapi_ingest::a_cyclic_response_ref_is_bounded_rather_than_expanded_forever ... ok
test openapi_ingest::a_method_the_ir_cannot_spell_is_reported_rather_than_dropped_silently ... ok
test openapi_ingest::a_missing_section_is_a_diagnostic_naming_it ... ok
test openapi_ingest::a_malformed_endpoint_is_a_diagnostic_naming_it_rather_than_a_failed_ingest ... ok
test openapi_ingest::a_recursive_request_remains_an_exact_contract_and_is_refused ... ok
test openapi_ingest::a_path_items_parameters_reach_every_operation_under_it ... ok
test openapi_ingest::a_recursive_response_is_bounded_without_dropping_the_operation ... ok
test openapi_ingest::an_external_ref_is_refused_rather_than_followed ... ok
test openapi_ingest::a_yaml_document_ingests_including_its_integer_response_keys ... ok
test openapi_ingest::an_object_request_body_becomes_named_body_parameters ... ok
test openapi_ingest::both_openapi_3_0_and_3_1_documents_ingest ... ok
test openapi_ingest::operation_order_does_not_depend_on_the_documents_key_order ... ok
test openapi_ingest::ingest_is_deterministic ... ok
test openapi_ingest::refs_resolve_including_nested_and_repeated_ones ... ok
test openapi_ingest::parameters_land_in_their_request_position_with_their_schemas ... ok
test openapi_ingest::source_fidelity_preserves_missing_item_constraints_and_literal_reference_data ... ok
test openapi_ingest::servers_carry_their_templating_and_their_variables ... ok
test openapi_ingest::source_fidelity_reports_unsupported_semantics_without_a_permissive_substitute ... ok
test openapi_ingest::source_fidelity_refuses_recursive_response_widening_and_nondefault_serialization ... ok
test openapi_ingest::the_anthropic_spec_has_authored_provenance ... ok
test lockfile::unchanged_inputs_reproduce_the_lockfile_byte_for_byte ... ok
test grafana::grafana_is_a_private_reachable_read_surface_with_connector_custody ... ok
test grafana::grafana_query_keeps_the_batch_bounded ... ok
test auth_archetypes::a_signing_secret_is_collected_like_any_credential_and_sent_nowhere ... ok
test operation_selection::a_path_prefix_matches_on_segment_boundaries ... ok
test operation_selection::a_deferred_operation_cannot_also_be_corrected ... ok
test operation_selection::a_bulk_conditional_still_owes_a_condition_per_operation ... ok
test operation_selection::a_deferral_reason_must_be_nonempty ... ok
test operation_selection::a_block_overrides_a_selectors_risk ... ok
test operation_selection::a_per_operation_block_wins_over_a_selector ... ok
test operation_selection::a_pin_naming_an_absent_operation_id_is_refused ... ok
test operation_selection::a_pin_overrides_the_rule_and_a_rename_overrides_the_pin ... ok
test operation_selection::a_selector_states_exposure_for_the_set ... ok
test operation_selection::a_selector_matches_by_service_path_prefix_and_method ... ok
test operation_selection::an_internal_path_is_never_selected ... ok
test operation_selection::an_operation_id_that_cannot_produce_a_legal_name_is_reported ... ok
test operation_selection::an_upstream_operation_id_rename_orphans_direction_and_refuses ... ok
test operation_selection::changing_only_upstream_methods_before_composition_preserves_authored_directions ... ok
test operation_selection::a_read_may_go_unstated ... ok
test operation_selection::description_corrections_preserve_bulk_selection_and_document_order ... ok
test operation_selection::description_corrections_refuse_conflicting_exact_patches ... ok
test operation_selection::description_corrections_refuse_stale_identities_and_empty_values ... ok
test operation_selection::a_selector_states_risk_and_idempotency_for_the_set ... ok
test operation_selection::a_selector_that_matches_nothing_is_refused ... ok
test config_fields::zendesk_declares_a_complete_connect_form ... ok
test operation_selection::a_spec_backed_provider_with_no_selector_publishes_nothing ... ok
test operation_selection::response_array_schema_rewrites_are_rejected_for_every_pointer ... ok
test operation_selection::response_array_schema_rewrites_cannot_wrap_valid_source_schemas ... ok
test operation_selection::deferring_an_operation_no_selector_matched_is_refused ... ok
test operation_selection::source_fidelity_preserves_whole_body_constraints_and_body_presence ... ok
test operation_selection::source_fidelity_refuses_schema_replacement_and_parameter_omission ... ok
test operation_selection::an_exact_deferral_withholds_one_selector_match ... ok
test operation_selection::exposure_still_defaults_to_exposed ... ok
test operation_selection::overlapping_selectors_that_disagree_are_refused ... ok
test operation_selection::there_is_no_hide_key ... ok
test credential_paths::a_single_instance_address_is_byte_identical_to_the_four_component_form ... ok
test operation_selection::the_derived_id_set_is_pinned ... ok
test operation_selection::overlapping_selectors_that_agree_are_accepted ... ok
test operation_spec_source::an_inline_operation_cannot_author_the_derived_marker ... ok
test operation_selection::silence_on_an_authored_write_refuses ... ok
test operator_pinned_origin::a_declared_origin_must_already_be_in_canonical_form ... ok
test operator_pinned_origin::an_open_origin_without_operator_approval_is_refused ... ok
test operator_pinned_origin::an_operator_pinned_origin_is_a_value_free_generic_config_declaration ... ok
test operator_pinned_origin::an_origin_accepts_only_an_absolute_https_origin_without_url_tail ... ok
test operator_pinned_origin::the_loader_accepts_exactly_the_canonical_origins_of_the_shared_corpus ... ok
test operation_selection::the_naming_rule_derives_the_declared_spelling ... ok
test auth_archetypes::basic_join_renders_two_fields_and_hides_the_vendor_marker ... ok
test operation_selection::two_operation_ids_deriving_one_op_id_refuse ... ok
test operation_selection::identical_inputs_produce_identical_ir ... ok
test param_omission::nothing_is_dropped_unless_the_patch_says_so ... ok
test param_omission::a_correction_is_applied_before_the_omission_that_depends_on_it ... ok
test credential_paths::two_instances_of_one_connector_for_one_tenant_render_different_addresses ... ok
test param_omission::omitting_a_parameter_the_document_does_not_declare_is_refused ... ok
test personal_oauth::explicit_personal_device_admission_survives_without_a_redirect_or_refresh_promise ... ok
test personal_oauth::explicit_personal_pkce_admission_survives_loading_without_changing_legacy_public_client ... ok
test personal_oauth::legacy_public_client_does_not_invent_personal_admission ... ok
test personal_oauth::oauth_pass1_explicit_empty_personal_admission_obeys_the_authoring_contract ... ok
test personal_oauth::personal_admission_refuses_ambiguous_flow_endpoints_evidence_and_registration_values ... ok
test produces_credential::a_credential_producing_operation_declares_the_handle_as_its_output ... ok
test produces_credential::a_produces_credential_location_naming_every_element_of_an_array_is_refused ... ok
test produces_credential::a_produces_credential_operation_declared_idempotent_is_refused ... ok
test produces_credential::a_produces_credential_operation_naming_no_secret_field_is_refused ... ok
test produces_credential::a_produces_credential_operation_on_a_connector_with_no_authority_is_refused ... ok
test produces_credential::a_produces_credential_operation_storing_an_undeclared_credential_is_refused ... ok
test produces_credential::a_produces_credential_operation_whose_response_schema_exposes_the_secret_is_refused ... ok
test produces_credential::an_operation_declaring_both_credential_declarations_is_refused_naming_which_governs ... ok
test param_omission::omitting_a_parameter_from_the_wrong_position_is_refused ... ok
test produces_credential::the_handle_field_is_the_word_the_runtime_answers_with ... ok
test produces_credential::two_operations_minting_one_credential_are_refused ... ok
test provider_schema::every_documented_object_forbids_additional_properties ... ok
test provider_schema::every_documented_object_lists_exactly_the_keys_the_loader_accepts ... ok
test param_omission::omitting_a_required_parameter_is_refused ... ok
test provider_schema::the_schema_documents_exactly_the_objects_the_loader_accepts ... ok
test provider_schema::every_ref_resolves_to_a_declared_def ... ok
test provider_schema::the_schema_is_published_and_self_describing ... ok
test provider_schema::the_schema_forbids_an_empty_auth_mechanism ... ok
test provider_schema::the_schema_publishes_the_exposure_default_the_loader_applies ... ok
test provider_schema::the_schema_marks_the_mandatory_keys_required ... ok
test provider_schema::the_schema_states_the_custody_only_conditional ... ok
test provider_schema::the_schema_publishes_the_loaders_own_repeatability_floor ... ok
test provider_toml::a_file_may_point_at_a_spec_and_still_declare_operations_inline ... ok
test provider_toml::a_hand_authored_file_produces_a_complete_connector ... ok
test provider_toml::a_basic_credential_can_state_a_literal_user_suffix ... ok
test provider_toml::a_spec_pointer_file_produces_the_patch_set ... ok
test provider_toml::authoring_order_inside_a_mechanism_does_not_reach_the_ir ... ok
test provider_toml::the_three_auth_states_survive_the_loader ... ok
test provider_toml::the_provider_file_hash_is_recorded_and_is_a_function_of_the_bytes ... ok
test provider_toml::unstated_patch_overrides_stay_distinguishable_from_stated_ones ... ok
test provider_toml_errors::every_required_rejection_has_a_fixture ... ok
test provider_toml_errors::no_snapshot_is_orphaned ... ok
test repeatability_condition_elision::an_operation_stating_a_condition_does_carry_it_into_the_hash_domain ... ok
test repeatability_condition_elision::an_operation_stating_no_condition_hashes_as_it_did_before_the_field_existed ... ok
test provider_toml_errors::every_rejection_matches_its_golden_snapshot ... ok
test param_omission::omitting_a_path_parameter_is_refused ... ok
test param_omission::omitting_a_parameter_changes_only_the_parameters ... ok
test credential_paths::several_instances_and_no_uuid_is_a_refusal_naming_what_would_have_worked ... ok
test param_omission::the_curated_argument_list_comes_back_when_the_patch_names_what_to_drop ... ok
test semantic_effects::consequential_effects_cannot_claim_idempotent ... ok
test semantic_effects::duplicate_semantic_effects_are_refused_instead_of_deduped ... ok
test semantic_effects::money_and_delete_require_the_destructive_risk_floor ... ok
test semantic_effects::pure_is_not_a_truthful_effect_for_an_http_operation ... ok
test semantic_effects::semantic_effects_have_one_canonical_order ... ok
test semantic_effects::unknown_semantic_effects_are_refused_at_load ... ok
test service_audiences::a_provider_level_audiences_key_is_refused ... ok
test service_audiences::a_repeated_audience_is_refused ... ok
test service_audiences::a_service_may_carry_several_audiences_and_the_provider_derives_the_union ... ok
test service_audiences::an_unknown_audience_is_refused_and_names_the_known_set ... ok
test config_fields::the_templated_providers_ask_for_their_tenant ... ok
test service_partition::a_connector_without_an_authority_or_a_version_renders_no_address ... ok
test service_partition::a_connector_without_services_has_exactly_the_default_one ... ok
test service_partition::a_gid_with_more_than_one_service_segment_is_refused ... ok
test service_partition::a_malformed_address_is_refused_component_by_component ... ok
test operation_spec_source::a_fully_inline_provider_has_no_derived_operation_source ... ok
test service_partition::a_service_overrides_the_connector_version_and_base_url ... ok
test service_partition::addresses_round_trip_through_the_default_elision ... ok
test service_partition::an_explicit_default_segment_is_refused ... ok
test service_partition::services_partition_the_operation_set ... ok
test service_partition::the_rendered_forms_are_the_ones_the_design_publishes ... ok
test service_partition::the_validators_decide_which_addresses_round_trip ... ok
test service_roles::a_default_service_entry_beside_a_named_service_is_refused ... ok
test service_roles::a_default_service_entry_carrying_no_roles_is_still_refused ... ok
test service_roles::a_default_service_entry_may_carry_nothing_but_roles ... ok
test service_roles::a_provider_declaring_no_roles_hashes_as_it_did_before_roles_existed ... ok
test service_roles::a_provider_level_roles_key_is_refused_and_points_at_the_service_level ... ok
test service_roles::a_providers_roles_are_the_union_of_its_services ... ok
test service_roles::a_required_member_is_matched_by_its_trailing_segments_not_its_full_id ... ok
test service_roles::a_role_declared_twice_on_one_service_is_refused ... ok
test service_roles::a_service_claiming_a_role_it_does_not_satisfy_is_refused ... ok
test service_roles::an_unknown_role_name_is_refused_and_names_the_known_set ... ok
test service_roles::declaring_default_does_not_give_a_multi_service_provider_an_implicit_service_back ... ok
test service_roles::only_an_operation_fills_a_role_slot_and_an_event_does_not ... ok
test service_roles::the_reserved_default_service_may_carry_roles ... ok
test service_tags::a_provider_level_tags_key_is_refused ... ok
test service_tags::a_providers_tags_are_the_union_of_its_services ... ok
test service_partition::a_provider_file_that_loads_publishes_only_round_tripping_addresses ... ok
test service_tags::a_repeated_tag_on_one_service_is_refused ... ok
test service_tags::a_service_may_carry_several_tags ... ok
test service_tags::an_unknown_tag_is_refused_and_names_the_known_set ... ok
test operation_spec_source::mixed_zendesk_services_classify_each_operation_instead_of_the_service ... ok
test services::a_declared_service_reaches_the_ir ... ok
test services::a_default_only_connector_hashes_no_service_fields ... ok
test services::a_duplicate_service_declaration_is_refused ... ok
test services::a_single_service_provider_encodes_no_service_at_all ... ok
test services::an_api_version_carrying_a_separator_is_refused ... ok
test services::an_empty_service_name_is_refused ... ok
test services::an_operation_naming_an_undeclared_service_is_refused_and_the_error_lists_what_exists ... ok
test services::an_unspellable_authority_is_refused ... ok
test services::an_unspellable_service_name_is_refused ... ok
test services::declaring_the_reserved_default_service_is_refused ... ok
test services::every_service_field_is_inside_the_hash_domain ... ok
test auth_workarounds::a_workaround_declared_on_one_connectors_auth_surface_does_not_reach_another ... ok
test services::omitting_the_service_in_a_multi_service_provider_is_refused ... ok
test services::stating_the_default_service_encodes_exactly_as_omitting_it ... ok
test services::the_service_key_walk_still_finds_an_ir_service_key_at_every_depth ... ok
test shared_endpoint_slot::a_sibling_service_placeholder_is_refused_when_nothing_shares_the_slot ... ok
test shared_endpoint_slot::a_sibling_service_that_does_not_exist_is_refused ... ok
test shared_endpoint_slot::listing_the_head_service_again_is_refused ... ok
test shared_endpoint_slot::one_field_fills_the_placeholder_of_a_sibling_service ... ok
test shared_endpoint_slot::sharing_a_non_endpoint_binding_is_refused ... ok
test auth_archetypes::and_sets_and_or_alternatives_are_the_grouping_a_form_renders ... ok
test operation_selection::the_canonical_surface_is_selected_and_the_file_stays_reviewable ... ok
test shipped_providers::babelforce_is_bearer_only_and_never_the_deprecated_header_pair ... ok
test shipped_providers::an_operation_that_takes_nothing_composes_an_empty_object_schema ... ok
test shipped_providers::jira_added_writes_have_exact_authored_source_evidence ... ok
test operation_spec_source::a_patch_selected_operation_retains_its_exact_vendor_source ... ok
test config_fields::no_shipped_provider_gives_a_secret_field_an_example ... ok
test shipped_providers::operation_selection_stays_curated ... ok
test shipped_providers::zendesk_declares_the_token_suffix_rather_than_pre_composing_it ... ok
test spec_backed_provider::a_declared_spec_hash_that_disagrees_with_the_document_is_refused ... ok
test spec_backed_provider::a_document_joining_an_undeclared_service_is_refused ... ok
test spec_backed_provider::a_document_that_cannot_be_ingested_fails_the_provider_naming_the_spec_path ... ok
test spec_backed_provider::a_hand_authored_file_ignores_a_document_supplied_beside_it ... ok
test spec_backed_provider::a_parameter_correction_that_matches_is_applied ... ok
test spec_backed_provider::a_parameter_correction_that_matches_nothing_is_refused ... ok
test spec_backed_provider::a_patch_naming_a_service_no_document_declares_is_refused ... ok
test spec_backed_provider::a_patch_that_names_no_service_is_refused_when_several_documents_are_declared ... ok
test spec_backed_provider::a_pin_that_resolves_to_nothing_is_refused_and_names_the_cache ... ok
test spec_backed_provider::a_pin_that_resolves_to_nothing_names_only_that_document ... ok
test spec_backed_provider::a_select_that_names_no_operation_is_refused_and_suggests_the_near_misses ... ok
test spec_backed_provider::a_selected_operation_carries_the_documents_request_and_the_authors_judgement ... ok
test spec_backed_provider::a_selected_operation_is_held_to_every_rule_an_inline_one_is ... ok
test spec_backed_provider::a_selection_that_states_no_rename_is_refused_rather_than_taking_the_operation_id ... ok
test spec_backed_provider::a_selection_that_states_no_risk_or_idempotency_is_refused ... ok
test spec_backed_provider::a_single_spec_table_and_a_one_entry_spec_array_compile_identically ... ok
test spec_backed_provider::a_spec_backed_provider_with_no_patch_publishes_nothing ... ok
test spec_backed_provider::an_exact_asyncapi_event_patch_carries_stable_name_schema_and_scoped_auth ... ok
test spec_backed_provider::an_operation_the_ingest_skipped_cannot_be_selected ... ok
test spec_backed_provider::an_unknown_key_in_a_spec_block_is_still_named_in_both_spellings ... ok
test spec_backed_provider::asyncapi_ingest_makes_messages_available_but_selects_none ... ok
test auth_archetypes::every_oauth_connector_generates_the_operator_connection_split ... ok
test spec_backed_provider::each_document_carries_its_own_provenance_and_its_own_hash_is_checked ... ok
test spec_backed_provider::inline_operations_and_selected_ones_land_in_one_connector ... ok
test spec_backed_provider::everything_the_document_declares_stays_available_to_patch ... ok
test spec_backed_provider::loading_a_spec_backed_provider_is_deterministic ... ok
test spec_backed_provider::one_documents_security_does_not_overwrite_the_others ... ok
test spec_backed_provider::one_operation_id_in_two_documents_is_two_operations ... ok
test spec_backed_provider::plain_load_refuses_a_spec_backed_file_rather_than_returning_a_skeleton ... ok
test spec_backed_provider::selecting_one_operation_twice_from_one_document_is_still_refused ... ok
test spec_backed_provider::the_pinned_document_is_compiled_even_when_a_later_one_sits_beside_it ... ok
test spec_backed_provider::two_documents_may_not_join_one_service ... ok
test strict_fields::a_credential_must_declare_its_scheme ... ok
test strict_fields::a_typoed_credential_env_key_is_rejected ... ok
test strict_fields::a_typoed_operation_auth_key_is_rejected ... ok
test strict_fields::an_operation_must_declare_its_vendor_state_direction ... ok
test strict_fields::strictness_does_not_break_the_ir_round_trip ... ok
test strict_fields::typoed_keys_are_rejected_at_every_nesting_depth ... ok
test vendored_specs::a_provenance_entry_is_spec_source_shaped_and_names_no_internal_url ... ok
test spec_backed_provider::several_documents_each_become_one_service ... ok
test vendored_specs::every_url_in_a_vendored_document_points_at_a_public_host ... ok
test vendored_specs::no_credential_shaped_example_value_survives ... ok
test vendored_specs::no_internal_marker_survives_in_a_vendored_document ... ok
test vendored_specs::no_pull_configuration_is_vendored ... ok
test vendored_specs::no_personal_identity_survives_in_a_vendored_document ... ok
test vendored_specs::provenance_records_every_vendored_document_and_its_hash_matches ... ok
test credential_paths::every_shipped_provider_declares_an_authority_and_renders_a_credential_path ... ok
test vendored_specs::the_five_babelforce_documents_are_vendored ... ok
test verification_conformance::a_body_sourced_verification_timestamp_does_not_load ... ok
test vendored_specs::no_scrubbed_literal_can_ever_reappear ... ok
test vendored_specs::the_declarations_survive_the_scrub ... ok
test verification_conformance::a_signed_template_that_covers_only_the_url_verifies_a_forged_payload ... ok
test verification_conformance::a_signed_template_that_omits_the_body_verifies_a_forged_payload ... ok
test verification_conformance::a_transport_outside_the_matrix_declares_that_it_cannot_verify ... ok
test verification_conformance::comparison_examines_every_byte_wherever_they_differ ... ok
test service_audiences::the_seeded_fleet_has_a_useful_cross_function_vocabulary ... ok
test verification_conformance::reserializing_the_json_body_breaks_verification ... ok
test verification_conformance::the_declared_timestamp_format_is_read_instead_of_sniffed ... ok
test verification_conformance::the_hmac_primitive_matches_rfc_4231 ... ok
test verification_conformance::the_hmac_sha1_primitive_matches_rfc_2202 ... ok
test verification_conformance::the_reassembled_form_is_a_derivation_of_the_body_and_not_its_bytes ... ok
test verification_conformance::a_signature_outside_its_window_is_refused ... ok
test credential_paths::every_shipped_credential_is_prefixed_with_its_connector_id ... ok
test credential_response::no_withheld_operation_is_in_the_shipped_catalogue ... ok
test response_schema_coverage::the_recorded_floor_is_the_measured_figure ... ok
test config_fields::no_shipped_provider_has_an_unbound_template_variable ... ok
test verification_conformance::a_tolerance_no_host_could_apply_does_not_load ... ok
test shipped_providers::every_shipped_provider_loads ... ok
test services::every_shipped_service_is_spellable_and_a_single_service_provider_declares_none ... ok
test response_schema_coverage::the_recorded_ceiling_is_the_measured_absence ... ok
test response_schema_coverage::no_operation_publishes_a_permissive_response_schema ... ok
test produces_credential::no_shipped_operation_declares_produces_credential_yet ... ok
test shipped_providers::every_operation_composes_an_input_schema_covering_its_parameters ... ok
test shipped_providers::no_provider_file_carries_a_credential_value ... ok
test service_tags::the_shipped_fleet_uses_several_distinct_tags ... ok
test response_schema_coverage::response_schema_coverage_does_not_fall_below_its_floor ... ok
test response_schema_coverage::a_connector_arriving_with_no_response_shapes_is_caught ... ok
test shipped_providers::operation_ids_are_declarable_in_flux ... ok
test service_tags::some_shipped_provider_has_services_whose_tags_diverge ... ok
test verification_conformance::vendor_signature_vectors_verify ... ok
test verification_conformance::every_shipped_hmac_scheme_is_covered_by_the_matrix ... ok

test result: ok. 517 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 41.84s

     Running unittests src/lib.rs (/dev/shm/b10x-cli-wave-wt-4f1de73d0685-nr00xjox/runtime-target/debug/deps/connectors_client-40f8b4d95ea43095)

running 37 tests
test identity::tests::hosted_request_families_select_the_smallest_available_scope ... ok
test identity::tests::keyring_account_contains_no_endpoint_or_principal ... ok
test identity::tests::mcp_invocation_uses_only_the_invoke_scope ... ok
test personal_oauth::personal_oauth_tests::personal_instruction_destination_is_exact_numeric_loopback_and_capability_is_header_only ... ok
test personal_oauth::personal_oauth_tests::personal_instruction_parser_preserves_optional_device_uri_and_refuses_origin_changes ... ok
test hosted_catalog::tests::posts_and_validates_a_catalog_frame ... ok
test personal_oauth::personal_oauth_tests::actual_private_instruction_request_keeps_capability_out_of_url_and_delivers_only_to_human_writer ... ok
test personal_oauth::personal_oauth_tests::personal_create_refusal_cannot_echo_private_daemon_text ... ok
test personal_oauth::personal_oauth_tests::actual_instruction_redirect_and_cacheable_reply_are_closed_refusals ... ok
test admin::tests::named_resources_are_typed_and_the_value_is_not_exposed ... ok
test personal_oauth::personal_oauth_tests::private_browser_authorization_is_never_a_provider_independent_redirect ... ok
test admin::tests::identity_pkce_exchange_returns_only_the_exact_access_credential ... ok
test personal_oauth::personal_oauth_tests::personal_poll_and_describe_refusals_close_inner_transport_and_daemon_text ... ok
test personal_oauth::personal_oauth_tests::valid_browser_only_session_reaches_the_explicit_personal_handoff ... ok
test personal_oauth::personal_oauth_tests::personal_success_refuses_another_session ... ok
test personal_oauth::personal_oauth_tests::personal_success_refuses_a_private_target_repeated_consistently ... ok
test personal_oauth::personal_oauth_tests::personal_success_refuses_another_credential_purpose ... ok
test tests::hosted_client_requires_https_except_on_loopback_or_internal_cluster_dns ... ok
test personal_oauth::personal_oauth_tests::expired_monotonic_instruction_budget_never_connects_even_with_future_wall_deadline ... ok
test tests::completion_endpoint_is_validated_before_secret_submission ... ok
test git_fetch_client::tests::response_is_bound_to_the_request_and_source_authority_is_redacted ... ok
test tests::hosted_client_posts_and_validates_a_datasource_frame ... ok
test tests::hosted_client_posts_the_same_typed_operation_frame ... ok
test personal_oauth::personal_oauth_tests::personal_success_refuses_a_changed_deadline ... ok
test tests::local_client_frames_and_correlates_an_operation ... ok
test personal_oauth::personal_oauth_tests::personal_success_refuses_degraded_description ... ok
test personal_oauth::personal_oauth_tests::personal_success_refuses_created_description ... ok
test personal_oauth::personal_oauth_tests::personal_success_retains_a_callable_correlated_description ... ok
test tests::hosted_subscription_client_refuses_a_cacheable_credential_boundary ... ok
test tests::hosted_subscription_client_redacts_and_redeems_one_attempt_capability ... ok
test tests::hosted_subscription_client_starts_and_completes_pkce_without_retaining_the_code ... ok
test personal_oauth::personal_oauth_tests::personal_success_refuses_another_describe_target ... ok
test personal_oauth::personal_oauth_tests::personal_success_refuses_another_integration_status ... ok
test response::tests::rate_stage2_hosted_client_never_resends_after_any_received_refusal_or_invalid_reply ... ok
test identity::tests::login_separates_the_session_and_refreshes_exact_scope_tokens ... ok
test personal_oauth::personal_oauth_tests::actual_connection_v1_polling_keeps_guarded_completion_private_and_never_repeats_create ... ok
test response::tests::rate_stage2_local_client_never_resends_after_any_received_refusal_or_invalid_reply ... ok

test result: ok. 37 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.32s

     Running tests/personal_oauth_adversary.rs (/dev/shm/b10x-cli-wave-wt-4f1de73d0685-nr00xjox/runtime-target/debug/deps/personal_oauth_adversary-381db7932e982540)

running 2 tests
test oauth_pass1_public_instruction_refusals_do_not_redirect_poll_or_echo_private_bytes ... ok
test oauth_pass1_public_handoff_waits_for_bound_callable_success_without_replay ... ok

test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.52s

     Running unittests src/lib.rs (/dev/shm/b10x-cli-wave-wt-4f1de73d0685-nr00xjox/runtime-target/debug/deps/protocol-97038e98e9ec497b)

running 37 tests
test approval::tests::approval_lifetime_is_bounded ... ok
test audio::tests::the_input_refuses_any_field_a_caller_invents ... ok
test approval::tests::realm_is_not_an_approval_or_route_coordinate ... ok
test audio::tests::empty_control_bearing_and_over_length_text_refuse ... ok
test browser::tests::a_page_view_cannot_be_built_without_its_untrusted_content_label ... ok
test audio::tests::bounded_single_line_text_is_admitted_and_counted_in_characters ... ok
test browser::tests::open_admits_an_absent_address_and_goto_does_not ... ok
test browser::tests::only_ordinary_web_addresses_are_admitted ... ok
test browser::tests::the_admitted_surface_carries_no_interaction_operation ... ok
test browser::tests::the_input_refuses_any_field_a_caller_invents ... ok
test catalog::tests::catalog_membership_carries_no_callability_or_credential_value ... ok
test catalog::tests::a_setup_profile_cannot_exist_without_a_setup_form ... ok
test connection::tests::materialization_accepts_only_an_opaque_observation_reference ... ok
test connection::tests::browser_completion_url_is_an_exact_loopback_capability ... ok
test connection::tests::mediated_route_is_value_free_closed_and_cannot_self_reference ... ok
test connection::tests::observations_are_value_free_and_lifecycle_consistent ... ok
test connection::tests::candidates_are_value_free_and_activation_selects_no_route ... ok
test connection::tests::secret_shaped_unknown_fields_are_refused ... ok
test connection::tests::pending_and_completed_sessions_cannot_mix_endpoint_and_connection ... ok
test datasource::tests::list_is_bounded_and_get_key_is_structured ... ok
test datasource::tests::response_refuses_ambiguous_success_and_failure ... ok
test event::tests::cursor_and_wait_are_bounded ... ok
test git_fetch::tests::response_refuses_secret_or_non_tls_locators ... ok
test operation::legacy::tests::a_terminal_status_cannot_omit_its_observed_reason ... ok
test git_fetch::tests::request_refuses_unbounded_or_ambiguous_revisions ... ok
test operation::legacy::tests::effect_bearing_operations_require_approval ... ok
test operation::legacy::tests::connection_audiences_are_bounded_discovery_metadata ... ok
test operation::legacy::tests::invoke_requires_a_description_lease_and_bounded_structured_input ... ok
test operation::legacy::tests::owner_context_is_not_defaultable ... ok
test sip::tests::a_number_alone_is_admitted_because_the_trunk_supplies_the_destination ... ok
test sip::tests::a_number_is_bounded_rather_than_truncated ... ok
test sip::tests::a_number_that_could_escape_the_uri_user_part_is_refused ... ok
test operation::legacy::tests::response_envelope_round_trips_and_refuses_unknown_fields ... ok
test sip::tests::aliases_admit_names_and_refuse_network_destinations ... ok
test sip::tests::an_absent_field_is_omitted_from_the_wire_rather_than_sent_as_null ... ok
test voice::tests::fixture_context_cannot_claim_trust ... ok
test voice::tests::owner_vectors_are_closed_and_unique ... ok

test result: ok. 37 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests/bundles.rs (/dev/shm/b10x-cli-wave-wt-4f1de73d0685-nr00xjox/runtime-target/debug/deps/bundles-f0b59fa93166e973)

running 25 tests
test connector_catalog_bundle_is_immutable ... ok
test connector_connection_bundle_is_immutable ... ok
test connector_catalog_vectors_match_the_strict_reader ... ok
test connector_connection_vectors_match_the_strict_reader ... ok
test connector_datasource_bundle_is_immutable ... ok
test connector_event_vectors_match_the_strict_reader ... ok
test connector_event_bundle_is_immutable ... ok
test connector_datasource_vectors_match_the_strict_reader ... ok
test kubernetes_service_route_round_trips_through_the_connection_response ... ok
test connector_operation_bundle_is_immutable ... ok
test connector_operation_vectors_match_the_strict_reader ... ok
test operation_v2_advice_preserves_all_categories_and_checks_the_interval ... ok
test operation_v2_projects_throttling_to_v1_without_optional_extensions ... ok
test operation_v2_only_rate_limited_carries_retry_delay ... ok
test owner_contract_bundle_is_immutable ... ok
test rtvbp_binding_bundle_is_immutable ... ok
test operation_version_reader_rejects_unknown_versions_and_preserves_authority_fields ... ok
test connector_operation_v2_bundle_is_immutable ... ok
test operation_version_decoder_refuses_duplicate_fields_before_dispatch ... ok
test rate_repair_source_uri_grammar_matches_wire_and_published_schema ... ok
test operation_v2_description_downgrade_loses_only_advice ... ok
test operation_v2_vectors_cover_every_request_result_and_error_variant ... ok
test operation_legacy_snapshot_preserves_deployed_schema_discrepancies ... ok
test operation_v2_complete_request_and_response_vectors_match_rust ... ok
test operation_v2_complete_request_and_response_vectors_match_schema ... ok

test result: ok. 25 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s

     Running tests/rate_adversary.rs (/dev/shm/b10x-cli-wave-wt-4f1de73d0685-nr00xjox/runtime-target/debug/deps/rate_adversary-735069fd71edcb71)

running 2 tests
test rate_adversary_published_schema_matches_source_url_reader ... ok
test rate_final_uri_composition_and_downgrade_validate_complete_frames ... ok

test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s

     Running unittests src/lib.rs (/dev/shm/b10x-cli-wave-wt-4f1de73d0685-nr00xjox/runtime-target/debug/deps/server-dd82a5540a2e86b9)

running 101 tests
test egress::tests::ambiguous_retry_after_is_not_flattened_into_advice ... ok
test egress::tests::egress_requires_a_nonempty_ascii_connection_or_session_reference ... ok
test egress::tests::cached_client_cannot_bypass_current_address_policy ... ok
test egress::tests::ipv4_mapped_ipv6_cannot_bypass_address_classification ... ok
test egress::tests::malformed_retry_after_does_not_hide_the_definite_provider_response ... ok
test egress::tests::exact_origin_cannot_be_widened_by_path_host_or_userinfo ... ok
test egress::tests::operator_network_may_admit_private_but_not_process_local_addresses ... ok
test egress::tests::public_dns_refuses_private_local_reserved_and_mixed_answers ... ok
test egress::tests::retry_after_extraction_keeps_only_one_valid_decimal_and_admitted_headers ... ok
test egress::tests::suffix_rule_requires_a_real_child_and_the_exact_scheme_and_port ... ok
test hosted::enforcement::tests::the_canonical_digest_ignores_member_order_and_nothing_else ... ok
test hosted::principal::tests::lease_seeds_survive_the_verifier_token_rotation_composition ... ok
test hosted::enforcement::tests::an_issued_record_round_trips_without_its_reference_in_any_key ... ok
test hosted::git_fetch::tests::rejected_control_identity_is_decided_before_the_request_body_is_polled ... ok
test hosted::git_fetch::tests::rejected_source_authority_is_decided_before_body_or_broker_exchange ... ok
test egress::tests::reused_connection_keeps_authorization_and_timeout_request_specific ... ok
test hosted::git_fetch::tests::control_and_internal_routes_are_separate_and_non_cacheable ... ok
test hosted::git_fetch::tests::ambiguous_protocol_or_source_headers_are_refused_before_reading_the_body ... ok
test hosted::tests::admin_routes::operator_group_without_the_exact_scope_cannot_write ... ok
test egress::tests::rate_stage2_definite_http_429_survives_oversized_or_broken_error_bodies ... ok
test hosted::tests::admin_routes::auth_metadata_is_public_and_selects_exact_authority ... ok
test hosted::tests::contract_validation::hosted_route_refuses_a_malformed_backend_contract ... ok
test hosted::tests::admin_routes::operator_can_write_and_status_never_returns_the_secret ... ok
test hosted::tests::contract_validation::rate_stage2_hosted_boundary_selects_response_version_even_for_early_refusals ... ok
test hosted::tests::contract_validation::rate_stage2_hosted_http_projects_refusals_once_and_rejects_invalid_frames_before_backend ... ok
test hosted::tests::a_human_issues_one_exact_input_approval_which_is_spent_once ... ok
test egress::rate_adversary_tests::rate_final_chunked_429_keeps_definite_status_without_body_or_untrusted_advice ... ok
test egress::tests::pool_is_bounded_and_separates_current_addresses_authorities_origins_and_policy ... ok
test hosted::tests::contract_validation::rate_stage2_invalid_http_correlation_is_a_bounded_versioned_client_refusal ... ok
test hosted::tests::contract_validation::rate_final_hosted_describe_versions_reject_bad_advice_before_loss ... ok
test hosted::tests::enforcement::a_granted_effect_without_approval_demand_dispatches_on_the_grant_alone ... ok
test hosted::tests::enforcement::a_granted_mutation_demanding_approval_refuses_without_one ... ok
test egress::rate_adversary_tests::rate_adversary_header_cardinality_and_unfinished_body_are_separate ... ok
test hosted::tests::enforcement::a_granted_mutation_with_a_demanded_approval_dispatches_with_one ... ok
test hosted::tests::enforcement::a_mutation_with_no_admitting_grant_refuses ... ok
test hosted::tests::enforcement::an_unbound_grant_store_is_an_outage_for_effects_only ... ok
test hosted::tests::enforcement::a_second_presentation_of_the_same_approval_refuses_and_journals_replay ... ok
test hosted::tests::hosted_completion_failures_are_non_cacheable_and_browser_hardened ... ok
test hosted::tests::hosted_completion_streams_fragments_into_a_redacted_bounded_submission ... ok
test hosted::tests::enforcement::the_read_only_path_is_unchanged_for_callers_without_grants ... ok
test hosted::tests::hosted_connection_route_uses_the_same_identity_boundary ... ok
test hosted::tests::hosted_datasource_route_passes_verified_groups_and_exact_tenant ... ok
test hosted::tests::contract_validation::rate_adversary_hosted_grant_and_approval_refusals_keep_requested_version ... ok
test hosted::tests::hosted_route_requires_identity_and_exact_tenant_binding ... ok
test hosted::tests::enforcement::every_enforcement_refusal_renders_the_same_bytes ... ok
test hosted::tests::mcp::a_busy_namespace_is_listed_whole_without_a_paging_surface ... ok
test hosted::tests::mcp::a_stale_authority_refusal_is_retried_exactly_once_with_a_fresh_lease ... ok
test hosted::tests::mcp::an_invoke_without_the_invoke_scope_surfaces_not_granted ... ok
test hosted::tests::mcp::an_op_backed_invoke_describes_then_invokes_with_the_fresh_lease ... ok
test hosted::tests::mcp::approval_demand_and_evidence_pass_through_the_admission_seam ... ok
test hosted::tests::docs::openapi_json_is_served_verbatim_with_a_content_hash_etag ... ok
test hosted::tests::docs::the_document_pins_the_exact_wire_contract_identities_and_audience ... ok
test hosted::tests::mcp::initialize_echoes_admitted_revisions_and_answers_ping ... ok
test hosted::tests::docs::a_request_example_with_an_unknown_field_is_refused ... ok
test hosted::tests::mcp::datasource_backed_tools_route_through_the_datasource_seam ... ok
test hosted::tests::docs::every_documented_mcp_request_example_is_answered_by_the_live_transport ... ok
test hosted::tests::mcp::the_mcp_route_is_stateless_post_only ... ok
test hosted::tests::docs::the_docs_page_is_served_unauthenticated_as_html ... ok
test hosted::tests::mcp::tools_list_returns_exactly_the_three_meta_tools ... ok
test hosted::tests::docs::every_documented_refusal_example_names_a_real_error_code ... ok
test hosted::tests::docs::the_docs_page_links_the_contract_and_renders_its_version ... ok
test hosted::tests::mcp::tool_describe_projects_the_underlying_description_without_a_lease ... ok
test hosted::tests::mcp::tool_search_projects_only_the_entries_the_callers_seam_results_support ... ok
test hosted::tests::mcp::rate_stage2_mcp_preserves_refusal_details_without_entering_stale_retry ... ok
test hosted::tests::docs::the_docs_page_makes_zero_external_requests ... ok
test hosted::tests::mcp_monitoring::a_monitoring_invoke_refuses_dishonest_targets_before_any_dispatch ... ok
test hosted::tests::monitoring_transport_gate_admits_only_configured_groups_or_operator ... ok
test hosted::tests::docs::the_docs_page_refusal_table_carries_every_documented_code ... ok
test hosted::tests::pod_log_transport_gate_admits_only_kubernetes_read_groups_or_operator ... ok
test hosted::tests::docs::every_docs_page_example_is_the_documents_example_after_json_normalization ... ok
test hosted::tests::mcp::rate_adversary_mcp_stale_then_rate_stops_without_losing_large_delay ... ok
test hosted::tests::self_event_scope_is_closed_to_slack_specific_requests ... ok
test hosted::tests::mcp_monitoring::a_stale_monitoring_invoke_re_resolves_the_same_target_exactly_once ... ok
test hosted::tests::docs::every_documented_request_example_is_accepted_by_its_protocol_type ... ok
test hosted::tests::production_router_publishes_client_discovery_without_authentication ... ok
test hosted::tests::mcp_monitoring::a_monitoring_invoke_routes_the_chosen_target_through_the_decided_seam ... ok
test hosted::tests::tenant_members_receive_only_read_only_module_invocation ... ok
test hosted::tests::signal::an_effect_bearing_session_signal_without_an_admitting_grant_refuses ... ok
test hosted::tests::mcp::transport_refusals_carry_the_designed_statuses_and_codes ... ok
test hosted::tests::subscription_oauth_start_is_identity_scoped_bounded_and_non_cacheable ... ok
test local::tests::a_broad_state_directory_refuses_without_repair ... ok
test hosted::tests::signal::a_granted_session_signal_dispatches_behind_the_sessions_grant ... ok
test hosted::tests::mcp_monitoring::the_acceptance_sequence_invokes_with_a_target_and_integer_epochs ... ok
test local::tests::one_socket_dispatches_the_value_free_connection_and_event_contracts ... ok
test local::tests::a_second_daemon_cannot_unlink_the_live_daemons_socket ... ok
test hosted::tests::signal::an_unbound_grant_store_is_an_outage_for_session_signals ... ok
test hosted::tests::subscription_credential_stays_in_custody_and_only_an_exact_lease_redeems ... ok
test local::tests::owner_socket_serves_one_strict_bounded_operation_frame ... ok
test local::tests::rate_stage2_actual_socket_serves_both_versions_without_resending ... ok
test hosted::tests::signal::a_signal_refusal_matches_the_invoke_refusal_bytes ... ok
test hosted::tests::mcp_monitoring::tool_search_lists_the_monitoring_tools_for_a_monitoring_read_principal ... ok
test hosted::tests::mcp_monitoring::tool_describe_enumerates_the_callers_configured_targets_without_a_lease ... ok
test hosted::tests::mcp_monitoring::monitoring_tool_schemas_state_the_documents_contract ... ok
test hosted::tests::mcp::a_pathological_namespace_is_cut_with_an_explicit_truncation_marker ... ok
test catalog_projection::tests::a_deployment_publishes_only_the_setup_flows_it_can_complete ... ok
test catalog_projection::tests::search_is_whole_catalog_and_describe_is_descriptive_only ... ok
test catalog_projection::tests::platform_provider_satisfies_the_catalog_wire_contract ... ok
test catalog_projection::tests::every_shipped_provider_description_satisfies_the_catalog_wire_contract ... ok
test hosted::tests::hosted_liveness_and_identity_backed_readiness_are_distinct ... ok
test hosted::tests::hosted_liveness_stays_local_when_a_backend_dependency_is_unready ... ok
test hosted::tests::docs::every_documented_route_exists_in_the_real_router ... ok

test result: ok. 101 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.89s

     Running tests/rate_adversary_local.rs (/dev/shm/b10x-cli-wave-wt-4f1de73d0685-nr00xjox/runtime-target/debug/deps/rate_adversary_local-e9ec62baa6625a53)

running 2 tests
test rate_final_local_describe_validates_before_version_loss ... ok
test rate_adversary_local_versions_validate_before_single_dispatch ... ok

test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running unittests src/lib.rs (/dev/shm/b10x-cli-wave-wt-4f1de73d0685-nr00xjox/runtime-target/debug/deps/service-3731ef93ea80f9c6)

running 64 tests
test audio::tests::a_route_for_another_connection_is_refused ... ok
test audio::tests::a_plan_for_another_driver_or_operation_is_refused ... ok
test admin::tests::status_reports_presence_without_reading_the_value ... ok
test audio::tests::a_speech_plan_and_its_deployment_route_admit_together ... ok
test admin::tests::write_requires_explicit_replacement_and_audits_no_secret ... ok
test audio::tests::relative_paths_absent_bounds_and_bad_digests_never_reach_the_device ... ok
test audio::tests::admitted_evidence_never_prints_as_a_serializable_route_secret ... ok
test audio::tests::status_never_admits_an_utterance ... ok
test audio::tests::the_caller_text_is_bounded_by_the_deployment_and_not_only_by_the_catalog ... ok
test authority::tests::debug_never_prints_authority_or_proof ... ok
test authority::tests::proof_is_bound_to_exact_upgrade_uri ... ok
test browser::tests::a_non_web_address_is_refused_before_any_browser_is_touched ... ok
test browser::tests::a_plan_for_another_driver_or_operation_is_refused ... ok
test browser::tests::a_route_for_another_connection_is_refused ... ok
test browser::tests::a_unary_lifecycle_is_refused_because_a_browser_spans_calls ... ok
test browser::tests::admitted_evidence_never_prints_as_a_serializable_route_secret ... ok
test authority::tests::session_lease_cannot_be_extended_by_authority_clock_skew ... ok
test browser::tests::every_admitted_browser_operation_and_its_route_admit_together ... ok
test browser::tests::the_operators_own_browser_profile_is_never_an_admitted_route ... ok
test browser::tests::only_open_may_omit_an_address_and_only_open_or_goto_may_carry_one ... ok
test authority::tests::serving_endpoint_redeems_once ... ok
test browser::tests::relative_paths_nested_artifacts_and_absent_bounds_never_reach_the_browser ... ok
test connect_session::tests::a_completion_claim_blocks_expiry_shutdown_and_duplicate_completion ... ok
test connect_session::tests::authority_and_claim_use_one_receiver_owned_instant ... ok
test connect_session::tests::capacity_counts_only_pending_sessions ... ok
test connect_session::tests::clock_failure_refuses_authorization_but_does_not_prevent_confirmed_abort ... ok
test connect_session::tests::preparing_and_uncertain_abort_cannot_publish_a_terminal_result ... ok
test connect_session::tests::shutdown_fails_only_pending_sessions_and_returns_their_endpoints ... ok
test connect_session::tests::terminal_transition_is_one_way_and_value_free ... ok
test connect_session::tests::the_claim_rechecks_the_original_target_and_inclusive_deadline_once ... ok
test connect_session::tests::guarded_sessions_keep_capacity_until_a_confirmed_outcome ... ok
test dispatch::tests::composition_order_is_policy_redaction_audit_driver_audit ... ok
test egress::tests::retry_delay_is_one_unsigned_decimal_with_only_http_whitespace ... ok
test authority::tests::audience_expiry_and_revocation_fail_before_redemption ... ok
test runtime::tests::delegated_execution_must_match_the_authenticated_actor ... ok
test runtime::tests::hosted_completion_submission_never_reallocates_received_secret_material ... ok
test planning::tests::provider_only_connection_refuses_a_caller_initiated_operation ... ok
test planning::tests::bidirectional_connection_still_requires_the_operation_admission ... ok
test planning::tests::mediated_http_plan_has_no_direct_origin_and_requires_the_closed_adapter ... ok
test planning::tests::sip_plan_has_no_http_fields ... ok
test runtime::tests::hosted_principals_do_not_fabricate_agent_revisions ... ok
test planning::tests::missing_driver_refuses_before_dispatch ... ok
test runtime::tests::local_principals_require_a_real_agent_revision ... ok
test runtime::tests::the_stable_authority_seed_distinguishes_absent_and_literal_default_realms ... ok
test runtime::tests::the_stable_authority_seed_survives_token_scoped_snapshot_fields ... ok
test runtime::tests::the_stable_authority_seed_ignores_request_scoped_provenance ... ok
test sip::tests::a_default_naming_an_absent_trunk_is_refused_when_the_table_is_built ... ok
test sip::tests::a_dial_with_no_alias_and_no_default_is_refused_rather_than_guessed ... ok
test sip::tests::a_partial_byte_prefix_masks_only_the_bits_it_declares ... ok
test sip::tests::a_named_trunk_is_resolved_before_admission_so_the_answer_is_aperture_checked ... ok
test sip::tests::a_prefix_aperture_admits_its_network_and_refuses_outside_it ... ok
test sip::tests::a_prefix_that_is_not_on_its_own_boundary_is_refused_rather_than_rounded ... ok
test sip::tests::a_dial_with_only_a_number_takes_the_declared_default_trunk ... ok
test sip::tests::a_trunk_that_does_not_admit_numbers_refuses_one ... ok
test sip::tests::an_aperture_never_matches_across_address_families ... ok
test sip::tests::an_exact_aperture_still_admits_exactly_one_address ... ok
test sip::tests::exact_loopback_route_produces_non_serializable_driver_evidence ... ok
test sip::tests::missing_organization_in_grant_evidence_refuses_before_the_driver ... ok
test sip::tests::sip_dial_resolves_only_an_exact_connection_owned_alias ... ok
test sip::tests::stable_network_and_aperture_widening_refuse_before_the_driver ... ok
test sip::tests::the_dialled_host_comes_from_the_trunk_and_never_from_the_caller ... ok
test sip::tests::zero_or_excessive_dial_deadlines_refuse_before_the_driver ... ok
test voice::tests::one_proof_joins_exact_sip_and_application_routes ... ok
test voice::tests::invalid_endpoint_and_authority_windows_refuse_before_io ... ok

test result: ok. 64 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

   Doc-tests catalog

running 1 test
test crates/catalog/src/lib.rs - (line 9) ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.98s

   Doc-tests catalog_build

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

   Doc-tests catalog_reader

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

   Doc-tests connector_oauth

running 3 tests
test crates/connector-oauth/src/device.rs - device::DeviceAuthorization (line 40) - compile fail ... ok
test crates/connector-oauth/src/device.rs - device::DeviceResponse (line 8) - compile fail ... ok
test crates/connector-oauth/src/device.rs - device::DevicePoll (line 164) - compile fail ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s

   Doc-tests connector_resolve

running 1 test
test crates/connector-resolve/src/lib.rs - (line 3) - compile ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s

   Doc-tests connector_spec

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

   Doc-tests connectors_client

running 2 tests
test crates/connectors-client/src/model.rs - model::PendingPersonalOAuth (line 312) - compile fail ... ok
test crates/connectors-client/src/model.rs - model::PersonalOAuthInstructions (line 338) - compile fail ... ok

test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.04s

   Doc-tests protocol

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

   Doc-tests server

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

   Doc-tests service

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

oauth-correction1-root-clippy, cwd ~/.local/state/worktree/trees/b10x/connectors/wt-4f1de73d0685, exit0:
```text
env -u RUSTC_WRAPPER CARGO_TARGET_DIR=/dev/shm/b10x-cli-wave-wt-4f1de73d0685-nr00xjox/runtime-target TMPDIR=~/.cache/cw6/oc1 CARGO_BUILD_JOBS=1 CARGO_INCREMENTAL=0 CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 cargo clippy --locked --offline -p connector-spec -p connector-resolve -p catalog -p catalog-build -p catalog-reader -p connectors-client -p protocol -p server -p service -p connector-oauth --all-targets -- -D warnings
```
Complete raw output:
```text
    Checking connector-spec v0.6.5 (~/.local/state/worktree/trees/b10x/connectors/wt-4f1de73d0685/crates/connector-spec)
    Checking catalog-build v0.6.5 (~/.local/state/worktree/trees/b10x/connectors/wt-4f1de73d0685/crates/catalog-build)
    Finished `dev` profile [unoptimized] target(s) in 7.15s
```

oauth-correction1-root-fmt, cwd ~/.local/state/worktree/trees/b10x/connectors/wt-4f1de73d0685, exit0:
```text
env -u RUSTC_WRAPPER CARGO_TARGET_DIR=/dev/shm/b10x-cli-wave-wt-4f1de73d0685-nr00xjox/runtime-target TMPDIR=~/.cache/cw6/oc1 CARGO_BUILD_JOBS=1 CARGO_INCREMENTAL=0 CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 cargo fmt --all -- --check
```
Complete raw output:
```text

```

5. Preserved scope and remaining gates

No other source/test, dependency/lock, Git/AEP/ESS/model, operator/daemon/live-provider, cleanup or publication action was performed. The provider source/schema and catalog artifacts remain byte-exact; fixed-point and contract cases in the full suite passed without accepting generator drift. This authoring fix does not change runtime credential custody, routing, authority, client cancellation or protocol identities. Runtime/console/CLI first-pass checks remain separate historical evidence; no fresh execution is claimed for them. Independent second review and the twelve-workspace combined gate remain necessary before whole-story completion. This is a package-scoped green correction, not whole-story approval.

The final target is unchanged in location and preserved for coordinator disposition: 9013673984 per-path allocated bytes, 17779 files, 354 executable ELF hashes. Allocated-byte sums may count hardlink aliases separately. The final full source inventory was rehashed after target inspection. No other tree or target was touched.

6. Outside paths

Every persistent correction scratch path is listed below and in outside-paths.json. Original reports/runners were preserved; the existing guarded runner was copied as original-run-private.py before adapting only scratch path/prefix in run-private.py. The brief and assigned scratch/TMPDIR were supplied by the coordinator; their inclusion is custody inventory, not a claim that this implementor authored them.

~/.cache/connectors-cli-wave-20260906/connect-session-oauth-custody-in-personal-posture/whole-unit-correction-1/auth-after.rs
~/.cache/connectors-cli-wave-20260906/connect-session-oauth-custody-in-personal-posture/whole-unit-correction-1/before-provenance.json
~/.cache/connectors-cli-wave-20260906/connect-session-oauth-custody-in-personal-posture/whole-unit-correction-1/before-source-inventory.json
~/.cache/connectors-cli-wave-20260906/connect-session-oauth-custody-in-personal-posture/whole-unit-correction-1/brief.md
~/.cache/connectors-cli-wave-20260906/connect-session-oauth-custody-in-personal-posture/whole-unit-correction-1/class-enumeration.json
~/.cache/connectors-cli-wave-20260906/connect-session-oauth-custody-in-personal-posture/whole-unit-correction-1/commands-and-counts.json
~/.cache/connectors-cli-wave-20260906/connect-session-oauth-custody-in-personal-posture/whole-unit-correction-1/diff-check.log
~/.cache/connectors-cli-wave-20260906/connect-session-oauth-custody-in-personal-posture/whole-unit-correction-1/evidence-check.log
~/.cache/connectors-cli-wave-20260906/connect-session-oauth-custody-in-personal-posture/whole-unit-correction-1/evidence.sha256
~/.cache/connectors-cli-wave-20260906/connect-session-oauth-custody-in-personal-posture/whole-unit-correction-1/format-owner.command.json
~/.cache/connectors-cli-wave-20260906/connect-session-oauth-custody-in-personal-posture/whole-unit-correction-1/format-owner.log
~/.cache/connectors-cli-wave-20260906/connect-session-oauth-custody-in-personal-posture/whole-unit-correction-1/freeze.json
~/.cache/connectors-cli-wave-20260906/connect-session-oauth-custody-in-personal-posture/whole-unit-correction-1/head.txt
~/.cache/connectors-cli-wave-20260906/connect-session-oauth-custody-in-personal-posture/whole-unit-correction-1/initial-correction.patch
~/.cache/connectors-cli-wave-20260906/connect-session-oauth-custody-in-personal-posture/whole-unit-correction-1/oauth-correction1-root-clippy.command.json
~/.cache/connectors-cli-wave-20260906/connect-session-oauth-custody-in-personal-posture/whole-unit-correction-1/oauth-correction1-root-clippy.log
~/.cache/connectors-cli-wave-20260906/connect-session-oauth-custody-in-personal-posture/whole-unit-correction-1/oauth-correction1-root-clippy.resources.jsonl
~/.cache/connectors-cli-wave-20260906/connect-session-oauth-custody-in-personal-posture/whole-unit-correction-1/oauth-correction1-root-clippy.result.json
~/.cache/connectors-cli-wave-20260906/connect-session-oauth-custody-in-personal-posture/whole-unit-correction-1/oauth-correction1-root-clippy.warm-target.json
~/.cache/connectors-cli-wave-20260906/connect-session-oauth-custody-in-personal-posture/whole-unit-correction-1/oauth-correction1-root-fmt.command.json
~/.cache/connectors-cli-wave-20260906/connect-session-oauth-custody-in-personal-posture/whole-unit-correction-1/oauth-correction1-root-fmt.log
~/.cache/connectors-cli-wave-20260906/connect-session-oauth-custody-in-personal-posture/whole-unit-correction-1/oauth-correction1-root-fmt.resources.jsonl
~/.cache/connectors-cli-wave-20260906/connect-session-oauth-custody-in-personal-posture/whole-unit-correction-1/oauth-correction1-root-fmt.result.json
~/.cache/connectors-cli-wave-20260906/connect-session-oauth-custody-in-personal-posture/whole-unit-correction-1/oauth-correction1-root-fmt.warm-target.json
~/.cache/connectors-cli-wave-20260906/connect-session-oauth-custody-in-personal-posture/whole-unit-correction-1/oauth-correction1-root-full.command.json
~/.cache/connectors-cli-wave-20260906/connect-session-oauth-custody-in-personal-posture/whole-unit-correction-1/oauth-correction1-root-full.log
~/.cache/connectors-cli-wave-20260906/connect-session-oauth-custody-in-personal-posture/whole-unit-correction-1/oauth-correction1-root-full.resources.jsonl
~/.cache/connectors-cli-wave-20260906/connect-session-oauth-custody-in-personal-posture/whole-unit-correction-1/oauth-correction1-root-full.result.json
~/.cache/connectors-cli-wave-20260906/connect-session-oauth-custody-in-personal-posture/whole-unit-correction-1/oauth-correction1-root-full.warm-target.json
~/.cache/connectors-cli-wave-20260906/connect-session-oauth-custody-in-personal-posture/whole-unit-correction-1/oauth-correction1-spec-empty-after.command.json
~/.cache/connectors-cli-wave-20260906/connect-session-oauth-custody-in-personal-posture/whole-unit-correction-1/oauth-correction1-spec-empty-after.log
~/.cache/connectors-cli-wave-20260906/connect-session-oauth-custody-in-personal-posture/whole-unit-correction-1/oauth-correction1-spec-empty-after.resources.jsonl
~/.cache/connectors-cli-wave-20260906/connect-session-oauth-custody-in-personal-posture/whole-unit-correction-1/oauth-correction1-spec-empty-after.result.json
~/.cache/connectors-cli-wave-20260906/connect-session-oauth-custody-in-personal-posture/whole-unit-correction-1/oauth-correction1-spec-empty-after.warm-target.json
~/.cache/connectors-cli-wave-20260906/connect-session-oauth-custody-in-personal-posture/whole-unit-correction-1/original-run-private.py
~/.cache/connectors-cli-wave-20260906/connect-session-oauth-custody-in-personal-posture/whole-unit-correction-1/outside-paths.json
~/.cache/connectors-cli-wave-20260906/connect-session-oauth-custody-in-personal-posture/whole-unit-correction-1/preimage/crates/connector-spec/schema/provider-toml.schema.json
~/.cache/connectors-cli-wave-20260906/connect-session-oauth-custody-in-personal-posture/whole-unit-correction-1/preimage/crates/connector-spec/src/auth.rs
~/.cache/connectors-cli-wave-20260906/connect-session-oauth-custody-in-personal-posture/whole-unit-correction-1/preimage/crates/connector-spec/src/provider/auth_validation.rs
~/.cache/connectors-cli-wave-20260906/connect-session-oauth-custody-in-personal-posture/whole-unit-correction-1/preimage/crates/connector-spec/tests/main/personal_oauth.rs
~/.cache/connectors-cli-wave-20260906/connect-session-oauth-custody-in-personal-posture/whole-unit-correction-1/provenance-audit-first-observation.json
~/.cache/connectors-cli-wave-20260906/connect-session-oauth-custody-in-personal-posture/whole-unit-correction-1/raw-report.md
~/.cache/connectors-cli-wave-20260906/connect-session-oauth-custody-in-personal-posture/whole-unit-correction-1/report.md
~/.cache/connectors-cli-wave-20260906/connect-session-oauth-custody-in-personal-posture/whole-unit-correction-1/resource-summary.json
~/.cache/connectors-cli-wave-20260906/connect-session-oauth-custody-in-personal-posture/whole-unit-correction-1/root-before-after-counts.json
~/.cache/connectors-cli-wave-20260906/connect-session-oauth-custody-in-personal-posture/whole-unit-correction-1/run-private.py
~/.cache/connectors-cli-wave-20260906/connect-session-oauth-custody-in-personal-posture/whole-unit-correction-1/seal.py
~/.cache/connectors-cli-wave-20260906/connect-session-oauth-custody-in-personal-posture/whole-unit-correction-1/source-check.log
~/.cache/connectors-cli-wave-20260906/connect-session-oauth-custody-in-personal-posture/whole-unit-correction-1/source-inventory.json
~/.cache/connectors-cli-wave-20260906/connect-session-oauth-custody-in-personal-posture/whole-unit-correction-1/source-preservation.json
~/.cache/connectors-cli-wave-20260906/connect-session-oauth-custody-in-personal-posture/whole-unit-correction-1/source-stat.txt
~/.cache/connectors-cli-wave-20260906/connect-session-oauth-custody-in-personal-posture/whole-unit-correction-1/source.patch
~/.cache/connectors-cli-wave-20260906/connect-session-oauth-custody-in-personal-posture/whole-unit-correction-1/source.sha256
~/.cache/connectors-cli-wave-20260906/connect-session-oauth-custody-in-personal-posture/whole-unit-correction-1/status.txt
~/.cache/connectors-cli-wave-20260906/connect-session-oauth-custody-in-personal-posture/whole-unit-correction-1/target-executables.sha256
~/.cache/connectors-cli-wave-20260906/connect-session-oauth-custody-in-personal-posture/whole-unit-correction-1/target-inventory.json

Assigned additional write roots: /dev/shm/b10x-cli-wave-wt-4f1de73d0685-nr00xjox/runtime-target; ~/.cache/cw6/oc1. Ordinary Cargo bookkeeping may update ~/.cargo/.global-cache, ~/.cargo/.package-cache and ~/.cargo/.package-cache-mutate. Target file names are fully inventoried; temporary compiler/test and ancillary cache objects are root-level disclosures, not an exact per-object trace. RUSTC_WRAPPER stayed unset. No cache/target cleanup or live/provider action occurred.

The portable report replaces only the original absolute home-directory prefix with a tilde. All other report prose, counts and embedded raw output remain identical. Separate raw/portable hashes and the immutable evidence manifest identify the result. No local Markdown link is introduced into the portable report.
