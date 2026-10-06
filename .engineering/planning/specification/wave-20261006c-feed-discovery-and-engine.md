---
format: aep.planning-md/3
id: specification:wave-20261006c-feed-discovery-and-engine
kind: specification
status: approved
title: 'Wave 20261006c: feed discovery and the catalog feed engine'
revision: 3
transitions:
- {from: "draft", to: "in_review", at: "2026-10-06T17:00:05Z", actor: "agent:claude", revision: 2}
- {from: "in_review", to: "approved", at: "2026-10-06T17:00:05Z", actor: "agent:claude", revision: 3}
---
## Wave 20261006c: feed discovery and the catalog feed engine

Opened 2026-10-06 by the coordinating session, `aep:implementing` 0.19.2 wave mode.

**Approval:** the operator approved every wave up front on 2026-10-05 ("I approve all waves upfront and now. do not ask for permission, you orchestrate this") and again on 2026-10-06 ("do it. dispatch next waves. i approved them").

## Units

| unit | story | branch | worktree |
|---|---|---|---|
| U1 | `story:feed-bindings-discoverable` | `unit/feed-bindings-discoverable` | `connectors-w3-disc` |
| U2 | `story:catalog-feed-engine` | `unit/catalog-feed-engine` | `connectors-w3-engine` |

## Commits approval authorises

One commit per unit through `b10x-gates bot`; their merges into `wave/20261006c`; the closing planning-store commit; the pull request into `main` and its merge. No release.
