---
format: aep.planning-md/3
id: story:slack-reactions-and-channel-info
kind: story
status: draft
title: Slack bot adds a reaction and reads a channel's info
revision: 1
---
## Need

An operator's agents post Slack updates and acknowledge a thread with a reaction. The `slack`
connector's selection reads threads and posts as the bot, but leaves reactions and channel info
out (docs/catalog-slack.md:941), so those calls go around Connectors today.

## Acceptance

- The `slack` connector (bot profile) offers `reactions.add` (a write, under the same approval and
  standing-grant rules as `chat.postMessage`, scoped by channel) and `conversations.info` (a read),
  both from Slack's published document, in connectors-definitions and the engine's fixture copy.
- A served-host test adds a reaction to a message in a granted channel, refuses one in another
  channel before any request, and reads a channel's info.
- docs/catalog-slack.md lists both and names the scopes they need (`reactions:write`,
  `channels:read`).
