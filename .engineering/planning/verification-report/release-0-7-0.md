---
format: aep.planning-md/1
id: verification-report:release-0-7-0
kind: verification-report
status: draft
title: Connectors v0.7.0 release verification
relations:
- verifies: story:release-0-7-0
- verifies: task:release-0-7-0
revision: 2
---
## Verified release

[Connectors v0.7.0](https://github.com/beyond10x/connectors/releases/tag/v0.7.0) was published on 2026-09-07. The source and annotated tag identify commit [e80b7ae](https://github.com/beyond10x/connectors/commit/e80b7ae1b2151d13aa9786cf67ea05e66717ee35). Direct source and tag authorship is the organization bot; the release workflow publishes the assets as github-actions[bot].

[Release workflow 34097008095](https://github.com/beyond10x/connectors/actions/runs/34097008095) completed successfully: all twelve workspace gates, shared catalog/documentation and history checks, the release-profile refusal, four native builds and smoke tests, and publication passed.

Every archive was downloaded and compared with the published SHA256SUMS. README.md, CHANGELOG.md and WHATS-NEW.md inside every archive match the tagged source. The downloaded x86_64 Linux executable reports connectors 0.7.0 and its help command exits successfully.

| Archive platform | Verified SHA256 |
|---|---|
| x86_64-unknown-linux-gnu | a41caf70824aa57be5e272ba0768deed55361a3d6fe073afa5d7e77be3d35502 |
| aarch64-unknown-linux-gnu | ca237b6b707f71a998643618f6d4073ec6f55c3dadcad4e41a3e07b41183e588 |
| x86_64-apple-darwin | 7a9ca1d254d241b0589ac689f6458391b7dbe82822394665ed61df4dcde79f4e |
| aarch64-apple-darwin | 0e6a66fbb02f1410ce4800ebb6578cadab4109063710ec906ab02e7e56056127 |

## Scope and preparation

The release includes the nine implemented CLI stories and changes since the previous downloadable v0.6.0 release. All 206 internal version references, 29 manifests and twelve Cargo lockfiles moved together. External dependency specifications and graphs are unchanged. Owner generation and two independent diff/check pairs verified 65 providers and 70 artifacts; generated changes are release identities and derived digests only, and frozen artifacts are unchanged.

WHATS-NEW.md gives users and consumers plain-language release highlights and upgrade notes, links from the documentation overview and ships in all archives. Multiple credentials remains excluded, and personal GitLab OAuth remains unsealed development functionality.

## Documentation delivery

The source bundle for the tagged release passed in run 34098024830. Website [PR 10](https://github.com/beyond10x/website/pull/10) published the refreshed source lock and Atlas-generated snapshot. The full Website gate passed 99 tests, its CI passed, and the managed Atlas portal check passed.

[Publication 34101442041](https://github.com/beyond10x/atlas/actions/runs/34101442041) selected the exact released Connectors source and completed successfully. Independent complete-layout verification passed for 357 routes and 1,332 site files. All 1,714 files in the self-contained publication match the durable Git artifact. Live root provenance and the Connectors overview return HTTP 200 and match the artifact byte for byte; the overview contains the WHATS-NEW.md link. Source-set freshness passed without an advanced source. The live Pages verifier passed 37 repository states, 26 Pages repositories and 52 delivery routes.

The public website release feed still retains older release facts. The updated Website snapshot contains v0.7.0, but Atlas bundle reconciliation reuses the previous publication's release facts. Website story publish-connectors-release-highlights retains this explicit remaining acceptance gap; the GitHub release and requested Connectors documentation are delivered.

## Operational observations

A mistaken legacy Website dispatch was cancelled before Pages settings or deployment mutations. The successful publication used the current immutable-bundle reconciliation mode. No release artifact changed as a result.

The complete organization fence ran unchanged and finished with three existing primary-workspace failures: documentation manifest compatibility, the primary Website dependency pin, and an ungrounded repository objective. Rust tests, catalog, public Pages delivery, projections, markdown and brand checks passed. This is not a claim that the whole organization fence is green.

Session-owned build caches were retired with source preservation checks. Finished release worktrees are ready for manager-controlled removal once these completion records are published. Older unpublished recovery work remains preserved. The several-credentials hold and existing public-history provenance obligation remain unchanged.
