# Execution-audit model closure verification

This evidence covers the specification-only execution-audit unit selected by
`specification:core-model-closure-20260909`. It does not execute persistence,
authorization, dispatch, a provider, an exporter or an audit query service.

## Baseline at `8a5cf563`

| Case | Required result | Baseline observation | Result |
|---|---|---|---|
| AU-01 | Owning prose selects one durable acknowledged admission anchor before every audited read or mutation dispatch | Compatibility states the ordering, but no audit owner document selects the record shape, retention or recovery rule | fail |
| AU-02 | One logical final observation has stable idempotent acknowledgement recovery and cannot authorize redispatch | Final append is required, but identity/conflict/recovery semantics are absent | fail |
| AU-03 | Gateway and executing-hop records are distinct | Delegation requires two audit writes, but no typed record distinguishes their hop roles | fail |
| AU-04 | Optional mutation correlation references `AttemptRecord` without owning it | The audit entity and reference do not exist | fail |
| AU-05 | Reads require audit; unavailable anchor is absence, while incomplete means an acknowledged anchor exists | Wire prose has the status rule, but there is no model for the anchor lifecycle | fail |
| AU-06 | Audit retention has no automatic expiry or deletion cascade; capacity refuses new admitted work | Retention and capacity remain unspecified | fail |
| AU-07 | The proposed model validates and compiles with the existing real ESS tree | `connectors.execution_audit` is absent from the domain registry and source tree | fail |

The baseline checks confirmed that `contracts/service/audit.md` and
`ess/domains/execution_audit.yaml` did not exist, the domain was not registered,
and `service_wire.yaml` still marked `AuditRecord` persistence as UNMAPPED.

## Initial integration finding

The initial audit candidate used public `audit_ref` alone as the ESS identity even
though the selected public identity is `{instance, audit_ref}`. Because two
instances may allocate the same opaque ref, that would have silently imposed
global public-ref uniqueness. This finding failed the initial handoff and was
corrected before integration.

## Corrected textual and model audit

| Case | Selected result | Status |
|---|---|---|
| AU-01 | `AuditRecord` has an acknowledged `admitted_execution` anchor; every audited read and mutation requires it before dispatch | pass |
| AU-02 | One owner-allocated final `observation_id` is appended; same-id/same-content retry acknowledges the original, while conflicting content/id refuses and never redispatches | pass (semantic rule; runtime UNMAPPED) |
| AU-03 | `gateway` and `execution` hop roles allocate independent host-qualified records | pass |
| AU-04 | Optional `attempt_id` has an ESS `references` relation to the existing `AttemptRecord`; neither owner cascades lifecycle or deletion | pass |
| AU-05 | `unavailable` is absence of an acknowledged anchor; `Anchored` projects incomplete until the final observation is acknowledged; reads follow the same rule | pass |
| AU-06 | No expiry/delete transition is selected; the first binding caps retained records at 100,000 per instance and refuses new audited work at capacity | pass (semantic rule; runtime UNMAPPED) |
| AU-07 | A scratch copy of the real ESS tree with the exact proposed domain registration validates and compiles | pass |
| AU-08 | Instances `alpha` and `beta` may each expose `audit_ref: same` while private injectively qualified `audit_record_ref` keys remain distinct; commands and views use only the private key | pass |

The private tagged, length-prefixed, base64url record ref is bounded to 512 ASCII
bytes. Selected instance/public audit/request/principal/connection refs remain
bounded to 128 UTF-8 bytes,
operation ids to 256, descriptor revisions to 128, safe final codes to 64 ASCII
bytes and the encoded aggregate to 4 KiB. `observation_id` is an internal UUID;
public `audit_ref` retains its existing opaque-string meaning.

The model also permits `early_refusal` anchors with optional coordinates. Only
independently verified facts may be populated, and such a record cannot satisfy
the execution dispatch gate. It is distinct from `unavailable`, which creates no
acknowledged record.

## Verification

- Baseline pinned ESS 0.20.0 validation: 14 files valid, exit 0.
- Baseline compile: 218 declarations, exit 0.
- Unit scratch-tree validation with `connectors.execution_audit` registered:
  15 files valid, exit 0.
- Corrected unit scratch-tree compile: 235 declarations, exit 0.
- Exact system, contract-join and model-join patches each pass
  `git apply --check` against `8a5cf563`, exit 0.
- `git diff --check`: exit 0.

The unit compile uses a copied real ESS tree plus only the proposed domain
registration. The tracked `ess/system.yaml` remains coordinator-owned. ESS proves
type/reference validity and lifecycle causation; it does not assign optional
fields, enforce byte/count/co-presence rules, prove trusted fact provenance,
perform durable/atomic writes, compare retry content, enforce clocks/capacity/
retention or prevent actual dispatch. Those remain implementation predicates.
