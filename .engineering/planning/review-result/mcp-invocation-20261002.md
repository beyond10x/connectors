---
format: aep.planning-md/3
id: review-result:mcp-invocation-20261002
kind: review-result
status: active
title: MCP invocation and corrected framing adversary
relations:
- reviews: story:mcp-outbound-invocation-results
revision: 1
---
unit: outbound invocation and framing correction — cb26g-out working tree over a2955675cb70b5811589ce86b40be95427ead888
verdict: CONFIRMED (one documentation warning)
cases: executed 28→31, red 0
origin: introduced 1 / pre-existing 0 / undecided 0
wrote-outside-worktree: none authored; normal Cargo/sccache and lease metadata only
needs-coordinator: reconcile invocation.md:140–143 with the corrected lifecycle owners

## 1. Review-only diff

`git --no-pager diff --stat` shows the two inherited root corrections; the author's
three new files are untracked. The review delta was compared against the saved
pre-review test file using `git diff --no-index --stat`:

```
 crates/connectors-build/tests/mcp_outbound_invocation_results.rs | 87 ++++++++++++++++++++++
 1 file changed, 87 insertions(+)
```

Only three new tests were appended. No existing test, checker function, contract,
case, scenario, model, manifest or AEP artifact was changed by this review.
Inherited five-file candidate: invocation.md, invocation-cases.json, its Rust
checker, semantics.md and unreadable-answer-is-not-the-callers-input.yaml.
The coordinator's source-before.sha256 pins that inherited state. The initial
28-case count comes from the author's recorded run before the two root framing
corrections; the 31-case run below includes those corrections and all additions.

## 2. New cases, written before execution and run alone first

All three are appended in `crates/connectors-build/tests/mcp_outbound_invocation_results.rs`.
Each operates on copies of authored observations through the existing checker;
none executes an MCP peer, transport or library. None produced a red result.

- `review_exact_octet_ceilings_and_terminal_loss_preserve_uncertainty`: both revisions,
  multibyte request, exact byte ceilings, one-byte request overflow, result overflow
  plus lost terminal boundary, no inferred non-effect and no redispatch.
- `review_non_string_and_unselected_result_types_across_selected_families`: modern
  tools/resources/prompts with null, numeric, object and future-string discriminators;
  malformed versus unsupported distinction and incomplete/unknown/no-redispatch.
- `review_peer_errors_and_malformed_answers_never_establish_non_execution`: complete
  malformed bytes, an unfamiliar integer protocol error with explicit null data,
  and a conflicting result/error envelope; preserve peer data and unknown effects.

Each command used `CARGO_BUILD_JOBS=2 RUSTC_WRAPPER=/usr/bin/sccache` and a task-owned
`TMPDIR` under `.local/tmp/mcp-invocation-review`, the existing isolated target,
`cargo test --locked --offline -p connectors-build --test mcp_outbound_invocation_results <case> -- --exact`.
The cases ran in the order above, before the suite. All exit statuses were 0.
Verbatim runner output (compiler paths omitted):

```
running 1 test
test review_exact_octet_ceilings_and_terminal_loss_preserve_uncertainty ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 8 filtered out; finished in 0.00s

running 1 test
test review_non_string_and_unselected_result_types_across_selected_families ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 8 filtered out; finished in 0.00s

running 1 test
test review_peer_errors_and_malformed_answers_never_establish_non_execution ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 8 filtered out; finished in 0.00s

```

## 3. Affected suite after the additions

Command (same bounded environment), exit 0:

```
cargo test --locked --offline -p connectors-build --test mcp_outbound_invocation_results --test mcp_outbound_connection_lifecycle --test mcp_outbound_connection_lifecycle_adversary --test mcp_outbound_connection_lifecycle_adversary_pass2
```

Verbatim runner output, 31 passed, 0 failed, 0 ignored:

