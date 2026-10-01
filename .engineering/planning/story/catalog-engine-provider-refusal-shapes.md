---
format: aep.planning-md/3
id: story:catalog-engine-provider-refusal-shapes
kind: story
status: implemented
title: An empty text body is the empty string, and a provider's quota 403 is reported as rate limited
relations:
- serves: vision:independent-contract-adapters
revision: 4
transitions:
- {from: "draft", to: "proposed", at: "2026-09-30T17:43:07Z", actor: "human:timo", revision: 2, decided_on: {"recorded":{"verification":1}}}
- {from: "proposed", to: "active", at: "2026-09-30T17:43:07Z", actor: "human:timo", revision: 3, decided_on: {"recorded":{"verification":1}}}
- {from: "active", to: "implemented", at: "2026-09-30T18:33:45Z", actor: "human:timo", revision: 4, decided_on: {"recorded":{"test_result":1,"verification":1}}}
---
## Source

Adversary pass 2 on Google Drive and Slides reads
(review-result:adversary-google-drive-slides-reads-pass-2, findings 1 and 2):

- `adapters/catalog/src/lib.rs:641-643` returns `null` for an empty 200 body before checking text
  mode, so an empty `files.export` (`text/csv` of an empty sheet) is `null` although the selection
  says text. Same code at b50ffba79, reached by GitLab's text selection too.
- `lib.rs:633` maps every 403 to `forbidden`; Google APIs signal quota with
  `403` and `error.errors[].reason` `rateLimitExceeded` / `userRateLimitExceeded` (Google's
  published Drive limits; not observed live), so a paging walk cannot tell quota from permission.

## Acceptance

- In text mode an empty 2xx body is `""`; `an_empty_text_export_is_the_empty_string` in
  `adapters/catalog/tests/google_reads_adversary_pass2.rs` asserts `""` (flipped from today's `null`).
- A selection may declare `rate_limit_reasons: [..]`; a 403 whose JSON body carries one of those
  reasons in `error.errors[].reason` or `error.status` is `rate_limited`. The Google selections
  declare `rateLimitExceeded` and `userRateLimitExceeded`;
  `a_drive_usage_limit_answer_is_rate_limited_not_forbidden` asserts `rate_limited`.
- Every other 403 stays `forbidden`.
