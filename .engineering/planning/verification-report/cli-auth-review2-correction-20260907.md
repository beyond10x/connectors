---
format: aep.planning-md/1
id: verification-report:cli-auth-review2-correction-20260907
kind: verification-report
status: draft
title: Final authentication review correction verification
relations:
- verifies: story:auth-as-tool-result
revision: 1
---
unit: auth-as-tool-result — final narrow served-document correction
verdict: green
cases: executed 118→118, red 0
origin: n/a
wrote-outside-worktree: assigned review2-correction scratch, ac2 TMPDIR, existing private target and normal Cargo bookkeeping paths enumerated below
needs-coordinator: yes — verify and integrate the one-file correction with all retained tests; root owns publication and final gates

1. Acceptance and measured boundary

Make the served Operation 409 response schema admit the already-supported v1, v2 and v3 envelopes, including non-authentication stale-description conflicts, while preserving every response status/body, authentication example and 503 alternative. This is the routed correction from the second/final ordinary auth review, not another adversary pass or a whole-story verdict. No test was added or edited, so the preserved full server count is expected to remain 118. The header red 0 refers to final remaining affected-server failures; the review’s red and this correction’s exact reproduction are retained below.

The complete immutable review is recorded as review-result:cli-auth-adversary-2-20260907; its report bytes were compared with the public record before application. review-inputs.json records the exact source/report/record hashes. The sole review finding is INFEASIBLE for the current built-in hosted production posture and has undecided origin. This implementor does not change that disposition: the actual router executes with a synthetic metadata/readiness backend and real grant store, while current built-in hosted acquisition remains Unsupported. An embedding can supply a custom backend; this is a served-document contract correction, not a demonstrated live hosted OAuth regression, authority bypass or provider operation.

The bounded class is schema coverage for every supported Operation response identity at HTTP 409. The retained twelve-row matrix tests v1/v2/v3 across missing credentials, degraded credentials, dependency outage and a stale description. Before correction only selected v1/v2 stale conflicts failed schema validation; their strict decoded bodies, stale_authority code, non-retriable state and 409 status were already correct. After correction all twelve rows pass. Existing exact assertions retain selected protocol identity, authentication presence/absence, error code, readiness ordering, and zero dispatch/session/approval activity. The schema now names the three existing response roots through oneOf; no frozen schema or runtime behavior changes. The description explicitly includes stale operation descriptions alongside v3 authentication remediation.

The actual owner and mechanism were checked against hosted/docs/openapi.json and the retained matrix before applying the smallest assigned edit. Semantic JSON comparison proves that only /paths/~1operations/post/responses/409/description and its application/json schema changed. All examples, 503 alternatives and every other JSON value remain exact. All 1,225 tracked source hashes are frozen; only the assigned document differs from the inherited review snapshot. All five dirty reviewer test files are byte-identical, and all ten original test prefixes and complete current test files remain exact.

2. Actual diff

```text
 crates/server/src/hosted/docs/openapi.json | 14 ++++++++++++--
 1 file changed, 12 insertions(+), 2 deletions(-)
```

Exact hunks:

```diff
diff --git a/crates/server/src/hosted/docs/openapi.json b/crates/server/src/hosted/docs/openapi.json
index 59183301..2906383a 100644
--- a/crates/server/src/hosted/docs/openapi.json
+++ b/crates/server/src/hosted/docs/openapi.json
@@ -723,11 +723,21 @@
             }
           },
           "409": {
-            "description": "The admitted operation was not attempted and needs explicit trusted authentication remediation. This does not renew Identity, spend approval, or resend the operation.",
+            "description": "The operation description is stale, or the admitted v0alpha3 operation was not attempted and needs explicit trusted authentication remediation. This does not renew Identity, spend approval, or resend the operation.",
             "content": {
               "application/json": {
                 "schema": {
-                  "$ref": "#/components/schemas/operation.v3.responseEnvelope"
+                  "oneOf": [
+                    {
+                      "$ref": "#/components/schemas/operation.v1.responseEnvelope"
+                    },
+                    {
+                      "$ref": "#/components/schemas/operation.responseEnvelope"
+                    },
+                    {
+                      "$ref": "#/components/schemas/operation.v3.responseEnvelope"
+                    }
+                  ]
                 },
                 "examples": {
                   "authentication_required": {
```

Owned source hash:

```json
{
  "crates/server/src/hosted/docs/openapi.json": "c6621edea2d57e3f020c4c3531be5f3eae0ad74349b7f0ec94d42a8a7cf0aa44"
}
```

3. Retained red evidence, verbatim

The final reviewer’s exact first matrix command and full raw output are retained unchanged as retained-auth-adversary2-hosted-first.command.json/.log/.result.json. The original report remains untouched. This correction also executed the identical Cargo argv before editing the document; only the prospectively assigned TMPDIR changes from av2 to ac2. That reproduced the same two invalid rows and ten valid controls.


Exact command/environment/resource policy:

