---
format: aep.planning-md/1
id: verification-report:cli-auth-service-port-checkpoint-20260906
kind: verification-report
status: draft
title: Partial authentication service-port checkpoint
relations:
- verifies: story:auth-as-tool-result
revision: 1
---
## Coordinator verification and limit

This partial checkpoint records the independent authentication service ports only. Full service grows from 64 to 67 passing cases, with no ignored tests; all three new cases execute, strict Clippy and service formatting pass. The initial missing-module compiler error is preserved as exit 101 with no executable product failure. The selected three are included in the full 67. Complete authentication runtime wiring, whole-unit review and integration/publication gates remain outstanding; the story stays active.

The coordinator verified all 80 evidence members, all 1,195 source files, all 1,869 target files and all 57 executable files against the returned manifests. The raw report SHA256 is db7042a7d5be877cc1cfb745bd09c3e1b27a0528de16ed76af0f206f615300b4, its portable rendering is 7f75ae32d8af2ef82fabf44409c326abe3871289efd8de942dbf488279d418b9, and the evidence manifest is a869b6df54fb426b0652b56186ed66d0f2d6ab6d04b37a80729e481804744c7a. Root independently proved that the portable rendering changes only the home-directory prefix to ~. Source patch SHA256 is 72fb990f68fba6848632de238cbe634fd38e8df9acac180923d98ac43c923ac8. Root proof auth-service-port-root-freeze-verification.json is retained under ~/.cache/cw6/p.

Root then committed exactly crates/service/src/lib.rs, runtime.rs and remediation.rs at ae53a93092453266cd5b40a5c6f9cee484478310, parent 05c94ac457938d2f9a8059f7f905dd8a87ec4dca, tree 79c487249c009f20bf882de36b9071406aa56aa0. Both author and committer are the organization bot. All 1,195 source hashes were reverified after commit and the managed tree is clean. Every old line in both existing files remains in order; no old assertion was removed. The source is locally committed and not published. The report below retains its earlier uncommitted observation unchanged.

The owned auth root target remains preserved. The worker released the sole compiling slot before sealing evidence; root explicitly returned it to the OAuth reviewer. Dry merge of frozen OAuth 6fae9df0 with this auth checkpoint produced tree 88574985438ffece4a3a6181bd6c30e9ff1292e9 without a conflict. That command changed no branch or worktree and performed no compilation, so it is structural evidence only.

## Complete frozen implementor report

unit:                   story:auth-as-tool-result — independent service-port slice
verdict:                green
cases:                  executed 64→67, red 0
origin:                 n/a
wrote-outside-worktree: ~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/stage1-protocol-implementation/stage2-service-port-implementation; compiler-managed TMPDIR ~/.cache/cw6/r and existing sccache writes
needs-coordinator:      no

This is the bounded implementation handoff authorized by story revision 61, not a whole-story or independent-review verdict. The header red 0 means final remaining executable failures. The initial tests-only result is retained below: one missing-module compiler diagnostic, with no executable product failure or test-summary count. No deliberately broken implementation was introduced. The three cases passed on their first executable run.

1. The unit and acceptance

Add the released service-only ports over protocol checkpoint 05c94ac457938d2f9a8059f7f905dd8a87ec4dca: closed readiness/errors, internal binding/metadata parameters, redacted diagnostics, and Unsupported defaults that perform no ordinary backend or authority work. Ordinary Operation wire-v2 and Connection-v1 methods and defaults remain byte-identical. The narrow exception and exact three owners are recorded at coordinator 538b0081e1a5ce2b0c19dc1d457fe1f5006b0871; brief SHA256 ba3ebeebc8138c7a493a7f59e9063138dc623c920beaa6761ee1aeb11347ee24. The committed source, authority files and prepared proposal are pinned in authority-inputs.json and preservation-before.json.

2. Exact source scope and diff

```text
$ git --no-pager diff --stat
 crates/service/src/lib.rs     |   6 ++
 crates/service/src/runtime.rs | 235 ++++++++++++++++++++++++++++++++++++++++++
 2 files changed, 241 insertions(+)
exit: 0

$ git --no-pager diff --no-index --stat -- /dev/null crates/service/src/remediation.rs
 /dev/null => crates/service/src/remediation.rs | 179 +++++++++++++++++++++++++
 1 file changed, 179 insertions(+)
exit: 1 (expected: new-file difference)
```

The tracked diff reports two files; its second read-only command records the new untracked remediation.rs separately. Together the patch adds 420 lines across exactly three owners, with no deletions: lib.rs adds 6 lines, runtime.rs adds 235, and remediation.rs adds 179. Final sizes are 77, 1068 and 179 lines. source.patch includes the new file, and source/ preserves the three exact final files.

- remediation.rs adds the five existing ESS readiness values, five value-free unit errors, exact target/session parameters, borrowed canonical catalog metadata, the existing eleven-field private binding value, a current-authority recheck trait, and bound request/result wrappers. No serialization or permissive authority implementation is added. Public Rust fields are internal application data, not authority proofs or wire output.
- runtime.rs adds four default methods. Ownership is false; metadata, readiness and bound handling remain Unsupported. It also contains the exact three prepared contract cases. The held seed test is absent.
- lib.rs registers and exports the new module. The inferred lib.rs and remediation.rs owners were checked against the tree: lib.rs already exists, remediation.rs is new, and existing protocol/catalog/domain dependencies satisfy the proposal without any manifest change.

