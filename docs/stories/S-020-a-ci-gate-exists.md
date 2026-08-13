---
id: S-020
title: "A CI gate exists, and it runs what the repository claims it runs"
pillar: Catalog
status: in-progress
priority: 6
design:
epic: post-m1
areas: [ci, catalog-build, web]
note: "The repository now gates governance, Rust, catalogue rebuild/diff/check and MSRV. S-018 and this story still own the web arm before S-020 can close"
---

# A CI gate exists, and it runs what the repository claims it runs

## Goal

Give the repository a mechanical gate: a workflow that builds, tests, lints, formats, rebuilds the
catalogue and verifies it, on the declared MSRV as well as on stable — so "green" is a fact about a
commit rather than a report about somebody's laptop. The Rust/governance arm now exists; the
unchecked acceptance items below name the remaining web and verifier arms.

## What M1 left

- The first gate now covers repository links, story-index consistency, the Rust workspace, generated
  catalogue drift, the independent lock verifier, and the declared MSRV. The web suite remains the
  one absent arm rather than a present-tense claim.
- **`cargo metadata --locked --offline` needs a fetched registry.** The MSRV fence and the no-network
  fence both shell out to it, and a partially-fetched registry has already broken it once
  (`zerocopy-derive`). A fence that fails for an environmental reason is indistinguishable, at the
  exit code, from one that found a breach.
- **Two committed node tests already assert workflow properties** — `web/test/ci_gate.test.mjs`
  (some workflow a pull request triggers builds the site and runs its suite, and the gate AGENTS.md
  documents is the gate the workflow enforces) and `web/test/release_assets.test.mjs`. They remain
  outside the landed gate until S-018 repairs the migrated site contract.

## Acceptance

- [x] `.github/workflows/ci.yml` exists, triggers on pull request and on push to the default branch,
      and runs the Rust gate: `cargo build --workspace`, `cargo test --workspace --no-fail-fast`,
      `cargo clippy --workspace --all-targets -- -D warnings`, `cargo fmt --all --check`.
- [x] The **catalogue** is gated, not just the code: read-only `catalog check` first verifies the
      committed state; `catalog build` then has to leave no tracked Git diff and `catalog diff`
      must report everything up to date ([S-003](S-003-the-lockfile-gets-a-verifier.md)). A drifted
      artifact fails CI instead of reaching a reviewer, which is what makes review-equals-execution
      mechanical.
- [x] **`cargo fetch` runs before any job that invokes `cargo metadata --locked --offline`** (the MSRV
      and no-network fences). The workflow states why in a comment, so the step is not later removed
      as redundant.
- [x] A job builds and tests the workspace **on the declared MSRV toolchain**, so `rust-version` stops
      being an unchecked number. If the workspace does not build on 1.87, the declared MSRV moves to
      the version that does — the fence's point is that the number is *true*, not that it is low —
      and `msrv_fence.rs`'s "there is no CI yet at all" paragraph is corrected to describe the
      coverage that now exists.
- [ ] A web job runs `npm ci && npm run build && npm test` in `web/`, landing together with
      [S-018](S-018-the-explorer-works-against-the-new-site-json.md) so it is green on arrival, and
      `web/test/ci_gate.test.mjs`'s wiring assertions pass against the real workflow.
- [x] AGENTS.md gains an explicit **gate** section naming the exact landed commands. A wiring test
      for the future web arm remains with S-018; story-index and link checks are already mechanical.
- [x] Toolchains are pinned by version and third-party actions by full commit SHA (with the release
      tag in a comment); any in-workflow commit uses the
      Actions `GITHUB_TOKEN` (`github-actions[bot]`), never the app key and never a PAT
      (AGENTS.md § Automation identity).
- [ ] **Failing-first:** seed one breach per gate arm — a formatting violation, a clippy warning, a
      hand-edited canonical document — and record that the workflow fails on each. A gate never
      observed failing is a gate nobody has tested.

## Progress
- Landed the pinned Rust 1.97.0 and MSRV 1.87.0 jobs, link/story governance checks, locked prefetch,
  build, tests, clippy, format, catalogue rebuild, drift rejection, and S-003's independent offline
  lock/input/artifact verification. The verifier's seeded artifact mutation supplies the catalogue
  arm's failing-first evidence.
- Still open by design: S-018's repaired web build/tests and recorded failing-first evidence for the
  other complete arms. Stable release remains gated by architecture ADR 0020 while private forge
  enforcement is unavailable.

## Notes

- **Suite-speed facts CI should assume (measured 2026-08-13):** the workspace suite was 289 s
  wall on 20 cores, 96% of it two binaries, and the whole wall was one test's critical path —
  `verification_conformance::every_shipped_hmac_scheme_is_covered_by_the_matrix` at 200 s,
  because every whole-catalogue sweep re-ingested 21 MB of vendored specs in debug-mode serde.
  Two fixes landed the same day: `[profile.dev.package."*"] opt-level = 2` in the root manifest
  (dependencies optimized, workspace crates still opt 0 — CI caches should key on it), and a
  memoized undoctored shipped-provider load in `tests/support/shipped_provider.rs` plus a cached
  `full_plan()` in `catalog_invariants.rs`. If CI adopts **cargo-nextest**, know both cache wins
  vanish (process per test) — nextest is worth having for per-test timing and slow-test flagging,
  not for wall time here; measure before switching the gate to it.
- Ordering: this arguably outranks the schema wave, because every "green" claim the other stories
  make is currently unverified — but it is ranked below them to avoid renumbering work already
  underway. If the coordinator disagrees, move it to 1; nothing in it depends on the wave.
- Keep the workflow small and readable. Two committed tests already read this YAML with a hand-rolled
  reader, so a clever generated workflow would break the thing that guards it.
- Out of scope, deliberately: release workflows. Architecture §8 says there are no release artifacts
  pre-v1, so the tag/asset half — and `release_assets.test.mjs`'s fate — belongs to S-018's suite
  triage and to the milestone that creates a release train.
