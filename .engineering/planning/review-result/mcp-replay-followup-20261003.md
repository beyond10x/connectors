---
format: aep.planning-md/3
id: review-result:mcp-replay-followup-20261003
kind: review-result
status: active
title: 'Replay final adversary: observation premise erasure rejected'
relations:
- reviews: story:mcp-inbound-mutation-replay
revision: 1
---
unit: story:mcp-inbound-mutation-replay — corrected cb26i-replay over b360aaaa771ad9a31cab46ff4249015b74a4d50f
verdict: nothing found
cases: executed 40→41, red 0
origin: introduced 0 / pre-existing 0 / undecided 0
wrote-outside-worktree: none
needs-coordinator: integrate retained tests and record final bounded review; no third campaign

## 1. Diff and source identity

`git --no-pager diff --stat` is empty because the three source files remain
untracked, inherited from the author. Reviewer delta against the frozen corrected
test file is append-only: one Rust test, 63 lines, in
crates/connectors-build/tests/mcp_inbound_mutation_replay.rs. Exact own-test.diff
and baseline.rs are retained beside this report. No inherited assertion or
function changed; both contract inputs are unchanged. The corrected author report
SHA256 is ea1a5a63e2228d9288c9ff1be7750a6df49649f494b81720f1c25e99d643ec86.

Before guard hash:
1382c020e8422f98be0ce92f1e3f2db43a2bba446880568d2295297d22036a26.
Final source SHA256:

```text
1f7884ccc7ededac8cef0173e18b04b0182fead6bb8b912caa59bdd348175fd3  adapters/mcp/contracts/server/v1alpha1/mutations.md
6ce39a6e9eef009ad91b9918e236838f70d381d0852a6fa95208c286f59c7e95  adapters/mcp/contracts/server/v1alpha1/mutation-cases.json
a85785382d0fbd83dfc65bd9e621c5d047d0386d12e8224062ac4c545af75a9b  crates/connectors-build/tests/mcp_inbound_mutation_replay.rs
```

## 2. One adjacent attack written before execution

`adversary_second_observer_knowledge_cannot_be_self_consistently_rewritten`
changes each of the six observer cases twice: once its available effect evidence,
once its terminal durability. It recomputes classification, code, replayability
and Markdown together. The guard must reject each copy specifically for missing
required observation premises, rather than an incidental correspondence mismatch.
Original input remains a positive control. All twelve copied-input mutations are
rejected; this is one executed Rust test, not twelve conformance scenarios.

The first compile attempt had a reviewer borrow-check typo (E0502), retained as
compile-attempt.log/exit. It executed no test and is not a finding or a red case.
Only the new test was corrected to end its mutable borrow before calling check.
First actual deciding execution, exit 0:

`CARGO_BUILD_JOBS=2 RUSTC_WRAPPER=/usr/bin/sccache cargo test --locked -p connectors-build --test mcp_inbound_mutation_replay adversary_second_observer_knowledge_cannot_be_self_consistently_rewritten -- --exact --nocapture`

Verbatim test output, compilation preamble retained privately:

```text
running 1 test
test adversary_second_observer_knowledge_cannot_be_self_consistently_rewritten ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 9 filtered out; finished in 0.02s

```

## 3. Affected suite after attack

Before count 40 is the frozen correction report's measured count. No pre-attack
suite was run. Command, exit 0:

`CARGO_BUILD_JOBS=2 RUSTC_WRAPPER=/usr/bin/sccache cargo test --locked -p connectors-build --no-fail-fast --test mcp_inbound_mutation_replay --test mcp_inbound_capability_projection --test mcp_inbound_local_binding --test mcp_inbound_local_binding_adversary --test mcp_inbound_local_binding_adversary_pass2`

Verbatim output:

```text
    Finished `test` profile [optimized] target(s) in 0.22s
     Running tests/mcp_inbound_capability_projection.rs (target/debug/deps/mcp_inbound_capability_projection-ab521dfd97d6ce13)

running 10 tests
test checker_rejects_ambiguous_names_and_invalid_inverse_spellings ... ok
test checker_rejects_weak_family_projection ... ok
test authored_projection_matches_closed_errors_and_named_cases ... ok
test adversary_rejects_contradictory_duplicate_visibility_row ... ok
test checker_rejects_missing_and_duplicate_error_rows ... ok
test checker_rejects_visibility_and_precedence_contradictions ... ok
test complete_unique_tables_remain_valid_after_row_reordering ... ok
test adversary_rejects_nonvisible_and_blockquoted_correspondence_tables ... ok
test every_authored_table_rejects_missing_unknown_rows_and_duplicate_tables ... ok
test every_authored_table_rejects_duplicate_or_contradictory_rows ... ok

test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.04s

     Running tests/mcp_inbound_local_binding.rs (target/debug/deps/mcp_inbound_local_binding-3c845919adb9283c)

running 16 tests
test the_binding_names_no_cloud_identity_or_network_dependency_it_does_not_refuse ... ok
test the_document_specifies_the_transport_the_matrix_selected_and_selects_nothing ... ok
test the_single_principal_boundary_is_stated_and_no_scenario_crosses_it ... ok
test an_acceptance_that_states_both_a_count_and_a_rule_is_refused_rather_than_resolved - should panic ... ok
test the_directory_ships_what_the_story_acceptance_states ... ok
test the_acceptance_rule_is_read_from_the_statement_in_either_form_it_can_take ... ok
test no_scenario_this_story_owns_names_an_outcome_the_document_does_not ... ok
test the_document_states_the_lease_obligation_its_traces_inherit ... ok
test every_behaviour_states_what_the_selected_transport_does_not_offer ... ok
test every_named_transition_is_one_the_sessions_lifecycle_declares ... ok
test every_behaviour_the_acceptance_names_has_an_outcome_a_transition_and_a_scenario ... ok
test every_transition_a_behaviour_section_asserts_is_bound_to_one_of_its_traces ... ok
test no_trace_issues_a_lease_longer_than_the_contract_ceiling ... ok
test every_close_deadline_a_trace_records_is_bounded_by_the_contract_and_the_live_lease ... ok
test each_scenario_performs_the_transition_its_behaviour_names ... ok
test no_trace_admits_data_or_renews_after_its_live_lease_deadline ... ok

test result: ok. 16 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s

     Running tests/mcp_inbound_local_binding_adversary.rs (target/debug/deps/mcp_inbound_local_binding_adversary-13d0e6a00555fd53)

running 3 tests
test no_lease_a_trace_issues_outlives_the_ceiling_the_sessions_contract_sets ... ok
test no_data_act_is_admitted_after_the_lease_that_admits_it_can_legally_be_live ... ok
test no_close_records_a_cutoff_later_than_the_lease_it_ends ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s

     Running tests/mcp_inbound_local_binding_adversary_pass2.rs (target/debug/deps/mcp_inbound_local_binding_adversary_pass2-c17fbdbd53c42577)

running 2 tests
test the_story_ships_the_number_of_scenario_files_its_acceptance_states ... ok
test no_act_that_needs_a_live_lease_is_admitted_at_or_after_its_effective_expiry ... ok

test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s

     Running tests/mcp_inbound_mutation_replay.rs (target/debug/deps/mcp_inbound_mutation_replay-67e2a2626349b7e6)

running 10 tests
test adversary_retained_admission_revision_and_winner_precedence ... ok
test adversary_recovery_case_must_exercise_recovery_observer ... ok
test authored_replay_contract_matches_existing_owners ... ok
test adversary_named_replay_cases_cannot_be_replaced_by_policy_denials ... ok
test copied_errors_and_correlation_cannot_erase_uncertainty ... ok
test copied_observations_preserve_unknown_and_applied_with_error ... ok
test adversary_second_observer_knowledge_cannot_be_self_consistently_rewritten ... ok
test copied_lifecycle_and_coordinate_inventories_remain_closed ... ok
test copied_cases_cannot_grant_replay_dispatch_or_bypass_admission ... ok
test every_document_table_rejects_extra_missing_and_contradictory_rows ... ok

test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.31s

```

Total 41 passed, 0 failed, 0 ignored. Both unchanged first-pass red tests now pass.
`cargo fmt -p connectors-build --check` exited 0.
`CARGO_BUILD_JOBS=2 RUSTC_WRAPPER=/usr/bin/sccache cargo clippy --locked -p connectors-build --test mcp_inbound_mutation_replay -- -D warnings` exited 0.

## 4. Findings

Nothing found in this final bounded attack; no remaining or additional finding.

## 5. Attack boundary

The corrected guard rejects erased observer evidence and durability even when
copied output/table values agree. This pass adds no MCP runtime, ledger recovery,
mutation advertisement or provider execution evidence. The first-pass failures
remain recorded in their immutable report; this report does not rewrite them.

## 6. Paths and handoff

No authored path outside the managed tree. Assigned scratch is
.local/mcp-two-waves/replay-review-second, holding baseline.rs/sha256,
own-test.diff, compile-attempt.log/exit, first-attack.log/exit, suite.log/exit,
fmt.log/exit, clippy.log/exit and this report. Existing tree-local target and
sccache were used with two jobs. Observed free space was 22,936,182,784 bytes,
above 8 GiB. No cleanup, commit, publication or AEP mutation was performed.
Reviewer lease released; coordinator owns integration and cleanup.

```findings
[]
```