The implementation matches the frozen proposal except for three comments updated from preparatory wording to describe applied source. Every proposed test assertion is preserved. No formatter rewrite was needed. Removing only the assigned additions reconstructs both old files byte-for-byte, including every old assertion and stable-authority method. All 1192 unrelated existing files remain unchanged. Source inventory: 1194 before, 1195 after; source-final.json pins every final tracked or assigned new file. All existing protocol readers, bundles, schema inventory, manifests, locks, ESS and planning bytes are preserved by that comparison. HEAD remains the clean checkpoint's commit; the three source changes are deliberately uncommitted.

3. First execution, before implementation

The unchanged full-service baseline executed 64 tests. The unchanged base filter selected 0 and filtered 64; after applying the exact separate contract fragment, the following command failed to compile because the module did not yet exist. This is API-absence evidence, not a failing behavioral assertion.

```text
$ cargo test --locked -p service remediation_contract_tests --no-fail-fast
   Compiling service v0.6.5 (~/.local/state/worktree/trees/b10x/connectors/wt-78f189927231/crates/service)
error[E0432]: unresolved import `crate::remediation`
   --> crates/service/src/runtime.rs:853:16
    |
853 |     use crate::remediation::*;
    |                ^^^^^^^^^^^ could not find `remediation` in the crate root

For more information about this error, try `rustc --explain E0432`.
error: could not compile `service` (lib test) due to 1 previous error
exit: 101
```

tests-only.patch and tests-only-source.json retain the source at that execution. The prepared implementation and contract patches remain separate. The seed-boundary test was never applied or executed. Initial scratch initialization and an unavailable presumed tool-prefix probe are separately recorded in artifact-preparation-observations.json; neither was a product test, source defect, or resource interruption.

4. Executed checks and exact outputs

All commands ran from the assigned worktree with CARGO_TARGET_DIR unset, TMPDIR=~/.cache/cw6/r, RUSTC_WRAPPER=/usr/bin/sccache, CARGO_INCREMENTAL=0, CARGO_PROFILE_DEV_DEBUG=0, CARGO_PROFILE_TEST_DEBUG=0, CARGO_BUILD_JOBS=1. PATH was prefixed with ~/.cargo/bin as recorded; actual PATH selection is /usr/bin/cargo via rustup, pinned in tool-identities.json. commands.json and each .command.json retain exact argv, complete explicit environment overrides, cwd, time and guard limits. All .log files contain full stdout/stderr; .exit, .result.json and .resources.jsonl retain the deciding status and resource timeline.

Full service: executed 64 → 67, exit 0 → 0; ignored 0 → 0. Doc-tests executed 0 → 0, exit 0 → 0.
Selected contract cases: executed 0 → 3, exit 0 → 0; final 64 filtered. The intermediate tests-only compiler attempt exits 101 and has no runner summary.
Strict all-target service Clippy: exit 0. Service formatting check: exit 0. Formatting ran on the same final source bytes; no source changed afterward.

The selected 3 are included in the full 67; these are not 70 unique cases. The final selected count grows from zero and the full count grows by three, so the new cases are actually selected. No case is ignored, removed, or weakened.

Unchanged full-service baseline:

