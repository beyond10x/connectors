---
format: aep.planning-md/3
id: story:windows-host-job-releases-the-cli-binary
kind: story
status: active
title: The Windows host job's MCP step can rebuild connectors.exe
summary: 'The MCP step failed: failed to remove target\debug\connectors.exe, so Windows serving was skipped'
relations:
- blocks: story:mcp-stdio-on-macos-and-windows
- serves: vision:independent-contract-adapters
scope:
- confidence: cited
  path: .github/workflows/macos-host.yml
- confidence: cited
  path: .github/workflows/windows-host.yml
revision: 6
transitions:
- {from: "draft", to: "proposed", at: "2026-11-23T08:18:23Z", actor: "human:timo", revision: 5}
- {from: "proposed", to: "active", at: "2026-11-23T08:18:24Z", actor: "human:timo", revision: 6}
---
# The Windows host job's MCP step can rebuild connectors.exe

## Need

In `.github/workflows/windows-host.yml`, run 43363104068 (PR 260, head 1ec1fd7fa41), the step
"The MCP stdio server's launch contract and lease timers" failed with
`error: failed to remove file D:\a\connectors\connectors\target\debug\connectors.exe`, after the
step "Test the host and the CLI" passed. The serving step was then skipped, so serving on Windows
is still not shown.

The same step failed the same way on every code change since: 13 of the 25 newest runs of the
workflow on 2026-11-22 and 2026-11-23 ended `failure` (all 8 pushes to `main` and 5 pull requests
with code), last in run 45124020139 (PR 283, connectors 0.75.0), 07:27:32 UTC: `Access is denied.
(os error 5)`. The job is not a required check, so the merges went through.

## Cause

Read from the log of run 45124020139 and from `cargo tree` on 5983a6b9502 (2026-11-23):

- "Test the host and the CLI" runs `cargo test -p connectors-host -p connectors`. The MCP step
  ran `cargo test -p connectors --test mcp_stdio_launch`. Cargo resolves features for the selected
  packages only: with `-p connectors` alone, tokio has no `test-util`, rustix no `net`, and
  `windows-sys` 5 features fewer (`cargo tree --target x86_64-pc-windows-msvc -f '{p} [{f}]'`).
- So the MCP step compiled the dependency chain again (the log lists slab, rustix, tokio, hyper,
  reqwest, connectors-host and connectors, 07:24:20 to 07:27:32) and then replaced
  `target\debug\connectors.exe`.
- Windows refused the replacement. Inferred, not shown by a log: a process started from that
  file by a test of the step before still ran and held it open.

Local proof on Linux, tree of 5983a6b9502, load 80: after `cargo test -p connectors-host -p
connectors --no-run`, each of the five commands with `-p connectors-host -p connectors` compiled
0 crates (2, 4, 16 and 3 tests passed in 1 s, 2 s, 7 s and 1 s; the serving test built with
`--no-run`); the old first command compiled 5 crates, `connectors` among them, in 126 s.

## Acceptance

- The five MCP commands of `.github/workflows/windows-host.yml` and
  `.github/workflows/macos-host.yml` select `-p connectors-host -p connectors`, as the test step
  before them does, so they build without replacing `connectors.exe`.
- A step after "Test the host and the CLI" prints every process that still runs from
  `target\` (id, name, start, parent, command line) and never fails the job.
- The Windows host run of the pull request ends `success`, with the steps "The MCP stdio
  server's launch contract and lease timers" and "The MCP stdio server serves a session on the
  platform's own custody" both run and passed; the macOS host run stays `success`.
- The count that step printed is in the pull request. If it is above 0, the tests that leave a
  process running get their own story, named here.

## Out of scope

Stopping the processes a test leaves running: not known to exist until the new step printed
them.
