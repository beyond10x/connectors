---
format: aep.planning-md/3
id: review-result:adversary-gateway-prefix-pass-1
kind: review-result
status: active
title: Adversary pass 1 on the catalog gateway prefix
relations:
- reviews: story:catalog-api-base-gateway-prefix
revision: 1
---
unit: story:catalog-api-base-gateway-prefix, uncommitted tree wave0930b-gateway on 94d7d25e3
verdict: NEEDS-CHANGE
cases: executed 80→84, red 1
origin: introduced 2 / pre-existing 0 / undecided 0
wrote-outside-worktree: scratch/adversary logs
needs-coordinator: the live basic-over-gateway observation is the coordinator's

Cases in adapters/catalog/tests/gateway_prefix_adversary.rs: each guide documents the basic gateway form a
service-account token uses (red), two valid prefixes on one api_base give two revisions (green), the acceptance form
with the gateway root as api_base (green), prefix parsing edges (green). Could not break: requests leaving api_base or
the prefix, prefix parsing, unprefixed bootstraps byte-identical, identity reads carry the prefix, outside-base refusal.

Coordinator routing: finding 1 to the implementor (docs: service accounts use atlassian.basic over the gateway, verified
live 2026-09-30 with 10 issues returned; Bearer only for OAuth, labelled unverified); finding 2 covered by the
adversary's revision case.

```findings
[
  {"file": "docs/catalog-jira.md", "line": 146, "category": "acceptance", "severity": "blocker", "verdict": "NEEDS-CHANGE", "origin": "introduced", "message": "the gateway sections of the Jira and Confluence guides send service-account API tokens as Bearer and show only an atlassian.bearer form, while a live run connected through the gateway with Basic"},
  {"file": "adapters/catalog/tests/gateway_prefix.rs", "line": 441, "category": "mutant", "severity": "note", "verdict": "CONFIRMED", "origin": "introduced", "message": "the revision assert compares configs that already differ in api_base and profile, so it stays green if request_prefix is dropped from the revision input"}
]
```
