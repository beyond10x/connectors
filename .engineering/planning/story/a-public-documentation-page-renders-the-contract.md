---
format: aep.planning-md/1
id: story:a-public-documentation-page-renders-the-contract
kind: story
status: implemented
title: A public documentation page renders the contract
refs:
- provider: legacy
  reference: S-068
relations:
- derived_from: epic:public-surface
- depends_on: story:the-server-serves-its-own-openapi-contract
scope:
- confidence: cited
  path: crates/server
- confidence: cited
  path: docs
revision: 7
---
## Acceptance

Verbatim from `docs/stories/S-068-a-public-documentation-page-renders-the-contract.md:20`. **read**

- `GET {base}/docs` serves one self-contained HTML page, unauthenticated: zero external
  requests (no CDN scripts, no fonts, no images fetched elsewhere) — the connect
  completion page precedent.
- Content sections: authentication (identity login → access token mint with audience and
  scopes), the five envelope endpoints with request/response examples lifted from the
  S-067 document (single source of truth — examples are extracted, not duplicated), the
  MCP endpoint with an initialize/tools-call example, datasources incl. the read verbs,
  and the refusal code table.
- The page links `openapi.json` and renders its version; a drift test asserts every
  example shown on the page exists in the OpenAPI document.
- Static and tenant-free: the handler reads no state, takes no auth, and cannot render
  request-derived content beyond the base path.
- b10x-branded; check-brand stays clean.

## Context

A person handed the connectors URL has nothing to read. `GET {base}/docs` renders the
public documentation: how to log in and mint a token, every endpoint with a working curl
example, the MCP entry point, datasources, and the refusal codes — one self-contained
HTML page, no external assets.

Source frontmatter: pillar Platform · areas [server, docs]. **read**

## Status

`done` in the source. Quoted from `docs/stories/S-068-a-public-documentation-page-renders-the-contract.md:5`: `status: done`. **read**

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

Migrated from `docs/stories/S-068-a-public-documentation-page-renders-the-contract.md`, which is not deleted and now names this artifact.

- First written 2026-08-24 · last touched 2026-08-24 · 2 revision(s)
- Legacy id `S-068`, recorded as the reference `legacy:S-068`
- Migrated 2026-09-04 by the `aep-planning:story-migration` skill

Source frontmatter also declares `depends_on: [S-067]`, migrated as the `depends_on` edge to
`story:the-server-serves-its-own-openapi-contract`. **read**
