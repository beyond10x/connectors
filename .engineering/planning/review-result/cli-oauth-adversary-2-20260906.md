---
format: aep.planning-md/1
id: review-result:cli-oauth-adversary-2-20260906
kind: review-result
status: active
title: Personal OAuth second whole-unit adversary report
relations:
- reviews: story:connect-session-oauth-custody-in-personal-posture
revision: 1
---
unit: connect-session-oauth-custody-in-personal-posture, formal pass 2; f9bf1d6a2e00b53711ef99473b7556b9e3743f25 plus retained tests.patch
verdict: nothing found
cases: executed 1509→1513, red 0
origin: introduced 0 / pre-existing 0 / undecided 0
wrote-outside-worktree: 134 retained reviewer scratch paths; assigned build/TMPDIR and tool cache roots below
needs-coordinator: yes — record the completed final ordinary pass and perform later combined integration routing/gates

````text
 .../catalog-build/tests/main/catalog_invariants.rs |  48 +++++++++
 crates/connector-spec/tests/main/personal_oauth.rs |  54 ++++++++++
 crates/connectors-console/tests/personal_oauth.rs  | 113 +++++++++++++++++++++
 .../src/oauth_adversary_tests.rs                   |  99 ++++++++++++++++++
 4 files changed, 314 insertions(+)
````

The stat is verbatim git --no-pager diff --stat. Only four existing test files gained cases. All original bytes in all twelve assigned owners remain exact prefixes, including every implementation/pass-1 assertion; every unowned file in the 1,193-file source inventory is unchanged. Comparisons: ~/.cache/connectors-cli-wave-20260906/connect-session-oauth-custody-in-personal-posture/whole-unit-adversary-2/final-original-test-preservation.json and ~/.cache/connectors-cli-wave-20260906/connect-session-oauth-custody-in-personal-posture/whole-unit-adversary-2/final-source-scope-proof.json.

This final ordinary pass covers the complete helper/FULL/custody/refresh/schema-4/runtime/trusted-consumer unit over published base 0c69450921ab1794c81dadec915b717a61bf0983. The full prior review, source/callers and acceptance were read in pass 1 and revisited here including the one-field correction; 1,192 other source hashes remain exact. The 227-member first-pass and 53-member correction seals were verified. Design 21/current OAuth story and ESS govern. Separate Design 22/auth protocol is outside this candidate. No production, manifest/dependency/schema/generated file, Git/AEP/worktree/operator/provider or cleanup mutation occurred.

Before count: the matching 1,509-case operational cohort reported by pass 1 (1,508 pass, one fail), not a baseline rerun. The correction separately passed its identical 1,104-case root selection; other pass-1 lanes were not new correction executions. This pass executes 1,513 operational cases, all passing, plus 50 separately counted helper/SQLite cases. The historical published-reader witness remains pass-1 evidence and was not rerun or counted. The first empty-admission finding and undecided origin remain immutable; its retained regression passes in this full run. No new origin claim is made from an unexecuted old loader.

Before any selected test, my direct rustfmt invocation used edition 2024 on two test files in this edition-2021 workspace. Preservation precheck caught reflow/import ordering in old lines. Observation and preimages: ~/.cache/connectors-cli-wave-20260906/connect-session-oauth-custody-in-personal-posture/whole-unit-adversary-2/formatting-precheck-observation.json and ~/.cache/connectors-cli-wave-20260906/connect-session-oauth-custody-in-personal-posture/whole-unit-adversary-2/formatting-precheck-observation. Exact original prefixes were restored before the first selection and additions formatted with edition 2021. No original assertion was executed in the transient formatting state.

The schema case initially used an invalid oracle: raw OAuth2Spec deserialization was compared with schema conditional matching-grant requirements while personal_flows remained present. It failed at catalog_invariants.rs:3066 (exit 101, one failed). Actual provider validation runs validate_personal_oauth at crates/connector-spec/src/provider/auth_validation.rs:297 and checks matching grants. The raw-serde construction does not establish loader acceptance. Only my new case was corrected to compare legacy defaults without personal admission; no product fix or new attack pass occurred. Initial source/patch/log: ~/.cache/connectors-cli-wave-20260906/connect-session-oauth-custody-in-personal-posture/whole-unit-adversary-2/schema-first-catalog-invariants.rs, ~/.cache/connectors-cli-wave-20260906/connect-session-oauth-custody-in-personal-posture/whole-unit-adversary-2/schema-first-tests.patch, ~/.cache/connectors-cli-wave-20260906/connect-session-oauth-custody-in-personal-posture/whole-unit-adversary-2/oauth-pass2-schema-first.log.

| New case | Assertion and reachability | Final selected result |
| --- | --- | --- |
| crates/connector-spec/tests/main/personal_oauth.rs:173 (oauth_pass2_presence_round_trips_across_formats_without_changing_legacy_arrays) | Public provider::load accepts omission and valid PKCE/device admissions; supplied empty admissions and wrong JSON/YAML shapes refuse. JSON/TOML/YAML round trips preserve valid admissions and omission. Legacy grants/scopes defaults remain separate. | 1 pass / 0 fail, exit 0 |
| crates/catalog-build/tests/main/catalog_invariants.rs:3032 (oauth_pass2_nearby_authoring_array_shapes_agree_with_the_active_schema) | Actual authoring schema and OAuth2Spec deserialization agree on personal_flows structural presence/shape and nearby legacy arrays when personal admission is omitted. Conditional personal grant requirements are validated later by the provider loader. | 1 pass / 0 fail, exit 0 |
| crates/integration-catalog/src/oauth_adversary_tests.rs:233 (oauth_pass2_shutdown_after_token_before_evidence_never_publishes_or_restarts) | Actual catalog Create/callback/device polling obtains a token, then holds token-info. Shutdown drops that future in both flows without access/custody publication. Reopening actual SQLite/file custody does not resume acquisition, reuse credentials or send an operation. | 1 pass / 0 fail, exit 0 |
| crates/connectors-console/tests/personal_oauth.rs:830 (oauth_pass2_private_file_expires_while_completion_grace_stays_bounded) | Public console dispatch consumes a synthetic local daemon/private device endpoint. One Create; private path removed and held inode emptied at expiry; bounded completion grace accepts matching Completed/Callable or times out without private echo. The delayed daemon does not itself prove a durable callback claim. | 1 pass / 0 fail, exit 0 |

Deciding cases were written before these actual first selections, reproduced in execution order:

Execution presence-first; started 2026-09-06T18:31:55.424023+00:00; finished 2026-09-06T18:32:00.752138+00:00. Exact command/cwd/environment:

````json
{
  "cwd": "~/.local/state/worktree/trees/b10x/connectors/wt-4f1de73d0685",
  "argv": [
    "env",
    "-u",
    "RUSTC_WRAPPER",
    "CARGO_TARGET_DIR=/dev/shm/b10x-cli-wave-wt-4f1de73d0685-nr00xjox/runtime-target",
    "TMPDIR=~/.cache/cw6/oa2",
    "CARGO_BUILD_JOBS=1",
    "CARGO_INCREMENTAL=0",
    "CARGO_PROFILE_DEV_DEBUG=0",
    "CARGO_PROFILE_TEST_DEBUG=0",
    "cargo",
    "test",
    "--locked",
    "--offline",
    "-p",
    "connector-spec",
    "oauth_pass2_presence_round_trips_across_formats_without_changing_legacy_arrays",
    "--",
    "--nocapture"
  ]
}
````

Actual complete output from ~/.cache/connectors-cli-wave-20260906/connect-session-oauth-custody-in-personal-posture/whole-unit-adversary-2/oauth-pass2-presence-first.log:

````text
   Compiling connector-spec v0.6.5 (~/.local/state/worktree/trees/b10x/connectors/wt-4f1de73d0685/crates/connector-spec)
    Finished `test` profile [unoptimized] target(s) in 2.20s
     Running unittests src/lib.rs (/dev/shm/b10x-cli-wave-wt-4f1de73d0685-nr00xjox/runtime-target/debug/deps/connector_spec-e6a0778184c9adb7)

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 28 filtered out; finished in 0.00s

     Running tests/main.rs (/dev/shm/b10x-cli-wave-wt-4f1de73d0685-nr00xjox/runtime-target/debug/deps/main-791231b9d8bface7)

running 1 test
test personal_oauth::oauth_pass2_presence_round_trips_across_formats_without_changing_legacy_arrays ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 517 filtered out; finished in 0.00s

````

Exit 0; 1 passed / 0 failed / 0 ignored. Guard interrupted: false.

Execution schema-first; started 2026-09-06T18:36:16.827604+00:00; finished 2026-09-06T18:36:27.324901+00:00. Exact command/cwd/environment:

````json
{
  "cwd": "~/.local/state/worktree/trees/b10x/connectors/wt-4f1de73d0685",
  "argv": [
    "env",
    "-u",
    "RUSTC_WRAPPER",
    "CARGO_TARGET_DIR=/dev/shm/b10x-cli-wave-wt-4f1de73d0685-nr00xjox/runtime-target",
    "TMPDIR=~/.cache/cw6/oa2",
    "CARGO_BUILD_JOBS=1",
    "CARGO_INCREMENTAL=0",
    "CARGO_PROFILE_DEV_DEBUG=0",
    "CARGO_PROFILE_TEST_DEBUG=0",
    "cargo",
    "test",
    "--locked",
    "--offline",
    "-p",
    "catalog-build",
    "oauth_pass2_nearby_authoring_array_shapes_agree_with_the_active_schema",
    "--",
    "--nocapture"
  ]
}
````

Actual complete output from ~/.cache/connectors-cli-wave-20260906/connect-session-oauth-custody-in-personal-posture/whole-unit-adversary-2/oauth-pass2-schema-first.log:

````text
   Compiling connector-spec v0.6.5 (~/.local/state/worktree/trees/b10x/connectors/wt-4f1de73d0685/crates/connector-spec)
   Compiling catalog-build v0.6.5 (~/.local/state/worktree/trees/b10x/connectors/wt-4f1de73d0685/crates/catalog-build)
    Finished `test` profile [unoptimized] target(s) in 8.29s
     Running unittests src/lib.rs (/dev/shm/b10x-cli-wave-wt-4f1de73d0685-nr00xjox/runtime-target/debug/deps/catalog_build-9986af463f2ef045)

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 64 filtered out; finished in 0.00s

     Running tests/main.rs (/dev/shm/b10x-cli-wave-wt-4f1de73d0685-nr00xjox/runtime-target/debug/deps/main-d7af594e1c2aad2b)

running 1 test

thread 'catalog_invariants::oauth_pass2_nearby_authoring_array_shapes_agree_with_the_active_schema' (552861) panicked at crates/catalog-build/tests/main/catalog_invariants.rs:3066:13:
assertion `left == right` failed: grants: {"authorize_path":"/oauth/authorize","endpoint":"login","personal_flows":[{"client_authentication":"public","flow":"authorization_code_pkce","redirect_shape":"loopback_ipv4_http","refresh_policy":"required","registration_use":"development_only","token_evidence":{"client_id_pointer":"/application/uid","endpoint":{"path":"/oauth/token/info","service":"login"},"scope_encoding":"string_array","scopes_pointer":"/scope","subject_pointer":"/resource_owner_id"}},{"client_authentication":"public","device_authorization_endpoint":{"path":"/oauth/authorize_device","service":"login"},"flow":"device_authorization","refresh_policy":"if_issued","registration_use":"development_only","token_evidence":{"client_id_pointer":"/application/uid","endpoint":{"path":"/oauth/token/info","service":"login"},"scope_encoding":"string_array","scopes_pointer":"/scope","subject_pointer":"/resource_owner_id"}}],"scopes":["api"],"token_path":"/oauth/token"}
  left: true
 right: false
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace
test catalog_invariants::oauth_pass2_nearby_authoring_array_shapes_agree_with_the_active_schema ... FAILED

failures:

failures:
    catalog_invariants::oauth_pass2_nearby_authoring_array_shapes_agree_with_the_active_schema

test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 85 filtered out; finished in 0.01s

error: test failed, to rerun pass `-p catalog-build --test main`
````

Exit 101; 0 passed / 1 failed / 0 ignored. Guard interrupted: false.

Execution schema-corrected; started 2026-09-06T18:37:25.931867+00:00; finished 2026-09-06T18:37:31.237703+00:00. Exact command/cwd/environment:

````json
{
  "cwd": "~/.local/state/worktree/trees/b10x/connectors/wt-4f1de73d0685",
  "argv": [
    "env",
    "-u",
    "RUSTC_WRAPPER",
    "CARGO_TARGET_DIR=/dev/shm/b10x-cli-wave-wt-4f1de73d0685-nr00xjox/runtime-target",
    "TMPDIR=~/.cache/cw6/oa2",
    "CARGO_BUILD_JOBS=1",
    "CARGO_INCREMENTAL=0",
    "CARGO_PROFILE_DEV_DEBUG=0",
    "CARGO_PROFILE_TEST_DEBUG=0",
    "cargo",
    "test",
    "--locked",
    "--offline",
    "-p",
    "catalog-build",
    "oauth_pass2_nearby_authoring_array_shapes_agree_with_the_active_schema",
    "--",
    "--nocapture"
  ]
}
````

Actual complete output from ~/.cache/connectors-cli-wave-20260906/connect-session-oauth-custody-in-personal-posture/whole-unit-adversary-2/oauth-pass2-schema-corrected.log:

````text
   Compiling catalog-build v0.6.5 (~/.local/state/worktree/trees/b10x/connectors/wt-4f1de73d0685/crates/catalog-build)
    Finished `test` profile [unoptimized] target(s) in 1.66s
     Running unittests src/lib.rs (/dev/shm/b10x-cli-wave-wt-4f1de73d0685-nr00xjox/runtime-target/debug/deps/catalog_build-9986af463f2ef045)

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 64 filtered out; finished in 0.00s

     Running tests/main.rs (/dev/shm/b10x-cli-wave-wt-4f1de73d0685-nr00xjox/runtime-target/debug/deps/main-d7af594e1c2aad2b)

running 1 test
test catalog_invariants::oauth_pass2_nearby_authoring_array_shapes_agree_with_the_active_schema ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 85 filtered out; finished in 0.01s

````

Exit 0; 1 passed / 0 failed / 0 ignored. Guard interrupted: false.

Execution shutdown-first; started 2026-09-06T18:38:07.618278+00:00; finished 2026-09-06T18:38:12.917790+00:00. Exact command/cwd/environment:

````json
{
  "cwd": "~/.local/state/worktree/trees/b10x/connectors/wt-4f1de73d0685/crates/connectors-runtime",
  "argv": [
    "env",
    "-u",
    "RUSTC_WRAPPER",
    "CARGO_TARGET_DIR=/dev/shm/b10x-cli-wave-wt-4f1de73d0685-nr00xjox/runtime-target",
    "TMPDIR=~/.cache/cw6/oa2",
    "CARGO_BUILD_JOBS=1",
    "CARGO_INCREMENTAL=0",
    "CARGO_PROFILE_DEV_DEBUG=0",
    "CARGO_PROFILE_TEST_DEBUG=0",
    "cargo",
    "test",
    "--locked",
    "--offline",
    "-p",
    "integration-catalog",
    "oauth_pass2_shutdown_after_token_before_evidence_never_publishes_or_restarts",
    "--",
    "--nocapture"
  ]
}
````

Actual complete output from ~/.cache/connectors-cli-wave-20260906/connect-session-oauth-custody-in-personal-posture/whole-unit-adversary-2/oauth-pass2-shutdown-first.log:

````text
   Compiling integration-catalog v0.6.5 (~/.local/state/worktree/trees/b10x/connectors/wt-4f1de73d0685/crates/integration-catalog)
    Finished `test` profile [unoptimized] target(s) in 3.00s
     Running unittests src/lib.rs (/dev/shm/b10x-cli-wave-wt-4f1de73d0685-nr00xjox/runtime-target/debug/deps/integration_catalog-aa8f6c5fcc137086)

running 1 test
test oauth::tests::adversary::oauth_pass2_shutdown_after_token_before_evidence_never_publishes_or_restarts ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 91 filtered out; finished in 1.13s

````

Exit 0; 1 passed / 0 failed / 0 ignored. Guard interrupted: false.

Execution expiry-first; started 2026-09-06T18:38:39.237875+00:00; finished 2026-09-06T18:39:05.165146+00:00. Exact command/cwd/environment:

````json
{
  "cwd": "~/.local/state/worktree/trees/b10x/connectors/wt-4f1de73d0685/crates/connectors-console",
  "argv": [
    "env",
    "-u",
    "RUSTC_WRAPPER",
    "CARGO_TARGET_DIR=/dev/shm/b10x-cli-wave-wt-4f1de73d0685-nr00xjox/runtime-target",
    "TMPDIR=~/.cache/cw6/oa2",
    "CARGO_BUILD_JOBS=1",
    "CARGO_INCREMENTAL=0",
    "CARGO_PROFILE_DEV_DEBUG=0",
    "CARGO_PROFILE_TEST_DEBUG=0",
    "cargo",
    "test",
    "--locked",
    "--offline",
    "--test",
    "personal_oauth",
    "oauth_pass2_private_file_expires_while_completion_grace_stays_bounded",
    "--",
    "--nocapture"
  ]
}
````

Actual complete output from ~/.cache/connectors-cli-wave-20260906/connect-session-oauth-custody-in-personal-posture/whole-unit-adversary-2/oauth-pass2-expiry-first.log:

````text
   Compiling connectors-console v0.6.5 (~/.local/state/worktree/trees/b10x/connectors/wt-4f1de73d0685/crates/connectors-console)
    Finished `test` profile [unoptimized] target(s) in 0.84s
     Running tests/personal_oauth.rs (/dev/shm/b10x-cli-wave-wt-4f1de73d0685-nr00xjox/runtime-target/debug/deps/personal_oauth-42c4eb7161a076c0)

running 1 test
test oauth_pass2_private_file_expires_while_completion_grace_stays_bounded ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 10 filtered out; finished in 20.03s

````

Exit 0; 1 passed / 0 failed / 0 ignored. Guard interrupted: false.

Affected full/strict/fmt commands follow the deciding cases. Tests use ordinary default parallelism and --no-fail-fast. No baseline suite or final twelve-workspace integration gate is claimed.

| Lane | Full executed / failed / ignored | Strict Clippy exit | Fmt exit |
| --- | --- | --- | --- |
| root | 1106 / 0 / 1 | 0 | 0 |
| runtime | 222 / 0 / 0 | 0 | 0 |
| console | 102 / 0 / 0 | 0 | 0 |
| cli | 133 / 0 / 0 | 0 | 0 |

Root 1,106 includes 36 connector-oauth helper cases; runtime 222 includes 14 state-sqlite cases; console 102; CLI 133. Total 1,563 = 1,513 operational + 50 helpers/SQLite; zero failed, one retained ignored case. The initial fixture failure is preserved separately from final red. Build output retains the unexercised live-Vault warning; no live provider coverage is claimed.

Execution root-full; started 2026-09-06T18:39:26.460248+00:00; finished 2026-09-06T18:40:44.828098+00:00. Exact command/cwd/environment:

````json
{
  "cwd": "~/.local/state/worktree/trees/b10x/connectors/wt-4f1de73d0685",
  "argv": [
    "env",
    "-u",
    "RUSTC_WRAPPER",
    "CARGO_TARGET_DIR=/dev/shm/b10x-cli-wave-wt-4f1de73d0685-nr00xjox/runtime-target",
    "TMPDIR=~/.cache/cw6/oa2",
    "CARGO_BUILD_JOBS=1",
    "CARGO_INCREMENTAL=0",
    "CARGO_PROFILE_DEV_DEBUG=0",
    "CARGO_PROFILE_TEST_DEBUG=0",
    "cargo",
    "test",
    "--locked",
    "--offline",
    "-p",
    "connector-spec",
    "-p",
    "connector-resolve",
    "-p",
    "catalog",
    "-p",
    "catalog-build",
    "-p",
    "catalog-reader",
    "-p",
    "connectors-client",
    "-p",
    "protocol",
    "-p",
    "server",
    "-p",
    "service",
    "-p",
    "connector-oauth",
    "--no-fail-fast"
  ]
}
````

Actual complete output from ~/.cache/connectors-cli-wave-20260906/connect-session-oauth-custody-in-personal-posture/whole-unit-adversary-2/oauth-pass2-root-full.log:

````text
   Compiling catalog-build v0.6.5 (~/.local/state/worktree/trees/b10x/connectors/wt-4f1de73d0685/crates/catalog-build)
   Compiling connector-spec v0.6.5 (~/.local/state/worktree/trees/b10x/connectors/wt-4f1de73d0685/crates/connector-spec)
    Finished `test` profile [unoptimized] target(s) in 3.63s
     Running unittests src/lib.rs (/dev/shm/b10x-cli-wave-wt-4f1de73d0685-nr00xjox/runtime-target/debug/deps/catalog-bf05e0919bf12a7e)

running 9 tests
test table::tests::a_minting_join_in_the_document_reaches_acquisition_minted ... ok
test table::tests::sip_v1_survives_the_generated_table ... ok
test table::tests::cdp_v1_survives_the_generated_table ... ok
test table::tests::the_document_tells_apart_the_pair_the_derivation_could_not ... ok
test tests::providers_are_listed_in_a_stable_order ... ok
test tests::an_operation_is_found_by_its_symbol ... ok
test tests::an_unknown_key_is_none_rather_than_a_panic ... ok
test tests::listing_by_provider_partitions_the_catalog ... ok
test tests::operation_ids_are_unique_across_the_catalog ... ok

test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.82s

     Running tests/main.rs (/dev/shm/b10x-cli-wave-wt-4f1de73d0685-nr00xjox/runtime-target/debug/deps/main-d5244403d8e52e4e)

running 15 tests
test consumer_api::rate_stage2_typed_history_advice_preserves_all_application_categories ... ok
test consumer_api::the_closed_vocabularies_match_exhaustively ... ok
test pack_table::every_closed_configuration_set_is_addressable ... ok
test consumer_api::the_lookup_surface_and_every_operation_field_are_reachable ... ok
test pack_table::grafana_discovery_is_available_as_closed_catalog_data ... ok
test pack_table::every_channel_binding_resolves_its_events ... ok
test pack_table::the_table_covers_the_whole_pack ... ok
test consumer_api::the_inbound_and_configuration_surfaces_are_reachable ... ok
test consumer_api::the_caller_contract_is_document_data_alone ... ok
test pack_table::every_credential_carries_a_leaf_and_a_placement ... ok
test pack_table::the_derived_operation_facts_agree_with_the_documents ... ok
test source_fidelity_catalog_describes_translated_input_and_output_from_the_document ... ok
test consumer_api::the_reader_reexport_serves_the_same_catalogue ... ok
test pack_table::every_operation_is_reachable_by_key_exactly_once ... ok
test pack_table::rate_adversary_pack_and_table_preserve_every_declared_rate ... ok

test result: ok. 15 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.05s

     Running unittests src/lib.rs (/dev/shm/b10x-cli-wave-wt-4f1de73d0685-nr00xjox/runtime-target/debug/deps/catalog_build-b548eee29482c29e)

running 64 tests
test artifact::tests::fixtures_live_in_the_build_directory_and_never_repeat_a_path ... ok
test artifact::tests::a_fixture_does_not_survive_a_panicking_test ... ok
test artifact::tests::write_atomic_creates_missing_directories ... ok
test artifact::tests::write_atomic_replaces_and_leaves_no_temporary ... ok
test artifact::tests::read_if_exists_distinguishes_absent_from_present ... ok
test contract::tests::an_operation_with_no_parameters_states_the_empty_object ... ok
test contract::tests::arrays_carry_their_item_type_and_the_fallback_reaches_inside ... ok
test contract::tests::a_const_pinned_body_field_reserves_the_symbol_a_later_field_must_shift_past ... ok
test contract::tests::scalars_map_across_and_constraints_are_dropped ... ok
test contract::tests::shapes_the_contract_cannot_express_fall_back_to_the_top_type ... ok
test contract::tests::the_description_extends_with_the_error_envelope ... ok
test diff::tests::a_new_file_is_all_additions ... ok
test diff::tests::a_replaced_middle_keeps_its_context ... ok
test diff::tests::a_removed_line_is_a_single_removal ... ok
test diff::tests::an_appended_line_is_a_single_addition ... ok
test diff::tests::a_wholly_different_file_shows_both_sides ... ok
test diff::tests::an_unchanged_file_produces_only_context ... ok
test document::tests::an_approved_origin_may_be_the_whole_base_url ... ok
test document::tests::a_declared_graph_is_refused_rather_than_dropped ... ok
test document::tests::a_declared_registration_value_is_a_build_error ... ok
test document::tests::a_braced_constant_header_is_refused ... ok
test inbound::tests::an_unset_webhook_never_launders_itself_as_connection_authenticated ... ok
test inbound::tests::the_tri_state_publishes_as_three_distinct_kinds ... ok
test document::tests::an_unclassifiable_brace_literal_is_a_build_error_never_a_degraded_document ... ok
test inbound::tests::only_a_deliberately_unverifiable_surface_reads_as_unverified ... ok
test net::tests::a_denied_checkpoint_fails_and_is_counted ... ok
test pack::tests::a_nested_operations_key_is_not_the_array ... ok
test pack::tests::spans_slice_each_operation_record_exactly ... ok
test pack::tests::disagreeing_schema_versions_are_refused ... ok
test pack::tests::a_duplicate_operation_id_is_refused_by_name ... ok
test pack::tests::the_compiled_pack_round_trips_its_own_spans ... ok
test scaffold::tests::a_select_argument_drops_fields_from_the_right ... ok
test scaffold::tests::a_prefix_matches_whole_segments ... ok
test scaffold::tests::a_select_argument_refuses_what_it_cannot_mean ... ok
test scaffold::tests::a_service_name_is_cut_at_the_pull_date ... ok
test seam::tests::a_pin_naming_an_absent_document_is_refused_and_lists_the_cache ... ok
test seam::tests::a_hand_authored_definition_loads_into_the_ir ... ok
test seam::tests::a_spec_backed_provider_ingests_its_operations ... ok
test seam::tests::a_spec_backed_provider_with_no_patch_publishes_no_operations ... ok
test seam::tests::an_empty_definition_is_rejected ... ok
test seam::tests::an_unknown_service_is_an_error_that_names_what_exists ... ok
test document::tests::source_fidelity_schema_three_preserves_the_frozen_schema_two_identity_and_bytes ... ok
test seam::tests::selecting_a_service_carries_no_other_services_config_graphs_or_verify ... ok
test seam::tests::selecting_a_service_drops_every_other_operation ... ok
test contract::tests::source_fidelity_preserves_nullable_enum_union_defaults_and_body_constraints ... ok
test seam::tests::the_loaders_own_diagnosis_survives ... ok
test document::tests::personal_acquisition_schema_four_keeps_frozen_schema_three_bytes_and_closed_registration_boundary ... ok
test seam::tests::the_pinned_document_is_the_one_ingested_not_the_last_in_the_cache ... ok
test seam::tests::selecting_a_service_keeps_the_connector_level_surfaces ... ok
test document::tests::a_custody_only_connector_renders_a_document_with_no_surface ... ok
test document::tests::an_ordinary_connector_does_not_carry_the_flag ... ok
test document::tests::a_form_body_is_spelled_structurally ... ok
test document::tests::rendering_is_deterministic_and_validates ... ok
test check::tests::provider_coverage_is_checked_in_both_directions ... ok
test check::tests::a_mutated_artifact_is_named_as_artifact_drift ... ok
test check::tests::a_comment_only_provider_edit_is_named_as_declaration_drift ... ok
test check::tests::a_wrong_lock_artifact_hash_is_named_as_lock_row_drift ... ok
test check::tests::a_revendored_spec_is_named_as_spec_drift ... ok
test check::tests::a_clean_tree_reports_the_provider_and_artifact_counts_without_writing ... ok
test check::tests::spec_coverage_is_checked_in_both_directions ... ok
test check::tests::unsafe_and_symlinked_lock_paths_are_refused_without_reading_the_target ... ok
test check::tests::artifact_coverage_is_checked_in_both_directions ... ok
test pipeline::tests::a_refusal_is_the_first_one_in_provider_order_at_every_width ... ok
test pipeline::tests::the_plan_is_identical_at_every_compile_width ... ok

