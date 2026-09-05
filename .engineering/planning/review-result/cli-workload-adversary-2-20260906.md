---
format: aep.planning-md/1
id: review-result:cli-workload-adversary-2-20260906
kind: review-result
status: active
title: Workload inventory adversary pass 2
refs:
- provider: git
  reference: d35baa0d06d68657286bcefec136ea6f6c39de22
relations:
- reviews: story:personal-local-workload-read
revision: 1
---
unit: story:personal-local-workload-read at d35baa0d06d68657286bcefec136ea6f6c39de22 plus test-only additions
verdict: nothing found
cases: executed 60→63, red 0
origin: introduced 0 / pre-existing 0 / undecided 0
wrote-outside-worktree: 14 report/log/exit files in assigned adversary scratch; assigned short TMPDIR used
needs-coordinator: none

```text
 .../src/local_inventory_tests.rs                   | 203 +++++++++++++++++++++
 1 file changed, 203 insertions(+)
```

## 1. Scope and test-only proof

The diff stat immediately above contains one test file only. Its full numstat is 203 insertions and zero deletions in `crates/integration-kubernetes/src/local_inventory_tests.rs`. No production file was changed, even temporarily, and no existing case was changed, skipped, weakened or deleted. No AEP or Git mutation command, live provider call, daemon action, or subagent was used.

This is the second and final adversarial pass. Its subject is the corrected head d35baa0d06d68657286bcefec136ea6f6c39de22 against source base 4769ce32a331bb8daddaf066b69459c4148fb749. The supplied correction report's actual final package runner provides the before count of 60; no test ran before all three new cases existed. The implementation diff, Acceptance, new schemas and ESS values, guide, current tests, prior adversary report and correction report were read. Local backend dispatch, registry lease translation, local wire validation and cursor-store callers were inspected without moving the tree.

Public-report handling: report.md replaces only the local home prefix with `~`; raw-report.md preserves the original commands, runner output and absolute paths. Outcomes, counts, findings and repository-relative source paths are unchanged.

## 2. Added cases and individual runs

All cases were added to `crates/integration-kubernetes/src/local_inventory_tests.rs` before the first command below. They drive the real KubernetesLocalBackend operation handler through an in-memory kube Client service, using no external cluster.

- Line 821, `adversary_final_invalid_inputs_do_not_spend_a_live_inventory_cursor`: rejects null, numeric strings, fractional/out-of-range and very large numeric limits, unknown fields, an uppercase namespace, a positional array and null cursor before further I/O; then uses the original live cursor with integer-valued decimal limit 100.0. Green on first run.
- Line 904, `adversary_final_overlapping_cursor_replays_dispatch_only_once`: yields while the first continuation request is in flight and runs a second invocation with the same cursor; exactly one succeeds, the other returns StaleAuthority, and only one continuation request reaches the cluster fixture. Green on first run.
- Line 964, `adversary_final_empty_fetch_budget_retains_continuation_and_malformed_pages_refuse`: eight empty upstream pages consume the fetch budget and still return an opaque continuation; the next invocation obtains the remaining deployment. Separate null/array continuation values and null items return Unavailable. Green on first run.

Each case follows an operator-reachable boundary: operation invoke passes input JSON into the local daemon; RequestEnvelope validation checks bounds rather than operation-specific JSON Schema; the registry delegates to this handler after translating its description lease. The concurrent case models two invocations using the same returned cursor; the malformed provider cases model an unsuccessful upstream response shape and make no claim about a real cluster having emitted it.

Every Cargo invocation used the assigned worktree ~/.local/state/worktree/trees/b10x/connectors/wt-728fb204f8cb and this environment:

```text
env -u CARGO_TARGET_DIR
TMPDIR=~/.cache/cw6/k
RUSTC_WRAPPER=/usr/bin/sccache
CARGO_INCREMENTAL=0
CARGO_PROFILE_DEV_DEBUG=0
CARGO_PROFILE_TEST_DEBUG=0
CARGO_BUILD_JOBS=3
```

The new test file was formatted using `rustfmt --edition 2021 crates/integration-kubernetes/src/local_inventory_tests.rs` before the first case ran; the zero-deletion diff confirms the existing cases were preserved.

