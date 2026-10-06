---
format: aep.planning-md/3
id: specification:wave-20261006b-feed-contract
kind: specification
status: implemented
title: 'Wave 20261006b: datasource.feed contract'
revision: 5
transitions:
- {from: "draft", to: "in_review", at: "2026-10-06T09:58:16Z", actor: "agent:claude", revision: 2}
- {from: "in_review", to: "approved", at: "2026-10-06T09:58:16Z", actor: "agent:claude", revision: 3}
- {from: "approved", to: "implemented", at: "2026-10-06T11:28:26Z", actor: "agent:claude", revision: 5}
---
## Wave 20261006b: datasource.feed contract

Opened 2026-10-06 by the coordinating session, `aep:implementing` 0.19.2 wave mode.

**Approval:** the operator approved every wave up front on 2026-10-05 ("I approve all waves upfront and now. do not ask for permission, you orchestrate this").

## Units

| unit | story | branch | worktree |
|---|---|---|---|
| U1 | `story:feed-contract` | `unit/feed-contract` | `connectors-w2-feed` |

## Commits approval authorises

One commit for the unit through `b10x-gates bot`; its merge into `wave/20261006b`; the closing planning-store commit; the pull request into `main` and its merge. No release.

## Outcome

Closed 2026-10-06. One unit merged into `wave/20261006b`; the repository gate passed in the pull request CI, run 37453226338 (PR #115).

| unit | story | commit |
|---|---|---|
| U1 | `story:feed-contract` | `469107fe8b` |

One `UNMAPPED:` marker stays on purpose (the type of `Body.content` belongs to each records profile). The system header moved to `ess/18` for the container-exists guard; the other 456 scenarios are byte-identical.
