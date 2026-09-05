---
format: aep.planning-md/1
id: review-result:cli-workload-adversary-1-20260906
kind: review-result
status: active
title: Workload inventory adversary pass 1
refs:
- provider: git
  reference: 99d4d74e096c6fffb9e95c464f3ef04e47871fd8
relations:
- reviews: story:personal-local-workload-read
revision: 1
---
unit: story:personal-local-workload-read at 99d4d74e096c6fffb9e95c464f3ef04e47871fd8 plus test-only additions
verdict: CONFIRMED
cases: executed 54→56, red 1
origin: introduced 1 / pre-existing 0 / undecided 0
wrote-outside-worktree: 7 report/log/exit files in assigned adversary scratch; assigned short TMPDIR used
needs-coordinator: align cursor validation with the published string-only schema and rerun the retained case

```text
 .../src/local_inventory_tests.rs                   | 59 ++++++++++++++++++++++
 1 file changed, 59 insertions(+)
```

## 1. Scope and test-only proof

The diff stat immediately above is `git --no-pager diff --stat` after this pass. It contains one test file only. No production file was edited, even temporarily; no existing case was changed, weakened, skipped, or deleted. No AEP, Git mutation, live provider call, daemon action, or subagent was used.

Read the complete adversary charter, original unit brief, implementor report, repository instructions, full implementation diff from 4769ce32a331bb8daddaf066b69459c4148fb749 to 99d4d74e096c6fffb9e95c464f3ef04e47871fd8, story Acceptance, ESS inventory values, new schemas and guide, and relevant backend, local server, registry, CLI and cursor-store callers before writing the cases. The before count of 54 comes from the implementor's final actual package runner; no suite ran before the new cases existed.

## 2. Added cases and individual runs

Both cases were appended to `crates/integration-kubernetes/src/local_inventory_tests.rs` before any test command ran in this pass.

- `adversary_inventory_rejects_null_cursor_as_its_published_schema_requires` at line 578: reads the operation's published cursor schema, supplies explicit JSON null through `KubernetesLocalBackend::handle`, and asserts InvalidInput with no cluster request. It is red: the backend returned a successful empty inventory and made one fixture cluster request.
- `adversary_inventory_walks_empty_pages_and_preserves_all_regular_container_images` at line 601: starts with an empty provider page carrying a continuation token containing reserved URL characters, then returns a dotted deployment name, two regular containers, omitted replicas and omitted status. It asserts encoded continuation, both image identities, desired replicas 1, ready replicas 0, and the exact credential-free inventory output. It is green.

Every runner used this environment from the assigned worktree:

```text
env -u CARGO_TARGET_DIR
TMPDIR=~/.cache/cw6/k
RUSTC_WRAPPER=/usr/bin/sccache
CARGO_INCREMENTAL=0
CARGO_PROFILE_DEV_DEBUG=0
CARGO_PROFILE_TEST_DEBUG=0
CARGO_BUILD_JOBS=3
```

First individual run, before the package suite:

```text
cargo test --manifest-path crates/connectors-runtime/Cargo.toml -p integration-kubernetes --locked adversary_inventory_rejects_null_cursor_as_its_published_schema_requires
   Compiling integration-kubernetes v0.6.5 (~/.local/state/worktree/trees/b10x/connectors/wt-728fb204f8cb/crates/integration-kubernetes)
    Finished `test` profile [unoptimized] target(s) in 3.23s
     Running unittests src/lib.rs (crates/connectors-runtime/target/debug/deps/integration_kubernetes-524a4913b92427e1)

running 1 test
test local::inventory_tests::adversary_inventory_rejects_null_cursor_as_its_published_schema_requires ... FAILED

failures:

---- local::inventory_tests::adversary_inventory_rejects_null_cursor_as_its_published_schema_requires stdout ----

thread 'local::inventory_tests::adversary_inventory_rejects_null_cursor_as_its_published_schema_requires' (537503) panicked at ~/.local/state/worktree/trees/b10x/connectors/wt-728fb204f8cb/crates/integration-kubernetes/src/local_inventory_tests.rs:592:9:
the published string-only cursor contract must refuse explicit null before cluster I/O; got Ok(Object {"connection_ref": String("connection:alpha"), "deployments": Array [], "namespace": String("apps")}) and 1 request(s)
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace


failures:
    local::inventory_tests::adversary_inventory_rejects_null_cursor_as_its_published_schema_requires

test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 55 filtered out; finished in 0.00s

error: test failed, to rerun pass `-p integration-kubernetes --lib`
exit: 101
```

