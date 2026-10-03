---
format: aep.planning-md/3
id: review-result:mcp-replay-20261003
kind: review-result
status: active
title: 'Replay case coverage adversary: required premises erased'
relations:
- reviews: story:mcp-inbound-mutation-replay
revision: 1
---
unit: story:mcp-inbound-mutation-replay — frozen cb26i-replay over b360aaaa771ad9a31cab46ff4249015b74a4d50f plus tests below
verdict: NEEDS-CHANGE
cases: executed 37→40, red 2
origin: introduced 2 / pre-existing 0 / undecided 0
wrote-outside-worktree: none
needs-coordinator: return guard corrections to author; preserve both failing cases before integration

## 1. Diff and charter

`git --no-pager diff --stat` produced empty output: all three authored files were
already untracked at handoff. This does not mean the reviewer changed nothing.
The inherited mutations.md and mutation-cases.json retain their frozen hashes.
The complete reviewer delta, compared against the copied frozen test file, is:

```text
 crates/connectors-build/tests/mcp_inbound_mutation_replay.rs | 96 +++++++++++++++++++
 1 file changed, 96 insertions(+)
```

The exact append-only diff is retained as own-test.diff. No inherited line was
changed; no assertion was removed, weakened or skipped. No contract, runtime,
model, planning artifact or implementation function was changed. The new tests
mutate in-memory copied document/case inputs only. Baseline source hashes:

```text
1f7884ccc7ededac8cef0173e18b04b0182fead6bb8b912caa59bdd348175fd3  adapters/mcp/contracts/server/v1alpha1/mutations.md
ada8dae41e6c28d13320f262530a42c837fb3aef524e7312faa31e1ffbb281e2  adapters/mcp/contracts/server/v1alpha1/mutation-cases.json
c1d36bbacd7253d14e68dac74681ee5dbf72d71242dd5159c85164e972bfe437  crates/connectors-build/tests/mcp_inbound_mutation_replay.rs
```

Reviewed test hash after additions:
`547452ec1d0e99ce1471e03302684a80a31f8ff8f603b21a9435fc275b20fc50`.
The author report hash was b9b80ad3ebcb8733b71e33f5c6083d2500907065fa3e38255654d505e4b01b56.
Before-count 37 comes from that frozen author report, not a pre-attack suite run.

## 2. Cases written before first execution

Three tests were appended before executing any Cargo test:

- `adversary_named_replay_cases_cannot_be_replaced_by_policy_denials`, line 689:
  positive control checks original documents; then independently replaces each
  named exact replay, revision-before-key, winner-after-miss and preflight-winner
  case with a policy denial, updating its displayed decision. Requires rejection
  because those cases no longer exercise their named obligation. RED: all four
  stripped cases are accepted by the guard.
- `adversary_recovery_case_must_exercise_recovery_observer`, line 724: positive
  original control, then changes recovery-after-dispatch's observer to live_host.
  Requires rejection because the named recovery case no longer covers recovery.
  RED: the guard accepts it without any change to the displayed document.
- `adversary_retained_admission_revision_and_winner_precedence`, line 742: literal
  expected decisions across Pending/Replayable/Quarantined, stale revision,
  current policy denial and final disclosure revocation; and exact/conflicting/
  unreadable concurrent winners for missing/refused/spent/preflight-unavailable
  candidate approvals. GREEN. This exercises document-decision logic only.

First deciding command (only the three new attacks selected), exit 101:
`CARGO_BUILD_JOBS=2 RUSTC_WRAPPER=/usr/bin/sccache cargo test --locked -p connectors-build --test mcp_inbound_mutation_replay adversary_ -- --nocapture`

Verbatim deciding test output follows; compilation preamble is omitted and is
retained unchanged in first-attack.log:

```text
running 3 tests
test adversary_retained_admission_revision_and_winner_precedence ... ok

thread 'adversary_recovery_case_must_exercise_recovery_observer' (3684498) panicked at crates/connectors-build/tests/mcp_inbound_mutation_replay.rs:735:5:
recovery obligation erased without changing any displayed row
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace
test adversary_recovery_case_must_exercise_recovery_observer ... FAILED

thread 'adversary_named_replay_cases_cannot_be_replaced_by_policy_denials' (3684497) panicked at crates/connectors-build/tests/mcp_inbound_mutation_replay.rs:717:5:
named obligations erased while checker accepts: ["exact-replay", "revision-before-key", "winner-after-miss", "preflight-winner"]
test adversary_named_replay_cases_cannot_be_replaced_by_policy_denials ... FAILED

failures:

failures:
    adversary_named_replay_cases_cannot_be_replaced_by_policy_denials
    adversary_recovery_case_must_exercise_recovery_observer

test result: FAILED. 1 passed; 2 failed; 0 ignored; 0 measured; 6 filtered out; finished in 0.03s

error: test failed, to rerun pass `-p connectors-build --test mcp_inbound_mutation_replay`
```

