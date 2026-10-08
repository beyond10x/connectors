---
format: aep.planning-md/3
id: task:http-host-audit-anchor
kind: task
status: active
title: The HTTP host anchors an audit record before it dispatches a v1alpha2 write
relations:
- decomposes: story:invoke-returns-attempt-id
- depends_on: task:v1alpha2-invoke-wire-codec
- serves: vision:independent-contract-adapters
revision: 5
transitions:
- {from: "draft", to: "proposed", at: "2026-10-08T12:03:59Z", actor: "human:timo", revision: 3}
- {from: "proposed", to: "active", at: "2026-10-08T12:03:59Z", actor: "human:timo", revision: 4}
---
## Outcome

The HTTP host records the execution-audit anchor of `contracts/service/audit.md` § 1-2 for every admitted v1alpha2 `external_write` invocation and has it acknowledged before provider dispatch, then records the single final observation (§ 3). A write that cannot get its anchor is refused before dispatch. The v1alpha2 Response of an audited write carries `audit_ref` and `audit_status: complete`; `incomplete` when the final observation could not be written. Reads keep the audit status the spec unit assigns them.

## Acceptance

- Spec first: the anchor and final observation persistence is declared in ESS (`connectors.execution_audit`, extended if needed) before the code.
- `crates/connectors-host/tests/`: a write answers `complete` with an `audit_ref` that reads back the anchor; an anchor write that fails refuses the invocation with zero adapter calls; a lost final observation answers `incomplete` and is recovered idempotently per § 3.
- Bounds and retention follow audit.md § 4.

## Spec gaps found by the wire-spec unit

Declare these in `connectors.execution_audit` before the code:

1. The anchor is written before the attempt (`contracts/service/v1alpha2/semantics.md` § 4 rule 4), but `attempt_id` is settable only by `AcknowledgeAnchor`. Add a command that links the recorded attempt to its anchor, idempotent and once per anchor.
2. `AnchorDecision` has only `allow | capacity`; add the store that fails or does not answer, which refuses before dispatch as HTTP 503 `unavailable`.
3. An admitted anchor needs `principal_ref`; declare where the static-bearer service principal identity of the HTTP host is configured.

## Implementation notes from the specification part (merged)

- Open: on host start with a new configuration revision, `AdvanceRegistryEpoch` sets only `registry_epoch` (`ess/domains/declarations.yaml:225`). Decision: the host start records the revision with the command the local owner already uses to move an instance to a new configuration revision; if no declared command moves `revision`, declare one that moves revision and epoch together, before the code.
- Verify with cargo that the `when_subject` refusals over `access` and the optional `attempt_id` lower to Entity Runtime; regenerate `crates/connectors-host/src/local/metadata/entity-runtime-definitions.json` with `connectors-build metadata-entities`.
- `AttemptRecord.connection_ref` is optional: update `local/metadata/mutations.sql:9`, `local/mutations/types.rs:50,206,275`, `local/mutations.rs:153-154,566-583`, `local/audit.rs:307-321`, `local/metadata/er.rs:294,1232,3092`, `local/owner/mutation.rs:329`.
- Service configuration `urn:connectors:config:v2:service` adds optional `state`; v1 stays readable (`crates/connectors-host/src/schema.rs:19-25`).