Second individual run, also before the package suite:

```text
cargo test --manifest-path crates/connectors-runtime/Cargo.toml -p integration-kubernetes --locked adversary_inventory_walks_empty_pages_and_preserves_all_regular_container_images
    Finished `test` profile [unoptimized] target(s) in 0.17s
     Running unittests src/lib.rs (crates/connectors-runtime/target/debug/deps/integration_kubernetes-524a4913b92427e1)

running 1 test
test local::inventory_tests::adversary_inventory_walks_empty_pages_and_preserves_all_regular_container_images ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 55 filtered out; finished in 0.00s

exit: 0
```

## 3. Relevant suite, run after both individual cases

```text
cargo test --manifest-path crates/connectors-runtime/Cargo.toml -p integration-kubernetes --locked
   Compiling integration-kubernetes v0.6.5 (~/.local/state/worktree/trees/b10x/connectors/wt-728fb204f8cb/crates/integration-kubernetes)
    Finished `test` profile [unoptimized] target(s) in 2.74s
     Running unittests src/lib.rs (crates/connectors-runtime/target/debug/deps/integration_kubernetes-524a4913b92427e1)

running 56 tests
test hosted::database_tests::a_404_from_a_served_group_is_an_error_not_an_empty_inventory ... ok
test hosted::database_tests::search_lists_the_databases_datasource_under_its_terms ... ok
test hosted::database_tests::database_endpoint_bindings_appear_per_admitted_namespace_only ... ok
test hosted::database_tests::a_database_read_on_a_non_admitted_namespace_is_not_granted ... ok
test hosted::tests::deployment_projection_requires_observed_available_replicas ... ok
test hosted::tests::an_upstream_log_refusal_surfaces_as_not_granted ... ok
test hosted::tests::a_status_invocation_survives_the_scope_change_between_describe_and_invoke ... ok
test hosted::tests::pod_log_description_carries_schemas_and_a_lease_for_read_principals_only ... ok
test hosted::tests::an_unknown_binding_is_named_rather_than_reported_as_an_ungranted_one ... ok
test hosted::tests::search_lists_pod_logs_for_read_group_principals_and_hides_it_otherwise ... ok
test hosted::tests::pod_log_invoke_enforces_the_input_caps_before_the_reader ... ok
test hosted::database_tests::a_cluster_without_crossplane_discovers_nothing ... ok
test hosted::tests::a_missing_namespace_grant_names_the_namespace_and_the_group_that_carries_it ... ok
test hosted::database_tests::database_datasource_description_names_the_projection_for_read_principals_only ... ok
test local::inventory_tests::adversary_inventory_rejects_null_cursor_as_its_published_schema_requires ... FAILED
test local::inventory_tests::adversary_inventory_walks_empty_pages_and_preserves_all_regular_container_images ... ok
test local::inventory_tests::inventory_namespaces_are_configured_admission_without_cluster_enumeration ... ok
test hosted::database_tests::database_endpoint_list_derives_descriptors_from_both_engines ... ok
test local::inventory_tests::inventory_preserves_rbac_refusal ... ok
test local::inventory_tests::inventory_preserves_owner_and_description_lease_admission ... ok
test hosted::database_tests::database_endpoint_get_returns_one_descriptor_by_name ... ok
test hosted::database_tests::no_secret_value_ever_appears_in_database_endpoint_output ... ok
test local::inventory_tests::inventory_fresh_cursor_cannot_cross_a_connection_or_namespace_boundary ... ok
test hosted::tests::datasource_projects_only_safe_workload_fields_for_granted_namespaces ... ok
test local::tests::a_half_attached_cluster_is_not_published ... ok
test local::tests::a_renamed_argocd_release_is_recognized_by_its_identity_label ... ok
test local::inventory_tests::inventory_refuses_nonactivated_connections_unadmitted_namespaces_and_bad_inputs_before_io ... ok
test local::tests::every_activated_cluster_is_published_in_a_stable_order ... ok
test local::tests::argocd_recognition_takes_the_api_service_and_none_of_its_siblings ... ok
test local::inventory_tests::inventory_reads_each_selected_cluster_and_retains_scaled_to_zero_template_images ... ok
test local::tests::monitoring_service_recognition_is_curated ... ok
test local::inventory_tests::inventory_shared_reader_preserves_the_existing_compact_datasource_shape ... ok
test local::tests::providers_that_need_a_credential_are_not_materializable_here ... ok
test hosted::database_tests::database_endpoint_listing_pages_across_both_engines ... ok
test hosted::tests::a_workload_read_survives_the_access_token_rotation_between_describe_and_read ... ok
test local::tests::service_observation_pins_uid_and_one_closed_tcp_port ... ok
test local_workloads::tests::a_binding_ref_that_names_no_configured_namespace_is_the_callers_mistake ... ok
test local_workloads::tests::a_name_that_is_not_a_dns_label_never_reaches_a_request_path ... ok
test local::tests::the_argocd_observation_pins_the_api_port_rather_than_the_redirect ... ok
test local_workloads::tests::query_values_are_encoded_rather_than_interpolated ... ok
test local_workloads::tests::a_cluster_listing_becomes_the_same_compact_record_the_deployment_returns ... ok
test local::inventory_tests::inventory_refuses_upstream_page_overrun_and_wrong_namespace ... ok
test local::inventory_tests::inventory_pagination_is_bounded_and_cursors_bind_connection_and_namespace ... ok
test local::tests::insecure_api_server_contexts_are_not_candidates ... ok
test local_workloads::tests::the_local_placement_publishes_the_deployments_projection_verbatim ... ok
test local::tests::passive_candidates_expose_only_context_label_and_opaque_evidence ... ok
test hosted::tests::hosted_connection_projection_is_value_free_and_tenant_bound ... ok
test hosted::tests::describing_without_a_read_grant_names_the_grant_rather_than_a_missing_datasource ... ok
test hosted::tests::read_only_status_is_description_bound_and_namespace_scoped ... ok
test hosted::tests::restart_requires_sre_group_and_exact_resource_authority_without_local_approval ... ok
test hosted::tests::pod_log_invoke_refuses_a_non_admitted_namespace_and_a_stale_lease ... ok
test hosted::tests::pod_log_invoke_passes_the_input_through_and_defaults_tail_lines ... ok
test local::inventory_tests::inventory_discovery_publishes_both_reads_for_every_activated_connection ... ok
test hosted::tests::an_oversized_log_body_is_front_trimmed_inside_a_validating_envelope ... ok
test local::inventory_tests::inventory_refuses_oversized_provider_and_projected_values ... ok
test hosted::paging_tests::a_busy_namespace_lists_in_full_despite_the_upstream_response_bound ... ok

failures:

---- local::inventory_tests::adversary_inventory_rejects_null_cursor_as_its_published_schema_requires stdout ----

thread 'local::inventory_tests::adversary_inventory_rejects_null_cursor_as_its_published_schema_requires' (540170) panicked at ~/.local/state/worktree/trees/b10x/connectors/wt-728fb204f8cb/crates/integration-kubernetes/src/local_inventory_tests.rs:592:9:
the published string-only cursor contract must refuse explicit null before cluster I/O; got Ok(Object {"connection_ref": String("connection:alpha"), "deployments": Array [], "namespace": String("apps")}) and 1 request(s)
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace


failures:
    local::inventory_tests::adversary_inventory_rejects_null_cursor_as_its_published_schema_requires

test result: FAILED. 55 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.07s

error: test failed, to rerun pass `-p integration-kubernetes --lib`
exit: 101
```