```json
{
  "label": "matrix-before",
  "argv": [
    "cargo",
    "test",
    "--locked",
    "--offline",
    "-p",
    "server",
    "--lib",
    "hosted::tests::remediation::auth_adversary2_hosted_non_auth_conflict_and_outage_keep_selected_status_contracts",
    "--",
    "--exact",
    "--nocapture"
  ],
  "cwd": "~/.local/state/worktree/trees/b10x/connectors/wt-78f189927231",
  "env_overrides": {
    "TMPDIR": "~/.cache/cw6/ac2",
    "CARGO_TARGET_DIR": "/dev/shm/b10x-cli-wave-wt-78f189927231-urhrmq5y/runtime-target",
    "CARGO_INCREMENTAL": "0",
    "CARGO_PROFILE_DEV_DEBUG": "0",
    "CARGO_PROFILE_TEST_DEBUG": "0",
    "CARGO_BUILD_JOBS": "1",
    "PATH": "~/.cargo/bin:/home/linuxbrew/.linuxbrew/Caskroom/codex/0.153.4/codex-path:~/.local/bin:~/.deno/bin:~/.codex/tmp/arg0/codex-arg0Bq2TOf:/home/linuxbrew/.linuxbrew/Caskroom/codex/0.153.4/codex-path:~/.bun/bin:/home/linuxbrew/.linuxbrew/bin:/home/linuxbrew/.linuxbrew/sbin:~/.opencode/bin:~/.fly/bin:~/.cargo/bin:~:~/anaconda/bin:/usr/bin:~/.rbenv:/usr/local/go/bin:~/go/bin:~/go:~/.local/share/gem/ruby/3.0.0/bin:~/.deno/bin:~/.yarn/bin:~/.pulumi/bin:/opt/rocm/bin:~/.bun/bin:/home/linuxbrew/.linuxbrew/bin:/home/linuxbrew/.linuxbrew/sbin:~/.sdkman/candidates/scala/current/bin:~/.sdkman/candidates/maven/current/bin:~/.sdkman/candidates/java/current/bin:~/.sdkman/candidates/groovy/current/bin:~/.sdkman/candidates/grails/current/bin:~/.sdkman/candidates/gradle/current/bin:~/.local/bin:/usr/local/bin:/bin:/usr/local/sbin:/opt/cuda/bin:/opt/cuda/integration/nsight-compute:/opt/cuda/integration/nsight-systems:/usr/lib/jvm/default/bin:/usr/bin/site_perl:/usr/bin/vendor_perl:/usr/bin/core_perl:/usr/lib/rustup/bin:/var/lib/snapd/snap/bin:/opt/cuda/bin:/opt/cuda/integration/nsight-compute:/opt/cuda/integration/nsight-systems"
  },
  "env_unset": [
    "RUSTC_WRAPPER"
  ],
  "limits": {
    "disk_free_bytes": 12884901888,
    "tmpfs_free_bytes": 8589934592,
    "mem_available_bytes": 17179869184,
    "target_max_bytes": 12884901888,
    "tmpdir_max_bytes": 134217728
  },
  "target": "/dev/shm/b10x-cli-wave-wt-78f189927231-urhrmq5y/runtime-target",
  "tmpfs": "/dev/shm",
  "sample_interval_seconds": 1,
  "started": "2026-09-07T00:11:46.017851+00:00"
}
```

Complete original stdout/stderr:

```text
    Finished `test` profile [unoptimized] target(s) in 0.20s
     Running unittests src/lib.rs (/dev/shm/b10x-cli-wave-wt-78f189927231-urhrmq5y/runtime-target/debug/deps/server-034f8d953ecabd82)

running 1 test
actual selected status/schema matrix, no dispatch: [(V0Alpha1, "missing", 503, true), (V0Alpha1, "degraded", 503, true), (V0Alpha1, "outage", 503, true), (V0Alpha1, "stale", 409, false), (V0Alpha2, "missing", 503, true), (V0Alpha2, "degraded", 503, true), (V0Alpha2, "outage", 503, true), (V0Alpha2, "stale", 409, false), (V0Alpha3, "missing", 409, true), (V0Alpha3, "degraded", 409, true), (V0Alpha3, "outage", 503, true), (V0Alpha3, "stale", 409, true)]

thread 'hosted::tests::remediation::auth_adversary2_hosted_non_auth_conflict_and_outage_keep_selected_status_contracts' (3647026) panicked at crates/server/src/hosted/tests/remediation.rs:705:5:
every selected response must satisfy the schema served for its actual HTTP status, including non-authentication conflicts
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace
test hosted::tests::remediation::auth_adversary2_hosted_non_auth_conflict_and_outage_keep_selected_status_contracts ... FAILED

failures:

failures:
    hosted::tests::remediation::auth_adversary2_hosted_non_auth_conflict_and_outage_keep_selected_status_contracts

test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 115 filtered out; finished in 0.93s

error: test failed, to rerun pass `-p server --lib`
```

Collected exit/resource/process result:

```json
{
  "label": "matrix-before",
  "exit": 101,
  "minimum": {
    "disk_free_bytes": 17773735936,
    "tmpfs_free_bytes": 13247201280,
    "mem_available_bytes": 33406529536
  },
  "maximum_target_bytes": 11334361088,
  "maximum_tmpdir_bytes": 4096,
  "interruptions": [],
  "remaining_process_group": [],
  "ended": "2026-09-07T00:11:48.956363+00:00"
}
```

4. Corrected deciding matrix and full affected gate

| Lane | Executed before → after | Final passed / failed / ignored | Exit |
| --- | --- | --- | --- |
| Exact retained matrix selection | 1 → 1 | 1 / 0 / 0 | 0 |
| Complete server package | 118 → 118 | 118 / 0 / 0 | 0 |
| Strict all-target server Clippy | not a test lane | n/a | 0 |
| Root workspace formatting | not a test lane | n/a | 0 |

The before full-package count is the final reviewer’s actual 117 passed + 1 failed, taken from the server runner segments of its multi-package root command; it is not a new standalone baseline execution. retained-server-baseline.json/.log preserve that provenance. The corrected standalone server command reports 116 passing unit cases plus two passing integration cases and zero doc tests. Existing first-review 503/versioned projection and authentication/revocation cases execute in this full package, so no duplicate selected run was needed. The twelve matrix rows are observations inside one test and are not counted as twelve runner cases; selected and full-suite executions are not added into a unique total. No assertions were added or weakened.


matrix-after


Exact command/environment/resource policy:

