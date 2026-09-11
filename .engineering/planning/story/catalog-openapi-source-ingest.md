---
format: aep.planning-md/1
id: story:catalog-openapi-source-ingest
kind: story
status: implemented
title: Ingest a pinned OpenAPI source with preserved provenance
relations:
- derived_from: specification:catalog-http-runtime-handoff
- decomposes: initiative:complete-local-connectors
- serves: vision:independent-contract-adapters
revision: 4
---
## Outcome

A pinned OpenAPI source document is ingested with its provenance preserved, so
every later artifact derived from it names the exact bytes it came from. An
unsupported document is refused by name rather than partially accepted.

## Scope

`crates/connectors-catalog/` only. Reads a local OpenAPI 3.0 or 3.1 document and
records: the SHA-256 of the exact source bytes, the declared OpenAPI version, the
document's own `info.version`, the declared licence when present, and the file
name it was read from. Version detection reads the document's `openapi` field; a
2.x document, a missing or unparseable version, a non-object root, or a document
over the bounded size limit is refused with a distinct reason.

No network access, no code generation, no runtime coupling. The recorded ESS
refusal is not closed here and no version string is rewritten.

## Acceptance

Ingesting the same bytes twice yields the same digest and the same record; a 3.0
and a 3.1 document are both accepted and report their own version; a Swagger 2.0
document, an oversized document and a malformed document are each refused with
their own reason, and no refusal produces a partial record.

## Verification

`cargo test --locked -p connectors-catalog`, with the exit code in the report.
