---
format: aep.planning-md/3
id: story:lease-early-recollection-relaunch-margin
kind: story
status: implemented
title: The hanging early recollection lease test holds on a loaded host
tags:
- from-finding
relations:
- informed_by: approval-record:connectors-wave-2026-11-06-lease-test-measure
- decomposes: epic:zero-open-issues-and-store-defects
- serves: vision:independent-contract-adapters
scope:
- confidence: inferred
  path: apps/connectors/tests/connection_lease_adversary.rs
- confidence: inferred
  path: crates/connectors-host/src/local/owner/supervisor.rs
revision: 7
transitions:
- {from: "draft", to: "proposed", at: "2026-11-09T21:59:39Z", actor: "human:operator", revision: 2, executor: "agent:connectors-zero-issues", correlation: "connectors-zero-issues"}
- {from: "proposed", to: "active", at: "2026-11-09T21:59:40Z", actor: "human:operator", revision: 3, executor: "agent:connectors-zero-issues", correlation: "connectors-zero-issues"}
- {from: "active", to: "implemented", at: "2026-11-11T06:19:13Z", actor: "human:operator", revision: 7, decided_on: {"recorded":{"test_result":1}}}
---
## Why

`apps/connectors/tests/connection_lease_adversary.rs`
`adversary_a_hanging_early_recollection_does_not_cost_a_read_its_current_evidence` failed three times on a loaded
host during the 2026-11-06 wave's local gate, each time with the read refused `timeout` at stage `admission` and 0
dispatched reads, after 9.7 s (gate stage S3), 10.3 s (targeted re-run) and 12.1 s (paired single-test run). Load
average was 13 to 22. The same test passed 4 of 4 runs on main `52b4fe8f1` and 2 of the 5 on the wave branch.

Measured cause, not the wave's code: an instrumented pair (the test binary run directly under `strace -f -tt`,
one run on each tree, load 17 to 21) passed on both and showed the same phases: the read's CLI starts, the early
recollection's probe hangs, and the owner launches a new adapter 5.65 s (main) and 5.95 s (wave) later, out of the
about 9 s of evidence the read was admitted on. The early recollection takes half of the remaining evidence
(`crates/connectors-host/src/local/owner/supervisor.rs:1488-1497`, `2 * EARLY_READ_MARGIN_MS`), and the relaunch at
`supervisor.rs:1596` must then start the adapter and finish its handshake within what is left. The adapter in this
test is the 133 MB test binary itself, started from the host's private copy; on a loaded host that start plus the
handshake outlasts the remaining evidence. The wave's diff touches none of the read, recollection or launch path.

## Outcome

The test holds its contract on a loaded host: either the fixture gives the relaunch a margin that does not depend
on host load (for example a longer evidence lifetime with the same proportions, or a small adapter fixture), or the
owner reserves a launch margin out of the remaining evidence before it bounds the early recollection. Which of the
two is a decision for the story's owner; the test's assertion is not weakened.

## Notes

Pre-existing, filed as a draft by the 2026-11-06 wave (approval-record:connectors-wave-2026-11-06-lease-test-route,
approval-record:connectors-wave-2026-11-06-lease-test-measure). No wave takes it before every planned wave is
delivered.

## Wave status

Merged 8caba7717f (commit 07d2443042), 2026-11-10. Decision (wave coordinator): the fixture route, not an owner launch margin.

- Change: `LIFETIME_MS` 20 000 → 60 000 in `apps/connectors/tests/connection_lease_adversary.rs`, same proportions, assertion unchanged; the doc comment states the margin it depends on. `supervisor.rs` untouched.
- Measured on the shared host, unpinned, normal nice, test binary run directly, base and change interleaved, load average 37.7 to 96.2: base 16 of 20 (one `timeout` at `admission` after 30.55 s, three `not_granted` at `admission` after 8.0 to 9.3 s, all with 0 dispatched reads); change 20 of 20. `connection_lease_adversary` 7 passed.
- Not held: pinned to one CPU with four busy workers at nice 19, the change passed 10 of 20 (20 000: 2 of 3 failed; 40 000: 3 of 6 passed), failing at admission after about 18 to 20 s, before the evidence ran out; that cause was not confirmed. A likely factor is the 10 s cap on the launch's executable capture (`crates/connectors-host/src/local/runtime/process.rs`), which a longer lifetime does not move.
