---
format: aep.planning-md/3
id: story:mcp-long-read-exits-under-load
kind: story
status: implemented
title: The local MCP server exits before answering a read longer than one lease when the host is loaded
relations:
- serves: vision:independent-contract-adapters
revision: 7
transitions:
- {from: "draft", to: "proposed", at: "2026-10-29T17:06:28Z", actor: "human:timo", revision: 3}
- {from: "proposed", to: "active", at: "2026-10-29T17:06:28Z", actor: "human:timo", revision: 4}
- {from: "active", to: "implemented", at: "2026-10-29T18:08:33Z", actor: "human:timo", revision: 6, decided_on: {"recorded":{"test_result":2}}}
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

## Cause

The server was wrong, not the test.

The stdio session's data lease runs 1900 ms and `Session::maintain` renews it every 500 ms on the
serve loop. Before the fix, a renewal connected to the owner through `owner::Client::connect_until`.
Its greeting read the store authority with `Metadata::authority_at_until`, which waits for the
exclusive `metadata.lock`. The owner holds that lock while it records work, and the store syncs with
`synchronous=FULL`. On a loaded host the owner kept the lock past the roughly 1.4 s of lease left at
renewal. The connect timed out, the serve loop ended, and the server exited with status 1 and
`mcp_unavailable` on stderr. A slow provider read made this likelier because the owner records
work around it.

Evidence:

- `adversary_a_store_held_past_one_lease_leaves_the_session_ready` holds `metadata.lock` for 3 s
  under an idle session. The server exited before the fix (red 1 of 1) and stays ready after it.
- Instrumented renewals (diagnostics since reverted) spent their time in the authority read: up
  to 1275 ms of a 1395 ms budget on a loaded host.
- CPU-only pinning (`taskset`, `nice -n 19`, 4 `yes` workers) did not reproduce the natural
  failure (0 of 20 single-test runs, 0 of 3 whole-binary runs). The likely missing ingredient is
  disk and fsync contention from other sessions' builds, but that is inferred, not verified.

Fix (cb6527c56, 5ba31999e):

- A renewal greets the owner by the authority the session was admitted under
  (`Client::reconnect_until`) and does not wait on the store lock.
- When the lock is free at the call, the renewal still compares the store's authority, so a
  replaced store ends the session (`adversary_a_store_replaced_under_the_owner_ends_the_session`).
- When another holder has the lock, the comparison waits for the next renewal, 500 ms later.

Second cause, found by the pinned whole-binary runs of `local_mcp_server`:
`stdin_close_refuses_a_call_not_yet_at_the_owner` failed 2 of 10 times at edb385bb3. Admission
read only the flag the serve loop sets when it reads the end of file. Under load the worker
passed that check first, and the call reached the owner after the client had closed stdin. One
run answered success. In the other, the reply missed the unrenewed lease and the exit was 1,
which the docs specify for that case. Since 1d2e8f468, admission also polls the input for
POLLHUP. The unit test `admission_refuses_once_the_peer_closed_its_input` was red before that.
