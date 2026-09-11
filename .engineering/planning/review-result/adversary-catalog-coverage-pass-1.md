---
format: aep.planning-md/1
id: review-result:adversary-catalog-coverage-pass-1
kind: review-result
status: active
title: Adversary pass 1 on the bundle coverage report
relations:
- reviews: story:catalog-coverage-report
revision: 1
---
# Adversary pass 1 — story:catalog-coverage-report

Worktree `wt-35ad348b4293` at `f9061f6` plus the unit's uncommitted work.

```
unit: 1
verdict: red
cases: executed 102→106, red 4
origin: introduced 5, pre-existing 0, undecided 0
wrote-outside-worktree: three log files under the assigned scratch root
needs-coordinator: no
```

One new file, `crates/connectors-catalog/tests/coverage_adversarial.rs`, four cases, all red. Every
bundle in them is built the way the unit's own fixture builds one — real bytes through `ingest`, the
parsed document through `inventory::extract` — so nothing is hand-assembled into a state the crate
cannot produce.

## What held

Determinism is order-independent by construction and survived permuted operations, gaps and
duplicate designations. `inventoried` cannot disagree with `Inventory::coverage`, because both come
from it verbatim and no arithmetic sits between them. Counts cannot overflow. Zero and non-zero
boundaries in both directions still print the combined line. The serialised form requires both count
fields. Empty reasons and duplicate gaps render oddly but separate no count.

## Findings

```findings
- file: crates/connectors-catalog/src/coverage.rs
  line: 126
  category: acceptance
  severity: blocker
  verdict: CONFIRMED
  origin: introduced
  message: "`Report::text` interpolates document-controlled strings unescaped, so a parameter name or an `info.version` carrying a newline adds a text line stating one count without the other, which the story's Acceptance forbids"
- file: crates/connectors-catalog/src/coverage.rs
  line: 165
  category: acceptance
  severity: blocker
  verdict: CONFIRMED
  origin: introduced
  message: an unsupported reason built from a document parameter name can put a quotable percentage line into the text form, defeating the Outcome statement that nobody can quote a percentage
- file: crates/connectors-catalog/src/coverage.rs
  line: 163
  category: contract-drift
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: a reason containing a newline makes the text form show more reason groups than the serialised form carries, so the two renderings of one value disagree about structure
- file: crates/connectors-catalog/src/coverage.rs
  line: 42
  category: judgement
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: source_file_name is not in the story's Scope enumeration and adds a third unvalidated caller-controlled string to the unescaped source line, so either the story or the field should change
- file: crates/connectors-catalog/src/coverage.rs
  line: 99
  category: judgement
  severity: note
  verdict: INFEASIBLE
  origin: introduced
  message: no producer in the tree emits a method outside METHODS (extract, authored and the digest-checked loader all prevent it), so the extension branch is untestable forward-compatibility rather than live behaviour
```