```text
$ cargo test --locked -p service --no-fail-fast
   Compiling proc-macro2 v1.0.107
   Compiling unicode-ident v1.0.24
   Compiling quote v1.0.47
   Compiling syn v3.0.3
   Compiling syn v2.0.119
   Compiling synstructure v0.13.2
   Compiling version_check v0.9.5
   Compiling zerofrom-derive v0.1.7
   Compiling zerofrom v0.1.8
   Compiling yoke-derive v0.8.2
   Compiling stable_deref_trait v1.2.1
   Compiling yoke v0.8.3
   Compiling serde_core v1.0.229
   Compiling zerovec-derive v0.11.4
   Compiling zerovec v0.11.7
   Compiling displaydoc v0.2.7
   Compiling cfg-if v1.0.4
   Compiling autocfg v1.5.1
   Compiling memchr v2.8.3
   Compiling serde v1.0.229
   Compiling num-traits v0.2.19
   Compiling serde_derive v1.0.229
   Compiling libc v0.2.189
   Compiling zmij v1.0.23
   Compiling tinystr v0.8.4
   Compiling generic-array v0.14.7
   Compiling serde_json v1.0.151
   Compiling writeable v0.6.4
   Compiling smallvec v1.15.2
   Compiling litemap v0.8.3
   Compiling icu_locale_core v2.3.0
   Compiling zerotrie v0.2.5
   Compiling potential_utf v0.1.6
   Compiling itoa v1.0.18
   Compiling utf8_iter v1.0.4
   Compiling ref-cast v1.0.26
   Compiling typenum v1.20.1
   Compiling icu_normalizer_data v2.3.0
   Compiling icu_properties_data v2.3.0
   Compiling icu_collections v2.3.0
   Compiling icu_provider v2.3.0
   Compiling num-integer v0.1.47
   Compiling ref-cast-impl v1.0.26
   Compiling thiserror v2.0.20
   Compiling thiserror-impl v2.0.20
   Compiling getrandom v0.3.4
   Compiling zerocopy v0.8.56
   Compiling icu_properties v2.3.0
   Compiling icu_normalizer v2.3.0
   Compiling num-bigint v0.4.8
   Compiling crypto-common v0.1.7
   Compiling block-buffer v0.10.4
   Compiling ahash v0.8.12
   Compiling parking_lot_core v0.9.12
   Compiling regex-syntax v0.8.11
   Compiling percent-encoding v2.3.2
   Compiling digest v0.10.7
   Compiling num-rational v0.4.2
   Compiling idna_adapter v1.2.2
   Compiling num-iter v0.1.46
   Compiling num-complex v0.4.6
   Compiling aho-corasick v1.1.5
   Compiling borrow-or-share v0.2.4
   Compiling cpufeatures v0.2.17
   Compiling once_cell v1.21.4
   Compiling semver v1.0.28
   Compiling scopeguard v1.2.0
   Compiling lock_api v0.4.14
   Compiling rustc_version v0.4.1
   Compiling fluent-uri v0.4.1
   Compiling regex-automata v0.4.18
   Compiling num v0.4.3
   Compiling idna v1.1.0
   Compiling async-trait v0.1.92
   Compiling serde_derive_internals v0.30.0
   Compiling lazy_static v1.5.0
   Compiling equivalent v1.0.2
   Compiling zeroize v1.9.0
   Compiling rustix v1.1.4
   Compiling allocator-api2 v0.2.21
   Compiling foldhash v0.2.0
   Compiling bit-vec v0.8.0
   Compiling unicode-general-category v1.1.0
   Compiling heck v0.5.0
   Compiling strum_macros v0.28.0
   Compiling bit-set v0.8.0
   Compiling hashbrown v0.17.1
   Compiling fraction v0.15.4
   Compiling schemars_derive v1.2.2
   Compiling parking_lot v0.12.5
   Compiling curve25519-dalek v4.1.3
   Compiling sha2 v0.10.9
   Compiling micromap v0.3.0
   Compiling connector-secrets v0.6.5 (~/.local/state/worktree/trees/b10x/connectors/wt-78f189927231/crates/connector-secrets)
   Compiling vsimd v0.8.0
   Compiling bytecount v0.6.9
   Compiling dyn-clone v1.0.20
   Compiling num-cmp v0.1.0
   Compiling linux-raw-sys v0.12.1
   Compiling bitflags v2.13.1
   Compiling outref v0.5.2
   Compiling uuid-simd v0.8.0
   Compiling jsonschema-value v0.49.9
   Compiling schemars v1.2.2
   Compiling referencing v0.49.9
   Compiling fancy-regex v0.19.0
   Compiling strum v0.28.0
   Compiling regex v1.13.1
   Compiling form_urlencoded v1.2.2
   Compiling jsonschema-regex v0.49.9
   Compiling connector-state v0.6.5 (~/.local/state/worktree/trees/b10x/connectors/wt-78f189927231/crates/connector-state)
   Compiling connector-address v0.6.5 (~/.local/state/worktree/trees/b10x/connectors/wt-78f189927231/crates/connector-address)
   Compiling fs2 v0.4.3
   Compiling email_address v0.2.9
   Compiling curve25519-dalek-derive v0.1.1
   Compiling signature v2.2.0
   Compiling subtle v2.6.1
   Compiling data-encoding v2.11.1
   Compiling catalog-reader v0.6.5 (~/.local/state/worktree/trees/b10x/connectors/wt-78f189927231/crates/catalog-reader)
   Compiling catalog v0.6.5 (~/.local/state/worktree/trees/b10x/connectors/wt-78f189927231/crates/catalog)
   Compiling jsonschema v0.49.9
   Compiling ed25519 v2.2.3
   Compiling domain v0.6.5 (~/.local/state/worktree/trees/b10x/connectors/wt-78f189927231/crates/domain)
   Compiling url v2.5.8
   Compiling protocol v0.6.5 (~/.local/state/worktree/trees/b10x/connectors/wt-78f189927231/crates/protocol)
   Compiling connector-resolve v0.6.5 (~/.local/state/worktree/trees/b10x/connectors/wt-78f189927231/crates/connector-resolve)
   Compiling ed25519-dalek v2.2.0
   Compiling tokio-macros v2.7.2
   Compiling base64 v0.22.1
   Compiling pin-project-lite v0.2.17
   Compiling tokio v1.53.1
   Compiling service v0.6.5 (~/.local/state/worktree/trees/b10x/connectors/wt-78f189927231/crates/service)
    Finished `test` profile [unoptimized] target(s) in 1m 02s
     Running unittests src/lib.rs (target/debug/deps/service-5b47d2fca9d0a754)

running 64 tests
test admin::tests::status_reports_presence_without_reading_the_value ... ok
test audio::tests::a_speech_plan_and_its_deployment_route_admit_together ... ok
test admin::tests::write_requires_explicit_replacement_and_audits_no_secret ... ok
test audio::tests::admitted_evidence_never_prints_as_a_serializable_route_secret ... ok
test audio::tests::relative_paths_absent_bounds_and_bad_digests_never_reach_the_device ... ok
test audio::tests::a_route_for_another_connection_is_refused ... ok
test audio::tests::a_plan_for_another_driver_or_operation_is_refused ... ok
test audio::tests::status_never_admits_an_utterance ... ok
test authority::tests::debug_never_prints_authority_or_proof ... ok
test audio::tests::the_caller_text_is_bounded_by_the_deployment_and_not_only_by_the_catalog ... ok
test authority::tests::proof_is_bound_to_exact_upgrade_uri ... ok
test browser::tests::a_non_web_address_is_refused_before_any_browser_is_touched ... ok
test browser::tests::a_plan_for_another_driver_or_operation_is_refused ... ok
test browser::tests::a_route_for_another_connection_is_refused ... ok
test browser::tests::a_unary_lifecycle_is_refused_because_a_browser_spans_calls ... ok
test browser::tests::admitted_evidence_never_prints_as_a_serializable_route_secret ... ok
test browser::tests::every_admitted_browser_operation_and_its_route_admit_together ... ok
test browser::tests::only_open_may_omit_an_address_and_only_open_or_goto_may_carry_one ... ok
test browser::tests::relative_paths_nested_artifacts_and_absent_bounds_never_reach_the_browser ... ok
test browser::tests::the_operators_own_browser_profile_is_never_an_admitted_route ... ok
test connect_session::tests::a_completion_claim_blocks_expiry_shutdown_and_duplicate_completion ... ok
test connect_session::tests::authority_and_claim_use_one_receiver_owned_instant ... ok
test connect_session::tests::capacity_counts_only_pending_sessions ... ok
test connect_session::tests::clock_failure_refuses_authorization_but_does_not_prevent_confirmed_abort ... ok
test authority::tests::serving_endpoint_redeems_once ... ok
test connect_session::tests::preparing_and_uncertain_abort_cannot_publish_a_terminal_result ... ok
test connect_session::tests::shutdown_fails_only_pending_sessions_and_returns_their_endpoints ... ok
test connect_session::tests::terminal_transition_is_one_way_and_value_free ... ok
test connect_session::tests::guarded_sessions_keep_capacity_until_a_confirmed_outcome ... ok
test connect_session::tests::the_claim_rechecks_the_original_target_and_inclusive_deadline_once ... ok
test dispatch::tests::composition_order_is_policy_redaction_audit_driver_audit ... ok
test egress::tests::retry_delay_is_one_unsigned_decimal_with_only_http_whitespace ... ok
test authority::tests::audience_expiry_and_revocation_fail_before_redemption ... ok
test authority::tests::session_lease_cannot_be_extended_by_authority_clock_skew ... ok
test runtime::tests::delegated_execution_must_match_the_authenticated_actor ... ok
test runtime::tests::hosted_principals_do_not_fabricate_agent_revisions ... ok
test planning::tests::provider_only_connection_refuses_a_caller_initiated_operation ... ok
test planning::tests::sip_plan_has_no_http_fields ... ok
test planning::tests::bidirectional_connection_still_requires_the_operation_admission ... ok
test runtime::tests::the_stable_authority_seed_distinguishes_absent_and_literal_default_realms ... ok
test runtime::tests::the_stable_authority_seed_ignores_request_scoped_provenance ... ok
test runtime::tests::the_stable_authority_seed_survives_token_scoped_snapshot_fields ... ok
test sip::tests::a_default_naming_an_absent_trunk_is_refused_when_the_table_is_built ... ok
test sip::tests::a_dial_with_no_alias_and_no_default_is_refused_rather_than_guessed ... ok
test sip::tests::a_dial_with_only_a_number_takes_the_declared_default_trunk ... ok
test runtime::tests::hosted_completion_submission_never_reallocates_received_secret_material ... ok
test sip::tests::a_prefix_aperture_admits_its_network_and_refuses_outside_it ... ok
test planning::tests::mediated_http_plan_has_no_direct_origin_and_requires_the_closed_adapter ... ok
test sip::tests::a_prefix_that_is_not_on_its_own_boundary_is_refused_rather_than_rounded ... ok
test sip::tests::a_named_trunk_is_resolved_before_admission_so_the_answer_is_aperture_checked ... ok
test sip::tests::an_aperture_never_matches_across_address_families ... ok
test sip::tests::a_trunk_that_does_not_admit_numbers_refuses_one ... ok
test sip::tests::an_exact_aperture_still_admits_exactly_one_address ... ok
test sip::tests::a_partial_byte_prefix_masks_only_the_bits_it_declares ... ok
test planning::tests::missing_driver_refuses_before_dispatch ... ok
test sip::tests::sip_dial_resolves_only_an_exact_connection_owned_alias ... ok
test sip::tests::exact_loopback_route_produces_non_serializable_driver_evidence ... ok
test runtime::tests::local_principals_require_a_real_agent_revision ... ok
test sip::tests::stable_network_and_aperture_widening_refuse_before_the_driver ... ok
test sip::tests::the_dialled_host_comes_from_the_trunk_and_never_from_the_caller ... ok
test sip::tests::zero_or_excessive_dial_deadlines_refuse_before_the_driver ... ok
test voice::tests::invalid_endpoint_and_authority_windows_refuse_before_io ... ok
test sip::tests::missing_organization_in_grant_evidence_refuses_before_the_driver ... ok
test voice::tests::one_proof_joins_exact_sip_and_application_routes ... ok

test result: ok. 64 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

   Doc-tests service

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

exit: 0
```

