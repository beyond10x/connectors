---
format: aep.planning-md/1
id: review-result:adversary-reads-pass-1
kind: review-result
status: active
title: Incremental reads adversary, pass 1
relations:
- reviews: story:brain-source-read-operations
revision: 1
---
unit: story:brain-source-read-operations, commit a402cc7e57e96d4156f76aa63410faa35be20069 over b57a674cc01c70c28363dc1df63f0b75d3b2b363 plus test-only additions
verdict: NEEDS-CHANGE
cases: executed 39→43, red 4
origin: introduced 4 / pre-existing 0 / undecided 0
wrote-outside-worktree: 6 paths
needs-coordinator: route all four failures to implementation, including personal catalog dispatch; do not activate new sources yet

## 1. Worktree diff

`git --no-pager diff --stat` (tracked paths; empty):
```text
```
The new file is deliberately untracked because staging is prohibited. `git --no-pager diff --no-index --stat /dev/null crates/integration-catalog/tests/incremental_dispatch.rs`:
```text
 .../tests/incremental_dispatch.rs                  | 66 ++++++++++++++++++++++
 1 file changed, 66 insertions(+)
```
Only the new integration test file was changed. No implementation, existing tests, planning, staging, commits or worktree lifecycle were changed.

## 2. Cases written before the first execution

All four cases were authored together in `crates/integration-catalog/tests/incremental_dispatch.rs` before any test command ran. Each was then selected alone, compiled successfully, ran exactly one case and exited 101. All are red now.

- `jira_incremental_personal_dispatch_translates_declared_input` at :35 asserts that a catalog-discovered Jira read composes the documented fixed JQL and maxResults.
- `gitlab_incremental_personal_dispatch_preserves_pagination_and_projection` at :45 asserts the personal backend scrubs identity objects, preserves X-Next-Page and returns the documented envelope. Its first privacy assertion is red; later pagination assertions have not executed in that case. Separate source inspection establishes response_headers is empty and raw JSON is returned.
- `confluence_rate_limit_is_retriable` at :55 asserts HTTP 429 becomes a retriable Unavailable.
- `confluence_expired_permission_is_not_granted` at :63 asserts HTTP 403 becomes NotGranted.

The fixture exercises the public CatalogBackend `bind_stored` → ConnectorBackend `handle(Describe)` → `handle(Invoke)` route with a valid local principal, existing read-only binding and in-memory stored synthetic credential. The stub is only the egress transport, recording the real composed request. No live API or credential was accessed.

Commands used the prefix `CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 CARGO_INCREMENTAL=0 CARGO_BUILD_JOBS=2 TMPDIR=/home/operator/.cache/org-brain/fresh-multisource/unit-reads`, followed by `cargo test --manifest-path crates/connectors-runtime/Cargo.toml -p integration-catalog --test incremental_dispatch <case-name> --locked`.

