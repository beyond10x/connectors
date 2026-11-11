---
format: aep.planning-md/3
id: story:mcp-startup-has-its-own-bound
kind: story
status: draft
title: The local MCP server's startup is not bounded by the request budget
tags:
- from-finding
relations:
- decomposes: epic:zero-open-issues-and-store-defects
- informed_by: review-result:wave-b-mcp-deadline-adversary-1
revision: 1
---
## Defect

Found by the adversary of `story:mcp-tool-deadline-keeps-the-server`
(`review-result:wave-b-mcp-deadline-adversary-1`), pre-existing: the local MCP server bounds its own
startup snapshot by the request budget (`apps/connectors/src/mcp/stdio.rs:114-120`,
`start + request_time.min(10 s)`) and maps a failure to `unavailable`. With a short
`request_milliseconds` (2 s in the test), a cold owner start under load takes longer, and the server exits
`mcp_unavailable` before it answers its first ping: 2 of 6 concurrent sessions in
`adversary_the_units_deadline_sequence_under_six_sibling_sessions`.

## Acceptance

- Server startup has its own bound, documented in `docs/local-mcp-stdio.md`, that does not shrink with
  `request_milliseconds`; a server started with a 2 s request budget beside a cold owner under load
  answers its first ping, and a startup that exceeds its own bound still ends with `unavailable`.
