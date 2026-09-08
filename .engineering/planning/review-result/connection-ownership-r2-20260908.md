---
format: aep.planning-md/1
id: review-result:connection-ownership-r2-20260908
kind: review-result
status: active
title: Independent connection ownership review round 2
relations:
- reviews: story:contracts-connection-readiness
- reviews: story:contracts-management-boundary
revision: 1
---
# Independent management/readiness recheck B — round 1

Verdict: **ship with the remaining selector/repair wording corrections**. Four substantive cross-document inconsistencies were fixed and independently re-read during this pass. The two remaining items are RCBR-05 (P2) and RCBR-06 (P3). This is a semantic draft review; the new evidence bundle was still being assembled and has not been reviewed or claimed as passing.

`source-hashes-r2.json` preserves the 23-file first draft in `recheck-1-snapshot/`. Four files with verified during-review corrections are separately frozen in `source-hashes-r2-corrections.json` and `recheck-1-corrections-snapshot/`; the original snapshot remains unchanged. Baseline is 12c11f4b43cd6cf5ff67d3243017887b28788344. No source/planning edits, runtime tests, external calls, other reviewer reports or extra agents were used.

## Remaining concrete corrections

### RCBR-05 — P2: optional Invocation.connection became “nullable” by implication

Management §1 calls the field “the existing nullable Invocation.connection selector” and says create/list reject a “non-null” selector. E02 compatibility §4 instead defines the field as an optional safe connection ref and permits omission only where the selected profile resolves one implicitly. The management instance/acquisition-scoped exception is intended, but the texts disagree both about omission and about present JSON null. The nullable F03 **signed claim** is a different object from the Invocation field.

Minimal fix: specify a present, nonempty exact string for connection-scoped describe/revoke/repair; omit the Invocation field for instance/acquisition-scoped create/list/status, rejecting a supplied selector including JSON null. A protected completion is not an Invocation. Align compatibility §4's omission rule with those selected management scopes, while keeping implicit configured resolution only for profiles that actually define it. Reject duplicate body selectors that disagree. Do not silently broaden the wire to null or invent an implicit business connection. A root-level `connection:null` example should not be accepted merely because its F03 claim uses null.

### RCBR-06 — P3: management says revoked records can be repaired immediately before prohibiting it

Management §1 says an admitted caller “must be able to inspect, repair or revoke pending, disabled, degraded and revoked records,” then says revoked records cannot be repaired or re-enabled. The second rule and the new terminal state reduction agree; the first sentence accidentally makes a conflicting promise.

Minimal fix: distinguish the actions directly: independently admitted inspection/local revoke can target non-ready records including revoked; repair may target only a non-revoked binding. The rule exempts management from provider readiness, not from action-specific lifecycle prerequisites.

## Corrections independently verified during this pass

| Finding | Corrected behavior |
|---|---|
| RCBR-01 — P2: stale evidence still forced global reauthorization_required | Evidence §4's remaining old bullet now uses the selected reduction: stale baseline -> pending/connection_not_ready; stale permission -> unavailable for its operation; stale operation verification -> connection_not_ready without global poisoning. Positive invalidity and consumed refresh remain different facts. |
| RCBR-02 — P2: host policy denial disagreed with E02 ErrorCode meaning | Connection §3/§4.1 and evidence conformance now distinguish receiver grant-policy not_granted from adapter/connection/resource and provider permission forbidden. Authority outage remains unavailable. This matches service v1alpha2 §3.5. |
| RCBR-03 — P2: eligibility required a public requires_auth alternative even when absent by design | Connection §4.1 now permits the exact explicitly configured binding when the member is absent under compatibility §4. It expressly forbids anonymous, missing-credential or different-binding fallback. Required checks/current authority still apply to either branch. |
| RCBR-04 — P2: mandatory UI channel accidentally blocked noninteractive client credentials | Management §3 and acquisition flow step 3 now limit the confidential continuation/ingress prerequisite to interactive flows. Noninteractive client-credentials begin follows admitted registration/provider/custody/current authority and can complete without a UI/action. |

## Foundation requirement disposition

RCB-01–RCB-08 are substantively addressed by the draft, subject to the two precise corrections above and final evidence confirmation:

- Host dispatch owns connection/acquisition orchestration, metadata publication/revocation, custody references and audit; provider hooks own provider construction/interpretation. No Adapter::invoke or generic public callback/refresh/custody port is implied.
- Separate management permissions and instance/connection/acquisition targets avoid borrowing business grants or requiring usable provider credentials for metadata/repair/revoke. Current disclosure remains necessary for returned records.
- One stable logical coordinator records origin/profile/registration/repair revision; current permission is checked before exchange and publication. Aliases cannot repoint an in-flight acquisition. Both gateway/leaf admit safe management; protected completion terminates at the owner through a separate binding.
- The draft deliberately chooses the stricter ordinary-result boundary: only safe acquisition ref/expiry/action kind is public; continuation URL and protected-entry schema travel to an independently admitted confidential UI channel. This is consistent with the corrected compatibility and acquisition documents. Callback state/credential material cannot be discovered or replayed through generic operation results.
- Local revocation and its serialization cutoff remain authoritative even if provider revocation is unsupported, failed or unknown. An ambiguous local acknowledgement grants no new provider call. Previously known business/provider effects are not erased; incomplete effect/approval/idempotency schemas remain advertisement gates.
- Seven-state viability is a deterministic first-match reduction over current authoritative metadata, local revocation/enablement and required common evidence. Operation scope/purpose, exact-resource permission and operation-only verification remain separate. Minimum grants are profile-wide publication prerequisites, never a union over enabled operations; optional narrowing can preserve an eligible read.
- Failed independent repair candidates preserve independently valid active material. Current-material replacement, expiry, revocation and consumed refresh retain F04/F05 generation/pin/publication protections. Cached global ready cannot bypass a final dispatch check.
- ESS adds value types for status facts, eligibility, requirements and management targets without inventing Connection/Acquisition/AuthProfile persistence ownership or a global readiness lifecycle. Generic optional projection, authority checks, ordered reduction, cross-value constraints, publication/replica fencing and protected codecs remain explicitly UNMAPPED.

No runtime implementation, exact management CRUD schema, issuer UI or new approval subject is required by this review. The remaining full binding/codec/persistence work must stay visibly gated; the final evidence review should confirm the textual cases and type/schema claims against the corrected source bytes.
