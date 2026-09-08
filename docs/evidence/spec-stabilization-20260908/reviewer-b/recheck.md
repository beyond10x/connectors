# Independent session specification review B — recheck

Date: 2026-09-08. This is a separate follow-up to `review.md` (SHA-256 `4928e05df65e579118404a7b7460d25c03a434675c9a580c4856bddbe91a5e08`), whose frozen observations, hashes and adversarial inputs remain unchanged. Final reviewed source hashes are in `recheck-sources.json`. Scope is the F06/E20 session specification and cross-references; unrelated working changes are outside this verdict. No other review report was inspected. No tracked files, planning records, runtime code or worktrees were changed by this reviewer.

Verdict: **approve specification hardening**. All five first-pass findings are resolved. Residual findings: **0 blocker / 0 major / 0 minor / 0 nit**. This verdict is not runtime conformance approval.

## Disposition

| Source finding | Result | Rechecked evidence |
|---|---|---|
| B-S1: recognized terminal authority denial did not latch | Fixed | `ess/domains/sessions.yaml:139` declares deny_ready/deny_data/deny_renewal into Closing; each applicable command's non-allow outcome takes its denial transition. The first trusted denial fixes terminal time and deadlines. `expired-data-denial-cannot-renew.yaml` denies expiry in Ready and then fences a stale allow renewal with StateConflict. The unverified readiness and revoked renewal cases enter Closing immediately; later BeginClose bookkeeping cannot reset the deadline. |
| B-S2: continuity loss during Closing omitted | Fixed | `ess/domains/sessions.yaml:156` admits Closing -> Lost. `continuity-lost-during-closing.yaml` preserves Lost after a later local-release attempt. The contract separates actual owner/continuity loss from merely unconfirmed peer shutdown, retaining first-terminal reason semantics. Ordinary unresponsive peers can still produce Closed on confirmed local release. |
| B-S3: fixture final cutoff exceeded existing expiry | Fixed | Accepted BeginClose cutoff fields are now capped by the already-issued lease expiry as well as terminal + 2 s. Oversized candidate deadlines remaining on refused duplicate BeginClose calls never become accepted assignments. The independent static audit below checked all accepted fixture deadlines. |
| B-S4: compilation versus sequential trace execution | Fixed | `verification.md:11` and its model/limits section explicitly state author/synthesize do not execute sequential lifecycle traces and can accept an impossible trace. The manual/static audit is separately attributed; runtime cases remain 0. The CLI's billing/oracle-fixture targets are not presented as Connectors execution. |
| B-S5: readiness circular prerequisite wording | Fixed | `ess/domains/sessions.yaml:168` distinguishes completed negotiation/redemption prerequisites of EstablishReady from the already-Ready lifecycle prerequisite of PermitData/RenewDataLease. |

A small expiry-reason ambiguity introduced during the first fix was clarified before this verdict: `GateDecision.expired` means live data-lease expiry and selects `lease_expired`; expiry of an unredeemed establishment token instead takes BeginClose(reason=expired) before readiness. This distinction is explicit in the final model comment and session §4.1, consistent with the existing trigger table. No additional entity or wire field was introduced.

## Independent verification

I inspected the revised declaration, all final traces, corrected deadline fields, normative session text, verification limitations and the media/auth/adapter cross-reference scope. A separate ephemeral audit selected each declared command outcome from its guard, checked transition source/target state, checked wrong-state payloads, expected/forbidden events, successful readiness/data lease validity, accepted cutoff ceilings, cleanup by the first terminal + 5 s, and final view states. Result: **13 traces / 77 command acts passed; 0 audit failures**. Output is `recheck-trace-audit.json` beside this report. This is static inspection of declarations and supplied fixture facts, not execution of a session implementation, clock, authorization system or device.

Pinned ESS checks were repeated:

```text
.local/toolchains/ess/0.20.0/bin/ess specify validate --path ess
connectors v1 — 8 file(s), valid
exit: 0

.local/toolchains/ess/0.20.0/bin/ess verify conform synthesize --path ess --target ir --scenarios contracts/sessions/v1alpha1/scenarios --out .local/spec-stabilization-20260908/reviewer-b/recheck-synthesized.json
201 scenario(s) (13 authored), 0 refusal(s)
exit: 0
```

After these checks, only the explicit live-lease versus establishment-token clarification changed the reviewed session prose/model comments; no declaration or scenario bytes changed. I read both final passages and captured the final source hashes afterward.

The root agent reports the final repository gate exited 0 and owns its logs and final evidence append. I did not independently rerun that full gate. The verification file at this review snapshot still marks that final evidence append as pending; adding actual gate/recheck evidence does not change this semantic verdict. Timing enforcement, authority authenticity and replay handling, field assignment/first-terminal retention, resource accounting and physical queue/device cutoff remain explicit future runtime obligations. The repository gate still needs the separately documented session author/synthesis command because its existing collector does not include this new authored directory.