adversary-jira-red.log — exit 101, verbatim:
```text
   Compiling smallvec v1.15.2
   Compiling percent-encoding v2.3.2
   Compiling icu_normalizer v2.3.0
   Compiling form_urlencoded v1.2.2
   Compiling idna_adapter v1.2.2
   Compiling catalog v0.6.5 (/home/operator/.local/state/worktree/trees/b10x/connectors/wt-365aa8e11239/crates/catalog)
   Compiling idna v1.1.0
   Compiling url v2.5.8
   Compiling protocol v0.6.5 (/home/operator/.local/state/worktree/trees/b10x/connectors/wt-365aa8e11239/crates/protocol)
   Compiling connector-resolve v0.6.5 (/home/operator/.local/state/worktree/trees/b10x/connectors/wt-365aa8e11239/crates/connector-resolve)
   Compiling tokio v1.53.1
   Compiling once_cell v1.21.4
   Compiling tempfile v3.27.0
   Compiling service v0.6.5 (/home/operator/.local/state/worktree/trees/b10x/connectors/wt-365aa8e11239/crates/service)
   Compiling connectors-config v0.6.5 (/home/operator/.local/state/worktree/trees/b10x/connectors/wt-365aa8e11239/crates/connectors-config)
   Compiling integration-catalog v0.6.5 (/home/operator/.local/state/worktree/trees/b10x/connectors/wt-365aa8e11239/crates/integration-catalog)
    Finished `test` profile [unoptimized] target(s) in 6.45s
     Running tests/incremental_dispatch.rs (crates/connectors-runtime/target/debug/deps/incremental_dispatch-53d7669bc238f644)

running 1 test
test jira_incremental_personal_dispatch_translates_declared_input ... FAILED

failures:

---- jira_incremental_personal_dispatch_translates_declared_input stdout ----

thread 'jira_incremental_personal_dispatch_translates_declared_input' (3792244) panicked at /home/operator/.local/state/worktree/trees/b10x/connectors/wt-365aa8e11239/crates/integration-catalog/tests/incremental_dispatch.rs:41:5:
assertion `left == right` failed
  left: None
 right: Some("project = \"PROJ\" AND updated >= 1700000000000 ORDER BY updated ASC, key ASC")
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace


failures:
    jira_incremental_personal_dispatch_translates_declared_input

test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 3 filtered out; finished in 1.00s

error: test failed, to rerun pass `-p integration-catalog --test incremental_dispatch`
```

adversary-gitlab-red.log — exit 101, verbatim:
```text
    Finished `test` profile [unoptimized] target(s) in 0.25s
     Running tests/incremental_dispatch.rs (crates/connectors-runtime/target/debug/deps/incremental_dispatch-53d7669bc238f644)

running 1 test
test gitlab_incremental_personal_dispatch_preserves_pagination_and_projection ... FAILED

failures:

---- gitlab_incremental_personal_dispatch_preserves_pagination_and_projection stdout ----

thread 'gitlab_incremental_personal_dispatch_preserves_pagination_and_projection' (3798424) panicked at /home/operator/.local/state/worktree/trees/b10x/connectors/wt-365aa8e11239/crates/integration-catalog/tests/incremental_dispatch.rs:49:5:
the personal read returned an unprojected person object: [{"id":1,"project_id":7,"updated_at":"2026-09-05T10:00:00Z","user":{"email":"private@example.test"}}]
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace


failures:
    gitlab_incremental_personal_dispatch_preserves_pagination_and_projection

test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 3 filtered out; finished in 1.18s

error: test failed, to rerun pass `-p integration-catalog --test incremental_dispatch`
```

adversary-confluence-rate-red.log — exit 101, verbatim:
```text
    Finished `test` profile [unoptimized] target(s) in 0.19s
     Running tests/incremental_dispatch.rs (crates/connectors-runtime/target/debug/deps/incremental_dispatch-53d7669bc238f644)

running 1 test
test confluence_rate_limit_is_retriable ... FAILED

failures:

---- confluence_rate_limit_is_retriable stdout ----

thread 'confluence_rate_limit_is_retriable' (3798984) panicked at /home/operator/.local/state/worktree/trees/b10x/connectors/wt-365aa8e11239/crates/integration-catalog/tests/incremental_dispatch.rs:59:5:
rate limiting must permit retry: OperationError { code: Unavailable, message: "the provider refused the request with HTTP 429 — the provider is rate-limiting this credential", retriable: false }
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace


failures:
    confluence_rate_limit_is_retriable

test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 3 filtered out; finished in 1.16s

error: test failed, to rerun pass `-p integration-catalog --test incremental_dispatch`
```

adversary-confluence-permission-red.log — exit 101, verbatim:
```text
    Finished `test` profile [unoptimized] target(s) in 0.17s
     Running tests/incremental_dispatch.rs (crates/connectors-runtime/target/debug/deps/incremental_dispatch-53d7669bc238f644)

running 1 test
test confluence_expired_permission_is_not_granted ... FAILED

failures:

---- confluence_expired_permission_is_not_granted stdout ----

thread 'confluence_expired_permission_is_not_granted' (3799490) panicked at /home/operator/.local/state/worktree/trees/b10x/connectors/wt-365aa8e11239/crates/integration-catalog/tests/incremental_dispatch.rs:65:5:
assertion `left == right` failed
  left: Unavailable
 right: NotGranted
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace


failures:
    confluence_expired_permission_is_not_granted

test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 3 filtered out; finished in 1.20s

error: test failed, to rerun pass `-p integration-catalog --test incremental_dispatch`
```

