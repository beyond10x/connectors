---
format: aep.planning-md/3
id: story:host-gate-load-sensitivity-w3
kind: story
status: active
title: Three more host and adapter tests bound their own waits
relations:
- serves: vision:independent-contract-adapters
- decomposes: epic:zero-open-issues-and-store-defects
scope:
- confidence: inferred
  path: adapters/sql/tests/local_runtime.rs
- confidence: inferred
  path: crates/connectors-host/src/local/owner/approval_issuance/tests.rs
- confidence: inferred
  path: crates/connectors-host/tests/service.rs
revision: 6
transitions:
- {from: "draft", to: "proposed", at: "2026-11-09T21:58:30Z", actor: "human:timo", revision: 2, executor: "agent:connectors-zero-issues", correlation: "connectors-zero-issues"}
- {from: "proposed", to: "active", at: "2026-11-09T21:58:31Z", actor: "human:timo", revision: 3, executor: "agent:connectors-zero-issues", correlation: "connectors-zero-issues"}
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
