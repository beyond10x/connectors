---
format: aep.planning-md/1
id: story:a-logged-in-person-needs-no-configuration
kind: story
status: implemented
title: A logged-in person needs no configuration
refs:
- provider: legacy
  reference: S-061
relations:
- derived_from: epic:endpoint-plane
revision: 5
---
## Acceptance

Verbatim from `docs/stories/S-061-a-logged-in-person-needs-no-configuration.md:22`. **read**

- On a machine with no prior state: `connectors login <connectors-base>` completes the browser
  flow, and a following catalog/search/invoke command against the selected hosted deployment
  succeeds with no endpoint or token flags; tokens refresh across the
  300-second expiry without re-login, proven by an integration test with a fake identity.
- The thin-CLI architecture fence stays green — the behaviour lands behind connectors-client
  per its prescription.

## Context

Today a person needs curl and token plumbing to use the hosted plane. Give the local Connectors
CLI a zero-configuration hosted mode: `connectors login <connectors-base>` reads that deployment's
public bootstrap document, drives the neutral Identity loopback flow, and places the opaque login
session in the OS keyring. Subsequent hosted CLI requests and the stdio MCP bridge exchange that
session for short-lived, exact-scope tokens transparently. Identity remains relying-party neutral:
Connectors publishes the Identity origin it trusts, never the other way around.

Source frontmatter: pillar Platform · areas [config, integrations] · design `../design/15-a-zero-configuration-endpoint-plane.md`. **read**

## Status

`done` in the source. Quoted from `docs/stories/S-061-a-logged-in-person-needs-no-configuration.md:5`: `status: done`. **read**

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

Migrated from `docs/stories/S-061-a-logged-in-person-needs-no-configuration.md`, which is not deleted and now names this artifact.

- First written 2026-08-24 · last touched 2026-09-02 · 2 revision(s)
- Legacy id `S-061`, recorded as the reference `legacy:S-061`
- Migrated 2026-09-04 by the `aep-planning:story-migration` skill
