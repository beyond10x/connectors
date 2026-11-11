---
format: aep.planning-md/3
id: review-result:wave-b-mcp-deadline-adversary-1
kind: review-result
status: active
title: Adversary pass 1 on mcp-tool-deadline-keeps-the-server (single defect wave)
relations:
- reviews: story:mcp-tool-deadline-keeps-the-server
revision: 1
---
```
unit: story:mcp-tool-deadline-keeps-the-server (commit 8972d6166 plus an uncommitted test file)
verdict: red
cases: executed 17→21, red 2
origin: introduced 4, pre-existing 0, undecided 0
wrote-outside-worktree: <scratch>/units/mcp-tool-deadline-keeps-the-server/scratch/adv/ (red1.log, probe1.log, red1b.log, case2.log, case3.log, suite.log, unit-suite-2.log, unit-suite-3.log, suite2.log)
needs-coordinator: yes
```

1. No source or existing test touched; new untracked `apps/connectors/tests/adversary_mcp_deadline.rs`.
2. Cases, each run alone first:
   - `adversary_a_reply_undrained_past_its_deadline_does_not_end_the_session` (:505), red: 200 pings, client reads 600 ms after their 2 s budget: `the session ended: exit status: 1, stderr "mcp_unavailable\n", 66 of 200 pings answered` (red after a ping handshake made it deterministic).
   - `adversary_a_late_timeout_queued_behind_a_full_pipe_is_still_written` (:530), green: guards `LATE_WRITE_GRACE` (a grace under about 0.6 s would turn it red; reasoned, not run against a mutant).
   - `adversary_the_units_deadline_sequence_under_six_sibling_sessions` (:613), red: `2 of 6 sessions failed: ["start (ping 1): no reply, server gone at 2.010550305s", ...]`.
3. Suite `--no-fail-fast -p connectors --test adversary_local_mcp_server --test adversary_mcp_deadline`: unit's binary `17 passed; 1 ignored`; adversary binary `2 passed; 2 failed`, exit 101 (probe `6 of 6 sessions failed`: 5 at `list 3 ... "code":"timeout"`, 1 at `start (ping 1) ... 2.015634642s`). The unit's binary alone, three runs: 2 of 3 failed `adversary_a_request_past_its_deadline_answers_timeout_and_the_session_continues` (`the server did not reply`, `mcp_unavailable`).
4. Findings:
   - Blocker, introduced (`adversary_local_mcp_server.rs:1161`): the acceptance case fails intermittently; server startup is bounded by `request_milliseconds` (`stdio.rs:114-120`, `start + request_time.min(10s)`), so with a 2 s budget a cold owner start under load exits the server before its first ping; the follow-up list can also miss 2 s. Fix, test only: warm the owner first with a default-budget server.
   - Warning, introduced (`docs/local-mcp-stdio.md:143`): "A request's timeout does not end the session" is false; the write deadline at `stdio.rs:179-181` (unchanged from base) still ends the session when a reply is not drained before its request's deadline.
   - Note, INFEASIBLE (`stdio.rs:164`): no case catches passing a late non-timeout reply through instead of replacing it; the replacement drops a late success's admitted envelope and `audit_ref`, which `projection.md:333` says to keep; a late success could not be constructed.
   - Note, CONFIRMED (`stdio.rs:63`): the unit's case cannot catch a `LATE_WRITE_GRACE` shorter than one loop turn; `adversary_mcp_deadline.rs:530` guards it.
5. Attacked, not broken: mutating calls (only read tools are listed; a write cannot be reported `timeout`); cancellation (a late unwritten reply is withdrawn; an early cancel moves work to `cancelled_work`); walks (pages stop `WALK_REPLY_RESERVE` before the deadline); ordering (FIFO with ids); worker overrun (`connect_until`, `governed_reply` bounded by `until`).

```findings
[
{"file":"apps/connectors/tests/adversary_local_mcp_server.rs","line":1161,"category":"acceptance","severity":"blocker","verdict":"NEEDS-CHANGE","origin":"introduced","message":"The unit's acceptance case failed 2 of 3 runs of its own binary because server startup is bounded by the 2 s request_milliseconds (stdio.rs:114-120), so a cold owner start under load exits the server before its first ping, and the follow-up list can also miss 2 s."},
{"file":"docs/local-mcp-stdio.md","line":143,"category":"contract-drift","severity":"warning","verdict":"NEEDS-CHANGE","origin":"introduced","message":"The new sentence 'A request's timeout does not end the session' is false: a reply left unread past its request's deadline still ends the session (adversary_mcp_deadline.rs:505 red, exit 1, 66 of 200 replies)."},
{"file":"apps/connectors/src/mcp/stdio.rs","line":164,"category":"mutant","severity":"note","verdict":"INFEASIBLE","origin":"introduced","message":"No case catches passing a late non-timeout reply through instead of replacing it, and the replacement drops a late success's admitted envelope and audit_ref, which projection.md:333 says to preserve; I could not construct a late success."},
{"file":"apps/connectors/src/mcp/stdio.rs","line":63,"category":"mutant","severity":"note","verdict":"CONFIRMED","origin":"introduced","message":"The unit's case cannot catch a LATE_WRITE_GRACE shorter than one loop turn; adversary_mcp_deadline.rs:530 (green) now guards it, though it was not run against a mutated build."}
]
```
