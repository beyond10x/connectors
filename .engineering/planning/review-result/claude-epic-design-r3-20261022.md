---
format: aep.planning-md/3
id: review-result:claude-epic-design-r3-20261022
kind: review-result
status: active
title: 'Plan-critic design, round 3: connectors with Claude'
summary: needs-revision, no blocker; addressed
relations:
- reviews: story:readme-names-mounted-custody
- reviews: story:claude-connects-to-the-mcp-server
- reviews: story:agent-tool-projection
- reviews: story:standing-write-approval-for-unattended-agents
revision: 1
---
# Plan-critic, design lens, round 3: connectors with Claude

**Verdict: needs-revision. 4 findings, no blocker. All are addressed in the revisions of 2026-10-22 after round 3.**

| Story | Finding | Disposition |
|---|---|---|
| readme-names-mounted-custody | not independent of docs-first-run-path | folded into story:docs-first-run-path and archived |
| claude-connects | the feed block adds 2 reads to the Jira `tools/list` (`adapters/catalog/src/lib.rs:1579-1597`) | `depends_on story:definition-reader-feed-extensions` |
| agent-tool-projection | adds files under directories the deletion stories remove | depends on story:instances-resolve-through-locked-index; the agent definitions arrive through the embedded release; it adds nothing under `adapters/catalog/providers/` or the vendored copy; the two deletion stories no longer depend on it |
| standing-write | gated on its own ADR | the ADR is `architecture-decision-record:standing-scoped-approval`, reviewed in the story's pull request before the implementation commits, with acceptance lines |

Noted, not findings: story:catalog-feed-engine-extensions is `active`; its code is on `main` since #197 (`75efd7263`, released in 0.56.0), and the 0.56.0 close kept the wave stories active. The rate counter's location and the unobserved Secret-update behaviour are now written into the standing-write body.
