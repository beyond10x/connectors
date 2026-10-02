---
format: aep.planning-md/3
id: review-result:mcp-projection-20261002
kind: review-result
status: active
title: 'MCP projection adversary: contradictory duplicate rows accepted'
relations:
- reviews: story:mcp-inbound-capability-projection
revision: 1
---
unit: story:mcp-inbound-capability-projection — candidate working tree on a2955675cb70b5811589ce86b40be95427ead888
verdict: NEEDS-CHANGE
cases: executed 26→27, red 1
origin: introduced 1 / pre-existing 0 / undecided 0
wrote-outside-worktree: none
needs-coordinator: return document checker to implementor; preserve appended regression

## 1. Review-only diff

`git --no-pager diff --stat` is empty because all three candidate files are
untracked at handoff. The inherited candidate is projection.md (336 lines),
projection-cases.json (564 lines), and the original checker (385 lines). Review
added only a test to the checker. Comparing its retained before-copy to the
handoff file gives:

```text
 crates/connectors-build/tests/mcp_inbound_capability_projection.rs | 16 ++++++++++++++++
 1 file changed, 16 insertions(+)
```

The exact no-index diff and stat are retained in review.patch and
review-diff-stat.txt. Both document hashes are unchanged. There are no checker
logic, document, case, model, gate, dependency, AEP or commit changes by this pass.

## 2. First case, before the suite

Added `adversary_rejects_contradictory_duplicate_visibility_row` at checker:388.
It copies the loaded document in memory and inserts a second `ready` row:
`| ready | omitted | not_granted |`, beside the actual
`| ready | visible | candidate_or_retained_key |` row. It asserts that the actual
checker rejects this contradictory contract. No source document is mutated.

First command:
`cargo test --locked -p connectors-build --test mcp_inbound_capability_projection adversary_rejects_contradictory_duplicate_visibility_row -- --exact`
Exit 101. One case executed, zero passed, one failed, five filtered out. This was
the first test execution of this review. Verbatim test-output portion follows;
raw compilation output is retained in first-case.log.

```text
running 1 test
test adversary_rejects_contradictory_duplicate_visibility_row ... FAILED

failures:

---- adversary_rejects_contradictory_duplicate_visibility_row stdout ----

thread 'adversary_rejects_contradictory_duplicate_visibility_row' (2549359) panicked at crates/connectors-build/tests/mcp_inbound_capability_projection.rs:396:5:
document checker accepted contradictory visibility/refusal rows for the same case
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace


failures:
    adversary_rejects_contradictory_duplicate_visibility_row

test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 5 filtered out; finished in 0.00s

error: test failed, to rerun pass `-p connectors-build --test mcp_inbound_capability_projection`
```

## 3. Affected suite after the new case

Command:
`cargo test --locked -p connectors-build --no-fail-fast --test mcp_inbound_capability_projection --test mcp_inbound_local_binding --test mcp_inbound_local_binding_adversary --test mcp_inbound_local_binding_adversary_pass2`
Exit 101. Verbatim test-output portion:

```text
running 6 tests
test checker_rejects_ambiguous_names_and_invalid_inverse_spellings ... ok
test adversary_rejects_contradictory_duplicate_visibility_row ... FAILED
test authored_projection_matches_closed_errors_and_named_cases ... ok
test checker_rejects_weak_family_projection ... ok
test checker_rejects_missing_and_duplicate_error_rows ... ok
test checker_rejects_visibility_and_precedence_contradictions ... ok

failures:

---- adversary_rejects_contradictory_duplicate_visibility_row stdout ----

thread 'adversary_rejects_contradictory_duplicate_visibility_row' (2565089) panicked at crates/connectors-build/tests/mcp_inbound_capability_projection.rs:396:5:
document checker accepted contradictory visibility/refusal rows for the same case
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace


failures:
    adversary_rejects_contradictory_duplicate_visibility_row

test result: FAILED. 5 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

error: test failed, to rerun pass `-p connectors-build --test mcp_inbound_capability_projection`
     Running tests/mcp_inbound_local_binding.rs (target/debug/deps/mcp_inbound_local_binding-3c845919adb9283c)

running 16 tests
test an_acceptance_that_states_both_a_count_and_a_rule_is_refused_rather_than_resolved - should panic ... ok
test every_behaviour_states_what_the_selected_transport_does_not_offer ... ok
test the_binding_names_no_cloud_identity_or_network_dependency_it_does_not_refuse ... ok
test no_scenario_this_story_owns_names_an_outcome_the_document_does_not ... ok
test the_directory_ships_what_the_story_acceptance_states ... ok
test the_document_specifies_the_transport_the_matrix_selected_and_selects_nothing ... ok
test the_acceptance_rule_is_read_from_the_statement_in_either_form_it_can_take ... ok
test no_trace_issues_a_lease_longer_than_the_contract_ceiling ... ok
test every_behaviour_the_acceptance_names_has_an_outcome_a_transition_and_a_scenario ... ok
test the_document_states_the_lease_obligation_its_traces_inherit ... ok
test every_close_deadline_a_trace_records_is_bounded_by_the_contract_and_the_live_lease ... ok
test every_named_transition_is_one_the_sessions_lifecycle_declares ... ok
test every_transition_a_behaviour_section_asserts_is_bound_to_one_of_its_traces ... ok
test the_single_principal_boundary_is_stated_and_no_scenario_crosses_it ... ok
test each_scenario_performs_the_transition_its_behaviour_names ... ok
test no_trace_admits_data_or_renews_after_its_live_lease_deadline ... ok

test result: ok. 16 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s

     Running tests/mcp_inbound_local_binding_adversary.rs (target/debug/deps/mcp_inbound_local_binding_adversary-13d0e6a00555fd53)

running 3 tests
test no_lease_a_trace_issues_outlives_the_ceiling_the_sessions_contract_sets ... ok
test no_data_act_is_admitted_after_the_lease_that_admits_it_can_legally_be_live ... ok
test no_close_records_a_cutoff_later_than_the_lease_it_ends ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests/mcp_inbound_local_binding_adversary_pass2.rs (target/debug/deps/mcp_inbound_local_binding_adversary_pass2-c17fbdbd53c42577)

running 2 tests
test the_story_ships_the_number_of_scenario_files_its_acceptance_states ... ok
test no_act_that_needs_a_live_lease_is_admitted_at_or_after_its_effective_expiry ... ok

test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

error: 1 target failed:
    `-p connectors-build --test mcp_inbound_capability_projection`
```

