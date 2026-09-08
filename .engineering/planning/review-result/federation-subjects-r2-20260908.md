---
format: aep.planning-md/1
id: review-result:federation-subjects-r2-20260908
kind: review-result
status: active
title: Independent F03 subjects review round 2
relations:
- reviews: story:contracts-federated-approval
revision: 1
---
# F03 independent semantic recheck A — round 2

Verdict: **ship the proposed specification with one verifier-rule fix**. There is **one remaining P1**, no P0. The draft resolves the original A01–A08 ownership and trace decisions. No runtime implementation is reviewed or approved by this verdict.

Reviewed baseline: `b5ace3f29d6deb1dbc9e5ddd50a8a8b02152cb2f`, with the uncommitted F03 draft frozen at 2026-09-08T15:36:43Z. `source-hashes-r2.json` records 23 exact inputs; `sources-r2/` contains matching immutable copies. All source references below refer to those copies. This independent review did not read reviewer B's findings and changed no source, planning, or runtime file.

## Remaining finding

### F03-A-R2-01 — P1 — Make maximum proof lifetime a receiver predicate

**Source:** `contracts/service/delegation.md:145`; related bounded approval policy at line 85 and receiver verification at lines 133, 141.

The issuer construction gives a delivery span of 30 seconds and an approval span of 300 seconds, but does not explicitly assign `iat = t`. The receiver's listed predicates are only `nbf <= lower`, `upper < exp`, and `nbf <= iat < exp`. They do not require the claimed interval to satisfy the selected profile's construction or maximum span.

A correctly signed token with `nbf = 995`, `iat = 1000`, and `exp = 1000000`, observed under `[lower, upper] = [998, 1002]`, satisfies every listed receiver predicate. It violates either selected issuer lifetime but could pass a verifier implemented directly from the receiver rules. A similarly malformed `iat` can be far in the future while still between nbf and exp. Signature validity establishes the configured issuer, not conformance to bounded claim semantics. This matters to short-lived delegated entry, approval freshness, key overlap and nonce capacity/retention assumptions.

**Required correction:** explicitly assign issuer `iat = t`; require receivers to validate the selected proof type's exact `nbf/iat/exp` relationship or its stated bounded equivalent before entry/spend. Retain checked arithmetic and conservative current-time checks. Do not rely on the issuer construction prose alone to reject an overlong signed proof.

**Verification:** add valid-boundary and correctly signed invalid vectors for overlong delivery, overlong approval, inconsistent iat/nbf, future iat with an otherwise currently valid interval, and expiry equality. Distinguish malformed lifetime from an ordinary valid profile token observed outside its interval.

## Original finding dispositions

