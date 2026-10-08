---
format: aep.planning-md/3
id: task:http-host-audit-anchor
kind: task
status: draft
title: The HTTP host anchors an audit record before it dispatches a v1alpha2 write
relations:
- decomposes: story:invoke-returns-attempt-id
- depends_on: task:v1alpha2-invoke-wire-codec
- serves: vision:independent-contract-adapters
revision: 1
---
## Outcome

The HTTP host records the execution-audit anchor of `contracts/service/audit.md` § 1-2 for every admitted v1alpha2 `external_write` invocation and has it acknowledged before provider dispatch, then records the single final observation (§ 3). A write that cannot get its anchor is refused before dispatch. The v1alpha2 Response of an audited write carries `audit_ref` and `audit_status: complete`; `incomplete` when the final observation could not be written. Reads keep the audit status the spec unit assigns them.

## Acceptance

- Spec first: the anchor and final observation persistence is declared in ESS (`connectors.execution_audit`, extended if needed) before the code.
- `crates/connectors-host/tests/`: a write answers `complete` with an `audit_ref` that reads back the anchor; an anchor write that fails refuses the invocation with zero adapter calls; a lost final observation answers `incomplete` and is recovered idempotently per § 3.
- Bounds and retention follow audit.md § 4.
