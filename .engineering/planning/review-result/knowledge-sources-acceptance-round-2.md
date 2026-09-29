---
format: aep.planning-md/3
id: review-result:knowledge-sources-acceptance-round-2
kind: review-result
status: active
title: Acceptance critic, round 2
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
epic:catalog-knowledge-sources — the third acceptance bullet, "The four provider stories and `story:catalog-basic-auth-profile` are implemented", names no observable and restates child status, so it can be voted true — .engineering/planning/epic/catalog-knowledge-sources.md:51
story:catalog-basic-auth-profile — the first bullet ("can declare a basic profile … `connections connect` accepts the credential document … from `--credential-file` or `--credential-stdin`") is two statements, and no later bullet says the tests connect through both entry routes — .engineering/planning/story/catalog-basic-auth-profile.md:25
story:catalog-jira-cloud-reads — "A paging test walks one list operation across two fixture pages" covers one of three list operations with three different end conditions, while the epic requires every list operation to page to its documented end condition — .engineering/planning/story/catalog-jira-cloud-reads.md:34 (epic :49)
story:catalog-zendesk-reads — the paging test walks "one list operation" of two, and `end_of_stream` and `meta.has_more` are different end conditions, so the epic's every-list-operation paging goes unshown — .engineering/planning/story/catalog-zendesk-reads.md:33 (epic :49)
story:catalog-confluence-reads — the paging test walks "one list operation" of two (`pages.changed` on `start`/`limit`, `space.pages` on `cursor`), so the epic's every-list-operation paging goes unshown — .engineering/planning/story/catalog-confluence-reads.md:36 (epic :49)
story:catalog-slack-reads — the paging test walks "one list operation" of two, so `conversations.history` and `conversations.replies` are not both shown to stop at the end condition — .engineering/planning/story/catalog-slack-reads.md:32 (epic :49)
story:catalog-jira-cloud-reads — "the unit verifies each against the pinned document and records any difference in the guide" cannot be told apart from having verified nothing when there is no difference, and says nothing about a difference that contradicts the exact-ids bullet — .engineering/planning/story/catalog-jira-cloud-reads.md:36
story:catalog-zendesk-reads — the same "verifies each … records any difference" bullet has no observable, so the verification is a vote — .engineering/planning/story/catalog-zendesk-reads.md:35
story:catalog-confluence-reads — the same "verifies each … records any difference" bullet has no observable, so the verification is a vote — .engineering/planning/story/catalog-confluence-reads.md:38
story:catalog-slack-reads — the same "verifies each … records any difference" bullet has no observable, so the verification is a vote — .engineering/planning/story/catalog-slack-reads.md:34

What I read: 7 artifacts (the epic, five stories, review-result round 1) via `aep plan artifact show`. I also checked `docs/local-catalog-provider.md` and `adapters/catalog/tests/local_runtime.rs` in the tree.

Round 1 status: all eight findings are fixed.

Could not establish: the basic-auth bullet at :31–32 says a below-minimum-scopes refusal is asserted "as the token profile's test does"; `adapters/catalog/tests/local_runtime.rs` has no such refusal assertion, it only checks `granted_scopes` contains `api` (:292). Out of lane: the Slack source is an archived Swagger 2.0 document (covered by the story's decision-blocker fallback); the `bundle_drift` check exists only as the shared `adapters/catalog/tests/bundle_drift.rs`, while each story names a per-provider `<provider>.rs`.

```findings
[
  {"file": ".engineering/planning/epic/catalog-knowledge-sources.md", "line": 51, "category": "acceptance", "severity": "warning", "verdict": "needs-revision", "origin": "introduced", "message": "the third acceptance bullet, \"The four provider stories and `story:catalog-basic-auth-profile` are implemented\", names no observable and restates child status, so it can be voted true"},
  {"file": ".engineering/planning/story/catalog-basic-auth-profile.md", "line": 25, "category": "acceptance", "severity": "warning", "verdict": "needs-revision", "origin": "introduced", "message": "the first bullet is two statements (declare a basic profile; connect accepts the credential document from --credential-file or --credential-stdin) and no later bullet says the tests connect through both entry routes"},
  {"file": ".engineering/planning/story/catalog-jira-cloud-reads.md", "line": 34, "category": "acceptance", "severity": "warning", "verdict": "needs-revision", "origin": "introduced", "message": "the paging test walks one list operation of three with three different end conditions, while the epic requires every list operation to page to its documented end condition"},
  {"file": ".engineering/planning/story/catalog-zendesk-reads.md", "line": 33, "category": "acceptance", "severity": "warning", "verdict": "needs-revision", "origin": "introduced", "message": "the paging test walks one list operation of two, and end_of_stream and meta.has_more are different end conditions, so the epic's every-list-operation paging goes unshown"},
  {"file": ".engineering/planning/story/catalog-confluence-reads.md", "line": 36, "category": "acceptance", "severity": "warning", "verdict": "needs-revision", "origin": "introduced", "message": "the paging test walks one list operation of two (pages.changed and space.pages page differently), so the epic's every-list-operation paging goes unshown"},
  {"file": ".engineering/planning/story/catalog-slack-reads.md", "line": 32, "category": "acceptance", "severity": "warning", "verdict": "needs-revision", "origin": "introduced", "message": "the paging test walks one list operation of two, so conversations.history and conversations.replies are not both shown to stop at the end condition"},
  {"file": ".engineering/planning/story/catalog-jira-cloud-reads.md", "line": 36, "category": "acceptance", "severity": "warning", "verdict": "needs-revision", "origin": "introduced", "message": "\"the unit verifies each against the pinned document and records any difference in the guide\" cannot be told apart from having verified nothing when there is no difference, and says nothing about a difference that contradicts the exact-ids bullet"},
  {"file": ".engineering/planning/story/catalog-zendesk-reads.md", "line": 35, "category": "acceptance", "severity": "warning", "verdict": "needs-revision", "origin": "introduced", "message": "the \"verifies each … records any difference\" bullet has no observable, so the verification is a vote"},
  {"file": ".engineering/planning/story/catalog-confluence-reads.md", "line": 38, "category": "acceptance", "severity": "warning", "verdict": "needs-revision", "origin": "introduced", "message": "the \"verifies each … records any difference\" bullet has no observable, so the verification is a vote"},
  {"file": ".engineering/planning/story/catalog-slack-reads.md", "line": 34, "category": "acceptance", "severity": "warning", "verdict": "needs-revision", "origin": "introduced", "message": "the \"verifies each … records any difference\" bullet has no observable, so the verification is a vote"}
]
```
