---
format: aep.planning-md/3
id: decision-blocker:wave-20261013a-scope
kind: decision-blocker
status: cleared
title: What wave 20261013a delivers toward the Slack message send
relations:
- blocks: story:slack-message-send
- serves: vision:independent-contract-adapters
revision: 2
transitions:
- {from: "open", to: "cleared", at: "2026-10-13T03:28:36Z", actor: "human:timo", revision: 2}
---
## Question

What wave 20261013a delivers, released as 0.47.0.

## Options

| option | what | cost |
|---|---|---|
| A | `story:slack-message-send` on the decided scope (channels and direct messages): the Slack catalog provider selects `chat.postMessage` and `conversations.open` as writes, each under the approval and attempt rules every catalog write has. A direct message is `conversations.open` naming the user, then `chat.postMessage` to the channel it returns. | One tree, package gates of the catalog crates (under 10G of target). No new host mechanism. |
| B | As A, with one composite `slack.message.send` operation that opens and posts in one attempt. | The catalog engine has no composite operation; it would need a new engine mechanism and its own specification step. |

Recommended: A. The selection works with the engine as it is; a composite operation is a later story if a caller needs it.

Default on silence: A.

## Decided

Option A, 2026-10-13: wave 20261013a delivers story:slack-message-send as two catalog write selections, `chat.postMessage` and `conversations.open`, in the Slack catalog provider, with the `im:write` scope documented for `conversations.open`. A direct message takes two invocations, each with its own approval and attempt record. One integration branch, one pull request, released as 0.47.0.
