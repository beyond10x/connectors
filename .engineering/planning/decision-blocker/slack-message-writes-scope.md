---
format: aep.planning-md/3
id: decision-blocker:slack-message-writes-scope
kind: decision-blocker
status: open
title: Nobody has decided whether Slack message sends may reach direct messages
relations:
- blocks: story:slack-message-send
- blocks: story:parity-slack-message-writes
- serves: vision:independent-contract-adapters
revision: 1
---
## Question

How far may `slack.message.send` reach: channels only, or channels and direct messages?

`slack.message.send` has 132 calls since 2026-09-09 (`docs/fluxplane-plugin-parity.md:479`). The
predecessor removed the `im:*` and `mpim:*` scopes on purpose, and withheld the parameter that
would reach a direct message (`CHANGELOG.md:2458-2459`, 0.5.9). A direct message needs
`conversations.open` before `chat.postMessage` (`adapters/slack/design.md:52`). Both methods are in
the pinned Slack document (`adapters/slack/upstream/slack_web_openapi_v2_without_examples.json`).

## Options

| option | what | cost |
|---|---|---|
| A | Channel posts only: select `chat.postMessage` for a conversation id the bot is already a member of. `conversations.open` is not selected and no `im:*`/`mpim:*` scope is declared. | One catalog selection under the approval and attempt rules every catalog write has. A user who wants a direct message keeps the old tool for it. |
| B | Channel posts and direct messages: A plus `conversations.open` and the `im:write` scope, so a send can name a user. | As A, plus one selection and a broader reach: a wrong user id puts a message in a person's direct messages. Reverses the predecessor's 0.5.9 removal. |
| C | Neither: Slack stays read-only. | No work. The 132 send calls stay on the old tool; users who cannot run it cannot post. |

Recommended: A. It gives posting to the users who need it with the narrower reach; direct messages
can follow as their own story when asked.

Default on silence: C (nothing is built).
