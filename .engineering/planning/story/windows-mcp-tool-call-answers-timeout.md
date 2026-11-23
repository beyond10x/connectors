---
format: aep.planning-md/3
id: story:windows-mcp-tool-call-answers-timeout
kind: story
status: draft
title: A tool call through the MCP stdio server answers on Windows
summary: On windows-latest tools/call answers timeout after 2 s; step 10 of the Windows host job is red
relations:
- blocks: story:mcp-stdio-on-macos-and-windows
- serves: vision:independent-contract-adapters
scope:
- confidence: inferred
  path: apps/connectors/src/mcp/projection.rs
- confidence: inferred
  path: apps/connectors/src/mcp/stdio.rs
- confidence: inferred
  path: apps/connectors/tests/mcp_stdio_serves.rs
- confidence: inferred
  path: crates/connectors-host/src/local/owner/transport.rs
- confidence: inferred
  path: crates/connectors-host/src/local/runtime/bounded.rs
- confidence: inferred
  path: crates/connectors-host/src/local/runtime/channel/windows.rs
revision: 8
---
## Defect

On `windows-latest` a tool call through the MCP stdio server answers `timeout` about 2 s after
the test binary starts. `initialize` and `tools/list` of the same session answer.

Run 45148199646 (pull request 284, head 0fadfb1878f, 2026-11-23), job "host and CLI on
windows-latest", step 10 "The MCP stdio server serves a session on the platform's own
custody": `the_stdio_server_serves_a_session_and_answers_a_call` panics at
`apps/connectors/tests/mcp_stdio_serves.rs:512` with

```
{"error":{"code":-32000,"data":{"audit_ref":null,"audit_status":"unavailable","error":{"code":"timeout","message":"Connectors request refused"},"request_id":null,"status":"error","version":"v1alpha2"},"message":"Connectors request refused"},"id":3,"jsonrpc":"2.0"}
```

`test result: FAILED. 0 passed; 1 failed; ... finished in 2.16s`. The server's stderr is empty.
`connections connect` of the test's adapter passes in the same run. macOS and Linux pass the
step.

The step was skipped in every Windows run on `main` (an earlier step failed since run
43370169181 of 2026-11-13, and the last green run, 43357584659, has no MCP step), so a tool call
through the MCP stdio server has not been shown to work on Windows. Whether a released
`connectors.exe` has the defect is not known; the code path is the same.

## Cause, read from code on 2026-11-23

The bounded read is compiled out off Unix. `ticks()` under `cfg(not(unix))` returns
`Failure::Unsupported` (`crates/connectors-host/src/local/runtime/bounded.rs:97-102`), so
`ReadBudget::start` cannot succeed on Windows, and `apps/connectors/src/mcp/projection.rs:743-745`
maps every failure of it to `refusal("timeout")`, before any owner call. `tools/list` builds
no budget. The owner refuses the same read too: `watch_cancel` under `cfg(not(unix))` answers
`Code::Unsupported` (`crates/connectors-host/src/local/owner/transport.rs:2527-2536`). The
test sets `request_milliseconds` to 120000, so no deadline ends at 2 s. Not yet shown by a
run on Windows.

Recorded before: `approval-record:wave-20261022`, orchestrator decision D1 of 2026-10-23,
"the bounded read is unix-only by design", with the test
`bounded::unsupported_tests::a_budget_cannot_start_off_unix`.
story:mcp-stdio-on-macos-and-windows (active, later) requires the opposite: "The macOS and
Windows CI jobs run the MCP stdio tests that run on Linux today". This story follows the
later one: the bounded read works on Windows.

## Design

- `ticks()` gets a `cfg(windows)` form on the unbiased interrupt time, as
  `crates/connectors-host/src/local/owner/mutation.rs:259-271` reads it.
- `watch_cancel` gets a Windows form; the owner pipe is overlapped and has no `try_clone`, so
  it needs a duplicated handle in `crates/connectors-host/src/local/runtime/channel/windows.rs`.
- `unsupported_tests` goes, and the `runtime::bounded::tests` run on every platform.
- `projection.rs:745` maps `Failure::Unsupported` to `unsupported`, so a platform without the
  read does not answer a false `timeout`.

## Decision D6, 2026-11-23

host-operator, about 11:40 CET: "D6 = A: merge #284 as it is, ship 0.76.0 right after, fix
Windows step 10 as its own story, named in the release's 'known' list." No
`continue-on-error` on the step: Timo, about 11:35 CET, "we dont build workarounds here". This
story comes next after 0.76.0.

## Acceptance

1. The cause above is shown by one run on
   `windows-latest` (a diagnostic the run prints, or the fix making the step pass), or the report names the other cause.
2. On a pull request's head, the job "host and CLI on windows-latest" ends `success` with
   step 10 `success`, and the same step stays `success` on `macos-latest`.
3. A test that runs on every platform in the repository gate covers the fixed line, or the
   report says why only the Windows job can.
4. `CHANGELOG.md` has a Fixed entry that says since which release the defect is, or that this
   is not known.
