---
format: aep.planning-md/1
id: review-result:adversary-settlement-pass-1-20260912
kind: review-result
status: active
title: Adversary pass 1 — terminal settlement CLI journey, unit 1
relations:
- reviews: story:guarded-gitlab-merge
revision: 1
---
```
unit: 1 — the uncommitted two-file change in the primary checkout, branch main, working tree over bfd01ae (nothing committed)
verdict: red
cases: executed 3→5, red 2
origin: introduced 3, pre-existing 0, undecided 0
wrote-outside-worktree: 1 path
needs-coordinator: yes
```

## 1. `git --no-pager diff --stat`

```
 adapters/gitlab/tests/local_runtime.rs             | 166 ++++++++++++++++++++-
 .../gitlab/tests/local_runtime/guarded_merge.rs    | 166 ++++++++++++++++++++-
 2 files changed, 328 insertions(+), 4 deletions(-)
```

Mine is the `local_runtime.rs` half (`+165/-1`); `guarded_merge.rs` is the unit's, untouched by me. Both paths are test files. `git --no-pager diff --stat -- crates/` is **empty** — no implementation file was edited, and no mutant was ever injected into the tree (I used a hand-built client inside a case I added instead).

## 2. Cases I added — both red, run alone before any suite run

Both in `adapters/gitlab/tests/local_runtime.rs`, non-ignored, default lane.

**a. `the_failure_matrix_row_stops_demanding_the_settlement_journey_that_exists`** (`local_runtime.rs:513`) — asserts the matrix row the unit says it delivers no longer lists the CLI evidence as remaining. Red now.

```
test the_failure_matrix_row_stops_demanding_the_settlement_journey_that_exists ... FAILED
panicked at adapters/gitlab/tests/local_runtime.rs:513:5:
the shipped CLI journey exists, and the row still lists it as remaining production CLI evidence: Fail settlement after a known native Applied/Refused response in the CLI; inspect the live response, restart observation, key state and exact provider effect count.
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 17 filtered out; finished in 0.00s
```

**b. `a_held_refusal_survives_the_release_the_settlement_journey_performs`** (`local_runtime.rs:619`) — replays the mode-9 break condition verbatim against the fixture, then does what the journey does next (release the hold), with the PUT body still in flight. Red now.

```
test a_held_refusal_survives_the_release_the_settlement_journey_performs ... FAILED
panicked at adapters/gitlab/tests/local_runtime.rs:619:5:
the held refusal was answered with the released mode instead: HTTP/1.1 200 fixture
Content-Length: 480
Connection: close

{...,"state":"merged",...}
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 17 filtered out; finished in 0.05s
```

## 3. Suite runs (after part 2 existed)

Default lane, all cases:
```
cargo test --locked --offline -p connectors-gitlab --test local_runtime -- --test-threads=1
test result: FAILED. 3 passed; 2 failed; 13 ignored; 0 measured; 0 filtered out; finished in 17.03s
```
Same lane with my two deselected by name (`--skip a_held_refusal_… --skip the_failure_matrix_row_…`), which is where `executed 3` comes from:
```
test result: ok. 3 passed; 0 failed; 13 ignored; 0 measured; 2 filtered out; finished in 16.90s
```
Ignored lane (the lane the work lives in; my cases are not `#[ignore]`, so it is unchanged by me):
```
CONNECTORS_TEST_CLI=…/target/debug/connectors cargo test … -- --ignored --test-threads=1
test result: ok. 13 passed; 0 failed; 0 ignored; 0 measured; 5 filtered out; finished in 533.85s
EXIT=0
```
The unit's own case alone: `…gitlab_cli_failed_settlement_keeps_the_known_effect_and_recovers_conservatively ... ok`, 64.06 s. **The implementor's green claim holds.**

## 4. Findings

| # | file:line | cat | sev | verdict | origin | what was measured | what reaches it |
|---|---|---|---|---|---|---|---|
| 1 | `docs/gitlab-write-failure-matrix.md:20` | contract-drift | warning | NEEDS-CHANGE | introduced | `local_runtime.rs:513`, exit 101 | the doc itself — the wave plan names this row as unit 1's deliverable; every other discharged row had both columns rewritten (e.g. "Native business response" now reads "Dedicated GitLab sandbox behavior remains required") |
| 2 | `adapters/gitlab/tests/local_runtime/guarded_merge.rs:546` | concurrency | warning | INFEASIBLE | introduced | `local_runtime.rs:619`, exit 101 | *nothing found* — see below |
| 3 | `adapters/gitlab/tests/local_runtime/guarded_merge.rs:517` | acceptance | warning | NEEDS-CHANGE | introduced | no case; read of `execution.rs:518` and of every journey's revoke site | `.engineering/planning/story/guarded-gitlab-merge.md:240` |

**1.** The row's "Existing evidence" cell still credits only mutation port tests and its "Remaining production CLI evidence" cell still demands exactly what `guarded_merge.rs:517-647` now does. The fix is one doc edit, and it should also name the injected cause (read-only metadata → `EACCES` on every read-write SQLite open), because that is the one persistence-failure shape the CLI journey covers — disk-full and `SQLITE_BUSY` fail at commit, not at open, and can leave the attempt `completed`, which is a different recovery path.