```json
{
  "label": "matrix-after",
  "argv": [
    "cargo",
    "test",
    "--locked",
    "--offline",
    "-p",
    "server",
    "--lib",
    "hosted::tests::remediation::auth_adversary2_hosted_non_auth_conflict_and_outage_keep_selected_status_contracts",
    "--",
    "--exact",
    "--nocapture"
  ],
  "cwd": "~/.local/state/worktree/trees/b10x/connectors/wt-78f189927231",
  "env_overrides": {
    "TMPDIR": "~/.cache/cw6/ac2",
    "CARGO_TARGET_DIR": "/dev/shm/b10x-cli-wave-wt-78f189927231-urhrmq5y/runtime-target",
    "CARGO_INCREMENTAL": "0",
    "CARGO_PROFILE_DEV_DEBUG": "0",
    "CARGO_PROFILE_TEST_DEBUG": "0",
    "CARGO_BUILD_JOBS": "1",
    "PATH": "~/.cargo/bin:/home/linuxbrew/.linuxbrew/Caskroom/codex/0.153.4/codex-path:~/.local/bin:~/.deno/bin:~/.codex/tmp/arg0/codex-arg0Bq2TOf:/home/linuxbrew/.linuxbrew/Caskroom/codex/0.153.4/codex-path:~/.bun/bin:/home/linuxbrew/.linuxbrew/bin:/home/linuxbrew/.linuxbrew/sbin:~/.opencode/bin:~/.fly/bin:~/.cargo/bin:~:~/anaconda/bin:/usr/bin:~/.rbenv:/usr/local/go/bin:~/go/bin:~/go:~/.local/share/gem/ruby/3.0.0/bin:~/.deno/bin:~/.yarn/bin:~/.pulumi/bin:/opt/rocm/bin:~/.bun/bin:/home/linuxbrew/.linuxbrew/bin:/home/linuxbrew/.linuxbrew/sbin:~/.sdkman/candidates/scala/current/bin:~/.sdkman/candidates/maven/current/bin:~/.sdkman/candidates/java/current/bin:~/.sdkman/candidates/groovy/current/bin:~/.sdkman/candidates/grails/current/bin:~/.sdkman/candidates/gradle/current/bin:~/.local/bin:/usr/local/bin:/bin:/usr/local/sbin:/opt/cuda/bin:/opt/cuda/integration/nsight-compute:/opt/cuda/integration/nsight-systems:/usr/lib/jvm/default/bin:/usr/bin/site_perl:/usr/bin/vendor_perl:/usr/bin/core_perl:/usr/lib/rustup/bin:/var/lib/snapd/snap/bin:/opt/cuda/bin:/opt/cuda/integration/nsight-compute:/opt/cuda/integration/nsight-systems"
  },
  "env_unset": [
    "RUSTC_WRAPPER"
  ],
  "limits": {
    "disk_free_bytes": 12884901888,
    "tmpfs_free_bytes": 8589934592,
    "mem_available_bytes": 17179869184,
    "target_max_bytes": 12884901888,
    "tmpdir_max_bytes": 134217728
  },
  "target": "/dev/shm/b10x-cli-wave-wt-78f189927231-urhrmq5y/runtime-target",
  "tmpfs": "/dev/shm",
  "sample_interval_seconds": 1,
  "started": "2026-09-07T00:12:35.706647+00:00"
}
```

Complete original stdout/stderr:

```text
   Compiling server v0.6.5 (~/.local/state/worktree/trees/b10x/connectors/wt-78f189927231/crates/server)
    Finished `test` profile [unoptimized] target(s) in 5.18s
     Running unittests src/lib.rs (/dev/shm/b10x-cli-wave-wt-78f189927231-urhrmq5y/runtime-target/debug/deps/server-034f8d953ecabd82)

running 1 test
actual selected status/schema matrix, no dispatch: [(V0Alpha1, "missing", 503, true), (V0Alpha1, "degraded", 503, true), (V0Alpha1, "outage", 503, true), (V0Alpha1, "stale", 409, true), (V0Alpha2, "missing", 503, true), (V0Alpha2, "degraded", 503, true), (V0Alpha2, "outage", 503, true), (V0Alpha2, "stale", 409, true), (V0Alpha3, "missing", 409, true), (V0Alpha3, "degraded", 409, true), (V0Alpha3, "outage", 503, true), (V0Alpha3, "stale", 409, true)]
test hosted::tests::remediation::auth_adversary2_hosted_non_auth_conflict_and_outage_keep_selected_status_contracts ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 115 filtered out; finished in 0.94s

```

Collected exit/resource/process result:

```json
{
  "label": "matrix-after",
  "exit": 0,
  "minimum": {
    "disk_free_bytes": 17539977216,
    "tmpfs_free_bytes": 13231058944,
    "mem_available_bytes": 31872131072
  },
  "maximum_target_bytes": 11343380480,
  "maximum_tmpdir_bytes": 4096,
  "interruptions": [],
  "remaining_process_group": [],
  "ended": "2026-09-07T00:12:42.926779+00:00"
}
```


server-full


Exact command/environment/resource policy:

```json
{
  "label": "server-full",
  "argv": [
    "cargo",
    "test",
    "--locked",
    "--offline",
    "-p",
    "server",
    "--no-fail-fast"
  ],
  "cwd": "~/.local/state/worktree/trees/b10x/connectors/wt-78f189927231",
  "env_overrides": {
    "TMPDIR": "~/.cache/cw6/ac2",
    "CARGO_TARGET_DIR": "/dev/shm/b10x-cli-wave-wt-78f189927231-urhrmq5y/runtime-target",
    "CARGO_INCREMENTAL": "0",
    "CARGO_PROFILE_DEV_DEBUG": "0",
    "CARGO_PROFILE_TEST_DEBUG": "0",
    "CARGO_BUILD_JOBS": "1",
    "PATH": "~/.cargo/bin:/home/linuxbrew/.linuxbrew/Caskroom/codex/0.153.4/codex-path:~/.local/bin:~/.deno/bin:~/.codex/tmp/arg0/codex-arg0Bq2TOf:/home/linuxbrew/.linuxbrew/Caskroom/codex/0.153.4/codex-path:~/.bun/bin:/home/linuxbrew/.linuxbrew/bin:/home/linuxbrew/.linuxbrew/sbin:~/.opencode/bin:~/.fly/bin:~/.cargo/bin:~:~/anaconda/bin:/usr/bin:~/.rbenv:/usr/local/go/bin:~/go/bin:~/go:~/.local/share/gem/ruby/3.0.0/bin:~/.deno/bin:~/.yarn/bin:~/.pulumi/bin:/opt/rocm/bin:~/.bun/bin:/home/linuxbrew/.linuxbrew/bin:/home/linuxbrew/.linuxbrew/sbin:~/.sdkman/candidates/scala/current/bin:~/.sdkman/candidates/maven/current/bin:~/.sdkman/candidates/java/current/bin:~/.sdkman/candidates/groovy/current/bin:~/.sdkman/candidates/grails/current/bin:~/.sdkman/candidates/gradle/current/bin:~/.local/bin:/usr/local/bin:/bin:/usr/local/sbin:/opt/cuda/bin:/opt/cuda/integration/nsight-compute:/opt/cuda/integration/nsight-systems:/usr/lib/jvm/default/bin:/usr/bin/site_perl:/usr/bin/vendor_perl:/usr/bin/core_perl:/usr/lib/rustup/bin:/var/lib/snapd/snap/bin:/opt/cuda/bin:/opt/cuda/integration/nsight-compute:/opt/cuda/integration/nsight-systems"
  },
  "env_unset": [
    "RUSTC_WRAPPER"
  ],
  "limits": {
    "disk_free_bytes": 12884901888,
    "tmpfs_free_bytes": 8589934592,
    "mem_available_bytes": 17179869184,
    "target_max_bytes": 12884901888,
    "tmpdir_max_bytes": 134217728
  },
  "target": "/dev/shm/b10x-cli-wave-wt-78f189927231-urhrmq5y/runtime-target",
  "tmpfs": "/dev/shm",
  "sample_interval_seconds": 1,
  "started": "2026-09-07T00:12:55.527845+00:00"
}
```

