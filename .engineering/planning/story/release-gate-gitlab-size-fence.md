---
format: aep.planning-md/1
id: story:release-gate-gitlab-size-fence
kind: story
status: implemented
title: Release runs v0.5.3 to v0.5.6 fail the size fence on the GitLab backend
summary: backend.rs is 2895 lines against a 2869-line waiver, and 28 manifests still say 0.5.3; every tag since v0.5.3 fails before building.
scope:
- confidence: cited
  path: crates/catalog-build/tests/main/architecture_fence.rs
- confidence: cited
  path: crates/integration-gitlab/src
revision: 5
---
## Context

`gh run list --workflow=release.yml` on 2026-09-04 shows v0.5.3, v0.5.4, v0.5.5 and v0.5.6 all
`failure`; the latest published GitHub release is v0.5.2. Each run fails in job `gate .` at
`architecture_fence::production_modules_obey_the_named_size_fence` (run 33765153362):
`crates/integration-gitlab/src/backend.rs` grew to 2895 lines beyond its 2869-line waiver. Commit
`d3707aa` (gitlab: support nested repository file paths, merged as PR #5) added the lines without
touching the waiver.

The same runs would then have failed the matrix leg's `--version` check: 28 crate manifests,
`crates/connectors-cli/Cargo.toml` among them, still carry `version = "0.5.3"` while
`[workspace.package] version` is 0.5.6, so the built binary reports 0.5.3
(`grep -rl '^version = "0.5.3"' crates/*/Cargo.toml`). The v0.5.3 release commit `3df0b1f` bumped
every manifest; the v0.5.4 to v0.5.6 commits bumped only the root.

## Acceptance

`cargo test -p catalog-build --test main architecture_fence` passes at the root, every manifest
under `crates/` carries the released version, and the next `v*` tag's release workflow reaches the
`publish` job.

## Scope

- `crates/integration-gitlab/src/backend.rs` and/or
  `crates/catalog-build/tests/main/architecture_fence.rs` — bring the file back under its waiver,
  or record the raise with the commit that caused it.
- The 28 manifests carrying `version = "0.5.3"` — bumped as part of the next release cut.
