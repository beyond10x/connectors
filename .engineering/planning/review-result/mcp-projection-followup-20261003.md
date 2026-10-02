---
format: aep.planning-md/3
id: review-result:mcp-projection-followup-20261003
kind: review-result
status: active
title: Bounded follow-up review of corrected inbound projection correspondence
relations:
- reviews: story:mcp-inbound-capability-projection
revision: 1
---
unit: story:mcp-inbound-capability-projection — corrected working tree on a2955675cb70b5811589ce86b40be95427ead888
verdict: nothing found
cases: executed 30→31, red 0
origin: introduced 0 / pre-existing 0 / undecided 0
wrote-outside-worktree: none
needs-coordinator: none

## 1. Review-only diff

`git --no-pager diff --stat` is empty: the three inherited candidate files remain
untracked. Against the exact corrected handoff checker, this pass adds only:

```text
 crates/connectors-build/tests/mcp_inbound_capability_projection.rs | 22 ++++++++++++++++++++++
 1 file changed, 22 insertions(+)
```

Exact no-index stat and patch are retained under this report's directory. The
contract and JSON remain unchanged. No checker logic or existing case changed;
no AEP write, commit, source-document mutation or external publication occurred.
The first review remains byte-identical at SHA256
bffc067e6a0dd502dcdb01c7458ff5b7180f8426375841b2924e16fba207c85e.

## 2. New nearby case before execution

Added `adversary_rejects_nonvisible_and_blockquoted_correspondence_tables`.
For each of the four correspondence tables, its in-memory document copy either
moves the required table into a fenced code block or appends a duplicate table
inside a blockquote. The former preserves raw source rows while removing the
rendered table; the latter tests nested rendered-table inventory. Both must be
rejected by the actual checker. The documents on disk are never modified.

First execution, after writing the case:
`cargo test --locked -p connectors-build --test mcp_inbound_capability_projection adversary_rejects_nonvisible_and_blockquoted_correspondence_tables -- --exact`
Exit 0. No red output occurred. Verbatim test output (raw compilation lines remain
in first-case.log):

```text
running 1 test
test adversary_rejects_nonvisible_and_blockquoted_correspondence_tables ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 9 filtered out; finished in 0.02s

```

## 3. Affected suite

After the new case ran alone:
`cargo test --locked -p connectors-build --no-fail-fast --test mcp_inbound_capability_projection --test mcp_inbound_local_binding --test mcp_inbound_local_binding_adversary --test mcp_inbound_local_binding_adversary_pass2`
Exit 0. Verbatim runner output (raw compilation/package-cache-lock lines remain
in suite.log):

