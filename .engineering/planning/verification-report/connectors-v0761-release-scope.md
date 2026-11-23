---
format: aep.planning-md/3
id: verification-report:connectors-v0761-release-scope
kind: verification-report
status: draft
title: connectors 0.76.1 release scope
summary: Scope, exclusions and evidence of the 0.76.1 cut
revision: 2
---
## Scope of connectors 0.76.1

Released 2026-11-23 on Timo's pick for question D5, relayed by host-operator: "A: stop
recording reads", "connectors only", under his release order of 03:59 CET.

- story:settled-read-uses-leave-the-store (issue #267, item 1): a read use is held in the
  memory of the owner process and is not recorded in the local metadata store. A read invoke
  appends no record. Measured, release profile, 50 recorded events, 100 read invokes: 3 records
  and 24,500 blob bytes per read invoke before (3d38f77b99f), 0 and 0 after (cfd7e016c, on a
  copy of the store the first run saved).
- Patch version: the CLI surface and the store's form do not change. A store that 0.76.0 wrote
  opens as before.
- The six differences in behaviour are in `CHANGELOG.md` under 0.76.1.

## Known at the release

- A store written by an earlier release keeps the records it has and does not shrink. Whether
  such stores get a way to shrink is an open question at host-operator; the default is to
  leave them, a new state directory being the way to a small store.
- Same-load comparison, one run each, host load 97 to 100: the 0.76.0 code (e535fc5f924)
  `median_ms=1039 median_cpu_ms=197 peak_rss_mb=87`; this release `median_ms=780
  median_cpu_ms=153 peak_rss_mb=70`. Not repeated.
- On the workstation at host load 74 two tests of the branch's set of 21 were killed at the
  60 s bound: `due_expiries_past_the_former_32_member_bound_join_the_callers_batch` (76 s
  without this change, 80 s with it) and
  `a_reopen_after_its_own_write_refuses_every_blob_altered_since` (58 s alone on main's code).
  story:store-tests-end-within-60s-under-load carries them. The forge's repository gate
  decides the merge.
- On `windows-latest` a tool call through the MCP stdio server answers `timeout`, unchanged
  since 0.76.0 (story:windows-mcp-tool-call-answers-timeout).

## Not in this release

- The Entity Runtime 0.33.1 pin (story:entity-runtime-0331-pin, branch `story/er-0331-pin`):
  0.76.2.
- Standing grants (0.77.0) and the pattern grant (0.78.0).
- The working-directory fix of the owner and its providers, and the Windows MCP fix.
