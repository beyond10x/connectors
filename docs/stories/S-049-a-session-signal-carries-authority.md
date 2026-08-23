---
id: S-049
title: "A session signal carries authority"
pillar: Platform
status: backlog
design: ../design/13-grant-evaluation-and-approval-redemption.md
epic: enforced-authority
areas: [server, domain, service]
---

# A session signal carries authority

## Goal

`SessionSignal` dispatches effect-bearing input (a keypress cannot be undone) behind scope and
operator-group policy only — no `GrantEvaluator`, no proof (S-046 and S-047 reviews, 2026-08-23).
It escaped the enforced-authority epic because it carries no operation ref, description, or
Connection: nothing a `GrantRequest` can name. Give session signals a describable authority —
either a described session-signal operation per provider or a session-scoped admission derived
from the Grant that admitted the session's creation — and bring the dispatch behind it.

## Acceptance

- An effect-bearing session signal is refused unless a Grant admits it; refusals are axis-free
  and journaled, matching the invoke path's semantics.
- The catalogue invariant family covers the session-signal seam so a future signal route cannot
  bypass admission silently.
- Hosted single-replica assumption for `ApprovalGate::recover` is either stated in design 13 or
  replaced with a fencing mechanism (S-047 review open question rides along here with the
  session-lifecycle work).

## Progress

- 2026-08-23 — filed from the S-046/S-047 review findings (ungated signal dispatch, recover()
  multi-replica question).
