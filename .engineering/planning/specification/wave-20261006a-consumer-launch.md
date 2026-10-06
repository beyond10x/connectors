---
format: aep.planning-md/3
id: specification:wave-20261006a-consumer-launch
kind: specification
status: approved
title: 'Wave 20261006a: consumer launch'
revision: 3
transitions:
- {from: "draft", to: "in_review", at: "2026-10-06T04:36:26Z", actor: "agent:claude", revision: 2}
- {from: "in_review", to: "approved", at: "2026-10-06T04:36:27Z", actor: "agent:claude", revision: 3}
---
## Wave 20261006a: consumer launch

Opened 2026-10-06 by the coordinating session, `aep:implementing` 0.19.2 wave mode.

**Approval:** the operator approved every wave up front on 2026-10-05 ("I approve all waves upfront and now. do not ask for permission, you orchestrate this") and delegated every decision until 10:00 on 2026-10-06.

## Units

| unit | story | branch | worktree | build dir |
|---|---|---|---|---|
| U1 | `story:launch-consumer-with-connection-credential` | `unit/launch-consumer` | `connectors-w-launch` | `~/.cache/b10x-target/connectors-w-launch` |

## Rules

- At most two adversary passes; the design is the story's `## Design decisions (2026-10-06)`.
- Package-scoped tests in the unit; the coordinator runs the repository gate on the integration branch.

## Commits approval authorises

One commit for the unit through `b10x-gates bot`; its merge into `wave/20261006a`; the closing planning-store commit; the pull request into `main` and its merge. No release.
