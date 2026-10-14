---
format: aep.planning-md/3
id: story:revision-change-keeps-a-recovery-path
kind: story
status: implemented
title: A bootstrap revision change leaves each connection one working recovery path
relations:
- serves: vision:independent-contract-adapters
revision: 5
transitions:
- {from: "draft", to: "proposed", at: "2026-10-14T09:25:50Z", actor: "human:timo", revision: 3}
- {from: "proposed", to: "active", at: "2026-10-14T09:25:50Z", actor: "human:timo", revision: 4}
- {from: "active", to: "implemented", at: "2026-10-14T22:06:08Z", actor: "human:timo", revision: 5, decided_on: {"recorded":{"test_result":1}}}
---
## Outcome

A configuration change that moves an adapter's bootstrap revision leaves every existing connection
of that adapter with one working recovery path, and no next action the CLI answers on the way loops
back to another refusal.

## Today

Observed 2026-10-14 on a local install, catalog provider, Slack bot connection in state `ready`:
raising `auth.evidence_lifetime_ms` from the default 60000 to 300000 in the native configuration,
copying the new bootstrap revision into `config.toml` and restarting the owner left the connection
with no recovery:

- `connections revalidate` answers `lifecycle_conflict`, `next_action: create_connection`;
- `connections connect` answers `lifecycle_conflict`, `next_action: retry_status`;
- `connections repair` with the same token answers `identity_mismatch`.

Reverting the lifetime and restarting the owner made `revalidate` answer `ready` again. Which part
of the revision (the evidence lifetime inside the adapter's description) makes the connection
non-upgradable is not yet traced in code.

## Change

Spec first in `ess/domains/cli.yaml` and the auth contracts: classify each part of an adapter
revision a connection depends on. A change that does not touch the connection's identity (instance,
adapter, provider authority, profile) is upgraded by `connections revalidate`; a change that does
is refused with one next action that works (`connections connect` creating a new connection, with
the old one revocable). The three refusals above get next actions that lead to that path.

## Acceptance

- A CLI test: a ready connection, the adapter's evidence lifetime changed, the owner restarted,
  `connections revalidate` answers `ready` (or the single documented path succeeds), within two
  commands.
- A test walks every next action the CLI answers after a revision change and shows none returns to
  an earlier refusal.
- docs/local-catalog-provider.md states that `invoke` does not revalidate, that evidence lasts
  60 s by default, and that a caller revalidates right before invoke.

## Also observed

On 2026-10-14, a guide run against a scratch state directory: after an adapter entry is edited
(the revision moved, or only `private_protocol` changed), the adapter process still running the
old entry makes `connections revalidate` answer `lifecycle_conflict` with `next_action:
retry_status`, which no retry satisfies until `adapters stop` stops that process. The next action
names `adapters stop` (with what to pass), or revalidation stops the stale adapter process itself.
