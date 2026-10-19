---
format: aep.planning-md/3
id: story:connection-lease-tests-pin-the-window
kind: story
status: draft
title: The lease tests pin the half-life boundary and prove the join they claim
summary: Two lease tests pass for windows and orderings other than the ones they name
relations:
- serves: vision:independent-contract-adapters
scope:
- confidence: cited
  path: apps/connectors/tests/connection_lease.rs
- confidence: cited
  path: apps/connectors/tests/connection_lease_adversary.rs
revision: 4
---
## Outcome

The lease tests fail if the early recollection window moves off half the evidence lifetime, and the
two joining tests fail if the revalidation did not in fact join a read's probe in flight.

## Why later

The second review of release 0.54.0 (2026-10-19, lane s4) found that
`apps/connectors/tests/connection_lease.rs:77-90` reads only at age ~0 and
`connection_lease_adversary.rs:90-116` at 11 s of 20 s, so a window of 90 % of the lifetime passes both;
and that `connection_lease_adversary.rs:138-163` and `:283-299` sleep 3 s and assume the read's probe
started, so a revalidation that probes alone also counts one probe. Both need the built CLI and a
user-namespace jail, so the verifier traced them and did not run them.

## Acceptance

- A read at 9.5 s of a 20 s lifetime does not recollect; a read at 10.5 s does.
- Both joining tests wait until the read's probe is recorded before they revalidate, and assert it.
