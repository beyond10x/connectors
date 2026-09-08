---
format: aep.planning-md/1
id: review-result:connection-semantics-r1-20260908
kind: review-result
status: active
title: Independent connection semantics review round 1
relations:
- reviews: story:contracts-connection-readiness
- reviews: story:contracts-management-boundary
revision: 1
---
# Connection semantics — independent initial reviewer A

**Verdict: the existing readiness reduction is contradictory; select the following rules before finalizing F07/E04.** This is a bounded specification review, not implementation or decomposition. Eight actionable requirements follow.

Baseline: `12c11f4b43cd6cf5ff67d3243017887b28788344`. `source-hashes.json` and `sources/` freeze 24 reviewed inputs. All normative contracts/design/ESS files match that baseline. The two planning bodies were advanced by the author during review to authorize the current contract/ESS slice; their exact active revisions are preserved and were read. No other reviewer output was consulted, no runtime test was run, and no source/planning file was changed by this reviewer.

## Requirements

### CS-A01 — Separate global viability from operation eligibility

**Defect:** connection:58 makes any missing scope for an enabled operation a global `insufficient_scope`; connection:74 refuses all non-ready connections. Evidence:132 explicitly requires a read to proceed when write scope is absent. Evidence:43/49/74 repeats the conflation by deriving one global state from operation-specific checks.

**Required rule:** derive connection viability from connection-wide requirements only. Derive eligibility for `(connection, operation, exact input/resource, caller, generation)` separately. An eligible invocation needs enabled implementation, current host admission, globally viable dependencies, one matching `requires_auth` alternative, and every required operation-specific check. Do not union grants across alternative profiles or connections. Missing optional write scope or denied resource permission leaves other eligible reads usable. Description and dispatch share the same reduction logic and subject, not an identical global boolean.

### CS-A02 — Fix lifecycle precedence and distinguish the two revocations

**Defect:** connection's public states at 54–62 omit `pending`; its future-model lifecycle at132 includes pending but omits disabled. Parent degradation at82 can overwrite even a child's terminal status. Evidence alone cannot express administrative state.

**Required rule:** publish one deterministic precedence table using authoritative connection facts plus viability evidence. At minimum, local terminal `revoked` outranks disabled and every evidence/parent overlay; disabled remains disabled despite successful evidence; parent degradation cannot resurrect or relabel a revoked child. Explicitly distinguish local connection revocation (terminal) from provider credential revocation (repairable `reauthorization_required`). Define pending only for a real unpublished/unvalidated connection binding. An acquisition in pending does not imply that a public Connection already exists; if pending Connection is exposed, specify how its required identity/metadata fields are represented before validation. If authoritative metadata cannot be read, return unavailable rather than invent a state. Keep per-invocation host denial separate and apply disclosure admission before revealing any of this.

### CS-A03 — Keep stale, unknown and not-required evidence distinct

**Defect:** evidence:77 maps every stale required check to global `reauthorization_required`, even when the stale check concerns only one operation. The public example includes unrelated denied permissions and a not-run verification in one global state. `collected_unix_ms` does not establish per-check freshness (47).

**Required rule:** only a fresh, correctly bound applicable check may support success. Unknown/unavailable/uncertain/not_run never becomes ok. A check that the selected profile does not require need not run and must not poison readiness. An optional operation's stale permission/verification refuses only that operation; stale identity/credential or other mandatory connection-wide checks can block all business use. Preserve F05 generation/subject binding and original deadlines; description is metadata-only and cannot silently run introspection, identity, permission or verification calls. Distinguish positively missing scope from unknown grants, and metadata outage from invalid credential. An unavailable observation does not prove reauthorization will fix it.

### CS-A04 — Define minimum acquisition grants versus requested and operation grants

**Defect:** profile:49 declares requestable/minimum scopes, while acquisition:70/114 says validate scope generically; acquisition:139 rejects fewer-than-minimum grants. No rule says whether an ungranted optional requested scope fails completion or poisons the whole connection. Narrowed refresh grants must still support unaffected reads (evidence:139).

**Required rule:** requestable scopes bound what may be requested, minimum scopes are the profile's baseline publication requirement, and operation `requires_auth` scopes are per-operation requirements. Requested access is not granted access or host authority. State whether missing optional requested scopes permits completion; the readiness objective supports completion when minimum grants and other baseline checks pass, recording actual grants and refusing only affected operations. Refresh may publish valid narrowed optional grants; a candidate missing minimum grants cannot publish. If its source was already consumed by refresh, retain F01/F05's blocked/repair outcome rather than reusing the old source. A failed independent repair may retain the old binding but cannot promise that independently invalidated credentials remain usable. Unknown/omitted grants require the profile's explicit preservation proof or refusal, never an assumed empty/sufficient set.

### CS-A05 — Classify verification requirements and completion outcomes

**Defect:** `AuthProfile.evidence` lists supported checks (profile:53/67; evidence:166) without selecting which are global versus operation/target requirements. Evidence:175 runs verification once at completion but says failure does not delete credentials; acquisition's completed status is otherwise easy to read as unconditional readiness. Verification is invalidated on refresh (evidence:116), so treating it as global can disable unrelated operations after every successful refresh.

