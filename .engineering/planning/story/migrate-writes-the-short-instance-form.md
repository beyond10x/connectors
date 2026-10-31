---
format: aep.planning-md/3
id: story:migrate-writes-the-short-instance-form
kind: story
status: draft
title: setup migrate moves every legacy entry to the short instance form, catalog named nowhere
relations:
- decomposes: epic:connector-vocabulary
revision: 3
---
## Problem

Measured 2026-10-22 on the operator's machine, connectors 0.56.0, `connectors setup migrate --apply`
on a legacy `[adapters.<alias>]` configuration of 8 entries:

| entry | result |
|---|---|
| forge, gmail-write, zendesk | migrated, but in the pinned form: `alias`, `instance_id`, `adapter_id = "catalog"`, `configuration_revision`, `protocol`, `private_protocol`, `executable` with `sha256` |
| slack-bot, slack-search | kept, `connector_undeclared` |
| jira, confluence | kept, `gateway_prefix` |
| tavily | kept, `implementation_unmapped` |

Afterwards `instances list` still shows `"connector":"catalog"` for 5 instances and
`"adapter_id":"catalog"` for 7. Connection states were the same before and after (`pending` for
forge and zendesk, none for gmail-write).

Operator, 2026-10-22, verbatim: "i dont want to keep anything old, just migrate".

## Outcome

- `setup migrate` moves every legacy entry of a shipped connector, Slack, Jira, Confluence and
  Tavily included, to `[connectors.<connector>.instances.<instance>]`.
- A migrated instance is written in the short form: `endpoint`, `description` and `profile` where
  needed; no `adapter_id`, executable, digest, protocol or revision.
- Existing connections stay bound to the migrated instance with no re-consent.
- An entry migrate cannot move is refused with the entry and the reason, and nothing is written.

## Acceptance (draft)

- A legacy configuration with one entry per shipped connector migrates to the short form with no
  `[adapters.*]` table left.
- `instances list` after the migration has no `catalog` value.
- A connection that was `live` before the migration is `live` after it.

## Cause of `connector_undeclared` for Slack

`apps/connectors/src/local/migrate.rs:23-36` compiled 12 declarations; `adapters/catalog/providers/slack`
had no `connector.json`, and `apps/connectors/tests/setup_migrate.rs:170-173` asserted that Slack stays
an adapter entry. `story:slack-connector-declaration` (0.63.1, D313) ships Slack's bot declaration,
so both Slack entries migrate to pinned instances of `slack`; writing them in the short form stays
with this story.