## 3. Scoped package suite, after all four single-case failures

Before count 39 comes from the coordinator's completed integration-catalog gate, not a preemptive rerun. The package suite then ran 39 existing cases and all 4 new cases: 39 passed, 4 failed, zero ignored. Cargo stopped before doctests because the integration target was red.

Command: same environment prefix, `cargo test --manifest-path crates/connectors-runtime/Cargo.toml -p integration-catalog --locked`; exit 101.

```text
   Compiling integration-catalog v0.6.5 (/home/operator/.local/state/worktree/trees/b10x/connectors/wt-365aa8e11239/crates/integration-catalog)
    Finished `test` profile [unoptimized] target(s) in 1.59s
     Running unittests src/lib.rs (crates/connectors-runtime/target/debug/deps/integration_catalog-9c80781441073646)

running 39 tests
test argocd::tests::an_unreachable_origin_names_the_aperture_as_a_possibility ... ok
test argocd::tests::a_missing_project_is_not_reported_as_a_permission_problem ... ok
test argocd::tests::a_login_without_projects_update_is_told_which_grant_it_lacks ... ok
test argocd::tests::a_zero_lifetime_is_refused_rather_than_meaning_forever ... ok
test argocd::tests::every_generated_policy_is_one_argo_cd_will_accept ... ok
test argocd::tests::an_existing_role_is_reused_and_its_policies_are_left_alone ... ok
test argocd::tests::a_rejected_sign_in_says_so_rather_than_reporting_a_missing_project ... ok
test argocd::tests::read_only_acquisition_carries_no_sync_policy ... ok
test argocd::tests::a_project_or_role_name_that_could_change_the_path_is_refused ... ok
test config::tests::a_value_without_operator_approval_says_so_rather_than_claiming_it ... ok
test argocd::tests::the_four_calls_happen_in_order_and_the_token_comes_back ... ok
test argocd::tests::the_password_appears_once_and_the_session_token_never_persists ... ok
test config::tests::an_endpoint_value_reaches_the_resolver_under_its_declared_name ... ok
test config::tests::an_unsupplied_variable_is_absent_rather_than_empty ... ok
test confluence_reads::tests::a_declared_link_cannot_smuggle_an_untyped_object ... ok
test confluence_reads::tests::missing_revision_is_not_a_successful_observation ... ok
test confluence_reads::tests::search_inputs_remain_closed_bounded_and_expansion_pinned ... ok
test argocd::tests::the_project_is_written_back_whole ... ok
test confluence_reads::tests::overlapping_pages_keep_versions_content_and_continuation_without_person_records ... ok
test tests::an_entry_stating_no_user_half_answers_none_rather_than_an_empty_string ... ok
test tests::an_unnamed_entry_keeps_the_address_it_had_before_instances_existed ... ok
test tests::the_instance_derivation_puts_the_provider_in_the_namespace ... ok
test tests::two_named_instances_of_one_provider_get_different_addresses ... ok
test tests::a_basic_credentials_user_half_reaches_the_resolver_from_the_entry ... ok
test tests::an_unsupplied_url_variable_refuses_rather_than_reaching_a_literal_placeholder ... ok
test tests::a_provider_with_a_fixed_base_url_needs_no_configuration ... ok
test tests::every_catalogued_provider_can_address_a_credential ... ok
test tests::a_network_effect_is_not_an_escalation_but_a_filesystem_one_would_be ... ok
test tests::every_declared_user_half_field_names_a_credential_that_actually_wants_one ... ok
test tests::the_default_ceiling_admits_reads_and_refuses_writes ... ok
test tests::the_aperture_is_derived_from_the_same_declaration_the_request_is ... ok
test tests::effect_class_comes_from_the_declaration_not_the_method ... ok
test tests::the_ceiling_reads_declared_facts_rather_than_an_operation_list ... ok
test tests::the_limit_drops_operations_rather_than_the_identities_that_serve_one ... ok
test tests::a_read_only_first_connection_does_not_hide_an_admitted_write ... ok
test tests::search_and_describe_require_approval_for_every_admitted_write ... ok
test tests::a_request_template_exists_for_every_operation_this_backend_would_offer ... ok
test hosted::tests::completion_does_not_overwrite_a_terminal_session_after_verification_awaits ... ok
test hosted::tests::two_people_receive_isolated_connections_and_credential_addresses ... ok

test result: ok. 39 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.05s

     Running tests/incremental_dispatch.rs (crates/connectors-runtime/target/debug/deps/incremental_dispatch-53d7669bc238f644)

running 4 tests
test confluence_expired_permission_is_not_granted ... FAILED
test confluence_rate_limit_is_retriable ... FAILED
test jira_incremental_personal_dispatch_translates_declared_input ... FAILED
test gitlab_incremental_personal_dispatch_preserves_pagination_and_projection ... FAILED

failures:

---- confluence_expired_permission_is_not_granted stdout ----

thread 'confluence_expired_permission_is_not_granted' (3810174) panicked at /home/operator/.local/state/worktree/trees/b10x/connectors/wt-365aa8e11239/crates/integration-catalog/tests/incremental_dispatch.rs:65:5:
assertion `left == right` failed
  left: Unavailable
 right: NotGranted
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace

---- confluence_rate_limit_is_retriable stdout ----

thread 'confluence_rate_limit_is_retriable' (3810175) panicked at /home/operator/.local/state/worktree/trees/b10x/connectors/wt-365aa8e11239/crates/integration-catalog/tests/incremental_dispatch.rs:59:5:
rate limiting must permit retry: OperationError { code: Unavailable, message: "the provider refused the request with HTTP 429 — the provider is rate-limiting this credential", retriable: false }

---- jira_incremental_personal_dispatch_translates_declared_input stdout ----

thread 'jira_incremental_personal_dispatch_translates_declared_input' (3810177) panicked at /home/operator/.local/state/worktree/trees/b10x/connectors/wt-365aa8e11239/crates/integration-catalog/tests/incremental_dispatch.rs:41:5:
assertion `left == right` failed
  left: None
 right: Some("project = \"PROJ\" AND updated >= 1700000000000 ORDER BY updated ASC, key ASC")

---- gitlab_incremental_personal_dispatch_preserves_pagination_and_projection stdout ----

thread 'gitlab_incremental_personal_dispatch_preserves_pagination_and_projection' (3810176) panicked at /home/operator/.local/state/worktree/trees/b10x/connectors/wt-365aa8e11239/crates/integration-catalog/tests/incremental_dispatch.rs:49:5:
the personal read returned an unprojected person object: [{"id":1,"project_id":7,"updated_at":"2026-09-05T10:00:00Z","user":{"email":"private@example.test"}}]


failures:
    confluence_expired_permission_is_not_granted
    confluence_rate_limit_is_retriable
    gitlab_incremental_personal_dispatch_preserves_pagination_and_projection
    jira_incremental_personal_dispatch_translates_declared_input

test result: FAILED. 0 passed; 4 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.05s

error: test failed, to rerun pass `-p integration-catalog --test incremental_dispatch`
```

