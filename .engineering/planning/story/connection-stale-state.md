---
format: aep.planning-md/3
id: story:connection-stale-state
kind: story
status: implemented
title: A connection whose credential the provider refused is listed Stale until a reconnect returns it to Live
relations:
- decomposes: epic:connector-vocabulary
- depends_on: story:connections-reconnect-verb
- informed_by: story:guarded-write-credential-refusal-says-repair
- depends_on: story:ess-connector-vocabulary-model
- serves: vision:independent-contract-adapters
scope:
- confidence: inferred
  path: adapters/catalog/tests/local_runtime/oauth2_refresh.rs
- confidence: inferred
  path: apps/connectors/src/local/connections.rs
- confidence: inferred
  path: crates/connectors-host/src/local/metadata/er.rs
- confidence: inferred
  path: crates/connectors-host/src/local/owner.rs
- confidence: inferred
  path: crates/connectors-host/src/local/registry/lifecycle.rs
- confidence: inferred
  path: crates/connectors-host/src/local/registry/observation.rs
revision: 10
transitions:
- {from: "draft", to: "proposed", at: "2026-10-16T15:38:10Z", actor: "human:timo", revision: 8}
- {from: "proposed", to: "active", at: "2026-10-16T15:38:11Z", actor: "human:timo", revision: 9}
- {from: "active", to: "implemented", at: "2026-10-17T01:36:39Z", actor: "human:timo", revision: 10, decided_on: {"recorded":{"test_result":1}}}
---
## Context

ADR decision 7 and `migration-plan:connector-vocabulary` step 7. A credential the provider refuses
is visible today only as an invoke failure with `next_action: repair_connection`
(`docs/catalog-google-oauth.md` § Repair). The registry records an invalid material
(`invalid_reason` on `registry_materials`) and projects it as `reauthorization_required`
(`crates/connectors-host/src/local/registry/observation.rs:255-290`), but the connection's lifecycle
stays `Live` (`ess/domains/auth_bindings.yaml:145-149` before the spec change).

## Input

The `Stale` lifecycle of `connectors.auth_bindings.Connection` as story:ess-connector-vocabulary-model
declares it, with `entity-runtime-definitions.json` already regenerated there. The intended
transitions (design § Connection lifecycle): `Live -> Stale` on a provider refusal of the stored
credential, `Stale -> Live` and `Live -> Live` on publication, `Live, Stale -> Revoked` on revoke.
This story implements what that model declares; where this text and the model differ, the model
wins.

## Change

- The owner records the refusal: a refresh-token refusal (`invalid_grant`) or a static-token
  refusal classified `InvalidCredential` moves the connection to `Stale` in the metadata authority,
  through the transition story:ess-connector-vocabulary-model declares.
- Refreshing an access token is not a transition (`adapters/catalog/src/local.rs:366-368`, `:515`).
- `connections list` and `connections describe` show `Stale`, through the projection that story's
  model declares. Two existing names are close and must stay distinguishable: the admission state
  `reauthorization_required` (`connectors.connection_admission.ConnectionState`) and
  `ConnectionDescription.stale`, a cached metadata flag (`ess/domains/cli.yaml:701-709`).
- `connections reconnect` (story:connections-reconnect-verb) publishes and returns it to `Live`;
  `connections revoke` from `Stale` ends in `Revoked`.
- No data migration: existing connections stay `Live` until their first refusal.

## Domain relations

- Connection -> instance and Connection -> AuthProfile are unchanged:
  `ess/domains/auth_bindings.yaml`, entity `connectors.auth_bindings.Connection`, relations
  `instance` and `auth_profile`. Only the lifecycle changes, and that comes from the spec input.

## Scope

- `crates/connectors-host/src/local/registry/{lifecycle,observation}.rs`
- `crates/connectors-host/src/local/metadata/er.rs`
- `crates/connectors-host/src/local/owner.rs` (where a provider refusal reaches the host)
- `apps/connectors/src/local/connections.rs`
- `adapters/catalog/tests/local_runtime/oauth2_refresh.rs` (fixture journey)

## Acceptance

In the catalog OAuth refresh fixture, after the token endpoint refuses the stored refresh token
with `invalid_grant` during an invoke, `connections list` reports that connection as `Stale`, and
after `connections reconnect` publishes a new credential it reports it as `Live` again.

## Notes

- Related: story:guarded-write-credential-refusal-says-repair (a write refused for its credential
  reports the remedy); a write refusal should move the connection to `Stale` the same way a read
  does once both land.
- Not here: whether the validation-evidence `pending` condition stays user-visible (design open
  question 1).

## Coordinator additions

## Coordinator additions

From the implementor report of story:ess-connector-vocabulary-model (commit c18d26dbab), 2026-10-16:

- The status projection has no `Stale` variant: a Stale connection reports
  `reauthorization_required` with next action `reconnect`. This story adds the variant to the
  connection status enum in `ess/domains/connection_admission.yaml` (now in scope) and regenerates,
  so `connections list` shows `Stale` as the acceptance says.
- A revalidation refused under a newer configuration revision records only
  `refused_configuration_revision` and does not make the connection Stale: the credential still
  works under its own revision (answer to the open point on Decision 17).
- `MarkConnectionStale` is accepted by the local metadata authority (`ess/components.yaml`); the
  host call site is this story.
- The host store model digest changed with the new definitions (`crates/connectors-host/src/local/er.rs:787-830`);
  this story runs the host store tests.
