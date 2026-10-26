---
format: aep.planning-md/3
id: story:tool-contracts-for-catalog-connectors
kind: story
status: draft
title: Catalog connector operations publish tool contracts
summary: Tool profiles beside each catalog provider declaration, published without refusal
relations:
- decomposes: epic:tool-contract-publication
- serves: vision:independent-contract-adapters
- depends_on: story:connector-projection
revision: 1
---
## Outcome

Every catalog connector operation a selection admits has a tool profile authored beside its provider
declaration (`adapters/catalog/providers/<provider>/`), and the catalog engine's instance descriptor
publishes through `connectors_tool_contracts::publish` without a refusal.

## Design

- A `tool-profiles.json` per provider, keyed by operation id; risk follows the operation's method and
  the provider's effect (a read is `network`, `secret`, `external`; a write adds its consequences).
- The descriptor is the catalog engine's per-instance one; selections and withheld parameters are
  already applied there.
- Schemas the converter refuses today (catalog operations with `number` or untyped JSON) are listed
  per provider and either typed in the declaration or refused by name; none is defaulted.

## Scope

- `adapters/catalog/providers/*/tool-profiles.json` (new)
- `adapters/catalog/src/` (exposing the per-instance descriptor to the publisher)
- `crates/connectors-tool-contracts/tests/` (one test per provider)

## Acceptance

- `every_catalog_operation_publishes`: for each provider's default selection, `publish` answers a
  contract that `tool_contract::validate` accepts, or the test names the refused operation and path.
- `catalog_contract_has_no_secret` for each provider.

## Not in this story

- Unrepresentable JSON value types: `b10x-tool-contract` 0.5.0 has no `Json` or `Number` type (tools
  `story:profile-declares-ports`).
