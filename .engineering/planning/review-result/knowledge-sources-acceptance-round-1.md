---
format: aep.planning-md/3
id: review-result:knowledge-sources-acceptance-round-1
kind: review-result
status: active
title: Acceptance critic, round 1
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
epic:catalog-knowledge-sources — the body has an Outcome and a requirements list but no Acceptance section, so "reads … the way it reads GitLab today" can only be asserted closed; sibling epics carry one — .engineering/planning/epic/catalog-knowledge-sources.md:12 (compare .engineering/planning/epic/retire-native-gitlab-adapter.md:56)
story:catalog-basic-auth-profile — "The identity read and minimum-scope checks work as for the token profile" names no check, output or refusal that shows it, so it can be voted true — .engineering/planning/story/catalog-basic-auth-profile.md:21
story:catalog-basic-auth-profile — "neither appears in TOML, arguments or environment" names no check (a scan, a fixture assertion) that shows the absence — .engineering/planning/story/catalog-basic-auth-profile.md:20
story:catalog-confluence-reads — the operation says "filtered or sorted by last modified" while the acceptance requires documenting "its time-window filter", so a sort-only endpoint passes and the epic's delta requirement goes unmet; the acceptance names no request that shows the storage-format body was asked for — .engineering/planning/story/catalog-confluence-reads.md:18
story:catalog-jira-cloud-reads — "stable ids" is unobservable because the acceptance names no ids and no test that pins them, though the epic requires ids stable across releases — .engineering/planning/story/catalog-jira-cloud-reads.md:24
story:catalog-zendesk-reads — "stable ids" is unobservable because the acceptance names no ids and no test that pins them — .engineering/planning/story/catalog-zendesk-reads.md:23
story:catalog-confluence-reads — "stable ids" is unobservable because the acceptance names no ids and no test that pins them — .engineering/planning/story/catalog-confluence-reads.md:23
story:catalog-slack-reads — "stable ids" is unobservable because the acceptance names no ids and no test that pins them — .engineering/planning/story/catalog-slack-reads.md:22

Could not establish: Slack publishes no official OpenAPI document, and Jira's `/search` endpoint is being replaced by `/search/jql` (scope/design lane); all four read stories write under `adapters/catalog/providers/` and edit the same provider guide (parallel-safety lane).

Recorder note: the findings block below is the critic's YAML block converted to JSON with unchanged content, because the store could not parse the YAML (quoted values).

```findings
[
  {
    "file": ".engineering/planning/epic/catalog-knowledge-sources.md",
    "line": 12,
    "category": "acceptance",
    "severity": "blocker",
    "verdict": "needs-revision",
    "origin": "introduced",
    "message": "the body has an Outcome and a requirements list but no Acceptance section, so \"reads … the way it reads GitLab today\" can only be asserted closed; sibling epics carry one"
  },
  {
    "file": ".engineering/planning/story/catalog-basic-auth-profile.md",
    "line": 21,
    "category": "acceptance",
    "severity": "blocker",
    "verdict": "needs-revision",
    "origin": "introduced",
    "message": "\"The identity read and minimum-scope checks work as for the token profile\" names no check, output or refusal that shows it, so it can be voted true"
  },
  {
    "file": ".engineering/planning/story/catalog-basic-auth-profile.md",
    "line": 20,
    "category": "acceptance",
    "severity": "warning",
    "verdict": "needs-revision",
    "origin": "introduced",
    "message": "\"neither appears in TOML, arguments or environment\" names no check (a scan, a fixture assertion) that shows the absence"
  },
  {
    "file": ".engineering/planning/story/catalog-confluence-reads.md",
    "line": 18,
    "category": "acceptance",
    "severity": "blocker",
    "verdict": "needs-revision",
    "origin": "introduced",
    "message": "the operation says \"filtered or sorted by last modified\" while the acceptance requires documenting \"its time-window filter\", so a sort-only endpoint passes and the epic delta requirement goes unmet; the acceptance names no request that shows the storage-format body was asked for"
  },
  {
    "file": ".engineering/planning/story/catalog-jira-cloud-reads.md",
    "line": 24,
    "category": "acceptance",
    "severity": "warning",
    "verdict": "needs-revision",
    "origin": "introduced",
    "message": "\"stable ids\" is unobservable because the acceptance names no ids and no test that pins them, though the epic requires ids stable across releases"
  },
  {
    "file": ".engineering/planning/story/catalog-zendesk-reads.md",
    "line": 23,
    "category": "acceptance",
    "severity": "warning",
    "verdict": "needs-revision",
    "origin": "introduced",
    "message": "\"stable ids\" is unobservable because the acceptance names no ids and no test that pins them"
  },
  {
    "file": ".engineering/planning/story/catalog-confluence-reads.md",
    "line": 23,
    "category": "acceptance",
    "severity": "warning",
    "verdict": "needs-revision",
    "origin": "introduced",
    "message": "\"stable ids\" is unobservable because the acceptance names no ids and no test that pins them"
  },
  {
    "file": ".engineering/planning/story/catalog-slack-reads.md",
    "line": 22,
    "category": "acceptance",
    "severity": "warning",
    "verdict": "needs-revision",
    "origin": "introduced",
    "message": "\"stable ids\" is unobservable because the acceptance names no ids and no test that pins them"
  }
]
```
