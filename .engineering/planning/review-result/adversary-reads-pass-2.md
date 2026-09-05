---
format: aep.planning-md/1
id: review-result:adversary-reads-pass-2
kind: review-result
status: active
title: Incremental reads adversary, pass 2
relations:
- reviews: story:brain-source-read-operations
revision: 1
---
unit: story:brain-source-read-operations — c00fdd233f8ea922179c5f265c123cfa97dbf292 over b57a674c, plus tests below
verdict: NEEDS-CHANGE
cases: executed 109→112, red 1
origin: introduced 1 / pre-existing 0 / undecided 0
wrote-outside-worktree: 4 paths
needs-coordinator: correct empty optional Jira description admission and verify the preserved case; this is the second and final attack

## 1. git --no-pager diff --stat

```text
 .../tests/incremental_dispatch.rs                  | 52 ++++++++++++++++++++++
 1 file changed, 52 insertions(+)
```

Only the existing test-only file was appended. No implementation, planning, staging, commit, branch, generated artifact, grant or live data was changed. The three cases were written before any test command ran. The before count is correction1.md's reported 109 executed runtime cases, not a preliminary suite run.

## 2. Cases added before execution

All are in `crates/integration-catalog/tests/incremental_dispatch.rs`:

- `adversary_empty_optional_jira_description_is_a_valid_issue` (line 454): real CatalogBackend description/invocation dispatch must preserve an empty optional description. Red now. It supplies an ordinary declared string value to the corrected personal provider route; rejection loses the whole page.
- `adversary_comment_pages_preserve_missing_and_group_restriction_evidence` (line 465): follow returned count from a short comment page; retain group restriction identifier; leave missing jsd_public null; strip nested members; permit empty content; reject zero-size and empty nonterminal pages. Green now.
- `adversary_gitlab_update_searches_retain_confidential_and_draft_state` (line 488): real personal issue/MR dispatch retains restriction/state booleans and empty prose, translates both update bounds and all-state without changing timezone text, and removes author identity. Green now.

The red case alone was the first executed test. Command, with the environment used for all commands:

```console
CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 CARGO_INCREMENTAL=0 CARGO_BUILD_JOBS=2 TMPDIR=/home/operator/.cache/org-brain/fresh-multisource/unit-reads cargo test --manifest-path crates/connectors-runtime/Cargo.toml -p integration-catalog --locked --test incremental_dispatch adversary_empty_optional_jira_description_is_a_valid_issue
```

Exit 101, verbatim output:

```text
   Compiling integration-catalog v0.6.5 (/home/operator/.local/state/worktree/trees/b10x/connectors/wt-365aa8e11239/crates/integration-catalog)
    Finished `test` profile [unoptimized] target(s) in 0.63s
     Running tests/incremental_dispatch.rs (crates/connectors-runtime/target/debug/deps/incremental_dispatch-9b00c4726c5ffbdb)

running 1 test
test adversary_empty_optional_jira_description_is_a_valid_issue ... FAILED

failures:

---- adversary_empty_optional_jira_description_is_a_valid_issue stdout ----

thread 'adversary_empty_optional_jira_description_is_a_valid_issue' (4190985) panicked at /home/operator/.local/state/worktree/trees/b10x/connectors/wt-365aa8e11239/crates/integration-catalog/tests/incremental_dispatch.rs:460:25:
an empty optional description must not block the complete issue page: OperationError { code: Protocol, message: "Incremental read response violates its projection or continuation contract", retriable: false }
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace


failures:
    adversary_empty_optional_jira_description_is_a_valid_issue

test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 10 filtered out; finished in 1.06s

error: test failed, to rerun pass `-p integration-catalog --test incremental_dispatch`
```

## 3. Scoped suites after the new cases existed

`cargo test --manifest-path crates/connectors-runtime/Cargo.toml -p integration-catalog --locked`, with the environment above; exit 101; 39 unit plus 11 dispatch cases executed (49 green, 1 red):

