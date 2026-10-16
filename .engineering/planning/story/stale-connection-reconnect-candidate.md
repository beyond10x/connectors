---
format: aep.planning-md/3
id: story:stale-connection-reconnect-candidate
kind: story
status: draft
title: A reconnect candidate on a Stale connection is a modelled Stale-to-Stale change
relations:
- decomposes: epic:connector-vocabulary
- depends_on: story:connection-stale-state
revision: 1
---
## Context

From the implementor of story:connection-stale-state (commit 032045190): a reconnect adds a
candidate custody version, which changes `superseded_custody_version_refs` on a Stale connection,
and the model has no Stale to Stale change for it; persisting it failed with `MetadataUnavailable`
(`update_command Connection "Stale" -> "Stale"`). The host now lists only completed acquisitions
for a Stale connection (`crates/connectors-host/src/local/metadata/er.rs`), which leaves one edge:
publishing a new connection under a new revision while a Stale one holds the instance.
Two texts are also out of date: `contracts/auth/connection/v1alpha1/semantics.md` §4.1 does not
name the `stale` status, and `ess/domains/auth_bindings.yaml` still says "so no status variant is
added".

## Change

- `ess/domains/auth_bindings.yaml`: the command that records a reconnect candidate is admitted
  from Stale and leaves it Stale; regenerate the entity definitions; remove the workaround in
  `er.rs`.
- Amend §4.1 of the connection contract and the stale comment in `auth_bindings.yaml`.

## Acceptance

A host test records a reconnect candidate on a Stale connection, persists it, and the connection
stays Stale with the candidate listed among its custody versions; the `er.rs` completed-only
filter is gone.
