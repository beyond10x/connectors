---
format: aep.planning-md/3
id: story:slack-message-send
kind: story
status: implemented
title: Post a Slack message through the catalog provider
relations:
- decomposes: epic:fluxplane-plugin-parity
- serves: vision:independent-contract-adapters
- depends_on: story:catalog-slack-reads
revision: 6
transitions:
- {from: "draft", to: "proposed", at: "2026-10-13T03:28:52Z", actor: "human:timo", revision: 4}
- {from: "proposed", to: "active", at: "2026-10-13T03:28:52Z", actor: "human:timo", revision: 5}
- {from: "active", to: "implemented", at: "2026-10-13T05:01:18Z", actor: "human:timo", revision: 6, decided_on: {"recorded":{"test_result":1,"review_outcome":1}}}
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

## Decided

Direct messages are in scope (`decision-blocker:slack-message-writes-scope`, option B): `conversations.open` is selected and the `im:write` scope is documented for it, restoring what the predecessor removed (`CHANGELOG.md:2458-2459`). The catalog engine has no composite operation, so the send is two selections (`decision-blocker:wave-20261013a-scope`, option A): `chat.postMessage` posts, `conversations.open` opens a direct message whose channel `chat.postMessage` then names.

## Acceptance

- Spec first: the selection and the Message record are modelled in the adapter's ESS model
  (`adapters/slack/spec/ess`) and the catalog selection, and validated with the newest `ess` before
  implementation.
- `connectors operations invoke` of `chat.postMessage` on a saved connection posts to a channel
  against a recorded provider fixture, and returns the posted message's `channel` and `ts`.
- Without an approval the send is refused before any request leaves the host.
- An ambiguous dispatch records the attempt as `outcome_unknown` and is not retried by itself.
- `conversations.open` naming a user returns the direct message's channel id against a recorded
  fixture, and `chat.postMessage` to that channel posts there; each invocation is its own approval
  and attempt.
- The parity page row of `slack.message.send` moves to covered.

## Out of scope

`slack.message.edit` and `slack.message.delete` stay in `story:parity-slack-message-writes`.

## Issue

https://github.com/beyond10x/connectors/issues/158
