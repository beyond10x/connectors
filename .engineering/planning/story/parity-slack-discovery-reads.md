---
format: aep.planning-md/3
id: story:parity-slack-discovery-reads
kind: story
status: implemented
title: Slack search, channels, users and workspace info
relations:
- decomposes: epic:fluxplane-plugin-parity
- serves: vision:independent-contract-adapters
- depends_on: story:catalog-slack-reads
revision: 6
transitions:
- {from: "draft", to: "proposed", at: "2026-10-09T04:56:05Z", actor: "human:timo", revision: 4, decided_on: {"recorded":{"test_result":1}}}
- {from: "proposed", to: "active", at: "2026-10-09T04:56:05Z", actor: "human:timo", revision: 5, decided_on: {"recorded":{"test_result":1}}}
- {from: "active", to: "implemented", at: "2026-10-09T04:56:05Z", actor: "human:timo", revision: 6, decided_on: {"recorded":{"test_result":1}}}
---
## Outcome

Slack search, channels, users and workspace info — parity unit U02 of `docs/fluxplane-plugin-parity.md`.

## Operations

`slack.search`, `slack.user.list`, `slack.channel.list`, `slack.info`, `slack.test`, `slack.emoji.list` (`slack.thread` and `slack.message.list` are `story:catalog-slack-reads`)

512 calls since 2026-09-09 (declared and mapped undeclared names), provider `slack`, planned wave W1.

## Surface

catalog provider after `story:catalog-slack-reads` (threads and history); a user-token profile for search.

## Acceptance

- Spec first: what the unit adds is modelled in the ESS specification it belongs to (adapter model, or pinned OpenAPI source and selection) and validated with the newest `ess` before implementation.
- Each operation above answers through `connectors operations invoke` on a saved connection, against a recorded provider fixture, with the same capability the fluxplane operation gives (the parity page names the gap per operation).
- The parity page row of each operation moves to covered, with the Connectors operation named.

## Note

The 512 calls include the 2 operations `story:catalog-slack-reads` delivers.

## Domain draft

Drafted 2026-10-07: `adapters/slack/spec/ess` (`connectors_slack`, 5 nouns: conversation, message, user, file, emoji; every relation UNMAPPED) and `adapters/slack/design.md`. Route: the catalog provider over Slack's pinned Swagger 2.0 document, after `story:catalog-swagger2-projection`. Open before scheduling: the UNMAPPED markers; search needs a user token while a catalog connection holds one auth profile; the planned document comes from an archived upstream repository. `slack.channel.list` is served by `conversations.list`, which `story:catalog-slack-reads` already selects.

## Wave 20261009a result

Written on wave/20261009a (unit commits fb41924d5, 4273a3ceb, 8bc743fec, merged cc28fb83f): `users.list`, `team.info`, `emoji.list`, `auth.test` on the bot-token connection, and `search.messages` on a second connection with a user-token profile (`user-operations.json`), with no host change.

The pinned Slack document marks `token` required on four of these methods; a new selection field `credential` names a parameter the auth profile carries instead, so it is never declared as input nor sent (documented in docs/local-catalog-provider.md, with refusal tests in adapters/catalog/tests/selection_credential.rs). Single-user reads by id or e-mail are partial: story:slack-single-user-lookups.

Not yet compiled or run: package gates of connectors-catalog are owed to the build; the engine change gets an adversary pass.
