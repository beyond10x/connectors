unit: story:ignored-suites-have-a-runner — The ignored suites have a command that runs them
verdict: red
cases: executed 140→151, red 8 initially; final package 151 passed; operator 31 passed/1 failed/1 missing
origin: n/a
wrote-outside-worktree: $HOME/.cache/cb26b-runner/tmp (approved); tool-managed worktree lease and Cargo/sccache caches
needs-coordinator: yes — operator Chrome failure and unavailable historical binary; final integrated gate and publication

The retained operator-host disposable run is RED: 45 ignored cases inventoried, 33 selected, 32 executed (31 passed, 1 failed), 1 missing prerequisite, 13 skipped, 0 unknown; command exit 1. The implementation propagates both the named failure and the required-family refusal. This is not a claim that the ignored suite is release-green.

1. Unit and acceptance

Implement a clap-derived Rust runner that discovers actual ignored libtest cases, accounts for every name, defaults to disposable fixtures, excludes subprocess helpers, requires explicit live/timing selection, reports missing prerequisites, and propagates selected failures. The inferred new module path was checked: ignored.rs did not exist on the base. Existing main dispatch and development guide were confirmed. No blocking dependency edge was found in the retained graph.json. The story's historical 35 is not used: compiled workspace inventory contains 45 cases in 134 test binaries (33 disposable, 3 live, 3 timing, 1 deferred, 5 helpers).

2. Change

Tracked diff before staging the new module:

```text
 Cargo.lock                          |  1 +
 crates/connectors-build/Cargo.toml  |  1 +
 crates/connectors-build/src/main.rs |  6 ++++
 docs/development.md                 | 59 +++++++++++++++++++++++++++++++++++++
 4 files changed, 67 insertions(+)
```

Git's ordinary diff stat omitted the new ignored.rs before staging; it contains the implementation and tests. Exact package+target+case classification fails closed for unknown cases. Reports contain every name, family, selection, status and reason. Each selected executable runs one exact ignored case and must report exactly one executed case. Required families refuse missing/empty execution. Live credentials never select a family. Existing gate.rs and workflows remain unchanged.

The routed first adversary found descendants left alive after both passing and failing test parents. The fix covers normal pass, assertion failure, timeout, and runner-error exit: an owned process-group guard observes the leader with waitid WNOWAIT, signals that group before reaping the leader, then uses bounded group-only waitpid and Linux child-subreaper adoption. It never scans unrelated processes. A new reviewer case checks guard drop on an injected error and confirms another live process group is unaffected. libc uses the already resolved version; only the dependency-name lock entry changed. The detached-session limitation remains documented.

3. Red test-first evidence

Initial command: `cargo test --locked --offline -p connectors-build ignored::tests -- --nocapture`; eight authored cases ran against stubs before implementation. Full verbatim output is appended below from red.log (exit 101; 0 passed, 8 failed). They exercise actual compiled libtest executables and exact selection, not source counting.

First independent adversary: adversary-1/report.md and logs retain 150 executed, 148 passed, 2 failed. Coordinator recorded and routed this report before the process-group fix. Its passing-parent and panicking-parent descendant cases subsequently passed (adversary-repair-focused.log).

4. Verification and operator evidence

Environment: CARGO_BUILD_JOBS=2; RUSTC_WRAPPER=/usr/bin/sccache; CONNECTORS_ESS=$HOME/.cache/ess/toolchains/0.45.0/ess; CONNECTORS_AEP=$HOME/.cache/aep/toolchains/0.65.0/aep-0.65.0-x86_64-unknown-linux-gnu/aep; TMPDIR=$HOME/.cache/cb26b-runner/tmp for final package and host execution. No shared CARGO_TARGET_DIR.

Package baseline `cargo test --locked --offline -p connectors-build`: 140 passed across 18 runner summaries (baseline-canonical.log). The initial alias-TMPDIR baseline had one existing docs-path assertion failure (baseline.log); canonicalizing the temporary root for the unchanged baseline produced the recorded 140-pass result.

Initial focused lane: measured 8 executed red→8 executed green; red exit 101, then 8 passed exit 0 (red.log, green-focused.log). A separate filtered baseline invocation was not retained; the whole-package baseline is the actual measured before count. Package after implementation: executed 140→148, exit 0 (green-package-final.log, package-final-exit.txt). After first adversary correction: executed 148→150, exit 0 (green-package-reviewed.log, package-reviewed-exit.txt). After final review guard-drop test: executed 150→151, exit 0 (adversary-2/package.log, package.exit). Final handoff rerun after diagnostic disposition: executed 140 baseline→151 final, 151 passed/0 failed/0 ignored across 18 runner summaries, exit 0 (package-handoff.log, package-handoff.exit).

The second adversary's isolated private-call SIGCHLD=SIG_IGN probe was measured red: 151 passed/1 failed (adversary-2/package-autoreap.log). The corresponding actual public CLI inherited the same disposition and refused during its earlier Cargo metadata wait with ECHILD, before fixture dispatch (adversary-2/autoreap-production.log, exit 1). The supported CLI therefore requires ordinary waitable-child semantics to get past inventory. Its private execution helper assumes that earlier precondition; it is not a separately supported API for callers that mutate process-global child signal disposition. Coordinator recorded the reviewer-recommended INFEASIBLE/warning no-op disposition and explicitly directed removal of only this diagnostic from the shipped test section. Its full source remains in adversary-2/tests-final.patch, with focused red and production-refusal logs preserved. No production signal guard was added. Both original descendant cases and the green guard-drop/unrelated-group case remain active. No third adversary pass was opened.

`cargo clippy --locked --offline -p connectors-build --all-targets -- -D warnings`: exit 0 (clippy-final.log, clippy-final-exit.txt). `cargo fmt -p connectors-build --check`: exit 0 (fmt-final.log, fmt-final-exit.txt). `git diff --check`: exit 0.

