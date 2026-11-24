---
format: aep.planning-md/3
id: review-result:pattern-grant-parallel-safety-round-3
kind: review-result
status: active
title: Parallel-safety critic, round 3, story:standing-grant-binds-a-pattern-or-a-value-set
relations:
- reviews: story:standing-grant-binds-a-pattern-or-a-value-set
revision: 1
---
verdict: needs-revision


My round-2 finding is resolved. The story no longer touches `crates/connectors-core` or `inventory.rs` (lines 206-209, 323), so `lib.rs` and `fix/258-gmail-control` are not shared.

The three added paths against the work in flight:
- `crates/connectors-host/Cargo.toml` is named among the files shared with 0.77.0 (lines 371-374). No in-flight tree touches it, and the pin changes only root `Cargo.toml` and `Cargo.lock`, which the story already names.
- `crates/connectors-host/tests/fixtures/whole_value_patterns.json` is a new file. The pin's fixture is a different directory, so there is no collision.
- `crates/connectors-catalog/tests/whole_value_patterns.rs` is a new file, but it is not buildable as the story describes it (the finding below).

```findings
- file: .engineering/planning/story/standing-grant-binds-a-pattern-or-a-value-set.md
  line: 208
  category: parallel-safety
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "crates/connectors-catalog/tests/whole_value_patterns.rs is built only once crates/connectors-catalog/tests/main.rs declares it (autotests = false, one [[test]] named main; main.rs header: 'A new file here is built only once it is declared'), yet neither Files nor the scope names tests/main.rs, and line 330 selects binary(whole_value_patterns), which does not exist; add crates/connectors-catalog/tests/main.rs (a mod line) to Files and the scope and name the test by its module path in the main binary (cited; no work in flight touches main.rs, but it is a shared registry file for every catalog test)"
```