Complete original stdout/stderr:

```text
   Compiling server v0.6.5 (~/.local/state/worktree/trees/b10x/connectors/wt-78f189927231/crates/server)
    Finished `test` profile [unoptimized] target(s) in 4.13s
     Running unittests src/lib.rs (/dev/shm/b10x-cli-wave-wt-78f189927231-urhrmq5y/runtime-target/debug/deps/server-034f8d953ecabd82)

running 116 tests
test egress::tests::ambiguous_retry_after_is_not_flattened_into_advice ... ok
test egress::tests::exact_origin_cannot_be_widened_by_path_host_or_userinfo ... ok
test egress::tests::egress_requires_a_nonempty_ascii_connection_or_session_reference ... ok
test egress::tests::ipv4_mapped_ipv6_cannot_bypass_address_classification ... ok
test egress::tests::operator_network_may_admit_private_but_not_process_local_addresses ... ok
test egress::tests::malformed_retry_after_does_not_hide_the_definite_provider_response ... ok
test egress::tests::cached_client_cannot_bypass_current_address_policy ... ok
test egress::tests::public_dns_refuses_private_local_reserved_and_mixed_answers ... ok
test egress::tests::retry_after_extraction_keeps_only_one_valid_decimal_and_admitted_headers ... ok
test egress::tests::suffix_rule_requires_a_real_child_and_the_exact_scheme_and_port ... ok
test hosted::enforcement::tests::the_canonical_digest_ignores_member_order_and_nothing_else ... ok
test hosted::enforcement::tests::an_issued_record_round_trips_without_its_reference_in_any_key ... ok
test hosted::git_fetch::tests::rejected_control_identity_is_decided_before_the_request_body_is_polled ... ok
test hosted::principal::tests::lease_seeds_survive_the_verifier_token_rotation_composition ... ok
test egress::tests::reused_connection_keeps_authorization_and_timeout_request_specific ... ok
test hosted::git_fetch::tests::rejected_source_authority_is_decided_before_body_or_broker_exchange ... ok
test hosted::mcp::toolset::authentication_projection_closes_valid_private_reference_and_message_fields ... ok
test hosted::git_fetch::tests::control_and_internal_routes_are_separate_and_non_cacheable ... ok
test hosted::git_fetch::tests::ambiguous_protocol_or_source_headers_are_refused_before_reading_the_body ... ok
test egress::tests::rate_stage2_definite_http_429_survives_oversized_or_broken_error_bodies ... ok
test hosted::tests::admin_routes::operator_group_without_the_exact_scope_cannot_write ... ok
test hosted::tests::admin_routes::auth_metadata_is_public_and_selects_exact_authority ... ok
test hosted::tests::admin_routes::operator_can_write_and_status_never_returns_the_secret ... ok
test hosted::tests::contract_validation::hosted_route_refuses_a_malformed_backend_contract ... ok
test egress::tests::pool_is_bounded_and_separates_current_addresses_authorities_origins_and_policy ... ok
test hosted::tests::a_human_issues_one_exact_input_approval_which_is_spent_once ... ok
test egress::rate_adversary_tests::rate_final_chunked_429_keeps_definite_status_without_body_or_untrusted_advice ... ok
test hosted::tests::contract_validation::rate_stage2_hosted_boundary_selects_response_version_even_for_early_refusals ... ok
test hosted::tests::contract_validation::rate_stage2_hosted_http_projects_refusals_once_and_rejects_invalid_frames_before_backend ... ok
test hosted::tests::contract_validation::rate_stage2_invalid_http_correlation_is_a_bounded_versioned_client_refusal ... ok
test egress::rate_adversary_tests::rate_adversary_header_cardinality_and_unfinished_body_are_separate ... ok
test hosted::tests::contract_validation::rate_final_hosted_describe_versions_reject_bad_advice_before_loss ... ok
test hosted::tests::contract_validation::rate_adversary_hosted_grant_and_approval_refusals_keep_requested_version ... ok
test hosted::tests::docs::openapi_json_is_served_verbatim_with_a_content_hash_etag ... ok
test hosted::tests::enforcement::a_granted_effect_without_approval_demand_dispatches_on_the_grant_alone ... ok
test hosted::tests::enforcement::a_granted_mutation_demanding_approval_refuses_without_one ... ok
test hosted::tests::docs::a_request_example_with_an_unknown_field_is_refused ... ok
test hosted::tests::enforcement::a_granted_mutation_with_a_demanded_approval_dispatches_with_one ... ok
test hosted::tests::enforcement::a_mutation_with_no_admitting_grant_refuses ... ok
test hosted::tests::docs::every_documented_refusal_example_names_a_real_error_code ... ok
test hosted::tests::docs::the_docs_page_is_served_unauthenticated_as_html ... ok
test hosted::tests::enforcement::an_unbound_grant_store_is_an_outage_for_effects_only ... ok
test hosted::tests::enforcement::a_second_presentation_of_the_same_approval_refuses_and_journals_replay ... ok
test hosted::tests::docs::auth_openapi_selects_each_supported_identity_explicitly ... ok
test hosted::tests::docs::auth_openapi_remediation_examples_keep_the_operation_unattempted ... ok
test hosted::tests::enforcement::the_read_only_path_is_unchanged_for_callers_without_grants ... ok
test hosted::tests::docs::the_document_pins_the_exact_wire_contract_identities_and_audience ... ok
test hosted::tests::docs::the_docs_page_links_the_contract_and_renders_its_version ... ok
test hosted::tests::hosted_completion_failures_are_non_cacheable_and_browser_hardened ... ok
test hosted::tests::hosted_datasource_route_passes_verified_groups_and_exact_tenant ... ok
test hosted::tests::hosted_completion_streams_fragments_into_a_redacted_bounded_submission ... ok
test hosted::tests::docs::every_documented_mcp_request_example_is_answered_by_the_live_transport ... ok
test hosted::tests::docs::the_docs_page_makes_zero_external_requests ... ok
test hosted::tests::docs::the_docs_page_refusal_table_carries_every_documented_code ... ok
test hosted::tests::hosted_connection_route_uses_the_same_identity_boundary ... ok
test hosted::tests::hosted_route_requires_identity_and_exact_tenant_binding ... ok
test hosted::tests::docs::every_docs_page_example_is_the_documents_example_after_json_normalization ... ok
test hosted::tests::mcp::an_invoke_without_the_invoke_scope_surfaces_not_granted ... ok
test hosted::tests::mcp::an_op_backed_invoke_describes_then_invokes_with_the_fresh_lease ... ok
test hosted::tests::enforcement::every_enforcement_refusal_renders_the_same_bytes ... ok
test hosted::tests::mcp::approval_demand_and_evidence_pass_through_the_admission_seam ... ok
test hosted::tests::mcp::the_mcp_route_is_stateless_post_only ... ok
test hosted::tests::mcp::initialize_echoes_admitted_revisions_and_answers_ping ... ok
test hosted::tests::mcp::a_stale_authority_refusal_is_retried_exactly_once_with_a_fresh_lease ... ok
test hosted::tests::docs::every_documented_request_example_is_accepted_by_its_protocol_type ... ok
test hosted::tests::mcp::tools_list_returns_exactly_the_three_meta_tools ... ok
test hosted::tests::mcp::datasource_backed_tools_route_through_the_datasource_seam ... ok
test hosted::tests::mcp::rate_stage2_mcp_preserves_refusal_details_without_entering_stale_retry ... ok
test hosted::tests::mcp_monitoring::a_stale_monitoring_invoke_re_resolves_the_same_target_exactly_once ... ok
test hosted::tests::mcp_monitoring::a_monitoring_invoke_refuses_dishonest_targets_before_any_dispatch ... ok
test hosted::tests::mcp::tool_search_projects_only_the_entries_the_callers_seam_results_support ... ok
test hosted::tests::mcp_monitoring::a_monitoring_invoke_routes_the_chosen_target_through_the_decided_seam ... ok
test hosted::tests::monitoring_transport_gate_admits_only_configured_groups_or_operator ... ok
test hosted::tests::pod_log_transport_gate_admits_only_kubernetes_read_groups_or_operator ... ok
test hosted::tests::mcp::tool_describe_projects_the_underlying_description_without_a_lease ... ok
test hosted::tests::mcp::transport_refusals_carry_the_designed_statuses_and_codes ... ok
test hosted::tests::production_router_publishes_client_discovery_without_authentication ... ok
test hosted::tests::mcp_monitoring::the_acceptance_sequence_invokes_with_a_target_and_integer_epochs ... ok
test hosted::tests::remediation::auth_connection_v2_ordinary_requests_select_exact_identity_and_refuse_duplicates ... ok
test hosted::tests::mcp::rate_adversary_mcp_stale_then_rate_stops_without_losing_large_delay ... ok
test hosted::tests::mcp_monitoring::tool_search_lists_the_monitoring_tools_for_a_monitoring_read_principal ... ok
test hosted::tests::mcp::a_busy_namespace_is_listed_whole_without_a_paging_surface ... ok
test hosted::tests::self_event_scope_is_closed_to_slack_specific_requests ... ok
test hosted::tests::mcp_monitoring::tool_describe_enumerates_the_callers_configured_targets_without_a_lease ... ok
test hosted::tests::signal::a_granted_session_signal_dispatches_behind_the_sessions_grant ... ok
test hosted::tests::signal::an_effect_bearing_session_signal_without_an_admitting_grant_refuses ... ok
test hosted::tests::signal::an_unbound_grant_store_is_an_outage_for_session_signals ... ok
test hosted::tests::signal::a_signal_refusal_matches_the_invoke_refusal_bytes ... ok
test hosted::tests::tenant_members_receive_only_read_only_module_invocation ... ok
test local::tests::a_broad_state_directory_refuses_without_repair ... ok
test hosted::tests::subscription_oauth_start_is_identity_scoped_bounded_and_non_cacheable ... ok
test hosted::tests::subscription_credential_stays_in_custody_and_only_an_exact_lease_redeems ... ok
test local::tests::a_second_daemon_cannot_unlink_the_live_daemons_socket ... ok
test local::tests::auth_one_shot_v3_refuses_before_backend_work_and_joins_shutdown ... ok
test local::tests::auth_one_shot_v3_serves_a_real_result_and_joins_shutdown ... ok
test local::tests::one_socket_dispatches_the_value_free_connection_and_event_contracts ... ok
test local::tests::owner_socket_serves_one_strict_bounded_operation_frame ... ok
test hosted::tests::mcp_monitoring::monitoring_tool_schemas_state_the_documents_contract ... ok
test local::tests::rate_stage2_actual_socket_serves_both_versions_without_resending ... ok
test hosted::tests::mcp::a_pathological_namespace_is_cut_with_an_explicit_truncation_marker ... ok
test hosted::tests::docs::auth_openapi_503_schemas_admit_all_supported_unavailable_versions ... ok
test hosted::tests::docs::auth_openapi_schema_projection_preserves_protocol_vector_results ... ok
test hosted::tests::remediation::auth_v3_does_not_publish_a_structurally_valid_private_backend_reference ... ok
test hosted::tests::remediation::auth_v3_requires_a_real_grant_and_keeps_unknown_targets_opaque ... ok
test hosted::tests::remediation::auth_connection_v2_hosted_start_has_real_grants_and_no_production_acquisition ... ok
test catalog_projection::tests::a_deployment_publishes_only_the_setup_flows_it_can_complete ... ok
test catalog_projection::tests::search_is_whole_catalog_and_describe_is_descriptive_only ... ok
test local::tests::auth_one_shot_v3_need_precedes_dispatch_and_joins_shutdown ... ok
test catalog_projection::tests::platform_provider_satisfies_the_catalog_wire_contract ... ok
test hosted::tests::remediation::auth_v3_need_precedes_real_approval_redemption_and_dispatch ... ok
test catalog_projection::tests::every_shipped_provider_description_satisfies_the_catalog_wire_contract ... ok
test hosted::tests::hosted_liveness_stays_local_when_a_backend_dependency_is_unready ... ok
test hosted::tests::hosted_liveness_and_identity_backed_readiness_are_distinct ... ok
test hosted::tests::docs::every_documented_route_exists_in_the_real_router ... ok
test hosted::tests::remediation::auth_adversary_hosted_selected_refusals_match_actual_status_schema_and_revocation ... ok
test hosted::tests::remediation::auth_adversary2_hosted_non_auth_conflict_and_outage_keep_selected_status_contracts ... ok

test result: ok. 116 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.99s

     Running tests/rate_adversary_local.rs (/dev/shm/b10x-cli-wave-wt-78f189927231-urhrmq5y/runtime-target/debug/deps/rate_adversary_local-719e44f4d536c406)

running 2 tests
test rate_final_local_describe_validates_before_version_loss ... ok
test rate_adversary_local_versions_validate_before_single_dispatch ... ok

test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

   Doc-tests server

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

```