test result: ok. 64 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.04s

     Running tests/main.rs (/dev/shm/b10x-cli-wave-wt-4f1de73d0685-nr00xjox/runtime-target/debug/deps/main-4f9b671ea94aed75)

running 86 tests
test architecture_fence::reusable_client_has_no_runtime_or_backend_ownership ... ok
test architecture_fence::product_cli_is_a_thin_frontend ... ok
test architecture_fence::every_runtime_isolation_boundary_is_explicit_and_locked ... ok
test catalog_invariants::a_session_signal_reaches_a_backend_only_through_the_admission_seam ... ok
test catalog_invariants::adversary_gitlab_pass1_translation_preserves_composed_constraint_truth_tables ... ok
test architecture_fence::service_owns_the_backend_port_and_server_only_adapts_transport ... ok
test catalog_invariants::oauth_pass2_nearby_authoring_array_shapes_agree_with_the_active_schema ... ok
test architecture_fence::runtime_is_the_only_adapter_composition_root ... ok
test catalog_invariants::rate_repair_source_uri_grammar_matches_authoring_and_both_schemas ... ok
test architecture_fence::production_modules_obey_the_named_size_fence ... ok
test catalog_invariants::no_effect_backend_is_reachable_without_an_admission_proof ... ok
test catalog_invariants::repository_authored_anthropic_sources_reproduce_only_the_api_connector ... ok
test catalog_invariants::rate_final_actual_provider_loading_preserves_uri_and_vendor_contract ... ok
test catalog_invariants::personal_acquisition_schema_four_preserves_actual_provider_uri_vectors_and_contract ... ok
test catalog_invariants::gitlab_official_source_inventory_accounts_for_every_operation ... ok
test catalog_invariants::substrate_axis_projection_is_pinned_total_and_non_mechanical ... ok
test catalog_invariants::adversary_gitlab_pass1_coverage_statuses_match_actual_importer_results ... ok
test catalog_invariants::source_fidelity_gitlab_preserves_every_selected_vendor_schema ... ok
test catalog_invariants::a_full_build_leaves_no_orphaned_artifact ... ok
test catalog_invariants::the_committed_tree_is_a_fixed_point_of_a_build ... ok
test catalog_invariants::promoted_operation_traits_equal_the_pre_migration_inventory ... ok
test catalog_invariants::a_custody_only_provider_publishes_no_surface_and_every_other_provider_does ... ok
test catalog_invariants::the_browser_surface_is_read_only_and_carries_no_interaction_member ... ok
test catalog_invariants::rate_adversary_canonical_source_urls_match_authoring_reader ... ok
test catalog_invariants::gitlab_schedule_slice_is_generated_and_preserves_legacy_contracts ... ok
test catalog_invariants::gitlab_user_and_automation_connections_are_distinct_and_scope_gated ... ok
test catalog_invariants::every_format_origin_field_lowers_to_the_origin_slot ... ok
test catalog_invariants::the_contract_and_the_params_state_the_same_symbols ... ok
test catalog_invariants::ids_are_unique_in_every_namespace_they_share ... ok
test dependency_fence::a_compiler_crate_cannot_reach_a_network_crate ... ok
test catalog_invariants::every_operations_required_parameters_are_the_ones_a_caller_must_send ... ok
test catalog_invariants::slack_surface_is_curated_and_credential_scopes_never_cross_purposes ... ok
test catalog_invariants::the_mcp_transport_reaches_a_backend_only_through_the_decided_seams ... ok
test catalog_invariants::personal_acquisition_schema_four_retains_the_rate_declaration_contract ... ok
test dependency_fence::every_workspace_member_is_classified ... ok
test dependency_fence::outbound_mcp_foundation_is_exactly_pinned ... ok
test dependency_fence::the_gate_and_the_release_workflow_state_the_gates_own_workspace_count ... ok
test dependency_fence::the_connectors_binary_is_an_isolated_locked_composition_leaf ... ok
test dependency_fence::released_http_integrations_cannot_bypass_connection_bound_egress ... ok
test dependency_fence::the_build_path_does_not_depend_on_the_secret_store ... ok
test dependency_fence::the_walk_finds_an_edge_that_is_not_direct ... ok
test dependency_fence::the_rtvbp_runtime_dependency_is_isolated_from_the_canonical_workspace ... ok
test engine_free::no_manifest_in_this_workspace_requires_an_engine_crate ... ok
test dependency_fence::the_voice_runtime_is_the_only_production_composition_leaf ... ok
test engine_free::the_walk_finds_an_engine_two_edges_away_and_stops_at_a_severed_one ... ok
test ess_citation_fence::every_declared_error_and_event_is_cited_or_unmapped ... ok
test ess_citation_fence::every_published_event_is_an_event_some_domain_declares ... ok
test engine_free::the_lockfile_names_no_engine_crate_at_all ... ok
test ess_citation_fence::every_connect_session_state_write_is_cited ... ok
test ess_claim_fence::deleting_session_terminates_declared_refusal_outcomes_is_refused ... ok
test ess_citation_fence::the_reobserve_site_leaves_a_connection_ref_and_the_specification_says_so ... ok
test ess_claim_fence::every_declared_wire_name_is_a_method_the_protocol_accepts ... ok
test ess_citation_fence::field_citations_span_the_struct_they_name ... ok
test ess_claim_fence::deleting_materializes_declared_refusal_outcomes_is_refused ... ok
test ess_citation_fence::the_channel_summary_that_carries_a_connection_ref_is_cited_and_modelled ... ok
test ess_claim_fence::materialize_declares_or_marks_every_refusal_its_cited_function_performs ... ok
test ess_claim_fence::deleting_the_sentence_that_counts_the_refusal_sites_is_refused ... ok
test ess_claim_fence::session_terminate_declares_or_marks_every_refusal_its_cited_function_performs ... ok
test ess_citation_fence::every_citation_of_the_specification_resolves ... ok
test json_governance::a_json_schema_invalid_against_its_declared_meta_schema_fails ... ok
test json_governance::an_invalid_owned_document_fails_its_schema ... ok
test json_governance::an_unclassified_json_file_fails_by_name ... ok
test json_governance::malformed_vendored_json_fails_even_though_it_is_syntax_only ... ok
test ess_claim_fence::the_hosted_registry_never_reaches_the_state_its_marker_says_it_cannot ... ok
test msrv_fence::the_running_toolchain_meets_the_declared_msrv ... ok
test ess_claim_fence::the_hosted_failed_claim_is_refused_from_either_side ... ok
test msrv_fence::the_walk_finds_a_breach_that_is_not_direct ... ok
test msrv_fence::versions_compare_numerically_and_tolerate_both_spellings ... ok
test catalog_invariants::no_input_or_artifact_carries_a_credential_shaped_value ... ok
test no_network::build_records_no_network_attempt ... ok
test no_network::check_records_no_network_attempt ... ok
test no_network::the_network_seam_is_the_only_door ... ok
test dependency_fence::the_sipx_network_dependency_is_exactly_pinned_and_isolated ... ok
test catalog_invariants::rate_stage2_conditional_history_advice_is_metadata_without_schema_edits ... ok
test catalog_invariants::the_credential_requirement_agrees_with_the_auth_list ... ok
test catalog_invariants::every_canonical_document_validates_against_the_committed_schema ... ok
test catalog_invariants::the_lockfile_agrees_with_every_input_and_every_artifact ... ok
test catalog_invariants::oauth_pass1_all_current_documents_retain_frozen_operation_meaning ... ok
test engine_free::no_workspace_member_reaches_an_engine_crate ... ok
test msrv_fence::no_resolved_dependency_declares_a_rust_version_above_the_crate_that_reaches_it ... ok
test catalog_invariants::the_document_carries_the_callers_contract ... ok
test catalog_invariants::sip_catalog_surface_is_the_bounded_platform_dial_member ... ok
test json_governance::every_repository_json_is_classified_and_valid ... ok
test catalog_invariants::the_pack_serves_the_committed_documents_byte_for_byte ... ok
test catalog_invariants::spec_backed_coverage_holds_in_both_directions ... ok
test catalog_invariants::two_plans_over_the_same_inputs_are_byte_identical ... ok

test result: ok. 86 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 31.69s

     Running unittests src/lib.rs (/dev/shm/b10x-cli-wave-wt-4f1de73d0685-nr00xjox/runtime-target/debug/deps/catalog_reader-44666dbfed70aba2)

running 2 tests
test sha256::tests::the_published_vectors_agree ... ok
test sha256::tests::the_million_a_vector_agrees ... ok

test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s

     Running tests/main.rs (/dev/shm/b10x-cli-wave-wt-4f1de73d0685-nr00xjox/runtime-target/debug/deps/main-787c16ebe6417ef9)

running 17 tests
test pack::measure_read_costs ... ignored, a measurement for predecessor:docs/designs/catalog-artifact.md, not an assertion
test pack::an_operation_naming_an_absent_provider_is_refused ... ok
test pack::a_payload_length_disagreement_is_refused ... ok
test pack::a_newer_schema_version_is_refused_by_name ... ok
test pack::personal_acquisition_does_not_reinterpret_schema_three_packs ... ok
test pack::a_span_outside_the_payload_is_refused ... ok
test pack::additive_growth_is_tolerated ... ok
test pack::something_that_is_not_a_pack_is_refused ... ok
test pack::source_fidelity_does_not_reinterpret_schema_two_packs ... ok
test pack::personal_acquisition_schema_four_is_admitted_before_any_record_is_served ... ok
test pack::the_vendored_sha256_agrees_with_sha2_across_padding_boundaries ... ok
test pack::a_newer_container_format_is_refused_by_name ... ok
test pack::a_tampered_payload_is_refused_before_any_record ... ok
test pack::load_serves_the_committed_pack ... ok
test pack::the_embedded_pack_serves_the_shipped_catalogue ... ok
test pack::every_embedded_record_agrees_with_its_canonical_document ... ok
test pack::oauth_pass1_full_published_pack_version_is_checked_before_nonempty_records ... ok

test result: ok. 16 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 3.05s

     Running unittests src/lib.rs (/dev/shm/b10x-cli-wave-wt-4f1de73d0685-nr00xjox/runtime-target/debug/deps/connector_oauth-9f92ae8f31968df6)

running 33 tests
test device::tests::caller_clock_rollback_never_shortens_the_polling_interval ... ok
test device::tests::device_instructions_do_not_invent_a_complete_uri_and_deadline_is_minimum ... ok
test device::tests::device_response_refuses_the_entire_origin_bounds_and_expiry_class ... ok
test pkce::tests::an_authorize_url_carries_the_pkce_pair_only_for_a_public_client ... ok
test pkce::tests::a_random_token_is_unpadded_url_safe_and_the_requested_width ... ok
test device::tests::device_poll_expiry_cancel_and_unsolicited_response_are_terminal ... ok
test pkce::tests::authorize_url_refuses_a_url_that_cannot_carry_a_path ... ok
test pkce::tests::two_random_tokens_differ ... ok
test pkce::tests::an_origin_carrying_a_query_or_fragment_contributes_neither ... ok
test state::tests::a_state_is_redeemable_once ... ok
test device::tests::device_poll_terminal_outcomes_never_authorize_again_or_retain_a_code ... ok
test pkce::tests::authorize_url_percent_encodes_the_redirect_and_appends_extras_in_order ... ok
test pkce::tests::the_pkce_challenge_is_the_s256_of_the_verifier ... ok
test state::tests::clear_drops_live_entries_too ... ok
test state::tests::contains_any_claims_an_expired_state_so_the_callback_is_refused_not_lost ... ok
test device::tests::device_poll_obeys_default_interval_pending_slow_down_and_timeout_backoff ... ok
test state::tests::an_expired_state_is_not_redeemable_and_does_not_linger ... ok
test state::tests::expire_drops_only_what_is_past ... ok
test state::tests::contains_reports_liveness_without_redeeming ... ok
test token::tests::a_conforming_response_validates_and_drops_unretained_scopes ... ok
test state::tests::insert_sweeps_expired_entries_before_refusing ... ok
test state::tests::insert_refuses_once_the_table_is_full_of_live_entries ... ok
test token::tests::comma_separated_scopes_are_split_and_trimmed ... ok
test token::tests::expiry_is_measured_from_our_clock ... ok
test token::tests::jira_tolerates_an_absent_created_at_and_an_absent_refresh_on_rotation ... ok
test token::tests::optional_refresh_is_bounded_even_when_issuance_is_not_required ... ok
test token::tests::a_required_scope_must_survive_the_retain_filter ... ok
test state::tests::remove_returns_an_expired_entry_so_a_caller_can_refuse_rather_than_not_find ... ok
test state::tests::replacing_a_live_state_does_not_count_against_capacity ... ok
test token::tests::a_zero_expires_in_passes_only_where_the_caller_does_not_rely_on_it ... ok
test token::tests::refresh_is_due_inside_the_skew_and_whenever_the_clock_is_unavailable ... ok
test token::tests::personal_bearer_accepts_case_and_requires_bounded_finite_expiry ... ok
test token::tests::every_gitlab_condition_refuses ... ok

test result: ok. 33 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running unittests src/lib.rs (/dev/shm/b10x-cli-wave-wt-4f1de73d0685-nr00xjox/runtime-target/debug/deps/connector_resolve-1c82d48a134d0594)

running 54 tests
test auth::tests::a_basic_join_composes_the_pair_the_vendor_expects ... ok
test auth::tests::a_query_placement_registers_the_encoded_form_it_sends ... ok
test config::tests::a_config_value_debug_carries_no_value ... ok
test auth::tests::a_header_the_template_already_sets_is_refused_rather_than_overwritten ... ok
test config::tests::a_username_prefix_is_the_one_reserved_qualifier ... ok
test auth::tests::the_assembled_debug_prints_no_value ... ok
test auth::tests::a_prefixed_header_carries_the_bare_value_inside_it ... ok
test auth::tests::base64_matches_rfc_4648s_own_vectors ... ok
test auth::tests::an_inbound_signing_secret_never_leaves ... ok
test document::tests::a_sip_session_driver_survives_the_canonical_document ... ok
test document::tests::a_pre_c552_document_without_symbols_falls_back_to_the_allocation ... ok
test plan::tests::a_plans_debug_carries_no_credential ... ok
test request::tests::a_fragment_survives_the_appended_query ... ok
test document::tests::the_emitters_own_symbols_are_reserved ... ok
test request::tests::a_duplicate_query_key_is_refused_rather_than_sent_twice ... ok
test request::tests::the_default_identity_names_this_software_and_this_repository ... ok
test request::tests::debug_prints_shape_and_never_a_value ... ok
test document::tests::the_symbol_allocation_reproduces_the_emitters ... ok
test plan::tests::sensitive_text_never_prints ... ok
test document::tests::a_stated_symbol_is_honored_over_the_naive_allocation ... ok
test request::tests::the_params_omit_what_is_absent ... ok
test document::tests::source_fidelity_refuses_missing_unknown_profiles_and_old_catalog_versions ... ok
test resolve::tests::source_fidelity_encodes_path_values_without_changing_route_or_authority ... ok
test resolve::tests::source_fidelity_keeps_json_string_values_and_explicit_null ... ok
test resolve::tests::source_fidelity_preserves_body_omission_null_and_inner_requiredness ... ok
test document::tests::a_shipped_document_parses_into_its_services_and_operations ... ok
test resolve::tests::a_caller_parameter_that_leaves_its_path_segment_is_refused ... ok
test resolve::tests::a_caller_value_spelling_a_configuration_variable_does_not_reach_the_wire ... ok
test document::tests::an_unknown_provider_or_operation_is_absent_rather_than_a_panic ... ok
test slot::tests::a_document_position_maps_onto_a_slot_and_anything_else_fails_closed ... ok
test resolve::tests::a_configuration_value_that_moves_the_authority_is_refused ... ok
test resolve::tests::an_optional_query_filter_may_simply_be_left_out ... ok
test slot::tests::a_host_rule_refuses_what_moves_the_authority ... ok
test resolve::tests::an_omitted_required_parameter_is_refused_and_names_itself ... ok
test slot::tests::an_unplaced_value_is_held_to_every_rule_including_the_hosts ... ok
test slot::tests::a_value_that_moves_the_authority_is_refused_in_context ... ok
test resolve::tests::every_request_carries_this_softwares_identity ... ok
test resolve::tests::a_null_query_field_is_omitted_rather_than_sent_empty ... ok
test resolve::tests::an_omitted_optional_body_field_is_not_a_missing_parameter ... ok
test template::tests::a_doubled_brace_is_an_escape_and_an_unterminated_one_is_text ... ok
test template::tests::markers_are_located_and_filled_by_offset ... ok
test template::tests::value_to_text_is_flux_langs ... ok
test template::tests::an_unfilled_placeholder_stays_verbatim ... ok
test template::tests::truthiness_is_flux_langs ... ok
test resolve::tests::gitlab_publication_keeps_the_reviewed_actions_in_one_json_request ... ok
test resolve::tests::the_plan_carries_the_placed_credential_and_the_set_to_redact ... ok
test document::tests::an_operation_resolves_through_the_embedded_pack_without_naming_its_provider ... ok
test resolve::tests::the_request_is_the_documents_request ... ok
test resolve::tests::source_fidelity_integer_parameters_accept_json_number_spellings_without_fractional_wire_text ... ok
test resolve::tests::source_fidelity_gitlab_requests_preserve_complete_body_values_and_defaults ... ok
test credentials::tests::a_basic_join_with_no_user_half_refuses_by_name ... ok
test credentials::tests::a_basic_join_reads_its_user_half_from_the_config_port ... ok
test credentials::tests::an_unstored_credential_refuses_and_names_the_alternatives ... ok
test credentials::tests::a_bearer_credential_assembles_and_lists_its_redaction ... ok

test result: ok. 54 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.82s

     Running tests/adversary_gitlab_pass1.rs (/dev/shm/b10x-cli-wave-wt-4f1de73d0685-nr00xjox/runtime-target/debug/deps/adversary_gitlab_pass1-82347af6da0fc8a7)

running 4 tests
test adversary_gitlab_pass1_schema_versions_and_profiles_fail_closed ... ok
test current_schema_four_and_reader_keep_all_version_and_profile_refusal_classes ... ok
test adversary_gitlab_pass1_body_literals_do_not_become_template_instructions ... ok
test adversary_gitlab_pass1_path_and_integer_boundaries_preserve_caller_values ... ok

test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.57s

     Running unittests src/lib.rs (/dev/shm/b10x-cli-wave-wt-4f1de73d0685-nr00xjox/runtime-target/debug/deps/connector_spec-d9099b8159241cd4)

running 28 tests
test config::tests::a_host_value_is_refused_for_what_no_request_position_would_catch ... ok
test config::tests::bindings_parse_and_carry_their_level_and_secrecy ... ok
test config::tests::every_template_variable_is_reported_not_only_the_first ... ok
test config::tests::a_multi_destination_field_carries_one_slot_into_every_pin ... ok
test config::tests::a_pinned_request_value_parses_and_is_connection_level_configuration ... ok
test config::tests::a_pinned_value_cannot_reshape_the_request_it_lands_in ... ok
test config::tests::a_username_head_qualifies_the_placeholder_of_its_request_pin ... ok
test graph::tests::a_chain_orders_topologically ... ok
test graph::tests::a_cycle_has_no_order_at_all ... ok
test config::tests::formats_validate_the_values_they_claim ... ok
test graph::tests::a_diamond_converges_because_data_edges_need_no_nesting ... ok
test graph::tests::enclosing_walks_outwards_and_detects_a_containment_cycle ... ok
test graph::tests::comparisons_map_to_flux_operators ... ok
test graph::tests::region_and_boundary_kinds_are_classified ... ok
test inbound::tests::a_count_too_large_to_scale_is_refused_rather_than_wrapped ... ok
test inbound::tests::a_window_is_a_whole_number_of_seconds_minutes_or_hours ... ok
test inbound::tests::a_window_no_host_could_apply_is_not_a_window ... ok
test inbound::tests::an_unterminated_placeholder_is_reported_as_one_no_host_can_fill ... ok
test inbound::tests::every_payload_placeholder_is_one_the_host_can_fill ... ok
test inbound::tests::paths_reject_empty_segments_and_whitespace ... ok
test inbound::tests::signed_templates_report_their_placeholders_in_order ... ok
test inbound::tests::symbols_are_snake_case_because_a_hyphen_reads_as_subtraction ... ok
test names::tests::a_leading_digit_is_prefixed ... ok
test names::tests::two_names_that_normalize_alike_stay_distinct ... ok
test names::tests::an_empty_or_brace_bearing_name_is_refused ... ok
test names::tests::a_dotted_vendor_name_becomes_an_identifier ... ok
test names::tests::the_reserved_symbols_are_not_handed_out ... ok
test names::tests::an_identifier_safe_name_is_left_alone ... ok

test result: ok. 28 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests/main.rs (/dev/shm/b10x-cli-wave-wt-4f1de73d0685-nr00xjox/runtime-target/debug/deps/main-3ba4a7286c43068f)

