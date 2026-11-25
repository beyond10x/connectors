---
format: aep.planning-md/3
id: review-result:seven-cli-shorten-design-reread-revision-10
kind: review-result
status: active
title: Design critic, re-read of the seven-cli-shorten story at revision 10
relations:
- reviews: story:seven-cli-tests-shorten-their-waits-or-a-production-wait
revision: 1
---
Verdict: needs-revision, for story:seven-cli-tests-shorten-their-waits-or-a-production-wait at
revision 10.

Read-only re-read by the design critic at revision 10, from main. Nothing was run. The round-3
record of this critic lists the story as a bare `approve`, so there was no note to apply. The
findings are new at revision 10; all apply the rule on windows: a window in which a test must
finish work stays at least 2 s wide, or a slow machine makes the test slower and not red.

## Findings

1. Blocker. Change item 1 (story lines 84 to 87), with
   `apps/connectors/tests/owner_cancel_adversary.rs` lines 795 to 809. The hold becomes 1,500 ms,
   and the story leaves the 1,500 ms sleep at line 803 between the send of read 4 and its cancel.
   The cancel then lands at about the moment the held launch ends, so the test no longer tests
   "cancelled during its launch", and a loaded host turns it into a race. Today the cancel comes
   at 1.5 s of a 5 s hold: a margin of 3.5 s. The sleep of line 809 is not "as today" either:
   today it is 7 s after the cancel, the hold's remaining 3.5 s plus 3.5 s.
   Settles it: name the sleep of line 803 as changed; state a margin of at least 2 s between the
   cancel and the end of the hold (keep the sleep at 1,500 ms with a hold of at least 3,500 ms,
   or cut the sleep and widen the hold); add the margin to acceptance 2 and derive the sleep
   after the cancel from it.
2. Blocker. Change item 6 (story lines 131 to 139) and acceptance 2, with
   `apps/connectors/tests/owner_tether_adversary.rs` lines 162 to 185 and
   `crates/connectors-host/src/local/owner/transport.rs` lines 1915 to 1930. The expected exit is
   about 250 ms (the owner's check interval) plus 1,000 ms of drain: about 1.25 s. The upper
   window is 2,000 ms: 0.75 s of slack. Today the exit is at about 5.25 s against a limit of 7 s:
   1.75 s. A slow runner makes this red.
   Settles it: an unbounded drain exits at about 10 s, so an upper window of 4 s or more loses no
   mutant. Widen it and recompute the test's estimate (2 s in the story).
3. Blocker. Change item 5 (story lines 115 to 130, 150 to 157, 204 to 205), with
   `crates/connectors-host/src/local/clock.rs` line 30 and `apps/connectors/tests/local_cli.rs`
   lines 325 to 393. A prompt source gets 800 ms for its exchange and verification where it has
   2 s today; rows 1 and 2 have three prompt sources, in a debug build. A forge runner took more
   than 1.5 s for 0.2 s of work (pull request 296), and a 200 ms deadline was missed on Windows on
   2026-11-25. Acceptance 4 measures it only on the implementor's host, and the story says so
   (lines 316 to 318).
   Settles it, one of two:
   (a) keep `QUERY_TIMEOUT` at 2 s and run the three rows concurrently, each with its own silent
       source (the `asked + 1` count at lines 367 to 377 is per source). The test then waits
       about 1.3 s plus 2 s, a slow machine only makes it slower, and the clock override, the new
       `clock_unavailable` variant and acceptance 3, 4 and 6 are not needed;
   (b) keep the override and state it as an accepted risk with a reason.
4. Note. Change item 2 (story lines 88 to 92). A page keeps 1,800 ms for everything but the
   provider, under 2 s, and the test's own comment (`apps/connectors/tests/invoke_walks.rs` line
   781) says a loaded host takes a lot of it. The 600 ms by which the third page passes the cutoff
   only grows on a slow machine, so that side is safe. `--deadline-seconds 4` with `page_ms` 1,500
   gives 2.5 s of slack and 4.5 s against the cutoff, for about 0.9 s more.
5. Note. Change item 3 (story lines 93 to 95), with `owner_cancel_adversary.rs` lines 914 to 927.
   The sleep at line 922 is a negative check: it must outlast the owner dispatching a wrongly kept
   queued read to the provider, and a slow machine would make the test green when it should be
   red. The story gives no reason for 300 ms and 500 ms and no step they outlast.
6. Note. Change item 5 (story lines 124 to 130), with `local_cli.rs` lines 58 to 68. The new
   function "builds its own command with the arguments of `command_from`" and so drops the
   ETXTBSY retry loop of lines 58 to 68. The binary is not freshly written, so the risk is small.
   Say whether the new function keeps the retry or why it can go without it.
7. Note. Change item 4 (story lines 107 to 109). The new upper bound is only "a value under the
   production 10 s", with no number, and acceptance 2 and 5 do not carry it. Name it: process
   start of the copied CLI under load plus the 500 ms deadline must fit inside it with at least
   2 s to spare, and it must stay clear of 10 s.

## Checked and right

- Every cited line matches main (`config.rs` 1597, 1627, 2296; `transport.rs` 1923; `clock.rs` 30
  and 69 to 77; `local_cli.rs` 40 to 69, 153 to 171, 237, 312, 325 to 393; the cancel file at 274,
  795, 809, 905; `invoke_walks.rs` 784; `catalog_provider_failure.rs` 105 and 201 to 215;
  `owner_tether_adversary.rs` 71 and 162; `standin.rs` 447 to 473). `Installation.root` is public.
- The `clock.rs` split has one reader; the test build keeps `set_test_query_timeout`.
- Edges: the helper, the slow-owner story and the five-host story are `depends_on`; the
  eleven-lease story depends back on this one; no cycle over 4 edges walked.
- The arithmetic 64.2 − 10.2 = 54.0 s and 24.5 s holds.
- The variable names carry `CONNECTORS_TEST_OVERRIDE_`.
