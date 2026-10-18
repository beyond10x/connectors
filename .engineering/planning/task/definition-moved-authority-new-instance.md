---
format: aep.planning-md/3
id: task:definition-moved-authority-new-instance
kind: task
status: implemented
title: A definition moved to another host resolves to a new instance id and connects fresh
summary: invoke --definition gives a definition whose authority or profile authentication no longer matches its instance's connections a new deterministic instance id, so connect works again; the old connection stays revocable
relations:
- implements: architecture-decision-record:provider-definitions-and-runtimes
- depends_on: story:invoke-definition-reuse-credential
- serves: vision:independent-contract-adapters
revision: 4
transitions:
- {from: "draft", to: "proposed", at: "2026-10-18T16:07:04Z", actor: "human:timo", revision: 2, decided_on: {"recorded":{"test_result":1}}}
- {from: "proposed", to: "active", at: "2026-10-18T16:07:04Z", actor: "human:timo", revision: 3, decided_on: {"recorded":{"test_result":1}}}
- {from: "active", to: "implemented", at: "2026-10-18T16:58:46Z", actor: "human:timo", revision: 4, decided_on: {"recorded":{"test_result":2,"review_outcome":1}}}
---
## Defect

`operations invoke --definition <file>` (`story:invoke-definition-file`) gives every definition of a
connector the one instance id `<connector>.definition`. When the file's provider authority (its
host) changes, the registry refuses a connect under that id as `lifecycle_conflict` with
`next_action = check_configuration`: "only a new instance id helps"
(`docs/local-connection-registry.md`, "Configuration upgrades"). The definition's instance id is
fixed, so an operator who edits the host in the YAML can never connect again under that file.
`story:invoke-definition-reuse-credential` recorded the dead end in its Delivery status and pinned it
in the journey `a_definition_moved_to_another_host_is_offered_no_reuse_and_answers_lifecycle_conflict`.
Cross-host reuse stays refused (operator decision A, 2026-10-18; D12 point 1); this task does not
change it.

## Fix

When a connection of `<connector>.definition`, revoked ones included, was admitted under another
provider authority or another authentication of the definition's profile (the registry's own
predicate, `revalidation::identity_changed`, which a connect under that id refuses on), the
definition resolves to the instance `definition-<16 hex>` instead: the digest of the connector id,
the provider authority and the profile's authentication (its declaration with the revision and the
evidence lifetime left out, so a lifetime-only change, which the registry admits under the same id,
keeps the id). The id is a pure function of the definition, so repeated runs reuse it, and a file
moved back to the original host resolves to `definition` again. An unchanged definition keeps
`<connector>.definition`, its cache key and its connection. The connector keeps one entry, so the
old instance id is no longer configured and its connections stay revocable through the moved-entry
path (`connections revoke` under any instance of the catalog implementation).

## Acceptance

The journey is restated as
`a_definition_moved_to_another_host_connects_fresh_under_a_new_instance_id`: with the definition's
own connection at host H (made by reuse from a configured instance at H) and the file then naming
H2, the next invoke offers no reuse and lists no candidate, asks the one connect question naming
`forge/definition-<16 hex>`, and on `y` and the token returns the operation's result from H2. A
second run asks nothing and selects the same instance. The connection left at H keeps its revision
and is revoked through the new instance with that revision; the configured instance's connection is
unchanged. It fails on the S3 head (`d123da5ed`) and passes after the fix.

## Not in scope

Cross-host reuse or moving a credential; forgetting the entry or the old connection; `instances
list` reporting definition instances.
