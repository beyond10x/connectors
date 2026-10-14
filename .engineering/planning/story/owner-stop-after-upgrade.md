---
format: aep.planning-md/3
id: story:owner-stop-after-upgrade
kind: story
status: draft
title: Stop an older owner after a CLI upgrade with one documented command
relations:
- serves: vision:independent-contract-adapters
revision: 1
---
## Outcome

After a CLI upgrade, an operator stops the owner process an older binary started with one
documented command, and the refusal that asks for it names that command.

## Today

A command run by a newer binary against an owner an older one started answers
`owner_build_mismatch` with `next_action: stop_owner` (`apps/connectors/src/local.rs:183`), and
`checkpoints` answers `lifecycle_conflict` with `stop_owner` (`apps/connectors/src/local/checkpoints.rs:30`).
Neither the CLI help nor docs/catalog-slack.md nor the CLI guide says how to stop an owner; an
operator found this after upgrading from 0.45 to 0.50.

## Change

Spec first in `ess/domains/cli.yaml`: if the CLI has no stop verb, add one (for example
`connectors setup owner-stop`) that asks the running owner of this state directory to drain and
exit, waits a bounded time, and refuses by name when the owner does not answer or belongs to
another user; it never kills a process it cannot prove is this state directory's owner. Then the
CLI guide gets an "After an upgrade" section, and the `stop_owner` next action's text names the
command.

## Acceptance

- A CLI test: an owner started by one build, the stop command from another build, the owner exits
  and the next command starts a fresh owner and succeeds.
- The stop command on a state directory with no owner answers a typed "no owner" outcome, exit 0.
- `connectors --help` and the CLI guide name the command; `cli --check` holds.
