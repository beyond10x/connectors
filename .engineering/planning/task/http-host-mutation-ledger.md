---
format: aep.planning-md/3
id: task:http-host-mutation-ledger
kind: task
status: draft
title: The HTTP host records an attempt for every admitted write and serves /v1alpha2/invoke
relations:
- decomposes: story:invoke-returns-attempt-id
- depends_on: task:v1alpha2-invoke-wire-codec
- serves: vision:independent-contract-adapters
- depends_on: task:http-host-audit-anchor
revision: 1
---
## Outcome

The HTTP host (`crates/connectors-host/src/server.rs`) serves `POST /v1alpha2/invoke`. For an admitted `external_write` operation it records a `connectors.mutations.AttemptRecord` before provider dispatch, and records the final observation after it; the Response carries the `MutationObservation` naming that attempt. Reads record nothing. `/v1/invoke` keeps its behaviour.

## Acceptance

- `crates/connectors-host/tests/service.rs`: a write returns an attempt id and the test reads the record back; a refusal before dispatch returns none; a provider answer lost after dispatch is `outcome_unknown` with the recorded attempt and no second dispatch; a read has no `mutation`.
- The ledger store, its bounds and its retention are declared in ESS before the code (`ess/domains/mutations.yaml`).
