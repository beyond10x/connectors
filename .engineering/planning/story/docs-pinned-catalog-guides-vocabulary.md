---
format: aep.planning-md/3
id: story:docs-pinned-catalog-guides-vocabulary
kind: story
status: implemented
title: The Slack guide and the catalog provider reference use the vocabulary with the pinned instance form
relations:
- decomposes: epic:connector-vocabulary
- depends_on: story:docs-vocabulary-readme-website
- serves: vision:independent-contract-adapters
scope:
- confidence: inferred
  path: docs/catalog-slack.md
- confidence: inferred
  path: docs/local-catalog-provider.md
revision: 8
transitions:
- {from: "draft", to: "proposed", at: "2026-10-16T19:08:18Z", actor: "human:timo", revision: 6, decided_on: {"recorded":{"review_outcome":1}}}
- {from: "proposed", to: "active", at: "2026-10-16T19:08:19Z", actor: "human:timo", revision: 7, decided_on: {"recorded":{"review_outcome":1}}}
- {from: "active", to: "implemented", at: "2026-10-17T01:36:43Z", actor: "human:timo", revision: 8, decided_on: {"recorded":{"test_result":1,"review_outcome":1}}}
---
## Context

Two catalog-related guides keep the pinned instance form and so cannot carry the no-pins claim of
story:docs-catalog-guides-vocabulary:

- `docs/catalog-slack.md` (83 matches of `grep -ci adapter`, 2026-10-16). Slack runs as two
  instances with different selection files (`adapters/catalog/providers/slack/operations.json`,
  bot; `user-operations.json`, user and search), so it has no connector declaration and is not
  derived until a later story decides one connector or two (story:catalog-connector-declarations
  § Change).
- `docs/local-catalog-provider.md` (42), the reference for the native configuration and the pinned
  form that existing installations and non-derived connectors still use.

Part of the docs set (the `docs` skill: `website/docs`, `README.md` and `AGENTS.md` as one set); the
terms are defined once in `README.md` by story:docs-vocabulary-readme-website and linked from here.

## Change

- `docs/catalog-slack.md`: two pinned instances (bot and user/search selections) in the form of
  story:connector-config-dual-read under the connector name `slack` (a pinned instance's connector
  name is not checked against declarations), with `--connector`/`--instance`,
  `connections reconnect`, `instances …` and the `Stale` state.
- `docs/local-catalog-provider.md`: the derived setup first (enable, add an instance, connect), then
  one reference section on the pinned form and `setup migrate` for existing installations.
- "Adapter" stays only where the vocabulary check's allowlist admits it. Text the change does not
  make stale is left alone. Commands shown are re-run and their output pasted from the run.

## Domain relations

None asserted; definitions are linked from `README.md`.

## Scope

- `docs/catalog-slack.md`
- `docs/local-catalog-provider.md`

## Acceptance

`connectors-docs vocabulary` (story:docs-vocabulary-readme-website, no flags) exits 0 over
`docs/catalog-slack.md` and `docs/local-catalog-provider.md`.
