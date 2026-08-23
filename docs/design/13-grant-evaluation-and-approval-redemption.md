# Design — Grant evaluation and approval redemption are enforced authority

**Status:** draft (2026-08-23) · **Pillar:** Platform · **Backs:** S-043, S-044, S-045, S-046,
S-047, S-049 · **Epic:** `enforced-authority`

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
| S-049 | session signals admitted by the session's own Grant; signal admission journal; catalogue invariant: the request grammar is a pinned closed set | S-046 |

## Amendments

### 2026-08-23 — S-049: a session signal carries the session's authority

`SessionSignal` escaped the epic above because it carries no operation ref, description, or
Connection — nothing a `GrantRequest` can name — and so fell through the hosted route's
catch-all dispatch behind scope and operator-group policy alone (S-046/S-047 reviews). Of the
two shapes the story offered, the landed one is the **session-scoped admission derived from the
Grant that admitted the session's creation**, not a described session-signal operation per
provider: a signal reaches whatever the session is already connected to, so its authority *is*
the session's, and inventing a parallel per-provider operation would have given the same
authority a second name to drift under.

Mechanics, all in the hosted route (`crates/server/src/hosted.rs` +
`hosted/enforcement.rs::admit_signal`):

- The route resolves the session's own record (operation ref, Connection) through the
  Connector backend, re-describes that operation, and runs the S-044 `GrantEvaluator` over it —
  worst-case facts, exactly as every invocation until declared facts travel with the
  description. The proof is an `AdmittedOperation` via `from_decision`, consumed by value at
  the signal's own dispatch seam, which cross-checks the proof against the exact session.
- **No approval redemption per signal.** The one-time approval governs establishment and was
  spent by the invocation that created the session; the Grant is the session's continuing
  authority, and revoking it refuses the next signal. A per-keypress one-time record would make
  interacting with a live far end impossible.
- **No read path.** A signal is always effect-bearing; a Grant refusal never falls back to the
  receiver-policy path that serves described reads.
- **Refusals are axis-free and journaled.** Signal refusals render byte-identically to the
  invoke path's 403; a session nobody holds answers the same bytes, so the seam is not an
  existence oracle for execution refs. Every decision — admitted and refused — lands in the
  signal admission journal (`signal.audit` state cell, NDJSON rows) before dispatch; a journal
  that cannot take the row refuses as unavailable rather than act unaudited. The terminal
  "signal sent" record stays where it already was: the Integration's own audit journal, written
  on success.
- The catalogue invariant family gains rule 16: the operation-protocol request grammar is a
  pinned closed set and the hosted route must name every variant, so the next signal-shaped
  route variant fails red until its admission is decided on purpose.

### 2026-08-23 — `ApprovalGate::recover` assumes a single hosted replica

The S-047 review asked whether the startup recovery scan is safe under multiple replicas. It is
not, and the assumption is now stated rather than fenced: `recover` reads the whole journal and
appends settlement rows with no fencing token, so two replicas recovering concurrently can
double-settle the same anchors, and a replica recovering while another is serving can settle a
live attempted presentation as `aborted` while its owner goes on to redeem it. The hosted
placement therefore runs **exactly one replica**, and `HostedRuntime` runs recovery to
completion before serving (`crates/connectors-runtime/src/composition.rs`). This is an honest
deployment constraint, not a policy: a multi-replica hosted placement must bring a fencing
mechanism — a lease or generation token on the journal cell, or a backend with cross-key
transactions — with the story that introduces it. Until then, running a second replica against
the same store is operator error.
