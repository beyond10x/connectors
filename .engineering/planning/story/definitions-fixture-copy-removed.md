---
format: aep.planning-md/3
id: story:definitions-fixture-copy-removed
kind: story
status: draft
title: The fixture copy of the definitions tree is deleted from connectors-catalog tests
relations:
- decomposes: epic:catalog-connectors-only-in-definitions
- serves: vision:independent-contract-adapters
- depends_on: story:definition-tests-read-pinned-release
- depends_on: story:catalog-tests-read-pinned-release-gitlab-engine
- depends_on: story:catalog-tests-read-pinned-release-providers
- depends_on: story:definition-reader-feed-extensions
revision: 2
---
## Outcome

`crates/connectors-catalog/tests/fixtures/definitions/` (15 connector files, 3 auth files, 15
upstream READMEs) and `crates/connectors-catalog/tests/fixtures/definition/providers/jira.yaml`
are deleted. The fixture part of epic step 4.

## Context

- Tracked files naming either path on 9201b1177, outside `.engineering/` (`git grep`):
  `adapters/catalog/tests/definition_auth.rs`, `adapters/catalog/tests/jira.rs`,
  `crates/connectors-build/src/main.rs`, `crates/connectors-catalog/tests/definition_import.rs`,
  `crates/connectors-catalog/tests/definition.rs`. The three stories this one depends on move
  each of them to the pinned release or a synthetic definition; `definition_import.rs` keeps only
  its reads of the in-tree bundles, which this story does not touch.
- The other two deletions of step 4, `adapters/catalog/providers/` and
  `adapters/catalog/generated/bundles/`, are not drafted. The catalog provider compiles both into
  its binary (`adapters/catalog/build.rs:1-5,27-76`) and `connectors setup migrate` compiles the
  declarations in (`apps/connectors/src/local/migrate.rs:19-36`). Replacing that is
  `story:instances-resolve-through-locked-index`, which waits on
  `decision-blocker:catalog-first-run-source`.

## Scope

- `crates/connectors-catalog/tests/fixtures/definitions/` (cited)
- `crates/connectors-catalog/tests/fixtures/definition/providers/jira.yaml` (cited)

## Acceptance

Both fixture paths are gone, `git grep` finds neither outside `.engineering/`, and the gate
passes.
