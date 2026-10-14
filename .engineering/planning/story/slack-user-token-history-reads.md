---
format: aep.planning-md/3
id: story:slack-user-token-history-reads
kind: story
status: active
title: Slack history and replies on a user token
relations:
- decomposes: epic:fluxplane-plugin-parity
- serves: vision:independent-contract-adapters
revision: 4
transitions:
- {from: "draft", to: "proposed", at: "2026-10-14T23:35:15Z", actor: "human:timo", revision: 3}
- {from: "proposed", to: "active", at: "2026-10-14T23:35:15Z", actor: "human:timo", revision: 4}
---
## Outcome

A Slack connection on a user token reads channel history and thread replies of every channel that
user can read, through the catalog provider, and the parity page scores the Slack history and
replies rows by what a user token can do.

## Today

Reported by a downstream consumer on connectors 0.50.0, 2026-10-14: the Slack user profile offers 0
operations, so reads run on the bot token and reach 13 of 54 channels (the channels the bot is a
member of). 4 operations, all used every round. `docs/fluxplane-plugin-parity.md` scores these rows
covered, which is wrong for a user who needs the channels the bot is not in.

## Change

Spec first: the Slack catalog selection gives the user-token profile the reads
`conversations.list`, `conversations.history`, `conversations.replies` and the user lookups, under
the user scopes they need (`channels:history`, `groups:history`, `im:history`, `mpim:history`,
`channels:read`, `groups:read`, `users:read`). No write operation on the user profile. Correct the
parity page rows to partial until this ships, then covered with the operation named.

## Acceptance

- Each read answers through `connections` of the user profile against a recorded provider fixture.
- The parity page names the user profile for these rows.

## Scope

Primary: adapters/catalog/providers/slack/user-operations.json, adapters/catalog/tests/slack.rs; docs fluxplane-plugin-parity.md § slack, catalog-slack.md, status.rs "Slack conversations", CHANGELOG. Not cli.yaml, not apps/connectors, not adapters/catalog/src. Open: which "user lookups" (users.list vs users.info/lookupByEmail).
