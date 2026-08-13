---
id: S-021
title: "Coverage regains its second direction: every gap between declared and published has a reason"
pillar: Catalog
status: backlog
priority:
design:
epic: post-m1
areas: [catalog-build, providers]
note: "M1 folded sixteen per-provider test files into one parameterised invariants file — right, and it dropped a direction: the per-vendor declared-operation count with a written reason per gap (babelforce's 389 + 5 + 3 = 397, checked both ways). Today an endpoint no patch mentions is neither invented nor a failed selector, so nothing fails and nothing explains it"
---

# Coverage regains its second direction: every gap between declared and published has a reason

## Goal

Restore the coverage direction the consolidation lost — *every operation a vendored document declares
and this catalogue does not publish has a written reason* — and restore it **as data**, iterated over
the whole catalogue, so it never becomes a per-provider test file again.

## What the consolidated invariant does and does not assert

`crates/catalog-build/tests/main/catalog_invariants.rs` §7
(`spec_backed_coverage_holds_in_both_directions`) asserts two things, over every spec-backed provider:

1. **Nothing invented** — every published spec-derived operation's `METHOD /path` is one some
   vendored document declares (ingest diagnostics included, so an endpoint ingest could not express
   still counts as declared).
2. **Every `[[patch.operations]]` selector lands** — the vendor `operationId` exists, and it is
   published unless it declares `defer`, in which case it must not be.

Both are real, and both are about operations somebody already **named**. The direction that is gone
is the one the deleted per-provider files carried: an endpoint the vendor declares that **no patch
mentions at all** is not invented, is not a failed selector, and is not counted anywhere. It simply
does not appear. The predecessor's `babelforce_coverage.rs` closed exactly that hole with an
accounting — `389 + 5 + 3 = 397`, checked in both directions — and AGENTS.md still promises the
property in prose: *"Coverage tests hold in both directions (an allowlist entry may not outlive the
gap it explains)."*

## Acceptance

- [ ] For every spec-backed provider the three numbers are **derived at test time** — declared by the
      vendored documents, published by the catalogue, and the difference — and the difference is
      **fully accounted for**: every operation in it is covered by a written reason. No free-floating
      remainder, and no "the rest are out of scope" bucket.
- [ ] Reasons are **authored where the connector is reviewed** (the provider declaration's existing
      `defer` shape is the precedent), not constants in a test file — so declining an endpoint is a
      reviewed decision in the same diff as the connector, and the reason text is prose a reader can
      judge.
- [ ] Counts are never hand-typed. If any number is committed, the test recomputes it from the bytes
      on disk and fails on drift (C-81's standing lesson: hand-maintained numbers are a defect class,
      and this repository already ships `plan.providers.len() >= 55` as the shape of the guard).
- [ ] **An entry may not outlive the gap it explains** — a reason naming an operation that is now
      published, or one no vendored document declares any more, fails by name. This is the half the
      predecessor's allowlists had and the current invariant lost, and it is what makes a spec refresh
      surface vanished operations instead of silently dropping them (AGENTS.md § Refreshing a source,
      step 3).
- [ ] The rule is stated **once and parameterised** over the catalogue, inside the consolidated
      invariants file. No per-provider test file returns — the boundary AGENTS.md draws is not
      relaxed by this story.
- [ ] **Failing-first:** remove one published operation's selector from a provider and the test names
      it as an unexplained gap; today nothing fails. Then add a reason and watch it pass, and delete
      the operation from the vendored document and watch the reason fail as stale.
- [ ] The whole-catalogue guard survives: the test still refuses to pass on a catalogue too small to
      mean anything (the existing `spec_backed >= 7` shape).

## Progress
- (not started)

## Notes

- The shape question worth deciding first: whether the gap reasons live per-provider in
  `providers/<id>.toml` (reviewed with the connector, but scattered) or in one counts-and-reasons
  file (one place to read the catalogue's coverage, but a second file to keep in step). The rule this
  story enforces is identical either way; record the choice and why.
- This is a *coverage* story, not a *selection* story: it must not become an argument for publishing
  more operations. AGENTS.md's ingest rule stands — a 398-operation document with no patch yields
  zero operations, deliberately — and declining an endpoint with a good reason is a passing state.
- Related: [S-016](S-016-sources-are-processed-by-code.md) makes the *source* index checked in both
  directions; this makes the *operation* inventory checked in both directions. Same discipline, one
  level down.
