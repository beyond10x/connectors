---
format: aep.planning-md/3
id: story:definition-format-write-members
kind: story
status: implemented
title: The definition format carries the write members of the 0.54.0 engine
relations:
- serves: vision:independent-contract-adapters
- informed_by: architecture-decision-record:provider-definitions-and-runtimes
scope:
- confidence: cited
  path: crates/connectors-catalog/src/definition.rs
- confidence: cited
  path: crates/connectors-catalog/src/import.rs
- confidence: inferred
  path: crates/connectors-catalog/tests/definition_writes.rs
- confidence: cited
  path: crates/connectors-catalog/tests/fixtures/definitions/connectors/zendesk.yaml
revision: 6
transitions:
- {from: "draft", to: "proposed", at: "2026-10-18T22:42:33Z", actor: "human:timo", revision: 4}
- {from: "proposed", to: "active", at: "2026-10-18T22:42:33Z", actor: "human:timo", revision: 5}
- {from: "active", to: "implemented", at: "2026-10-19T05:54:43Z", actor: "human:timo", revision: 6, decided_on: {"recorded":{"test_result":2,"review_outcome":1}}}
---
## Outcome

The connector-definition format (`urn:beyond10x:connectors-definition:1`) expresses every member
the 0.54.0 catalog engine admits on a write, so the shipped selections round-trip through the
lowerer and importer again: `guard.before.mismatch` (`conflict`), `conflict_status`,
`body_enums`, `body_any_of`, `body_prefixes` and the body type `string_list`. The three Zendesk
ticket writes of 0.54.0 are expressed in `connectors/zendesk.yaml` of the definitions repository
and in its byte-identical fixture here.

## Why

On 2026-10-18 the release gate failed at `crates/connectors-catalog/tests/definition_import.rs:131`
(`unknown field 'mismatch'`): the wave's engine and the definition lowerer were built on
different branches, and the format (ADR D11: the definition owns the declaration) had not been
extended for the new members.

## Acceptance

- `definition_import::zendesk_round_trips` and `zendesk_is_committed` pass; every other shipped
  selection still round-trips.
- A `conflict_status` other than 409 or 412, and a `body_any_of` with one key, are refused by the
  lowerer by name.
- `task check` in the definitions repository validates the extended schema and the Zendesk file.

## Scope

Primary: `crates/connectors-catalog/src/definition.rs`, `crates/connectors-catalog/src/import.rs`,
`crates/connectors-catalog/tests/`, the definitions repository's `spec/domains/definition.yaml`,
`generated/schema/types/*`, `connectors/zendesk.yaml`.
