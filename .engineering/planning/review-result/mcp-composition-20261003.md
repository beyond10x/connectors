---
format: aep.planning-md/3
id: review-result:mcp-composition-20261003
kind: review-result
status: active
title: Composition fixture correlation adversary pass
relations:
- reviews: story:mcp-composition-provenance
revision: 1
---
unit: story:mcp-composition-provenance at published base 0f648160 plus frozen author files
verdict: CONFIRMED
cases: executed 58→59, red 1
origin: introduced 1 / pre-existing 0 / undecided 0
wrote-outside-worktree: none
needs-coordinator: repair correlation fixture validation; retain added regression

## Review scope and diff

Coordinator performed a separate adversary pass after the requested independent worker failed at account usage limit. This is disclosed role separation, not a claim that an independent agent ran. Authoring was by another agent. Only one new test was appended to crates/connectors-build/tests/mcp_composition_provenance.rs; all authored documents and existing tests were unchanged. Base had none of the three unit files.

## Deciding case

Added adversary_success_pairs_require_real_correlation_and_exclusive_result. It perturbs copies of successful pair cases across both outbound revisions and all three families; missing IDs and ambiguous result/error envelopes must not count as successful correlated fixture coverage. First command:

`CARGO_BUILD_JOBS=2 RUSTC_WRAPPER=/usr/bin/sccache cargo test --locked -p connectors-build --test mcp_composition_provenance adversary_success_pairs_require_real_correlation_and_exclusive_result`

Exit101, 0 passed/1 failed. First failure: `invalid success fixture accepted: 2025-11-25/tools/missing-both-ids`. serde_json missing-field indexing supplies null on both sides, so equality accepts an uncorrelated fixture. This is a document guard defect, not a measured runtime vulnerability. Source-edit reachability: editing the authored JSON fixture plus its expected observation can erase request/response identity while the document still claims a successful independent hop.

## Affected suite

Ran the six author lanes after the new case, with --no-fail-fast. Exit101: composition10passed/1failed; projection10, mutation10, auth8, lifecycle11, invocation9 passed. Total58passed/1failed, noignored. Raw commands/logs retained in composition-review; no machine paths reproduced here.

| File:line | Verdict | Origin | Finding |
|---|---|---|---|
| crates/connectors-build/tests/mcp_composition_provenance.rs:369 | CONFIRMED | introduced | Successful composition fixtures can omit both correlation IDs because missing-field null values compare equal. |

The fix should validate actual request/response envelope fields, types and exclusive success/error shape before equality. Keep all semantic, inventory, wrapper, budget and unknown-observation checks. Fixture hygiene need not become a complete MCP schema implementation. No new runtime or ESS relation is required.

Other inspected boundaries: all four revision pairs and three families are enumerated; late semantic coverage resists early denials; parsed inventory refuses raw HTML and duplicate/misplaced cases; remaining budgets and host/caller observations are separately represented. No additional unmeasured claim is returned.

```findings
- file: crates/connectors-build/tests/mcp_composition_provenance.rs
  line: 369
  category: boundary
  severity: blocker
  verdict: CONFIRMED
  origin: introduced
  message: Successful composition fixtures can omit both correlation IDs because missing-field null values compare equal.
```
