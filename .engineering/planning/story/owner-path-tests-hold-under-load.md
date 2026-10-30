---
format: aep.planning-md/3
id: story:owner-path-tests-hold-under-load
kind: story
status: draft
title: Owner and store path tests hold under machine load
relations:
- informed_by: story:connectors-do-short-invocation
revision: 2
---
## Problem

The connectors-host library test
`local::registry::tests::a_read_invoke_against_a_grown_store_replays_it_at_most_once`
(`crates/connectors-host/src/local/registry/tests.rs:1770`) fails under machine load. On
2026-10-30, the full gate on the D218 integration head `df4d2e1d1` ran it at load average 50. It
failed with `called Result::unwrap() on an Err value: OutcomeUnknown`, then passed alone (1/1).
`concurrent_callers_share_one_exchange` failed the same way earlier in the wave, under load
averages of 30 to 56. The path is the owner and store path that the owner-start deadline fix
(0.59.3) touches.

On the gate rerun, `apps/connectors/tests/local_mcp_server.rs`
`stdin_close_still_answers_requests_already_read` failed at load average 38. The server was still
running, its stderr said `mcp_unavailable`, and no reply came within 5 s. Run alone twice, it
passed once and failed once. The wave did not change the MCP server code.

## Outcome

The test either passes at any machine load, or it names the deadline it depends on and fails with
that deadline in its message. A slow machine yields a timing result the test expects, never
`OutcomeUnknown`.

## Acceptance (draft)

- The test passes 20 runs in a row with every core busy (for example under `stress-ng --cpu 0`).
- Where the outcome under load is legitimately unknown, the test asserts that outcome and its
  next action instead of unwrapping.
