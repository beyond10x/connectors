---
format: aep.planning-md/1
id: story:coverage-regains-its-second-direction
kind: story
status: draft
title: 'Coverage regains its second direction: every gap between declared and published has a reason'
refs:
- provider: legacy
  reference: S-021
relations:
- derived_from: epic:post-m1
revision: 1
---
## Acceptance

Verbatim from `docs/stories/S-021-coverage-regains-its-second-direction.md:40`. **read**

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

## Context

Restore the coverage direction the consolidation lost — *every operation a vendored document declares
and this catalogue does not publish has a written reason* — and restore it **as data**, iterated over
the whole catalogue, so it never becomes a per-provider test file again.

Source frontmatter: pillar Catalog · areas [catalog-build, providers]. **read**

Source `note:` field, quoted: “M1 folded sixteen per-provider test files into one parameterised invariants file — right, and it dropped a direction: the per-vendor declared-operation count with a written reason per gap (babelforce's 389 + 5 + 3 = 397, checked both ways). Today an endpoint no patch mentions is neither invented nor a failed selector, so nothing fails and nothing explains it”

## Status

`backlog` in the source. Quoted from `docs/stories/S-021-coverage-regains-its-second-direction.md:5`: `status: backlog`. **read**

## Provenance

Migrated from `docs/stories/S-021-coverage-regains-its-second-direction.md`, which is not deleted and now names this artifact.

- First written 2026-08-13 · last touched 2026-08-13 · 1 revision(s)
- Legacy id `S-021`, recorded as the reference `legacy:S-021`
- Migrated 2026-09-04 by the `aep-planning:story-migration` skill
