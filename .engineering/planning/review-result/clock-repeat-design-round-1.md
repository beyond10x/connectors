---
format: aep.planning-md/3
id: review-result:clock-repeat-design-round-1
kind: review-result
status: active
title: 'clock repeat story: design critic, round 1'
relations:
- reviews: story:an-approved-write-survives-one-refused-clock-acquisition
revision: 1
---
verdict: needs-revision

Q1 ruling: one repeat does not change what an attacker on the path gains.
- Every attempt builds a fresh request with a fresh nonce (`crates/connectors-host/src/local/clock.rs:544`, `contracts/service/clock.md:92-93`).
- Every attempt binds a fresh socket (`clock.rs:386`).
- The counted set and the answer come from one attempt only (`clock.rs:432-449`, `clock.md:119-121`). A first-attempt reply is not reusable in the second.
- An injected or wrong-nonce datagram already ends an exchange as `Unverified` (`clock.rs:585-587`), and `Unverified` is excluded from the repeat (`clock.md:220`).
- A drop-only attacker already refuses the write and now has to drop twice.
- A response is no larger than the request (`clock.md:98`), so there is no amplification.
- The story's reason "a repeat signs nothing more" is the wrong ground. Replace it with: fresh nonce, fresh socket, attempts never merged. The story must say "attempts are not merged" and "a fresh socket per attempt", because a late first-attempt reply to a reused socket would score as `unverified`.

Q2 ruling: the repeat does not belong in `acquire()` for every caller, and it also does not fit "only the two write paths". Make it a separate entry point on `ApprovalClock`, for example `acquire_with_repeat(until)`. Leave `acquire_explained` as one exchange. The production callers I found:
- `owner/approval_issuance.rs:391-393` (`issue`): repeat.
- `owner/approval_issuance.rs:630-633` (`standing_reading`, `grant-standing`): same bare `unavailable` and a signed write, so repeat.
- `owner/mutation/execution.rs:91-93` (`time()`, called at `:297` and `:433`): repeat.
- `owner/mutation/recovery.rs:47-52`: repeat (it settles a Prepared write).
- `owner/maintenance.rs:106-111`: `.ok()`, the next scan retries, so leave it.
- `apps/connectors/src/local.rs:873` (`acquire_explained`, `approvals clock-check`): must stay one exchange. It is the diagnostic, and a repeat would hide the dropouts it exists to show. `contracts/cli/v1alpha1/semantics.md:131` says "asks every source once ... no automatic retry", and tests at `clock/tests.rs:277,455,488` and `contracts/cli/v1alpha1/fixtures/values.json:1064` read it.
- The story names only two paths, so it leaves `grant-standing` and recovery on the same defect.

```findings
- file: .engineering/planning/story/an-approved-write-survives-one-refused-clock-acquisition.md
  line: 53
  category: design
  severity: blocker
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "Change 1 reuses the first clock for the second `time()` (execution.rs:297 and :433), but the second acquisition is deliberately fresh after native preflight and provider work (execution.rs:427-428: 'No policy/key/metadata lock is held during native preflight or fresh clock acquisition'). A BoundedClock refuses any reading once a counted source's elapsed upper bound from S_i passes 30 s (clock.rs:817-818, clock.md:168-169), and widens by the rate error up to the 8,000 ms cap (clock.rs:738). A slow preflight therefore turns a write that succeeds today into `unavailable` after audit admission and preflight, the worse place to fail. The story never mentions the 30 s window. Fix: name the 30 s window and the width growth in change 1 and state the fallback (reuse only while the first clock still reads, else acquire again, with the repeat); or drop change 1 and keep three acquisitions, each with the repeat. Add an acceptance with a preflight longer than the window."
- file: .engineering/planning/story/an-approved-write-survives-one-refused-clock-acquisition.md
  line: 56
  category: design
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "Change 2 says 'inside the caller's deadline', but `acquire()` and `acquire_explained()` take no deadline (clock.rs:376,381). The callers hold `until` (approval_issuance.rs:351,371; execution.rs:84). The story needs the new entry point taking `until`, and the rule that the repeat starts only if at least one QUERY_TIMEOUT (clock.rs:28) plus the local checks remain. Apply the Q2 ruling: that entry point serves issue, standing_reading, time() and recovery; clock-check keeps one exchange."
- file: .engineering/planning/story/an-approved-write-survives-one-refused-clock-acquisition.md
  line: 61
  category: design
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "Change 4 omits statements the repeat falsifies: clock.md:94-99 (one shared two-second deadline, 'no automatic network retries'), the doc comment clock.rs:373-375, and docs/local-clock.md:88-90. The network wait now grows to two deadlines. State that the total wait is at most two deadlines, that attempts are not merged and use fresh sockets, and that semantics.md:131 still holds for clock-check only. Add each file to the change list and an acceptance that greps for the old 'no retries' wording."
- file: .engineering/planning/story/an-approved-write-survives-one-refused-clock-acquisition.md
  line: 59
  category: design
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "Change 3 adds `clock_refusal` and `clock_sources` to the CLI answers of `approvals issue` and `invoke`. It names neither the plumbing nor the consumers. The precedent is `Error::clock` (owner.rs:261-272) and the `clock_format_retired` member (owner.rs:191-195). It must carry the combined refusal and the source lines instead of mapping to a bare `Code::Unavailable`. The CLI mapping today is `clock_failure` (apps/connectors/src/local.rs:565-570), at stage `observation`, while the incident shows stage `readiness`/`admission`: state which stage these carry. Name the consumers of the new members (the repository rule for a changed answer member). Source addresses also reach `invoke` callers: say so."
- file: .engineering/planning/story/an-approved-write-survives-one-refused-clock-acquisition.md
  line: 53
  category: design
  severity: note
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "Two things in one story: change 1 (freshness semantics of the second reading) and changes 2-3 (repeat, refusal detail). The seam is change 1. The repeat alone fixes the incident class (each acquisition fails with p squared) and ships without the 30 s coupling. Splitting change 1 into its own story lets the repeat land first. This is the drafter's call, but if kept together the acceptance must cover the change-1 risk above."
- file: .engineering/planning/story/an-approved-write-survives-one-refused-clock-acquisition.md
  line: 30
  category: design
  severity: note
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "Citation 'owner/approval_issuance.rs:381-388' does not show the acquisition: those lines validate the proof output path and begin `selected`. The `acquire()` call is at approval_issuance.rs:391-393. The other citations hold: clock.rs:28 is QUERY_TIMEOUT, :375 is the 'no retry occurs' comment, :377 is the drop of `refused.failure`, execution.rs:84-96 is `time()` called at :297 and :433, and `BoundedClock::now` is clock.rs:833-838 (rate error through bounds_ns)."
- file: .engineering/planning/story/an-approved-write-survives-one-refused-clock-acquisition.md
  line: 86
  category: design
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: "Open questions: rulings above. Replace both 'Open before it is proposed' bullets with Q1's grounds (fresh nonce and socket, no merging, `unverified` not repeated) and Q2's entry point and caller list."
```

Out of your lane, not set into my verdict: acceptance 5 ("existing tests pass unchanged") does not check the change-1 risk. Acceptance 2 and 3 name no timing for the 2 s waits, so their test time against the 60 s bound is for the acceptance critic. The covered-parent check on "None of 0.74.1 to 0.76.2 changed these files" is for scope. Neither `aep plan artifact graph` nor `validate` was run: with one story and one `serves` edge there is nothing for a cycle or a hidden dependency to attach to.
