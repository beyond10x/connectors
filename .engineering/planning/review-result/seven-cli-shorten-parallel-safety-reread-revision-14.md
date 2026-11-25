---
format: aep.planning-md/3
id: review-result:seven-cli-shorten-parallel-safety-reread-revision-14
kind: review-result
status: active
title: Parallel-safety critic, re-read of the seven-cli-shorten story at revision 14
relations:
- reviews: story:seven-cli-tests-shorten-their-waits-or-a-production-wait
revision: 1
---
Verdict: approve, for story:seven-cli-tests-shorten-their-waits-or-a-production-wait at revision
14 of its front matter (the body calls itself revision 11). The findings below are notes; none
blocks.

Read-only re-read by the parallel-safety critic, from the planning tree of the revision and the
code on main. Nothing was built or timed. The helper story's code was not read, only its scope and
edges.

## Findings of the revision-10 record

1. Launch sleep against the hold (blocker): applied. The hold is 3,000 ms and the sleep before the
   cancel is 1,000 ms, which leaves the 2 s margin the record asked for. The last sleep is
   recomputed to 4 s after the cancel.
2. Walk page window (blocker): applied. Pages of 1,500 ms under 4 s leave 2.5 s per page, and the
   walk's total of 4.5 s is past 4 s.
3. Clock query window of 800 ms (blocker): applied differently. There is no override and the
   production 2 s stays. The rows run at once, each with its own silent source.
4. Tether exit window (blocker): applied. The windows are 1,000 ms and 4,250 ms: the drain, the
   interval and 2 s of slack.
5. Provider upper bound (note): applied. The bound is 6 s.
6. Which process reads the print variable (note): applied in part; see new note 1.

## New findings, all notes

1. Change item 4 and "Not established". `setup check` runs `installed_provider_runs` in the CLI
   process: `apps/connectors/src/local.rs:773` sits in the CLI crate and reads the config before
   it. The case needs only `Command::env` on its own command; the owner forwarding does not matter
   here. Settles it: replace "the implementor follows in which process" with "the CLI".
2. Change item 4. `Installation` has a private `root` (`support/standin.rs:413`). The case can
   derive it from `config()` (line 447) and `provider()` (line 517) without editing `standin.rs`.
   `run` (line 451) runs the copy `<root>/bin/connectors`, because the provider is found beside
   the CLI. The case's own command must use that copy and not `CARGO_BIN_EXE_connectors`. Settles
   it: name the binary path in the item.
3. Change item 5. The story says "a variant of `start_owner` sets the one variable". Settles it:
   state that this is `Command::env` on the spawned owner (`owner_tether_adversary.rs:71-97`) and
   never `set_var`, which would reach sibling tests when they run as threads.
4. Change item 5. The first assertion ("no exit within 1,000 ms") is safe only if the owner has
   counted the client inside the test's 300 ms sleep (line 169 today). If the owner is slower than
   that, it has an empty drain and leaves at about 250 ms, and the test is red. The 2 s check of
   today has the same weakness. Settles it: lengthen that sleep, or wait until the client is
   counted. Also: `BOUND + 2 s - removed.elapsed()` is a `Duration` subtraction that panics on
   underflow; the new form uses `saturating_sub`.
5. Acceptance 3. A threshold of "under 7 s" read in the forge job can fail a correct test on a
   loaded runner. Settles it: 8.5 s, which rows one after another cannot reach; that is the proof
   the story already states.
6. Scope. Two other draft stories write `crates/connectors-host/src/local/config.rs` with no edge
   to this one: `definition-files-in-user-config` and `openrouter-inference-key-profile`. The
   active `connections-list-answers-through-the-owner` writes `transport.rs` with no edge either.
   Their stated regions (definition loading, migrate, the request kind) are not
   `Executable::print` at line 2296 or the drain in `accept` at line 1923. Nothing needs ordering;
   the Scope may say "disjoint regions".
7. Doc comments to update. Two lines in `owner_cancel_adversary.rs` still say 5 s: line 770 "held
   5 s by the fixture" and line 808 "Past the held launch (5 s)". The case comment at
   `catalog_provider_failure.rs:198-199` says "ten-second deadline ... near ten seconds".
   Acceptance 2 covers comments beside values, not these lines.

## Checked and found right

- Launch test (`owner_cancel_adversary.rs:795-822`): the cancel arrives at least 2 s before the
  hold ends; the assertions at lines 811 and 816 can fail only if the test thread stalls more than
  2 s between the send and the cancel; nothing else reads 5,000 ms, the 7 s or `START_DELAY`
  (written at line 795, read at line 222, by this test alone); the 20 s read budget is untouched.
- Walk (`invoke_walks.rs:784-805`): `page_ms` is set by this test alone (line 786) and read by
  the fixture (line 344); the expected output is the same for any page time.
- Clock rows in `std::thread::scope`: the shared `good`, `wrong_key` and `wide` sources are warmed
  servers whose one thread answers requests in turn; `clock_root` makes its own temporary root per
  row; `command` takes no shared state; `clock-check` makes one attempt
  (`apps/connectors/src/local.rs:873`); a silent source per row keeps "asked once" meaningful (0
  then 1); `slow_first_clock_source` answers late only for its first request, and `answering`
  (lines 139-150) takes that request before the rows start.
- Tether: the owner leaves at about the removal plus the interval plus the drain; the 5 s `until`
  is at `transport.rs:1923`; an unbounded drain leaves at about 10 s; an owner that ignores the
  variable leaves at about 5,250 ms, 1 s past 4,250 ms.
- Provider case: a hanging provider gives `Deadline` at any value; 6 s against 10 s leaves 4 s.
- Typed `scope:` has 7 paths and matches the Scope section; no `clock.rs`; the edge to
  `five-host-unit-tests-shorten-the-waits-they-chose` is gone.
- `transport.rs` order: helper, idle-exit, slow-owner (edges present), this story, then the
  eleven-lease story (its edge to this story is present).
- `local_cli.rs`: the regions of the idle-exit story and the 100-run story are disjoint from lines
  325-393; the changed line of the idle-exit story matches.
- Sums recomputed: 8.0 + 6.4 + 5.0 + 1.5 + 2.6 = 23.5, plus 7.0 = 30.5, bound 36; the parent's
  338.8, 346.2, 49.4, 246.2, 362.2, 351.2, 1,649.2 and 412.3.
