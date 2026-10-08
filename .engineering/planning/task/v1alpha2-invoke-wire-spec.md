---
format: aep.planning-md/3
id: task:v1alpha2-invoke-wire-spec
kind: task
status: active
title: Specify the first v1alpha2 invoke binding and its mutation observation
relations:
- decomposes: story:invoke-returns-attempt-id
- serves: vision:independent-contract-adapters
revision: 3
transitions:
- {from: "draft", to: "proposed", at: "2026-10-08T10:59:19Z", actor: "human:timo", revision: 2}
- {from: "proposed", to: "active", at: "2026-10-08T10:59:19Z", actor: "human:timo", revision: 3}
---
## Outcome

The first v1alpha2 binding is selected and specified: `POST /v1alpha2/invoke` only (describe stays on its own later unit), its closed request envelope and the § 5 Response with `version`, `request_id`, `status`, one of `result`/`error`, `audit_ref`, `audit_status` and `mutation?: connectors.service_wire.MutationObservation`. `contracts/service/compatibility.md` and `contracts/service/v1alpha2/semantics.md` state which § 4 invocation members this first binding admits and refuse the rest, and what `audit_status` it answers while the execution audit contract is unimplemented.

## Acceptance

- `ess/domains/service_wire.yaml` declares the request and Response; `ess validate --path ess` passes with the pinned `ess`.
- The generated JSON Schema is committed through the repository task and drift-checked by `task check`.
- The contract names the admitted members, the refused members and the `audit_status` value, each with a conformance scenario name.
