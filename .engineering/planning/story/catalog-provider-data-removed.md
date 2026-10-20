---
format: aep.planning-md/3
id: story:catalog-provider-data-removed
kind: story
status: draft
title: adapters/catalog/providers and adapters/catalog/generated/bundles are deleted; a build gets catalog connectors only from the pinned release it embeds
relations:
- decomposes: epic:catalog-connectors-only-in-definitions
- serves: vision:independent-contract-adapters
- depends_on: story:instances-resolve-through-locked-index
- depends_on: story:definitions-fixture-copy-removed
revision: 1
---
## Outcome

`adapters/catalog/providers/` (14 directories, 27 files) and `adapters/catalog/generated/bundles/`
(14 bundles and `index.json`) are deleted, and every reader still left is changed or removed. The
repository then holds no copy of a connector definition, bundle, selection or declaration; a build
gets its catalog connectors only from the pinned release it embeds. The rest of epic step 4.

## How a build gets the pinned release

- The gate builds and tests with `cargo build --workspace --locked --offline` and
  `cargo test --workspace --locked --offline` (`crates/connectors-build/src/gate.rs:149-150`); CI
  runs `cargo fetch --locked` and then the gate (`.github/workflows/rust-gate.yml:56,62-63`).
- Before cargo runs, the release named by the committed pin is fetched and digest-checked into a
  cache outside the working tree. That is the step `story:definitions-pin-covers-catalog` adds to
  the workflows and to `AGENTS.md`, using the fetch-and-verify core of `story:definitions-install`.
  `build.rs` reads the release from that cache offline
  (`story:instances-resolve-through-locked-index`). The release is never committed.

## Readers left at this point

Every other reader has moved in the stories this one depends on.

- `adapters/catalog/tests/bundle_drift.rs:175` holds the committed bundles to the pipeline. With no
  committed bundle it has nothing to hold, and it is removed.
- `adapters/catalog/tests/connector_declarations.rs` reads `providers/` and the committed
  `connector.json` files (`:88-91,195,217,345-352,450,524-527`) and maps the `hand_written/`
  placeholders onto the tree (`:104-135`). Its comparisons with committed files go. Its revision
  equality between derived instances and the `hand_written/` configurations stays, reading the
  embedded release (ADR D11: "the lowered blocks must keep today's values, absent optional fields
  included, so the `derived_instance` and `connector_declarations` tests stay green").
- `adapters/catalog/tests/guide_instances.rs:199-205` reads the google declarations from
  `providers/`; it reads them from the pinned release through the test support of
  `story:definitions-pin-covers-catalog`.
- `crates/connectors-catalog/src/import.rs` (`shipped_selections`, `:561-575`), its command
  `connectors-build definition import` (`crates/connectors-build/src/main.rs:166-186`) and its test
  `crates/connectors-catalog/tests/definition_import.rs` (in-tree reads at lines 122, 162, 175,
  216, 332, 495). The import writes a definition from a shipped selection; with none shipped it
  has no input, and it is removed with its test. Whether a byte-equality check moves to
  connectors-definitions is the open D11 question on the epic.
- Text that names the paths: `AGENTS.md:87` (the onboarding rule),
  `contracts/catalog/v1alpha1/semantics.md`, 14 pages under `docs/`, 17 pages under
  `website/docs/`, and comments in two files under `adapters/*/spec/` and `adapters/*/contracts/`.
  Each says where the data lives now.

## Open before this lands

Slack and OpenRouter ship no declaration and are configured in the pinned form, with
`bundle_directory` and `operations_file` pointing into a checkout of this tree
(`docs/catalog-slack.md:403,412,470`, `docs/catalog-openrouter.md:68,77,197,206`;
`adapters/catalog/tests/connector_declarations.rs:186-190`). After this deletion those paths do
not exist, and the D5 install cache holds definitions, not lowered bundles. Nobody has decided how
such an instance gets its bundle and selection. The question is on the epic; this story does not
answer it, and it does not land before it is answered.

## Domain relations

None beyond those of `story:instances-resolve-through-locked-index`.

## Scope

- `adapters/catalog/providers/`, `adapters/catalog/generated/bundles/` (cited: deleted)
- `adapters/catalog/tests/bundle_drift.rs`, `connector_declarations.rs`, `guide_instances.rs`
  (cited)
- `crates/connectors-catalog/src/import.rs`, `crates/connectors-catalog/tests/definition_import.rs`,
  `crates/connectors-build/src/main.rs` (cited)
- `AGENTS.md`, `contracts/catalog/v1alpha1/semantics.md`, `docs/`, `website/docs/` (cited)
- `adapters/*/spec/`, `adapters/*/contracts/` (cited: two comments)

## Not in this story

- Vendored upstream sources under `adapters/*/upstream/` (not in the epic order).
- `adapters/catalog/tests/hand_written/`: its JSON keeps the placeholder paths as the record of
  what the guides showed (`adapters/catalog/tests/hand_written/mod.rs:1-7`).
- The build-time fetch and embedding: `story:instances-resolve-through-locked-index`.

## Acceptance

- `adapters/catalog/providers` and `adapters/catalog/generated/bundles` are gone, and `git grep`
  finds neither path outside `.engineering/`, `CHANGELOG.md` and
  `adapters/catalog/tests/hand_written/`.
- In a clean checkout, with the pinned release fetched into the cache before cargo runs, the gate
  passes.