```text
cargo test --manifest-path crates/connectors-runtime/Cargo.toml -p integration-kubernetes --locked adversary_final_invalid_inputs_do_not_spend_a_live_inventory_cursor
   Compiling integration-kubernetes v0.6.5 (~/.local/state/worktree/trees/b10x/connectors/wt-728fb204f8cb/crates/integration-kubernetes)
    Finished `test` profile [unoptimized] target(s) in 6.05s
     Running unittests src/lib.rs (crates/connectors-runtime/target/debug/deps/integration_kubernetes-524a4913b92427e1)

running 1 test
test local::inventory_tests::adversary_final_invalid_inputs_do_not_spend_a_live_inventory_cursor ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 62 filtered out; finished in 0.00s

exit: 0
```

```text
cargo test --manifest-path crates/connectors-runtime/Cargo.toml -p integration-kubernetes --locked adversary_final_overlapping_cursor_replays_dispatch_only_once
    Finished `test` profile [unoptimized] target(s) in 0.15s
     Running unittests src/lib.rs (crates/connectors-runtime/target/debug/deps/integration_kubernetes-524a4913b92427e1)

running 1 test
test local::inventory_tests::adversary_final_overlapping_cursor_replays_dispatch_only_once ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 62 filtered out; finished in 0.00s

exit: 0
```

```text
cargo test --manifest-path crates/connectors-runtime/Cargo.toml -p integration-kubernetes --locked adversary_final_empty_fetch_budget_retains_continuation_and_malformed_pages_refuse
    Finished `test` profile [unoptimized] target(s) in 0.15s
     Running unittests src/lib.rs (crates/connectors-runtime/target/debug/deps/integration_kubernetes-524a4913b92427e1)

running 1 test
test local::inventory_tests::adversary_final_empty_fetch_budget_retains_continuation_and_malformed_pages_refuse ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 62 filtered out; finished in 0.00s

exit: 0
```

## 3. Relevant suite and focused checks

The package suite ran after all three individual cases.

