---
format: aep.planning-md/1
id: epic:substrate-integration
kind: epic
status: active
title: Substrate integration
revision: 4
---
## Provenance

Read from the `epic: substrate-integration` key in 1 file(s) under `docs/stories/`. No source document
exists for this epic itself; the grouping is a frontmatter value and nothing else. Migrated
2026-09-04 by the `aep-planning:story-migration` skill. **read**

## Stories

- `S-029` `story:substrate-events-enter-durable-delivery-with-gap-recovery` — blocked — Substrate events enter durable delivery with gap recovery

## Status

`active`, derived from its stories (blocked: 1). **inferred** — no source document states a status
for this epic, so the rung follows the work underneath it: all done is `implemented`, any work
started is `active`, nothing started is `draft`.

## Disposition

**Left `active` on 2026-09-15 — needs the operator.** `docs/design/21-clean-room-rewrite.md` was accepted
that day (decision sheet items 1 and 15) and six sibling epics archived under it; this one does not
classify cleanly and was not moved.

It is the one epic that is neither absorbed nor excluded. Design 21 §4 does not strike substrate or
eventing, and §2 keeps eventing as K6 ("channel supervision, intake, ack/replay, delivery/subscription");
the successor does not carry it — "This slice does not advertise writes, managed OAuth acquisition,
durable events, process execution or media sessions" (`connectors_v2/README.md`). Its single story S-029
(substrate events enter durable delivery with gap recovery) is `blocked`. So it is v1-only work in a
repository design 21 §6 phase 5 archives as predecessor, which leaves it with no home rather than with a
disposition.

What settles it: whether durable substrate delivery is extracted as a K6 contract for the successor, or
named as a further exclusion in design 21 §4.
