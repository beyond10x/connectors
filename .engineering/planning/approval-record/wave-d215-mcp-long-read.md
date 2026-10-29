---
format: aep.planning-md/3
id: approval-record:wave-d215-mcp-long-read
kind: approval-record
status: approved
title: 'Wave D215: the MCP server exit on a long read under load, accepted by the orchestrator'
relations:
- decides: story:mcp-long-read-exits-under-load
revision: 3
transitions:
- {from: "draft", to: "approved", at: "2026-10-29T17:06:28Z", actor: "human:timo", revision: 3}
---
## Approval

The directing session `b10x-chef-orchestrator` approved item D215 on 2026-10-29, under its grant: "a connectors wave for story:mcp-long-read-exits-under-load and its release", "I accept the story (approval-record citing D215). First decide from evidence whether the server or the test is wrong: reproduce under pinned CPU / load (taskset, nice, a parallel load), capture the server's stderr and exit path. If the server exits on a slow read, that is a product defect: fix the server (failing test first). If only the test's timing is wrong, fix the test without loosening what it asserts. Never add #[ignore] or retries. Pass 50 of 50 pinned runs after the fix. Adversary pass. Check connection_lease the same way if it reproduces."

It also ordered, under the workspace ESS rule (2026-10-29): "this wave changes connectors, so move its ESS crates and pin to the newest released ESS (0.77.0) in the same wave and regenerate with a private copy of that binary."

## Selection

- story:mcp-long-read-exits-under-load: accepted by this record.
- The store close of 0.59.1 (`plan/close-0591`, 1a5155e8f) rides this wave's branch.
- ESS crates and pin move from 0.75.0 to 0.77.0.