Collected exit/resource/process result:

```json
{
  "label": "server-full",
  "exit": 0,
  "minimum": {
    "disk_free_bytes": 17529847808,
    "tmpfs_free_bytes": 13247070208,
    "mem_available_bytes": 31588679680
  },
  "maximum_target_bytes": 11334361088,
  "maximum_tmpdir_bytes": 8192,
  "interruptions": [],
  "remaining_process_group": [],
  "ended": "2026-09-07T00:13:01.686748+00:00"
}
```


server-clippy


Exact command/environment/resource policy:

```json
{
  "label": "server-clippy",
  "argv": [
    "cargo",
    "clippy",
    "--locked",
    "--offline",
    "-p",
    "server",
    "--all-targets",
    "--",
    "-D",
    "warnings"
  ],
  "cwd": "~/.local/state/worktree/trees/b10x/connectors/wt-78f189927231",
  "env_overrides": {
    "TMPDIR": "~/.cache/cw6/ac2",
    "CARGO_TARGET_DIR": "/dev/shm/b10x-cli-wave-wt-78f189927231-urhrmq5y/runtime-target",
    "CARGO_INCREMENTAL": "0",
    "CARGO_PROFILE_DEV_DEBUG": "0",
    "CARGO_PROFILE_TEST_DEBUG": "0",
    "CARGO_BUILD_JOBS": "1",
    "PATH": "~/.cargo/bin:/home/linuxbrew/.linuxbrew/Caskroom/codex/0.153.4/codex-path:~/.local/bin:~/.deno/bin:~/.codex/tmp/arg0/codex-arg0Bq2TOf:/home/linuxbrew/.linuxbrew/Caskroom/codex/0.153.4/codex-path:~/.bun/bin:/home/linuxbrew/.linuxbrew/bin:/home/linuxbrew/.linuxbrew/sbin:~/.opencode/bin:~/.fly/bin:~/.cargo/bin:~:~/anaconda/bin:/usr/bin:~/.rbenv:/usr/local/go/bin:~/go/bin:~/go:~/.local/share/gem/ruby/3.0.0/bin:~/.deno/bin:~/.yarn/bin:~/.pulumi/bin:/opt/rocm/bin:~/.bun/bin:/home/linuxbrew/.linuxbrew/bin:/home/linuxbrew/.linuxbrew/sbin:~/.sdkman/candidates/scala/current/bin:~/.sdkman/candidates/maven/current/bin:~/.sdkman/candidates/java/current/bin:~/.sdkman/candidates/groovy/current/bin:~/.sdkman/candidates/grails/current/bin:~/.sdkman/candidates/gradle/current/bin:~/.local/bin:/usr/local/bin:/bin:/usr/local/sbin:/opt/cuda/bin:/opt/cuda/integration/nsight-compute:/opt/cuda/integration/nsight-systems:/usr/lib/jvm/default/bin:/usr/bin/site_perl:/usr/bin/vendor_perl:/usr/bin/core_perl:/usr/lib/rustup/bin:/var/lib/snapd/snap/bin:/opt/cuda/bin:/opt/cuda/integration/nsight-compute:/opt/cuda/integration/nsight-systems"
  },
  "env_unset": [
    "RUSTC_WRAPPER"
  ],
  "limits": {
    "disk_free_bytes": 12884901888,
    "tmpfs_free_bytes": 8589934592,
    "mem_available_bytes": 17179869184,
    "target_max_bytes": 12884901888,
    "tmpdir_max_bytes": 134217728
  },
  "target": "/dev/shm/b10x-cli-wave-wt-78f189927231-urhrmq5y/runtime-target",
  "tmpfs": "/dev/shm",
  "sample_interval_seconds": 1,
  "started": "2026-09-07T00:13:22.998494+00:00"
}
```

