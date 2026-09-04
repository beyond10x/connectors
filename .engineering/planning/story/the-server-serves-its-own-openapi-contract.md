---
format: aep.planning-md/1
id: story:the-server-serves-its-own-openapi-contract
kind: story
status: implemented
title: The server serves its own OpenAPI contract
refs:
- provider: legacy
  reference: S-067
relations:
- derived_from: epic:public-surface
scope:
- confidence: cited
  path: crates/server
- confidence: cited
  path: docs
revision: 6
---
## Acceptance

Verbatim from `docs/stories/S-067-the-server-serves-its-own-openapi-contract.md:20`. **read**

- `GET {base}/openapi.json` serves a committed OpenAPI 3.1 document, unauthenticated,
  `application/json`, immutable per build (ETag or content hash header).
- The document covers: `/operations`, `/connections`, `/catalog`, `/events`,
  `/datasources` (envelope request/response schemas incl. the closed method sets and
  refusal codes), `/mcp` (JSON-RPC shape, the three meta-tools), the health probes, and
  the bearer security scheme naming the identity audience and scopes.
- Drift is impossible silently: a test suite validates every example in the document
  against the real `protocol` serde types — each request example must deserialize
  (deny_unknown_fields), each refusal example must match a real error shape, and a
  deliberately wrong example must fail the suite.
- No new dependency; the document is authored as a file and served verbatim from a new
  `crates/server/src/hosted/docs.rs`, with only route registration touching `hosted.rs`.
- The document is b10x-branded; no b10x string at the surface (check-brand stays
  clean).

## Context

Consumers of the hosted API — the CLI, the harness, external teams — have no
machine-readable contract to generate against or validate with. The server knows its
surface precisely (five strict envelope endpoints, the MCP transport, health probes), and
the repo already pins contracts as committed artifacts; the OpenAPI document joins that
family.

Source frontmatter: pillar Platform · areas [server, docs]. **read**

## Status

`done` in the source. Quoted from `docs/stories/S-067-the-server-serves-its-own-openapi-contract.md:5`: `status: done`. **read**

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

Migrated from `docs/stories/S-067-the-server-serves-its-own-openapi-contract.md`, which is not deleted and now names this artifact.

- First written 2026-08-24 · last touched 2026-08-24 · 3 revision(s)
- Legacy id `S-067`, recorded as the reference `legacy:S-067`
- Migrated 2026-09-04 by the `aep-planning:story-migration` skill
