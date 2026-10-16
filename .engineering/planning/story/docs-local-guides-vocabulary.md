---
format: aep.planning-md/3
id: story:docs-local-guides-vocabulary
kind: story
status: draft
title: The local runtime guides use connectors, instances and reconnect
relations:
- decomposes: epic:connector-vocabulary
- depends_on: story:docs-vocabulary-readme-website
scope:
- confidence: inferred
  path: adapters/grafana/README.md
- confidence: inferred
  path: adapters/loki/README.md
- confidence: inferred
  path: adapters/prometheus/README.md
- confidence: inferred
  path: docs/local-approval-keys.md
- confidence: inferred
  path: docs/local-approvals.md
- confidence: inferred
  path: docs/local-clock.md
- confidence: inferred
  path: docs/local-connection-registry.md
- confidence: inferred
  path: docs/local-consumer-launch.md
- confidence: inferred
  path: docs/local-container.md
- confidence: inferred
  path: docs/local-er-metadata.md
- confidence: inferred
  path: docs/local-gitlab-merge.md
- confidence: inferred
  path: docs/local-kubernetes-cli.md
- confidence: inferred
  path: docs/local-mcp-cli.md
- confidence: inferred
  path: docs/local-mounted-secret-custody.md
- confidence: inferred
  path: docs/local-mutation-ledger.md
- confidence: inferred
  path: docs/local-mysql-cli.md
- confidence: inferred
  path: docs/local-postgres-cli.md
- confidence: inferred
  path: docs/local-runtime-foundation.md
- confidence: inferred
  path: docs/local-tavily.md
- confidence: inferred
  path: docs/running-services.md
revision: 10
---
## Context

The runtime guides under `docs/` describe configuration entries, CLI selectors and the repair verb
in the old vocabulary. Counted 2026-10-16 with `grep -ci adapter`: `local-kubernetes-cli.md` 39,
`local-mysql-cli.md` 25, `local-postgres-cli.md` 23, `local-mcp-cli.md` 21, `local-tavily.md` 13,
`local-runtime-foundation.md` 12, `local-approval-keys.md` 8, `local-connection-registry.md` 8,
`local-gitlab-merge.md` 7, `local-approvals.md` 6, `local-consumer-launch.md` 6,
`local-container.md` 5, `local-clock.md` 4, `running-services.md` 4, `local-er-metadata.md` 2,
`local-mounted-secret-custody.md` 2, `local-mutation-ledger.md` 1.

Part of the docs set (the `docs` skill: `website/docs`, `README.md` and `AGENTS.md` as one set); the
terms are defined once in `README.md` by story:docs-vocabulary-readme-website and linked from here.

## Change

- Rewrite what the vocabulary change makes stale: configuration examples in the instance form,
  `--connector`/`--instance` for `--adapter`, `connections reconnect` for `connections repair`,
  `instances …` for `adapters …`, the `Stale` state where a guide explains a refused credential,
  and `setup migrate` where a guide tells an existing user what to change.
- Non-catalog implementations (Kubernetes, SQL, MCP, Tavily, Loki, Prometheus, Grafana) are not derived
  (story:connector-instance-derived-fields), and story:ess-connector-vocabulary-model leaves their
  connector mapping UNMAPPED. Their guides show the pinned instance form of
  story:connector-config-dual-read under a connector name equal to the implementation's
  (`kubernetes`, `postgres`, `mysql`, `mcp`, `tavily`, `loki`, `prometheus`, `grafana`), which that
  story admits for pinned entries;
  if the model later names them otherwise, the guides follow.
- "Adapter" stays only where the vocabulary check's allowlist admits it.
- Text the change does not make stale is left alone. Commands shown are re-run and their output
  pasted from the run.

## Domain relations

None asserted; definitions are linked from `README.md`.

## Scope

- `docs/local-approval-keys.md`, `docs/local-approvals.md`, `docs/local-clock.md`,
  `docs/local-connection-registry.md`, `docs/local-consumer-launch.md`, `docs/local-container.md`,
  `docs/local-er-metadata.md`, `docs/local-gitlab-merge.md`, `docs/local-kubernetes-cli.md`,
  `docs/local-mcp-cli.md`, `docs/local-mounted-secret-custody.md`, `docs/local-mutation-ledger.md`,
  `docs/local-mysql-cli.md`, `docs/local-postgres-cli.md`, `docs/local-runtime-foundation.md`,
  `docs/local-tavily.md`, `docs/running-services.md` (the 17 files counted above;
  `docs/local-approval-binding.md`, `local-execution-audit.md` and `local-secret-service.md` have no
  match and are untouched; `docs/local-catalog-provider.md` is in story:docs-pinned-catalog-guides-vocabulary)

- The operator guides of three native adapters, which show `[adapters.<alias>]` and `--adapter`
  (`grep -ci adapter`, 2026-10-16): `adapters/loki/README.md` 16, `adapters/prometheus/README.md` 13,
  `adapters/grafana/README.md` 13.

## Acceptance

`connectors-docs vocabulary` (story:docs-vocabulary-readme-website, no flags) exits 0 over every
file listed under Scope: the 17 files under `docs/` and `adapters/{loki,prometheus,grafana}/README.md`.

## Notes

- Not in scope: historical documents (`docs/waves/`, `docs/evidence/`, `docs/plans/`,
  `docs/cli-migration-v1-to-v2.md`) and internal design (`docs/design.md`), where "adapter" is the
  internal term.
