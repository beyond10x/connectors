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
revision: 8
transitions:
- {from: "draft", to: "proposed", at: "2026-11-09T21:58:30Z", actor: "human:operator", revision: 2, executor: "agent:connectors-zero-issues", correlation: "connectors-zero-issues"}
- {from: "proposed", to: "active", at: "2026-11-09T21:58:31Z", actor: "human:operator", revision: 3, executor: "agent:connectors-zero-issues", correlation: "connectors-zero-issues"}
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

## Wave status

Merged 023b00c4b9 (commit 2772ff619b), 2026-11-10.

- `clock_exchange_precedes_leases_and_policy_is_rechecked_after_network`: every wall-clock bound
  it waits on is now set by the test (one 120 s `CONTENDED_LOCK_WAIT`). Not reproduced by
  contention alone (20/20 at 1 CPU with 40 busy workers); with a 7 s delay injected into
  `clock::exchange` the base passed 0 of 20 under 2-CPU contention and the change 20 of 20. Under
  1 CPU, 40 workers and the injection it still failed 4 of 20, on the metadata store's 30 s SQLite
  busy timeout in setup and an `Unavailable` from `issue`: product waits the test cannot set.
- `sql_fixture_repeated_control_packets_preserve_provider_refusal` (`adapters/sql/tests/local_runtime.rs`):
  the base failed 9 of 20 under 1-CPU contention on the fixed 10 s start cap inside
  `Child::spawn`; the test now starts its adapter through `Child::spawn_within` with its own
  budget and bounds its other waits with one 120 s `WAIT`; 20 of 20.
- `crates/connectors-host/tests/service.rs` (by line drift,
  `federation_refreshes_atomically_without_replaying_and_rotates_credentials`): unchanged. It sets
  no wall-clock bound of its own and waits only on the client's 20 s request / 5 s connect and the
  server's 20 s timeouts, which have no knob; it passed 20 of 20 at 1 CPU with 40 workers.
- Not met: "every wall-clock bound it waits on set by the test" for `service.rs`, which would need
  a new product timeout knob; and the full gate at load 30 or more, which runs at wave end. The
  story stays active until both are settled.

## Finding, 2026-11-11 (wave/load-and-flakes)

No change made: the three named tests are already bounded on base 4265b2ec0b5. The Wave status
above names the wrong tests for two lines; resolved against the wave-3 files (952380f1c01):

- approval_issuance/tests.rs:497 is `clock_exchange_precedes_leases_and_policy_is_rechecked_after_network`;
  every wait comes from its 120 s `CONTENDED_LOCK_WAIT` (file unchanged since 2772ff619bf).
- adapters/sql/tests/local_runtime.rs:372 is `a_dispatched_read_reaches_the_database_and_returns_its_refusal`;
  `Child::spawn_within(.., WAIT)`, a 120 s deadline and a 120 s cancellation poll.
- crates/connectors-host/tests/service.rs:377 (now :456) is
  `invalid_identifiers_cannot_inject_log_lines_or_terminal_controls`; its failure was a tracing
  callsite cached as disabled by another thread, fixed by d5ce4c36449 (2026-09-30). Removing that
  guard fails it every time (1 failed); with it, 1 passed.

Single runs, one thread, all passed: the three above, `federation_refreshes_atomically…` and
`sql_fixture_repeated_control_packets…`.

Open: `federation_refreshes_atomically…` waits on the client's hard-coded 20 s request and 5 s
connect timeouts (crates/connectors-client/src/lib.rs:42-43) and the server's 20 s
(server.rs:466); bounding them from the test needs a product knob. The acceptance line "passes the
full gate at load 30 or more" cannot be observed on the shared workstation, where the operator forbids load
tests: decision-blocker:load-acceptance-without-load-tests.
