---
format: aep.planning-md/3
id: review-result:clock-repeat-scope-round-2
kind: review-result
status: active
title: 'clock repeat story: scope critic, round 2'
relations:
- reviews: story:an-approved-write-survives-one-refused-clock-acquisition
revision: 1
---
verdict: approve

Round 1 findings of the scope axis (`.engineering/planning/review-result/clock-repeat-scope-round-1.md`):
- Finding 1 (blocker, CLI members on `issue` and `invoke`): left the story. It is the sibling `a-clock-refusal-of-issue-and-invoke-names-its-sources` (draft, `Change (draft)` 1-2, line 32-36), and change 5 of this story says no CLI member changes.
- Finding 2 (docs and `acquire()` comment): answered. Change 4 now names `clock.md:94-99`, `approval-clock-quorum.md:18`, `:133`, `:346` and `clock.rs:373-375`. I checked each line against the tree.
- Finding 3 (callers narrowed): answered. Change 2 takes `standing_reading` and recovery. Change 3 names `clock-check` and the maintenance scan as kept at one attempt, with reasons. The maintenance reason holds: the sweep re-runs every 5 s (`owner/maintenance.rs:49-55`).
- Notes 4 and 5: the `time()` standing route is now covered by statements 11 and 12, and the citations stand.

Coverage of the parent ("What happened" and "Cause"): 8 promises extracted, 8 traced.
- Three acquisitions per write: sibling `an-approved-write-acquires-the-clock-once` (the `Not in this story` list names it).
- No repeat, and the dropouts of the two answering sources and of the uplink: change 1 and the callers in change 2.
- The bare `unavailable`: sibling `a-clock-refusal-of-issue-and-invoke-names-its-sources`.
- Cloudflare wire version: named exclusion.
- Moved `time.txryan.com` address: "Done already".
- Contract and guide stating the behaviour: change 4.
- Post script retry: "Done already".

Nothing is claimed twice and nothing reaches beyond the parent.

Thread-local question: the owner tests do reach the acquisition on the thread that set the timeout. `execute(...)` is called directly at `owner/mutation/tests.rs:974`, and `issue(...)` directly in `approval_issuance/tests.rs`. `query_timeout()` is read on the caller thread (`clock.rs:68-70`, `:389`), before the per-source threads spawn. The statements do not say the tests must set it, though, and the existing owner tests set it to 50 s or 120 s (finding 1).

```findings
- file: .engineering/planning/story/an-approved-write-survives-one-refused-clock-acquisition.md
  line: 137
  category: scope
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "The 500 ms bound is written for 'Unit tests' and statements 9-12 do not repeat it. The owner tests it points at use a long bound. `owner/mutation/tests.rs:696` has WAIT = 50 s, `site()` sets `set_test_query_timeout(WAIT)` at :718, and `execute` gets `until: Instant::now() + WAIT` (:989). `approval_issuance/tests.rs:17-18` makes `until()` 120 s. Built literally, a dropped first request costs 50 s, no deadline remains before `until`, and no second attempt starts (statement 11 stays red and would exceed 60 s). Fix: under 'Through the owner' add 'each test sets `set_test_query_timeout` to at most 500 ms on its own thread after `site()` or setup, and keeps `until` at least 2 s plus that bound away'. Also state that `site()` (`mutation/tests.rs:726-733`) builds its sources with `Server::fixture` and keeps them in the private `_clocks`, so statement 11 changes `site()`, not `server.rs`. 'No fixture change is needed' is true only of `server.rs`. That is the line this story shares with the pattern-grant story."
- file: .engineering/planning/story/an-approved-write-survives-one-refused-clock-acquisition.md
  line: 177
  category: scope
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "Statement 13 has no test name. The nextest filter of statement 18 (:200) cannot match it, so the fourth caller ('recovery', `recovery.rs`) has no counted test. No existing test uses UDP sources for recovery: the only users of the fixture are `approval_issuance/tests.rs:522,575`, `mutation/tests.rs:730` and `local_cli.rs:106,128,332`. The conditional 'should no recovery test use UDP sources' is therefore already decided. Fix: name the test (for example `recovery_settles_a_prepared_write_when_the_sources_drop_the_first_request`), add `drop_the_first_request` to its name so the filter matches, and drop the conditional."
- file: .engineering/planning/story/an-approved-write-survives-one-refused-clock-acquisition.md
  line: 92
  category: scope
  severity: note
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "Two citations land short of the acquisition they name. `approval_issuance.rs:630-633` is the config and `ok_or`; the `selected.acquire()` is at :633-635 (`standing_reading` is :617). `recovery.rs:47-52` ends at `Some(`; the `.acquire()` and `.map_err` are at :53-54. Fix: cite `:628-635` and `recovery.rs:45-55`. The other citations I read match the tree: `approval_issuance.rs:391-393`, `execution.rs:84-97`, `:297`, `:433`, `clock.rs:28`, `:373-377`, `:386`, `:544`, `:585-587`, `:817-818`, `clock.md:92-99`, `:168-169`, `server.rs:58-60`, `local_cli.rs:325`, `local.rs:873`."
- file: .engineering/planning/story/an-approved-write-survives-one-refused-clock-acquisition.md
  line: 99
  category: scope
  severity: note
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "Change 4 edits `contracts/service/clock.md` and names no consumer of the changed text (repo rule: a contract change names its consumers). The files that read clock.md or its retry and deadline statements are `contracts/service/local-mutations.md` (`:161`, `:190`, `:287`, which speak of a 'fresh' reading and a settlement clock), `ess/domains/clock.yaml:73` ('no reply within the shared deadline'), `contracts/cli/v1alpha1/semantics.md:131` ('asks every source once', a `clock-check` text) and `apps/connectors/src/local.rs:873`. Fix: add one line to change 4 saying these read the text, that none of them changes (a repeat is still a fresh reading), and that the `clock-check` statement stays true because `clock-check` keeps one attempt."
```

Out of my lane: statement 2 is red before the change only because `acquire_within` does not exist, so it is a compile failure and not a behaviour check (acceptance). Whether recovery's `Quiescent` token can be built in a unit test is for design and acceptance to judge.
