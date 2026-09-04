---
format: aep.planning-md/1
id: story:activate-hosted-slack-org-bot
kind: story
status: implemented
title: Activate a hosted Slack organization bot operationally
relations:
- derived_from: epic:credential-production
scope:
- confidence: cited
  path: CHANGELOG.md
- confidence: cited
  path: crates/connectors-config/src/hosted.rs
- confidence: cited
  path: crates/connectors-runtime/src/composition.rs
- confidence: cited
  path: crates/integration-slack/src/backend.rs
- confidence: cited
  path: crates/integration-slack/src/lib.rs
revision: 9
---
## Outcome

A hosted Slack organization bot can be activated from value-free deployment policy plus app and bot credentials installed through the administrative API.

## Context

The runtime already knows how to materialize the tenant-wide organization bot from `app_token` and `bot_token`, but the hosted administration surface exposed only a personal OAuth client secret. Existing Socket Mode credentials therefore had no supported operational installation path. Personal user OAuth and organization-bot credentials are separate capabilities and must remain visibly separate.

## Acceptance

- Hosted Slack configuration may omit the OAuth client pair when only bot or companion operation is configured.
- The administrative surface accepts optional `app_token` and `bot_token` requirements at their exact tenant-wide credential addresses.
- A personal OAuth client secret is required and advertised only when the paired OAuth client ID and redirect URI are configured.
- The runtime creates the organization connection only after both Slack credentials exist and pass the existing workspace-bound verification.
- No credential value enters configuration, source, logs, planning artifacts, or deployment values.
