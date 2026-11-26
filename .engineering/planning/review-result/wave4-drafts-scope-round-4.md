---
format: aep.planning-md/3
id: review-result:wave4-drafts-scope-round-4
kind: review-result
status: active
title: Scope critic, round 4, two drafts of wave 4
relations:
- reviews: story:the-sixteen-ess-model-tests-read-schemas-generated-once-per-run
- reviews: story:a-provider-budget-that-ends-in-an-unanswered-tls-handshake-is-marked-as-the-providers-timeout
revision: 1
---
## Verdict

Read at 9fc5d4b18. No build, no test and no `ess` call was run.

- Sixteen ess_model tests: needs-revision, one warning.
- Provider budget that ends in an unanswered TLS handshake: needs-revision, one warning.

## The findings of round 3 on the ess_model draft

Both are resolved in the text as it stands.

1. The test `the_model_admits_exactly_the_read_profiles_the_executable_admits` now ends with
   "No story".
2. The 12 other test files that start `ess generate` are named, with "No story claims them".
   The story on the 1298 s names four other binaries, and none of the 12 files is in them.

## The ess_model draft

- The frontmatter scope, the Scope section, item 1 of "Change" and acceptance 7 all name
  `adapters/sql/tests/ess_model.rs` alone.
- The counts hold: 12 files besides `ess_model.rs`, 6 under `adapters/*/tests` and 6 under
  `crates/connectors-build/tests`, with 25 and 17 test functions, 42 in all.
- Leaving the 12 files without a story is a stated exclusion that holds: the source names the
  16 tests of this one file.

Finding (warning): "a run makes at most 42 such calls" is wrong. The tests of
`crates/connectors-build/tests/fresh_checkout_regeneration.rs` run the `connectors-build`
binary, which starts `ess generate` itself in several places, so one test function makes
several calls. The bound of 3.7 s rests on the 42. The line says that 42 counts test
functions, names that file, drops the arithmetic and keeps "No story".

## The provider budget after its revision

- Verified: `with_read_posts` is called in production by `adapters/grafana/src/local.rs` and
  `adapters/tavily/src/local.rs` alone; `with_provider_budget` by
  `adapters/catalog/src/local.rs` alone, in three places.
- Both exclusions stand under "Not in this story" with their reasons: the read POST and the
  band under 500 ms. "No story" is acceptable for each.
- The three contract texts cite lines 11 to 35, 30 to 36 and 11 to 25 of `http.rs`. Three
  generated pages under `website/docs/reference` repeat the citations. "Adds no line above
  line 37" is enough for all six.

Finding (warning): the one measured run of "Defect" used a budget of 300 ms, which is inside
the band under 500 ms that the story leaves as today, and the summary promises the change
without the floor. The body says that the run with 300 ms stays marked after this story and
that acceptance 1 uses 1,000 ms for that reason; the summary takes "with at least 500 ms of
the budget left".

## Not established

- Whether the 12 files make more calls than the one literal per file: one file and the call
  sites of `connectors-build` were read, no other helper chain.
- How often the production catalog adapter connects with under 500 ms of budget left.