```text
cargo test --manifest-path crates/connectors-runtime/Cargo.toml -p integration-kubernetes --locked
   Compiling integration-kubernetes v0.6.5 (~/.local/state/worktree/trees/b10x/connectors/wt-728fb204f8cb/crates/integration-kubernetes)
    Finished `test` profile [unoptimized] target(s) in 2.78s
     Running unittests src/lib.rs (crates/connectors-runtime/target/debug/deps/integration_kubernetes-524a4913b92427e1)

running 63 tests
test hosted::database_tests::a_404_from_a_served_group_is_an_error_not_an_empty_inventory ... ok
test hosted::database_tests::database_endpoint_bindings_appear_per_admitted_namespace_only ... ok
test hosted::database_tests::search_lists_the_databases_datasource_under_its_terms ... ok
test hosted::database_tests::a_database_read_on_a_non_admitted_namespace_is_not_granted ... ok
test hosted::tests::a_status_invocation_survives_the_scope_change_between_describe_and_invoke ... ok
test hosted::tests::deployment_projection_requires_observed_available_replicas ... ok
test hosted::tests::an_upstream_log_refusal_surfaces_as_not_granted ... ok
test hosted::tests::describing_without_a_read_grant_names_the_grant_rather_than_a_missing_datasource ... ok
test hosted::tests::hosted_connection_projection_is_value_free_and_tenant_bound ... ok
test hosted::tests::a_missing_namespace_grant_names_the_namespace_and_the_group_that_carries_it ... ok
test hosted::tests::pod_log_invoke_enforces_the_input_caps_before_the_reader ... ok
test hosted::tests::pod_log_invoke_passes_the_input_through_and_defaults_tail_lines ... ok
test hosted::database_tests::a_cluster_without_crossplane_discovers_nothing ... ok
test hosted::tests::an_unknown_binding_is_named_rather_than_reported_as_an_ungranted_one ... ok
test hosted::database_tests::database_endpoint_list_derives_descriptors_from_both_engines ... ok
test hosted::database_tests::database_datasource_description_names_the_projection_for_read_principals_only ... ok
test local::inventory_tests::adversary_inventory_rejects_null_cursor_as_its_published_schema_requires ... ok
test local::inventory_tests::inventory_discovery_publishes_both_reads_for_every_activated_connection ... ok
test local::inventory_tests::adversary_inventory_walks_empty_pages_and_preserves_all_regular_container_images ... ok
test local::inventory_tests::inventory_namespaces_are_configured_admission_without_cluster_enumeration ... ok
test local::inventory_tests::adversary_final_overlapping_cursor_replays_dispatch_only_once ... ok
test local::inventory_tests::inventory_inputs_require_the_published_object_shape ... ok
test local::inventory_tests::inventory_fresh_cursor_cannot_cross_a_connection_or_namespace_boundary ... ok
test local::inventory_tests::inventory_preserves_owner_and_description_lease_admission ... ok
test local::inventory_tests::inventory_optional_output_cursor_is_omitted_or_a_string_never_null ... ok
test hosted::database_tests::no_secret_value_ever_appears_in_database_endpoint_output ... ok
test local::inventory_tests::inventory_preserves_rbac_refusal ... ok
test hosted::database_tests::database_endpoint_get_returns_one_descriptor_by_name ... ok
test hosted::tests::a_workload_read_survives_the_access_token_rotation_between_describe_and_read ... ok
test local::inventory_tests::inventory_reads_each_selected_cluster_and_retains_scaled_to_zero_template_images ... ok
test local::inventory_tests::inventory_optional_inputs_distinguish_absence_from_explicit_values ... ok
test local::tests::a_half_attached_cluster_is_not_published ... ok
test local::tests::a_renamed_argocd_release_is_recognized_by_its_identity_label ... ok
test local::tests::argocd_recognition_takes_the_api_service_and_none_of_its_siblings ... ok
test local::tests::every_activated_cluster_is_published_in_a_stable_order ... ok
test local::inventory_tests::inventory_shared_reader_preserves_the_existing_compact_datasource_shape ... ok
test local::inventory_tests::inventory_refuses_nonactivated_connections_unadmitted_namespaces_and_bad_inputs_before_io ... ok
test hosted::tests::datasource_projects_only_safe_workload_fields_for_granted_namespaces ... ok
test local::tests::monitoring_service_recognition_is_curated ... ok
test local::inventory_tests::inventory_optional_limit_accepts_every_in_range_integer_number_representation ... ok
test local::tests::providers_that_need_a_credential_are_not_materializable_here ... ok
test local::tests::service_observation_pins_uid_and_one_closed_tcp_port ... ok
test local_workloads::tests::a_binding_ref_that_names_no_configured_namespace_is_the_callers_mistake ... ok
test local::tests::the_argocd_observation_pins_the_api_port_rather_than_the_redirect ... ok
test local_workloads::tests::a_name_that_is_not_a_dns_label_never_reaches_a_request_path ... ok
test local_workloads::tests::a_cluster_listing_becomes_the_same_compact_record_the_deployment_returns ... ok
test local_workloads::tests::query_values_are_encoded_rather_than_interpolated ... ok
test hosted::database_tests::database_endpoint_listing_pages_across_both_engines ... ok
test local::inventory_tests::inventory_refuses_upstream_page_overrun_and_wrong_namespace ... ok
test local::tests::insecure_api_server_contexts_are_not_candidates ... ok
test local::inventory_tests::inventory_pagination_is_bounded_and_cursors_bind_connection_and_namespace ... ok
test local_workloads::tests::the_local_placement_publishes_the_deployments_projection_verbatim ... ok
test local::tests::passive_candidates_expose_only_context_label_and_opaque_evidence ... ok
test hosted::tests::search_lists_pod_logs_for_read_group_principals_and_hides_it_otherwise ... ok
test hosted::tests::pod_log_description_carries_schemas_and_a_lease_for_read_principals_only ... ok
test hosted::tests::pod_log_invoke_refuses_a_non_admitted_namespace_and_a_stale_lease ... ok
test hosted::tests::restart_requires_sre_group_and_exact_resource_authority_without_local_approval ... ok
test hosted::tests::read_only_status_is_description_bound_and_namespace_scoped ... ok
test local::inventory_tests::adversary_final_invalid_inputs_do_not_spend_a_live_inventory_cursor ... ok
test local::inventory_tests::adversary_final_empty_fetch_budget_retains_continuation_and_malformed_pages_refuse ... ok
test hosted::tests::an_oversized_log_body_is_front_trimmed_inside_a_validating_envelope ... ok
test local::inventory_tests::inventory_refuses_oversized_provider_and_projected_values ... ok
test hosted::paging_tests::a_busy_namespace_lists_in_full_despite_the_upstream_response_bound ... ok

test result: ok. 63 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.10s

   Doc-tests integration_kubernetes

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

exit: 0
```

