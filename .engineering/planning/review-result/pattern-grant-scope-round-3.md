---
format: aep.planning-md/3
id: review-result:pattern-grant-scope-round-3
kind: review-result
status: active
title: Scope critic, round 3, story:standing-grant-binds-a-pattern-or-a-value-set
relations:
- reviews: story:standing-grant-binds-a-pattern-or-a-value-set
revision: 1
---
verdict: approve


**Round 2 findings**
1. **Shared function in `connectors-core`.** Resolved. Design point 2 (lines 107-115) gives the host its own function in `standing.rs`. It also says `connectors-catalog` is not changed. Acceptance 21a checks by `git diff --name-only` that nothing under `crates/connectors-core` or `crates/connectors-catalog/src` changes. What remains outside the host is one catalog test file reading a shared vector file (lines 207-209, 337-338). It adds no behaviour, and the design lane asked for it. I do not count it as reach beyond the parents.
2. **Document check before the key.** Resolved. Design point 7 (lines 137-139) keeps the 0.77.0 order, with the key and keyring first. The document refusals are checked in-process on the request check that 0.77.0 splits from `grant`. Lines 364-365 add no CLI test. No contract sentence on the order remains.

**New**
- None. Every Acceptance line and Files entry traces to the pick, the record or the consumer. The 64-pattern cap and its contract text sit in Acceptance 20 and 25. The Out of scope lines cover the second confirmation, the `not_after` bound, the object-member relaxation and who may run `grant-standing`.

I re-read lines 98-140 and 316-365 of the story (the frontmatter now shows revision 28). I did not re-read the rest, so Acceptance 1-19 are judged as in round 2.

```findings
[]
```
