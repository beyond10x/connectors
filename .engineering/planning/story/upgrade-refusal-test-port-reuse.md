---
format: aep.planning-md/3
id: story:upgrade-refusal-test-port-reuse
kind: story
status: draft
title: The upgrade refusal test does not race port reuse
relations:
- serves: vision:independent-contract-adapters
revision: 1
---
## Observed
`crates/connectors-host/tests/upgrade_adversary.rs:386`
(`upgrade_refusals_before_sending_are_not_sent`) failed once on a busy host with "connect refusal
was not not_sent", and passed 3 of 3 reruns of the same binary on the same commit. The test binds
`127.0.0.1:0`, drops the listener and then connects to that port, expecting a refusal; another
process on the host can take the freed port between the drop and the connect, so the connect
reaches a live listener instead.

## Acceptance
- The refused-connect case uses an address no other process can claim (for example a listener
  kept bound with a zero backlog, or a reserved port the test owns for its whole run), so the
  test cannot pass or fail on another process's port use.
- The test still fails when the upgrade path sends a request after a connect refusal.
