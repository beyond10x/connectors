---
format: aep.planning-md/3
id: task:client-invoke-v1alpha2
kind: task
status: draft
title: The client invokes over v1alpha2 and returns the attempt it produced
relations:
- decomposes: story:invoke-returns-attempt-id
- depends_on: task:http-host-mutation-ledger
revision: 1
---
## Outcome

`connectors-client` selects the v1alpha2 binding explicitly (no fallback to `/v1`) and offers an invoke that returns the result value with the optional `MutationObservation`. `Client::invoke` keeps its signature and the v1 binding.

## Acceptance

- A client test against the host: the returned attempt id equals the recorded one; an older host answering 404 on `/v1alpha2/invoke` is reported as unsupported and not retried on `/v1`.
- `crates/connectors-conformance` runs one v1alpha2 write scenario.
- CHANGELOG and the website service guide describe the route and the client entry point.
