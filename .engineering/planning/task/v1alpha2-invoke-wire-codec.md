---
format: aep.planning-md/3
id: task:v1alpha2-invoke-wire-codec
kind: task
status: draft
title: Decode and encode the v1alpha2 invoke envelope in connectors-core
relations:
- decomposes: story:invoke-returns-attempt-id
- depends_on: task:v1alpha2-invoke-wire-spec
- serves: vision:independent-contract-adapters
revision: 1
---
## Outcome

`connectors-core` gains the v1alpha2 invoke request and Response types: strict decoding, unknown and duplicate members refused, version checked against the route, `mutation` present only when the Response carries one. The v1alpha1 types are unchanged.

## Acceptance

- Round-trip and refusal vectors in `crates/connectors-core/tests/` for both bindings; the existing v1alpha1 vectors pass unchanged.
- A test validates every v1alpha2 vector against the schema generated in `task:v1alpha2-invoke-wire-spec`.
