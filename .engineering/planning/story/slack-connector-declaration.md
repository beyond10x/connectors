---
format: aep.planning-md/3
id: story:slack-connector-declaration
kind: story
status: implemented
title: Slack's bot selection ships a connector declaration, so legacy Slack entries migrate
summary: Declare connector slack (profile slack.bot, operations.json) from connectors-definitions slack.yaml; 0.63.1
tags:
- d298
- d313
relations:
- decomposes: epic:connector-vocabulary
- serves: vision:independent-contract-adapters
revision: 4
transitions:
- {from: "draft", to: "proposed", at: "2026-10-31T00:29:17Z", actor: "human:timo", revision: 2, executor: "agent:conn-d298-u1", correlation: "wave-d298-u1"}
- {from: "proposed", to: "active", at: "2026-10-31T00:29:17Z", actor: "human:timo", revision: 3, executor: "agent:conn-d298-u1", correlation: "wave-d298-u1"}
- {from: "active", to: "implemented", at: "2026-10-31T02:50:07Z", actor: "human:timo", revision: 4, decided_on: {"recorded":{"test_result":1,"verification":1}}, executor: "agent:conn-d298", correlation: "wave-d298"}
---
## Problem

connectors 0.62.1 and 0.63.0 ship no connector declaration for Slack:
`adapters/catalog/providers/slack/` holds only `operations.json` and `user-operations.json`. So
`--connector slack --instance <name>` cannot be derived, and `setup migrate` keeps every legacy
Slack entry as `[adapters.<alias>]` with the reason `connector_undeclared`
(`apps/connectors/src/local/migrate.rs`, `connector_of`). Since 0.60.0 such an entry is no instance
(`legacy_entry_unmigrated`). A downstream deployment that ingests Slack is blocked on this today.

The definitions already exist in `beyond10x/connectors-definitions`: `connectors/slack.yaml` (the
bot selection, profile `slack.bot`) and `connectors/slack-user.yaml` (`catalog: slack`, the user
selection, profiles `slack.user` and `slack.bot`). `decision-blocker:slack-openrouter-pinned-source`
decided that Slack stays two connectors, `slack` and `slack-user`, as the definitions are.

## Outcome

- `adapters/catalog/providers/slack/connector.json` is the declaration `connectors-build definition`
  lowers from `connectors/slack.yaml`: connector `slack`, provider `slack`, default endpoint
  `https://slack.com/api`, selection `operations.json`, profile `slack.bot`.
- The `connectors` binary compiles it in (`apps/connectors/src/local/declarations.rs`), so
  `setup migrate` moves a legacy Slack entry, bot or user selection, to a pinned instance of the
  connector `slack` under its alias, every pin unchanged.
- The user selection gets no declaration here: a catalog directory holds one `connector.json`,
  and lowering `slack-user.yaml` writes the same path. A user-token instance stays pinned; a
  derived instance refuses the profile `slack.user`. Declaring `slack-user` is
  `story:slack-user-connector-declaration`.

## Acceptance

- `adapters/catalog/tests/connector_declarations.rs`
  `slack_derived_from_its_declaration_is_the_documented_bot_instance`: `--connector slack
  --instance work` derives the Slack guide's bot configuration, revision included; `--profile
  slack.user` is refused.
- `every_catalog_provider_but_openrouter_ships_one_declaration` and
  `every_declared_profile_derives_what_its_hand_written_configuration_derives` hold Slack's
  declaration like every other.
- `apps/connectors/tests/setup_migrate.rs`
  `both_slack_entries_migrate_to_pinned_instances_of_slack`: a bot entry and a user entry both
  migrate to `[connectors.slack.instances.<alias>]` with identical before and after records;
  `each_kind_is_migrated_or_kept_naming_why_and_keeps_its_records` migrates the Slack entry and
  keeps an OpenRouter one as `connector_undeclared`.
- `crates/connectors-catalog/tests/definition_import.rs`: the importer's Slack definition lowers to
  the committed declaration byte for byte.

## Release

Ships alone as 0.63.1 under D169 (a fix a consumer is blocked on today), approved as D313.
