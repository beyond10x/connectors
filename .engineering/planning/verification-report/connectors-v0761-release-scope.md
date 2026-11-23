---
format: aep.planning-md/3
id: verification-report:connectors-v0761-release-scope
kind: verification-report
status: accepted
title: connectors 0.76.1 release scope
summary: Scope, exclusions and evidence of the 0.76.1 cut
revision: 4
transitions:
- {from: "draft", to: "accepted", at: "2026-11-23T19:56:33Z", actor: "human:timo", revision: 4}
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

- A store written by an earlier release keeps the records it has and does not shrink. D18,
  host-operator under Timo's delegation, 2026-11-23 about 21:00 CET: "A": leave old stores as
  they are; a new state directory plus reconnecting is the way to a small store.
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

## Verified after the release

- Pull request 287 merged 2026-11-23 20:37 CET as c97cb5660d9; tag `v0.76.1` is on that commit.
- Release page published 20:39 CET; the release-binaries run 45251192789 ended `success`; the
  four archives match `SHA256SUMS`; the image `ghcr.io/beyond10x/connectors:0.76.1` carries
  revision c97cb5660d9 for linux/amd64 and linux/arm64.
- Checks of pull request 287: all passed except "host and CLI on windows-latest", red at its
  step 10 as at 0.76.0. Three macOS jobs were first cancelled by the forge for lack of a
  runner and passed on one rerun.
- A workstation install swapped from 0.76.0 to 0.76.1 (binary only, same store): one read each
  through three instances answered status 200; three more read invokes left
  `metadata.sqlite3` at 4,882,432 bytes.
- Issue #267 holds the measurement report (comment 7112288332); the issue stays open for its
  other items.
