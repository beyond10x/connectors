---
format: aep.planning-md/3
id: story:guard-number-inputs-compare-by-type
kind: story
status: draft
title: A guard input from a number-typed body key matches only a JSON number
tags:
- from-finding
relations:
- decomposes: epic:zero-open-issues-and-store-defects
- informed_by: review-result:wave-b-guard-typed-adversary-1
revision: 1
---
## Defect

Found by the adversary of `story:catalog-guard-typed-comparison`
(`review-result:wave-b-guard-typed-adversary-1`), origin undecided: a guard `input` expectation that
resolves to a JSON number still compares as text (`adapters/catalog/src/lib.rs:2016` on
`impl/catalog-guard-typed-comparison` `8f11ed6fc`). `project.create` accepts a string `"42"` answered
for `namespace_id` 42, before and after the write. What reaches it: nothing found; GitLab answers
namespace ids as integers.

A global number arm would break path-parameter checks that compare as text (Drive `/version`,
`pipeline_id`, Zendesk `ticket_id`); a typed comparison would have to be limited to body keys that
`body_types` types `integer` or `number`, and every shipped provider's answer for those keys checked.

## Acceptance

- A guard input taken from a body key typed `integer` or `number` matches only a JSON number in the
  provider's answer, before and after the write; path parameters and string inputs keep comparing as
  text; every shipped guard keeps passing its recorded fixtures.
