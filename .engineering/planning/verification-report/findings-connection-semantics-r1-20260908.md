---
format: aep.planning-md/1
id: verification-report:findings-connection-semantics-r1-20260908
kind: verification-report
status: draft
title: Structured transcription of review-result:connection-semantics-r1-20260908
tags:
- review-findings-supplement
refs:
- provider: review-body-sha256
  reference: d407e8cd82f60b72ea37f6653e320f4e616b6921a1a34c3c38bdfb25890c0713
relations:
- verifies: review-result:connection-semantics-r1-20260908
- derived_from: specification:review-findings-toolchain-upgrade-20260911
revision: 1
---
# Structured transcription of review-result:connection-semantics-r1-20260908

This supplements [the immutable original](../review-result/connection-semantics-r1-20260908.md).
It records no new critic run, approval, review outcome or current implementation
finding. Source binding uses the SHA-256 of the exact original body returned by AEP.
The original review's scope, verdict, source snapshot and historical date remain authoritative.

## Original conclusion

The original report's verdict and scope remain authoritative; no new verdict is assigned.

## Transcription method

8 findings remain in the scope of this report's final stated conclusion.
Closed items in a same-pass disposition and verification/command tables are excluded.
Each nonempty message reproduces a source section or table row verbatim, including
its original identifier and citations. P0/P1, major and blocking map to blocker;
P2, minor and should-fix map to warning; P3 and nit map to note. Ungraded observations
remain unspecified. No per-finding verdict or introduced/pre-existing classification
is invented. The file is the first explicit source citation; when only shorthand
citations are present, legacy-source-excerpt identifies the original report itself.
The full citation context remains in the message and original. This transcription
makes no claim that paraphrased findings across different historical rounds have
identical comparison signatures.

## Findings

