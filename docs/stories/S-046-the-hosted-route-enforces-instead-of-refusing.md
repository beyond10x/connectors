---
id: S-046
title: "The hosted route enforces instead of refusing"
pillar: Platform
status: done
design: ../design/13-grant-evaluation-and-approval-redemption.md
epic: enforced-authority
areas: [server, service]
note: "server::hosted::HostedAuthority (GrantEvaluator + ApprovalGate over the one bound S-041 store) replaces both fences; described ReadOnly+NotRequired keeps the pre-existing policy path when no Grant admits; every enforcement 403 is one byte-identical axis-free body, replay distinct only in the journal; issued approval records live at approval.issued.<sha256(reference)> via the issue_approval bootstrap; the description carries no declared risk/effects/idempotency yet, so evaluation claims worst-case facts and exact allow exceptions are the admission shape until S-047 threads the proofs and facts onward"
---

# The hosted route enforces instead of refusing

## Goal

Replace the two hosted fences (approval-reference refusal; ReadOnly+NotRequired re-describe gate)
with the real gates: every Invoke passes the GrantEvaluator, and descriptions demanding approval
additionally pass the approval gate. Effect-bearing hosted invocation becomes reachable — only
through proofs.

## Acceptance

- A caller with a valid route-family token but no admitting Grant is refused (403) on a mutation.
- A granted mutation with a demanded approval refuses without one, dispatches with one, and
  refuses the same approval's second use.
- The pre-existing read-only path is unchanged for callers without Grants covering effects.
- End-to-end test drives the full chain over the hosted HTTP surface.

## Progress

- 2026-08-23: Done. Both fences in `crates/server/src/hosted.rs` are replaced by
  `server::hosted::HostedAuthority` (`crates/server/src/hosted/enforcement.rs`): every Invoke is
  re-described and evaluated by the S-044 `GrantEvaluator`; descriptions demanding approval
  additionally pass the S-045 `ApprovalGate`, whose one-time redemption carries the attempted
  audit row before dispatch and whose `conclude` writes the terminal row after. The
  `AdmittedOperation` proof is constructed only via `from_decision` against wall-clock time.
  Described `ReadOnly` + `NotRequired` invocations keep the pre-existing receiver-policy path
  when no Grant admits them. Route-level tests live in
  `crates/server/src/hosted/tests/enforcement.rs`. Left for S-047: thread the proofs into the
  effect backends, replace their local approval-presence checks, and carry the declared
  risk/effects/idempotency facts to the evaluation seam (until then evaluation claims worst-case
  facts, so selector grants admit hosted effects only when they admit everything — exact allow
  exceptions are the admission shape).
