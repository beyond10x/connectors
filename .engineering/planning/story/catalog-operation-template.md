---
format: aep.planning-md/1
id: story:catalog-operation-template
kind: story
status: active
title: Bind an inventoried operation into a bounded request template
relations:
- derived_from: specification:catalog-http-runtime-handoff
- decomposes: initiative:complete-local-connectors
- serves: vision:independent-contract-adapters
scope:
- confidence: inferred
  path: crates/connectors-catalog/src/lib.rs
- confidence: cited
  path: crates/connectors-catalog/src/template.rs
- confidence: cited
  path: crates/connectors-catalog/tests/template.rs
revision: 6
---
## Outcome

An inventoried operation becomes a bounded request template: the caller supplies
named values, and the template produces the path, the query, the headers and the
chosen media type, or refuses. Nothing in a template executes; it describes a
request and never builds one from arbitrary code.

## Scope

`crates/connectors-catalog/src/template.rs` and its tests. Built from one
`inventory::Operation`. Binding takes a map of name to value and returns the
resolved path with every `{placeholder}` replaced, the query pairs in a stable
order, the header pairs, and the selected request media type.

Refused by name, each with the parameter or media type it concerns: a required
parameter with no value, a value supplied for a parameter the operation does not
declare, a path placeholder the operation declares no parameter for, a path
parameter whose value is empty, and a requested media type the operation does
not offer. Cookie parameters are declared unsupported by this template pass
rather than silently dropped.

Path values are percent-encoded for the path segment set; query values are
encoded for the query set. No credential, no host, no scheme and no transport
live here: a template is joined to those by the runtime that owns them.

## Acceptance

An operation with a path parameter, a query parameter and a header binds to an
exact path, query and header set from a supplied map; each refusal case names
the offending parameter or media type; a value containing a slash or a space is
encoded rather than changing the path structure; and binding the same inputs
twice produces byte-identical output.

## Verification

`cargo test --locked -p connectors-catalog`, with the exit code in the report.