## 4. Findings covering a402cc7e plus the new cases

| File:line | Verdict / origin / severity | What was measured | What reaches it |
|---|---|---|---|
| providers/jira.toml:799 | NEEDS-CHANGE / introduced / blocker | New Jira search composes no jql, with public project_key/updated_since_ms sent unchanged; assertion at tests/incremental_dispatch.rs:41 exits 101. | PersonalRuntime composition.rs:397–440 composes CatalogBackend for configured catalog Jira; CatalogBackend::invoke calls the generic resolver at lib.rs:537, without native prepare_query. |
| providers/gitlab.toml:1159 | NEEDS-CHANGE / introduced / blocker | New GitLab personal read returns raw user.email; assertion at tests/incremental_dispatch.rs:49 exits 101. Source confirms continuation headers are unrequested and native envelope is absent. | Same PersonalRuntime catalog composition; lib.rs:609 applies projection only to Confluence and returns raw JSON for GitLab. |
| crates/integration-catalog/src/lib.rs:948 | NEEDS-CHANGE / introduced / warning | The new Confluence endpoint returns retriable:false for 429; assertion at tests/incremental_dispatch.rs:59 exits 101. | confluence-page-search is served by generic invoke, whose non-success branch calls refusal with false. This new endpoint promises retryable rate-limit behavior in the supplied contract. |
| crates/integration-catalog/src/lib.rs:596 | NEEDS-CHANGE / introduced / warning | The new Confluence endpoint maps HTTP 403 to Unavailable, not NotGranted; assertion at tests/incremental_dispatch.rs:65 exits 101. | Same new Confluence endpoint and non-success branch; the provided contract promises expired/missing permission as NotGranted. |

