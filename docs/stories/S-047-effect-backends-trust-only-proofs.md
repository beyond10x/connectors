---
id: S-047
title: "Effect backends trust only proofs"
pillar: Platform
status: done
design: ../design/13-grant-evaluation-and-approval-redemption.md
epic: enforced-authority
areas: [integrations, testing]
note: "every Integration's local approval-presence check is deleted (kubernetes hosted+local, monitoring, jira, gitlab, sip, slack, b10x) — the S-043..S-046 proof chain is the only admission authority; InvokeAdmission::Proven now carries the AdmittedOperation and hosted dispatch consumes it by value; catalog_invariants rule 15 scans every integration crate for approval_evidence_ref reads with an empty named audit-correlation allowlist; Slack's event-authorized companion reply (ReplyClaimStore) went with its check — an event that should authorize a reply must be issued as an approval record; ApprovalGate::recover is wired into HostedRuntime startup; GrantEffect::ALL makes the worst-case facts exhaustive by construction"
---

# Effect backends trust only proofs

## Goal

Delete the per-integration approval-presence checks (gitlab, kubernetes, slack, sip, jira,
monitoring) — the proof chain upstream is the authority — and add the catalogue-wide invariant
test that no effect-bearing backend code path is reachable without an `AdmittedOperation` built
from proofs.

## Acceptance

- No integration reads `approval_evidence_ref` for admission; the field reaches backends only as
  audit correlation.
- One parameterised invariant in `catalog_invariants.rs` (house rule: one rule, all providers)
  fails if a future effect operation bypasses the proof chain.
- The workspace gate is green.

## Progress

- 2026-08-23: Done. Every Integration's local approval-presence check is deleted — kubernetes
  (hosted restart/status and the local service route), monitoring, jira, gitlab, sip (including
  the deployment-config evidence string, which must never act as a shared approval password),
  slack, and b10x's categorical write fence — because the S-043..S-046 proof chain upstream
  is the only admission authority. `InvokeAdmission::Proven` now carries the `AdmittedOperation`
  and the hosted route's dispatch seam (`dispatch_admitted`, `crates/server/src/hosted.rs`)
  consumes it by value, cross-checking the proof's operation and Connection against the dispatch.
  Slack's event-authorized companion reply (`ReplyClaimStore`, `authorize_companion_event_reply`)
  was that Integration's approval logic and went with its check: an event that should authorize a
  reply must be issued as an approval record, and the issuance surface is design 13's declared
  non-goal. Invariant 15 in `catalog_invariants.rs` scans every `crates/integration-*` crate's
  production sources for `approval_evidence_ref` reads (empty named audit-correlation allowlist;
  demonstrated red against a deliberately wired bypass). Also landed from the S-046 review:
  `ApprovalGate::recover` wired into `HostedRuntime::bind` (a journal that cannot be settled
  refuses to serve), and `GrantEffect::ALL` (exhaustive by construction) behind
  `undeclared_facts()`. Still open, deliberately: declared risk/effects/idempotency facts do not
  yet travel with the description, and `SessionSignal` remains outside the Grant evaluator —
  neither is reachable by this story's parameterised-over-operations invariant.
