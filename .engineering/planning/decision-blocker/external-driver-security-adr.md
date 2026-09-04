---
format: aep.planning-md/1
id: decision-blocker:external-driver-security-adr
kind: decision-blocker
status: open
title: No security ADR has decided external driver artifacts
relations:
- blocks: story:external-driver-artifacts-stay-deferred
revision: 1
---
## What would clear this

Delivery item 6 is a gate, not implementation scope; unblock only after built-in pressure and a separate security ADR.

Quoted from `docs/stories/S-028-external-driver-artifacts-stay-deferred.md`, the source's own `note:` field. **read**

## Provenance

Migrated 2026-09-04 by the `aep-planning:story-migration` skill. The source recorded this as
`status: blocked`. A status field cannot tell an item parked on an external gate from one somebody
is working on today, so the block is this artifact and the story it blocks stands at `active`.
