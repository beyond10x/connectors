---
format: aep.planning-md/3
id: review-result:adversary-cli-minor-pass-1
kind: review-result
status: active
title: Adversary pass 1 on the CLI minor findings and contract text
relations:
- reviews: story:cli-surface-minor-findings-0-18-0
revision: 1
---
unit: story:cli-surface-minor-findings-0-18-0, uncommitted tree wave0930b-minor on e9af199d8
verdict: NEEDS-CHANGE
cases: executed 54→59, red 3
origin: introduced 2 / pre-existing 3 / undecided 0
wrote-outside-worktree: scratch/adversary
needs-coordinator: a leftover owner process from the implementor's run (stopped by the coordinator); recheck the connect row after the connect fix merges

Cases in apps/connectors/tests/cli_surface_minor_adversary.rs: approvals prepare/issue answer an undecodable document
(red), depth excess past the decoder limit (red), unknown alias when the input file is absent (red), inline vs stdin
classification (green), every list cursor this tree cannot honour is stale_cursor (green).

Coordinator routing: F1-F3 fixed in the contract text to match the binary; F4 fixed by removing the dead <= 256 checks;
F5 rechecked after the connect fix merges.

```findings
[
 {"file":"contracts/cli/v1alpha1/semantics.md","line":192,"category":"contract-drift","severity":"blocker","verdict":"NEEDS-CHANGE","origin":"introduced","message":"approvals prepare and issue answer an undecodable business document with failure/invalid_input, not the cli_dynamic_input this new row assigns"},
 {"file":"contracts/cli/v1alpha1/scenarios.md","line":25,"category":"boundary","severity":"warning","verdict":"CONFIRMED","origin":"pre-existing","message":"depth excess of 127+ levels hits serde_json's recursion limit and answers cli_dynamic_input, not the owner invalid_input C05 now states"},
 {"file":"contracts/cli/v1alpha1/semantics.md","line":520,"category":"acceptance","severity":"warning","verdict":"CONFIRMED","origin":"pre-existing","message":"invoke, prepare, issue and policy-set refuse an absent input file with invalid_input before the unknown alias is admitted, while connect admits the alias first; the contract states no precedence"},
 {"file":"crates/connectors-host/src/local/config.rs","line":202,"category":"judgement","severity":"note","verdict":"CONFIRMED","origin":"pre-existing","message":"the <= 256 selector check (and approval_policy.rs:71) is dead because valid_id caps at 128"},
 {"file":"contracts/cli/v1alpha1/semantics.md","line":490,"category":"contract-drift","severity":"note","verdict":"INFEASIBLE","origin":"introduced","message":"connect pre-publication row describes stage dispatch/retry_explicitly that this tree's owner_failure mapping gives only for ServiceFailure; expected until the connect fix merges"}
]
```