Complete original stdout/stderr:

```text
    Checking server v0.6.5 (~/.local/state/worktree/trees/b10x/connectors/wt-78f189927231/crates/server)
    Finished `dev` profile [unoptimized] target(s) in 3.25s
```

Collected exit/resource/process result:

```json
{
  "label": "server-clippy",
  "exit": 0,
  "minimum": {
    "disk_free_bytes": 17425027072,
    "tmpfs_free_bytes": 13252792320,
    "mem_available_bytes": 33157423104
  },
  "maximum_target_bytes": 11334361088,
  "maximum_tmpdir_bytes": 4096,
  "interruptions": [],
  "remaining_process_group": [],
  "ended": "2026-09-07T00:13:28.114949+00:00"
}
```


root-fmt


Exact command/environment/resource policy:

```json
{
  "label": "root-fmt",
  "argv": [
    "cargo",
    "fmt",
    "--all",
    "--check"
  ],
  "cwd": "~/.local/state/worktree/trees/b10x/connectors/wt-78f189927231",
  "env_overrides": {
    "TMPDIR": "~/.cache/cw6/ac2",
    "CARGO_TARGET_DIR": "/dev/shm/b10x-cli-wave-wt-78f189927231-urhrmq5y/runtime-target",
    "CARGO_INCREMENTAL": "0",
    "CARGO_PROFILE_DEV_DEBUG": "0",
    "CARGO_PROFILE_TEST_DEBUG": "0",
    "CARGO_BUILD_JOBS": "1",
    "PATH": "~/.cargo/bin:/home/linuxbrew/.linuxbrew/Caskroom/codex/0.153.4/codex-path:~/.local/bin:~/.deno/bin:~/.codex/tmp/arg0/codex-arg0Bq2TOf:/home/linuxbrew/.linuxbrew/Caskroom/codex/0.153.4/codex-path:~/.bun/bin:/home/linuxbrew/.linuxbrew/bin:/home/linuxbrew/.linuxbrew/sbin:~/.opencode/bin:~/.fly/bin:~/.cargo/bin:~:~/anaconda/bin:/usr/bin:~/.rbenv:/usr/local/go/bin:~/go/bin:~/go:~/.local/share/gem/ruby/3.0.0/bin:~/.deno/bin:~/.yarn/bin:~/.pulumi/bin:/opt/rocm/bin:~/.bun/bin:/home/linuxbrew/.linuxbrew/bin:/home/linuxbrew/.linuxbrew/sbin:~/.sdkman/candidates/scala/current/bin:~/.sdkman/candidates/maven/current/bin:~/.sdkman/candidates/java/current/bin:~/.sdkman/candidates/groovy/current/bin:~/.sdkman/candidates/grails/current/bin:~/.sdkman/candidates/gradle/current/bin:~/.local/bin:/usr/local/bin:/bin:/usr/local/sbin:/opt/cuda/bin:/opt/cuda/integration/nsight-compute:/opt/cuda/integration/nsight-systems:/usr/lib/jvm/default/bin:/usr/bin/site_perl:/usr/bin/vendor_perl:/usr/bin/core_perl:/usr/lib/rustup/bin:/var/lib/snapd/snap/bin:/opt/cuda/bin:/opt/cuda/integration/nsight-compute:/opt/cuda/integration/nsight-systems"
  },
  "env_unset": [
    "RUSTC_WRAPPER"
  ],
  "limits": {
    "disk_free_bytes": 12884901888,
    "tmpfs_free_bytes": 8589934592,
    "mem_available_bytes": 17179869184,
    "target_max_bytes": 12884901888,
    "tmpdir_max_bytes": 134217728
  },
  "target": "/dev/shm/b10x-cli-wave-wt-78f189927231-urhrmq5y/runtime-target",
  "tmpfs": "/dev/shm",
  "sample_interval_seconds": 1,
  "started": "2026-09-07T00:13:38.490356+00:00"
}
```

