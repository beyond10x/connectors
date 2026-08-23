---
id: S-048
title: "A companion reply is claimed exactly once, locally too"
pillar: Platform
status: backlog
design: ../design/13-grant-evaluation-and-approval-redemption.md
epic: enforced-authority
areas: [integrations, state, config]
---

# A companion reply is claimed exactly once, locally too

## Goal

S-047 deleted the Slack backend's `ReplyClaimStore` with the approval-presence check it rode on,
so on the local placement nothing enforces one reply per companion mention any more: a retried
invoke can post the same Slack reply twice (S-047 review, 2026-08-23). Restore the exactly-once
claim as an honest local mechanism — a durable one-time claim keyed on the triggering event,
made at the dispatch seam rather than by the Integration re-reading `approval_evidence_ref`
(which catalog invariant rule 15 now forbids; its named allowlist
`APPROVAL_EVIDENCE_AUDIT_CORRELATION_USES` is the sanctioned escape hatch if correlation by that
field turns out to be the right shape).

## Acceptance

- On the local placement, two presentations of the same companion event produce exactly one
  outward Slack reply; the second refuses and is journaled, proven by a real-concurrency test.
- Catalog invariant rule 15 stays green without weakening its empty-by-default posture.
- The SIP `approval_evidence_ref` config field, which no code consumes since S-047, is retired
  from `connectors-config` (`personal.rs`), the examples, and the connectors-cli README — or its
  retention is justified in the config docs.

## Progress

- 2026-08-23 — filed from the S-047 review findings (local one-shot regression, dead config
  field).
- 2026-08-23 — implemented on `impl/S-048`. The claim lives at the local dispatch seam:
  `EventReplyClaims` (`crates/connectors-runtime/src/claims.rs`) spends an `event:` evidence
  reference exactly once over the S-041 state port, and `BackendRegistry::with_event_reply_claims`
  applies it in the Invoke branch before any Integration is reached, only when the resolved
  description demands approval. The personal composition binds it over a `SqliteState` at
  `<state_root>/event-reply-claims.sqlite` from first boot; the hosted registry stays claim-free
  because its `ApprovalGate` redeems upstream. A refused replay journals under
  `local.event-reply-refusals` and returns `ApprovalDenied`; a claim that cannot be made durable
  refuses rather than replies. Real-concurrency proof:
  `registry::tests::a_companion_reply_is_claimed_exactly_once_locally` (two OS threads, one
  barrier, exactly one dispatch) plus `claims::tests::parallel_presentations_take_exactly_one_claim`
  (8 threads) and a restart-durability test. Rule 15 untouched and green, allowlist still empty.
  The dead SIP `approval_evidence_ref` config field is retired from `ConnectionConfig`, both
  examples, the sip test fixture, and the connectors-cli README. Known-red left in place:
  `architecture_fence::product_cli_is_a_thin_frontend` fails at the merge base (856 > 828 CLI
  production lines) independently of this story.
