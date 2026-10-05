---
format: aep.planning-md/3
id: story:websearch-contract
kind: story
status: draft
title: The datasource.websearch/v1alpha1 family is stated and modelled
relations:
- decomposes: epic:generic-websearch
revision: 1
---
## Outcome

`contracts/datasources/websearch/v1alpha1/semantics.md` states the family: operations
`websearch.search`, `websearch.fetch` and `websearch.crawl`, their inputs and result obligations, and
what a binding's native profile owns. `contracts/README.md` indexes it. `ess/domains/websearch.yaml`
models its values in the shared model, and `ess specify validate --path ess` with ESS 0.45.0 is valid.

## Acceptance

- Every result carries `complete`, `truncation` and `provenance` (adapter, profile, request instant),
  and bounds `content` by bytes, as the logs family does.
- Every operation is a read: it changes nothing at the provider. The text says provider credits are
  spent and that a read is not free.
- An input a binding cannot honour is refused by name before any request; it is never dropped.
- No provider name appears in the shared text or model.
