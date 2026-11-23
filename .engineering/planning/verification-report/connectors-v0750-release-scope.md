---
format: aep.planning-md/3
id: verification-report:connectors-v0750-release-scope
kind: verification-report
status: draft
title: connectors 0.75.0 release scope
summary: Scope, exclusions and evidence of the 0.75.0 cut
revision: 1
---
## Scope of connectors 0.75.0

Released 2026-11-23 under Timo's instruction of that day: "We always merge what we can merge, and
if you're done, cut a new release" (`AGENTS.md`, Cutting a release), and host-operator's order of
03:59 CET that the gaps of Runbook and dev-operator go first.

- story:invoke-admission-queues-under-load: `connectors invoke --deadline-seconds <n>` names a
  read's deadline, 1 to 120 seconds, 30 when left out. Consumer gap: dev-operator's parallel
  reads were refused `timeout` at `admission` after the fixed 30 seconds.
- A read on `invoke` has one cutoff, taken before its owner is started. Before, a read that
  started its owner had up to 30 seconds for the start and 30 more for the read.
- A read that validates its stored credential again after an executable change ends that
  validation at the earlier of its deadline and the 30-second capture lease.
- Minor release: one added flag. Compatibility: a cold read that needed between 30 and 60
  seconds in total answered before and is now refused `timeout` unless it names a longer
  deadline.

## Not in this release

- Issue #267 (store growth per read): the Entity Runtime 0.33.0 pin
  (story:entity-runtime-0330-records-name-definition-by-digest) waits for that release, and
  story:settled-read-uses-leave-the-store follows it.
- story:timeout-refusal-says-how-long-it-waited (`waited_ms`), draft.
- Acceptance line 11 of story:invoke-admission-queues-under-load, the measurement of 16 parallel
  reads with `--deadline-seconds 120` on a real instance: it runs with the released binary and is
  recorded on the story and on issue #267. The story stays active until then.
- The other draft stories under epic:review-20261123-findings and epic:slack-complete-web-api.

## Evidence

The release pull request's required checks on the release commit, then the tag, release page,
archives, `SHA256SUMS` and image, verified after publication.
