---
format: aep.planning-md/3
id: story:mcp-tool-deadline-keeps-the-server
kind: story
status: implemented
title: A tool request that reaches its own deadline answers timeout and the MCP server keeps running
tags:
- cr-001
- from-review
relations:
- decomposes: epic:zero-open-issues-and-store-defects
- serves: vision:independent-contract-adapters
scope:
- confidence: inferred
  path: apps/connectors/src/mcp/stdio.rs
- confidence: inferred
  path: apps/connectors/tests/adversary_local_mcp_server.rs
revision: 5
transitions:
- {from: "draft", to: "proposed", at: "2026-11-10T09:11:59Z", actor: "human:operator", revision: 2, executor: "agent:connectors-zero-issues", correlation: "connectors-zero-issues"}
- {from: "proposed", to: "active", at: "2026-11-10T09:12:00Z", actor: "human:operator", revision: 3, executor: "agent:connectors-zero-issues", correlation: "connectors-zero-issues"}
- {from: "active", to: "implemented", at: "2026-11-11T06:19:19Z", actor: "human:operator", revision: 5, decided_on: {"recorded":{"test_result":1,"review_outcome":7}}}
---
## Defect

Code review finding CR-001 (S2): A tool request that reaches its own deadline ends the whole MCP server.

- Citations: `apps/connectors/src/mcp/stdio.rs:156-158` (`if Instant::now() >= job.until { return Err(Code::Timeout.into()); }`), `:166-168`, `:329-331` (the worker gets the same `until`); `apps/connectors/src/mcp/projection.rs:49-55`, `:595-607` (only walks reserve reply time).
- What happens: the worker's own `timeout` refusal is produced at or after `until`. The serve loop finds the reply late and returns `Err(Timeout)`, so the process exits and the client loses the session with no reply.
- Trigger: `tools/list` or a plain `tools/call` while the owner does not answer within `request_milliseconds`, for example while the store is held. `docs/local-mcp-stdio.md:133-138` does not say a timeout ends the session.
- Status: code path verified by reading; the timing trigger is inferred. A case in the `adversary_local_mcp_server` test binary that holds the store past the budget would confirm it.
- Fix: give every worker a deadline a reserve earlier than `job.until`, or enqueue the late refusal with a short write grace and keep the session.

- Review's story note: new (`story:mcp-long-read-exits-under-load` covers lease renewal only).

## Source

The operator-ordered code review of the repository at `23b9f1bc4` (origin/main, 0.69.0), 2026-11-10, finding CR-001. No cargo command was run by the review: "verified" means the code path was traced by reading, "inferred" means it was not.

## Acceptance

- A case in the `adversary_local_mcp_server` test binary holds the owner past `request_milliseconds` and shows the server answers the request with `timeout` and then answers a following request; it is red on the base.
- If no test can make it red, that evidence is recorded and nothing is changed.
