---
format: aep.planning-md/3
id: review-result:clock-repeat-adversary-pass-1
kind: review-result
status: active
title: 'Adversary pass 1: an approved write survives one refused clock acquisition'
relations:
- reviews: story:an-approved-write-survives-one-refused-clock-acquisition
revision: 1
---
unit: story:an-approved-write-survives-one-refused-clock-acquisition; working tree `worktree clock-stories`, HEAD `faa134f0d` plus the unstaged change
verdict: CONFIRMED (2 warnings, 2 notes, no blocker; no defect found in the decision or the deadline)
cases: executed 93→111, red 0 now (93 is the implementor's `regression.log` for the same filter)
origin: introduced 4 / pre-existing 0 / undecided 0
wrote-outside-worktree: 8 paths
needs-coordinator: the six in-tree mutants of point 5 were not run, so the reserve survivors are read, not measured

**1. Diff.** `git diff --stat -- . ':!.engineering'`: 15 files, 736 insertions, 27 deletions (before: 726/27). My part:
- `crates/connectors-host/src/local/clock/tests.rs` +4 and `owner/mutation/tests.rs` +6: one module declaration each, appended.
- Two new files, both named `second_attempt_adversary_tests.rs`, under `clock/` and `owner/mutation/`.
- No production or fixture file changed: `sha256sum -c` of the six is OK against my snapshot.
- Deviation from the brief: the clock module is declared in `clock/tests.rs`, not `clock.rs`, and the owner file is pulled in with `include!` inside `mod recheck`. Both keep my diff to test files.

**2. Cases added.** 18, all green when first run alone at 08:52 CET (4.2 s step); none is red.
- Clock file, 12 cases, 0.01 to 2.7 s:
  - eight kinds of `unverified` reply, each at both table positions, end after one attempt;
  - `too_imprecise` is not repeated; a third attempt that would answer is not made;
  - the refusal after two attempts names the second attempt's sources;
  - a passed `until` and `Duration::MAX` reserve leave one attempt without panic;
  - the retired table refuses at once;
  - three "twin" cases re-run the implementor's scenarios against a copy of `acquire_within` with one condition changed.
- Owner file, 6 cases, 2.1 to 4.7 s (up to 7.4 s in the loaded run): one per call site, with the time left chosen between two reserves so the request count names the reserve.

**3. Suite.** `cargo nextest run --locked -p connectors-host --no-fail-fast -E 'test(/clock|recheck|approval_issuance/)'`:
- 08:54 CET: 111 run, 109 passed, 2 failed, 13.3 s, exit 100. One failure is F1; the other was my own case, which unwrapped a `Timeout`, since fixed.
- 08:59 CET: 111 passed, 7.6 s.
- `cargo clippy --locked -p connectors-host --all-targets -- -D warnings` exit 0; `cargo fmt --package connectors-host -- --check` exit 0.

**4. Findings** (all cover the tree above)

| id | severity | file:line | verdict / origin |
|---|---|---|---|
| F1 | warning | `crates/connectors-host/src/local/owner/mutation/tests.rs:1272` | CONFIRMED / introduced |
| F2 | warning | `crates/connectors-host/src/local/owner/mutation/execution.rs:442` | CONFIRMED by reading / introduced |
| F3 | note | `docs/design/approval-clock-quorum.md:346` | CONFIRMED / introduced |
| F4 | note | `crates/connectors-host/src/local/clock.rs:422` | INFEASIBLE / introduced |

- **F1: statement 17's guard is red on a loaded host.**
  - Evidence: in the 08:54 run `recovery_refused_inside_the_reserve_answers_as_today` panicked at `tests.rs:1238` with ``called `Result::unwrap()` on an `Err` value: Error { code: Timeout, …``. It passed at 08:59 and in the implementor's logs.
  - Cause: `until` is 1.7 s, which leaves 1.2 s for all of recovery's non-clock work.
  - What reaches it: any gate run on a busy pool.
  - Smallest fix: a longer query deadline in that test, for example 1,500 ms with `until` 3.9 s after the call (2.4 s left after the attempt, under the 2.5 s that would start a second), about 1 s more runtime. Statement 16 (2 s) has the same shape and passed both runs.
- **F2: the reserve is unobserved at three of five call sites.**
  - Reserve 0 at `approval_issuance.rs:394`, `:634` and `execution.rs:442` changes no implementor test: every test through them has 50 s or 120 s left.
  - A swap to the 1 s constant at `execution.rs:306` passes statement 16 (2 s deadline: under 1.5 s left either way); a swap to the function at `:442` is untested.
  - What reaches it: a later edit of a call site.
  - Fix: keep my six owner cases. They are green now; red on the mutants is expected by arithmetic, not run.
- **F3: the network-attacker row overstates.** A forged or replayed datagram that arrives at or after the deadline is `unreachable` (`clock.rs:634-636`, before verification), and a second attempt follows. Nothing is gained by it (new socket, new nonce). Fix: add "received inside the deadline" to the row.
- **F4: the second attempt has no spacing.** It follows at once when the first ended early on a receive or connect error (`clock.rs:620-628`), so both attempts sample the same moment. Read from the code, not run; the story's measurements show no such dropout.

**5. Attacked, not broken**
- **Decision (point 1).** The second attempt runs only for `BelowQuorum` whose sources are each `Verified` or `Unreachable`. It does not run for `Split`, `TooImprecise`, a refusal with no `Refusal` (local timer, continuity, calendar, invalid configuration, retired), or any `Unverified` source. `Agreed` and `Outlier` never occur in a refusal.
- **Deadline (point 2).** `until` already past, `Duration::MAX` reserve and query: one attempt, no panic. Every refusal returned before `until`.
- **Twice-acquired state (point 3).** An attempt writes no log, counter or audit record, so nothing is lost or doubled. After two refusals the answer is the second attempt's, as the story says.
- **Callers (point 4).** All five reserves match the story's table. There is no sixth write-path caller: only `maintenance.rs:110` and `apps/connectors/src/local.rs:873` keep one attempt.
- **Mutants (point 5).**
  - `>=`→`>`: killed by `the_second_attempt_needs_one_deadline_and_the_reserve`, by reading.
  - `unverified` dropped, any refusal, loop of three: the twins give [2,2], [2,2], [3,3] where the implementor's tests pin [1,1], [1,1], [2,2].
  - Reserve 0 at `execution.rs:306` and `recovery.rs:54`: killed, per the implementor's `probe-reserve-zero.log`.
  - `Retired` given a second attempt: equivalent, since that arm has no network.
- **Texts (point 6).** Statements 19 to 24 hold; the hunks of the two `clock.md` files are identical; no other single-attempt claim found.

Not run:
- The in-tree mutants: my role file forbids editing a file under attack even briefly, and the brief forbids building a scratch copy. To measure, apply each mutant and run `-E 'test(/second_attempt_adversary/)'`.
- The Windows-target clippy (statement 27) with my files.
- `connectors-docs generate --check`.
- `clock_check_below_quorum_refusal` in `-p connectors`.
- Any run against the base commit.

No worktree lease was acquired.

**6. Paths outside the worktree**, all in `<session scratch>/clock/adversary/`: `run-1-own.log`, `run-2-narrow.log`, `run-3-narrow.log` (one marker line appended by me), `clippy.log`, `diff-stat-before.txt`, `numstat-before.txt`, `numstat-after.txt`, `sha-before.txt`.

```findings
- file: crates/connectors-host/src/local/owner/mutation/tests.rs
  line: 1272
  category: boundary
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: the recovery reserve guard leaves 1.2 s for all non-clock work and answered Err(Timeout) at tests.rs:1238 in one loaded run of the narrow filter
- file: crates/connectors-host/src/local/owner/mutation/execution.rs
  line: 442
  category: mutant
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: reserve 0 at approval_issuance.rs:394, :634 and execution.rs:442 and a swapped reserve at execution.rs:306 or :442 change no implementor test, read from the deadlines and not run as mutants
- file: docs/design/approval-clock-quorum.md
  line: 346
  category: contract-drift
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: a forged or replayed datagram arriving at or after the deadline is unreachable, not unverified, and a second attempt follows it
- file: crates/connectors-host/src/local/clock.rs
  line: 422
  category: judgement
  severity: note
  verdict: INFEASIBLE
  origin: introduced
  message: the second attempt starts at once when the first ended early on a receive or connect error, so both sample the same moment; no measured dropout of that kind
```
