---
format: aep.planning-md/1
id: task:release-0-7-0
kind: task
status: active
title: Cut and verify Connectors v0.7.0
relations:
- derived_from: runbook:cli-ten-slack-first
- decomposes: story:release-0-7-0
revision: 5
---
## Outcome

Publish and verify Connectors v0.7.0 from the reviewed nine-story implementation, after the operator explicitly approved the release cut.

## Acceptance

The approved v0.7.0 tag produces a successful release workflow and four published Unix archives whose downloaded checksums and native Linux version/help are verified.

## Delivery checks

- Update every package and internal dependency version from 0.6.5 to 0.7.0; preserve the external dependency graph and refresh every nested lock with Cargo.
- Regenerate the catalog and lock through their owner, prove deterministic diff/check, and date the complete release changelog.
- Verify the exact release source with the authoritative twelve-workspace gate, shared checks, history scan and four native builds. Publish only bot-authored direct commits and the approved v0.7.0 tag.
- Require a successful tag workflow and published GitHub Release with all four Unix archives and SHA256SUMS. Download and verify the archives and native Linux version/help.
- Record results and preserve the held several-credentials story, development-only OAuth limits and all existing unrelated obligations.

## Scope

Cited from the current manifest inventory, gate script and release workflow: Cargo.toml, crates, catalog, connectors.lock and CHANGELOG.md. Changes under crates are package/dependency identities and Cargo lockfiles only. Generated catalog changes are version identity and derived digests only. Existing product behavior and workflow bytes are retained.

## Preparation checkpoint — 2026-09-07

Updated 206 classified internal version references across 29 manifests, refreshed all twelve locks with Cargo, and dated the changelog. Semantic comparison proves external dependency specifications and every external dependency graph unchanged. All twelve workspace format checks, repository-portable markdown links, story consistency and diff whitespace checks passed. Product implementation and release workflow bytes are unchanged.

Catalog compilation remains pending: the guarded runner refused before launching because the agreed 12 GiB disk reserve was unavailable. An earlier local-identity check invoked compilation and was interrupted; its process group is gone and its owned cache was cleaned. That check has no passing result and will run in CI. Completed build caches owned by this session were retired after verifying no active users and preserving all source hashes.

The disk subsequently filled during a scratch checkpoint write. AEP refused the empty input before changing this task; no evidence was lost. Free space began recovering afterward, but the reserve must pass before compiling.

No release commit, push or tag has occurred. Catalog generation, full CI, history scan, main integration, tag publication and downloaded archive verification remain outstanding. The operator's release approval remains valid.

## Release source ready — 2026-09-07

After the operator instructed "just release it now", the offline catalog compiler ran in a private memory-backed target with a 2 GiB size cap, one compiler job, and monitored disk, tmpfs and memory reserves. It completed without an interruption. Catalog build, two diff runs and two independent checks succeeded: 65 providers and 70 artifacts verified. Semantic comparison proves the 65 changed catalog documents differ only in generator version, with the lock and pack changing only the corresponding identities and derived digests; frozen artifacts are unchanged.

The operator also requested plain-language release highlights at the repository root. WHATS-NEW.md now explains the latest release for users and consumers, with a link from README.md and inclusion in each archive. CHANGELOG.md remains the detailed historical record. Documentation delivery will refresh the Website source lock after source publication.

The approved tag workflow will run the full gate and four native builds before it can publish. This source is not yet claimed as fully verified or released.
