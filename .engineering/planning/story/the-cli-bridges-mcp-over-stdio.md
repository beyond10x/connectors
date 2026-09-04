---
format: aep.planning-md/1
id: story:the-cli-bridges-mcp-over-stdio
kind: story
status: implemented
title: The CLI bridges MCP over stdio
refs:
- provider: legacy
  reference: S-056
relations:
- derived_from: epic:mcp-entry
revision: 5
---
## Acceptance

Verbatim from `docs/stories/S-056-the-cli-bridges-mcp-over-stdio.md:22`. **read**

- A stdio MCP client completes initialize/tools-list/tools-call against a hosted deployment
  through the bridge, across a token expiry, without manual re-auth.
- The thin-CLI architecture fence stays green with a dated, bounded cap raise covering parser and
  dispatch declarations only.

## Context

Interactive MCP clients hold a static bearer, but identity access tokens live 300 seconds, so
the hosted `/mcp` endpoint alone serves scripted callers well and interactive ones poorly.
Add a `connectors mcp` subcommand: a stdio MCP server that proxies to the hosted `/mcp`
endpoint with automatic per-scope token refresh from the login session established by
`connectors login`. The behaviour belongs in a `connectors-client` module; the subcommand itself
stays a thin frontend.

Source frontmatter: pillar Platform · areas [config, integrations] · design `../design/14-mcp-transport-for-the-hosted-connectors-server.md`. **read**

## Status

`done` in the source. Quoted from `docs/stories/S-056-the-cli-bridges-mcp-over-stdio.md:5`: `status: done`. **read**

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

Migrated from `docs/stories/S-056-the-cli-bridges-mcp-over-stdio.md`, which is not deleted and now names this artifact.

- First written 2026-08-24 · last touched 2026-09-02 · 2 revision(s)
- Legacy id `S-056`, recorded as the reference `legacy:S-056`
- Migrated 2026-09-04 by the `aep-planning:story-migration` skill
