---
format: aep.planning-md/3
id: story:docs-connector-onboarding
kind: story
status: implemented
title: 'The site onboards every running connector: enable, add an instance, connect, read, approved write, reconnect'
relations:
- decomposes: epic:connector-vocabulary
- depends_on: story:docs-vocabulary-readme-website
- depends_on: story:connector-instance-derived-fields
- depends_on: story:connections-reconnect-verb
- depends_on: story:connection-stale-state
- depends_on: story:connect-refused-while-instance-holds-connection
- serves: vision:independent-contract-adapters
revision: 4
transitions:
- {from: "draft", to: "proposed", at: "2026-10-16T22:00:09Z", actor: "human:timo", revision: 2}
- {from: "proposed", to: "active", at: "2026-10-16T22:00:10Z", actor: "human:timo", revision: 3}
- {from: "active", to: "implemented", at: "2026-10-17T01:36:42Z", actor: "human:timo", revision: 4, decided_on: {"recorded":{"test_result":1}}}
---
## Context

The operator asked on 2026-10-16 for the public documentation to be overhauled for the connector
vocabulary, with onboarding for every connector that runs today. The site is
`https://beyond10x.github.io/connectors/` (the repository's homepage; the old
`https://beyond10x.github.io/docs/connectors/` is a redirect page to `/connectors/docs/`). The
hand-written guides under `website/docs/guides/` are three (`set-up-the-local-cli.md`,
`build-a-catalog-bundle.md`, `federate-adapter-services.md`); per-connector setup lives only in
the repository's `docs/catalog-*.md` and `docs/local-*.md`, which the site does not carry as
onboarding.

Connectors that run today, from the README support table (`README.md:126-140`) and the catalog
selections (`adapters/catalog/providers/*/operations.json`): Gmail, Google Drive, Google Calendar,
Google Slides, GitLab, Jira, Confluence, Zendesk, HubSpot, Slack, Opsgenie, Runpod, Kubernetes,
PostgreSQL, MySQL, Tavily, Loki, Grafana, Prometheus and MCP. The implementor confirms the list
against the README and the shipped selections before writing, and lists any difference in the
report.

## Change

- One onboarding page per connector under `website/docs/connectors/` (a new sidebar section
  "Connectors"), each in the same order: what it reads and writes; prerequisites at the provider
  (for Google: the OAuth client, *In production* for a personal-use app, the consent screen);
  enable the connector and add an instance; connect (the exact command); a first read; a first
  guarded write with approval where the connector has writes; reconnect when the connection is
  `Stale`; limits. Commands use the connector and instance form the code stories ship; a connector
  that still needs pins (Slack, Atlassian gateway instances, non-catalog kinds) shows its pinned
  form and says why.
- One index page listing every connector with its provider, reads, writes and auth profiles,
  generated from the shipped declarations where the site generator can read them, hand-written
  otherwise.
- `website/docs/guides/set-up-the-local-cli.md` ends by linking the index.
- Follow the repository's documentation workflow (the `docs` skill: `website/docs`, `README.md`
  and `AGENTS.md` as one set); generated pages are regenerated, never edited.

## Scope

- `website/docs/connectors/**` (new)
- `website/sidebars.*` or the sidebar configuration the site uses
- `website/docs/guides/set-up-the-local-cli.md`
- `crates/connectors-docs/` only if the index is generated

## Acceptance

`connectors-docs vocabulary` (from story:docs-vocabulary-readme-website) extended with an
onboarding rule exits 0 over `website/docs/connectors/`, and the rule fails unless every connector
in the README support table has a page there with the eight headings of Change in order.

## Notes

- Depends on the vocabulary check and the README vocabulary section, and on the code stories whose
  commands the pages show.
