---
format: aep.planning-md/3
id: review-result:adversary-list-paging-pass-1
kind: review-result
status: active
title: Adversary pass 1 on list paging with a cursor
relations:
- reviews: story:list-commands-page-with-a-cursor
revision: 1
---
unit: story:list-commands-page-with-a-cursor, uncommitted tree wave0930b-paging on e9af199d8
verdict: NEEDS-CHANGE
cases: executed 54→61, red 1
origin: introduced 1 / pre-existing 0 / undecided 0
wrote-outside-worktree: scratch/adversary
needs-coordinator: none

Cases in apps/connectors/tests/list_paging_adversary.rs: an operations cursor is bound to the adapter it was issued for
(red), every entry appears exactly once around the limit, limits outside 1..500, one spelling of a cursor, forged
offsets, no crossing between the two lists, operations pages cover every permitted operation once (green).

Coordinator routing: the finding to the implementor (alias and instance in the operations digest); the hand-written
base64 replaced by the base64 crate in the same round.

```findings
[
  {"file": "apps/connectors/src/local/operations.rs", "line": 46, "category": "acceptance", "severity": "warning", "verdict": "NEEDS-CHANGE", "origin": "introduced", "message": "The operations-list cursor digest omits the adapter alias, so a cursor issued for one configured instance is accepted by another with the same descriptor revision and grants."}
]
```
