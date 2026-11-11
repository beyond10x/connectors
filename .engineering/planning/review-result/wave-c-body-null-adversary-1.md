---
format: aep.planning-md/3
id: review-result:wave-c-body-null-adversary-1
kind: review-result
status: active
title: Adversary pass 1 on required-body-key-refuses-null
relations:
- reviews: story:required-body-key-refuses-null
revision: 1
---
## Adversary pass 1 on story:required-body-key-refuses-null

Reviewed commit f3559c4c0a on impl/required-body-key-refuses-null (base 9b97114df). Model: opus.

```
verdict: green
cases: executed 44→45, red 0
origin: introduced 2, pre-existing 0, undecided 0
```

A new uncommitted case, `a_lone_null_edit_member_is_refused_and_one_beside_a_change_is_sent` in
`jira_issue_writes_adversary.rs`, records the behaviour change. Under the change, Jira
`issue.edit` with only `description`, `priority` or `assignee` set to null is refused. A null
member beside a non-null change is still sent.

The suite passed:

- adversary_catalog_body_types: 4
- body_keys_adversary: 4
- jira_issue_writes: 11
- jira_issue_writes_adversary: 16
- nested_closed_body: 10

What held:

- `prepare` checks the closed body before it validates against the schema.
- A null parent counts its required keys as missing.
- Removing each changed line turns a test red.
- The `nested_closed_body` expectation is a literal value.
- No other shipped selection loses a null clear: Zendesk keys are typed, and GitLab `actions` has
  no meaningful null.

What the pinned Jira document says: `IssueUpdateDetails.fields` admits null. Only `assignIssue`
documents null as "unassign", and editIssue names no other way to clear a field.

```findings
[
  {"file": "adapters/catalog/src/closed.rs", "line": 455, "category": "acceptance", "severity": "warning", "verdict": "CONFIRMED", "origin": "introduced", "message": "Jira issue.edit with only description, priority or assignee set to null (a clear that passed and was sent before) is now refused, and a clear works only beside another member that is not null; the pinned editIssue schema admits null and only assignIssue documents null as unassign."},
  {"file": "docs/local-catalog-provider.md", "line": 1043, "category": "contract-drift", "severity": "note", "verdict": "NEEDS-CHANGE", "origin": "introduced", "message": "The body_required and body_any_of docs (and selection.yaml:60) do not say that null now counts as missing, and the refusal at closed.rs:461 says the body carries none of the keys when it carries one as null."}
]
```