Complete original stdout/stderr:

```text
```

Collected exit/resource/process result:

```json
{
  "label": "root-fmt",
  "exit": 0,
  "minimum": {
    "disk_free_bytes": 17417945088,
    "tmpfs_free_bytes": 13242306560,
    "mem_available_bytes": 33322233856
  },
  "maximum_target_bytes": 11334361088,
  "maximum_tmpdir_bytes": 4096,
  "interruptions": [],
  "remaining_process_group": [],
  "ended": "2026-09-07T00:13:40.309765+00:00"
}
```

5. Preservation, resources and limits

No production Rust, tests, manifests, dependencies, generated contracts/catalogs, planning, Git index/ref, installed binary, provider, daemon or operator configuration changed. No cleanup or extra target was created. Unchanged runtime, console, CLI, protocol and service suites were not rerun; root owns final publication verification. The source and existing target remain paused for root handoff.

Every command has a before/after source proof covering all 1,225 tracked files and proving zero concurrent source changes. All five process groups were checked empty after completion, and the compiler was released before target/report hashing. The copied final-review sampler differs only in its new owner-only ac2 TMPDIR, tolerates vanished fixture entries, and retains its own-process-group termination on other active sampler errors. No sampler error or resource interruption occurred in this correction. Limits stayed at 12 GiB disk, 8 GiB tmpfs, 16 GiB MemAvailable, 12 GiB target and 128 MiB TMPDIR; jobs=1, incremental/dev-debug/test-debug=0 and RUSTC_WRAPPER unset. The root formatting command was cargo fmt --all --check.

```json
{
  "minimum": {
    "disk_free_bytes": 17417945088,
    "tmpfs_free_bytes": 13231058944,
    "mem_available_bytes": 31588679680
  },
  "maximum_target_bytes": 11343380480,
  "maximum_tmpdir_bytes": 8192,
  "interruptions": [],
  "commands": 5
}
```

The final target inventory contains 20405 entries and 415 executable entries; 0 temp entries remain. All source hashes were verified again after target hashing. The report’s public copy is solely the established original absolute home-directory prefix replacement with ~; the raw report, full logs and earlier evidence remain unmodified.

6. Every retained outside-worktree path

All correction scratch files, including the report pair and seal:

