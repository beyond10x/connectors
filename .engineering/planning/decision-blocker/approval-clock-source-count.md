---
format: aep.planning-md/3
id: decision-blocker:approval-clock-source-count
kind: decision-blocker
status: cleared
title: Does a verified clock source wider than the cap count toward the quorum?
relations:
- blocks: story:approval-clock-quorum
revision: 4
transitions:
- {from: "open", to: "cleared", at: "2026-10-14T20:34:43Z", actor: "human:timo", revision: 4}
---
## Question
The approval clock quorum's live check on 2026-10-14 (`independent_live_quorum`, run explicitly):
Cloudflare and roughtime.se `unreachable`; int08h verified but `too_imprecise` (radius 5,000 ms,
its own interval over the 8,000 ms per-source cap); time.txryan.com verified (radius 3,000 ms).
One counted source, quorum 2: `below_quorum`. As designed, the clock answers nowhere today, and
Slack writes stay blocked behind it. The cap cannot rise past 10,000 ms without changing the proof
profile (`docs/design/approval-clock-quorum.md`, "Width cap"), and a 5 s radius exceeds that too.

## Options
- **A (recommended): count every verified source; cap only the answer.** Step 1 drops the
  per-source width test. Step 5 still refuses an answer wider than 8,000 ms. The guarantee
  ("if `quorum` counted intervals contain true time, the answer does") needs no per-source bound,
  so safety is unchanged. Today int08h and time.txryan.com form quorum 2 of 2, and the answer lies
  within the narrower interval. Cost: two reachable sources at quorum 2 tolerate no outage and no
  liar until a third source answers.
- **B: keep the design.** The clock stays unavailable until a third source with a radius of
  3 s or less answers. Slack writes stay blocked.
- **C: quorum 1 over the reachable sources.** That is the single-source trust the decision
  replaced.

## Decided (2026-10-14)

Option A. A verified source counts toward the quorum whatever its own radius; the combined answer
stays capped at 8,000 ms. The design's "Width cap" section and the clock model say so first.
Until a third source answers, the clock tolerates no outage and no lying source, and the release
note states it. `roughtime-1` is removed: no source answers it and no reply can be recorded.
