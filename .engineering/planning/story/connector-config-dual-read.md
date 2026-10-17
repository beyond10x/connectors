---
format: aep.planning-md/3
id: story:connector-config-dual-read
kind: story
status: implemented
title: The host reads connector instances beside adapter entries with equal revisions and selections
relations:
- decomposes: epic:connector-vocabulary
- informed_by: migration-plan:connector-vocabulary
- depends_on: story:ess-connector-vocabulary-model
- serves: vision:independent-contract-adapters
scope:
- confidence: inferred
  path: crates/connectors-host/src/local/config.rs
revision: 11
transitions:
- {from: "draft", to: "proposed", at: "2026-10-16T12:54:48Z", actor: "human:timo", revision: 9, decided_on: {"recorded":{"review_outcome":1}}}
- {from: "proposed", to: "active", at: "2026-10-16T12:54:49Z", actor: "human:timo", revision: 10, decided_on: {"recorded":{"review_outcome":1}}}
- {from: "active", to: "implemented", at: "2026-10-17T01:36:40Z", actor: "human:timo", revision: 11, decided_on: {"recorded":{"test_result":1,"review_outcome":1}}}
---
## Context

Step 1 of `migration-plan:connector-vocabulary` (expand). The host reads only `[adapters.<alias>]`
(`crates/connectors-host/src/local/config.rs:268-306`, `ConfigInput` with `deny_unknown_fields`).
Every persisted record that must survive the vocabulary change is keyed by values this entry
produces, so the new form has to produce them byte for byte before anything is derived or
migrated.

## Change

- A new configuration format version admits `[connectors.<connector>]` (`enabled`) and
  `[connectors.<connector>.instances.<instance>]` (`description`, `endpoint`), beside the legacy
  `[adapters.<alias>]` table in the same file. The connector and instance names follow the design's
  sketch (`docs/design/connector-vocabulary.md` § What changes); `provider` is not a configuration
  key (that design's open question 3 is taken as "no"; adding it later is an optional key).
- An instance entry may carry the legacy pins explicitly: `instance_id`, `configuration_revision`,
  `executable`, `private_protocol`, `startup`, `restart`, `permissions` and the legacy `alias`. With
  all of them present it loads to the same `Adapter` value as the legacy entry. The
  `[adapters.<alias>.permissions.custody]` table the OpenRouter custody unit adds on main is carried
  as `permissions.custody` of the instance with the same meaning; rebase on that unit before
  implementing.
- Validation is the legacy entry's (`config.rs:510-545`: selectors, reserved names, limits) applied
  to both tables. Its existing instance-id uniqueness check (`!instances.insert(&entry.instance_id)`)
  runs over both tables together, so an instance and an alias with one `instance_id` are refused
  by the rule that already refuses two aliases with one.
- The connector name of an instance carrying all pins is not checked against connector
  declarations: non-catalog implementations have no declaration yet
  (story:ess-connector-vocabulary-model leaves their mapping UNMAPPED), and their pinned instances
  still need a name. An instance without pins must name a declared connector
  (story:connector-instance-derived-fields).
- `default_instance` (the counterpart of `default_adapter`) is not here: it is a selector and lands
  with story:cli-connector-instance-selectors.

## Invariant

For an instance carrying the pins of a legacy entry: equal `instance_id`, equal
`configuration_revision`, equal `Adapter::selection()` digest (`config.rs:305`, which is the
approval policy's `executable_selection`, `owner/approval_issuance.rs:83`), and the same alias for
mounted-secret custody item names. The migration plan's table lists the first two only; the
selection digest and the custody item name are added here.

## Domain relations

- Connection -> instance (ServiceConfiguration), many-to-one, references: `ess/domains/auth_bindings.yaml`,
  entity `connectors.auth_bindings.Connection`, relation `instance` (`via: instance_id`).
- ServiceConfiguration -> AdapterSpecification, many-to-one, references:
  `ess/domains/declarations.yaml`, entity `connectors.declarations.ServiceConfiguration`, relation
  `adapter` (`via: adapter_id`).
- Approval policy -> instance, keyed by `instance_id`, `configuration_revision` and
  `executable_selection`: inferred from `crates/connectors-host/src/local/owner/approval_issuance.rs:80-83`;
  `ess/domains/local_approval_policy.yaml` declares the fields, not a relation.
- Mounted-secret custody item -> alias, named `<alias>/<profile>`: inferred from
  `crates/connectors-custody/src/mounted.rs:164-185`.
- Instance -> Connector: from story:ess-connector-vocabulary-model (the instance,
  `connectors.declarations.ServiceConfiguration`, referencing `connectors.declarations.Connector`);
  cited from that model once it validates.

## Scope

- `crates/connectors-host/src/local/config.rs` (`ConfigInput`, `Config`, `validate`, new instance table)
- the test module of `config.rs`
- `ess/domains/cli.yaml` `connectors.cli.ConfigurationRule` only if a new refusal needs a name

## Acceptance

A config test loads, for each adapter kind in the repository (catalog, credential binary, MCP, SQL,
Kubernetes), one file holding a legacy `[adapters.<alias>]` entry and one holding the same pins as
a `[connectors.<c>.instances.<i>]` entry, and gets the same instance id, configuration revision and
executable selection digest from both.

## Notes

- Not here: deriving pins from the installed connector (story:connector-instance-derived-fields),
  rewriting files (story:setup-migrate-adapter-entries), CLI selectors and `default_instance`
  (story:cli-connector-instance-selectors).
- `config.rs` is edited after this story by, in order, story:cli-connector-instance-selectors,
  story:connector-instance-derived-fields and story:setup-migrate-adapter-entries (`depends_on` edges).
- Instance-connection cardinality is unchanged; see `decision-blocker:instance-connection-cardinality`.
