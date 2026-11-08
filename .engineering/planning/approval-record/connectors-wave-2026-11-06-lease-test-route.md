---
format: aep.planning-md/3
id: approval-record:connectors-wave-2026-11-06-lease-test-route
kind: approval-record
status: approved
title: 'Wave 2026-11-06: route for the lease adversary test that failed twice in the gate'
relations:
- decides: story:provider-programs-run-on-macos
- decides: story:owner-bounded-read-realization-matches-projection
revision: 2
transitions:
- {from: "draft", to: "approved", at: "2026-11-06T12:40:47Z", actor: "human:operator", revision: 2}
---
## Decision (orchestrator, 2026-11-06)

The staged gate of wave 2026-11-06 stopped at S3: `apps/connectors/tests/connection_lease_adversary.rs`
`adversary_a_hanging_early_recollection_does_not_cost_a_read_its_current_evidence` failed in S3 (admission timeout
after 9.7 s, 0 dispatched reads) and again in a targeted re-run (10.3 s), with the host's load average at 13 to 22.
The wave's source diff does not touch the connection lease.

The directing session `b10x-chef-orchestrator` chose option A:

> Option A (record this in your store). Run apps/connectors/tests/connection_lease_adversary.rs once on main 52b4fe8f1 in a scratch tree (through build-slot.sh, ~2G, discard after). Fails there too: pre-existing, load-dependent — file a draft story naming the test and both timings, finish S4a-S5c, open the bot PR, ci-waits row, end your turn. Passes on main: find what in the wave changes it and report before any fix.
