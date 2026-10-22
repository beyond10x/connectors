---
format: aep.planning-md/3
id: review-result:claude-epic-parallel-safety-r1-20261022
kind: review-result
status: active
title: 'Plan-critic parallel-safety, round 1: connectors with Claude'
summary: needs-revision; findings addressed in revision 2
relations:
- reviews: epic:connectors-with-claude
- reviews: story:claude-getting-started-tutorial
- reviews: story:claude-connects-to-the-mcp-server
- reviews: story:agent-tool-projection
- reviews: story:definition-reader-feed-extensions
revision: 1
---
# Plan-critic, parallel-safety lens, round 1: connectors with Claude and the definition feed story

**Verdict: needs-revision. 9 findings, 3 blockers. 7 are addressed in revision 2; the 2 on story:docs-first-run-path's own body and the 1 on story:mcp-stdio-runtime-branch are pre-existing and are settled by ordering edges, not by edits to those bodies.**

| Item | Finding | Disposition |
|---|---|---|
| feed (blocker) | `jira.md` shared with the tutorial; falsified claims not in Scope | the tutorial no longer edits `jira.md` or `slack.md`; the feed Scope lists the 5 claims including `status.rs:309-317` |
| docs-first-run-path (blocker) | 5 files shared with the tutorial, no edge | tutorial `depends_on story:docs-first-run-path` |
| docs-first-run-path (blocker) | `README.md` shared with the MCP story and the runtime branch | MCP story `depends_on` the tutorial, which depends on docs-first-run-path; the runtime branch lands in a wave without docs-first-run-path |
| mcp | `getting-started.md` edited without an edge | link edit removed; the tutorial owns it |
| mcp | `onboarding.rs:493-497`, `:919-923` and the generated index unnamed | named in Scope |
| projection | no code surface named | `definition.rs`, `import.rs`; `depends_on story:definition-reader-feed-extensions` |
| projection | the measured Jira selection changes under the feed story | the same `depends_on` edge |
| mcp-stdio-runtime-branch (pre-existing) | a 195-file branch with no named paths | not changed here; that story narrows into units when it is scoped for its wave |
| tutorial | generated pages in the `cargo` sweep | sources named: `crates/connectors-docs/src/crates.rs:148`, `contracts/datasources/feed/v1alpha1/semantics.md:231` |
