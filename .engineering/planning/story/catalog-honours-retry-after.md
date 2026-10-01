---
format: aep.planning-md/3
id: story:catalog-honours-retry-after
kind: story
status: draft
title: A catalog read answered 429 honours Retry-After
relations:
- decomposes: epic:tech-debt-review-20260930
- serves: vision:independent-contract-adapters
scope:
- confidence: inferred
  path: adapters/catalog/src/lib.rs
- confidence: cited
  path: docs/catalog-confluence.md
- confidence: cited
  path: docs/catalog-hubspot.md
- confidence: cited
  path: docs/catalog-jira.md
- confidence: cited
  path: docs/local-catalog-provider.md
revision: 3
---
## Defect

The catalog provider "does not retry on `429`; a rate-limited read is returned as a refusal"
(`docs/catalog-confluence.md`, `docs/catalog-hubspot.md` Limits; `docs/local-catalog-provider.md`
Limits). A caller walking pages of Jira, Confluence or HubSpot fails the walk at the first 429 and
must implement back-off itself, without the provider's `Retry-After`.

## Change

A read answered 429 with a `Retry-After` the engine can parse is retried once after that delay when
the delay fits inside the invocation deadline; otherwise the refusal carries the delay so the caller
can wait. Writes are never retried.

## Scope

- `adapters/catalog/src/lib.rs` — inferred: the engine classifies responses there.
- `docs/local-catalog-provider.md` and the provider guides' Limits — cited.

## Acceptance

- A fixture answering 429 with `Retry-After: 1` then 200 yields one successful read and two requests.
- A `Retry-After` beyond the deadline, or absent, yields a refusal that states the delay or its absence,
  after one request.
- A write answered 429 is sent once and refused.

## Ordering

Lands after wave `20260930c` (Google), which changes `adapters/catalog/src/lib.rs`.
