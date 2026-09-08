---
format: aep.planning-md/1
id: review-result:connection-semantics-r2-20260908
kind: review-result
status: active
title: Independent connection semantics review round 2
relations:
- reviews: story:contracts-connection-readiness
- reviews: story:contracts-management-boundary
revision: 1
---
# Connection semantics — independent reviewer A, draft recheck

**Current semantic verdict: no remaining defect found after the same-pass corrections. Final acceptance awaits the assembled evidence.** The first reviewed draft had three P2 contradictions, preserved below with its immutable source snapshot. All three corrections were subsequently read and verified. No other reviewer output was inspected; this reviewer changed no source/planning file and ran no runtime test.

Source provenance: baseline `12c11f4b43cd6cf5ff67d3243017887b28788344`. `source-hashes-r2.json` / `sources-r2/` preserve the initial 27-source draft. `source-hashes-r2-corrected.json` / `sources-r2-corrected/` preserve the 27 same-pass corrected sources. References in findings identify the initial snapshot; correction observations identify the corrected snapshot. Neither snapshot is replaced by subsequent author work.

## Findings in the initial draft and verified corrections

### CS-A-R2-01 — P2 — Surviving stale-evidence rule still poisons global state

**Initial source:** `contracts/auth/evidence/v1alpha1/semantics.md:77`. The surviving bullet unconditionally mapped an unrecollectable stale required check to `connection_not_ready` and global `reauthorization_required`. This contradicted the new adjacent line76 and connection§4.1: stale baseline evidence yields pending; stale operation-only permission refuses that operation as unavailable; stale operation verification leaves global state unchanged.

**Verified correction:** the bullet now dispatches to the applicable reduction and explicitly says staleness is not positive credential invalidity. Baseline pending, operation-local permission unavailable and operation-local verification connection_not_ready agree with the connection owner. Required re-collection remains separately admitted; no implicit provider probe is introduced.

### CS-A-R2-02 — P2 — Interactive UI prerequisite accidentally applies to client credentials

**Initial sources:** `contracts/auth/management.md:41` and acquisition flow step3 at71 versus acquisition:124. The new unconditional rule refused begin without a trusted continuation/UI channel even though the selected client-credentials flow has no browser and may exchange/complete inside begin.

**Verified correction:** management and acquisition now scope continuation delivery, action kind and UI availability to flows actually needing browser/protected entry. Non-interactive client credentials use their admitted registration/provider/custody path and current authority, with no fabricated interactive action. The secret-free ordinary result rule and protected delivery requirement remain intact for interactive flows.

### CS-A-R2-03 — P2 — Inactive repair candidate mismatch still globally revokes readiness

**Initial source:** `contracts/auth/evidence/v1alpha1/semantics.md:88` versus connection:106/135 and management:20. The old statement classified every repair identity/binding mismatch as connection_not_ready plus reauthorization_required. That poisons an independently valid active generation when only a new inactive repair candidate is wrong.

**Verified correction:** evidence§4.1 now rejects an inactive candidate as identity_mismatch/safe binding refusal without replacing or poisoning a still-valid active generation. Detected substitution of material currently used by the connection still blocks dispatch and a proven identity/binding mismatch requires reauthorization. Independently current expiry, revocation and consumed-refresh facts continue to constrain the retained binding. This preserves both failed-repair isolation and F05's replacement fence.

## Initial requirement dispositions

| Requirement | Draft disposition |
|---|---|
| CS-A01 global viability versus exact eligibility | Resolved. Seven-state global reduction uses baseline requirements; current caller/operation/profile/grants/resource checks are separate. Known missing optional write scope no longer globally blocks a baseline read. |
| CS-A02 state/administrative precedence | Resolved. Metadata availability is checked first; local revoked precedes disabled, established failure, custody, parent, pending and ready. Provider credential revocation differs from terminal local revocation. Pending identity is explicitly null before establishment; acquisition pending need not expose a Connection. |
| CS-A03 freshness/applicability | Resolved after R2-01. Obsolete-generation positives and negatives are inapplicable; unknown/stale/not_run cannot become success. Observation/validity bounds are separate from since time and per-check deadlines. Description remains metadata-only. |
| CS-A04 acquisition/requested/operation grants | Resolved. Profile minimum is the fixed publication baseline; requestable bounds requests; optional requested grants may be omitted; actual grants control each operation. Baseline-invalid/unknown candidates cannot publish; consumed rotating sources cannot be restored. |
| CS-A05 verification/completion | Resolved. Authored evidence_requirements distinguishes supported, universal and operation-only checks. Mandatory F05 material/identity/validity and minimum grants cannot be disabled. Universal verification must pass before publication; optional verification can fail without blocking baseline publication. Completed records definite baseline publication, not permanent readiness. |
| CS-A06 vocabulary | Resolved. insufficient_scope is canonical acquisition/operation refusal, removed from global state. Current grant-policy denial uses not_granted; adapter/connection/resource restrictions and provider permission denial use forbidden. Unknown grants and unavailable permission evidence remain distinct from positively missing scopes. |
| CS-A07 management independence/owners | Resolved after R2-02/R2-03. Host dispatches management, provider hooks supply protocol behavior, and custody holds sensitive material. Metadata inspection/local revocation do not require provider readiness. Repair cannot restore a locally revoked binding. Provider revocation failure cannot undo local cutoff. |
| CS-A08 federation/model boundary | Resolved within story scope. Safe management retains owner/target identity and current policy at both hops; protected completion stays with its owning coordinator. No generic completion or secret-bearing action URL is exposed. Minimal ESS values reuse existing evidence/bindings and claim no persistent Connection/Acquisition/AuthProfile lifecycle or executed authority predicate. |

The corrected draft also preserves the E02 absent-requires_auth branch for an explicitly configured binding. It does not force every configured operation to invent a public profile alternative or turn omission into anonymous access. Instance/acquisition-scoped management can have no selected business connection; contradictory selectors refuse.

## Public and model consistency checked

The global state example now treats missing write scope and namespace denial as operation-local under an explicitly assumed valid baseline. Safe status adds separate observation/validity times and permits null unestablished identity; the compatibility owner records these payload changes and the authored evidence_requirements reader requirement. Neither adds fields to today's strict adapter schema or Descriptor.auth_profiles projection.

The new ESS domain contains status, trusted viability/eligibility and management-target values, with state and next-action enums. It does not synthesize persistent entities or commands to make the reduction look executed. Dependency/check applicability, exact subject/generation, alternative/scope matching, clock observations and co-presence are explicit normative predicates outside generic schema validation. Generic Optional omission remains distinct from required-null wire spelling.

The management document intentionally leaves exact effect/approval/idempotency/result and protected codec bindings unavailable until specified. This is consistent with E04's ownership scope, including its explicit refusal to fabricate an F03 instance-scoped creation approval subject. It is not a claim of an implemented or advertisable management API.

## Narrow final evidence check

Final evidence should exercise the selected precedence with overlapping faults, absent dependencies and no published credential; baseline versus optional scopes/verification; stale/unknown versus positive invalidity; inactive failed repair versus active substitution; current host denial and configured no-selector binding; interactive versus non-interactive begin; and local revocation with unsupported/failed/uncertain provider cleanup. Model value acceptance, authored arithmetic/truth tables and textual traces must remain distinct from runtime execution and from projected wire-codec conformance.

No further concrete semantic contradiction was found in this pass. Counts at the initial draft were P0 0 / P1 0 / P2 3; after verified corrections all are zero, with evidence review pending.
