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
- depends_on: story:definition-tests-read-pinned-release
revision: 4
---
## Outcome

- The catalog provider and `connectors` embed, at build time, the connectors-definitions release
  that the committed pin of `story:definitions-pin-covers-catalog` names. `build.rs` reads that
  release from the cache the pre-build fetch fills, checks it against the pinned index SHA-256,
  and never reads `adapters/catalog/providers` or `adapters/catalog/generated/bundles`.
- Every one of the 15 definitions of that release is served by its connector name, `slack`,
  `slack-user` and `openrouter` included. Slack stays two connectors, as the definitions are.
- With no install, an instance configured by connector name and instance name resolves from the
  embedded release. After a D5 install of another tag, the same instance resolves from the locked
  index. That install stays explicit; nothing replaces the embedded release by itself.
- The Slack and OpenRouter guides configure instances with `--connector`. An instance still
  configured by path moves through `connectors setup migrate`.

This realises option C of `decision-blocker:catalog-first-run-source` (decided by the operator),
option A of `decision-blocker:slack-openrouter-pinned-source`, and the last sentence of ADR D5:
"Instance configuration names a provider and the locked index instead of an absolute path."

## Context

- First run, verbatim from its blocker: "The build embeds the pinned connectors-definitions
  release: the release named by the pin of story:definitions-pin-covers-catalog is fetched and
  digest-checked before the build, and the binary carries it. The repository holds no copy of a
  definition. A D5 install of another tag stays explicit."
- Slack and OpenRouter, verbatim from their blocker: "Serve `slack`, `slack-user` and
  `openrouter` by name from the embedded release: Slack stays two connectors, as the definitions
  are. The Slack and OpenRouter guides move to `--connector`; an instance configured by path moves
  through `setup migrate`, with a test that a path-configured instance migrates."
- Today the catalog provider compiles the declarations, selections and bundles of the tree into
  its binary (`adapters/catalog/build.rs:1-5,27-76`), and an instance configured by connector name
  is derived only from them (`adapters/catalog/src/local.rs:264-268,290-310,317-334,348-413`).
  `connectors setup migrate` compiles the 12 declarations in separately
  (`apps/connectors/src/local/migrate.rs:19-36`), and `apps/connectors/tests/setup_migrate.rs:144-146`
  holds that list to the provider directories of the tree.
- Slack and OpenRouter are held out of the served set today, and each hold-out changes here:
  `adapters/catalog/tests/connector_declarations.rs:186-193` (every provider but slack and
  openrouter ships one declaration), `:594-608` (`--connector slack` refused),
  `adapters/catalog/tests/guide_instances.rs:25` (`UNDECLARED`), and the hand-written index rows
  of `crates/connectors-docs/src/onboarding.rs:23,356-371`.
- `setup migrate` maps a path-configured entry to the declaration whose `catalog_provider` equals
  the native configuration's `provider` (`apps/connectors/src/local/migrate.rs:236-239`).
  `slack` and `slack-user` share the catalog `slack` (`slack-user.yaml` carries `catalog: slack`),
  so the match also compares the selection the entry names with the declaration's `selection`
  (`adapters/catalog/src/local.rs:256-258`) (inferred).
- OpenRouter: the guide configures two instances of one profile, a management key
  (`docs/catalog-openrouter.md:66-77`) and an inference key (`:195-206`), with different `label`
  values (`:74`, `:203`). The definition carries the first label only (design section 14), and
  `label` is part of the native configuration (`adapters/catalog/src/local.rs:137`). If it is
  digested into the configuration revision, `setup migrate` keeps an inference-key entry with
  `selection_changes` naming `configuration_revision` (`migrate.rs:43-53,184`) instead of moving
  it. The delivered story records what
  its test shows (inferred risk).
- The guides and their site pages are authored: `website/docs/connectors/slack.md` and
  `openrouter.md` name `docs/catalog-slack.md` and `docs/catalog-openrouter.md` as their
  `source` and describe "two pinned instances ... because no ... declaration ships yet" (lines
  4-6). The JSON configurations the guides drop are kept as fixtures under
  `adapters/catalog/tests/hand_written/`, as was done for the other 12
  (`adapters/catalog/tests/hand_written/mod.rs:1-7`) (inferred).
- `build.rs` never fetches: the gate builds with `--offline`
  (`crates/connectors-build/src/gate.rs:149-150`). When the cache holds no entry for the pinned
  index SHA-256, the build fails naming the fetch command. The fetch runs before cargo and is not
  a `connectors-build` subcommand, because `connectors-build` depends on
  `connectors-catalog-provider` (`crates/connectors-build/Cargo.toml:11`) (inferred).
- The release archive is built by `.github/workflows/release-binaries.yml:119-125`
  (`cargo build --locked --release`); the fetch step goes before it.
- A hand-written native configuration naming an absolute `bundle_directory`
  (`adapters/catalog/src/local.rs:176`) is still read (design section 12).
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
- `depends_on story:definition-tests-read-pinned-release`: both stories change
  `crates/connectors-docs/src/onboarding.rs`.

## Domain relations

- Binary to embedded release: one build embeds exactly one release, the one the committed pin
  names. Decided by option C (`decision-blocker:catalog-first-run-source`); no `ess/1` document
  declares it.
- Instance to connector definition: the locked release when a lock exists, otherwise the embedded
  release, under the connector name; one definition per connector, 15 connectors. Decided by ADR
  D5 and the two blockers; no `ess/1` document declares it (`connectors.declarations.Connector` in
  `ess/domains/declarations.yaml` names no source); this story declares it there (placement
  inferred).
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
- `crates/connectors-docs/src/onboarding.rs` (cited: the Slack and OpenRouter hand rows)
- guides: `docs/catalog-slack.md`, `docs/catalog-openrouter.md`,
  `website/docs/connectors/slack.md`, `website/docs/connectors/openrouter.md` (cited)
- tests changed: `adapters/catalog/tests/local_runtime/derived_instance.rs`,
  `adapters/catalog/tests/connector_declarations.rs`, `adapters/catalog/tests/guide_instances.rs`,
  `apps/connectors/tests/derived_instance.rs`, `apps/connectors/tests/setup_migrate.rs` (cited:
  the path-configured Slack and OpenRouter migration cases go here);
  `adapters/catalog/tests/hand_written/catalog-slack.*.json` and `catalog-openrouter.*.json`
  (inferred); a new install-then-resolve test under `apps/connectors/tests/` (inferred)

## Not in this story

- Deleting `adapters/catalog/providers` and `adapters/catalog/generated/bundles`, and the reads
  of them by path that remain in `connector_declarations.rs`, `guide_instances.rs` and
  `bundle_drift.rs`: `story:catalog-provider-data-removed`.
- One Slack connector with two auth profiles (option B of the Slack and OpenRouter blocker, still
  possible later).
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
- Served set: `--connector` accepts the connector of each of the 15 definitions of the pinned
  release, `slack`, `slack-user` and `openrouter` included.
- Path-configured migration: in `apps/connectors/tests/setup_migrate.rs`, path-configured entries
  for the Slack bot selection, the Slack user selection and the OpenRouter management key migrate
  to `slack`, `slack-user` and `openrouter` respectively.
- Guides: `docs/catalog-slack.md`, `docs/catalog-openrouter.md` and their two site pages configure
  instances with `--connector` and show no `bundle_directory` or `operations_file`.
- `connectors setup migrate` maps entries onto the declarations of the embedded release, and
  `apps/connectors/tests/setup_migrate.rs` holds its list to that release, not to a directory of
  the tree.
- `release-binaries.yml` fetches and digest-checks the pinned release before its build step.
