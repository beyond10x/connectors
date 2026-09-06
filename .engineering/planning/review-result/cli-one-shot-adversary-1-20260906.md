---
format: aep.planning-md/1
id: review-result:cli-one-shot-adversary-1-20260906
kind: review-result
status: active
title: 'One-shot review: runtime cases pass, ESS citation fence fails'
relations:
- reviews: story:one-shot-operations-without-a-daemon
revision: 1
---
unit: story:one-shot-operations-without-a-daemon at 970e4af56f7a4ca1b9689c885fabd3a519bbe0ca plus the final tests-only working diff
verdict: CONFIRMED
cases: executed 618→626, red 1 (the red belongs to the additional complete root workspace; all 626 affected cases pass)
origin: introduced 0 / pre-existing 0 / undecided 2 (one gate finding and one invalid-probe disposition)
wrote-outside-worktree: 4 namespaces; exact retained paths and transient-write limitations are inventoried in part 6
needs-coordinator: refresh the stale ESS source citations, route origin using the source diff, and record this report before a corrected follow-up pass
```text
$ git --no-pager diff --stat
 crates/connectors-cli/tests/one_shot_operations.rs | 194 ++++++++++++++++
 .../connectors-runtime/tests/one_shot_runtime.rs   | 245 +++++++++++++++++++++
 2 files changed, 439 insertions(+)
```

Only two test files changed. No production, ESS, AEP or Git state was edited by this adversary. The coordinator corrected one newly authored invalid probe with the exact returned patch; no pre-pass retained case was changed, deleted, ignored, skipped or weakened. The complete final test-only patch is retained as final-tests-only.patch.

This report is the first full attack of the complete unit. It is not an approval or an independent-verification claim. The unit source is 970e4af56f7a4ca1b9689c885fabd3a519bbe0ca, whole-unit base 3df1cd2df32472a4afc8feeaac8c46c9de8d1b55. No base worktree was assigned, and no case was executed against the base. Origin is therefore undecided, including where the source diff explains a likely cause.

Public redaction declaration: the public variant mechanically replaces the absolute local home prefix with `~`; all other bytes are identical. This changes local filesystem locations only. Commands, test output, statuses, counts, findings and the original invalid probe are retained.

1. Preparation and write boundary

Read the full adversary charter, repository AGENTS.md, unit brief, acceptance artifact, implementor report, complete base-to-HEAD diff across all 19 changed files, and the modified owner/caller paths. The worktree skill was applied by reusing the assigned managed tree. The Connectors skill was consulted; this pass uses synthetic provider fixtures and performs no engineering integration invocation. The ESS skill was consulted when the existing ESS citation fence failed; no specification or generated output was changed.

Callers examined: CLI serve/operation/connection/event dispatch; PersonalRuntime bind/compose/one_shot; LocalStateOwnership acquire/require_absent_socket; LocalOperationDaemon bind_owned/serve_until; LocalOneShot operation/connection and dispatch_frame; registry operation ownership, description merge/lease translation, claims, dispatch and shutdown; Catalog, Platform, Slack, Monitoring and Kubernetes lifetime owners and constructors; existing local client framing and response correlation; Platform alias resolution/schema/transport; Slack recovery, declared instances and supervisor creation; Kubernetes generation-local activation, observations and workload cursors. All touched unit files were read in the complete diff before attack theories were turned into cases.

Eight new test cases were written and formatted before the first execution. The immutable cases-before-execution.patch captures that initial state. Its timestamp is 2026-09-06 03:23:57.439936457 +0200; the first deciding log completed at 03:24:06.299735194 +0200. No passing suite was run beforehand. The supplied implementor before count is 618 passing / 2 existing ignored: CLI 116, runtime 328, console 87, server 87. It is not a base execution by this adversary.

All Cargo commands used the assigned managed tree as cwd and this exact environment:

```text
cwd=~/.local/state/worktree/trees/b10x/connectors/wt-fc49f37347f5
CARGO_TARGET_DIR unset
TMPDIR=~/.cache/cw6/o
RUSTC_WRAPPER=/usr/bin/sccache
CARGO_INCREMENTAL=0
CARGO_PROFILE_DEV_DEBUG=0
CARGO_PROFILE_TEST_DEBUG=0
CARGO_BUILD_JOBS=3
```

Only owned in-tree Cargo targets were used. No output was deleted. Free disk was 29,902,090,240 bytes before execution and 28,140,941,312 bytes at final verification, above the 20 GB reporting floor.

2. Added cases and their first executions

| File:line | Case and asserted property | Final state |
|---|---|---|
| crates/connectors-cli/tests/one_shot_operations.rs:536 | Actual invoke transmitted once to the fixture control socket; EOF, malformed JSON and wrong request correlation after socket deletion never trigger provider egress through a new local runtime | green |
| crates/connectors-cli/tests/one_shot_operations.rs:601 | Missing, malformed, oversize and conflicting JSON sources fail before local runtime state creation | green |
| crates/connectors-cli/tests/one_shot_operations.rs:641 | Explicit hosted failures and hosted/local option conflicts never construct local state; target conflict precedes opening the missing input file | green |
| crates/connectors-cli/tests/one_shot_operations.rs:672 | Corrected case: permitted extra caller keys cannot rebind the configured module request or Authorization header; a changed Grant rejects the old description before egress | green; original invalid expectation retained below |
| crates/connectors-runtime/tests/one_shot_runtime.rs:280 | A socket appearing between the caller's absence probe and composition is preserved and refused before reply-claim journal/configuration work | green |
| crates/connectors-runtime/tests/one_shot_runtime.rs:344 | A real asynchronous backend shutdown barrier keeps ownership locked and the one-shot future incomplete, on both success and refusal, until teardown completes | green |
| crates/connectors-runtime/tests/one_shot_runtime.rs:422 | Two backends claiming bounded lifetime cannot admit an unknown or ambiguous invocation owner; all backends are shut down without dispatch | green |
| crates/connectors-runtime/tests/one_shot_runtime.rs:469 | All four session control variants and all four persistent Connection mutation/acquisition variants refuse before even reading missing configuration or creating state | green |

The first case alone, before any suite:

```text
$ cargo test --manifest-path crates/connectors-cli/Cargo.toml --test one_shot_operations --locked adversary_uncertain_invoke_never_resends_after_the_control_socket_disappears -- --exact --nocapture
warning: connector-secrets@0.6.5: no live Vault was offered (CONNECTOR_SECRETS_VAULT_ADDR and CONNECTOR_SECRETS_VAULT_TOKEN are unset), so the reqwest HttpTransport is UNEXERCISED by this build: tests/vault_live.rs is compiled #[ignore]d
   Compiling connectors-cli v0.6.5 (~/.local/state/worktree/trees/b10x/connectors/wt-fc49f37347f5/crates/connectors-cli)
    Finished `test` profile [unoptimized] target(s) in 1.68s
     Running tests/one_shot_operations.rs (crates/connectors-cli/target/debug/deps/one_shot_operations-f584650961a855c8)

running 1 test
test adversary_uncertain_invoke_never_resends_after_the_control_socket_disappears ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 15 filtered out; finished in 7.06s


exit 0
```

```text
$ cargo test --manifest-path crates/connectors-cli/Cargo.toml --test one_shot_operations --locked adversary_json_source_and_size_refusals_precede_one_shot_state_creation -- --exact --nocapture
    Blocking waiting for file lock on package cache
    Blocking waiting for file lock on package cache
    Blocking waiting for file lock on package cache
warning: connector-secrets@0.6.5: no live Vault was offered (CONNECTOR_SECRETS_VAULT_ADDR and CONNECTOR_SECRETS_VAULT_TOKEN are unset), so the reqwest HttpTransport is UNEXERCISED by this build: tests/vault_live.rs is compiled #[ignore]d
    Finished `test` profile [unoptimized] target(s) in 0.41s
     Running tests/one_shot_operations.rs (crates/connectors-cli/target/debug/deps/one_shot_operations-f584650961a855c8)

running 1 test
test adversary_json_source_and_size_refusals_precede_one_shot_state_creation ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 15 filtered out; finished in 0.03s


exit 0
```

```text
$ cargo test --manifest-path crates/connectors-cli/Cargo.toml --test one_shot_operations --locked adversary_hosted_refusal_and_target_conflict_never_construct_the_local_runtime -- --exact --nocapture
warning: connector-secrets@0.6.5: no live Vault was offered (CONNECTOR_SECRETS_VAULT_ADDR and CONNECTOR_SECRETS_VAULT_TOKEN are unset), so the reqwest HttpTransport is UNEXERCISED by this build: tests/vault_live.rs is compiled #[ignore]d
    Finished `test` profile [unoptimized] target(s) in 0.25s
     Running tests/one_shot_operations.rs (crates/connectors-cli/target/debug/deps/one_shot_operations-f584650961a855c8)

running 1 test
test adversary_hosted_refusal_and_target_conflict_never_construct_the_local_runtime ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 15 filtered out; finished in 0.02s


exit 0
```

```text
$ cargo test --manifest-path crates/connectors-cli/Cargo.toml --test one_shot_operations --locked adversary_caller_route_injection_and_revoked_grant_stop_before_fixture_egress -- --exact --nocapture
warning: connector-secrets@0.6.5: no live Vault was offered (CONNECTOR_SECRETS_VAULT_ADDR and CONNECTOR_SECRETS_VAULT_TOKEN are unset), so the reqwest HttpTransport is UNEXERCISED by this build: tests/vault_live.rs is compiled #[ignore]d
    Finished `test` profile [unoptimized] target(s) in 0.25s
     Running tests/one_shot_operations.rs (crates/connectors-cli/target/debug/deps/one_shot_operations-f584650961a855c8)

running 1 test

thread 'adversary_caller_route_injection_and_revoked_grant_stop_before_fixture_egress' (2362274) panicked at tests/one_shot_operations.rs:695:9:
assertion failed: matches!(egress.accept(), Err(error) if error.kind() ==
    std::io::ErrorKind::WouldBlock)
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace
test adversary_caller_route_injection_and_revoked_grant_stop_before_fixture_egress ... FAILED

failures:

failures:
    adversary_caller_route_injection_and_revoked_grant_stop_before_fixture_egress

test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 15 filtered out; finished in 33.01s

error: test failed, to rerun pass `--test one_shot_operations`

exit 101
```

```text
$ cargo test --manifest-path crates/connectors-runtime/Cargo.toml --test one_shot_runtime --locked adversary_socket_publication_after_absence_probe_is_preserved_and_refused -- --exact --nocapture
    Blocking waiting for file lock on package cache
    Blocking waiting for file lock on package cache
    Blocking waiting for file lock on package cache
warning: connector-secrets@0.6.5: no live Vault was offered (CONNECTOR_SECRETS_VAULT_ADDR and CONNECTOR_SECRETS_VAULT_TOKEN are unset), so the reqwest HttpTransport is UNEXERCISED by this build: tests/vault_live.rs is compiled #[ignore]d
   Compiling connectors-runtime v0.6.5 (~/.local/state/worktree/trees/b10x/connectors/wt-fc49f37347f5/crates/connectors-runtime)
    Finished `test` profile [unoptimized] target(s) in 2.05s
     Running tests/one_shot_runtime.rs (crates/connectors-runtime/target/debug/deps/one_shot_runtime-3e4e9c14b5867ec0)

running 1 test
test adversary_socket_publication_after_absence_probe_is_preserved_and_refused ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 7 filtered out; finished in 0.00s


exit 0
```

```text
$ cargo test --manifest-path crates/connectors-runtime/Cargo.toml --test one_shot_runtime --locked adversary_state_lock_outlives_async_shutdown_on_success_and_refusal -- --exact --nocapture
warning: connector-secrets@0.6.5: no live Vault was offered (CONNECTOR_SECRETS_VAULT_ADDR and CONNECTOR_SECRETS_VAULT_TOKEN are unset), so the reqwest HttpTransport is UNEXERCISED by this build: tests/vault_live.rs is compiled #[ignore]d
    Finished `test` profile [unoptimized] target(s) in 0.22s
     Running tests/one_shot_runtime.rs (crates/connectors-runtime/target/debug/deps/one_shot_runtime-3e4e9c14b5867ec0)

running 1 test
test adversary_state_lock_outlives_async_shutdown_on_success_and_refusal ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 7 filtered out; finished in 0.00s


exit 0
```

```text
$ cargo test --manifest-path crates/connectors-runtime/Cargo.toml --test one_shot_runtime --locked adversary_lifetime_capability_never_admits_unknown_or_ambiguous_owners -- --exact --nocapture
warning: connector-secrets@0.6.5: no live Vault was offered (CONNECTOR_SECRETS_VAULT_ADDR and CONNECTOR_SECRETS_VAULT_TOKEN are unset), so the reqwest HttpTransport is UNEXERCISED by this build: tests/vault_live.rs is compiled #[ignore]d
    Finished `test` profile [unoptimized] target(s) in 0.22s
     Running tests/one_shot_runtime.rs (crates/connectors-runtime/target/debug/deps/one_shot_runtime-3e4e9c14b5867ec0)

running 1 test
test adversary_lifetime_capability_never_admits_unknown_or_ambiguous_owners ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 7 filtered out; finished in 0.00s


exit 0
```

```text
$ cargo test --manifest-path crates/connectors-runtime/Cargo.toml --test one_shot_runtime --locked adversary_every_persistent_request_class_refuses_before_configuration_or_state -- --exact --nocapture
warning: connector-secrets@0.6.5: no live Vault was offered (CONNECTOR_SECRETS_VAULT_ADDR and CONNECTOR_SECRETS_VAULT_TOKEN are unset), so the reqwest HttpTransport is UNEXERCISED by this build: tests/vault_live.rs is compiled #[ignore]d
    Finished `test` profile [unoptimized] target(s) in 0.22s
     Running tests/one_shot_runtime.rs (crates/connectors-runtime/target/debug/deps/one_shot_runtime-3e4e9c14b5867ec0)

running 1 test
test adversary_every_persistent_request_class_refuses_before_configuration_or_state ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 7 filtered out; finished in 0.00s


exit 0
```

The original route probe failed for an invalid expectation: `work.requests.list` advertises an input schema permitting additional properties (catalog/b10x.catalog.json:7233). It was wrong to assert that the mere presence of caller keys named `base_url` or `Authorization` must prevent any request. The failure measured a request reaching the configured fixture socket after its normal timeout; it did not measure an attacker-selected destination or header. This is an INFEASIBLE attack claim, not a production defect. The coordinator explicitly applied invalid-probe-correction.patch and formatted only this new test file. The corrected test serves the legitimate request, checks the actual fixed path and absent injected authorization, and separately preserves the Grant-revocation no-egress assertion.

Original invalid test, preserved exactly from the pre-execution patch:

```rust
#[test]
fn adversary_caller_route_injection_and_revoked_grant_stop_before_fixture_egress() {
    let fixture = Fixture::new();
    let egress = fixture.platform();
    egress.set_nonblocking(true).unwrap();
    let description =
        success(&fixture.run(&["operation", "describe", "--operation", "work.requests.list"]));
    for input in [
        r#"{"cursor":"","limit":1,"base_url":"http://attacker.invalid"}"#,
        r#"{"cursor":"","limit":1,"Authorization":"Bearer attacker"}"#,
    ] {
        let refused = fixture.run(&[
            "operation",
            "invoke",
            "--operation",
            "work.requests.list",
            "--connection",
            "connection-fixture",
            "--description-ref",
            description["description_ref"].as_str().unwrap(),
            "--input-json",
            input,
        ]);
        assert!(!refused.status.success(), "{refused:?}");
        assert!(matches!(egress.accept(), Err(error)
            if error.kind() == std::io::ErrorKind::WouldBlock));
    }
    let changed = fs::read_to_string(&fixture.config)
        .unwrap()
        .replace("grant-fixture", "replacement-grant");
    fs::write(&fixture.config, changed).unwrap();
    let refused = fixture.run(&[
        "operation",
        "invoke",
        "--operation",
        "work.requests.list",
        "--connection",
        "connection-fixture",
        "--description-ref",
        description["description_ref"].as_str().unwrap(),
        "--input-json",
        r#"{"cursor":"","limit":1}"#,
    ]);
    assert!(!refused.status.success(), "{refused:?}");
    assert_eq!(value(&refused)["error"]["code"], "stale_authority");
    assert!(matches!(egress.accept(), Err(error)
        if error.kind() == std::io::ErrorKind::WouldBlock));
}
```

Exact correction returned to and applied by the coordinator:

```diff
--- a/crates/connectors-cli/tests/one_shot_operations.rs
+++ b/crates/connectors-cli/tests/one_shot_operations.rs
@@ -670,7 +670,7 @@
 }
 
 #[test]
-fn adversary_caller_route_injection_and_revoked_grant_stop_before_fixture_egress() {
+fn adversary_caller_input_cannot_rebind_routes_or_revoked_grants() {
     let fixture = Fixture::new();
     let egress = fixture.platform();
     egress.set_nonblocking(true).unwrap();
@@ -680,7 +680,8 @@
         r#"{"cursor":"","limit":1,"base_url":"http://attacker.invalid"}"#,
         r#"{"cursor":"","limit":1,"Authorization":"Bearer attacker"}"#,
     ] {
-        let refused = fixture.run(&[
+        let serving = serve_http(egress.try_clone().unwrap());
+        let invoked = fixture.run(&[
             "operation",
             "invoke",
             "--operation",
@@ -691,9 +692,11 @@
             "--input-json",
             input,
         ]);
-        assert!(!refused.status.success(), "{refused:?}");
-        assert!(matches!(egress.accept(), Err(error)
-            if error.kind() == std::io::ErrorKind::WouldBlock));
+        success(&invoked);
+        let actual = serving.join().unwrap();
+        assert!(actual.starts_with("GET /api/work/v2/requests?"), "{actual}");
+        assert!(!actual.contains("attacker"), "caller input changed the request: {actual}");
+        assert!(!actual.to_ascii_lowercase().contains("authorization:"), "{actual}");
     }
     let changed = fs::read_to_string(&fixture.config)
         .unwrap()
```

The runtime, console and root suites had started after all eight initial cases existed and had been executed individually. The coordinator's corrected probe was then executed alone, before the full CLI lane. This timing is recorded explicitly; the corrected case is not claimed to precede those independent suites.

```text
$ cargo test --manifest-path crates/connectors-cli/Cargo.toml --test one_shot_operations --locked adversary_caller_input_cannot_rebind_routes_or_revoked_grants -- --exact --nocapture
warning: connector-secrets@0.6.5: no live Vault was offered (CONNECTOR_SECRETS_VAULT_ADDR and CONNECTOR_SECRETS_VAULT_TOKEN are unset), so the reqwest HttpTransport is UNEXERCISED by this build: tests/vault_live.rs is compiled #[ignore]d
   Compiling connectors-cli v0.6.5 (~/.local/state/worktree/trees/b10x/connectors/wt-fc49f37347f5/crates/connectors-cli)
    Finished `test` profile [unoptimized] target(s) in 1.30s
     Running tests/one_shot_operations.rs (crates/connectors-cli/target/debug/deps/one_shot_operations-f584650961a855c8)

running 1 test
test adversary_caller_input_cannot_rebind_routes_or_revoked_grants ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 15 filtered out; finished in 7.54s


exit 0
```

3. Complete test and strict-check executions