Authorized acceptance-load command `cargo test --locked --offline --workspace --no-fail-fast`: command started 12:20:12 UTC, actual libtest execution observed from 12:22:19 UTC, ended 12:30:13 UTC; 1187 passed, 1 failed, 45 ignored, exit 101 (workspace.log, workspace-start.txt, workspace-end.txt, workspace-exit.txt). The failure was the new runner temporary-root test using a nonprivate tempfile root. The test fixture was corrected to explicit mode 700, as was production runner task-directory creation; package then passed without weakening assertions. Coordinator requested no repeat of the entire workspace at this stage: final integrated gate remains theirs. The SQL agent's counted 50 repetitions, 12:22:53–12:24:29 UTC, were fully inside actual ordinary workspace test execution; coordinator retains their logs. Earlier pre-load repetitions do not count.

Operator command:

```console
cargo run --locked --offline -p connectors-build -- ignored --required --tmpdir $HOME/.cache/cb26b-runner/tmp --report .local/wave-20261002b/operator-disposable.json
```

Start 2026-10-02T12:43:02Z; end 2026-10-02T12:54:19Z; exit 1. operator-disposable.json has all 45 named outcomes; operator-disposable.log and operator-disposable.logs retain raw output. Missing prerequisite: connectors::local_cli::adversary2_a_real_pre_handshake_owner_is_refused_by_name_and_the_reverse_is_observed requires CONNECTORS_ADVERSARY_PRE_HANDSHAKE naming an executable. Failure: connectors-host::connectors_host::local::oauth_adversary_tests::chrome_opens_one_connection_per_navigation; unchanged test observed two empty Chrome connections instead of one. Exact log: operator-disposable.logs/ec2bbaaa1b2d53ae452f75a8a8b672336ee71d580ee003fcfaf1aa4b054b442a.log. All other 31 executed fixtures passed. No live/timing/deferred family was selected. This run used real dbus, qualified GNOME Keyring, browser, CLI and catalog binaries on the operator host.

5. Scope and limits

No planning writes, provider effects, workflow edits, gate enrollment, or inventoried-test edits. Coordinator subsequently authorized one local bot commit of exactly the five assigned files: Cargo.lock, crates/connectors-build/Cargo.toml, crates/connectors-build/src/main.rs, crates/connectors-build/src/ignored.rs and docs/development.md. No assertion was weakened for Chrome or the missing historical compatibility binary. Existing fixtures which deliberately detach into another session remain responsible for their own shutdown. Current inventory is workspace default features; unknown additional inventory refuses execution. Root must reconcile the one lock dependency entry with its planned version bump and run the final integrated gate. Detailed final review records are retained under adversary-2.

6. Outside-worktree paths and custody

Coordinator explicitly approved $HOME/.cache/cb26b-runner/tmp, including its parent $HOME/.cache/cb26b-runner, mode 700, to satisfy physical Unix socket custody paths in this long managed checkout. The supplied temporary root was empty after the operator run; coordinator owns its removal. Tool-managed worktree lease and Cargo/sccache caches are the only other outside-tree writes. Scratch, logs, fixture sources, test outputs and target/ remain inside this assigned managed worktree. Build output is approximately 5.2 GiB and is retained for coordinator cleanup. No managed tree or build directory was removed.

Verbatim initial red output:

```text
   Compiling connectors-build v0.24.0 ($HOME/.local/state/worktree/trees/b10x/connectors/cb26b-runner/crates/connectors-build)
warning: fields `package`, `target`, and `executable` are never read
  --> crates/connectors-build/src/ignored.rs:35:5
   |
34 | struct Suite {
   |        ----- fields in this struct
35 |     package: String,
   |     ^^^^^^^
36 |     target: String,
   |     ^^^^^^
37 |     executable: PathBuf,
   |     ^^^^^^^^^^
   |
   = note: `Suite` has derived impls for the traits `Debug` and `Clone`, but these are intentionally ignored during dead code analysis
   = note: `#[warn(dead_code)]` (part of `#[warn(unused)]`) on by default

warning: fields `family`, `helper`, and `prerequisites` are never read
  --> crates/connectors-build/src/ignored.rs:43:5
   |
