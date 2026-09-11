---
format: aep.planning-md/1
id: story:catalog-bundle-pipeline
kind: story
status: implemented
title: Take a pinned document to an indexed bundle in one recorded run
relations:
- derived_from: specification:catalog-http-runtime-handoff
- decomposes: initiative:complete-local-connectors
- serves: vision:independent-contract-adapters
scope:
- confidence: cited
  path: crates/connectors-catalog/src/bundle.rs
- confidence: inferred
  path: crates/connectors-catalog/src/lib.rs
- confidence: cited
  path: crates/connectors-catalog/src/pipeline.rs
- confidence: cited
  path: crates/connectors-catalog/tests/pipeline.rs
revision: 8
---
## Outcome

One call takes a pinned OpenAPI document from a file to an indexed bundle, and
returns a single record of what it did: what was ingested, what was inventoried,
what was named unsupported, and where the bundle landed. A step that refuses
stops the run and the record says which step and why, so a half-built bundle is
never indexed.

## Scope

`crates/connectors-catalog/src/pipeline.rs` and its tests. It composes the four
modules that already exist — ingest, inventory, coverage, bundle — and adds no
behaviour of its own beyond sequencing and the record.

The record carries the source record, the coverage report, the index entry the
write produced, and the ordered steps with their outcome. A refusal is returned
as the refusing step's own error, unwrapped and unrewritten, with the step named
beside it.

Ordering is fixed and stated: read, ingest, extract, report, write. Nothing is
written to the bundle directory until every earlier step has succeeded, so a
refusal leaves the directory exactly as it was — including the index.

## Acceptance

A run over a valid document writes the bundle, indexes it, and returns a record
whose coverage counts equal the inventory's own and whose index entry matches
what `bundle::read_index` reports; a document the ingest refuses leaves the
directory untouched with no index file created and names the ingest step in its
error; a provider already indexed refuses at the write step unless replacement
is asked for, and the earlier steps' results are still in the record; and two
runs over the same bytes into two directories produce equal records apart from
the paths.

## Verification

`cargo test --locked -p connectors-catalog`, with the exit code in the report.
