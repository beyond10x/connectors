---
id: S-045
title: "An approval is redeemed exactly once"
pillar: Platform
status: done
priority: 2
design: ../design/13-grant-evaluation-and-approval-redemption.md
epic: enforced-authority
areas: [domain, service, state, audit]
note: "ApprovalGate in domain::approval — axis-free verification; the one-time claim is a bounded append to approval.redemption.<sha256(reference)> whose payload IS the attempted-audit row; replay is its own journal kind; ApprovalGate::recover is the documented startup crash scan. SQLite concurrency/replay/crash evidence in state-sqlite/tests/approval_gate.rs; S-046 wires the hosted route"
---

# An approval is redeemed exactly once

## Goal

Implement the approval verifier and redemption store: an externally issued approval record
(issuer, subject, operation, Connection, canonical input digest, expiry) is verified against the
invocation and atomically redeemed — the attempted-audit row and the redemption land in one
transaction before dispatch; the terminal outcome row follows after. A second presentation of the
same reference refuses and audits as replay.

## Acceptance

- Redemption is atomic under concurrent presentation: exactly one of N concurrent identical
  requests proceeds (test with real concurrency, not sequential simulation).
- A mismatched operation, Connection, digest or expired record refuses without naming the axis.
- Attempted audit exists for every dispatch attempt, including refusals and crashes between
  redemption and terminal write (proven by a kill-point test or documented crash-recovery scan).

## Superseded by

`story:an-approval-is-redeemed-exactly-once` in the AEP planning store, at
`.engineering/planning/story/an-approval-is-redeemed-exactly-once.md`, now carries this story. Migrated 2026-09-04 by the
`aep-planning:story-migration` skill; this file is kept as the source it was migrated from and is
not the place the rung is recorded any more. The artifact quotes this file's `## Acceptance`
verbatim and names it in its own `## Provenance`.
