---
format: aep.planning-md/3
id: story:setup-check-reports-weakened-guarantees
kind: story
status: draft
title: setup check names the platform guarantees this build holds only in a weaker form
relations:
- decomposes: epic:local-cli-on-macos-and-windows
- depends_on: story:host-runs-on-macos
- serves: vision:independent-contract-adapters
revision: 1
---
## Outcome

`connectors setup check` names every platform guarantee the running build holds only in a weaker form, so a person on macOS sees what differs from Linux without reading the contract.

## Today

Read 2026-10-13 on wave/20261013b. `connectors.cli.PlatformGuaranteeStatement` (`ess/domains/cli.yaml`) states each of the seven guarantees per operating system, and `contracts/cli/v1alpha1/semantics.md` § "macOS platform guarantees" lists six weaker forms on macOS. `SetupCheckResult` carries none of them: the check's JSON answer is the same shape on both platforms.

## Change

- Specification: a `weakened_guarantees` field on `SetupCheckResult`, a list of a new `connectors.cli.PlatformGuaranteeGap` struct (`guarantee`, `support`, `weaker_form`). A separate type from the statement, because `ess specify cli` refuses invariants on a CLI type. An unapplied draft of this model change exists (a patch against `ess/domains/cli.yaml` at 0789e64bf).
- CLI: `setup check` emits the field: empty on Linux, six entries on macOS.
- A JSON fixture for each platform, and the regenerated CLI reference.

## Acceptance

- `ess validate` passes with the newest `ess`, and the generated CLI types carry the field.
- On Linux, `setup check --json` reports `weakened_guarantees: []`.
- On the macOS runner, the same command reports the six weaker forms of the semantics table, each matching its row.