The unit-test lane executed all 63 cases, with 63 passed and 0 failed. Documentation tests selected 0 cases. No new case was filtered out of the complete suite. The repository-wide gate remains the coordinator's work.

```text
cargo fmt --manifest-path crates/connectors-runtime/Cargo.toml -p integration-kubernetes -- --check
exit: 0
```

```text
cargo clippy --manifest-path crates/connectors-runtime/Cargo.toml -p integration-kubernetes --all-targets --locked -- -D warnings
    Checking integration-kubernetes v0.6.5 (~/.local/state/worktree/trees/b10x/connectors/wt-728fb204f8cb/crates/integration-kubernetes)
    Finished `dev` profile [unoptimized] target(s) in 2.13s
exit: 0
```

`git --no-pager diff --check` returned exit 0 with no output. Build output stayed in the assigned worktree's default `crates/connectors-runtime/target` directory.

## 4. Findings

Nothing found.

## 5. Attacks that stayed green

- The corrected input-schema class rejects malformed values before spending a live cursor or reaching cluster I/O, while preserving a valid integral decimal limit.
- Overlapping reuse of a continuation cursor dispatches only once.
- An all-empty upstream fetch budget remains resumable, and malformed upstream collection/continuation shapes refuse.
- All inherited 60 cases, including both first-pass adversarial cases and the four correction-class cases, remain passing.

## 6. Paths written outside the worktree

The fourteen persistent files written by this pass are:

- ~/.cache/connectors-cli-wave-20260906/personal-local-workload-read/adversary-2/invalid-live-cursor.log
- ~/.cache/connectors-cli-wave-20260906/personal-local-workload-read/adversary-2/invalid-live-cursor.exit
- ~/.cache/connectors-cli-wave-20260906/personal-local-workload-read/adversary-2/overlapping-replay.log
- ~/.cache/connectors-cli-wave-20260906/personal-local-workload-read/adversary-2/overlapping-replay.exit
- ~/.cache/connectors-cli-wave-20260906/personal-local-workload-read/adversary-2/empty-budget-malformed.log
- ~/.cache/connectors-cli-wave-20260906/personal-local-workload-read/adversary-2/empty-budget-malformed.exit
- ~/.cache/connectors-cli-wave-20260906/personal-local-workload-read/adversary-2/package-suite.log
- ~/.cache/connectors-cli-wave-20260906/personal-local-workload-read/adversary-2/package-suite.exit
- ~/.cache/connectors-cli-wave-20260906/personal-local-workload-read/adversary-2/fmt.log
- ~/.cache/connectors-cli-wave-20260906/personal-local-workload-read/adversary-2/fmt.exit
- ~/.cache/connectors-cli-wave-20260906/personal-local-workload-read/adversary-2/clippy.log
- ~/.cache/connectors-cli-wave-20260906/personal-local-workload-read/adversary-2/clippy.exit
- ~/.cache/connectors-cli-wave-20260906/personal-local-workload-read/adversary-2/raw-report.md
- ~/.cache/connectors-cli-wave-20260906/personal-local-workload-read/adversary-2/report.md

The assigned temporary root ~/.cache/cw6/k was supplied to compiler/test tools. The coordinator-configured sccache service manages its existing shared compiler cache. No alternative scratch or build target was selected. The provided brief and earlier reports were read only.

```findings
[]
```
