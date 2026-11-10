---
format: aep.planning-md/3
id: review-result:wave-b-mcp-deadline-security-1
kind: review-result
status: active
title: Security review 1 on mcp-tool-deadline-keeps-the-server (single defect wave)
relations:
- reviews: story:mcp-tool-deadline-keeps-the-server
revision: 1
---
```
unit: story:mcp-tool-deadline-keeps-the-server, commit 8972d6166 (base 63e774d11) plus an uncommitted test file
verdict: red
cases: executed 17→29, red 1
origin: introduced 2, pre-existing 0, undecided 1
wrote-outside-worktree: <scratch>/units/mcp-tool-deadline-keeps-the-server/scratch/security/ (tail.rs, ontime-doc.txt, run-cases*.log, run-suite*.log)
needs-coordinator: yes
```

1. Diff: no source touched; untracked `apps/connectors/tests/security_mcp_deadline.rs` (and the adversary's `adversary_mcp_deadline.rs`).
2. Cases (harness copied from `adversary_local_mcp_server`):
   - `security_requests_past_their_deadline_each_get_one_reply_on_their_own_id`, green: store held, 2 s budget, four requests in one write (list with string id `"late-1"`, list, call, ping): each id gets exactly one reply (`timeout`, `capacity`, `capacity`, result); a second deadline in a row leaves the session up; clean exit.
   - `security_control_a_late_reply_to_a_paused_reader_keeps_the_session`, green: 20 s budget, store held past the deadline, client reads 0.5 s after it; `timeout`, next ping answered.
   - `security_an_on_time_reply_written_across_its_deadline_keeps_the_session`, red: same client and pause, owner answers at once. Alone: `on time: reply None, pinged false, exit Some(ExitStatus(unix_wait_status(256)))`, fails at `security_mcp_deadline.rs:882` with stdout `["4096 bytes after 20.504679801s: {\"id\":2,...` (cut-off frame) and stderr `mcp_unavailable`; `FAILED. 0 passed; 1 failed`, exit 101.
   - Below `execution_ms` (20 000) no tool is listed (projection.rs:243-247), so a short budget cannot make a reply large enough to block.
3. Suite `--no-fail-fast -p connectors --test security_mcp_deadline --test adversary_local_mcp_server --test local_mcp_server_write_refusal`: adversary_local_mcp_server `ok. 17 passed; 1 ignored`; local_mcp_server_write_refusal `ok. 8 passed`; security_mcp_deadline `FAILED. 3 passed; 1 failed`; exit 101. The run before (load 100-118) failed the story's own case `adversary_a_request_past_its_deadline_answers_timeout_and_the_session_continues` at `:542` ("the server did not reply", `mcp_unavailable`).
4. Findings:
   - F1 (`stdio.rs:179`, warning, NEEDS-CHANGE, undecided): an on-time reply still unwritten at its request's deadline ends the session with a cut-off frame and no reply, while a late reply gets `now + LATE_WRITE_GRACE` (`:164-171`) and an on-time one keeps `job.until` (`:172`). `docs/local-mcp-stdio.md:140-143` ("A request's timeout does not end the session") is untrue. Suggested: queue every work reply with `until = max(job.until, now + LATE_WRITE_GRACE)`, or qualify the doc. The Acceptance holds without it.
   - F2 (`adversary_local_mcp_server.rs:1161`, note, CONFIRMED, introduced): the story's case went red in 1 of 2 runs under load; inferred cause: the startup snapshot is bounded by `min(request_milliseconds, 10 s)` (stdio.rs:114-120), 2 s here.
   - F3 (`stdio.rs:164-171`, note, INFEASIBLE, introduced): a late success is replaced by `timeout`, which for a write would turn a known applied outcome into an unknown one; unreachable today (`tools/call` admits only reads, projection.rs:208-233, governed.rs:140; `local_mcp_server_write_refusal` 8 passed).
5. Cleared: reply ids for late, capacity and string-id replies; worker bound unchanged; queue byte bound unchanged (a late refusal is 264 bytes, one per request slot); a zero grace is caught by the story's case; cancelling a late request withdraws its unwritten reply; no provider write is reachable over MCP.

```findings
[
{"file":"apps/connectors/src/mcp/stdio.rs","line":179,"category":"boundary","severity":"warning","verdict":"NEEDS-CHANGE","origin":"undecided","message":"An on-time reply still unwritten at its request's deadline ends the session with a cut-off frame and no reply, while a late reply gets a 2 s grace, contradicting the new doc sentence that a request's timeout does not end the session."},
{"file":"apps/connectors/tests/adversary_local_mcp_server.rs","line":1161,"category":"judgement","severity":"note","verdict":"CONFIRMED","origin":"introduced","message":"The story's case went red once at load ~100 (server exited mcp_unavailable without a reply), likely because the 2 s budget also bounds the server's startup snapshot."},
{"file":"apps/connectors/src/mcp/stdio.rs","line":164,"category":"judgement","severity":"note","verdict":"INFEASIBLE","origin":"introduced","message":"A late success is replaced by timeout, which would erase a known applied outcome for a write; unreachable today because MCP tools/call admits only reads."}
]
```