Unchanged base selected filter:

```text
$ cargo test --locked -p service remediation_contract_tests --no-fail-fast
    Finished `test` profile [unoptimized] target(s) in 0.10s
     Running unittests src/lib.rs (target/debug/deps/service-5b47d2fca9d0a754)

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 64 filtered out; finished in 0.00s

exit: 0
```

First executable selected cases:

```text
$ cargo test --locked -p service remediation_contract_tests --no-fail-fast
   Compiling service v0.6.5 (~/.local/state/worktree/trees/b10x/connectors/wt-78f189927231/crates/service)
    Finished `test` profile [unoptimized] target(s) in 1.06s
     Running unittests src/lib.rs (target/debug/deps/service-5b47d2fca9d0a754)

running 3 tests
test runtime::remediation_contract_tests::remediation_defaults_do_not_run_existing_backend_work ... ok
test runtime::remediation_contract_tests::remediation_default_bound_methods_refuse_without_authority_or_operation_work ... ok
test runtime::remediation_contract_tests::remediation_internal_diagnostics_do_not_trust_printable_reference_fields ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 64 filtered out; finished in 0.00s

exit: 0
```

Full final service suite:

```text
$ cargo test --locked -p service --no-fail-fast
   Compiling service v0.6.5 (~/.local/state/worktree/trees/b10x/connectors/wt-78f189927231/crates/service)
    Finished `test` profile [unoptimized] target(s) in 0.82s
     Running unittests src/lib.rs (target/debug/deps/service-5b47d2fca9d0a754)

running 67 tests
test audio::tests::a_plan_for_another_driver_or_operation_is_refused ... ok
test audio::tests::a_speech_plan_and_its_deployment_route_admit_together ... ok
test audio::tests::relative_paths_absent_bounds_and_bad_digests_never_reach_the_device ... ok
test admin::tests::status_reports_presence_without_reading_the_value ... ok
test audio::tests::a_route_for_another_connection_is_refused ... ok
test admin::tests::write_requires_explicit_replacement_and_audits_no_secret ... ok
test audio::tests::admitted_evidence_never_prints_as_a_serializable_route_secret ... ok
test audio::tests::the_caller_text_is_bounded_by_the_deployment_and_not_only_by_the_catalog ... ok
test audio::tests::status_never_admits_an_utterance ... ok
test browser::tests::a_non_web_address_is_refused_before_any_browser_is_touched ... ok
test browser::tests::a_plan_for_another_driver_or_operation_is_refused ... ok
test browser::tests::a_route_for_another_connection_is_refused ... ok
test browser::tests::a_unary_lifecycle_is_refused_because_a_browser_spans_calls ... ok
test authority::tests::debug_never_prints_authority_or_proof ... ok
test browser::tests::admitted_evidence_never_prints_as_a_serializable_route_secret ... ok
test authority::tests::proof_is_bound_to_exact_upgrade_uri ... ok
test browser::tests::only_open_may_omit_an_address_and_only_open_or_goto_may_carry_one ... ok
test browser::tests::relative_paths_nested_artifacts_and_absent_bounds_never_reach_the_browser ... ok
test browser::tests::every_admitted_browser_operation_and_its_route_admit_together ... ok
test connect_session::tests::a_completion_claim_blocks_expiry_shutdown_and_duplicate_completion ... ok
test connect_session::tests::clock_failure_refuses_authorization_but_does_not_prevent_confirmed_abort ... ok
test authority::tests::session_lease_cannot_be_extended_by_authority_clock_skew ... ok
test connect_session::tests::authority_and_claim_use_one_receiver_owned_instant ... ok
test connect_session::tests::capacity_counts_only_pending_sessions ... ok
test connect_session::tests::guarded_sessions_keep_capacity_until_a_confirmed_outcome ... ok
test browser::tests::the_operators_own_browser_profile_is_never_an_admitted_route ... ok
test connect_session::tests::shutdown_fails_only_pending_sessions_and_returns_their_endpoints ... ok
test connect_session::tests::preparing_and_uncertain_abort_cannot_publish_a_terminal_result ... ok
test connect_session::tests::terminal_transition_is_one_way_and_value_free ... ok
test connect_session::tests::the_claim_rechecks_the_original_target_and_inclusive_deadline_once ... ok
test dispatch::tests::composition_order_is_policy_redaction_audit_driver_audit ... ok
test egress::tests::retry_delay_is_one_unsigned_decimal_with_only_http_whitespace ... ok
test authority::tests::serving_endpoint_redeems_once ... ok
test runtime::remediation_contract_tests::remediation_default_bound_methods_refuse_without_authority_or_operation_work ... ok
test planning::tests::missing_driver_refuses_before_dispatch ... ok
test planning::tests::provider_only_connection_refuses_a_caller_initiated_operation ... ok
test planning::tests::bidirectional_connection_still_requires_the_operation_admission ... ok
test planning::tests::mediated_http_plan_has_no_direct_origin_and_requires_the_closed_adapter ... ok
test runtime::remediation_contract_tests::remediation_defaults_do_not_run_existing_backend_work ... ok
test planning::tests::sip_plan_has_no_http_fields ... ok
test runtime::tests::hosted_completion_submission_never_reallocates_received_secret_material ... ok
test runtime::tests::hosted_principals_do_not_fabricate_agent_revisions ... ok
test authority::tests::audience_expiry_and_revocation_fail_before_redemption ... ok
test runtime::tests::local_principals_require_a_real_agent_revision ... ok
test runtime::tests::the_stable_authority_seed_survives_token_scoped_snapshot_fields ... ok
test sip::tests::a_default_naming_an_absent_trunk_is_refused_when_the_table_is_built ... ok
test runtime::tests::delegated_execution_must_match_the_authenticated_actor ... ok
test sip::tests::a_dial_with_no_alias_and_no_default_is_refused_rather_than_guessed ... ok
test sip::tests::a_dial_with_only_a_number_takes_the_declared_default_trunk ... ok
test runtime::tests::the_stable_authority_seed_distinguishes_absent_and_literal_default_realms ... ok
test sip::tests::a_named_trunk_is_resolved_before_admission_so_the_answer_is_aperture_checked ... ok
test sip::tests::a_prefix_that_is_not_on_its_own_boundary_is_refused_rather_than_rounded ... ok
test sip::tests::a_partial_byte_prefix_masks_only_the_bits_it_declares ... ok
test sip::tests::an_aperture_never_matches_across_address_families ... ok
test sip::tests::a_trunk_that_does_not_admit_numbers_refuses_one ... ok
test runtime::tests::the_stable_authority_seed_ignores_request_scoped_provenance ... ok
test sip::tests::a_prefix_aperture_admits_its_network_and_refuses_outside_it ... ok
test sip::tests::an_exact_aperture_still_admits_exactly_one_address ... ok
test sip::tests::exact_loopback_route_produces_non_serializable_driver_evidence ... ok
test sip::tests::missing_organization_in_grant_evidence_refuses_before_the_driver ... ok
test sip::tests::sip_dial_resolves_only_an_exact_connection_owned_alias ... ok
test sip::tests::stable_network_and_aperture_widening_refuse_before_the_driver ... ok
test sip::tests::the_dialled_host_comes_from_the_trunk_and_never_from_the_caller ... ok
test runtime::remediation_contract_tests::remediation_internal_diagnostics_do_not_trust_printable_reference_fields ... ok
test sip::tests::zero_or_excessive_dial_deadlines_refuse_before_the_driver ... ok
test voice::tests::invalid_endpoint_and_authority_windows_refuse_before_io ... ok
test voice::tests::one_proof_joins_exact_sip_and_application_routes ... ok

test result: ok. 67 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

   Doc-tests service

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

exit: 0
```

