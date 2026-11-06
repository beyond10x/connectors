---
format: aep.planning-md/3
id: approval-record:connectors-wave-2026-11-06-lease-test-measure
kind: approval-record
status: approved
title: 'Wave 2026-11-06: measure the lease test''s slower wave runs before routing'
relations:
- decides: story:provider-programs-run-on-macos
- decides: story:owner-bounded-read-realization-matches-projection
revision: 2
transitions:
- {from: "draft", to: "approved", at: "2026-11-06T12:50:48Z", actor: "human:timo", revision: 2}
---
## Decision (orchestrator, 2026-11-06)

The lease adversary test passed 4 of 4 times on main `52b4fe8f1` and 2 of 5 on the wave branch `9413efbdd`. In three
alternating single-test runs the wave took 29.2 to 31.4 s against 22.6 to 23.5 s on main at comparable load (main's
one run at load 24 took 48.4 s and passed).

The directing session `b10x-chef-orchestrator` decided:

> A first: the wave is 6-8 s slower per run even at close load (29.2 s at 13.8 vs 22.6 s at 12.2), which is too consistent to wave off for 2 minutes of measurement. Run the instrumented pair (--nocapture, timestamps) and locate the 6-8 s. If it is outside the wave's code (e.g. a test fixture or build-profile difference, adapter startup unrelated to the diff): go B at once — draft story with the test, the timings (9.7, 10.3, 12.1 s), supervisor.rs:1488 margin and the measured cause; finish S4a-S5c, bot PR, ci-waits row, end your turn; discard conn-w1106-base. If it is in the wave's code: report to me before any fix.
