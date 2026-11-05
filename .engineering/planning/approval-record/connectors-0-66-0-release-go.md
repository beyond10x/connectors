---
format: aep.planning-md/3
id: approval-record:connectors-0-66-0-release-go
kind: approval-record
status: approved
title: Release go for connectors 0.66.0 on green CI of pull request 239
relations:
- decides: approval-record:connectors-wave-2026-11-04
revision: 2
transitions:
- {from: "draft", to: "approved", at: "2026-11-05T01:37:04Z", actor: "human:timo", revision: 2}
---
## Release go for 0.66.0 (2026-11-05)

The directing session `b10x-chef-orchestrator`, under the operator's grant for class-C items, gave the go
for pull request 239 (`wave/2026-11-04-connectors`, head d3a8d6d08):

> when CI-DONE shows every required check on PR 239 green, merge through PUT /pulls/239/merge, tag
> v0.66.0 on the merge commit, create the release page, run verify-release.sh into
> ~/.cache/b10x-layers/verify-connectors-v0.66.0.txt with an EXIT= line, write the announcement file,
> send me both paths, then finish the wave trees with --discard-cache --archive. A red check, including
> the new provider-only ignored-tests step: stop and send me the failing job and its log lines, with no
> rerun of your own.

Conditions: every required check green, including the first full run of
`connectors-build ignored --required --only-prerequisite provider`; a red check stops the release with
no rerun.
