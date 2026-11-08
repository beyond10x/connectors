---
format: aep.planning-md/3
id: story:owner-bounded-generic-read-inline-schemas
kind: story
status: draft
title: The owner's bounded generic read requires inline schemas, as the projection does
tags:
- from-finding
relations:
- informed_by: story:owner-bounded-read-realization-matches-projection
revision: 1
---
## Why

Wave 2026-11-06 U2 (commit f42d53975) made the owner's bounded read accept `realization: implemented` only for the `resource` profile with inline input and output schemas, as the MCP projection does. It kept the `generic` arm unchanged, as its brief required. The projection applies the same inline-schema rule to generic reads (`apps/connectors/src/mcp/projection.rs` `Snapshot::limits`, the `inline_schema` checks before the realization match), so the owner still accepts a generic `generic-http` or `generic-http-page` read over a non-inline schema that the projection never lists. A same-uid client sending a `BoundedRead` frame directly reaches that difference. The U2 implementor reported it (wave 2026-11-06 U2 report); inferred from code, not run.

## Outcome

The owner's bounded read refuses a `generic` realization whose input or output schema is not inline, as `unsupported` before any provider request, using the shared `connectors_core` inline-schema check. A case drives the owner directly with a generic read over a `$ref` schema and asserts a request count of 0.

## Notes

Pre-existing in the generic arm, filed as a draft by the 2026-11-06 wave. No wave takes it before every planned wave is delivered.
