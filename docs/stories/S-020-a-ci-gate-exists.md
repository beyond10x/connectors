---
id: S-020
title: "A CI gate exists, and it runs what the repository claims it runs"
pillar: Catalog
status: ready
priority: 6
design:
epic: post-m1
areas: [ci, catalog-build, web]
note: "M1 report: there is no .github/ directory at all. Nothing builds this workspace on its declared MSRV (msrv_fence.rs says so in prose), the node suites run nowhere, and two committed tests already assert workflow properties that cannot hold. Green currently means 'somebody ran it locally'"
---

# A CI gate exists, and it runs what the repository claims it runs

## Goal

Give the repository a mechanical gate: a workflow that builds, tests, lints, formats, rebuilds the
catalogue and verifies it, on the declared MSRV as well as on stable — so "green" is a fact about a
commit rather than a report about somebody's laptop. Everything else here is a checked claim; the
checking itself currently is not.

## What M1 left

- **No `.github/` directory exists.** Every fence, invariant and determinism test in the workspace
  runs only when a human runs it.
- **The declared MSRV is untested, and the fence says so.** `[workspace.package] rust-version =
  "1.87"` is inherited by every crate; `crates/catalog-build/tests/main/msrv_fence.rs` reads
  *declarations* and states its own limit in the header — *"nothing builds this workspace on 1.87 —
  there is no CI yet at all"* — with `the_declared_msrv_is_below_the_toolchain_this_runs_on` existing
  precisely so the fence cannot be mistaken for coverage it does not have.
- **`cargo metadata --locked --offline` needs a fetched registry.** The MSRV fence and the no-network
  fence both shell out to it, and a partially-fetched registry has already broken it once
  (`zerocopy-derive`). A fence that fails for an environmental reason is indistinguishable, at the
  exit code, from one that found a breach.
- **Two committed node tests already assert workflow properties** — `web/test/ci_gate.test.mjs`
  (some workflow a pull request triggers builds the site and runs its suite, and the gate AGENTS.md
  documents is the gate the workflow enforces) and `web/test/release_assets.test.mjs`. Neither can
  pass against a repository with no workflows.

## Acceptance

- [ ] `.github/workflows/ci.yml` exists, triggers on pull request and on push to the default branch,
      and runs the Rust gate: `cargo build --workspace`, `cargo test --workspace --no-fail-fast`,
      `cargo clippy --workspace --all-targets -- -D warnings`, `cargo fmt --all --check`.
- [ ] The **catalogue** is gated, not just the code: `catalog build` → `diff` must report everything
      up to date → `catalog check` ([S-003](S-003-the-lockfile-gets-a-verifier.md)). A drifted
      artifact fails CI instead of reaching a reviewer, which is what makes review-equals-execution
      mechanical.
- [ ] **`cargo fetch` runs before any job that invokes `cargo metadata --locked --offline`** (the MSRV
      and no-network fences). The workflow states why in a comment, so the step is not later removed
      as redundant.
- [ ] A job builds and tests the workspace **on the declared MSRV toolchain**, so `rust-version` stops
      being an unchecked number. If the workspace does not build on 1.87, the declared MSRV moves to
      the version that does — the fence's point is that the number is *true*, not that it is low —
      and `msrv_fence.rs`'s "there is no CI yet at all" paragraph is corrected to describe the
      coverage that now exists.
- [ ] A web job runs `npm ci && npm run build && npm test` in `web/`, landing together with
      [S-018](S-018-the-explorer-works-against-the-new-site-json.md) so it is green on arrival, and
      `web/test/ci_gate.test.mjs`'s wiring assertions pass against the real workflow.
- [ ] AGENTS.md gains an explicit **gate** section naming the exact commands, and the workflow and
      that list are held together by a test rather than by discipline (`ci_gate.test.mjs` is the
      shape, and its own circularity note is worth reading before copying it).
- [ ] Toolchains and third-party actions are pinned by version; any in-workflow commit uses the
      Actions `GITHUB_TOKEN` (`github-actions[bot]`), never the app key and never a PAT
      (AGENTS.md § Automation identity).
- [ ] **Failing-first:** seed one breach per gate arm — a formatting violation, a clippy warning, a
      hand-edited canonical document — and record that the workflow fails on each. A gate never
      observed failing is a gate nobody has tested.

## Progress
- (not started)

## Notes

- Ordering: this arguably outranks the schema wave, because every "green" claim the other stories
  make is currently unverified — but it is ranked below them to avoid renumbering work already
  underway. If the coordinator disagrees, move it to 1; nothing in it depends on the wave.
- Keep the workflow small and readable. Two committed tests already read this YAML with a hand-rolled
  reader, so a clever generated workflow would break the thing that guards it.
- Out of scope, deliberately: release workflows. Architecture §8 says there are no release artifacts
  pre-v1, so the tag/asset half — and `release_assets.test.mjs`'s fate — belongs to S-018's suite
  triage and to the milestone that creates a release train.
