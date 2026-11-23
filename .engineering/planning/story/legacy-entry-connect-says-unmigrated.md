---
format: aep.planning-md/3
id: story:legacy-entry-connect-says-unmigrated
kind: story
status: draft
title: connections connect on a legacy adapters entry answers legacy_entry_unmigrated
relations:
- decomposes: epic:review-20261123-findings
- serves: vision:independent-contract-adapters
revision: 1
---
## Outcome

`connections connect <connector>/<instance>` on a legacy `[adapters.<alias>]` entry that no
connector is derived for answers `legacy_entry_unmigrated`, as selecting it elsewhere does, and
names what to write instead.

## Observed

2026-11-23, connectors 0.74.1, while setting up a Jira read-only instance for the dev-operator
consumer. A configuration with one legacy entry `[adapters.jira-read]` whose native file names
provider `jira` and an `api_base` on the Atlassian gateway with `request_prefix`:

- `instances list` answered `{"adapters":[],"unmigrated":["jira-read"]}`.
- `setup migrate --dry-run` answered `disposition: unchanged` with no reason.
- `connections connect jira/jira-read --profile atlassian.basic --credential-stdin` answered
  `invalid_input` with `field: endpoint` at `arguments`, although no `--endpoint` was passed.
- `connections connect --adapter jira-read ...` passed the arguments stage.
- Writing the same entry by hand as `[connectors.jira.instances.jira-read]` with every pin made
  `jira/jira-read` resolve, connect and answer reads.

docs/local-catalog-provider.md documents `invalid_input` for `endpoint` only for a connect that
passes `--endpoint` on a legacy entry.

## Acceptance

- The connect above answers `legacy_entry_unmigrated`, not `invalid_input` for `endpoint`; a test
  covers it.
- `setup migrate --dry-run` names, per entry it leaves unchanged, why no connector is derived
  for it.