## 3. Affected suite, after the attacks

Command, exit 101:
`CARGO_BUILD_JOBS=2 RUSTC_WRAPPER=/usr/bin/sccache cargo test --locked -p connectors-build --no-fail-fast --test mcp_inbound_mutation_replay --test mcp_inbound_capability_projection --test mcp_inbound_local_binding --test mcp_inbound_local_binding_adversary --test mcp_inbound_local_binding_adversary_pass2`

Verbatim output:

```text
    Blocking waiting for file lock on package cache
    Finished `test` profile [optimized] target(s) in 2.70s
     Running tests/mcp_inbound_capability_projection.rs (target/debug/deps/mcp_inbound_capability_projection-ab521dfd97d6ce13)

running 10 tests
test checker_rejects_weak_family_projection ... ok
test authored_projection_matches_closed_errors_and_named_cases ... ok
test checker_rejects_missing_and_duplicate_error_rows ... ok
test checker_rejects_ambiguous_names_and_invalid_inverse_spellings ... ok
test adversary_rejects_contradictory_duplicate_visibility_row ... ok
test complete_unique_tables_remain_valid_after_row_reordering ... ok
test checker_rejects_visibility_and_precedence_contradictions ... ok
test adversary_rejects_nonvisible_and_blockquoted_correspondence_tables ... ok
test every_authored_table_rejects_missing_unknown_rows_and_duplicate_tables ... ok
test every_authored_table_rejects_duplicate_or_contradictory_rows ... ok

test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.07s

     Running tests/mcp_inbound_local_binding.rs (target/debug/deps/mcp_inbound_local_binding-3c845919adb9283c)

running 16 tests
test the_binding_names_no_cloud_identity_or_network_dependency_it_does_not_refuse ... ok
test every_transition_a_behaviour_section_asserts_is_bound_to_one_of_its_traces ... ok
test every_named_transition_is_one_the_sessions_lifecycle_declares ... ok
test the_acceptance_rule_is_read_from_the_statement_in_either_form_it_can_take ... ok
test the_document_specifies_the_transport_the_matrix_selected_and_selects_nothing ... ok
test the_document_states_the_lease_obligation_its_traces_inherit ... ok
test every_behaviour_states_what_the_selected_transport_does_not_offer ... ok
test the_single_principal_boundary_is_stated_and_no_scenario_crosses_it ... ok
test an_acceptance_that_states_both_a_count_and_a_rule_is_refused_rather_than_resolved - should panic ... ok
test no_scenario_this_story_owns_names_an_outcome_the_document_does_not ... ok
test no_trace_issues_a_lease_longer_than_the_contract_ceiling ... ok
test every_close_deadline_a_trace_records_is_bounded_by_the_contract_and_the_live_lease ... ok
test the_directory_ships_what_the_story_acceptance_states ... ok
test every_behaviour_the_acceptance_names_has_an_outcome_a_transition_and_a_scenario ... ok
test each_scenario_performs_the_transition_its_behaviour_names ... ok
test no_trace_admits_data_or_renews_after_its_live_lease_deadline ... ok

test result: ok. 16 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.05s

     Running tests/mcp_inbound_local_binding_adversary.rs (target/debug/deps/mcp_inbound_local_binding_adversary-13d0e6a00555fd53)

running 3 tests
test no_data_act_is_admitted_after_the_lease_that_admits_it_can_legally_be_live ... ok
test no_close_records_a_cutoff_later_than_the_lease_it_ends ... ok
test no_lease_a_trace_issues_outlives_the_ceiling_the_sessions_contract_sets ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s

     Running tests/mcp_inbound_local_binding_adversary_pass2.rs (target/debug/deps/mcp_inbound_local_binding_adversary_pass2-c17fbdbd53c42577)

running 2 tests
test the_story_ships_the_number_of_scenario_files_its_acceptance_states ... ok
test no_act_that_needs_a_live_lease_is_admitted_at_or_after_its_effective_expiry ... ok

test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s

     Running tests/mcp_inbound_mutation_replay.rs (target/debug/deps/mcp_inbound_mutation_replay-67e2a2626349b7e6)

running 9 tests
test adversary_retained_admission_revision_and_winner_precedence ... ok
test authored_replay_contract_matches_existing_owners ... ok
test adversary_recovery_case_must_exercise_recovery_observer ... FAILED
test adversary_named_replay_cases_cannot_be_replaced_by_policy_denials ... FAILED
test copied_observations_preserve_unknown_and_applied_with_error ... ok
test copied_errors_and_correlation_cannot_erase_uncertainty ... ok
test copied_lifecycle_and_coordinate_inventories_remain_closed ... ok
test copied_cases_cannot_grant_replay_dispatch_or_bypass_admission ... ok
test every_document_table_rejects_extra_missing_and_contradictory_rows ... ok

failures:

---- adversary_recovery_case_must_exercise_recovery_observer stdout ----

thread 'adversary_recovery_case_must_exercise_recovery_observer' (3699855) panicked at crates/connectors-build/tests/mcp_inbound_mutation_replay.rs:735:5:
recovery obligation erased without changing any displayed row
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace

---- adversary_named_replay_cases_cannot_be_replaced_by_policy_denials stdout ----

thread 'adversary_named_replay_cases_cannot_be_replaced_by_policy_denials' (3699854) panicked at crates/connectors-build/tests/mcp_inbound_mutation_replay.rs:717:5:
named obligations erased while checker accepts: ["exact-replay", "revision-before-key", "winner-after-miss", "preflight-winner"]


failures:
    adversary_named_replay_cases_cannot_be_replaced_by_policy_denials
    adversary_recovery_case_must_exercise_recovery_observer

test result: FAILED. 7 passed; 2 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.49s

error: test failed, to rerun pass `-p connectors-build --test mcp_inbound_mutation_replay`
error: 1 target failed:
    `-p connectors-build --test mcp_inbound_mutation_replay`
```