running 518 tests
test auth_archetypes::a_public_client_is_exempt_from_the_client_secret_a_confidential_one_owes ... ok
test auth_hazard::a_credential_declaring_no_hazard_carries_none ... ok
test auth_archetypes::the_operator_level_is_expressible ... ok
test auth_hazard::the_near_miss_hazard_spelling_is_refused_naming_the_value ... ok
test auth_hazard::a_password_grant_that_declares_no_hazard_is_refused ... ok
test auth_hazard::a_grant_list_without_the_password_grant_needs_no_hazard ... ok
test auth_hazard::the_declared_hazard_is_the_word_the_consuming_deployment_gate_reads ... ok
test auth_prefix::a_declared_prefix_round_trips_through_the_encoding ... ok
test auth_prefix::a_header_placement_carries_an_arbitrary_scheme_word ... ok
test auth_prefix::a_prefix_may_carry_punctuation_and_still_be_a_prefix ... ok
test auth_prefix::a_scheme_word_that_is_not_oauth2_is_still_just_a_prefix ... ok
test auth_prefix::a_prefix_missing_its_trailing_separator_is_refused ... ok
test auth_prefix::repeated_punctuation_is_the_vendors_business_and_still_loads ... ok
test auth_prefix::a_prefix_may_not_spell_a_resolution_marker ... ok
test auth_prefix::a_prefix_may_not_break_out_of_the_header_value ... ok
test auth_prefix::a_prefix_may_not_end_in_an_alphanumeric_character ... ok
test auth_prefix::a_whitespace_only_prefix_is_refused ... ok
test auth_prefix::the_preset_schemes_carry_no_prefix_of_their_own ... ok
test auth_prefix::an_omitted_prefix_is_empty_and_does_not_serialize ... ok
test auth_prefix::a_prefix_may_not_carry_leading_or_doubled_whitespace ... ok
test auth_prefix::a_prefix_may_not_name_the_credential_or_its_env_var ... ok
test auth_prefix::there_is_no_suffix_axis ... ok
test auth_workarounds::a_token_endpoint_workaround_without_a_grant_is_refused ... ok
test auth_workarounds::a_workaround_measured_on_a_non_date_is_refused ... ok
test auth_workarounds::a_workaround_does_not_reach_a_sibling_credential_in_the_same_connector ... ok
test auth_workarounds::a_workaround_without_attribution_is_refused ... ok
test auth_workarounds::two_workarounds_for_one_grant_are_refused ... ok
test channel_bindings::a_binding_carrying_an_undeclared_event_is_refused ... ok
test channel_bindings::a_complete_binding_loads_and_composes_an_event_with_a_reply ... ok
test channel_bindings::a_channel_and_an_operation_may_not_share_a_name ... ok
test channel_bindings::a_duplicate_operation_id_is_reported_once_and_not_also_as_a_namespace_collision ... ok
test channel_bindings::a_cursor_on_a_transport_that_is_not_polled_is_refused ... ok
test channel_bindings::a_poll_binding_with_a_cursor_loads_and_may_omit_its_events ... ok
test channel_bindings::a_payload_source_path_with_an_empty_segment_is_refused ... ok
test channel_bindings::a_payload_key_that_is_not_a_flux_symbol_is_refused ... ok
test channel_bindings::a_poll_binding_without_a_cursor_is_refused ... ok
test channel_bindings::a_generic_socket_round_trips_every_connect_event_payload_and_config_fact ... ok
test channel_bindings::a_push_binding_carrying_no_events_is_refused ... ok
test channel_bindings::a_parameter_cannot_be_both_bound_and_the_journey_result ... ok
test channel_bindings::a_reply_binding_from_a_symbol_the_payload_never_produces_is_refused ... ok
test channel_bindings::a_reply_binding_a_parameter_the_operation_does_not_declare_is_refused ... ok
test channel_bindings::a_reply_leaving_a_required_parameter_unbound_is_refused ... ok
test auth_archetypes::raw_value_header_renders_the_same_form_as_a_bearer ... ok
test channel_bindings::a_reply_with_no_result_leaves_the_journey_output_parameter_unbound ... ok
test channel_bindings::a_reply_naming_an_operation_nobody_declares_is_refused ... ok
test channel_bindings::a_session_binding_loads_as_non_event_ingress_with_closed_admission_facts ... ok
test channel_bindings::a_signed_template_covering_only_the_url_is_refused ... ok
test channel_bindings::a_signed_template_that_never_interpolates_the_body_is_refused ... ok
test channel_bindings::a_signed_template_the_host_cannot_fill_is_refused ... ok
test channel_bindings::a_socket_binding_may_declare_manual_vendor_side_setup ... ok
test channel_bindings::a_timestamped_hmac_scheme_loads_with_its_window_and_its_selector ... ok
test channel_bindings::a_timestamped_scheme_without_a_timestamp_selector_is_refused ... ok
test channel_bindings::a_tolerance_that_is_not_a_duration_is_refused ... ok
test channel_bindings::a_timestamped_scheme_without_a_tolerance_is_refused ... ok
test channel_bindings::a_verification_timestamp_read_from_the_body_is_refused ... ok
test channel_bindings::a_tolerance_too_large_to_scale_is_refused_by_the_loader ... ok
test channel_bindings::a_webhook_binding_may_declare_itself_unverifiable_deliberately ... ok
test channel_bindings::a_webhook_binding_that_states_no_verification_is_refused ... ok
test channel_bindings::a_timestamp_format_loads_beside_its_selector ... ok
test channel_bindings::an_event_name_may_carry_the_vendors_own_dots_and_underscores ... ok
test channel_bindings::an_event_and_an_operation_may_not_share_a_name ... ok
test channel_bindings::a_webhook_secret_that_no_credential_declares_is_refused ... ok
test channel_bindings::an_event_name_that_could_not_travel_in_an_address_is_refused ... ok
test channel_bindings::a_webhook_secret_declared_as_an_outbound_credential_is_refused ... ok
test channel_bindings::every_member_kind_addresses_and_round_trips_through_one_oip_form ... ok
test channel_bindings::an_operation_cannot_authenticate_with_a_signing_credential ... ok
test channel_bindings::verification_on_a_transport_that_cannot_use_it_is_refused ... ok
test config_choices::a_config_field_declares_a_closed_set_of_values_and_a_value_outside_it_is_refused ... ok
test channel_bindings::twilios_url_and_sorted_form_scheme_is_declarable ... ok
test config_choices::a_pinned_field_checks_every_choice_against_its_request_position ... ok
test channel_bindings::session_ingress_refuses_missing_facts_and_event_only_fields ... ok
test config_choices::a_choice_must_be_renderable_and_distinct ... ok
test config_choices::an_example_outside_the_closed_set_is_refused ... ok
test config_choices::a_secret_field_cannot_declare_a_closed_set ... ok
test config_choices::every_permitted_value_still_satisfies_the_fields_format ... ok
test config_fields::a_binding_that_is_not_a_binding_at_all_is_refused ... ok
test config_fields::a_complete_configuration_surface_loads_and_derives_its_levels ... ok
test config_choices::a_set_with_one_value_is_refused_and_an_empty_one_is_an_open_field ... ok
test config_fields::a_connector_with_no_configuration_at_all_is_refused_when_it_needs_some ... ok
test config_fields::a_connector_with_a_literal_base_url_needs_no_endpoint_field ... ok
test config_fields::a_config_field_can_pin_a_path_segment ... ok
test auth_archetypes::basic_join_without_a_marker_is_a_distinct_form ... ok
test config_fields::a_credential_field_that_claims_not_to_be_secret_is_refused ... ok
test config_fields::a_field_binding_a_credential_nobody_declares_is_refused ... ok
test config_fields::a_destination_named_twice_is_refused ... ok
test config_fields::a_field_binding_a_template_variable_that_does_not_exist_is_refused ... ok
test config_fields::a_field_without_a_label_is_refused ... ok
test config_fields::a_credential_cannot_also_reach_a_request_position ... ok
test config_fields::a_field_without_help_is_refused ... ok
test config_fields::a_header_pin_on_an_auth_owned_header_is_refused ... ok
test config_fields::a_non_secret_field_may_still_declare_an_example ... ok
test config_fields::a_non_credential_field_that_claims_to_be_secret_is_refused ... ok
test config_fields::a_path_pin_no_operation_carries_is_refused ... ok
test channel_bindings::socket_connect_declarations_fail_closed_at_load ... ok
test config_fields::a_pin_parses_to_its_position_and_derives_its_level_and_secrecy ... ok
test config_fields::a_pin_that_claims_to_be_secret_is_refused ... ok
test config_fields::a_pinned_query_parameter_that_is_also_an_argument_is_refused ... ok
test config_fields::a_query_parameter_and_a_header_can_be_pinned_too ... ok
test config_fields::a_further_destination_that_is_not_a_request_position_is_refused ... ok
test config_fields::a_template_variable_nothing_binds_is_refused ... ok
test config_fields::a_path_pin_whose_example_escapes_its_segment_is_refused ... ok
test config_fields::a_value_that_composes_a_host_is_refused_when_it_could_move_the_authority ... ok
test config_fields::a_username_field_is_not_secret ... ok
test auth_archetypes::a_connector_with_no_credential_still_has_a_form ... ok
test config_fields::a_username_field_on_a_non_basic_credential_is_refused ... ok
test config_fields::a_verify_operation_loads_and_resolves ... ok
test config_fields::a_value_that_is_both_pinned_and_declared_as_a_parameter_is_refused ... ok
test config_fields::a_verify_operation_that_writes_is_refused ... ok
test config_fields::a_secret_field_that_declares_an_example_is_refused ... ok
test config_fields::an_example_that_fails_its_own_format_is_refused ... ok
test config_fields::an_oauth_field_without_an_oauth_credential_is_refused ... ok
test config_fields::config_names_join_the_shared_member_namespace ... ok
test config_fields::a_verify_operation_that_does_not_exist_is_refused ... ok
test config_fields::an_optional_pin_is_refused ... ok
test config_fields::every_permitted_choice_is_checked_against_every_destination ... ok
test config_fields::an_example_is_checked_against_every_destination_and_not_only_the_first ... ok
test config_fields::one_field_declares_two_destinations_and_one_value_reaches_both ... ok
test config_fields::the_username_placeholder_prefix_is_reserved_from_endpoint_fields ... ok
test config_fields::two_fields_writing_one_header_are_refused ... ok
test config_fields::two_fields_that_would_share_one_placeholder_are_refused ... ok
test constant_headers::a_constant_header_must_state_a_value ... ok
test constant_headers::a_constant_header_survives_the_ir_round_trip ... ok
test constant_headers::a_constant_header_value_may_not_carry_a_line_break ... ok
test constant_headers::a_provider_level_constant_header_is_distributed_onto_every_operation ... ok
test constant_headers::a_constant_header_name_must_be_an_http_field_name ... ok
test constant_headers::a_provider_level_refusal_is_reported_once ... ok
test constant_headers::an_operation_level_constant_header_may_not_carry_a_credential_either ... ok
test constant_headers::an_operation_without_constant_headers_encodes_as_it_always_did ... ok
test constant_headers::an_operations_own_constant_header_replaces_the_providers ... ok
test constant_headers::a_constant_header_may_not_carry_a_credential ... ok
test config_fields::the_atlassian_connectors_address_the_cloud_gateway_by_id ... ok
test credential_paths::a_path_from_another_convention_is_refused_rather_than_guessed_at ... ok
test credential_paths::an_explicitly_spelled_default_service_does_not_parse ... ok
test credential_paths::an_instance_that_is_not_a_uuid_is_refused_and_the_refusal_names_the_component ... ok
test credential_paths::an_instanced_path_cannot_be_confused_with_a_service ... ok
test config_choices::the_shipped_connectors_that_have_regions_declare_them ... ok
test config_fields::the_atlassian_connectors_prefer_their_service_account ... ok
test credential_paths::the_default_service_is_elided_and_the_elision_stays_unambiguous ... ok
test credential_paths::the_tenant_validator_is_public_so_a_host_can_check_before_it_builds ... ok
test credential_paths::every_admissible_reference_round_trips_and_no_rejected_one_renders ... ok
test channel_bindings::the_shipped_slack_bindings_describe_both_of_slacks_real_transports ... ok
test credential_response::a_credential_at_the_response_root_is_reachable ... ok
test credential_response::a_credential_inside_an_array_of_objects_is_reachable ... ok
test auth_archetypes::slack_has_distinct_bot_install_and_delegated_user_oauth_flows ... ok
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
test auth_archetypes::slack_bearers_keep_bot_user_admin_and_app_purposes_separate ... ok
test determinism::requirement_encoding_ignores_authoring_order ... ok
test determinism::serde_json_object_keys_stay_sorted ... ok
test discovery::discovery_cannot_name_a_nonexistent_operation ... ok
test discovery::duplicate_vendor_type_mapping_is_refused_instead_of_precedence_ordered ... ok
test discovery::one_read_can_declare_a_closed_native_provider_mapping ... ok
test determinism::repeated_serialization_is_stable ... ok
test execution_facts::a_seeded_write_remains_write_and_carries_write_effects ... ok
test execution_facts::audio_v1_is_a_closed_unary_device_driver ... ok
test execution_facts::effects_are_required_and_never_derived_from_http ... ok
test execution_facts::every_catalog_operation_requires_a_non_empty_description ... ok
test execution_facts::cdp_v1_is_a_closed_leased_session_browser_driver ... ok
test execution_facts::host_and_semantic_effects_remain_independent_axes ... ok
test execution_facts::predecessor_runtime_and_quirks_vocabularies_are_refused_by_name ... ok
test execution_facts::sip_v1_is_a_closed_session_establishment_driver ... ok
test execution_facts::sql_v1_is_a_closed_unary_database_driver ... ok
test execution_facts::unknown_effect_driver_and_capability_values_are_refused_by_name ... ok
test auth_archetypes::a_signing_secret_is_collected_like_any_credential_and_sent_nowhere ... ok
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
test graphs::the_worked_example_loads_and_composes_declared_members ... ok
test ir_roundtrip::a_hand_authored_toml_defines_a_complete_operation ... ok
test ir_roundtrip::an_exposed_operation_omits_only_the_default_exposure_field ... ok
test ir_roundtrip::an_unexposed_operation_reaches_the_hash_domain ... ok
test ir_roundtrip::auth_requirement_cardinalities_round_trip ... ok
test ir_roundtrip::auth_requirement_is_a_set ... ok
test ir_roundtrip::auth_scheme_matches_the_flux_plugin_protocol_vocabulary ... ok
test ir_roundtrip::body_encoding_is_closed_and_its_default_is_invisible ... ok
test ir_roundtrip::credentials_resolve_to_declared_auth_methods ... ok
test ir_roundtrip::operation_metadata_uses_the_flux_vocabulary ... ok
test ir_roundtrip::operation_traits_and_provenance_round_trip ... ok
test ir_roundtrip::parameter_and_response_schemas_survive_the_round_trip ... ok
test ir_roundtrip::rate_adversary_fixed_and_conditional_roundtrip_keep_distinct_meanings ... ok
test ir_roundtrip::rate_stage2_conditional_declarations_are_bounded_without_extending_fixed_rates ... ok
test ir_roundtrip::unset_and_explicit_empty_auth_differ_on_the_wire ... ok
test legacy_default_service::a_default_beside_a_named_service_without_the_legacy_marker_stays_refused ... ok
test legacy_default_service::a_legacy_marker_without_a_named_sibling_is_refused ... ok
test credential_paths::slack_derives_a_path_for_each_of_its_credentials ... ok
test legacy_default_service::a_named_service_cannot_claim_the_legacy_default_marker ... ok
test legacy_default_service::an_explicit_legacy_default_can_coexist_with_a_named_service ... ok
test legacy_default_service::every_spec_document_of_a_mixed_connector_must_state_its_service ... ok
test lockfile::a_changed_generator_moves_the_artifact_hashes_alone ... ok
test legacy_default_service::every_member_of_a_mixed_connector_must_state_its_service ... ok
test lockfile::a_changed_spec_moves_the_spec_hash ... ok
test lockfile::a_comment_only_edit_moves_the_toml_hash_alone ... ok
test lockfile::a_changed_toml_moves_the_toml_hash_and_nothing_upstream_of_it ... ok
test lockfile::a_rebuild_replaces_a_row_rather_than_duplicating_it ... ok
test lockfile::a_lockfile_from_another_format_version_is_refused ... ok
test lockfile::a_second_spec_hash_is_additive ... ok
test lockfile::an_unknown_key_is_refused ... ok
test lockfile::the_hash_domain_excludes_every_provenance_field ... ok
test lockfile::the_hash_domain_excludes_fetched_at ... ok
test lockfile::the_hash_domain_covers_the_compiled_meaning ... ok
test lockfile::the_lockfile_carries_no_credential_and_no_endpoint ... ok
test lockfile::the_lockfile_records_no_timestamp ... ok
test lockfile::the_pack_section_renders_and_round_trips ... ok
test lockfile::the_rendered_shape_is_pinned ... ok
test lockfile::the_hash_is_stable_across_repeated_computation ... ok
test oauth2_acquisition::a_plain_credential_declares_no_oauth2 ... ok
test oauth2_acquisition::a_redirect_uri_is_an_operator_level_registration_field ... ok
test oauth2_acquisition::a_redirect_uri_without_a_grant_is_refused ... ok
test oauth2_acquisition::a_scope_response_location_cannot_name_credential_material ... ok
test oauth2_acquisition::a_scope_response_location_must_be_a_json_pointer ... ok
test oauth2_acquisition::an_oauth2_credential_loads_with_every_field_intact ... ok
test oauth2_acquisition::declaring_both_an_oauth2_grant_and_a_minting_operation_is_refused ... ok
test oauth_token_endpoint::a_confidential_client_omits_the_discriminator ... ok
test oauth_token_endpoint::a_dangling_token_endpoint_is_refused_naming_it ... ok
test oauth_token_endpoint::a_public_client_loads_and_is_marked_public ... ok
test oauth_token_endpoint::a_two_host_declaration_loads_and_carries_both_services ... ok
test oauth_token_endpoint::an_absent_token_endpoint_is_skipped_entirely ... ok
test openapi_ingest::a_body_the_ir_cannot_express_skips_the_operation_rather_than_dropping_the_body ... ok
test openapi_ingest::a_cookie_parameter_skips_the_operation_rather_than_being_dropped_from_it ... ok
test openapi_ingest::a_cyclic_response_ref_is_bounded_rather_than_expanded_forever ... ok
test openapi_ingest::a_document_this_ingest_cannot_read_is_a_whole_document_error ... ok
test openapi_ingest::a_duplicate_operation_id_is_reported_rather_than_resolved_by_position ... ok
test openapi_ingest::a_malformed_endpoint_is_a_diagnostic_naming_it_rather_than_a_failed_ingest ... ok
test openapi_ingest::a_method_the_ir_cannot_spell_is_reported_rather_than_dropped_silently ... ok
test openapi_ingest::a_missing_section_is_a_diagnostic_naming_it ... ok
test openapi_ingest::a_path_items_parameters_reach_every_operation_under_it ... ok
test openapi_ingest::a_recursive_request_remains_an_exact_contract_and_is_refused ... ok
test openapi_ingest::a_recursive_response_is_bounded_without_dropping_the_operation ... ok
test openapi_ingest::a_yaml_document_ingests_including_its_integer_response_keys ... ok
test openapi_ingest::an_external_ref_is_refused_rather_than_followed ... ok
test credential_paths::the_three_outcomes_are_distinguishable ... ok
test openapi_ingest::an_object_request_body_becomes_named_body_parameters ... ok
test openapi_ingest::both_openapi_3_0_and_3_1_documents_ingest ... ok
test openapi_ingest::operation_order_does_not_depend_on_the_documents_key_order ... ok
test openapi_ingest::parameters_land_in_their_request_position_with_their_schemas ... ok
test openapi_ingest::refs_resolve_including_nested_and_repeated_ones ... ok
test openapi_ingest::ingest_is_deterministic ... ok
test openapi_ingest::source_fidelity_preserves_missing_item_constraints_and_literal_reference_data ... ok
test openapi_ingest::servers_carry_their_templating_and_their_variables ... ok
test openapi_ingest::source_fidelity_refuses_recursive_response_widening_and_nondefault_serialization ... ok
test lockfile::unchanged_inputs_reproduce_the_lockfile_byte_for_byte ... ok
test openapi_ingest::source_fidelity_reports_unsupported_semantics_without_a_permissive_substitute ... ok
test openapi_ingest::the_anthropic_spec_has_authored_provenance ... ok
test grafana::grafana_is_a_private_reachable_read_surface_with_connector_custody ... ok
test grafana::grafana_query_keeps_the_batch_bounded ... ok
test auth_archetypes::slack_socket_mode_token_enters_through_a_connect_session_and_never_ambient_env ... ok
test operation_selection::a_block_overrides_a_selectors_risk ... ok
test operation_selection::a_deferral_reason_must_be_nonempty ... ok
test operation_selection::a_deferred_operation_cannot_also_be_corrected ... ok
test operation_selection::a_path_prefix_matches_on_segment_boundaries ... ok
test operation_selection::a_per_operation_block_wins_over_a_selector ... ok
test operation_selection::a_bulk_conditional_still_owes_a_condition_per_operation ... ok
test operation_selection::a_pin_naming_an_absent_operation_id_is_refused ... ok
test operation_selection::a_read_may_go_unstated ... ok
test operation_selection::a_pin_overrides_the_rule_and_a_rename_overrides_the_pin ... ok
test operation_selection::a_selector_states_risk_and_idempotency_for_the_set ... ok
test operation_selection::an_internal_path_is_never_selected ... ok
test operation_selection::an_operation_id_that_cannot_produce_a_legal_name_is_reported ... ok
test operation_selection::an_upstream_operation_id_rename_orphans_direction_and_refuses ... ok
test operation_selection::changing_only_upstream_methods_before_composition_preserves_authored_directions ... ok
test credential_paths::two_instances_of_one_connector_for_one_tenant_render_different_addresses ... ok
test operation_selection::description_corrections_preserve_bulk_selection_and_document_order ... ok
test operation_selection::description_corrections_refuse_conflicting_exact_patches ... ok
test operation_selection::description_corrections_refuse_stale_identities_and_empty_values ... ok
test operation_selection::a_selector_states_exposure_for_the_set ... ok
test operation_selection::a_selector_matches_by_service_path_prefix_and_method ... ok
test operation_selection::a_selector_that_matches_nothing_is_refused ... ok
test operation_selection::a_spec_backed_provider_with_no_selector_publishes_nothing ... ok
test operation_selection::response_array_schema_rewrites_are_rejected_for_every_pointer ... ok
test operation_selection::response_array_schema_rewrites_cannot_wrap_valid_source_schemas ... ok
test operation_selection::deferring_an_operation_no_selector_matched_is_refused ... ok
test operation_selection::source_fidelity_preserves_whole_body_constraints_and_body_presence ... ok
test operation_selection::source_fidelity_refuses_schema_replacement_and_parameter_omission ... ok
test operation_selection::an_exact_deferral_withholds_one_selector_match ... ok
test operation_selection::overlapping_selectors_that_disagree_are_refused ... ok
test operation_selection::silence_on_an_authored_write_refuses ... ok
test operation_selection::there_is_no_hide_key ... ok
test config_fields::zendesk_declares_a_complete_connect_form ... ok
test operation_selection::overlapping_selectors_that_agree_are_accepted ... ok
test operation_selection::exposure_still_defaults_to_exposed ... ok
test operation_spec_source::an_inline_operation_cannot_author_the_derived_marker ... ok
test credential_paths::a_single_instance_address_is_byte_identical_to_the_four_component_form ... ok
test operator_pinned_origin::a_declared_origin_must_already_be_in_canonical_form ... ok
test operator_pinned_origin::an_open_origin_without_operator_approval_is_refused ... ok
test operator_pinned_origin::an_operator_pinned_origin_is_a_value_free_generic_config_declaration ... ok
test operator_pinned_origin::an_origin_accepts_only_an_absolute_https_origin_without_url_tail ... ok
test operator_pinned_origin::the_loader_accepts_exactly_the_canonical_origins_of_the_shared_corpus ... ok
test operation_selection::the_naming_rule_derives_the_declared_spelling ... ok
test operation_selection::the_derived_id_set_is_pinned ... ok
test operation_selection::two_operation_ids_deriving_one_op_id_refuse ... ok
test param_omission::a_correction_is_applied_before_the_omission_that_depends_on_it ... ok
test operation_selection::identical_inputs_produce_identical_ir ... ok
test auth_archetypes::basic_join_renders_two_fields_and_hides_the_vendor_marker ... ok
test param_omission::nothing_is_dropped_unless_the_patch_says_so ... ok
test param_omission::omitting_a_parameter_from_the_wrong_position_is_refused ... ok
test personal_oauth::explicit_personal_device_admission_survives_without_a_redirect_or_refresh_promise ... ok
test personal_oauth::explicit_personal_pkce_admission_survives_loading_without_changing_legacy_public_client ... ok
test personal_oauth::legacy_public_client_does_not_invent_personal_admission ... ok
test personal_oauth::oauth_pass1_explicit_empty_personal_admission_obeys_the_authoring_contract ... ok
test personal_oauth::oauth_pass2_presence_round_trips_across_formats_without_changing_legacy_arrays ... ok
test personal_oauth::personal_admission_refuses_ambiguous_flow_endpoints_evidence_and_registration_values ... ok
test produces_credential::a_credential_producing_operation_declares_the_handle_as_its_output ... ok
test produces_credential::a_produces_credential_location_naming_every_element_of_an_array_is_refused ... ok
test produces_credential::a_produces_credential_operation_declared_idempotent_is_refused ... ok
test produces_credential::a_produces_credential_operation_naming_no_secret_field_is_refused ... ok
test produces_credential::a_produces_credential_operation_on_a_connector_with_no_authority_is_refused ... ok
test produces_credential::a_produces_credential_operation_storing_an_undeclared_credential_is_refused ... ok
test produces_credential::a_produces_credential_operation_whose_response_schema_exposes_the_secret_is_refused ... ok
test produces_credential::an_operation_declaring_both_credential_declarations_is_refused_naming_which_governs ... ok
test param_omission::omitting_a_path_parameter_is_refused ... ok
test produces_credential::the_handle_field_is_the_word_the_runtime_answers_with ... ok
test produces_credential::two_operations_minting_one_credential_are_refused ... ok
test provider_schema::every_documented_object_forbids_additional_properties ... ok
test provider_schema::every_documented_object_lists_exactly_the_keys_the_loader_accepts ... ok
test provider_schema::every_ref_resolves_to_a_declared_def ... ok
test provider_schema::the_schema_documents_exactly_the_objects_the_loader_accepts ... ok
test provider_schema::the_schema_forbids_an_empty_auth_mechanism ... ok
test provider_schema::the_schema_is_published_and_self_describing ... ok
test provider_schema::the_schema_marks_the_mandatory_keys_required ... ok
test provider_schema::the_schema_publishes_the_exposure_default_the_loader_applies ... ok
test provider_schema::the_schema_publishes_the_loaders_own_repeatability_floor ... ok
test provider_schema::the_schema_states_the_custody_only_conditional ... ok
test provider_toml::a_basic_credential_can_state_a_literal_user_suffix ... ok
test provider_toml::a_file_may_point_at_a_spec_and_still_declare_operations_inline ... ok
test provider_toml::a_hand_authored_file_produces_a_complete_connector ... ok
test provider_toml::a_spec_pointer_file_produces_the_patch_set ... ok
test provider_toml::authoring_order_inside_a_mechanism_does_not_reach_the_ir ... ok
test provider_toml::the_provider_file_hash_is_recorded_and_is_a_function_of_the_bytes ... ok
test provider_toml::the_three_auth_states_survive_the_loader ... ok
test provider_toml::unstated_patch_overrides_stay_distinguishable_from_stated_ones ... ok
test provider_toml_errors::every_rejection_matches_its_golden_snapshot ... ok
test provider_toml_errors::every_required_rejection_has_a_fixture ... ok
test provider_toml_errors::no_snapshot_is_orphaned ... ok
test repeatability_condition_elision::an_operation_stating_a_condition_does_carry_it_into_the_hash_domain ... ok
test repeatability_condition_elision::an_operation_stating_no_condition_hashes_as_it_did_before_the_field_existed ... ok
test param_omission::omitting_a_parameter_the_document_does_not_declare_is_refused ... ok
test param_omission::the_curated_argument_list_comes_back_when_the_patch_names_what_to_drop ... ok
test param_omission::omitting_a_parameter_changes_only_the_parameters ... ok
test param_omission::omitting_a_required_parameter_is_refused ... ok
test credential_paths::several_instances_and_no_uuid_is_a_refusal_naming_what_would_have_worked ... ok
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
test operation_spec_source::a_fully_inline_provider_has_no_derived_operation_source ... ok
test service_partition::a_connector_without_an_authority_or_a_version_renders_no_address ... ok
test service_partition::a_connector_without_services_has_exactly_the_default_one ... ok
test service_partition::a_gid_with_more_than_one_service_segment_is_refused ... ok
test service_partition::a_malformed_address_is_refused_component_by_component ... ok
test operation_spec_source::mixed_zendesk_services_classify_each_operation_instead_of_the_service ... ok
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
test service_tags::a_repeated_tag_on_one_service_is_refused ... ok
test service_tags::a_service_may_carry_several_tags ... ok
test service_tags::an_unknown_tag_is_refused_and_names_the_known_set ... ok
test auth_workarounds::a_workaround_declared_on_one_connectors_auth_surface_does_not_reach_another ... ok
test service_partition::a_provider_file_that_loads_publishes_only_round_tripping_addresses ... ok
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
test auth_archetypes::and_sets_and_or_alternatives_are_the_grouping_a_form_renders ... ok
test services::omitting_the_service_in_a_multi_service_provider_is_refused ... ok
test services::stating_the_default_service_encodes_exactly_as_omitting_it ... ok
test services::the_service_key_walk_still_finds_an_ir_service_key_at_every_depth ... ok
test shared_endpoint_slot::a_sibling_service_placeholder_is_refused_when_nothing_shares_the_slot ... ok
test shared_endpoint_slot::a_sibling_service_that_does_not_exist_is_refused ... ok
test shared_endpoint_slot::listing_the_head_service_again_is_refused ... ok
test shared_endpoint_slot::one_field_fills_the_placeholder_of_a_sibling_service ... ok
test shared_endpoint_slot::sharing_a_non_endpoint_binding_is_refused ... ok
test config_fields::the_templated_providers_ask_for_their_tenant ... ok
test shipped_providers::an_operation_that_takes_nothing_composes_an_empty_object_schema ... ok
test shipped_providers::babelforce_is_bearer_only_and_never_the_deprecated_header_pair ... ok
test operation_selection::the_canonical_surface_is_selected_and_the_file_stays_reviewable ... ok
test shipped_providers::jira_added_writes_have_exact_authored_source_evidence ... ok
test operation_spec_source::a_patch_selected_operation_retains_its_exact_vendor_source ... ok
test produces_credential::no_shipped_operation_declares_produces_credential_yet ... ok
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
test spec_backed_provider::each_document_carries_its_own_provenance_and_its_own_hash_is_checked ... ok
test spec_backed_provider::everything_the_document_declares_stays_available_to_patch ... ok
test spec_backed_provider::inline_operations_and_selected_ones_land_in_one_connector ... ok
test spec_backed_provider::loading_a_spec_backed_provider_is_deterministic ... ok
test spec_backed_provider::one_documents_security_does_not_overwrite_the_others ... ok
test spec_backed_provider::one_operation_id_in_two_documents_is_two_operations ... ok
test spec_backed_provider::plain_load_refuses_a_spec_backed_file_rather_than_returning_a_skeleton ... ok
test spec_backed_provider::selecting_one_operation_twice_from_one_document_is_still_refused ... ok
test spec_backed_provider::several_documents_each_become_one_service ... ok
test spec_backed_provider::the_pinned_document_is_compiled_even_when_a_later_one_sits_beside_it ... ok
test spec_backed_provider::two_documents_may_not_join_one_service ... ok
test strict_fields::a_credential_must_declare_its_scheme ... ok
test strict_fields::a_typoed_credential_env_key_is_rejected ... ok
test strict_fields::a_typoed_operation_auth_key_is_rejected ... ok
test strict_fields::an_operation_must_declare_its_vendor_state_direction ... ok
test strict_fields::strictness_does_not_break_the_ir_round_trip ... ok
test strict_fields::typoed_keys_are_rejected_at_every_nesting_depth ... ok
test vendored_specs::a_provenance_entry_is_spec_source_shaped_and_names_no_internal_url ... ok
test vendored_specs::every_url_in_a_vendored_document_points_at_a_public_host ... ok
test vendored_specs::no_credential_shaped_example_value_survives ... ok
test vendored_specs::no_internal_marker_survives_in_a_vendored_document ... ok
test vendored_specs::no_personal_identity_survives_in_a_vendored_document ... ok
test vendored_specs::no_pull_configuration_is_vendored ... ok
test vendored_specs::no_scrubbed_literal_can_ever_reappear ... ok
test vendored_specs::provenance_records_every_vendored_document_and_its_hash_matches ... ok
test vendored_specs::the_declarations_survive_the_scrub ... ok
test vendored_specs::the_five_babelforce_documents_are_vendored ... ok
test verification_conformance::a_body_sourced_verification_timestamp_does_not_load ... ok
test service_tags::some_shipped_provider_has_services_whose_tags_diverge ... ok
test response_schema_coverage::response_schema_coverage_does_not_fall_below_its_floor ... ok
test response_schema_coverage::a_connector_arriving_with_no_response_shapes_is_caught ... ok
test shipped_providers::operation_ids_are_declarable_in_flux ... ok
test response_schema_coverage::no_operation_publishes_a_permissive_response_schema ... ok
test response_schema_coverage::the_recorded_floor_is_the_measured_figure ... ok
test verification_conformance::comparison_examines_every_byte_wherever_they_differ ... ok
test verification_conformance::a_transport_outside_the_matrix_declares_that_it_cannot_verify ... ok
test verification_conformance::reserializing_the_json_body_breaks_verification ... ok
test verification_conformance::the_hmac_primitive_matches_rfc_4231 ... ok
test verification_conformance::the_hmac_sha1_primitive_matches_rfc_2202 ... ok
test verification_conformance::the_declared_timestamp_format_is_read_instead_of_sniffed ... ok
test verification_conformance::the_reassembled_form_is_a_derivation_of_the_body_and_not_its_bytes ... ok
test credential_response::no_withheld_operation_is_in_the_shipped_catalogue ... ok
test shipped_providers::every_operation_composes_an_input_schema_covering_its_parameters ... ok
test config_fields::no_shipped_provider_has_an_unbound_template_variable ... ok
test shipped_providers::every_shipped_provider_loads ... ok
test services::every_shipped_service_is_spellable_and_a_single_service_provider_declares_none ... ok
test config_fields::no_shipped_provider_gives_a_secret_field_an_example ... ok
test service_audiences::the_seeded_fleet_has_a_useful_cross_function_vocabulary ... ok
test shipped_providers::no_provider_file_carries_a_credential_value ... ok
test service_tags::the_shipped_fleet_uses_several_distinct_tags ... ok
test response_schema_coverage::the_recorded_ceiling_is_the_measured_absence ... ok
test credential_paths::every_shipped_provider_declares_an_authority_and_renders_a_credential_path ... ok
test auth_archetypes::every_oauth_connector_generates_the_operator_connection_split ... ok
test credential_paths::every_shipped_credential_is_prefixed_with_its_connector_id ... ok
test verification_conformance::a_tolerance_no_host_could_apply_does_not_load ... ok
test verification_conformance::a_signed_template_that_covers_only_the_url_verifies_a_forged_payload ... ok
test verification_conformance::a_signed_template_that_omits_the_body_verifies_a_forged_payload ... ok
test verification_conformance::a_signature_outside_its_window_is_refused ... ok
test verification_conformance::vendor_signature_vectors_verify ... ok
test verification_conformance::every_shipped_hmac_scheme_is_covered_by_the_matrix ... ok