```
     Running tests/mcp_outbound_connection_lifecycle.rs (target/debug/deps/mcp_outbound_connection_lifecycle-55b446a1c20745b9)

running 11 tests
test the_document_does_not_close_the_revision_set_the_model_leaves_open ... ok
test every_refusal_names_a_declared_wire_code_and_loss_is_not_failure ... ok
test every_state_the_story_names_has_a_named_outcome_and_a_scenario_file ... ok
test outbound_stdio_is_held_by_its_blocker_and_nothing_else_is ... ok
test every_refusal_names_whose_act_it_reports ... ok
test every_scenario_this_story_owns_names_a_row_of_the_register ... ok
test no_citation_into_an_editable_document_carries_a_line_number ... ok
test no_state_is_answered_by_sending_the_request_again ... ok
test every_citation_resolves_into_an_archived_file_of_the_pin ... ok
test every_section_and_scenario_resolves_which_revision_it_holds_for ... ok
test no_sentence_writes_a_field_of_the_binding_entity ... ok

test result: ok. 11 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.85s

     Running tests/mcp_outbound_connection_lifecycle_adversary.rs (target/debug/deps/mcp_outbound_connection_lifecycle_adversary-6525b6d0dc1f8f74)

running 6 tests
test a_selection_names_as_many_revisions_as_the_outcome_and_the_model_can_hold ... ok
test the_document_does_not_call_a_four_family_transport_space_a_pair ... ok
test cancellation_says_which_revision_its_signal_holds_for ... ok
test a_request_the_interoperability_revision_makes_this_client_answer_has_an_outcome ... ok
test an_answer_that_was_never_streamed_can_also_be_lost ... ok
test no_binding_field_is_both_unchanged_by_an_observation_and_written_from_one ... ok

test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests/mcp_outbound_connection_lifecycle_adversary_pass2.rs (target/debug/deps/mcp_outbound_connection_lifecycle_adversary_pass2-09e26e9af5440828)

running 5 tests
test a_state_does_not_carry_two_outcomes_that_deny_each_other_s_precondition ... ok
test a_refusal_does_not_name_the_peer_for_a_reason_in_which_no_peer_was_seen ... ok
test the_selection_scenario_states_a_framing_the_interoperability_revision_has_not ... ok
test the_advertised_capability_scenario_reads_a_result_the_interoperability_revision_has_not ... ok
test no_sentence_of_this_contract_writes_a_field_of_the_binding_entity ... ok

test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.17s

     Running tests/mcp_outbound_invocation_results.rs (target/debug/deps/mcp_outbound_invocation_results-e1c0ff323d459de1)

running 9 tests
test modern_resource_cache_metadata_is_required_by_the_pinned_schema ... ok
test review_exact_octet_ceilings_and_terminal_loss_preserve_uncertainty ... ok
test review_peer_errors_and_malformed_answers_never_establish_non_execution ... ok
test review_non_string_and_unselected_result_types_across_selected_families ... ok
test revision_and_error_and_limit_mutations_are_rejected ... ok
test contradictory_effect_retry_authority_and_bytes_are_rejected ... ok
test invocation_document_and_cases_agree_with_selected_sources ... ok
test partial_business_and_structured_observations_cannot_be_relabelled ... ok
test missing_duplicate_and_document_only_mappings_are_rejected ... ok

test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.15s

```

Raw log SHA256 values retained under `.local/mcp-invocation-review`:

- first.log: 357ff9c79c01a5a927062a125c851b58d7a0e0ec58d577f0634977ca2f654289
- discriminator.log: 66090dc51edaddffb2c1edd77cd639499f05a435856134ba80c308c063888c94
- errors.log: 2606a2d986b1c789f0428a7059035952b637a03c879ee20e59992ab1f0683825
- suite.log: 175cd979d0f1791bd7d4744fb51b38ad7d2e52b6667eab05401973300cecfbf7

## 4. Judgement finding

| Location | Verdict / origin | What was measured | What reaches it |
|---|---|---|---|
| adapters/mcp/contracts/client/v1alpha1/invocation.md:140 | CONFIRMED / introduced; warning | The new invocation document still quotes a claim that the candidate has removed from semantics.md:368 and says reconciliation is pending; the corrected scenario likewise explicitly preserves unknown effects. | A reader of the combined candidate is told the lifecycle contract still contradicts invocation when that correction is already part of the candidate. |

Finding: **The invocation document still describes lifecycle framing reconciliation
as pending although the candidate's lifecycle prose and scenario already preserve
unknown effects after dispatch.** Remove the stale integration paragraph or
replace it with an accurate link to the corrected owner. This is documentary
residue, not a demonstrated runtime failure or an unproven remote-effect claim.
`git ls-tree a2955675cb70b5811589ce86b40be95427ead888` has no invocation.md entry;
the inaccurate combined-candidate statement is introduced. The original lifecycle
non-execution wording was pre-existing, but this review does not re-report that
already corrected defect as a current finding.

## 5. Attacks that did not break the bounded mapping

Exact UTF-8 ceilings, overflow precedence and terminal-loss uncertainty remained distinct.
Modern missing/non-string/future result discriminators remained distinct across all three families.
Peer errors preserved code/message/data; malformed output did not acquire caller blame or non-effect knowledge.
Selected source examples agree with inspected archived result fields, including modern resource cache metadata.
The root framing corrections preserve upstream_protocol and peer attribution without changing predispatch refusals.
Known-shape schema validation, output-schema dialects, timers, actual decoding/interoperability and runtime effects remain outside this document-check target.
No full repository gate was run in this bounded pass. Rustfmt ran only on the affected test file. Observed filesystem free space before tests was 12 GiB, above the 8 GiB reserve.

## 6. Outside writes and handoff

No authored path outside the assigned tree. Cargo/sccache and worktree tooling
used their normal global caches and lease metadata. Scratch logs, saved author
source, patch and this report are under `.local/mcp-invocation-review`; compiler
output remains in the tree's existing target. The task-owned temporary directory
was empty and removed. The coordinator owns integration, AEP, retained evidence,
publication and eventual managed-tree cleanup. No commit or planning command ran.

```findings
- file: adapters/mcp/contracts/client/v1alpha1/invocation.md
  line: 140
  category: contract-drift
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: The invocation document still describes lifecycle framing reconciliation as pending although the candidate's lifecycle prose and scenario already preserve unknown effects after dispatch.
```
