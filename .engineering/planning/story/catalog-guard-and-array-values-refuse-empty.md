---
format: aep.planning-md/3
id: story:catalog-guard-and-array-values-refuse-empty
kind: story
status: draft
title: A guard-bound value or an array element for a required parameter is refused when empty
summary: The empty-required-value refusal skips guard-bound values and elements of a required repeated parameter
tags:
- from-finding
relations:
- decomposes: epic:zero-open-issues-and-store-defects
- serves: vision:independent-contract-adapters
- informed_by: story:required-parameter-refuses-empty
revision: 1
---
## Defect

Found while implementing story:required-parameter-refuses-empty on 2026-11-10. The defect was
there before that work.

The catalog engine now refuses a required path or query parameter given as an empty string
(`Engine::parameter_values`, `adapters/catalog/src/lib.rs`). Two neighbouring routes still let an
empty value reach the request:

- a guard-bound value (`bind()` in `prepare`) goes through `resolve` without the check, so an
  empty guard value for a required parameter is sent;
- an empty element inside a required repeated (array) parameter is not refused.

## Acceptance

- A write whose guard binds a required path or query parameter to an empty string is refused as
  `invalid_input` naming the parameter, before any request.
- A required repeated parameter with an empty element is refused the same way.
- The provider package's suites stay green; a shipped selection that needs either form is named
  here before the rule changes, or there is none.