test result: ok. 518 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 26.13s

     Running unittests src/lib.rs (/dev/shm/b10x-cli-wave-wt-4f1de73d0685-nr00xjox/runtime-target/debug/deps/connectors_client-40f8b4d95ea43095)

running 37 tests
test identity::tests::mcp_invocation_uses_only_the_invoke_scope ... ok
test identity::tests::keyring_account_contains_no_endpoint_or_principal ... ok
test identity::tests::hosted_request_families_select_the_smallest_available_scope ... ok
test hosted_catalog::tests::posts_and_validates_a_catalog_frame ... ok
test personal_oauth::personal_oauth_tests::personal_instruction_destination_is_exact_numeric_loopback_and_capability_is_header_only ... ok
test personal_oauth::personal_oauth_tests::personal_instruction_parser_preserves_optional_device_uri_and_refuses_origin_changes ... ok
test admin::tests::identity_pkce_exchange_returns_only_the_exact_access_credential ... ok
test admin::tests::named_resources_are_typed_and_the_value_is_not_exposed ... ok
test personal_oauth::personal_oauth_tests::actual_private_instruction_request_keeps_capability_out_of_url_and_delivers_only_to_human_writer ... ok
test personal_oauth::personal_oauth_tests::personal_create_refusal_cannot_echo_private_daemon_text ... ok
test personal_oauth::personal_oauth_tests::actual_instruction_redirect_and_cacheable_reply_are_closed_refusals ... ok
test personal_oauth::personal_oauth_tests::private_browser_authorization_is_never_a_provider_independent_redirect ... ok
test personal_oauth::personal_oauth_tests::personal_success_refuses_a_changed_deadline ... ok
test personal_oauth::personal_oauth_tests::valid_browser_only_session_reaches_the_explicit_personal_handoff ... ok
test tests::completion_endpoint_is_validated_before_secret_submission ... ok
test personal_oauth::personal_oauth_tests::personal_success_refuses_a_private_target_repeated_consistently ... ok
test personal_oauth::personal_oauth_tests::personal_success_refuses_another_integration_status ... ok
test personal_oauth::personal_oauth_tests::personal_poll_and_describe_refusals_close_inner_transport_and_daemon_text ... ok
test personal_oauth::personal_oauth_tests::personal_success_refuses_another_session ... ok
test personal_oauth::personal_oauth_tests::personal_success_refuses_another_credential_purpose ... ok
test git_fetch_client::tests::response_is_bound_to_the_request_and_source_authority_is_redacted ... ok
test tests::hosted_client_posts_and_validates_a_datasource_frame ... ok
test tests::hosted_client_posts_the_same_typed_operation_frame ... ok
test personal_oauth::personal_oauth_tests::personal_success_refuses_created_description ... ok
test personal_oauth::personal_oauth_tests::personal_success_refuses_another_describe_target ... ok
test tests::hosted_client_requires_https_except_on_loopback_or_internal_cluster_dns ... ok
test personal_oauth::personal_oauth_tests::personal_success_retains_a_callable_correlated_description ... ok
test tests::local_client_frames_and_correlates_an_operation ... ok
test tests::hosted_subscription_client_refuses_a_cacheable_credential_boundary ... ok
test personal_oauth::personal_oauth_tests::personal_success_refuses_degraded_description ... ok
test tests::hosted_subscription_client_redacts_and_redeems_one_attempt_capability ... ok
test tests::hosted_subscription_client_starts_and_completes_pkce_without_retaining_the_code ... ok
test personal_oauth::personal_oauth_tests::expired_monotonic_instruction_budget_never_connects_even_with_future_wall_deadline ... ok
test response::tests::rate_stage2_hosted_client_never_resends_after_any_received_refusal_or_invalid_reply ... ok
test identity::tests::login_separates_the_session_and_refreshes_exact_scope_tokens ... ok
test personal_oauth::personal_oauth_tests::actual_connection_v1_polling_keeps_guarded_completion_private_and_never_repeats_create ... ok
test response::tests::rate_stage2_local_client_never_resends_after_any_received_refusal_or_invalid_reply ... ok

test result: ok. 37 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.28s

     Running tests/personal_oauth_adversary.rs (/dev/shm/b10x-cli-wave-wt-4f1de73d0685-nr00xjox/runtime-target/debug/deps/personal_oauth_adversary-381db7932e982540)

running 2 tests
test oauth_pass1_public_instruction_refusals_do_not_redirect_poll_or_echo_private_bytes ... ok
test oauth_pass1_public_handoff_waits_for_bound_callable_success_without_replay ... ok

test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.53s

     Running unittests src/lib.rs (/dev/shm/b10x-cli-wave-wt-4f1de73d0685-nr00xjox/runtime-target/debug/deps/protocol-97038e98e9ec497b)

running 37 tests
test approval::tests::approval_lifetime_is_bounded ... ok
test audio::tests::bounded_single_line_text_is_admitted_and_counted_in_characters ... ok
test audio::tests::empty_control_bearing_and_over_length_text_refuse ... ok
test audio::tests::the_input_refuses_any_field_a_caller_invents ... ok
test approval::tests::realm_is_not_an_approval_or_route_coordinate ... ok
test browser::tests::a_page_view_cannot_be_built_without_its_untrusted_content_label ... ok
test browser::tests::open_admits_an_absent_address_and_goto_does_not ... ok
test browser::tests::the_admitted_surface_carries_no_interaction_operation ... ok
test browser::tests::the_input_refuses_any_field_a_caller_invents ... ok
test browser::tests::only_ordinary_web_addresses_are_admitted ... ok
test catalog::tests::a_setup_profile_cannot_exist_without_a_setup_form ... ok
test catalog::tests::catalog_membership_carries_no_callability_or_credential_value ... ok
test connection::tests::materialization_accepts_only_an_opaque_observation_reference ... ok
test connection::tests::browser_completion_url_is_an_exact_loopback_capability ... ok
test connection::tests::candidates_are_value_free_and_activation_selects_no_route ... ok
test connection::tests::mediated_route_is_value_free_closed_and_cannot_self_reference ... ok
test connection::tests::secret_shaped_unknown_fields_are_refused ... ok
test connection::tests::pending_and_completed_sessions_cannot_mix_endpoint_and_connection ... ok
test datasource::tests::list_is_bounded_and_get_key_is_structured ... ok
test connection::tests::observations_are_value_free_and_lifecycle_consistent ... ok
test event::tests::cursor_and_wait_are_bounded ... ok
test datasource::tests::response_refuses_ambiguous_success_and_failure ... ok
test git_fetch::tests::response_refuses_secret_or_non_tls_locators ... ok
test operation::legacy::tests::a_terminal_status_cannot_omit_its_observed_reason ... ok
test git_fetch::tests::request_refuses_unbounded_or_ambiguous_revisions ... ok
test operation::legacy::tests::invoke_requires_a_description_lease_and_bounded_structured_input ... ok
test operation::legacy::tests::owner_context_is_not_defaultable ... ok
test sip::tests::a_number_alone_is_admitted_because_the_trunk_supplies_the_destination ... ok
test sip::tests::a_number_is_bounded_rather_than_truncated ... ok
test operation::legacy::tests::connection_audiences_are_bounded_discovery_metadata ... ok
test operation::legacy::tests::response_envelope_round_trips_and_refuses_unknown_fields ... ok
test voice::tests::fixture_context_cannot_claim_trust ... ok
test voice::tests::owner_vectors_are_closed_and_unique ... ok
test operation::legacy::tests::effect_bearing_operations_require_approval ... ok
test sip::tests::a_number_that_could_escape_the_uri_user_part_is_refused ... ok
test sip::tests::an_absent_field_is_omitted_from_the_wire_rather_than_sent_as_null ... ok
test sip::tests::aliases_admit_names_and_refuse_network_destinations ... ok

test result: ok. 37 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s

     Running tests/bundles.rs (/dev/shm/b10x-cli-wave-wt-4f1de73d0685-nr00xjox/runtime-target/debug/deps/bundles-f0b59fa93166e973)

running 25 tests
test connector_catalog_bundle_is_immutable ... ok
test connector_catalog_vectors_match_the_strict_reader ... ok
test connector_datasource_bundle_is_immutable ... ok
test connector_event_vectors_match_the_strict_reader ... ok
test connector_connection_bundle_is_immutable ... ok
test connector_datasource_vectors_match_the_strict_reader ... ok
test connector_connection_vectors_match_the_strict_reader ... ok
test connector_event_bundle_is_immutable ... ok
test connector_operation_vectors_match_the_strict_reader ... ok
test connector_operation_bundle_is_immutable ... ok
test kubernetes_service_route_round_trips_through_the_connection_response ... ok
test operation_v2_advice_preserves_all_categories_and_checks_the_interval ... ok
test operation_v2_projects_throttling_to_v1_without_optional_extensions ... ok
test operation_v2_only_rate_limited_carries_retry_delay ... ok
test rtvbp_binding_bundle_is_immutable ... ok
test owner_contract_bundle_is_immutable ... ok
test connector_operation_v2_bundle_is_immutable ... ok
test operation_version_reader_rejects_unknown_versions_and_preserves_authority_fields ... ok
test operation_version_decoder_refuses_duplicate_fields_before_dispatch ... ok
test rate_repair_source_uri_grammar_matches_wire_and_published_schema ... ok
test operation_v2_description_downgrade_loses_only_advice ... ok
test operation_v2_vectors_cover_every_request_result_and_error_variant ... ok
test operation_legacy_snapshot_preserves_deployed_schema_discrepancies ... ok
test operation_v2_complete_request_and_response_vectors_match_rust ... ok
test operation_v2_complete_request_and_response_vectors_match_schema ... ok

test result: ok. 25 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.04s

     Running tests/rate_adversary.rs (/dev/shm/b10x-cli-wave-wt-4f1de73d0685-nr00xjox/runtime-target/debug/deps/rate_adversary-735069fd71edcb71)

running 2 tests
test rate_adversary_published_schema_matches_source_url_reader ... ok
test rate_final_uri_composition_and_downgrade_validate_complete_frames ... ok

test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s

     Running unittests src/lib.rs (/dev/shm/b10x-cli-wave-wt-4f1de73d0685-nr00xjox/runtime-target/debug/deps/server-dd82a5540a2e86b9)

running 101 tests
test egress::tests::ipv4_mapped_ipv6_cannot_bypass_address_classification ... ok
test egress::tests::ambiguous_retry_after_is_not_flattened_into_advice ... ok
test egress::tests::egress_requires_a_nonempty_ascii_connection_or_session_reference ... ok
test egress::tests::exact_origin_cannot_be_widened_by_path_host_or_userinfo ... ok
test egress::tests::operator_network_may_admit_private_but_not_process_local_addresses ... ok
test egress::tests::cached_client_cannot_bypass_current_address_policy ... ok
test egress::tests::malformed_retry_after_does_not_hide_the_definite_provider_response ... ok
test egress::tests::suffix_rule_requires_a_real_child_and_the_exact_scheme_and_port ... ok
test egress::tests::retry_after_extraction_keeps_only_one_valid_decimal_and_admitted_headers ... ok
test hosted::enforcement::tests::an_issued_record_round_trips_without_its_reference_in_any_key ... ok
test hosted::principal::tests::lease_seeds_survive_the_verifier_token_rotation_composition ... ok
test egress::tests::public_dns_refuses_private_local_reserved_and_mixed_answers ... ok
test hosted::enforcement::tests::the_canonical_digest_ignores_member_order_and_nothing_else ... ok
test hosted::git_fetch::tests::rejected_source_authority_is_decided_before_body_or_broker_exchange ... ok
test hosted::git_fetch::tests::rejected_control_identity_is_decided_before_the_request_body_is_polled ... ok
test egress::tests::rate_stage2_definite_http_429_survives_oversized_or_broken_error_bodies ... ok
test hosted::tests::admin_routes::auth_metadata_is_public_and_selects_exact_authority ... ok
test egress::tests::reused_connection_keeps_authorization_and_timeout_request_specific ... ok
test hosted::git_fetch::tests::control_and_internal_routes_are_separate_and_non_cacheable ... ok
test hosted::git_fetch::tests::ambiguous_protocol_or_source_headers_are_refused_before_reading_the_body ... ok
test hosted::tests::admin_routes::operator_group_without_the_exact_scope_cannot_write ... ok
test hosted::tests::contract_validation::hosted_route_refuses_a_malformed_backend_contract ... ok
test hosted::tests::admin_routes::operator_can_write_and_status_never_returns_the_secret ... ok
test egress::rate_adversary_tests::rate_final_chunked_429_keeps_definite_status_without_body_or_untrusted_advice ... ok
test hosted::tests::contract_validation::rate_stage2_hosted_http_projects_refusals_once_and_rejects_invalid_frames_before_backend ... ok
test hosted::tests::a_human_issues_one_exact_input_approval_which_is_spent_once ... ok
test hosted::tests::contract_validation::rate_stage2_hosted_boundary_selects_response_version_even_for_early_refusals ... ok
test hosted::tests::contract_validation::rate_final_hosted_describe_versions_reject_bad_advice_before_loss ... ok
test hosted::tests::contract_validation::rate_stage2_invalid_http_correlation_is_a_bounded_versioned_client_refusal ... ok
test egress::tests::pool_is_bounded_and_separates_current_addresses_authorities_origins_and_policy ... ok
test egress::rate_adversary_tests::rate_adversary_header_cardinality_and_unfinished_body_are_separate ... ok
test hosted::tests::enforcement::a_granted_effect_without_approval_demand_dispatches_on_the_grant_alone ... ok
test hosted::tests::enforcement::a_granted_mutation_demanding_approval_refuses_without_one ... ok
test hosted::tests::enforcement::a_granted_mutation_with_a_demanded_approval_dispatches_with_one ... ok
test hosted::tests::enforcement::a_mutation_with_no_admitting_grant_refuses ... ok
test hosted::tests::enforcement::a_second_presentation_of_the_same_approval_refuses_and_journals_replay ... ok
test hosted::tests::enforcement::an_unbound_grant_store_is_an_outage_for_effects_only ... ok
test hosted::tests::hosted_completion_failures_are_non_cacheable_and_browser_hardened ... ok
test hosted::tests::hosted_completion_streams_fragments_into_a_redacted_bounded_submission ... ok
test hosted::tests::hosted_connection_route_uses_the_same_identity_boundary ... ok
test hosted::tests::enforcement::the_read_only_path_is_unchanged_for_callers_without_grants ... ok
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
test hosted::tests::docs::every_documented_refusal_example_names_a_real_error_code ... ok
test hosted::tests::mcp::datasource_backed_tools_route_through_the_datasource_seam ... ok
test hosted::tests::mcp::initialize_echoes_admitted_revisions_and_answers_ping ... ok
test hosted::tests::docs::every_documented_request_example_is_accepted_by_its_protocol_type ... ok
test hosted::tests::docs::a_request_example_with_an_unknown_field_is_refused ... ok
test hosted::tests::mcp::the_mcp_route_is_stateless_post_only ... ok
test hosted::tests::mcp::tools_list_returns_exactly_the_three_meta_tools ... ok
test hosted::tests::mcp::rate_stage2_mcp_preserves_refusal_details_without_entering_stale_retry ... ok
test hosted::tests::docs::the_document_pins_the_exact_wire_contract_identities_and_audience ... ok
test hosted::tests::mcp::tool_search_projects_only_the_entries_the_callers_seam_results_support ... ok
test hosted::tests::docs::the_docs_page_is_served_unauthenticated_as_html ... ok
test hosted::tests::mcp::tool_describe_projects_the_underlying_description_without_a_lease ... ok
test hosted::tests::mcp::transport_refusals_carry_the_designed_statuses_and_codes ... ok
test hosted::tests::mcp_monitoring::a_stale_monitoring_invoke_re_resolves_the_same_target_exactly_once ... ok
test hosted::tests::mcp_monitoring::a_monitoring_invoke_refuses_dishonest_targets_before_any_dispatch ... ok
test hosted::tests::monitoring_transport_gate_admits_only_configured_groups_or_operator ... ok
test hosted::tests::pod_log_transport_gate_admits_only_kubernetes_read_groups_or_operator ... ok
test hosted::tests::mcp_monitoring::a_monitoring_invoke_routes_the_chosen_target_through_the_decided_seam ... ok
test hosted::tests::self_event_scope_is_closed_to_slack_specific_requests ... ok
test hosted::tests::docs::every_documented_mcp_request_example_is_answered_by_the_live_transport ... ok
test hosted::tests::mcp::rate_adversary_mcp_stale_then_rate_stops_without_losing_large_delay ... ok
test hosted::tests::docs::the_docs_page_links_the_contract_and_renders_its_version ... ok
test hosted::tests::production_router_publishes_client_discovery_without_authentication ... ok
test hosted::tests::docs::the_docs_page_makes_zero_external_requests ... ok
test hosted::tests::signal::a_granted_session_signal_dispatches_behind_the_sessions_grant ... ok
test hosted::tests::tenant_members_receive_only_read_only_module_invocation ... ok
test hosted::tests::signal::an_effect_bearing_session_signal_without_an_admitting_grant_refuses ... ok
test local::tests::a_broad_state_directory_refuses_without_repair ... ok
test hosted::tests::subscription_oauth_start_is_identity_scoped_bounded_and_non_cacheable ... ok
test hosted::tests::mcp_monitoring::the_acceptance_sequence_invokes_with_a_target_and_integer_epochs ... ok
test hosted::tests::signal::an_unbound_grant_store_is_an_outage_for_session_signals ... ok
test hosted::tests::docs::the_docs_page_refusal_table_carries_every_documented_code ... ok
test hosted::tests::subscription_credential_stays_in_custody_and_only_an_exact_lease_redeems ... ok
test hosted::tests::docs::every_docs_page_example_is_the_documents_example_after_json_normalization ... ok
test local::tests::a_second_daemon_cannot_unlink_the_live_daemons_socket ... ok
test local::tests::one_socket_dispatches_the_value_free_connection_and_event_contracts ... ok
test local::tests::owner_socket_serves_one_strict_bounded_operation_frame ... ok
test local::tests::rate_stage2_actual_socket_serves_both_versions_without_resending ... ok
test hosted::tests::signal::a_signal_refusal_matches_the_invoke_refusal_bytes ... ok
test hosted::tests::mcp_monitoring::tool_describe_enumerates_the_callers_configured_targets_without_a_lease ... ok
test hosted::tests::mcp_monitoring::tool_search_lists_the_monitoring_tools_for_a_monitoring_read_principal ... ok
test hosted::tests::mcp_monitoring::monitoring_tool_schemas_state_the_documents_contract ... ok
test hosted::tests::mcp::a_pathological_namespace_is_cut_with_an_explicit_truncation_marker ... ok
test catalog_projection::tests::a_deployment_publishes_only_the_setup_flows_it_can_complete ... ok
test catalog_projection::tests::search_is_whole_catalog_and_describe_is_descriptive_only ... ok
test catalog_projection::tests::platform_provider_satisfies_the_catalog_wire_contract ... ok
test catalog_projection::tests::every_shipped_provider_description_satisfies_the_catalog_wire_contract ... ok
test hosted::tests::hosted_liveness_and_identity_backed_readiness_are_distinct ... ok
test hosted::tests::hosted_liveness_stays_local_when_a_backend_dependency_is_unready ... ok
test hosted::tests::docs::every_documented_route_exists_in_the_real_router ... ok