```text
    Finished `test` profile [unoptimized] target(s) in 0.15s
     Running unittests src/lib.rs (crates/connectors-runtime/target/debug/deps/integration_catalog-a06470efb1a5785a)

running 39 tests
test argocd::tests::a_rejected_sign_in_says_so_rather_than_reporting_a_missing_project ... ok
test argocd::tests::a_zero_lifetime_is_refused_rather_than_meaning_forever ... ok
test argocd::tests::a_project_or_role_name_that_could_change_the_path_is_refused ... ok
test argocd::tests::a_login_without_projects_update_is_told_which_grant_it_lacks ... ok
test argocd::tests::a_missing_project_is_not_reported_as_a_permission_problem ... ok
test argocd::tests::an_unreachable_origin_names_the_aperture_as_a_possibility ... ok
test argocd::tests::read_only_acquisition_carries_no_sync_policy ... ok
test argocd::tests::every_generated_policy_is_one_argo_cd_will_accept ... ok
test argocd::tests::an_existing_role_is_reused_and_its_policies_are_left_alone ... ok
test config::tests::an_endpoint_value_reaches_the_resolver_under_its_declared_name ... ok
test config::tests::an_unsupplied_variable_is_absent_rather_than_empty ... ok
test config::tests::a_value_without_operator_approval_says_so_rather_than_claiming_it ... ok
test argocd::tests::the_four_calls_happen_in_order_and_the_token_comes_back ... ok
test confluence_reads::tests::a_declared_link_cannot_smuggle_an_untyped_object ... ok
test argocd::tests::the_password_appears_once_and_the_session_token_never_persists ... ok
test argocd::tests::the_project_is_written_back_whole ... ok
test confluence_reads::tests::missing_revision_is_not_a_successful_observation ... ok
test confluence_reads::tests::search_inputs_remain_closed_bounded_and_expansion_pinned ... ok
test confluence_reads::tests::overlapping_pages_keep_versions_content_and_continuation_without_person_records ... ok
test tests::an_entry_stating_no_user_half_answers_none_rather_than_an_empty_string ... ok
test tests::an_unnamed_entry_keeps_the_address_it_had_before_instances_existed ... ok
test tests::the_instance_derivation_puts_the_provider_in_the_namespace ... ok
test tests::two_named_instances_of_one_provider_get_different_addresses ... ok
test tests::a_basic_credentials_user_half_reaches_the_resolver_from_the_entry ... ok
test tests::a_provider_with_a_fixed_base_url_needs_no_configuration ... ok
test tests::a_request_template_exists_for_every_operation_this_backend_would_offer ... ok
test tests::an_unsupplied_url_variable_refuses_rather_than_reaching_a_literal_placeholder ... ok
test tests::a_network_effect_is_not_an_escalation_but_a_filesystem_one_would_be ... ok
test tests::the_ceiling_reads_declared_facts_rather_than_an_operation_list ... ok
test tests::the_limit_drops_operations_rather_than_the_identities_that_serve_one ... ok
test tests::the_default_ceiling_admits_reads_and_refuses_writes ... ok
test tests::every_declared_user_half_field_names_a_credential_that_actually_wants_one ... ok
test tests::every_catalogued_provider_can_address_a_credential ... ok
test tests::the_aperture_is_derived_from_the_same_declaration_the_request_is ... ok
test tests::effect_class_comes_from_the_declaration_not_the_method ... ok
test tests::a_read_only_first_connection_does_not_hide_an_admitted_write ... ok
test tests::search_and_describe_require_approval_for_every_admitted_write ... ok
test hosted::tests::completion_does_not_overwrite_a_terminal_session_after_verification_awaits ... ok
test hosted::tests::two_people_receive_isolated_connections_and_credential_addresses ... ok

test result: ok. 39 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.03s

     Running tests/incremental_dispatch.rs (crates/connectors-runtime/target/debug/deps/incremental_dispatch-9b00c4726c5ffbdb)

running 11 tests
test confluence_expired_permission_is_not_granted ... ok
test confluence_rate_limit_is_retriable ... ok
test jira_incremental_personal_dispatch_translates_declared_input ... ok
test adversary_empty_optional_jira_description_is_a_valid_issue ... FAILED
test personal_incremental_reads_reject_foreign_projects_and_broken_pages ... ok
test every_personal_incremental_read_rejects_unbounded_or_unknown_input_before_dispatch ... ok
test every_personal_incremental_read_classifies_expired_permission_and_rate_limiting ... ok
test gitlab_incremental_personal_dispatch_preserves_pagination_and_projection ... ok
test adversary_gitlab_update_searches_retain_confidential_and_draft_state ... ok
test adversary_comment_pages_preserve_missing_and_group_restriction_evidence ... ok
test every_personal_incremental_read_projects_its_declared_envelope ... ok

failures:

---- adversary_empty_optional_jira_description_is_a_valid_issue stdout ----

thread 'adversary_empty_optional_jira_description_is_a_valid_issue' (8106) panicked at /home/operator/.local/state/worktree/trees/b10x/connectors/wt-365aa8e11239/crates/integration-catalog/tests/incremental_dispatch.rs:460:25:
an empty optional description must not block the complete issue page: OperationError { code: Protocol, message: "Incremental read response violates its projection or continuation contract", retriable: false }
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace


failures:
    adversary_empty_optional_jira_description_is_a_valid_issue

test result: FAILED. 10 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.13s

error: test failed, to rerun pass `-p integration-catalog --test incremental_dispatch`
```

