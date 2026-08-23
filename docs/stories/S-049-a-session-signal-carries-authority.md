---
id: S-049
title: "A session signal carries authority"
pillar: Platform
status: done
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
- 2026-08-23 — implemented on `impl/S-049` as the session-scoped admission (design 13,
  amendment of this date): the hosted route resolves the session's own operation and Connection,
  re-describes, and runs the `GrantEvaluator` (`HostedAuthority::admit_signal`,
  `crates/server/src/hosted/enforcement.rs`); the proof is consumed by value at
  `dispatch_admitted_signal` (`crates/server/src/hosted.rs`), which cross-checks it against the
  exact session. Refusals render byte-identically to the invoke path's 403 — including an
  unknown `execution_ref`, so the seam is no existence oracle — and every decision lands in the
  `signal.audit` journal before dispatch. No approval redemption per signal (establishment spent
  it; the Grant is the continuing authority) and no read path (a signal is always
  effect-bearing). Route tests: `crates/server/src/hosted/tests/signal.rs`. Catalogue invariant
  rule 16 (`a_session_signal_reaches_a_backend_only_through_the_admission_seam`) pins the
  request grammar as a closed set and the seam's presence; demonstrated red against both a
  grammar extension and a deleted seam. The `ApprovalGate::recover` single-replica assumption is
  stated in design 13's second amendment rather than fenced.
- 2026-08-23 — review rework: rule 16's grammar parser is now closed over line shapes (a
  struct- or unit-shaped variant is parsed, an unrecognized line inside the enum fails rather
  than skips; demonstrated red against `SessionBarge { … }`, `SessionBarge,` and a non-variant
  line). The seam scan cuts line comments before matching. A signal addressing a session no
  backend holds now journals its refusal too (empty operation/connection — the deployment holds
  no session to name), so ghost-ref spraying leaves a trail; design 13's amendment records that
  and the deliberate gap that revocation silences but cannot end an established session while
  hosted `SessionTerminate` stays 503-fenced.
- 2026-08-23 — merged to main as `b4fa118` after independent review (round 1 REWORK: the rule-16
  grammar parser silently skipped struct/unit variants; rework `397ff25` closes the parser over
  line shapes, journals ghost-ref refusals, strips comments from the seam scan, and completes the
  design-13 revocation-vs-termination sentence; re-review PASS, five empirical attacks all red).
