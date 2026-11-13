---
format: aep.planning-md/3
id: story:windows-host-job-releases-the-cli-binary
kind: story
status: draft
title: The Windows host job's MCP step can rebuild connectors.exe
summary: 'The MCP step failed: failed to remove target\debug\connectors.exe, so Windows serving was skipped'
relations:
- blocks: story:mcp-stdio-on-macos-and-windows
revision: 1
---
# The Windows host job's MCP step can rebuild connectors.exe

## Need

In `.github/workflows/windows-host.yml`, run 43363104068 (PR 260, head 1ec1fd7fa41), the step
"The MCP stdio server's launch contract and lease timers" failed with
`error: failed to remove file D:\a\connectors\connectors\target\debug\connectors.exe`, after the
step "Test the host and the CLI" passed. The serving step was then skipped, so serving on Windows
is still not shown. Inferred cause: a process started from `connectors.exe` by an earlier test (an
owner) still runs and holds the file open.

## Acceptance

- After "Test the host and the CLI", no process started from `target\debug\connectors.exe` is
  left running, or the MCP step builds without replacing that file.
- The Windows serving scenario step runs and passes, so
  story:mcp-stdio-on-macos-and-windows can be closed.
