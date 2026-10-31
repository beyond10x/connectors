---
format: aep.planning-md/3
id: story:migrate-writes-the-short-instance-form
kind: story
status: active
title: setup migrate moves every legacy entry to the short instance form, catalog named nowhere
relations:
- decomposes: epic:connector-vocabulary
- serves: vision:independent-contract-adapters
revision: 6
transitions:
- {from: "draft", to: "proposed", at: "2026-10-31T02:53:19Z", actor: "human:timo", revision: 5, executor: "agent:conn-d298", correlation: "wave-d298"}
- {from: "proposed", to: "active", at: "2026-10-31T02:53:20Z", actor: "human:timo", revision: 6, executor: "agent:conn-d298", correlation: "wave-d298"}
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

## Scope in wave D298

Decided by the implementing session on 2026-10-31 from a read-only scoping of origin/main 5bc94d505,
following `migration-plan:connector-vocabulary` (Invariant; steps 2 and 4).

Verified in code: connections, approval policies, audits and mutations are keyed by `instance_id`
(`crates/connectors-host/src/local/registry.rs` `Binding`, `follow` returns `Changed` when the id
differs), and a derived instance's id is `<connector>.<name>` (`adapters/catalog/src/local.rs`
`instance_configuration`). A short-form instance without its old id would orphan every record.

In this wave (U2):

- The short form takes an optional `instance_id`: the migrated instance's old id, carried
  explicitly (migration plan step 2). The derived native configuration uses it, so its revision is
  the legacy entry's when everything else is equal.
- `setup migrate` writes the short form (`alias`, `instance_id`, and `endpoint` and `profile` only
  where they differ from the declaration's default) for an entry whose derived revision equals its
  pinned revision. An entry whose revision would change stays a pinned instance, as today, and the
  result names the differing field (step 4).
- Connections, approval policies and custody stay bound because the instance id and revision are
  unchanged.

Not in this wave, each a later story:

- Jira and Confluence behind the Atlassian gateway (`gateway_prefix`): `api_path` exists, but an
  endpoint that already ends in it is refused, so documented site-form instances need a rule first.
- Tavily: a separate implementation (`adapters/tavily`), not a catalog connector; migrate cannot
  map it without a design decision.
- Slack's user selection: needs `story:slack-user-connector-declaration`.
- Refusing the whole run when any entry cannot move: until the three above land it would stop
  every configuration that holds one of them from migrating at all.
