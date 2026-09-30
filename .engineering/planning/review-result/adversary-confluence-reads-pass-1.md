---
format: aep.planning-md/3
id: review-result:adversary-confluence-reads-pass-1
kind: review-result
status: active
title: Adversary pass 1 on Confluence reads with the shared Atlassian profile
relations:
- reviews: story:catalog-confluence-reads
revision: 1
---
unit: story:catalog-confluence-reads, uncommitted tree wave0930-confluence on 885463148
verdict: NEEDS-CHANGE
cases: executed 187→194, red 4
origin: introduced 4 / pre-existing 0 / undecided 0
wrote-outside-worktree: scratch/adversary
needs-coordinator: who writes the CHANGELOG entry for the Jira profile break

Cases: crates/connectors-catalog/tests/server_path_adversary.rs (path-item and operation servers overrides red;
authority shapes green); adapters/catalog/tests/confluence_reads_adversary.rs (stop rule without version red;
CHANGELOG red; base-path refusal green; request passthrough green). Could not break: server-path shapes, the
base-path refusal both ways, a Jira connection used for Confluence or the reverse, the limit bound, credential
paths, names.

Coordinator routing: overrides named unsupported; a page without version counts as changed; the guide states that
pages.changed needs sort=-modified-date; the CHANGELOG entry written by the coordinator in the unit tree. Notes 4
and 5 kept as notes.

```findings
[
  {"file": "crates/connectors-catalog/src/inventory.rs", "line": 245, "category": "contract-drift", "severity": "warning", "verdict": "INFEASIBLE", "origin": "introduced", "message": "server_path reads only document-level servers, so a path-item or operation servers override is recorded under the document base instead of being honoured or named unsupported"},
  {"file": "docs/catalog-confluence.md", "category": "acceptance", "severity": "warning", "verdict": "NEEDS-CHANGE", "origin": "introduced", "message": "the pages.changed stop rule claims every listed page carries version.createdAt although PageBulk does not require version"},
  {"file": "CHANGELOG.md", "category": "contract-drift", "severity": "warning", "verdict": "NEEDS-CHANGE", "origin": "introduced", "message": "the breaking move from jira.basic/jira.user to atlassian.basic/atlassian.account is not recorded in the CHANGELOG"},
  {"file": "crates/connectors-catalog/src/inventory.rs", "line": 261, "category": "judgement", "severity": "note", "verdict": "INFEASIBLE", "origin": "introduced", "message": "a document whose paths already carry its server path is now prefixed twice on rebuild; no shipped document does this"},
  {"file": "adapters/catalog/providers/confluence/operations.json", "line": 5, "category": "judgement", "severity": "note", "verdict": "INFEASIBLE", "origin": "introduced", "message": "pages.changed is only correct with sort=-modified-date but the selection cannot fix it"}
]
```
