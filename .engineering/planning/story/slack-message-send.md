---
format: aep.planning-md/3
id: story:slack-message-send
kind: story
status: draft
title: Post a Slack message through the catalog provider
relations:
- decomposes: epic:fluxplane-plugin-parity
- serves: vision:independent-contract-adapters
- depends_on: story:catalog-slack-reads
revision: 2
---
## Outcome

A user posts a Slack message through `connectors operations invoke` on a saved `slack` catalog
connection: `slack.message.send`, the operation with 132 calls since 2026-09-09
(`docs/fluxplane-plugin-parity.md:479`), parity unit U08 (`adapters/slack/design.md:52`).

## Route

The catalog provider `slack`, over the pinned Slack Web API document already in
`adapters/slack/upstream/`: `chat.postMessage`, plus `conversations.open` when the target is a
direct message. Both methods are in that document. The write runs under the approval and attempt
rules every catalog write has (approval key, attempt record, `outcome_unknown` on an ambiguous
dispatch); no new host mechanism.

## Held

Delivery waits on `decision-blocker:slack-message-writes-scope`: channel posts only, or channel posts
and direct messages. The predecessor removed the direct-message scopes on purpose
(`CHANGELOG.md:2458-2459`). Under option A, `conversations.open` is not selected and no `im:*` scope
is declared.

## Acceptance

- Spec first: the selection and the Message record are modelled in the adapter's ESS model
  (`adapters/slack/spec/ess`) and the catalog selection, and validated with the newest `ess` before
  implementation.
- `connectors operations invoke` of `slack.message.send` on a saved connection posts to a channel
  against a recorded provider fixture, and returns the posted message's `channel` and `ts`.
- Without an approval the send is refused before any request leaves the host.
- An ambiguous dispatch records the attempt as `outcome_unknown` and is not retried by itself.
- Only if option B is chosen: a send naming a user opens the direct message with
  `conversations.open` and posts to the returned channel, in one attempt.
- The parity page row of `slack.message.send` moves to covered.

## Out of scope

`slack.message.edit` and `slack.message.delete` stay in `story:parity-slack-message-writes`.

## Issue

https://github.com/beyond10x/connectors/issues/158
