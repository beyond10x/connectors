---
format: aep.planning-md/3
id: review-result:wave-20261021-adversary-u0-u4
kind: review-result
status: active
title: Adversary pass over wave 20261021, units U0 to U4
summary: Four findings on the feed engine and bindings; all fixed in unit U6
relations:
- reviews: story:catalog-feed-engine-extensions
- reviews: story:jira-feed-binding
- reviews: story:confluence-token-kind-binding
revision: 1
---
# Adversary pass over wave 20261021, units U0 to U4

**Verdict: four findings; all four fixed in unit U6 before the release.**

An adversary pass on 2026-10-22 over `e26481df09..2c527a716` (U0 to U4) tried to break the feed engine extensions, the bindings and the Jira feed against `contracts/catalog/v1alpha1/semantics.md` and `contracts/datasources/feed/v1alpha1/semantics.md`. It wrote 4 red cases.

| Finding | Area | Reach | Disposition |
|---|---|---|---|
| F1 | a time watermark the read cannot resume was judged before the listing answered, against the watermark-scope rule | test fixtures only | fixed in `3072be7b46` |
| F2 | a minute-precision watermark passed its ceiling at about 290 items changed in one minute and stayed `unavailable` | test fixtures only | fixed in `3072be7b46`: the seen set holds the pairs at the kept instant only |
| F3 | watermark handling in the feed engine | a shipped selection; predates the wave | fixed in `3072be7b46` |
| N1 | a binding `next` status other than 401 or 403 moved on to the next binding | configuration | fixed in `3072be7b46`: refused at load |

`adapters/catalog/tests/feed_utc_offsets.rs` asserted the opposite of the contract for F1 and was changed to the contract. The adversary's cases are kept as `adapters/catalog/tests/{feed,gitlab_feed,jira_feed}_wave1021_adversary.rs`. All 4 were red against `2c527a716` and are green after `3072be7b46` (`cargo test -p connectors-catalog-provider`: 843 passed, 0 failed), merged into `wave/20261021` as `caf75e173`.
