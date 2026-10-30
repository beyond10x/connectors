---
format: aep.planning-md/3
id: review-result:adversary-d240-s3-pass-1
kind: review-result
status: active
title: Adversary pass 1 on story:catalog-operation-declares-its-walk (wave D240)
tags:
- d240
relations:
- reviews: story:catalog-operation-declares-its-walk
revision: 1
---
## Adversary pass 1, unit S3 (wave D240), as returned

unit: story:catalog-operation-declares-its-walk (S3, D240), tree conn-d240-s3 at 3b7f2fba1; adversary cases committed as 77102164b
verdict: NEEDS-CHANGE
cases: executed 51→55, red 4
origin: introduced 2 / pre-existing 0 / undecided 0
wrote-outside-worktree: 11 files, all in the assigned scratch adv/ directory
needs-coordinator: I took no worktree lease, so check that none is outstanding

Cases: `adapters/catalog/tests/walk_adversary.rs` (3 red:
`a_first_only_parameter_the_selection_requires_is_refused_when_the_selection_loads`,
`a_walk_query_value_of_another_type_is_refused_when_the_selection_loads`,
`a_walk_query_value_outside_the_selection_bounds_is_refused_when_the_selection_loads`) and
`crates/connectors-catalog/tests/walk_import_adversary.rs` (1 red:
`an_import_refuses_by_name_a_walk_the_definition_cannot_say`: "the selection's five walks were dropped
and the import reported success; the written definition says `walk` 0 times").

Suite: walk_declarations ok 10 passed, walk_adversary 0 passed 3 failed (exit 101); definition_import
ok 41 passed, walk_import_adversary 0 passed 1 failed (exit 101).

A (`crates/connectors-catalog/src/import.rs:2171`, NEEDS-CHANGE, introduced): the module contract
(`import.rs:9-11`) says an unexpressible selection is refused by name; the stopgap drops `walk` and
reports success. Reached by `connectors-build` import on any of the 10 walked selection files.
B (`adapters/catalog/src/lib.rs:3285`, INFEASIBLE, introduced): the loader admits a `query` value the
operation's schema rejects and a `first_only` parameter the selection requires; no shipped walk does.

Not broken: Confluence v1 string `start`, load-time refusals, walker semantics (repeated cursor,
has_more, null vs absent, non-string positions, integer end_time), `url_parameter` never followed, all
27 shipped declarations match their docs, `x-connectors-walk` refused from callers by
additionalProperties false and ignored by host schema compilation and MCP inline_schema.

```findings
- file: crates/connectors-catalog/src/import.rs
  line: 2171
  category: contract-drift
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: the importer stopgap writes a walked selection as a definition without its walks and reports success, against its module contract to refuse an unexpressible selection by name (test an_import_refuses_by_name_a_walk_the_definition_cannot_say)
- file: adapters/catalog/src/lib.rs
  line: 3285
  category: boundary
  severity: note
  verdict: INFEASIBLE
  origin: introduced
  message: the walk loader admits a query value of the wrong type or outside the selection bounds and a first_only parameter the selection requires, so the loaded walk builds pages its own operation refuses as invalid_input; no shipped walk does (tests a_walk_query_value_of_another_type_is_refused_when_the_selection_loads, a_walk_query_value_outside_the_selection_bounds_is_refused_when_the_selection_loads, a_first_only_parameter_the_selection_requires_is_refused_when_the_selection_loads)
```
