---
format: aep.planning-md/3
id: review-result:seven-cli-shorten-parallel-safety-reread-revision-10
kind: review-result
status: active
title: Parallel-safety critic, re-read of the seven-cli-shorten story at revision 10
relations:
- reviews: story:seven-cli-tests-shorten-their-waits-or-a-production-wait
revision: 1
---
Verdict: needs-revision, for story:seven-cli-tests-shorten-their-waits-or-a-production-wait at
revision 10.

Read-only re-read by the parallel-safety critic at revision 10, from main. Nothing was run. The
round-3 record of this critic lists the story as `approve` with no note, so there was nothing to
apply. Round-2 finding 5 (`clock_unavailable` and a new function) is kept in revision 10.

## Findings

1. Blocker. Change item 1 (story lines 84 to 87). The story shortens the hold to 1,500 ms and
   leaves the sleep of 1,500 ms before `cancel(&mut peer, 4)` unchanged
   (`apps/connectors/tests/owner_cancel_adversary.rs:803`). The cancel was meant to land in a
   launch held 5 s: a margin of 3.5 s. With a hold of 1,500 ms it lands at about the moment the
   launch ends: a margin of zero. On a fast machine the read passes the launch before it is
   cancelled; the test loses its premise or fails `requests() == 2` (line 812).
   Settles it: name that sleep in item 1 and keep (hold minus that sleep) at 2 s or more; then
   recompute the final sleep (hold plus 2 s) and the estimate.
2. Blocker. Change item 2 (lines 88 to 92). The page window is 1,800 ms (3,000 minus 1,200),
   under the 2 s a window must have. Today it is 5 s (`invoke_walks.rs:784`).
   Settles it: a pair with at least 2 s per page and a walk total still past the deadline, for
   example pages of 1,500 ms under 4 s: 2.5 s per page and 4.5 s for the walk.
3. Blocker. Change item 5 (lines 115 to 130). Three prompt sources must answer inside the query
   window of 800 ms (`good` after its probe, `wrong_key` and `wide`, `local_cli.rs:325-345`).
   800 ms is below the 1.5 s the forge took in pull request 296 and below 2 s. A late source is
   counted `unreachable` and the test fails. Acceptance 4 (20 runs) is read on the implementor's
   host (story lines 316 to 318). `clock-check` makes one attempt
   (`apps/connectors/src/local.rs:873`), so a short timeout cannot add a second request to the
   silent source: that part is safe.
   Settles it: a query window of at least 2 s (item 5 then saves only the probe delay), or a test
   where a slow source makes it slower and not red.
4. Blocker. Change item 6 and acceptance 2 (lines 131 to 139, 224 to 225). The drain is 1,000 ms,
   the exit window 2,000 ms, the check interval about 250 ms
   (`owner_tether_adversary.rs:162-190`): the owner exits at about 1,250 ms, 750 ms of slack.
   Today the slack is 2 s (`BOUND + 2 s`, line 183).
   Settles it: an upper window of the drain plus at least 2 s plus the interval. An unbounded
   drain exits at about 10 s, so the test still catches it.
5. Note. Change item 4 (lines 107 to 110). The upper bound "a value under the production 10 s"
   has no number (`catalog_provider_failure.rs:211-215`, 15 s today). Name one number at least
   2 s above the 500 ms deadline and below 10 s.
6. Note. Scope (lines 286 to 297). The print override acts in whichever process runs
   `installed_provider_runs`, CLI or owner; the story says this was not established (lines 322 to
   324). If it is the owner, the case needs the helper story's forwarding; the `depends_on` edge
   exists. Say in the case which process reads the variable once the helper story has landed.

## Held

- Scope placement: 8 cited, 0 inferred, 0 unplaced.
- `transport.rs`: the `depends_on` chain is helper, idle-exit, slow-owner, this story, eleven;
  the order is stated and the regions are disjoint.
- `local_cli.rs`: the 100-run story (line 1691) and the idle-exit story (lines 972 and 1959)
  have no edge to this story; their regions are disjoint from this story's (153 to 171 and 325
  to 393). `clock_unavailable` has a third caller at line 368, inside the test this story edits.
- `clock.rs`: no other draft or active story writes it.
- `config.rs`, `invoke_walks.rs`, `catalog_provider_failure.rs`, `owner_cancel_adversary.rs`,
  `owner_tether_adversary.rs`: sole writer among the 19 decomposing stories.
- Reader beside producer: 3 traced, all 3 recorded by `depends_on`.
- Item 3 (queued-read test): the waits of 300 ms and 500 ms only weaken the mutant check on a
  slow machine; they do not make it red. `Command::env` is per process.
