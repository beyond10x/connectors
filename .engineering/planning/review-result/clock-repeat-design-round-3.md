---
format: aep.planning-md/3
id: review-result:clock-repeat-design-round-3
kind: review-result
status: active
title: 'clock repeat story: design critic, round 3'
relations:
- reviews: story:an-approved-write-survives-one-refused-clock-acquisition
revision: 1
---
verdict: approve

Round 2 findings of the design axis, one line each:
- Blocker (the start condition let the repeat spend the whole remaining time): answered by `acquire_within(until, reserve)`, the two reserves and guards 16 and 17 (story lines 86-121, 262-273).
- Warning (4 s for `issue`, 8 s for a write): answered in "What the repeat costs" (story lines 158-164).
- Warning (the recovery test could not be built): answered. The test is in `owner/mutation/tests.rs`, runs on the test thread and uses a `Quiescent` test constructor. The constructor needs one change, see finding 1.
- Warning (the 120 s and 50 s query timeouts): answered by the 500 ms override after `setup()` or `site()` (story lines 189-191, 222-224). One citation is wrong, see finding 3.
- Warning (wrong citation of `semantics.md:131`): answered by change 6, which cites `semantics.md:123-124`.
- Note (`:47-52` cut the call): answered, the story cites `:44-55`.

```findings
- file: .engineering/planning/story/an-approved-write-survives-one-refused-clock-acquisition.md
  line: 255
  category: design
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "The `#[cfg(test)]` constructor of `Quiescent` is used only from `mod recheck`, and that module is `#[cfg(unix)]` (owner/mutation/tests.rs:668). On `x86_64-pc-windows-gnu` the constructor has no caller, so `cargo clippy ... -D warnings` (statement 27) fails on `dead_code`. The fixture constructor is covered by `#![allow(dead_code)]` (server.rs:4); this one is not. Fix: write it as `#[cfg(all(test, unix))]`, the gate `supervisor.rs:583` already uses, and add that to statement 15."
- file: .engineering/planning/story/an-approved-write-survives-one-refused-clock-acquisition.md
  line: 108
  category: design
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "Change 2 says 'Two constants' and defines the second as 'the query deadline + 1 s'. A `const` cannot follow the thread-local `set_test_query_timeout` (clock.rs:36-45, read through `query_timeout()` at clock.rs:387-389). With a `const` of 3 s, statement 16's arithmetic ('2 s are needed') is false in the 500 ms tests. Fix: say `RESERVE_AFTER_CLOCK` is a constant and the reserve before the second reading is `query_timeout() + RESERVE_AFTER_CLOCK`, computed at the call, so the tests see 1.5 s."
- file: .engineering/planning/story/an-approved-write-survives-one-refused-clock-acquisition.md
  line: 222
  category: design
  severity: note
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "'`setup()` or `site()`, which set a longer one (`approval_issuance/tests.rs:518`, ...)' is true only for `site()` (mutation/tests.rs:716-718). `setup()` (approval_issuance/tests.rs:66-135) sets no timeout. :518 sits inside one test and sets 120 s there, so the default is 2 s. Also 'keeps `until` at 50 s' holds for `site()`'s `WAIT` (tests.rs:697) but not for the `issue` tests, whose `until()` is `CONTENDED_LOCK_WAIT`, 120 s (approval_issuance/tests.rs:14-16; metadata.rs:240). Fix: cite only mutation/tests.rs:718 and write 'until is 50 s in `owner/mutation/tests.rs`, 120 s in `approval_issuance/tests.rs`'."
- file: .engineering/planning/story/an-approved-write-survives-one-refused-clock-acquisition.md
  line: 140
  category: design
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "docs/design/approval-clock-quorum.md:133-134 reads 'There are no retries. The total network wait is therefore 2,000 ms whatever the number of sources, the same bound as today.' The story replaces the first sentence and statement 23 greps only it. The second sentence stays false for the four repeating callers (4 s). Fix: change 5 names `:133-134` and replaces both sentences; statement 23 adds `rg -n 'same bound as today' docs/design/approval-clock-quorum.md` finds no line."
- file: .engineering/planning/story/an-approved-write-survives-one-refused-clock-acquisition.md
  line: 234
  category: design
  severity: note
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "`owner/approval_keys.rs:641-655` does not exist. The file is crates/connectors-host/src/local/approval_keys.rs; `with_signer` is at :641 and `custody::Store::open_at` at :651. Fix: cite `local/approval_keys.rs:641-655`."
- file: .engineering/planning/story/an-approved-write-survives-one-refused-clock-acquisition.md
  line: 102
  category: design
  severity: note
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "The reserve rationale ('the caller's next `check(until)` turns today's refusal into `Timeout`') holds for the two `time()` sites and for recovery. It does not hold for `issue` and `standing_reading`, where a refusal returns at once through `?` (approval_issuance.rs:391-393, :632-634), so their reserve protects only the success path. Fix: add 'in `issue` and the standing reading the reserve covers the work after an answer only'. Statements 16 and 17 guard only the write and recovery reserves; no guard covers the `issue` reserve, which is acceptable once the text says so."
- file: .engineering/planning/story/an-approved-write-survives-one-refused-clock-acquisition.md
  line: 132
  category: design
  severity: note
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "The net-zero edit is coherent. Lines 98-99 hold the replaced sentence (clock.md:98 'There', :99 'are no automatic network retries.'), and the website page holds it at :113-114. The only line citation outside the story, `ess/domains/cli.yaml:189` -> `clock.md:163`, stays valid. The anchors in `semantics.md:129` (`#refusals`) are not touched. Two small points. `### Second attempt` after line 235 lands inside `## CLI and verification`, after the 'Verification must cover' paragraph of `### Refusals`, although it describes acquisition. Line 99's sentence says 'the callers of the new section', which does not link to it. Fix: the replaced sentence links `[second attempt](#second-attempt)`; accept the placement as the price of stable line numbers."
- file: .engineering/planning/story/an-approved-write-survives-one-refused-clock-acquisition.md
  line: 116
  category: design
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: "Defensibility of the reserves. After an answer, `issue`, the standing reading and recovery run only local work: resolve, SQLite, flock and custody. The policy lease waits up to `until` under contention (approval_policy.rs:405-437), so no fixed reserve bounds it, but that is today's behaviour too. The 3 s before the first write reading (2 s attempt + 1 s) covers resolve, `admit`, audit admission and native preflight (execution.rs:297-433). Preflight is a child round trip bounded only by `until` (runtime/process.rs:1014-1043). A slow adapter can use up the 1 s. I name no caller whose code shows more than 1 s as the normal case, and no case is worse than today, because the first attempt of each reading runs as before. With `until` at 20 s the reserves seldom bind."
- file: .engineering/planning/story/an-approved-write-survives-one-refused-clock-acquisition.md
  line: 264
  category: design
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: "Tests buildable as written, checked against the cited code. Statements 1-10 build on `Server::spawn/shared` (server.rs:100-141) with a shared counter for `Server::dropping`, and the clock tests module is a child of `clock`, so a private decision function is reachable. The owner tests reach the acquisition on the thread that set the timeout: `execute` (mutation/tests.rs:959), `issue`, `standing_reading` (approval_issuance.rs:617, child module) and `recover_observation` are all called directly, with no worker thread. Statement 15 fits: `Quiescent` has only `instance` and `_child: &mut Option<Child>` (supervisor.rs:332-335), `store(paths, NoClock)` and `candidate` are reachable from `recheck`, `prepare` reads no clock (mutations.rs:123-160), and `recover` takes its clock from `RecoveryClock` (recovery.rs:10-15, :63). The test must pass `&mut None`. Statements 16 and 17 need `write()` to take an `until` (mutation/tests.rs:984 fixes it to `Instant::now() + WAIT`); the story names this only for the revoke hook. Timing margins hold in the loaded direction: delay only shortens what is left."
```

Out of my lane: the checkability of statement 24 (it compares two runs, a before and an after) and the coverage of the incident parent belong to the acceptance and scope critics. The seam to the two draft stories is unchanged and clean. I walked no `aep` edges and ran no build.
