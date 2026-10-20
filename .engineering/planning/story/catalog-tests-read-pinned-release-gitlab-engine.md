---
format: aep.planning-md/3
id: story:catalog-tests-read-pinned-release-gitlab-engine
kind: story
status: draft
title: Catalog provider tests of gitlab and five smaller connectors read the pinned release; engine and security tests use a synthetic definition
relations:
- decomposes: epic:catalog-connectors-only-in-definitions
- serves: vision:independent-contract-adapters
- depends_on: story:definitions-pin-covers-catalog
revision: 4
---
## Outcome

The 45 test files listed under Scope stop reading `adapters/catalog/providers/` and
`adapters/catalog/generated/bundles/` by path:

- the 27 gitlab and smaller-connector files read lowered connectors from the pinned, installed
  release through the test-support function of `story:definitions-pin-covers-catalog`;
- the 18 engine and security files use a small synthetic definition of their own, lowered with
  the reference lowerer, and read nothing from the pinned release (epic, In scope: "The engine and
  lowerer tests keep a small synthetic definition of their own instead of a provider copy").

Epic step 3, first of two batches in `adapters/catalog/tests/`.

## Context

- The tests load a bundle with `bundle::load(&root().join("generated/bundles"), ...)`, read a
  selection from `providers/<name>/operations.json`, or write a native configuration whose
  `bundle_directory` and `operations_file` point into the tree (for example
  `adapters/catalog/tests/gitlab_mr_writes.rs:87,92,283,295`, `hubspot.rs:70,84,381,385`).
- The engine and security files use a real selection only as convenient data: `slack` in
  `adversary_credential_pass1.rs` and `binary_adversary.rs`, `google-gmail` in
  `body_keys_adversary.rs`, `gitlab` in `engine_adversary.rs`, `engine_adversary_pass2.rs` and
  `selection_withhold_adversary.rs`, `zendesk` in `security_reuse_source_token_origin.rs`, `jira`
  in both `write_postflight_present_adversary` files. A synthetic definition carries the shape each
  case needs. The reference lowerer is `connectors_catalog::definition`
  (`crates/connectors-catalog/src/definition.rs`), which `adapters/catalog` already depends on
  (`adapters/catalog/Cargo.toml`, `[dependencies]`). The `widgets` definition of
  `crates/connectors-catalog/tests/definition_layout.rs:54-59` is the precedent.
- `definition_auth.rs` asserts that every auth profile of every committed definition loads in the
  catalog provider. With a synthetic definition it holds the loading path only; the
  every-real-definition part is data, and is recorded under Removed tests if dropped.
- A test, or part of one, that uses an instance derived from the declarations compiled into the
  catalog provider (`adapters/catalog/build.rs:1-5`, `adapters/catalog/src/local.rs:348-413`)
  keeps doing so; that path is `story:instances-resolve-through-locked-index`.
- ADR Consequences: this repository loses "the per-provider tests that only exercise data". A
  test function whose assertions are only about provider data (which operations a selection
  holds, which source a bundle records) is removed rather than moved. Name patterns that point at
  such tests, for the implementor to confirm by reading each one: `shipped_*_selections_*`,
  `the_pinned_source_is_the_one_the_bundle_and_its_record_name`.
- Why the rename check builds first: `adapters/catalog/build.rs:13-16,29-32` reads both
  directories when the crate builds and panics on a missing one, so the check renames them only
  after the test binaries exist.
- File-disjoint from `story:catalog-tests-read-pinned-release-providers`. Neither batch touches
  `hand_written/` (its JSON carries tree paths behind the placeholder `/absolute/path/`; only
  `connector_declarations.rs:104-135`, in neither batch, maps it onto the tree), and neither
  touches a manifest or `Cargo.lock` (`story:definitions-pin-covers-catalog` adds the
  dev-dependency).

## Domain relations

None beyond those of `story:definitions-pin-covers-catalog`.

## Scope

All under `adapters/catalog/tests/`, cited by a search for `providers` and `generated/bundles`
on 9201b1177.

- gitlab (21), pinned release: `adversary_gitlab_mr_writes.rs`, `gitlab_ci_project_writes.rs`,
  `gitlab_ci_project_writes_adversary.rs`, `gitlab_code_search.rs`, `gitlab_commit_reads.rs`,
  `gitlab_commit_reads_adversary.rs`, `gitlab_commit_reads_adversary_pass2.rs`,
  `gitlab_deployment_reads.rs`, `gitlab_deployment_reads_adversary.rs`, `gitlab_feed.rs`,
  `gitlab_feed_u23_adversary.rs`, `gitlab_feed_u23_adversary_pass2.rs`, `gitlab_issue_create.rs`,
  `gitlab_mr_reads.rs`, `gitlab_mr_variants.rs`, `gitlab_mr_writes.rs`,
  `gitlab_raw_file_languages.rs`, `gitlab_repository_reads_adversary.rs`,
  `gitlab_repository_tree_reads.rs`, `gitlab_tags_releases.rs`, `shipped.rs`
- hubspot, openrouter, opsgenie, runpod, zendesk (6), pinned release: `hubspot.rs`,
  `openrouter.rs`, `opsgenie.rs`, `runpod.rs`, `runpod_adversary.rs`, `zendesk.rs`
- engine and security (18), synthetic definition: `adversary_credential_pass1.rs`,
  `binary_adversary.rs`, `body_keys_adversary.rs`, `credential_revision_security.rs`,
  `definition_auth.rs`, `engine_adversary.rs`, `engine_adversary_pass2.rs`, `gateway_prefix.rs`,
  `gateway_prefix_adversary.rs`, `guard_read_fixed_body_adversary.rs`, `parameter_types.rs`,
  `parameter_types_adversary.rs`, `security_reuse_source_token_origin.rs`,
  `selection_bounds_adversary.rs`, `selection_credential.rs`, `selection_withhold_adversary.rs`,
  `write_postflight_present_adversary.rs`, `write_postflight_present_adversary_2.rs`
- the synthetic definitions and their sources (inferred: a directory under
  `adapters/catalog/tests/`)

## Not in this story

- `connector_declarations.rs`, `guide_instances.rs`: they hold the compiled-in declarations and
  their byte equality with the committed `connector.json`
  (`story:instances-resolve-through-locked-index`; D11 question, open on the epic).
- `bundle_drift.rs`: it holds the committed bundles to the pipeline and goes with them.

## Acceptance

- With the `connectors-catalog-provider` test binaries built first and
  `adapters/catalog/providers`, `adapters/catalog/generated/bundles` and
  `crates/connectors-catalog/tests/fixtures/definitions` then renamed away, every
  file of this batch still in the tree passes when those binaries run.
- None of the 18 engine and security files calls the pinned-release test support; each runs on a
  synthetic definition of its own.
- The delivered story body has a `## Removed tests` section naming each removed test function
  with its file and the reason, or stating that none was removed.
