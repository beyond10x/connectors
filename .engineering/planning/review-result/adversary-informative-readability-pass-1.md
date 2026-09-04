---
format: aep.planning-md/1
id: review-result:adversary-informative-readability-pass-1
kind: review-result
status: active
title: Adversary, pass 1 — informative-command-readability
relations:
- reviews: story:informative-command-readability
revision: 1
---
# Adversary, pass 1 — informative-command-readability

```
unit: informative-command-readability
verdict: red
cases: executed 55->62, red 6
origin: introduced 9, pre-existing 1, undecided 0
wrote-outside-worktree: .../scratch/{probe/,mutant/,full-mutant/,emit.err}
needs-coordinator: no
```

Worktree `wt-d41dd2700a2e`, `impl/informative-command-readability` @ `7daeb22`, base `7dc4c07`.
Added `crates/connectors-console/tests/adversary_readability.rs`, 302 lines, 6 red cases plus one
green helper. No implementation file was edited.

**The findings block below is the adversary's, with one mechanical change: each `message` is written
as a YAML block scalar.** As returned, four messages began with a backtick, which YAML forbids at
the start of a plain scalar, and `aep artifact new` refused the record. No wording was altered.

## Suite

```
running 55 tests
test result: ok. 55 passed; 0 failed
     Running tests/adversary_readability.rs
running 7 tests
test result: FAILED. 1 passed; 6 failed
failures:
    a_record_whose_cells_are_all_empty_is_rendered_as_a_blank_line
    a_wide_character_cell_leaves_the_column_after_it_ragged
    an_unranked_table_lets_a_cell_sit_where_the_severity_marker_sits
    compact_no_longer_puts_one_record_on_every_line
    doctor_spreads_one_check_over_several_unmarked_lines_when_the_configuration_is_malformed
    providers_starts_its_last_column_past_the_width_of_any_terminal
exit status 101
```

## Attacked and could not break

- `-o json` / `-o yaml` bytes: 10 values rendered through base and HEAD side by side, **0 byte
  differences**. The unit's central claim holds.
- Severity through a pipe: no escape sequence, no TTY detection, no colour; markers are ASCII.
- `compact` dropping a field: no value found at any nesting depth whose field vanishes.
- `payload` unwrapping, mixed and degenerate arrays, ragged records, `unwrap_single_array` with two
  arrays, and the index arithmetic in the column move all held.

## Fixes the adversary named, and did not apply

1. fold a newline in `cell`/`inline` to a visible escape, as empty is folded to `(none)`
2. cap or elide columns in `providers`, or give the table a width budget
3. update `Format::Compact`'s doc, or prefix the summary line so a consumer can skip it
4. measure width with a display-width function
5. always emit the marker column, blank when unranked
8. rank `not-callable` `Warn`

## Findings

```findings
- file: crates/connectors-console/src/output.rs
  line: 389
  category: acceptance
  severity: blocker
  verdict: NEEDS-CHANGE
  origin: introduced
  message: >-
    a newline inside a cell is written verbatim, so `connectors doctor` over a malformed TOML config renders one check as six lines with the toml caret diagram sitting in the marker column.
- file: crates/connectors-console/src/output.rs
  line: 322
  category: acceptance
  severity: blocker
  verdict: NEEDS-CHANGE
  origin: introduced
  message: >-
    moving the widest column last cannot stop `connectors providers` wrapping, because the eleven columns before it already start the last column at terminal column 196 and the widest row is 314.
- file: crates/connectors-console/src/output.rs
  line: 45
  category: contract-drift
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: >-
    `Format::Compact` still documents "One record per line, for `grep` and `wc -l`" while `providers` now emits a leading summary line that no reader or script can tell from a record, moving `wc -l` from 65 to 66.
- file: crates/connectors-console/src/output.rs
  line: 311
  category: boundary
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: >-
    column widths are counted in `char`s rather than terminal columns, so one wide character in a cell displaces every column after it by one on that row alone.
- file: crates/connectors-console/src/output.rs
  line: 335
  category: acceptance
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: >-
    a table with no ranked record omits the marker column entirely, so a cell beginning `x ` occupies the exact position the reader has been taught means a failed row.
- file: crates/connectors-console/src/output.rs
  line: 371
  category: boundary
  severity: note
  verdict: INFEASIBLE
  origin: introduced
  message: >-
    `write_row` trims the finished line, so a record whose cells are all empty strings renders as a blank line with neither keys nor the `-` sentinel; no command was found that emits one.
- file: crates/connectors-console/src/output.rs
  line: 322
  category: mutant
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: >-
    four mutants survive all 55 tests - deleting the whole widest-column-move block, dropping `line.trim_end()`, making `strictly_widest` ignore ties, and replacing the `-` for an absent key with an empty cell.
- file: crates/connectors-console/src/output.rs
  line: 490
  category: judgement
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: >-
    `not-callable` ranks as a failure while the same row's `absent` credentials rank as a warning, so a fresh install marks every configured provider `x` under the module's own reasoning for the opposite.
- file: crates/connectors-console/src/output.rs
  line: 853
  category: judgement
  severity: note
  verdict: INFEASIBLE
  origin: introduced
  message: >-
    the severity-word scan reads only this package's own `src/` for same-line literals, not the wire vocabulary the doc claims it guards (`AdminCredentialState::Present` ranks Unknown), though no marker column reaches that word today.
- file: crates/connectors-cli/src/lib.rs
  line: 459
  category: judgement
  severity: note
  verdict: CONFIRMED
  origin: pre-existing
  message: >-
    `emit` returns Err(BrokenPipe) rather than treating a closed pipe as success, and only `completions` maps BrokenPipe to `Ok(())`, so `connectors providers | head -1` exits non-zero once the output exceeds the pipe buffer.
```
