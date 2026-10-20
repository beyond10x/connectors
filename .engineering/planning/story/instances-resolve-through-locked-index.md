---
format: aep.planning-md/3
id: story:instances-resolve-through-locked-index
kind: story
status: draft
title: Instances resolve catalog connectors from the pinned release the binary embeds, or from the locked index after an install
relations:
- decomposes: epic:catalog-connectors-only-in-definitions
- serves: vision:independent-contract-adapters
- depends_on: story:definitions-install
- depends_on: story:definitions-pin-covers-catalog
- depends_on: story:catalog-tests-read-pinned-release-gitlab-engine
- depends_on: story:catalog-tests-read-pinned-release-providers
revision: 3
---
## Outcome

- The catalog provider and `connectors` embed, at build time, the connectors-definitions release
  that the committed pin of `story:definitions-pin-covers-catalog` names. `build.rs` reads that
  release from the cache the pre-build fetch fills, checks it against the pinned index SHA-256,
  and never reads `adapters/catalog/providers` or `adapters/catalog/generated/bundles`.
- With no install, an instance configured by connector name and instance name resolves from the
  embedded release.
- After a D5 install of another tag, the same instance resolves from the locked index. That
  install stays explicit; nothing replaces the embedded release by itself.

This realises option C of `decision-blocker:catalog-first-run-source` (cleared 2026-10-20, decided
by the operator) and the last sentence of ADR D5: "Instance configuration names a provider and the
locked index instead of an absolute path."

## Context

- The decision, verbatim from the blocker: "The build embeds the pinned connectors-definitions
  release: the release named by the pin of story:definitions-pin-covers-catalog is fetched and
  digest-checked before the build, and the binary carries it. The repository holds no copy of a
  definition. A D5 install of another tag stays explicit."
- Today the catalog provider compiles the declarations, selections and bundles of the tree into
  its binary (`adapters/catalog/build.rs:1-5,27-76`), and an instance configured by connector name
  is derived only from them (`adapters/catalog/src/local.rs:264-268,290-310,317-334,348-413`).
  `connectors setup migrate` compiles the 12 declarations in separately
  (`apps/connectors/src/local/migrate.rs:19-36`), and `apps/connectors/tests/setup_migrate.rs:144-146`
  holds that list to the provider directories of the tree.
- `build.rs` never fetches: the gate builds with `--offline`
  (`crates/connectors-build/src/gate.rs:149-150`). When the cache holds no entry for the pinned
  index SHA-256, the build fails naming the fetch command. The fetch runs before cargo and is not
  a `connectors-build` subcommand, because `connectors-build` depends on
  `connectors-catalog-provider` (`crates/connectors-build/Cargo.toml:11`), whose build now needs
  the release (inferred).
- The set of connectors served by name stays the 12 of today. Slack and OpenRouter ship no
  declaration and stay pinned (`adapters/catalog/tests/connector_declarations.rs:186-190`;
  `story:catalog-connector-declarations`, "Whether Slack is one connector or two is left to a
  later story"), although the lowerer writes a declaration for every definition (design section
  14).
- The release archive is built by `.github/workflows/release-binaries.yml:119-125`
  (`cargo build --locked --release`); the fetch step goes before it.
- A hand-written native configuration naming an absolute `bundle_directory`
  (`adapters/catalog/src/local.rs:176`) is not changed here (design section 12).
- The catalog provider runs as a confined child of the host: pinned executable, empty
  environment, one channel (`crates/connectors-host/src/local/runtime/process.rs:47-99`). The
  location of a locked release travels in what the host passes it, not in the environment
  (inferred).
- What a change of release does to an existing connection is decided by ADR D12 point 2: bundle,
  source or selection only, the connection follows; authority or profile authentication,
  `lifecycle_conflict`; CA, token CA, allowed hosts or request prefix, `stale`
  (`Binding`, `crates/connectors-host/src/local/registry.rs:195`).

## Ordering

