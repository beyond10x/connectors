---
format: aep.planning-md/3
id: story:docs-catalog-guides-vocabulary
kind: story
status: draft
title: The catalog guides configure a connector and a named instance, with no native JSON or pins
relations:
- decomposes: epic:connector-vocabulary
- depends_on: story:docs-vocabulary-readme-website
scope:
- confidence: inferred
  path: docs/catalog-confluence.md
- confidence: inferred
  path: docs/catalog-hubspot.md
- confidence: inferred
  path: docs/catalog-jira.md
- confidence: inferred
  path: docs/catalog-opsgenie.md
- confidence: inferred
  path: docs/catalog-runpod.md
- confidence: inferred
  path: docs/catalog-zendesk.md
revision: 8
---
## Context

The catalog guides tell a user to write a native JSON file, a TOML `[adapters.<alias>]` entry with a
revision copied from `--print-local-bootstrap` and an executable SHA-256, and to select it with
`--adapter`. After story:connector-instance-derived-fields a catalog instance needs none of that.
Counted 2026-10-16 with `grep -ci adapter`: `catalog-confluence.md` 60, `catalog-jira.md` 16,
`catalog-runpod.md` 16, `catalog-opsgenie.md` 14, `catalog-zendesk.md` 14, `catalog-hubspot.md` 10.
`catalog-slack.md` (a credential binary, not derived) and `local-catalog-provider.md` (keeps the
pinned form as reference) are in story:docs-pinned-catalog-guides-vocabulary.

Part of the docs set (the `docs` skill: `website/docs`, `README.md` and `AGENTS.md` as one set); the
terms are defined once in `README.md` by story:docs-vocabulary-readme-website and linked from here.

## Change

- Each guide's setup becomes: enable the connector, add a named instance (endpoint only where the
  connector has no default, such as a self-hosted host), connect. Native JSON, revision copying,
  executable pins and operation lists leave these guides; existing installations are pointed to
  `setup migrate` and the pinned reference in `docs/local-catalog-provider.md`.
- `--connector`/`--instance`, `connections reconnect`, `instances …`, and the `Stale` state where a
  guide explains a refused credential.
- "Adapter" stays only where the vocabulary check's allowlist admits it. Text the change does not
  make stale is left alone. Commands shown are re-run and their output pasted from the run.

## Domain relations

None asserted; definitions are linked from `README.md`.

## Scope

- `docs/catalog-{confluence,hubspot,jira,opsgenie,runpod,zendesk}.md`

## Acceptance

`connectors-docs vocabulary --no-pins` (story:docs-vocabulary-readme-website) exits 0 over
`docs/catalog-{confluence,hubspot,jira,opsgenie,runpod,zendesk}.md`.

## Coordinator additions

## Coordinator additions

- 20 hand-written files still name `connections repair` or `adapters …` as the fix (45 lines;
  reconnect adversary, 2026-10-16), among them `docs/local-tavily.md:97`, `docs/catalog-google-*.md`,
  `adapters/{grafana,loki,prometheus}/README.md` and
  `adapters/mcp/contracts/protocol/v1alpha1/discovery-contract.{md,json}`. Tests in
  `adapters/catalog/tests/google_*.rs` pin guide text and move with the guides.
- The generated website reference pages are regenerated once on the wave branch by the
  coordinator; do not hand-edit them.
