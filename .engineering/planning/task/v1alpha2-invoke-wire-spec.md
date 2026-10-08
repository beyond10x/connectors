---
format: aep.planning-md/3
id: task:v1alpha2-invoke-wire-spec
kind: task
status: implemented
title: Specify the first v1alpha2 invoke binding and its mutation observation
relations:
- decomposes: story:invoke-returns-attempt-id
- serves: vision:independent-contract-adapters
revision: 5
transitions:
- {from: "draft", to: "proposed", at: "2026-10-08T10:59:19Z", actor: "human:timo", revision: 2}
- {from: "proposed", to: "active", at: "2026-10-08T10:59:19Z", actor: "human:timo", revision: 3}
- {from: "active", to: "implemented", at: "2026-10-08T14:14:23Z", actor: "human:timo", revision: 5}
---
## Outcome

The first v1alpha2 binding is selected and specified: `POST /v1alpha2/invoke` only (describe stays on its own later unit), its closed request envelope and the § 5 Response with `version`, `request_id`, `status`, one of `result`/`error`, `audit_ref`, `audit_status` and `mutation?: connectors.service_wire.MutationObservation`. `contracts/service/compatibility.md` and `contracts/service/v1alpha2/semantics.md` state which § 4 invocation members this first binding admits and refuse the rest, and what `audit_status` it answers while the execution audit contract is unimplemented.

## Acceptance

- `ess/domains/service_wire.yaml` declares the request and Response; `ess validate --path ess` passes with the pinned `ess`.
- The repository gate generates the JSON Schema into a temporary directory and checks the codec vectors against it, as it does for CLI values (`crates/connectors-build/src/cli.rs` `validate_values`); no generated tree is committed.
- The contract names the admitted members, the refused members and the `audit_status` value, each with a conformance scenario name.
