---
format: aep.planning-md/1
id: story:a-ci-gate-exists
kind: story
status: active
title: A CI gate exists, and it runs what the monorepo claims it runs
refs:
- provider: legacy
  reference: S-020
relations:
- derived_from: epic:post-m1
scope:
- confidence: cited
  path: crates/catalog-build
revision: 4
---
## Acceptance

Verbatim from `docs/stories/S-020-a-ci-gate-exists.md:36`. **read**

- [ ] A monorepo hosted workflow triggers on pull requests and pushes to the default branch and
      invokes the root exhaustive gate without recreating a component-private workflow.
- [x] The **catalogue** is gated, not just the code: read-only `catalog check` first verifies the
      committed state; `catalog build` then has to leave no tracked Git diff and `catalog diff`
      must report everything up to date ([S-003](../../../docs/stories/S-003-the-lockfile-gets-a-verifier.md)). A drifted
      artifact fails CI instead of reaching a reviewer, which is what makes review-equals-execution
      mechanical.
- [x] **`cargo fetch` runs before any job that invokes `cargo metadata --locked --offline`** (the MSRV
      and no-network fences). The workflow states why in a comment, so the step is not later removed
      as redundant.
- [ ] A hosted job builds and tests the workspace **on the declared MSRV toolchain**, so `rust-version` stops
      being an unchecked number. If the workspace does not build on 1.88, the declared MSRV moves to
      the version that does — the fence's point is that the number is *true*, not that it is low —
      and `msrv_fence.rs`'s "there is no CI yet at all" paragraph is corrected to describe the
      coverage that now exists.
- [x] AGENTS.md gains an explicit **gate** section naming the exact landed commands; story-index and
      link checks are already mechanical.
- [x] Toolchains are pinned by version and third-party actions by full commit SHA (with the release
      tag in a comment); any in-workflow commit uses the
      Actions `GITHUB_TOKEN` (`github-actions[bot]`), never the app key and never a PAT
      (AGENTS.md § Automation identity).
- [ ] **Failing-first:** seed one breach per gate arm — a formatting violation, a clippy warning, a
      hand-edited canonical document — and record that the workflow fails on each. A gate never
      observed failing is a gate nobody has tested.

## Context

Give the monorepo a mechanical hosted gate that builds, tests, lints, formats, rebuilds the catalogue
and verifies it, on the declared MSRV as well as on stable — so "green" is a fact about a commit
rather than a report about somebody's laptop. The local Rust/governance/catalogue gate exists;
the unchecked acceptance items below name the hosted, web, and verifier work that remains.

Source frontmatter: pillar Catalog · areas [ci, catalog-build] · priority 6. **read**

Source `note:` field, quoted: “The monorepo local gate covers governance, Rust, and catalogue checks; hosted CI, MSRV, and failing-first evidence remain before S-020 can close”

## Status

`in-progress` in the source. Quoted from `docs/stories/S-020-a-ci-gate-exists.md:5`: `status: in-progress`. **read**

## Provenance

Migrated from `docs/stories/S-020-a-ci-gate-exists.md`, which is not deleted and now names this artifact.

- First written 2026-08-13 · last touched 2026-09-02 · 11 revision(s)
- Legacy id `S-020`, recorded as the reference `legacy:S-020`
- Migrated 2026-09-04 by the `aep-planning:story-migration` skill
