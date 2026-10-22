---
format: aep.planning-md/3
id: review-result:claude-epic-scope-r1-20261022
kind: review-result
status: active
title: 'Plan-critic scope, round 1: connectors with Claude'
summary: needs-revision; findings addressed in revision 2
relations:
- reviews: epic:connectors-with-claude
- reviews: story:claude-getting-started-tutorial
- reviews: story:claude-connects-to-the-mcp-server
- reviews: story:agent-tool-projection
- reviews: story:definition-reader-feed-extensions
revision: 1
---
# Plan-critic, scope lens, round 1: connectors with Claude and the definition feed story

**Verdict: needs-revision. 3 findings, no blockers. All are addressed in revision 2. 15 of 15 promises are traced to an item; nothing excluded (documentation skill or agent, OS chooser) is drafted.**

| Story | Finding | Disposition in revision 2 |
|---|---|---|
| tutorial | the `cargo` ban was narrowed to the site; Timo said "across all the documentation" | the rule covers `website/docs/`, `README.md` and `docs/*.md`; 3 developer pages keep their commands under a Development heading |
| feed | the six fields omitted `precision`, `page`, the query struct and `parent` | a field table from `jira.yaml` and `confluence-v1.yaml` at `536945d`; `parent` accepted as a pointer |
| feed | the summary's rules were promised but not all tested | 10 rules, 13 named refusals |
