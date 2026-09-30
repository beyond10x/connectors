---
format: aep.planning-md/3
id: story:catalog-repeated-query-parameters
kind: story
status: implemented
title: A form-exploded array query parameter is sent as one pair per element
relations:
- decomposes: epic:google-workspace-reads
- serves: vision:independent-contract-adapters
scope:
- confidence: inferred
  path: adapters/catalog/src/lib.rs
- confidence: inferred
  path: crates/connectors-catalog/src/inventory.rs
- confidence: inferred
  path: crates/connectors-catalog/src/template.rs
- confidence: inferred
  path: crates/connectors-catalog/tests/inventory.rs
revision: 6
transitions:
- {from: "draft", to: "proposed", at: "2026-09-30T17:43:06Z", actor: "human:timo", revision: 4, decided_on: {"recorded":{"review_outcome":5,"verification":1}}}
- {from: "proposed", to: "active", at: "2026-09-30T17:43:06Z", actor: "human:timo", revision: 5, decided_on: {"recorded":{"review_outcome":5,"verification":1}}}
- {from: "active", to: "implemented", at: "2026-09-30T18:33:43Z", actor: "human:timo", revision: 6, decided_on: {"recorded":{"test_result":1,"review_outcome":5,"verification":1}}}
---
## Outcome

A query parameter declared as an array with `style: form, explode: true` accepts a JSON array in
`operations invoke` input and is sent as one `name=value` pair per element. Needed by Gmail
`labelIds` and Calendar `eventTypes` (5 and 6 `repeated` parameters in the live Discovery
documents, 2026-09-30).

## Design

- `crates/connectors-catalog/src/inventory.rs` (`Parameter`, `:50-76`): add `repeated: bool`,
  `skip_serializing_if` false, set for `in: query` parameters whose schema is `type: array` with
  `explode` absent or `true` and `style` absent or `form`. An array query parameter with any other
  style or `explode: false` is recorded `Unsupported` with a named reason. Existing bundles keep
  their bytes.
- `crates/connectors-catalog/src/template.rs` (`:571`): emit one pair per element, in input order,
  each escaped like a scalar value; an empty array emits nothing.
- `adapters/catalog/src/lib.rs` (`:128-136,381-392`): accept a JSON array of scalars only where
  `repeated` is set; an array elsewhere, or a nested array/object element, is `invalid_input`
  before any request. The declared input schema shows `type: array` for such parameters. Bounds
  (`:142-169`) apply per element.
- Confluence's comma-separated `space-id` guidance (`docs/catalog-confluence.md:156-157`) is
  unchanged: that document declares it differently.

## Acceptance

- `inventory_marks_form_explode_array_repeated`, `inventory_records_unsupported_array_style`.
- `template_repeats_query_pairs` (order kept, each element escaped), `template_empty_array_omits`.
- `engine_sends_repeated_pairs`: `operations invoke` input `{"labelIds": ["INBOX", "UNREAD"]}` on a
  fixture operation with a repeated `labelIds` sends `labelIds=INBOX&labelIds=UNREAD` (asserted on
  the recorded request).
- `engine_declares_array_schema`: the operation's declared input schema gives the repeated
  parameter `type: array` with scalar `items`.
- `engine_refuses_array_on_scalar_parameter`, `engine_refuses_nested_array_element`,
  `engine_bounds_apply_per_element` — all with no request sent.
- `bundle_drift.rs` reproduces every existing bundle byte for byte.

## Out of scope

Arrays in path or header parameters; `style: spaceDelimited` / `pipeDelimited`; object parameters.
