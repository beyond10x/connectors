# Design — Grant evaluation and approval redemption are enforced authority

**Status:** draft (2026-08-23) · **Pillar:** Platform · **Backs:** S-043, S-044, S-045, S-046,
S-047 · **Epic:** `enforced-authority`

## Problem

The 2026-08-17 full-system architecture review's F-001 (critical) found that the Grant and
approval gates were representational: `AdmittedOperation::from_grant_decision` is a public
constructor over caller-supplied strings, and effect backends checked only the *presence* of an
approval reference before dispatching
([review, pinned](https://github.com/b10x/b10x/blob/1e0749233b711744b6e50f9106bba2c33dbbf396/architecture/docs/reviews/2026-08-17-full-system-architecture-review.md)).

The refusal half of the disposition has since landed: the hosted route refuses any
caller-supplied approval reference by name, and re-describes every invocation, refusing anything
that is not `ReadOnly` + `NotRequired` (`crates/server/src/hosted.rs`). That fence is honest but
it is a fence: every effect-bearing hosted operation — including the Kubernetes rollout-restart
the SRE deployment posture wants — is disabled until the constructive half exists.

This design is the constructive half. The domain model already states the invariants it must
satisfy (`01-domain-model.md`, Grant): admission chains through unconstructible proof types;
no store bound is an outage (503) and an empty store is a refusal (403); refusals never name the
axis that refused; deny beats allow beats predicate; grant mutation is CAS-revisioned.

## Shape

Two new authorities, both Connector-owned, both in front of every effect backend:

```text
Invoke ──▶ GrantEvaluator ──GrantDecision──▶ ApprovalGate ──ApprovalRedemption──▶ AdmittedOperation ──▶ backend
                 │ 403/503                        │ 403 / one-time
                 ▼                                ▼
             GrantStore                    RedemptionStore + attempted-audit row   (both via the S-041 state port)
```

1. **`GrantEvaluator`** (in `crates/domain`, storage behind the S-041 `StateStore` port).
   Evaluates the exact operation against the tenant's revisioned Grants using the declared
   selector semantics: risk ceiling, effects subset, idempotency class, explicit allow/deny
   exceptions with **deny > allow > predicate**, inbound events as closed sets. Its only success
   output is a **`GrantDecision`** — private fields, no public constructor, buildable only inside
   the evaluator module — carrying: issuer, tenant, `sub`, `act`, operation ref, Connection ref,
   catalog generation and description lease, grant revision, canonical input digest, decision,
   expiry, and a one-time id.
2. **Approval verification and redemption.** Operations whose description demands approval
   additionally need an **`ApprovalRedemption`**: the verifier checks an externally issued
   approval record (issuer, subject, operation, Connection, input digest, expiry) and **redeems
   it atomically** in the durable store — a second presentation of the same reference refuses.
   The attempted-audit row is written in the same transaction **before** dispatch; the terminal
   outcome row follows after.
3. **`AdmittedOperation` becomes reachable only through proofs.** The public
   `from_grant_decision(strings…)` constructor is removed. Hosted construction takes a
   `GrantDecision` (+ `ApprovalRedemption` where the description demands it). Personal-local
   placements — the owner speaking over their own 0700 socket — keep a separate, honestly named
   `local owner admission` path, so local single-owner use is neither blocked nor dressed up as
   a Grant decision.

## Refusal semantics

Straight from the domain model: no Grant store configured → 503; store empty or nothing admits →
403; the refusal text never says which axis (risk, effect, exception, expiry) refused. Approval
refusals are 403 except replay, which is its own audit event. Every refusal writes its audit row.

## Non-goals

Grant management UX and proposal/receipt CAS surfaces (`connectors.grants.manage` route family),
Identity-side changes, and F-002 (the browser capability) are out of scope. The existing hosted
fences stay in place until S-046 replaces them with enforcement; no story here widens exposure
before its gates exist.

## Stories

| Story | Slice | Depends on |
|---|---|---|
| S-043 | proof types sealed; `from_grant_decision` removed; local-owner path named; all callers moved; no behavior change | — |
| S-044 | `GrantEvaluator` with selector semantics over the state port; 403/503 semantics; unit + refusal tests | S-043 |
| S-045 | approval verifier + atomic one-time redemption + attempted/terminal audit transaction | S-043 |
| S-046 | hosted route replaces its two fences with evaluator + approval gate; effect-bearing invocation enabled behind them; end-to-end refusal/replay/success tests | S-044, S-045 |
| S-047 | integrations drop local approval-presence checks in favor of the proofs; catalogue invariant: no effect backend reachable without proof | S-046 |