**2.** The mode-9 half releases its held refusal when `sent == 1 && merge_effects == 0`. `merge_effects` is zero before the request exists, so the whole condition is "a PUT appears in `methods`" — recorded at `local_runtime.rs:102`, before the body is read and before the merge branch latches `merge_mode` at `:127`. The release can therefore reach the handler before the hold does, and the fixture answers the *released* mode: 200 applied plus a real provider effect, in a journey that asserts `forbidden` and zero effects. **Mode 8 has no such hole** — `merge_effects` is incremented at `:138`, after the latch, so observing the effect proves the latch. What reaches it: I constructed the state by withholding the body; in the real journey reqwest sends headers and body together and the poll interval is 10 ms against a microsecond latch, and I did not see it fire in 2 real mode-9 executions. Hence `INFEASIBLE`, not `CONFIRMED`. Fix: break on a latch observable (a held-counter, or `merge_mode` having been read), not on `methods`.

**3.** The story's own next-case sentence names five inspections — "the live result, **final disclosure admission**, restart observation, retained key and exact provider effect count". The case does four; the matrix row lists only four and omits the same one. The unexercised code is `execution.rs:518`, the post-settlement `issuance::resolve`/`unchanged` block, whose failure arms no CLI journey reaches: `guarded_merge.rs:683` revokes while the **preflight** is held (the `initial` branch), and `background_recovery.rs:69` revokes in the observe path. Deleting that whole block leaves the new case green. I did not raise it as a blocker: a bare `Err` there omits the mutation projection rather than reporting `not_attempted`, which `contracts/service/local-mutations.md:209` permits, and the story itself says the conflict "must be resolved against the owning contracts". Coordinator call: extend the case, or correct the story sentence.

## 5. Attacked, could not break

- **Does the fault reach the settlement write** — yes. `mutations.rs:89` opens a fresh read-write `Metadata` connection per transaction, so `0400` fails at open; spend and gate are committed before the PUT the loop waits on, so `ledger.settle` (`execution.rs:499`) is the first write after the injection. `cause.stage == "attempt_store"` (`guarded_merge.rs:575`) is set only at `execution.rs:512` when `settled.is_err()`, so an audit-only failure could not pass this case.
- **PUT-count assertion** — present, `guarded_merge.rs:640-647`, plus `provider.count()` and `merge_effects` equalities at two points. The brief's suspicion that it was missing is wrong.
- **All four row items** — live response `:568-579`, key/record state `:585` and `:631`, restart observation `:613-623`, effect count `:588` and `:642`. Each has an assertion.
- **`assert_ne!(refused["mutation"]["attempt"], …)` at `:610`** — not vacuous, as I first theorised: the reuse refusal comes from the `pre_gate` spend, which calls `correlation` with a genuinely new attempt, so both sides are objects.
- **Cross-journey leakage** — `merge_mode` 5 and 6 are new values used nowhere else; the `0400` window is inside a per-journey tempdir, restored at `:592`. 13/13 ignored journeys pass in one serialized run.
- **Deadlock on a panic before the release** — `OwnedProcess::drop` kills and reaps (`cli_journey.rs:52`), and both new hold loops select on the stop channel (`local_runtime.rs:132`, `:151`).
- **Mutant discrimination on the cause branch** — flipping `if settled.is_err()` to `is_ok()` is caught *only* by the new case; modes 0–3 never assert the absence of a cause on success. Argued from the code, not injected: I did not touch `crates/`.

## 6. Paths written outside the worktree

- `~/.cache/cv2-wave-20260912/adversary-1/ignored-lane.log` (1.6 K) — my assigned scratch root.

Also created inside the worktree, per the brief: `.local/tmp` (the `TMPDIR` the brief mandates; git-ignored, empty).

```findings
- file: docs/gitlab-write-failure-matrix.md
  line: 20
  category: contract-drift
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: the Attempt terminal settlement row still lists the production CLI evidence as remaining and still credits only mutation port tests, while the journey that discharges it shipped in the same change; the rewrite should also name the one persistence-failure shape injected, a read-only metadata file that fails every read-write open.
- file: adapters/gitlab/tests/local_runtime/guarded_merge.rs
  line: 546
  category: concurrency
  severity: warning
  verdict: INFEASIBLE
  origin: introduced
  message: the mode-9 break condition reduces to "a PUT appears in methods", which is recorded before the fixture latches merge_mode, so the release can overtake the hold and answer 200 applied with a real provider effect in a journey asserting a refusal; I could only construct it by withholding the request body, and the real journey's window is a microsecond against a 10 ms poll.
- file: adapters/gitlab/tests/local_runtime/guarded_merge.rs
  line: 517
  category: acceptance
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: the story's acceptance sentence names five inspections including final disclosure admission, and the case performs four; the post-settlement issuance::resolve block at execution.rs:518 is reached by no CLI journey in a failing state, so deleting it leaves the new case green.
```
