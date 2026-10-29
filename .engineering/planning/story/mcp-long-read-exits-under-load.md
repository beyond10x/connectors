---
format: aep.planning-md/3
id: story:mcp-long-read-exits-under-load
kind: story
status: draft
title: The local MCP server exits before answering a read longer than one lease when the host is loaded
relations:
- serves: vision:independent-contract-adapters
revision: 2
---
## Defect

`adversary_a_read_longer_than_one_lease_is_answered_and_the_session_stays_ready`
(`apps/connectors/tests/adversary_local_mcp_server.rs`, panic at :791) gets no reply to the
`tools/call` whose provider answers in 3 s. The local MCP server exits with status 1 before the
provider receives the request.

Observed on 2026-10-29 on Linux, tree `wave/d210` at `fcaa67e68` (ESS 0.75.0; the release commit
changes no code that this test reaches), with the host's load average at 25 to 36 from other
sessions:

- In the repository gate (`cargo run --locked -p connectors-build -- gate`): "after 6.618218162s
  the server is None, provider requests 1".
- Run alone, 11 runs at `nice -n 19` with `--exact`: 2 failed. One failure reported "after
  4.30923951s the server is Some(ExitStatus(unix_wait_status(256))), provider requests 0". Passing
  runs took 8.4 to 35.6 s.

## Outcome

The server answers a read that runs longer than one lease and stays `Ready` whatever the host's
load. The fix is in the server or the supervisor if one of them ends the session early, and the
cause is named.

## Acceptance

- The cause of the exit with status 1 under load is named with a reproduction.
- The test passes 50 of 50 runs on Linux under load (`nice -n 19`, at most 4 workers).
- No `#[ignore]`, no retry and no longer deadline unless the named cause calls for it.

## Further observations

The same tree failed the repository gate twice more on 2026-10-29, with the host's load average between 27 and 36. Each run stopped at a different timing test, none of them changed by the tree:

- 14:15Z: `connection_lease::a_revalidated_connection_serves_reads_across_three_evidence_lifetimes` (`apps/connectors/tests/connection_lease.rs:58`): "read 14, 25000 ms after the revalidation, was refused: ... not_granted".
- 14:43Z: `local_mcp_server::a_hidden_operation_is_not_listed_and_its_call_makes_no_request` and `local_mcp_server::stdio_lists_and_calls_an_admitted_read_on_2026_07_28` (`apps/connectors/tests/local_mcp_server.rs:522`): "the server did not reply".

`cargo test --workspace --no-fail-fast` on the same head at 14:23Z passed all three of these binaries.
