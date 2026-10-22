---
format: aep.planning-md/3
id: review-result:feed-extensions-security-20261022
kind: review-result
status: active
title: Security review of the feed engine extensions
summary: Six findings on unit U1; five fixed in the unit, one carried to the Jira binding
relations:
- reviews: story:catalog-feed-engine-extensions
revision: 1
---
# Security review of the feed engine extensions (wave 20261021)

**Verdict: six findings; findings 1-5 fixed in the unit before any binding built on it, finding 6 carried to `story:jira-feed-binding`.**

Reviewed 2026-10-22 by an independent security-reviewer pass over unit U1 (`unit/w1021-feed-engine`, commit `75efd72636`) against `contracts/catalog/v1alpha1/semantics.md` §3.3 and `adapters/catalog/src/feed.rs`. Every finding was rated infeasible to exploit through the shipped selections; each was still held to the contract.

| Finding | Area | Disposition |
|---|---|---|
| 1, 2, 4, 5 | the query, next-link, relative-link, paging and compatibility rules (one case each in the file below) | fixed in `e67277d427` |
| 3 | next-link and relative-link host checks refused valid API bases (IPv6, underscore host, trailing dot): a regression | fixed in `e67277d427` |
| 6 | the query time literal is read in UTC while the provider reads it in the caller's zone | carried to `story:jira-feed-binding` (declared `utc_offsets` widening) |

The review's cases live in `adapters/catalog/tests/feed_extensions_security_review.rs`: 5 were red against `75efd72636`, and all 12 are green after `e67277d427` (`cargo test -p connectors-catalog-provider`, 794 passed), merged into `wave/20261021` as `91419baaa9`.
