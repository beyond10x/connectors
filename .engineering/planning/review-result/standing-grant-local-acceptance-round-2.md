---
format: aep.planning-md/3
id: review-result:standing-grant-local-acceptance-round-2
kind: review-result
status: active
title: Acceptance critic, round 2, story:local-invoke-admits-a-keyed-write-under-a-standing-grant and story:standing-grant-closes-the-input-to-listed-members
relations:
- reviews: story:local-invoke-admits-a-keyed-write-under-a-standing-grant
- reviews: story:standing-grant-closes-the-input-to-listed-members
revision: 1
---
verdict: needs-revision

```findings
- file: .engineering/planning/story/local-invoke-admits-a-keyed-write-under-a-standing-grant.md
  line: 387
  category: acceptance
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "The Outcome's promise (a call the grant does not cover makes no provider request and no attempt) has no CI line: lines 1-19 never reach `execute` (needs a live Child, `resolve` needs descriptor snapshot, connection, policy, key store; 'No test calls execute in process'), and the journey is Deferred, so 'the proof in CI is lines 1-19' overstates. Either add `a_refusal_under_the_grant_path_returns_before_the_audit_anchor_and_any_child_call` (the grant check is one function called first in the closure, taking the child lazily or a fake child as `runtime/process.rs:1392-1451` builds, asserting no anchor, no attempt row, no use counted, `approval_required`/`not_attempted`, use released on failed preflight/prepare/bind) or write: 'CI does not prove: refusal before any request, no attempt row, refusal codes of the owner, replay before the grant check (ADR decision 3), use release, rechecks, `open_dispatch` with not_required, grant id bound to the attempt, the CLI gate admitting a keyed write; only the journey proves them, on the workstation'."
- file: .engineering/planning/story/local-invoke-admits-a-keyed-write-under-a-standing-grant.md
  line: 303
  category: acceptance
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "`-E 'test(standing)'` matches the test name only, so names 1-13 (no 'standing' in them) run only if they sit in `local::standing::tests`; it also selects 12 existing tests (8 in `local::standing::tests`, 4 in `http_standing_grant.rs`) and exits 0 when a named test is missing. Name the module of each test (tests 1-8, 11-13 in `local::standing::tests`, 9-10 and 15 where they live) and add 'and `cargo nextest list -p connectors-host -E 'test(standing)'` prints the 15 names'."
- file: .engineering/planning/story/local-invoke-admits-a-keyed-write-under-a-standing-grant.md
  line: 305
  category: acceptance
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "The seam is unnamed. `signer` (:109) and `fixture` (:150) are private to `approvals::tests`, and `with_signer` needs the keyring. Add: 'the grant file's sign, verify, grant, revoke and list are functions of (state directory, `&Signer`, `&ConfiguredApprovalKey`, clock interval) in `host/standing.rs`; `signer()` is `pub(crate)` under `cfg(test)` or the RFC 8032 seed is repeated; `fixture` is not used (bind takes any Uuid)'. Tests 9 and 10 sit inside `execute` (:375, :440, :502) and are reachable only with the function named above."
- file: .engineering/planning/story/local-invoke-admits-a-keyed-write-under-a-standing-grant.md
  line: 313
  category: acceptance
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "Lines 4, 9 and 10 have no control, so a recheck or file check that always refuses passes them; start each with 'after the control (the same grant admitted)'. Line 8 tests the guard on `Uses` only; add 'a refusal, a failed preflight, a failed `prepare` and a failed `bind` each leave the next take admitted at max_per_hour 1, and an `unknown` outcome leaves a fresh key `rate_limited`' (round 1, finding 7, open)."
- file: .engineering/planning/story/local-invoke-admits-a-keyed-write-under-a-standing-grant.md
  line: 366
  category: acceptance
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "The journey cannot use `Slack`, `clock()`, `carries_token`, `refused` or `posts()`: they are private to `slack_approval.rs`, which line 24 forbids changing. State which: copy them into `slack_standing.rs`, or drop `slack_approval.rs` from line 24 and make them `pub(super)`. The total request count (`requests`, auth.test included) and the ledger attempt count (rusqlite on `metadata.sqlite3`) need a named helper. Step g must name the refusal code ('today's refusal' is no observable). Step b must name `.grants[0].grant_id`; `not_after_unix_ms` must come from the fixture clock."
- file: .engineering/planning/story/local-invoke-admits-a-keyed-write-under-a-standing-grant.md
  line: 366
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "Round 1, finding 6 stays open: no step replays key c after `not_after` or at the cap (only after revocation, step i). Add 'with the clock past not_after (and at the cap) the key of step c still replays with the grant id'."
- file: .engineering/planning/story/local-invoke-admits-a-keyed-write-under-a-standing-grant.md
  line: 354
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "Line 19 names no command; the rewriter tests are unit tests in `apps/connectors/src/local/migrate.rs` and `select.rs`, outside `--test standing_grant --test invoke_short`. Add 'run by `cargo nextest run -p connectors -E 'test(keeps_standing_grants)'`, which lists two tests'."
- file: .engineering/planning/story/local-invoke-admits-a-keyed-write-under-a-standing-grant.md
  line: 352
  category: acceptance
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "'no owner is started' has no observer, and 'a key' is ambiguous. Say: 'with `--idempotency-key k` and an empty `--approval-file`, stderr equals the bytes pinned at `invoke_short.rs:319-342` (stage admission, next_action none, approval_steps) and the state directory holds no owner socket or lock file (name it)'. Today the owner answers with other bytes, so the line fails before the change."
- file: .engineering/planning/story/local-invoke-admits-a-keyed-write-under-a-standing-grant.md
  line: 347
  category: acceptance
  severity: note
  verdict: needs-revision
  origin: introduced
  message: "State what no CI line proves of the commands: the CLI wiring of `grant-standing` (arguments, `{grant_id, path}` output, real keyring signing) is proven only by journey step b."
- file: .engineering/planning/story/standing-grant-closes-the-input-to-listed-members.md
  line: 110
  category: acceptance
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "A scalar root can never return `OutsideInput`: `input.pointer(target)` is None first, so `covering` returns `MissingMember`. Change to 'a scalar root: `Err(Uncovered::MissingMember)`' and drop 'A root that is a leaf is not covered' from the Design. Add 'a member `/body/text2` beside listed `/body/text` returns `OutsideInput`' (a prefix test on the string, not the token, would admit it, and line 4 checks parse only)."
- file: .engineering/planning/story/standing-grant-closes-the-input-to-listed-members.md
  line: 121
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "Line 4 lacks: an `input_keys` pointer equal to a target pointer, `~2` (bad escape), and 17 pointers counted over `input_keys` alone (say so). Line 3 or 1 lacks two grants in file order, the first outside the input, the second covering (admitted by the second)."
- file: .engineering/planning/story/standing-grant-closes-the-input-to-listed-members.md
  line: 130
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "'pass with no change to their grant files' is no command; the first command also runs 36 tests while `test(standing::)` selects only the 8 unit tests. Add 'git diff -U0 against the merge base shows only added lines in the three test files'."
```
