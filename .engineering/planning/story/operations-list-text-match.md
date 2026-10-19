---
format: aep.planning-md/3
id: story:operations-list-text-match
kind: story
status: draft
title: operations list --match filters admitted operations by text
relations:
- decomposes: epic:fluxplane-plugin-parity
- serves: vision:independent-contract-adapters
revision: 1
---
## Outcome

`connectors operations list --match <text>` lists the admitted operations of the selected
instances whose key or description contains the text, case-insensitive, across all configured
connectors when no `--connector` is given. The fluxplane verb `operation search` carried 121
calls in the baseline (`docs/fluxplane-plugin-parity.md`, "CLI verbs"); `operations list` filters
by `--connector`, `--instance`, `--family`, `--limit` and `--cursor` only, so an agent that
knows a capability but not its key lists every connector and reads.

## Surface

One optional argument on the existing passive read in `apps/connectors/spec/cli.yaml`, applied to
the cached descriptions the host already serves to `operations list`; no provider is started for
it (an instance with no cached description is listed as `description_missing`, as today).

## Acceptance

- Spec first: the argument is modelled in `ess/domains/cli.yaml`, validated with the pinned `ess`,
  regenerated.
- With three configured fixture instances, `operations list --match "merge request"` answers the
  matching keys of each instance with the instance named, and nothing else; `--match` with
  `--connector` narrows to that connector.
- The command remains passive (no owner start, no checkpoint).
- The CLI reference shows the command run in the tree.

## Scope

Primary: `apps/connectors/spec/cli.yaml`, `apps/connectors/src/local/operations.rs` (or the
module that serves `list`), `ess/domains/cli.yaml`, `website/docs/reference/cli.md` (generated).
Collides with story:operations-usage-report and story:refusals-name-the-field on `cli.yaml`.