`cargo test --manifest-path crates/connectors-runtime/Cargo.toml -p integration-jira -p integration-gitlab --locked`, with the same environment; exit 0; 17 Jira plus 45 GitLab cases executed:

```text
   Compiling server v0.6.5 (/home/operator/.local/state/worktree/trees/b10x/connectors/wt-365aa8e11239/crates/server)
   Compiling integration-jira v0.6.5 (/home/operator/.local/state/worktree/trees/b10x/connectors/wt-365aa8e11239/crates/integration-jira)
   Compiling integration-gitlab v0.6.5 (/home/operator/.local/state/worktree/trees/b10x/connectors/wt-365aa8e11239/crates/integration-gitlab)
    Finished `test` profile [unoptimized] target(s) in 5.78s
     Running unittests src/lib.rs (crates/connectors-runtime/target/debug/deps/integration_gitlab-3e777fa2337b4ae6)

running 45 tests
test backend::git_fetch::tests::source_authority_digest_comparison_checks_every_byte ... ok
test backend::git_fetch::tests::upload_pack_request_is_bound_to_exact_commit_and_depth ... ok
test backend::git_fetch::tests::upstream_repository_is_derived_without_forwarding_provider_metadata ... ok
test backend::git_fetch::tests::removed_principal_connection_revokes_a_live_session ... ok
test backend::git_fetch::tests::upload_pack_stream_is_bounded_and_spends_the_session ... ok
test backend::git_fetch::tests::discovery_advertises_only_the_exact_default_branch_snapshot ... ok
test backend::git_fetch::tests::idempotent_replay_keeps_locator_and_rotates_transient_authority ... ok
test backend::git_fetch::tests::project_and_branch_authority_reads_overlap_on_creation_and_each_exchange ... ok
test backend::git_fetch::tests::current_grant_and_provider_default_tip_are_revalidated ... ok
test backend::git_fetch::v2::tests::capabilities_require_v2_shallow_sha1_and_strip_expanding_features ... ok
test backend::git_fetch::tests::unsupported_protocol_is_refused_before_provider_egress ... ok
test backend::tests::a_gitlab_refresh_response_without_scope_is_accepted_for_live_reverification ... ok
test backend::incremental_reads::tests::malformed_continuations_permissions_and_foreign_projects_fail_closed ... ok
test backend::git_fetch::v2::tests::commands_are_closed_and_prefixes_cannot_expand_upstream_discovery ... ok
test backend::tests::datasource_cursors_are_bound_to_connection_and_project ... ok
test backend::tests::datasource_projection_drops_sensitive_and_unknown_fields ... ok
test backend::git_fetch::v2::tests::request_framing_refuses_truncation_ambiguity_and_oversize ... ok
test backend::tests::pat_shape_rejects_whitespace_and_oversize_values ... ok
test backend::tests::profiles_are_closed_and_self_service ... ok
test backend::tests::origins_are_exact_https_only ... ok
test backend::tests::incremental_reads_without_an_owned_read_connection_refuse_before_dispatch ... ok
test backend::tests::recorded_scopes_are_the_retained_subset_sorted_and_deduped ... ok
test backend::tests::repository_file_paths_cannot_traverse_or_change_root ... ok
test backend::tests::malformed_state_and_empty_grant_policy_still_fail_closed ... ok
test backend::git_fetch::tests::v2_drop_budget_revocation_and_foreign_authority_fail_closed ... ok
test backend::tests::the_refresh_policy_still_requires_bearer_and_a_refresh_token ... ok
test transport::tests::oauth_forms_encode_secret_delimiters_without_logging_values ... ok
test backend::tests::the_adapter_carries_every_field_gitlab_sends ... ok
test backend::tests::the_refresh_policy_ignores_the_two_fields_that_path_recomputes ... ok
test backend::tests::legacy_connection_starts_unusable_and_preserves_pending_custody ... ok
test backend::tests::project_admission_refuses_a_non_advancing_continuation ... ok
test backend::tests::incremental_activity_operations_are_read_only_and_discoverable ... ok
test transport::tests::page_decoding_reads_only_the_selected_cursor_header ... ok
test backend::git_fetch::tests::per_principal_capacity_is_atomic_and_never_evicts_a_live_session ... ok
test backend::tests::project_admission_reaches_a_repository_after_the_first_hundred ... ok
test backend::git_fetch::tests::v2_negotiation_is_bound_to_generation_and_only_completed_fetch_spends_it ... ok
test backend::git_fetch::v2::tests::pack_is_streamed_but_final_framing_and_sections_are_enforced ... ok
test backend::tests::the_exchange_policy_refuses_everything_the_inline_condition_refused ... ok
test backend::git_fetch::tests::global_capacity_refuses_without_evicting_an_inflight_other_principal ... ok
test backend::git_fetch::v2::tests::refs_filter_prefix_collisions_and_verify_full_response_before_emission ... ok
test backend::git_fetch::tests::stream_expiry_revokes_a_stalled_upload ... ok
test backend::incremental_reads::tests::incremental_inputs_preserve_integer_bounds_and_closed_fields ... ok
test backend::incremental_reads::tests::two_pages_preserve_overlap_revisions_and_scrub_unknown_members ... ok
test backend::git_fetch::http_tests::real_v2_clone_preserves_depth_and_reduces_many_ref_discovery_bytes ... ok
test backend::tests::repository_file_paths_are_encoded_as_one_gitlab_segment ... ok

test result: ok. 45 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.51s

     Running unittests src/lib.rs (crates/connectors-runtime/target/debug/deps/integration_jira-69891cee2672e919)

running 17 tests
test backend::auth::tests::oauth_scopes_are_canonical_and_exact ... ok
test backend::auth::tests::the_refresh_policy_allows_a_response_that_rotates_only_the_access_token ... ok
test backend::auth::tests::the_exchange_policy_refuses_everything_the_inline_condition_refused ... ok
test backend::auth::tests::the_service_policy_needs_only_the_read_scope_and_no_refresh_token ... ok
test backend::datasource::tests::projection_drops_sensitive_and_unknown_provider_fields ... ok
test backend::operations::incremental::tests::project_filtering_does_not_hide_a_provider_continuation ... ok
test backend::operations::incremental::tests::missing_repeated_cursor_or_foreign_issue_refuses_the_page ... ok
test backend::operations::tests::issue_keys_bind_an_exact_project ... ok
test backend::operations::tests::all_writes_require_approval ... ok
test backend::tests::delegated_connection_is_withdrawn_when_its_grant_changes ... ok
test backend::tests::organization_and_user_profiles_are_distinct ... ok
test backend::operations::tests::operation_inputs_are_closed_and_bounded_before_request_assembly ... ok
test backend::datasource::tests::schemas_are_closed_and_projection_is_stable ... ok
test backend::operations::incremental::tests::overlapping_pages_keep_stable_issue_ids_revisions_and_canonical_links ... ok
test backend::operations::tests::incremental_issue_read_admits_bounded_overlap_without_query_injection ... ok
test backend::operations::tests::operation_outputs_are_closed_safe_projections ... ok
test backend::operations::incremental::tests::bounded_comment_read_is_admitted_and_preserves_restrictions ... ok

test result: ok. 17 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.09s

   Doc-tests integration_gitlab

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

   Doc-tests integration_jira

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

```

