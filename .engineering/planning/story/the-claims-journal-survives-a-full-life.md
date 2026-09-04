---
format: aep.planning-md/1
id: story:the-claims-journal-survives-a-full-life
kind: story
status: draft
title: The claims journal survives a full life
refs:
- provider: legacy
  reference: S-050
relations:
- derived_from: epic:enforced-authority
revision: 1
---
## Acceptance

Verbatim from `docs/stories/S-050-the-claims-journal-survives-a-full-life.md:22`. **read**

- A claims journal at capacity with expired entries accepts a new claim after pruning; a journal
  at capacity with only live entries still refuses, and the refusal names the condition.
- The replay-refusal journal row is asserted under an injected append failure — either the refusal
  still journals through a fallback, or the degraded posture is declared where the semantics live.
- Pruning never weakens exactly-once: a pruned claim's event is past its approval window, proven
  by a test that replays a pruned event and is refused on window grounds.

## Context

The local event-reply claims journal introduced by S-048 refuses all further event-authorized
replies once full — 10k claims / 4 MiB, no pruning (S-048 implementation notes, 2026-08-23), and
the replay-refusal journal append is best-effort, so a refusal under a failing store is enforced
but unjournaled (S-048 review minor). Give the journal a bounded life a deployment can live with:
expired-claim pruning (an event's approval window bounds how long its claim matters), an explicit
posture when the store is unavailable, and a journaled refusal that is proven, not hoped for.

Source frontmatter: pillar Platform · areas [state, integrations] · design `../design/13-grant-evaluation-and-approval-redemption.md`. **read**

## Status

`backlog` in the source. Quoted from `docs/stories/S-050-the-claims-journal-survives-a-full-life.md:5`: `status: backlog`. **read**

## Provenance

Migrated from `docs/stories/S-050-the-claims-journal-survives-a-full-life.md`, which is not deleted and now names this artifact.

- First written 2026-08-23 · last touched 2026-08-23 · 1 revision(s)
- Legacy id `S-050`, recorded as the reference `legacy:S-050`
- Migrated 2026-09-04 by the `aep-planning:story-migration` skill