| Initial finding | Disposition in this draft | Evidence |
|---|---|---|
| F03-A01: canonical leaf subject | Resolved. A closed subject binds stable leaf instance, resolved leaf operation/connection and semantic revisions, contract/profile, exact authority, trusted origin/route, canonical input digest and approval mode. Private provider material is excluded. | `contracts/service/delegation.md:23–64` |
| F03-A02: client/issuer visibility and managed selection | Resolved. A host-owned preparation read resolves both implicit configured and explicit managed connections. It returns the leaf subject plus separate presentation and issuer policy. Preparation grants no execution authority, lease, business reservation or spend. Issuer policy and authenticated delivery of the result are explicit. | `contracts/service/delegation.md:66–89` |
| F03-A03: revision/remapping semantics | Resolved. Leaf and route revisions are signed execution identity; unrelated gateway projection freshness is separate. Current submitted projection is checked. Required nullable route enters the versioned F02 fingerprint without creating a new origin namespace. Relevant changes invalidate a new approval and conflict with an existing live key. | `contracts/service/delegation.md:57–64`; `contracts/operations/v1alpha1/semantics.md` fingerprint section; `ess/domains/idempotency.yaml:33–50` |
| F03-A04: trusted identity and optional executor | Resolved. Gateway issuer, originating actor, approval issuer, executing leaf and current leaf policy are separate. Leaf identity projection is injective; explicit absence is preserved. Executor assertion must match independently admitted binding and cannot replace authentication context. | `contracts/service/delegation.md:13–19,51–55,97,125–133`; `ess/domains/idempotency.yaml:17`; `ess/domains/delegation.yaml:11–27` |
| F03-A05: one-time redemption owner | Resolved ownership/ordering. The logical executing leaf owns a shared durable `(approval issuer, reference)` spend authority. A receipt references exactly one Prepared AttemptRecord. Gateway does not spend; spend is separate from the dispatch gate; ambiguous acknowledgement cannot send. Add the remaining lifetime predicate above to the verifier rules. | `contracts/service/delegation.md:139–167`; `ess/domains/delegation.yaml:136–173` |
| F03-A06: delivery replay versus business replay | Resolved ownership/trace. Delivery and approval have distinct signed types/audiences. Exact request bytes, purpose, receiver and nonce are bound. A duplicate delivery never re-enters; a deliberate fresh delivery may observe an existing admitted key before revalidating spent/expired original approval. No automatic resign/resend. Add the remaining lifetime predicate above. | `contracts/service/delegation.md:93–155,161–182` |
| F03-A07: gateway failure observation | Resolved. Correlated leaf observations retain original attempt/request/effect/replay and leaf audit facts. Gateway audit failure cannot erase a known effect. Lost/corrupt/miscorrelated replies cannot manufacture non-dispatch or original identity, and cannot authorize reroute/resend. | `contracts/service/delegation.md:135,165–182` |
| F03-A08: ESS identity/value and relation ownership | Resolved within the declared model boundary. Preparation, subject, authority, route and proofs are values. Immutable delivery and redemption receipts have explicit identities and reference the existing executing configuration; redemption references the existing attempt. No duplicate business attempt or guessed ownership of issuer, identity, route or custody is introduced. Runtime uniqueness, cryptographic checks, fencing, retention and authority remain explicitly UNMAPPED. | `ess/domains/delegation.yaml:7–173`; `ess/domains/idempotency.yaml:8–50`; `contracts/service/delegation.md:184–190` |

## Preparation and compatibility checks

The helper has a selected `operations/v1alpha1` / `approval-subject` operation profile and closed input/result shapes, so it does not smuggle a new top-level wire envelope member into the E02 codec. Its host-reserved id rejects collisions. Legacy projection excludes it. Curation accounts for gateway control network traffic while forbidding provider I/O. Reading current connection metadata is compatible with auth.connection's existing metadata-only describe rule; no readiness probe or credential material resolution is authorized by preparation.

The dedicated 272 KiB request / 64 KiB result and 20/15/5-second ceilings are advertised separately from generic execution. The downstream-control interpretation is explicitly documented in the compatibility owner. Nested target size validation prevents the helper wrapper from silently relaxing the target mutation's own bound. Scope, revisions, selected managed connection and executor are revalidated at invocation; the helper's success is not a durable promise.

The compatibility matrix, service proposal, operation contract, index, CLI and stack guidance consistently treat this as a selected future binding. They do not claim current core/host implementation, legacy HMAC compatibility, general connection management completion, multi-hop authority, or external rollout.

## Evidence still required before final acceptance

The task handoff says F03 verification vectors are not yet assembled. This report therefore does not treat the proposal's fixture list or successful ESS declaration validation as verifier, persistence, clock, gateway or provider execution evidence.

The existing six idempotency scenario files in this snapshot still contain fingerprint objects without the newly required nullable `route` coordinate. Reconcile them with the normative `mutation-request/v2` representation while assembling the vectors; use explicit `route: null` for direct origin and structured non-null route for federation. This is known pending evidence work, not evidence of an implemented regression.

Final recheck should inspect canonical subject equality/difference vectors for both connection modes and harmless gateway refresh versus relevant remapping; absent realm/executor; direct/different gateway origin; request/type/algorithm/audience/key/body substitution; the lifetime cases above and nonce retirement; duplicate/ambiguous nonce versus approval spend; exact-key observation with an expired original approval; and definite versus missing/corrupt gateway observations around the leaf gate. Label authored model/scenario and cryptographic vectors separately from runtime conformance.

No new source-level defect was found in the chosen preparation semantics or receipt/attempt ownership beyond F03-A-R2-01.
