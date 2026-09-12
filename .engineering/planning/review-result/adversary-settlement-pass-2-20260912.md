---
format: aep.planning-md/1
id: review-result:adversary-settlement-pass-2-20260912
kind: review-result
status: active
title: Adversary pass 2 — terminal settlement CLI journey, unit 1
relations:
- reviews: story:guarded-gitlab-merge
revision: 1
---
```
unit: 1
verdict: red
cases: executed 18→20, red 1
origin: introduced 3, pre-existing 0, undecided 0
wrote-outside-worktree: 3 paths
needs-coordinator: yes
```

## 1. `git --no-pager diff --stat`

```
 .engineering/planning/journal.jsonl                |   5 +
 adapters/gitlab/tests/local_runtime.rs             | 226 ++++++++++++++-
 .../gitlab/tests/local_runtime/guarded_merge.rs    | 314 ++++++++++++++++++++-
 docs/gitlab-write-failure-matrix.md                |   2 +-
 4 files changed, 541 insertions(+), 6 deletions(-)
```

**Two non-test paths are in that diff and neither is mine.** `.engineering/planning/journal.jsonl` and `docs/gitlab-write-failure-matrix.md` were both already modified when pass 2 started — the coordinator's own pass-1 record and its row rewrite. I wrote nothing under `.engineering/planning/` and did not touch the matrix. `git --no-pager diff --stat -- crates/` is **empty**; no implementation file was edited and no mutant was ever placed in the tree. My additions this round are confined to `local_runtime.rs` (one appended `#[test]`) and `guarded_merge.rs` (an `11 => 5` mode arm, an `#[ignore]` wrapper, and the `if mode == 11` branch at `:525-604`).

## 2. Cases I added — one red, run alone before the suite

**`the_disclosure_row_records_the_post_effect_revocation_journey_that_exists`** (`local_runtime.rs:676`), red now:

```
test the_disclosure_row_records_the_post_effect_revocation_journey_that_exists ... FAILED
panicked at adapters/gitlab/tests/local_runtime.rs:676:5:
assertion `left != right` failed: guarded_merge.rs now runs a connection race after the native response with effect counts and a safe response check, and this row's remaining column is byte-unchanged; its existing-evidence column still names only preflight revocation and background recovery: CLI revocation during preflight completes the admitted audit and refuses dispatch. Revoked/removed-target background recovery later refuses result disclosure.
  left: "Complete policy/key/connection races after native response and during final audit handling, with effect counts and safe response checks."
 right: "Complete policy/key/connection races after native response and during final audit handling, with effect counts and safe response checks."
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 19 filtered out; finished in 0.00s
```

**`adversary_revoked_disclosure_of_an_applied_merge_never_answers_not_attempted`** (journey mode 11, `guarded_merge.rs:525`) — **green**, and I am keeping it deliberately: it is the probe behind findings B and C and it makes both reproducible. Written before it was run; it passed alone (17.01 s) and in both full lanes. It asserts the three things mode 10 leaves unsaid: a refused disclosure never answers `not_attempted` for an applied merge, the durable record after restart is one of the two conservative pairs with no settlement or replay window, and the refused run starts no owner.

## 3. Suite runs

| lane | result | time |
|---|---|---|
| default | **5 passed, 1 failed, 14 ignored** | 18.91 s |
| `--ignored` run 1 | 14 passed, 0 failed | 577.57 s |
| `--ignored` run 2 | 14 passed, 0 failed | 554.60 s |
| settlement case (modes 8/9/10) x4 | ok, ok, ok, ok | 77.7 / 82.2 / 86.0 / 77.7 s |

The one failure is mine. **Both implementor figures confirmed**: default 5 passed (3 original + my two pass-1 cases, both now genuinely green), ignored 13 passed — 14 here because my probe is in that lane. Six executions of the mode-8/9/10 case showed no flakiness; the three-hold-plus-counter fixture and the concurrent revoke are stable.

## 4. Findings

| # | file:line | cat | sev | verdict | origin | what was measured | what reaches it |
|---|---|---|---|---|---|---|---|
| A | `docs/gitlab-write-failure-matrix.md:22` | contract-drift | warning | CONFIRMED | introduced | `local_runtime.rs:676`, exit 101 | the doc; the same class the coordinator already accepted on row 20 |
| B | `adapters/gitlab/tests/local_runtime/guarded_merge.rs:682` | mutant | warning | NEEDS-CHANGE | introduced | probe dump of the live response | `background_recovery.rs:157` is the in-repo idiom mode 10 does not use |
| C | `adapters/gitlab/tests/local_runtime/guarded_merge.rs:694` | acceptance | warning | NEEDS-CHANGE | introduced | 30 s poll after restart; `owner.sock` absent | the revoked restart path, every run |

**A.** Mode 10 revokes the connection after a known Applied provider effect and checks disclosure is refused with one PUT and one effect. That is not the Attempt terminal settlement row — it is a *connection race after the native response with effect counts and a safe response check*, which is verbatim what the **Final result disclosure** row lists as remaining. One row was rewritten when the evidence landed; this one is byte-unchanged, so it understates coverage and overstates what is left. Fix: move the post-effect revocation into that row's existing-evidence cell and narrow its remaining cell to the policy and key races and the final-audit-handling race, which mode 10 does not cover.

