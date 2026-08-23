---
id: S-045
title: "An approval is redeemed exactly once"
pillar: Platform
status: ready
priority: 2
design: ../design/13-grant-evaluation-and-approval-redemption.md
epic: enforced-authority
areas: [domain, service, state, audit]
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
