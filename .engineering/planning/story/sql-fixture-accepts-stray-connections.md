---
format: aep.planning-md/3
id: story:sql-fixture-accepts-stray-connections
kind: story
status: draft
title: The SQL adapter's fake server test fails when a stray connection reaches its port
relations:
- serves: vision:independent-contract-adapters
revision: 1
---
## Observed

2026-10-02, closing gate of wave 20261002a at 67fb9da30 from a managed worktree on a shared, loaded host:
`a_dispatched_read_reaches_the_database_and_returns_its_refusal` (`adapters/sql/tests/local_runtime.rs`) failed with
`assertion failed: startup.windows(7).any(|w| w == b"reader\0")` (:83) and `an invalid query opened a session,
left: 2, right: 1` (:372): the fake Postgres server accepted a second connection whose startup packet did not carry
the test's user. The same test passed 5 of 5 runs alone right after. No commit of the wave touched `adapters/sql`.

## Acceptance

- The fixture counts and asserts only connections that carry the test's startup packet (or binds so that no other
  process can reach it), and the test holds 50 of 50 runs under a parallel full workspace test run.