| Lane | Supplied comparable before | Executed after | Passed | Failed | Ignored | Exit |
|---|---:|---:|---:|---:|---:|---:|
| CLI workspace | 116 | 120 | 120 | 0 | 0 | 0 |
| Runtime workspace | 328 | 332 | 332 | 0 | 2 | 0 |
| Console workspace | 87 | 87 | 87 | 0 | 0 | 0 |
| Server package, selected within root workspace below | 87 | 87 | 87 | 0 | 0 | 0 for this test binary |
| Comparable affected cases | 618 | 626 | 626 | 0 | 2 | all affected binaries green |
| Complete root workspace (includes the server's 87 cases) | not supplied | 1141 | 1140 | 1 | 4 | 101 |

Server belongs to the root workspace. `--workspace` therefore selected the complete root workspace, including the existing catalog/ESS fence. Across the four full workspace invocations, 1680 cases actually executed, 1679 passed, 1 failed and 6 were ignored. This broader number has no supplied before comparator; it must not be described as 1062 new tests. The comparable affected count increased by exactly the eight new cases. All two runtime ignores and four root ignores predate this pass; none were changed or bypassed. No unchanged test lane was borrowed from the implementor.

The full root lane's red is the successful finding of this pass. It was found by an existing selected case after the new adversarial cases existed; it is not misrepresented as a newly written test that failed before the suite. The isolated reproduction below was run after that suite failure, against the same source.

```text
$ cargo test --manifest-path crates/connectors-runtime/Cargo.toml --workspace --locked --no-fail-fast
    Blocking waiting for file lock on package cache
    Blocking waiting for file lock on package cache
    Blocking waiting for file lock on package cache
warning: connector-secrets@0.6.5: no live Vault was offered (CONNECTOR_SECRETS_VAULT_ADDR and CONNECTOR_SECRETS_VAULT_TOKEN are unset), so the reqwest HttpTransport is UNEXERCISED by this build: tests/vault_live.rs is compiled #[ignore]d
    Finished `test` profile [unoptimized] target(s) in 0.31s
     Running unittests src/lib.rs (crates/connectors-runtime/target/debug/deps/connect_session_transport-7d8b04b7913a096f)

running 4 tests
test tests::browser_capability_comparison_rejects_prefixes_and_differences ... ok
test tests::unsafe_directory_refuses ... ok
test tests::endpoint_is_owner_only_one_use_and_removed_after_submission ... ok
test tests::browser_page_submits_directly_to_the_one_use_endpoint ... ok

test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running unittests src/lib.rs (crates/connectors-runtime/target/debug/deps/connectors_config-212202abf35bc8cd)

running 20 tests
test personal::tests::a_trunk_address_may_be_a_literal_or_a_name_and_nothing_else ... ok
test hosted::tests::claude_code_custody_is_explicit_and_requires_the_hosted_secret_store ... ok
test hosted::tests::hosted_configuration_uses_same_handle_and_refuses_mutable_or_symlinked_files ... ok
test hosted::tests::kubernetes_namespace_groups_are_exact_sorted_and_restart_is_a_read_subset ... ok
test hosted::tests::an_existing_hosted_config_with_the_old_b10x_section_parses_unchanged ... ok
test personal::tests::an_existing_personal_config_with_the_old_b10x_section_parses_unchanged ... ok
test hosted::tests::hosted_integrations_are_explicit_and_fail_closed ... ok
test hosted::tests::hosted_vault_requires_a_valid_distinct_sip_digest_pair ... ok
test personal::tests::slack_only_configuration_contains_policy_but_no_secret_source ... ok
test hosted::tests::hosted_vault_is_all_or_nothing ... ok
test personal::tests::grafana_configuration_names_origin_and_independent_target_grants_only ... ok
test hosted::tests::hosted_grafana_requires_vault_exact_groups_and_digest_bound_targets ... ok
test hosted::tests::hosted_jira_service_api_token_excludes_a_service_oauth_client_id ... ok
test hosted::tests::hosted_gitlab_requires_vault_and_a_same_origin_callback ... ok
test personal::tests::kubernetes_configuration_is_policy_only ... ok
test hosted::tests::hosted_jira_separates_organization_and_user_authority ... ok
test hosted::tests::hosted_deployment_accepts_planner_integration ... ok
test personal::tests::documented_development_configuration_stays_strict_and_valid ... ok
test personal::tests::the_named_default_trunk_example_parses_and_dials_by_number ... ok
test personal::tests::deployment_configuration_cannot_be_symlinked_or_group_writable ... ok

test result: ok. 20 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests/catalog_usernames.rs (crates/connectors-runtime/target/debug/deps/catalog_usernames-dd761370b694050c)

running 3 tests
test an_entry_with_no_user_half_still_reads_and_reports_none ... ok
test a_basic_credential_carries_its_user_half_beside_its_endpoints ... ok
test a_user_half_is_refused_when_it_is_empty_or_could_not_travel ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running unittests src/lib.rs (crates/connectors-runtime/target/debug/deps/connectors_runtime-a5a96cefa8910dec)

running 32 tests
test composition::tests::an_empty_variable_is_no_store_at_all ... ok
test claims::tests::a_journal_this_build_cannot_parse_refuses_to_open ... ok
test claims::tests::only_an_event_reference_is_claimable ... ok
test composition::tests::git_fetch_environment_override_is_atomic_and_secret_free ... ok
test claims::tests::a_claim_survives_a_daemon_restart ... ok
test composition::tests::git_fetch_environment_override_refuses_an_invalid_listener ... ok
test composition::tests::naming_both_stores_is_refused_rather_than_one_of_them_quietly_winning ... ok
test composition::tests::naming_no_store_is_refused_rather_than_a_database_file_appearing_somewhere ... ok
test composition::tests::an_unopenable_sqlite_path_is_named_in_the_refusal ... ok
test composition::tests::the_refusal_names_both_stores_a_deployment_may_choose ... ok
test composition::tests::working_tree_state_roots_are_refused ... ok
test registry::tests::a_topical_datasource_query_that_matches_nothing_returns_the_admitted_set ... ok
test registry::tests::ambiguous_exclusive_claims_fail_with_typed_protocol_errors_before_dispatch ... ok
test registry::tests::readiness_requires_every_configured_backend ... ok
test registry::tests::direct_dispatch_selects_the_unique_claim_without_not_found_probing ... ok
test registry::tests::the_registry_lease_ignores_request_scoped_provenance ... ok
test registry::tests::describe_merges_connections_and_invoke_receives_the_selected_local_lease ... ok
test service_bundle::tests::malformed_manifests_and_deployments_are_refused ... ok
test service_bundle::tests::catalog_dispatch_and_backend_ownership_mismatches_are_refused ... ok
test service_bundle::tests::identity_and_operation_collisions_are_refused ... ok
test service_bundle::tests::bundle_order_and_policy_projection_are_deterministic ... ok
test registry::tests::duplicate_connection_references_fail_search ... ok
test registry::tests::an_undemanded_reference_is_not_spent_at_the_local_seam ... ok
test registry::tests::duplicate_channel_references_fail_search ... ok
test registry::tests::search_aggregates_compatible_operations_and_deduplicates_the_operation ... ok
test service_bundle::tests::registration_is_inert_until_an_explicit_overlay_is_present ... ok
test registry::tests::a_companion_reply_is_claimed_exactly_once_locally ... ok
test claims::tests::parallel_presentations_take_exactly_one_claim ... ok
test tls_listener::tests::established_connection_permit_is_lifetime_bound_and_reads_time_out ... ok
test composition::tests::a_hosted_placement_keeps_its_state_in_a_file_when_no_database_is_offered ... ok
test composition::tests::empty_personal_runtime_binds_and_cleans_without_a_credential_store ... ok
test tls_listener::tests::listener_serves_the_internal_application_over_tls ... ok

test result: ok. 32 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.04s

     Running tests/local_catalog_writes.rs (crates/connectors-runtime/target/debug/deps/local_catalog_writes-1195535faf535091)

running 8 tests
test describing_a_write_directly_skips_the_first_read_only_connection ... ok
test only_read_only_connections_hide_the_write_and_describe_its_missing_grant ... ok
test adversary_a_read_description_cannot_authorize_a_different_write_operation ... ok
test adversary_an_approval_reference_cannot_raise_the_selected_read_only_grant ... ok
test adversary_http_200_application_refusal_survives_the_documented_socket_path ... ok
test stale_description_and_provider_refusal_have_distinct_actionable_results ... ok
test writable_first_still_discovers_describes_and_posts_through_the_writer ... ok
test read_only_first_still_discovers_describes_and_posts_through_the_writer ... ok

test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.23s

     Running tests/one_shot_runtime.rs (crates/connectors-runtime/target/debug/deps/one_shot_runtime-3e4e9c14b5867ec0)

running 8 tests
test unknown_backend_lifetime_is_refused_and_shutdown_releases_ownership ... ok
test an_unsafe_socket_refuses_before_opening_the_reply_claim_journal ... ok
test adversary_socket_publication_after_absence_probe_is_preserved_and_refused ... ok
test adversary_every_persistent_request_class_refuses_before_configuration_or_state ... ok
test an_existing_owner_refuses_before_opening_the_reply_claim_journal ... ok
test adversary_lifetime_capability_never_admits_unknown_or_ambiguous_owners ... ok
test adversary_state_lock_outlives_async_shutdown_on_success_and_refusal ... ok
test ephemeral_catalog_uses_the_described_credential_and_rejects_read_only_writes_before_egress ... ok

test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.29s

     Running unittests src/lib.rs (crates/connectors-runtime/target/debug/deps/hosted_secrets-a71a17433d1f3e50)

running 1 test
test tests::wire_reference_round_trips ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running unittests src/bin/migrate.rs (crates/connectors-runtime/target/debug/deps/connectors_secrets_migrate-6a73e3e76a0987d8)

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running unittests src/lib.rs (crates/connectors-runtime/target/debug/deps/hosted_state-4cea7146872613d6)

running 4 tests
test port_tests::the_postgres_backend_conforms ... ignored, requires a PostgreSQL named by CONNECTORS_DATABASE_URL
test port_tests::the_postgres_backend_serves_grant_evaluation ... ignored, requires a PostgreSQL named by CONNECTORS_DATABASE_URL
test tests::live_postgres_round_trip_is_bounded_and_atomic ... ok
test tests::state_keys_are_closed_and_bounded ... ok

test result: ok. 2 passed; 0 failed; 2 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running unittests src/lib.rs (crates/connectors-runtime/target/debug/deps/hosted_vault-f000e9b3cf1de480)

running 6 tests
test adapter::tests::only_healthy_vault_status_is_ready ... ok
test adapter::tests::vault_origin_and_role_are_closed ... ok
test prepared::tests::clean_initialize_does_not_create_an_empty_journal ... ok
test adapter::tests::readiness_requires_health_and_an_accepted_session_without_reading_a_credential ... ok
test prepared::tests::a_journal_in_the_shared_store_recovers_a_committed_transaction_after_a_restart ... ok
test prepared::tests::candidate_values_stay_in_the_secret_store_and_are_invisible_until_commit ... ok

test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s

     Running unittests src/lib.rs (crates/connectors-runtime/target/debug/deps/identity_http-af89a6def36d77ae)

running 3 tests
test adapter::tests::approval_issuance_scope_is_admitted ... ok
test adapter::tests::a_routable_plaintext_identity_origin_is_refused_in_every_build ... ok
test adapter::tests::hosted_verifier_requires_https_origin_and_closed_access_token_shape ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running unittests src/lib.rs (crates/connectors-runtime/target/debug/deps/integration_catalog-407c8077d12decc1)

running 35 tests
test argocd::tests::a_rejected_sign_in_says_so_rather_than_reporting_a_missing_project ... ok
test argocd::tests::a_project_or_role_name_that_could_change_the_path_is_refused ... ok
test argocd::tests::read_only_acquisition_carries_no_sync_policy ... ok
test argocd::tests::a_zero_lifetime_is_refused_rather_than_meaning_forever ... ok
test argocd::tests::every_generated_policy_is_one_argo_cd_will_accept ... ok
test argocd::tests::a_missing_project_is_not_reported_as_a_permission_problem ... ok
test argocd::tests::an_existing_role_is_reused_and_its_policies_are_left_alone ... ok
test argocd::tests::a_login_without_projects_update_is_told_which_grant_it_lacks ... ok
test argocd::tests::an_unreachable_origin_names_the_aperture_as_a_possibility ... ok
test argocd::tests::the_four_calls_happen_in_order_and_the_token_comes_back ... ok
test config::tests::a_value_without_operator_approval_says_so_rather_than_claiming_it ... ok
test config::tests::an_unsupplied_variable_is_absent_rather_than_empty ... ok
test config::tests::an_endpoint_value_reaches_the_resolver_under_its_declared_name ... ok
test argocd::tests::the_project_is_written_back_whole ... ok
test argocd::tests::the_password_appears_once_and_the_session_token_never_persists ... ok
test tests::an_entry_stating_no_user_half_answers_none_rather_than_an_empty_string ... ok
test tests::an_unnamed_entry_keeps_the_address_it_had_before_instances_existed ... ok
test tests::the_instance_derivation_puts_the_provider_in_the_namespace ... ok
test tests::two_named_instances_of_one_provider_get_different_addresses ... ok
test tests::a_basic_credentials_user_half_reaches_the_resolver_from_the_entry ... ok
test tests::a_network_effect_is_not_an_escalation_but_a_filesystem_one_would_be ... ok
test tests::a_provider_with_a_fixed_base_url_needs_no_configuration ... ok
test tests::a_request_template_exists_for_every_operation_this_backend_would_offer ... ok
test tests::the_default_ceiling_admits_reads_and_refuses_writes ... ok
test tests::effect_class_comes_from_the_declaration_not_the_method ... ok
test tests::an_unsupplied_url_variable_refuses_rather_than_reaching_a_literal_placeholder ... ok
test tests::the_ceiling_reads_declared_facts_rather_than_an_operation_list ... ok
test tests::every_catalogued_provider_can_address_a_credential ... ok
test tests::every_declared_user_half_field_names_a_credential_that_actually_wants_one ... ok
test tests::the_aperture_is_derived_from_the_same_declaration_the_request_is ... ok
test tests::a_read_only_first_connection_does_not_hide_an_admitted_write ... ok
test tests::the_limit_drops_operations_rather_than_the_identities_that_serve_one ... ok
test tests::search_and_describe_require_approval_for_every_admitted_write ... ok
test hosted::tests::completion_does_not_overwrite_a_terminal_session_after_verification_awaits ... ok
test hosted::tests::two_people_receive_isolated_connections_and_credential_addresses ... ok

test result: ok. 35 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.10s

     Running unittests src/lib.rs (crates/connectors-runtime/target/debug/deps/integration_gitlab-d5d37e238ff28b61)

running 40 tests
test backend::git_fetch::tests::source_authority_digest_comparison_checks_every_byte ... ok
test backend::git_fetch::tests::upload_pack_request_is_bound_to_exact_commit_and_depth ... ok
test backend::git_fetch::tests::upstream_repository_is_derived_without_forwarding_provider_metadata ... ok
test backend::git_fetch::tests::unsupported_protocol_is_refused_before_provider_egress ... ok
test backend::git_fetch::tests::removed_principal_connection_revokes_a_live_session ... ok
test backend::tests::a_gitlab_refresh_response_without_scope_is_accepted_for_live_reverification ... ok
test backend::tests::datasource_cursors_are_bound_to_connection_and_project ... ok
test backend::tests::datasource_projection_drops_sensitive_and_unknown_fields ... ok
test backend::git_fetch::tests::current_grant_and_provider_default_tip_are_revalidated ... ok
test backend::git_fetch::v2::tests::request_framing_refuses_truncation_ambiguity_and_oversize ... ok
test backend::tests::pat_shape_rejects_whitespace_and_oversize_values ... ok
test backend::git_fetch::tests::discovery_advertises_only_the_exact_default_branch_snapshot ... ok
test backend::tests::profiles_are_closed_and_self_service ... ok
test backend::git_fetch::tests::project_and_branch_authority_reads_overlap_on_creation_and_each_exchange ... ok
test backend::git_fetch::tests::idempotent_replay_keeps_locator_and_rotates_transient_authority ... ok
test backend::git_fetch::tests::upload_pack_stream_is_bounded_and_spends_the_session ... ok
test backend::tests::recorded_scopes_are_the_retained_subset_sorted_and_deduped ... ok
test backend::git_fetch::v2::tests::commands_are_closed_and_prefixes_cannot_expand_upstream_discovery ... ok
test backend::tests::repository_file_paths_cannot_traverse_or_change_root ... ok
test backend::tests::the_adapter_carries_every_field_gitlab_sends ... ok
test backend::tests::origins_are_exact_https_only ... ok
test backend::tests::the_exchange_policy_refuses_everything_the_inline_condition_refused ... ok
test backend::tests::the_refresh_policy_ignores_the_two_fields_that_path_recomputes ... ok
test backend::tests::the_refresh_policy_still_requires_bearer_and_a_refresh_token ... ok
test transport::tests::oauth_forms_encode_secret_delimiters_without_logging_values ... ok
test backend::tests::malformed_state_and_empty_grant_policy_still_fail_closed ... ok
test backend::git_fetch::v2::tests::capabilities_require_v2_shallow_sha1_and_strip_expanding_features ... ok
test transport::tests::page_decoding_reads_only_the_selected_cursor_header ... ok
test backend::tests::legacy_connection_starts_unusable_and_preserves_pending_custody ... ok
test backend::git_fetch::tests::v2_drop_budget_revocation_and_foreign_authority_fail_closed ... ok
test backend::tests::project_admission_refuses_a_non_advancing_continuation ... ok
test backend::tests::project_admission_reaches_a_repository_after_the_first_hundred ... ok
test backend::git_fetch::tests::v2_negotiation_is_bound_to_generation_and_only_completed_fetch_spends_it ... ok
test backend::git_fetch::tests::per_principal_capacity_is_atomic_and_never_evicts_a_live_session ... ok
test backend::git_fetch::tests::global_capacity_refuses_without_evicting_an_inflight_other_principal ... ok
test backend::git_fetch::v2::tests::refs_filter_prefix_collisions_and_verify_full_response_before_emission ... ok
test backend::git_fetch::v2::tests::pack_is_streamed_but_final_framing_and_sections_are_enforced ... ok
test backend::git_fetch::tests::stream_expiry_revokes_a_stalled_upload ... ok
test backend::git_fetch::http_tests::real_v2_clone_preserves_depth_and_reduces_many_ref_discovery_bytes ... ok
test backend::tests::repository_file_paths_are_encoded_as_one_gitlab_segment ... ok

test result: ok. 40 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.51s

     Running unittests src/lib.rs (crates/connectors-runtime/target/debug/deps/integration_jira-85992ec5e607f3ec)

running 12 tests
test backend::auth::tests::oauth_scopes_are_canonical_and_exact ... ok
test backend::auth::tests::the_service_policy_needs_only_the_read_scope_and_no_refresh_token ... ok
test backend::auth::tests::the_refresh_policy_allows_a_response_that_rotates_only_the_access_token ... ok
test backend::auth::tests::the_exchange_policy_refuses_everything_the_inline_condition_refused ... ok
test backend::operations::tests::all_writes_require_approval ... ok
test backend::operations::tests::issue_keys_bind_an_exact_project ... ok
test backend::tests::delegated_connection_is_withdrawn_when_its_grant_changes ... ok
test backend::operations::tests::operation_inputs_are_closed_and_bounded_before_request_assembly ... ok
test backend::tests::organization_and_user_profiles_are_distinct ... ok
test backend::datasource::tests::projection_drops_sensitive_and_unknown_provider_fields ... ok
test backend::datasource::tests::schemas_are_closed_and_projection_is_stable ... ok
test backend::operations::tests::operation_outputs_are_closed_safe_projections ... ok

test result: ok. 12 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.11s

     Running unittests src/lib.rs (crates/connectors-runtime/target/debug/deps/integration_kubernetes-a922c63d16f06780)

running 63 tests
test hosted::database_tests::a_404_from_a_served_group_is_an_error_not_an_empty_inventory ... ok
test hosted::database_tests::database_endpoint_bindings_appear_per_admitted_namespace_only ... ok
test hosted::database_tests::a_database_read_on_a_non_admitted_namespace_is_not_granted ... ok
test hosted::database_tests::search_lists_the_databases_datasource_under_its_terms ... ok
test hosted::tests::deployment_projection_requires_observed_available_replicas ... ok
test hosted::tests::a_status_invocation_survives_the_scope_change_between_describe_and_invoke ... ok
test hosted::tests::an_upstream_log_refusal_surfaces_as_not_granted ... ok
test hosted::tests::hosted_connection_projection_is_value_free_and_tenant_bound ... ok
test hosted::tests::read_only_status_is_description_bound_and_namespace_scoped ... ok
test hosted::tests::pod_log_invoke_enforces_the_input_caps_before_the_reader ... ok
test hosted::tests::a_missing_namespace_grant_names_the_namespace_and_the_group_that_carries_it ... ok
test hosted::database_tests::a_cluster_without_crossplane_discovers_nothing ... ok
test hosted::tests::an_unknown_binding_is_named_rather_than_reported_as_an_ungranted_one ... ok
test local::inventory_tests::adversary_inventory_rejects_null_cursor_as_its_published_schema_requires ... ok
test hosted::database_tests::database_datasource_description_names_the_projection_for_read_principals_only ... ok
test local::inventory_tests::adversary_inventory_walks_empty_pages_and_preserves_all_regular_container_images ... ok
test hosted::database_tests::database_endpoint_list_derives_descriptors_from_both_engines ... ok
test local::inventory_tests::adversary_final_overlapping_cursor_replays_dispatch_only_once ... ok
test local::inventory_tests::inventory_discovery_publishes_both_reads_for_every_activated_connection ... ok
test hosted::database_tests::no_secret_value_ever_appears_in_database_endpoint_output ... ok
test local::inventory_tests::inventory_namespaces_are_configured_admission_without_cluster_enumeration ... ok
test hosted::database_tests::database_endpoint_get_returns_one_descriptor_by_name ... ok
test local::inventory_tests::adversary_final_empty_fetch_budget_retains_continuation_and_malformed_pages_refuse ... ok
test local::inventory_tests::adversary_final_invalid_inputs_do_not_spend_a_live_inventory_cursor ... ok
test local::inventory_tests::inventory_fresh_cursor_cannot_cross_a_connection_or_namespace_boundary ... ok
test hosted::tests::a_workload_read_survives_the_access_token_rotation_between_describe_and_read ... ok
test local::inventory_tests::inventory_inputs_require_the_published_object_shape ... ok
test local::inventory_tests::inventory_preserves_owner_and_description_lease_admission ... ok
test local::inventory_tests::inventory_preserves_rbac_refusal ... ok
test local::tests::a_half_attached_cluster_is_not_published ... ok
test local::tests::a_renamed_argocd_release_is_recognized_by_its_identity_label ... ok
test local::tests::argocd_recognition_takes_the_api_service_and_none_of_its_siblings ... ok
test local::inventory_tests::inventory_shared_reader_preserves_the_existing_compact_datasource_shape ... ok
test local::tests::every_activated_cluster_is_published_in_a_stable_order ... ok
test local::tests::monitoring_service_recognition_is_curated ... ok
test local::inventory_tests::inventory_refuses_nonactivated_connections_unadmitted_namespaces_and_bad_inputs_before_io ... ok
test local::tests::providers_that_need_a_credential_are_not_materializable_here ... ok
test local::inventory_tests::inventory_optional_limit_accepts_every_in_range_integer_number_representation ... ok
test local::inventory_tests::inventory_optional_inputs_distinguish_absence_from_explicit_values ... ok
test local::inventory_tests::inventory_optional_output_cursor_is_omitted_or_a_string_never_null ... ok
test local::inventory_tests::inventory_reads_each_selected_cluster_and_retains_scaled_to_zero_template_images ... ok
test local::tests::service_observation_pins_uid_and_one_closed_tcp_port ... ok
test local::tests::the_argocd_observation_pins_the_api_port_rather_than_the_redirect ... ok
test local_workloads::tests::a_binding_ref_that_names_no_configured_namespace_is_the_callers_mistake ... ok
test local_workloads::tests::a_name_that_is_not_a_dns_label_never_reaches_a_request_path ... ok
test hosted::tests::datasource_projects_only_safe_workload_fields_for_granted_namespaces ... ok
test local_workloads::tests::a_cluster_listing_becomes_the_same_compact_record_the_deployment_returns ... ok
test local_workloads::tests::query_values_are_encoded_rather_than_interpolated ... ok
test hosted::database_tests::database_endpoint_listing_pages_across_both_engines ... ok
test local::inventory_tests::inventory_refuses_upstream_page_overrun_and_wrong_namespace ... ok
test local::tests::insecure_api_server_contexts_are_not_candidates ... ok
test local::tests::passive_candidates_expose_only_context_label_and_opaque_evidence ... ok
test local_workloads::tests::the_local_placement_publishes_the_deployments_projection_verbatim ... ok
test local::inventory_tests::inventory_pagination_is_bounded_and_cursors_bind_connection_and_namespace ... ok
test hosted::tests::an_oversized_log_body_is_front_trimmed_inside_a_validating_envelope ... ok
test hosted::tests::search_lists_pod_logs_for_read_group_principals_and_hides_it_otherwise ... ok
test hosted::tests::describing_without_a_read_grant_names_the_grant_rather_than_a_missing_datasource ... ok
test local::inventory_tests::inventory_refuses_oversized_provider_and_projected_values ... ok
test hosted::tests::pod_log_invoke_refuses_a_non_admitted_namespace_and_a_stale_lease ... ok
test hosted::tests::pod_log_description_carries_schemas_and_a_lease_for_read_principals_only ... ok
test hosted::tests::restart_requires_sre_group_and_exact_resource_authority_without_local_approval ... ok
test hosted::tests::pod_log_invoke_passes_the_input_through_and_defaults_tail_lines ... ok
test hosted::paging_tests::a_busy_namespace_lists_in_full_despite_the_upstream_response_bound ... ok

test result: ok. 63 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.11s

     Running unittests src/lib.rs (crates/connectors-runtime/target/debug/deps/integration_mcp-43ed8b0161225b81)

running 2 tests
test tests::changed_live_snapshot_is_refused_before_a_factory_exists ... ok
test tests::frozen_reviewed_tools_cross_connector_custody_and_egress ... ok

test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s

     Running unittests src/lib.rs (crates/connectors-runtime/target/debug/deps/integration_monitoring-0d9dece07ad0caa1)

running 16 tests
test backend::tests::refusal_log_record_names_operation_route_and_exact_upstream_status ... ok
test backend::tests::safe_projections_drop_provider_secrets_and_redact_free_text ... ok
test backend::tests::readiness_checks_only_the_mandatory_credential_store ... ok
test backend::tests::standalone_adapter_refuses_unowned_requests_without_fallthrough ... ok
test backend::tests::failed_credential_custody_rolls_back_discovery_and_parent_state ... ok
test backend::tests::hosted_federation_is_digest_bound_group_scoped_and_has_no_connect_session ... ok
test backend::tests::connect_session_uses_shared_transport_and_publishes_only_after_secret_custody ... ok
test backend::tests::credential_custody_failure_is_distinguished_from_upstream_failures ... ok
test backend::tests::oversized_upstream_body_refuses_as_result_bound_not_unreachable ... ok
test backend::tests::mediated_alertmanager_dispatch_resolves_the_v2_api_path ... ok
test backend::tests::discovery_materialization_and_query_stay_on_the_grafana_route ... ok
test backend::tests::prometheus_range_accepts_integer_epoch_seconds_on_the_mediated_route ... ok
test backend::tests::dashboards_list_dispatches_the_documents_required_only_input_over_http ... ok
test backend::tests::concurrent_completions_publish_exactly_one_parent_connection ... ok
test backend::tests::dashboards_list_pages_upstream_with_a_bounded_limit_and_fetch_budget ... ok
test backend::tests::refused_dispatches_distinguish_upstream_status_class_from_transport ... ok

test result: ok. 16 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.13s

     Running unittests src/lib.rs (crates/connectors-runtime/target/debug/deps/integration_platform-b3750dcf27f1b300)

running 23 tests
test tests::ontology_nullable_fields_are_still_strict_after_catalog_lowering ... ok
test tests::every_declared_write_requires_external_approval ... ok
test tests::browser_catalog_symbol_is_translated_into_the_closed_driver_input ... ok
test work_events::tests::cursors_events_and_replay_are_partitioned_by_tenant ... ok
test tests::a_mutating_post_dispatch_failure_is_not_declared_retriable ... ok
test tests::every_projected_operation_has_a_response_schema ... ok
test tests::work_owner_events_are_checkpointed_into_connector_sequence_space ... ok
test tests::a_workspace_read_survives_the_access_token_rotation_between_describe_and_read ... ok
test tests::an_unknown_workspace_binding_is_named_rather_than_reported_as_stale ... ok
test tests::planner_owner_events_are_checkpointed_into_connector_sequence_space ... ok
test tests::workspace_datasource_projects_only_the_logical_read_model ... ok
test tests::total_http_deadline_bounds_a_stalled_private_service ... ok
test tests::hosted_tenant_member_defaults_are_an_explicit_module_ceiling ... ok
test tests::search_names_each_operation_once_and_never_by_its_second_name ... ok
test tests::search_projects_only_configured_capabilities ... ok
test tests::every_name_of_an_operation_describes_one_operation ... ok
test tests::module_global_ids_resolve_for_declarative_ui_requirements ... ok
test tests::a_write_passes_no_local_approval_gate ... ok
test tests::planner_invocation_crosses_the_private_http_boundary_with_signed_authority ... ok
test tests::invalid_post_dispatch_output_is_audited_as_indeterminate ... ok
test tests::work_invocation_crosses_the_private_http_boundary_with_signed_authority ... ok
test tests::ontology_invocation_carries_request_bound_signed_authority ... ok
test tests::local_work_invocation_is_constrained_to_the_configured_unix_socket ... ok

test result: ok. 23 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.62s

     Running unittests src/lib.rs (crates/connectors-runtime/target/debug/deps/integration_sip-33734966db804966)

running 14 tests
test raw::tests::a_chosen_device_is_the_one_bound ... ok
test raw::tests::readiness_contacts_nothing ... ok
test raw::tests::a_host_with_no_sound_stack_still_composes_a_launcher ... ok
test raw::tests::the_receipt_claims_no_application_channel ... ok
test runtime::tests::stored_credential_readiness_is_value_free_and_reports_store_unavailability ... ok
test runtime::tests::missing_sip_credentials_fail_closed ... ok
test runtime::tests::stored_credentials_are_tenant_scoped_ordered_and_redacted ... ok
test runtime::tests::authority_key_must_be_an_owner_only_real_file ... ok
test backend::tests::readiness_delegates_to_the_mandatory_launcher_probe_without_launching ... ok
test backend::tests::an_unknown_session_is_not_found_and_a_refused_signal_is_reported ... ok
test backend::tests::a_binding_that_cannot_signal_refuses_rather_than_dropping_the_keypress ... ok
test backend::tests::a_signal_reaches_the_live_session_and_leaves_it_established ... ok
test backend::tests::catalog_projection_invocation_session_control_and_audit_share_one_path ... ok
test backend::tests::stale_owner_provider_only_unknown_alias_and_restart_reconciliation_refuse ... ok

test result: ok. 14 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.33s

     Running unittests src/lib.rs (crates/connectors-runtime/target/debug/deps/integration_slack-a2db89441bc2eb36)

running 27 tests
test backend::tests::hosted_companion_completion_requires_distinct_app_and_bot_credentials ... ok
test backend::tests::a_local_companion_submission_is_one_bot_token_and_nothing_else ... ok
test backend::tests::hosted_completion_errors_separate_conflicts_from_store_outages ... ok
test backend::tests::datasource_projection_excludes_unreviewed_slack_profile_fields ... ok
test backend::tests::a_declared_instance_name_fixes_its_identity_for_good ... ok
test backend::tests::hosted_setup_page_requires_capability_and_distinguishes_safe_failures ... ok
test backend::tests::message_loop_guards_and_closed_event_grants_are_applied_before_storage ... ok
test backend::tests::only_the_inner_admitted_event_is_projected ... ok
test backend::tests::slack_auth_test_provider_and_transport_failures_are_unavailable ... ok
test backend::tests::slack_auth_test_refuses_only_explicit_invalid_credentials ... ok
test backend::tests::socket_ticket_destination_is_closed_to_slack_tls_hosts ... ok
test backend::tests::a_credential_file_other_accounts_can_read_is_refused_rather_than_used ... ok
test backend::tests::operation_audit_is_durable_bounded_and_value_free ... ok
test backend::tests::a_connection_receiving_fewer_events_than_the_policy_lists_is_still_admitted ... ok
test backend::tests::event_is_durable_and_deduplicated_before_pull_and_replay ... ok
test tests::organization_credentials_do_not_claim_personal_oauth_is_configured ... ok
test backend::tests::describing_without_a_bound_connection_names_the_connection_not_a_missing_datasource ... ok
test backend::tests::ephemeral_open_never_starts_a_socket_mode_supervisor ... ok
test backend::tests::organization_bot_is_admitted_for_reads_without_an_event_channel ... ok
test backend::tests::hosted_sessions_expire_and_release_pending_capacity_without_submission ... ok
test backend::tests::invalid_hosted_capability_cannot_consume_a_connect_session ... ok
test backend::tests::one_use_completion_publishes_only_value_free_connection_state ... ok
test backend::tests::datasource_description_lease_ignores_request_scoped_provenance ... ok
test backend::tests::read_ownership_matches_describe_ownership_for_every_slack_datasource ... ok
test backend::tests::slack_readiness_is_value_free_and_tracks_the_secret_store ... ok
test backend::tests::standalone_adapter_claims_only_its_connection_and_event_families ... ok
test backend::tests::stale_grant_metadata_cannot_reenter_any_connection_or_event_surface ... ok

test result: ok. 27 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.05s

     Running unittests src/lib.rs (crates/connectors-runtime/target/debug/deps/monitoring_model-bff90c2a730223eb)

running 4 tests
test tests::loki_timestamps_stay_strings_and_the_refusal_names_the_encoding ... ok
test tests::the_validator_admits_the_documents_required_only_input ... ok
test tests::the_validator_still_refuses_outside_the_documents_contract ... ok
test tests::prometheus_timestamps_accept_integer_epoch_seconds_beside_strings ... ok

test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.51s

     Running unittests src/lib.rs (crates/connectors-runtime/target/debug/deps/state_sqlite-d2d68a7b2e04f0bb)

running 6 tests
test tests::concatenation_would_have_corrupted_binary_and_the_transaction_does_not ... ok
test tests::the_in_memory_backend_conforms ... ok
test tests::the_in_memory_backend_serves_grant_evaluation ... ok
test tests::a_cell_survives_reopening_the_file ... ok
test tests::the_file_backend_serves_grant_evaluation ... ok
test tests::the_file_backend_conforms ... ok

test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.04s

     Running tests/approval_gate.rs (crates/connectors-runtime/target/debug/deps/approval_gate-71f55b4f8f6e6bf3)

running 3 tests
test sixteen_concurrent_identical_presentations_redeem_exactly_once ... ok
test a_replay_survives_reopening_the_database ... ok
test a_crash_between_redemption_and_terminal_write_leaves_a_recoverable_attempted_row ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.09s

   Doc-tests connect_session_transport

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

   Doc-tests connectors_config

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

   Doc-tests connectors_runtime

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

   Doc-tests hosted_secrets

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

   Doc-tests hosted_state

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

   Doc-tests hosted_vault

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

   Doc-tests identity_http

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

   Doc-tests integration_catalog

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

   Doc-tests integration_gitlab

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

   Doc-tests integration_jira

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

   Doc-tests integration_kubernetes

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

   Doc-tests integration_mcp

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

   Doc-tests integration_monitoring

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

   Doc-tests integration_platform

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

   Doc-tests integration_sip

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

   Doc-tests integration_slack

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

   Doc-tests monitoring_model

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

   Doc-tests state_sqlite

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s


exit 0
```

```text
$ cargo test --manifest-path crates/connectors-console/Cargo.toml --workspace --locked --no-fail-fast
    Blocking waiting for file lock on package cache
    Blocking waiting for file lock on package cache
    Blocking waiting for file lock on package cache
   Compiling integration-catalog v0.6.5 (~/.local/state/worktree/trees/b10x/connectors/wt-fc49f37347f5/crates/integration-catalog)
   Compiling connectors-console v0.6.5 (~/.local/state/worktree/trees/b10x/connectors/wt-fc49f37347f5/crates/connectors-console)
    Finished `test` profile [unoptimized] target(s) in 2.61s
     Running unittests src/lib.rs (crates/connectors-console/target/debug/deps/connectors_console-a9e445f497b91e81)

running 72 tests
test auth::tests::nothing_in_the_result_can_carry_a_secret ... ok
test auth::tests::a_basic_credential_row_reports_whether_its_user_half_is_configured_and_never_the_value ... ok
test admin::tests::explicit_secret_file_must_be_owner_only ... ok
test connect::tests::the_error_for_an_unknown_provider_names_it ... ok
test auth::tests::the_store_preference_matches_what_the_runtime_composes ... ok
test doctor::tests::a_short_state_root_passes_both_budgets ... ok
test doctor::tests::a_report_is_unhealthy_only_when_something_cannot_work ... ok
test doctor::tests::a_missing_configuration_is_fatal_and_names_the_command_that_fixes_it ... ok
test connect::tests::a_catalogued_provider_whose_curated_backend_is_absent_takes_the_catalogue_path ... ok
test doctor::tests::every_state_a_check_can_report_reaches_the_reader_as_its_own_marker ... ok
test admin::tests::command_shape_accepts_secret_stdin_without_a_secret_argument ... ok
test doctor::tests::doctor_names_the_default_local_target_and_its_socket ... ok
test connect::tests::a_provider_outside_the_guided_set_is_refused_by_name ... ok
test doctor::tests::the_report_renders_every_check_as_data ... ok
test enrol::tests::a_provider_outside_the_catalogue_is_named_rather_than_guessed_at ... ok
test doctor::tests::the_budget_is_measured_against_the_deepest_path_the_daemon_binds ... ok
test envelope::tests::a_result_loses_its_envelope_and_its_discriminant ... ok
test envelope::tests::an_envelope_carrying_neither_is_a_named_failure_not_an_empty_success ... ok
test envelope::tests::a_refusal_becomes_an_error_rather_than_a_result ... ok
test init::tests::admitting_a_credential_plugin_is_a_choice_and_its_absence_is_explained ... ok
test init::tests::an_agent_id_is_stable_across_calls ... ok
test init::tests::the_separator_keeps_a_concatenation_from_colliding ... ok
test init::tests::the_snapshot_digest_is_stable_and_moves_with_the_admitted_set ... ok
test input::tests::an_inline_object_is_parsed ... ok
test init::tests::an_existing_configuration_is_never_replaced_silently ... ok
test input::tests::no_source_names_all_three_rather_than_defaulting_to_empty ... ok
test input::tests::input_accepts_only_the_stdin_marker ... ok
test output::tests::a_payload_carrying_its_own_value_field_is_left_alone ... ok
test output::tests::a_field_a_record_does_not_carry_reads_as_absent_rather_than_blank ... ok
test output::tests::a_record_that_is_not_an_object_keeps_the_name_the_report_gave_it ... ok
test output::tests::a_structured_format_carries_its_failure_on_stdout ... ok
test input::tests::a_file_is_read_from_its_path ... ok
test output::tests::a_row_shows_its_severity_before_anybody_reads_it ... ok
test output::tests::a_table_reads_left_to_right_with_the_column_that_runs_long_last ... ok
test output::tests::a_wide_character_cell_keeps_the_column_after_it_aligned ... ok
test output::tests::an_empty_listing_is_an_empty_stream_rather_than_a_line_shaped_like_a_record ... ok
test output::tests::a_word_the_renderer_cannot_rank_is_marked_unknown_rather_than_good ... ok
test output::tests::an_object_with_two_arrays_is_not_unwrapped ... ok
test output::tests::an_unranked_table_still_keeps_the_marker_column ... ok
test output::tests::columns_of_equal_width_keep_the_order_the_record_carries ... ok
test output::tests::compact_leaves_a_single_record_as_one_line ... ok
test output::tests::compact_keeps_a_field_a_record_carries_below_its_top_level ... ok
test output::tests::compact_keeps_the_scalar_a_list_response_carries_beside_its_records ... ok
test output::tests::compact_unwraps_the_one_array_a_list_response_carries ... ok
test output::tests::every_protocol_state_this_renderer_can_be_handed_has_a_rank ... ok
test output::tests::text_does_not_quote_a_string_a_person_is_reading ... ok
test output::tests::no_cell_is_ever_empty_so_no_row_can_end_in_whitespace ... ok
test output::tests::severity_survives_a_pipe_because_it_is_not_carried_by_colour ... ok
test output::tests::text_says_none_rather_than_printing_an_empty_bracket ... ok
test output::tests::text_keeps_every_field_a_record_carries_including_a_nested_list ... ok
test output::tests::the_result_discriminant_is_stripped_so_compact_can_see_the_records ... ok
test output::tests::text_spends_one_aligned_row_on_each_record ... ok
test output::tests::the_widest_column_moves_last_even_when_the_record_puts_it_first ... ok
test output::tests::the_structured_formats_render_the_bytes_they_rendered_before ... ok
test output::tests::yaml_renders_through_the_maintained_crate ... ok
test output::tests::a_cell_never_carries_a_character_that_breaks_the_row ... ok
test output::tests::every_status_word_this_package_emits_is_one_the_renderer_can_rank ... ok
test init::tests::a_configuration_the_daemon_would_refuse_is_not_left_on_disk ... ok
test init::tests::what_init_writes_is_what_the_daemon_can_read ... ok
test auth::tests::the_catalogue_is_what_says_a_credential_has_a_user_half ... ok
test enrol::tests::most_of_the_catalogue_asks_no_configuration_question_at_all ... ok
test enrol::tests::a_self_hosted_origin_is_the_case_operator_approval_exists_for ... ok
test enrol::tests::gitlab_asks_for_nothing_when_its_default_origin_is_wanted ... ok
test enrol::tests::slack_declares_a_bot_and_a_user_credential_which_one_identity_may_both_hold ... ok
test providers::tests::an_unmatched_query_is_an_empty_listing_rather_than_the_whole_catalogue ... ok
test providers::tests::a_provider_without_a_probe_is_not_ready_and_says_why_by_omission ... ok
test providers::tests::a_query_narrows_to_one_provider_and_its_summary_follows ... ok
test providers::tests::the_shipped_catalogue_is_reported_rather_than_asserted ... ok
test output::tests::two_providers_that_differ_in_their_id_differ_on_screen ... ok
test output::tests::a_table_too_wide_for_a_terminal_starts_its_last_column_inside_the_budget ... ok
test output::tests::the_budget_is_documented_as_what_it_is_and_a_real_row_is_wider_than_it ... ok
test output::tests::a_cell_the_budget_cut_says_so_and_the_column_names_are_cut_last ... ok

test result: ok. 72 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.00s

     Running tests/adversary_budget_prose.rs (crates/connectors-console/target/debug/deps/adversary_budget_prose-21b48019874fc83a)

running 3 tests
test pass3_render_helper_child ... ok
test the_quoted_module_header_sentence_is_at_the_line_the_pass_two_suite_cites ... ok
test the_widths_the_documents_state_are_the_widths_the_renderer_prints ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.01s

     Running tests/adversary_readability.rs (crates/connectors-console/target/debug/deps/adversary_readability-94d6777785959bc6)

running 7 tests
test render_helper_child ... ok
test a_record_whose_cells_are_all_empty_is_rendered_as_a_blank_line ... ok
test an_unranked_table_lets_a_cell_sit_where_the_severity_marker_sits ... ok
test a_wide_character_cell_leaves_the_column_after_it_ragged ... ok
test doctor_spreads_one_check_over_several_unmarked_lines_when_the_configuration_is_malformed ... ok
test compact_no_longer_puts_one_record_on_every_line ... ok
test providers_starts_its_last_column_past_the_width_of_any_terminal ... ok

test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.05s

     Running tests/adversary_readability_pass2.rs (crates/connectors-console/target/debug/deps/adversary_readability_pass2-09563cbc31cb5150)

running 5 tests
test pass2_render_helper_child ... ok
test compact_drops_the_name_of_the_array_a_report_carries ... ok
test a_column_the_budget_squeezes_to_nothing_pushes_every_later_column_out_of_line ... ok
test compact_answers_an_empty_listing_with_a_line_that_is_not_a_record ... ok
test the_last_column_of_providers_begins_one_column_past_the_terminal_it_is_laid_out_for ... ok

test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.24s

   Doc-tests connectors_console

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s


exit 0
```

```text
$ cargo test --manifest-path crates/server/Cargo.toml --workspace --locked --no-fail-fast
   Compiling syn v3.0.3
   Compiling syn v2.0.119
   Compiling typenum v1.20.1
   Compiling serde_json v1.0.151
   Compiling serde_derive v1.0.229
   Compiling thiserror-impl v2.0.20
   Compiling synstructure v0.13.2
   Compiling generic-array v0.14.7
   Compiling thiserror v2.0.20
   Compiling zerofrom-derive v0.1.7
   Compiling crypto-common v0.1.7
   Compiling block-buffer v0.10.4
   Compiling yoke-derive v0.8.2
   Compiling serde v1.0.229
   Compiling digest v0.10.7
   Compiling zerovec-derive v0.11.4
   Compiling zerofrom v0.1.8
   Compiling displaydoc v0.2.7
   Compiling yoke v0.8.3
   Compiling sha2 v0.10.9
   Compiling async-trait v0.1.92
   Compiling zerovec v0.11.7
   Compiling zerotrie v0.2.5
   Compiling rustix v1.1.4
   Compiling tinystr v0.8.4
   Compiling icu_locale_core v2.3.0
   Compiling potential_utf v0.1.6
   Compiling icu_provider v2.3.0
   Compiling icu_collections v2.3.0
   Compiling connector-address v0.6.5 (~/.local/state/worktree/trees/b10x/connectors/wt-fc49f37347f5/crates/connector-address)
   Compiling icu_properties v2.3.0
   Compiling icu_normalizer v2.3.0
   Compiling linux-raw-sys v0.12.1
   Compiling idna_adapter v1.2.2
   Compiling idna v1.1.0
   Compiling ref-cast-impl v1.0.26
   Compiling tokio-macros v2.7.2
   Compiling ref-cast v1.0.26
   Compiling slab v0.4.12
   Compiling url v2.5.8
   Compiling tracing-attributes v0.1.31
   Compiling futures-macro v0.3.34
   Compiling tokio v1.53.1
   Compiling serde_core v1.0.229
   Compiling serde_derive_internals v0.30.0
   Compiling futures-util v0.3.34
   Compiling parking v2.2.1
   Compiling winnow v1.0.4
   Compiling schemars_derive v1.2.2
   Compiling tracing v0.1.44
   Compiling connector-state v0.6.5 (~/.local/state/worktree/trees/b10x/connectors/wt-fc49f37347f5/crates/connector-state)
   Compiling hybrid-array v0.4.14
   Compiling num v0.4.3
   Compiling crossbeam-utils v0.8.22
   Compiling hashbrown v0.17.1
   Compiling indexmap v2.14.0
   Compiling toml_parser v1.1.3+spec-1.1.0
   Compiling fastrand v2.5.0
   Compiling schemars v1.2.2
   Compiling toml_datetime v1.1.1+spec-1.1.0
   Compiling toml_edit v0.25.13+spec-1.1.0
   Compiling hyper v1.11.0
   Compiling connector-secrets v0.6.5 (~/.local/state/worktree/trees/b10x/connectors/wt-fc49f37347f5/crates/connector-secrets)
   Compiling enumflags2_derive v0.7.12
   Compiling futures-io v0.3.34
   Compiling domain v0.6.5 (~/.local/state/worktree/trees/b10x/connectors/wt-fc49f37347f5/crates/domain)
   Compiling hyper-util v0.1.20
   Compiling zvariant_utils v4.2.0
   Compiling concurrent-queue v2.5.0
   Compiling proc-macro-crate v3.5.0
   Compiling tower v0.5.3
   Compiling tokio-rustls v0.26.4
   Compiling crypto-common v0.2.2
   Compiling event-listener v5.4.2
   Compiling toml_datetime v0.6.11
   Compiling serde_spanned v0.6.9
   Compiling event-listener-strategy v0.5.4
   Compiling toml_edit v0.22.27
   Compiling futures-lite v2.6.1
   Compiling serde_urlencoded v0.7.1
   Compiling zvariant_derive v5.15.0
   Compiling endi v1.1.1
   Compiling serde_norway v0.9.42
   Compiling hyper-rustls v0.27.9
   Compiling tower-http v0.6.11
   Compiling protocol v0.6.5 (~/.local/state/worktree/trees/b10x/connectors/wt-fc49f37347f5/crates/protocol)
   Compiling catalog v0.6.5 (~/.local/state/worktree/trees/b10x/connectors/wt-fc49f37347f5/crates/catalog)
   Compiling toml v0.8.23
   Compiling tokio-util v0.7.19
   Compiling async-io v2.6.0
   Compiling aho-corasick v1.1.5
   Compiling cmov v0.5.4
   Compiling ctutils v0.4.2
   Compiling regex-automata v0.4.18
   Compiling reqwest v0.12.28
   Compiling ahash v0.8.12
   Compiling parking_lot_core v0.9.12
   Compiling enumflags2 v0.7.12
   Compiling zcheapstr v1.1.0
   Compiling block-buffer v0.12.1
   Compiling block-padding v0.4.2
   Compiling polling v3.11.0
   Compiling errno v0.3.14
   Compiling const-oid v0.10.2
   Compiling async-task v4.7.1
   Compiling digest v0.11.3
   Compiling fraction v0.15.4
   Compiling fluent-uri v0.4.1
   Compiling signal-hook-registry v1.4.8
   Compiling inout v0.2.2
   Compiling zvariant v5.15.0
   Compiling parking_lot v0.12.5
   Compiling connector-spec v0.6.5 (~/.local/state/worktree/trees/b10x/connectors/wt-fc49f37347f5/crates/connector-spec)
   Compiling strum_macros v0.28.0
   Compiling async-channel v2.5.0
   Compiling jsonschema-value v0.49.9
   Compiling referencing v0.49.9
   Compiling axum-core v0.5.6
   Compiling strum v0.28.0
   Compiling zbus_names v4.3.4
   Compiling cipher v0.5.2
   Compiling async-signal v0.2.14
   Compiling fancy-regex v0.19.0
   Compiling regex v1.13.1
   Compiling async-lock v3.4.2
   Compiling piper v0.2.5
   Compiling email_address v0.2.9
   Compiling curve25519-dalek-derive v0.1.1
   Compiling cpufeatures v0.3.0
   Compiling getrandom v0.4.3
   Compiling axum v0.8.9
   Compiling jsonschema v0.49.9
   Compiling curve25519-dalek v4.1.3
   Compiling blocking v1.7.0
   Compiling async-process v2.5.0
   Compiling zbus_macros v5.19.0
   Compiling hmac v0.13.0
   Compiling async-executor v1.14.0
   Compiling connector-resolve v0.6.5 (~/.local/state/worktree/trees/b10x/connectors/wt-fc49f37347f5/crates/connector-resolve)
   Compiling async-broadcast v0.7.2
   Compiling connector-oauth v0.6.5 (~/.local/state/worktree/trees/b10x/connectors/wt-fc49f37347f5/crates/connector-oauth)
   Compiling ordered-stream v0.2.0
   Compiling async-recursion v1.1.1
   Compiling serde_repr v0.1.21
   Compiling uuid v1.26.0
   Compiling cpubits v0.1.1
   Compiling utf8parse v0.2.2
   Compiling zbus v5.19.0
   Compiling anstyle-parse v1.0.0
   Compiling aes v0.9.2
   Compiling hkdf v0.13.0
   Compiling ed25519-dalek v2.2.0
   Compiling sha2 v0.11.0
   Compiling cbc v0.2.1
   Compiling anstyle-query v1.1.5
   Compiling anstyle v1.0.14
   Compiling is_terminal_polyfill v1.70.2
   Compiling colorchoice v1.0.5
   Compiling catalog-build v0.6.5 (~/.local/state/worktree/trees/b10x/connectors/wt-fc49f37347f5/crates/catalog-build)
   Compiling anstream v1.0.0
   Compiling keyring-core v1.0.0
   Compiling sha1 v0.10.7
   Compiling clap_lex v1.1.0
   Compiling strsim v0.11.1
   Compiling clap_builder v4.6.6
   Compiling tungstenite v0.28.0
   Compiling service v0.6.5 (~/.local/state/worktree/trees/b10x/connectors/wt-fc49f37347f5/crates/service)
   Compiling subscription-custody v0.6.5 (~/.local/state/worktree/trees/b10x/connectors/wt-fc49f37347f5/crates/subscription-custody)
   Compiling clap_derive v4.6.4
   Compiling tokio-tungstenite v0.28.0
   Compiling identity-client v0.5.6 (https://github.com/beyond10x/identity.git?tag=0.5.6#e3231bc3)
   Compiling tempfile v3.27.0
   Compiling server v0.6.5 (~/.local/state/worktree/trees/b10x/connectors/wt-fc49f37347f5/crates/server)
   Compiling clap v4.6.6
   Compiling catalog-cli v0.6.5 (~/.local/state/worktree/trees/b10x/connectors/wt-fc49f37347f5/crates/catalog-cli)
   Compiling secret-service v5.2.0
   Compiling zbus-secret-service-keyring-store v1.0.1
   Compiling keyring v4.2.0
   Compiling connectors-client v0.6.5 (~/.local/state/worktree/trees/b10x/connectors/wt-fc49f37347f5/crates/connectors-client)
   Compiling catalog-reader v0.6.5 (~/.local/state/worktree/trees/b10x/connectors/wt-fc49f37347f5/crates/catalog-reader)
    Finished `test` profile [unoptimized] target(s) in 1m 07s
     Running unittests src/lib.rs (target/debug/deps/catalog-bf05e0919bf12a7e)

running 9 tests
test table::tests::cdp_v1_survives_the_generated_table ... ok
test table::tests::sip_v1_survives_the_generated_table ... ok
test table::tests::a_minting_join_in_the_document_reaches_acquisition_minted ... ok
test table::tests::the_document_tells_apart_the_pair_the_derivation_could_not ... ok
test tests::an_operation_is_found_by_its_symbol ... ok
test tests::providers_are_listed_in_a_stable_order ... ok
test tests::an_unknown_key_is_none_rather_than_a_panic ... ok
test tests::listing_by_provider_partitions_the_catalog ... ok
test tests::operation_ids_are_unique_across_the_catalog ... ok

test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.85s

     Running tests/main.rs (target/debug/deps/main-d5244403d8e52e4e)

running 12 tests
test consumer_api::the_lookup_surface_and_every_operation_field_are_reachable ... ok
test consumer_api::the_caller_contract_is_document_data_alone ... ok
test consumer_api::the_inbound_and_configuration_surfaces_are_reachable ... ok
test pack_table::the_table_covers_the_whole_pack ... ok
test consumer_api::the_closed_vocabularies_match_exhaustively ... ok
test pack_table::every_closed_configuration_set_is_addressable ... ok
test pack_table::grafana_discovery_is_available_as_closed_catalog_data ... ok
test pack_table::every_channel_binding_resolves_its_events ... ok
test pack_table::every_credential_carries_a_leaf_and_a_placement ... ok
test pack_table::the_derived_operation_facts_agree_with_the_documents ... ok
test consumer_api::the_reader_reexport_serves_the_same_catalogue ... ok
test pack_table::every_operation_is_reachable_by_key_exactly_once ... ok

test result: ok. 12 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.84s

     Running unittests src/lib.rs (target/debug/deps/catalog_build-1d7e1eea40e5e3a5)

running 61 tests
test artifact::tests::fixtures_live_in_the_build_directory_and_never_repeat_a_path ... ok
test contract::tests::an_operation_with_no_parameters_states_the_empty_object ... ok
test contract::tests::arrays_carry_their_item_type_and_the_fallback_reaches_inside ... ok
test contract::tests::a_const_pinned_body_field_reserves_the_symbol_a_later_field_must_shift_past ... ok
test contract::tests::scalars_map_across_and_constraints_are_dropped ... ok
test contract::tests::shapes_the_contract_cannot_express_fall_back_to_the_top_type ... ok
test contract::tests::the_description_extends_with_the_error_envelope ... ok
test diff::tests::a_wholly_different_file_shows_both_sides ... ok
test diff::tests::an_appended_line_is_a_single_addition ... ok
test diff::tests::a_new_file_is_all_additions ... ok
test diff::tests::a_removed_line_is_a_single_removal ... ok
test diff::tests::an_unchanged_file_produces_only_context ... ok
test diff::tests::a_replaced_middle_keeps_its_context ... ok
test document::tests::an_approved_origin_may_be_the_whole_base_url ... ok
test document::tests::a_declared_graph_is_refused_rather_than_dropped ... ok
test document::tests::a_declared_registration_value_is_a_build_error ... ok
test document::tests::a_braced_constant_header_is_refused ... ok
test inbound::tests::an_unset_webhook_never_launders_itself_as_connection_authenticated ... ok
test inbound::tests::the_tri_state_publishes_as_three_distinct_kinds ... ok
test inbound::tests::only_a_deliberately_unverifiable_surface_reads_as_unverified ... ok
test document::tests::an_unclassifiable_brace_literal_is_a_build_error_never_a_degraded_document ... ok
test net::tests::a_denied_checkpoint_fails_and_is_counted ... ok
test pack::tests::a_nested_operations_key_is_not_the_array ... ok
test pack::tests::a_duplicate_operation_id_is_refused_by_name ... ok
test pack::tests::spans_slice_each_operation_record_exactly ... ok
test pack::tests::disagreeing_schema_versions_are_refused ... ok
test pack::tests::the_compiled_pack_round_trips_its_own_spans ... ok
test scaffold::tests::a_prefix_matches_whole_segments ... ok
test scaffold::tests::a_select_argument_drops_fields_from_the_right ... ok
test scaffold::tests::a_select_argument_refuses_what_it_cannot_mean ... ok
test scaffold::tests::a_service_name_is_cut_at_the_pull_date ... ok
test seam::tests::a_hand_authored_definition_loads_into_the_ir ... ok
test seam::tests::a_pin_naming_an_absent_document_is_refused_and_lists_the_cache ... ok
test seam::tests::a_spec_backed_provider_ingests_its_operations ... ok
test seam::tests::a_spec_backed_provider_with_no_patch_publishes_no_operations ... ok
test seam::tests::an_empty_definition_is_rejected ... ok
test seam::tests::an_unknown_service_is_an_error_that_names_what_exists ... ok
test seam::tests::selecting_a_service_carries_no_other_services_config_graphs_or_verify ... ok
test document::tests::a_custody_only_connector_renders_a_document_with_no_surface ... ok
test document::tests::an_ordinary_connector_does_not_carry_the_flag ... ok
test seam::tests::selecting_a_service_drops_every_other_operation ... ok
test document::tests::a_form_body_is_spelled_structurally ... ok
test seam::tests::the_loaders_own_diagnosis_survives ... ok
test document::tests::rendering_is_deterministic_and_validates ... ok
test seam::tests::the_pinned_document_is_the_one_ingested_not_the_last_in_the_cache ... ok
test seam::tests::selecting_a_service_keeps_the_connector_level_surfaces ... ok
test artifact::tests::a_fixture_does_not_survive_a_panicking_test ... ok
test artifact::tests::read_if_exists_distinguishes_absent_from_present ... ok
test artifact::tests::write_atomic_creates_missing_directories ... ok
test pipeline::tests::a_refusal_is_the_first_one_in_provider_order_at_every_width ... ok
test artifact::tests::write_atomic_replaces_and_leaves_no_temporary ... ok
test pipeline::tests::the_plan_is_identical_at_every_compile_width ... ok
test check::tests::a_wrong_lock_artifact_hash_is_named_as_lock_row_drift ... ok
test check::tests::a_mutated_artifact_is_named_as_artifact_drift ... ok
test check::tests::a_revendored_spec_is_named_as_spec_drift ... ok
test check::tests::a_comment_only_provider_edit_is_named_as_declaration_drift ... ok
test check::tests::a_clean_tree_reports_the_provider_and_artifact_counts_without_writing ... ok
test check::tests::spec_coverage_is_checked_in_both_directions ... ok
test check::tests::provider_coverage_is_checked_in_both_directions ... ok
test check::tests::artifact_coverage_is_checked_in_both_directions ... ok
test check::tests::unsafe_and_symlinked_lock_paths_are_refused_without_reading_the_target ... ok

test result: ok. 61 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.10s

     Running tests/main.rs (target/debug/deps/main-804ecdf12074747a)

running 73 tests
test architecture_fence::reusable_client_has_no_runtime_or_backend_ownership ... ok
test architecture_fence::product_cli_is_a_thin_frontend ... ok
test catalog_invariants::substrate_axis_projection_is_pinned_total_and_non_mechanical ... ok
test catalog_invariants::a_session_signal_reaches_a_backend_only_through_the_admission_seam ... ok
test architecture_fence::every_runtime_isolation_boundary_is_explicit_and_locked ... ok
test architecture_fence::service_owns_the_backend_port_and_server_only_adapts_transport ... ok
test catalog_invariants::repository_authored_anthropic_sources_reproduce_only_the_api_connector ... ok
test architecture_fence::runtime_is_the_only_adapter_composition_root ... ok
test catalog_invariants::no_effect_backend_is_reachable_without_an_admission_proof ... ok
test catalog_invariants::the_mcp_transport_reaches_a_backend_only_through_the_decided_seams ... ok
test architecture_fence::production_modules_obey_the_named_size_fence ... ok
test catalog_invariants::a_full_build_leaves_no_orphaned_artifact ... ok
test dependency_fence::a_compiler_crate_cannot_reach_a_network_crate ... ok
test dependency_fence::every_workspace_member_is_classified ... ok
test dependency_fence::outbound_mcp_foundation_is_exactly_pinned ... ok
test dependency_fence::released_http_integrations_cannot_bypass_connection_bound_egress ... ok
test dependency_fence::the_build_path_does_not_depend_on_the_secret_store ... ok
test dependency_fence::the_connectors_binary_is_an_isolated_locked_composition_leaf ... ok
test dependency_fence::the_gate_and_the_release_workflow_state_the_gates_own_workspace_count ... ok
test dependency_fence::the_rtvbp_runtime_dependency_is_isolated_from_the_canonical_workspace ... ok
test catalog_invariants::the_committed_tree_is_a_fixed_point_of_a_build ... ok
test dependency_fence::the_voice_runtime_is_the_only_production_composition_leaf ... ok
test dependency_fence::the_walk_finds_an_edge_that_is_not_direct ... ok
test engine_free::no_manifest_in_this_workspace_requires_an_engine_crate ... ok
test dependency_fence::the_sipx_network_dependency_is_exactly_pinned_and_isolated ... ok
test engine_free::the_lockfile_names_no_engine_crate_at_all ... ok
test engine_free::the_walk_finds_an_engine_two_edges_away_and_stops_at_a_severed_one ... ok
test ess_citation_fence::every_citation_of_the_specification_resolves ... ok
test ess_citation_fence::every_connect_session_state_write_is_cited ... ok
test ess_citation_fence::every_declared_error_and_event_is_cited_or_unmapped ... ok
test ess_citation_fence::every_published_event_is_an_event_some_domain_declares ... ok
test ess_citation_fence::field_citations_span_the_struct_they_name ... ok
test ess_citation_fence::the_channel_summary_that_carries_a_connection_ref_is_cited_and_modelled ... ok
test ess_citation_fence::the_reobserve_site_leaves_a_connection_ref_and_the_specification_says_so ... ok
test ess_claim_fence::deleting_materializes_declared_refusal_outcomes_is_refused ... ok
test ess_claim_fence::deleting_session_terminates_declared_refusal_outcomes_is_refused ... ok
test catalog_invariants::the_lockfile_agrees_with_every_input_and_every_artifact ... ok
test ess_claim_fence::every_declared_wire_name_is_a_method_the_protocol_accepts ... ok
test ess_claim_fence::materialize_declares_or_marks_every_refusal_its_cited_function_performs ... FAILED
test ess_claim_fence::session_terminate_declares_or_marks_every_refusal_its_cited_function_performs ... ok
test ess_claim_fence::deleting_the_sentence_that_counts_the_refusal_sites_is_refused ... ok
test ess_claim_fence::the_hosted_registry_never_reaches_the_state_its_marker_says_it_cannot ... ok
test ess_claim_fence::the_hosted_failed_claim_is_refused_from_either_side ... ok
test json_governance::a_json_schema_invalid_against_its_declared_meta_schema_fails ... ok
test json_governance::an_invalid_owned_document_fails_its_schema ... ok
test json_governance::an_unclassified_json_file_fails_by_name ... ok
test json_governance::malformed_vendored_json_fails_even_though_it_is_syntax_only ... ok
test catalog_invariants::the_credential_requirement_agrees_with_the_auth_list ... ok
test msrv_fence::the_running_toolchain_meets_the_declared_msrv ... ok
test msrv_fence::the_walk_finds_a_breach_that_is_not_direct ... ok
test msrv_fence::versions_compare_numerically_and_tolerate_both_spellings ... ok
test catalog_invariants::the_contract_and_the_params_state_the_same_symbols ... ok
test catalog_invariants::promoted_operation_traits_equal_the_pre_migration_inventory ... ok
test catalog_invariants::the_browser_surface_is_read_only_and_carries_no_interaction_member ... ok
test no_network::build_records_no_network_attempt ... ok
test catalog_invariants::every_canonical_document_validates_against_the_committed_schema ... ok
test catalog_invariants::gitlab_user_and_automation_connections_are_distinct_and_scope_gated ... ok
test catalog_invariants::no_input_or_artifact_carries_a_credential_shaped_value ... ok
test catalog_invariants::every_format_origin_field_lowers_to_the_origin_slot ... ok
test catalog_invariants::a_custody_only_provider_publishes_no_surface_and_every_other_provider_does ... ok
test catalog_invariants::slack_surface_is_curated_and_credential_scopes_never_cross_purposes ... ok
test no_network::the_network_seam_is_the_only_door ... ok
test no_network::check_records_no_network_attempt ... ok
test engine_free::no_workspace_member_reaches_an_engine_crate ... ok
test msrv_fence::no_resolved_dependency_declares_a_rust_version_above_the_crate_that_reaches_it ... ok
test catalog_invariants::ids_are_unique_in_every_namespace_they_share ... ok
test catalog_invariants::every_operations_required_parameters_are_the_ones_a_caller_must_send ... ok
test catalog_invariants::the_document_carries_the_callers_contract ... ok
test catalog_invariants::sip_catalog_surface_is_the_bounded_platform_dial_member ... ok
test json_governance::every_repository_json_is_classified_and_valid ... ok
test catalog_invariants::the_pack_serves_the_committed_documents_byte_for_byte ... ok
test catalog_invariants::spec_backed_coverage_holds_in_both_directions ... ok
test catalog_invariants::two_plans_over_the_same_inputs_are_byte_identical ... ok

failures:

---- ess_claim_fence::materialize_declares_or_marks_every_refusal_its_cited_function_performs stdout ----

thread 'ess_claim_fence::materialize_declares_or_marks_every_refusal_its_cited_function_performs' (2401868) panicked at crates/catalog-build/tests/main/ess_claim_fence.rs:448:5:
ess/system/domains/connection.yaml declares `connectors.connection.MaterializeObservation` without citing `fn materialize` at crates/integration-monitoring/src/backend.rs:1182, which is the function this fence measures
  crates/integration-monitoring/src/backend.rs:1190 refuses — `return Err(ConnectionError::new(` — and no outcome of `connectors.connection.MaterializeObservation` cites it and no `UNMAPPED:` marker of `connectors.connection.MaterializeObservation` cites it. A refusal the tree performs and the specification neither declares nor marks is a refusal a caller is never told about.
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace


failures:
    ess_claim_fence::materialize_declares_or_marks_every_refusal_its_cited_function_performs

test result: FAILED. 72 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 21.18s

error: test failed, to rerun pass `-p catalog-build --test main`
     Running unittests src/main.rs (target/debug/deps/catalog-669eb5136aee609b)

running 6 tests
test tests::the_declared_surface_is_well_formed ... ok
test tests::a_verb_outside_this_surface_is_refused ... ok
test tests::check_accepts_no_scope ... ok
test tests::the_scope_flags_reach_the_invocation ... ok
test tests::every_verb_maps_to_its_command ... ok
test tests::selection_belongs_to_scaffold_alone ... ok

test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests/offline_binary.rs (target/debug/deps/offline_binary-7dea66df47ec0922)

running 1 test
test build_succeeds_with_networking_unavailable ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.04s

     Running unittests src/lib.rs (target/debug/deps/catalog_reader-44666dbfed70aba2)

running 2 tests
test sha256::tests::the_published_vectors_agree ... ok
test sha256::tests::the_million_a_vector_agrees ... ok

test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s

     Running tests/main.rs (target/debug/deps/main-787c16ebe6417ef9)

running 13 tests
test pack::an_operation_naming_an_absent_provider_is_refused ... ok
test pack::a_payload_length_disagreement_is_refused ... ok
test pack::additive_growth_is_tolerated ... ok
test pack::a_span_outside_the_payload_is_refused ... ok
test pack::a_newer_schema_version_is_refused_by_name ... ok
test pack::measure_read_costs ... ignored, a measurement for predecessor:docs/designs/catalog-artifact.md, not an assertion
test pack::something_that_is_not_a_pack_is_refused ... ok
test pack::the_vendored_sha256_agrees_with_sha2_across_padding_boundaries ... ok
test pack::a_newer_container_format_is_refused_by_name ... ok
test pack::a_tampered_payload_is_refused_before_any_record ... ok
test pack::load_serves_the_committed_pack ... ok
test pack::the_embedded_pack_serves_the_shipped_catalogue ... ok
test pack::every_embedded_record_agrees_with_its_canonical_document ... ok

test result: ok. 12 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 0.99s

     Running unittests src/lib.rs (target/debug/deps/connector_address-1265e50043973104)

running 10 tests
test credential::tests::a_dotted_credential_leaf_is_refused ... ok
test credential::tests::a_tenant_id_cannot_traverse ... ok
test credential::tests::a_path_that_is_not_ours_is_refused_rather_than_guessed_at ... ok
test credential::tests::a_path_renders_and_parses_back ... ok
test credential::tests::a_realistic_tenant_id_is_admitted ... ok
test credential::tests::an_instanced_path_renders_and_parses_back ... ok
test credential::tests::an_over_long_tenant_id_is_refused ... ok
test credential::tests::every_other_component_is_validated_too ... ok
test credential::tests::the_default_service_is_elided_and_still_round_trips ... ok
test credential::tests::which_instance_an_address_carries_is_never_guessed ... ok

test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests/main.rs (target/debug/deps/main-823e58da077a00ec)

running 6 tests
test origin::equivalent_spellings_are_one_value_and_a_different_origin_is_not ... ok
test origin::the_components_agree_with_the_canonical_text ... ok
test origin::the_corpus_covers_every_refusal_class_and_both_declaration_answers ... ok
test origin::every_corpus_case_parses_to_its_recorded_outcome ... ok
test origin::no_refusal_and_no_debug_rendering_reproduces_the_supplied_text ... ok
test origin::normalization_is_idempotent_and_canonical_declarations_round_trip ... ok

test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running unittests src/lib.rs (target/debug/deps/connector_oauth-9f92ae8f31968df6)

running 25 tests
test pkce::tests::an_origin_carrying_a_query_or_fragment_contributes_neither ... ok
test pkce::tests::authorize_url_percent_encodes_the_redirect_and_appends_extras_in_order ... ok
test pkce::tests::a_random_token_is_unpadded_url_safe_and_the_requested_width ... ok
test pkce::tests::an_authorize_url_carries_the_pkce_pair_only_for_a_public_client ... ok
test pkce::tests::the_pkce_challenge_is_the_s256_of_the_verifier ... ok
test pkce::tests::two_random_tokens_differ ... ok
test pkce::tests::authorize_url_refuses_a_url_that_cannot_carry_a_path ... ok
test state::tests::a_state_is_redeemable_once ... ok
test state::tests::contains_any_claims_an_expired_state_so_the_callback_is_refused_not_lost ... ok
test state::tests::clear_drops_live_entries_too ... ok
test state::tests::an_expired_state_is_not_redeemable_and_does_not_linger ... ok
test state::tests::expire_drops_only_what_is_past ... ok
test state::tests::contains_reports_liveness_without_redeeming ... ok
test state::tests::insert_refuses_once_the_table_is_full_of_live_entries ... ok
test state::tests::insert_sweeps_expired_entries_before_refusing ... ok
test state::tests::remove_returns_an_expired_entry_so_a_caller_can_refuse_rather_than_not_find ... ok
test state::tests::replacing_a_live_state_does_not_count_against_capacity ... ok
test token::tests::a_conforming_response_validates_and_drops_unretained_scopes ... ok
test token::tests::a_required_scope_must_survive_the_retain_filter ... ok
test token::tests::a_zero_expires_in_passes_only_where_the_caller_does_not_rely_on_it ... ok
test token::tests::comma_separated_scopes_are_split_and_trimmed ... ok
test token::tests::expiry_is_measured_from_our_clock ... ok
test token::tests::every_gitlab_condition_refuses ... ok
test token::tests::jira_tolerates_an_absent_created_at_and_an_absent_refresh_on_rotation ... ok
test token::tests::refresh_is_due_inside_the_skew_and_whenever_the_clock_is_unavailable ... ok

test result: ok. 25 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running unittests src/lib.rs (target/debug/deps/connector_resolve-ee5a5c844fbe7732)

running 48 tests
test auth::tests::a_basic_join_composes_the_pair_the_vendor_expects ... ok
test auth::tests::a_query_placement_registers_the_encoded_form_it_sends ... ok
test auth::tests::a_prefixed_header_carries_the_bare_value_inside_it ... ok
test auth::tests::a_header_the_template_already_sets_is_refused_rather_than_overwritten ... ok
test auth::tests::an_inbound_signing_secret_never_leaves ... ok
test auth::tests::base64_matches_rfc_4648s_own_vectors ... ok
test config::tests::a_config_value_debug_carries_no_value ... ok
test config::tests::a_username_prefix_is_the_one_reserved_qualifier ... ok
test auth::tests::the_assembled_debug_prints_no_value ... ok
test document::tests::a_pre_c552_document_without_symbols_falls_back_to_the_allocation ... ok
test document::tests::a_sip_session_driver_survives_the_canonical_document ... ok
test document::tests::a_stated_symbol_is_honored_over_the_naive_allocation ... ok
test plan::tests::a_plans_debug_carries_no_credential ... ok
test plan::tests::sensitive_text_never_prints ... ok
test request::tests::the_default_identity_names_this_software_and_this_repository ... ok
test request::tests::a_duplicate_query_key_is_refused_rather_than_sent_twice ... ok
test request::tests::the_params_omit_what_is_absent ... ok
test request::tests::a_fragment_survives_the_appended_query ... ok
test request::tests::debug_prints_shape_and_never_a_value ... ok
test document::tests::the_symbol_allocation_reproduces_the_emitters ... ok
test document::tests::the_emitters_own_symbols_are_reserved ... ok
test slot::tests::a_host_rule_refuses_what_moves_the_authority ... ok
test slot::tests::a_document_position_maps_onto_a_slot_and_anything_else_fails_closed ... ok
test slot::tests::an_unplaced_value_is_held_to_every_rule_including_the_hosts ... ok
test template::tests::a_doubled_brace_is_an_escape_and_an_unterminated_one_is_text ... ok
test slot::tests::a_value_that_moves_the_authority_is_refused_in_context ... ok
test template::tests::an_unfilled_placeholder_stays_verbatim ... ok
test template::tests::markers_are_located_and_filled_by_offset ... ok
test template::tests::truthiness_is_flux_langs ... ok
test template::tests::value_to_text_is_flux_langs ... ok
test document::tests::a_shipped_document_parses_into_its_services_and_operations ... ok
test document::tests::an_unknown_provider_or_operation_is_absent_rather_than_a_panic ... ok
test resolve::tests::a_caller_value_spelling_a_configuration_variable_does_not_reach_the_wire ... ok
test resolve::tests::a_null_query_field_is_omitted_rather_than_sent_empty ... ok
test resolve::tests::an_omitted_optional_body_field_is_not_a_missing_parameter ... ok
test resolve::tests::a_configuration_value_that_moves_the_authority_is_refused ... ok
test resolve::tests::a_caller_parameter_that_leaves_its_path_segment_is_refused ... ok
test resolve::tests::an_omitted_required_parameter_is_refused_and_names_itself ... ok
test resolve::tests::an_optional_query_filter_may_simply_be_left_out ... ok
test resolve::tests::gitlab_publication_keeps_the_reviewed_actions_in_one_json_request ... ok
test resolve::tests::the_plan_carries_the_placed_credential_and_the_set_to_redact ... ok
test resolve::tests::every_request_carries_this_softwares_identity ... ok
test document::tests::an_operation_resolves_through_the_embedded_pack_without_naming_its_provider ... ok
test resolve::tests::the_request_is_the_documents_request ... ok
test credentials::tests::an_unstored_credential_refuses_and_names_the_alternatives ... ok
test credentials::tests::a_basic_join_reads_its_user_half_from_the_config_port ... ok
test credentials::tests::a_bearer_credential_assembles_and_lists_its_redaction ... ok
test credentials::tests::a_basic_join_with_no_user_half_refuses_by_name ... ok

test result: ok. 48 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.83s

     Running unittests src/lib.rs (target/debug/deps/connector_secrets-303c889b2e34bd3a)

running 49 tests
test file::prepared::tests::retired_prepared_and_terminal_records_are_refused_by_parser_and_encoder ... ok
test file::tests::a_foreign_owned_directory_is_refused_without_repair ... ignored, requires euid 0 to plant a foreign-owned directory; CI invokes it explicitly with sudo
test file::tests::a_foreign_owned_store_is_refused_without_repair ... ignored, requires euid 0 to plant a foreign-owned file; CI invokes it explicitly with sudo
test file::tests::a_fifo_swap_is_refused_without_blocking_or_reading ... ok
test file::tests::a_shared_parent_refusal_recommends_an_owner_only_child_not_chmodding_the_parent ... ok
test file::portable_tests::bounded_reads_use_metadata_and_a_same_handle_max_plus_one_read ... ok
test file::portable_tests::bounded_writes_refuse_before_allocating_or_changing_the_prior_file ... ok
test file::tests::a_file_from_another_version_is_refused ... ok
test file::tests::a_directory_symlink_is_refused_without_changing_it ... ok
test file::tests::hex_round_trips_every_byte ... ok
test file::tests::a_fresh_store_is_0600_inside_a_0700_directory ... ok
test file::tests::a_write_failure_names_no_value ... ok
test file::tests::a_temporary_left_by_a_crash_is_reaped_on_the_next_open ... ok
test file::tests::the_write_path_keeps_all_three_legs_of_its_atomicity ... ok
test file::tests::wrong_object_kinds_are_refused_without_repair ... ok
test file::transaction_crash_tests::lease_child ... ok
test file::transaction_crash_tests::legacy_writer_child ... ok
test file::tests::a_world_readable_directory_is_refused ... ok
test file::tests::a_parse_failure_names_the_line_and_never_the_value ... ok
test file::transaction_crash_tests::prepared_crash_child ... ok
test keyring::tests::a_credential_round_trips_through_the_real_keyring ... ignored, requires a running freedesktop Secret Service and secret-tool
test file::tests::a_world_readable_store_is_refused ... ok
test file::tests::a_store_symlink_is_refused_without_reading_or_changing_either_object ... ok
test keyring::tests::addresses_differing_only_in_credential_are_different_keys ... ok
test keyring::tests::every_entry_carries_the_component_wide_service_attribute ... ok
test keyring::tests::an_address_without_an_instance_still_carries_the_attribute ... ok
test memory::tests::a_value_round_trips_and_then_is_gone ... ok
test secret::tests::debug_leaks_neither_the_value_nor_its_length ... ok
test secret::tests::debug_still_does_not_leak_when_nested ... ok
test secret::tests::equality_holds_for_the_ordinary_cases ... ok
test secret::tests::the_value_is_readable_through_one_conspicuous_call ... ok
test secret::tests::every_exit_from_the_wrapper_is_named_expose_secret ... ok
test file::tests::an_oversized_store_is_refused_before_its_contents_are_parsed ... ok
test file::tests::an_unopenable_store_is_refused_without_reading_or_repair ... ok
test file::portable_tests::logical_v1_reads_bound_entries_and_individual_values ... ok
test file::tests::debug_carries_neither_a_value_nor_an_address ... ok
test file::transaction_crash_tests::two_children_prove_lease_refusal_and_abrupt_release ... ok
test file::tests::the_store_is_object_safe ... ok
test file::tests::a_value_survives_the_store_being_dropped_and_reopened ... ok
test file::tests::an_atomic_move_survives_reopen_as_one_state ... ok
test file::portable_tests::an_injected_write_failure_preserves_the_prior_file_and_batch ... ok
test file::portable_tests::a_multi_credential_connection_migration_survives_restart ... ok
test file::transaction_crash_tests::native_upgrade_fixture_proves_legacy_quiescence_and_v2_refusal ... ok
test file::tests::a_damaged_file_is_refused_rather_than_partly_loaded ... ok
test file::tests::mutations_revalidate_widened_directory_and_file_before_writing_a_temporary ... ok
test file::tests::nothing_collides_across_tenants_or_across_services_of_one_vendor ... ok
test file::tests::a_write_leaves_no_temporary_and_no_truncated_file ... ok
test file::tests::a_concurrent_reader_never_sees_a_half_written_store ... ok
test file::transaction_crash_tests::every_durable_transaction_boundary_recovers_one_complete_state ... ok

test result: ok. 46 passed; 0 failed; 3 ignored; 0 measured; 0 filtered out; finished in 6.85s

     Running tests/main.rs (target/debug/deps/main-30436b0718aa4196)

running 25 tests
test layout_composition::a_custom_layout_cannot_widen_what_an_address_may_contain ... ok
test prepared_transactions::abort_before_prepare_fences_delayed_work_and_reclaim_retires_the_generation ... ok
test file_store_portable::file_store_is_unconditionally_public ... ok
test layout_composition::a_layout_refusal_reaches_the_caller_as_a_layout_error ... ok
test layout_composition::a_non_default_layout_changes_the_path_and_nothing_else ... ok
test layout_composition::the_layout_contract_still_applies_to_a_custom_one ... ok
test prepared_transactions::every_acknowledged_cross_id_abort_survives_a_later_staged_commit ... ok
test prepared_transactions::exhaustive_replay_and_winner_table_is_value_free ... ok
test prepared_transactions::concurrent_commit_and_abort_have_exactly_one_terminal_winner ... ok
test prepared_transactions::prepared_reservation_blocks_every_ordinary_mutation ... ok
test prepared_transactions::payload_free_state_and_error_renderings_are_fixed ... ok
test prepared_transactions::prepared_store_is_object_safe_and_keeps_the_candidate_invisible_until_commit ... ok
test prepared_transactions::protocol_types_are_fixed_width_nonzero_and_opaque ... ok
test scoped_batch::inventory_is_scoped_and_contains_no_values ... ok
test scoped_batch::a_batch_cannot_cross_its_scope ... ok
test scoped_batch::a_checked_batch_is_all_or_nothing ... ok
test prepared_transactions::terminal_capacity_refuses_without_eviction_until_owner_acknowledgement ... ok
test prepared_transactions::unix_lease_metadata_is_owner_only_one_link_and_never_repaired ... ok
test prepared_transactions::file_store_holds_a_lifetime_non_blocking_lease ... ok
test prepared_transactions::file_store_abort_tombstone_and_retirement_survive_reopen ... ok
test prepared_transactions::public_renderings_and_non_credential_files_hide_encoded_sentinels ... ok
test prepared_transactions::v1_stays_byte_identical_until_the_first_transaction_use ... ok
test prepared_transactions::v2_live_and_stage_bytes_are_canonical_and_fixture_pinned ... ok
test prepared_transactions::file_store_reopens_a_prepared_candidate_and_commits_the_complete_v2_image ... ok
test prepared_transactions::file_store_concurrent_cross_id_calls_refuse_without_erasing_prior_tombstones ... ok

test result: ok. 25 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.08s

     Running unittests src/lib.rs (target/debug/deps/connector_spec-bdfbef9affc503e5)

running 28 tests
test config::tests::a_host_value_is_refused_for_what_no_request_position_would_catch ... ok
test config::tests::a_multi_destination_field_carries_one_slot_into_every_pin ... ok
test config::tests::a_pinned_request_value_parses_and_is_connection_level_configuration ... ok
test config::tests::a_username_head_qualifies_the_placeholder_of_its_request_pin ... ok
test config::tests::a_pinned_value_cannot_reshape_the_request_it_lands_in ... ok
test config::tests::every_template_variable_is_reported_not_only_the_first ... ok
test config::tests::formats_validate_the_values_they_claim ... ok
test config::tests::bindings_parse_and_carry_their_level_and_secrecy ... ok
test graph::tests::a_cycle_has_no_order_at_all ... ok
test graph::tests::a_diamond_converges_because_data_edges_need_no_nesting ... ok
test graph::tests::comparisons_map_to_flux_operators ... ok
test graph::tests::a_chain_orders_topologically ... ok
test graph::tests::region_and_boundary_kinds_are_classified ... ok
test graph::tests::enclosing_walks_outwards_and_detects_a_containment_cycle ... ok
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
test names::tests::two_names_that_normalize_alike_stay_distinct ... ok
test names::tests::the_reserved_symbols_are_not_handed_out ... ok

test result: ok. 28 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests/main.rs (target/debug/deps/main-fc16e5fd0767bbe5)

running 503 tests
test auth_hazard::a_credential_declaring_no_hazard_carries_none ... ok
test auth_archetypes::the_operator_level_is_expressible ... ok
test auth_hazard::a_grant_list_without_the_password_grant_needs_no_hazard ... ok
test auth_hazard::the_near_miss_hazard_spelling_is_refused_naming_the_value ... ok
test auth_archetypes::a_public_client_is_exempt_from_the_client_secret_a_confidential_one_owes ... ok
test auth_hazard::a_password_grant_that_declares_no_hazard_is_refused ... ok
test auth_hazard::the_declared_hazard_is_the_word_the_consuming_deployment_gate_reads ... ok
test auth_prefix::a_header_placement_carries_an_arbitrary_scheme_word ... ok
test auth_prefix::a_declared_prefix_round_trips_through_the_encoding ... ok
test auth_prefix::a_prefix_may_carry_punctuation_and_still_be_a_prefix ... ok
test auth_prefix::a_scheme_word_that_is_not_oauth2_is_still_just_a_prefix ... ok
test auth_prefix::a_prefix_missing_its_trailing_separator_is_refused ... ok
test auth_prefix::a_prefix_may_not_break_out_of_the_header_value ... ok
test auth_prefix::repeated_punctuation_is_the_vendors_business_and_still_loads ... ok
test auth_prefix::a_prefix_may_not_end_in_an_alphanumeric_character ... ok
test auth_prefix::a_whitespace_only_prefix_is_refused ... ok
test auth_prefix::a_prefix_may_not_carry_leading_or_doubled_whitespace ... ok
test auth_prefix::a_prefix_may_not_spell_a_resolution_marker ... ok
test auth_prefix::an_omitted_prefix_is_empty_and_does_not_serialize ... ok
test auth_prefix::there_is_no_suffix_axis ... ok
test auth_prefix::a_prefix_may_not_name_the_credential_or_its_env_var ... ok
test auth_prefix::the_preset_schemes_carry_no_prefix_of_their_own ... ok
test auth_workarounds::a_token_endpoint_workaround_without_a_grant_is_refused ... ok
test auth_workarounds::a_workaround_measured_on_a_non_date_is_refused ... ok
test auth_workarounds::a_workaround_without_attribution_is_refused ... ok
test auth_workarounds::two_workarounds_for_one_grant_are_refused ... ok
test auth_workarounds::a_workaround_does_not_reach_a_sibling_credential_in_the_same_connector ... ok
test channel_bindings::a_binding_carrying_an_undeclared_event_is_refused ... ok
test channel_bindings::a_cursor_on_a_transport_that_is_not_polled_is_refused ... ok
test channel_bindings::a_complete_binding_loads_and_composes_an_event_with_a_reply ... ok
test channel_bindings::a_channel_and_an_operation_may_not_share_a_name ... ok
test channel_bindings::a_duplicate_operation_id_is_reported_once_and_not_also_as_a_namespace_collision ... ok
test channel_bindings::a_parameter_cannot_be_both_bound_and_the_journey_result ... ok
test channel_bindings::a_payload_key_that_is_not_a_flux_symbol_is_refused ... ok
test channel_bindings::a_payload_source_path_with_an_empty_segment_is_refused ... ok
test channel_bindings::a_generic_socket_round_trips_every_connect_event_payload_and_config_fact ... ok
test channel_bindings::a_reply_binding_a_parameter_the_operation_does_not_declare_is_refused ... ok
test channel_bindings::a_poll_binding_without_a_cursor_is_refused ... ok
test channel_bindings::a_push_binding_carrying_no_events_is_refused ... ok
test channel_bindings::a_poll_binding_with_a_cursor_loads_and_may_omit_its_events ... ok
test channel_bindings::a_reply_binding_from_a_symbol_the_payload_never_produces_is_refused ... ok
test channel_bindings::a_reply_leaving_a_required_parameter_unbound_is_refused ... ok
test channel_bindings::a_reply_naming_an_operation_nobody_declares_is_refused ... ok
test channel_bindings::a_session_binding_loads_as_non_event_ingress_with_closed_admission_facts ... ok
test channel_bindings::a_reply_with_no_result_leaves_the_journey_output_parameter_unbound ... ok
test channel_bindings::a_signed_template_covering_only_the_url_is_refused ... ok
test channel_bindings::a_signed_template_that_never_interpolates_the_body_is_refused ... ok
test channel_bindings::a_socket_binding_may_declare_manual_vendor_side_setup ... ok
test channel_bindings::a_signed_template_the_host_cannot_fill_is_refused ... ok
test channel_bindings::a_timestamped_scheme_without_a_tolerance_is_refused ... ok
test channel_bindings::a_timestamped_hmac_scheme_loads_with_its_window_and_its_selector ... ok
test channel_bindings::a_tolerance_that_is_not_a_duration_is_refused ... ok
test channel_bindings::a_timestamped_scheme_without_a_timestamp_selector_is_refused ... ok
test channel_bindings::a_verification_timestamp_read_from_the_body_is_refused ... ok
test channel_bindings::a_timestamp_format_loads_beside_its_selector ... ok
test channel_bindings::a_tolerance_too_large_to_scale_is_refused_by_the_loader ... ok
test channel_bindings::a_webhook_binding_may_declare_itself_unverifiable_deliberately ... ok
test channel_bindings::a_webhook_binding_that_states_no_verification_is_refused ... ok
test channel_bindings::a_webhook_secret_that_no_credential_declares_is_refused ... ok
test channel_bindings::an_event_and_an_operation_may_not_share_a_name ... ok
test channel_bindings::a_webhook_secret_declared_as_an_outbound_credential_is_refused ... ok
test channel_bindings::an_event_name_may_carry_the_vendors_own_dots_and_underscores ... ok
test channel_bindings::an_event_name_that_could_not_travel_in_an_address_is_refused ... ok
test channel_bindings::an_operation_cannot_authenticate_with_a_signing_credential ... ok
test channel_bindings::every_member_kind_addresses_and_round_trips_through_one_oip_form ... ok
test channel_bindings::verification_on_a_transport_that_cannot_use_it_is_refused ... ok
test channel_bindings::twilios_url_and_sorted_form_scheme_is_declarable ... ok
test channel_bindings::session_ingress_refuses_missing_facts_and_event_only_fields ... ok
test config_choices::a_secret_field_cannot_declare_a_closed_set ... ok
test auth_archetypes::raw_value_header_renders_the_same_form_as_a_bearer ... ok
test config_choices::a_pinned_field_checks_every_choice_against_its_request_position ... ok
test config_choices::a_config_field_declares_a_closed_set_of_values_and_a_value_outside_it_is_refused ... ok
test config_choices::every_permitted_value_still_satisfies_the_fields_format ... ok
test config_choices::a_choice_must_be_renderable_and_distinct ... ok
test config_choices::an_example_outside_the_closed_set_is_refused ... ok
test config_choices::a_set_with_one_value_is_refused_and_an_empty_one_is_an_open_field ... ok
test config_fields::a_binding_that_is_not_a_binding_at_all_is_refused ... ok
test config_fields::a_complete_configuration_surface_loads_and_derives_its_levels ... ok
test config_fields::a_connector_with_no_configuration_at_all_is_refused_when_it_needs_some ... ok
test config_fields::a_connector_with_a_literal_base_url_needs_no_endpoint_field ... ok
test config_fields::a_config_field_can_pin_a_path_segment ... ok
test config_fields::a_credential_field_that_claims_not_to_be_secret_is_refused ... ok
test config_fields::a_destination_named_twice_is_refused ... ok
test config_fields::a_field_binding_a_credential_nobody_declares_is_refused ... ok
test config_fields::a_field_binding_a_template_variable_that_does_not_exist_is_refused ... ok
test config_fields::a_credential_cannot_also_reach_a_request_position ... ok
test config_fields::a_field_without_a_label_is_refused ... ok
test config_fields::a_field_without_help_is_refused ... ok
test config_fields::a_header_pin_on_an_auth_owned_header_is_refused ... ok
test config_fields::a_non_credential_field_that_claims_to_be_secret_is_refused ... ok
test config_fields::a_non_secret_field_may_still_declare_an_example ... ok
test channel_bindings::socket_connect_declarations_fail_closed_at_load ... ok
test config_fields::a_path_pin_no_operation_carries_is_refused ... ok
test config_fields::a_pin_parses_to_its_position_and_derives_its_level_and_secrecy ... ok
test config_fields::a_pin_that_claims_to_be_secret_is_refused ... ok
test config_fields::a_pinned_query_parameter_that_is_also_an_argument_is_refused ... ok
test config_fields::a_query_parameter_and_a_header_can_be_pinned_too ... ok
test config_fields::a_further_destination_that_is_not_a_request_position_is_refused ... ok
test config_fields::a_path_pin_whose_example_escapes_its_segment_is_refused ... ok
test config_fields::a_secret_field_that_declares_an_example_is_refused ... ok
test auth_archetypes::basic_join_without_a_marker_is_a_distinct_form ... ok
test config_fields::a_template_variable_nothing_binds_is_refused ... ok
test config_fields::a_value_that_composes_a_host_is_refused_when_it_could_move_the_authority ... ok
test config_fields::a_username_field_is_not_secret ... ok
test config_fields::a_username_field_on_a_non_basic_credential_is_refused ... ok
test config_fields::a_value_that_is_both_pinned_and_declared_as_a_parameter_is_refused ... ok
test config_fields::a_verify_operation_loads_and_resolves ... ok
test config_fields::a_verify_operation_that_does_not_exist_is_refused ... ok
test config_fields::a_verify_operation_that_writes_is_refused ... ok
test config_fields::an_example_that_fails_its_own_format_is_refused ... ok
test config_fields::an_optional_pin_is_refused ... ok
test config_fields::every_permitted_choice_is_checked_against_every_destination ... ok
test config_fields::config_names_join_the_shared_member_namespace ... ok
test config_fields::an_oauth_field_without_an_oauth_credential_is_refused ... ok
test config_fields::an_example_is_checked_against_every_destination_and_not_only_the_first ... ok
test config_fields::one_field_declares_two_destinations_and_one_value_reaches_both ... ok
test auth_archetypes::a_connector_with_no_credential_still_has_a_form ... ok
test config_fields::two_fields_that_would_share_one_placeholder_are_refused ... ok
test config_fields::the_username_placeholder_prefix_is_reserved_from_endpoint_fields ... ok
test config_fields::two_fields_writing_one_header_are_refused ... ok
test constant_headers::a_constant_header_must_state_a_value ... ok
test constant_headers::a_constant_header_name_must_be_an_http_field_name ... ok
test constant_headers::a_constant_header_value_may_not_carry_a_line_break ... ok
test constant_headers::a_constant_header_survives_the_ir_round_trip ... ok
test constant_headers::a_provider_level_constant_header_is_distributed_onto_every_operation ... ok
test constant_headers::a_provider_level_refusal_is_reported_once ... ok
test constant_headers::an_operation_level_constant_header_may_not_carry_a_credential_either ... ok
test credential_paths::a_path_from_another_convention_is_refused_rather_than_guessed_at ... ok
test constant_headers::an_operations_own_constant_header_replaces_the_providers ... ok
test constant_headers::an_operation_without_constant_headers_encodes_as_it_always_did ... ok
test credential_paths::an_explicitly_spelled_default_service_does_not_parse ... ok
test credential_paths::an_instance_that_is_not_a_uuid_is_refused_and_the_refusal_names_the_component ... ok
test constant_headers::a_constant_header_may_not_carry_a_credential ... ok
test credential_paths::an_instanced_path_cannot_be_confused_with_a_service ... ok
test config_fields::the_atlassian_connectors_address_the_cloud_gateway_by_id ... ok
test config_choices::the_shipped_connectors_that_have_regions_declare_them ... ok
test config_fields::the_atlassian_connectors_prefer_their_service_account ... ok
test credential_paths::the_default_service_is_elided_and_the_elision_stays_unambiguous ... ok
test credential_paths::the_tenant_validator_is_public_so_a_host_can_check_before_it_builds ... ok
test auth_workarounds::a_workaround_declared_on_one_connectors_auth_surface_does_not_reach_another ... ok
test credential_paths::every_admissible_reference_round_trips_and_no_rejected_one_renders ... ok
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
test auth_archetypes::slack_has_distinct_bot_install_and_delegated_user_oauth_flows ... ok
test determinism::requirement_encoding_ignores_authoring_order ... ok
test determinism::serde_json_object_keys_stay_sorted ... ok
test discovery::discovery_cannot_name_a_nonexistent_operation ... ok
test discovery::duplicate_vendor_type_mapping_is_refused_instead_of_precedence_ordered ... ok
test determinism::repeated_serialization_is_stable ... ok
test discovery::one_read_can_declare_a_closed_native_provider_mapping ... ok
test execution_facts::a_seeded_write_remains_write_and_carries_write_effects ... ok
test execution_facts::audio_v1_is_a_closed_unary_device_driver ... ok
test execution_facts::effects_are_required_and_never_derived_from_http ... ok
test execution_facts::every_catalog_operation_requires_a_non_empty_description ... ok
test execution_facts::cdp_v1_is_a_closed_leased_session_browser_driver ... ok
test execution_facts::host_and_semantic_effects_remain_independent_axes ... ok
test execution_facts::predecessor_runtime_and_quirks_vocabularies_are_refused_by_name ... ok
test execution_facts::sip_v1_is_a_closed_session_establishment_driver ... ok
test execution_facts::unknown_effect_driver_and_capability_values_are_refused_by_name ... ok
test execution_facts::sql_v1_is_a_closed_unary_database_driver ... ok
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
test channel_bindings::the_shipped_slack_bindings_describe_both_of_slacks_real_transports ... ok
test ir_roundtrip::unset_and_explicit_empty_auth_differ_on_the_wire ... ok
test ir_roundtrip::parameter_and_response_schemas_survive_the_round_trip ... ok
test legacy_default_service::a_default_beside_a_named_service_without_the_legacy_marker_stays_refused ... ok
test legacy_default_service::a_legacy_marker_without_a_named_sibling_is_refused ... ok
test legacy_default_service::a_named_service_cannot_claim_the_legacy_default_marker ... ok
test legacy_default_service::an_explicit_legacy_default_can_coexist_with_a_named_service ... ok
test legacy_default_service::every_spec_document_of_a_mixed_connector_must_state_its_service ... ok
test legacy_default_service::every_member_of_a_mixed_connector_must_state_its_service ... ok
test lockfile::a_changed_generator_moves_the_artifact_hashes_alone ... ok
test lockfile::a_changed_toml_moves_the_toml_hash_and_nothing_upstream_of_it ... ok
test lockfile::a_changed_spec_moves_the_spec_hash ... ok
test lockfile::a_comment_only_edit_moves_the_toml_hash_alone ... ok
test lockfile::a_lockfile_from_another_format_version_is_refused ... ok
test lockfile::a_rebuild_replaces_a_row_rather_than_duplicating_it ... ok
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
test auth_archetypes::slack_bearers_keep_bot_user_admin_and_app_purposes_separate ... ok
test oauth2_acquisition::a_scope_response_location_cannot_name_credential_material ... ok
test oauth2_acquisition::a_scope_response_location_must_be_a_json_pointer ... ok
test oauth2_acquisition::an_oauth2_credential_loads_with_every_field_intact ... ok
test oauth2_acquisition::declaring_both_an_oauth2_grant_and_a_minting_operation_is_refused ... ok
test oauth_token_endpoint::a_dangling_token_endpoint_is_refused_naming_it ... ok
test oauth_token_endpoint::a_confidential_client_omits_the_discriminator ... ok
test oauth_token_endpoint::a_public_client_loads_and_is_marked_public ... ok
test oauth_token_endpoint::a_two_host_declaration_loads_and_carries_both_services ... ok
test oauth_token_endpoint::an_absent_token_endpoint_is_skipped_entirely ... ok
test openapi_ingest::a_cookie_parameter_skips_the_operation_rather_than_being_dropped_from_it ... ok
test openapi_ingest::a_cyclic_response_ref_is_bounded_rather_than_expanded_forever ... ok
test openapi_ingest::a_document_this_ingest_cannot_read_is_a_whole_document_error ... ok
test openapi_ingest::a_body_the_ir_cannot_express_skips_the_operation_rather_than_dropping_the_body ... ok
test openapi_ingest::a_duplicate_operation_id_is_reported_rather_than_resolved_by_position ... ok
test openapi_ingest::a_method_the_ir_cannot_spell_is_reported_rather_than_dropped_silently ... ok
test openapi_ingest::a_missing_section_is_a_diagnostic_naming_it ... ok
test openapi_ingest::a_path_items_parameters_reach_every_operation_under_it ... ok
test openapi_ingest::a_malformed_endpoint_is_a_diagnostic_naming_it_rather_than_a_failed_ingest ... ok
test openapi_ingest::a_recursive_request_remains_an_exact_contract_and_is_refused ... ok
test openapi_ingest::a_recursive_response_is_bounded_without_dropping_the_operation ... ok
test openapi_ingest::an_external_ref_is_refused_rather_than_followed ... ok
test openapi_ingest::an_object_request_body_becomes_named_body_parameters ... ok
test openapi_ingest::a_yaml_document_ingests_including_its_integer_response_keys ... ok
test openapi_ingest::both_openapi_3_0_and_3_1_documents_ingest ... ok
test openapi_ingest::operation_order_does_not_depend_on_the_documents_key_order ... ok
test openapi_ingest::parameters_land_in_their_request_position_with_their_schemas ... ok
test openapi_ingest::refs_resolve_including_nested_and_repeated_ones ... ok
test openapi_ingest::ingest_is_deterministic ... ok
test openapi_ingest::servers_carry_their_templating_and_their_variables ... ok
test openapi_ingest::the_anthropic_spec_has_authored_provenance ... ok
test lockfile::unchanged_inputs_reproduce_the_lockfile_byte_for_byte ... ok
test credential_paths::slack_derives_a_path_for_each_of_its_credentials ... ok
test grafana::grafana_is_a_private_reachable_read_surface_with_connector_custody ... ok
test grafana::grafana_query_keeps_the_batch_bounded ... ok
test credential_paths::the_three_outcomes_are_distinguishable ... ok
test operation_selection::a_block_overrides_a_selectors_risk ... ok
test operation_selection::a_deferral_reason_must_be_nonempty ... ok
test operation_selection::a_deferred_operation_cannot_also_be_corrected ... ok
test operation_selection::a_bulk_conditional_still_owes_a_condition_per_operation ... ok
test operation_selection::a_pin_naming_an_absent_operation_id_is_refused ... ok
test operation_selection::a_path_prefix_matches_on_segment_boundaries ... ok
test operation_selection::a_per_operation_block_wins_over_a_selector ... ok
test operation_selection::a_selector_that_matches_nothing_is_refused ... ok
test operation_selection::a_selector_states_risk_and_idempotency_for_the_set ... ok
test operation_selection::an_internal_path_is_never_selected ... ok
test operation_selection::an_operation_id_that_cannot_produce_a_legal_name_is_reported ... ok
test operation_selection::an_upstream_operation_id_rename_orphans_direction_and_refuses ... ok
test operation_selection::changing_only_upstream_methods_before_composition_preserves_authored_directions ... ok
test operation_selection::a_pin_overrides_the_rule_and_a_rename_overrides_the_pin ... ok
test operation_selection::description_corrections_preserve_bulk_selection_and_document_order ... ok
test operation_selection::description_corrections_refuse_conflicting_exact_patches ... ok
test operation_selection::description_corrections_refuse_stale_identities_and_empty_values ... ok
test operation_selection::a_selector_matches_by_service_path_prefix_and_method ... ok
test operation_selection::a_selector_states_exposure_for_the_set ... ok
test operation_selection::a_read_may_go_unstated ... ok
test operation_selection::a_spec_backed_provider_with_no_selector_publishes_nothing ... ok
test operation_selection::deferring_an_operation_no_selector_matched_is_refused ... ok
test credential_paths::a_single_instance_address_is_byte_identical_to_the_four_component_form ... ok
test operation_selection::exposure_still_defaults_to_exposed ... ok
test operation_selection::silence_on_an_authored_write_refuses ... ok
test operation_selection::there_is_no_hide_key ... ok
test operation_selection::overlapping_selectors_that_disagree_are_refused ... ok
test operation_selection::overlapping_selectors_that_agree_are_accepted ... ok
test config_fields::zendesk_declares_a_complete_connect_form ... ok
test operation_spec_source::an_inline_operation_cannot_author_the_derived_marker ... ok
test operation_selection::an_exact_deferral_withholds_one_selector_match ... ok
test operator_pinned_origin::a_declared_origin_must_already_be_in_canonical_form ... ok
test operator_pinned_origin::an_open_origin_without_operator_approval_is_refused ... ok
test operator_pinned_origin::an_operator_pinned_origin_is_a_value_free_generic_config_declaration ... ok
test operator_pinned_origin::an_origin_accepts_only_an_absolute_https_origin_without_url_tail ... ok
test operator_pinned_origin::the_loader_accepts_exactly_the_canonical_origins_of_the_shared_corpus ... ok
test operation_selection::the_naming_rule_derives_the_declared_spelling ... ok
test operation_selection::two_operation_ids_deriving_one_op_id_refuse ... ok
test operation_selection::the_derived_id_set_is_pinned ... ok
test credential_paths::several_instances_and_no_uuid_is_a_refusal_naming_what_would_have_worked ... ok
test operation_selection::identical_inputs_produce_identical_ir ... ok
test param_omission::a_correction_is_applied_before_the_omission_that_depends_on_it ... ok
test param_omission::nothing_is_dropped_unless_the_patch_says_so ... ok
test param_omission::omitting_a_parameter_from_the_wrong_position_is_refused ... ok
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
test param_omission::omitting_a_required_parameter_is_refused ... ok
test provider_toml::a_file_may_point_at_a_spec_and_still_declare_operations_inline ... ok
test provider_toml::a_hand_authored_file_produces_a_complete_connector ... ok
test provider_toml::a_spec_pointer_file_produces_the_patch_set ... ok
test provider_toml::authoring_order_inside_a_mechanism_does_not_reach_the_ir ... ok
test provider_toml::the_three_auth_states_survive_the_loader ... ok
test provider_toml::unstated_patch_overrides_stay_distinguishable_from_stated_ones ... ok
test provider_toml::the_provider_file_hash_is_recorded_and_is_a_function_of_the_bytes ... ok
test provider_toml_errors::every_required_rejection_has_a_fixture ... ok
test provider_toml_errors::no_snapshot_is_orphaned ... ok
test repeatability_condition_elision::an_operation_stating_a_condition_does_carry_it_into_the_hash_domain ... ok
test repeatability_condition_elision::an_operation_stating_no_condition_hashes_as_it_did_before_the_field_existed ... ok
test param_omission::omitting_a_parameter_the_document_does_not_declare_is_refused ... ok
test credential_paths::two_instances_of_one_connector_for_one_tenant_render_different_addresses ... ok
test param_omission::the_curated_argument_list_comes_back_when_the_patch_names_what_to_drop ... ok
test param_omission::omitting_a_parameter_changes_only_the_parameters ... ok
test auth_archetypes::basic_join_renders_two_fields_and_hides_the_vendor_marker ... ok
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
test provider_toml_errors::every_rejection_matches_its_golden_snapshot ... ok
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
test config_fields::the_templated_providers_ask_for_their_tenant ... ok
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
test auth_archetypes::and_sets_and_or_alternatives_are_the_grouping_a_form_renders ... ok
test services::omitting_the_service_in_a_multi_service_provider_is_refused ... ok
test services::stating_the_default_service_encodes_exactly_as_omitting_it ... ok
test services::the_service_key_walk_still_finds_an_ir_service_key_at_every_depth ... ok
test shared_endpoint_slot::a_sibling_service_placeholder_is_refused_when_nothing_shares_the_slot ... ok
test shared_endpoint_slot::a_sibling_service_that_does_not_exist_is_refused ... ok
test shared_endpoint_slot::listing_the_head_service_again_is_refused ... ok
test shared_endpoint_slot::one_field_fills_the_placeholder_of_a_sibling_service ... ok
test shared_endpoint_slot::sharing_a_non_endpoint_binding_is_refused ... ok
test service_partition::a_provider_file_that_loads_publishes_only_round_tripping_addresses ... ok
test shipped_providers::babelforce_is_bearer_only_and_never_the_deprecated_header_pair ... ok
test shipped_providers::an_operation_that_takes_nothing_composes_an_empty_object_schema ... ok
test operation_selection::the_canonical_surface_is_selected_and_the_file_stays_reviewable ... ok
test shipped_providers::jira_added_writes_have_exact_authored_source_evidence ... ok
test operation_spec_source::a_patch_selected_operation_retains_its_exact_vendor_source ... ok
test credential_paths::every_shipped_credential_is_prefixed_with_its_connector_id ... ok
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
test shipped_providers::every_operation_composes_an_input_schema_covering_its_parameters ... ok
test service_tags::the_shipped_fleet_uses_several_distinct_tags ... ok
test shipped_providers::no_provider_file_carries_a_credential_value ... ok
test response_schema_coverage::the_recorded_floor_is_the_measured_figure ... ok
test verification_conformance::a_transport_outside_the_matrix_declares_that_it_cannot_verify ... ok
test verification_conformance::comparison_examines_every_byte_wherever_they_differ ... ok
test response_schema_coverage::response_schema_coverage_does_not_fall_below_its_floor ... ok
test verification_conformance::reserializing_the_json_body_breaks_verification ... ok
test verification_conformance::the_declared_timestamp_format_is_read_instead_of_sniffed ... ok
test verification_conformance::the_hmac_primitive_matches_rfc_4231 ... ok
test verification_conformance::the_hmac_sha1_primitive_matches_rfc_2202 ... ok
test verification_conformance::the_reassembled_form_is_a_derivation_of_the_body_and_not_its_bytes ... ok
test shipped_providers::every_shipped_provider_loads ... ok
test services::every_shipped_service_is_spellable_and_a_single_service_provider_declares_none ... ok
test response_schema_coverage::the_recorded_ceiling_is_the_measured_absence ... ok
test response_schema_coverage::a_connector_arriving_with_no_response_shapes_is_caught ... ok
test service_tags::some_shipped_provider_has_services_whose_tags_diverge ... ok
test credential_response::no_withheld_operation_is_in_the_shipped_catalogue ... ok
test response_schema_coverage::no_operation_publishes_a_permissive_response_schema ... ok
test shipped_providers::operation_ids_are_declarable_in_flux ... ok
test credential_paths::every_shipped_provider_declares_an_authority_and_renders_a_credential_path ... ok
test auth_archetypes::every_oauth_connector_generates_the_operator_connection_split ... ok
test config_fields::no_shipped_provider_has_an_unbound_template_variable ... ok
test config_fields::no_shipped_provider_gives_a_secret_field_an_example ... ok
test service_audiences::the_seeded_fleet_has_a_useful_cross_function_vocabulary ... ok
test produces_credential::no_shipped_operation_declares_produces_credential_yet ... ok
test verification_conformance::a_signed_template_that_covers_only_the_url_verifies_a_forged_payload ... ok
test verification_conformance::a_tolerance_no_host_could_apply_does_not_load ... ok
test verification_conformance::a_signed_template_that_omits_the_body_verifies_a_forged_payload ... ok
test verification_conformance::a_signature_outside_its_window_is_refused ... ok
test verification_conformance::vendor_signature_vectors_verify ... ok
test verification_conformance::every_shipped_hmac_scheme_is_covered_by_the_matrix ... ok

test result: ok. 503 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 26.39s

     Running unittests src/lib.rs (target/debug/deps/connector_state-318cc462d7ad8fa4)

running 3 tests
test tests::a_zero_bound_is_a_caller_mistake_not_a_full_store ... ok
test tests::the_key_grammar_is_closed ... ok
test tests::the_memory_backend_conforms ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running unittests src/lib.rs (target/debug/deps/connectors_client-1bac2ec7216d029d)

running 16 tests
test identity::tests::keyring_account_contains_no_endpoint_or_principal ... ok
test identity::tests::mcp_invocation_uses_only_the_invoke_scope ... ok
test identity::tests::hosted_request_families_select_the_smallest_available_scope ... ok
test tests::completion_endpoint_is_validated_before_secret_submission ... ok
test tests::hosted_client_requires_https_except_on_loopback_or_internal_cluster_dns ... ok
test hosted_catalog::tests::posts_and_validates_a_catalog_frame ... ok
test tests::hosted_client_posts_and_validates_a_datasource_frame ... ok
test admin::tests::named_resources_are_typed_and_the_value_is_not_exposed ... ok
test git_fetch_client::tests::response_is_bound_to_the_request_and_source_authority_is_redacted ... ok
test admin::tests::identity_pkce_exchange_returns_only_the_exact_access_credential ... ok
test identity::tests::login_separates_the_session_and_refreshes_exact_scope_tokens ... ok
test tests::local_client_frames_and_correlates_an_operation ... ok
test tests::hosted_subscription_client_refuses_a_cacheable_credential_boundary ... ok
test tests::hosted_client_posts_the_same_typed_operation_frame ... ok
test tests::hosted_subscription_client_redacts_and_redeems_one_attempt_capability ... ok
test tests::hosted_subscription_client_starts_and_completes_pkce_without_retaining_the_code ... ok

test result: ok. 16 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s

     Running unittests src/lib.rs (target/debug/deps/domain-b9f635ddb3151208)

running 43 tests
test approval::tests::a_matching_presentation_redeems_and_the_proof_carries_the_facts ... ok
test approval::tests::a_refusal_does_not_spend_the_approval ... ok
test approval::tests::a_mismatched_presentation_of_a_spent_approval_reads_as_refused_not_replay ... ok
test approval::tests::an_unwritable_journal_refuses_before_any_dispatch_could_go_unaudited ... ok
test approval::tests::a_second_presentation_refuses_and_audits_as_replay ... ok
test approval::tests::an_oversized_presentation_refuses_and_is_still_on_the_record ... ok
test approval::tests::conclude_writes_the_terminal_row_and_consumes_the_proof ... ok
test approval::tests::every_verification_failure_refuses_without_naming_the_axis ... ok
test approval::tests::the_redemption_key_obeys_the_state_key_grammar ... ok
test audio::tests::a_device_with_no_stack_says_so_rather_than_naming_one ... ok
test audio::tests::a_mono_format_occupies_two_bytes_a_sample ... ok
test approval::tests::journal_kind_tokens_round_trip_and_are_closed ... ok
test audio::tests::every_candidate_is_distinct_and_ordered ... ok
test approval::tests::the_redemption_cell_is_the_attempted_audit_row ... ok
test audio::tests::the_recorded_token_round_trips_through_serde ... ok
test approval::tests::recovery_distinguishes_dead_losers_from_the_settled_winner ... ok
test approval::tests::recovery_settles_a_crash_before_redemption_as_aborted_and_the_approval_stays_redeemable ... ok
test approval::tests::recovery_settles_a_redeemed_but_nonterminal_presentation_as_indeterminate ... ok
test approval::tests::concurrent_identical_presentations_redeem_exactly_once_in_memory ... ok
test connection::tests::inactive_is_a_lifecycle_state_not_an_empty_policy ... ok
test connection::tests::mediated_route_is_explicit_and_cannot_self_reference ... ok
test connection::tests::the_three_policies_are_explicit_sets ... ok
test discovery::tests::recognized_observation_yields_a_candidate_but_not_authority ... ok
test discovery::tests::trusted_local_candidate_proposes_only_a_direct_route ... ok
test discovery::tests::unknown_vendor_type_remains_an_observation_without_a_candidate ... ok
test evaluator::tests::a_request_that_does_not_identify_itself_refuses ... ok
test evaluator::tests::no_store_bound_is_an_outage_not_a_refusal ... ok
test evaluator::tests::a_deny_in_one_grant_beats_an_allow_in_another ... ok
test evaluator::tests::an_expired_decision_refuses_at_use ... ok
test evaluator::tests::an_explicit_allow_beats_the_predicate ... ok
test evaluator::tests::deny_beats_allow_on_the_same_operation ... ok
test evaluator::tests::no_refusal_names_the_axis_that_refused ... ok
test evaluator::tests::the_decision_binds_the_whole_decided_context ... ok
test grant::tests::a_damaged_cell_is_an_outage_not_a_policy ... ok
test grant::tests::a_tenant_outside_the_key_grammar_has_no_grants ... ok
test evaluator::tests::the_selector_admits_only_within_all_three_axes ... ok
test grant::tests::a_grant_set_round_trips_through_its_cell ... ok
test grant::tests::an_absent_or_empty_set_refuses ... ok
test grant::tests::closed_sets_refuse_wildcard_and_empty_entries ... ok
test plan::tests::the_local_owner_path_carries_the_same_evidence_shape ... ok
test evaluator::tests::the_memory_backend_serves_grant_evaluation ... ok
test plan::tests::a_grant_decision_is_the_hosted_admission_path ... ok
test grant::tests::deployment_grants_merge_idempotently_without_erasing_other_owners ... ok

test result: ok. 43 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running unittests src/lib.rs (target/debug/deps/protocol-7b30125b8e3f07c2)

running 37 tests
test approval::tests::approval_lifetime_is_bounded ... ok
test audio::tests::bounded_single_line_text_is_admitted_and_counted_in_characters ... ok
test audio::tests::the_input_refuses_any_field_a_caller_invents ... ok
test audio::tests::empty_control_bearing_and_over_length_text_refuse ... ok
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
test connection::tests::mediated_route_is_value_free_closed_and_cannot_self_reference ... ok
test connection::tests::observations_are_value_free_and_lifecycle_consistent ... ok
test connection::tests::secret_shaped_unknown_fields_are_refused ... ok
test datasource::tests::response_refuses_ambiguous_success_and_failure ... ok
test connection::tests::pending_and_completed_sessions_cannot_mix_endpoint_and_connection ... ok
test datasource::tests::list_is_bounded_and_get_key_is_structured ... ok
test git_fetch::tests::request_refuses_unbounded_or_ambiguous_revisions ... ok
test event::tests::cursor_and_wait_are_bounded ... ok
test connection::tests::candidates_are_value_free_and_activation_selects_no_route ... ok
test operation::tests::connection_audiences_are_bounded_discovery_metadata ... ok
test operation::tests::effect_bearing_operations_require_approval ... ok
test operation::tests::invoke_requires_a_description_lease_and_bounded_structured_input ... ok
test sip::tests::a_number_alone_is_admitted_because_the_trunk_supplies_the_destination ... ok
test git_fetch::tests::response_refuses_secret_or_non_tls_locators ... ok
test operation::tests::a_terminal_status_cannot_omit_its_observed_reason ... ok
test sip::tests::a_number_that_could_escape_the_uri_user_part_is_refused ... ok
test operation::tests::response_envelope_round_trips_and_refuses_unknown_fields ... ok
test operation::tests::owner_context_is_not_defaultable ... ok
test sip::tests::aliases_admit_names_and_refuse_network_destinations ... ok
test sip::tests::a_number_is_bounded_rather_than_truncated ... ok
test sip::tests::an_absent_field_is_omitted_from_the_wire_rather_than_sent_as_null ... ok
test voice::tests::fixture_context_cannot_claim_trust ... ok
test voice::tests::owner_vectors_are_closed_and_unique ... ok

test result: ok. 37 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests/bundles.rs (target/debug/deps/bundles-3bc3d52557544115)

running 13 tests
test connector_catalog_vectors_match_the_strict_reader ... ok
test connector_datasource_vectors_match_the_strict_reader ... ok
test connector_event_vectors_match_the_strict_reader ... ok
test connector_connection_vectors_match_the_strict_reader ... ok
test connector_operation_bundle_is_immutable ... ok
test connector_operation_vectors_match_the_strict_reader ... ok
test connector_catalog_bundle_is_immutable ... ok
test connector_datasource_bundle_is_immutable ... ok
test kubernetes_service_route_round_trips_through_the_connection_response ... ok
test connector_event_bundle_is_immutable ... ok
test owner_contract_bundle_is_immutable ... ok
test rtvbp_binding_bundle_is_immutable ... ok
test connector_connection_bundle_is_immutable ... ok

test result: ok. 13 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running unittests src/lib.rs (target/debug/deps/server-f949655ccc430532)

running 87 tests
test egress::tests::egress_requires_a_nonempty_ascii_connection_or_session_reference ... ok
test egress::tests::ipv4_mapped_ipv6_cannot_bypass_address_classification ... ok
test egress::tests::exact_origin_cannot_be_widened_by_path_host_or_userinfo ... ok
test egress::tests::operator_network_may_admit_private_but_not_process_local_addresses ... ok
test egress::tests::public_dns_refuses_private_local_reserved_and_mixed_answers ... ok
test egress::tests::suffix_rule_requires_a_real_child_and_the_exact_scheme_and_port ... ok
test egress::tests::cached_client_cannot_bypass_current_address_policy ... ok
test hosted::enforcement::tests::the_canonical_digest_ignores_member_order_and_nothing_else ... ok
test hosted::enforcement::tests::an_issued_record_round_trips_without_its_reference_in_any_key ... ok
test hosted::principal::tests::lease_seeds_survive_the_verifier_token_rotation_composition ... ok
test hosted::git_fetch::tests::rejected_control_identity_is_decided_before_the_request_body_is_polled ... ok
test hosted::git_fetch::tests::rejected_source_authority_is_decided_before_body_or_broker_exchange ... ok
test hosted::git_fetch::tests::control_and_internal_routes_are_separate_and_non_cacheable ... ok
test hosted::git_fetch::tests::ambiguous_protocol_or_source_headers_are_refused_before_reading_the_body ... ok
test hosted::tests::admin_routes::operator_group_without_the_exact_scope_cannot_write ... ok
test egress::tests::reused_connection_keeps_authorization_and_timeout_request_specific ... ok
test hosted::tests::admin_routes::auth_metadata_is_public_and_selects_exact_authority ... ok
test hosted::tests::contract_validation::hosted_route_refuses_a_malformed_backend_contract ... ok
test hosted::tests::admin_routes::operator_can_write_and_status_never_returns_the_secret ... ok
test hosted::tests::enforcement::a_granted_effect_without_approval_demand_dispatches_on_the_grant_alone ... ok
test hosted::tests::enforcement::a_granted_mutation_demanding_approval_refuses_without_one ... ok
test hosted::tests::a_human_issues_one_exact_input_approval_which_is_spent_once ... ok
test egress::tests::pool_is_bounded_and_separates_current_addresses_authorities_origins_and_policy ... ok
test hosted::tests::enforcement::a_mutation_with_no_admitting_grant_refuses ... ok
test hosted::tests::enforcement::a_granted_mutation_with_a_demanded_approval_dispatches_with_one ... ok
test hosted::tests::enforcement::an_unbound_grant_store_is_an_outage_for_effects_only ... ok
test hosted::tests::enforcement::a_second_presentation_of_the_same_approval_refuses_and_journals_replay ... ok
test hosted::tests::hosted_completion_streams_fragments_into_a_redacted_bounded_submission ... ok
test hosted::tests::enforcement::the_read_only_path_is_unchanged_for_callers_without_grants ... ok
test hosted::tests::hosted_completion_failures_are_non_cacheable_and_browser_hardened ... ok
test hosted::tests::hosted_connection_route_uses_the_same_identity_boundary ... ok
test hosted::tests::hosted_datasource_route_passes_verified_groups_and_exact_tenant ... ok
test hosted::tests::hosted_route_requires_identity_and_exact_tenant_binding ... ok
test hosted::tests::enforcement::every_enforcement_refusal_renders_the_same_bytes ... ok
test hosted::tests::mcp::a_busy_namespace_is_listed_whole_without_a_paging_surface ... ok
test hosted::tests::mcp::a_stale_authority_refusal_is_retried_exactly_once_with_a_fresh_lease ... ok
test hosted::tests::mcp::an_invoke_without_the_invoke_scope_surfaces_not_granted ... ok
test hosted::tests::mcp::an_op_backed_invoke_describes_then_invokes_with_the_fresh_lease ... ok
test hosted::tests::mcp::approval_demand_and_evidence_pass_through_the_admission_seam ... ok
test hosted::tests::mcp::datasource_backed_tools_route_through_the_datasource_seam ... ok
test hosted::tests::mcp::initialize_echoes_admitted_revisions_and_answers_ping ... ok
test hosted::tests::mcp::the_mcp_route_is_stateless_post_only ... ok
test hosted::tests::mcp::tool_describe_projects_the_underlying_description_without_a_lease ... ok
test hosted::tests::mcp::tool_search_projects_only_the_entries_the_callers_seam_results_support ... ok
test hosted::tests::mcp::tools_list_returns_exactly_the_three_meta_tools ... ok
test hosted::tests::mcp::transport_refusals_carry_the_designed_statuses_and_codes ... ok
test hosted::tests::docs::openapi_json_is_served_verbatim_with_a_content_hash_etag ... ok
test hosted::tests::mcp_monitoring::a_monitoring_invoke_refuses_dishonest_targets_before_any_dispatch ... ok
test hosted::tests::mcp_monitoring::a_stale_monitoring_invoke_re_resolves_the_same_target_exactly_once ... ok
test hosted::tests::mcp::a_pathological_namespace_is_cut_with_an_explicit_truncation_marker ... ok
test hosted::tests::mcp_monitoring::a_monitoring_invoke_routes_the_chosen_target_through_the_decided_seam ... ok
test hosted::tests::docs::the_document_pins_the_exact_wire_contract_identities_and_audience ... ok
test hosted::tests::docs::every_documented_refusal_example_names_a_real_error_code ... ok
test hosted::tests::docs::a_request_example_with_an_unknown_field_is_refused ... ok
test hosted::tests::monitoring_transport_gate_admits_only_configured_groups_or_operator ... ok
test hosted::tests::pod_log_transport_gate_admits_only_kubernetes_read_groups_or_operator ... ok
test hosted::tests::self_event_scope_is_closed_to_slack_specific_requests ... ok
test hosted::tests::production_router_publishes_client_discovery_without_authentication ... ok
test hosted::tests::mcp_monitoring::the_acceptance_sequence_invokes_with_a_target_and_integer_epochs ... ok
test hosted::tests::signal::a_granted_session_signal_dispatches_behind_the_sessions_grant ... ok
test hosted::tests::signal::an_effect_bearing_session_signal_without_an_admitting_grant_refuses ... ok
test hosted::tests::signal::an_unbound_grant_store_is_an_outage_for_session_signals ... ok
test hosted::tests::signal::a_signal_refusal_matches_the_invoke_refusal_bytes ... ok
test hosted::tests::tenant_members_receive_only_read_only_module_invocation ... ok
test local::tests::a_broad_state_directory_refuses_without_repair ... ok
test hosted::tests::docs::every_documented_request_example_is_accepted_by_its_protocol_type ... ok
test hosted::tests::subscription_oauth_start_is_identity_scoped_bounded_and_non_cacheable ... ok
test local::tests::a_second_daemon_cannot_unlink_the_live_daemons_socket ... ok
test hosted::tests::subscription_credential_stays_in_custody_and_only_an_exact_lease_redeems ... ok
test hosted::tests::mcp_monitoring::tool_search_lists_the_monitoring_tools_for_a_monitoring_read_principal ... ok
test local::tests::one_socket_dispatches_the_value_free_connection_and_event_contracts ... ok
test hosted::tests::docs::the_docs_page_is_served_unauthenticated_as_html ... ok
test local::tests::owner_socket_serves_one_strict_bounded_operation_frame ... ok
test hosted::tests::mcp_monitoring::tool_describe_enumerates_the_callers_configured_targets_without_a_lease ... ok
test hosted::tests::docs::the_docs_page_links_the_contract_and_renders_its_version ... ok
test hosted::tests::docs::every_docs_page_example_is_the_documents_example_after_json_normalization ... ok
test hosted::tests::docs::the_docs_page_makes_zero_external_requests ... ok
test hosted::tests::docs::the_docs_page_refusal_table_carries_every_documented_code ... ok
test hosted::tests::docs::every_documented_mcp_request_example_is_answered_by_the_live_transport ... ok
test hosted::tests::mcp_monitoring::monitoring_tool_schemas_state_the_documents_contract ... ok
test catalog_projection::tests::a_deployment_publishes_only_the_setup_flows_it_can_complete ... ok
test catalog_projection::tests::search_is_whole_catalog_and_describe_is_descriptive_only ... ok
test catalog_projection::tests::platform_provider_satisfies_the_catalog_wire_contract ... ok
test catalog_projection::tests::every_shipped_provider_description_satisfies_the_catalog_wire_contract ... ok
test hosted::tests::hosted_liveness_and_identity_backed_readiness_are_distinct ... ok
test hosted::tests::hosted_liveness_stays_local_when_a_backend_dependency_is_unready ... ok
test hosted::tests::docs::every_documented_route_exists_in_the_real_router ... ok

test result: ok. 87 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.84s

     Running unittests src/lib.rs (target/debug/deps/service-28db7c33fc926dd9)

running 57 tests
test audio::tests::a_route_for_another_connection_is_refused ... ok
test audio::tests::a_plan_for_another_driver_or_operation_is_refused ... ok
test admin::tests::status_reports_presence_without_reading_the_value ... ok
test admin::tests::write_requires_explicit_replacement_and_audits_no_secret ... ok
test audio::tests::admitted_evidence_never_prints_as_a_serializable_route_secret ... ok
test audio::tests::status_never_admits_an_utterance ... ok
test audio::tests::relative_paths_absent_bounds_and_bad_digests_never_reach_the_device ... ok
test audio::tests::a_speech_plan_and_its_deployment_route_admit_together ... ok
test audio::tests::the_caller_text_is_bounded_by_the_deployment_and_not_only_by_the_catalog ... ok
test authority::tests::proof_is_bound_to_exact_upgrade_uri ... ok
test authority::tests::debug_never_prints_authority_or_proof ... ok
test browser::tests::a_route_for_another_connection_is_refused ... ok
test browser::tests::a_unary_lifecycle_is_refused_because_a_browser_spans_calls ... ok
test browser::tests::a_non_web_address_is_refused_before_any_browser_is_touched ... ok
test browser::tests::a_plan_for_another_driver_or_operation_is_refused ... ok
test browser::tests::admitted_evidence_never_prints_as_a_serializable_route_secret ... ok
test browser::tests::only_open_may_omit_an_address_and_only_open_or_goto_may_carry_one ... ok
test browser::tests::every_admitted_browser_operation_and_its_route_admit_together ... ok
test browser::tests::relative_paths_nested_artifacts_and_absent_bounds_never_reach_the_browser ... ok
test browser::tests::the_operators_own_browser_profile_is_never_an_admitted_route ... ok
test connect_session::tests::capacity_counts_only_pending_sessions ... ok
test connect_session::tests::shutdown_fails_only_pending_sessions_and_returns_their_endpoints ... ok
test connect_session::tests::terminal_transition_is_one_way_and_value_free ... ok
test dispatch::tests::composition_order_is_policy_redaction_audit_driver_audit ... ok
test authority::tests::session_lease_cannot_be_extended_by_authority_clock_skew ... ok
test planning::tests::missing_driver_refuses_before_dispatch ... ok
test planning::tests::bidirectional_connection_still_requires_the_operation_admission ... ok
test runtime::tests::delegated_execution_must_match_the_authenticated_actor ... ok
test planning::tests::provider_only_connection_refuses_a_caller_initiated_operation ... ok
test planning::tests::mediated_http_plan_has_no_direct_origin_and_requires_the_closed_adapter ... ok
test planning::tests::sip_plan_has_no_http_fields ... ok
test authority::tests::serving_endpoint_redeems_once ... ok
test runtime::tests::local_principals_require_a_real_agent_revision ... ok
test runtime::tests::the_stable_authority_seed_distinguishes_absent_and_literal_default_realms ... ok
test runtime::tests::hosted_completion_submission_never_reallocates_received_secret_material ... ok
test runtime::tests::the_stable_authority_seed_ignores_request_scoped_provenance ... ok
test runtime::tests::hosted_principals_do_not_fabricate_agent_revisions ... ok
test runtime::tests::the_stable_authority_seed_survives_token_scoped_snapshot_fields ... ok
test sip::tests::a_default_naming_an_absent_trunk_is_refused_when_the_table_is_built ... ok
test sip::tests::a_dial_with_only_a_number_takes_the_declared_default_trunk ... ok
test sip::tests::a_partial_byte_prefix_masks_only_the_bits_it_declares ... ok
test sip::tests::a_prefix_aperture_admits_its_network_and_refuses_outside_it ... ok
test sip::tests::a_named_trunk_is_resolved_before_admission_so_the_answer_is_aperture_checked ... ok
test sip::tests::a_dial_with_no_alias_and_no_default_is_refused_rather_than_guessed ... ok
test sip::tests::a_prefix_that_is_not_on_its_own_boundary_is_refused_rather_than_rounded ... ok
test authority::tests::audience_expiry_and_revocation_fail_before_redemption ... ok
test sip::tests::a_trunk_that_does_not_admit_numbers_refuses_one ... ok
test sip::tests::an_aperture_never_matches_across_address_families ... ok
test sip::tests::an_exact_aperture_still_admits_exactly_one_address ... ok
test sip::tests::exact_loopback_route_produces_non_serializable_driver_evidence ... ok
test sip::tests::missing_organization_in_grant_evidence_refuses_before_the_driver ... ok
test sip::tests::sip_dial_resolves_only_an_exact_connection_owned_alias ... ok
test sip::tests::stable_network_and_aperture_widening_refuse_before_the_driver ... ok
test sip::tests::the_dialled_host_comes_from_the_trunk_and_never_from_the_caller ... ok
test sip::tests::zero_or_excessive_dial_deadlines_refuse_before_the_driver ... ok
test voice::tests::invalid_endpoint_and_authority_windows_refuse_before_io ... ok
test voice::tests::one_proof_joins_exact_sip_and_application_routes ... ok

test result: ok. 57 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running unittests src/lib.rs (target/debug/deps/subscription_custody-10c17d37fad573f0)

running 6 tests
test tests::an_initial_oauth_response_without_scopes_is_still_refused ... ok
test tests::custody_never_exports_but_an_exact_attempt_lease_can_redeem ... ok
test tests::disconnect_revokes_live_leases_and_removes_presence ... ok
test tests::replacing_a_credential_revokes_every_lease_over_the_old_generation ... ok
test tests::pkce_completion_refuses_a_missing_or_mismatched_returned_state_before_exchange ... ok
test tests::pkce_completion_stays_in_custody_and_refreshes_at_redemption ... ok

test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

   Doc-tests catalog

running 1 test
test crates/catalog/src/lib.rs - (line 9) ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.86s

   Doc-tests catalog_build

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

   Doc-tests catalog_reader

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

   Doc-tests connector_address

running 2 tests
test crates/connector-address/src/lib.rs - (line 51) ... ok
test crates/connector-address/src/origin.rs - origin (line 44) ... ok

test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.05s

   Doc-tests connector_oauth

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

   Doc-tests connector_resolve

running 1 test
test crates/connector-resolve/src/lib.rs - (line 3) - compile ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s

   Doc-tests connector_secrets

running 5 tests
test crates/connector-secrets/src/secret.rs - secret::Secret (line 18) - compile fail ... ok
test crates/connector-secrets/src/transaction.rs - transaction::SecretTransactionId (line 59) - compile fail ... ok
test crates/connector-secrets/src/transaction.rs - transaction::SecretTransactionId (line 52) - compile fail ... ok
test crates/connector-secrets/src/secret.rs - secret::Secret (line 25) ... ok
test crates/connector-secrets/src/lib.rs - (line 25) ... ok

test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.16s

   Doc-tests connector_spec

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

   Doc-tests connector_state

running 1 test
test crates/connector-state/src/conformance.rs - conformance (line 9) - compile ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s

   Doc-tests connectors_client

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

   Doc-tests domain

running 2 tests
test crates/domain/src/evaluator.rs - evaluator::conformance (line 414) - compile ... ok
test crates/domain/src/evaluator.rs - evaluator::GrantDecision (line 141) - compile fail ... ok

test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s

   Doc-tests protocol

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

   Doc-tests server

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

   Doc-tests service

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

   Doc-tests subscription_custody

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

error: 1 target failed:
    `-p catalog-build --test main`

exit 101
```

```text
$ cargo test --manifest-path Cargo.toml -p catalog-build --test main --locked ess_claim_fence::materialize_declares_or_marks_every_refusal_its_cited_function_performs -- --exact --nocapture
    Blocking waiting for file lock on package cache
    Blocking waiting for file lock on package cache
    Blocking waiting for file lock on package cache
    Finished `test` profile [unoptimized] target(s) in 0.29s
     Running tests/main.rs (target/debug/deps/main-d1fd840af6e284f8)

running 1 test

thread 'ess_claim_fence::materialize_declares_or_marks_every_refusal_its_cited_function_performs' (2414487) panicked at crates/catalog-build/tests/main/ess_claim_fence.rs:448:5:
ess/system/domains/connection.yaml declares `connectors.connection.MaterializeObservation` without citing `fn materialize` at crates/integration-monitoring/src/backend.rs:1182, which is the function this fence measures
  crates/integration-monitoring/src/backend.rs:1190 refuses — `return Err(ConnectionError::new(` — and no outcome of `connectors.connection.MaterializeObservation` cites it and no `UNMAPPED:` marker of `connectors.connection.MaterializeObservation` cites it. A refusal the tree performs and the specification neither declares nor marks is a refusal a caller is never told about.
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace
test ess_claim_fence::materialize_declares_or_marks_every_refusal_its_cited_function_performs ... FAILED

failures:

failures:
    ess_claim_fence::materialize_declares_or_marks_every_refusal_its_cited_function_performs

test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 72 filtered out; finished in 0.00s

error: test failed, to rerun pass `-p catalog-build --test main`

exit 101
```

```text
$ cargo test --manifest-path crates/connectors-cli/Cargo.toml --workspace --locked --no-fail-fast
    Blocking waiting for file lock on package cache
    Blocking waiting for file lock on package cache
    Blocking waiting for file lock on package cache
warning: connector-secrets@0.6.5: no live Vault was offered (CONNECTOR_SECRETS_VAULT_ADDR and CONNECTOR_SECRETS_VAULT_TOKEN are unset), so the reqwest HttpTransport is UNEXERCISED by this build: tests/vault_live.rs is compiled #[ignore]d
    Finished `test` profile [unoptimized] target(s) in 0.32s
     Running unittests src/lib.rs (crates/connectors-cli/target/debug/deps/connectors_cli-fccd3c25d12631f8)

running 5 tests
test tests::slack_connect_needs_no_internal_reference_or_path_argument ... ok
test tests::grafana_connect_uses_the_same_guided_surface ... ok
test tests::kubernetes_connect_accepts_an_exact_context_selection ... ok
test tests::normal_help_exposes_the_guided_flow_and_hides_acquisition_plumbing ... ok
test tests::every_supported_shell_gets_a_script_naming_the_whole_surface ... ok

test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s

     Running unittests src/main.rs (crates/connectors-cli/target/debug/deps/connectors-d3c17156352c01ce)

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests/adversary_cli_cap_pass3.rs (crates/connectors-cli/target/debug/deps/adversary_cli_cap_pass3-571f8002650cebbe)

running 1 test
test the_cap_the_design_page_says_is_measured_is_declared_and_asserted ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests/adversary_fence_probe.rs (crates/connectors-cli/target/debug/deps/adversary_fence_probe-01065856beff2fd0)

running 6 tests
test the_wire_name_rule_citation_in_the_design_document_points_at_the_rule ... ok
test the_wire_name_rule_citation_in_the_specification_points_at_the_rule ... ok
test the_typeable_words_are_the_words_the_design_document_names ... ok
test the_copies_this_probe_carries_are_still_copies ... ok
test a_forwarding_reason_that_names_no_command_is_refused_whatever_kind_it_carries ... ok
test every_entry_that_is_not_a_lifecycle_step_is_refused_when_it_claims_to_be_one ... ok

test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.12s

     Running tests/adversary_fence_probe_pass2.rs (crates/connectors-cli/target/debug/deps/adversary_fence_probe_pass2-2f4eed594364a710)

running 3 tests
test every_file_the_committed_contract_opens_is_a_file_a_clone_has ... ok
test the_kinds_the_design_document_says_rest_on_no_sentence_rest_on_no_sentence ... ok
test the_paths_the_design_document_says_send_no_protocol_request_send_none ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests/adversary_shim_pass3.rs (crates/connectors-cli/target/debug/deps/adversary_shim_pass3-630c1081d61d3bb5)

running 6 tests
test the_serve_group_advertises_a_help_subcommand ... ok
test a_help_path_of_the_new_tree_under_serve_is_left_alone ... ok
test connectors_help_still_answers_for_a_path_that_moved ... ok
test a_moved_path_typed_with_the_global_output_flag_still_works ... ok
test the_group_word_whose_only_command_moved_still_points_somewhere ... ok
test nothing_this_product_prints_names_a_moved_path_behind_a_global_flag ... ok

test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.13s

     Running tests/adversary_shim_pass4.rs (crates/connectors-cli/target/debug/deps/adversary_shim_pass4-c5e29d0a4a560a59)

running 3 tests
test the_table_this_suite_copies_by_hand_is_the_table_the_binary_ships ... ok
test the_auth_group_still_answers_the_help_subcommand_it_advertised ... ok
test a_two_word_path_that_moved_works_with_the_global_flag_between_its_words ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.05s

     Running tests/adversary_shim_pass5.rs (crates/connectors-cli/target/debug/deps/adversary_shim_pass5-c727a9e500a5d459)

running 4 tests
test a_positional_value_spelled_help_is_a_value_not_a_help_request ... ok
test an_argument_neither_the_group_nor_the_leaf_declares_is_not_the_old_leaf ... ok
test the_double_dash_escape_is_not_a_word_that_moved ... ok
test serve_with_only_global_options_is_the_group_in_every_spelling_and_position ... ok

test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.05s

     Running tests/cli_surface.rs (crates/connectors-cli/target/debug/deps/cli_surface-7f752a2de196c700)

running 34 tests
test every_kind_of_exception_is_used_and_every_entry_gives_a_reason ... ok
test every_named_exception_is_still_a_path_of_the_parser ... ok
test no_word_of_the_parser_answers_to_a_name_the_specification_cannot_declare ... ok
test every_declared_group_is_a_group_of_the_parser ... ok
test every_declared_group_help_line_is_the_summary_the_specification_declares ... ok
test no_path_is_both_declared_and_excepted ... ok
test every_declaration_the_adversary_probe_copies_is_still_a_copy ... ok
test every_path_of_the_parser_is_declared_or_a_named_exception ... ok
test the_committed_generated_tree_is_the_specification_word_for_word ... ok
test every_citation_this_unit_wrote_resolves ... ok
test the_old_login_selected_target_guard_is_absent ... ok
test the_read_verb_enumeration_partitions_the_protocols_it_names ... ok
test the_regeneration_command_the_documents_name_is_the_one_the_gate_runs ... ok
test a_read_stops_being_an_exception_once_the_specification_declares_a_view ... ok
test a_command_absorbed_into_the_exception_list_alone_is_refused ... ok
test the_parser_accepts_target_before_and_after_each_dual_target_leaf ... ok
test the_specification_names_the_binary_the_parser_builds ... ok
test the_exception_list_is_the_set_the_specification_enumerates ... ok
test the_target_countdown_is_exactly_what_the_parser_still_owes ... ok
test every_citation_that_names_a_symbol_lands_on_its_declaration ... ok
test the_kinds_the_tree_derives_are_the_kinds_the_list_carries ... ok
test targeted_errors_keep_the_target_in_yaml_and_text ... ok
test target_conflict_does_not_wait_for_open_stdin ... ok
test an_explicit_hosted_target_requires_a_login_by_name_for_every_group ... ok
test target_conflict_precedes_invoke_payload_loading ... ok
test target_conflict_precedes_missing_or_malformed_inline_input ... ok
test hosted_refuses_each_local_only_option_for_every_group ... ok
test an_exception_whose_kind_the_tree_contradicts_is_refused ... ok
test selected_target_preserves_provider_owned_target_fields_in_every_renderer ... ok
test broken_explicit_hosted_selection_never_falls_back_to_a_local_listener ... ok
test local_success_and_protocol_refusals_report_the_selected_target ... ok
test all_target_conflicts_precede_local_and_hosted_state_access ... ok
test every_local_leaf_ignores_broken_login_metadata_and_preserves_its_request ... ok
test an_omitted_target_ignores_a_saved_login_for_every_dual_target_group ... ok

test result: ok. 34 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 4.58s

     Running tests/cli_surface_drift.rs (crates/connectors-cli/target/debug/deps/cli_surface_drift-632672f2230f40ef)

running 10 tests
test the_copied_declarations_are_still_copies ... ok
test the_thin_frontend_citation_points_at_the_thin_frontend_test ... ok
test a_committed_tree_whose_group_about_no_longer_matches_the_specification_is_refused ... ok
test a_command_added_under_the_wrong_declared_group_is_refused ... ok
test a_committed_tree_that_swaps_completions_for_an_undeclared_word_is_refused ... ok
test a_command_added_under_a_declared_group_is_refused ... ok
test cutting_the_admin_group_over_to_the_generated_tree_is_refused ... ok
test the_restated_contract_is_green_against_the_unchanged_tree ... ok
test a_target_flag_removed_from_a_group_is_refused_by_the_countdown ... ok
test cargo_can_read_the_committed_emitted_manifest ... ok

test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s

     Running tests/cli_surface_pass_two.rs (crates/connectors-cli/target/debug/deps/cli_surface_pass_two-6b780902e573fedf)

running 6 tests
test the_design_document_names_only_constants_that_exist ... ok
test the_design_document_states_the_shape_of_the_exception_list ... ok
test the_design_document_describes_the_countdown_assertion_the_contract_makes ... ok
test the_target_countdown_candidates_are_derived_from_every_protocol_a_deployment_answers ... ok
test the_drift_suites_copies_are_checked_rather_than_cited ... ok
test the_drift_suite_attributes_nothing_to_the_contract_that_is_not_there ... ok

test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests/closed_pipe.rs (crates/connectors-cli/target/debug/deps/closed_pipe-5c565e9aa9e82598)

running 20 tests
test completion_scripts_keep_other_output_write_failures_unsuccessful ... ok
test completion_scripts_still_accept_a_closed_reader ... ok
test admin_authentication_failures_remain_unsuccessful_with_a_closed_reader ... ok
test stdin_read_errors_stay_unsuccessful_with_a_closed_output_reader ... ok
test a_closed_transport_stays_unsuccessful ... ok
test compact_consumer_closes_early ... ok
test json_consumer_closes_early ... ok
test a_healthy_doctor_accepts_a_closed_report_reader ... ok
test an_unhealthy_doctor_stays_unsuccessful_when_its_report_reader_closes ... ok
test setup_init_commits_its_result_before_a_reader_close_but_keeps_repeat_refusal ... ok
test text_consumer_closes_early ... ok
test every_admin_leaf_preserves_non_broken_pipe_output_failures ... ok
test successful_admin_results_accept_a_closed_reader_in_every_format ... ok
test successful_admin_credential_write_accepts_a_closed_reader ... ok
test yaml_consumer_closes_early ... ok
test every_format_really_emits_more_than_a_64_kib_pipe_buffer ... ok
test each_unhealthy_report_class_keeps_its_failure_when_output_closes ... ok
test protocol_refusals_keep_their_failure_when_a_result_reader_closes ... ok
test each_protocol_search_distinguishes_closed_readers_from_other_write_failures ... ok
test a_real_non_broken_pipe_output_failure_stays_unsuccessful ... ok

test result: ok. 20 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 4.51s

     Running tests/first_level_groups.rs (crates/connectors-cli/target/debug/deps/first_level_groups-7f9db665d69fea16)

running 5 tests
test the_first_level_is_eight_words ... ok
test doctor_reports_the_same_installation_at_both_paths ... ok
test the_serve_group_answers_bare_and_with_help_like_the_other_groups ... ok
test a_path_of_the_new_tree_is_left_alone ... ok
test every_moved_path_still_works_and_names_where_it_went ... ok

test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.16s

     Running tests/moved_paths_are_not_taught.rs (crates/connectors-cli/target/debug/deps/moved_paths_are_not_taught-80a8762e273e0578)

running 1 test
test nothing_this_product_prints_names_a_path_that_moved ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.12s

     Running tests/one_shot_operations.rs (crates/connectors-cli/target/debug/deps/one_shot_operations-f584650961a855c8)

running 16 tests
test a_running_daemon_is_used_without_constructing_a_local_runtime ... ok
test a_transport_that_drops_the_request_is_never_retried_locally ... ok
test connection_mutations_require_daemon_before_creating_continuation_state ... ok
test adversary_hosted_refusal_and_target_conflict_never_construct_the_local_runtime ... ok
test events_and_session_signals_name_the_persistent_daemon_requirement ... ok
test existing_or_unsafe_socket_objects_never_trigger_ephemeral_fallback ... ok
test adversary_json_source_and_size_refusals_precede_one_shot_state_creation ... ok
test doctor_enumerates_bounded_and_persistent_verbs ... ok
test invalid_bounds_and_unsafe_state_refuse_before_runtime_state_is_opened ... ok
test ordinary_search_and_connection_list_use_default_paths_without_a_daemon ... ok
test separate_describe_and_invoke_processes_reuse_the_same_authority_without_a_daemon ... ok
test concurrent_commands_and_daemon_start_cannot_take_the_in_flight_invocation_state ... ok
test adversary_uncertain_invoke_never_resends_after_the_control_socket_disappears ... ok
test a_changed_authority_or_selected_connection_never_reaches_fixture_egress ... ok
test adversary_caller_input_cannot_rebind_routes_or_revoked_grants ... ok
test browser_session_operations_are_refused_under_canonical_and_published_aliases ... ok

test result: ok. 16 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 24.49s

   Doc-tests connectors_cli

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s


exit 0
```

All four strict clippy and formatting checks completed with exit 0. Exact commands and complete outputs follow.

```text
$ cargo clippy --manifest-path crates/connectors-runtime/Cargo.toml --workspace --all-targets --locked -- -D warnings
warning: connector-secrets@0.6.5: no live Vault was offered (CONNECTOR_SECRETS_VAULT_ADDR and CONNECTOR_SECRETS_VAULT_TOKEN are unset), so the reqwest HttpTransport is UNEXERCISED by this build: tests/vault_live.rs is compiled #[ignore]d
    Checking connectors-runtime v0.6.5 (~/.local/state/worktree/trees/b10x/connectors/wt-fc49f37347f5/crates/connectors-runtime)
    Finished `dev` profile [unoptimized] target(s) in 1.43s

exit 0
```

```text
$ cargo fmt --manifest-path crates/connectors-runtime/Cargo.toml --all -- --check

exit 0
```

```text
$ cargo clippy --manifest-path crates/connectors-console/Cargo.toml --workspace --all-targets --locked -- -D warnings
    Checking integration-catalog v0.6.5 (~/.local/state/worktree/trees/b10x/connectors/wt-fc49f37347f5/crates/integration-catalog)
    Checking connectors-console v0.6.5 (~/.local/state/worktree/trees/b10x/connectors/wt-fc49f37347f5/crates/connectors-console)
    Finished `dev` profile [unoptimized] target(s) in 1.42s

exit 0
```

```text
$ cargo fmt --manifest-path crates/connectors-console/Cargo.toml --all -- --check

exit 0
```

```text
$ cargo clippy --manifest-path crates/server/Cargo.toml --workspace --all-targets --locked -- -D warnings
    Checking typenum v1.20.1
    Checking serde v1.0.229
    Checking thiserror v2.0.20
    Checking zerofrom v0.1.8
    Checking serde_json v1.0.151
    Checking yoke v0.8.3
    Checking zerovec v0.11.7
    Checking generic-array v0.14.7
    Checking tinystr v0.8.4
    Checking zerotrie v0.2.5
    Checking crypto-common v0.1.7
    Checking block-buffer v0.10.4
    Checking icu_locale_core v2.3.0
    Checking potential_utf v0.1.6
    Checking digest v0.10.7
    Checking icu_collections v2.3.0
    Checking sha2 v0.10.9
    Checking slab v0.4.12
    Checking linux-raw-sys v0.12.1
    Checking icu_provider v2.3.0
    Checking ref-cast v1.0.26
    Checking tokio v1.53.1
    Checking icu_properties v2.3.0
    Checking icu_normalizer v2.3.0
    Checking rustix v1.1.4
    Checking idna_adapter v1.2.2
    Checking idna v1.1.0
    Checking connector-address v0.6.5 (~/.local/state/worktree/trees/b10x/connectors/wt-fc49f37347f5/crates/connector-address)
    Checking num-traits v0.2.19
    Checking tracing v0.1.44
    Checking num-integer v0.1.47
    Checking futures-util v0.3.34
    Checking parking v2.2.1
    Checking num-bigint v0.4.8
    Checking url v2.5.8
    Checking hybrid-array v0.4.14
    Checking equivalent v1.0.2
    Checking foldhash v0.2.0
    Checking allocator-api2 v0.2.21
    Checking num-rational v0.4.2
    Checking hashbrown v0.17.1
    Checking num-iter v0.1.46
    Checking num-complex v0.4.6
    Checking fastrand v2.5.0
    Checking crossbeam-utils v0.8.22
    Checking num v0.4.3
    Checking futures-io v0.3.34
    Checking indexmap v2.14.0
    Checking concurrent-queue v2.5.0
    Checking crypto-common v0.2.2
    Checking event-listener v5.4.2
    Checking unicode-ident v1.0.24
    Checking hyper v1.11.0
    Checking proc-macro2 v1.0.107
    Checking schemars v1.2.2
    Checking event-listener-strategy v0.5.4
    Checking futures-lite v2.6.1
    Checking toml_datetime v0.6.11
    Checking serde_spanned v0.6.9
    Checking connector-state v0.6.5 (~/.local/state/worktree/trees/b10x/connectors/wt-fc49f37347f5/crates/connector-state)
    Checking toml_write v0.1.2
    Checking regex-syntax v0.8.11
    Checking winnow v0.7.15
    Checking hyper-util v0.1.20
    Checking quote v1.0.47
    Checking toml_edit v0.22.27
    Checking tower v0.5.3
    Checking tokio-rustls v0.26.4
    Checking aho-corasick v1.1.5
    Checking scopeguard v1.2.0
    Checking cmov v0.5.4
    Checking unsafe-libyaml-norway v0.2.15
    Checking serde_norway v0.9.42
    Checking ctutils v0.4.2
    Checking toml v0.8.23
    Checking regex-automata v0.4.18
    Checking lock_api v0.4.14
    Checking syn v3.0.3
    Checking ahash v0.8.12
    Checking parking_lot_core v0.9.12
    Checking domain v0.6.5 (~/.local/state/worktree/trees/b10x/connectors/wt-fc49f37347f5/crates/domain)
    Checking connector-secrets v0.6.5 (~/.local/state/worktree/trees/b10x/connectors/wt-fc49f37347f5/crates/connector-secrets)
    Checking block-padding v0.4.2
    Checking block-buffer v0.12.1
    Checking polling v3.11.0
    Checking serde_urlencoded v0.7.1
    Checking errno v0.3.14
    Checking lazy_static v1.5.0
    Checking async-task v4.7.1
    Checking winnow v1.0.4
    Checking borrow-or-share v0.2.4
    Checking const-oid v0.10.2
    Checking bit-vec v0.8.0
    Checking bit-set v0.8.0
    Checking digest v0.11.3
    Checking fluent-uri v0.4.1
    Checking signal-hook-registry v1.4.8
    Checking zvariant_utils v4.2.0
    Checking fraction v0.15.4
    Checking async-io v2.6.0
    Checking inout v0.2.2
    Checking parking_lot v0.12.5
    Checking hyper-rustls v0.27.9
    Checking tower-http v0.6.11
    Checking async-channel v2.5.0
    Checking enumflags2 v0.7.12
    Checking tokio-util v0.7.19
    Checking zcheapstr v1.1.0
    Checking vsimd v0.8.0
    Checking outref v0.5.2
    Checking endi v1.1.1
    Checking bytecount v0.6.9
    Checking micromap v0.3.0
    Checking num-cmp v0.1.0
    Checking zvariant v5.15.0
    Checking referencing v0.49.9
    Checking jsonschema-value v0.49.9
    Checking uuid-simd v0.8.0
    Checking reqwest v0.12.28
    Checking strum v0.28.0
    Checking cipher v0.5.2
    Checking async-signal v0.2.14
    Checking fancy-regex v0.19.0
    Checking unicode-general-category v1.1.0
    Checking regex v1.13.1
    Checking protocol v0.6.5 (~/.local/state/worktree/trees/b10x/connectors/wt-fc49f37347f5/crates/protocol)
    Checking connector-spec v0.6.5 (~/.local/state/worktree/trees/b10x/connectors/wt-fc49f37347f5/crates/connector-spec)
    Checking jsonschema-regex v0.49.9
    Checking async-lock v3.4.2
    Checking catalog v0.6.5 (~/.local/state/worktree/trees/b10x/connectors/wt-fc49f37347f5/crates/catalog)
    Checking piper v0.2.5
    Checking email_address v0.2.9
    Checking cpufeatures v0.3.0
    Checking utf8parse v0.2.2
    Checking anstyle-parse v1.0.0
    Checking jsonschema v0.49.9
    Checking blocking v1.7.0
    Checking async-process v2.5.0
    Checking zbus_names v4.3.4
    Checking anyhow v1.0.104
    Checking hmac v0.13.0
    Checking async-executor v1.14.0
    Checking async-broadcast v0.7.2
    Checking ordered-stream v0.2.0
    Checking uuid v1.26.0
    Checking colorchoice v1.0.5
    Checking anstyle-query v1.1.5
    Checking is_terminal_polyfill v1.70.2
    Checking anstyle v1.0.14
    Checking cpubits v0.1.1
    Checking anstream v1.0.0
    Checking aes v0.9.2
    Checking axum-core v0.5.6
    Checking zbus v5.19.0
    Checking curve25519-dalek v4.1.3
    Checking hkdf v0.13.0
    Checking getrandom v0.4.3
    Checking sha2 v0.11.0
    Checking cbc v0.2.1
    Checking strsim v0.11.1
    Checking clap_lex v1.1.0
    Checking clap_builder v4.6.6
    Checking axum v0.8.9
    Checking ed25519-dalek v2.2.0
    Checking catalog-build v0.6.5 (~/.local/state/worktree/trees/b10x/connectors/wt-fc49f37347f5/crates/catalog-build)
    Checking connector-resolve v0.6.5 (~/.local/state/worktree/trees/b10x/connectors/wt-fc49f37347f5/crates/connector-resolve)
    Checking connector-oauth v0.6.5 (~/.local/state/worktree/trees/b10x/connectors/wt-fc49f37347f5/crates/connector-oauth)
    Checking keyring-core v1.0.0
    Checking sha1 v0.10.7
    Checking tungstenite v0.28.0
    Checking clap v4.6.6
    Checking secret-service v5.2.0
    Checking subscription-custody v0.6.5 (~/.local/state/worktree/trees/b10x/connectors/wt-fc49f37347f5/crates/subscription-custody)
    Checking tokio-tungstenite v0.28.0
    Checking service v0.6.5 (~/.local/state/worktree/trees/b10x/connectors/wt-fc49f37347f5/crates/service)
    Checking identity-client v0.5.6 (https://github.com/beyond10x/identity.git?tag=0.5.6#e3231bc3)
    Checking tempfile v3.27.0
    Checking zbus-secret-service-keyring-store v1.0.1
    Checking server v0.6.5 (~/.local/state/worktree/trees/b10x/connectors/wt-fc49f37347f5/crates/server)
    Checking keyring v4.2.0
    Checking connectors-client v0.6.5 (~/.local/state/worktree/trees/b10x/connectors/wt-fc49f37347f5/crates/connectors-client)
    Checking catalog-cli v0.6.5 (~/.local/state/worktree/trees/b10x/connectors/wt-fc49f37347f5/crates/catalog-cli)
    Checking catalog-reader v0.6.5 (~/.local/state/worktree/trees/b10x/connectors/wt-fc49f37347f5/crates/catalog-reader)
    Finished `dev` profile [unoptimized] target(s) in 30.59s

exit 0
```

```text
$ cargo fmt --manifest-path crates/server/Cargo.toml --all -- --check

exit 0
```

```text
$ cargo clippy --manifest-path crates/connectors-cli/Cargo.toml --workspace --all-targets --locked -- -D warnings
warning: connector-secrets@0.6.5: no live Vault was offered (CONNECTOR_SECRETS_VAULT_ADDR and CONNECTOR_SECRETS_VAULT_TOKEN are unset), so the reqwest HttpTransport is UNEXERCISED by this build: tests/vault_live.rs is compiled #[ignore]d
    Checking connectors-cli v0.6.5 (~/.local/state/worktree/trees/b10x/connectors/wt-fc49f37347f5/crates/connectors-cli)
    Finished `dev` profile [unoptimized] target(s) in 0.72s

exit 0
```

```text
$ cargo fmt --manifest-path crates/connectors-cli/Cargo.toml --all -- --check

exit 0
```

4. Finding table, measurement and reachability

| File:line | Category | Severity | Verdict | Origin | Finding |
|---|---|---|---|---|---|
| ess/system/domains/connection.yaml:606 | contract-drift | blocker | CONFIRMED | undecided | MaterializeObservation still cites the old Monitoring materialize/refusal line locations, so the complete root workspace's required ESS refusal-coverage fence fails. |
| crates/connectors-cli/tests/one_shot_operations.rs:695 (initial test captured in cases-before-execution.patch) | boundary | note | INFEASIBLE | undecided | The initial extra-key probe assumes work.requests.list forbids additional properties, although its advertised schema permits them; the observed configured-socket request does not establish caller-controlled route or header authority. |

CONFIRMED measurement: crates/catalog-build/tests/main/ess_claim_fence.rs:448 fails in the complete root suite and again in the exact single-test reproduction, both exit 101. The declaration points to `fn materialize` at1177 and current-state refusal1184-1189; current source is crates/integration-monitoring/src/backend.rs:1182 and refusal1190-1194. What reaches it: the repository's mandatory root `cargo test --workspace --locked` lane and scripts/gate.sh's root workspace shard, without a synthetic configuration or provider request. It is reachable in normal validation. The source diff adds exactly five lines above this function and does not change ess/system/domains/connection.yaml. This is source evidence for coordinator origin routing, not a claim that the base was executed. Refresh all affected source citations without changing the declared refusal semantics, and rerun the actual fence.

INFEASIBLE measurement: the initial test's line695 asserts no request reached the fixture listener, while that listener receives the operation request; the exact first red is retained above, exit101. What reaches it: the fixture explicitly adds unknown keys to a schema that permits them. No path to attacker-controlled destination or Authorization was established; the corrected real invocation verifies both remain fixture-owned and passes. This row disposes of the failed theory and is not a production blocker. It has no valid base reproduction or origin claim.

Findings cover source970e4af56f7a4ca1b9689c885fabd3a519bbe0ca with the test states identified above. No source fix or ESS citation correction was performed by this adversary, and no AEP calls were made. The coordinator owns correction, origin routing and all publication/closure.

5. Attacks that did not break the contract

- Separate real describe/invoke processes retain the ordinary registry description/Connection authority, issue exactly one fixture request, and publish no control socket; inherited acceptance and new transport cases pass.
- Omitted/default and explicit local routes, saved hosted-login isolation and explicit hosted refusal retain their expected target; inherited and new process cases pass.
- EOF, invalid JSON and wrong correlation after a transmitted invoke/socket deletion never cause ephemeral resend; three deciding scenarios pass.
- Existing file/symlink/stale-socket objects, unsafe state/lock paths, live command/daemon contention and a socket published after the absence probe refuse without journal creation or socket removal; inherited and new tests pass.
- Shutdown holds the state lock until an asynchronous backend teardown barrier is released, on success and policy refusal; the new barrier test passes.
- Unknown or ambiguous registry owners remain refused even when backends advertise bounded lifetime; the new synthetic boundary test passes and all owners shut down.
- All session-control, Connection activation/materialization/acquisition variants, browser canonical/published aliases, and absent-daemon event paths retain named daemon requirements; the new class test and inherited tests pass.
- Missing/malformed/oversize/conflicting JSON sources and target conflicts fail before local runtime state appears; new cases pass. Permitted extra input properties are not elevated into route/header authority, and old description references stop at a changed Grant.
- Distinct named catalog instances retain exact fixture credential selection; read-only writes remain refused before egress. Existing runtime coverage passes unchanged. The held credential-unit source was not imported.
- Existing Slack tests confirm ephemeral construction does not start Socket Mode supervision, and shared registry shutdown executes. No live provider, operator credential/configuration, daemon restart, external message or new child agent was used.

Limits: this pass did not inject cancellation into a running embedding API or execute a base checkout, and it does not claim to prove every possible process scheduling interleaving. The socket-publication and ambiguous-owner cases deliberately construct their respective boundary conditions; successful refusal is what was measured. The eight added cases use existing automatically selected CLI/runtime integration-test files, with no module-hookup or production seam required.

6. Outside-worktree write inventory

The four namespaces are:

- ~/.cache/connectors-cli-wave-20260906/one-shot-operations-without-a-daemon/adversary-1 — assigned reports, raw logs, exit files, original/final patches, correction patch, assembly script, marker, inventory and report checksums.
- ~/.cache/cw6/o — assigned TMPDIR; compiler and test-created fixtures, including new shot-<pid>-<counter> fixtures and runtime tempfile directories. Existing tests also retain synthetic configuration/login fixtures here.
- ~/.cache/sccache — shared tool-managed compiler cache; sccache reports this exact cache root. Its daemon also serves other agents, so mtime observations are an over-inclusive write inventory, not exclusive attribution.
- ~/.cargo — Cargo-managed registry/cache and lock paths used by Cargo. The inventory records observed changes, while ordinary lock-file access may leave no mtime change.

In-tree compiler writes are limited to target/, crates/connectors-cli/target/, crates/connectors-runtime/target/ and crates/connectors-console/target/ in the assigned tree. The server lane uses target/ because server is a member of the root workspace. CARGO_TARGET_DIR remained unset. No compiler outputs were moved or deleted.

The test APIs remove their own ephemeral fixtures on normal completion/panic. Retrospective metadata enumeration cannot recover paths that were already removed; their exact parent namespace and test naming rules are given above, and that limitation is explicit. The retained mtime inventory below covers all observed paths newer than start.marker in the temporary/cache namespaces. It can include concurrent shared-cache writes and is not used to claim exclusive attribution. No existing operator configuration or secret file contents were inspected.

Every report-directory path (brief.md predates this pass; included for completeness):

```text
~/.cache/connectors-cli-wave-20260906/one-shot-operations-without-a-daemon/adversary-1/adversary_caller_route_injection_and_revoked_grant_stop_before_fixture_egress.exit
~/.cache/connectors-cli-wave-20260906/one-shot-operations-without-a-daemon/adversary-1/adversary_caller_route_injection_and_revoked_grant_stop_before_fixture_egress.log
~/.cache/connectors-cli-wave-20260906/one-shot-operations-without-a-daemon/adversary-1/adversary_every_persistent_request_class_refuses_before_configuration_or_state.exit
~/.cache/connectors-cli-wave-20260906/one-shot-operations-without-a-daemon/adversary-1/adversary_every_persistent_request_class_refuses_before_configuration_or_state.log
~/.cache/connectors-cli-wave-20260906/one-shot-operations-without-a-daemon/adversary-1/adversary_hosted_refusal_and_target_conflict_never_construct_the_local_runtime.exit
~/.cache/connectors-cli-wave-20260906/one-shot-operations-without-a-daemon/adversary-1/adversary_hosted_refusal_and_target_conflict_never_construct_the_local_runtime.log
~/.cache/connectors-cli-wave-20260906/one-shot-operations-without-a-daemon/adversary-1/adversary_json_source_and_size_refusals_precede_one_shot_state_creation.exit
~/.cache/connectors-cli-wave-20260906/one-shot-operations-without-a-daemon/adversary-1/adversary_json_source_and_size_refusals_precede_one_shot_state_creation.log
~/.cache/connectors-cli-wave-20260906/one-shot-operations-without-a-daemon/adversary-1/adversary_lifetime_capability_never_admits_unknown_or_ambiguous_owners.exit
~/.cache/connectors-cli-wave-20260906/one-shot-operations-without-a-daemon/adversary-1/adversary_lifetime_capability_never_admits_unknown_or_ambiguous_owners.log
~/.cache/connectors-cli-wave-20260906/one-shot-operations-without-a-daemon/adversary-1/adversary_socket_publication_after_absence_probe_is_preserved_and_refused.exit
~/.cache/connectors-cli-wave-20260906/one-shot-operations-without-a-daemon/adversary-1/adversary_socket_publication_after_absence_probe_is_preserved_and_refused.log
~/.cache/connectors-cli-wave-20260906/one-shot-operations-without-a-daemon/adversary-1/adversary_state_lock_outlives_async_shutdown_on_success_and_refusal.exit
~/.cache/connectors-cli-wave-20260906/one-shot-operations-without-a-daemon/adversary-1/adversary_state_lock_outlives_async_shutdown_on_success_and_refusal.log
~/.cache/connectors-cli-wave-20260906/one-shot-operations-without-a-daemon/adversary-1/assemble-report.sh
~/.cache/connectors-cli-wave-20260906/one-shot-operations-without-a-daemon/adversary-1/brief.md
~/.cache/connectors-cli-wave-20260906/one-shot-operations-without-a-daemon/adversary-1/cases-before-execution.patch
~/.cache/connectors-cli-wave-20260906/one-shot-operations-without-a-daemon/adversary-1/corrected-route-authority.exit
~/.cache/connectors-cli-wave-20260906/one-shot-operations-without-a-daemon/adversary-1/corrected-route-authority.log
~/.cache/connectors-cli-wave-20260906/one-shot-operations-without-a-daemon/adversary-1/deciding-invoke.exit
~/.cache/connectors-cli-wave-20260906/one-shot-operations-without-a-daemon/adversary-1/deciding-invoke.log
~/.cache/connectors-cli-wave-20260906/one-shot-operations-without-a-daemon/adversary-1/final-tests-only.patch
~/.cache/connectors-cli-wave-20260906/one-shot-operations-without-a-daemon/adversary-1/final-tests-only.stat
~/.cache/connectors-cli-wave-20260906/one-shot-operations-without-a-daemon/adversary-1/full-cli-clippy.exit
~/.cache/connectors-cli-wave-20260906/one-shot-operations-without-a-daemon/adversary-1/full-cli-clippy.log
~/.cache/connectors-cli-wave-20260906/one-shot-operations-without-a-daemon/adversary-1/full-cli-fmt.exit
~/.cache/connectors-cli-wave-20260906/one-shot-operations-without-a-daemon/adversary-1/full-cli-fmt.log
~/.cache/connectors-cli-wave-20260906/one-shot-operations-without-a-daemon/adversary-1/full-cli-tests.exit
~/.cache/connectors-cli-wave-20260906/one-shot-operations-without-a-daemon/adversary-1/full-cli-tests.log
~/.cache/connectors-cli-wave-20260906/one-shot-operations-without-a-daemon/adversary-1/full-connectors-console-clippy.exit
~/.cache/connectors-cli-wave-20260906/one-shot-operations-without-a-daemon/adversary-1/full-connectors-console-clippy.log
~/.cache/connectors-cli-wave-20260906/one-shot-operations-without-a-daemon/adversary-1/full-connectors-console-fmt.exit
~/.cache/connectors-cli-wave-20260906/one-shot-operations-without-a-daemon/adversary-1/full-connectors-console-fmt.log
~/.cache/connectors-cli-wave-20260906/one-shot-operations-without-a-daemon/adversary-1/full-connectors-console-tests.exit
~/.cache/connectors-cli-wave-20260906/one-shot-operations-without-a-daemon/adversary-1/full-connectors-console-tests.log
~/.cache/connectors-cli-wave-20260906/one-shot-operations-without-a-daemon/adversary-1/full-runtime-clippy.exit
~/.cache/connectors-cli-wave-20260906/one-shot-operations-without-a-daemon/adversary-1/full-runtime-clippy.log
~/.cache/connectors-cli-wave-20260906/one-shot-operations-without-a-daemon/adversary-1/full-runtime-fmt.exit
~/.cache/connectors-cli-wave-20260906/one-shot-operations-without-a-daemon/adversary-1/full-runtime-fmt.log
~/.cache/connectors-cli-wave-20260906/one-shot-operations-without-a-daemon/adversary-1/full-runtime-tests.exit
~/.cache/connectors-cli-wave-20260906/one-shot-operations-without-a-daemon/adversary-1/full-runtime-tests.log
~/.cache/connectors-cli-wave-20260906/one-shot-operations-without-a-daemon/adversary-1/full-server-clippy.exit
~/.cache/connectors-cli-wave-20260906/one-shot-operations-without-a-daemon/adversary-1/full-server-clippy.log
~/.cache/connectors-cli-wave-20260906/one-shot-operations-without-a-daemon/adversary-1/full-server-fmt.exit
~/.cache/connectors-cli-wave-20260906/one-shot-operations-without-a-daemon/adversary-1/full-server-fmt.log
~/.cache/connectors-cli-wave-20260906/one-shot-operations-without-a-daemon/adversary-1/full-server-tests.exit
~/.cache/connectors-cli-wave-20260906/one-shot-operations-without-a-daemon/adversary-1/full-server-tests.log
~/.cache/connectors-cli-wave-20260906/one-shot-operations-without-a-daemon/adversary-1/invalid-probe-correction.patch
~/.cache/connectors-cli-wave-20260906/one-shot-operations-without-a-daemon/adversary-1/outside-retained-inventory.txt
~/.cache/connectors-cli-wave-20260906/one-shot-operations-without-a-daemon/adversary-1/raw-report.md
~/.cache/connectors-cli-wave-20260906/one-shot-operations-without-a-daemon/adversary-1/root-fence-reproduction.exit
~/.cache/connectors-cli-wave-20260906/one-shot-operations-without-a-daemon/adversary-1/root-fence-reproduction.log
~/.cache/connectors-cli-wave-20260906/one-shot-operations-without-a-daemon/adversary-1/start.marker
~/.cache/connectors-cli-wave-20260906/one-shot-operations-without-a-daemon/adversary-1/report.md
~/.cache/connectors-cli-wave-20260906/one-shot-operations-without-a-daemon/adversary-1/reports.sha256
```

Retained temporary/tool-cache inventory:

```text
~/.cache/cw6/o
~/.cache/cw6/o/t24de5f1
~/.cache/cw6/o/t24de5f1/c
~/.cache/cw6/o/t24de5f1/c/b10x
~/.cache/cw6/o/t24de5f1/c/b10x/connectors.toml
~/.cache/cw6/o/t24de5f22
~/.cache/cw6/o/t24de5f22/c
~/.cache/cw6/o/t24de5f22/c/b10x
~/.cache/cw6/o/t24de5f22/c/b10x/connectors.toml
~/.cache/cw6/o/t24de5f22/s
~/.cache/cw6/o/t24de5f22/s/b10x
~/.cache/cw6/o/t24de5f22/s/b10x/connectors
~/.cache/cw6/o/t24de5f22/s/b10x/connectors/identity-sessions.json
~/.cache/cw6/o/t24de5f22/s/b10x/connectors/connectors.sock
~/.cache/cw6/o/t24de5f10
~/.cache/cw6/o/t24de5f10/c
~/.cache/cw6/o/t24de5f10/c/b10x
~/.cache/cw6/o/t24de5f10/c/b10x/connectors.toml
~/.cache/cw6/o/t24de5f10/s
~/.cache/cw6/o/t24de5f10/s/b10x
~/.cache/cw6/o/t24de5f10/s/b10x/connectors
~/.cache/cw6/o/t24de5f10/s/b10x/connectors/identity-sessions.json
~/.cache/cw6/o/t24de5f10/s/b10x/connectors/connectors.sock
~/.cache/cw6/o/t24de5f1f
~/.cache/cw6/o/t24de5f1f/c
~/.cache/cw6/o/t24de5f1f/c/b10x
~/.cache/cw6/o/t24de5f1f/c/b10x/connectors.toml
~/.cache/cw6/o/t24de5f1f/s
~/.cache/cw6/o/t24de5f1f/s/b10x
~/.cache/cw6/o/t24de5f1f/s/b10x/connectors
~/.cache/cw6/o/t24de5f1f/s/b10x/connectors/identity-sessions.json
~/.cache/cw6/o/t24de5f1f/s/b10x/connectors/connectors.sock
~/.cache/cw6/o/t24de5f6
~/.cache/cw6/o/t24de5f6/c
~/.cache/cw6/o/t24de5f6/c/b10x
~/.cache/cw6/o/t24de5f6/c/b10x/connectors.toml
~/.cache/cw6/o/t24de5f6/s
~/.cache/cw6/o/t24de5f6/s/b10x
~/.cache/cw6/o/t24de5f6/s/b10x/connectors
~/.cache/cw6/o/t24de5f6/s/b10x/connectors/identity-sessions.json
~/.cache/cw6/o/t24de5f6/connectors.sock
~/.cache/cw6/o/t24de5fb
~/.cache/cw6/o/t24de5fb/c
~/.cache/cw6/o/t24de5fb/c/b10x
~/.cache/cw6/o/t24de5fb/c/b10x/connectors.toml
~/.cache/cw6/o/t24de5fa
~/.cache/cw6/o/t24de5fa/c
~/.cache/cw6/o/t24de5fa/c/b10x
~/.cache/cw6/o/t24de5fa/c/b10x/connectors.toml
~/.cache/cw6/o/t24de5f1b
~/.cache/cw6/o/t24de5f1b/c
~/.cache/cw6/o/t24de5f1b/c/b10x
~/.cache/cw6/o/t24de5f1b/c/b10x/connectors.toml
~/.cache/cw6/o/t24de5f1b/s
~/.cache/cw6/o/t24de5f1b/s/b10x
~/.cache/cw6/o/t24de5f1b/s/b10x/connectors
~/.cache/cw6/o/t24de5f1b/s/b10x/connectors/identity-sessions.json
~/.cache/cw6/o/t24de5f1b/s/b10x/connectors/connectors.sock
~/.cache/cw6/o/t24de5f2
~/.cache/cw6/o/t24de5f2/c
~/.cache/cw6/o/t24de5f2/c/b10x
~/.cache/cw6/o/t24de5f2/c/b10x/connectors.toml
~/.cache/cw6/o/t24de5f2/s
~/.cache/cw6/o/t24de5f2/s/b10x
~/.cache/cw6/o/t24de5f2/s/b10x/connectors
~/.cache/cw6/o/t24de5f2/s/b10x/connectors/event-reply-claims.sqlite
~/.cache/cw6/o/t24de5f2/s/b10x/connectors/credentials.store
~/.cache/cw6/o/t24de5f2/s/b10x/connectors/identity-sessions.json
~/.cache/cw6/o/t24de5f2/s/b10x/connectors/.connectors.lock
~/.cache/cw6/o/t24de5f2/s/b10x/connectors/.credentials.store.lease
~/.cache/cw6/o/t24de5f7
~/.cache/cw6/o/t24de5f7/c
~/.cache/cw6/o/t24de5f7/c/b10x
~/.cache/cw6/o/t24de5f7/c/b10x/connectors.toml
~/.cache/cw6/o/t24de5f7/s
~/.cache/cw6/o/t24de5f7/s/b10x
~/.cache/cw6/o/t24de5f7/s/b10x/connectors
~/.cache/cw6/o/t24de5f7/s/b10x/connectors/identity-sessions.json
~/.cache/cw6/o/t24de5f7/s/b10x/connectors/connectors.sock
~/.cache/cw6/o/t24de5f16
~/.cache/cw6/o/t24de5f16/c
~/.cache/cw6/o/t24de5f16/c/b10x
~/.cache/cw6/o/t24de5f16/c/b10x/connectors.toml
~/.cache/cw6/o/t24de5f16/s
~/.cache/cw6/o/t24de5f16/s/b10x
~/.cache/cw6/o/t24de5f16/s/b10x/connectors
~/.cache/cw6/o/t24de5f16/s/b10x/connectors/identity-sessions.json
~/.cache/cw6/o/t24de5f16/connectors.sock
~/.cache/cw6/o/t24de5f4
~/.cache/cw6/o/t24de5f4/c
~/.cache/cw6/o/t24de5f4/c/b10x
~/.cache/cw6/o/t24de5f4/c/b10x/connectors.toml
~/.cache/cw6/o/t24de5f4/s
~/.cache/cw6/o/t24de5f4/s/b10x
~/.cache/cw6/o/t24de5f4/s/b10x/connectors
~/.cache/cw6/o/t24de5f4/s/b10x/connectors/identity-sessions.json
~/.cache/cw6/o/t24de5f4/s/b10x/connectors/connectors.sock
~/.cache/cw6/o/t24de5f1c
~/.cache/cw6/o/t24de5f1c/c
~/.cache/cw6/o/t24de5f1c/c/b10x
~/.cache/cw6/o/t24de5f1c/c/b10x/connectors.toml
~/.cache/cw6/o/t24de5f1c/s
~/.cache/cw6/o/t24de5f1c/s/b10x
~/.cache/cw6/o/t24de5f1c/s/b10x/connectors
~/.cache/cw6/o/t24de5f1c/s/b10x/connectors/identity-sessions.json
~/.cache/cw6/o/t24de5f1c/s/b10x/connectors/connectors.sock
~/.cache/cw6/o/t24de5f27
~/.cache/cw6/o/t24de5f27/c
~/.cache/cw6/o/t24de5f27/c/b10x
~/.cache/cw6/o/t24de5f27/c/b10x/connectors.toml
~/.cache/cw6/o/t24de5f27/s
~/.cache/cw6/o/t24de5f27/s/b10x
~/.cache/cw6/o/t24de5f27/s/b10x/connectors
~/.cache/cw6/o/t24de5f27/s/b10x/connectors/identity-sessions.json
~/.cache/cw6/o/t24de5f27/s/b10x/connectors/connectors.sock
~/.cache/cw6/o/t24de5f18
~/.cache/cw6/o/t24de5f18/c
~/.cache/cw6/o/t24de5f18/c/b10x
~/.cache/cw6/o/t24de5f18/c/b10x/connectors.toml
~/.cache/cw6/o/t24de5f18/s
~/.cache/cw6/o/t24de5f18/s/b10x
~/.cache/cw6/o/t24de5f18/s/b10x/connectors
~/.cache/cw6/o/t24de5f18/s/b10x/connectors/identity-sessions.json
~/.cache/cw6/o/t24de5f18/connectors.sock
~/.cache/cw6/o/t24de5f0
~/.cache/cw6/o/t24de5f0/c
~/.cache/cw6/o/t24de5f0/c/b10x
~/.cache/cw6/o/t24de5f0/c/b10x/connectors.toml
~/.cache/cw6/o/t24de5f0/s
~/.cache/cw6/o/t24de5f0/s/b10x
~/.cache/cw6/o/t24de5f0/s/b10x/connectors
~/.cache/cw6/o/t24de5f0/s/b10x/connectors/identity-sessions.json
~/.cache/cw6/o/t24de5f12
~/.cache/cw6/o/t24de5f12/c
~/.cache/cw6/o/t24de5f12/c/b10x
~/.cache/cw6/o/t24de5f12/c/b10x/connectors.toml
~/.cache/cw6/o/t24de5f12/s
~/.cache/cw6/o/t24de5f12/s/b10x
~/.cache/cw6/o/t24de5f12/s/b10x/connectors
~/.cache/cw6/o/t24de5f12/s/b10x/connectors/identity-sessions.json
~/.cache/cw6/o/t24de5f12/s/b10x/connectors/connectors.sock
~/.cache/cw6/o/t24de5f21
~/.cache/cw6/o/t24de5f21/c
~/.cache/cw6/o/t24de5f21/c/b10x
~/.cache/cw6/o/t24de5f21/c/b10x/connectors.toml
~/.cache/cw6/o/t24de5f21/s
~/.cache/cw6/o/t24de5f21/s/b10x
~/.cache/cw6/o/t24de5f21/s/b10x/connectors
~/.cache/cw6/o/t24de5f21/s/b10x/connectors/identity-sessions.json
~/.cache/cw6/o/t24de5f21/s/b10x/connectors/connectors.sock
~/.cache/cw6/o/t24de5f28
~/.cache/cw6/o/t24de5f28/c
~/.cache/cw6/o/t24de5f28/c/b10x
~/.cache/cw6/o/t24de5f28/c/b10x/connectors.toml
~/.cache/cw6/o/t24de5f28/s
~/.cache/cw6/o/t24de5f28/s/b10x
~/.cache/cw6/o/t24de5f28/s/b10x/connectors
~/.cache/cw6/o/t24de5f28/s/b10x/connectors/identity-sessions.json
~/.cache/cw6/o/t24de5f28/s/b10x/connectors/connectors.sock
~/.cache/cw6/o/t24de5f17
~/.cache/cw6/o/t24de5f17/c
~/.cache/cw6/o/t24de5f17/c/b10x
~/.cache/cw6/o/t24de5f17/c/b10x/connectors.toml
~/.cache/cw6/o/t24de5f17/s
~/.cache/cw6/o/t24de5f17/s/b10x
~/.cache/cw6/o/t24de5f17/s/b10x/connectors
~/.cache/cw6/o/t24de5f17/s/b10x/connectors/identity-sessions.json
~/.cache/cw6/o/t24de5f17/s/b10x/connectors/connectors.sock
~/.cache/cw6/o/t24de5f19
~/.cache/cw6/o/t24de5f19/c
~/.cache/cw6/o/t24de5f19/c/b10x
~/.cache/cw6/o/t24de5f19/c/b10x/connectors.toml
~/.cache/cw6/o/t24de5f19/s
~/.cache/cw6/o/t24de5f19/s/b10x
~/.cache/cw6/o/t24de5f19/s/b10x/connectors
~/.cache/cw6/o/t24de5f19/s/b10x/connectors/identity-sessions.json
~/.cache/cw6/o/t24de5f19/s/b10x/connectors/connectors.sock
~/.cache/cw6/o/t24de5fd
~/.cache/cw6/o/t24de5fd/c
~/.cache/cw6/o/t24de5fd/c/b10x
~/.cache/cw6/o/t24de5fd/c/b10x/connectors.toml
~/.cache/cw6/o/t24de5fd/s
~/.cache/cw6/o/t24de5fd/s/b10x
~/.cache/cw6/o/t24de5fd/s/b10x/connectors
~/.cache/cw6/o/t24de5fd/s/b10x/connectors/identity-sessions.json
~/.cache/cw6/o/t24de5fd/s/b10x/connectors/connectors.sock
~/.cache/cw6/o/t24de5f8
~/.cache/cw6/o/t24de5f8/c
~/.cache/cw6/o/t24de5f8/c/b10x
~/.cache/cw6/o/t24de5f8/c/b10x/connectors.toml
~/.cache/cw6/o/t24de5f2a
~/.cache/cw6/o/t24de5f2a/c
~/.cache/cw6/o/t24de5f2a/c/b10x
~/.cache/cw6/o/t24de5f2a/c/b10x/connectors.toml
~/.cache/cw6/o/t24de5f2a/s
~/.cache/cw6/o/t24de5f2a/s/b10x
~/.cache/cw6/o/t24de5f2a/s/b10x/connectors
~/.cache/cw6/o/t24de5f2a/s/b10x/connectors/identity-sessions.json
~/.cache/cw6/o/t24de5f2a/s/b10x/connectors/connectors.sock
~/.cache/cw6/o/t24de5f24
~/.cache/cw6/o/t24de5f24/c
~/.cache/cw6/o/t24de5f24/c/b10x
~/.cache/cw6/o/t24de5f24/c/b10x/connectors.toml
~/.cache/cw6/o/t24de5f24/s
~/.cache/cw6/o/t24de5f24/s/b10x
~/.cache/cw6/o/t24de5f24/s/b10x/connectors
~/.cache/cw6/o/t24de5f24/s/b10x/connectors/identity-sessions.json
~/.cache/cw6/o/t24de5f24/s/b10x/connectors/connectors.sock
~/.cache/cw6/o/t24de5f5
~/.cache/cw6/o/t24de5f5/c
~/.cache/cw6/o/t24de5f5/c/b10x
~/.cache/cw6/o/t24de5f5/c/b10x/connectors.toml
~/.cache/cw6/o/t24de5f1e
~/.cache/cw6/o/t24de5f1e/c
~/.cache/cw6/o/t24de5f1e/c/b10x
~/.cache/cw6/o/t24de5f1e/c/b10x/connectors.toml
~/.cache/cw6/o/t24de5f1e/s
~/.cache/cw6/o/t24de5f1e/s/b10x
~/.cache/cw6/o/t24de5f1e/s/b10x/connectors
~/.cache/cw6/o/t24de5f1e/s/b10x/connectors/identity-sessions.json
~/.cache/cw6/o/t24de5f1e/s/b10x/connectors/connectors.sock
~/.cache/cw6/o/t24de5f9
~/.cache/cw6/o/t24de5f9/c
~/.cache/cw6/o/t24de5f9/c/b10x
~/.cache/cw6/o/t24de5f9/c/b10x/connectors.toml
~/.cache/cw6/o/t24de5fe
~/.cache/cw6/o/t24de5fe/c
~/.cache/cw6/o/t24de5fe/c/b10x
~/.cache/cw6/o/t24de5fe/c/b10x/connectors.toml
~/.cache/cw6/o/t24de5fe/s
~/.cache/cw6/o/t24de5fe/s/b10x
~/.cache/cw6/o/t24de5fe/s/b10x/connectors
~/.cache/cw6/o/t24de5fe/s/b10x/connectors/identity-sessions.json
~/.cache/cw6/o/t24de5fe/connectors.sock
~/.cache/cw6/o/t24de5f1d
~/.cache/cw6/o/t24de5f1d/c
~/.cache/cw6/o/t24de5f1d/c/b10x
~/.cache/cw6/o/t24de5f1d/c/b10x/connectors.toml
~/.cache/cw6/o/t24de5f1d/s
~/.cache/cw6/o/t24de5f1d/s/b10x
~/.cache/cw6/o/t24de5f1d/s/b10x/connectors
~/.cache/cw6/o/t24de5f1d/s/b10x/connectors/identity-sessions.json
~/.cache/cw6/o/t24de5f1d/s/b10x/connectors/connectors.sock
~/.cache/cw6/o/t24de5f14
~/.cache/cw6/o/t24de5f14/c
~/.cache/cw6/o/t24de5f14/c/b10x
~/.cache/cw6/o/t24de5f14/c/b10x/connectors.toml
~/.cache/cw6/o/t24de5f14/s
~/.cache/cw6/o/t24de5f14/s/b10x
~/.cache/cw6/o/t24de5f14/s/b10x/connectors
~/.cache/cw6/o/t24de5f14/s/b10x/connectors/identity-sessions.json
~/.cache/cw6/o/t24de5f14/connectors.sock
~/.cache/cw6/o/t24de5f11
~/.cache/cw6/o/t24de5f11/c
~/.cache/cw6/o/t24de5f11/c/b10x
~/.cache/cw6/o/t24de5f11/c/b10x/connectors.toml
~/.cache/cw6/o/t24de5f11/s
~/.cache/cw6/o/t24de5f11/s/b10x
~/.cache/cw6/o/t24de5f11/s/b10x/connectors
~/.cache/cw6/o/t24de5f11/s/b10x/connectors/identity-sessions.json
~/.cache/cw6/o/t24de5f11/connectors.sock
~/.cache/cw6/o/t24de5f15
~/.cache/cw6/o/t24de5f15/c
~/.cache/cw6/o/t24de5f15/c/b10x
~/.cache/cw6/o/t24de5f15/c/b10x/connectors.toml
~/.cache/cw6/o/t24de5f15/s
~/.cache/cw6/o/t24de5f15/s/b10x
~/.cache/cw6/o/t24de5f15/s/b10x/connectors
~/.cache/cw6/o/t24de5f15/s/b10x/connectors/identity-sessions.json
~/.cache/cw6/o/t24de5f15/s/b10x/connectors/connectors.sock
~/.cache/cw6/o/t24de5f1a
~/.cache/cw6/o/t24de5f1a/c
~/.cache/cw6/o/t24de5f1a/c/b10x
~/.cache/cw6/o/t24de5f1a/c/b10x/connectors.toml
~/.cache/cw6/o/t24de5f1a/s
~/.cache/cw6/o/t24de5f1a/s/b10x
~/.cache/cw6/o/t24de5f1a/s/b10x/connectors
~/.cache/cw6/o/t24de5f1a/s/b10x/connectors/identity-sessions.json
~/.cache/cw6/o/t24de5f1a/s/b10x/connectors/connectors.sock
~/.cache/cw6/o/t24de5f20
~/.cache/cw6/o/t24de5f20/c
~/.cache/cw6/o/t24de5f20/c/b10x
~/.cache/cw6/o/t24de5f20/c/b10x/connectors.toml
~/.cache/cw6/o/t24de5f20/s
~/.cache/cw6/o/t24de5f20/s/b10x
~/.cache/cw6/o/t24de5f20/s/b10x/connectors
~/.cache/cw6/o/t24de5f20/s/b10x/connectors/identity-sessions.json
~/.cache/cw6/o/t24de5f20/s/b10x/connectors/connectors.sock
~/.cache/cw6/o/t24de5f13
~/.cache/cw6/o/t24de5f13/c
~/.cache/cw6/o/t24de5f13/c/b10x
~/.cache/cw6/o/t24de5f13/c/b10x/connectors.toml
~/.cache/cw6/o/t24de5f13/s
~/.cache/cw6/o/t24de5f13/s/b10x
~/.cache/cw6/o/t24de5f13/s/b10x/connectors
~/.cache/cw6/o/t24de5f13/s/b10x/connectors/identity-sessions.json
~/.cache/cw6/o/t24de5f13/s/b10x/connectors/connectors.sock
~/.cache/cw6/o/t24de5f23
~/.cache/cw6/o/t24de5f23/c
~/.cache/cw6/o/t24de5f23/c/b10x
~/.cache/cw6/o/t24de5f23/c/b10x/connectors.toml
~/.cache/cw6/o/t24de5f23/s
~/.cache/cw6/o/t24de5f23/s/b10x
~/.cache/cw6/o/t24de5f23/s/b10x/connectors
~/.cache/cw6/o/t24de5f23/s/b10x/connectors/identity-sessions.json
~/.cache/cw6/o/t24de5f23/s/b10x/connectors/connectors.sock
~/.cache/cw6/o/t24de5f3
~/.cache/cw6/o/t24de5f3/c
~/.cache/cw6/o/t24de5f3/c/b10x
~/.cache/cw6/o/t24de5f3/c/b10x/connectors.toml
~/.cache/cw6/o/t24de5f3/s
~/.cache/cw6/o/t24de5f3/s/b10x
~/.cache/cw6/o/t24de5f3/s/b10x/connectors
~/.cache/cw6/o/t24de5f3/s/b10x/connectors/identity-sessions.json
~/.cache/cw6/o/t24de5f3/s/b10x/connectors/connectors.sock
~/.cache/cw6/o/t24de5ff
~/.cache/cw6/o/t24de5ff/c
~/.cache/cw6/o/t24de5ff/c/b10x
~/.cache/cw6/o/t24de5ff/c/b10x/connectors.toml
~/.cache/cw6/o/t24de5ff/s
~/.cache/cw6/o/t24de5ff/s/b10x
~/.cache/cw6/o/t24de5ff/s/b10x/connectors
~/.cache/cw6/o/t24de5ff/s/b10x/connectors/identity-sessions.json
~/.cache/cw6/o/t24de5ff/s/b10x/connectors/connectors.sock
~/.cache/cw6/o/t24de5fc
~/.cache/cw6/o/t24de5fc/c
~/.cache/cw6/o/t24de5fc/c/b10x
~/.cache/cw6/o/t24de5fc/c/b10x/connectors.toml
~/.cache/cw6/o/t24de5fc/s
~/.cache/cw6/o/t24de5fc/s/b10x
~/.cache/cw6/o/t24de5fc/s/b10x/connectors
~/.cache/cw6/o/t24de5fc/s/b10x/connectors/identity-sessions.json
~/.cache/cw6/o/t24de5fc/s/b10x/connectors/connectors.sock
~/.cache/cw6/o/t24de5f29
~/.cache/cw6/o/t24de5f29/c
~/.cache/cw6/o/t24de5f29/c/b10x
~/.cache/cw6/o/t24de5f29/c/b10x/connectors.toml
~/.cache/cw6/o/t24de5f29/s
~/.cache/cw6/o/t24de5f29/s/b10x
~/.cache/cw6/o/t24de5f29/s/b10x/connectors
~/.cache/cw6/o/t24de5f29/s/b10x/connectors/identity-sessions.json
~/.cache/cw6/o/t24de5f29/s/b10x/connectors/connectors.sock
~/.cache/cw6/o/t24de5f25
~/.cache/cw6/o/t24de5f25/c
~/.cache/cw6/o/t24de5f25/c/b10x
~/.cache/cw6/o/t24de5f25/c/b10x/connectors.toml
~/.cache/cw6/o/t24de5f25/s
~/.cache/cw6/o/t24de5f25/s/b10x
~/.cache/cw6/o/t24de5f25/s/b10x/connectors
~/.cache/cw6/o/t24de5f25/s/b10x/connectors/identity-sessions.json
~/.cache/cw6/o/t24de5f25/s/b10x/connectors/connectors.sock
~/.cache/cw6/o/t24de5f26
~/.cache/cw6/o/t24de5f26/c
~/.cache/cw6/o/t24de5f26/c/b10x
~/.cache/cw6/o/t24de5f26/c/b10x/connectors.toml
~/.cache/cw6/o/t24de5f26/s
~/.cache/cw6/o/t24de5f26/s/b10x
~/.cache/cw6/o/t24de5f26/s/b10x/connectors
~/.cache/cw6/o/t24de5f26/s/b10x/connectors/identity-sessions.json
~/.cache/cw6/o/t24de5f26/s/b10x/connectors/connectors.sock
~/.cache/sccache
~/.cache/sccache/d/d
~/.cache/sccache/d/d/dd2fec692d04fb5060b26a720c5bdf1f46d3f331950630bd31d9dce3f090acc8
~/.cache/sccache/d/d/dd4af401eb618f1eb62e5ae9064f4e568c31fde832ac5112bcdb9e37b9f2e2d3
~/.cache/sccache/d/d/dd6bae42504bf3284d53bf89d7e18f89adf6318cd3e4438449d6aad00229e85f
~/.cache/sccache/d/d/dd572755e7494d0715a46d90bc72781aab3509c9b1e4ef1dee8f2affd5425f56
~/.cache/sccache/d/0
~/.cache/sccache/d/0/d0c05d433b724cea17162d587522cdad7096671d8a61b72f9ec722de5a1284f9
~/.cache/sccache/d/0/d0b5cee02808da0187cf2fcd3a759853debbeb8f213174fa6a45cab12d7294dc
~/.cache/sccache/d/0/d0347f18293a26ea0722633e23129bd02701e89c2110fc1c70630f29191eb282
~/.cache/sccache/d/0/d0fee73551f241bed28a685d26ae25da6f0f91cb83afb31cc7b1aaf588e62a98
~/.cache/sccache/d/0/d0c065418b92fc27a94b0d789453db30c3d2ca9c9838179061a7458f6703b6ff
~/.cache/sccache/d/2
~/.cache/sccache/d/b
~/.cache/sccache/d/b/db027071ea4bb40982a36e110d04a7b6b5df81dc1da707f7f630ed8c98aadec7
~/.cache/sccache/d/a
~/.cache/sccache/d/a/da61672f5babc3514ddbf3c9d4d5fa2fa564e6aa819497ff252cb061d126f170
~/.cache/sccache/d/a/da512570b0ec1c414e44b9d877acf184d02ee07de88a4c30b6b8ed9cabc5954e
~/.cache/sccache/d/a/daa46d5458bacc2e19a437102e8892dd68ac30dd1d4761c29d9672b9be36ac80
~/.cache/sccache/d/a/da6c1c18a1f071770e31bf13489df0a1acaac46ced7d07036f56672533c1c9ba
~/.cache/sccache/d/c
~/.cache/sccache/d/c/dc0f3ada9e3f76448ceedee7a5e90711ddc10b00106d06c871ef3753ed492ede
~/.cache/sccache/d/c/dc6460160854a36b69b3574ab4f4bec4ae4cbb9dc24defabd4bdea0bab2b6b95
~/.cache/sccache/d/c/dcb1f3bbca30cb969c1ead28d6707fbb8636476d62c2ab04c77b17f31ee9daba
~/.cache/sccache/d/8
~/.cache/sccache/d/8/d8190d045cfd8877bba1953a9bcd340204c6e8d0c49f0bd08d1aefbaf1e692db
~/.cache/sccache/d/8/d893ed524c569bbea85b927aec29637c43af7ea1bbc0fb375167d27bd5fe2ec5
~/.cache/sccache/d/8/d836d6110256c4dc05db25a8e445d94304782133f3386a5df72970bccb3e0cbb
~/.cache/sccache/d/9
~/.cache/sccache/d/9/d949053258e146cabca093b4f50de87e90ff90f55cf84018f3a572339be4395f
~/.cache/sccache/d/9/d91c75ed82b17cc6a1ebb1e7582326ea27d0d76d07071dba328194242f004fe4
~/.cache/sccache/d/9/d9a3f84a9bda6fee1dac9214a136faab0e9a3da9faa0c5b2bb552c1b61287c8e
~/.cache/sccache/d/9/d98630d8afb7202a618f9bc539607115cacb51c47870373971f5d5cf308c5881
~/.cache/sccache/d/9/d9295a4cfce0e9be689fb2ee971fe4a6dd1c93782257683fb1a471b104face70
~/.cache/sccache/d/f
~/.cache/sccache/d/f/df6825c636083028c72e5b65a3b4852862f908b632951236867e74f0f027cc63
~/.cache/sccache/d/f/dfbad73461c359afc323a24617432aac85b804d57e3a23d67a1b8e7f6657afe8
~/.cache/sccache/d/f/df55aef311b4a0be8e91370cb38769c2a92bdb2ffe2ff0f40490f7e6bacaf610
~/.cache/sccache/d/e
~/.cache/sccache/d/e/dec9dfda25a6b5f89758a3d16cf6cda1b2170e1a6c579c9ffb243d9dbc0893ec
~/.cache/sccache/d/e/def4db4a8c947eb911d1302b001cd64c59839a90032dd5b702141e6d63286a8b
~/.cache/sccache/d/5
~/.cache/sccache/d/5/d55b5d51063d38c8160439e5b66f5336b995d461a8b72f2d6167a3e2c78935c9
~/.cache/sccache/d/5/d5378ced4515da795730183be3c72188d90bb5e7aabe428978291ed3c2ae8cbc
~/.cache/sccache/d/5/d594eeff37490f5abe5202e9d1705ddbe44efc98c4d4fb0a92aac1801b8e24ee
~/.cache/sccache/d/5/d54baa0990f1401824443edf81f8d6c1011b31027bfdde19e564b93890503150
~/.cache/sccache/d/3
~/.cache/sccache/d/3/d3b06697cd2adc2515903dc338c16d354dab94a8168e0c591bfeab06b92bfd18
~/.cache/sccache/d/3/d3058c0697dfcf8472eb2f5157e07ef9d446574b455d087a1145a1b0c0f2c9ad
~/.cache/sccache/d/3/d31247bd4249ef510546a2ee4ea03ca72440bcc8752538889bb7cf11a8c47c55
~/.cache/sccache/d/1
~/.cache/sccache/d/1/d1643b0a1f8b3db9190273cbd4c86026ede4eb5c6b447bfca0a471656229608e
~/.cache/sccache/d/1/d11fde2c22a8322c7cf6b70e2840dcafe43dd957cb3c6268a0f234db4d5ecda6
~/.cache/sccache/d/1/d13a1981035c8828ef2bef7e9868f67b723126e7c8a9909c2b86809fa5247d1a
~/.cache/sccache/d/1/d163dbbc78e17051f3bca8ce6fe8535e67705e8ee05c08bee73bc0243e98cbb1
~/.cache/sccache/d/6
~/.cache/sccache/d/6/d66845faecd4e91b02dd0fe2ad7aaca918f2c8edb4f1b32a1dd5c9db8edf6a03
~/.cache/sccache/d/6/d65e47aac0f73a313e118e5c9c79e29605549ab8e289c0bd28130a3bb23ad105
~/.cache/sccache/d/6/d663bc51ffebed5c7f6cb40a48c2915519dbe21f7498974c48426e10caf02416
~/.cache/sccache/d/7
~/.cache/sccache/d/7/d74535e33cdb788a5f2e85d63ce360771dd67cafed9a8aced4666ae5c841f302
~/.cache/sccache/0/d
~/.cache/sccache/0/d/0db6dd5da7a16a3819b0098b12ddda05a6edfe6d23eb9e15b23791fe7669a380
~/.cache/sccache/0/0
~/.cache/sccache/0/0/0023e1515ed926bf25b2a3edd9f5e2fd335454c279da918bf46a711000dc50c7
~/.cache/sccache/0/0/008e7f7c2210a367cf5c1758879e9e3f8d6b5bc587fa94b1ce119d988d0b8f6a
~/.cache/sccache/0/0/00640482feab44e61716ed17c00752e74d22922739b5d94172723083fa2322b7
~/.cache/sccache/0/0/003d9e1ce1628546f833807315855ee0f46358c0ff418d0108c359247bd67f71
~/.cache/sccache/0/0/005f573e817ad97c6c3e3b4185287c2a7e0ccfcc46fb9982010af999734bc2df
~/.cache/sccache/0/2
~/.cache/sccache/0/2/02d9daa61e336f84d515d2311fddf4db0ea0a4aaee30b1d442cfd527e1bdd0b2
~/.cache/sccache/0/b
~/.cache/sccache/0/b/0be7ced2023d4978f9f865ca955c5f3ad749866aa722de9f82677d1af955ff63
~/.cache/sccache/0/b/0bc5135299ae0aee1bf273b2e9f6103b483509159c9de2b82b1648d7accfc28c
~/.cache/sccache/0/a
~/.cache/sccache/0/a/0a73c30275c523ab78e815803d7dce4110d06232903b37f5d33550c8cccd8238
~/.cache/sccache/0/a/0a86aa2670e52b91704234eaad157e74944ff0eb783ba380b49374965445335a
~/.cache/sccache/0/a/0a94a72ac0e0ce03a105d94f2254161f64e82fda28fd0a7a5635df9d63a58558
~/.cache/sccache/0/c
~/.cache/sccache/0/c/0c2b89c29ab1b0099046d62fbd741b00a74fadbc6e8de4400d5ea75d07ab750a
~/.cache/sccache/0/c/0c2f8385aab72d833f911c1a8daee0b48b4b4bbb7d7a23f879dcd217e09df7d3
~/.cache/sccache/0/8
~/.cache/sccache/0/8/087fdab23f224f3d5ab8f0a3a35220d164295358a040c95d3ca5a8d5c175bf67
~/.cache/sccache/0/8/0871cc201d7231d12b2b2de1ffba5ab5e5c66d8f1a03f243f960b4bbe7045cb5
~/.cache/sccache/0/8/0858706b1948be14471746da704e412e406d72b6f62600ce5863acb21ef3483c
~/.cache/sccache/0/9
~/.cache/sccache/0/9/09330e592412fd502fed1bce5aed3f62c679c4d6fe25d3a70f846c4d1cb2888f
~/.cache/sccache/0/9/09b0680817ebdc22bed5de4daf85761919424e6b2360a25524a5f9c96dcfa2e4
~/.cache/sccache/0/9/09963293c47f16c88da7a65341b1ad4b825cf805d1fa71f629cf33ce9f5ee4dd
~/.cache/sccache/0/9/093c2edc7b8f4946831b6e773b2c218a77e080649f6bc46996bc01d57597a1ad
~/.cache/sccache/0/f
~/.cache/sccache/0/f/0f5685025a14828d4e7a8e4c1a1563c59de916ee0cab3653ecba6e413d2d7cc8
~/.cache/sccache/0/f/0f04560b447e99b0e6ed33260822ddbd1ff7834b00f464e7a12db573e0bfb29d
~/.cache/sccache/0/f/0f8e7483a3836732fea7fc8c443e4ce76b2cc152ca4165515096b987b438ad58
~/.cache/sccache/0/5
~/.cache/sccache/0/5/05dc943b3dadee20438b44fdf7d169904a19ec080ff34f66db18e86848dc778f
~/.cache/sccache/0/5/05d370030795c65da6b963383a254136899d4670d49204dafa3b0d6ca127f40f
~/.cache/sccache/0/5/056386e712cde6154f959207130609cc49feb87f6b5d108687061684397ec829
~/.cache/sccache/0/3
~/.cache/sccache/0/3/03a33bd81a5263a4e2fc4c1146709ff01424188614547b51cd5638d6ea31b890
~/.cache/sccache/0/3/03d48485d47de528ba2da5c4f9a2c7560e412d2d18786f0d531a1b9cde08199b
~/.cache/sccache/0/4
~/.cache/sccache/0/4/043d44285d37b34498caaee386f30dd37cbfb23c4973ccd5fd18f98502668830
~/.cache/sccache/0/4/04b4eff431a144c1bc6c7a920bef1310aea3bde47acf4ddefe776a0ecf31d0b6
~/.cache/sccache/0/1
~/.cache/sccache/0/1/01c57eba5009d28522a0795963764f88517adbb3bd09205b9e221717e3b78d5f
~/.cache/sccache/0/6
~/.cache/sccache/0/6/06c79f57a53bb3cc51b5233dfc7df18968e06ac127f6a3de72065355e463498c
~/.cache/sccache/0/6/0687cfbf8e46bd6b1c09770adc20d630fe072d721dcb7200f3f637c56dff0e32
~/.cache/sccache/0/7
~/.cache/sccache/0/7/075dc38473b380f5b406650e4a3e11ff9e9f9729dd0421c0cccad2dd93361c98
~/.cache/sccache/0/7/0788d1677a76dc4b6ad7b79762be0431083e9f53b3b00c28a90bb1effa38b5b9
~/.cache/sccache/0/7/074c821e07627613d4a6e616e8316812bea7b43a89f00105797a8e2f4611cb31
~/.cache/sccache/0/7/0775318101013c38ee14bc4f07d712441862ea5d74d335cdca080d70b6d6b3f6
~/.cache/sccache/0/7/075a14f552592db8ba2d3a5312c11780af4b8e7410a0dfbc141b6ad52fb5bb0c
~/.cache/sccache/2/d
~/.cache/sccache/2/d/2d902cea20e78b5d1255683f85b337d83d9752c633d4c865b1a2160da8ae2f28
~/.cache/sccache/2/d/2daee795a00fcd3f2eced4d0f268a8f93392a8b6ccd746b9e48361575be3abbc
~/.cache/sccache/2/d/2dec775bfdd2597ef77c93eeb6af50405999495c77c8980d456f6238891403b9
~/.cache/sccache/2/d/2de25678ff111ee6fcc50c30c152ad901593d666135a7b7b9eaad6d5c972fa60
~/.cache/sccache/2/d/2d4eff9c29886d4c4367bf851b40f3693ce68d5a24d547e97f71b74f7b5368e2
~/.cache/sccache/2/0
~/.cache/sccache/2/0/20745fa7b9e87d010a846c2f3633233a9fc5679a042d6b44eef32a352f019ed6
~/.cache/sccache/2/0/200a5681b1f9da161de83f2d52ab3a0a229fe1db740f9226c52f8b6bef4ef748
~/.cache/sccache/2/0/2073e5bebeb73a774df25e44bbd70bc48d031ebe79896df1c5e760c5790b72b1
~/.cache/sccache/2/0/20ee81d7131311496c91319f1c6b0c8308a44cef350a084b82af25937c7eb3e4
~/.cache/sccache/2/2
~/.cache/sccache/2/2/2242fbb07e2b78100eb23df54ea65a007f9436225d8333e38ce09ae43f80576c
~/.cache/sccache/2/2/22245944b0a24b6bb1fc7d6b0f6eb96b0e071cc859a71df63b1ccd94f1f30fb2
~/.cache/sccache/2/2/2200fcf50d647759a7e977a986922d19b8189a189d2f4ba26a134563b8cc6c79
~/.cache/sccache/2/b
~/.cache/sccache/2/b/2bfb6d26ceafc08baafb95f59392cb4d7b5d687d6a5a886d9910019ae9347b3c
~/.cache/sccache/2/b/2be0126769df1cf93de3d7130d8daec9ce67512531aa8df030e6002582e3df39
~/.cache/sccache/2/b/2b1a30a34201b8d0e52f53bd172365d5f86d740843ad318696177100938f1d18
~/.cache/sccache/2/b/2bb2d79b5ba7b73fac88a60541d26da6b7990f207eb5745ae3af8c478dd52813
~/.cache/sccache/2/b/2ba44e35cfb37eceb5ec79e83dbd535c287908706ca8166e96a3e42f4f6e3ddf
~/.cache/sccache/2/b/2b7f4b4838df540068c251e0b1abff7ae2cc882d9605b8fdc6513859cb164598
~/.cache/sccache/2/b/2b699831498901b492cd8efa050c4906410d40dec0fba64e84fdf93c786490a6
~/.cache/sccache/2/a
~/.cache/sccache/2/a/2a05c9931a029fb2a0fd2380222dc69df4710ed1c44a1d52d5a75142e209f0e8
~/.cache/sccache/2/a/2ae581aec0ca45420016150f3ae9f0c1006147f9cc868f008b5940c1a0daa7f4
~/.cache/sccache/2/a/2abeecd3bd28e2ee03e3f13146c43e40e73670f90008df3df745980421e541dd
~/.cache/sccache/2/a/2ac7efe0fcb5a3e25e277703ec00c339b79601e24fc1c86b7529f60afcc6c028
~/.cache/sccache/2/c
~/.cache/sccache/2/c/2c07253cc4cc4eba3e0a4d1e22baa6140f426bfacc071badfd0536713910479a
~/.cache/sccache/2/8
~/.cache/sccache/2/8/281f4fab6f9f8f312d2a76a4185fde9c0e382f46e7d9a73913df5c48de0cbd09
~/.cache/sccache/2/8/28ffc026026f24e7c32ad6a718c48d9fae51612ef638dc1ea1572ad5d17289ef
~/.cache/sccache/2/8/289786fd19fcd8b8ad12f59f66c2fd936d514405a201143ec8e9bd825ab955ce
~/.cache/sccache/2/9/29b15f302f1d86cd1cf02c8c40569ec37bea853880d4e55dde7ed864cd5bc9ea
~/.cache/sccache/2/f/2fa2dbb18a9254df7a577c0dbcb06b4233a09d8eb11d02c3fda8582a9d4a10c1
~/.cache/sccache/2/f/2f140d60ce5684d9f80118ccf5cb3c15f9ae5ff1cf48c714c3177bf6cec9e95a
~/.cache/sccache/2/f/2f5ddd327059c267730fc42ff968a4ee269e72baa2af810b0195b8bfd107efda
~/.cache/sccache/2/e/2e2bf6d69d4ca170221a8a0f4f8ac8b3f1c9dd06b33948f0df0d999a39b617eb
~/.cache/sccache/2/5
~/.cache/sccache/2/3
~/.cache/sccache/2/3/23218e11a29fefc104645b654902802a31b8d7659e126e1addaf5e51609810da
~/.cache/sccache/2/3/236135cf4cbfff1d04e0408825236f4fca2b32af7f2c8923e9bd085ba3e66f5b
~/.cache/sccache/2/3/234b1358641c3533be8fda09d2f67640c510335ffa846db6b9eec5d2e8579046
~/.cache/sccache/2/4
~/.cache/sccache/2/4/243944cb1fb895ae6e633643e76bf57e3f05ea93119780e431e82a47843b20d7
~/.cache/sccache/2/4/2470e87c58a1238dbabc04a4566e775db9a3c0720f08d244d5fe8218a7c556a3
~/.cache/sccache/2/1
~/.cache/sccache/2/1/216828a31909dcfbe368ffef034d8a0468fdc82c9e53164929d3c73da654e6f7
~/.cache/sccache/2/1/216e033795df4834ab8b97028ffeed5548b44514ec6ac5f2ea562ee868289802
~/.cache/sccache/2/1/2122cb2ddfbc8297ea4583a13fa734a4e39dfcc0d25bf718f6c69dfaa0dfb4e2
~/.cache/sccache/2/6
~/.cache/sccache/2/6/26bbcba0f28cd941203832e6d9f5ced8ec54f2195f6a5c7a53368d74cb2b6f1c
~/.cache/sccache/2/7
~/.cache/sccache/2/7/2766ec917d5a1cb1bcf4dc1f481ae59854657bdc085fcb54c36aa43412175f7c
~/.cache/sccache/2/7/27181bcc1f2e7cb7f0d1f3863bfda0569b611a77d58f82e557503cb6145d8e47
~/.cache/sccache/b/d
~/.cache/sccache/b/d/bd571ee86b5782869e87886163ce9cfad16f91cf75661cd41472a66b95415d33
~/.cache/sccache/b/d/bd88001d765122c417adb8d84088e94e17ae82dcbbdb01cec2d28d8be5a0dd73
~/.cache/sccache/b/d/bdddd1a0c466b3cf82c86f3a9b4d604625d694eedccd6ea2eb61ef9c629140ec
~/.cache/sccache/b/0
~/.cache/sccache/b/2
~/.cache/sccache/b/b
~/.cache/sccache/b/b/bb7450eb15cfe55a464a36656380d92e5b723e7cfb6fc2c528926f677408c042
~/.cache/sccache/b/b/bbab2c35448a841bf1d7f6d12b1940dbbdff1440936367a9672ba3a60917e295
~/.cache/sccache/b/a
~/.cache/sccache/b/a/ba397d9baede23936d7991b6ef8133b0d809e4145eb60325776351488b4bc446
~/.cache/sccache/b/a/bab288f0c98fef0157bcf60cf18ca10945a2f64ca2f64cde68eec414c1b84538
~/.cache/sccache/b/a/bad49431fdd09eeeeed57c86c0b7b2bfb3818a8e7eb375fbff4f67430493e77a
~/.cache/sccache/b/c
~/.cache/sccache/b/c/bc4d6108710faa7136c16239bf6b64073b8071ea531fc511003dfb0b9adc8e6d
~/.cache/sccache/b/c/bce6d35d6f93b12896239741be24a0ad82740e2385179f0eb2ed1a09cd5e7229
~/.cache/sccache/b/c/bc513e7e52ce5b56eb6f379f37a494a6df277088f52525a85415f86ad28866ef
~/.cache/sccache/b/8
~/.cache/sccache/b/8/b83e9633d44d2ecd7d52ce6864c9ef9f420a0c7043067947454ff562b218371b
~/.cache/sccache/b/9
~/.cache/sccache/b/9/b99191fd886cccc5fc3ce938dc89a01f630470a59b666d7ac1dade1997ba726f
~/.cache/sccache/b/9/b9e193897879bcbb859650df667908a2f1d2bb321768b978fb2751a81951f2e7
~/.cache/sccache/b/9/b9ca881f8c460c4ed467f66ca170970c745dd49e45718553f52b8cb7af79650d
~/.cache/sccache/b/9/b9d5295ebf967fd61faea8024d97d49ad2d58e701034625e0aea3d84d8afebce
~/.cache/sccache/b/9/b9c2861b4758d71a99a1aed12e30bb8cf4c343636812e6e5717367014a368d9a
~/.cache/sccache/b/9/b92f71f3ab15ff2fba06ddc25cfe1f91422d2e05179abd822e6198b8e71d0a9a
~/.cache/sccache/b/f
~/.cache/sccache/b/f/bf9ceb676eb603a3f176c9fd9d6d3ee15769a2e2817be3e03d26814d7d293526
~/.cache/sccache/b/e
~/.cache/sccache/b/e/be0b90efe4805db36daf5fc7461875aa00ee33ec4259f9bc00a5c163f569b994
~/.cache/sccache/b/e/be020fc151be4f45d681a2d7943a6edd65e05f547446fc6e1289ad232aba19cb
~/.cache/sccache/b/5
~/.cache/sccache/b/3
~/.cache/sccache/b/3/b31d30c63d0f72e228b22d6070ed0eef8f2627b412d2c3aadc4e9ea4a551c163
~/.cache/sccache/b/4
~/.cache/sccache/b/4/b452a75aaf6bd855246aa8fdd39513008dd731b1c7a85a12586da3cc7a437da9
~/.cache/sccache/b/4/b4b404f98a8a4b74221d49a8f148fe00ff40212ca13608aa78b539c9551ed17a
~/.cache/sccache/b/1/b1f5e2aba63e6ee134d1b3fff9086ba46266049ab63927675d36dec9fdc22df4
~/.cache/sccache/b/6
~/.cache/sccache/b/6/b6c62b7183e7a5f3c8f5dc31cfd62a3afd0e1c031c646f0160eccbe2c9c1fe29
~/.cache/sccache/b/6/b600146d96eb6eaf57b02df22b992d3c5c5ac4db7080ed13cf8a18d79aa2d30a
~/.cache/sccache/b/7
~/.cache/sccache/b/7/b7d2663a8f37bba70d5605f5db452b1601d70e114f6ad89e049da659b733c9ba
~/.cache/sccache/b/7/b76c132ce11eacd7c2c2827cf2ecf3ecd022653794f419abecfabc280f5d3049
~/.cache/sccache/b/7/b7004bf1e070697a71fae0f95d3bb4ae0023227bffe36fdfc323f1975a1fc240
~/.cache/sccache/a/d
~/.cache/sccache/a/d/ad8cd45b0dcf88e1d741b2ce08af34871ac35bdbf09cdc8853f71e42a6f420da
~/.cache/sccache/a/d/ad1d0706d3d06135cc29a04221b40902b72c6633b2ba2ae8c5d245f6cd79caa1
~/.cache/sccache/a/d/ad4799f2e78aed3ab43913bf843125e09dace949eb54afb2be1c1036506a5217
~/.cache/sccache/a/d/ad4ed120ee3877a6b10fa664244686d3210e99db66e5a8a3d8c22a94eed656a5
~/.cache/sccache/a/0
~/.cache/sccache/a/0/a033098238a56c23269cc5a4dbef23d36af00f5ff10c54097ff1aa5b7961d984
~/.cache/sccache/a/0/a0cd35a4455df6a386fb7d7381cd4664d5764e0469494c862b80f446e86c760f
~/.cache/sccache/a/0/a04ec0c68589045e16311056f7642f80828096dfe111d0f6a96c8ef3c8fff4db
~/.cache/sccache/a/2
~/.cache/sccache/a/2/a286a1e74b2d7df1271bf5d01e1ad53d01be73447c747d6be88dce7f5ba10bd1
~/.cache/sccache/a/2/a2e1fb402aa81613a8a836c5e19d51c8779657f3983483282bbe049dafca0e81
~/.cache/sccache/a/b
~/.cache/sccache/a/b/abde7c5ccb2353601a4cc1b617589f73a03ea88a394cab41f9668cf27e9c5cd7
~/.cache/sccache/a/b/ab432ea26de84026ebc10e43050db7a74d653fc60eee8dcf19b4f6e867a3e897
~/.cache/sccache/a/b/abad58657223334d92918848df8be924c11a51bf8088571b15b32dbf00906eb6
~/.cache/sccache/a/c
~/.cache/sccache/a/c/ac8b7cb8ffcb962b4f047d931487e3d3ce37bd5ccfe2f94776d4507c8c64d8ff
~/.cache/sccache/a/c/ac8eb68d0ca909e2b3a8c07032e4301a059a060c4e97d19bbc588dd5a40d1d51
~/.cache/sccache/a/8
~/.cache/sccache/a/8/a8553d9b194a3c7e4135523b4c54247be45dadbdc756d6329c04fbb900845005
~/.cache/sccache/a/9
~/.cache/sccache/a/9/a9ef4ed687dda27198cd924401aa38562a91d1687051e503f409a6b1fc6062c6
~/.cache/sccache/a/f
~/.cache/sccache/a/f/af3a829c644ae80786acf9c14f1448794e1512da08b3c6010d9747c53e664ffe
~/.cache/sccache/a/f/afb318084f551f4efd47eb582816d0926dde784815edaf6a7dab1861e6328443
~/.cache/sccache/a/f/afbc600808a1ef20aff978b38c094586f06aa9eea67e9cb65c9e7f74f983f7e4
~/.cache/sccache/a/f/aff75da5d5e84b4bc1b00fd3be7321316b497b4fe9a9de9fa80a42d14a7a77ad
~/.cache/sccache/a/e
~/.cache/sccache/a/e/aecb2d87aa5e6c6eecf07c68790a424efb8a864d6bc44def475a1c4b030e9375
~/.cache/sccache/a/e/aed2caf7ad9d01bff06f06635876806b652f0ea16a6b4d37e4da3f5f88e5aa32
~/.cache/sccache/a/5
~/.cache/sccache/a/5/a5ee695157abed97929f41672c88fd211772111e84c67cc52990f82154275e23
~/.cache/sccache/a/5/a5aa193e9e8e0872a3866d0cd459a147b88553cc5df7868af31fae1fcc9a6ef7
~/.cache/sccache/a/3
~/.cache/sccache/a/3/a3b7b8529c7371bc8fa544e4cbe38a68566bd33291dfd5dacc28e64155367276
~/.cache/sccache/a/3/a30ef8c20df955b371e9e13cf2f6d87c9232cd520d4dd33e95505f208afb5c59
~/.cache/sccache/a/4
~/.cache/sccache/a/4/a45e17557b1cd0e5eb688f2d620549fbf55ac40809d0b4d5ba42395760a13abf
~/.cache/sccache/a/4/a4ff656c8e09465ad289023027ce3ccdb0191aed82161e390756d4ce566772ba
~/.cache/sccache/a/1
~/.cache/sccache/a/1/a11d30fa5ed3881167ffd9f53bf873bd83a244868e2054bc298e122ea04bdf53
~/.cache/sccache/a/6
~/.cache/sccache/a/6/a6bd77c85579098ad97de67348d801f0d8932b2f5be91300048ccebef6397d64
~/.cache/sccache/a/6/a6ba7b075d3aa9b693219551f0de4de92c75f4a0a4f06419b01db123dcd39309
~/.cache/sccache/a/6/a6489fe0ceba8cbd1fc72568fe772e25414011cb03a6dc1a96cfd300f649040d
~/.cache/sccache/a/7
~/.cache/sccache/a/7/a735b5e1a71ff2497ac71b397d05f651b3433d18839d20607c16d39f3e94c039
~/.cache/sccache/a/7/a7fcd359b2325d744cbdf10db7572c35d7680125dedcd0d0b65946c76bc3f0ec
~/.cache/sccache/a/7/a762b47d1a09b107ad4049cd9b952acdb17769dc75e50a58cd92c2bc6382e3cd
~/.cache/sccache/a/7/a7dd26cb238be69c6fedeaea83ffe0bcba62984ba871841ac62fc6532e0a9448
~/.cache/sccache/c/d
~/.cache/sccache/c/d/cdfe91fe01abf6d38a814a68223ba177c501d4ea87d24f64d4bcd1ce72a3e178
~/.cache/sccache/c/0
~/.cache/sccache/c/0/c0e71e16d460947e6513019f141886cc52301989d3c324bf98691fc4218203f6
~/.cache/sccache/c/0/c045c59f1d1e1c860b16a75d791616648d6fedb7fd278581b43c8676bba17070
~/.cache/sccache/c/2
~/.cache/sccache/c/2/c210643225933fd871a5890d1ab193c75452405080a59b6a6a5c48fe8d7a495a
~/.cache/sccache/c/2/c22dd392b0ed9754d482ac308bec3228de151bd6e3c48bad4d64c9b55dab2e6e
~/.cache/sccache/c/b
~/.cache/sccache/c/b/cb7152a388d3719cae11753800029615379a7ffbbb2a8ad1f4c3d5ed3dbb7442
~/.cache/sccache/c/a
~/.cache/sccache/c/a/cac360c8b5583423643c042181fac4b89afc93844bd6f22c22a1fe0577200273
~/.cache/sccache/c/a/caff09cf996873144028dafdcfe0434a119368f0b22ed8212339c720beaad2cc
~/.cache/sccache/c/a/cad551b75a53ff8ac480eb8c59b909ea8a9d6fb6b481766e29a67f379e0cdcdf
~/.cache/sccache/c/c
~/.cache/sccache/c/c/ccb1ee75feebb70b211c15b74fecc92229f743103901f174d091a9ad5c10cff4
~/.cache/sccache/c/8/c898a2432762544265c9b7670895868c7ec400952aaac7e9595fa427e9c8c4c1
~/.cache/sccache/c/9
~/.cache/sccache/c/9/c94d331e8af038e7c0151f5e9206e8cf849953cfb5dddd8aec43ee0645f27ec1
~/.cache/sccache/c/9/c9ee32f1d5f4e13ee49abc5c856f1770c22faa4e6a1162feaff0aeac8e54c8bc
~/.cache/sccache/c/9/c960ee2347577d030916852afe4ffa8d6d84dfd318f38f03120dc5326d6a3ae1
~/.cache/sccache/c/f
~/.cache/sccache/c/e
~/.cache/sccache/c/e/ce7c9790d25b2cacda81560b92ccdcab682895c9df8a21382de2b8bb5e7349a2
~/.cache/sccache/c/e/ce4ff06b01bc9942b51cd8e2cb146d938aaaeafac2d38e1e977dd5e90adba548
~/.cache/sccache/c/5/c53d2c9557bef43eb3b0e15929256c73c3e034aef35eb4b67dc46cd6723b1da5
~/.cache/sccache/c/3
~/.cache/sccache/c/3/c3126b22d114de698fc2f957635a1f7577be05bd7816669dfc98337daa318b90
~/.cache/sccache/c/3/c30f757c2c354285be881b166705e2f2741f16f9380b922b74dbebc458ceb405
~/.cache/sccache/c/4
~/.cache/sccache/c/4/c401a99b0a87b386a528737739420265d6b4ba01f5ddaf24102bfbbfbdf7c628
~/.cache/sccache/c/4/c4395f52392b60f150ef1315725da019240f14c5c0db6c3f1cb1a4f4532a4fd3
~/.cache/sccache/c/4/c4f538eba5a3adbf7b4035bf780a91c802e785b81d8527436ec01dac9ef4144d
~/.cache/sccache/c/1
~/.cache/sccache/c/1/c16ab92304dc8864bb3b68fa0b0fd0df8cd04234ec69b6d3ffcd08e494d4a273
~/.cache/sccache/c/7
~/.cache/sccache/8/d
~/.cache/sccache/8/d/8d31df8d812477f5c2d6d0f5584689c9a36aa9ebc6cd67fa546cbe2f06d4de30
~/.cache/sccache/8/0
~/.cache/sccache/8/0/80d14d48939ccb2d694713e21b1b316518685d198066255706bf0db5773bf70b
~/.cache/sccache/8/0/80c84228bae5df022c446f109252cceb0d77c46185c210593f7a1028755ef460
~/.cache/sccache/8/2
~/.cache/sccache/8/2/8294a9c509ee26f72e12f6b3df7e9054fe10525eb6fcea05a3378d5a997d9672
~/.cache/sccache/8/b
~/.cache/sccache/8/b/8b489fcdbb6af6cd2ca168f4c1fdc59af5eb4b1b4aac41a97e3204fea3f922ed
~/.cache/sccache/8/b/8b03072090e4dc41b316f79d7ad45a0bc5618bc0f134080ac04c5200492331c8
~/.cache/sccache/8/b/8bb8479dfdc18f8025972aa5a9d4c80378313bfd81da9b5e0526c429ea3ee7c9
~/.cache/sccache/8/b/8b5e4b01ddda703ae10624fb948a2801ea18dac2fb8eae6d7ca9c5e0e09e9568
~/.cache/sccache/8/b/8b43e04c8d690adc4edf23af2e06013b72e88514a26a10a1511599b97011967a
~/.cache/sccache/8/a
~/.cache/sccache/8/a/8a3d2a68911f8d919b509a4c1616bcd814921aa54556bb799ad20e3f679e612e
~/.cache/sccache/8/c
~/.cache/sccache/8/c/8c9d5a574abbf5c87709657498bb1f33ef7b29135ce25e9ee68e842611063679
~/.cache/sccache/8/c/8c019acabecb0c76625e131eb79c518fba836b63b43bf47f93d18ff877870097
~/.cache/sccache/8/c/8ccdecb767a7b2d6929b160a405f87d504749248d8ebd1509e0eaab6c6478569
~/.cache/sccache/8/8
~/.cache/sccache/8/8/88bd04f134074ff2cd11f9331554f2d934b4b1e01bb8e9f221f4df29f51b1389
~/.cache/sccache/8/9
~/.cache/sccache/8/9/8906c6c811b325fc0483ff6f18b7a341846ab59efb095e61c827843cacd1e534
~/.cache/sccache/8/f
~/.cache/sccache/8/f/8f576156a8519adf55581e17a9af6ab4eae05058abbcfda6088786818988c920
~/.cache/sccache/8/f/8f5557394232acb3eb4c0be3b1478c451ec7eee7a0d1dc88d1b128e15e90ba86
~/.cache/sccache/8/f/8f88c5463b127cbdef354d5f977ef3b1efd3012454e11b5423d9f25c7e0e7c15
~/.cache/sccache/8/f/8f029c81eba7da8ee6dd882fe64ab6623a0f49d5a38180e3790c05e4cf355a58
~/.cache/sccache/8/e
~/.cache/sccache/8/e/8e25f713a23e0d343dd5ce92713fcb637265087601d3c30d9873cf17b6d2d4f1
~/.cache/sccache/8/e/8e34facd2f73286a42f7bec47c00d4a8a981d608b330e66468fda36d68df352c
~/.cache/sccache/8/3
~/.cache/sccache/8/3/83f4d206d424df4283e261eacc7750b531c644163a5f63ce75fe82c2f9cf7f35
~/.cache/sccache/8/3/834c2bd23235888c998272ac874748b043b288f128a878ac69283d139fd2218e
~/.cache/sccache/8/3/83ab07eaac3f70f9e01d501b75df9705bb28aa0160567247c3caac32f9251be9
~/.cache/sccache/8/3/83d5bc833a208ceaffc1436acd4bb465c14ae3e31edefbfd82fbe811f270255c
~/.cache/sccache/8/4
~/.cache/sccache/8/4/84f934e5b925b85af09c234a076f1bec37ebdc46b0aa84d4bffad3d34f5ba3ec
~/.cache/sccache/8/4/849fe94b9fa2024ec304d7bd1de737703990a9506eef09125ff18ae3b5b93b62
~/.cache/sccache/8/4/8413c60279e2caf7a81b02f98e4f6589bf37ea6859db58a2ee2a4375d562aea1
~/.cache/sccache/8/4/842e9df5f92a71225fbdb98f61f260949f3933666f277bb29521e39eb7793651
~/.cache/sccache/8/1
~/.cache/sccache/8/1/81d91c4774ee0741d55f3e89b3ad5d415992e0420ef6729ec207547bb7887e3f
~/.cache/sccache/8/1/81b0dbbc181f5d7c86853eeb1de1b76f440ee6e451bb67fe0acb587312f9d3b0
~/.cache/sccache/8/6
~/.cache/sccache/8/6/8636a0536c1f1ac735c64a3145e9e78e2715ff445ccf0c80e07a28488618e83e
~/.cache/sccache/8/6/86e2ec7325961ca1140e7ec0d9347a76645167a82efde56b312dcc83110361aa
~/.cache/sccache/8/7
~/.cache/sccache/8/7/873de105218a49b12b3992f430bdaed0f1ce5a4871c4e511c521d9c8d597575f
~/.cache/sccache/8/7/872a45e2208a5f704a108da59eacfd099c7d8637e7f8b3bead8eec2360ece563
~/.cache/sccache/8/7/87d7e757b541a32b561261b66a2821e5e0518f386b03f6ed697ea5e866bd8324
~/.cache/sccache/9/d
~/.cache/sccache/9/d/9d4db16783b84ccd77ec9aeedbe0a17325ebb82341a8db9c2dfdbc51e2828ea5
~/.cache/sccache/9/0
~/.cache/sccache/9/0/90b2865d2177f1a83bcf1e7a0fcbeea360235a2fff1fe1b03a02decd273ec526
~/.cache/sccache/9/0/908bccd8426a015ad1113f43b7333cac896137f3d385d2095121f33d656cc662
~/.cache/sccache/9/0/90fb6a2b7722a10ef61c387146aae8751c1a23032e914d59d905ebd881e33a5e
~/.cache/sccache/9/2
~/.cache/sccache/9/2/9219fcd0a6bafc071820d9aa5af69933062ea80e9584f52102e9ec24c9f81345
~/.cache/sccache/9/2/922d75428cd7925dbe688c19a7c60e424c6a0de5c0d6dc2965b705883fbd5505
~/.cache/sccache/9/2/92948ebfbd7f0aeb4ab12328fffa99508c76435689b4592e0eb4cf7cccd5fa3a
~/.cache/sccache/9/b
~/.cache/sccache/9/a
~/.cache/sccache/9/a/9a1beef227ac6a858f5fca53d2ac1e544a0a4e2b51e8e7ad30f6b9b55d006261
~/.cache/sccache/9/a/9a937be8760d26a589842dde610a8d5b6d5b65d120e4a8cd2f318753aa0e81f4
~/.cache/sccache/9/a/9a81644e58876113e6ebba26931d457e6c0e930f9a586e5cad821f66627b8b45
~/.cache/sccache/9/c
~/.cache/sccache/9/c/9cde706b2ffa258ca379122a7a1acf04268b985bad28a7207cf035b9f964dd9c
~/.cache/sccache/9/c/9c875db7b47ebcea1f750a8634f21ee1794afdb7ecaef51694e0d85876a11f9b
~/.cache/sccache/9/c/9c0ed8f04a61f767a67a47eabee34eaef9fb63c9744aa2d3603f7db96954f2a2
~/.cache/sccache/9/8
~/.cache/sccache/9/8/989cf279d5eb4640f43dfc340b229e60258f8fd8480e52e9a40ebac711b9190b
~/.cache/sccache/9/9
~/.cache/sccache/9/9/99a02fd340480f0c389e8f6fa852846ffa140addf8e24f6f2e14dc11a4922b0e
~/.cache/sccache/9/9/99c74c944a822e94aba5f8b0e1fd823cd6d32a80d9fd06478a2c3dca1bf94d0a
~/.cache/sccache/9/f
~/.cache/sccache/9/f/9f29603466a5273a4adb8ab0df476feda5d8477bfffe547f9463cd14b16e475b
~/.cache/sccache/9/f/9f41da0ffad140cbea3888cc277aa717ae75b683609621de6a504c29e0d21978
~/.cache/sccache/9/e
~/.cache/sccache/9/e/9ebe3d9ad2e87988d42ae0d636fc156be65c4c1ea91155f51902d85453449d83
~/.cache/sccache/9/e/9e539e6709644eb6c692a507ebfa86ef997e2e89bd9602c8161c292203fce0ae
~/.cache/sccache/9/e/9e81e57a64e38b88af97d2fc8e3145b45ed18930c0c4b8bc9cb0ead563fbcc12
~/.cache/sccache/9/5
~/.cache/sccache/9/5/9597c591bbcca614e1dd74eec9f613758ca755dab525b935bf3984466c50a974
~/.cache/sccache/9/5/957030cef1dc0b0518a70539f8b92991c86f802e269ed8d1200ac957a8bd8ad6
~/.cache/sccache/9/3
~/.cache/sccache/9/3/93a3d420ad2c334dddeeda9fbcf8f15cc694a1ae79793eb6f65d499291dfb5b6
~/.cache/sccache/9/3/931d81bf4b8eadc2095bddda2495b852ddaba95397bb5aa2b1515f143c1abfda
~/.cache/sccache/9/4
~/.cache/sccache/9/4/94a98ecc620e94e899848b8b67df580cd7a55ad9f9a514b8211af1a5d07b77b1
~/.cache/sccache/9/4/94a841456fe96aa3409821599a94495a93376565472e9b0229e90b1954a5643e
~/.cache/sccache/9/1
~/.cache/sccache/9/1/9117bf0f9f1cd33b51b451ddac7b38b9bfde6fc64b5a34d368576ec64dfa8d3e
~/.cache/sccache/9/6/96752b28ae6fceaca7ce90d27e77abdbea51032132a868ba46451822f0a30c35
~/.cache/sccache/9/7
~/.cache/sccache/9/7/97175848de0264226963c851683f9b3d873ae984b1caec5755d28deb8da2f060
~/.cache/sccache/9/7/97b3d1ee8be96202323ecf5047c4fc63a30dd57c3a1aa398333daa849aed8f8a
~/.cache/sccache/f/d
~/.cache/sccache/f/d/fdc6a759587351a5f486de91c34dec83bf34adbd058bca7692de64a114fba540
~/.cache/sccache/f/d/fd2d2376c110b3be0adbd6639a8e4dbb4ff55375264d174b4af9f4ad91277f47
~/.cache/sccache/f/d/fd2890046907673d0aa0ed2ed9aebccfc7e04f9381d4b4e41ac37e0ee8e879d2
~/.cache/sccache/f/d/fd6f3969dd29b706308f058c9c4c0870eab411ac0933a5c26f38bc6a4e851f1c
~/.cache/sccache/f/d/fd723a8718301701bba00cb2f091fa6a531e17215bcf2cbf6b0f8001cfd136bc
~/.cache/sccache/f/0
~/.cache/sccache/f/0/f02d944de8685185862378bdd142db53278cb4df17c45217b01736af58e7c762
~/.cache/sccache/f/0/f04f97fffad110a047b9fb65eb493911e033e777aa522d8b62464fe169b5f42d
~/.cache/sccache/f/2
~/.cache/sccache/f/b
~/.cache/sccache/f/b/fb9f7325892574d93960d2702904c41982a546aef4f94e39eb40daf605feca91
~/.cache/sccache/f/b/fba8dddcd22d6eec67f3fcf19944ce45dd95890afc18cd8984f5f8fce62260f1
~/.cache/sccache/f/b/fb7f325ad48d1b5dc4266c5ef2af2e4350fb17650464b31ac3b1d525eb9611e8
~/.cache/sccache/f/a
~/.cache/sccache/f/a/fab1adbff1c31438b2bf8960f3e5fc0973adba9b8f030595113b1ce976e3abf8
~/.cache/sccache/f/c
~/.cache/sccache/f/c/fc8dbab8fc5a39502b0980d8d75a45c26831ef4370f19a76d609b4fe3ff4bc57
~/.cache/sccache/f/c/fcb0752cf872c7b0d6bb0ac4c8a0aec3da34e716713330c7d688aad578132774
~/.cache/sccache/f/8
~/.cache/sccache/f/8/f8d6599d0716cb2ebc8967c80e1788fb9d55d0fdc9df544d1308a63f39598ed5
~/.cache/sccache/f/8/f84817c9b7f8d22f7f6382e1058f1ceee5c9ae61a79412caf888fdec7b1432c1
~/.cache/sccache/f/8/f8f41debfbaf7930f861922075145f1f5d2107cecf5dec0a6e705ea1de8e3f67
~/.cache/sccache/f/9
~/.cache/sccache/f/9/f9f243cbd8651bf94c960f6702359f37d864c220b1252c11ebb6b9125b8e659f
~/.cache/sccache/f/f
~/.cache/sccache/f/f/fffbdac5bde6b7b43c7e2128cc8ea55ba2483bef372204c69a5bd7a0b9d41200
~/.cache/sccache/f/f/ff2c9e428105a7154fb4028734122825d80a98d9aa7245a23423dd2f955a0909
~/.cache/sccache/f/e
~/.cache/sccache/f/e/fed511d530fe5cdd250abcabdab42650e0678f4bb3412b33c3af29943956a5d1
~/.cache/sccache/f/e/fe94b0ae3db75fca7a39806bb28053ceff50296609064400f9c62c51ae085627
~/.cache/sccache/f/e/fee50bf9f7de8bb70aa22559bacbbb6628bbb4f35612627376bcd3748353df52
~/.cache/sccache/f/5
~/.cache/sccache/f/5/f555755a608a6de6a2a5299d04a18b94241c3c1fce6ea3c695dc33a6f313ca7d
~/.cache/sccache/f/5/f5a7927dbe423ac87a6649705a0cac12d161b5414cac26578cd89c3f6840c297
~/.cache/sccache/f/5/f510d9e57454b8a9862c10c860ba5c63129a9a72ea631f56db7c00b58078c12d
~/.cache/sccache/f/5/f5d7507cf132c7419e68b6a466a479cb8bdb8072946a337d90d06ce4f8b9a2b2
~/.cache/sccache/f/5/f5030e4217394ac24f6031b3978888daa9c21c20de4c2e661c31b3af35f7a4e1
~/.cache/sccache/f/3
~/.cache/sccache/f/4
~/.cache/sccache/f/4/f4e67c30712cd270645cc1d3a45123e1ef2c6480410a4a5ceb4325c7224116db
~/.cache/sccache/f/4/f4ce78bf054bad84462509bbb00cb19cab3cb99a45216a7c2d2ef730b8328012
~/.cache/sccache/f/1
~/.cache/sccache/f/1/f1319132752676153ccc9653e2e7275eb23ef1a8f17e8657dd4e9714d2affb21
~/.cache/sccache/f/1/f1928a14089a1526e49752a2f37c842afffab69eb40df814c1a4ff79f419ca84
~/.cache/sccache/f/1/f1ebd2ce67a58a98259f72c2fcc38a22daf52e588dfd1b8ad600de3bc5036419
~/.cache/sccache/f/6
~/.cache/sccache/f/6/f66ad89ae8f8442830a65fcf45532c0ad050f3954e8985a3ce44a6be52d9d8e6
~/.cache/sccache/f/6/f6f0fc64dd18003180edeff1736e8ccc20dd8efd187620327684b73035fc0184
~/.cache/sccache/f/7
~/.cache/sccache/f/7/f7779fa579b41157868c6368e0d50f03018cc2da0174e97cf3e1842128adb300
~/.cache/sccache/e/0
~/.cache/sccache/e/0/e05b6bd89bd8f4b83a3274234d07bae3f85d9d73f7ca8f0627f0c0f2f1df31ed
~/.cache/sccache/e/0/e0b4d0cb2ca4085b0f5525ca7ffdafcfbe0524d72da45c359eba3ba4f21c55ac
~/.cache/sccache/e/0/e02823f85eaaa9c075eb58d1f73f97110974fbde0cdfee777a408092ee57856a
~/.cache/sccache/e/0/e0d0dd9d738a455091bb3ca4223703739900573498308a0fe5d4ed3ae8c87b39
~/.cache/sccache/e/2
~/.cache/sccache/e/b
~/.cache/sccache/e/b/eb7f95cd471aac277783ccf8198eb84c9c6fa80f41520fb29f2d00ac6625335a
~/.cache/sccache/e/a
~/.cache/sccache/e/a/ea03116628993e143e3a242af39340e022a917da3821910422489c97badf1578
~/.cache/sccache/e/a/eab87f8938248cc51c7d30cb0190e374a614b403feaed83b029f3598c06c361c
~/.cache/sccache/e/c
~/.cache/sccache/e/c/ec43f9529d18366e7cb3268b80c6c5a1436494076149f86c094a8f3c80bafec7
~/.cache/sccache/e/8
~/.cache/sccache/e/9
~/.cache/sccache/e/9/e9a64f959797fb8070d4c8dd31e14da746e46113f3cc9a4d83d09535a33f8486
~/.cache/sccache/e/9/e915480863c04f47926503e76816440fddc97df4aaff7e8f283f8f52e74243c7
~/.cache/sccache/e/9/e9e178e95664eb6db593d58e7ea6dcc4912631428e6a2b67a79bdeb7900e6548
~/.cache/sccache/e/9/e94f9a3f3100154af72b774fdc78eb233bf4ae522092a59e2056cc5b4ad18781
~/.cache/sccache/e/f
~/.cache/sccache/e/f/ef8aca5083ac687bf45c0935d29780704647d0ea54c3e23db73a4d934ef24d05
~/.cache/sccache/e/f/efcc54b6962df92cd6470536dc3638d069cfd401de28ab7949d147662feda61b
~/.cache/sccache/e/f/efb58fdf081f718417fe223cda0a3ed5a4c3e37a581bd1d36ba7961450818e8d
~/.cache/sccache/e/e
~/.cache/sccache/e/e/ee62e16b17f9ad39e9540584c053efaf7e47b786cc077b86ba2691e32f65edc5
~/.cache/sccache/e/5
~/.cache/sccache/e/5/e5119afae2edb1daadb208440e4c39c8660c4d983d09bee62458811100777c42
~/.cache/sccache/e/5/e51f984534a5b2132b4fbcfe27b6b53d19ce7c6bc0fadc58b1de2bbcf2906262
~/.cache/sccache/e/3
~/.cache/sccache/e/3/e3ff7c82bb3e92522d30aeaa24decb2bd5d4b98ce71a6cec3f643490b3c28387
~/.cache/sccache/e/3/e38361c720c7bfb53429b3f75c19887dca4edd4b80054c237a61daf81eb3300b
~/.cache/sccache/e/4
~/.cache/sccache/e/4/e40248b6a10e345b556580cdefde09af9bc02d04d9d432b2635b59d7dddfa592
~/.cache/sccache/e/1/e1191b5de544c0138ea2fdbd89ff4fcf736cd115f6d1d1761a8d094b21ba663a
~/.cache/sccache/e/6
~/.cache/sccache/e/6/e66cee4b95f002fe4bceaf961658dcc19b33d7442c45b3bf8c2789200ff00f36
~/.cache/sccache/e/7
~/.cache/sccache/e/7/e7f961fbff2f38c5c6675babb5950665edcf3768f301515dee79e614c0fcd044
~/.cache/sccache/5/d
~/.cache/sccache/5/d/5db8fd238689312db286a44a000ab12fe3613103ba30804cad9875341b6dbb8a
~/.cache/sccache/5/d/5dc659d90fac2b3d27a125fa15eb480a823df729420a225cb664b7cb145c2b66
~/.cache/sccache/5/d/5da72de6c26df3d34ceb947a7775b0c9806928127f1ec3d33b769243d356b00f
~/.cache/sccache/5/0
~/.cache/sccache/5/0/505485e1d47cd2775cee95904b4400b9513d469a823b17b87af4e52a554f3c29
~/.cache/sccache/5/0/50761f49f8ff01d26d8be251beaf6679b8794238425932aaa8a8f55e8dcf058a
~/.cache/sccache/5/2
~/.cache/sccache/5/2/52a4012aff7dcf9eececd9ef6cbbee140f052bc7f7b62327cf3b07f6de020be7
~/.cache/sccache/5/2/52c42b36da73c7508609c9b5a24363fa49f9404ad92cb38877fa3d5c48da8131
~/.cache/sccache/5/2/5248bfacb0c684107703d6b60deb0a3ed366e2d37d1c5f718067ffa75bbd30e0
~/.cache/sccache/5/b
~/.cache/sccache/5/a/5ab44da3c065e47f613c6fe617ef2e6e74328a74e3841e2aba097b2c1bcc569a
~/.cache/sccache/5/a/5a65645b72b4fdce591221839ee0f5f3c910be959dad909985d32032d9f0357b
~/.cache/sccache/5/a/5a9b730481ec13c00641ec99bf605c4e907d9e99a4f5d0e524a87258f7f63b9e
~/.cache/sccache/5/c
~/.cache/sccache/5/c/5c97655f41bcb480bf49b0775fda202d6c5989094e4cefbd77ac7701357302d3
~/.cache/sccache/5/c/5c77fb4c3c11ba976a0221316987f77d80c07b0bb8db019f525cb24413ffc924
~/.cache/sccache/5/8
~/.cache/sccache/5/8/5860e5210ce6a602766c24a15775a45aa7b6737bceeb09e602da6cda37dd18b3
~/.cache/sccache/5/9
~/.cache/sccache/5/9/5918f0d847143b3d71d0fa30af9261d607c7963f96854e7f82738a386a348d00
~/.cache/sccache/5/9/594e9cd7bfc4b6cc0a23452ac0dc7ca77c8cbe17ef24836fef5d66260a769b94
~/.cache/sccache/5/9/5959b132b9c2d355727aa2ddef06ea919c12ed679033a8f484ec2f704718d5b1
~/.cache/sccache/5/f
~/.cache/sccache/5/f/5f27ff1a74294af1ba8030d29fc6c569dd0eef835bc7edc888e477b6618cdb0e
~/.cache/sccache/5/f/5fa21959414b10615489e4d7ebb110b8894cd8c1448d04165e0e6d7b9a383708
~/.cache/sccache/5/e
~/.cache/sccache/5/e/5e73915ce09310b0a19baeeae488eb9ce32840361687229d3c8e921ff8c5dbe0
~/.cache/sccache/5/e/5e4237e140c9110c755a2aa1c87e58e3d026f08fa0117a91ef72b9e72bc843c1
~/.cache/sccache/5/e/5ea842302534cc34387127a18b3d0e58487211f487bbbe1c9feb3925f3207502
~/.cache/sccache/5/5
~/.cache/sccache/5/5/55c002b350f7b6d4d5e352171eed356fe5dc6112d488c03931046d04f769a892
~/.cache/sccache/5/5/55c57f6c53355b8d3cece8b660992e367cfb152f49b4f454b57e3e5677250c2d
~/.cache/sccache/5/3
~/.cache/sccache/5/3/5352dd5fc809e9963f38dec5e277de4b28cdd0ef079d06179c3b9aba44d7a1c1
~/.cache/sccache/5/3/5313ef61a6a230c5378b92578cec0c00818fe9fe23a45f6849bdedb61a826939
~/.cache/sccache/5/3/530b108893283111fc934770c316941ca6352067858aa314790643b8e224812a
~/.cache/sccache/5/4
~/.cache/sccache/5/4/543f210f73388cd73b05216c61ae341cda6c27e554a3f2fe5958f9e0633a2a51
~/.cache/sccache/5/1
~/.cache/sccache/5/1/514d691f8df1b9263465f6261f70954fae93ffb47452f353bb1dbf5c892e0e18
~/.cache/sccache/5/1/51389dd8a23b56b4e32bdb4bb50696d088b177e007041418feb5a8ea101928af
~/.cache/sccache/5/6
~/.cache/sccache/5/7
~/.cache/sccache/5/7/57c5263d7e5498f434c91e847dda41af1d669587b0734f855da56c72148cf1ab
~/.cache/sccache/5/7/571d46cb07ff8a6fc0bf5de509684e4e7189400748d3f2ad4eda34e768d6b2bc
~/.cache/sccache/3/d
~/.cache/sccache/3/d/3d0a2b35f323753677711b05eadd5713d43e83fa9d810c90a367861d62ccb59a
~/.cache/sccache/3/d/3d7eadd77bc69bdc49e22d8edeb454c886cd49f91d2f53df04b63e594c406e9e
~/.cache/sccache/3/d/3d05473af7deacae3f427d542d6fb46f7b1ec7147e15e80cbc5328843bdbda9f
~/.cache/sccache/3/0
~/.cache/sccache/3/0/30b93e5d1c54ca0c73eb92e7007374a4d4ed92167518ad0b26f9905efee159d2
~/.cache/sccache/3/2
~/.cache/sccache/3/2/326849fc07feaebf79875f002b27c94cb157c956e019e350f3072816746fd2a3
~/.cache/sccache/3/2/3252c57e8d9702427ecefc55e17e68676c0b773372696ce7697e8f489394d2f1
~/.cache/sccache/3/2/325a7a020bb920bb16c498c4890e32547070cb953b745ed2b2d91819e7f04c61
~/.cache/sccache/3/b/3bb11c83aaa55ee7a1ef48a904e52c5f0183e546f5c6640e56c40b38da86d0b1
~/.cache/sccache/3/b/3bb42113de6617234528f501a8f83aaa341ac525cc8f3e60f4f32e502e84bb7f
~/.cache/sccache/3/a
~/.cache/sccache/3/c
~/.cache/sccache/3/c/3c680d843fcdb162896bd2d8515b7ba4b7bf4abedae5e64f2bcee266f6de982c
~/.cache/sccache/3/c/3cdf08555e013a3b7f034a5d7f5fd6de0b6e4ee989d293df31ac59126cb12a5f
~/.cache/sccache/3/8
~/.cache/sccache/3/8/381041ea188f643a1dd6157cb3c0f8482d0c913828c27ffa74207840ef013cce
~/.cache/sccache/3/8/38ad8a91fd2693990911b24c4f70bb26091c0fef41dd36ec62a34c58759a8ec3
~/.cache/sccache/3/8/3826148a024b717053ff5bd826e8be643004d81105e4e674f4c1dd20c7e310a3
~/.cache/sccache/3/8/388c440801b10741ab26168f23c6f78f8bd43fe3e5f7ed022acaf5909a9121ab
~/.cache/sccache/3/9
~/.cache/sccache/3/9/3973038eb80a42337191695e2b15d0ba5741593c97324832bd3fb5258f683d5a
~/.cache/sccache/3/9/398079a904d7767adbe0caf6fdd09d410968ca4c3fdf036ea3ea136a15a125ae
~/.cache/sccache/3/9/39240c8f024fa7d0467d83cdb6be84d267ab6e06be92062fe380e8e39eea5361
~/.cache/sccache/3/f
~/.cache/sccache/3/f/3f6ebbfa1485416efb8f9f3705ec1a283a55b62d1b4cc7edffbb76d381d74b11
~/.cache/sccache/3/e
~/.cache/sccache/3/e/3e2717301f81ceec6a1a5a89e5a042e79e7e930d03e76a39ebac8593238b2978
~/.cache/sccache/3/e/3ecf0b1933597d51758a050ec7f44220e635556097c75010996c3ead9c54e5aa
~/.cache/sccache/3/e/3ebfb985968b8b757943f8291eb24ae4d92db919d766cbb63a1beb759a30f878
~/.cache/sccache/3/e/3e580d71f1fa7ad669817f354f2f587975810378ddfe711cabe7184c55e87d66
~/.cache/sccache/3/5
~/.cache/sccache/3/5/356f56ea5e84d78cfdd33b1a143a17d7120d9464373f02c3c833896cbd1502f5
~/.cache/sccache/3/5/3522991d1135b3bc296a4bf6f6097115ef8098e62758b4cae03577fdc545de84
~/.cache/sccache/3/5/355bd14faf9880e26f250c5883abaaa2ab38783b92bc68bab6b1cdee79d750fd
~/.cache/sccache/3/3
~/.cache/sccache/3/3/33ee73e362a33cf38742509510f1f03c93e59ad74eed5d1ca191af301b669bcb
~/.cache/sccache/3/3/335872a78fe833bc9b2920f5b891184a6413da46711e8ed6e1e32f44a13822f2
~/.cache/sccache/3/3/3320897bce69b14af04539afd061d232e0b2dcf5562a2b0ec9885ab370433ea8
~/.cache/sccache/3/4
~/.cache/sccache/3/4/341f8c814e958f27fdc2484ec14f1a240309b6944e46beacff253a14965d53a4
~/.cache/sccache/3/4/3409d69900b2de7e3d2abc95569e0d360992c71ef7e5a674632c55b64a819f33
~/.cache/sccache/3/1
~/.cache/sccache/3/6
~/.cache/sccache/3/7
~/.cache/sccache/3/7/375d38e387099c4e567c6536801ef6d16095e9d8ff337657e0a05493132ec559
~/.cache/sccache/3/7/37c8fab6aed5c9fc395f3c6e8230fc95471771dd2cf46251f5077f9e9201cdf7
~/.cache/sccache/4/d
~/.cache/sccache/4/d/4d9d3ff71edc5b0ca441e5bb582dc99229d22671255d08270707214cfb200ac1
~/.cache/sccache/4/d/4df4509448bdb296307135d9d511cae43e5604d9f15714d67369deadd1c426be
~/.cache/sccache/4/d/4dcd0c51d9990df8e84b616c647eadb9fbd2ed96760d2f9e9b9010dfbc5a32d9
~/.cache/sccache/4/0
~/.cache/sccache/4/0/400bb566e2bbb99ac2faf051e407e7bc8d7b351744ada83ff4f73c61eb00d5d4
~/.cache/sccache/4/2
~/.cache/sccache/4/2/424df3c6886428bd21df09b7e404070d3e6e0c3025521a2165ae124111b2ae38
~/.cache/sccache/4/b
~/.cache/sccache/4/b/4b80e4f85983046fc07557829c0518d689bd68112d456fd3ab62190c0f201b5a
~/.cache/sccache/4/b/4b96e4f8ed4e9f67b01a500791392d0136d4d4e2502c7e81227bf0506b8ebb05
~/.cache/sccache/4/a
~/.cache/sccache/4/a/4aa683f11f99ec48ddfffcba7c453d74e6f55461dd7ec6dabdfe1018cadefe61
~/.cache/sccache/4/a/4ae4da15b8e4a4fbf3f1aaa110e203275a707850699ca338408724a0d25f187c
~/.cache/sccache/4/c
~/.cache/sccache/4/c/4cff5b5ba31833b68bc343f8c2596631082e82d910cde6e92c5fa92073e963f7
~/.cache/sccache/4/c/4c47f1f2585a9b88346550f21f5b9664081002701acd251c8c6d3e8485398b17
~/.cache/sccache/4/c/4c5ec3d412180d6b8f5ae8838580ac82b5462af7f1d466dc21160b6077d696cd
~/.cache/sccache/4/8
~/.cache/sccache/4/8/48ccf5503c3d4f96862979f70ac6cd0198c3cfd6dde2147ef79462668952e29b
~/.cache/sccache/4/9
~/.cache/sccache/4/9/49e031c00985018dd86579cbdfbd34e253c2c01c1094287dc76655b7417d427c
~/.cache/sccache/4/9/49a79a6483cca1ea494854e526568434e54b722c392785c24097248581295037
~/.cache/sccache/4/f
~/.cache/sccache/4/f/4ffbfac9fcd8a75b75487bc9231eb2623056bff796f1ee6b370da63e25eca8a1
~/.cache/sccache/4/e
~/.cache/sccache/4/e/4e3b33733c4f42446a97865aaaeb537707144bcfeb0e755e236f3717195b4c81
~/.cache/sccache/4/e/4e6306464c2c199e9bbc3a625c27f649dd69d9cd18bcb44a7bdd0e2f3a1e7b9f
~/.cache/sccache/4/e/4e4745a3f37f66aed9169199c528291d6c0515bf13b7d0e4e1323b0c63474cd1
~/.cache/sccache/4/5
~/.cache/sccache/4/4
~/.cache/sccache/4/4/44a03643ce3d7eeb8992e730041eff7653f6a97c7b465292aa40c14273711381
~/.cache/sccache/4/4/44603e2f43e3254fc18888fbaf5392c73703557b2c15ba0d65aa8d13ba0b2319
~/.cache/sccache/4/1/41bf4bb32a9a6b74600a6de099eb79fe883ee923fbad66fcd398093e1f122aca
~/.cache/sccache/4/6
~/.cache/sccache/4/6/46c65d24182851f18538c44b9e69ea4efdaf76db51227d8570f623768587c6f9
~/.cache/sccache/4/6/461bfd5f090ee96ac7c94cd9b343aaa93b483d3eb51a429d6204e0d62e8be68e
~/.cache/sccache/4/7
~/.cache/sccache/4/7/479c4ecafe98e1600c33fd86df8c922bf7f14dbe41786bf7d1eb01c010f8cc22
~/.cache/sccache/preprocessor/2/6/f
~/.cache/sccache/preprocessor/b/8/8
~/.cache/sccache/preprocessor/b/8/6
~/.cache/sccache/preprocessor/b/6/b
~/.cache/sccache/preprocessor/c/3/6
~/.cache/sccache/preprocessor/3/5/6
~/.cache/sccache/preprocessor/3/4/a
~/.cache/sccache/preprocessor/1/4/c
~/.cache/sccache/preprocessor/7/b/e
~/.cache/sccache/1/d/1d65c179e1c0253a92d56d49f413a9b0c6a3f07cf2333dda1ddc0fe0d27b6a54
~/.cache/sccache/1/2
~/.cache/sccache/1/2/126b24d2a3665f7fab50e8ea0ec5d2a58c910d38d25a83ddb5a41923b8f8886b
~/.cache/sccache/1/b
~/.cache/sccache/1/b/1b6323452def2f7bcd233ae773da9906d810e203b733c9c81f0c02cd6c213f37
~/.cache/sccache/1/b/1b0d48dd4510b275acea37931500cb69dda42c25f92418554be200d36d80a6ac
~/.cache/sccache/1/a
~/.cache/sccache/1/a/1a1d3c95b0648dee5916c099c6313abfc53452b6856b800324e1fbe89aad3bd8
~/.cache/sccache/1/a/1a93d7ba570bef7d271afbcf4b955cac6f9a1874e31b1553b15317173f00d196
~/.cache/sccache/1/c
~/.cache/sccache/1/c/1ccdd75f109a4fab183ce727a8d1dba0d1ccd6ac93c3ef63495adff90fc8909c
~/.cache/sccache/1/c/1c5fca1b8dfabced46a80aceec2e00956c48a4a759db72c4442fb6490cf723f1
~/.cache/sccache/1/8
~/.cache/sccache/1/8/187ea25601993abb774eafb17ccaf74a5ace33b1c8cb338832020f8c873384ce
~/.cache/sccache/1/9
~/.cache/sccache/1/9/192a067a9b922119476843782181c15ae489a58c86b4446dbbf10e71091e7f2e
~/.cache/sccache/1/9/1993e70ba4baaf7bdc41fa62a4d3ec6008d0e7915cf878cfe3b44b35511dbd53
~/.cache/sccache/1/9/197b1dee3526407c84095077cc5e2c303e7f69cc32c5d00e7c47ad181a13ebfc
~/.cache/sccache/1/9/19f2ebccc00c0cb6b4e08600820455f49f36aade48f0e94d69695588c7495431
~/.cache/sccache/1/f
~/.cache/sccache/1/f/1f3a69219c14c118bd00165479786ae0d0fde6d0aa0accfcdd16faf926083044
~/.cache/sccache/1/e
~/.cache/sccache/1/e/1e2f1533d99c7341d608c0b0f86611bd9a3ca2b5093d49c490432de4fe90ffd1
~/.cache/sccache/1/5
~/.cache/sccache/1/3
~/.cache/sccache/1/3/139c0bd79ed735a519380cf629d9889a9aad6a6d469fc57c8b26016bdf343aa1
~/.cache/sccache/1/3/13c3603382dd2cd437683cd1a36fef71c30dd6f4582aa662f53e08a7551a1849
~/.cache/sccache/1/4
~/.cache/sccache/1/4/14afd232f33b0de8d555639e8633e187d23623d2bf0ba8de02e34c01984209b4
~/.cache/sccache/1/1
~/.cache/sccache/1/1/11a901955f3457a64029e90faf89c9bc156ecdf9f165b8073cbea4a64f3a1297
~/.cache/sccache/1/1/11ad04daadf98e4b5c741a07d44a3fec4e9bb5e7facd103813ccd1e60a8fc22d
~/.cache/sccache/1/6
~/.cache/sccache/1/6/16c20515c82e6f02dcc8c89c8dd6d4e4395933657c9620f847494ed1ba5d03d5
~/.cache/sccache/1/7
~/.cache/sccache/1/7/171c8f3af0d3209b8353b4607e668cacfd748c6032bc6c6bdf86d2eb4010ff1c
~/.cache/sccache/6/d
~/.cache/sccache/6/d/6db85b34a64304ffa6bc4f12c0166d6dd13e819e8c750c9c79effd1ef1423fc7
~/.cache/sccache/6/d/6d1a4934a879ad8fe8fe8e7338b7338146e0e13733e3fed137e12e4bdc2bcb4a
~/.cache/sccache/6/d/6d729d6a2e0956402e01a5ab055a4d992cd4e5ce8e873fe938704d33a7ebb4f8
~/.cache/sccache/6/d/6d30bbe75c4109499107fa49ff16cfa5039fa744b06c59094bf191f0a2503569
~/.cache/sccache/6/0
~/.cache/sccache/6/0/607ff6e1d520ebc24043913eb39a3bee3e9d37c60b0bfb066c3485ff46a31b91
~/.cache/sccache/6/0/60356971d1dfb896a3d839cb8f8011264216b4bad240cd3246bec15c7a0d2c70
~/.cache/sccache/6/0/603f2f6e427efe730663b2d7f4890fa8b6603c34741b16f8a8b703587487d81f
~/.cache/sccache/6/0/60a7e86d4740f19b99c139cc972a61b70d92c20221894403bd58e323a3f00a74
~/.cache/sccache/6/0/60dd306cf4e503328eaec9c08b9d27b2d1d2baa7b3c76e545af668cad8730b60
~/.cache/sccache/6/2
~/.cache/sccache/6/2/62fe2edcd71a834c47b6191a3174a663cdaa48995fcd33adc5cea4e191615695
~/.cache/sccache/6/2/629f3d9a8cb51d0ab3f9b22ac1d8c827447c541e254bfe6bfa952fe990032162
~/.cache/sccache/6/b
~/.cache/sccache/6/b/6b49d8eba6945314567e75c1bd836b73ecc5035d2c2e1ddef5e45a81f44235a1
~/.cache/sccache/6/b/6bd448d0b8ac426b431ff119bd4cb71ee6c136f6816907e9b26a2b1597df8b30
~/.cache/sccache/6/b/6b8d4a8f278543366a016e05a83545c7544f4ceee5f644b45134357f84706b58
~/.cache/sccache/6/b/6ba671f3517299e732ea0a30349ca523890b40a8b34f5c04041f9be8ac95311b
~/.cache/sccache/6/a
~/.cache/sccache/6/a/6a9d1dd3760a290a52e63c52127396247722efeae8f33ea9aaa5db65e5dbf801
~/.cache/sccache/6/a/6ae68fb16ce229cd33a6bd7a42095fcbbc72ebea648ce3b06d830c506184cf69
~/.cache/sccache/6/a/6a7d92de2973126efa6cf77ec1feb69b90323b130ce0ac307bafd46fb2702522
~/.cache/sccache/6/c
~/.cache/sccache/6/c/6cb42f46032b80f542a6008a86d13f89576c4bfd4354dfc374983029ef707625
~/.cache/sccache/6/c/6cf888cf70c613432b23f00dd71f7d70aa893f3f22fe6143947720d83161775a
~/.cache/sccache/6/c/6c5cd6bbfdb8c4adb0986dbb6d636c844dac3887301c1d28c5d642e20f39eafc
~/.cache/sccache/6/8
~/.cache/sccache/6/9
~/.cache/sccache/6/9/69a350183e046b17f8848fe979c6da95c71162c980ec856abf9c0995b8af0e20
~/.cache/sccache/6/9/6985d60b03d1aa1310ce40f5f33955c3ae59a3f3ff4e42900cd45a48d76d7119
~/.cache/sccache/6/9/696732597d3846958a2c7d64c65f95032051855768d0f1411ebcf497136d5f8a
~/.cache/sccache/6/9/690ee77bcc7cbd16728c3b16c400a09b288005ad935a89030c20354050e4d9a1
~/.cache/sccache/6/f
~/.cache/sccache/6/f/6fe31561fc16e0a8ff001fb12455c0cd20cecdc6fe9e98fac671c89c83fe8c58
~/.cache/sccache/6/f/6f580fb122a88ad71ba6812a89818ca5a9d88c652c9f96915c18c511a94131e3
~/.cache/sccache/6/f/6f8cc8d65dc1d6d2be2bfca275cc6c0a84a835416657beacfd51d6684994cbbe
~/.cache/sccache/6/e
~/.cache/sccache/6/e/6ef3df0d4a9bb6222881f4bea99ff21abdff782c693e31d1bf0fdde7da2db019
~/.cache/sccache/6/e/6e9ce5e130d2c534d01a7c627ce2cb545fe0f626bc3fa2ad8463445ec4a8c336
~/.cache/sccache/6/e/6eab81cd59d5e63d54018e614863ae8d4be5605c064b7a9644f452e5f608019e
~/.cache/sccache/6/5
~/.cache/sccache/6/5/6503d0d21bcf1067a5e1b1ff877a78cd387846268dcd95fcbf41476856835659
~/.cache/sccache/6/3
~/.cache/sccache/6/3/63a74d65f0359433731daccf5f5bf036883244cbc4964fc611d9414638f5e787
~/.cache/sccache/6/3/63b9e21948fcd5c33478a9f83563d3cca731e7e28eb964695cba477338b4b90b
~/.cache/sccache/6/4
~/.cache/sccache/6/4/645744a371f44f603faa887e8bf3beede6dd9fad0ee32bdf178cfe0d03cc7c94
~/.cache/sccache/6/4/640fe8269024553c7e95b4c4553e520a38a42cecf2c691bde7e73f27bb471c0b
~/.cache/sccache/6/4/6434db9970b9e5e28d3adb8e7c2fbe7970cd59cf2197042a0bd50e277aa00cbd
~/.cache/sccache/6/1
~/.cache/sccache/6/1/6139a294d20f63d3d9dd6edbf23827662b504c22ace9e035d34e7001b03a31d5
~/.cache/sccache/6/1/614d4e557da14c16c29cc58846702ea5d35ef8a8e5ea2e379955d7733daa245f
~/.cache/sccache/6/1/615627dfda48de332af6216f472c83f8bfb333329b5f52aea57eefe56b9c38f0
~/.cache/sccache/6/1/61ce0791b7862993099e35bd0f7e8873e9da0ece16914c19449b66ba065aa390
~/.cache/sccache/6/1/6105aeacca3a62be453445aedaa6bb53028f95d86c02b62d9e5cb14db19f7ad4
~/.cache/sccache/6/1/610fbc2248cc2194d7157f527634f96eb427af52902d6a1c78769a2d5f0dd4f5
~/.cache/sccache/6/1/61c50a8a01809c71f111ce4c3a0839edd4020541f5d61a12ad5657c5cbb01aca
~/.cache/sccache/6/1/610d39297cc821eb44fc25c83c35c13e34bb4f986175ca43b74313224c9ff924
~/.cache/sccache/6/6
~/.cache/sccache/6/6/667c7a96f38a9ef0c9618f771081771385040eb8eb5d5448b9ba7fb14ef18c0e
~/.cache/sccache/6/6/66932431e8b25d64eae0c34a0c8eae692a1fe868c8659566574b6fd789fd9217
~/.cache/sccache/6/7
~/.cache/sccache/6/7/6732a19b486d0d1ed926c0217f10156130e4c9f144cba2c43684be9d0b759b7f
~/.cache/sccache/6/7/6747d2b4abf76639a2a476f479e6bf18dd98607755c364c2ace029cb515a906d
~/.cache/sccache/6/7/6728406bb51d343e84e4fd4d1b073e522b24550f7c246e5844b897e7523d65c9
~/.cache/sccache/7/d
~/.cache/sccache/7/d/7d845f243a22e4a2a749ca04afc2c78b7cd37d86ba2865aec177bb0f20b1a8ed
~/.cache/sccache/7/0
~/.cache/sccache/7/2
~/.cache/sccache/7/2/72f51a7a139d38d717ebc760bf3e38f4960b14537010110ddbf4e70b6d9d7975
~/.cache/sccache/7/2/72b774d8915a9192c583705332aa4cd9471df5484e901310d7e131826c081271
~/.cache/sccache/7/2/72023cb44b06e9bdd81eecbf4e84e13e7c56ab79388a010d11283bbf7aa4233d
~/.cache/sccache/7/2/727535d92076b903b0ec17582e38887bd0e03c073b38561a1572ad8314040b83
~/.cache/sccache/7/b
~/.cache/sccache/7/a
~/.cache/sccache/7/a/7af7cf65e745a6c53cbf2f70f742bcb13168c8ac70b272bc256e4768ceabe15e
~/.cache/sccache/7/c
~/.cache/sccache/7/8
~/.cache/sccache/7/9
~/.cache/sccache/7/9/791fe2961e3b72b3aa6e3e6f0af6cadc627dbf64a21bd971f879369e6ad41056
~/.cache/sccache/7/f
~/.cache/sccache/7/f/7fe6fba4f0d7bf07e264d859a0de55d67d0289c5ceeef263261b653ff5ff5ff7
~/.cache/sccache/7/f/7f9a7c0a1d259a5d1517c5469ee6e6811a3ab667cb32a8376e0fbb3c6c4df22f
~/.cache/sccache/7/e
~/.cache/sccache/7/e/7eb1afe9efdf6519292e96c641b0c33e01e7970c68d2ef833e13d9eca81bf945
~/.cache/sccache/7/e/7e05717fbe1ba3b3324a8ac308cae91f94a9eb5f26622c2932ee27b3bac593c5
~/.cache/sccache/7/5/75c4e2eb2b3bd3d871926bc1f9bd520e19ce1a23e31003708bd713718425485c
~/.cache/sccache/7/3
~/.cache/sccache/7/3/732462fc5fdae7b647982c88b4b1ea91e8db8e329b025b966f26a96597305cbb
~/.cache/sccache/7/3/73e752ded276525a07e53eef9a7228d7ce819838f927d7478a2671c9a3604671
~/.cache/sccache/7/3/733d9dd7e3e19aa2a3196b841e36913036e0abebb46806e2658b313a5fcba7c3
~/.cache/sccache/7/3/73306d3f4d0a8900fd50d446dc457d4e1d356e6809fef346da47836e36166da0
~/.cache/sccache/7/4
~/.cache/sccache/7/4/74f1ff29cfce5bab45df0976dc3736251fe0bd284a782e81d04d91f113a41fb5
~/.cache/sccache/7/4/74b00a4b09030b4f78b1fb7ea6e809e13fcabc7f2fd4a117330e0fa14ad9b270
~/.cache/sccache/7/4/7485e0ae646ed5714e80b8785c8e0c3d5d9faa5d339b44c3768bbf921209a857
~/.cache/sccache/7/1
~/.cache/sccache/7/1/718bd09c3af7d744723b060e112837bba9bb651fa17964b6fa2116a3578a3373
~/.cache/sccache/7/6
~/.cache/sccache/7/6/7685cdb1357f622dc12dbadef693af3305abdbfbb048e9c8fcb576d867398c56
~/.cache/sccache/7/6/76a2ba51f87ba6386bdf480abfaa52a096bb734423d5a0d600572b5e16470220
~/.cache/sccache/7/6/766a786c358294eda5753f51cd91f9818e6ec4d00692f021e3cf1c96614145bf
~/.cargo
~/.cargo/.global-cache
```

```findings
- file: ess/system/domains/connection.yaml
  line: 606
  category: contract-drift
  severity: blocker
  verdict: CONFIRMED
  origin: undecided
  message: MaterializeObservation still cites the old Monitoring materialize/refusal line locations, so the complete root workspace's required ESS refusal-coverage fence fails.
- file: crates/connectors-cli/tests/one_shot_operations.rs
  line: 695
  category: boundary
  severity: note
  verdict: INFEASIBLE
  origin: undecided
  message: The initial extra-key probe assumes work.requests.list forbids additional properties, although its advertised schema permits them; the observed configured-socket request does not establish caller-controlled route or header authority.
```
