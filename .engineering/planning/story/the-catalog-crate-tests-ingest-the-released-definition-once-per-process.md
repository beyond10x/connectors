---
format: aep.planning-md/3
id: story:the-catalog-crate-tests-ingest-the-released-definition-once-per-process
kind: story
status: draft
title: One catalog crate test reads the committed lowering of the released definition; twelve keep lowering
summary: The committed lowering reaches one of thirteen tests (1.583 s); the other twelve lower a changed definition or test lower itself and are not claimed by a change
relations:
- decomposes: story:the-gates-test-step-ends-within-120-s-on-the-forge
- informed_by: test-plan:where-the-gates-test-step-spends-its-time
- serves: vision:independent-contract-adapters
- depends_on: story:the-catalog-provider-tests-read-a-committed-lowering-of-the-released-definition
scope:
- confidence: cited
  path: crates/connectors-catalog/tests/definition_published_features.rs
revision: 5
---
## Defect

Thirteen tests of the binary `connectors-catalog::main` lower the released definition
`connectors/github.yaml`. One lowering takes about 1.5 s. The plan counts eleven of them at 1 s or
more, 31.6 s of the test step.

This story was drafted as "the ingest is made once per process". Under nextest a process is one
test, so that is not one of the two accepted forms of the folder rule "Tests stay fast". The
revision asked for another seam: the tests read the lowering that
story:the-catalog-provider-tests-read-a-committed-lowering-of-the-released-definition commits,
except the tests that test `lower` itself. Read at 0.78.1, that seam reaches one test. The id of
this story keeps the old name; an id cannot be changed.

Source of the durations: the ones nextest printed in the forge's gate job 138270416028 of pull
request 291, 2026-11-24, as listed in test-plan:where-the-gates-test-step-spends-its-time (group
L2): one test of 7.4 s and 10 tests of 1 s to under 5 s, 24.2 s. The plan read the sources at
0.77.2.

The thirteen tests, read at 0.78.1. Both files have a helper `lower_file`, which lowers the
released file unchanged, and a helper `lower`, which lowers the released head plus one appended
operation (`definition_published_features.rs` lines 58 to 77,
`definition_published_features_adversary.rs` lines 35 to 58).

`crates/connectors-catalog/tests/definition_published_features.rs`, 7 tests:

- `the_released_github_definition_lowers` (line 272), 1.583 s: one lowering of the released file,
  unchanged.
- `embedded_text_where_the_engine_refuses_it_is_refused`, 7.4 s: lowers changed definitions.
- `query_fixed_reaches_the_selection`, 4.411 s; `an_operation_rate_limit_headers_replace_the_default`,
  3.103 s; `query_folds_reach_the_selection`, 2.991 s; `embedded_text_reaches_the_selection`;
  `defaults_rate_limit_headers_reach_every_operation`: each lowers changed definitions.

`crates/connectors-catalog/tests/definition_published_features_adversary.rs`, 6 tests:

- `lowering_the_released_definition_twice_writes_the_same_bytes` (line 74), 2.991 s: lowers the
  released file twice and compares. It tests `lower` itself.
- `query_fixed_and_fold_key_order_does_not_change_the_selection`,
  `a_repeated_query_fixed_key_is_refused`,
  `a_partial_operation_rate_limit_headers_is_refused_not_merged_with_the_default`,
  `embedded_text_members_reach_the_selection_as_written`,
  `defaults_reach_writes_and_reads_beside_an_embedded_text_read_as_documented`: each lowers
  changed definitions.

11 against 13: the plan counts the tests of 1 s or more. Which two of the thirteen ran under 1 s
is not in the plan. Inferred: the two refusal tests (`a_repeated_query_fixed_key_is_refused`,
`a_partial_operation_rate_limit_headers_is_refused_not_merged_with_the_default`), if `lower`
refuses before its costly steps.

## Change

1. `the_released_github_definition_lowers` reads the committed directory
   `crates/connectors-catalog/tests/fixtures/released-github-lowering/` in place of lowering, and
   keeps its assertions. That the committed bytes are a lowering of the released file is held by
   the fast check and the lane test of
   story:the-catalog-provider-tests-read-a-committed-lowering-of-the-released-definition.
2. Twelve tests keep lowering: the eleven that lower a changed definition, because a changed
   definition is not the committed one, and
   `lowering_the_released_definition_twice_writes_the_same_bytes`, because it tests `lower`.
3. No production code changes. `crates/connectors-catalog/src/definition.rs` is not written.

The plan's estimate of 5.0 s for the eleven (0.3 s per test) is dropped: it rested on an ingest
that every test reuses, and no accepted form gives that without a change of `lower`.

Estimate after for the one test (computed; not timed): about 0.14 s, the mean of a test of the
provider binary that lowers nothing (the plan: 7.5 s over 55 `slack` tests). Before: 1.583 s.

## Acceptance

1. `connectors-catalog::main::definition_published_features::the_released_github_definition_lowers`
   passes in the test step of the forge's "repository gate" job of this story's pull request in at
   most 1 s. Before: 1.583 s. Estimate: 0.14 s. Bound: 1.5 × 0.14 = 0.21, rounded up to 1 s.
2. `grep` finds no call of `lower_file` in that test.
3. The other twelve tests pass unchanged.

## Scope

- `crates/connectors-catalog/tests/definition_published_features.rs`: the one test.

Read, not written: `crates/connectors-catalog/tests/fixtures/released-github-lowering/`, which
story:the-catalog-provider-tests-read-a-committed-lowering-of-the-released-definition creates.
This story `depends_on` it.
story:eight-adapter-and-catalog-tests-drop-waits-that-prove-nothing edits
`crates/connectors-catalog/tests/bundle_adversary_2.rs`, another file of the same test binary.

## Not in this story

- The ten tests of the plan's count that keep lowering: 7.4 s
  (`embedded_text_where_the_engine_refuses_it_is_refused`, one of the 81 tests of 5 s or more) and
  about 22.6 s (computed: 24.2 s less 1.583 s). No story of this set names a change for them.
  They need a seam that lets `lower` start from a saved ingest of the released head, kept on disk
  under `target/` and keyed by a digest. That is a change of
  `crates/connectors-catalog/src/definition.rs`. With it this story would come before the provider
  story, because that story's digest covers `definition.rs`.
- The provider-side tests (group J):
  story:the-catalog-provider-tests-read-a-committed-lowering-of-the-released-definition.
- The other tests of `connectors-catalog::main` in the band of 1 s to under 5 s that the plan
  names in `definition_auth` and `definition_feed`, and the tests of every other group.
- The gate failing when its test step runs over 120 s:
  story:the-gates-test-step-ends-within-120-s-on-the-forge.
- Not established: which two tests ran under 1 s; the cost of `definition::lower` taken apart and
  which sub-step dominates (the plan did not measure it); the "after" figure on the forge.
