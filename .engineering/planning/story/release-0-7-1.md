---
format: aep.planning-md/1
id: story:release-0-7-1
kind: story
status: implemented
title: Release Connectors 0.7.2
summary: Retargeted after concurrent v0.7.1 tag; immutable ID retains original planning identity
relations:
- derived_from: runbook:small-wave-after-0-7-0
scope:
- confidence: cited
  path: CHANGELOG.md
- confidence: cited
  path: Cargo.toml
- confidence: cited
  path: WHATS-NEW.md
- confidence: cited
  path: catalog
- confidence: cited
  path: connectors.lock
- confidence: cited
  path: crates
- confidence: cited
  path: docs/architecture/interfaces.md
revision: 16
---
## Acceptance

The approved Connectors v0.7.2 release contains the reviewed `inspect upgrade` command, preserves published main, passes the authoritative release checks, publishes four Unix archives with matching checksums and consumer notes, delivers updated public documentation, and retires only safely published wave-owned worktrees.

## Scope

Release identity across internal Cargo manifests/locks and generated catalog artifacts; CHANGELOG.md and WHATS-NEW.md; command documentation; source and tag delivery through Atlas; deterministic Website source lock and Atlas snapshot publication. Keep external dependency versions, frozen contracts and release workflow semantics unchanged relative to the integrated published-main baseline. The feature remains `story:the-binary-says-what-it-carries`; v0.7.1 source-read changes and earlier OAuth/remediation fixes are preserved as prior release history. Only the inspection feature is new in this release's top-level notes.

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

## Published v0.7.2 and verified artifacts

The approved release is published at https://github.com/beyond10x/connectors/releases/tag/v0.7.2 (2026-09-07T13:08:04Z). Exact source/main commit: `c30e4f2475f6288b5e83c87ed09843882a882123`; annotated tag object: `8b4205e80698421794c829094b906103bd032c01`. PR #25 is merged. GitHub's merge API initially reported branch-policy refusal with no pending checks/reviews; the already verified bot-authored and bot-committed release commit was published through Atlas's authorized fast-forward push. No rule, hook or authority setting changed.

Actual tag workflow https://github.com/beyond10x/connectors/actions/runs/34123190042 completed successfully: all twelve workspace gates, every shared check including tag/version and CHANGELOG agreement, four native Unix build/smoke jobs and publication. Native Cargo summaries report 180 target summaries, 2,673 passed executions, 0 failed and 27 existing ignored cases, including both runtime feature configurations. Complete gate logs and job JSON are retained in coordinator evidence.

The four published Linux/macOS archives and SHA256SUMS were downloaded. All four checksums passed. Every archive contains the binary, README, CHANGELOG and WHATS-NEW. The published x86_64 Linux archive is byte-identical to the CI artifact whose binary reported 0.7.2 and passed inspect-upgrade in text, compact, JSON and YAML under an empty environment with TOKIO_WORKER_THREADS=0. The source pack's independently computed in-band digest equals the binary report; stored format strings ["1","2"], schema 4 and session metadata 1 match their owners.

The coordinator's initial jq assertion incorrectly expected numeric credential versions and the pack's whole-file hash. The owning source establishes string versions and an in-band content digest. Correcting those two probe assumptions preserved all checks; no product code or test changed. Full native probe results and stdout/stderr are retained in `ci-linux-072/` within coordinator evidence. No installed CLI or provider configuration was replaced.

The unit tree and merged local branch have been removed through managed GC and ordinary branch deletion. Three unit build targets plus the coordinator catalog target were cleaned natively. Assigned unit scratch roots and private compiler cache were removed after tar comparison and checksum verification of the retained 1,968-member evidence archive; stale inactive socket nodes are not serializable tar members. No listener remained in those scratch roots, and the private sccache port had already exited (a stop request received connection refused; absence was verified). Other servers and worktrees were preserved.

## Documentation delivered and cleanup handoff

Website's full 19-step gate passed at `29b944d2dec6d49a852c71ade13ef9c6ff1d48f0`: 99 tests, no failures or skips; 357 routes and 23,847 references verified. Atlas's scoped portal check passed for 24 sources, 25 surfaces and 52 delivery records. The complete source-freshness check passed before Website publication. Native Atlas snapshot generation recorded the actual published v0.7.2 release.

Atlas reconciliation https://github.com/beyond10x/atlas/actions/runs/34126589527 succeeded. Live provenance names Connectors release source `c30e4f2475f6288b5e83c87ed09843882a882123` and source-set SHA256 `01491a19e72315310675a2d76aca04a69cff3fb4882f5127b2ee1ef5f7b62607`; the live commands-and-interfaces page contains inspect upgrade. Native root Pages verification passed 27 delivery routes. Website runtime and Docs System pins remain unchanged. The existing stale production release feed remains separately tracked by Website's `story:publish-connectors-release-highlights`; the command documentation, root WHATS-NEW notes and GitHub release are current.

Release and documentation evidence is retained in `~/.cache/connectors-small-wave-20260907`, including the checked downloads, job logs, live provenance and an independently compared unit-evidence archive (SHA256 `19ee2cbcdc3836d0830128acbe6d6002a8a2a848552085c7cb8ca2560ccd1e93`). Unit targets/scratch/private cache and Website build/dependency caches were removed after preserving evidence. No active listener remained in assigned test roots; the private compiler server was already absent. The unit managed tree and its merged local branch are already removed.

Publish this closing record, then finish and inspect exact-id GC for the remaining owned trees only: Connectors `wt-83fe13d0c209`, Website `wt-connectors-071-docs-20260907`, and Atlas `wt-840d0a4626a0`. Their actual removal results and final branch inventory belong to the retained native cleanup evidence; this paragraph records the authorized next operation rather than claiming removal before it happens. Older recovery trees and all other sessions' work remain outside that cleanup.