The unit-test lane executed 56 cases: 55 passed and the new cursor-schema case failed. The package command stopped at the failed unit-test lane, so this pass does not claim documentation-test execution. `git diff --check` also returned exit 0. All build outputs remained in the assigned worktree's default runtime target directory.

## 4. Findings

Covered production commit: 99d4d74e096c6fffb9e95c464f3ef04e47871fd8.

| File:line | Category | Severity | Verdict | Origin | Finding |
|---|---|---|---|---|---|
| crates/integration-kubernetes/src/local_inventory.rs:34 | contract-drift | warning | CONFIRMED | introduced | The new workload input accepts explicit cursor:null as a fresh read even though its published cursor schema permits only a nonempty string. |

What was measured: the new case failed at `crates/integration-kubernetes/src/local_inventory_tests.rs:592`, exit 101, after observing `Ok` and one fixture API request. `Option<String>` at production line 34 deserializes null into None; the length check at lines 117–120 then has no value to reject. The schema at line 213 advertises only a nonempty string. This is a contract-consistency warning; the fixture did not bypass namespace, owner or connection admission.

What reaches it: an operator can pass `--input-json '{"namespace":"apps","cursor":null}'` to `connectors operation invoke --operation kubernetes.workload.list` with an activated connection and current description lease. `crates/connectors-cli/src/lib.rs:1186` reads the JSON unchanged; the local daemon at `crates/server/src/local.rs:157` uses RequestEnvelope::validate, whose invocation branch at `crates/protocol/src/operation.rs:318` checks references and byte bounds rather than the operation's JSON Schema. The registry's invoke branch at `crates/connectors-runtime/src/registry.rs:230` checks ownership and the lease and delegates the unchanged input. The new backend handler at `crates/integration-kubernetes/src/local.rs:954` then reaches inventory_output. Explicit local config/state flags select this path on the reviewed base CLI. No live call was made.

