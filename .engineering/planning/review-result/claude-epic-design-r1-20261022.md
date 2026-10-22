---
format: aep.planning-md/3
id: review-result:claude-epic-design-r1-20261022
kind: review-result
status: active
title: 'Plan-critic design, round 1: connectors with Claude'
summary: needs-revision; findings addressed in revision 2
relations:
- reviews: epic:connectors-with-claude
- reviews: story:claude-getting-started-tutorial
- reviews: story:claude-connects-to-the-mcp-server
- reviews: story:agent-tool-projection
- reviews: story:definition-reader-feed-extensions
revision: 1
---
# Plan-critic, design lens, round 1: connectors with Claude and the definition feed story

**Verdict: needs-revision. 8 findings, 2 blockers. All are addressed in revision 2 of each body.**

| Story | Finding | Disposition in revision 2 |
|---|---|---|
| projection (blocker) | trimming inputs and outputs conflicts with `projection.md:141-146`, while the epic promises no MCP contract change | a projection is a selection. `adapters/catalog/src/lib.rs:1352-1356` already removes fixed parameters from the declared input. Trimming answers is out of the epic |
| projection (blocker) | the first projections needed write tools that no story builds | writes removed; the epic names write tools over MCP as out of scope |
| projection | a second place to fix and default inputs | none: `query_fixed` and `description` stay in the selection |
| projection | `operations.json` is deleted by epic:catalog-connectors-only-in-definitions | selections named by their connectors-definitions identity (`jira-agent.yaml`, `slack-agent.yaml`) |
| mcp | it duplicated the runtime story's list and call tests | only Claude-specific tests remain; the runtime story owns the generic ones |
| tutorial | it shared `getting-started.md` and guides with story:docs-first-run-path without an edge | `depends_on story:docs-first-run-path` |
| tutorial | Slack by name needs story:instances-resolve-through-locked-index | `depends_on` recorded |
| feed | double claim on the shipped `feed` block with the binding stories | this story owns it; story:jira-feed-binding and story:confluence-feed-binding `depends_on` it |
