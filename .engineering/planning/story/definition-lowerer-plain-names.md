---
format: aep.planning-md/3
id: story:definition-lowerer-plain-names
kind: story
status: implemented
title: The definition lowerer refuses a catalog or file stem that is not a plain name
summary: catalog '..' or a file stem '..' made lower() write the declaration and selection outside --out; both are now refused as not_plain_name before any write
relations:
- implements: architecture-decision-record:provider-definitions-and-runtimes
- serves: vision:independent-contract-adapters
revision: 4
transitions:
- {from: "draft", to: "proposed", at: "2026-10-18T12:00:09Z", actor: "human:timo", revision: 2, decided_on: {"recorded":{"test_result":1}}}
- {from: "proposed", to: "active", at: "2026-10-18T12:00:09Z", actor: "human:timo", revision: 3, decided_on: {"recorded":{"test_result":1}}}
- {from: "active", to: "implemented", at: "2026-10-18T12:00:10Z", actor: "human:timo", revision: 4, decided_on: {"recorded":{"test_result":1}}}
---
## Outcome

The definition lowerer refuses a `catalog` or a file stem that is not a plain name before it writes
anything, so a definition can no longer place its declaration or selection outside `--out`. Every
caller of `connectors_catalog::definition::lower` and `connector` is covered, not only the
`operations invoke --definition` path, which carries its own guard (`story:invoke-definition-file`).

## Defect

`lower()` writes the declaration and the selection to `--out/<catalog>/`, where `<catalog>` is the
definition's `catalog` or, without one, the file stem. `parse()` checked `catalog` only with
`connectors_core::valid_id`, which admits `.` and `..`, and the stem was not checked at all.
Observed on the red run: `catalog: '..'` and a file `connectors/...yaml` (stem `..`) both lowered
and wrote `connector.json` and the selection beside `--out` instead of under it.

## Change

- `crates/connectors-catalog/src/definition.rs`: a new refusal `NotPlainName { field, value }`,
  named `not_plain_name`. `parse()` refuses a `catalog` that is empty, `.`, `..`, or holds `/`, `\`
  or NUL before its provider-id check; `stem()` refuses such a file stem. Both run before anything
  is written, in `lower()` and in `connector()`.
- The selection file name is `operations.json` or `<suffix>-operations.json`, where the suffix is
  part of the stem, so it is plain once the stem is.
- A `catalog` holding `/`, `\`, NUL or nothing was refused before as `malformed`; it is now refused
  as `not_plain_name`.

## Acceptance

1. `crates/connectors-catalog/tests/definition_layout.rs`
   `a_catalog_that_is_not_a_plain_name_is_refused_before_anything_is_written`: `catalog` `..`, `.`,
   `a/b`, `a\b`, empty and `a<NUL>b` are each refused as `not_plain_name` by `lower()` and by
   `connector()`, and nothing is written in or beside the output directory.
2. Same file,
   `a_file_stem_that_is_not_a_plain_name_is_refused_before_anything_is_written`: stems `.`, `..`
   and `a\b` are refused the same way.
3. `definition_import`: the 15 shipped definitions still lower byte-identical.
