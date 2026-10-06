---
format: aep.planning-md/3
id: specification:wave-20261006d-feed-bindings
kind: specification
status: approved
title: 'Wave 20261006d: feed bindings for Jira, GitLab, Confluence and the Slack provider'
revision: 3
transitions:
- {from: "draft", to: "in_review", at: "2026-10-06T19:55:18Z", actor: "agent:claude", revision: 2}
- {from: "in_review", to: "approved", at: "2026-10-06T19:55:18Z", actor: "agent:claude", revision: 3}
---
## Wave 20261006d: feed bindings for Jira, GitLab, Confluence and the Slack provider

Opened 2026-10-06 by the coordinating session, `aep:implementing` 0.19.2 wave mode.

**Approval:** the operator approved every wave up front on 2026-10-05 ("I approve all waves upfront and now. do not ask for permission, you orchestrate this") and again on 2026-10-06 ("do it. dispatch next waves. i approved them").

## Units

| unit | story | branch | worktree |
|---|---|---|---|
| U1 | `story:jira-feed-binding` | `unit/jira-feed-binding` | `connectors-w4-jira` |
| U2 | `story:gitlab-feed-binding` | `unit/gitlab-feed-binding` | `connectors-w4-gitlab` |
| U3 | `story:confluence-feed-binding` | `unit/confluence-feed-binding` | `connectors-w4-confluence` |
| U4 | `story:catalog-slack-reads` | `unit/catalog-slack-reads` | `connectors-w4-slack` |

## Commits approval authorises

One commit per unit through `b10x-gates bot`; their merges into `wave/20261006d`; the closing planning-store commit; the pull request into `main` and its merge. No release.
