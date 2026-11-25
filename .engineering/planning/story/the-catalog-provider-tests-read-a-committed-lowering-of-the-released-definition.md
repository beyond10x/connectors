---
format: aep.planning-md/3
id: story:the-catalog-provider-tests-read-a-committed-lowering-of-the-released-definition
kind: story
status: active
title: '40 catalog provider tests read a committed lowering of the released definition: about 94.9 s become an estimated 9.7 s'
summary: engine() lowers the released definition per call, about 1.5 s each; a committed generated directory with a digest of its sources replaces it; 26 tests that lower a changed definition keep theirs
relations:
- decomposes: story:the-gates-test-step-ends-within-120-s-on-the-forge
- informed_by: test-plan:where-the-gates-test-step-spends-its-time
- serves: vision:independent-contract-adapters
- depends_on: story:the-lane-slow-runs-the-ignored-timing-family
scope:
- confidence: cited
  path: adapters/catalog/tests/definition_published_features_adversary.rs
- confidence: cited
  path: adapters/catalog/tests/github_definition_reads.rs
- confidence: cited
  path: adapters/catalog/tests/github_definition_reads_adversary.rs
- confidence: cited
  path: adapters/catalog/tests/main.rs
- confidence: inferred
  path: adapters/catalog/tests/released_lowering.rs
- confidence: inferred
  path: crates/connectors-catalog/tests/fixtures/released-github-lowering
revision: 12
transitions:
- {from: "draft", to: "proposed", at: "2026-11-25T07:57:05Z", actor: "human:operator", revision: 11}
- {from: "proposed", to: "active", at: "2026-11-25T07:57:05Z", actor: "human:operator", revision: 12}
---
## Defect

Forty tests of the binary `connectors-catalog-provider::main` lower the released definition
`connectors/github.yaml`, unchanged, again in each call of their helper `engine()`. One lowering
takes about 1.5 s (`definition::lower`: gunzip, sha256, ingest, bundle write). This is the mistake
the folder rule "Tests stay fast" names: an expensive product made per test.

Source of the durations: the ones nextest printed in the forge's gate job 138270416028 of pull
request 291, 2026-11-24, as listed in test-plan:where-the-gates-test-step-spends-its-time (group
J). The plan read the sources at 0.77.2. The function names and counts below were read in the tree
of 0.78.1; the lines cited in this story are the same at 0.78.2.

The plan's figures for group J: 66 tests, 175.4 s. Four tests of 5 s or more (35.9 s; the four
rows, printed with one decimal, sum to 35.8 s):

- 11.9 s, `adapters/catalog/tests/github_definition_reads_adversary.rs`,
  `a_spent_limit_with_a_past_missing_or_unreadable_reset_is_rate_limited_without_a_wait`: 8 calls
  of `spent_limit`, each builds `engine()`.
- 9.0 s, same file, `the_raw_reads_hold_exactly_1_mib`: `raw_read` builds `engine()` per call,
  with base64 bodies of 1 MiB.
- 7.5 s, `adapters/catalog/tests/github_definition_reads.rs`,
  `each_list_refuses_a_per_page_above_its_bound_before_any_request`: `engine()` per list id.
- 7.4 s, same file, `each_list_passes_page_and_per_page_through_unchanged`: `engine()` per list
  id.

And 62 tests of 1 s to under 5 s, 139.5 s: `definition_published_features_adversary.rs` 32 tests
89.5 s, `github_definition_reads_adversary.rs` 17 tests 29.6 s, `github_definition_reads.rs` 13
tests 20.4 s.

Read at 0.78.1, which of them a committed lowering reaches:

- `github_definition_reads.rs`: 15 tests, all reach `engine()`.
- `github_definition_reads_adversary.rs`: 19 tests, all reach `engine()` (through `raw_read`,
  `spent_limit` or directly).
- `definition_published_features_adversary.rs`: 33 tests. Six call `engine()`: the tests whose
  name starts with `the_released_` (lines 101 to 245). 26 call `lowerer_refuses` or `control`,
  which lower a changed definition through `outcome` (line 290): the released head plus one
  appended operation. A changed definition is not the committed one, so these 26 keep lowering.
  One, `the_lowerer_limits_are_the_engine_limits` (line 358), lowers nothing.

So the committed lowering reaches 15 + 19 + 6 = 40 tests, not the 66 of the plan.

66 against 67: the three files hold 15 + 19 + 33 = 67 test attributes. The plan's 66 are the tests
of 1 s or more. The 67th is `the_lowerer_limits_are_the_engine_limits` (inferred: it lowers
nothing, so it ran under 1 s).

Before, for the 40 tests: 35.9 + 20.4 + 29.6 = 85.9 s for the 34 tests of the two reads files (the
plan), plus about 9.0 s for the six `the_released_` tests (inferred: 6 × 1.5 s, one lowering each;
the plan gives no time per test for them). About 94.9 s.

## Change

