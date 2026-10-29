---
format: aep.planning-md/3
id: release-plan:connectors-v0592-mcp-long-read
kind: release-plan
status: active
title: 'Release 0.59.2: the stdio MCP lease renewal and stdin-close admission fixes, ESS 0.77.0'
relations:
- serves: vision:independent-contract-adapters
revision: 3
transitions:
- {from: "draft", to: "active", at: "2026-10-29T19:24:59Z", actor: "human:timo", revision: 3, decided_on: {"recorded":{"test_result":1}}}
---
## Outcome

Patch release 0.59.2 of wave D215 (`approval-record:wave-d215-mcp-long-read`):

- The local MCP stdio server no longer exits with status 1 while it answers a read longer than
  one lease on a loaded host. Renewing the lease no longer waits on the store lock. The renewal
  still compares the store's authority whenever the lock is free
  (`story:mcp-long-read-exits-under-load`).
- Admission refuses a call once the client has closed standard input, even before the serve loop
  reads the end of file (same story).
- ESS 0.77.0 (from 0.75.0). The regenerated types differ only in their version stamps.

This is a patch rather than a minor release: the CLI, the owner, the contracts and the models
are unchanged apart from the version.

Approved by the directing session `b10x-chef-orchestrator` under its grant, item D215
(2026-10-29): "a connectors wave for story:mcp-long-read-exits-under-load and its release".

## Scope

| story | status |
|---|---|
| story:mcp-long-read-exits-under-load | implemented |

## Limitations

- `story:cli-connect-admission-timeout-under-load` (draft) stays open. `connections connect` can
  time out at admission on a host pinned to 2 CPUs beside 4 busy workers.
- `story:mcp-reply-after-hangup-owner-latency-under-load` (draft) stays open. A call already at
  the owner when stdin closes is discarded, with exit 1 as documented, when the owner takes
  longer than the lease that was live at the close. That happened 1 time in 60 pinned runs.
- One run of the long-read test in 150 pinned runs at 1d2e8f468 ended with the server closing
  stdout, cause not captured. The test helpers now print the server's stderr when a test fails.