The before count 26 is inherited from the author's report, not a review pre-run.
After: projection 5 passed/1 failed; local binding 16 passed; existing adversary
3 passed; existing pass2 2 passed. Total 27 executed, 26 passed, one failed, zero
ignored. These are native document checks, not MCP runtime conformance cases.
Package format check (`cargo fmt -p connectors-build --check`) exits 0 with empty
output. Cargo uses two jobs, sccache and this tree's existing target; temporary
files stay in assigned review scratch. Free disk was 14 GiB before the build.

## 4. Finding

| File:line | Verdict | Origin | Finding |
|---|---|---|---|
| crates/connectors-build/tests/mcp_inbound_capability_projection.rs:245 | NEEDS-CHANGE | introduced | The document checker accepts contradictory duplicate visibility/refusal rows, violating the named negative-control obligation. |

Measured: the new test at checker:396 fails, exit 101, because `check` returns
`Ok(())` for a document carrying both decisions. The current code checks only
whether a matching row exists; it does not validate the visibility table's whole
row set or uniqueness. The story requires contradicted visibility/refusal rows
to fail, and projection.md states that promise in its final proof boundary.

Reachability: this is a normal authored-contract editing input to the actual
checker called by `authored_projection_matches_closed_errors_and_named_cases`.
The in-memory document copy models a contradictory row added during a future
edit. The checked-in document does not currently contain that duplicate; no
runtime policy failure or production exploit is claimed. This is a confirmed
guard gap that leaves the promised contradiction check incomplete, not a claim
that a consumer already receives two decisions.

Origin: the checker and both documents are absent from base
`a2955675cb70b5811589ce86b40be95427ead888` (`git ls-tree` exact paths is empty).
The unit introduced the checker under attack. Fix direction: validate a closed,
unique visibility table against the case inventory rather than finding any one
matching line. The implementor owns the repair.

## 5. Other scope checked

The five author checks still reject their existing error/name/family/precedence
mutants; those 5 passes are inherited test coverage, not five new attacks.
Source inspection found explicit policy/revision precedence, three-family result
profiles, withheld mutation replay, single-owner scope and eight retained
unresolved relations. This pass does not prove wire codec behavior, credential
non-disclosure, live withdrawal, arbitrary table mutations or runtime conformance.

## 6. Paths and identities

No authored paths outside the assigned worktree. Tool-managed Cargo/sccache and
worktree lease state are ordinary effects; no external target was selected.
Review scratch is `.local/mcp-projection-review`; root owns retention and cleanup.

Author report SHA256:
`7be73654e02ab8bc90c4dd40547820ac2ae8554e5a35a075007487eabaf5db21`.
Original checker SHA256:
`302ffb99e70da4fcf5d16f0b3abd884ed262f360efbcc26c6cea4e139f97b0ef`.
Final checker (including regression) SHA256:
`af05f0f48d6417ba1d8e5828c2b00ccd375c746a6a16d31fceb907b5b18fb033`.
Unchanged projection.md SHA256:
`910d35f161bf453ef3f08a997d698f016a53c520eeb0f9847c35aa23908b9e9a`.
Unchanged projection-cases.json SHA256:
`d970f26b6a140ea7b454895ab47338ca87770b3e0f621f9369c41816d139e89d`.
Exact raw first-case.log SHA256:
`8b9013911925513e3e1d4c6a6645edadb68c63aaaa4ed9c24228985e9f04a759`.
Exact raw suite.log SHA256:
`ec7a020dba22513a16308d0e8cd149fa60f34c4fea77d98b8df65b5ee3e9fd09`.
Exact review.patch SHA256:
`74c11293734588cbdc744519584db0498a0214a1f423dc10cf63ba2879454ff4`.

```findings
- file: crates/connectors-build/tests/mcp_inbound_capability_projection.rs
  line: 245
  category: mutant
  severity: blocker
  verdict: NEEDS-CHANGE
  origin: introduced
  message: The document checker accepts contradictory duplicate visibility/refusal rows, violating the named negative-control obligation.
```
