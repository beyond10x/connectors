---
format: aep.planning-md/3
id: review-result:adversary-d240-s2-pass-1
kind: review-result
status: active
title: Adversary pass 1 on story:mcp-tool-call-shapes-its-result (wave D240)
tags:
- d240
relations:
- reviews: story:mcp-tool-call-shapes-its-result
revision: 1
---
## Adversary pass 1, unit S2 (wave D240), with the read-only security invariants, as returned

unit: story:mcp-tool-call-shapes-its-result, tree conn-d240-s2 at f266905a1 (findings cover fbbeebc1c plus the test file)
verdict: CONFIRMED blocker (a walk that runs past the request budget kills the server instead of replying)
cases: executed 7→12, red 4
origin: introduced 2 / pre-existing 1 / undecided 0
wrote-outside-worktree: 13 paths under the assigned scratch s2/adv/
needs-coordinator: S4 is rewriting walk::run internals on the integration branch, so the two walk::run cases need a rerun there.

Cases in `apps/connectors/tests/mcp_tool_call_shapes_adversary.rs` (f266905a1), red:
- `a_walk_that_outlasts_the_request_budget_answers_the_records_taken`: "no reply to request 2
  (Disconnected after 20.213091945s); the server's exit: ... unix_wait_status(256)".
- `max_pages_written_as_an_integral_number_is_the_integer_the_schema_admits`: `max_pages: 2.0`
  refused as `Invalid arguments.max_pages`.
- `a_page_the_bound_refused_does_not_decide_missing`: `missing: []`, expected `["x"]`.
- `a_first_page_the_bound_refused_reports_nothing_missing`: `missing: ["y"]`, expected `[]`.

Suite: mcp_tool_call_shapes ok 7 passed (EXIT=0); mcp_tool_call_shapes_adversary 1 passed 4 failed (EXIT=101).

Not broken: validation order and codes, stdin close mid-walk with 300 ms pages, text copy equal to
structuredContent, error Responses and non-2xx unshaped, response bound, read-only (only
arguments.input reaches the owner; every page through Reader::read admission and audit).

```findings
- file: apps/connectors/src/mcp/stdio.rs
  line: 156
  category: acceptance
  severity: blocker
  verdict: CONFIRMED
  origin: introduced
  message: a walk that outlasts the request budget replies after its deadline, so the server exits with status 1 and no reply instead of the promised success stopped refused/timeout with its records (a_walk_that_outlasts_the_request_budget_answers_the_records_taken)
- file: apps/connectors/src/mcp/projection.rs
  line: 513
  category: contract-drift
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: max_pages 2.0 passes the advertised JSON Schema integer but is refused as Invalid arguments.max_pages (max_pages_written_as_an_integral_number_is_the_integer_the_schema_admits)
- file: apps/connectors/src/walk.rs
  line: 201
  category: boundary
  severity: note
  verdict: CONFIRMED
  origin: pre-existing
  message: a page the sink refuses still decides missing, so a walk stopped by the bound reports missing paths of records it never returned (a_page_the_bound_refused_does_not_decide_missing, a_first_page_the_bound_refused_reports_nothing_missing)
```