The first form of the folder rule: a committed generated file with a digest of its sources. No
seam in product code.

1. New: the fixture directory `crates/connectors-catalog/tests/fixtures/released-github-lowering/`,
   beside the definitions tree it is made from. It holds what `definition::lower` returns and
   writes for `crates/connectors-catalog/tests/fixtures/definitions-0.5.2/connectors/github.yaml`:
   the bundle directory, the operations as a file, and the record `inputs.json` with the digest
   of the sources.
2. The digest's inputs: every file under
   `crates/connectors-catalog/tests/fixtures/definitions-0.5.2/` and every `.rs` file under
   `crates/connectors-catalog/src/`. `Cargo.lock` is not an input: a dependency that changes the
   bytes is caught by the lane test of point 5. The function that computes the digest takes its
   input roots as a parameter, so that a test can give it a temporary directory.
3. New: the module `adapters/catalog/tests/released_lowering.rs`, entered in
   `adapters/catalog/tests/main.rs`. It holds one function that builds the engine from the
   committed directory. The three copies of `engine()` call it and no longer call
   `definition::lower`.
4. The fast check is a test of that module, so it runs in the gate's test step:
   `released_lowering::the_committed_lowering_is_current` computes the digest of the inputs and
   compares it with `inputs.json`. It lowers nothing. Its precedent is
   `the_committed_metadata_emissions_are_current` in
   `crates/connectors-build/src/metadata_conformance.rs` (line 966), whose failure names the
   command that regenerates (lines 988 to 992). The fast check does the same: its failure says
   that the committed lowering is stale, lists the input files that changed (the record holds one
   sha256 per input file, as the precedent's record does), and prints
   the regeneration command of point 5.
5. Regeneration from the sources, outside the gate: the ignored test
   `released_lowering::slow_lane_the_committed_lowering_equals_a_fresh_lowering` lowers the
   released definition and compares every byte with the committed directory. With the variable
   `CONNECTORS_WRITE_RELEASED_LOWERING` set it writes the directory and `inputs.json` in place of
   comparing; that is how a developer regenerates. The regeneration command is
   `CONNECTORS_WRITE_RELEASED_LOWERING=1 cargo test --locked -p connectors-catalog-provider --test main -- --ignored --exact released_lowering::slow_lane_the_committed_lowering_equals_a_fresh_lowering`.
   The comparison is one function of the module that takes two directories and returns the first
   file whose bytes differ; the lane test calls it with the committed directory and the fresh
   lowering. The rule of story:the-lane-slow-runs-the-ignored-timing-family puts the lane test in
   the timing family by its name. This story edits neither `ignored.rs` nor `slow.yml`.
6. The rule for every later change: who changes an input of the digest regenerates the committed
   lowering in the same change. An input is any file under
   `crates/connectors-catalog/tests/fixtures/definitions-0.5.2/` and any `.rs` file under
   `crates/connectors-catalog/src/`. The change runs the regeneration command of point 5 and
   commits the directory with the new `inputs.json`; when the lowering's bytes did not change,
   only `inputs.json` changes. The rule is written in the module comment of `released_lowering.rs`
   and is enforced by the fast check, which is red in the gate until the change regenerates.
   Eight drafts outside this set write an input by their recorded scope, each a file under
   `crates/connectors-catalog/src/`: story:catalog-zendesk-reads (`inventory.rs`),
   story:definition-amendments-pinned-by-digest (`amendment.rs`, `definition.rs`),
   story:definition-files-in-user-config (`definition.rs`), story:definitions-pin-covers-catalog
   (`import.rs`), story:slack-sdk-upstream-import (`import.rs`),
   story:definitions-default-write-guard (`definition.rs`), story:agent-tool-projection
   (`definition.rs`, `import.rs`), story:slack-four-profile-definition-generated
   (`definition.rs`). Whichever of them merges after this story meets the fast check and follows
   the rule; this story adds no edge to them and does not edit them.
7. The 26 tests that lower a changed definition are not changed.
   `lowering_the_released_definition_twice_writes_the_same_bytes`, a test of the catalog crate,
   keeps lowering twice.

Estimate after (computed from the plan's figures; not timed): 4.7 s for the four tests of 5 s or
more (the plan) and 36 × 0.14 = 5.0 s for the other 36, sum 9.7 s. The 0.14 s per test is the
plan's: the mean of the 55 `slack` tests of the same binary (7.5 s over 55 tests).

## Acceptance

1. The 40 tests named above (all tests of `github_definition_reads` and
   `github_definition_reads_adversary`, and the six `the_released_` tests of
   `definition_published_features_adversary`) pass, and their durations in the test step of the
   forge's "repository gate" job of this story's pull request sum to at most 15 s. Before: about
   94.9 s, of which 9.0 s inferred. Estimate: 9.7 s. Bound: 1.5 × 9.7 = 14.55, rounded up to 15 s.
   The numbers are read from the durations nextest prints in that job.
2. `grep` finds no call of `definition::lower` in `github_definition_reads.rs` and
   `github_definition_reads_adversary.rs`, and one in
   `definition_published_features_adversary.rs` (in `outcome`).
3. The fast check passes in the gate's test step:
   `connectors-catalog-provider::main::released_lowering::the_committed_lowering_is_current`.
   Alone it runs with
   `cargo test --locked -p connectors-catalog-provider --test main -- --exact released_lowering::the_committed_lowering_is_current`.
4. One changed byte of an input gives another digest:
   `released_lowering::a_changed_input_byte_changes_the_digest` passes in the gate's test step. It
   writes two files into a temporary directory, computes the digest with that directory as the
   input root, changes one byte of one file and asserts that the second digest differs from the
   first. Alone it runs with
   `cargo test --locked -p connectors-catalog-provider --test main -- --exact released_lowering::a_changed_input_byte_changes_the_digest`.
5. A stale digest names the way out: `released_lowering::a_stale_digest_prints_the_regeneration_command`
   passes in the gate's test step. It calls the check of Change point 4 with a recorded digest that
   differs from the current one and asserts that the failure text holds the regeneration command
   of Change point 5, character for character. Alone it runs with
   `cargo test --locked -p connectors-catalog-provider --test main -- --exact released_lowering::a_stale_digest_prints_the_regeneration_command`.
6. `connectors-catalog-provider::main::released_lowering::slow_lane_the_committed_lowering_equals_a_fresh_lowering`
   passes in the lane `slow`. The lane's command
   `cargo run --locked --bin connectors-build -- ignored --family timing --required --only-prerequisite provider`
   lists it as `passed`. Alone it runs with
   `cargo test --locked -p connectors-catalog-provider --test main -- --ignored --exact released_lowering::slow_lane_the_committed_lowering_equals_a_fresh_lowering`.
7. The byte comparison fails on one changed byte:
   `released_lowering::a_changed_byte_in_a_copy_of_the_fixture_fails_the_comparison` passes in the
   gate's test step. It copies the committed directory into a temporary directory, changes one
   byte of one file of the copy, calls the comparison function of Change point 5 with the committed
   directory and the copy, and asserts that the function fails and names the changed file. It
   lowers nothing. Alone it runs with
   `cargo test --locked -p connectors-catalog-provider --test main -- --exact released_lowering::a_changed_byte_in_a_copy_of_the_fixture_fails_the_comparison`.
8. The gate's ignored step still passes: the new ignored test is not unknown to it.

## Scope

- `adapters/catalog/tests/github_definition_reads.rs`: `engine()`.
- `adapters/catalog/tests/github_definition_reads_adversary.rs`: `engine()`.
- `adapters/catalog/tests/definition_published_features_adversary.rs`: `engine()` only.
- `adapters/catalog/tests/released_lowering.rs` (new) and `adapters/catalog/tests/main.rs` (one
  line).
- `crates/connectors-catalog/tests/fixtures/released-github-lowering/` (new, generated).

story:the-catalog-crate-tests-ingest-the-released-definition-once-per-process reads the fixture
directory, writes none of these files and `depends_on` this story.
story:eight-adapter-and-catalog-tests-drop-waits-that-prove-nothing edits other files of
`adapters/catalog/tests/` and not `main.rs`.
`crates/connectors-catalog/tests/definition_published_features_adversary.rs` is another file with
the same file name, in the catalog crate; this story does not write it.

The generated directory has later writers outside this set: every change that follows the rule of
Change point 6 rewrites `inputs.json`, and the bundle when the lowering changed. Two such changes that
are open at once both write `inputs.json`; the one that merges second regenerates again after it
takes the first.

## Not in this story

- The 26 tests of `definition_published_features_adversary.rs` that lower a changed definition:
  about 80.5 s (inferred: the module's 89.5 s less the 9.0 s above). No story of this set names a
  change for them. They need a seam that lets `lower` start from a saved ingest of the released
  head; that is a change of `crates/connectors-catalog/src/definition.rs`.
- The other tests of `connectors-catalog-provider::main` in the band of 1 s to under 5 s (13 tests,
  27.3 s, the plan) and the tests of every other group.
- An edit of the eight drafts named in Change point 6, and an edge to them.
- The lane `slow` itself: story:the-lane-slow-runs-the-ignored-timing-family.
- The gate failing when its test step runs over 120 s:
  story:the-gates-test-step-ends-within-120-s-on-the-forge.
- Not established: whether what `lower` returns (`Lowered`, `definition.rs` line 1573) can be
  written to files and read back without a change of the catalog crate; whether a lowering on the
  forge's runner writes the same bytes as one on a developer's machine; the time of each of the
  six `the_released_` tests in the job; the cost of `definition::lower` taken apart; every "after"
  figure on the forge; which further drafts change an input without a recorded scope on it (the
  critic inferred ten more from their defect sites; they were not read here).
