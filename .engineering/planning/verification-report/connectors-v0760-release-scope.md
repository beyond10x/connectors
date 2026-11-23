---
format: aep.planning-md/3
id: verification-report:connectors-v0760-release-scope
kind: verification-report
status: draft
title: connectors 0.76.0 release scope
summary: Scope, exclusions and evidence of the 0.76.0 cut
revision: 3
---
## Scope of connectors 0.76.0

Released 2026-11-23 on Timo's pick for question D4, relayed by host-operator at about 10:55 CET:
"A: ship now", with the CHANGELOG Breaking entry and the way back, under his release order of
03:59 CET, once the Windows host fix (pull request 284) is merged.

- story:entity-runtime-0330-records-name-definition-by-digest (issue #267, item 2): Entity
  Runtime 0.33.0 on Eventlog 0.9.4. Every record written from this release on is `er.record/5`
  and names its entity definition by digest. Measured: 78,488 bytes per read invoke before,
  24,500 after (0.312).
- Breaking: connectors 0.75.0 and earlier do not open a metadata store once this release wrote
  to it (`metadata_unavailable`). The way back is in `CHANGELOG.md`: move the state directory
  aside and connect again.
- Minor version: the CLI surface does not change.

## Known at the release

- Time per read invoke rose in the test profile (`er.execute_batch` 314.5 and 350.5 ms before,
  420.2 and 419.6 ms after; a release build was not measured). The entity-runtime session is
  bisecting it; a patch version of Entity Runtime would be a later pin.
- Acceptance line 8 of the pin story is not met on the workstation at host load 68 to 85: five
  store tests run 57 to 130 s and are killed at 60 s; four of them are that slow on Entity
  Runtime 0.32.0 too. story:store-tests-end-within-60s-under-load carries them. The forge's
  repository gate decides the merge.
- Records already in a store keep their size.
- On `windows-latest` a tool call through the MCP stdio server answers `timeout` (step 10 of
  the job "host and CLI on windows-latest", run 45148199646). The step never ran on Windows
  before pull request 284. Read from code, not yet shown by a run: the bounded read behind a
  tool call is built for Unix only (`runtime/bounded.rs:97-102`), so earlier releases have the
  defect too. story:windows-mcp-tool-call-answers-timeout carries it and comes next.
  host-operator, D6, 2026-11-23 about 11:40 CET: "merge #284 as it is, ship 0.76.0 right
  after, fix Windows step 10 as its own story, named in the release's 'known' list".

## Not in this release

- story:settled-read-uses-leave-the-store (issue #267, item 1): Timo's pick for D5 is "A: stop
  recording reads"; the story is rewritten to it and built next.
- story:connections-list-answers-through-the-owner (issue #267, item 3): in implementation.
- story:timeout-refusal-says-how-long-it-waited, draft.

## Evidence

The release pull request's checks on the release commit, then the tag, release page, archives,
`SHA256SUMS` and image, verified after publication.
