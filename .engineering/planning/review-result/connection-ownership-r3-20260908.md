---
format: aep.planning-md/1
id: review-result:connection-ownership-r3-20260908
kind: review-result
status: active
title: Independent final connection ownership approval
relations:
- reviews: story:contracts-connection-readiness
- reviews: story:contracts-management-boundary
revision: 1
---
# Independent management/readiness recheck B — final confirmation

Verdict: **approve both semantic checkpoints**: E04 (`contracts-management-boundary`) and F07/E19/E21 (`contracts-connection-readiness`). Remaining findings: **0 blockers, 0 majors, 0 minors, 0 nits**. RCB-01–RCB-08 and RCBR-01–RCBR-06 are closed in the reviewed final scope.

This approves the selected specification and its bounded evidence. It does not approve runtime advertisement: current authority/clock checks, protected UI/callback codecs, complete management payload/effect/approval/idempotency bindings, durable Connection/Acquisition/AuthProfile models and publication/replica fencing remain explicit prerequisites.

## Source freeze and independence

`source-hashes-final.json` freezes 43 final normative/evidence files in `recheck-2-snapshot/`, including the two separate session outputs. All 43 working-file hashes matched at completion. The baseline remains 12c11f4b43cd6cf5ff67d3243017887b28788344. Initial/recheck snapshots remain unchanged. AEP story context is preserved at its initial baseline in `source-hashes-initial.json`; later workflow revisions/status bookkeeping are intentionally outside this final semantic freeze.

No source/planning files were edited, no repository gate or runtime tests were run, and no other reviewer report, external integration or additional agent was used. The only executed checks were offline schema/value and evidence-byte comparisons; manual decision assessment did not implement or run a readiness reducer.

## Closure of the last corrections

| Finding | Final disposition |
|---|---|
| RCBR-05, P2 — optional selector versus explicit null | Closed. Management §1 and E02 compatibility §4 require an exact nonempty Invocation.connection string for describe/revoke/repair. Instance/acquisition-scoped create/list/status omit the field and reject every supplied selector, including null. Contradictory duplicate body targets refuse. Protected completion has its own binding; nullable F03 signed claims do not change the Invocation field. Trace M04 matches these rules. |
| RCBR-06, P3 — repairing terminal revoked records | Closed. The management exemption now permits independently admitted inspection/local revoke of non-ready records, while repair is restricted to permitted non-revoked bindings. Terminal revocation cannot be repaired/re-enabled. Trace M03 agrees. |

RCBR-01–04 remain fixed: stale baseline validation yields pending rather than invented credential invalidity; operation-only failures do not poison global state; receiver grant refusal is not_granted while adapter/connection/resource and provider permission refusals use forbidden; absent requires_auth preserves the exact explicitly configured binding; and noninteractive client credentials do not require a UI continuation channel. The corrected inactive-repair-candidate rule preserves independently valid active material without weakening current-material replacement or refresh fences.

The original eight requirements are reflected consistently across management, connection, acquisition, evidence, profile, compatibility and service v1alpha2. Host orchestration/private provider mechanics/custody ownership are distinct. Management permission and target scope are independent of business readiness. One coordinator retains acquisition ownership through federation, checks current stored authority before exchange/publication, and receives completion through protected ingress. Ordinary outputs contain safe refs/action kinds, while actionable continuation authority is confined to the admitted confidential UI channel. Local revocation remains authoritative despite provider cleanup uncertainty. The ordered seven-state viability reduction is separate from exact-operation eligibility and profile minimum versus optional grants. ESS value types do not pretend to be persistent owner lifecycles or executable admission.

## Independent evidence assessment

The detailed observations are in `recheck-2-evidence-audit.json`.

- All 16 viability examples agree with the documented first-match priorities, including metadata outage without an invented state, revoked/disabled dominance, known credential failure, custody/parent precedence, unpublished/stale baseline and ready.
- All 18 eligibility examples agree with current authority/enablement, exact configured or alternative binding, global viability and operation scope/permission/verification rules. In particular, read-without-optional-write remains eligible and denied current grant precedes disclosure of revoked state.
- All seven management target values agree with the selected instance/connection/acquisition owners. They are semantic target values, not proof of wire parsing or permission.
- The 16 textual traces correctly cover publication, optional grant narrowing, failed independent repair, protected actions, absent/null selector refusal, retained federation owner, withdrawn management permission, local revoke versus uncertain cleanup and no automatic business replay.
- All 80 generated-schema expectations were independently reproduced, including the four intended structural/enum refusals. The generic schema also accepts the deliberately contradictory `{eligible:true,error:not_granted}` value as recorded; the semantic contract refuses it. Type acceptance does not establish admission or cross-value consistency.
- The two generated directories contain 176 byte-identical artifacts. All 13 retained copies match their source artifacts and manifest hashes. Generated optional fields use omission and semantic Timestamp values; they are not the public Unix-millisecond or required-null wire codec.
- The retained gate log records a completed passing gate, 50 existing passing Rust tests, Rust 1.88 checking, 11 valid ESS files/167 declarations and 222 compiled scenarios including 34 authored cases with zero refusals. The separate preserved session outputs contain 13 authored and 201 synthesized scenarios, with every authored case present in the synthesized set. These are compiler/existing-code observations, not execution of the new management/readiness semantics.

The remaining implementation and binding prerequisites are disclosed accurately and do not contradict the selected boundary. No additional concrete defect remains in these two story scopes.
