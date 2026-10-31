---
format: aep.planning-md/3
id: review-result:adversary-d240-s1-pass-1
kind: review-result
status: active
title: Adversary pass 1 on story:invoke-shapes-its-result (wave D240)
tags:
- d240
relations:
- reviews: story:invoke-shapes-its-result
revision: 1
---
## Adversary pass 1, unit S1 (wave D240), as returned

unit: story:invoke-shapes-its-result, tree conn-d240-s1 at 68c4f7bbf plus adversary commit 34cc99d31
verdict: CONFIRMED (3 red cases, all from the unit's own contract documents)
cases: executed 24→27, red 3
origin: introduced 4 / pre-existing 0 / undecided 0
wrote-outside-worktree: 6 paths in the assigned scratch directory (part 6)
needs-coordinator: whether the fix for F2/F3 is a code change or a narrower sentence in semantics.md

Cases added in `apps/connectors/tests/invoke_shapes_adversary.rs` (commit 34cc99d31), all red:
`adversary_no_field_takes_the_records_member_of_invoke_shaped` (F1),
`adversary_a_pointer_names_a_key_holding_a_comma` (F2),
`adversary_a_pointer_names_a_key_ending_in_whitespace` (F3).
Suite after: lib 12 passed, invoke_shapes 12 passed, invoke_shapes_adversary 0 passed / 3 failed, EXIT=101.
Before (file deselected): 12 + 12 passed, EXIT=0.

Judgement F4 (`apps/connectors/src/local/invoke.rs:740`, INFEASIBLE, introduced, note): with
`--definition` on a terminal, the connect question and provider validation in
`definition::prepare_with` run before `--each` on a write is refused; `approval_required` is ordered
the same way and predates the unit. Untested: needs a controlling terminal.

Attacked and not broken: pointer escapes, empty segments, `[]` placement, alias collisions, step and
path bounds, unicode, numeric keys, `body.` fallback, refusals and non-2xx never shaped,
`request_id`/`mutation`/`source_audit` copied from the write projection and not renamable,
`outcome_unknown` an Error reply, `--fields`/`--each` absent from the rewritten argv, `--each` on a
write refused before `approval_required`, trailing `--output json`.

```findings
- file: apps/connectors/src/shape.rs
  line: 27
  category: contract-drift
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: RESERVED omits records, so --fields records:<path> prints a provider value as InvokeShaped.records, which ess/domains/cli.yaml declares Optional<Integer> and no field may take (test adversary_no_field_takes_the_records_member_of_invoke_shaped)
- file: apps/connectors/src/shape.rs
  line: 184
  category: contract-drift
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: the path list is split at every comma before pointers are read, so a key holding a comma cannot be named although semantics.md says such a key is written as an RFC 6901 pointer (test adversary_a_pointer_names_a_key_holding_a_comma)
- file: apps/connectors/src/shape.rs
  line: 185
  category: boundary
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: each item is trimmed before it is read as a pointer, so /a with a trailing space silently names the key a instead of a-space (test adversary_a_pointer_names_a_key_ending_in_whitespace)
- file: apps/connectors/src/local/invoke.rs
  line: 740
  category: judgement
  severity: note
  verdict: INFEASIBLE
  origin: introduced
  message: with --definition on a terminal the connect and its provider validation run in definition::prepare_with before --each on a write is refused, so that refusal is not ahead of every provider call; untested, needs a controlling terminal
```