**B.** `refusal(output, "revoked")` at `:682` discards its return value, so mode 10 asserts nothing about the *shape* of the refusal. Measured shape: `{"code":"revoked","kind":"operational","next_action":"create_connection","stage":"admission"}` — correct today, no mutation projection, no `not_attempted` lie for a merge that applied. But a regression that started attaching a projection, a classification or the original attempt id to that error would pass mode 10 unchanged. The sibling revoked-disclosure journey already asserts exactly this at `background_recovery.rs:157` (`assert!(refused.get("mutation").is_none())`). Fix: bind the return value and make the same assertion.

**C.** The declared residual — "the abandoned attempt may later be quarantined by background maintenance, so assert counts, not record state, after restart" — rests on a mechanism that does not operate on this path. The revoked restart run is refused at admission and **starts no owner** (`owner.sock` absent, measured), so no sweep can run: the record sat at `("dispatching", "pending", None, None)` for the full 30 s I polled it. The restart-side state is fixed, not timing-dependent, and the safety property is deterministic either way — the refused disclosure must never settle the attempt or release its business key. My mode-11 probe asserts it and is green. Fix: fold that assertion into mode 10, or keep mode 11.

**Carried from pass 1: nothing.** All three pass-1 findings are closed, and the absence of their signatures from the block below is deliberate:

| pass-1 signature | state now |
|---|---|
| `docs/gitlab-write-failure-matrix.md:20` CONFIRMED/introduced | row rewritten; my case passes, and the new cells are accurate against the code |
| `guarded_merge.rs:546` INFEASIBLE/introduced | fixed at the fixture, not the break condition; my case was red against the unlatched tree and is green against this one, which is the before/after pair |
| `guarded_merge.rs:517` NEEDS-CHANGE/introduced | mode 10 added; it discriminates the `execution.rs:518` failure arm, not only its deletion |

## 5. Attacked, could not break

- **Were my cases weakened?** No. Both pass-1 cases are byte-identical to what I wrote — no assertion relaxed, no string loosened, no wait made unfailable. They pass because the fixture and the doc changed.
- **Is the latch ahead of the observable?** Yes, completely. `local_runtime.rs:112` latches, `:113` is the single `observed_methods` push, nothing branches or awaits between them, and `let mode = merge;` is the only consumer. No other journey depended on the PUT branch re-reading the mode live; the four remaining live reads (`:127`, `:144`, `:155`, `:164`) are release checks and must stay live.
- **Does `merge_held` race?** No. For mode 5 the effect increment precedes the hold increment, so `merge_held == 1` implies `merge_effects == 1`; for mode 6 it implies zero. A release landing between `fetch_add` and the first `while` check exits the loop and answers the *latched* mode, which is the correct answer. One PUT per journey, so the never-reset counter cannot be pre-satisfied; per-`Provider`, so it cannot leak.
- **Does mode 10 assert what its name claims?** Yes. The revoke lands on admission with metadata still writable, and nothing between `native.commit()` and `execution.rs:518` can produce `revoked`. It separates itself from journey 4's identical `revoked` code by `merge_effects == 1` and one PUT. Failure-arm mutants (fall into the Timeout arm; return a different code) all kill it, not only deletion.
- **Deadlock risk from revoking while the owner holds `control.lifecycle` across `native.commit()`** — the revoke is not queued behind that lock; six runs, no hang.
- **Flakiness** — two full ignored lanes and four extra runs of the mode-8/9/10 case, all green.

## 6. Paths written outside the worktree

- `~/.cache/cv2-wave-20260912/adversary-2/ignored-lane-1.log`
- `~/.cache/cv2-wave-20260912/adversary-2/ignored-lane-2.log`
- `~/.cache/cv2-wave-20260912/adversary-2/settlement-repeat.log`

(Pass 1's `~/.cache/cv2-wave-20260912/adversary-1/ignored-lane.log` still exists and was reported then. `$PWD/.local/tmp` is inside the worktree and git-ignored.)

```findings
- file: docs/gitlab-write-failure-matrix.md
  line: 22
  category: contract-drift
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: journey mode 10 supplies the connection race after the native response with effect counts and a safe response check that the Final result disclosure row lists as remaining, and that row is byte-unchanged while the Attempt terminal settlement row was rewritten for the same delivery.
- file: adapters/gitlab/tests/local_runtime/guarded_merge.rs
  line: 682
  category: mutant
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: mode 10 discards the refusal return value and asserts nothing about the shape of the revoked response, so a regression attaching a mutation projection or a classification to a refused disclosure of an applied merge would pass, although background_recovery.rs:157 already asserts that projection is absent.
- file: adapters/gitlab/tests/local_runtime/guarded_merge.rs
  line: 694
  category: acceptance
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: the restart-side record state was omitted because background maintenance might sweep the attempt, but the revoked restart run starts no owner so no sweep can run and the record stayed dispatching/pending for 30 s; the deterministic invariant that the refused disclosure neither settles the attempt nor releases its key goes unasserted.
```