Origin is introduced because these operation IDs do not exist at the base, and the unit exposes new calls through the pre-existing generic path. The old generic error behavior is not alleged to have originated here; the new operation contract mismatch is. `git show`/diff confirms operation additions; no checkout or base mutation was made.

The real personal caller is established by the repository's composition code and corroborated by coordinator inspection of the intended deployment. Resolving these findings by documenting a native-only restriction would leave the requested personal workflow unusable. Shared pure admission/projection logic or correct personal composition can resolve it without new grants. This attack did not modify either implementation.

## 5. Other inspected boundaries

Confluence's added closed-input and scalar-only projection tests passed in the 39-case existing suite, including unknown nested objects and prohibited expansion.
Native Jira's fixed project admission and GitLab's foreign-project refusal were read; their local helpers do not repair the personal dispatch route above.
The suspected absence of updated_at in GitLab pipeline list was rejected after checking the current official vendor example: https://docs.gitlab.com/api/pipelines/#list-project-pipelines includes it. No finding is claimed for that hypothesis.
No full multi-workspace gate or live grants test was run; this was one bounded catalog dispatch attack. Generated artifact fixed-point results remain the implementor's supplied results.

## 6. Every path authored outside the worktree

- /home/operator/.cache/org-brain/fresh-multisource/unit-reads/adversary-jira-red.log
- /home/operator/.cache/org-brain/fresh-multisource/unit-reads/adversary-gitlab-red.log
- /home/operator/.cache/org-brain/fresh-multisource/unit-reads/adversary-confluence-rate-red.log
- /home/operator/.cache/org-brain/fresh-multisource/unit-reads/adversary-confluence-permission-red.log
- /home/operator/.cache/org-brain/fresh-multisource/unit-reads/adversary-suite.log
- /home/operator/.cache/org-brain/fresh-multisource/unit-reads/adversary-1.md

Compiler output remained in the worktree's existing crates/connectors-runtime/target. No external build directory was selected. Cargo's transient TMPDIR was the assigned scratch directory. No scratch script or vendor document file was created. Minimum observed free disk was 44 GiB before the scoped build.

```findings
- file: providers/jira.toml
  line: 799
  category: acceptance
  severity: blocker
  verdict: NEEDS-CHANGE
  origin: introduced
  message: New Jira search composes no jql in the personal catalog backend and sends its public project_key and updated_since_ms untranslated.
- file: providers/gitlab.toml
  line: 1159
  category: acceptance
  severity: blocker
  verdict: NEEDS-CHANGE
  origin: introduced
  message: New GitLab reads return raw identity objects and omit their continuation envelope through the personal catalog backend.
- file: crates/integration-catalog/src/lib.rs
  line: 948
  category: contract-drift
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: New Confluence page search returns retriable false for rate limiting despite its retryable continuation contract.
- file: crates/integration-catalog/src/lib.rs
  line: 596
  category: contract-drift
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: New Confluence page search reports permission denial as Unavailable rather than the promised NotGranted.
```
