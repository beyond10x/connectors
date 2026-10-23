---
format: aep.planning-md/3
id: review-result:w1022-u3-adversary-r1
kind: review-result
status: active
title: 'Adversary pass 1, wave 20261022 U3: MCP stdio runtime re-landing'
summary: 'break: 3 warnings, 3 notes (F1-F4, J1, J2); F1-F4 and J1 fixed in the unit commit'
relations:
- reviews: story:mcp-stdio-runtime-branch
revision: 1
---
Adversary pass 1 on U3 (story:mcp-stdio-runtime-branch). **Verdict: red**, 3 warnings, 2 notes (+1 note without test). Cases executed 2259→2265, red 4. New file `apps/connectors/tests/adversary_local_mcp_server.rs` (807 lines), jail harness with 12,000-byte tool descriptions.

| # | sev | test | red line | cause |
|---|---|---|---|---|
| F1 | warning | `adversary_cancel_racing_a_partly_written_reply_keeps_the_session_ready` | session ended, stdout line lengths [4096], stderr mcp_unavailable | `apps/connectors/src/mcp/stdio.rs:253-257` (`q.offset > 0` → `Err(Code::Interrupted)`) |
| F2 | warning | `adversary_a_read_longer_than_one_lease_is_answered_and_the_session_stays_ready` | no reply to a 3 s read (execution_ms 20000, provider_ms 15000) after 1.74 s | `stdio.rs:277` (`until.min(session.until()?)`), `stdio.rs:135-136`, `session.rs:256` |
| F3 | warning | `adversary_a_reply_drained_after_one_lease_still_arrives` | a reply drained 2.5 s late ended the session, stdout [4096] | `stdio.rs:277` (lease cap sets the reply's `until`), `stdio.rs:146-147` |
| F4 | note | `adversary_legacy_ping_before_initialized_is_answered` | ping between initialize and initialized refused -32600 | `adapters/mcp/runtime/src/protocol.rs:93` |
| J1 | note | none | stdin close drops replies to requests already read, exit 0 | `stdio.rs:206-208`, `:305` `queue.clear()` |
| J2 | note | none | lease timeout and cancellation teardown both print `mcp_unavailable` on stderr | — |

- F1 contradicts contract §3.4 (`adapters/mcp/contracts/server/v1alpha1/semantics.md:305-316`): the race "is expected and is not an error", session stays `Ready`.
- F2/F3 contradict row `renewed-lease-keeps-a-long-request-ready` (`semantics.md:245-291`) and §3.5 (`:332`). The guide admits F2 at `docs/local-mcp-stdio.md:114-117`; F3 is undocumented.
- Control `adversary_control_a_prompt_reader_gets_the_reply_and_the_ping` passes.
- Not broken: admission and name canonicalization, curation duplicates and absent operations, owner re-admission, bounded reads (no gzip, size cap, timeout covers body), duplicate JSON keys and depth limit, unknown protocol version, cancel for unknown/finished id, custody `read_for_until`, non-Linux `mcp_unavailable`, `"mcp": "deferred"`, ignored.rs entry.

## Disposition (implementor round 2, uncommitted in conn-w1022-u3)

| # | disposition |
|---|---|
| F1 | fixed: a cancel removes only replies not yet begun; a partly written reply finishes (`apps/connectors/src/mcp/stdio.rs`) |
| F2 | fixed: lease cap on the request deadline removed; `Session::until` private |
| F3 | fixed: the reply's write deadline is the request's own `request_milliseconds` |
| F4 | fixed for `2025-11-25`: ping answered between `initialize` and `notifications/initialized` (`adapters/mcp/runtime/src/protocol.rs`); other requests still `-32600` |
| J1 | fixed: on stdin EOF pending replies are written while the lease live at the close lasts (sessions §4.1, server `semantics.md:332`); jail case `stdin_close_still_answers_requests_already_read` |
| J2 | not in scope |

Red then green per case is in the implementor's report; package gates: connectors 289 passed, connectors-mcp 50, connectors-host 693, clippy and fmt 0.
