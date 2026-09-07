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
revision: 11
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

## Version preparation after the wave

PR #23 merged the green inspection wave at `af55c7c5a2c255f907caa1b0493e86c7662629ed`. Native `cargo set-version 0.7.2 --workspace --offline` ran for every workspace returned by `scripts/gate.sh --list-workspaces`; native `cargo upgrade` selected only explicit local dependency names at 0.7.2 with recursive upgrades disabled. Offline Cargo metadata refreshed all twelve lockfiles. Sorted external name/version/source graphs match their pre-bump 0.7.1 baselines byte for byte, and no 0.7.1 manifest pin remains. Native Cargo commands performed the manifest and lock mutations; no external package was upgraded.

The 0.7.2 CHANGELOG and WHATS-NEW sections describe the inspection command; 0.7.1 history remains intact. Catalog generation and its own deterministic diff/check are the remaining local release checks. The tag workflow must validate the final release identity and produce all four shipping archives before publication is complete.

The finished implementation worktree held only three ignored Cargo targets; reports and raw probes were retained in assigned scratch. Native cargo clean removed those reproducible targets. Managed finish succeeded; reviewed exact-id GC is responsible for its removal. Other profile records, including eligible work belonging to another session, remain untouched.

## Local release verification

The native 0.7.2 catalog generator completed build → diff → check, then the required second build → diff. Final outputs were:

```text
65 providers, 70 artifacts up to date; nothing written
70 artifacts up to date (65 providers checked)
65 providers, 70 artifacts verified
```

Portable link and story-index checks passed. The complete pre-version wave gate is recorded at `e13630fef05210c61772d29ca5c02ba297b7fbf8`; actual tag validation and shipping builds remain pending. Native command logs are retained in `~/.cache/cw7/upgrade/release-072-catalog-*.log`. Existing vendor-source diagnostics were retained in those logs; this release changes no provider contract.

The implementation tree `wt-inspect-upgrade-20260907` was removed by exact reviewed managed GC with recovery proved through remote main and PR #23. Its merged local branch was deleted normally. The managed apply result and subsequent worktree inventory confirm removal. Other sessions' trees and older recovery trees were preserved.