test result: ok. 101 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.84s

     Running tests/rate_adversary_local.rs (/dev/shm/b10x-cli-wave-wt-4f1de73d0685-nr00xjox/runtime-target/debug/deps/rate_adversary_local-e9ec62baa6625a53)

running 2 tests
test rate_final_local_describe_validates_before_version_loss ... ok
test rate_adversary_local_versions_validate_before_single_dispatch ... ok

test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running unittests src/lib.rs (/dev/shm/b10x-cli-wave-wt-4f1de73d0685-nr00xjox/runtime-target/debug/deps/service-3731ef93ea80f9c6)

running 64 tests
test audio::tests::a_route_for_another_connection_is_refused ... ok
test audio::tests::a_plan_for_another_driver_or_operation_is_refused ... ok
test audio::tests::a_speech_plan_and_its_deployment_route_admit_together ... ok
test admin::tests::status_reports_presence_without_reading_the_value ... ok
test audio::tests::status_never_admits_an_utterance ... ok
test admin::tests::write_requires_explicit_replacement_and_audits_no_secret ... ok
test audio::tests::admitted_evidence_never_prints_as_a_serializable_route_secret ... ok
test audio::tests::relative_paths_absent_bounds_and_bad_digests_never_reach_the_device ... ok
test audio::tests::the_caller_text_is_bounded_by_the_deployment_and_not_only_by_the_catalog ... ok
test browser::tests::a_non_web_address_is_refused_before_any_browser_is_touched ... ok
test browser::tests::a_route_for_another_connection_is_refused ... ok
test browser::tests::a_plan_for_another_driver_or_operation_is_refused ... ok
test authority::tests::debug_never_prints_authority_or_proof ... ok
test browser::tests::admitted_evidence_never_prints_as_a_serializable_route_secret ... ok
test authority::tests::proof_is_bound_to_exact_upgrade_uri ... ok
test browser::tests::only_open_may_omit_an_address_and_only_open_or_goto_may_carry_one ... ok
test connect_session::tests::authority_and_claim_use_one_receiver_owned_instant ... ok
test connect_session::tests::clock_failure_refuses_authorization_but_does_not_prevent_confirmed_abort ... ok
test connect_session::tests::shutdown_fails_only_pending_sessions_and_returns_their_endpoints ... ok
test connect_session::tests::preparing_and_uncertain_abort_cannot_publish_a_terminal_result ... ok
test authority::tests::session_lease_cannot_be_extended_by_authority_clock_skew ... ok
test connect_session::tests::terminal_transition_is_one_way_and_value_free ... ok
test authority::tests::serving_endpoint_redeems_once ... ok
test browser::tests::every_admitted_browser_operation_and_its_route_admit_together ... ok
test connect_session::tests::the_claim_rechecks_the_original_target_and_inclusive_deadline_once ... ok
test connect_session::tests::capacity_counts_only_pending_sessions ... ok
test authority::tests::audience_expiry_and_revocation_fail_before_redemption ... ok
test connect_session::tests::a_completion_claim_blocks_expiry_shutdown_and_duplicate_completion ... ok
test browser::tests::the_operators_own_browser_profile_is_never_an_admitted_route ... ok
test dispatch::tests::composition_order_is_policy_redaction_audit_driver_audit ... ok
test egress::tests::retry_delay_is_one_unsigned_decimal_with_only_http_whitespace ... ok
test connect_session::tests::guarded_sessions_keep_capacity_until_a_confirmed_outcome ... ok
test planning::tests::bidirectional_connection_still_requires_the_operation_admission ... ok
test planning::tests::missing_driver_refuses_before_dispatch ... ok
test browser::tests::a_unary_lifecycle_is_refused_because_a_browser_spans_calls ... ok
test planning::tests::mediated_http_plan_has_no_direct_origin_and_requires_the_closed_adapter ... ok
test planning::tests::sip_plan_has_no_http_fields ... ok
test runtime::tests::hosted_completion_submission_never_reallocates_received_secret_material ... ok
test runtime::tests::delegated_execution_must_match_the_authenticated_actor ... ok
test runtime::tests::the_stable_authority_seed_distinguishes_absent_and_literal_default_realms ... ok
test runtime::tests::the_stable_authority_seed_ignores_request_scoped_provenance ... ok
test runtime::tests::the_stable_authority_seed_survives_token_scoped_snapshot_fields ... ok
test sip::tests::a_default_naming_an_absent_trunk_is_refused_when_the_table_is_built ... ok
test sip::tests::a_dial_with_no_alias_and_no_default_is_refused_rather_than_guessed ... ok
test sip::tests::a_dial_with_only_a_number_takes_the_declared_default_trunk ... ok
test sip::tests::a_named_trunk_is_resolved_before_admission_so_the_answer_is_aperture_checked ... ok
test runtime::tests::hosted_principals_do_not_fabricate_agent_revisions ... ok
test sip::tests::a_prefix_aperture_admits_its_network_and_refuses_outside_it ... ok
test planning::tests::provider_only_connection_refuses_a_caller_initiated_operation ... ok
test sip::tests::a_partial_byte_prefix_masks_only_the_bits_it_declares ... ok
test browser::tests::relative_paths_nested_artifacts_and_absent_bounds_never_reach_the_browser ... ok
test runtime::tests::local_principals_require_a_real_agent_revision ... ok
test sip::tests::a_prefix_that_is_not_on_its_own_boundary_is_refused_rather_than_rounded ... ok
test sip::tests::a_trunk_that_does_not_admit_numbers_refuses_one ... ok
test sip::tests::an_exact_aperture_still_admits_exactly_one_address ... ok
test sip::tests::exact_loopback_route_produces_non_serializable_driver_evidence ... ok
test sip::tests::missing_organization_in_grant_evidence_refuses_before_the_driver ... ok
test sip::tests::an_aperture_never_matches_across_address_families ... ok
test sip::tests::the_dialled_host_comes_from_the_trunk_and_never_from_the_caller ... ok
test sip::tests::sip_dial_resolves_only_an_exact_connection_owned_alias ... ok
test sip::tests::stable_network_and_aperture_widening_refuse_before_the_driver ... ok
test voice::tests::one_proof_joins_exact_sip_and_application_routes ... ok
test sip::tests::zero_or_excessive_dial_deadlines_refuse_before_the_driver ... ok
test voice::tests::invalid_endpoint_and_authority_windows_refuse_before_io ... ok

test result: ok. 64 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

   Doc-tests catalog

running 1 test
test crates/catalog/src/lib.rs - (line 9) ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.88s

   Doc-tests catalog_build

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

   Doc-tests catalog_reader

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

   Doc-tests connector_oauth

running 3 tests
test crates/connector-oauth/src/device.rs - device::DevicePoll (line 164) - compile fail ... ok
test crates/connector-oauth/src/device.rs - device::DeviceAuthorization (line 40) - compile fail ... ok
test crates/connector-oauth/src/device.rs - device::DeviceResponse (line 8) - compile fail ... ok

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
test crates/connectors-client/src/model.rs - model::PersonalOAuthInstructions (line 338) - compile fail ... ok
test crates/connectors-client/src/model.rs - model::PendingPersonalOAuth (line 312) - compile fail ... ok

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

````

Exit 0; 1106 passed / 0 failed / 1 ignored. Guard interrupted: false.

Execution root-clippy; started 2026-09-06T18:40:56.422200+00:00; finished 2026-09-06T18:41:01.740913+00:00. Exact command/cwd/environment:

````json
{
  "cwd": "~/.local/state/worktree/trees/b10x/connectors/wt-4f1de73d0685",
  "argv": [
    "env",
    "-u",
    "RUSTC_WRAPPER",
    "CARGO_TARGET_DIR=/dev/shm/b10x-cli-wave-wt-4f1de73d0685-nr00xjox/runtime-target",
    "TMPDIR=~/.cache/cw6/oa2",
    "CARGO_BUILD_JOBS=1",
    "CARGO_INCREMENTAL=0",
    "CARGO_PROFILE_DEV_DEBUG=0",
    "CARGO_PROFILE_TEST_DEBUG=0",
    "cargo",
    "clippy",
    "--locked",
    "--offline",
    "-p",
    "connector-spec",
    "-p",
    "connector-resolve",
    "-p",
    "catalog",
    "-p",
    "catalog-build",
    "-p",
    "catalog-reader",
    "-p",
    "connectors-client",
    "-p",
    "protocol",
    "-p",
    "server",
    "-p",
    "service",
    "-p",
    "connector-oauth",
    "--all-targets",
    "--",
    "-D",
    "warnings"
  ]
}
````

Actual complete output from ~/.cache/connectors-cli-wave-20260906/connect-session-oauth-custody-in-personal-posture/whole-unit-adversary-2/oauth-pass2-root-clippy.log:

````text
    Checking catalog-build v0.6.5 (~/.local/state/worktree/trees/b10x/connectors/wt-4f1de73d0685/crates/catalog-build)
    Checking connector-spec v0.6.5 (~/.local/state/worktree/trees/b10x/connectors/wt-4f1de73d0685/crates/connector-spec)
    Finished `dev` profile [unoptimized] target(s) in 2.03s
````

Exit 0; 0 passed / 0 failed / 0 ignored. Guard interrupted: false.

Execution root-fmt; started 2026-09-06T18:41:14.886028+00:00; finished 2026-09-06T18:41:20.181550+00:00. Exact command/cwd/environment:

````json
{
  "cwd": "~/.local/state/worktree/trees/b10x/connectors/wt-4f1de73d0685",
  "argv": [
    "env",
    "-u",
    "RUSTC_WRAPPER",
    "CARGO_TARGET_DIR=/dev/shm/b10x-cli-wave-wt-4f1de73d0685-nr00xjox/runtime-target",
    "TMPDIR=~/.cache/cw6/oa2",
    "CARGO_BUILD_JOBS=1",
    "CARGO_INCREMENTAL=0",
    "CARGO_PROFILE_DEV_DEBUG=0",
    "CARGO_PROFILE_TEST_DEBUG=0",
    "cargo",
    "fmt",
    "--all",
    "--",
    "--check"
  ]
}
````

Actual complete output from ~/.cache/connectors-cli-wave-20260906/connect-session-oauth-custody-in-personal-posture/whole-unit-adversary-2/oauth-pass2-root-fmt.log:

````text

````

Exit 0; 0 passed / 0 failed / 0 ignored. Guard interrupted: false.

Execution runtime-full; started 2026-09-06T18:41:36.907198+00:00; finished 2026-09-06T18:42:02.846435+00:00. Exact command/cwd/environment:

````json
{
  "cwd": "~/.local/state/worktree/trees/b10x/connectors/wt-4f1de73d0685/crates/connectors-runtime",
  "argv": [
    "env",
    "-u",
    "RUSTC_WRAPPER",
    "CARGO_TARGET_DIR=/dev/shm/b10x-cli-wave-wt-4f1de73d0685-nr00xjox/runtime-target",
    "TMPDIR=~/.cache/cw6/oa2",
    "CARGO_BUILD_JOBS=1",
    "CARGO_INCREMENTAL=0",
    "CARGO_PROFILE_DEV_DEBUG=0",
    "CARGO_PROFILE_TEST_DEBUG=0",
    "cargo",
    "test",
    "--locked",
    "--offline",
    "-p",
    "connectors-config",
    "-p",
    "connect-session-transport",
    "-p",
    "integration-catalog",
    "-p",
    "connectors-runtime",
    "-p",
    "state-sqlite",
    "--no-fail-fast"
  ]
}
````

Actual complete output from ~/.cache/connectors-cli-wave-20260906/connect-session-oauth-custody-in-personal-posture/whole-unit-adversary-2/oauth-pass2-runtime-full.log:

````text
warning: connector-secrets@0.6.5: no live Vault was offered (CONNECTOR_SECRETS_VAULT_ADDR and CONNECTOR_SECRETS_VAULT_TOKEN are unset), so the reqwest HttpTransport is UNEXERCISED by this build: tests/vault_live.rs is compiled #[ignore]d
   Compiling integration-catalog v0.6.5 (~/.local/state/worktree/trees/b10x/connectors/wt-4f1de73d0685/crates/integration-catalog)
    Finished `test` profile [unoptimized] target(s) in 2.99s
     Running unittests src/lib.rs (/dev/shm/b10x-cli-wave-wt-4f1de73d0685-nr00xjox/runtime-target/debug/deps/connect_session_transport-1c37bfd94d5374e6)

running 24 tests
test oauth::tests::already_expired_instruction_request_refuses_before_reading_or_writing ... ok
test oauth::tests::device_deadline_is_capped_by_private_authorization_expiry ... ok
test oauth::tests::fixed_redirect_policy_refuses_aliases_implicit_ports_and_ambiguous_paths ... ok
test oauth::tests::callback_query_is_strict_bounded_and_distinguishes_unknown_state ... ok
test tests::browser_capability_comparison_rejects_prefixes_and_differences ... ok
test oauth::tests::liveness_observer_drop_and_rebind_cannot_revive_old_receiver ... ok
test oauth::tests::request_parser_requires_exact_host_get_origin_form_and_bounded_headers ... ok
test oauth::tests::device_bridge_shows_only_human_instructions_and_has_no_callback ... ok
test oauth::tests::already_accepted_replay_cannot_survive_the_callback_claim ... ok
test tests::unsafe_directory_refuses ... ok
test tests::endpoint_is_owner_only_one_use_and_removed_after_submission ... ok
test tests::browser_page_submits_directly_to_the_one_use_endpoint ... ok
test oauth::tests::expiry_caps_a_stalled_read_and_future_drop_closes_the_port ... ok
test oauth::tests::liveness_observer_uses_original_receiver_deadline_without_polling_receive ... ok
test oauth::tests::liveness_observer_is_retired_after_matching_callback ... ok
test oauth::tests::fixed_port_conflict_and_drop_do_not_fall_back_or_leave_a_listener ... ok
test oauth::tests::callback_claim_closes_fixed_port_and_keeps_capability_out_of_provider_url ... ok
test oauth::tests::oauth_and_raw_completion_endpoints_remain_independent ... ok
test oauth::tests::request_read_deadline_is_five_seconds_and_size_bound_never_waits_for_a_newline ... ok
test oauth::tests::local_pkce_binding_inconsistency_is_a_safe_terminal_refusal ... ok
test oauth::tests::callback_origin_is_optional_but_exact_if_present_and_denial_retires_session ... ok
test oauth::tests::protected_instructions_refuse_capability_and_origin_without_spending_state ... ok
test oauth::tests::accepted_connection_budget_retires_session_without_starting_a_second_flow ... ok
test oauth::tests::oauth_pass1_last_callback_slot_survives_invalid_duplicates_and_closes_once ... ok

test result: ok. 24 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.05s

     Running unittests src/lib.rs (/dev/shm/b10x-cli-wave-wt-4f1de73d0685-nr00xjox/runtime-target/debug/deps/connectors_config-b27509ec4288e178)

running 24 tests
test hosted::tests::hosted_configuration_uses_same_handle_and_refuses_mutable_or_symlinked_files ... ok
test hosted::tests::an_existing_hosted_config_with_the_old_b10x_section_parses_unchanged ... ok
test hosted::tests::claude_code_custody_is_explicit_and_requires_the_hosted_secret_store ... ok
test personal::tests::a_trunk_address_may_be_a_literal_or_a_name_and_nothing_else ... ok
test hosted::tests::hosted_grafana_requires_vault_exact_groups_and_digest_bound_targets ... ok
test hosted::tests::hosted_vault_is_all_or_nothing ... ok
test hosted::tests::kubernetes_namespace_groups_are_exact_sorted_and_restart_is_a_read_subset ... ok
test hosted::tests::hosted_integrations_are_explicit_and_fail_closed ... ok
test personal::tests::an_existing_personal_config_with_the_old_b10x_section_parses_unchanged ... ok
test hosted::tests::hosted_jira_separates_organization_and_user_authority ... ok
test hosted::tests::hosted_vault_requires_a_valid_distinct_sip_digest_pair ... ok
test hosted::tests::hosted_jira_service_api_token_excludes_a_service_oauth_client_id ... ok
test hosted::tests::hosted_deployment_accepts_planner_integration ... ok
test personal::tests::slack_only_configuration_contains_policy_but_no_secret_source ... ok
test hosted::tests::hosted_gitlab_requires_vault_and_a_same_origin_callback ... ok
test personal::tests::kubernetes_configuration_is_policy_only ... ok
test personal::tests::grafana_configuration_names_origin_and_independent_target_grants_only ... ok
test personal_oauth_tests::personal_oauth_configuration_admits_device_for_a_remote_browser_without_redirect ... ok
test personal_oauth_tests::personal_oauth_configuration_admits_explicit_development_public_pkce ... ok
test personal::tests::documented_development_configuration_stays_strict_and_valid ... ok
test personal::tests::deployment_configuration_cannot_be_symlinked_or_group_writable ... ok
test personal::tests::the_named_default_trunk_example_parses_and_dials_by_number ... ok
test personal_oauth_tests::personal_oauth_configuration_refuses_nonlocal_redirects_and_implicit_custody ... ok
test personal_oauth_tests::oauth_pass1_scope_ceiling_and_ttl_boundaries_survive_real_config_loading ... ok

test result: ok. 24 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s

     Running tests/catalog_usernames.rs (/dev/shm/b10x-cli-wave-wt-4f1de73d0685-nr00xjox/runtime-target/debug/deps/catalog_usernames-a0036467f1f25b3e)

running 3 tests
test an_entry_with_no_user_half_still_reads_and_reports_none ... ok
test a_basic_credential_carries_its_user_half_beside_its_endpoints ... ok
test a_user_half_is_refused_when_it_is_empty_or_could_not_travel ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running unittests src/lib.rs (/dev/shm/b10x-cli-wave-wt-4f1de73d0685-nr00xjox/runtime-target/debug/deps/connectors_runtime-125226347f478b03)

running 33 tests
test claims::tests::a_journal_this_build_cannot_parse_refuses_to_open ... ok
test composition::tests::an_empty_variable_is_no_store_at_all ... ok
test claims::tests::only_an_event_reference_is_claimable ... ok
test composition::tests::git_fetch_environment_override_is_atomic_and_secret_free ... ok
test claims::tests::a_claim_survives_a_daemon_restart ... ok
test composition::tests::git_fetch_environment_override_refuses_an_invalid_listener ... ok
test composition::tests::an_unopenable_sqlite_path_is_named_in_the_refusal ... ok
test composition::tests::naming_both_stores_is_refused_rather_than_one_of_them_quietly_winning ... ok
test composition::tests::naming_no_store_is_refused_rather_than_a_database_file_appearing_somewhere ... ok
test composition::tests::the_refusal_names_both_stores_a_deployment_may_choose ... ok
test composition::tests::working_tree_state_roots_are_refused ... ok
test registry::tests::a_topical_datasource_query_that_matches_nothing_returns_the_admitted_set ... ok
test registry::tests::ambiguous_exclusive_claims_fail_with_typed_protocol_errors_before_dispatch ... ok
test registry::tests::direct_dispatch_selects_the_unique_claim_without_not_found_probing ... ok
test registry::tests::duplicate_channel_references_fail_search ... ok
test registry::tests::duplicate_connection_references_fail_search ... ok
test registry::tests::rate_stage2_advice_changes_refuse_merging_and_invalidate_the_registry_lease ... ok
test registry::tests::describe_merges_connections_and_invoke_receives_the_selected_local_lease ... ok
test registry::tests::the_registry_lease_ignores_request_scoped_provenance ... ok
test registry::tests::search_aggregates_compatible_operations_and_deduplicates_the_operation ... ok
test registry::tests::readiness_requires_every_configured_backend ... ok
test service_bundle::tests::registration_is_inert_until_an_explicit_overlay_is_present ... ok
test service_bundle::tests::catalog_dispatch_and_backend_ownership_mismatches_are_refused ... ok
test service_bundle::tests::bundle_order_and_policy_projection_are_deterministic ... ok
test registry::tests::an_undemanded_reference_is_not_spent_at_the_local_seam ... ok
test registry::tests::a_companion_reply_is_claimed_exactly_once_locally ... ok
test service_bundle::tests::malformed_manifests_and_deployments_are_refused ... ok
test service_bundle::tests::identity_and_operation_collisions_are_refused ... ok
test claims::tests::parallel_presentations_take_exactly_one_claim ... ok
test tls_listener::tests::established_connection_permit_is_lifetime_bound_and_reads_time_out ... ok
test composition::tests::empty_personal_runtime_binds_and_cleans_without_a_credential_store ... ok
test composition::tests::a_hosted_placement_keeps_its_state_in_a_file_when_no_database_is_offered ... ok
test tls_listener::tests::listener_serves_the_internal_application_over_tls ... ok

test result: ok. 33 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.04s

     Running tests/local_catalog_writes.rs (/dev/shm/b10x-cli-wave-wt-4f1de73d0685-nr00xjox/runtime-target/debug/deps/local_catalog_writes-0a1b5c1cb7423510)

running 8 tests
test describing_a_write_directly_skips_the_first_read_only_connection ... ok
test only_read_only_connections_hide_the_write_and_describe_its_missing_grant ... ok
test adversary_a_read_description_cannot_authorize_a_different_write_operation ... ok
test adversary_an_approval_reference_cannot_raise_the_selected_read_only_grant ... ok
test adversary_http_200_application_refusal_survives_the_documented_socket_path ... ok
test stale_description_and_provider_refusal_have_distinct_actionable_results ... ok
test read_only_first_still_discovers_describes_and_posts_through_the_writer ... ok
test writable_first_still_discovers_describes_and_posts_through_the_writer ... ok

test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.15s

     Running tests/local_gitlab_schedules.rs (/dev/shm/b10x-cli-wave-wt-4f1de73d0685-nr00xjox/runtime-target/debug/deps/local_gitlab_schedules-69369d064767eff1)

running 6 tests
test configured_gitlab_connection_is_the_same_passive_reference_in_both_discovery_surfaces ... ok
test schedule_words_find_only_operations_admitted_by_the_existing_write_policy ... ok
test read_only_connection_refuses_schedule_mutations_before_custody_or_egress ... ok
test adversary_gitlab_pass1_missing_credentials_and_forged_approval_never_widen_a_placement ... ok
test describe_exposes_the_exact_generated_input_and_output_contracts ... ok
test schedule_requests_preserve_complete_json_values_and_optional_update_omission ... ok

test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.11s

     Running tests/one_shot_runtime.rs (/dev/shm/b10x-cli-wave-wt-4f1de73d0685-nr00xjox/runtime-target/debug/deps/one_shot_runtime-a4dd1c4edc6d661f)

running 10 tests
test an_unsafe_socket_refuses_before_opening_the_reply_claim_journal ... ok
test adversary_socket_publication_after_absence_probe_is_preserved_and_refused ... ok
test adversary_every_persistent_request_class_refuses_before_configuration_or_state ... ok
test unknown_backend_lifetime_is_refused_and_shutdown_releases_ownership ... ok
test an_existing_owner_refuses_before_opening_the_reply_claim_journal ... ok
test adversary_lifetime_capability_never_admits_unknown_or_ambiguous_owners ... ok
test final_adversary_malformed_backend_reply_is_reduced_before_shutdown_and_lock_release ... ok
test adversary_state_lock_outlives_async_shutdown_on_success_and_refusal ... ok
test final_adversary_invalid_envelopes_refuse_before_reading_config_or_creating_state ... ok
test ephemeral_catalog_uses_the_described_credential_and_rejects_read_only_writes_before_egress ... ok

test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.17s

     Running tests/personal_oauth.rs (/dev/shm/b10x-cli-wave-wt-4f1de73d0685-nr00xjox/runtime-target/debug/deps/personal_oauth-ad0314c0b1c57652)

running 5 tests
test one_shot_create_refuses_before_configuration_custody_and_listener ... ok
test unsupported_production_registration_refuses_before_readiness_or_oauth_store ... ok
test composition_opens_only_the_explicit_oauth_custody_and_reports_unsealed_readiness ... ok
test oauth_pass1_daemon_refuses_ambiguous_v1_profile_without_using_label_as_target ... ok
test mixed_raw_and_oauth_bindings_have_exactly_one_invoke_owner_and_keep_aggregation ... ok

test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.19s

     Running tests/rate_adversary_registry.rs (/dev/shm/b10x-cli-wave-wt-4f1de73d0685-nr00xjox/runtime-target/debug/deps/rate_adversary_registry-f2bd087fa205bb2f)

running 1 test
test rate_adversary_registry_checks_advice_through_describe_and_invoke ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running unittests src/lib.rs (/dev/shm/b10x-cli-wave-wt-4f1de73d0685-nr00xjox/runtime-target/debug/deps/integration_catalog-c79e64ccd55fa21f)

