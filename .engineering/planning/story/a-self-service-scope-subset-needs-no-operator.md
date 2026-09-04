---
format: aep.planning-md/1
id: story:a-self-service-scope-subset-needs-no-operator
kind: story
status: implemented
title: A self-service scope subset needs no operator
refs:
- provider: legacy
  reference: S-055
relations:
- derived_from: epic:mcp-entry
revision: 5
---
## Acceptance

Verbatim from `docs/stories/S-055-a-self-service-scope-subset-needs-no-operator.md:25`. **read**

- In beyond10x/identity: a member principal mints `connectors.catalog.read connectors.invoke`
  (and the full self-service four); `connectors.catalog.read connectors.events.read` still
  demands `operator`; operator behaviour is unchanged — all proven in the vocabulary test.
- Identity's `bash scripts/gate.sh` exits 0; the identity commit is referenced here on close.

## Context

This story's change lands in the separate beyond10x/identity repository (from `a664b99b`); it
is tracked here so the board shows the end-to-end dependency. Identity's
`admitted_connector_scope` compares the whole canonical scope string against four single-scope
literals, so any multi-scope request demands the `operator` group — an `sre`/`dev` principal
cannot hold `connectors.catalog.read connectors.invoke` in one token, making the MCP endpoint
operator-only in practice. Admit any subset of the four self-service scopes
(`connectors.catalog.read`, `connectors.connections.self`, `connectors.events.self`,
`connectors.invoke`) for any authenticated principal; any other member in the set still
demands `operator`.

Source frontmatter: pillar Platform · areas [identity] · design `../design/14-mcp-transport-for-the-hosted-connectors-server.md`. **read**

## Status

`done` in the source. Quoted from `docs/stories/S-055-a-self-service-scope-subset-needs-no-operator.md:5`: `status: done`. **read**

This artifact reached `implemented` with `aep artifact move --evidence test_result=1`. The journal
records that move as resting on an **assertion**, not on a run this migration observed. The flag is
what the CLI provides for evidence that lives outside the store.

What was asserted, and where it came from:

- The source records `status: done` at the line quoted above. **read**
- `bash scripts/gate.sh` was green at commit `a48030b` on 2026-09-04 — exit 0, 136 `test result: ok`
  lines across 11 workspaces. **read**, from `~/.cache/connectors-gate/gate2.log`

No per-story run was attributed to this story. The gate is a repository-wide fact, and reading it as
proof of one story's acceptance would be an inference this record does not make.

## Provenance

Migrated from `docs/stories/S-055-a-self-service-scope-subset-needs-no-operator.md`, which is not deleted and now names this artifact.

- First written 2026-08-24 · last touched 2026-08-24 · 2 revision(s)
- Legacy id `S-055`, recorded as the reference `legacy:S-055`
- Migrated 2026-09-04 by the `aep-planning:story-migration` skill
