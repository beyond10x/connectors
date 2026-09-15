---
format: aep.planning-md/1
id: epic:native-voice
kind: epic
status: archived
title: Native voice
revision: 5
---
## Provenance

Read from the `epic: native-voice` key in 2 file(s) under `docs/stories/`. No source document
exists for this epic itself; the grouping is a frontmatter value and nothing else. Migrated
2026-09-04 by the `aep-planning:story-migration` skill. **read**

## Stories

- `S-032` `story:sip-driver-terminates-one-governed-call` — in-progress — The SIP driver terminates one governed call
- `S-033` `story:neutral-rtvbp-bridges-the-call-to-an-application-channel` — in-progress — Neutral RTVBP bridges the call to an application channel

## Status

`active`, derived from its stories (in-progress: 2). **inferred** — no source document states a status
for this epic, so the rung follows the work underneath it: all done is `implemented`, any work
started is `active`, nothing started is `draft`.

## Disposition

Archived 2026-09-15 on the acceptance of `docs/design/21-clean-room-rewrite.md` (org-state review
decision sheet 2026-09-15, items 1 and 15; atlas ADR 0051, 2026-09-15).

**Excluded by design 21 §4, first row.** "Voice / SIP / RTVBP stack — `rtvbp-voice-endpoint`,
`voice-runtime`, `voice-local-audio`, `driver-sip`, `driver-audio`, `driver-speech`, `integration-sip`;
designs 05, 15 — 10,717 LOC — **Relocate** to its own repository — it is a different product with a
socket-owning driver exception carved through this one's fences." Both stories are exactly that stack:
S-032 the SIP driver terminating a governed call, S-033 the neutral RTVBP bridge. The successor does not
carry it either — "This slice does not advertise writes, managed OAuth acquisition, durable events,
process execution or media sessions" (`connectors_v2/README.md`).

**Where the code goes is still open.** Design 21 §7 D1 (relocate / keep / delete) is unanswered: the
decision sheet does not reach it and no sibling checkout under `~/beyond10x` carries a voice, SIP or
RTVBP repository (checked 2026-09-15). §4 says "Not deleted: operator decides (§7 D1)", and archiving
this epic keeps the record rather than removing it.

Its 2 `docs/stories/S-*` records keep their own statuses (both `in-progress`). This repository's `AGENTS.md` states no
convention for stories under an archived epic, so neither was moved.