Origin: the complete base-to-head diff adds `local_inventory.rs`, including both the nullable deserializer and string-only schema, and adds the workload.list operation to local dispatch. `git show 4769ce32a331bb8daddaf066b69459c4148fb749:crates/integration-kubernetes/src/local_inventory.rs` reports that the path did not exist at the base. No tree was switched or mutated to inspect origin. The warning belongs to this newly introduced operation contract.

Suggested correction for the coordinator to route: distinguish absent cursor from explicit null and refuse null in accordance with the published contract. The failing case is retained. No correction was applied during this pass.

## 5. Attacks that stayed green

- Empty upstream page with continuation is followed and reserved cursor characters stay one encoded query value.
- Multiple regular container images survive projection with omitted replica/status fields; unrelated environment values are absent from the exact result.
- The existing 54 cases stayed green, covering multi-connection selection, activation and owner/lease admission, configured namespaces, RBAC refusal, bounded pages and results, cursor separation and the compact datasource projection.

## 6. Paths written outside the worktree

The seven persistent report/log/exit files written by this pass are:

- `~/.cache/connectors-cli-wave-20260906/personal-local-workload-read/adversary-1/null-cursor.log`
- `~/.cache/connectors-cli-wave-20260906/personal-local-workload-read/adversary-1/null-cursor.exit`
- `~/.cache/connectors-cli-wave-20260906/personal-local-workload-read/adversary-1/empty-pages.log`
- `~/.cache/connectors-cli-wave-20260906/personal-local-workload-read/adversary-1/empty-pages.exit`
- `~/.cache/connectors-cli-wave-20260906/personal-local-workload-read/adversary-1/package-suite.log`
- `~/.cache/connectors-cli-wave-20260906/personal-local-workload-read/adversary-1/package-suite.exit`
- `~/.cache/connectors-cli-wave-20260906/personal-local-workload-read/adversary-1/report.md`

The assigned temporary directory `~/.cache/cw6/k` was supplied to compiler/test tools. The coordinator-configured sccache service managed its existing shared compiler cache; no alternative build target or scratch location was selected. The provided brief was read only.

```findings
- file: crates/integration-kubernetes/src/local_inventory.rs
  line: 34
  category: contract-drift
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: The new workload input accepts explicit cursor:null as a fresh read even though its published cursor schema permits only a nonempty string.
```


Report publication: local home prefixes are mechanically replaced with `~`; raw runner output remains in private wave scratch. Counts, assertions and findings are unchanged.
