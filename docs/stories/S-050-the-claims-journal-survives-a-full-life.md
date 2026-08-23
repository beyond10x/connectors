---
id: S-050
title: "The claims journal survives a full life"
pillar: Platform
status: backlog
design: ../design/13-grant-evaluation-and-approval-redemption.md
epic: enforced-authority
areas: [state, integrations]
---

# The claims journal survives a full life

## Goal

The local event-reply claims journal introduced by S-048 refuses all further event-authorized
replies once full — 10k claims / 4 MiB, no pruning (S-048 implementation notes, 2026-08-23), and
the replay-refusal journal append is best-effort, so a refusal under a failing store is enforced
but unjournaled (S-048 review minor). Give the journal a bounded life a deployment can live with:
expired-claim pruning (an event's approval window bounds how long its claim matters), an explicit
posture when the store is unavailable, and a journaled refusal that is proven, not hoped for.

## Acceptance

- A claims journal at capacity with expired entries accepts a new claim after pruning; a journal
  at capacity with only live entries still refuses, and the refusal names the condition.
- The replay-refusal journal row is asserted under an injected append failure — either the refusal
  still journals through a fallback, or the degraded posture is declared where the semantics live.
- Pruning never weakens exactly-once: a pruned claim's event is past its approval window, proven
  by a test that replays a pruned event and is refused on window grounds.

## Progress

- 2026-08-23 — filed from the S-048 review minors and implementation risk notes (journal capacity,
  best-effort refusal append).
