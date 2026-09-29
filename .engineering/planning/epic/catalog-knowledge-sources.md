---
format: aep.planning-md/3
id: epic:catalog-knowledge-sources
kind: epic
status: draft
title: Read-only knowledge sources through the catalog provider
relations:
- informed_by: architecture-decision-record:declarative-http-provider-runtime
- informed_by: specification:catalog-http-runtime-handoff
revision: 5
---
## Outcome

A knowledge-ingest consumer reads issue trackers, help desks, wikis and chat through the local
`connectors` CLI the way it reads GitLab today (`docs/local-catalog-provider.md`): a pinned
OpenAPI source, a reviewed read-only selection set, `operations describe` / `operations invoke`.
Requested by the knowledge-ingest consumer on 2026-09-29, in this order: Jira Cloud, Zendesk,
Confluence Cloud, Slack.

## What every source must give the consumer

- operation ids that stay stable across releases (the selection ids);
- documented pagination and its end condition, per list operation;
- a time-window filter for deltas where the provider offers one on the top-level list, and a
  statement of how deltas are taken where it does not;
- the provider's item returned unchanged.

## Constraint found

The catalog provider offers one authentication profile per configuration, "a token in one
header"; "OAuth, basic and signing profiles are not offered by this provider yet"
(`docs/local-catalog-provider.md`, Limits). Jira Cloud, Confluence Cloud and Zendesk API tokens
authenticate with HTTP basic (account email plus token), so a basic profile is a prerequisite
(`story:catalog-basic-auth-profile`). Slack bot tokens use `Authorization: Bearer`, which the
catalog already supports (`bearer: true`).

## Shared surfaces

Every provider story compiles its bundle into `adapters/catalog/generated/bundles/`, which rewrites
the one shared `index.json`, and `bundle::load` refuses a provider with no index row
(`crates/connectors-catalog/src/bundle.rs:299-318`). `adapters/catalog/tests/bundle_drift.rs`
compares against a one-provider index today (`:27-29`). Both files are therefore shared, and the
provider stories run in sequence through `depends_on` edges, in the operator's priority of
2026-09-29: Jira → Confluence → Slack → Grafana, then Zendesk (no longer requested; kept last).
`story:catalog-jira-cloud-reads` generalizes `bundle_drift.rs` to every indexed provider; each later
story adds its row to `index.json` on top of the previous one. Each provider has its own guide
(`docs/catalog-<provider>.md`), its own test file (`adapters/catalog/tests/<provider>.rs`), its own
bundle (`adapters/catalog/generated/bundles/<provider>.bundle.json`) and its own pinned source files.

## Acceptance

- Each of the four providers has a shipped selection set under
  `adapters/catalog/providers/<provider>/operations.json`, and `adapters/catalog/tests/<provider>.rs`
  passes in the gate.
- For every list operation of every provider, a fixture test pages it to its documented end
  condition through `operations invoke`.
- `adapters/catalog/tests/bundle_drift.rs` reproduces every indexed bundle, including all four new
  ones, byte for byte.

## Out of scope

Writes of any kind; OAuth flows; webhooks; any source beyond the four named.
