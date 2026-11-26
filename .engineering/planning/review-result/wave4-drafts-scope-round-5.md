---
format: aep.planning-md/3
id: review-result:wave4-drafts-scope-round-5
kind: review-result
status: active
title: Scope critic, round 5, two drafts of wave 4
relations:
- reviews: story:the-sixteen-ess-model-tests-read-schemas-generated-once-per-run
- reviews: story:a-provider-budget-that-ends-in-an-unanswered-tls-handshake-is-marked-as-the-providers-timeout
revision: 1
---
## Verdict

Read at d7124123d. No build, no test and no `ess` call was run. Both approvals are for the
text at that commit.

- Sixteen ess_model tests: approve. No finding.
- Provider budget that ends in an unanswered TLS handshake: approve. No finding.

## The two warnings of round 4

- ess_model: resolved. The line on the 12 other test files says that 42 counts test
  functions, not calls, names `fresh_checkout_regeneration.rs` as running the
  `connectors-build` binary, holds no arithmetic on their time and ends with "No story".
  Verified: that test file runs the binary, and the binary has call sites of `ess generate`
  in seven source files.
- Provider budget: resolved. "Defect" says the run with 300 ms ends marked after this story
  too and that acceptance 1 uses 1,000 ms for that reason. The summary carries "with at least
  500 ms of the budget left".

## The other changes

The ess_model draft adds no scope and drops none.

- The frontmatter scope and acceptance 7 name `adapters/sql/tests/ess_model.rs` alone.
- Item 4 and acceptance 3 change the copy of `domains/profiles.yaml` alone;
  `adapters/sql/spec/ess` is read, for the key and as the default source directory.
- The child test's name and variable stay inside the one file; no file outside the planning
  store names the variable.
- Line 20 of `domains/profiles.yaml` is the first `lte: 64` of the file, under `ProfileId`.
- Not verified in source: the two `maxLength` values of the schema file. They rest on the
  draft's own read of 2026-11-25.

The provider budget adds no scope and drops none. `post_json` in `http.rs` is the read POST;
acceptance 5 names the right function. The scope list is unchanged.

## Not established

- Whether the 12 other files make more calls of `ess generate` than the ones read.
