---
format: aep.planning-md/1
id: approval-record:inspect-upgrade-and-release-20260907
kind: approval-record
status: draft
title: Operator approves binary inspection wave and release
relations:
- decides: runbook:small-wave-after-0-7-0
- decides: task:release-0-7-1
- decides: story:release-0-7-1
revision: 1
---
On 2026-09-07 the operator replied: "approved, do it, then cut release with that" to runbook:small-wave-after-0-7-0. This approves the one-feature wave, its planned commits and integration, then the release cut and necessary bot-authenticated source/tag publication, documentation delivery, archive verification and managed cleanup. Retain the two recovery trees and other sessions' trees named in the proposal; approval permits the new isolated wave alongside them and does not authorize deleting their work. The intended additive/fix release is v0.7.1, subject to verifying the version remains available at cut time.
