---
format: aep.planning-md/1
id: story:catalog-operation-inventory
kind: story
status: implemented
title: Extract a deterministic operation inventory with named gaps
relations:
- derived_from: specification:catalog-http-runtime-handoff
- decomposes: initiative:complete-local-connectors
- serves: vision:independent-contract-adapters
revision: 4
---
## Outcome

An ingested OpenAPI source yields a deterministic operation inventory: every
operation the document declares, in a stable order, with what a later template
needs to call it. What the inventory cannot represent is named rather than
dropped, so a coverage count is truthful about its own gaps.

## Scope

`crates/connectors-catalog/` only, on top of the source ingest. For each path
item and each HTTP method it declares, record the method, the path template, the
`operationId` when present, each parameter with its location and whether it is
required, the request body media types, and the response status codes with their
media types. Order is the document's path order, then a fixed method order, so
two runs over the same bytes produce the same inventory.

Named unsupported cases, counted separately and never silently skipped: an
operation whose path item or body uses a `$ref` this pass does not resolve, a
parameter whose location is outside path, query, header and cookie, and
a document member this build does not read — `webhooks` and `callbacks`.

## Acceptance

The same source bytes produce a byte-identical inventory twice; an operation
with no `operationId` is still inventoried and identified by method and path; a
`$ref` body and an unknown parameter location each appear in the unsupported
list with their operation named; and the coverage count reports inventoried and
unsupported separately, never one total that hides the difference.

## Verification

`cargo test --locked -p connectors-catalog`, with the exit code in the report.
