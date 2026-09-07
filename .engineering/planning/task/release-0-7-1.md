---
format: aep.planning-md/1
id: task:release-0-7-1
kind: task
status: active
title: Release Connectors 0.7.2
summary: Retargeted after concurrent v0.7.1 tag; immutable ID retains original planning identity
relations:
- derived_from: runbook:small-wave-after-0-7-0
- decomposes: story:release-0-7-1
revision: 8
---
## Acceptance

## Acceptance

The approved Connectors v0.7.2 release contains the reviewed `inspect upgrade` command, preserves published main, passes the authoritative release checks, publishes four Unix archives with matching checksums and consumer notes, delivers updated public documentation, and retires only safely published wave-owned worktrees.

## Scope

## Scope

Release identity across internal Cargo manifests/locks and generated catalog artifacts; CHANGELOG.md and WHATS-NEW.md; command documentation; source and tag delivery through Atlas; deterministic Website source lock and Atlas snapshot publication. Keep external dependency versions, frozen contracts and release workflow semantics unchanged relative to the integrated published-main baseline. The feature remains `story:the-binary-says-what-it-carries`; v0.7.1 source-read changes and earlier OAuth/remediation fixes are preserved as prior release history. Only the inspection feature is new in this release's top-level notes.

## Concurrent release preparation

## Concurrent release preparation

The concurrent source-read work reached remote main `5c93cc661e6dc634ebdd900981258c7bfab1b983` and created the v0.7.1 tag. This run therefore selects **v0.7.2** for the approved inspection feature, preserving the published source-read changes and earlier fixes. The user was informed; their release authorization names the feature, not a mandatory version number. The v0.7.1 tag is not replaced.

The release story/task retain their original immutable 0.7.1 IDs as planning history; native `artifact set` updates their visible titles and summaries to 0.7.2. This avoids inventing a second release task for the same approved work. Acceptance and consumer notes now name 0.7.2. Before the next full gate, the integration branch incorporates the exact published main source above, including its unchanged release workflow and new source-installation contract guidance in AGENTS.md. No installed CLI is replaced by this release task.
