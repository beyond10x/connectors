---
format: aep.planning-md/3
id: decision-blocker:wave-20261017a-scope
kind: decision-blocker
status: cleared
title: 'What wave 20261017a delivers: the connection lease and two test-hygiene fixes'
relations:
- blocks: story:connection-lease-holds-unattended
- blocks: story:build-signal-test-leaves-no-core
- blocks: story:approval-issuance-prepare-test-flake
- blocks: story:registry-passive-approval-test-flake
- serves: vision:independent-contract-adapters
revision: 2
transitions:
- {from: "open", to: "cleared", at: "2026-10-17T05:07:35Z", actor: "human:timo", revision: 2}
---
## Question

What wave 20261017a delivers: the first cross-cutting reliability row of the plugin parity epic
(a revalidated connection keeps serving reads; concurrent revalidations finish within the
admission bound), and two test-hygiene fixes beside it.

## Options

| option | what | cost |
|---|---|---|
| A | The lease unit alone, resumed from its parked work, then the next cross-cutting rows in later waves. | Serial; idle build slots. |
| B | The lease unit plus the hygiene units whose files it does not write: the gate's signalled-command test leaving core files, and the two data_version passivity flakes. | Three trees of package gates. |

Recommended: B.

## Decided

Option B, 2026-10-17: one integration branch `wave/20261017a`, one pull request, then a release.

Width: three units. story:upgrade-moves-catalog-adapters and story:refusals-name-the-field both
write ess/domains/cli.yaml and apps/connectors/src/local.rs, which the lease unit writes, so they
follow it in order. story:oauth-drain-macos-race and story:clock-quorum-windows-unreachable-flake
need a macOS or Windows runner to reproduce. Disk: about 52G free on the host at the start.

## Units

| unit | tree | branch | base | stage |
|---|---|---|---|---|
| story:connection-lease-holds-unattended | conn-u-lease | unit/lease | 38f69d337, main merged | dispatched |
| story:build-signal-test-leaves-no-core | conn-h-core | unit/build-signal-no-core | 0df6fe2b6 | dispatched |
| story:approval-issuance-prepare-test-flake, story:registry-passive-approval-test-flake | conn-h-passive | unit/passive-data-version | 0df6fe2b6 | dispatched |

story:approval-prepare-data-version-flake and story:approval-target-passive-test-flake duplicate
the two passivity stories and are archived.

## Shared files, kept by the coordinator at integration

- CHANGELOG.md
- crates/connectors-docs/src/status.rs and the generated pages, regenerated once on the integrated head
