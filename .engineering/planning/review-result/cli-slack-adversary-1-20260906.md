---
format: aep.planning-md/1
id: review-result:cli-slack-adversary-1-20260906
kind: review-result
status: active
title: Slack local write adversary pass 1
refs:
- provider: git
  reference: afca141990fb81052dc44852436d9e55252c854b
relations:
- reviews: story:personal-local-writes-are-usable
revision: 1
---
unit: story:personal-local-writes-are-usable — afca141990fb81052dc44852436d9e55252c854b plus the test-only working diff
verdict: nothing found
cases: executed 72→75, red 0
origin: introduced 0 / pre-existing 0 / undecided 0
wrote-outside-worktree: 15 report/log files, assigned temporary root and shared compiler cache; inventory below
needs-coordinator: none for this pass; publication, installed-daemon delivery and integration gates remain coordinator-owned

```text
 .../tests/local_catalog_writes.rs                  | 78 ++++++++++++++++++++--
 1 file changed, 73 insertions(+), 5 deletions(-)
```

## 1. Test-only diff

The diff above is `git --no-pager diff --stat` against the clean reviewed head. Only `crates/connectors-runtime/tests/local_catalog_writes.rs` changed. No existing case was removed, skipped, or weakened. The fixture now permits an explicit response-body override; its default response and every original assertion remain unchanged. No implementation file was modified, even temporarily.

Read the complete unit diff from source base `4769ce32a331bb8daddaf066b69459c4148fb749`, the head commit, the story's Acceptance and scope sections, the implementor's runner report and the callers through CatalogBackend, BackendRegistry and LocalOperationDaemon. The production delta is actionable refusal guidance; the original mixed-grant discovery fix is already in the assigned base.

## 2. Cases written before the first test execution

All three cases below were written before running any case or suite. Each was then run alone, in the order shown. They were green on their first executions, so there is no red output to report. All external provider traffic is synthetic; no installed daemon or live provider was contacted.

### adversary_http_200_application_refusal_survives_the_documented_socket_path

File: `crates/connectors-runtime/tests/local_catalog_writes.rs:377`. The new guide says Slack may return an application error in HTTP 200. This case returns exactly HTTP 200 with `ok: false` and asserts the complete provider refusal remains visible in the result after crossing the real local socket. Current result: green.

Command: `env -u CARGO_TARGET_DIR TMPDIR=~/.cache/cw6/s RUSTC_WRAPPER=/usr/bin/sccache CARGO_INCREMENTAL=0 CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 CARGO_BUILD_JOBS=3 cargo test --manifest-path crates/connectors-runtime/Cargo.toml -p connectors-runtime --locked --test local_catalog_writes adversary_http_200_application_refusal_survives_the_documented_socket_path -- --exact`

```text
warning: connector-secrets@0.6.5: no live Vault was offered (CONNECTOR_SECRETS_VAULT_ADDR and CONNECTOR_SECRETS_VAULT_TOKEN are unset), so the reqwest HttpTransport is UNEXERCISED by this build: tests/vault_live.rs is compiled #[ignore]d
   Compiling connectors-runtime v0.6.5 (~/.local/state/worktree/trees/b10x/connectors/wt-6ae00b19e482/crates/connectors-runtime)
    Finished `test` profile [unoptimized] target(s) in 2.65s
     Running tests/local_catalog_writes.rs (crates/connectors-runtime/target/debug/deps/local_catalog_writes-851af512c879ce4f)

running 1 test
test adversary_http_200_application_refusal_survives_the_documented_socket_path ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 7 filtered out; finished in 1.12s

```

Exit: 0

### adversary_a_read_description_cannot_authorize_a_different_write_operation

File: `crates/connectors-runtime/tests/local_catalog_writes.rs:397`. A caller may retain a description for a read and later attempt a post. Even with an otherwise writable selected connection, that different operation's description is refused as StaleAuthority before fake egress. Current result: green.

Command: `env -u CARGO_TARGET_DIR TMPDIR=~/.cache/cw6/s RUSTC_WRAPPER=/usr/bin/sccache CARGO_INCREMENTAL=0 CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 CARGO_BUILD_JOBS=3 cargo test --manifest-path crates/connectors-runtime/Cargo.toml -p connectors-runtime --locked --test local_catalog_writes adversary_a_read_description_cannot_authorize_a_different_write_operation -- --exact`

