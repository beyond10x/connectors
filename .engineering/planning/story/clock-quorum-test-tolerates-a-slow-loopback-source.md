---
format: aep.planning-md/3
id: story:clock-quorum-test-tolerates-a-slow-loopback-source
kind: story
status: draft
title: clock_check_below_quorum_refusal holds when a loopback clock fixture answers late on a loaded runner
summary: 'One red on main (Windows host run 41721788043): the first loopback fixture was counted unreachable instead of verified'
tags:
- from-finding
revision: 3
---
## Defect

`apps/connectors/tests/local_cli.rs:266` (`clock_check_below_quorum_refusal`) failed once on `origin/main`
`413c60efc`, Windows host run 41721788043, job 126216158541, 2026-11-03: expected source results
`["verified","unreachable","unverified","verified"]`, got `["unreachable","unreachable","unverified","verified"]`.
The first loopback clock fixture (`127.0.0.1`) was counted unreachable instead of verified. Wave D417 changed
neither the clock check nor this test, and the same tree passed on windows-latest in PR 236 (run 41711608302).
Inferred, not reproduced: the fixture answered past the check's per-source wait on a loaded runner. Found by the
v0.65.1 release verification (D433).

## Acceptance

- The cause is named from evidence: the fixture's answer time against the check's per-source wait, or another
  mechanism observed.
- The test no longer depends on a loopback fixture answering within that wait on a loaded runner, or the wait it
  relies on is stated and held, and it passes on windows-latest repeatedly.
