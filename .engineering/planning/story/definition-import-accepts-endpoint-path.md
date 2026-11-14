---
format: aep.planning-md/3
id: story:definition-import-accepts-endpoint-path
kind: story
status: draft
title: The definition importer accepts a declaration's endpoint_path
summary: import.rs refuses endpoint_path that 0.73.0 lowering writes
relations:
- serves: vision:independent-contract-adapters
- blocks: story:catalog-basecamp-reads
revision: 1
---
# The definition importer accepts a declaration's endpoint_path

## Need

0.73.0 lowers a definition whose `base_url` path ends in a config-declared variable and records the
path in the declaration as `endpoint_path` (`story:catalog-path-prefix-from-config`). The definition
importer (`crates/connectors-catalog/src/import.rs`) reads a provider's `connector.json` with
unknown fields denied and does not know the field:

```
connector.json: unknown field `endpoint_path`, expected one of `format`, `connector_id`,
`provider_id`, `adapter_id`, `catalog_provider`, `default_endpoint`, `api_path`, `selection`,
`default_profile`, `profiles`
```

No shipped definition uses such a base URL yet, so the gate on main is green; the first one that
does (Basecamp, `story:catalog-basecamp-reads`) fails every import.

## Acceptance

- A case that imports a provider directory whose `connector.json` carries `endpoint_path` fails
  first, then passes.
- The imported declaration keeps `endpoint_path` byte for byte; every other shipped provider
  imports to the same bytes as before.
- Once `link_next` (PR 264) merges, the same holds for a paging style's `link_next`.
