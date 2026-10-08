---
format: aep.planning-md/3
id: story:invoke-returns-attempt-id
kind: story
status: draft
title: A successful invoke names the attempt it produced
relations:
- serves: vision:independent-contract-adapters
revision: 1
---
## Outcome

A caller of `connectors_client::Client::invoke` learns which attempt its successful invocation
produced: the answer carries the id of the `connectors.mutations.AttemptRecord` the host recorded for
it, beside the result value (for example `{ value, attempt_id }`, or an `invoke_with_attempt`
variant that keeps `invoke` source-compatible). A downstream consumer correlates its own records
with the attempt through that id.

## Source

`Client::invoke` (`crates/connectors-client/src/lib.rs:97`) returns only the result `Value`; the wire
`Response` (`crates/connectors-core/src/lib.rs:132`) carries no attempt id. `AttemptRecord` is
modelled in `ess/domains/mutations.yaml:82`.

## Acceptance

- Spec first: the response that carries the attempt id is modelled in the ESS specification and
  validated with the pinned `ess`; the wire type is generated or checked against it.
- A successful invocation through `Client` returns the attempt id, and that id names the
  `AttemptRecord` the host recorded for the invocation (test reads the record back).
- A refused or failed invocation returns no attempt id it did not record.
- Existing `invoke` callers keep compiling, or the CHANGELOG states the breaking change and its
  migration.
