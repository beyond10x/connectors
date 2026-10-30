---
format: aep.planning-md/3
id: approval-record:d248-owner-start-release-0593
kind: approval-record
status: draft
title: Release 0.59.3 alone from main with the owner start fix (D248)
relations:
- decides: story:owner-start-outlasts-a-fixed-bound
revision: 2
---
## Decision

D248 of the directing session `b10x-chef-orchestrator`, 2026-10-30, under its grant (verbatim): "yes, cut
0.59.3 from main. It ships alone because a consumer is blocked on it today: the triage session's Zendesk
reads time out at "admission" on 0.59.1."

The fix for `story:owner-start-outlasts-a-fixed-bound` was first written on the 0.60.0 wave's base. 0.60.0
removes `operations invoke`, which the blocked local consumer still calls, so the fix also ships alone as
0.59.3 from `main`. The 0.60.0 wave takes it by merging `main` after this release merges.

## Authorises

One branch from `origin/main` with the fix and the 0.59.3 release metadata, pushed through `b10x-gates bot`;
one pull request through `b10x-gates api`, auto-merged; the annotated tag `v0.59.3` on the merge commit and
its release page through the bot, per `AGENTS.md` § Cutting a release.