- `depends_on story:definitions-install`: the locked index this story resolves from.
- `depends_on story:definitions-pin-covers-catalog`: the pin names the release the build embeds,
  and both stories change `adapters/catalog/Cargo.toml`, `apps/connectors/Cargo.toml` and
  `Cargo.lock` (that story adds dev-dependencies, this one build-dependencies).
- `depends_on story:catalog-tests-read-pinned-release-gitlab-engine` and
  `story:catalog-tests-read-pinned-release-providers`: this story changes `build.rs` and the
  compiled-in path that both batches use through the shared `connectors-catalog-provider` test
  binaries. `adapters/catalog/tests/local_runtime/derived_instance.rs`, changed here, is a module of
  the providers batch file `local_runtime.rs` (one test binary). Both batches read the current
  `build.rs` in their rename-after-build acceptance.

## Domain relations

- Binary to embedded release: one build embeds exactly one release, the one the committed pin
  names. Decided by option C (`decision-blocker:catalog-first-run-source`, Decided by the
  operator); no `ess/1` document declares it.
- Instance to connector definition: the locked release when a lock exists, otherwise the embedded
  release, under the connector name. Decided by ADR D5 and option C; no `ess/1` document declares
  it (`connectors.declarations.Connector` in `ess/domains/declarations.yaml` names no source); this
  story declares it there (placement inferred).
- Release change to connection: ADR D12 point 2, held in code by `Binding`
  (`crates/connectors-host/src/local/registry.rs:195`, inferred).

## Scope

- `adapters/catalog/build.rs`, `adapters/catalog/src/local.rs` (cited)
- `apps/connectors/src/local/migrate.rs` (cited) and how `connectors` embeds the same release
  (inferred: its own build script or a shared embedding)
- manifests: `adapters/catalog/Cargo.toml` gains `[build-dependencies]` for checking and lowering
  the release at build time (inferred: `connectors-catalog` for the reference lowerer and the
  crate holding the fetch-and-verify core of `story:definitions-install`);
  `apps/connectors/Cargo.toml` the same if it embeds on its own; `Cargo.lock` (inferred)
- `crates/connectors-host/src/local/config.rs` (inferred: resolution through the lock)
- `ess/domains/declarations.yaml`, `contracts/cli/v1alpha1/semantics.md` (cited)
- `.github/workflows/release-binaries.yml` (cited: the fetch step before the release build)
- tests changed: `adapters/catalog/tests/local_runtime/derived_instance.rs`,
  `apps/connectors/tests/derived_instance.rs`, `apps/connectors/tests/setup_migrate.rs` (cited);
  a new install-then-resolve test under `apps/connectors/tests/` (inferred)

## Not in this story

- Deleting `adapters/catalog/providers` and `adapters/catalog/generated/bundles`, and the tests
  that read them by path (`connector_declarations.rs`, `guide_instances.rs`, `bundle_drift.rs`):
  `story:catalog-provider-data-removed`.
- Serving Slack or OpenRouter by connector name.
- Where the D11 byte-equality checks live once the tree has no `connector.json` (epic open
  question).

## Acceptance

- Embedded: with no install and with `adapters/catalog/providers` and
  `adapters/catalog/generated/bundles` renamed away before the build, an instance configured by
  connector name and instance name runs on the declaration, selection and bundle lowered from the
  pinned release.
- Locked: after `connectors definitions install` of another tag in a test, the same configuration
  file runs on the declaration, selection and bundle lowered from the locked release.
- Missing release: a build whose cache holds no entry for the pinned index SHA-256 fails before
  compiling the catalog provider, naming the fetch command, and opens no network connection.
- Same set: the connectors that `--connector` accepts are the 12 that ship a declaration on
  9201b1177; `slack`, `slack-user` and `openrouter` are not among them.
- `connectors setup migrate` maps entries onto the declarations of the embedded release, and
  `apps/connectors/tests/setup_migrate.rs` holds its list to that release, not to a directory of
  the tree.
- `release-binaries.yml` fetches and digest-checks the pinned release before its build step.
