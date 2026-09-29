---
format: aep.planning-md/3
id: review-result:knowledge-sources-scope-round-2
kind: review-result
status: active
title: Scope critic, round 2
relations:
- reviews: epic:catalog-knowledge-sources
- reviews: story:catalog-basic-auth-profile
- reviews: story:catalog-jira-cloud-reads
- reviews: story:catalog-zendesk-reads
- reviews: story:catalog-confluence-reads
- reviews: story:catalog-slack-reads
revision: 1
---
needs-revision
epic:catalog-knowledge-sources — the epic's acceptance says "pages every list operation to its documented end condition", but each provider story's paging test walks only "one list operation" (Jira has three list operations, Zendesk, Confluence and Slack two each), so the promise is narrowed to one per provider with nothing recording the drop; either each story's paging test covers every list operation or the epic states one per provider — .engineering/planning/epic/catalog-knowledge-sources.md:49 vs .engineering/planning/story/catalog-jira-cloud-reads.md:34 (same wording at catalog-zendesk-reads.md:33, catalog-confluence-reads.md:36, catalog-slack-reads.md:32)

What I read: 7 artifacts (the epic, the 5 stories and review-result:knowledge-sources-scope-round-1), via `aep plan artifact show` on each, `aep plan artifact graph`, and the epic and Confluence story files with line numbers. I extracted 8 promises from the epic and traced 7 to an item. The 8th is the paging-coverage promise above, which is claimed only in part.

Round 1 finding: fixed. The Confluence `space.pages` row now reads "none (v2 has no updated-since filter); deltas come from `pages.changed`" (catalog-confluence-reads.md:21), and `pages.changed` carries the CQL `lastmodified` filter (line 20). The epic's time-window bullet also now allows for "a statement of how deltas are taken where it does not" (epic line 24-25), which the Jira, Zendesk and Confluence rows use.

No reach beyond the parent: the basic profile is traceable to the epic's "Constraint found", and Slack correctly has no dependency on it. Nothing touches writes, OAuth, webhooks or a fifth source. No two items claim one outcome.

What I could not establish:
- Whether comments, changelog and replies count as "reading" issue trackers and chat. The epic names no operation list, so I did not call them reach.
- Whether the API facts in the tables are true. The stories say they were written from general knowledge, and that is the design and acceptance critics' lane.

```findings
[
  {
    "file": ".engineering/planning/epic/catalog-knowledge-sources.md",
    "line": 49,
    "category": "scope",
    "severity": "warning",
    "verdict": "needs-revision",
    "origin": "introduced",
    "message": "the epic's acceptance says \"pages every list operation to its documented end condition\", but each provider story's paging test walks only \"one list operation\" (Jira three list operations, Zendesk, Confluence and Slack two each), narrowing the promise to one per provider with nothing recording the drop; either each story's paging test covers every list operation or the epic states one per provider"
  }
]
```
