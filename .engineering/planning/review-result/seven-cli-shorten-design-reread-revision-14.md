---
format: aep.planning-md/3
id: review-result:seven-cli-shorten-design-reread-revision-14
kind: review-result
status: active
title: Design critic, re-read of the seven-cli-shorten story at revision 14
relations:
- reviews: story:seven-cli-tests-shorten-their-waits-or-a-production-wait
revision: 1
---
Verdict: approve, for story:seven-cli-tests-shorten-their-waits-or-a-production-wait at revision
14 of its front matter (the body calls itself revision 11). Every finding is a note; none blocks.

Read-only re-read by the design critic, from the planning tree of the revision and the code on
main. Nothing was built or run.

## Findings of the revision-10 record

1. Launch test, cancel too near the end of the hold: applied. The cancel lands 2 s before the hold
   ends, the sleep at line 803 is named as changed, and the last sleep is derived (2 s + 2 s).
2. Tether test, exit window of 2,000 ms: applied. The window is 4,250 ms and the owner exits at
   about 2.2 s: 2 s of slack.
3. Clock test, 800 ms query timeout: applied by option (a). The three rows run at once at the
   production 2 s, each with its own silent source; `clock.rs`, the override and the five-host
   edge are gone.
4. Walk, 1,800 ms for everything but the provider: applied. A page keeps 2,500 ms, and the third
   page ends at 4,500 ms against a cutoff of 4 s.
5. Queued-read sleeps with no named step they outlast: resolved differently. The test is not
   changed, and the story says why.
6. ETXTBSY retry dropped from the clock test's new function: resolved, the function is gone. The
   same defect reappears for the provider case, see new finding 1.
7. Provider-case upper bound without a number: applied. The bound is 6 s and acceptance 5 carries
   it.

## New findings, all notes

1. Change 4, with `apps/connectors/tests/support/standin.rs:451-473`. The case's own command
   replaces `Installation::run`, and the story does not say what it keeps. `run` clears the
   environment, closes stdin and retries ETXTBSY (`standin.rs:454,460,466-472`). The binary is the
   one `fs::copy` wrote at `standin.rs:430-436`, so the retry is not idle there. `prerequisite` is
   not edited, so the case repeats its parse of the `setup check` output
   (`catalog_provider_failure.rs:105-117`) or goes without it. Settles it: say the command keeps
   `env_clear()`, a null stdin and the retry loop, with only the one variable added, and that the
   output is parsed the way `prerequisite` parses it.
2. Change 3, with `local_cli.rs:95-101,342`. The story says "one thread each" but not what a
   thread holds. `clock_root` runs `setup init`, a CLI process. Inside the thread, one row is in
   its 2 s window while the other two start processes. Settles it: build the three roots and the
   three silent sources before `thread::scope`, and run only `clock_unavailable` and the asserts
   in the threads. Sharing is otherwise safe: `good`, `wrong_key` and `wide` are shared read-only,
   the nonces differ, each row has its own temporary directory and `reap::Root`, and `asked` is
   per source.
3. Change 1 and Change 5, stale comments. `owner_cancel_adversary.rs:768` says "held 5 s", line
   793 "for 5 s", line 808 "(5 s)". `owner_tether_adversary.rs:156-161` is the doc comment, and
   line 184 the message "bounded at 5 s". Settles it: add them to acceptance 2's duty.
4. Change 5. The owner's check interval is 200 ms
   (`crates/connectors-host/src/local/owner/tether.rs:14`, `EVERY`), not 250 ms; the 250 ms is the
   test's own comment. The windows are still right. "A slow machine cannot turn this red" holds
   only if the owner has counted the client before it notices the loss, within about 300 + 200 ms;
   that is the same as today. Say so, or drop "cannot".

## Answers to the author's four points

- Launch test: the hold is read once, at adapter start (`owner_cancel_adversary.rs:222-227`). A
  slow machine starts the hold later, so the hold ends later than the cancel, which only weakens
  the check. An assertion fails only if the test thread or the owner stalls more than 2 s between
  the send and the cancel. Nothing else depends on 5,000 ms or the 7 s apart from the comments of
  finding 3; `START_DELAY` is written only at line 795.
- Clock test: the rows can run in threads; the clock query already runs one thread per source
  (`clock.rs:439-445`). "Asked once" stays meaningful: a second attempt would show 2 and each
  row's silent source is its own. The probe still takes the first answer, 2.5 s late
  (`server.rs:116-120`, `local_cli.rs:139-150`), before the rows start. `good` serves three
  requests on one thread, which is fast work.
- Tether test: 1,000 ms and 4,250 ms are right against `transport.rs:1918-1930` and
  `tether.rs:14`. An owner that ignores the variable leaves at 5.0 s plus the 0.2 s check
  interval, never before 5.0 s. An unbounded drain leaves at about 10 s. The test's owner inherits
  the environment (`owner_tether_adversary.rs:71-95` does not clear it), so a variant that sets
  the variable reaches it.
- Walk: `page_ms` is read only in this test's provider (`invoke_walks.rs:342-349`). The expected
  outputs do not depend on 3,000 ms or 8 s. `--deadline-seconds 4` is inside the allowed 1 to 120.

## Checked and right

- All cited lines match main: `config.rs` 1505, 1597, 1627, 2296; `transport.rs` 1923; `clock.rs`
  30; the lines of the cancel test, the tether test, the provider case and the walk; `local_cli.rs`
  325 to 393.
- `setup check` asks `installed_provider_runs` in the CLI itself
  (`apps/connectors/src/local.rs:773`), and one print is waited at line 2296.
- The 10 s at line 1505 is a different wait, reached from `endpoint.rs:427` and not from
  `setup check`.
- The story's arithmetic: 6.1 against 9.6, 8.0, 6.4, 5.0, 2.6, 23.5, 30.5, 47.0 and 36.
- The parent's sums: 338.8, 346.2, 246.2, 362.2, 351.2, 1,649.2, 412.3, and the five tests at
  49.4 s.
- The idle-exit story's one-line scope change matches the narrowed lines of the clock test.
- Edges: no cycle over 8 edges walked, stories outside the set included.
- The count of `CONNECTORS_TEST_OVERRIDE` in `local_cli.rs` is 0 on main.

Outside the design lane: acceptance 3 ("under 7 s" read from the forge) leaves 2.5 s over the
estimate; a slow runner could read a passing test as red. That belongs to the acceptance critic.
