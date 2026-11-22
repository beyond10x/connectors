---
format: aep.planning-md/3
id: story:catalog-path-prefix-from-config
kind: story
status: implemented
title: A definition's base URL configuration fills a leading path parameter
summary: A declared leading /{v} of every operation is taken from the base URL, not from the caller
relations:
- serves: vision:independent-contract-adapters
- blocks: story:catalog-basecamp
revision: 4
transitions:
- {from: "draft", to: "proposed", at: "2026-11-13T10:50:30Z", actor: "human:operator", revision: 2}
- {from: "proposed", to: "active", at: "2026-11-13T10:50:30Z", actor: "human:operator", revision: 3}
- {from: "active", to: "implemented", at: "2026-11-14T01:30:43Z", actor: "human:operator", revision: 4, decided_on: {"recorded":{"test_result":1}}}
---
# A definition's base URL configuration fills a leading path parameter

## Need

Every Basecamp operation starts with `/{accountId}/`. Nothing in the definition format fixes a path
parameter: `config` fills only `base_url` (`crates/connectors-catalog/src/definition.rs:2359-2383`)
and an amendment must keep every path parameter (`crates/connectors-catalog/src/amendment.rs:16-21`).

## Acceptance

- When `base_url` ends in `/{v}` and `config` declares `v`, the lowerer drops the leading `/{v}`
  segment and its parameter from each selected operation, so the caller never passes it.
- An operation that does not start with that segment is refused at lowering.
- A leading parameter the definition's `config` does not declare is still refused.
- Every existing definition lowers byte-equal; the catalog format reference documents the rule.