running 92 tests
test argocd::tests::every_generated_policy_is_one_argo_cd_will_accept ... ok
test argocd::tests::a_zero_lifetime_is_refused_rather_than_meaning_forever ... ok
test argocd::tests::a_project_or_role_name_that_could_change_the_path_is_refused ... ok
test argocd::tests::read_only_acquisition_carries_no_sync_policy ... ok
test argocd::tests::an_unreachable_origin_names_the_aperture_as_a_possibility ... ok
test argocd::tests::a_rejected_sign_in_says_so_rather_than_reporting_a_missing_project ... ok
test argocd::tests::a_login_without_projects_update_is_told_which_grant_it_lacks ... ok
test argocd::tests::a_missing_project_is_not_reported_as_a_permission_problem ... ok
test argocd::tests::an_existing_role_is_reused_and_its_policies_are_left_alone ... ok
test argocd::tests::the_password_appears_once_and_the_session_token_never_persists ... ok
test argocd::tests::the_four_calls_happen_in_order_and_the_token_comes_back ... ok
test config::tests::a_value_without_operator_approval_says_so_rather_than_claiming_it ... ok
test config::tests::an_endpoint_value_reaches_the_resolver_under_its_declared_name ... ok
test config::tests::an_unsupplied_variable_is_absent_rather_than_empty ... ok
test argocd::tests::the_project_is_written_back_whole ... ok
test custody::tests::distinct_bindings_cannot_alias_the_same_reserved_credential_addresses ... ok
test custody::tests::a_reclaimed_publication_retains_its_original_authorization_timing ... ok
test custody::tests::a_reopened_publication_is_unavailable_until_its_store_retirement_is_checked ... ok
test custody::tests::a_claimed_decision_write_can_finish_after_the_authorization_deadline ... ok
test custody::tests::a_full_decision_precedes_secret_commit_and_coherent_publication ... ok
test custody::tests::a_durable_but_error_decision_is_recovered_without_an_immediate_secret_commit ... ok
test custody::tests::a_held_full_write_blocks_commit_but_not_private_status_or_expiry_checks ... ok
test custody::tests::a_missing_or_wrong_credential_store_cannot_restore_published_journal_evidence ... ok
test custody::tests::proposal_bounds_digest_and_journal_serialization_keep_private_values_out ... ok
test custody::refresh_tests::refresh_of_expired_access_uses_one_timely_claim_without_a_session ... ok
test custody::refresh_tests::a_stale_refresh_handle_cannot_prepare_after_another_valid_publication ... ok
test custody::refresh_tests::binding_gate_preserves_the_new_generation_for_the_waiting_operation ... ok
test custody::refresh_tests::waiting_for_prepare_does_not_restart_the_refresh_window ... ok
test custody::refresh_tests::a_timely_refresh_claim_survives_a_held_full_decision_write ... ok
test custody::tests::generation_exhaustion_refuses_before_io_and_resolves_its_private_guard ... ok
test custody::tests::a_preparing_committed_or_decided_absent_store_state_cannot_invent_authorization ... ok
test custody::tests::dropping_the_future_at_prepared_or_decided_boundaries_leaves_recoverable_ownership ... ok
test custody::tests::mismatched_proposal_or_stale_prior_generation_never_prepares ... ok
test custody::tests::one_unresolved_store_slot_blocks_a_second_binding_and_generation_allocation ... ok
test custody::refresh_tests::invalid_refresh_decision_timing_cannot_publish_on_reopen ... ok
test custody::refresh_tests::cancelled_refresh_store_io_holds_the_binding_gate_until_recovery ... ok
test custody::tests::expiry_or_revocation_before_the_claim_aborts_without_a_decision ... ok
test custody::tests::replacement_preserves_identity_and_publishes_only_same_generation_evidence ... ok
test custody::refresh_tests::uncertain_refresh_decisions_reopen_as_finish_or_abort_without_a_session ... ok
test custody::tests::unknown_journal_or_secret_state_stays_unavailable_until_reconciled ... ok
test custody::refresh_tests::refresh_start_requires_current_operation_authority_and_a_valid_receiver_clock ... ok
test custody::tests::every_journal_write_boundary_recovers_real_sqlite_and_file_store_after_reopen ... ok
test hosted::tests::completion_does_not_overwrite_a_terminal_session_after_verification_awaits ... ok
test hosted::tests::two_people_receive_isolated_connections_and_credential_addresses ... ok
test custody::tests::uncertain_secret_operations_are_resolved_from_state_and_never_assumed_rolled_back ... ok
test oauth::tests::admitted_response_reference_is_the_actual_configured_backend_identity ... ok
test oauth::tests::pending_callback_shutdown_joins_receiver_and_cannot_revive_session_after_reopen ... ok
test custody::tests::malformed_or_inconsistent_decisions_never_recover_a_publication ... ok
test oauth::tests::adversary::oauth_pass2_shutdown_after_token_before_evidence_never_publishes_or_restarts ... ok
test oauth::tests::actual_prepare_then_preclaim_expiry_aborts_without_completed_or_credentials ... ok
test oauth::tests::actual_authority_revocation_before_completion_claim_aborts_prepared_credentials ... ok
test oauth::tests::actual_timely_claim_survives_full_decision_write_after_session_deadline ... ok
test oauth::tests::actual_pkce_uses_observed_evidence_and_same_store_for_dispatch_and_reopen ... ok
test tests::a_basic_credentials_user_half_reaches_the_resolver_from_the_entry ... ok
test tests::a_network_effect_is_not_an_escalation_but_a_filesystem_one_would_be ... ok
test tests::a_provider_with_a_fixed_base_url_needs_no_configuration ... ok
test tests::a_read_only_first_connection_does_not_hide_an_admitted_write ... ok
test tests::a_request_template_exists_for_every_operation_this_backend_would_offer ... ok
test tests::an_entry_stating_no_user_half_answers_none_rather_than_an_empty_string ... ok
test tests::an_unnamed_entry_keeps_the_address_it_had_before_instances_existed ... ok
test tests::an_unsupplied_url_variable_refuses_rather_than_reaching_a_literal_placeholder ... ok
test tests::effect_class_comes_from_the_declaration_not_the_method ... ok
test tests::every_catalogued_provider_can_address_a_credential ... ok
test tests::every_declared_user_half_field_names_a_credential_that_actually_wants_one ... ok
test oauth::tests::invalid_lease_or_source_input_refuses_before_refresh_marker_and_egress ... ok
test tests::rate_adversary_catalog_checks_binding_and_lease_before_rate_disclosure ... ok
test oauth::tests::actual_refresh_request_is_cancelled_at_its_original_egress_budget ... ok
test tests::the_aperture_is_derived_from_the_same_declaration_the_request_is ... ok
test tests::the_ceiling_reads_declared_facts_rather_than_an_operation_list ... ok
test tests::the_default_ceiling_admits_reads_and_refuses_writes ... ok
test tests::the_instance_derivation_puts_the_provider_in_the_namespace ... ok
test tests::the_limit_drops_operations_rather_than_the_identities_that_serve_one ... ok
test tests::two_named_instances_of_one_provider_get_different_addresses ... ok
test tests::search_and_describe_require_approval_for_every_admitted_write ... ok
test tests::rate_stage2_catalog_refusal_keeps_only_trusted_optional_delay ... ok
test oauth::tests::actual_metadata_publication_before_receipt_reclamation_recovers_on_reopen ... ok
test oauth::tests::actual_secret_commit_before_metadata_publication_recovers_on_reopen ... ok
test oauth::tests::actual_unknown_decision_without_durable_decision_aborts_on_reopen ... ok
test oauth::tests::actual_full_decision_followed_by_error_recovers_after_deadline_on_reopen ... ok
test oauth::tests::unique_binding_and_owner_refusals_happen_before_listener_session_or_egress ... ok
test oauth::tests::wrong_owner_unknown_profile_and_nonpersistent_create_have_zero_egress ... ok
test oauth::tests::refresh_does_not_reset_its_original_window_after_token_egress ... ok
test oauth::tests::requested_scope_ceiling_never_substitutes_for_observed_operation_scopes ... ok
test oauth::tests::refresh_rotation_before_bad_token_info_blocks_old_dispatch_across_reopen ... ok
test oauth::tests::unknown_full_marker_confirmation_sends_zero_refresh_egress_and_survives_reopen ... ok
test oauth::tests::rotation_followed_by_real_file_prepare_refusal_blocks_old_token_after_reopen ... ok
test oauth::tests::adversary::oauth_pass1_explicit_reauthorization_repairs_only_coherent_subject_after_rotation ... ok
test oauth::tests::successful_refresh_replaces_both_secrets_once_and_remains_callable_after_reopen ... ok
test custody::refresh_tests::refresh_claim_rechecks_time_authority_generation_and_captured_evidence ... ok
test custody::tests::journal_capacity_is_reserved_for_publication_before_secret_prepare ... ok
test oauth::tests::device_optional_refresh_omission_deletes_previous_secret_and_refuses_on_expiry ... ok
test oauth::tests::adversary::oauth_pass1_device_slowdown_and_denial_keep_one_authorization_and_original_deadline ... ok

test result: ok. 92 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 13.49s

     Running unittests src/lib.rs (/dev/shm/b10x-cli-wave-wt-4f1de73d0685-nr00xjox/runtime-target/debug/deps/state_sqlite-6810760f0982d696)

running 11 tests
test tests::concatenation_would_have_corrupted_binary_and_the_transaction_does_not ... ok
test tests::full_open_refuses_unusable_paths ... ok
test tests::the_in_memory_backend_conforms ... ok
test tests::the_in_memory_backend_serves_grant_evaluation ... ok
test tests::the_file_backend_serves_grant_evaluation ... ok
test tests::existing_openers_keep_normal_synchronization ... ok
test tests::the_file_backend_conforms ... ok
test tests::a_cell_survives_reopening_the_file ... ok
test tests::full_open_configures_wal_and_full_synchronization_on_every_open ... ok
test tests::full_commits_are_visible_before_close_and_survive_reopening ... ok
test tests::the_full_file_backend_preserves_state_and_grant_conformance ... ok

test result: ok. 11 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.12s

     Running tests/approval_gate.rs (/dev/shm/b10x-cli-wave-wt-4f1de73d0685-nr00xjox/runtime-target/debug/deps/approval_gate-3e082fb65824fe77)

running 3 tests
test sixteen_concurrent_identical_presentations_redeem_exactly_once ... ok
test a_replay_survives_reopening_the_database ... ok
test a_crash_between_redemption_and_terminal_write_leaves_a_recoverable_attempted_row ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.05s

   Doc-tests connect_session_transport

running 2 tests
test ~/.local/state/worktree/trees/b10x/connectors/wt-4f1de73d0685/crates/connect-session-transport/src/oauth.rs - oauth::BoundOAuthEndpoint (line 100) - compile fail ... ok
test ~/.local/state/worktree/trees/b10x/connectors/wt-4f1de73d0685/crates/connect-session-transport/src/oauth.rs - oauth::OAuthCallback (line 55) - compile fail ... ok

test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.05s

   Doc-tests connectors_config

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

   Doc-tests connectors_runtime

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

   Doc-tests integration_catalog

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

   Doc-tests state_sqlite

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

````

Exit 0; 222 passed / 0 failed / 0 ignored. Guard interrupted: false.

Execution runtime-clippy; started 2026-09-06T18:42:11.324563+00:00; finished 2026-09-06T18:42:16.632794+00:00. Exact command/cwd/environment:

````json
{
  "cwd": "~/.local/state/worktree/trees/b10x/connectors/wt-4f1de73d0685/crates/connectors-runtime",
  "argv": [
    "env",
    "-u",
    "RUSTC_WRAPPER",
    "CARGO_TARGET_DIR=/dev/shm/b10x-cli-wave-wt-4f1de73d0685-nr00xjox/runtime-target",
    "TMPDIR=~/.cache/cw6/oa2",
    "CARGO_BUILD_JOBS=1",
    "CARGO_INCREMENTAL=0",
    "CARGO_PROFILE_DEV_DEBUG=0",
    "CARGO_PROFILE_TEST_DEBUG=0",
    "cargo",
    "clippy",
    "--locked",
    "--offline",
    "-p",
    "connectors-config",
    "-p",
    "connect-session-transport",
    "-p",
    "integration-catalog",
    "-p",
    "connectors-runtime",
    "-p",
    "state-sqlite",
    "--all-targets",
    "--",
    "-D",
    "warnings"
  ]
}
````

Actual complete output from ~/.cache/connectors-cli-wave-20260906/connect-session-oauth-custody-in-personal-posture/whole-unit-adversary-2/oauth-pass2-runtime-clippy.log:

````text
warning: connector-secrets@0.6.5: no live Vault was offered (CONNECTOR_SECRETS_VAULT_ADDR and CONNECTOR_SECRETS_VAULT_TOKEN are unset), so the reqwest HttpTransport is UNEXERCISED by this build: tests/vault_live.rs is compiled #[ignore]d
    Checking integration-catalog v0.6.5 (~/.local/state/worktree/trees/b10x/connectors/wt-4f1de73d0685/crates/integration-catalog)
    Finished `dev` profile [unoptimized] target(s) in 1.82s
````

Exit 0; 0 passed / 0 failed / 0 ignored. Guard interrupted: false.

Execution runtime-fmt; started 2026-09-06T18:44:38.796889+00:00; finished 2026-09-06T18:44:44.128565+00:00. Exact command/cwd/environment:

````json
{
  "cwd": "~/.local/state/worktree/trees/b10x/connectors/wt-4f1de73d0685/crates/connectors-runtime",
  "argv": [
    "env",
    "-u",
    "RUSTC_WRAPPER",
    "CARGO_TARGET_DIR=/dev/shm/b10x-cli-wave-wt-4f1de73d0685-nr00xjox/runtime-target",
    "TMPDIR=~/.cache/cw6/oa2",
    "CARGO_BUILD_JOBS=1",
    "CARGO_INCREMENTAL=0",
    "CARGO_PROFILE_DEV_DEBUG=0",
    "CARGO_PROFILE_TEST_DEBUG=0",
    "cargo",
    "fmt",
    "--all",
    "--",
    "--check"
  ]
}
````

Actual complete output from ~/.cache/connectors-cli-wave-20260906/connect-session-oauth-custody-in-personal-posture/whole-unit-adversary-2/oauth-pass2-runtime-fmt.log:

````text

````

Exit 0; 0 passed / 0 failed / 0 ignored. Guard interrupted: false.

Execution console-full; started 2026-09-06T18:44:57.050317+00:00; finished 2026-09-06T18:45:22.972830+00:00. Exact command/cwd/environment:

````json
{
  "cwd": "~/.local/state/worktree/trees/b10x/connectors/wt-4f1de73d0685/crates/connectors-console",
  "argv": [
    "env",
    "-u",
    "RUSTC_WRAPPER",
    "CARGO_TARGET_DIR=/dev/shm/b10x-cli-wave-wt-4f1de73d0685-nr00xjox/runtime-target",
    "TMPDIR=~/.cache/cw6/oa2",
    "CARGO_BUILD_JOBS=1",
    "CARGO_INCREMENTAL=0",
    "CARGO_PROFILE_DEV_DEBUG=0",
    "CARGO_PROFILE_TEST_DEBUG=0",
    "cargo",
    "test",
    "--locked",
    "--offline",
    "--workspace",
    "--no-fail-fast"
  ]
}
````

Actual complete output from ~/.cache/connectors-cli-wave-20260906/connect-session-oauth-custody-in-personal-posture/whole-unit-adversary-2/oauth-pass2-console-full.log:

````text
    Finished `test` profile [unoptimized] target(s) in 0.19s
     Running unittests src/lib.rs (/dev/shm/b10x-cli-wave-wt-4f1de73d0685-nr00xjox/runtime-target/debug/deps/connectors_console-beb8902d60b048cc)

running 76 tests
test auth::tests::nothing_in_the_result_can_carry_a_secret ... ok
test auth::tests::a_basic_credential_row_reports_whether_its_user_half_is_configured_and_never_the_value ... ok
test connect::tests::a_provider_outside_the_guided_set_is_refused_by_name ... ok
test admin::tests::explicit_secret_file_must_be_owner_only ... ok
test connect::tests::the_error_for_an_unknown_provider_names_it ... ok
test auth::tests::the_store_preference_matches_what_the_runtime_composes ... ok
test doctor::tests::a_missing_configuration_is_fatal_and_names_the_command_that_fixes_it ... ok
test doctor::tests::a_report_is_unhealthy_only_when_something_cannot_work ... ok
test doctor::tests::a_short_state_root_passes_both_budgets ... ok
test connect::personal_oauth_tests::instruction_file_refuses_shared_parent_symlink_and_existing_content ... ok
test doctor::tests::the_budget_is_measured_against_the_deepest_path_the_daemon_binds ... ok
test doctor::tests::the_report_renders_every_check_as_data ... ok
test doctor::tests::every_state_a_check_can_report_reaches_the_reader_as_its_own_marker ... ok
test enrol::tests::a_provider_outside_the_catalogue_is_named_rather_than_guessed_at ... ok
test doctor::tests::doctor_names_the_default_local_target_and_its_socket ... ok
test admin::tests::command_shape_accepts_secret_stdin_without_a_secret_argument ... ok
test connect::tests::a_catalogued_provider_whose_curated_backend_is_absent_takes_the_catalogue_path ... ok
test envelope::tests::a_result_loses_its_envelope_and_its_discriminant ... ok
test init::tests::an_agent_id_is_stable_across_calls ... ok
test envelope::personal_oauth_tests::ordinary_connection_result_payload_has_no_private_instruction_endpoint ... ok
test envelope::tests::an_envelope_carrying_neither_is_a_named_failure_not_an_empty_success ... ok
test init::tests::admitting_a_credential_plugin_is_a_choice_and_its_absence_is_explained ... ok
test init::tests::the_snapshot_digest_is_stable_and_moves_with_the_admitted_set ... ok
test init::tests::the_separator_keeps_a_concatenation_from_colliding ... ok
test init::tests::an_existing_configuration_is_never_replaced_silently ... ok
test envelope::tests::a_refusal_becomes_an_error_rather_than_a_result ... ok
test input::tests::input_accepts_only_the_stdin_marker ... ok
test input::tests::a_file_is_read_from_its_path ... ok
test input::tests::an_inline_object_is_parsed ... ok
test output::tests::a_structured_format_carries_its_failure_on_stdout ... ok
test output::tests::a_row_shows_its_severity_before_anybody_reads_it ... ok
test output::tests::a_table_reads_left_to_right_with_the_column_that_runs_long_last ... ok
test output::tests::a_payload_carrying_its_own_value_field_is_left_alone ... ok
test input::tests::no_source_names_all_three_rather_than_defaulting_to_empty ... ok
test output::tests::a_wide_character_cell_keeps_the_column_after_it_aligned ... ok
test output::tests::a_word_the_renderer_cannot_rank_is_marked_unknown_rather_than_good ... ok
test output::tests::an_empty_listing_is_an_empty_stream_rather_than_a_line_shaped_like_a_record ... ok
test output::tests::an_object_with_two_arrays_is_not_unwrapped ... ok
test output::tests::an_unranked_table_still_keeps_the_marker_column ... ok
test output::tests::columns_of_equal_width_keep_the_order_the_record_carries ... ok
test output::tests::a_record_that_is_not_an_object_keeps_the_name_the_report_gave_it ... ok
test output::tests::a_field_a_record_does_not_carry_reads_as_absent_rather_than_blank ... ok
test output::tests::compact_leaves_a_single_record_as_one_line ... ok
test output::tests::compact_unwraps_the_one_array_a_list_response_carries ... ok
test output::tests::every_protocol_state_this_renderer_can_be_handed_has_a_rank ... ok
test output::tests::compact_keeps_the_scalar_a_list_response_carries_beside_its_records ... ok
test output::tests::compact_keeps_a_field_a_record_carries_below_its_top_level ... ok
test output::tests::text_does_not_quote_a_string_a_person_is_reading ... ok
test output::tests::no_cell_is_ever_empty_so_no_row_can_end_in_whitespace ... ok
test output::tests::text_says_none_rather_than_printing_an_empty_bracket ... ok
test output::tests::the_result_discriminant_is_stripped_so_compact_can_see_the_records ... ok
test output::tests::severity_survives_a_pipe_because_it_is_not_carried_by_colour ... ok
test output::tests::text_keeps_every_field_a_record_carries_including_a_nested_list ... ok
test output::tests::text_spends_one_aligned_row_on_each_record ... ok
test output::tests::the_structured_formats_render_the_bytes_they_rendered_before ... ok
test output::tests::the_widest_column_moves_last_even_when_the_record_puts_it_first ... ok
test output::tests::yaml_renders_through_the_maintained_crate ... ok
test output::tests::a_cell_never_carries_a_character_that_breaks_the_row ... ok
test connect::personal_oauth_tests::instruction_file_is_exclusive_owner_only_and_cleared_on_drop ... ok
test output::tests::every_status_word_this_package_emits_is_one_the_renderer_can_rank ... ok
test init::tests::a_configuration_the_daemon_would_refuse_is_not_left_on_disk ... ok
test init::tests::what_init_writes_is_what_the_daemon_can_read ... ok
test connect::personal_oauth_tests::instruction_cleanup_never_removes_a_replacement_inode ... ok
test auth::tests::the_catalogue_is_what_says_a_credential_has_a_user_half ... ok
test enrol::tests::a_self_hosted_origin_is_the_case_operator_approval_exists_for ... ok
test enrol::tests::gitlab_asks_for_nothing_when_its_default_origin_is_wanted ... ok
test enrol::tests::slack_declares_a_bot_and_a_user_credential_which_one_identity_may_both_hold ... ok
test enrol::tests::most_of_the_catalogue_asks_no_configuration_question_at_all ... ok
test providers::tests::an_unmatched_query_is_an_empty_listing_rather_than_the_whole_catalogue ... ok
test providers::tests::a_provider_without_a_probe_is_not_ready_and_says_why_by_omission ... ok
test providers::tests::a_query_narrows_to_one_provider_and_its_summary_follows ... ok
test providers::tests::the_shipped_catalogue_is_reported_rather_than_asserted ... ok
test output::tests::a_table_too_wide_for_a_terminal_starts_its_last_column_inside_the_budget ... ok
test output::tests::two_providers_that_differ_in_their_id_differ_on_screen ... ok
test output::tests::the_budget_is_documented_as_what_it_is_and_a_real_row_is_wider_than_it ... ok
test output::tests::a_cell_the_budget_cut_says_so_and_the_column_names_are_cut_last ... ok

test result: ok. 76 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.00s

     Running tests/adversary_budget_prose.rs (/dev/shm/b10x-cli-wave-wt-4f1de73d0685-nr00xjox/runtime-target/debug/deps/adversary_budget_prose-b3e74865aa4ac969)

running 3 tests
test pass3_render_helper_child ... ok
test the_quoted_module_header_sentence_is_at_the_line_the_pass_two_suite_cites ... ok
test the_widths_the_documents_state_are_the_widths_the_renderer_prints ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.01s

     Running tests/adversary_readability.rs (/dev/shm/b10x-cli-wave-wt-4f1de73d0685-nr00xjox/runtime-target/debug/deps/adversary_readability-f81fa2895436cbf7)

running 7 tests
test render_helper_child ... ok
test a_record_whose_cells_are_all_empty_is_rendered_as_a_blank_line ... ok
test an_unranked_table_lets_a_cell_sit_where_the_severity_marker_sits ... ok
test a_wide_character_cell_leaves_the_column_after_it_ragged ... ok
test doctor_spreads_one_check_over_several_unmarked_lines_when_the_configuration_is_malformed ... ok
test compact_no_longer_puts_one_record_on_every_line ... ok
test providers_starts_its_last_column_past_the_width_of_any_terminal ... ok

test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.00s

     Running tests/adversary_readability_pass2.rs (/dev/shm/b10x-cli-wave-wt-4f1de73d0685-nr00xjox/runtime-target/debug/deps/adversary_readability_pass2-ee94985bff930141)

running 5 tests
test pass2_render_helper_child ... ok
test compact_drops_the_name_of_the_array_a_report_carries ... ok
test a_column_the_budget_squeezes_to_nothing_pushes_every_later_column_out_of_line ... ok
test compact_answers_an_empty_listing_with_a_line_that_is_not_a_record ... ok
test the_last_column_of_providers_begins_one_column_past_the_terminal_it_is_laid_out_for ... ok

test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.01s

     Running tests/personal_oauth.rs (/dev/shm/b10x-cli-wave-wt-4f1de73d0685-nr00xjox/runtime-target/debug/deps/personal_oauth-42c4eb7161a076c0)

running 11 tests
test ambiguous_profile_refuses_before_output_file_or_daemon_connection ... ok
test doctor_retains_ordinary_credential_store_diagnostic_for_an_owner_only_config ... ok
test headless_oauth_requires_private_file_before_session_creation ... ok
test doctor_reports_exact_redirect_and_unsealed_custody_without_client_material ... ok
test unsafe_private_destination_refuses_before_any_daemon_connection ... ok
test explicit_private_file_is_reserved_before_create_and_erased_before_public_success ... ok
test successful_private_daemon_label_cannot_reach_the_public_summary ... ok
test oauth_pass1_cancellation_erases_already_written_private_inode_before_return ... ok
test oauth_pass1_real_controlling_pty_handoff_keeps_redirected_outputs_private ... ok
test private_daemon_refusal_is_closed_on_stdout_and_stderr_in_all_formats ... ok
test oauth_pass2_private_file_expires_while_completion_grace_stays_bounded ... ok

test result: ok. 11 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 20.01s

   Doc-tests connectors_console

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

````

Exit 0; 102 passed / 0 failed / 0 ignored. Guard interrupted: false.

Execution console-clippy; started 2026-09-06T18:47:14.179025+00:00; finished 2026-09-06T18:47:19.474739+00:00. Exact command/cwd/environment:

````json
{
  "cwd": "~/.local/state/worktree/trees/b10x/connectors/wt-4f1de73d0685/crates/connectors-console",
  "argv": [
    "env",
    "-u",
    "RUSTC_WRAPPER",
    "CARGO_TARGET_DIR=/dev/shm/b10x-cli-wave-wt-4f1de73d0685-nr00xjox/runtime-target",
    "TMPDIR=~/.cache/cw6/oa2",
    "CARGO_BUILD_JOBS=1",
    "CARGO_INCREMENTAL=0",
    "CARGO_PROFILE_DEV_DEBUG=0",
    "CARGO_PROFILE_TEST_DEBUG=0",
    "cargo",
    "clippy",
    "--locked",
    "--offline",
    "--workspace",
    "--all-targets",
    "--",
    "-D",
    "warnings"
  ]
}
````

