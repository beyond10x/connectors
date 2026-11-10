---
format: aep.planning-md/3
id: story:catalog-guard-typed-comparison
kind: story
status: active
title: A catalog guard compares JSON values by type
relations:
- serves: vision:independent-contract-adapters
- decomposes: epic:zero-open-issues-and-store-defects
scope:
- confidence: inferred
  path: adapters/catalog/src/lib.rs
revision: 5
transitions:
- {from: "draft", to: "proposed", at: "2026-11-09T21:57:51Z", actor: "human:timo", revision: 2, executor: "agent:connectors-zero-issues", correlation: "connectors-zero-issues"}
- {from: "proposed", to: "active", at: "2026-11-09T21:57:51Z", actor: "human:timo", revision: 3, executor: "agent:connectors-zero-issues", correlation: "connectors-zero-issues"}
---
## Outcome

A catalog guard compares JSON booleans by type: a boolean expectation, or an input that resolves to a
JSON boolean, matches only a JSON boolean in the provider's answer. Numbers taken from number-typed
body keys are `story:guard-number-inputs-compare-by-type` (narrowed on 2026-11-10 after
`review-result:wave-b-guard-typed-adversary-1`: a global number arm would break path-parameter checks).

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
