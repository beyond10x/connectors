---
format: aep.planning-md/3
id: story:host-gate-load-sensitivity-w3
kind: story
status: draft
title: Three more host and adapter tests bound their own waits
relations:
- serves: vision:independent-contract-adapters
revision: 1
---
## Acceptance

Each test below passes the full gate on a host at load average 30 or more, with every wall-clock
bound it waits on set by the test:

- `local::owner::approval_issuance::tests::clock_exchange_precedes_leases_and_policy_is_rechecked_after_network`
  (`crates/connectors-host/src/local/owner/approval_issuance/tests.rs:497`)
- `adapters/sql/tests/local_runtime.rs:372`
- `crates/connectors-host/tests/service.rs:377`

## Observed

Wave 3 (2026-09-27): each failed one gate run at load 31–40 and passed alone (the first 3 of 3 at
load 33) and in the next full gate.