```text
warning: connector-secrets@0.6.5: no live Vault was offered (CONNECTOR_SECRETS_VAULT_ADDR and CONNECTOR_SECRETS_VAULT_TOKEN are unset), so the reqwest HttpTransport is UNEXERCISED by this build: tests/vault_live.rs is compiled #[ignore]d
    Finished `test` profile [unoptimized] target(s) in 0.23s
     Running tests/local_catalog_writes.rs (crates/connectors-runtime/target/debug/deps/local_catalog_writes-851af512c879ce4f)

running 1 test
test adversary_a_read_description_cannot_authorize_a_different_write_operation ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 7 filtered out; finished in 1.12s

```

Exit: 0

### adversary_an_approval_reference_cannot_raise_the_selected_read_only_grant

File: `crates/connectors-runtime/tests/local_catalog_writes.rs:410`. A caller can supply the optional approval reference on invocation. Presenting an intentionally untrusted fixture reference alongside a valid post description and a selected read-only connection remains NotGranted, with grant guidance and zero fake egress. Current result: green.

Command: `env -u CARGO_TARGET_DIR TMPDIR=~/.cache/cw6/s RUSTC_WRAPPER=/usr/bin/sccache CARGO_INCREMENTAL=0 CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 CARGO_BUILD_JOBS=3 cargo test --manifest-path crates/connectors-runtime/Cargo.toml -p connectors-runtime --locked --test local_catalog_writes adversary_an_approval_reference_cannot_raise_the_selected_read_only_grant -- --exact`

```text
warning: connector-secrets@0.6.5: no live Vault was offered (CONNECTOR_SECRETS_VAULT_ADDR and CONNECTOR_SECRETS_VAULT_TOKEN are unset), so the reqwest HttpTransport is UNEXERCISED by this build: tests/vault_live.rs is compiled #[ignore]d
    Finished `test` profile [unoptimized] target(s) in 0.27s
     Running tests/local_catalog_writes.rs (crates/connectors-runtime/target/debug/deps/local_catalog_writes-851af512c879ce4f)

running 1 test
test adversary_an_approval_reference_cannot_raise_the_selected_read_only_grant ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 7 filtered out; finished in 1.10s

```

Exit: 0

## 3. Affected suite and checks

The implementor's report supplies the before count: runtime 32 unit plus 5 socket cases, catalog 35, total 72. Only after the three individually executed cases above did this pass run the combined affected-package suite. It executed runtime 32 unit plus 8 socket cases and catalog 35, total 75; doc-test lanes each executed zero. The unoffered live Vault warning describes an unchanged dependency lane and is not a claim that Vault or a real provider was tested.

Command: `env -u CARGO_TARGET_DIR TMPDIR=~/.cache/cw6/s RUSTC_WRAPPER=/usr/bin/sccache CARGO_INCREMENTAL=0 CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 CARGO_BUILD_JOBS=3 cargo test --manifest-path crates/connectors-runtime/Cargo.toml -p connectors-runtime -p integration-catalog --locked`

