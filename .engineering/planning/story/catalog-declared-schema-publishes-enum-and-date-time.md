---
format: aep.planning-md/3
id: story:catalog-declared-schema-publishes-enum-and-date-time
kind: story
status: draft
title: An operation's declared input schema lists the pinned enum and date-time format the engine checks
summary: The engine refuses values outside a pinned enum or date-time format, but the declared input schema does not publish either
tags:
- from-finding
relations:
- decomposes: epic:zero-open-issues-and-store-defects
- serves: vision:independent-contract-adapters
- informed_by: review-result:wave-b-enum-date-adversary-1
revision: 1
---
## Defect

Found by the adversary pass on story:catalog-validates-enum-and-date-time
(review-result:wave-b-enum-date-adversary-1, finding on `adapters/catalog/src/lib.rs` `declare()`).

The catalog engine refuses a value outside a parameter's pinned `enum`, or a `format: date-time`
value that is not RFC 3339, as `invalid_input`. The input schema an operation declares
(`operations describe`) publishes neither: for example, a pinned integer enum is declared as
`{"anyOf":[{"type":"integer"},{"pattern":"^-?[0-9]+$","type":"string"}]}`. A caller that validates
against the declared schema therefore sends values the engine then refuses. `declare()` already
publishes a selection's own `bounds` as an `enum`.

## Acceptance

- A parameter with a pinned `enum` declares that enum in the operation's input schema (for a
  repeated parameter, on its elements), intersected with any selection `bounds`.
- A parameter with `format: date-time` declares `"format": "date-time"`.
- Every value the declared schema accepts for such a parameter passes the engine's check, and a
  test asserts it for one string enum, one integer enum and one date-time parameter.
- Revisions and digests that move are re-pinned in one change; the guide says what the schema
  declares.