Strict all-target service Clippy:

```text
$ cargo clippy --locked -p service --all-targets -- -D warnings
    Checking zerofrom v0.1.8
    Checking stable_deref_trait v1.2.1
    Checking yoke v0.8.3
    Checking zerovec v0.11.7
    Checking serde_core v1.0.229
    Checking cfg-if v1.0.4
    Checking memchr v2.8.3
    Checking serde v1.0.229
    Checking num-traits v0.2.19
    Checking libc v0.2.189
    Checking tinystr v0.8.4
    Checking smallvec v1.15.2
    Checking writeable v0.6.4
    Checking litemap v0.8.3
    Checking icu_locale_core v2.3.0
    Checking zmij v1.0.23
    Checking zerotrie v0.2.5
    Checking potential_utf v0.1.6
    Checking utf8_iter v1.0.4
    Checking itoa v1.0.18
    Checking typenum v1.20.1
    Checking generic-array v0.14.7
    Checking serde_json v1.0.151
    Checking icu_collections v2.3.0
    Checking icu_provider v2.3.0
    Checking num-integer v0.1.47
    Checking ref-cast v1.0.26
    Checking icu_properties_data v2.3.0
    Checking icu_normalizer_data v2.3.0
    Checking thiserror v2.0.20
    Checking icu_normalizer v2.3.0
    Checking icu_properties v2.3.0
    Checking num-bigint v0.4.8
    Checking crypto-common v0.1.7
    Checking block-buffer v0.10.4
    Checking regex-syntax v0.8.11
    Checking percent-encoding v2.3.2
    Checking digest v0.10.7
    Checking num-rational v0.4.2
    Checking idna_adapter v1.2.2
    Checking getrandom v0.3.4
    Checking zerocopy v0.8.56
    Checking num-iter v0.1.46
    Checking num-complex v0.4.6
    Checking aho-corasick v1.1.5
    Checking cpufeatures v0.2.17
    Checking borrow-or-share v0.2.4
    Checking once_cell v1.21.4
    Checking scopeguard v1.2.0
    Checking lock_api v0.4.14
    Checking ahash v0.8.12
    Checking fluent-uri v0.4.1
    Checking regex-automata v0.4.18
    Checking num v0.4.3
    Checking idna v1.1.0
    Checking parking_lot_core v0.9.12
    Checking foldhash v0.2.0
    Checking zeroize v1.9.0
    Checking allocator-api2 v0.2.21
    Checking lazy_static v1.5.0
    Checking bit-vec v0.8.0
    Checking equivalent v1.0.2
    Checking hashbrown v0.17.1
    Checking bit-set v0.8.0
    Checking fraction v0.15.4
    Checking parking_lot v0.12.5
    Checking sha2 v0.10.9
    Checking linux-raw-sys v0.12.1
    Checking micromap v0.3.0
    Checking outref v0.5.2
    Checking vsimd v0.8.0
   Compiling connector-secrets v0.6.5 (~/.local/state/worktree/trees/b10x/connectors/wt-78f189927231/crates/connector-secrets)
    Checking bitflags v2.13.1
    Checking num-cmp v0.1.0
    Checking bytecount v0.6.9
    Checking dyn-clone v1.0.20
    Checking schemars v1.2.2
    Checking jsonschema-value v0.49.9
    Checking rustix v1.1.4
    Checking uuid-simd v0.8.0
    Checking referencing v0.49.9
    Checking strum v0.28.0
    Checking fancy-regex v0.19.0
    Checking unicode-general-category v1.1.0
    Checking regex v1.13.1
    Checking form_urlencoded v1.2.2
    Checking jsonschema-regex v0.49.9
    Checking connector-state v0.6.5 (~/.local/state/worktree/trees/b10x/connectors/wt-78f189927231/crates/connector-state)
    Checking connector-address v0.6.5 (~/.local/state/worktree/trees/b10x/connectors/wt-78f189927231/crates/connector-address)
    Checking fs2 v0.4.3
    Checking email_address v0.2.9
    Checking data-encoding v2.11.1
    Checking subtle v2.6.1
    Checking catalog-reader v0.6.5 (~/.local/state/worktree/trees/b10x/connectors/wt-78f189927231/crates/catalog-reader)
    Checking signature v2.2.0
    Checking ed25519 v2.2.3
    Checking catalog v0.6.5 (~/.local/state/worktree/trees/b10x/connectors/wt-78f189927231/crates/catalog)
    Checking curve25519-dalek v4.1.3
    Checking jsonschema v0.49.9
    Checking domain v0.6.5 (~/.local/state/worktree/trees/b10x/connectors/wt-78f189927231/crates/domain)
    Checking url v2.5.8
    Checking protocol v0.6.5 (~/.local/state/worktree/trees/b10x/connectors/wt-78f189927231/crates/protocol)
    Checking connector-resolve v0.6.5 (~/.local/state/worktree/trees/b10x/connectors/wt-78f189927231/crates/connector-resolve)
    Checking ed25519-dalek v2.2.0
    Checking pin-project-lite v0.2.17
    Checking base64 v0.22.1
    Checking tokio v1.53.1
    Checking service v0.6.5 (~/.local/state/worktree/trees/b10x/connectors/wt-78f189927231/crates/service)
    Finished `dev` profile [unoptimized] target(s) in 23.06s
exit: 0
```