```text
warning: connector-secrets@0.6.5: no live Vault was offered (CONNECTOR_SECRETS_VAULT_ADDR and CONNECTOR_SECRETS_VAULT_TOKEN are unset), so the reqwest HttpTransport is UNEXERCISED by this build: tests/vault_live.rs is compiled #[ignore]d
   Compiling integration-catalog v0.6.5 (~/.local/state/worktree/trees/b10x/connectors/wt-6ae00b19e482/crates/integration-catalog)
    Finished `test` profile [unoptimized] target(s) in 1.14s
     Running unittests src/lib.rs (crates/connectors-runtime/target/debug/deps/connectors_runtime-6d6e2af6f07ff6c2)

running 32 tests
test composition::tests::git_fetch_environment_override_is_atomic_and_secret_free ... ok
test claims::tests::a_journal_this_build_cannot_parse_refuses_to_open ... ok
test composition::tests::an_empty_variable_is_no_store_at_all ... ok
test composition::tests::git_fetch_environment_override_refuses_an_invalid_listener ... ok
test composition::tests::an_unopenable_sqlite_path_is_named_in_the_refusal ... ok
test claims::tests::only_an_event_reference_is_claimable ... ok
test claims::tests::a_claim_survives_a_daemon_restart ... ok
test composition::tests::naming_both_stores_is_refused_rather_than_one_of_them_quietly_winning ... ok
test composition::tests::naming_no_store_is_refused_rather_than_a_database_file_appearing_somewhere ... ok
test composition::tests::the_refusal_names_both_stores_a_deployment_may_choose ... ok
test composition::tests::working_tree_state_roots_are_refused ... ok
test registry::tests::a_topical_datasource_query_that_matches_nothing_returns_the_admitted_set ... ok
test registry::tests::ambiguous_exclusive_claims_fail_with_typed_protocol_errors_before_dispatch ... ok
test registry::tests::direct_dispatch_selects_the_unique_claim_without_not_found_probing ... ok
test registry::tests::duplicate_channel_references_fail_search ... ok
test registry::tests::duplicate_connection_references_fail_search ... ok
test registry::tests::readiness_requires_every_configured_backend ... ok
test registry::tests::search_aggregates_compatible_operations_and_deduplicates_the_operation ... ok
test registry::tests::the_registry_lease_ignores_request_scoped_provenance ... ok
test registry::tests::describe_merges_connections_and_invoke_receives_the_selected_local_lease ... ok
test service_bundle::tests::registration_is_inert_until_an_explicit_overlay_is_present ... ok
test service_bundle::tests::malformed_manifests_and_deployments_are_refused ... ok
test service_bundle::tests::catalog_dispatch_and_backend_ownership_mismatches_are_refused ... ok
test service_bundle::tests::bundle_order_and_policy_projection_are_deterministic ... ok
test service_bundle::tests::identity_and_operation_collisions_are_refused ... ok
test registry::tests::an_undemanded_reference_is_not_spent_at_the_local_seam ... ok
test registry::tests::a_companion_reply_is_claimed_exactly_once_locally ... ok
test claims::tests::parallel_presentations_take_exactly_one_claim ... ok
test tls_listener::tests::established_connection_permit_is_lifetime_bound_and_reads_time_out ... ok
test composition::tests::a_hosted_placement_keeps_its_state_in_a_file_when_no_database_is_offered ... ok
test composition::tests::empty_personal_runtime_binds_and_cleans_without_a_credential_store ... ok
test tls_listener::tests::listener_serves_the_internal_application_over_tls ... ok

test result: ok. 32 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s

     Running tests/local_catalog_writes.rs (crates/connectors-runtime/target/debug/deps/local_catalog_writes-851af512c879ce4f)

running 8 tests
test describing_a_write_directly_skips_the_first_read_only_connection ... ok
test only_read_only_connections_hide_the_write_and_describe_its_missing_grant ... ok
test adversary_a_read_description_cannot_authorize_a_different_write_operation ... ok
test adversary_an_approval_reference_cannot_raise_the_selected_read_only_grant ... ok
test adversary_http_200_application_refusal_survives_the_documented_socket_path ... ok
test stale_description_and_provider_refusal_have_distinct_actionable_results ... ok
test writable_first_still_discovers_describes_and_posts_through_the_writer ... ok
test read_only_first_still_discovers_describes_and_posts_through_the_writer ... ok

test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.14s

     Running unittests src/lib.rs (crates/connectors-runtime/target/debug/deps/integration_catalog-344a4a587b0a9113)

running 35 tests
test argocd::tests::a_project_or_role_name_that_could_change_the_path_is_refused ... ok
test argocd::tests::a_missing_project_is_not_reported_as_a_permission_problem ... ok
test argocd::tests::an_unreachable_origin_names_the_aperture_as_a_possibility ... ok
test argocd::tests::a_login_without_projects_update_is_told_which_grant_it_lacks ... ok
test argocd::tests::a_zero_lifetime_is_refused_rather_than_meaning_forever ... ok
test argocd::tests::a_rejected_sign_in_says_so_rather_than_reporting_a_missing_project ... ok
test argocd::tests::an_existing_role_is_reused_and_its_policies_are_left_alone ... ok
test argocd::tests::every_generated_policy_is_one_argo_cd_will_accept ... ok
test argocd::tests::read_only_acquisition_carries_no_sync_policy ... ok
test config::tests::a_value_without_operator_approval_says_so_rather_than_claiming_it ... ok
test argocd::tests::the_four_calls_happen_in_order_and_the_token_comes_back ... ok
test config::tests::an_endpoint_value_reaches_the_resolver_under_its_declared_name ... ok
test argocd::tests::the_project_is_written_back_whole ... ok
test argocd::tests::the_password_appears_once_and_the_session_token_never_persists ... ok
test config::tests::an_unsupplied_variable_is_absent_rather_than_empty ... ok
test tests::an_entry_stating_no_user_half_answers_none_rather_than_an_empty_string ... ok
test tests::the_instance_derivation_puts_the_provider_in_the_namespace ... ok
test tests::an_unnamed_entry_keeps_the_address_it_had_before_instances_existed ... ok
test tests::two_named_instances_of_one_provider_get_different_addresses ... ok
test tests::a_basic_credentials_user_half_reaches_the_resolver_from_the_entry ... ok
test tests::a_request_template_exists_for_every_operation_this_backend_would_offer ... ok
test tests::the_limit_drops_operations_rather_than_the_identities_that_serve_one ... ok
test tests::effect_class_comes_from_the_declaration_not_the_method ... ok
test tests::a_network_effect_is_not_an_escalation_but_a_filesystem_one_would_be ... ok
test tests::the_ceiling_reads_declared_facts_rather_than_an_operation_list ... ok
test tests::every_catalogued_provider_can_address_a_credential ... ok
test tests::a_provider_with_a_fixed_base_url_needs_no_configuration ... ok
test tests::the_default_ceiling_admits_reads_and_refuses_writes ... ok
test tests::every_declared_user_half_field_names_a_credential_that_actually_wants_one ... ok
test tests::an_unsupplied_url_variable_refuses_rather_than_reaching_a_literal_placeholder ... ok
test tests::the_aperture_is_derived_from_the_same_declaration_the_request_is ... ok
test tests::a_read_only_first_connection_does_not_hide_an_admitted_write ... ok
test tests::search_and_describe_require_approval_for_every_admitted_write ... ok
test hosted::tests::completion_does_not_overwrite_a_terminal_session_after_verification_awaits ... ok
test hosted::tests::two_people_receive_isolated_connections_and_credential_addresses ... ok

test result: ok. 35 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.09s

   Doc-tests connectors_runtime

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

   Doc-tests integration_catalog

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

```

