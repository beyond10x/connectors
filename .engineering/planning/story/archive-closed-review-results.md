---
format: aep.planning-md/2
id: story:archive-closed-review-results
kind: story
status: draft
title: Archive the closed review-results that record no findings
summary: 144 of 189 review-results carry no findings block; archive the ones whose subject is closed
revision: 2
---
# Story: Archive the closed review-results that record no findings

## Outcome

A reader can diff this store one run against the next, because a `review-result`
that is still `active` is one whose findings are still open — not one that
recorded nothing and was never moved.

## Context

Measured at `df0f6c4` on 2026-09-15 (`aep plan artifact validate --strict`;
`ls .engineering/planning/review-result`; `git log --since=2026-09-08 --oneline`):

| measure | value |
|---|---|
| review-results | 189 — 89 `active`, 100 `archived` |
| of those, recording no findings block | 144 — 76 `active`, 68 `archived` |
| carrying a findings block | 45 |
| implemented stories | 65 |
| review-results per implemented story | 2.91 |
| artifacts in the store | 350 |
| commits since 2026-09-08 | 198 on `HEAD`, 206 over all refs (`git rev-list --count --since=2026-09-08`, local time Europe/Berlin; the reviewer measured 196 / 204) |

Org-state review 2026-09-15 run 2, lane 08 F7; ledger ORG-0087, which supersedes
ORG-0223 from run 1.

The practice that produced them has already stopped: the org-state review
decision sheet of 2026-09-15, item 16, took its default — stop transcribing
review-results into verification-reports. What is undecided is what happens to
the 144 that already exist, and that is what this story holds. It is a draft
until the operator takes it, and **nothing has been moved**.

## Acceptance

- Each of the 144 is either `archived` with its subject's closure named, or kept
  `active` with a findings block written into it. No third outcome, and none
  deleted — `rm` shows up in `validate` as a deletion no command made.
- `aep plan artifact validate --strict` in this store either no longer refuses on
  "recording no findings block", or refuses on a number this repository's own
  documentation states and justifies.
- `docs/development.md:25-28`, which decision sheet item 16 names as depending on
  the transcription practice, says what it expects instead.

## Out of Scope

- The 45 review-results that do carry a findings block.
- Stopping the practice: decision sheet item 16 already did that. This story is
  the backlog that decision left behind.
- Any move made before the operator answers. The counts above are the whole of
  what this story asserts today.

## Ambiguities

- `requires-stakeholder-input` — whether the 76 no-findings review-results still
  `active` are archived wholesale or read one at a time. The operator decides;
  decision sheet item 16 settled the practice, not this backlog.
- `inferable` — what a review-result has to carry to stop being counted is in the
  strict validator's own message, "states its findings as prose only — nothing can
  enumerate what it found", emitted by `aep plan artifact validate --strict`.

## Open Questions

None beyond the ambiguity above.
