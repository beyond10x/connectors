---
format: aep.planning-md/3
id: review-result:connectors-wave-2026-11-04-u2-adversary
kind: review-result
status: active
title: Adversary pass on the GitHub definition reads (wave 2026-11-04)
relations:
- reviews: story:catalog-github-reads
revision: 1
---
## Adversary pass: unit u2-github-reads (catalog-github-reads)

Verdict: holds. Added 19 cases in `adapters/catalog/tests/github_definition_reads_adversary.rs`, all
passing on 995ec56d2; committed as e34fd8032. Pre-existing findings: none.

Findings (all introduced, all closed by the added cases):

1. `adapters/catalog/tests/github_definition_reads.rs:409` (should-fix): the search page bound was tested
   only at 1001; a maximum of 999 or a minimum of 0 survived. Held now by
   `review_requests_list_sends_page_1000_the_last_page_its_bound_admits` and
   `review_requests_list_refuses_page_0_before_any_request`.
2. `github_definition_reads.rs:387` (note): the `per_page` minimum was untested. Held by
   `each_list_refuses_per_page_0_before_any_request`, `each_list_sends_per_page_1_its_minimum`.
3. `github_definition_reads.rs:129` (note): a caller's `q`, `sort`, `order`, `filter` and a widening
   `since` were not held as refused. Held by the three `..._refuses_a_caller_...` cases.
4. `website/docs/connectors/github.md:35` (note): the directory, binary and 1 MiB claims had no test or
   output. Held by `file_raw_refuses_a_directory`, `file_raw_refuses_a_binary_file`,
   `the_raw_reads_hold_exactly_1_mib`, `file_raw_refuses_a_file_github_does_not_inline_as_too_large`.
5. `adapters/catalog/tests/definition_published_features_adversary.rs:200` (note): a past or missing reset
   was asserted only by code. Held by
   `a_spent_limit_with_a_past_missing_or_unreadable_reset_is_rate_limited_without_a_wait` and three more.

Checks: github_definition_reads 15 passed; github_definition_reads_adversary 19 passed; fmt and clippy
`-D warnings` exit 0.