Exit: 0

A formatter check found wrapping only in this pass's new test fixture. The suggested whitespace was applied only in that test file. This was an authoring correction, not an implementation finding; no test assertion or production behavior changed.

Command: `cargo fmt --manifest-path crates/connectors-runtime/Cargo.toml -p connectors-runtime -p integration-catalog -- --check`

```text
Diff in ~/.local/state/worktree/trees/b10x/connectors/wt-6ae00b19e482/crates/connectors-runtime/tests/local_catalog_writes.rs:66:
         Ok(EgressHttpResponse {
             status,
             headers: BTreeMap::new(),
-            body: self.body_override.lock().unwrap().clone().unwrap_or_else(|| {
-                if status == 200 {
-                    br#"{"ok":true,"channel":"C-FIXTURE","ts":"1.000001"}"#.to_vec()
-                } else {
-                    br#"{"ok":false,"error":"fixture-provider-body-must-stay-private"}"#.to_vec()
-                }
-            }),
+            body: self
+                .body_override
+                .lock()
+                .unwrap()
+                .clone()
+                .unwrap_or_else(|| {
+                    if status == 200 {
+                        br#"{"ok":true,"channel":"C-FIXTURE","ts":"1.000001"}"#.to_vec()
+                    } else {
+                        br#"{"ok":false,"error":"fixture-provider-body-must-stay-private"}"#
+                            .to_vec()
+                    }
+                }),
         })
     }
 
```

Exit: 1

Command: `env -u CARGO_TARGET_DIR TMPDIR=~/.cache/cw6/s RUSTC_WRAPPER=/usr/bin/sccache CARGO_INCREMENTAL=0 CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 CARGO_BUILD_JOBS=3 cargo clippy --manifest-path crates/connectors-runtime/Cargo.toml -p connectors-runtime -p integration-catalog --all-targets --locked -- -D warnings`