Affected formatting:

```text
$ cargo fmt -p service -- --check
exit: 0
```

The first two new cases exercise an actual Arc<dyn ConnectorBackend> implementation with only the old mandatory methods. Unknown/private-looking target references and all Start/Status/Acknowledge requests retain Unsupported; backend work and authority callback counters remain zero. The third case decodes an actually valid hostile Operation-v3 envelope whose printable references and message carry a synthetic private URL, confirms that DTO validity preserves that message, and checks exact redacted Debug strings for the binding/request/trusted acknowledgement wrapper. These observations establish bounded default and diagnostic behavior; they do not establish safe client, console, MCP or hosted output.

The adapted runner continuously sampled all four guards at one-second intervals and targeted only its own process group. Minimum free disk 28948119552 bytes; minimum free tmpfs 24777031680 bytes; minimum MemAvailable 40032649216 bytes. Their floors were 12884901888, 8589934592 and 17179869184 bytes respectively. Maximum owned target was 555450368 bytes against the 8589934592-byte cap, using the greater of allocated and apparent du totals. All seven lanes had zero interruptions and empty remaining process groups. compile-slot-release.json records the explicit release at 2026-09-06T17:20:30.071710+00:00, before report sealing. No compilation remains assigned to this implementor.

Target started absent and remains retained without cleanup. target-final-inventory.json hashes all 1869 final target files; target-executables.json separately pins 57 executable files. Full source and those target hashes were verified again at sealing. All six previous manifests remain unchanged: 261 original protocol entries, 43 runtime-plan entries, 21 broader-validation entries, 12 cleanup entries, 45 service-proposal entries and 32 hosted-proposal entries, totaling 414 retained manifest entries. No prior report or manifest was rewritten.