The full repository gate and catalog regeneration remain coordinator-owned. No unrelated workspace was compiled. Existing per-tree target was used, with no CARGO_TARGET_DIR. Available disk was 52 GiB before execution and 50 GiB afterwards, above the 10 GiB stop floor.

## 4. Findings covering the corrected commit

| File:line | Verdict / origin / severity | What was measured | What reaches it |
|---|---|---|---|
| crates/integration-catalog/src/incremental_reads/jira.rs:135 | NEEDS-CHANGE / introduced / blocker | Empty optional description yields non-retryable Protocol in the added dispatch assertion at tests/incremental_dispatch.rs:460, isolated and package run exit 101. | CatalogBackend::invoke calls incremental_reads::project for jira-issue-search; its personal Jira optional-field closure calls the nonempty string helper. Every returned issue is projected before the page succeeds. The native path delegates to datasource::insert_optional_string at datasource.rs:599, which accepts the same empty string, and its detail schema has no nonempty constraint. |

The requested read is reachable through actual personal CatalogBackend composition, as exercised by the fixture's real describe/invoke sequence and trusted request lowering. An optional description represented as an empty string is within the existing string contract and native behavior; this is not an invalid credential, impossible state or constructed foreign record. No claim is made about how often the live deployment currently contains this value. Optional text admission must preserve valid empty content while retaining its size/type bounds; identity and revision fields can keep nonempty checks.

