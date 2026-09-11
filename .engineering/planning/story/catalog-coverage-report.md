---
format: aep.planning-md/1
id: story:catalog-coverage-report
kind: story
status: active
title: Report what a bundle covers and what it does not
relations:
- derived_from: specification:catalog-http-runtime-handoff
- decomposes: initiative:complete-local-connectors
- serves: vision:independent-contract-adapters
scope:
- confidence: cited
  path: crates/connectors-catalog/src/coverage.rs
- confidence: inferred
  path: crates/connectors-catalog/src/lib.rs
- confidence: cited
  path: crates/connectors-catalog/tests/coverage.rs
revision: 6
---
## Outcome

A bundle can say what it covers and what it does not, in one report a person
reads and a command compares. Coverage is never a single number: the report
carries what was inventoried, what was named unsupported and why, and the source
it was all derived from, so nobody can quote a percentage that hides its gaps.

## Scope

`crates/connectors-catalog/src/coverage.rs` and its tests. Built from a
`bundle::Bundle`. The report carries the provider, the source digest and the
document's own version strings, the inventoried operation count, the unsupported
count, the unsupported reasons grouped by reason with the designations under
each in a stable order, the methods present with a count each, and the number of
operations that declare no `operationId`.

Two renderings of one value: a `Report` that serialises, and a plain-text form
for a person. Both are deterministic — the same bundle renders byte-identically
twice — and the text form states both counts wherever it states either.

No percentage is computed. A ratio invites quoting one number, and the pair is
what the report exists to keep together.

## Acceptance

A bundle with three operations and two unsupported entries reports 3 and 2, the
two reasons grouped with their designations, and the methods with their counts;
rendering the same bundle twice gives byte-identical output in both forms; an
empty bundle reports zeroes rather than refusing; and the text form contains the
unsupported count even when it is zero.

## Verification

`cargo test --locked -p connectors-catalog`, with the exit code in the report.
