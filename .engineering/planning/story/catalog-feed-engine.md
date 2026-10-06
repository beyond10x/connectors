---
format: aep.planning-md/3
id: story:catalog-feed-engine
kind: story
status: implemented
title: The catalog engine realizes a feed binding from declarations
relations:
- decomposes: epic:generic-datasource-feeds
- serves: vision:independent-contract-adapters
- depends_on: story:feed-contract
scope:
- confidence: inferred
  path: adapters/catalog/src
- confidence: inferred
  path: contracts/catalog/v1alpha1/semantics.md
revision: 5
transitions:
- {from: "draft", to: "proposed", at: "2026-10-06T17:00:04Z", actor: "agent:claude", revision: 3}
- {from: "proposed", to: "active", at: "2026-10-06T17:00:05Z", actor: "agent:claude", revision: 4}
- {from: "active", to: "implemented", at: "2026-10-06T18:33:41Z", actor: "agent:claude", revision: 5, decided_on: {"recorded":{"test_result":1}}}
---
## Outcome

The catalog engine realizes a `datasource.feed/v1alpha1` binding from a provider's declarations (data, not per-provider code): `feed.containers` from a list endpoint and `feed.items` from a time- or cursor-filtered list endpoint, with the watermark formed from the provider's cursor or time filter as the binding declares it.

## Why

Plan review 2026-10-06: the four binding stories land under `adapters/catalog/`, and the catalog contract states the engine serves unary `generic-http` and `generic-http-page` only (`contracts/catalog/v1alpha1/semantics.md`). Nothing owned the engine work, as `story:read-post-capability` did for websearch.

## Acceptance

A fixture provider declared only as data answers `feed.containers` and `feed.items` through the catalog engine, passes the family's conformance suite, and resumes from a returned watermark; no Rust code names that provider.

## Depends on

`story:feed-contract`.
