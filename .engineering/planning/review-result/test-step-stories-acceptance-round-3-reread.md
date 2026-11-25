---
format: aep.planning-md/3
id: review-result:test-step-stories-acceptance-round-3-reread
kind: review-result
status: active
title: Acceptance critic, re-read after round 3, the test-step story set
relations:
- reviews: story:the-gates-test-step-ends-within-120-s-on-the-forge
revision: 1
---
Verdict: approve, for story:seven-cli-tests-shorten-their-waits-or-a-production-wait at revision 10.

Read-only re-read by the acceptance critic after its round-3 report. Nothing was run; the story and
the code were read. Part 1 is a full read of that one story. Part 2 covers only the acceptance
statements that changed when the round-3 notes were applied to stories all four critics had
approved.

## Part 1: story:seven-cli-tests-shorten-their-waits-or-a-production-wait

No blocker. The round-3 blocker is resolved:

- Acceptance 2 says the test's own probe takes the slow first answer.
- Acceptance 5 no longer claims that `clock_check_below_quorum_refusal` passing shows the clock
  variable is read. The unit test and the `grep` on `query_timeout()` cover the wiring.
- Acceptance 4 puts the summary line into `--source`.

Arithmetic of acceptance 6, checked against the code:

- `acquire_explained` (`crates/connectors-host/src/local/clock.rs` lines 436 to 438) sets one
  shared deadline of `query_timeout()` per acquisition and joins all source threads. A silent
  source costs that timeout once per row. `clock-check` makes one attempt (lines 399 to 401).
- `answering()` (`apps/connectors/tests/local_cli.rs` lines 139 to 150) blocks on the first
  answer, which `slow_first_clock_source` delays: 1.3 s after this story, 2.5 s today.
- The rest of the test is three rows of `setup init` plus `clock-check` (lines 366 to 368).
- The story's 3.7 s (1.3 + 3 × 0.8) and 7.3 s (1.3 + 3 × 2) are correct. The plan's 9.0 s equals
  2.5 + 3 × 2 plus 0.5 s of other work.
- With the variable read: about 4.2 s. Without it: about 7.8 s, and never under 7.3 s, because the
  silent source never answers.
- "Under 6 s" lies 1.8 s from each. A run that does not read the variable cannot end under 6 s, so
  a pass is conclusive. On a loaded runner the only wrong result is a red run of a correct
  change, when other work takes 2.3 s or more.

Note: acceptance 6 can only be read in the pull request's gate job. The run of acceptance 4
prints the same duration before that.

## Part 2: changed acceptance statements

- story:four-idle-exit-tests-no-longer-span-several-5-s-recovery-sweeps, acceptance 3: checkable.
  `Background::start(` appears at lines 1865, 4811 and 4878 of
  `crates/connectors-host/src/local/owner/transport.rs`; the owner's call is the one at 1865.
  `sweep_interval()` and the test do not exist yet, so the check fails today.
- story:eight-adapter-and-catalog-tests-drop-waits-that-prove-nothing, acceptance 2: checkable.
  `bundle_adversary_2.rs` has no `swaps` today.
- story:the-catalog-crate-tests-ingest-the-released-definition-once-per-process, acceptance 4:
  checkable. Note: a comment would satisfy the `grep`; acceptances 1 and 2 cover the rest.
- story:six-cli-tests-take-jail-binaries-built-once-per-run, acceptance 1: checkable. The four
  names are distinct and the filter `mounted::tests::install_` matches none of the nine tests now
  in `mounted/tests.rs`.
- story:store-tests-end-within-60s-under-load, first acceptance statement: note. The command form
  carried no time, so "the two times are written as one evidence record" was not literally true,
  and a host load of at least 60 cannot be produced on demand.
- story:the-lane-slow-runs-the-ignored-timing-family, acceptance 2: checkable. `mod tests` is at
  line 1393 of `crates/connectors-build/src/ignored.rs`; `slow.yml` names no `crates/`, `apps/`
  or `adapters/` path today, so the test fails before the work.

```findings
[]
```