Total executed 40: 38 passed, 2 failed, 0 ignored. Inherited 37 remain green;
three added cases account for one additional pass and the two failures.
`cargo fmt -p connectors-build --check` exited 0, empty output. No full workspace
or live-provider suite was run. The existing isolated target, two Cargo jobs and
sccache were used; observed free storage after first execution was 28,025,618,432
bytes, above the 8 GiB reserve. No build directory or tree was removed.

## 4. Findings

| File:line | Verdict | Origin | Finding | What was measured | What reaches it |
|---|---|---|---|---|---|
| crates/connectors-build/tests/mcp_inbound_mutation_replay.rs:386 | NEEDS-CHANGE | introduced | Required decision IDs are checked without their semantic premises, allowing named replay, revision and winner-recheck coverage to disappear behind policy denials. | Four copied case/table replacements were all accepted; added assertion at line 717 fails, exit 101. | The authored document guard reads mutation-cases.json and mutations.md through inputs/check; the normal workspace test gate selects it. This is a reachable document-edit regression, not an MCP runtime exploit. |
| crates/connectors-build/tests/mcp_inbound_mutation_replay.rs:476 | NEEDS-CHANGE | introduced | The required recovery-after-dispatch case does not require a recovery observer, so it can silently become a live-host case. | Changing only its observer from recovery to live_host remains accepted; added assertion at line 735 fails, exit 101. | The same authored-case input and normal test gate; no provider or runtime recovery process was executed. |

Both findings cover the frozen working tree plus the appended tests. The guard and
case files do not exist in base b360aaaa; these are introduced coverage holes in
the new unit, not a pre-existing production defect. The unmodified authored cases
have the intended premises today. These failures show that the promised named
coverage is not retained by the guard when its own inputs change. Correct by
validating each required case's distinguishing premises or the complete semantic
coverage matrix; preserve legitimate irrelevant variation and keep the new tests.
No fix was applied here. There are no additional judgement-only findings.

## 5. Attacked without a break

Current policy and final disclosure denial beat retained observation in the
explicit Pending, Replayable and Quarantined combinations tested.
Stale revision beats key/result observation in those same combinations.
After a miss, exact/conflicting/unreadable winners beat all four tested candidate
approval/preflight refusals, and final disclosure denial still wins.
Neither successful document checks nor these attacks claim MCP runtime
conformance, mutation advertisement, real ledger recovery or provider effects.

## 6. Paths and handoff

No authored paths outside this managed worktree. Reviewer scratch is exclusively
.local/mcp-two-waves/replay-review: baseline.rs, baseline.sha256, own-test.diff,
first-attack.log/exit, suite.log/exit, fmt.log/exit and report.md. Cargo used the
existing tree-local target; normal tool-managed lease and sccache activity is not
an authored artifact. Only the reviewer lease is released. The coordinator owns
correction, AEP recording, publication, evidence retention and eventual cleanup.

```findings
- file: crates/connectors-build/tests/mcp_inbound_mutation_replay.rs
  line: 386
  category: mutant
  severity: blocker
  verdict: NEEDS-CHANGE
  origin: introduced
  message: Required decision IDs are checked without their semantic premises, allowing named replay, revision and winner-recheck coverage to disappear behind policy denials.
- file: crates/connectors-build/tests/mcp_inbound_mutation_replay.rs
  line: 476
  category: mutant
  severity: blocker
  verdict: NEEDS-CHANGE
  origin: introduced
  message: The required recovery-after-dispatch case does not require a recovery observer, so it can silently become a live-host case.
```