Origin is introduced: the new personal adaptation file and operation are absent at b57a674c, as read by the base diff. No base checkout or tree mutation occurred.

Trend from pass one: all four earlier findings are resolved in the scoped dispatch rerun: Jira query translation, GitLab projection/continuation envelope, Confluence retryable 429, and Confluence NotGranted permission denial. Carried findings: 0. Resolved: 4. New: 1. This is final-pass residue for coordinator correction verification, not a request for a third attack.

## 5. Attacked without another break

The preserved matrix rejects oversized, fractional and unknown input before credential reads or transport for all ten operations.
Comment short-page continuation, group visibility, absent publication evidence and nested identity omission held through the actual personal dispatch path.
GitLab issue/MR confidentiality and draft booleans and both update-window query values held through the actual personal dispatch path.
All four original red cases now run green; all existing native Jira/GitLab package cases remain green.
Native/personal comment projection and declared bounds were compared in source; no new direct native HTTP fixture was added, and source inspection is not represented as a test execution.
No live credentials, remote messages, provider grants or shared daemon state were accessed.
The coordinator raised a possible ADF body mismatch before this pass closed. The corrected providers and authored specs bind Jira `/rest/api/2/search/jql` and `/rest/api/2/issue/{issueIdOrKey}/comment`. [Atlassian’s v2 introduction](https://developer.atlassian.com/cloud/jira/platform/rest/v2/intro/#version) identifies ADF support as a v3 distinction. A v3-only ADF fixture was therefore not promoted to a reachable v2 defect or added as a blocker. The official v2 search and comment references were also opened; no provider-version migration occurred.

## 6. Every authored path outside the worktree

- /home/operator/.cache/org-brain/fresh-multisource/unit-reads/adversary-2-empty-description.log
- /home/operator/.cache/org-brain/fresh-multisource/unit-reads/adversary-2-suite.log
- /home/operator/.cache/org-brain/fresh-multisource/unit-reads/adversary-2-native-suite.log
- /home/operator/.cache/org-brain/fresh-multisource/unit-reads/adversary-2.md

Compiler output remained in the existing worktree's crates/connectors-runtime/target. TMPDIR was the assigned scratch directory. No separate scratch script, vendor download or build directory was authored.

```findings
- file: crates/integration-catalog/src/incremental_reads/jira.rs
  line: 135
  category: contract-drift
  severity: blocker
  verdict: NEEDS-CHANGE
  origin: introduced
  message: Personal Jira issue search rejects a valid empty optional description and fails the whole page although the native projection and declared optional string contract accept it.
```
