---
format: aep.planning-md/3
id: story:definition-tests-read-pinned-release
kind: story
status: draft
title: Definition tests, build-tool checks and the docs generator read the pinned release; lowerer tests use a synthetic definition
relations:
- decomposes: epic:catalog-connectors-only-in-definitions
- serves: vision:independent-contract-adapters
- depends_on: story:definitions-pin-covers-catalog
revision: 2
---
## Outcome

Every reader of connector data listed under Scope, under `crates/` and `apps/connectors/tests/`,
reads it from the pinned, installed release through the test-support function of
`story:definitions-pin-covers-catalog`, or, where a test only exercises the lowerer, from a small
synthetic definition of its own. Epic step 3, outside `adapters/catalog/tests/`. One file is
exempt from leaving the tree by name: `crates/connectors-catalog/tests/definition_import.rs`
keeps comparing the lowered pinned release with the in-tree bundles, selections and declarations
until those are deleted.

## Context

- Epic, In scope: "The engine and lowerer tests keep a small synthetic definition of their own
  instead of a provider copy." Precedent: the `widgets` definition and source that
  `crates/connectors-catalog/tests/definition_layout.rs:54-59` builds in a temporary directory.
- Readers of the fixture copy `crates/connectors-catalog/tests/fixtures/definitions/`:
  `crates/connectors-catalog/tests/definition_import.rs:22-23`, and
  `adapters/catalog/tests/definition_auth.rs:28` (moved by
  `story:catalog-tests-read-pinned-release-gitlab-engine`).
- Readers of the three-operation Jira copy
  `crates/connectors-catalog/tests/fixtures/definition/providers/jira.yaml`:
  `crates/connectors-catalog/tests/definition.rs:20`, `crates/connectors-build/src/main.rs:796,897`,
  and `adapters/catalog/tests/jira.rs:846` (moved by
  `story:catalog-tests-read-pinned-release-providers`).
- Readers of `adapters/catalog/providers/` or `adapters/catalog/generated/bundles/` outside
  `adapters/catalog/`: the `definition_*.rs` tests listed under Scope;
  `crates/connectors-build/src/main.rs:800-863`; `crates/connectors-build/src/docs.rs:259-266`
  (walkthrough fixtures checked against the shipped GitLab selection);
  `crates/connectors-build/src/catalog_feed_conformance.rs:598-600`;
  `crates/connectors-build/src/gitlab_feed_conformance.rs:293`;
  `crates/connectors-build/tests/catalog_binary_model.rs:19`, `catalog_identity_model.rs:49`,
  `catalog_guard_model.rs:20`; `crates/connectors-docs/src/onboarding.rs:43` (the connectors
  index generated from the declarations, run by the gate docs step and `pages.yml:51-52`); and
  `apps/connectors/tests/failed_connect.rs:380`.
- `definition_import.rs` reads the in-tree bundles and selections at lines 122, 162, 175, 216, 332
  and 495. It stops reading the fixture copy here and keeps those reads: it is the byte-equality
  guard for Jira, and the reason the first tag needs connectors-definitions pull request 4
  (`story:definitions-release-index`). Its comparison goes when the in-tree bundles are deleted.
- The manifests and `Cargo.lock` are changed by `story:definitions-pin-covers-catalog`, not here.
- Vendored sources under `adapters/*/upstream/` stay and may still be read; they are not in the
  epic order.

## Domain relations

None beyond those of `story:definitions-pin-covers-catalog`.

## Scope

- `crates/connectors-catalog/tests/definition_import.rs`, `definition.rs`, `definition_auth.rs`,
  `definition_feed.rs`, `definition_paging.rs`, `definition_present_adversary.rs`,
  `definition_writes.rs`, `definition_writes_adversary.rs`, `definition_layout.rs` (cited)
- `crates/connectors-build/src/main.rs`, `docs.rs`, `catalog_feed_conformance.rs`,
  `gitlab_feed_conformance.rs` (cited)
- `crates/connectors-build/tests/catalog_binary_model.rs`, `catalog_identity_model.rs`,
  `catalog_guard_model.rs` (cited)
- `crates/connectors-docs/src/onboarding.rs` (cited)
- `apps/connectors/tests/failed_connect.rs` (cited)

## Not in this story

- The comparison in `definition_import.rs` against the in-tree bundles, selections and
  declarations: it goes with the deletion of `adapters/catalog/providers` and
  `adapters/catalog/generated/bundles`.
- `crates/connectors-catalog/src/import.rs` and `connectors-build definition import`: they read
  this tree as their source and go with its removal.
- `apps/connectors/src/local/migrate.rs` and `apps/connectors/tests/setup_migrate.rs`: they hold
  the declarations compiled into the binary (`story:instances-resolve-through-locked-index`).
- Deleting the fixture copy: `story:definitions-fixture-copy-removed`.

## Acceptance

No file listed under Scope reads a path under `tests/fixtures/definitions` or
`tests/fixtures/definition/providers`; none of them except
`crates/connectors-catalog/tests/definition_import.rs` reads a path under
`adapters/catalog/providers` or `adapters/catalog/generated/bundles`; and the gate passes with
each of them reading the pinned release or a synthetic definition.
