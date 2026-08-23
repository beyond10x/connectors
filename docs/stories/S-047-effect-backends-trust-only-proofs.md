---
id: S-047
title: "Effect backends trust only proofs"
pillar: Platform
status: backlog
design: ../design/13-grant-evaluation-and-approval-redemption.md
epic: enforced-authority
areas: [integrations, testing]
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
