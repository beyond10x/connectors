---
format: aep.planning-md/3
id: story:feed-cursor-unadvanced-is-unavailable
kind: story
status: draft
title: A cursor feed answer that is incomplete and does not advance is unavailable
summary: 'review run 3 C2 F1: returned as a page with an unchanged watermark'
relations:
- serves: vision:independent-contract-adapters
revision: 1
---
## Outcome

A cursor-paged feed answer that is not complete and does not advance the cursor (cursor absent,
empty, or equal to the one sent) is refused as `unavailable`, not returned as a page with an unchanged
`next_watermark`.

## Why later

Post-0.54.0 review run 3 (lane C2 F1, 2026-10-19): `adapters/catalog/src/feed.rs:947-954` sets the
cursor only when the answer's `next` is non-empty and never refuses "not complete, no advance"; a
caller that follows `next_watermark` until `complete` re-reads the same page forever. Scratch test
`adapters/catalog/tests/verify_c2_1.rs` failed at line 67 for `{"changes":[],"has_more":true}`
(EXIT=101). Low: the only shipped feed (gitlab) uses `cursor.last`, which already refuses its
analogue (feed.rs:736-744), and the conformance harness bounds its loop
(`crates/connectors-build/src/feed_conformance.rs:776`).

## Acceptance

- The scratch cases (cursor absent, `""`, the value already sent) are tests in
  `adapters/catalog/tests/feed_engine.rs` and answer `unavailable`.
- `contracts/datasources/feed/v1alpha1/semantics.md` states the rule beside the `cursor.last` one.