Actual complete output from ~/.cache/connectors-cli-wave-20260906/connect-session-oauth-custody-in-personal-posture/whole-unit-adversary-2/oauth-pass2-console-clippy.log:

````text
    Checking connectors-console v0.6.5 (~/.local/state/worktree/trees/b10x/connectors/wt-4f1de73d0685/crates/connectors-console)
    Finished `dev` profile [unoptimized] target(s) in 0.36s
````

Exit 0; 0 passed / 0 failed / 0 ignored. Guard interrupted: false.

Execution console-fmt; started 2026-09-06T18:47:38.900057+00:00; finished 2026-09-06T18:47:44.198041+00:00. Exact command/cwd/environment:

````json
{
  "cwd": "~/.local/state/worktree/trees/b10x/connectors/wt-4f1de73d0685/crates/connectors-console",
  "argv": [
    "env",
    "-u",
    "RUSTC_WRAPPER",
    "CARGO_TARGET_DIR=/dev/shm/b10x-cli-wave-wt-4f1de73d0685-nr00xjox/runtime-target",
    "TMPDIR=~/.cache/cw6/oa2",
    "CARGO_BUILD_JOBS=1",
    "CARGO_INCREMENTAL=0",
    "CARGO_PROFILE_DEV_DEBUG=0",
    "CARGO_PROFILE_TEST_DEBUG=0",
    "cargo",
    "fmt",
    "--all",
    "--",
    "--check"
  ]
}
````

Actual complete output from ~/.cache/connectors-cli-wave-20260906/connect-session-oauth-custody-in-personal-posture/whole-unit-adversary-2/oauth-pass2-console-fmt.log:

````text

````

Exit 0; 0 passed / 0 failed / 0 ignored. Guard interrupted: false.

Execution cli-full; started 2026-09-06T18:48:00.220853+00:00; finished 2026-09-06T18:48:35.859361+00:00. Exact command/cwd/environment:

````json
{
  "cwd": "~/.local/state/worktree/trees/b10x/connectors/wt-4f1de73d0685/crates/connectors-cli",
  "argv": [
    "env",
    "-u",
    "CARGO_TARGET_DIR",
    "RUSTC_WRAPPER=/usr/bin/sccache",
    "TMPDIR=~/.cache/cw6/oa2",
    "CARGO_BUILD_JOBS=1",
    "CARGO_INCREMENTAL=0",
    "CARGO_PROFILE_DEV_DEBUG=0",
    "CARGO_PROFILE_TEST_DEBUG=0",
    "cargo",
    "test",
    "--locked",
    "--offline",
    "--workspace",
    "--no-fail-fast"
  ]
}
````

Actual complete output from ~/.cache/connectors-cli-wave-20260906/connect-session-oauth-custody-in-personal-posture/whole-unit-adversary-2/oauth-pass2-cli-full.log:

````text
warning: connector-secrets@0.6.5: no live Vault was offered (CONNECTOR_SECRETS_VAULT_ADDR and CONNECTOR_SECRETS_VAULT_TOKEN are unset), so the reqwest HttpTransport is UNEXERCISED by this build: tests/vault_live.rs is compiled #[ignore]d
    Finished `test` profile [unoptimized] target(s) in 0.41s
     Running unittests src/lib.rs (target/debug/deps/connectors_cli-477d0f8a36775b0f)

running 5 tests
test tests::slack_connect_needs_no_internal_reference_or_path_argument ... ok
test tests::grafana_connect_uses_the_same_guided_surface ... ok
test tests::kubernetes_connect_accepts_an_exact_context_selection ... ok
test tests::normal_help_exposes_the_guided_flow_and_hides_acquisition_plumbing ... ok
test tests::every_supported_shell_gets_a_script_naming_the_whole_surface ... ok

test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s

     Running unittests src/main.rs (target/debug/deps/connectors-ff356cefd6648ce6)

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests/adversary_cli_cap_pass3.rs (target/debug/deps/adversary_cli_cap_pass3-3966260af721e3d2)

running 1 test
test the_cap_the_design_page_says_is_measured_is_declared_and_asserted ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests/adversary_fence_probe.rs (target/debug/deps/adversary_fence_probe-39a319a50760b981)

running 6 tests
test the_wire_name_rule_citation_in_the_design_document_points_at_the_rule ... ok
test the_wire_name_rule_citation_in_the_specification_points_at_the_rule ... ok
test the_typeable_words_are_the_words_the_design_document_names ... ok
test the_copies_this_probe_carries_are_still_copies ... ok
test a_forwarding_reason_that_names_no_command_is_refused_whatever_kind_it_carries ... ok
test every_entry_that_is_not_a_lifecycle_step_is_refused_when_it_claims_to_be_one ... ok

test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.13s

     Running tests/adversary_fence_probe_pass2.rs (target/debug/deps/adversary_fence_probe_pass2-4861be45867a10d9)

running 3 tests
test every_file_the_committed_contract_opens_is_a_file_a_clone_has ... ok
test the_kinds_the_design_document_says_rest_on_no_sentence_rest_on_no_sentence ... ok
test the_paths_the_design_document_says_send_no_protocol_request_send_none ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests/adversary_shim_pass3.rs (target/debug/deps/adversary_shim_pass3-def8c8a98018da02)

running 6 tests
test the_serve_group_advertises_a_help_subcommand ... ok
test a_help_path_of_the_new_tree_under_serve_is_left_alone ... ok
test connectors_help_still_answers_for_a_path_that_moved ... ok
test a_moved_path_typed_with_the_global_output_flag_still_works ... ok
test the_group_word_whose_only_command_moved_still_points_somewhere ... ok
test nothing_this_product_prints_names_a_moved_path_behind_a_global_flag ... ok

test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.32s

     Running tests/adversary_shim_pass4.rs (target/debug/deps/adversary_shim_pass4-a430f0057d226787)

running 3 tests
test the_table_this_suite_copies_by_hand_is_the_table_the_binary_ships ... ok
test the_auth_group_still_answers_the_help_subcommand_it_advertised ... ok
test a_two_word_path_that_moved_works_with_the_global_flag_between_its_words ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.05s

     Running tests/adversary_shim_pass5.rs (target/debug/deps/adversary_shim_pass5-4f1341cbb1b96153)

running 4 tests
test a_positional_value_spelled_help_is_a_value_not_a_help_request ... ok
test the_double_dash_escape_is_not_a_word_that_moved ... ok
test an_argument_neither_the_group_nor_the_leaf_declares_is_not_the_old_leaf ... ok
test serve_with_only_global_options_is_the_group_in_every_spelling_and_position ... ok

test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.05s

     Running tests/cli_surface.rs (target/debug/deps/cli_surface-082b408f0cb702cb)

running 36 tests
test every_kind_of_exception_is_used_and_every_entry_gives_a_reason ... ok
test no_word_of_the_parser_answers_to_a_name_the_specification_cannot_declare ... ok
test every_named_exception_is_still_a_path_of_the_parser ... ok
test every_declared_group_is_a_group_of_the_parser ... ok
test every_declared_group_help_line_is_the_summary_the_specification_declares ... ok
test no_path_is_both_declared_and_excepted ... ok
test every_path_of_the_parser_is_declared_or_a_named_exception ... ok
test personal_oauth_setup_requires_explicit_profile_and_private_instruction_option ... ok
test every_declaration_the_adversary_probe_copies_is_still_a_copy ... ok
test the_committed_generated_tree_is_the_specification_word_for_word ... ok
test every_citation_this_unit_wrote_resolves ... ok
test the_old_login_selected_target_guard_is_absent ... ok
test a_read_stops_being_an_exception_once_the_specification_declares_a_view ... ok
test a_command_absorbed_into_the_exception_list_alone_is_refused ... ok
test the_regeneration_command_the_documents_name_is_the_one_the_gate_runs ... ok
test the_parser_accepts_target_before_and_after_each_dual_target_leaf ... ok
test every_citation_that_names_a_symbol_lands_on_its_declaration ... ok
test target_conflict_does_not_wait_for_open_stdin ... ok
test the_read_verb_enumeration_partitions_the_protocols_it_names ... ok
test the_specification_names_the_binary_the_parser_builds ... ok
test the_kinds_the_tree_derives_are_the_kinds_the_list_carries ... ok
test the_target_countdown_is_exactly_what_the_parser_still_owes ... ok
test the_exception_list_is_the_set_the_specification_enumerates ... ok
test target_conflict_precedes_invoke_payload_loading ... ok
test targeted_errors_keep_the_target_in_yaml_and_text ... ok
test an_explicit_hosted_target_requires_a_login_by_name_for_every_group ... ok
test target_conflict_precedes_missing_or_malformed_inline_input ... ok
test an_exception_whose_kind_the_tree_contradicts_is_refused ... ok
test hosted_refuses_each_local_only_option_for_every_group ... ok
test selected_target_preserves_provider_owned_target_fields_in_every_renderer ... ok
test local_success_and_protocol_refusals_report_the_selected_target ... ok
test broken_explicit_hosted_selection_never_falls_back_to_a_local_listener ... ok
test all_target_conflicts_precede_local_and_hosted_state_access ... ok
test every_local_leaf_ignores_broken_login_metadata_and_preserves_its_request ... ok
test oauth_pass1_cli_private_setup_refusal_closes_all_output_formats_and_clears_file ... ok
test an_omitted_target_ignores_a_saved_login_for_every_dual_target_group ... ok

test result: ok. 36 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 4.64s

     Running tests/cli_surface_drift.rs (target/debug/deps/cli_surface_drift-2bd417bc86fb1a78)

running 10 tests
test the_copied_declarations_are_still_copies ... ok
test the_thin_frontend_citation_points_at_the_thin_frontend_test ... ok
test a_command_added_under_a_declared_group_is_refused ... ok
test a_command_added_under_the_wrong_declared_group_is_refused ... ok
test a_committed_tree_whose_group_about_no_longer_matches_the_specification_is_refused ... ok
test a_committed_tree_that_swaps_completions_for_an_undeclared_word_is_refused ... ok
test the_restated_contract_is_green_against_the_unchanged_tree ... ok
test cutting_the_admin_group_over_to_the_generated_tree_is_refused ... ok
test a_target_flag_removed_from_a_group_is_refused_by_the_countdown ... ok
test cargo_can_read_the_committed_emitted_manifest ... ok

test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s

     Running tests/cli_surface_pass_two.rs (target/debug/deps/cli_surface_pass_two-856e479f93a7e600)

running 6 tests
test the_design_document_names_only_constants_that_exist ... ok
test the_design_document_states_the_shape_of_the_exception_list ... ok
test the_target_countdown_candidates_are_derived_from_every_protocol_a_deployment_answers ... ok
test the_design_document_describes_the_countdown_assertion_the_contract_makes ... ok
test the_drift_suites_copies_are_checked_rather_than_cited ... ok
test the_drift_suite_attributes_nothing_to_the_contract_that_is_not_there ... ok

test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests/closed_pipe.rs (target/debug/deps/closed_pipe-2a55dfdb82652fa8)

running 20 tests
test completion_scripts_keep_other_output_write_failures_unsuccessful ... ok
test completion_scripts_still_accept_a_closed_reader ... ok
test stdin_read_errors_stay_unsuccessful_with_a_closed_output_reader ... ok
test admin_authentication_failures_remain_unsuccessful_with_a_closed_reader ... ok
test a_closed_transport_stays_unsuccessful ... ok
test an_unhealthy_doctor_stays_unsuccessful_when_its_report_reader_closes ... ok
test a_healthy_doctor_accepts_a_closed_report_reader ... ok
test text_consumer_closes_early ... ok
test json_consumer_closes_early ... ok
test compact_consumer_closes_early ... ok
test setup_init_commits_its_result_before_a_reader_close_but_keeps_repeat_refusal ... ok
test successful_admin_results_accept_a_closed_reader_in_every_format ... ok
test successful_admin_credential_write_accepts_a_closed_reader ... ok
test yaml_consumer_closes_early ... ok
test every_admin_leaf_preserves_non_broken_pipe_output_failures ... ok
test every_format_really_emits_more_than_a_64_kib_pipe_buffer ... ok
test each_unhealthy_report_class_keeps_its_failure_when_output_closes ... ok
test protocol_refusals_keep_their_failure_when_a_result_reader_closes ... ok
test each_protocol_search_distinguishes_closed_readers_from_other_write_failures ... ok
test a_real_non_broken_pipe_output_failure_stays_unsuccessful ... ok

test result: ok. 20 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 4.44s

     Running tests/first_level_groups.rs (target/debug/deps/first_level_groups-1419b64225476cb5)

running 5 tests
test the_first_level_is_eight_words ... ok
test doctor_reports_the_same_installation_at_both_paths ... ok
test the_serve_group_answers_bare_and_with_help_like_the_other_groups ... ok
test a_path_of_the_new_tree_is_left_alone ... ok
test every_moved_path_still_works_and_names_where_it_went ... ok

test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.14s

     Running tests/moved_paths_are_not_taught.rs (target/debug/deps/moved_paths_are_not_taught-ae98344639011258)

running 1 test
test nothing_this_product_prints_names_a_path_that_moved ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.13s

     Running tests/one_shot_operations.rs (target/debug/deps/one_shot_operations-89c3fca002f9618f)

running 23 tests
test a_transport_that_drops_the_request_is_never_retried_locally ... ok
test a_running_daemon_is_used_without_constructing_a_local_runtime ... ok
test doctor_enumerates_bounded_and_persistent_verbs ... ok
test connection_mutations_require_daemon_before_creating_continuation_state ... ok
test adversary_hosted_refusal_and_target_conflict_never_construct_the_local_runtime ... ok
test existing_or_unsafe_socket_objects_never_trigger_ephemeral_fallback ... ok
test events_and_session_signals_name_the_persistent_daemon_requirement ... ok
test adversary_json_source_and_size_refusals_precede_one_shot_state_creation ... ok
test invalid_bounds_and_unsafe_state_refuse_before_runtime_state_is_opened ... ok
test rate_final_cli_describe_spelling_and_invalid_advice_never_resend ... ok
test final_adversary_kubernetes_candidates_never_publish_a_dead_connection_or_run_auth_exec ... ok
test rate_adversary_cli_keeps_integer_extremes_and_never_resends_before_exit ... ok
test rate_stage2_cli_json_and_yaml_preserve_delay_and_never_resend_an_invoke ... ok
test rate_stage2_one_shot_refusals_preserve_retriable_without_inventing_delay ... ok
test ordinary_search_and_connection_list_use_default_paths_without_a_daemon ... ok
test separate_describe_and_invoke_processes_reuse_the_same_authority_without_a_daemon ... ok
test adversary_uncertain_invoke_never_resends_after_the_control_socket_disappears ... ok
test final_adversary_invalid_provider_output_is_not_resent_and_releases_the_state_root ... ok
test concurrent_commands_and_daemon_start_cannot_take_the_in_flight_invocation_state ... ok
test final_adversary_provider_cursor_survives_two_distinct_one_shot_processes ... ok
test a_changed_authority_or_selected_connection_never_reaches_fixture_egress ... ok
test adversary_caller_input_cannot_rebind_routes_or_revoked_grants ... ok
test browser_session_operations_are_refused_under_canonical_and_published_aliases ... ok

test result: ok. 23 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 18.72s

     Running tests/search_bounds.rs (target/debug/deps/search_bounds-06c509b96b3df413)

running 4 tests
test every_search_help_names_its_protocol_range_and_existing_default ... ok
test every_search_parser_refuses_zero_and_values_above_the_protocol_maximum ... ok
test every_search_preserves_its_default_and_accepts_both_protocol_edges ... ok
test invalid_search_limits_exit_before_target_configuration_or_transport ... ok

test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.27s

   Doc-tests connectors_cli

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

````

Exit 0; 133 passed / 0 failed / 0 ignored. Guard interrupted: false.

Execution cli-clippy; started 2026-09-06T18:48:48.894360+00:00; finished 2026-09-06T18:48:54.046649+00:00. Exact command/cwd/environment:

````json
{
  "cwd": "~/.local/state/worktree/trees/b10x/connectors/wt-4f1de73d0685/crates/connectors-cli",
  "argv": [
    "env",
    "-u",
    "CARGO_TARGET_DIR",
    "RUSTC_WRAPPER=/usr/bin/sccache",
    "TMPDIR=~/.cache/cw6/oa2",
    "CARGO_BUILD_JOBS=1",
    "CARGO_INCREMENTAL=0",
    "CARGO_PROFILE_DEV_DEBUG=0",
    "CARGO_PROFILE_TEST_DEBUG=0",
    "cargo",
    "clippy",
    "--locked",
    "--offline",
    "--workspace",
    "--all-targets",
    "--",
    "-D",
    "warnings"
  ]
}
````

Actual complete output from ~/.cache/connectors-cli-wave-20260906/connect-session-oauth-custody-in-personal-posture/whole-unit-adversary-2/oauth-pass2-cli-clippy.log:

````text
warning: connector-secrets@0.6.5: no live Vault was offered (CONNECTOR_SECRETS_VAULT_ADDR and CONNECTOR_SECRETS_VAULT_TOKEN are unset), so the reqwest HttpTransport is UNEXERCISED by this build: tests/vault_live.rs is compiled #[ignore]d
    Finished `dev` profile [unoptimized] target(s) in 0.36s
````

Exit 0; 0 passed / 0 failed / 0 ignored. Guard interrupted: false.

Execution cli-fmt; started 2026-09-06T18:49:08.819431+00:00; finished 2026-09-06T18:49:13.976866+00:00. Exact command/cwd/environment:

````json
{
  "cwd": "~/.local/state/worktree/trees/b10x/connectors/wt-4f1de73d0685/crates/connectors-cli",
  "argv": [
    "env",
    "-u",
    "CARGO_TARGET_DIR",
    "RUSTC_WRAPPER=/usr/bin/sccache",
    "TMPDIR=~/.cache/cw6/oa2",
    "CARGO_BUILD_JOBS=1",
    "CARGO_INCREMENTAL=0",
    "CARGO_PROFILE_DEV_DEBUG=0",
    "CARGO_PROFILE_TEST_DEBUG=0",
    "cargo",
    "fmt",
    "--all",
    "--",
    "--check"
  ]
}
````

Actual complete output from ~/.cache/connectors-cli-wave-20260906/connect-session-oauth-custody-in-personal-posture/whole-unit-adversary-2/oauth-pass2-cli-fmt.log:

````text

````

Exit 0; 0 passed / 0 failed / 0 ignored. Guard interrupted: false.

Judgement findings: none for this final candidate and test patch.

| File:line | Verdict | Origin | What was measured | What reaches it |
| --- | --- | --- | --- | --- |
| None | None | None | No product failure in the bounded final cases/full runs | Actual callers and synthetic limits stated above |

Attacked without a product failure:

- Corrected authoring presence/serialization, nearby structural schema boundaries and legacy omission defaults.
- PKCE/device cancellation between token receipt and evidence, actual future drop, SQLite/file reopen and credential-reuse refusal.
- Trusted private-file expiry/inode clearing, bounded finish grace, one Create, stable configured target and private-output containment.
- Retained cases for callback consumption/recovery, device slowdown, custody coherence/refresh, raw credentials, schema-4/frozen-reader compatibility, CLI refusal and TTY/MCP privacy.

The compiling slot was released after checks. Resource observations: {"cli_target_cap": 8589934592, "compiling_slots": 1, "disk_floor": 12884901888, "guard_interrupts": 0, "jobs": 1, "maximum_cli_target": 2782941184, "maximum_private_target": 9016225792, "memory_floor": 17179869184, "minimum_disk": 28833808384, "minimum_memory": 38025138176, "minimum_tmpfs": 24585625600, "private_target_cap": 12884901888, "tmpfs_floor": 8589934592}. Exact warm inventories, continuous samples and exits are retained; no guard crossing or cleanup occurred. Root/runtime/console use only this tree’s assigned private target. CLI uses its existing default target and sccache.

Every retained path written outside the worktree follows and is recorded in ~/.cache/connectors-cli-wave-20260906/connect-session-oauth-custody-in-personal-posture/whole-unit-adversary-2/outside-inventory.json. Temporary fixtures are bounded by assigned TMPDIR; surviving files are inventoried without claiming a syscall trace. Cargo/sccache roots are disclosed without a per-object cache write claim.

