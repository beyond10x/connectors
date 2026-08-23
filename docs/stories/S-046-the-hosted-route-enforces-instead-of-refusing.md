---
id: S-046
title: "The hosted route enforces instead of refusing"
pillar: Platform
status: backlog
design: ../design/13-grant-evaluation-and-approval-redemption.md
epic: enforced-authority
areas: [server, service]
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
