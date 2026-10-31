---
format: aep.planning-md/3
id: story:slack-user-connector-declaration
kind: story
status: draft
title: Slack's user selection ships its own declaration as the connector slack-user
summary: One declaration per selection in a catalog directory, so slack-user derives with slack.user
tags:
- d298
relations:
- decomposes: epic:connector-vocabulary
- serves: vision:independent-contract-adapters
revision: 1
---
## Problem

`story:slack-connector-declaration` declares Slack's bot selection as the connector `slack`. The
user selection (`adapters/catalog/providers/slack/user-operations.json`, profile `slack.user`,
search) still has no declaration, so a user-token instance must be pinned.
`decision-blocker:slack-openrouter-pinned-source` decided Slack is two connectors, `slack` and
`slack-user`, as `beyond10x/connectors-definitions` has them (`connectors/slack-user.yaml`,
`catalog: slack`).

The layout does not hold a second declaration: a catalog directory holds one `connector.json`
(`adapters/catalog/build.rs`, `adapters/catalog/src/local.rs` `declaration`, which requires
`catalog_provider` to equal the directory), and `connectors-build definition` writes the lowered
declaration of every definition of one catalog to `<out>/<catalog>/connector.json`
(`crates/connectors-catalog/src/definition.rs`, `lower`), so `slack-user.yaml` overwrites
`slack.yaml`'s.

## Outcome

- A catalog directory may hold one declaration per selection, named after it (for example
  `user-connector.json` beside `user-operations.json`), read by the build script, the provider,
  `connectors list`, `setup migrate` and the docs index alike.
- The lowerer writes each definition's declaration to its own file.
- The connector `slack-user` is derived from its declaration with the profile `slack.user`.

## Acceptance (draft)

- `--connector slack-user --instance <name>` derives the Slack guide's user configuration and its
  revision.
- Lowering `slack.yaml` and `slack-user.yaml` into one output directory keeps both declarations.
- `setup migrate` maps a legacy user-selection entry to `slack-user`.
