---
format: aep.planning-md/3
id: review-result:slack-message-send-adversary-20261013
kind: review-result
status: active
title: Adversary review of the Slack message send
relations:
- reviews: story:slack-message-send
revision: 1
---
## Adversary review of the Slack message send

Reviewed on 2026-10-13: commit 4b938b086 (`chat.postMessage` and `conversations.open` as catalog writes). The cases are in `adapters/catalog/tests/slack_send_adversary.rs`, commit f9f493d42. Verdict: needs change. 7 cases, 4 red.

| # | finding | verdict | fixed by |
|---|---|---|---|
| 1 | The after-write check compared `/ok` as text, so an acknowledgement with `ok` as the string `"true"` beside a Slack error was recorded as applied. | infeasible (Slack sends a boolean) | 563ba71d1: `{"boolean": true}` guard expectation |
| 2 | The `auth.test` pre-write check compared `/ok` as text, so a string `"true"` with `invalid_auth` let the post be sent. | infeasible | 563ba71d1 |
| 3 | An empty `text` or `channel` was sent, although the Slack model requires both non-empty. The attempt was left `outcome_unknown`. | needs change | 229b9f482: `body_nonempty` |
| 4 | An empty `users` was sent to `conversations.open`. | needs change | 229b9f482 |

These attacks held:
- An ambiguous acknowledgement (no `ok`, a non-JSON body, 429, 5xx, a lost response) is unknown and sent once.
- A failed `auth.test` refuses before anything is sent.
- A member that is not admitted is refused.
- The token never reaches the body, the query or an error.
- The media-type amendment touches only the two writes.

Follow-ups found during the fixes, recorded here and not in a new issue:
- Three other guards compared a JSON boolean as text (GitLab `/resolvable` and `/merge_when_pipeline_succeeds`, Jira `/transitions/0/isAvailable`). 229b9f482 moves them to `{"boolean": true}`, and a load now refuses `literal: "true"` and `literal: "false"`.
- Still open: GitLab `merge_request.discussion.resolve` compares `/resolved` with `{"input": "body.resolved"}` as text. An input reference to a boolean-typed body key is not compared by JSON type.
