---
format: aep.planning-md/1
id: epic:local-product
kind: epic
status: archived
title: Local product
revision: 5
---
## Provenance

Read from the `epic: local-product` key in 4 file(s) under `docs/stories/`. No source document
exists for this epic itself; the grouping is a frontmatter value and nothing else. Migrated
2026-09-04 by the `aep-planning:story-migration` skill. **read**

## Stories

- `S-035` `story:the-cli-runs-without-being-configured-by-hand` — in-progress — The CLI runs without being configured by hand
- `S-036` `story:the-os-keyring-becomes-the-local-store` — in-progress — The OS keyring becomes the local credential store
- `S-041` `story:state-becomes-a-port` — in-progress — State becomes a port, with a SQLite backend
- `S-042` `story:one-composed-local-placement` — backlog — One composed local placement, called by both the CLI and Zwirn

## Status

`active`, derived from its stories (backlog: 1, in-progress: 3). **inferred** — no source document states a status
for this epic, so the rung follows the work underneath it: all done is `implemented`, any work
started is `active`, nothing started is `draft`.

## Disposition

Archived 2026-09-15 on the acceptance of `docs/design/21-clean-room-rewrite.md` (org-state review
decision sheet 2026-09-15, items 1 and 15; atlas ADR 0051, 2026-09-15).

**Absorbed by the successor lineage.** Design 21 §2 keeps credential custody (K7), the CLI surface (K9)
and the personal-local posture (K10); the successor ships all three today: "The local CLI implements
setup, configured adapter management, protected connect/repair, saved-credential revalidation, connection
inspection/revoke … SQLite records metadata and a qualified Secret Service keyring stores credentials"
(`connectors_v2/README.md` § *Where the project stands*) — which is S-035 (run without hand
configuration), S-036 (OS keyring as the local store) and S-041 (state as a port with a SQLite backend).

S-042 ("one composed local placement, called by both the CLI and Zwirn") is the exception and does not
follow the successor: zwirn is a consumer of *this* line (design 21 §3) and its pin is a bare rev,
`1e0eb9f`, that no tag contains. Repinning the four consumers is design 21 §6 phase 5 work, not this
epic's.

Its 4 `docs/stories/S-*` records keep their own statuses (3 `in-progress`, 1 `backlog`). This repository's `AGENTS.md`
states no convention for stories under an archived epic, so none was moved. Archiving keeps the record.