```text
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/stage2-runtime-implementation/review2-correction/brief.md
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/stage2-runtime-implementation/review2-correction/commands.json
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/stage2-runtime-implementation/review2-correction/compile-slot-release.json
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/stage2-runtime-implementation/review2-correction/counts.json
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/stage2-runtime-implementation/review2-correction/diff-hunks.txt
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/stage2-runtime-implementation/review2-correction/diff-stat.txt
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/stage2-runtime-implementation/review2-correction/evidence.sha256
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/stage2-runtime-implementation/review2-correction/final-executables.json
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/stage2-runtime-implementation/review2-correction/final-seal-verification.json
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/stage2-runtime-implementation/review2-correction/final-source.json
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/stage2-runtime-implementation/review2-correction/final-target.json
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/stage2-runtime-implementation/review2-correction/final-tmp.json
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/stage2-runtime-implementation/review2-correction/inherited-tests-before.json
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/stage2-runtime-implementation/review2-correction/inherited-tests.patch
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/stage2-runtime-implementation/review2-correction/json-closure.json
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/stage2-runtime-implementation/review2-correction/matrix-after.command.json
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/stage2-runtime-implementation/review2-correction/matrix-after.exit
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/stage2-runtime-implementation/review2-correction/matrix-after.log
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/stage2-runtime-implementation/review2-correction/matrix-after.process.json
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/stage2-runtime-implementation/review2-correction/matrix-after.resources.jsonl
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/stage2-runtime-implementation/review2-correction/matrix-after.result.json
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/stage2-runtime-implementation/review2-correction/matrix-after.source-proof.json
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/stage2-runtime-implementation/review2-correction/matrix-after.warm-target.json
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/stage2-runtime-implementation/review2-correction/matrix-before.command.json
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/stage2-runtime-implementation/review2-correction/matrix-before.exit
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/stage2-runtime-implementation/review2-correction/matrix-before.log
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/stage2-runtime-implementation/review2-correction/matrix-before.process.json
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/stage2-runtime-implementation/review2-correction/matrix-before.resources.jsonl
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/stage2-runtime-implementation/review2-correction/matrix-before.result.json
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/stage2-runtime-implementation/review2-correction/matrix-before.source-proof.json
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/stage2-runtime-implementation/review2-correction/matrix-before.warm-target.json
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/stage2-runtime-implementation/review2-correction/openapi.json
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/stage2-runtime-implementation/review2-correction/owned-source.json
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/stage2-runtime-implementation/review2-correction/preparation.json
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/stage2-runtime-implementation/review2-correction/report-raw.md
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/stage2-runtime-implementation/review2-correction/report.md
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/stage2-runtime-implementation/review2-correction/report.py
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/stage2-runtime-implementation/review2-correction/resources-summary.json
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/stage2-runtime-implementation/review2-correction/retained-auth-adversary2-hosted-first.command.json
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/stage2-runtime-implementation/review2-correction/retained-auth-adversary2-hosted-first.log
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/stage2-runtime-implementation/review2-correction/retained-auth-adversary2-hosted-first.result.json
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/stage2-runtime-implementation/review2-correction/retained-auth-adversary2-root-full.command.json
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/stage2-runtime-implementation/review2-correction/retained-auth-adversary2-root-full.log
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/stage2-runtime-implementation/review2-correction/retained-finding-reachability.json
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/stage2-runtime-implementation/review2-correction/retained-server-baseline.json
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/stage2-runtime-implementation/review2-correction/retained-server-baseline.log
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/stage2-runtime-implementation/review2-correction/review-inputs.json
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/stage2-runtime-implementation/review2-correction/root-fmt.command.json
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/stage2-runtime-implementation/review2-correction/root-fmt.exit
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/stage2-runtime-implementation/review2-correction/root-fmt.log
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/stage2-runtime-implementation/review2-correction/root-fmt.process.json
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/stage2-runtime-implementation/review2-correction/root-fmt.resources.jsonl
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/stage2-runtime-implementation/review2-correction/root-fmt.result.json
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/stage2-runtime-implementation/review2-correction/root-fmt.source-proof.json
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/stage2-runtime-implementation/review2-correction/root-fmt.warm-target.json
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/stage2-runtime-implementation/review2-correction/run_frozen_lane.py
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/stage2-runtime-implementation/review2-correction/run_lane.py
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/stage2-runtime-implementation/review2-correction/seal.py
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/stage2-runtime-implementation/review2-correction/server-clippy.command.json
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/stage2-runtime-implementation/review2-correction/server-clippy.exit
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/stage2-runtime-implementation/review2-correction/server-clippy.log
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/stage2-runtime-implementation/review2-correction/server-clippy.process.json
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/stage2-runtime-implementation/review2-correction/server-clippy.resources.jsonl
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/stage2-runtime-implementation/review2-correction/server-clippy.result.json
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/stage2-runtime-implementation/review2-correction/server-clippy.source-proof.json
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/stage2-runtime-implementation/review2-correction/server-clippy.warm-target.json
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/stage2-runtime-implementation/review2-correction/server-full.command.json
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/stage2-runtime-implementation/review2-correction/server-full.exit
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/stage2-runtime-implementation/review2-correction/server-full.log
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/stage2-runtime-implementation/review2-correction/server-full.process.json
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/stage2-runtime-implementation/review2-correction/server-full.resources.jsonl
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/stage2-runtime-implementation/review2-correction/server-full.result.json
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/stage2-runtime-implementation/review2-correction/server-full.source-proof.json
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/stage2-runtime-implementation/review2-correction/server-full.warm-target.json
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/stage2-runtime-implementation/review2-correction/source-before.json
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/stage2-runtime-implementation/review2-correction/source-preservation.json
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/stage2-runtime-implementation/review2-correction/source.patch
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/stage2-runtime-implementation/review2-correction/ten-test-prefixes.json
```

Assigned target and temp descendants are enumerated in final-target.json, final-executables.json and final-tmp.json. Existing Cargo cache bookkeeping paths are listed without claiming shared-cache cleanup:

```text
/dev/shm/b10x-cli-wave-wt-78f189927231-urhrmq5y/runtime-target
~/.cache/cw6/ac2
~/.cargo/.global-cache
~/.cargo/.package-cache
~/.cargo/.package-cache-mutate
```
