---
format: aep.planning-md/3
id: story:instances-resolve-through-locked-index
kind: story
status: draft
title: Instances resolve catalog connectors through the locked index
relations:
- decomposes: epic:catalog-connectors-only-in-definitions
- serves: vision:independent-contract-adapters
- depends_on: story:definitions-install
revision: 1
---
## Outcome

An instance of a catalog connector, configured by connector name and instance name, runs on the
declaration, selection and bundle lowered from the release the lock names, not on the data
compiled into the catalog provider. Installing another tag changes what the same instance runs
on, with no edit to the configuration file. This is the last sentence of ADR D5: "Instance
configuration names a provider and the locked index instead of an absolute path."

## Waits on

`decision-blocker:catalog-first-run-source`: nobody has decided where a shipped binary gets its
catalog connectors before any definitions install. That answer decides what the compiled-in
data becomes, so this story is not started before the blocker clears. The outcome above, after
an install, does not depend on the answer.

## Context

- Today the catalog provider compiles every declaration, selection and bundle of the tree into its
  binary (`adapters/catalog/build.rs:1-5,27-76`), and an instance configured by connector name is
  derived only from that (`adapters/catalog/src/local.rs:264-268,290-310,317-334,348-413`).
  `connectors setup migrate` compiles the 12 declarations in as well
  (`apps/connectors/src/local/migrate.rs:19-36`).
- A hand-written native configuration naming an absolute `bundle_directory`
  (`adapters/catalog/src/local.rs:176`) is not changed here: the bundle format stays readable
  until consumers have moved (design section 12).
- The catalog provider runs as a confined child of the host: pinned executable, empty
  environment, one channel (`crates/connectors-host/src/local/runtime/process.rs:47-99`, design
  section 9). Whatever tells it where the locked release is travels in what the host passes it,
  not in the environment (inferred).
- What a lock change does to an existing connection is decided by ADR D12 point 2: bundle,
  source or selection only, the connection follows; authority or profile authentication,
  `lifecycle_conflict`; CA, token CA, allowed hosts or request prefix, `stale`. The binding is
  `Binding` in `crates/connectors-host/src/local/registry.rs:195`.

## Domain relations

- Instance to connector definition, through the lock: an instance resolves exactly one definition,
  the one the locked release holds under its connector name. Decided by ADR D5; no `ess/1`
  document declares it (`connectors.declarations.Connector` in `ess/domains/declarations.yaml`
  names no source for a declaration); this story declares it there (placement inferred).
- Locked release change to connection: ADR D12 point 2, held in code by `Binding`
  (`crates/connectors-host/src/local/registry.rs:195`, inferred).

## Scope

- `adapters/catalog/src/local.rs`, `adapters/catalog/build.rs` (cited)
- `apps/connectors/src/local/migrate.rs` (cited)
- `crates/connectors-host/src/local/config.rs` (inferred: instance resolution)
- `ess/domains/declarations.yaml`, `contracts/cli/v1alpha1/semantics.md` (cited)
- `adapters/catalog/tests/connector_declarations.rs`, `adapters/catalog/tests/guide_instances.rs`,
  `adapters/catalog/tests/local_runtime/derived_instance.rs`,
  `apps/connectors/tests/derived_instance.rs`, `apps/connectors/tests/setup_migrate.rs` (cited:
  they hold the compiled-in path today)
- `docs/local-catalog-provider.md`, `docs/catalog-*.md`, `website/docs/connectors/` (inferred:
  user docs that name tree paths)

## Not in this story

- What a binary serves before any install: `decision-blocker:catalog-first-run-source`.
- Deleting `adapters/catalog/providers` and `adapters/catalog/generated/bundles` (the rest of
  epic step 4).
- Where the D11 byte-equality checks live once the tree has no `connector.json` (epic open
  question).

## Acceptance

With a release installed and locked in a test, an instance configured by connector name and
instance name runs on the declaration, selection and bundle lowered from that release, and after
an install of another tag whose selection differs, the same configuration file runs on the new
selection.
