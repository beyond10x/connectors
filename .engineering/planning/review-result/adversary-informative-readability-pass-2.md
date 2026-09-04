---
format: aep.planning-md/1
id: review-result:adversary-informative-readability-pass-2
kind: review-result
status: active
title: Adversary, pass 2 — informative-command-readability
relations:
- reviews: story:informative-command-readability
revision: 1
---
# Adversary, pass 2 — informative-command-readability

```
unit: informative-command-readability
verdict: red
cases: executed 72->77, red 4
origin: introduced 6, pre-existing 2, undecided 0
wrote-outside-worktree: .../scratch/{adv2probe/,adv2probe-base/,adv2-*,x.json,y.json,z.json}
needs-coordinator: yes
```

Attacking `06e1e22`, base `7dc4c07c`. Added
`crates/connectors-console/tests/adversary_readability_pass2.rs`, 4 red cases plus one helper. No
implementation file was edited; both mutation probes ran against copies in scratch.

**The header's own arithmetic disagrees with the block below.** It says 6 introduced and 2
pre-existing, 8 in total; the findings block carries 9 rows, 7 introduced and 2 pre-existing.
Recorded as returned, with the discrepancy named rather than silently corrected.

**Each `message` is written as a YAML block scalar.** Several begin with a backtick, which YAML
forbids at the start of a plain scalar. No wording was altered.

## Suite

```
running 65 tests   test result: ok.     65 passed; 0 failed   (lib)
running  7 tests   test result: ok.      7 passed; 0 failed   (pass 1 cases)
running  5 tests   test result: FAILED.  1 passed; 4 failed   (pass 2 cases)
EXIT=101
```

## Attacked and could not break

- `-o json` / `-o yaml` byte identity: six comparisons across three fixtures (control characters,
  nulls, empty keys, nested arrays) against a base-code probe. No difference.
- Severity through a pipe: no escape sequence, no `is_terminal`, no `NO_COLOR` branch.
- Report-level field colliding with a record key in `compact`: all seven shapes this crate builds
  enumerated. No collision.
- `fit_to_budget` termination and arithmetic: no underflow, no infinite loop, no panic at one
  column, zero leading columns, or a pad wider than the budget.
- `elide` never over-runs its allocation for any width >= 1, wide characters included.
- `doctor` and `auth status` both fit the budget with room to spare, at 36 and 62 columns. Only
  `providers` reaches it.

## Deviation the adversary reported against itself

Three probe files were first written to `/tmp`, against the standing rule. They were moved into the
assigned scratch root and nothing remains in `/tmp`.

## Findings

```findings
- file: crates/connectors-console/src/output.rs
  line: 181
  category: contract-drift
  severity: blocker
  verdict: CONFIRMED
  origin: pre-existing
  message: >-
    compact answers an empty listing with one line that parses as a record, which is the exact thing the Format::Compact doc this correction wrote says a line must never be.
- file: crates/connectors-console/src/output.rs
  line: 416
  category: boundary
  severity: blocker
  verdict: CONFIRMED
  origin: introduced
  message: >-
    fit_to_budget lets the last column begin at display column 120 of the 120-column budget, so on the terminal the module documents itself for not one character of connectors providers' last column is on the line.
- file: crates/connectors-console/src/output.rs
  line: 213
  category: acceptance
  severity: warning
  verdict: CONFIRMED
  origin: pre-existing
  message: >-
    when the single array a report carries holds scalars, compact writes each element bare and drops the array's own key, so `connect slack -o compact` loses the name `events` that the acceptance says no format may drop.
- file: crates/connectors-console/src/output.rs
  line: 508
  category: boundary
  severity: warning
  verdict: INFEASIBLE
  origin: introduced
  message: >-
    elide cannot honour a width of zero and always emits its one-column mark, so a column the budget squeezed to zero puts the header and every row one column out of step - reachable only through an empty JSON key, which nothing here was shown to produce.
- file: crates/connectors-console/src/output.rs
  line: 411
  category: judgement
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: >-
    the phase-one floor is the column name width, so required_config_fields keeps 22 of the 94 available columns for one-character cells while provider is cut to 8 (18 of 65 catalogued ids truncated) and four pairs of distinct authorities render identically.
- file: crates/connectors-console/src/output.rs
  line: 471
  category: contract-drift
  severity: note
  verdict: INFEASIBLE
  origin: introduced
  message: >-
    display_width's doc asserts control characters cannot reach it because one_line folded them, but column names are taken raw from fields.keys() and never pass through one_line.
- file: crates/connectors-console/src/output.rs
  line: 623
  category: judgement
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: >-
    ConnectionState::Revoked, this repository's own protocol word for a connection that cannot work, ranks Unknown and shows `?` rather than the `x` the renderer defines for exactly that meaning.
- file: crates/connectors-console/src/output.rs
  line: 472
  category: property
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: >-
    an emoji in a Grafana-chosen target label displaces every later column on that row by one, measured at 14 against 13 on a scratch copy.
- file: crates/connectors-console/src/output.rs
  line: 592
  category: judgement
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: >-
    boolean ready:false marks 13 of 65 catalogued providers with `x`, the glyph defined as "this cannot work", for providers that merely declare no verify probe - the same reasoning the correction used to downgrade not-callable to warn.
```
