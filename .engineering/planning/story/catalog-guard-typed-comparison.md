---
format: aep.planning-md/3
id: story:catalog-guard-typed-comparison
kind: story
status: draft
title: A catalog guard compares JSON values by type
relations:
- serves: vision:independent-contract-adapters
revision: 1
---
## Outcome

A catalog guard compares JSON values by type: a boolean literal matches only a JSON boolean, a
number only a number, a string only a string.

## Why

Guard comparisons turn both sides into text before comparing (`adapters/catalog/src/lib.rs`,
`scalar`), so the literal `true` at `/resolvable` in `merge_request.discussion.resolve` also
accepts the JSON string `"true"`. No provider answer was seen to do that; the adversary review of
`story:parity-gitlab-mr-writes` constructed one. It is the first boolean literal in a shipped
guard, and every later one inherits the looseness.

## Acceptance

- Spec first: the comparison rule is stated in the catalog ESS model or the selection format's
  semantics before the engine changes.
- A preflight answer carrying a string where the guard expects a boolean refuses before the
  write; a postflight answer doing the same is reported as unknown.
- Every shipped guard keeps passing its existing tests unchanged.
