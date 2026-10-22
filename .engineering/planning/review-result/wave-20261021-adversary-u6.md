---
format: aep.planning-md/3
id: review-result:wave-20261021-adversary-u6
kind: review-result
status: active
title: Adversary pass over wave 20261021, unit U6
summary: Five findings on the U6 fixes; three fixed in U8, one corrected in U7, one pinned
relations:
- reviews: story:catalog-feed-engine-extensions
revision: 1
---
# Adversary pass over wave 20261021, unit U6

**Verdict: five findings. Three fixed in unit U8, one already corrected in U7, one pinned by a new case.**

An adversary pass on 2026-10-22 over `c1866ffa0..caf75e173` (U6, the fixes from the first pass) attacked the deferred watermark judgement, the seen set kept at the instant, the load refusal of a binding's `next` status and the stated bounds.

| Finding | Area | Reach | Disposition |
|---|---|---|---|
| R1 | the watermark ceiling counts the container id, which the Ceilings row did not say | test fixtures only | the row and the ceiling message name the container id (`356e46d24`); the container id length stays unbounded |
| R2 | a watermark the binding issued and cannot resume answered `unavailable`, against the feed family's `stale_cursor` | shipped GitLab; predates the wave | `stale_cursor` (`356e46d24`); 100 or more merge requests in one millisecond still bring a first read back to the same watermark (gitlab.md) |
| R4 | the restart claim after a full re-read window | test fixtures only | already corrected in `94858f74b` |
| R5 | a listing 404 after the lookup read differently from an unknown container | shipped GitLab; predates the wave | answers as an unknown container (`356e46d24`) |
| R6 | U6's guard for a full answer of held items had no test; deleting it left 88 cases green | test fixtures only | pinned by the adversary's case |

The cases are kept in `adapters/catalog/tests/feed_wave1021_adversary_u6.rs`, without R4. R1, R2 and R5 were red against `caf75e173`. All are green after `356e46d24` (`cargo test -p connectors-catalog-provider`: 849 passed, 0 failed), merged into `wave/20261021` as `d0c32e43c`.