```findings
[
  {
    "file": ".engineering/planning/review-result/connection-semantics-r1-20260908.md",
    "category": "legacy-source-excerpt",
    "severity": "unspecified",
    "message": "### CS-A01 — Separate global viability from operation eligibility\n\n**Defect:** connection:58 makes any missing scope for an enabled operation a global `insufficient_scope`; connection:74 refuses all non-ready connections. Evidence:132 explicitly requires a read to proceed when write scope is absent. Evidence:43/49/74 repeats the conflation by deriving one global state from operation-specific checks.\n\n**Required rule:** derive connection viability from connection-wide requirements only. Derive eligibility for `(connection, operation, exact input/resource, caller, generation)` separately. An eligible invocation needs enabled implementation, current host admission, globally viable dependencies, one matching `requires_auth` alternative, and every required operation-specific check. Do not union grants across alternative profiles or connections. Missing optional write scope or denied resource permission leaves other eligible reads usable. Description and dispatch share the same reduction logic and subject, not an identical global boolean."
  },
  {
    "file": ".engineering/planning/review-result/connection-semantics-r1-20260908.md",
    "category": "legacy-source-excerpt",
    "severity": "unspecified",
    "message": "### CS-A02 — Fix lifecycle precedence and distinguish the two revocations\n\n**Defect:** connection's public states at 54–62 omit `pending`; its future-model lifecycle at132 includes pending but omits disabled. Parent degradation at82 can overwrite even a child's terminal status. Evidence alone cannot express administrative state.\n\n**Required rule:** publish one deterministic precedence table using authoritative connection facts plus viability evidence. At minimum, local terminal `revoked` outranks disabled and every evidence/parent overlay; disabled remains disabled despite successful evidence; parent degradation cannot resurrect or relabel a revoked child. Explicitly distinguish local connection revocation (terminal) from provider credential revocation (repairable `reauthorization_required`). Define pending only for a real unpublished/unvalidated connection binding. An acquisition in pending does not imply that a public Connection already exists; if pending Connection is exposed, specify how its required identity/metadata fields are represented before validation. If authoritative metadata cannot be read, return unavailable rather than invent a state. Keep per-invocation host denial separate and apply disclosure admission before revealing any of this."
  },
  {
    "file": ".engineering/planning/review-result/connection-semantics-r1-20260908.md",
    "category": "legacy-source-excerpt",
    "severity": "unspecified",
    "message": "### CS-A03 — Keep stale, unknown and not-required evidence distinct\n\n**Defect:** evidence:77 maps every stale required check to global `reauthorization_required`, even when the stale check concerns only one operation. The public example includes unrelated denied permissions and a not-run verification in one global state. `collected_unix_ms` does not establish per-check freshness (47).\n\n**Required rule:** only a fresh, correctly bound applicable check may support success. Unknown/unavailable/uncertain/not_run never becomes ok. A check that the selected profile does not require need not run and must not poison readiness. An optional operation's stale permission/verification refuses only that operation; stale identity/credential or other mandatory connection-wide checks can block all business use. Preserve F05 generation/subject binding and original deadlines; description is metadata-only and cannot silently run introspection, identity, permission or verification calls. Distinguish positively missing scope from unknown grants, and metadata outage from invalid credential. An unavailable observation does not prove reauthorization will fix it."
  },
  {
    "file": ".engineering/planning/review-result/connection-semantics-r1-20260908.md",
    "category": "legacy-source-excerpt",
    "severity": "unspecified",
    "message": "### CS-A04 — Define minimum acquisition grants versus requested and operation grants\n\n**Defect:** profile:49 declares requestable/minimum scopes, while acquisition:70/114 says validate scope generically; acquisition:139 rejects fewer-than-minimum grants. No rule says whether an ungranted optional requested scope fails completion or poisons the whole connection. Narrowed refresh grants must still support unaffected reads (evidence:139).\n\n**Required rule:** requestable scopes bound what may be requested, minimum scopes are the profile's baseline publication requirement, and operation `requires_auth` scopes are per-operation requirements. Requested access is not granted access or host authority. State whether missing optional requested scopes permits completion; the readiness objective supports completion when minimum grants and other baseline checks pass, recording actual grants and refusing only affected operations. Refresh may publish valid narrowed optional grants; a candidate missing minimum grants cannot publish. If its source was already consumed by refresh, retain F01/F05's blocked/repair outcome rather than reusing the old source. A failed independent repair may retain the old binding but cannot promise that independently invalidated credentials remain usable. Unknown/omitted grants require the profile's explicit preservation proof or refusal, never an assumed empty/sufficient set."
  },
  {
    "file": ".engineering/planning/review-result/connection-semantics-r1-20260908.md",
    "category": "legacy-source-excerpt",
    "severity": "unspecified",
    "message": "### CS-A05 — Classify verification requirements and completion outcomes\n\n**Defect:** `AuthProfile.evidence` lists supported checks (profile:53/67; evidence:166) without selecting which are global versus operation/target requirements. Evidence:175 runs verification once at completion but says failure does not delete credentials; acquisition's completed status is otherwise easy to read as unconditional readiness. Verification is invalidated on refresh (evidence:116), so treating it as global can disable unrelated operations after every successful refresh.\n\n**Required rule:** identify which checks each profile requires for initial publication/global viability and which apply only to a named operation or exact target. A supported hook is not automatically mandatory for every operation. Define whether completed means durable acquisition/publication with a separately non-ready status, or whether a mandatory verification failure prevents publication; reconcile both documents. Optional verification failure must not prevent baseline publication or erase valid material. A required operation verification that is not_run blocks that operation until separately admitted verification; a universal prerequisite must be explicitly declared as such. Keep verification credential use, effects, freshness and deadline inside its separately admitted boundary."
  },
  {
    "file": ".engineering/planning/review-result/connection-semantics-r1-20260908.md",
    "category": "legacy-source-excerpt",
    "severity": "unspecified",
    "message": "### CS-A06 — Use one vocabulary without conflating result classes\n\n**Defect:** acquisition failure payload and scenario use `scope_insufficient` (47,139), while refresh, connection and operation refusal use `insufficient_scope` (61,100; connection:58; evidence:132). Connection only lists `connection_not_ready` and route_unavailable, yet evidence/profile require operation-local insufficient_scope.\n\n**Required rule:** use `insufficient_scope` for that same failure throughout. Keep generic evidence `result: insufficient`, acquisition payload reason, private refresh result, global state (if retained for minimum-grant failure), and public ErrorCode explicitly distinct. Publish the state-to-business-refusal mapping including pending, disabled, revoked, custody failure and parent degradation. Operation-local permission denial is not global scope failure; a known missing grant is not custody_unavailable. Use the existing E02 code set and selected payload schema rather than inventing another error string or tunneling data through message text."
  },
  {
    "file": ".engineering/planning/review-result/connection-semantics-r1-20260908.md",
    "category": "legacy-source-excerpt",
    "severity": "unspecified",
    "message": "### CS-A07 — Management must work when business readiness fails\n\n**Defect:** connection:64/141 calls all management ordinary adapter operations, although host metadata/coordinator ownership is explicit at123 and design:527/551/576. Applying connection:74's blanket ready requirement to these operations makes a broken connection impossible to inspect, repair or revoke. `auth.begin` creates state and client-credentials begin can immediately exchange (acquisition:67–73,122), so “safe management” cannot mean read-only.\n\n**Required rule:** host orchestrates admission, scoped metadata, acquisition state, publication and local revocation; provider hooks own protocol request construction/interpretation, with custody owning sensitive versions. List/describe/status need current management disclosure and metadata, not provider readiness. Repair/begin/complete need their declared coordinator/registration/custody prerequisites, not a usable target business credential. Local revoke must commit its terminal cutoff even if vendor revocation is unsupported, unreachable or uncertain; report provider cleanup separately and never restore local authority. Declare management operation curation/admission according to actual effects. Begin/list may have no existing target connection: do not fabricate one or accidentally select a configured business connection to satisfy an ordinary invocation path. If required approval is selected for such a mutation, its canonical target needs an explicit binding; F03's existing connection-bound subject is not a generic placeholder."
  },
  {
    "file": ".engineering/planning/review-result/connection-semantics-r1-20260908.md",
    "category": "legacy-source-excerpt",
    "severity": "unspecified",
    "message": "### CS-A08 — Preserve the selected federation and model boundaries\n\n**Defect:** management ownership ambiguity affects routing a repair/status request to the correct leaf and risks interpreting callback/provider functions as ordinary forwardable operations. Prose future-model tables claim Connection/AuthProfile/Acquisition lifecycles/relations that are not the implemented ESS declarations.\n\n**Required rule:** admitted safe management uses one configured owner route, current gateway and leaf policy, unchanged ownership of acquisition/connection handles, and no reroute/resend after uncertainty. Completion reaches the owning coordinator via its protected callback/entry binding, never generic `auth.complete` invocation; descriptor discovery exposes neither callback authority nor registration secrets. A dedicated public HTTP route is not required merely by design§16.1. Keep F03 preparation metadata-only when the target is not business-ready. New readiness/admission records should be typed values over existing bindings, not fake persistent Connection/Acquisition entities or commands that pretend to validate current policy. Existing ESS EvidenceCheck already binds generation, subject and deadlines; DispatchAdmission already names the trusted transient decision. Preserve those owners, explicitly mark new cross-value predicates UNMAPPED, and correct future-model wording instead of claiming missing entities/cardinalities already exist."
  }
]
```