5. Remaining work and deliberate boundaries

Actual OAuth acquisition, readiness classification, authority construction/recheck invocation, custody publication, grant admission, registry routing, lifecycle/one-use behavior, server/local/hosted transports, client/CLI/console/MCP projections and explicit resumption remain held for the reviewed OAuth handoff and exact later scope. The current methods implement none of those behaviors. Future production hosted acquisition must stay Unsupported until a supported adapter exists. The generic hostile-envelope output obligation remains open; Debug redaction and fixed service errors do not discharge it.

No seed-boundary change, hosted proposal, extraction, new dependency, lock, protocol, generated artifact, JSON inventory, ESS, AEP, Git commit/ref, daemon, provider, operator or external application change occurred. The existing real policy, grant reference/revision and digest owners still supply all authority facts. No fabricated authority or new durable entity was introduced. No broader workspace validation or independent adversary pass is claimed by this service-only handoff. No extra source owner or dependency gap was found; needs-coordinator:no means no additional patch is required for this bounded slice, not permission to apply runtime wiring or publish it.

6. Every explicit outside-worktree write

All explicit scratch writes are listed below and in outside-writes.txt. Source writes are only the three assigned worktree files. Compiler-managed transient writes used the authorized TMPDIR; ordinary sccache wrapper writes used its existing configuration. Their individual transient/cache files were not instrumented and are not claimed as a complete per-file cache inventory; no manual target/cache cleanup or other-tree write occurred. outside-write-roots.json records that distinction. The retained target inventory is inside the assigned worktree.

