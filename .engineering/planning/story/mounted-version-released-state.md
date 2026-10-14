---
format: aep.planning-md/3
id: story:mounted-version-released-state
kind: story
status: draft
title: Record a released mounted-secret version
relations:
- serves: vision:independent-contract-adapters
- depends_on: story:mounted-secret-custody
revision: 1
---
## Outcome

A mounted-secret version the operator retires reaches a `Released` state the registry records, so
`connections list` and the audit name a retired mounted item as released rather than active, and a
reclaim can end it `Deleted`.

## Today

Mounted custody (story:mounted-secret-custody) never writes the mounted item. Each version's
lifecycle is re-derived from its `registry_materials` row, which has no column for a released
mounted version, so the state cannot be recorded without a metadata migration. Production never
reclaims versions today (`delete_retired` has no caller).

## Acceptance

- The metadata store records a released mounted version through a migration the store-writer
  marker orders; an older binary refuses the migrated store by name.
- Revoking a mounted connection leaves the item in place and records the version `Released`.
