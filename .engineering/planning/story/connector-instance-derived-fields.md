---
format: aep.planning-md/3
id: story:connector-instance-derived-fields
kind: story
status: implemented
title: A Gmail instance configured by name alone connects once and runs reads and guarded writes
relations:
- decomposes: epic:connector-vocabulary
- depends_on: story:catalog-connector-declarations
- depends_on: story:connector-config-dual-read
- informed_by: story:upgrade-moves-catalog-adapters
- depends_on: story:ess-connector-vocabulary-model
- depends_on: story:cli-connector-instance-selectors
- serves: vision:independent-contract-adapters
scope:
- confidence: inferred
  path: adapters/catalog/tests/local_runtime
- confidence: inferred
  path: adapters/catalog/tests/local_runtime.rs
- confidence: inferred
  path: apps/connectors/tests
- confidence: inferred
  path: crates/connectors-host/src/local/config.rs
- confidence: inferred
  path: crates/connectors-host/src/local/runtime.rs
revision: 10
transitions:
- {from: "draft", to: "proposed", at: "2026-10-16T14:07:01Z", actor: "human:timo", revision: 8, decided_on: {"recorded":{"review_outcome":1}}}
- {from: "proposed", to: "active", at: "2026-10-16T14:07:01Z", actor: "human:timo", revision: 9, decided_on: {"recorded":{"review_outcome":1}}}
- {from: "active", to: "implemented", at: "2026-10-17T01:36:41Z", actor: "human:timo", revision: 10, decided_on: {"recorded":{"test_result":1,"review_outcome":1}}}
---
## Context

Steps 2 and 3 of `migration-plan:connector-vocabulary`. Configuring one Gmail write instance today
takes a 20-line native JSON file and a 15-line TOML entry with a hand-copied
`configuration_revision` and executable SHA-256 (`architecture-decision-record:connector-vocabulary`
§ Context). With story:catalog-connector-declarations the provider binary knows what a connector
ships, and with story:connector-config-dual-read the host reads instance entries; this story lets
an instance entry name nothing but itself.

## Change

- For a catalog connector instance without pins, the host derives `adapter_id`, the executable
  (the installed provider binary, path and SHA-256 computed by the host), `private_protocol`
  (`connectors-private/2`, needed for writes), `startup`/`restart` defaults, `permissions.profiles`
  (the connector's profiles), every operation of the connector (ADR decision 5) and the
  `configuration_revision` (from the provider's bootstrap, not from the user).
- `endpoint` absent uses the connector's default endpoint; the auth profile is chosen at connect
  time when the connector declares more than one.
- Stable ids: a new instance gets the id `<connector>.<instance>`. `/` is not admitted by the
  selector check (`crates/connectors-core/src/lib.rs:360-366` admits ASCII alphanumerics and
  `-_.` only), which answers the migration plan's UNMAPPED on step 2. To keep the id unambiguous,
  connector and instance names admit `[A-Za-z0-9_-]` only. An instance carrying `instance_id`
  (a migrated one) keeps it.
- Non-catalog implementations (credential binaries, MCP, SQL, Kubernetes) are not derived here;
  their instances keep explicit pins until a later story declares them as connectors. Slack, a
  catalog provider with two selections, is not derived either (story:catalog-connector-declarations
  ships no Slack declaration, for the reason given there).

## Domain relations

- Connection -> instance (ServiceConfiguration), many-to-one: `ess/domains/auth_bindings.yaml`,
  entity `connectors.auth_bindings.Connection`, relation `instance`.
- Connection -> AuthProfile, many-to-one: same entity, relation `auth_profile`.
- ServiceConfiguration -> AdapterSpecification, many-to-one: `ess/domains/declarations.yaml`,
  entity `connectors.declarations.ServiceConfiguration`, relation `adapter`.
- Instance -> Connector and Connector -> implementation: from story:ess-connector-vocabulary-model
  (`connectors.declarations.Connector`); cited from that model once it validates.

## Scope

- `crates/connectors-host/src/local/config.rs` (derivation of the `Adapter` value; after
  story:cli-connector-instance-selectors, before story:setup-migrate-adapter-entries)
- `crates/connectors-host/src/local/runtime.rs` or the bootstrap call that yields the revision
- `adapters/catalog/tests/local_runtime.rs` (registers the new journey module)
- `adapters/catalog/tests/local_runtime/` (Gmail fixture journey: connect, read, two guarded writes)
- `apps/connectors/tests/` (a configuration with no pins)

## Acceptance

With a configuration whose only Gmail lines are `[connectors.gmail]` with `enabled = true` and an
empty `[connectors.gmail.instances.private]`, one `connections connect --connector gmail --instance
private` against the HTTPS fixture yields one connection under which a Gmail read,
`users.drafts.create` and `users.drafts.send` (each write with its approval proof) succeed.

## Notes

- Live Google verification waits on `credential-blocker:google-oauth-client`.
- Related: story:upgrade-moves-catalog-adapters (provider binary after an upgrade). A derived
  executable moves with the installed binary, so derived instances get that story's outcome for
  free; pinned ones do not.
