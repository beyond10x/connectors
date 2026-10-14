---
format: aep.planning-md/3
id: story:upgrade-moves-catalog-adapters
kind: story
status: draft
title: After a CLI upgrade, catalog adapters move to the release's provider binary and allowlist
relations:
- decomposes: epic:fluxplane-plugin-parity
- serves: vision:independent-contract-adapters
- informed_by: story:owner-stop-after-upgrade
- depends_on: story:connection-lease-holds-unattended
revision: 2
---
## Outcome

After a CLI upgrade, the configured catalog adapters run the provider binary and the operation
allowlist of the new release, or the CLI says by name which adapters still run the older one and the
one command that moves them.

## Today

Reported by a downstream consumer on connectors 0.50.0, 2026-10-14: an upgraded CLI left the
configured catalog adapters on the older provider binary and its allowlist. 11 operations (5 of
them used every round) stayed unavailable after the upgrade. The adapter entry pins the provider
executable by path and sha256, so a new binary installed elsewhere, or at the same path with a new
digest, is not picked up.

Related: story:owner-stop-after-upgrade (the owner the older binary started) and
story:connection-follows-configuration-upgrade (connections across a configuration change). This
story covers the adapter entry and its provider binary; it does not repeat either.

## Change

Spec first in `ess/domains/cli.yaml`: `setup check` reports each adapter entry whose provider
executable's version is older than the CLI's, and a command (or a documented `setup` step) moves an
entry to the provider binary of the running release, rewriting path and digest, after which the
selection's allowlist is the new release's. Connections follow under
story:connection-follows-configuration-upgrade's rules.

## Acceptance

- A CLI test: an entry pinned to an older provider build is reported by `setup check` with the move
  command; after the move, `operations list --adapter <alias>` lists the new release's selection.
- The upgrade guide names the steps, run in the tree and pasted from that run.

## Scope

Primary: ess/domains/cli.yaml (SetupCheckResult, Prerequisite, NextAction), apps/connectors/src/local.rs setup-check, apps/connectors/spec/cli.yaml, connectors-host local/config.rs (first rewriter of existing config), owner.rs, adapters/catalog main.rs/local.rs (provider version), contracts/cli semantics.md, generated CLI contract, docs. Collides with owner-stop (wave c, A) and any cli.yaml unit; sequence after wave c merges. Open: provider version mechanism, permissions.operations rewrite, configuration_revision handoff to connection-follows-configuration-upgrade.
