---
format: aep.planning-md/3
id: review-result:clock-repeat-design-round-2
kind: review-result
status: active
title: 'clock repeat story: design critic, round 2'
relations:
- reviews: story:an-approved-write-survives-one-refused-clock-acquisition
revision: 1
---
verdict: needs-revision

Round 1 findings of the design axis, one line each:
- Change 1 (reuse the first clock, 30 s window): left the story. It is the draft story `an-approved-write-acquires-the-clock-once` and is carried at story lines 211-213 and 220-221.
- `acquire()` has no deadline: answered. `acquire_within(until)` and its start condition are at story lines 80-84, and the four callers are named. Point 1 below finds the condition too weak.
- Change 4 left out statements the repeat falsifies: answered at story lines 99-109. The `clock.md:94-99`, `clock.rs:373-375`, `local-clock.md` and `approval-clock-quorum.md` lines all exist.
- Change 3 (new CLI members): left the story. It is the draft story `a-clock-refusal-of-issue-and-invoke-names-its-sources`.
- The `:381-388` citation: answered. The story now cites `:391-393`, which shows the `acquire()` call.
- The "open questions" bullets: answered by "Why a repeat gives an attacker nothing" and "Decided".

Ruling 1. "At least one query deadline remains before `until`" is necessary but not sufficient in all four callers. After the acquisition each caller still runs `check(until)` or `resolve(..., until)`. `resolve` calls `check` first (`approval_issuance.rs:209`), and `check` returns `Code::Timeout` once `until` has passed (`:26-31`).
- Recovery: `observe(paths, alias, request, Some(key), until)?` (`recovery.rs:74`) calls `resolve`. Today a clock refusal leaves time, `observe` runs, and the caller gets `Ok(value)` with cause `Unavailable`. A repeat that spends the last deadline makes `observe` return `Err(Timeout)` through the `?`.
- Write path: `time()` ends in `issuance::check(until)?` (`execution.rs:96`). A timeout inside `initial` reaches the error branch, where `observe_as` also fails on `resolve`. The write ends `Err(Timeout)` instead of the `Ok(failure value)` with `not_attempted` that a refusal gives today. The first `time()` (`:297`) is followed by native preflight, the second acquisition and dispatch, so one spare deadline leaves them nothing.
- `issue` and `standing_reading` still resolve, lease the policy and the key, and sign (`approval_issuance.rs:395-420`, `:630-640`).

Ruling 2. Recovery holds an `until`. It is the parameter at `recovery.rs:19`, taken from the job's task (`supervisor.rs:1180-1188`) and checked at `:44` and `:58`. `acquire_within(until)` fits recovery. Only the `Prepared` branch acquires; `Dispatching` takes no clock.

```findings
- file: .engineering/planning/story/an-approved-write-survives-one-refused-clock-acquisition.md
  line: 82
  category: design
  severity: blocker
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "The start condition 'at least one query deadline remains before until' lets the repeat spend the whole remaining time. Recovery then ends Err(Timeout) where a refusal ends Ok(value, cause unavailable) (recovery.rs:74, approval_issuance.rs:209, :26-31). A write ends Err(Timeout) after audit admission (execution.rs:96). The sentence at story line 117-118, 'never moves a refusal past until', is false. Fix: give the entry point a reserve, acquire_within(until, reserve), and start the second attempt only when until - now >= QUERY_TIMEOUT + reserve. Name the reserve per caller as what the caller still needs after the acquisition: issue and standing (resolve, leases, signing), recovery (the two resolves, the policy lease, recover, the observe), first write time() (preflight, the second time() and dispatch), second write time() (ledger prepare, spend, dispatch). Add a caller-level acceptance: with until leaving exactly one deadline, recovery and a write return the value of today's refusal, not Timeout, and make the recovery one red before the change. Replace line 117-118 with the reserve rule."
- file: .engineering/planning/story/an-approved-write-survives-one-refused-clock-acquisition.md
  line: 115
  category: design
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "'One approved write has three acquisitions ... at most 12 s ... of the caller's deadline; the CLI's write deadline is 20 s' mixes two commands. approvals issue has its own deadline; the write's deadline holds two acquisitions (execution.rs:297, :433), at most 8 s of network wait. Fix: state 8 s for the write and 4 s for issue, and name what remains of the write's deadline for preflight and dispatch."
- file: .engineering/planning/story/an-approved-write-survives-one-refused-clock-acquisition.md
  line: 177
  category: design
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "Statement 13 cannot be built as written. No recovery test exists (mutation/tests.rs has none, recovery.rs has no tests module). recover_observation takes a Quiescent whose fields are private to supervisor.rs (supervisor.rs:332-335), so a test in mutation/ cannot construct it. Through the supervisor the call runs on a worker thread (supervisor.rs:719-721), where the thread-local set_test_query_timeout (clock.rs:17-20) is not set, so the test waits the production 2 s per attempt. Fix: say where the test lives (a test in supervisor.rs that sends the recovery job), and either make the test override a process-wide value (an atomic, with the thread-local kept as the default) or have the test call recover_observation on the test thread through a pub(super) test constructor of Quiescent. Name the time of the test."
- file: .engineering/planning/story/an-approved-write-survives-one-refused-clock-acquisition.md
  line: 163
  category: design
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "Owner tests reach the acquisition on the thread that set the timeout: issue and execute are called directly on the test thread (approval_issuance/tests.rs:518, mutation/tests.rs:718 and :831). But the existing setups set the query timeout to 120 s and 50 s (approval_issuance/tests.rs:17-19, :518; mutation/tests.rs:696, :718), and a dropped first request makes the client wait the whole query timeout. Statements 9-12 would run 50 to 120 s per dropped request. Fix: statements 9-12 state that the test overrides set_test_query_timeout to at most 500 ms after setup()/site() on the same thread, and name the time of each test."
- file: .engineering/planning/story/an-approved-write-survives-one-refused-clock-acquisition.md
  line: 95
  category: design
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "The story cites contracts/cli/v1alpha1/semantics.md:131 as saying 'asks every source once'. Line 131 says nothing of the kind (it holds the clock_refusal field text). The one-attempt rule of clock-check reaches the CLI contract only through semantics.md:123-124, 'performs the bounded authenticated exchange specified in the clock binding', which points at the clock.md text this story replaces. The story also changes contracts/service/clock.md without naming its readers. Fix: cite semantics.md:123-124, and add to change 4 that the new clock.md text states clock-check keeps one attempt; name semantics.md:123-124, docs/local-clock.md:91-92 and the clock-check tests (apps/connectors/tests/local_cli.rs:325) as the readers of the changed text."
- file: .engineering/planning/story/an-approved-write-survives-one-refused-clock-acquisition.md
  line: 93
  category: design
  severity: note
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "The recovery acquisition is at recovery.rs:51-53 (the selected lookup is :45-50). The cited :47-52 cuts the call. Cite :44-53."
```

The seam between this story and the two draft stories is clean: no promise of the parent falls between them, and none of their halves shares an abstraction with this one. I walked no edges (no `aep` graph command run). The acceptance critic owns the checkability and timing of statements 18 and 19; the scope critic owns coverage of the incident parent. The story-text citations of `clock.rs` (:28, :373-377, :386, :432-449, :544, :585-587, :817-818), `owner.rs:261-272`, `maintenance.rs:106-111`, `local.rs:873` and the fixture (`server.rs:58-60`, `:98`) all show what the story says, apart from the two cited above.
