---
format: aep.planning-md/1
id: story:one-placement-several-credentials
kind: story
status: proposed
title: One placement, several credentials, a Connection per identity
tags:
- ready
- wave-cli
scope:
- confidence: cited
  path: crates/connectors-config/src/personal.rs
- confidence: cited
  path: crates/connectors-console/src/auth.rs
- confidence: cited
  path: crates/connectors-console/src/enrol.rs
- confidence: inferred
  path: crates/connectors-console/src/output_tests.rs
- confidence: cited
  path: crates/integration-catalog/src/hosted.rs
- confidence: cited
  path: crates/integration-catalog/src/lib.rs
- confidence: cited
  path: docs/guides/connect-slack.md
- confidence: inferred
  path: ess/system/domains/connection.yaml
- confidence: inferred
  path: ess/system/domains/deployment.yaml
revision: 29
---
# Story: one placement, several credentials, a Connection per identity

## Goal
A personal-local `[[catalog]]` placement holds one credential and answers as one identity
(`CatalogIntegrationConfig.credential`, `crates/connectors-config/src/personal.rs`; the Connection
reference is derived from provider + instance). A Slack app is two identities at once: its bot
token posts as the app, the operator's user token (issued to the same app) reads what the operator
can read. Today that needs two placements with two instance names (`timo-ai` for the bot token,
`timo-ai-user` for the user token), and nothing records that they are one app — the keyring already
holds five credential slots per instance, so the storage knows, the placement model does not.

## Shape
- A placement may declare `credentials = ["slack.bot_token", "slack.user_token"]` (or keep the
  single `credential`); each declared credential yields its own Connection,
  `connection:<provider>:<instance>` for the first and `connection:<provider>:<instance>/<leaf>` for
  the others, all labelled from the one placement and reported together by `connection list` with
  the credential each answers as.
- `connectors connect <provider> --instance <i> --as <credential>` adds a credential to an existing
  placement instead of refusing or creating a second one; `auth status` lists which of the
  placement's Connections are callable.
- Operation discovery lists a Connection only for the mechanisms its credential satisfies, so a
  read-only user Connection never advertises a write the bot alone may do.
- Documentation: `docs/guides/connect-slack.md` gains the two-identity setup.

## Acceptance
- One `[[catalog]]` entry with two credentials produces two callable Slack Connections; reads on the
  user one succeed on a public channel the bot is not in; `slack-chat-post-message` is offered on
  the bot one only.
- An adopter's configuration that names one credential is unchanged in behaviour.

## Readiness

Selected for implementation on 2026-09-06 at the operator's request. wave-cli: 6 of 10. The ready tag records selection; proposed is the pre-implementation lifecycle state, and existing active work stays active. Existing dependencies and implementation evidence requirements still apply.

## Scope

Derived 2026-09-06 by aep-drive story-scoper; coordinator records the returned surfaces.

- `crates/connectors-config/src/personal.rs` — cited.
- `crates/integration-catalog/src/lib.rs` — cited.
- `crates/integration-catalog/src/hosted.rs` — cited.
- `crates/connectors-console/src/enrol.rs` — cited.
- `crates/connectors-console/src/auth.rs` — cited.
- `crates/connectors-console/src/output_tests.rs` — inferred.
- `ess/system/domains/deployment.yaml` — inferred.
- `ess/system/domains/connection.yaml` — inferred.
- `docs/guides/connect-slack.md` — cited.

Medium confidence. Preserve the legacy first-connection digest; derive additional bindings explicitly and prevent choosing a different credential at invocation. Slack supports both bot and user token posting in the current catalog: token subject alone is not a permission rule. Use explicit per-binding grants. Global Credential-to-Connection cardinality remains UNMAPPED; settle only the local config projection before dispatch.

Would collide with any unit editing these files; directory entries require an additional containment review because AEP compares scope strings exactly.

## Execution queue

CLI execution queue 2026-09-06: 4 of 10. The urgent Slack delivery follow-up leads the queue. Original wave-cli readiness ordering remains historical context. Dependencies and measured scope govern dispatch order; priority is not a claim that prerequisites have landed.
