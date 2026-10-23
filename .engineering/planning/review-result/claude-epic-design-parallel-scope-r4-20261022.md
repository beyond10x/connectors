---
format: aep.planning-md/3
id: review-result:claude-epic-design-parallel-scope-r4-20261022
kind: review-result
status: active
title: 'Plan-critic design-parallel-scope, round 4: connectors with Claude'
summary: needs-revision, 6 warnings; addressed
relations:
- reviews: story:mcp-stdio-runtime-branch
- reviews: story:docs-first-run-path
- reviews: story:mounted-custody-kind-secret-volume
- reviews: story:agent-tool-projection
- reviews: story:claude-connects-to-the-mcp-server
revision: 1
---
# Plan-critic, design, parallel-safety and scope lenses, round 4: connectors with Claude

**Verdicts: needs-revision on each. 6 warnings, no blocker. All are addressed in the revisions after round 4.**

| Lens | Story | Finding | Disposition |
|---|---|---|---|
| design | mcp-stdio-runtime-branch | its README change is MCP claim prose inside the block docs-first-run-path replaces, and the only reason for its edge | the README change is not re-landed; `README.md` leaves its scope; the edge to docs-first-run-path is removed, so it can run in wave A |
| design | docs-first-run-path | no destination for the install section `README.md:339-383` | `website/docs/getting-started.md` § "Or install a release binary" (`:32`), replacing the link at `:44`; the tutorial rewrites it afterwards. The support table (`README.md:284`) is kept whole |
| parallel-safety | mounted-custody-kind-secret-volume | `ignored.rs` shared with the runtime story and the tutorial, no edge | depends on story:mcp-stdio-runtime-branch; the tutorial depends on it |
| parallel-safety | mcp-stdio-runtime-branch | `ignored.rs` missing from scope | added |
| parallel-safety | agent-tool-projection | scope still listed the two `agent-operations.json` files and `main.rs` | removed; the body names the lock story:definitions-pin-covers-catalog creates (path not fixed yet) |
| scope | claude-connects-to-the-mcp-server | the closing request for the connectors-definitions files was only in the epic | added to its Ownership |

Noted, not changed: the README Jira and Confluence rows (`README.md:291-292`) say nothing about feeds, so story:definition-reader-feed-extensions does not edit README. The duplicated heading in docs-first-run-path is removed.
