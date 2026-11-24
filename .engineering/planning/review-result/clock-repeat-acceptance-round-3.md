---
format: aep.planning-md/3
id: review-result:clock-repeat-acceptance-round-3
kind: review-result
status: active
title: 'clock repeat story: acceptance critic, round 3'
relations:
- reviews: story:an-approved-write-survives-one-refused-clock-acquisition
revision: 1
---
verdict: needs-revision

Round 2 findings of the acceptance axis (round 2 file lines 25-96):
- Proof file and `grant_id` infeasible (:165): answered. Statements 11 and 12 now check the request count, not a written proof.
- Generated contract page (:186): answered. `website/docs/reference/contracts/clock.md` is in scope, and statements 20, 22 and 25 cover it.
- Site has no proof, `Server::fixture`, thread-local (:171): answered. Statements 13 and 14 name the standing-grant route, the `Server::dropping` change to `Site` and the override after `site()`.
- Statement 2 shares sources with statement 1 (:148): answered. Statement 2 uses "two other new sources".
- Recovery test unnamed (:177): answered. Statements 15 and 17 name the tests and how to build them.
- Statement 14 count (:184): answered. Statement 18 says "grown by 1" after each row and names the command.
- `semantics.md:131` citation (:97): answered. The story now cites `docs/local-clock.md:91-92` and `contracts/service/clock.md:98-99`.
- Statement 17 judgement (:195): answered. Statements 21 to 24 use exact `rg` strings; I confirmed `second attempt` is absent today in all six files (0 hits), and "There are no retries" is on one line at `approval-clock-quorum.md:133`.
- `acquire_within` only on `ApprovalClock` (:80): answered. It is now on `Configuration` and `ApprovalClock` and returns `Refused`.
- Late reply and the maintenance scan (:86): answered. Statements 10 and 19 cover them.

Buildability of each named test:
- Statements 1 to 10 (clock): buildable as written. `Server::spawn` (`server.rs:68`) and `Server::silent` (`:66`) cover the closure-based sources. `fields` is public (`:223`), and the request offset 12 matches `Fixture::reply` (`:261`).
- Statement 11: buildable. `ClockServer`, `setup`, `fake_issuer`, `policy`, `prepare` and `issue` run on the test thread. After the clock, `fake_issuer` gives no key record, so site 1 most likely ends `NotFound` (`approval_keys.rs:489`). That is a different code from the clock `Unavailable`, so the red claim holds on both the code and the count.
- Statement 12: buildable. `standing_reading(config, adapter, until)` is private and `tests` is a child module (`approval_issuance.rs:24`, `:617`). `Target.adapter` can be cloned with `standing_grants` set.
- Statements 13 and 14: buildable. `execute` runs `time()` in plain closures on the calling thread (no `spawn` or `scope` in `execution.rs`), so the thread-local reaches the acquisition. `Site._clocks` is `[Server; 2]` (`mutation/tests.rs:~700`). `write()` needs an optional hook step and an `until` parameter.
- Statement 15: buildable. `candidate`, `store` and `NoClock` are reachable through the glob chain. `Store::prepare` is public (`mutations.rs:123`), and `binding()` only needs the published connection that `site()` already creates. `recover_observation` is re-exported (`mutation.rs:15`). `Quiescent` has the private fields `instance` and `_child: &mut Option<Child>` (`supervisor.rs:332-335`), so a test constructor taking `&mut None` works.
- Statements 16 and 17: buildable.

Timings:
- Statement 8 holds. About 500 ms is left against 800 ms needed.
- Statement 16 holds. About 1.5 s is left against 2 s needed.
- Statement 17 holds. About 1.2 s is left against 1.5 s needed. Slow setup before the acquisition only widens these margins.
- A guard is red when its reserve is 0. 16 and 17 then repeat (`requests` is 2).

"Red before the change":
- Statements 1, 3, 5 to 10 fail to compile before the change. Statement 2 is declared green.
- Statements 11 to 13 and 15 fail on assertions, as the story says.
- Statements 16 and 17 are guards (green before, red with reserve 0).

```findings
- file: .engineering/planning/story/an-approved-write-survives-one-refused-clock-acquisition.md
  line: 256
  category: acceptance
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "The `#[cfg(test)]` constructor of `Quiescent` is called only from `mod recheck`, which is `#[cfg(unix)]` (`owner/mutation/tests.rs:668-669`). `owner`, `mutation` and `supervisor` are not cfg-gated (`owner.rs:5-11`), so on `x86_64-pc-windows-gnu` the constructor is dead code and statement 27 (`-D warnings`, :306-309) goes red. Statement 27 only covers the fixture constructor. Fix: say the constructor is `#[cfg(all(test, unix))]`, and extend statement 27 to name it."
- file: .engineering/planning/story/an-approved-write-survives-one-refused-clock-acquisition.md
  line: 107
  category: acceptance
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "Change 2 calls `RESERVE_BEFORE_SECOND_READING` a constant equal to 'the query deadline + 1 s (3 s in production)'. The deadline under test is a thread-local (`clock.rs:36-45`, `query_timeout()` at `:68`/`:73`), so a `const` is always 3 s. Statement 16 (:266-267) then needs 3.5 s, not the stated 2 s, and a reserve of 0 on `RESERVE_AFTER_CLOCK` alone does not turn it red. Fix: define it as a function `query_timeout() + RESERVE_AFTER_CLOCK`, or change statement 16's numbers to 3.5 s needed and name the constant it zeroes."
- file: .engineering/planning/story/an-approved-write-survives-one-refused-clock-acquisition.md
  line: 189
  category: acceptance
  severity: note
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "The sentence says `setup()` and `site()` set a longer deadline, citing `approval_issuance/tests.rs:518`. Line 518 is in the body of `clock_exchange_precedes_leases...`. `setup()` sets nothing, so the default 2 s applies until the test overrides it. Line 222-224 says `until` stays at 50 s, but the approval_issuance tests use `until()`, which is 120 s (`metadata.rs:240`). Fix: cite only `mutation/tests.rs:718` and say the approval_issuance tests set the 500 ms deadline themselves; give 120 s for statements 11 and 12."
- file: .engineering/planning/story/an-approved-write-survives-one-refused-clock-acquisition.md
  line: 265
  category: acceptance
  severity: note
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "Statements 16 and 17 need `until` as a parameter. `write()` hard-codes `Instant::now() + WAIT` (`mutation/tests.rs:~975-990`) and statement 13 names only a hook parameter. Fix: add 'and an `until`' to the `write()` change in statement 13."
- file: .engineering/planning/story/an-approved-write-survives-one-refused-clock-acquisition.md
  line: 171
  category: acceptance
  severity: note
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "Citations are one to two lines off. The counted-set text is at `contracts/service/clock.md:117-118`, not `:119-121`, which hold the restart and no-socket sentences. The 30 s window is at `:167-168`, not `:168-169`. Fix: cite 117-118 and 167-168."
```

I read only; no build, test or cargo call.
