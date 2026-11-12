---
format: aep.planning-md/3
id: story:mcp-stdio-on-macos-and-windows
kind: story
status: active
title: The MCP stdio server serves on macOS and Windows
relations:
- decomposes: epic:business-teams-google-and-hubspot
- serves: vision:independent-contract-adapters
revision: 3
transitions:
- {from: "draft", to: "proposed", at: "2026-11-12T16:07:33Z", actor: "human:operator", revision: 2}
- {from: "proposed", to: "active", at: "2026-11-12T16:07:34Z", actor: "human:operator", revision: 3}
---
## Need

Business teams use Claude on macOS and Windows. `connectors server --transport stdio` serves on
Linux only: its data lease runs on `CLOCK_BOOTTIME`, and elsewhere a valid launch exits 1 with
`mcp_unavailable` (`apps/connectors/src/mcp/entry.rs:66-77`, `docs/local-mcp-stdio.md:10-13`).

## Acceptance

- The stdio server serves on macOS and Windows with a lease clock that counts across sleep, as
  `CLOCK_BOOTTIME` does (macOS `mach_continuous_time` or `CLOCK_MONOTONIC`; Windows
  `QueryInterruptTime`), each with a test.
- The macOS and Windows CI jobs run the MCP stdio tests that run on Linux today.
- `docs/local-mcp-stdio.md` and `website/docs/connectors/mcp.md` drop the Linux-only statement.
