---
format: aep.planning-md/1
id: dependency-blocker:substrate-bundle-unsigned
kind: dependency-blocker
status: open
title: No owner-signed substrate bundle exists
relations:
- blocks: story:substrate-events-enter-durable-delivery-with-gap-recovery
revision: 1
---
## What would clear this

Architecture closed by ADR 0017 and substrate phase 3 is green; implementation remains blocked on an owner-signed substrate bundle.

Quoted from `docs/stories/S-029-substrate-events-enter-durable-delivery-with-gap-recovery.md`, the source's own `note:` field. **read**

## Provenance

Migrated 2026-09-04 by the `aep-planning:story-migration` skill. The source recorded this as
`status: blocked`. A status field cannot tell an item parked on an external gate from one somebody
is working on today, so the block is this artifact and the story it blocks stands at `active`.
