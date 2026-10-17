---
format: aep.planning-md/3
id: epic:connector-vocabulary
kind: epic
status: draft
title: Adopt the provider, connector, instance and connection vocabulary in configuration, CLI and model
summary: Enable a connector by name, add a named instance, connect once.
relations:
- informed_by: architecture-decision-record:connector-vocabulary
revision: 3
---
## Outcome

A user enables a connector by name, adds a named instance, and connects it once; the CLI, the
configuration file, the ESS model and the guides use the vocabulary of
`architecture-decision-record:connector-vocabulary` and `docs/design/connector-vocabulary.md`.

## Acceptance

## Acceptance

- Enabling Gmail and connecting instance `private` of connector `gmail` takes a configuration entry
  that names the connector and the instance, with no hand-copied revision, executable SHA-256,
  endpoint, auth profile fields or operation list.
- One Gmail instance runs both reads and the two guarded draft writes under one connection.
- `connections reconnect` replaces `connections repair`; `--connector` and `--instance` replace
  `--adapter`.
- `connections list` reports a connection whose credential the provider refused as `Stale`, and a
  reconnect returns it to `Live`.
- An existing configuration with `[adapters.<alias>]` entries migrates without re-entering
  credentials.
- No user-facing guide, CLI help text or error message says "adapter".
- Guide statements this change makes wrong are corrected, among them
  `docs/catalog-google-oauth.md:117-118`: a personal-use OAuth app can be set to *In production*
  without verification (design § Consequences).

## Out of scope

- Operation groups or per-instance operation filtering.
- One consent shared by instances of different connectors.

## Open

## Open

- The three open questions in `docs/design/connector-vocabulary.md` § Open questions.
- Migrating `[adapters.<alias>]` entries whose implementation has no connector mapping (credential
  binaries, MCP, SQL, Kubernetes, Loki, Prometheus, Grafana, and Slack's two selections) is not
  done by `setup migrate`. They stay pinned and keep working until a later story maps them in
  `story:ess-connector-vocabulary-model`'s model; that story is not drafted.