**Required rule:** identify which checks each profile requires for initial publication/global viability and which apply only to a named operation or exact target. A supported hook is not automatically mandatory for every operation. Define whether completed means durable acquisition/publication with a separately non-ready status, or whether a mandatory verification failure prevents publication; reconcile both documents. Optional verification failure must not prevent baseline publication or erase valid material. A required operation verification that is not_run blocks that operation until separately admitted verification; a universal prerequisite must be explicitly declared as such. Keep verification credential use, effects, freshness and deadline inside its separately admitted boundary.

### CS-A06 — Use one vocabulary without conflating result classes

**Defect:** acquisition failure payload and scenario use `scope_insufficient` (47,139), while refresh, connection and operation refusal use `insufficient_scope` (61,100; connection:58; evidence:132). Connection only lists `connection_not_ready` and route_unavailable, yet evidence/profile require operation-local insufficient_scope.

**Required rule:** use `insufficient_scope` for that same failure throughout. Keep generic evidence `result: insufficient`, acquisition payload reason, private refresh result, global state (if retained for minimum-grant failure), and public ErrorCode explicitly distinct. Publish the state-to-business-refusal mapping including pending, disabled, revoked, custody failure and parent degradation. Operation-local permission denial is not global scope failure; a known missing grant is not custody_unavailable. Use the existing E02 code set and selected payload schema rather than inventing another error string or tunneling data through message text.

### CS-A07 — Management must work when business readiness fails

**Defect:** connection:64/141 calls all management ordinary adapter operations, although host metadata/coordinator ownership is explicit at123 and design:527/551/576. Applying connection:74's blanket ready requirement to these operations makes a broken connection impossible to inspect, repair or revoke. `auth.begin` creates state and client-credentials begin can immediately exchange (acquisition:67–73,122), so “safe management” cannot mean read-only.

**Required rule:** host orchestrates admission, scoped metadata, acquisition state, publication and local revocation; provider hooks own protocol request construction/interpretation, with custody owning sensitive versions. List/describe/status need current management disclosure and metadata, not provider readiness. Repair/begin/complete need their declared coordinator/registration/custody prerequisites, not a usable target business credential. Local revoke must commit its terminal cutoff even if vendor revocation is unsupported, unreachable or uncertain; report provider cleanup separately and never restore local authority. Declare management operation curation/admission according to actual effects. Begin/list may have no existing target connection: do not fabricate one or accidentally select a configured business connection to satisfy an ordinary invocation path. If required approval is selected for such a mutation, its canonical target needs an explicit binding; F03's existing connection-bound subject is not a generic placeholder.

### CS-A08 — Preserve the selected federation and model boundaries

**Defect:** management ownership ambiguity affects routing a repair/status request to the correct leaf and risks interpreting callback/provider functions as ordinary forwardable operations. Prose future-model tables claim Connection/AuthProfile/Acquisition lifecycles/relations that are not the implemented ESS declarations.

**Required rule:** admitted safe management uses one configured owner route, current gateway and leaf policy, unchanged ownership of acquisition/connection handles, and no reroute/resend after uncertainty. Completion reaches the owning coordinator via its protected callback/entry binding, never generic `auth.complete` invocation; descriptor discovery exposes neither callback authority nor registration secrets. A dedicated public HTTP route is not required merely by design§16.1. Keep F03 preparation metadata-only when the target is not business-ready. New readiness/admission records should be typed values over existing bindings, not fake persistent Connection/Acquisition entities or commands that pretend to validate current policy. Existing ESS EvidenceCheck already binds generation, subject and deadlines; DispatchAdmission already names the trusted transient decision. Preserve those owners, explicitly mark new cross-value predicates UNMAPPED, and correct future-model wording instead of claiming missing entities/cardinalities already exist.

## Minimum textual decision matrix

| Case | Required observation |
|---|---|
| Baseline read grants present, optional write grant absent | Global viability unchanged; read eligible, write insufficient_scope. |
| Request optional write grant, provider returns baseline only | Explicit chosen completion/publication rule; no implied write grant. |
| Initial candidate below profile minimum | Canonical insufficient_scope failure; no active candidate publication. |
| Refresh removes optional write grant | Valid new baseline generation may publish; fresh read admission succeeds, write refuses. |
| Consumed refresh source yields below-minimum/unknown grants | Refuse candidate; old consumed source does not regain dispatch/refresh authority. |
| Permission denied for one namespace/target | Refuse/report that target under its contract; no global reconnect classification. |
| Optional check not_run; required operation check stale | Ignore the non-required check; refuse only the operation requiring the stale check. |
| Global identity/credential evidence stale or mismatched generation | No business dispatch; no implicit provider probe during description/invoke. |
| Local revoked plus parent failure/custody outage | Revoked remains terminal; safe management status still reports it under admission. |
| Disabled plus successful provider evidence | Disabled remains non-callable; explicitly admitted control/inspection still possible. |
| Acquisition pending with no published connection | No fabricated completed connection reference or ready status. |
| Metadata unavailable | No invented positive, pending or reconnect state from absence of evidence. |
| Custody outage during describe/repair/revoke | Metadata-only inspection and local revocation retain their independent prerequisites; provider use remains blocked. |
| Required/optional verification fails at acquisition completion | One documented publication/completed/status consequence across acquisition/evidence/connection. |
| Host denies access despite provider permission success | Safe host refusal first; no readiness or scope disclosure bypass. |
| Federated begin/status/complete/revoke | Every step names its owning host; private evidence stays on protected completion/provider ports. |

The acquisition-profile specifics, permission fan-out budgets, read-refresh retry and persistence backends have separate owners. This report requires their existing boundaries to remain intact and does not select new provider behavior or implementation work.