```text
warning: connector-secrets@0.6.5: no live Vault was offered (CONNECTOR_SECRETS_VAULT_ADDR and CONNECTOR_SECRETS_VAULT_TOKEN are unset), so the reqwest HttpTransport is UNEXERCISED by this build: tests/vault_live.rs is compiled #[ignore]d
    Checking connectors-runtime v0.6.5 (~/.local/state/worktree/trees/b10x/connectors/wt-6ae00b19e482/crates/connectors-runtime)
    Finished `dev` profile [unoptimized] target(s) in 0.43s
```

Exit: 0

Command: `cargo fmt --manifest-path crates/connectors-runtime/Cargo.toml -p connectors-runtime -p integration-catalog -- --check`

```text
```

Exit: 0

Command: `git diff --check`

```text
```

Exit: 0

## 4. Findings

Nothing found in this pass against afca141990fb81052dc44852436d9e55252c854b plus the test-only diff; no severity, verdict or origin rows to route.

## 5. Attacked boundaries

- The new guide's HTTP 200 application-error caveat matches the real socket result: `ok: false` remains visible.
- A read description cannot be reused for a post, even through an otherwise writable connection.
- A supplied approval reference does not raise the selected read-only grant.
- The existing mixed-order, selected-credential, entirely read-only, stale-description and HTTP 403 cases still run and pass.

This pass exercises the catalog, registry and socket with synthetic egress. It does not establish source publication, executable selection, daemon replacement, a real Slack send, a hosted approval flow, or the full twelve-workspace gate. Those were outside the assigned adversary scope. No approval or independent-verifier claim is made.

## 6. External path inventory

The coordinator-written brief in this directory was read only. This pass wrote the following 15 persistent report/log files:

```text
~/.cache/connectors-cli-wave-20260906/personal-local-writes-are-usable/adversary-1/http-200.log
~/.cache/connectors-cli-wave-20260906/personal-local-writes-are-usable/adversary-1/http-200.exit
~/.cache/connectors-cli-wave-20260906/personal-local-writes-are-usable/adversary-1/crossed-lease.log
~/.cache/connectors-cli-wave-20260906/personal-local-writes-are-usable/adversary-1/crossed-lease.exit
~/.cache/connectors-cli-wave-20260906/personal-local-writes-are-usable/adversary-1/approval-grant.log
~/.cache/connectors-cli-wave-20260906/personal-local-writes-are-usable/adversary-1/approval-grant.exit
~/.cache/connectors-cli-wave-20260906/personal-local-writes-are-usable/adversary-1/suite.log
~/.cache/connectors-cli-wave-20260906/personal-local-writes-are-usable/adversary-1/suite.exit
~/.cache/connectors-cli-wave-20260906/personal-local-writes-are-usable/adversary-1/fmt.log
~/.cache/connectors-cli-wave-20260906/personal-local-writes-are-usable/adversary-1/fmt.exit
~/.cache/connectors-cli-wave-20260906/personal-local-writes-are-usable/adversary-1/clippy.log
~/.cache/connectors-cli-wave-20260906/personal-local-writes-are-usable/adversary-1/clippy.exit
~/.cache/connectors-cli-wave-20260906/personal-local-writes-are-usable/adversary-1/fmt-final.log
~/.cache/connectors-cli-wave-20260906/personal-local-writes-are-usable/adversary-1/fmt-final.exit
~/.cache/connectors-cli-wave-20260906/personal-local-writes-are-usable/adversary-1/report.md
```

Assigned ephemeral fixture root: `~/.cache/cw6/s`. Tests created and removed their own temporary fixtures there. The mandated compiler wrapper may write shared cache state under `~/.cache/sccache`; that shared cache is not this unit's cleanup surface. Compiler output remained inside the assigned worktree at `~/.local/state/worktree/trees/b10x/connectors/wt-6ae00b19e482/crates/connectors-runtime/target`. No `CARGO_TARGET_DIR` was set and no scratch path outside these assignments was used.

```findings
[]
```



Report publication: local home prefixes are mechanically replaced with `~`; raw runner output remains in private wave scratch. Counts, assertions and findings are unchanged.
