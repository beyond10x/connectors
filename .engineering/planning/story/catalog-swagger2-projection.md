---
format: aep.planning-md/3
id: story:catalog-swagger2-projection
kind: story
status: draft
title: The catalog ingests Swagger 2.0 through an exact recorded projection
relations:
- decomposes: epic:generic-datasource-feeds
- serves: vision:independent-contract-adapters
scope:
- confidence: inferred
  path: crates/connectors-catalog/src
- confidence: inferred
  path: crates/connectors-catalog/tests
revision: 2
---
## Outcome

The catalog ingests a Swagger 2.0 document through an exact, recorded projection to OpenAPI 3, as it ingests Google Discovery documents today, so a provider published only as Swagger 2.0 (Slack's Web API) gets a bundle without a hand-written source.

## Why

Wave 20261006d (2026-10-06): `connectors-build catalog --provider slack` refused the pinned Slack document with `ingest step refused: InvalidInput: source declares no openapi version` (`crates/connectors-catalog/src/lib.rs:36-38`, pinned by `crates/connectors-catalog/tests/ingest.rs:40`). `adapters/catalog/design.md:31,135` plans "Swagger 2.0 by exact projection as a recorded transform"; only the Google Discovery projection exists (`crates/connectors-catalog/src/discovery.rs`, `pipeline::run_derived`). The decision-blocker draft and the fetched document are kept by the coordinating session.

## Work

- A Swagger 2.0 → OpenAPI 3 projection in `crates/connectors-catalog`, run through `pipeline::run_derived` with a projection record, and refused for any construct it cannot project exactly (named in the refusal).
- `bundle_drift.rs` checks the projection's freshness as it does for Google.

## Acceptance

- The pinned Slack document (`slackapi/slack-api-specs` `bc08db49625630e3585bf2f1322128ea04f2a7f3`, `web-api/slack_web_openapi_v2_without_examples.json` or the redacted full file) projects and ingests; `conversations.list`, `conversations.history` and `conversations.replies` are present with their parameters.
- A Swagger construct with no exact projection is refused by name.