40 | struct Entry {
   |        ----- fields in this struct
...
43 |     family: Option<Family>,
   |     ^^^^^^
44 |     helper: bool,
   |     ^^^^^^
45 |     prerequisites: Vec<&'static str>,
   |     ^^^^^^^^^^^^^
   |
   = note: `Entry` has derived impls for the traits `Debug` and `Clone`, but these are intentionally ignored during dead code analysis

warning: `connectors-build` (bin "connectors-build" test) generated 2 warnings
    Finished `test` profile [optimized] target(s) in 17.37s
     Running unittests src/main.rs (target/debug/deps/connectors_build-b336cbc3f2901527)

running 8 tests
test ignored::tests::ignored_runner_has_an_explicit_cli_and_defaults_to_disposable ... FAILED
test ignored::tests::missing_prerequisite_is_explicit ... FAILED
test ignored::tests::unknown_inventory_refuses_before_any_execution ... FAILED
test ignored::tests::selected_test_failure_is_nonzero ... FAILED
test ignored::tests::subprocess_helper_never_top_level ... FAILED
test ignored::tests::ignored_inventory_accounted ... FAILED
test ignored::tests::inventory_mode_never_executes_selected_cases ... FAILED
test ignored::tests::zero_test_success_is_not_execution ... FAILED

failures:

---- ignored::tests::ignored_runner_has_an_explicit_cli_and_defaults_to_disposable stdout ----

thread 'ignored::tests::ignored_runner_has_an_explicit_cli_and_defaults_to_disposable' (1018729) panicked at crates/connectors-build/src/ignored.rs:153:14:
the ignored suite runner is an independent CLI command: ErrorInner { kind: InvalidSubcommand, context: FlatMap { keys: [InvalidSubcommand, Usage], values: [String("ignored"), StyledStr(StyledStr("\u{1b}[1m\u{1b}[4mUsage:\u{1b}[0m \u{1b}[1mconnectors-build\u{1b}[0m [OPTIONS] <COMMAND>"))] }, message: None, source: None, help_flag: Some("--help"), styles: Styles { header: Style { fg: None, bg: None, underline: None, effects: Effects(BOLD | UNDERLINE) }, error: Style { fg: Some(Ansi(Red)), bg: None, underline: None, effects: Effects(BOLD) }, usage: Style { fg: None, bg: None, underline: None, effects: Effects(BOLD | UNDERLINE) }, literal: Style { fg: None, bg: None, underline: None, effects: Effects(BOLD) }, placeholder: Style { fg: None, bg: None, underline: None, effects: Effects() }, valid: Style { fg: Some(Ansi(Green)), bg: None, underline: None, effects: Effects() }, invalid: Style { fg: Some(Ansi(Yellow)), bg: None, underline: None, effects: Effects() }, context: Style { fg: None, bg: None, underline: None, effects: Effects() }, context_value: None }, color_when: Auto, color_help_when: Auto, backtrace: None }
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace

---- ignored::tests::missing_prerequisite_is_explicit stdout ----

thread 'ignored::tests::missing_prerequisite_is_explicit' (1018731) panicked at crates/connectors-build/src/ignored.rs:173:9:
assertion `left == right` failed
  left: (0, 0, 0)
 right: (1, 0, 1)

---- ignored::tests::unknown_inventory_refuses_before_any_execution stdout ----

thread 'ignored::tests::unknown_inventory_refuses_before_any_execution' (1018734) panicked at crates/connectors-build/src/ignored.rs:213:9:
assertion `left == right` failed
  left: 0
 right: 1

---- ignored::tests::selected_test_failure_is_nonzero stdout ----

thread 'ignored::tests::selected_test_failure_is_nonzero' (1018732) panicked at crates/connectors-build/src/ignored.rs:189:9:
assertion `left == right` failed
  left: (0, 0)
 right: (1, 1)

---- ignored::tests::subprocess_helper_never_top_level stdout ----

thread 'ignored::tests::subprocess_helper_never_top_level' (1018733) panicked at crates/connectors-build/src/ignored.rs:202:9:
assertion `left == right` failed
  left: 0
 right: 2

---- ignored::tests::ignored_inventory_accounted stdout ----

thread 'ignored::tests::ignored_inventory_accounted' (1018728) panicked at crates/connectors-build/src/ignored.rs:158:9:
assertion `left == right` failed
  left: (0, 0, 0, 0)
 right: (3, 1, 1, 2)

---- ignored::tests::inventory_mode_never_executes_selected_cases stdout ----

thread 'ignored::tests::inventory_mode_never_executes_selected_cases' (1018730) panicked at crates/connectors-build/src/ignored.rs:222:9:
assertion `left == right` failed
  left: (0, 0, 0)
 right: (3, 1, 0)

---- ignored::tests::zero_test_success_is_not_execution stdout ----

thread 'ignored::tests::zero_test_success_is_not_execution' (1018735) panicked at crates/connectors-build/src/ignored.rs:230:9:
assertion failed: !execute_one(&suite, "nonexistent", root.path()).unwrap()


failures:
    ignored::tests::ignored_inventory_accounted
    ignored::tests::ignored_runner_has_an_explicit_cli_and_defaults_to_disposable
    ignored::tests::inventory_mode_never_executes_selected_cases
    ignored::tests::missing_prerequisite_is_explicit
    ignored::tests::selected_test_failure_is_nonzero
    ignored::tests::subprocess_helper_never_top_level
    ignored::tests::unknown_inventory_refuses_before_any_execution
    ignored::tests::zero_test_success_is_not_execution

test result: FAILED. 0 passed; 8 failed; 0 ignored; 0 measured; 47 filtered out; finished in 0.45s

error: test failed, to rerun pass `-p connectors-build --bin connectors-build`
```

Verbatim package output after first adversary correction (`cargo test --locked --offline -p connectors-build`, exit 0):

```text
    Blocking waiting for file lock on package cache
    Finished `test` profile [optimized] target(s) in 3.67s
     Running unittests src/main.rs (target/debug/deps/connectors_build-4e0acae3b18f6d3b)

running 57 tests
test cli::tests::cache_labels_and_page_deadlines_cannot_assert_unobserved_freshness ... ok
test docs::tests::refuses_unknown_projection_nodes ... ok
test cli::tests::public_acquisition_examples_cannot_expose_internal_states_or_wrong_terminal_fields ... ok
test docs::tests::imported_markdown_cannot_execute_html_or_script_links ... ok
test docs::tests::cannot_publish_a_source_outside_its_owner ... ok
test docs::tests::reference_title_keeps_anchor_and_text_without_a_second_h1 ... ok
test aep_toolchain::tests::the_pin_is_one_exact_release ... ok
test docs::tests::preserves_rules_and_stable_heading_anchors ... ok
test docs::tests::resolves_only_selected_local_links ... ok
test ess_boundary::tests::admits_a_root_component_source_that_owns_only_declared_domains ... ok
test ess_boundary::tests::matches_case_and_separator_variants_without_substring_false_positives ... ok
test docs::tests::rejects_duplicate_and_escaping_routes ... ok
test docs::tests::public_audit_checks_embedded_binary_paths ... ok
test ess_boundary::tests::allows_shared_protocols_and_separate_provider_models ... ok
test ess_boundary::tests::protocol_adapter_does_not_ban_shared_protocol_or_suppress_native_leaks ... ok
test ess_boundary::tests::refuses_linked_source_files_and_directories ... ok
test ess_boundary::tests::refuses_a_component_source_owning_undeclared_domains_or_outside_the_root ... ok
test docs::tests::example_model_carries_the_example_domains_and_their_references_only ... ok
test gate::tests::gate_cargo_keeps_the_toolchain_selector_first ... ok
test gate::tests::gate_cargo_leaves_the_compiler_wrapper_on_the_callers_tmpdir ... ok
test ess_boundary::tests::discovers_design_only_and_spec_only_owners_without_runtime_declarations ... ok
test gate::adversary_pass2_tests::runner_passes_test_arguments_byte_for_byte ... ok
test ignored::tests::ignored_runner_has_an_explicit_cli_and_defaults_to_disposable ... ok
test gate::tests::gate_cargo_keys_the_runner_to_the_host_triple ... ok
test gate::adversary_tests::runner_hands_the_root_to_a_binary_whose_path_contains_an_equals_sign ... ok
test ess_boundary::tests::checks_all_paths_and_requires_complete_local_domain_inventory ... ok
test gate::adversary_tests::runner_executes_a_failing_test_binary_whose_path_contains_an_equals_sign ... ok
test ess_boundary::tests::rejects_malformed_optional_alias_inputs_and_model_sources ... ok
test metadata_conformance::tests::kernel_invariant_violation_is_a_failed_command_not_unsupported ... ok
test metadata_conformance::tests::named_optional_members_round_trip_through_the_runtime_form ... ok
test metadata_conformance::tests::other_kernel_errors_stay_unsupported ... ok
test metadata_conformance::tests::unknown_fixture_is_an_error_not_a_default ... ok
test source_hashes::tests::a_manifest_without_retained_archives_is_not_this_check ... ok
test gate::tests::gate_cargo_hands_the_temporary_root_to_test_binaries_through_a_runner ... ok
test gate::adversary_pass2_tests::runner_preserves_exit_codes_above_one ... ok
test gate::adversary_pass2_tests::runner_executes_awkward_binary_paths ... ok
test gate::adversary_pass2_tests::runner_hands_over_roots_with_a_leading_dash_or_newline ... ok
test gate::adversary_tests::runner_preserves_awkward_roots ... ok
test ess_boundary::tests::rejects_names_comments_fields_encoded_values_and_new_adapter_ids ... ok
test source_hashes::tests::recorded_digests_are_rederived_from_the_archived_bytes ... ok
test ignored::tests::ignored_inventory_accounted ... ok
test tests::catalog_without_derived_from_records_none ... ok
test tests::catalog_derived_from_mismatch_refused ... ok
test tests::discovery_writes_the_projection_and_its_record ... ok
test ignored::tests::adversary_passing_fixture_does_not_leave_running_descendants ... ok
test ignored::tests::subprocess_helper_never_top_level ... ok
test tests::catalog_records_derivation ... ok
test ignored::tests::missing_prerequisite_is_explicit ... ok
test docs::tests::walkthrough_fixtures_follow_the_shipped_selection ... ok
test ignored::tests::selected_test_failure_is_nonzero ... ok
test ignored::tests::unknown_inventory_refuses_before_any_execution ... ok
test ignored::tests::zero_test_success_is_not_execution ... ok
test ignored::tests::inventory_mode_never_executes_selected_cases ... ok
test ignored::tests::adversary_failing_fixture_does_not_leave_running_descendants ... ok
test gate::adversary_pass2_tests::runner_preserves_a_killing_signal ... ok
test docs::tests::example_model_synthesizes_for_every_example_target_with_the_pinned_ess ... ok
test metadata_conformance::tests::emitted_manifest_is_admitted_by_the_pinned_ess ... ok

test result: ok. 57 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 24.33s

     Running tests/cli_spec_mapping_adversary.rs (target/debug/deps/cli_spec_mapping_adversary-144de18fb6ce964b)

running 5 tests
test the_er_rs_citation_lands_on_the_cursor_expiry_path ... ok
test the_corrected_semantics_citations_land_on_their_rules ... ok
test the_owner_greeting_request_the_cli_sends_is_declared ... ok
test the_owner_greeting_reply_is_declared_with_the_hosts_fields ... ok
test the_build_digest_format_is_declared ... ok

test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s

     Running tests/cli_spec_mapping_adversary_pass2.rs (target/debug/deps/cli_spec_mapping_adversary_pass2-0ead9d1ec130e4b1)

running 5 tests
test the_build_digest_comment_cites_the_file_that_encodes_it ... ok
test the_owner_greeting_types_its_uuid_fields ... ok
test the_owner_hello_types_its_uuid_fields ... ok
test the_build_digest_admits_exactly_lowercase_64_hex ... ok
test the_owner_frame_tag_is_modelled_the_same_way_on_every_frame ... ok

test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s

     Running tests/mcp_domain_model_adversary.rs (target/debug/deps/mcp_domain_model_adversary-50919bcc7e7813ce)

running 3 tests
test the_one_stated_census_edge_agrees_with_the_field_that_realises_it ... ok
test the_adapter_owner_document_does_not_deny_the_model_this_unit_added ... ok
test a_binding_can_select_every_version_a_server_can_report_as_supported ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests/mcp_domain_model_adversary_pass2.rs (target/debug/deps/mcp_domain_model_adversary_pass2-214fe694069a776b)

running 3 tests
test every_relation_no_source_answers_carries_the_marker_the_acceptance_names ... ok
test a_census_rows_verdict_cannot_be_supplied_by_the_row_beside_it ... ok
test no_state_carrier_closes_a_set_the_pinned_schema_leaves_open ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests/mcp_domain_model_census.rs (target/debug/deps/mcp_domain_model_census-3bfc3659d9861070)

running 4 tests
test the_protocol_domain_still_disclaims_making_a_selection ... ok
test the_protocol_revision_values_are_the_negotiated_strings_not_the_trap_constants ... ok
test the_three_credential_kinds_are_three_distinct_declared_types ... ok
test every_census_edge_is_marked_and_no_unreadable_one_is_realised ... ok

test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s

     Running tests/mcp_inbound_local_binding.rs (target/debug/deps/mcp_inbound_local_binding-fbc738e613f1ddf6)

running 16 tests
test an_acceptance_that_states_both_a_count_and_a_rule_is_refused_rather_than_resolved - should panic ... ok
test each_scenario_performs_the_transition_its_behaviour_names ... ok
test the_single_principal_boundary_is_stated_and_no_scenario_crosses_it ... ok
test every_behaviour_states_what_the_selected_transport_does_not_offer ... ok
test the_acceptance_rule_is_read_from_the_statement_in_either_form_it_can_take ... ok
test no_trace_issues_a_lease_longer_than_the_contract_ceiling ... ok
test the_binding_names_no_cloud_identity_or_network_dependency_it_does_not_refuse ... ok
test the_document_specifies_the_transport_the_matrix_selected_and_selects_nothing ... ok
test every_transition_a_behaviour_section_asserts_is_bound_to_one_of_its_traces ... ok
test every_named_transition_is_one_the_sessions_lifecycle_declares ... ok
test the_document_states_the_lease_obligation_its_traces_inherit ... ok
test no_scenario_this_story_owns_names_an_outcome_the_document_does_not ... ok
test every_behaviour_the_acceptance_names_has_an_outcome_a_transition_and_a_scenario ... ok
test no_trace_admits_data_or_renews_after_its_live_lease_deadline ... ok
test every_close_deadline_a_trace_records_is_bounded_by_the_contract_and_the_live_lease ... ok
test the_directory_ships_what_the_story_acceptance_states ... ok

test result: ok. 16 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.04s

     Running tests/mcp_inbound_local_binding_adversary.rs (target/debug/deps/mcp_inbound_local_binding_adversary-0eaf0b757a9878fb)

running 3 tests
test no_data_act_is_admitted_after_the_lease_that_admits_it_can_legally_be_live ... ok
test no_close_records_a_cutoff_later_than_the_lease_it_ends ... ok
test no_lease_a_trace_issues_outlives_the_ceiling_the_sessions_contract_sets ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s

     Running tests/mcp_inbound_local_binding_adversary_pass2.rs (target/debug/deps/mcp_inbound_local_binding_adversary_pass2-f7e00cfa879ea3e0)

running 2 tests
test the_story_ships_the_number_of_scenario_files_its_acceptance_states ... ok
test no_act_that_needs_a_live_lease_is_admitted_at_or_after_its_effective_expiry ... ok

test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests/mcp_outbound_connection_lifecycle.rs (target/debug/deps/mcp_outbound_connection_lifecycle-fad87bc9b8c3c1dc)

running 11 tests
test every_refusal_names_a_declared_wire_code_and_loss_is_not_failure ... ok
test every_refusal_names_whose_act_it_reports ... ok
test the_document_does_not_close_the_revision_set_the_model_leaves_open ... ok
test outbound_stdio_is_held_by_its_blocker_and_nothing_else_is ... ok
test no_state_is_answered_by_sending_the_request_again ... ok
test no_citation_into_an_editable_document_carries_a_line_number ... ok
test every_scenario_this_story_owns_names_a_row_of_the_register ... ok
test every_state_the_story_names_has_a_named_outcome_and_a_scenario_file ... ok
test every_citation_resolves_into_an_archived_file_of_the_pin ... ok
test every_section_and_scenario_resolves_which_revision_it_holds_for ... ok
test no_sentence_writes_a_field_of_the_binding_entity ... ok

test result: ok. 11 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.87s

     Running tests/mcp_outbound_connection_lifecycle_adversary.rs (target/debug/deps/mcp_outbound_connection_lifecycle_adversary-6159e1bccc3da86a)

running 6 tests
test a_selection_names_as_many_revisions_as_the_outcome_and_the_model_can_hold ... ok
test the_document_does_not_call_a_four_family_transport_space_a_pair ... ok
test an_answer_that_was_never_streamed_can_also_be_lost ... ok
test cancellation_says_which_revision_its_signal_holds_for ... ok
test a_request_the_interoperability_revision_makes_this_client_answer_has_an_outcome ... ok
test no_binding_field_is_both_unchanged_by_an_observation_and_written_from_one ... ok

test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s

     Running tests/mcp_outbound_connection_lifecycle_adversary_pass2.rs (target/debug/deps/mcp_outbound_connection_lifecycle_adversary_pass2-344a9ab73be884bf)

running 5 tests
test a_refusal_does_not_name_the_peer_for_a_reason_in_which_no_peer_was_seen ... ok
test a_state_does_not_carry_two_outcomes_that_deny_each_other_s_precondition ... ok
test the_selection_scenario_states_a_framing_the_interoperability_revision_has_not ... ok
test the_advertised_capability_scenario_reads_a_result_the_interoperability_revision_has_not ... ok
test no_sentence_of_this_contract_writes_a_field_of_the_binding_entity ... ok

test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.37s

     Running tests/mcp_profile_selection_matrix.rs (target/debug/deps/mcp_profile_selection_matrix-fbf40d70ffaa53bb)

running 12 tests
test every_deferred_row_inheriting_the_other_sides_marker_is_stated_as_inheriting_it ... ok
test no_row_resting_on_an_open_decision_blocker_claims_support_or_refusal ... ok
test the_matrix_states_that_a_supported_row_is_a_selection_not_implemented_support ... ok
test every_deferred_row_names_a_record_and_every_quoted_marker_exists_in_the_model ... ok
test no_revision_outside_the_pin_is_dispositioned_supported ... ok
test each_pinned_schema_declares_exactly_two_capability_interfaces_with_optional_fields ... ok
test every_nested_capability_setting_is_named_in_the_section_of_the_side_that_declares_it ... ok
test an_inbound_header_refusal_excepts_every_supported_revision_whose_opener_carries_none ... ok
test each_row_carries_a_named_disposition_a_reason_and_a_resolvable_source_line ... ok
test every_revision_exclusion_is_stated_by_the_matrix_and_still_removes_a_token ... ok
test every_extension_the_pinned_revisions_identify_is_named_by_the_matrix ... ok
test every_feature_the_pinned_specification_names_is_dispositioned_once_per_direction ... ok

test result: ok. 12 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.10s

     Running tests/mcp_profile_selection_matrix_adversary.rs (target/debug/deps/mcp_profile_selection_matrix_adversary-4647c5a48416dc42)

running 3 tests
test every_deferred_row_rests_on_a_blocker_whose_own_record_names_that_feature ... ok
test every_nested_setting_is_named_in_the_section_of_the_side_whose_schema_declares_it ... ok
test every_protocol_version_the_archives_tell_a_peer_to_assume_is_dispositioned ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.14s

     Running tests/mcp_profile_selection_matrix_adversary_pass2.rs (target/debug/deps/mcp_profile_selection_matrix_adversary_pass2-08892ff74f99b045)

running 2 tests
test the_inbound_refusal_of_a_header_less_request_excepts_the_legacy_initialize_it_supports ... ok
test every_extension_the_pinned_revisions_identify_is_named_by_the_matrix ... ok

test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.11s

     Running tests/mcp_specification_pin_adversary.rs (target/debug/deps/mcp_specification_pin_adversary-a105d50d7a2877c2)

running 4 tests
test manifest_provenance_fields_agree_with_the_revision_each_entry_claims ... ok
test version_negotiation_row_cites_the_interop_revisions_negotiation_section ... ok
test interop_revision_names_its_own_version_string_outside_a_documentation_path ... ok
test every_recorded_digest_and_byte_length_rederives_from_the_archived_bytes ... ok

test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.09s

     Running tests/mcp_specification_pin_adversary_pass2.rs (target/debug/deps/mcp_specification_pin_adversary_pass2-93e6fffabbad4cca)

running 4 tests
test record_does_not_count_a_banner_document_twice_when_naming_running_text_mentions ... ok
test documented_provenance_check_refuses_a_path_repointed_within_its_own_revision ... ok
test documented_provenance_check_refuses_a_wholesale_revision_to_commit_swap ... ok
test design_document_accounts_for_every_transport_binding_the_pinned_revisions_specify ... ok

test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.21s

     Running tests/metadata_conformance_fixture_adversary.rs (target/debug/deps/metadata_conformance_fixture_adversary-4583e554883aaf53)

running 5 tests
test an_unobservable_invariant_is_failed_as_a_violation_is ... ok
test an_invariant_violation_is_failed_through_the_runner ... ok
test a_well_formed_anchor_passes ... ok
test an_unknown_fixture_name_errors_the_run ... ok
test a_planted_wrong_outcome_fails_the_run ... ok

test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.87s

```

Final handoff checks: package exit 0, Clippy all-targets with -D warnings exit 0, package formatting check exit 0 (package-handoff.exit, clippy-handoff.exit, fmt-handoff.exit). Final source fingerprint for ignored.rs: a0053c69779a7b00e369cde7e18914f189e639023933baee375f88e0cdc04d85, identical to the reviewed green guard-drop state. Staged diff: exactly five files, 1084 insertions, including the 1017-line new module.

Verbatim final package command output (`cargo test --locked --offline -p connectors-build`, exit 0):

```text
   Compiling connectors-build v0.24.0 ($HOME/.local/state/worktree/trees/b10x/connectors/cb26b-runner/crates/connectors-build)
    Finished `test` profile [optimized] target(s) in 12.12s
     Running unittests src/main.rs (target/debug/deps/connectors_build-4e0acae3b18f6d3b)

running 58 tests
test cli::tests::public_acquisition_examples_cannot_expose_internal_states_or_wrong_terminal_fields ... ok
test aep_toolchain::tests::the_pin_is_one_exact_release ... ok
test cli::tests::cache_labels_and_page_deadlines_cannot_assert_unobserved_freshness ... ok
test docs::tests::refuses_unknown_projection_nodes ... ok
test docs::tests::cannot_publish_a_source_outside_its_owner ... ok
test docs::tests::rejects_duplicate_and_escaping_routes ... ok
test ess_boundary::tests::matches_case_and_separator_variants_without_substring_false_positives ... ok
test docs::tests::preserves_rules_and_stable_heading_anchors ... ok
test docs::tests::imported_markdown_cannot_execute_html_or_script_links ... ok
test docs::tests::reference_title_keeps_anchor_and_text_without_a_second_h1 ... ok
test docs::tests::resolves_only_selected_local_links ... ok
test ess_boundary::tests::allows_shared_protocols_and_separate_provider_models ... ok
test ess_boundary::tests::admits_a_root_component_source_that_owns_only_declared_domains ... ok
test docs::tests::public_audit_checks_embedded_binary_paths ... ok
test ess_boundary::tests::protocol_adapter_does_not_ban_shared_protocol_or_suppress_native_leaks ... ok
test ess_boundary::tests::discovers_design_only_and_spec_only_owners_without_runtime_declarations ... ok
test ess_boundary::tests::rejects_malformed_optional_alias_inputs_and_model_sources ... ok
test ess_boundary::tests::refuses_linked_source_files_and_directories ... ok
test ess_boundary::tests::checks_all_paths_and_requires_complete_local_domain_inventory ... ok
test ess_boundary::tests::refuses_a_component_source_owning_undeclared_domains_or_outside_the_root ... ok
test gate::tests::gate_cargo_leaves_the_compiler_wrapper_on_the_callers_tmpdir ... ok
test gate::tests::gate_cargo_keeps_the_toolchain_selector_first ... ok
test ignored::tests::ignored_runner_has_an_explicit_cli_and_defaults_to_disposable ... ok
test gate::adversary_tests::runner_hands_the_root_to_a_binary_whose_path_contains_an_equals_sign ... ok
test gate::adversary_pass2_tests::runner_passes_test_arguments_byte_for_byte ... ok
test ignored::tests::adversary_error_drops_live_group_without_reaping_another_fixture ... ok
test gate::tests::gate_cargo_keys_the_runner_to_the_host_triple ... ok
test gate::adversary_pass2_tests::runner_preserves_exit_codes_above_one ... ok
test gate::adversary_pass2_tests::runner_executes_awkward_binary_paths ... ok
test metadata_conformance::tests::kernel_invariant_violation_is_a_failed_command_not_unsupported ... ok
test metadata_conformance::tests::named_optional_members_round_trip_through_the_runtime_form ... ok
test metadata_conformance::tests::other_kernel_errors_stay_unsupported ... ok
test metadata_conformance::tests::unknown_fixture_is_an_error_not_a_default ... ok
test gate::adversary_tests::runner_executes_a_failing_test_binary_whose_path_contains_an_equals_sign ... ok
test gate::adversary_pass2_tests::runner_hands_over_roots_with_a_leading_dash_or_newline ... ok
test source_hashes::tests::a_manifest_without_retained_archives_is_not_this_check ... ok
test gate::tests::gate_cargo_hands_the_temporary_root_to_test_binaries_through_a_runner ... ok
test gate::adversary_tests::runner_preserves_awkward_roots ... ok
test ess_boundary::tests::rejects_names_comments_fields_encoded_values_and_new_adapter_ids ... ok
test docs::tests::walkthrough_fixtures_follow_the_shipped_selection ... ok
test source_hashes::tests::recorded_digests_are_rederived_from_the_archived_bytes ... ok
test tests::discovery_writes_the_projection_and_its_record ... ok
test tests::catalog_derived_from_mismatch_refused ... ok
test docs::tests::example_model_carries_the_example_domains_and_their_references_only ... ok
test tests::catalog_without_derived_from_records_none ... ok
test ignored::tests::inventory_mode_never_executes_selected_cases ... ok
test tests::catalog_records_derivation ... ok
test ignored::tests::unknown_inventory_refuses_before_any_execution ... ok
test ignored::tests::ignored_inventory_accounted ... ok
test ignored::tests::missing_prerequisite_is_explicit ... ok
test ignored::tests::zero_test_success_is_not_execution ... ok
test ignored::tests::selected_test_failure_is_nonzero ... ok
test ignored::tests::subprocess_helper_never_top_level ... ok
test ignored::tests::adversary_failing_fixture_does_not_leave_running_descendants ... ok
test ignored::tests::adversary_passing_fixture_does_not_leave_running_descendants ... ok
test gate::adversary_pass2_tests::runner_preserves_a_killing_signal ... ok
test docs::tests::example_model_synthesizes_for_every_example_target_with_the_pinned_ess ... ok
test metadata_conformance::tests::emitted_manifest_is_admitted_by_the_pinned_ess ... ok

test result: ok. 58 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 9.05s

     Running tests/cli_spec_mapping_adversary.rs (target/debug/deps/cli_spec_mapping_adversary-144de18fb6ce964b)

running 5 tests
test the_corrected_semantics_citations_land_on_their_rules ... ok
test the_er_rs_citation_lands_on_the_cursor_expiry_path ... ok
test the_build_digest_format_is_declared ... ok
test the_owner_greeting_request_the_cli_sends_is_declared ... ok
test the_owner_greeting_reply_is_declared_with_the_hosts_fields ... ok

test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s

     Running tests/cli_spec_mapping_adversary_pass2.rs (target/debug/deps/cli_spec_mapping_adversary_pass2-0ead9d1ec130e4b1)

running 5 tests
test the_owner_greeting_types_its_uuid_fields ... ok
test the_owner_hello_types_its_uuid_fields ... ok
test the_build_digest_admits_exactly_lowercase_64_hex ... ok
test the_build_digest_comment_cites_the_file_that_encodes_it ... ok
test the_owner_frame_tag_is_modelled_the_same_way_on_every_frame ... ok

test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s

     Running tests/mcp_domain_model_adversary.rs (target/debug/deps/mcp_domain_model_adversary-50919bcc7e7813ce)

running 3 tests
test the_one_stated_census_edge_agrees_with_the_field_that_realises_it ... ok
test a_binding_can_select_every_version_a_server_can_report_as_supported ... ok
test the_adapter_owner_document_does_not_deny_the_model_this_unit_added ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests/mcp_domain_model_adversary_pass2.rs (target/debug/deps/mcp_domain_model_adversary_pass2-214fe694069a776b)

running 3 tests
test every_relation_no_source_answers_carries_the_marker_the_acceptance_names ... ok
test a_census_rows_verdict_cannot_be_supplied_by_the_row_beside_it ... ok
test no_state_carrier_closes_a_set_the_pinned_schema_leaves_open ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests/mcp_domain_model_census.rs (target/debug/deps/mcp_domain_model_census-3bfc3659d9861070)

running 4 tests
test the_protocol_domain_still_disclaims_making_a_selection ... ok
test the_protocol_revision_values_are_the_negotiated_strings_not_the_trap_constants ... ok
test the_three_credential_kinds_are_three_distinct_declared_types ... ok
test every_census_edge_is_marked_and_no_unreadable_one_is_realised ... ok

test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s

     Running tests/mcp_inbound_local_binding.rs (target/debug/deps/mcp_inbound_local_binding-fbc738e613f1ddf6)

running 16 tests
test the_binding_names_no_cloud_identity_or_network_dependency_it_does_not_refuse ... ok
test the_document_specifies_the_transport_the_matrix_selected_and_selects_nothing ... ok
test every_named_transition_is_one_the_sessions_lifecycle_declares ... ok
test every_transition_a_behaviour_section_asserts_is_bound_to_one_of_its_traces ... ok
test each_scenario_performs_the_transition_its_behaviour_names ... ok
test every_behaviour_states_what_the_selected_transport_does_not_offer ... ok
test the_directory_ships_what_the_story_acceptance_states ... ok
test the_acceptance_rule_is_read_from_the_statement_in_either_form_it_can_take ... ok
test no_scenario_this_story_owns_names_an_outcome_the_document_does_not ... ok
test an_acceptance_that_states_both_a_count_and_a_rule_is_refused_rather_than_resolved - should panic ... ok
test the_single_principal_boundary_is_stated_and_no_scenario_crosses_it ... ok
test the_document_states_the_lease_obligation_its_traces_inherit ... ok
test every_behaviour_the_acceptance_names_has_an_outcome_a_transition_and_a_scenario ... ok
test no_trace_issues_a_lease_longer_than_the_contract_ceiling ... ok
test every_close_deadline_a_trace_records_is_bounded_by_the_contract_and_the_live_lease ... ok
test no_trace_admits_data_or_renews_after_its_live_lease_deadline ... ok

test result: ok. 16 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s

     Running tests/mcp_inbound_local_binding_adversary.rs (target/debug/deps/mcp_inbound_local_binding_adversary-0eaf0b757a9878fb)

running 3 tests
test no_lease_a_trace_issues_outlives_the_ceiling_the_sessions_contract_sets ... ok
test no_close_records_a_cutoff_later_than_the_lease_it_ends ... ok
test no_data_act_is_admitted_after_the_lease_that_admits_it_can_legally_be_live ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s

     Running tests/mcp_inbound_local_binding_adversary_pass2.rs (target/debug/deps/mcp_inbound_local_binding_adversary_pass2-f7e00cfa879ea3e0)

running 2 tests
test the_story_ships_the_number_of_scenario_files_its_acceptance_states ... ok
test no_act_that_needs_a_live_lease_is_admitted_at_or_after_its_effective_expiry ... ok

test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s

     Running tests/mcp_outbound_connection_lifecycle.rs (target/debug/deps/mcp_outbound_connection_lifecycle-fad87bc9b8c3c1dc)

running 11 tests
test every_refusal_names_a_declared_wire_code_and_loss_is_not_failure ... ok
test the_document_does_not_close_the_revision_set_the_model_leaves_open ... ok
test every_state_the_story_names_has_a_named_outcome_and_a_scenario_file ... ok
test outbound_stdio_is_held_by_its_blocker_and_nothing_else_is ... ok
test every_refusal_names_whose_act_it_reports ... ok
test every_scenario_this_story_owns_names_a_row_of_the_register ... ok
test no_citation_into_an_editable_document_carries_a_line_number ... ok
test no_state_is_answered_by_sending_the_request_again ... ok
test every_citation_resolves_into_an_archived_file_of_the_pin ... ok
test every_section_and_scenario_resolves_which_revision_it_holds_for ... ok
test no_sentence_writes_a_field_of_the_binding_entity ... ok

test result: ok. 11 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.01s

     Running tests/mcp_outbound_connection_lifecycle_adversary.rs (target/debug/deps/mcp_outbound_connection_lifecycle_adversary-6159e1bccc3da86a)

running 6 tests
test a_request_the_interoperability_revision_makes_this_client_answer_has_an_outcome ... ok
test a_selection_names_as_many_revisions_as_the_outcome_and_the_model_can_hold ... ok
test the_document_does_not_call_a_four_family_transport_space_a_pair ... ok
test cancellation_says_which_revision_its_signal_holds_for ... ok
test an_answer_that_was_never_streamed_can_also_be_lost ... ok
test no_binding_field_is_both_unchanged_by_an_observation_and_written_from_one ... ok

test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s

     Running tests/mcp_outbound_connection_lifecycle_adversary_pass2.rs (target/debug/deps/mcp_outbound_connection_lifecycle_adversary_pass2-344a9ab73be884bf)

running 5 tests
test a_state_does_not_carry_two_outcomes_that_deny_each_other_s_precondition ... ok
test a_refusal_does_not_name_the_peer_for_a_reason_in_which_no_peer_was_seen ... ok
test the_selection_scenario_states_a_framing_the_interoperability_revision_has_not ... ok
test the_advertised_capability_scenario_reads_a_result_the_interoperability_revision_has_not ... ok
test no_sentence_of_this_contract_writes_a_field_of_the_binding_entity ... ok

test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.26s

     Running tests/mcp_profile_selection_matrix.rs (target/debug/deps/mcp_profile_selection_matrix-fbf40d70ffaa53bb)

running 12 tests
test the_matrix_states_that_a_supported_row_is_a_selection_not_implemented_support ... ok
test no_row_resting_on_an_open_decision_blocker_claims_support_or_refusal ... ok
test no_revision_outside_the_pin_is_dispositioned_supported ... ok
test every_nested_capability_setting_is_named_in_the_section_of_the_side_that_declares_it ... ok
test every_deferred_row_names_a_record_and_every_quoted_marker_exists_in_the_model ... ok
test each_pinned_schema_declares_exactly_two_capability_interfaces_with_optional_fields ... ok
test every_deferred_row_inheriting_the_other_sides_marker_is_stated_as_inheriting_it ... ok
test an_inbound_header_refusal_excepts_every_supported_revision_whose_opener_carries_none ... ok
test each_row_carries_a_named_disposition_a_reason_and_a_resolvable_source_line ... ok
test every_revision_exclusion_is_stated_by_the_matrix_and_still_removes_a_token ... ok
test every_extension_the_pinned_revisions_identify_is_named_by_the_matrix ... ok
test every_feature_the_pinned_specification_names_is_dispositioned_once_per_direction ... ok

test result: ok. 12 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.06s

     Running tests/mcp_profile_selection_matrix_adversary.rs (target/debug/deps/mcp_profile_selection_matrix_adversary-4647c5a48416dc42)

running 3 tests
test every_nested_setting_is_named_in_the_section_of_the_side_whose_schema_declares_it ... ok
test every_deferred_row_rests_on_a_blocker_whose_own_record_names_that_feature ... ok
test every_protocol_version_the_archives_tell_a_peer_to_assume_is_dispositioned ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.04s

     Running tests/mcp_profile_selection_matrix_adversary_pass2.rs (target/debug/deps/mcp_profile_selection_matrix_adversary_pass2-08892ff74f99b045)

running 2 tests
test the_inbound_refusal_of_a_header_less_request_excepts_the_legacy_initialize_it_supports ... ok
test every_extension_the_pinned_revisions_identify_is_named_by_the_matrix ... ok

test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.04s

     Running tests/mcp_specification_pin_adversary.rs (target/debug/deps/mcp_specification_pin_adversary-a105d50d7a2877c2)

running 4 tests
test version_negotiation_row_cites_the_interop_revisions_negotiation_section ... ok
test manifest_provenance_fields_agree_with_the_revision_each_entry_claims ... ok
test interop_revision_names_its_own_version_string_outside_a_documentation_path ... ok
test every_recorded_digest_and_byte_length_rederives_from_the_archived_bytes ... ok

test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.26s

     Running tests/mcp_specification_pin_adversary_pass2.rs (target/debug/deps/mcp_specification_pin_adversary_pass2-93e6fffabbad4cca)

running 4 tests
test record_does_not_count_a_banner_document_twice_when_naming_running_text_mentions ... ok
test documented_provenance_check_refuses_a_path_repointed_within_its_own_revision ... ok
test documented_provenance_check_refuses_a_wholesale_revision_to_commit_swap ... ok
test design_document_accounts_for_every_transport_binding_the_pinned_revisions_specify ... ok

test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s

     Running tests/metadata_conformance_fixture_adversary.rs (target/debug/deps/metadata_conformance_fixture_adversary-4583e554883aaf53)

running 5 tests
test an_unknown_fixture_name_errors_the_run ... ok
test a_well_formed_anchor_passes ... ok
test an_invariant_violation_is_failed_through_the_runner ... ok
test a_planted_wrong_outcome_fails_the_run ... ok
test an_unobservable_invariant_is_failed_as_a_violation_is ... ok

test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.40s

```

The publication report applies only a literal home-directory-prefix replacement with `$HOME`. All commands, counts, failures and test output otherwise remain unchanged; the raw report is retained separately.

Authorized local bot commit: a2f408dbed07e2558a11db3bc38e4eac70fa1ea7 (`feat(build): account for and run classified ignored suites`). Both author and committer are b10x-bot[bot] <316511680+b10x-bot[bot]@users.noreply.github.com>. Exactly the five assigned files were staged and committed; tracked working tree is clean. The first bot CLI invocation included an unnecessary `git` token and was rejected before mutation; the corrected documented subcommand succeeded (commit.log, commit-final.log). No hook or policy was bypassed. No push was performed.

Own worktree lease codex-cb26b-runner released at handoff; managed worktree, target, source and evidence retained for coordinator integration and cleanup.
