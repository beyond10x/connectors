---
format: aep.planning-md/3
id: review-result:wave-20261021-adversary-u5
kind: review-result
status: active
title: Adversary pass over wave 20261021, unit U5
summary: Four contract claims the Confluence and Jira feeds did not hold; contract text corrected
relations:
- reviews: story:confluence-feed-binding
- reviews: story:jira-feed-binding
revision: 1
---
# Adversary pass over wave 20261021, unit U5 (Confluence page feed)

**Verdict: four findings, all contract text that claimed more than the engine does; corrected in `94858f74b`. Two earlier findings also apply to the Confluence fixture and were already fixed.**

An adversary pass on 2026-10-22 over `2c527a716..c1866ffa0` attacked the Confluence REST v1 page feed. Every finding is reachable only through the test fixtures, since no shipped selection carries a feed. Each rests on a provider fact or a volume nobody has observed.

| Finding | Claim in the contract | What the engine does | Disposition |
|---|---|---|---|
| A | a change that moves `lastmodified` without a new version is dropped as held | every read of the space answers `unavailable` until the page gets a new version | contract corrected (`confluence-v1.md`) |
| B | none on the search cursor | under a positional cursor, a page versioned between two searches of one call can be skipped while the read reports complete | stated as not established (`confluence-v1.md`) |
| C | after the re-read wall, a first read returns every page again | the first read reaches the same wall | claim removed (`confluence-v1.md`) |
| C2 | the same restart claim for the Jira feed (predates U5) | the same | claim removed (`jira.md`) |

Earlier findings checked against the Confluence fixture: F1 and F2 apply and are held by `adapters/catalog/tests/confluence_v1_feed.rs` since `3072be7b46`. F3 and N1 do not apply. The adversary's red cases assert the old claims and are not kept; the corrected text is merged into `wave/20261021` as `e393cc23b`.
