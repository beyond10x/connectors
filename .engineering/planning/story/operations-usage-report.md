---
format: aep.planning-md/3
id: story:operations-usage-report
kind: story
status: draft
title: operations usage reports invoked and refused operations from the execution audit
relations:
- decomposes: epic:fluxplane-plugin-parity
- serves: vision:independent-contract-adapters
revision: 1
---
## Outcome

`connectors operations usage --since <date>` reports, from the local execution audit, per
connector and instance: the operations invoked and how often, the refusals by code, and the
operation keys refused as `not_found` (what callers asked for that no selection serves). It is the
measurement the parity epic's exit criteria E4 reads, and it replaces the hand count of
`docs/fluxplane-plugin-parity.md` for the next recount: demand from a team's sessions is read
from the audit, not from transcripts.

## Surface

The audit binding (`docs/local-execution-audit.md`; `crates/connectors-host/src/local/audit.rs`)
already records per invocation the instance, the operation, the bounded outcome and the safe
refusal code; the public query service is "unimplemented" there. This story adds one passive CLI
read in `apps/connectors/spec/cli.yaml` (`operations usage`) and a view in the host over the
audit store, returning counts only: no input, no provider output, no credential material, no
approval proof, which the anchor does not hold anyway. A `not_found` refusal of an unknown
operation key is recorded at admission today only if the host anchors early refusals for it; if
it does not, the story records that key in the anchor's early-refusal fields first.

## Acceptance

- Spec first: the view and its row shape are modelled in `ess/domains/execution_audit.yaml` and
  `ess/domains/cli.yaml`, validated with the pinned `ess`, regenerated.
- After three invocations (two served, one refused `not_found` for `issue.frobnicate`) on a
  fixture instance, `operations usage --since <yesterday> --output json` answers one row per
  operation with its count and outcome code, and `issue.frobnicate` under `not_found`.
- The command is passive: it writes no checkpoint and starts no owner (the passivity test shape
  the metadata store uses).
- The site's CLI reference and `docs/local-execution-audit.md` name the read, run in the tree.

## Scope

Primary: `apps/connectors/spec/cli.yaml`, `apps/connectors/src/local/` (new `usage.rs`),
`crates/connectors-host/src/local/audit.rs`, `ess/domains/execution_audit.yaml`,
`ess/domains/cli.yaml`, `docs/local-execution-audit.md`. Collides with story:refusals-name-the-field
on `cli.yaml`: run in different waves or the second merges the first.