````text
~/.cache/connectors-cli-wave-20260906/connect-session-oauth-custody-in-personal-posture/whole-unit-adversary-2/candidate-preservation.json
~/.cache/connectors-cli-wave-20260906/connect-session-oauth-custody-in-personal-posture/whole-unit-adversary-2/case-inventory.json
~/.cache/connectors-cli-wave-20260906/connect-session-oauth-custody-in-personal-posture/whole-unit-adversary-2/commands-and-counts.json
~/.cache/connectors-cli-wave-20260906/connect-session-oauth-custody-in-personal-posture/whole-unit-adversary-2/compile-slot-release.json
~/.cache/connectors-cli-wave-20260906/connect-session-oauth-custody-in-personal-posture/whole-unit-adversary-2/correction.patch
~/.cache/connectors-cli-wave-20260906/connect-session-oauth-custody-in-personal-posture/whole-unit-adversary-2/deciding-final-test-inventory.json
~/.cache/connectors-cli-wave-20260906/connect-session-oauth-custody-in-personal-posture/whole-unit-adversary-2/deciding-final-tests.patch
~/.cache/connectors-cli-wave-20260906/connect-session-oauth-custody-in-personal-posture/whole-unit-adversary-2/deciding-tests.patch
~/.cache/connectors-cli-wave-20260906/connect-session-oauth-custody-in-personal-posture/whole-unit-adversary-2/diff-stat.txt
~/.cache/connectors-cli-wave-20260906/connect-session-oauth-custody-in-personal-posture/whole-unit-adversary-2/evidence.sha256
~/.cache/connectors-cli-wave-20260906/connect-session-oauth-custody-in-personal-posture/whole-unit-adversary-2/final-original-test-preservation.json
~/.cache/connectors-cli-wave-20260906/connect-session-oauth-custody-in-personal-posture/whole-unit-adversary-2/final-source-inventory.json
~/.cache/connectors-cli-wave-20260906/connect-session-oauth-custody-in-personal-posture/whole-unit-adversary-2/final-source-scope-proof.json
~/.cache/connectors-cli-wave-20260906/connect-session-oauth-custody-in-personal-posture/whole-unit-adversary-2/formatting-precheck-observation.json
~/.cache/connectors-cli-wave-20260906/connect-session-oauth-custody-in-personal-posture/whole-unit-adversary-2/formatting-precheck-observation/crates/catalog-build/tests/main/catalog_invariants.rs
~/.cache/connectors-cli-wave-20260906/connect-session-oauth-custody-in-personal-posture/whole-unit-adversary-2/formatting-precheck-observation/crates/connector-spec/tests/main/personal_oauth.rs
~/.cache/connectors-cli-wave-20260906/connect-session-oauth-custody-in-personal-posture/whole-unit-adversary-2/frozen-source-inventory.json
~/.cache/connectors-cli-wave-20260906/connect-session-oauth-custody-in-personal-posture/whole-unit-adversary-2/oauth-pass2-cli-clippy.command.json
~/.cache/connectors-cli-wave-20260906/connect-session-oauth-custody-in-personal-posture/whole-unit-adversary-2/oauth-pass2-cli-clippy.log
~/.cache/connectors-cli-wave-20260906/connect-session-oauth-custody-in-personal-posture/whole-unit-adversary-2/oauth-pass2-cli-clippy.resources.jsonl
~/.cache/connectors-cli-wave-20260906/connect-session-oauth-custody-in-personal-posture/whole-unit-adversary-2/oauth-pass2-cli-clippy.result.json
~/.cache/connectors-cli-wave-20260906/connect-session-oauth-custody-in-personal-posture/whole-unit-adversary-2/oauth-pass2-cli-clippy.warm-target.json
~/.cache/connectors-cli-wave-20260906/connect-session-oauth-custody-in-personal-posture/whole-unit-adversary-2/oauth-pass2-cli-fmt.command.json
~/.cache/connectors-cli-wave-20260906/connect-session-oauth-custody-in-personal-posture/whole-unit-adversary-2/oauth-pass2-cli-fmt.log
~/.cache/connectors-cli-wave-20260906/connect-session-oauth-custody-in-personal-posture/whole-unit-adversary-2/oauth-pass2-cli-fmt.resources.jsonl
~/.cache/connectors-cli-wave-20260906/connect-session-oauth-custody-in-personal-posture/whole-unit-adversary-2/oauth-pass2-cli-fmt.result.json
~/.cache/connectors-cli-wave-20260906/connect-session-oauth-custody-in-personal-posture/whole-unit-adversary-2/oauth-pass2-cli-fmt.warm-target.json
~/.cache/connectors-cli-wave-20260906/connect-session-oauth-custody-in-personal-posture/whole-unit-adversary-2/oauth-pass2-cli-full.command.json
~/.cache/connectors-cli-wave-20260906/connect-session-oauth-custody-in-personal-posture/whole-unit-adversary-2/oauth-pass2-cli-full.log
~/.cache/connectors-cli-wave-20260906/connect-session-oauth-custody-in-personal-posture/whole-unit-adversary-2/oauth-pass2-cli-full.resources.jsonl
~/.cache/connectors-cli-wave-20260906/connect-session-oauth-custody-in-personal-posture/whole-unit-adversary-2/oauth-pass2-cli-full.result.json
~/.cache/connectors-cli-wave-20260906/connect-session-oauth-custody-in-personal-posture/whole-unit-adversary-2/oauth-pass2-cli-full.warm-target.json
~/.cache/connectors-cli-wave-20260906/connect-session-oauth-custody-in-personal-posture/whole-unit-adversary-2/oauth-pass2-console-clippy.command.json
~/.cache/connectors-cli-wave-20260906/connect-session-oauth-custody-in-personal-posture/whole-unit-adversary-2/oauth-pass2-console-clippy.log
~/.cache/connectors-cli-wave-20260906/connect-session-oauth-custody-in-personal-posture/whole-unit-adversary-2/oauth-pass2-console-clippy.resources.jsonl
~/.cache/connectors-cli-wave-20260906/connect-session-oauth-custody-in-personal-posture/whole-unit-adversary-2/oauth-pass2-console-clippy.result.json
~/.cache/connectors-cli-wave-20260906/connect-session-oauth-custody-in-personal-posture/whole-unit-adversary-2/oauth-pass2-console-clippy.warm-target.json
~/.cache/connectors-cli-wave-20260906/connect-session-oauth-custody-in-personal-posture/whole-unit-adversary-2/oauth-pass2-console-fmt.command.json
~/.cache/connectors-cli-wave-20260906/connect-session-oauth-custody-in-personal-posture/whole-unit-adversary-2/oauth-pass2-console-fmt.log
~/.cache/connectors-cli-wave-20260906/connect-session-oauth-custody-in-personal-posture/whole-unit-adversary-2/oauth-pass2-console-fmt.resources.jsonl
~/.cache/connectors-cli-wave-20260906/connect-session-oauth-custody-in-personal-posture/whole-unit-adversary-2/oauth-pass2-console-fmt.result.json
~/.cache/connectors-cli-wave-20260906/connect-session-oauth-custody-in-personal-posture/whole-unit-adversary-2/oauth-pass2-console-fmt.warm-target.json
~/.cache/connectors-cli-wave-20260906/connect-session-oauth-custody-in-personal-posture/whole-unit-adversary-2/oauth-pass2-console-full.command.json
~/.cache/connectors-cli-wave-20260906/connect-session-oauth-custody-in-personal-posture/whole-unit-adversary-2/oauth-pass2-console-full.log
~/.cache/connectors-cli-wave-20260906/connect-session-oauth-custody-in-personal-posture/whole-unit-adversary-2/oauth-pass2-console-full.resources.jsonl
~/.cache/connectors-cli-wave-20260906/connect-session-oauth-custody-in-personal-posture/whole-unit-adversary-2/oauth-pass2-console-full.result.json
~/.cache/connectors-cli-wave-20260906/connect-session-oauth-custody-in-personal-posture/whole-unit-adversary-2/oauth-pass2-console-full.warm-target.json
~/.cache/connectors-cli-wave-20260906/connect-session-oauth-custody-in-personal-posture/whole-unit-adversary-2/oauth-pass2-expiry-first.command.json
~/.cache/connectors-cli-wave-20260906/connect-session-oauth-custody-in-personal-posture/whole-unit-adversary-2/oauth-pass2-expiry-first.log
~/.cache/connectors-cli-wave-20260906/connect-session-oauth-custody-in-personal-posture/whole-unit-adversary-2/oauth-pass2-expiry-first.resources.jsonl
~/.cache/connectors-cli-wave-20260906/connect-session-oauth-custody-in-personal-posture/whole-unit-adversary-2/oauth-pass2-expiry-first.result.json
~/.cache/connectors-cli-wave-20260906/connect-session-oauth-custody-in-personal-posture/whole-unit-adversary-2/oauth-pass2-expiry-first.warm-target.json
~/.cache/connectors-cli-wave-20260906/connect-session-oauth-custody-in-personal-posture/whole-unit-adversary-2/oauth-pass2-presence-first.command.json
~/.cache/connectors-cli-wave-20260906/connect-session-oauth-custody-in-personal-posture/whole-unit-adversary-2/oauth-pass2-presence-first.log
~/.cache/connectors-cli-wave-20260906/connect-session-oauth-custody-in-personal-posture/whole-unit-adversary-2/oauth-pass2-presence-first.resources.jsonl
~/.cache/connectors-cli-wave-20260906/connect-session-oauth-custody-in-personal-posture/whole-unit-adversary-2/oauth-pass2-presence-first.result.json
~/.cache/connectors-cli-wave-20260906/connect-session-oauth-custody-in-personal-posture/whole-unit-adversary-2/oauth-pass2-presence-first.warm-target.json
~/.cache/connectors-cli-wave-20260906/connect-session-oauth-custody-in-personal-posture/whole-unit-adversary-2/oauth-pass2-root-clippy.command.json
~/.cache/connectors-cli-wave-20260906/connect-session-oauth-custody-in-personal-posture/whole-unit-adversary-2/oauth-pass2-root-clippy.log
~/.cache/connectors-cli-wave-20260906/connect-session-oauth-custody-in-personal-posture/whole-unit-adversary-2/oauth-pass2-root-clippy.resources.jsonl
~/.cache/connectors-cli-wave-20260906/connect-session-oauth-custody-in-personal-posture/whole-unit-adversary-2/oauth-pass2-root-clippy.result.json
~/.cache/connectors-cli-wave-20260906/connect-session-oauth-custody-in-personal-posture/whole-unit-adversary-2/oauth-pass2-root-clippy.warm-target.json
~/.cache/connectors-cli-wave-20260906/connect-session-oauth-custody-in-personal-posture/whole-unit-adversary-2/oauth-pass2-root-fmt.command.json
~/.cache/connectors-cli-wave-20260906/connect-session-oauth-custody-in-personal-posture/whole-unit-adversary-2/oauth-pass2-root-fmt.log
~/.cache/connectors-cli-wave-20260906/connect-session-oauth-custody-in-personal-posture/whole-unit-adversary-2/oauth-pass2-root-fmt.resources.jsonl
~/.cache/connectors-cli-wave-20260906/connect-session-oauth-custody-in-personal-posture/whole-unit-adversary-2/oauth-pass2-root-fmt.result.json
~/.cache/connectors-cli-wave-20260906/connect-session-oauth-custody-in-personal-posture/whole-unit-adversary-2/oauth-pass2-root-fmt.warm-target.json
~/.cache/connectors-cli-wave-20260906/connect-session-oauth-custody-in-personal-posture/whole-unit-adversary-2/oauth-pass2-root-full.command.json
~/.cache/connectors-cli-wave-20260906/connect-session-oauth-custody-in-personal-posture/whole-unit-adversary-2/oauth-pass2-root-full.log
~/.cache/connectors-cli-wave-20260906/connect-session-oauth-custody-in-personal-posture/whole-unit-adversary-2/oauth-pass2-root-full.resources.jsonl
~/.cache/connectors-cli-wave-20260906/connect-session-oauth-custody-in-personal-posture/whole-unit-adversary-2/oauth-pass2-root-full.result.json
~/.cache/connectors-cli-wave-20260906/connect-session-oauth-custody-in-personal-posture/whole-unit-adversary-2/oauth-pass2-root-full.warm-target.json
~/.cache/connectors-cli-wave-20260906/connect-session-oauth-custody-in-personal-posture/whole-unit-adversary-2/oauth-pass2-runtime-clippy.command.json
~/.cache/connectors-cli-wave-20260906/connect-session-oauth-custody-in-personal-posture/whole-unit-adversary-2/oauth-pass2-runtime-clippy.log
~/.cache/connectors-cli-wave-20260906/connect-session-oauth-custody-in-personal-posture/whole-unit-adversary-2/oauth-pass2-runtime-clippy.resources.jsonl
~/.cache/connectors-cli-wave-20260906/connect-session-oauth-custody-in-personal-posture/whole-unit-adversary-2/oauth-pass2-runtime-clippy.result.json
~/.cache/connectors-cli-wave-20260906/connect-session-oauth-custody-in-personal-posture/whole-unit-adversary-2/oauth-pass2-runtime-clippy.warm-target.json
~/.cache/connectors-cli-wave-20260906/connect-session-oauth-custody-in-personal-posture/whole-unit-adversary-2/oauth-pass2-runtime-fmt.command.json
~/.cache/connectors-cli-wave-20260906/connect-session-oauth-custody-in-personal-posture/whole-unit-adversary-2/oauth-pass2-runtime-fmt.log
~/.cache/connectors-cli-wave-20260906/connect-session-oauth-custody-in-personal-posture/whole-unit-adversary-2/oauth-pass2-runtime-fmt.resources.jsonl
~/.cache/connectors-cli-wave-20260906/connect-session-oauth-custody-in-personal-posture/whole-unit-adversary-2/oauth-pass2-runtime-fmt.result.json
~/.cache/connectors-cli-wave-20260906/connect-session-oauth-custody-in-personal-posture/whole-unit-adversary-2/oauth-pass2-runtime-fmt.warm-target.json
~/.cache/connectors-cli-wave-20260906/connect-session-oauth-custody-in-personal-posture/whole-unit-adversary-2/oauth-pass2-runtime-full.command.json
~/.cache/connectors-cli-wave-20260906/connect-session-oauth-custody-in-personal-posture/whole-unit-adversary-2/oauth-pass2-runtime-full.log
~/.cache/connectors-cli-wave-20260906/connect-session-oauth-custody-in-personal-posture/whole-unit-adversary-2/oauth-pass2-runtime-full.resources.jsonl
~/.cache/connectors-cli-wave-20260906/connect-session-oauth-custody-in-personal-posture/whole-unit-adversary-2/oauth-pass2-runtime-full.result.json
~/.cache/connectors-cli-wave-20260906/connect-session-oauth-custody-in-personal-posture/whole-unit-adversary-2/oauth-pass2-runtime-full.warm-target.json
~/.cache/connectors-cli-wave-20260906/connect-session-oauth-custody-in-personal-posture/whole-unit-adversary-2/oauth-pass2-schema-corrected.command.json
~/.cache/connectors-cli-wave-20260906/connect-session-oauth-custody-in-personal-posture/whole-unit-adversary-2/oauth-pass2-schema-corrected.log
~/.cache/connectors-cli-wave-20260906/connect-session-oauth-custody-in-personal-posture/whole-unit-adversary-2/oauth-pass2-schema-corrected.resources.jsonl
~/.cache/connectors-cli-wave-20260906/connect-session-oauth-custody-in-personal-posture/whole-unit-adversary-2/oauth-pass2-schema-corrected.result.json
~/.cache/connectors-cli-wave-20260906/connect-session-oauth-custody-in-personal-posture/whole-unit-adversary-2/oauth-pass2-schema-corrected.warm-target.json
~/.cache/connectors-cli-wave-20260906/connect-session-oauth-custody-in-personal-posture/whole-unit-adversary-2/oauth-pass2-schema-first.command.json
~/.cache/connectors-cli-wave-20260906/connect-session-oauth-custody-in-personal-posture/whole-unit-adversary-2/oauth-pass2-schema-first.log
~/.cache/connectors-cli-wave-20260906/connect-session-oauth-custody-in-personal-posture/whole-unit-adversary-2/oauth-pass2-schema-first.resources.jsonl
~/.cache/connectors-cli-wave-20260906/connect-session-oauth-custody-in-personal-posture/whole-unit-adversary-2/oauth-pass2-schema-first.result.json
~/.cache/connectors-cli-wave-20260906/connect-session-oauth-custody-in-personal-posture/whole-unit-adversary-2/oauth-pass2-schema-first.warm-target.json
~/.cache/connectors-cli-wave-20260906/connect-session-oauth-custody-in-personal-posture/whole-unit-adversary-2/oauth-pass2-shutdown-first.command.json
~/.cache/connectors-cli-wave-20260906/connect-session-oauth-custody-in-personal-posture/whole-unit-adversary-2/oauth-pass2-shutdown-first.log
~/.cache/connectors-cli-wave-20260906/connect-session-oauth-custody-in-personal-posture/whole-unit-adversary-2/oauth-pass2-shutdown-first.resources.jsonl
~/.cache/connectors-cli-wave-20260906/connect-session-oauth-custody-in-personal-posture/whole-unit-adversary-2/oauth-pass2-shutdown-first.result.json
~/.cache/connectors-cli-wave-20260906/connect-session-oauth-custody-in-personal-posture/whole-unit-adversary-2/oauth-pass2-shutdown-first.warm-target.json
~/.cache/connectors-cli-wave-20260906/connect-session-oauth-custody-in-personal-posture/whole-unit-adversary-2/original-run-cli.py
~/.cache/connectors-cli-wave-20260906/connect-session-oauth-custody-in-personal-posture/whole-unit-adversary-2/original-run-private.py
~/.cache/connectors-cli-wave-20260906/connect-session-oauth-custody-in-personal-posture/whole-unit-adversary-2/original-test-files/crates/catalog-build/tests/main/catalog_invariants.rs
~/.cache/connectors-cli-wave-20260906/connect-session-oauth-custody-in-personal-posture/whole-unit-adversary-2/original-test-files/crates/catalog-reader/tests/main/pack.rs
~/.cache/connectors-cli-wave-20260906/connect-session-oauth-custody-in-personal-posture/whole-unit-adversary-2/original-test-files/crates/connect-session-transport/src/oauth_tests.rs
~/.cache/connectors-cli-wave-20260906/connect-session-oauth-custody-in-personal-posture/whole-unit-adversary-2/original-test-files/crates/connector-spec/tests/main/personal_oauth.rs
~/.cache/connectors-cli-wave-20260906/connect-session-oauth-custody-in-personal-posture/whole-unit-adversary-2/original-test-files/crates/connectors-cli/tests/cli_surface.rs
~/.cache/connectors-cli-wave-20260906/connect-session-oauth-custody-in-personal-posture/whole-unit-adversary-2/original-test-files/crates/connectors-client/tests/personal_oauth_adversary.rs
~/.cache/connectors-cli-wave-20260906/connect-session-oauth-custody-in-personal-posture/whole-unit-adversary-2/original-test-files/crates/connectors-config/src/personal_oauth_tests.rs
~/.cache/connectors-cli-wave-20260906/connect-session-oauth-custody-in-personal-posture/whole-unit-adversary-2/original-test-files/crates/connectors-console/tests/personal_oauth.rs
~/.cache/connectors-cli-wave-20260906/connect-session-oauth-custody-in-personal-posture/whole-unit-adversary-2/original-test-files/crates/connectors-runtime/tests/personal_oauth.rs
~/.cache/connectors-cli-wave-20260906/connect-session-oauth-custody-in-personal-posture/whole-unit-adversary-2/original-test-files/crates/integration-catalog/src/custody_refresh_tests.rs
~/.cache/connectors-cli-wave-20260906/connect-session-oauth-custody-in-personal-posture/whole-unit-adversary-2/original-test-files/crates/integration-catalog/src/oauth_adversary_tests.rs
~/.cache/connectors-cli-wave-20260906/connect-session-oauth-custody-in-personal-posture/whole-unit-adversary-2/original-test-files/crates/integration-catalog/src/oauth_tests.rs
~/.cache/connectors-cli-wave-20260906/connect-session-oauth-custody-in-personal-posture/whole-unit-adversary-2/outside-inventory.json
~/.cache/connectors-cli-wave-20260906/connect-session-oauth-custody-in-personal-posture/whole-unit-adversary-2/owned-test-paths.json
~/.cache/connectors-cli-wave-20260906/connect-session-oauth-custody-in-personal-posture/whole-unit-adversary-2/prior-evidence-verification.json
~/.cache/connectors-cli-wave-20260906/connect-session-oauth-custody-in-personal-posture/whole-unit-adversary-2/raw-report.md
~/.cache/connectors-cli-wave-20260906/connect-session-oauth-custody-in-personal-posture/whole-unit-adversary-2/report-hashes.json
~/.cache/connectors-cli-wave-20260906/connect-session-oauth-custody-in-personal-posture/whole-unit-adversary-2/report.md
~/.cache/connectors-cli-wave-20260906/connect-session-oauth-custody-in-personal-posture/whole-unit-adversary-2/resource-summary.json
~/.cache/connectors-cli-wave-20260906/connect-session-oauth-custody-in-personal-posture/whole-unit-adversary-2/run-cli.py
~/.cache/connectors-cli-wave-20260906/connect-session-oauth-custody-in-personal-posture/whole-unit-adversary-2/run-private.py
~/.cache/connectors-cli-wave-20260906/connect-session-oauth-custody-in-personal-posture/whole-unit-adversary-2/schema-first-catalog-invariants.rs
~/.cache/connectors-cli-wave-20260906/connect-session-oauth-custody-in-personal-posture/whole-unit-adversary-2/schema-first-tests.patch
~/.cache/connectors-cli-wave-20260906/connect-session-oauth-custody-in-personal-posture/whole-unit-adversary-2/schema-fixture-correction.json
~/.cache/connectors-cli-wave-20260906/connect-session-oauth-custody-in-personal-posture/whole-unit-adversary-2/seal-report.py
~/.cache/connectors-cli-wave-20260906/connect-session-oauth-custody-in-personal-posture/whole-unit-adversary-2/status.txt
~/.cache/connectors-cli-wave-20260906/connect-session-oauth-custody-in-personal-posture/whole-unit-adversary-2/summarize-runs.py
~/.cache/connectors-cli-wave-20260906/connect-session-oauth-custody-in-personal-posture/whole-unit-adversary-2/tests.patch
~/.cache/connectors-cli-wave-20260906/connect-session-oauth-custody-in-personal-posture/whole-unit-adversary-2/tmpdir-final-inventory.json
~/.cache/connectors-cli-wave-20260906/connect-session-oauth-custody-in-personal-posture/whole-unit-adversary-2/whole-unit-source.patch
````

Read-only coordinator input: ~/.cache/connectors-cli-wave-20260906/connect-session-oauth-custody-in-personal-posture/whole-unit-adversary-2/brief.md.

Assigned TMPDIR: ~/.cache/cw6/oa2.
Assigned outside build target: /dev/shm/b10x-cli-wave-wt-4f1de73d0685-nr00xjox/runtime-target.
Inside-worktree CLI target: ~/.local/state/worktree/trees/b10x/connectors/wt-4f1de73d0685/crates/connectors-cli/target.
Existing tool-managed cache roots that may be updated: ~/.cache/sccache, ~/.cargo/.global-cache, ~/.cargo/.package-cache, ~/.cargo/.package-cache-mutate.
Surviving temporary files: ~/.cache/cw6/oa2/t924e40/c/b10x/connectors.toml, ~/.cache/cw6/oa2/t924e40/s/b10x/connectors/identity-sessions.json, ~/.cache/cw6/oa2/t924e41/c/b10x/connectors.toml, ~/.cache/cw6/oa2/t924e410/c/b10x/connectors.toml, ~/.cache/cw6/oa2/t924e410/s/b10x/connectors/identity-sessions.json, ~/.cache/cw6/oa2/t924e411/c/b10x/connectors.toml, ~/.cache/cw6/oa2/t924e411/s/b10x/connectors/identity-sessions.json, ~/.cache/cw6/oa2/t924e412/c/b10x/connectors.toml, ~/.cache/cw6/oa2/t924e412/s/b10x/connectors/identity-sessions.json, ~/.cache/cw6/oa2/t924e413/c/b10x/connectors.toml, ~/.cache/cw6/oa2/t924e413/s/b10x/connectors/identity-sessions.json, ~/.cache/cw6/oa2/t924e414/c/b10x/connectors.toml, ~/.cache/cw6/oa2/t924e414/s/b10x/connectors/identity-sessions.json, ~/.cache/cw6/oa2/t924e415/c/b10x/connectors.toml, ~/.cache/cw6/oa2/t924e415/s/b10x/connectors/identity-sessions.json, ~/.cache/cw6/oa2/t924e416/c/b10x/connectors.toml, ~/.cache/cw6/oa2/t924e416/s/b10x/connectors/identity-sessions.json, ~/.cache/cw6/oa2/t924e417/c/b10x/connectors.toml, ~/.cache/cw6/oa2/t924e417/s/b10x/connectors/identity-sessions.json, ~/.cache/cw6/oa2/t924e418/c/b10x/connectors.toml, ~/.cache/cw6/oa2/t924e418/s/b10x/connectors/identity-sessions.json, ~/.cache/cw6/oa2/t924e419/c/b10x/connectors.toml, ~/.cache/cw6/oa2/t924e419/s/b10x/connectors/identity-sessions.json, ~/.cache/cw6/oa2/t924e41a/c/b10x/connectors.toml, ~/.cache/cw6/oa2/t924e41a/s/b10x/connectors/identity-sessions.json, ~/.cache/cw6/oa2/t924e41b/c/b10x/connectors.toml, ~/.cache/cw6/oa2/t924e41b/s/b10x/connectors/identity-sessions.json, ~/.cache/cw6/oa2/t924e41c/c/b10x/connectors.toml, ~/.cache/cw6/oa2/t924e41c/s/b10x/connectors/identity-sessions.json, ~/.cache/cw6/oa2/t924e41d/c/b10x/connectors.toml, ~/.cache/cw6/oa2/t924e41d/s/b10x/connectors/identity-sessions.json, ~/.cache/cw6/oa2/t924e41e/c/b10x/connectors.toml, ~/.cache/cw6/oa2/t924e41e/s/b10x/connectors/identity-sessions.json, ~/.cache/cw6/oa2/t924e41f/c/b10x/connectors.toml, ~/.cache/cw6/oa2/t924e41f/s/b10x/connectors/identity-sessions.json, ~/.cache/cw6/oa2/t924e42/c/b10x/connectors.toml, ~/.cache/cw6/oa2/t924e42/s/b10x/connectors/.connectors.lock, ~/.cache/cw6/oa2/t924e42/s/b10x/connectors/.credentials.store.lease, ~/.cache/cw6/oa2/t924e42/s/b10x/connectors/credentials.store, ~/.cache/cw6/oa2/t924e42/s/b10x/connectors/event-reply-claims.sqlite, ~/.cache/cw6/oa2/t924e42/s/b10x/connectors/identity-sessions.json, ~/.cache/cw6/oa2/t924e420/c/b10x/connectors.toml, ~/.cache/cw6/oa2/t924e420/s/b10x/connectors/identity-sessions.json, ~/.cache/cw6/oa2/t924e421/c/b10x/connectors.toml, ~/.cache/cw6/oa2/t924e421/s/b10x/connectors/identity-sessions.json, ~/.cache/cw6/oa2/t924e422/c/b10x/connectors.toml, ~/.cache/cw6/oa2/t924e422/s/b10x/connectors/identity-sessions.json, ~/.cache/cw6/oa2/t924e423/c/b10x/connectors.toml, ~/.cache/cw6/oa2/t924e423/s/b10x/connectors/identity-sessions.json, ~/.cache/cw6/oa2/t924e424/c/b10x/connectors.toml, ~/.cache/cw6/oa2/t924e424/s/b10x/connectors/identity-sessions.json, ~/.cache/cw6/oa2/t924e425/c/b10x/connectors.toml, ~/.cache/cw6/oa2/t924e425/s/b10x/connectors/identity-sessions.json, ~/.cache/cw6/oa2/t924e426/c/b10x/connectors.toml, ~/.cache/cw6/oa2/t924e426/s/b10x/connectors/identity-sessions.json, ~/.cache/cw6/oa2/t924e427/c/b10x/connectors.toml, ~/.cache/cw6/oa2/t924e427/s/b10x/connectors/identity-sessions.json, ~/.cache/cw6/oa2/t924e428/c/b10x/connectors.toml, ~/.cache/cw6/oa2/t924e428/s/b10x/connectors/identity-sessions.json, ~/.cache/cw6/oa2/t924e429/c/b10x/connectors.toml, ~/.cache/cw6/oa2/t924e429/s/b10x/connectors/identity-sessions.json, ~/.cache/cw6/oa2/t924e42a/c/b10x/connectors.toml, ~/.cache/cw6/oa2/t924e42a/s/b10x/connectors/identity-sessions.json, ~/.cache/cw6/oa2/t924e42b/c/b10x/connectors.toml, ~/.cache/cw6/oa2/t924e42b/s/b10x/connectors/identity-sessions.json, ~/.cache/cw6/oa2/t924e42c/c/b10x/connectors.toml, ~/.cache/cw6/oa2/t924e42d/c/b10x/connectors.toml, ~/.cache/cw6/oa2/t924e42e/c/b10x/connectors.toml, ~/.cache/cw6/oa2/t924e43/c/b10x/connectors.toml, ~/.cache/cw6/oa2/t924e43/s/b10x/connectors/identity-sessions.json, ~/.cache/cw6/oa2/t924e44/c/b10x/connectors.toml, ~/.cache/cw6/oa2/t924e44/s/b10x/connectors/identity-sessions.json, ~/.cache/cw6/oa2/t924e45/c/b10x/connectors.toml, ~/.cache/cw6/oa2/t924e46/c/b10x/connectors.toml, ~/.cache/cw6/oa2/t924e46/s/b10x/connectors/identity-sessions.json, ~/.cache/cw6/oa2/t924e47/c/b10x/connectors.toml, ~/.cache/cw6/oa2/t924e48/c/b10x/connectors.toml, ~/.cache/cw6/oa2/t924e48/s/b10x/connectors/identity-sessions.json, ~/.cache/cw6/oa2/t924e49/c/b10x/connectors.toml, ~/.cache/cw6/oa2/t924e4a/c/b10x/connectors.toml, ~/.cache/cw6/oa2/t924e4b/c/b10x/connectors.toml, ~/.cache/cw6/oa2/t924e4c/c/b10x/connectors.toml, ~/.cache/cw6/oa2/t924e4d/c/b10x/connectors.toml, ~/.cache/cw6/oa2/t924e4d/s/b10x/connectors/identity-sessions.json, ~/.cache/cw6/oa2/t924e4e/c/b10x/connectors.toml, ~/.cache/cw6/oa2/t924e4e/s/b10x/connectors/identity-sessions.json, ~/.cache/cw6/oa2/t924e4f/c/b10x/connectors.toml, ~/.cache/cw6/oa2/t924e4f/s/b10x/connectors/identity-sessions.json.

The public counterpart replaces only the absolute local home-directory prefix with ~; all other prose, commands/output and findings remain unchanged. Paths are plain/code references with no local Markdown links. ~/.cache/connectors-cli-wave-20260906/connect-session-oauth-custody-in-personal-posture/whole-unit-adversary-2/report-hashes.json contains report/test/source hashes; ~/.cache/connectors-cli-wave-20260906/connect-session-oauth-custody-in-personal-posture/whole-unit-adversary-2/evidence.sha256 seals retained scratch files except itself.

```findings
[]
```