```text
running 10 tests
test checker_rejects_ambiguous_names_and_invalid_inverse_spellings ... ok
test checker_rejects_weak_family_projection ... ok
test adversary_rejects_contradictory_duplicate_visibility_row ... ok
test checker_rejects_missing_and_duplicate_error_rows ... ok
test authored_projection_matches_closed_errors_and_named_cases ... ok
test checker_rejects_visibility_and_precedence_contradictions ... ok
test complete_unique_tables_remain_valid_after_row_reordering ... ok
test adversary_rejects_nonvisible_and_blockquoted_correspondence_tables ... ok
test every_authored_table_rejects_missing_unknown_rows_and_duplicate_tables ... ok
test every_authored_table_rejects_duplicate_or_contradictory_rows ... ok

test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.05s

     Running tests/mcp_inbound_local_binding.rs (target/debug/deps/mcp_inbound_local_binding-3c845919adb9283c)

running 16 tests
test the_binding_names_no_cloud_identity_or_network_dependency_it_does_not_refuse ... ok
test the_document_specifies_the_transport_the_matrix_selected_and_selects_nothing ... ok
test the_directory_ships_what_the_story_acceptance_states ... ok
test the_single_principal_boundary_is_stated_and_no_scenario_crosses_it ... ok
test an_acceptance_that_states_both_a_count_and_a_rule_is_refused_rather_than_resolved - should panic ... ok
test the_acceptance_rule_is_read_from_the_statement_in_either_form_it_can_take ... ok
test no_scenario_this_story_owns_names_an_outcome_the_document_does_not ... ok
test the_document_states_the_lease_obligation_its_traces_inherit ... ok
test every_behaviour_states_what_the_selected_transport_does_not_offer ... ok
test every_named_transition_is_one_the_sessions_lifecycle_declares ... ok
test every_behaviour_the_acceptance_names_has_an_outcome_a_transition_and_a_scenario ... ok
test no_trace_issues_a_lease_longer_than_the_contract_ceiling ... ok
test every_close_deadline_a_trace_records_is_bounded_by_the_contract_and_the_live_lease ... ok
test every_transition_a_behaviour_section_asserts_is_bound_to_one_of_its_traces ... ok
test each_scenario_performs_the_transition_its_behaviour_names ... ok
test no_trace_admits_data_or_renews_after_its_live_lease_deadline ... ok

test result: ok. 16 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s

     Running tests/mcp_inbound_local_binding_adversary.rs (target/debug/deps/mcp_inbound_local_binding_adversary-13d0e6a00555fd53)

running 3 tests
test no_lease_a_trace_issues_outlives_the_ceiling_the_sessions_contract_sets ... ok
test no_close_records_a_cutoff_later_than_the_lease_it_ends ... ok
test no_data_act_is_admitted_after_the_lease_that_admits_it_can_legally_be_live ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s

     Running tests/mcp_inbound_local_binding_adversary_pass2.rs (target/debug/deps/mcp_inbound_local_binding_adversary_pass2-c17fbdbd53c42577)

running 2 tests
test the_story_ships_the_number_of_scenario_files_its_acceptance_states ... ok
test no_act_that_needs_a_live_lease_is_admitted_at_or_after_its_effective_expiry ... ok

test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

```

Before 30 is inherited from the correction report; no pre-case suite was run.
After: projection 10, local binding 16, existing adversary 3, existing pass2 2;
31 passed, zero failed/ignored/measured/filtered. The original contradictory-row
regression now passes unchanged, and all three correction class controls pass.
These are document checks, not MCP runtime conformance. Format check
`cargo fmt -p connectors-build --check` exits 0 with empty output.

## 4. Findings

Nothing found in this bounded correction and nearby table-rendering attack.
The earlier finding is no longer reproduced by its retained regression. No
runtime policy, codec or provider assertion follows from that result.

## 5. Attack boundary

The corrected whole-document table inventory rejects the original contradictory
duplicate, and the added test rejects fenced required tables and blockquoted
duplicates across all four table classes. This does not prove arbitrary prose,
HTML/CSS rendering behavior, all Markdown extensions, wire codecs, policy
execution, secret non-disclosure, mutation replay or live withdrawal.

## 6. Retention and identities

No authored paths outside this managed tree. Scratch stays under
`.local/mcp-projection-review/pass2`; Cargo used the existing isolated target,
two jobs and sccache. Disk was 14 GiB free before execution, above the 8 GiB
reserve. Root owns retention and cleanup; only this pass's lease is released.

Input correction report SHA256:
`b7f5e3de57333ff8d7713db40ba0038111498a3a770fb27b014dbf7edfbb870f`.
Input corrected checker SHA256:
`4aabc5bc03eb8c20cb566373057642e5e84b7b1e0be9ccfe8e971dbb96d83723`.
Final checker SHA256:
`8da4a3882c30b0dd7696afe7754a293f0ee22c603fd2cd9669ed47f3d653cbf1`.
Unchanged projection.md SHA256:
`910d35f161bf453ef3f08a997d698f016a53c520eeb0f9847c35aa23908b9e9a`.
Unchanged projection-cases.json SHA256:
`d970f26b6a140ea7b454895ab47338ca87770b3e0f621f9369c41816d139e89d`.
Raw first-case.log SHA256:
`bef1928e14d86ea6a0e0a73904ff8fcd3453e9a937a62a77cd0d7149739df4d2`.
Raw suite.log SHA256:
`c7984b3ecd296b9f0032bbe1ef9a0ead38fb080022885acaa5bcae2f87eb7fd7`.
Review-only patch SHA256:
`60d664f1ea89292d50d52288c1baa01e03e53477f11a4df2e7e444616d3f0f8a`.

```findings
[]
```
