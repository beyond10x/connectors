---
format: aep.planning-md/3
id: review-result:invoke-read-deadline-design-round-2
kind: review-result
status: active
title: Design critic, round 2, story:invoke-admission-queues-under-load
relations:
- reviews: story:invoke-admission-queues-under-load
revision: 1
---
verdict: approve

No blockers remain in my lane. I read the revised story and its split sibling, `story:timeout-refusal-says-how-long-it-waited`. I also checked that `CONNECTORS_TEST_OWNER_START_DELAY_MS` exists in `owner_slow_start.rs` and `transport.rs`, and that the walk page path calls `run_selected` (`invoke.rs:742-775`). I ran nothing.

The earlier findings now hold:
- **1, one cutoff:** the cutoff is a single absolute value that both the owner start and `session.rs:600` read. Acceptance 4 and 7 test it.
- **2, claim and pre-deadline exchanges:** the "n plus grace, not 2n" claim is gone. The 30/40 s exchanges (`transport.rs:1037, 1052, 1068`) are listed under Unchanged.
- **3, reads only:** the refusal covers `--idempotency-key`, `--approval-file` and `--describe`, and `--definition` is stated. Acceptance 6 covers it, with no owner started.
- **4, executable-changed path:** the story ends validation at the earlier of the read deadline and the lease expiry. `supervisor.rs` and `owner.rs` are in scope. Acceptance 9 tests it at a 60 s read deadline against a 30 s lease.
- **5, `lapses_early`:** it is under Unchanged.
- **6 and 8, `waited_ms`:** it is split out with a `depends_on` edge on this story. `connectors.cli.Failure` is untouched here.
- **7, SIGTERM story:** it is named under "Shares files with".

Notes (not blockers):
1. **Carrying the cutoff from `prepare` to the runner.** `prepare` returns rewritten args and the runner builds its own `Session` (`local.rs:68-72`). The story says "one value" but not how it crosses that boundary. Name the carrier, for example a process-wide cell set in `prepare`. The implementer may pick it.
2. **First page of a `--walk`.** The owner start runs once in `prepare`, and every page then enters `run_selected` (`invoke.rs:760-775`). If each page takes a fresh cutoff, page 1 still has two windows, start then read. This is the same as today. It does not affect acceptance 8. Say so in one line, or let page 1 reuse the `prepare` cutoff.
3. **`story:invoke-exits-promptly-on-sigterm`.** The ordering with it is prose only: "runs its blocked-admission test with `--deadline-seconds 120` too". No `depends_on` edge records it. Add one from the SIGTERM story to this one when that story is next revised, so `waves` orders them.
4. **`story:refusals-name-the-field`.** It also edits `Failure` in `ess/domains/cli.yaml`. It is not in "Shares files with". It now matters to the split `waited_ms` story, not to this one.
