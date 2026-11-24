---
format: aep.planning-md/3
id: verification-report:connectors-v0762-release-scope
kind: verification-report
status: draft
title: Scope of connectors 0.76.2
summary: Scope, exclusions and evidence of the 0.76.2 cut
revision: 2
---
## Scope of connectors 0.76.2

Released 2026-11-23 under Timo's release order of 03:59 CET (relayed by host-operator): finished
green work is integrated and released when a user sees something.

- story:owner-and-providers-run-in-root-directory: on Linux and macOS the owner and every
  provider process it starts run in `/`. Before, they kept the working directory of the CLI
  call that started the owner and outlived that call. Reported by a consumer on 0.65.2: a
  checkout whose run had ended could not be removed while an owner and two providers ran in it.
  Test `the_owner_and_its_provider_run_in_the_root_directory_not_in_the_caller_s`
  (`apps/connectors/tests/cold_owner_start.rs`): red without the two lines, green in 6 of 6
  runs with them, 20 to 38 s at host load 55 to 59.
- story:entity-runtime-0331-pin: Entity Runtime 0.33.1 (from 0.33.0). No change of the store's
  form. Measured on the code of 0.76.0, three runs per pin at host load 52 to 67:
  `er.execute_batch` per read invoke 110.3 ms with 0.33.1, 129.5 ms with 0.33.0, 122.9 ms with
  0.32.0 (medians).
- story:windows-mcp-tool-call-answers-timeout (pull request 288, merged 2026-11-24 00:27 CET as
  fa36ffbbd49): on Windows a tool call through the MCP stdio server answers. Before, every
  `tools/call` answered `timeout`: the bounded read was built for Unix only. Proof: the check
  "host and CLI on windows-latest" of pull request 288 passed all steps; it was red at its
  step 10 at 0.76.0 and 0.76.1.
- Patch version: the CLI surface and the store's form do not change.

## Known at the release

- A relative path in a provider's `executable.args` resolves against `/` in the provider the
  owner starts; in the CLI's own `--print-*` and `--version` calls of that provider it resolves
  against the directory the CLI was called from.
- No test holds the provider's `current_dir("/")` on its own: once the owner runs in `/`, the
  provider inherits it. One measurement with the owner's line removed showed the provider in
  the root and the owner in the caller's directory.
- The pin was not measured again on the code of 0.76.1, where a read invoke appends nothing.
  CPU time of one read invoke on the code of 0.76.0: 175 ms with 0.33.1, 140 ms with 0.32.0.

## Not in this release

- `connections list` and `describe` through a running owner
  (story:connections-list-answers-through-the-owner): three of its tests failed in 4 of 9 runs
  that took over 50 s on the loaded workstation. It stays on branch
  `story/connections-through-owner` until they hold.
- Standing grants (0.77.0) and the pattern grant (0.78.0).