```text
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/stage1-protocol-implementation/stage2-service-port-implementation/artifact-preparation-observations.json
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/stage1-protocol-implementation/stage2-service-port-implementation/authority-inputs.json
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/stage1-protocol-implementation/stage2-service-port-implementation/brief.md
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/stage1-protocol-implementation/stage2-service-port-implementation/clippy-after.command.json
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/stage1-protocol-implementation/stage2-service-port-implementation/clippy-after.exit
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/stage1-protocol-implementation/stage2-service-port-implementation/clippy-after.log
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/stage1-protocol-implementation/stage2-service-port-implementation/clippy-after.process.json
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/stage1-protocol-implementation/stage2-service-port-implementation/clippy-after.resources.jsonl
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/stage1-protocol-implementation/stage2-service-port-implementation/clippy-after.result.json
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/stage1-protocol-implementation/stage2-service-port-implementation/commands.json
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/stage1-protocol-implementation/stage2-service-port-implementation/compile-slot-release.json
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/stage1-protocol-implementation/stage2-service-port-implementation/coordinator-story-revision61.md
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/stage1-protocol-implementation/stage2-service-port-implementation/evidence.sha256
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/stage1-protocol-implementation/stage2-service-port-implementation/final-verification.json
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/stage1-protocol-implementation/stage2-service-port-implementation/fmt-first.command.json
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/stage1-protocol-implementation/stage2-service-port-implementation/fmt-first.exit
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/stage1-protocol-implementation/stage2-service-port-implementation/fmt-first.log
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/stage1-protocol-implementation/stage2-service-port-implementation/fmt-first.process.json
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/stage1-protocol-implementation/stage2-service-port-implementation/fmt-first.resources.jsonl
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/stage1-protocol-implementation/stage2-service-port-implementation/fmt-first.result.json
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/stage1-protocol-implementation/stage2-service-port-implementation/implementation-first-source.json
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/stage1-protocol-implementation/stage2-service-port-implementation/lane-summaries.json
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/stage1-protocol-implementation/stage2-service-port-implementation/outside-write-roots.json
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/stage1-protocol-implementation/stage2-service-port-implementation/outside-writes.txt
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/stage1-protocol-implementation/stage2-service-port-implementation/preflight.json
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/stage1-protocol-implementation/stage2-service-port-implementation/preimages/lib.rs
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/stage1-protocol-implementation/stage2-service-port-implementation/preimages/runtime.rs
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/stage1-protocol-implementation/stage2-service-port-implementation/prepared-contract-tests.patch
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/stage1-protocol-implementation/stage2-service-port-implementation/prepared-implementation.patch
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/stage1-protocol-implementation/stage2-service-port-implementation/preservation-after.json
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/stage1-protocol-implementation/stage2-service-port-implementation/preservation-before.json
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/stage1-protocol-implementation/stage2-service-port-implementation/report-portable.md
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/stage1-protocol-implementation/stage2-service-port-implementation/report.md
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/stage1-protocol-implementation/stage2-service-port-implementation/resource-summary.json
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/stage1-protocol-implementation/stage2-service-port-implementation/run_lane.py
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/stage1-protocol-implementation/stage2-service-port-implementation/runner-adaptation.patch
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/stage1-protocol-implementation/stage2-service-port-implementation/runner-preimage.py
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/stage1-protocol-implementation/stage2-service-port-implementation/seal_reports.py
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/stage1-protocol-implementation/stage2-service-port-implementation/selected-after.command.json
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/stage1-protocol-implementation/stage2-service-port-implementation/selected-after.exit
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/stage1-protocol-implementation/stage2-service-port-implementation/selected-after.log
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/stage1-protocol-implementation/stage2-service-port-implementation/selected-after.process.json
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/stage1-protocol-implementation/stage2-service-port-implementation/selected-after.resources.jsonl
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/stage1-protocol-implementation/stage2-service-port-implementation/selected-after.result.json
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/stage1-protocol-implementation/stage2-service-port-implementation/selected-before.command.json
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/stage1-protocol-implementation/stage2-service-port-implementation/selected-before.exit
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/stage1-protocol-implementation/stage2-service-port-implementation/selected-before.log
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/stage1-protocol-implementation/stage2-service-port-implementation/selected-before.process.json
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/stage1-protocol-implementation/stage2-service-port-implementation/selected-before.resources.jsonl
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/stage1-protocol-implementation/stage2-service-port-implementation/selected-before.result.json
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/stage1-protocol-implementation/stage2-service-port-implementation/selected-tests-first.command.json
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/stage1-protocol-implementation/stage2-service-port-implementation/selected-tests-first.exit
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/stage1-protocol-implementation/stage2-service-port-implementation/selected-tests-first.log
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/stage1-protocol-implementation/stage2-service-port-implementation/selected-tests-first.process.json
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/stage1-protocol-implementation/stage2-service-port-implementation/selected-tests-first.resources.jsonl
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/stage1-protocol-implementation/stage2-service-port-implementation/selected-tests-first.result.json
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/stage1-protocol-implementation/stage2-service-port-implementation/service-after.command.json
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/stage1-protocol-implementation/stage2-service-port-implementation/service-after.exit
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/stage1-protocol-implementation/stage2-service-port-implementation/service-after.log
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/stage1-protocol-implementation/stage2-service-port-implementation/service-after.process.json
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/stage1-protocol-implementation/stage2-service-port-implementation/service-after.resources.jsonl
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/stage1-protocol-implementation/stage2-service-port-implementation/service-after.result.json
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/stage1-protocol-implementation/stage2-service-port-implementation/service-before.command.json
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/stage1-protocol-implementation/stage2-service-port-implementation/service-before.exit
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/stage1-protocol-implementation/stage2-service-port-implementation/service-before.log
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/stage1-protocol-implementation/stage2-service-port-implementation/service-before.process.json
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/stage1-protocol-implementation/stage2-service-port-implementation/service-before.resources.jsonl
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/stage1-protocol-implementation/stage2-service-port-implementation/service-before.result.json
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/stage1-protocol-implementation/stage2-service-port-implementation/source-before.json
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/stage1-protocol-implementation/stage2-service-port-implementation/source-diff-stat.log
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/stage1-protocol-implementation/stage2-service-port-implementation/source-final.json
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/stage1-protocol-implementation/stage2-service-port-implementation/source-tracked.patch
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/stage1-protocol-implementation/stage2-service-port-implementation/source.patch
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/stage1-protocol-implementation/stage2-service-port-implementation/source/lib.rs
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/stage1-protocol-implementation/stage2-service-port-implementation/source/remediation.rs
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/stage1-protocol-implementation/stage2-service-port-implementation/source/runtime.rs
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/stage1-protocol-implementation/stage2-service-port-implementation/target-executables.json
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/stage1-protocol-implementation/stage2-service-port-implementation/target-final-inventory.json
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/stage1-protocol-implementation/stage2-service-port-implementation/tests-only-source.json
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/stage1-protocol-implementation/stage2-service-port-implementation/tests-only.patch
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/stage1-protocol-implementation/stage2-service-port-implementation/tool-identities.json
```

report.md is the full raw report. report-portable.md is its mechanical portable rendering: the original home-directory prefix is replaced with ~. All other report text is identical. Raw logs retain exact machine paths. evidence.sha256 is the final scratch manifest, excluding only itself; no frozen predecessor entry is replaced. The source/evidence handoff remains uncommitted for coordinator disposition.
