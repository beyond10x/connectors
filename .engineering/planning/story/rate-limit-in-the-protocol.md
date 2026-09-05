---
format: aep.planning-md/1
id: story:rate-limit-in-the-protocol
kind: story
status: proposed
title: A provider's rate limit is a protocol fact, not a sentence
tags:
- ready
- wave-cli
scope:
- confidence: inferred
  path: catalog/slack.catalog.json
- confidence: inferred
  path: connectors.lock
- confidence: inferred
  path: crates/catalog-build
- confidence: inferred
  path: crates/connector-spec
- confidence: inferred
  path: crates/connectors-runtime
- confidence: inferred
  path: crates/integration-catalog
- confidence: inferred
  path: crates/integration-slack
- confidence: inferred
  path: crates/protocol
- confidence: inferred
  path: providers/slack.toml
revision: 5
---
# Story: a provider's rate limit is a protocol fact, not a sentence

## Defect
`OperationErrorCode` has no rate-limit variant and the response envelope has no `retry_after`;
a 429 from Slack surfaces as `unavailable` with the text "the provider refused the request with
HTTP 429 — the provider is rate-limiting this credential". A caller that must pace itself (the
org-brain ledger walked 530 pages in one run and hit 51 such refusals) has to match the wording.

## Shape
- `OperationErrorCode::RateLimited` with `retry_after_seconds: Option<u64>` taken from the
  provider's `Retry-After` header when present.
- Catalogue operations may declare their provider's rate tier so `describe` exposes a suggested
  interval; the personal runtime may pace requests per connection to that interval.

## Acceptance
- A 429 with `Retry-After: 30` is returned as `rate_limited` with `retry_after_seconds: 30`.
- `slack-conversations-history` `describe` names the interval Slack's tier implies.

## Readiness

Selected for implementation on 2026-09-06 at the operator's request. wave-cli: 9 of 10. The ready tag records selection; proposed is the pre-implementation lifecycle state, and existing active work stays active. Existing dependencies and implementation evidence requirements still apply.
