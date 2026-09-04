---
format: aep.planning-md/1
id: story:the-platform-connector-is-named-platform
kind: story
status: implemented
title: The platform connector is named platform
refs:
- provider: legacy
  reference: S-052
scope:
- confidence: cited
  path: crates/server
revision: 7
---
## Acceptance

Verbatim from `docs/stories/S-052-the-platform-connector-is-named-platform.md:25`. **read**

- No non-allowlisted `b10x` remains in crate names, type/function/variant names, or prose;
  `scripts/check-brand.sh`'s allowlist shrinks accordingly and stays negative-tested.
- An existing personal config carrying the old section name parses unchanged, proven by a test.
- `bash scripts/gate.sh` exits 0.

## Context

D5 is resolved: the platform's name is "platform" (extraction plan resolution note, 2026-08-23).
The brand sweep left ~154 code-level occurrences blocked on that decision. Rename the code-level
family: crate `integration-b10x` becomes `integration-platform`, and
`B10xConnectionConfig`, `B10xIntegrationConfig`, `B10xBackend`,
`ConnectionInitiator::B10x`, `b10x_only()` and companions follow, with prose
("B10x Identity/Integration/Provider/Session Authority", "Authorize B10x" consent
pages) moving to platform naming. Explicitly kept, each on the brand-fence allowlist: provider id
`b10x` and authority `io.b10x` (public identity, irreversible), `urn:b10x:*`,
`x-b10x-*` headers, `b10x.*.vN` contract ids, hash domains, and serialized state
(keyring service attribute). The serde-visible `[b10x]` config section moves only behind a
serde alias or a documented migration — an existing personal config must keep parsing.

Source frontmatter: pillar Platform · areas [integrations, config, server]. **read**

## Status

`done` in the source. Quoted from `docs/stories/S-052-the-platform-connector-is-named-platform.md:5`: `status: done`. **read**

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

Migrated from `docs/stories/S-052-the-platform-connector-is-named-platform.md`, which is not deleted and now names this artifact.

- First written 2026-08-23 · last touched 2026-08-24 · 3 revision(s)
- Legacy id `S-052`, recorded as the reference `legacy:S-052`
- Migrated 2026-09-04 by the `aep-planning:story-migration` skill
