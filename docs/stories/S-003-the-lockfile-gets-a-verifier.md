---
id: S-003
title: "`catalog check` verifies every addressable hash and refuses unverifiable claims"
pillar: Catalog
status: done
design:
epic: catalog-day-one
areas: [catalog-build, connector-spec]
note: "the predecessor's lockfile has a writer (C-189) and no verifier (C-14, never built), so provenance is computed, committed, and never checked. Architecture §2 day-one change 2. Review-equals-execution is a claim until something recomputes it in CI"
---

# `catalog check` verifies every addressable hash and refuses unverifiable claims

## Goal

Give `connectors.lock` the verifier it never had, so vision principle 1 — *review equals execution;
every derived form is byte-identical to the committed canonical documents* — is enforced by a command
CI runs rather than asserted in a CHANGELOG. Drift is not preventable; it is **detectable**, and that
is the whole design.

## What the predecessor left

- C-189 built the **writer**: `connectors.lock` is a planned whole-catalog artifact, keyed by
  repository-relative path (bare file names collide across the directories one provider emits into),
  hashing provider TOML **bytes including comments**, with a per-document `specs` row list so a
  connector compiled from several documents records a hash for each.
- C-14 designed the **verifier** and it was never built: no command recomputes those hashes, so an
  edited declaration, a re-vendored spec or a hand-touched artifact reaches CI unnoticed. C-189's
  own closing note names this as the gap it deliberately left open.

## Acceptance

- [x] `catalog check` recomputes every hash in `connectors.lock` whose bytes are identified by the
      current v1 format — each provider's
      declaration bytes, each vendored spec document, each committed artifact by repository-relative
      path — and **exits non-zero** on any mismatch, naming the provider and which input moved. A
      populated predecessor-reserved hash without addressable bytes is refused as unverifiable.
      The ignored site projection is regenerated build output, not reviewed state, so it is not
      counted as a verified committed artifact.
- [x] `check` performs **no network IO**, so CI runs it offline and hermetically. Upstream drift
      (has the vendor's document moved?) is a different question with a different answer and is not
      folded in here.
- [x] Four drift classes are distinct, named failures, each proven by a seeded case: a mutated
      artifact; an edited provider declaration (**including a comment-only edit** — the hash domain
      is file bytes, and a comment can change what an operator believes the connector does); a
      re-vendored spec whose bytes changed; and a lock row that no longer matches its artifact.
- [x] Coverage is checked in both directions: a provider with no lock row fails, and a lock row with
      no provider fails. A verifier that only checks the rows it finds cannot see a deletion.
- [x] Failing-first test: a mutated committed artifact makes `check` exit non-zero and name it; a
      clean tree exits zero and prints the counts it verified (providers, artifacts) rather than a
      bare "ok".
- [x] `check` is in the repository's gate, and the gate's definition says so — a verifier nothing
      runs is the same as no verifier.

## Progress
- 2026-08-13 — done. `catalog check` now parses the committed lock, discovers providers independently,
  hashes provider and selected-spec bytes before compiling, rebuilds the full catalogue offline,
  and compares committed lock claims, current bytes, and freshly planned rows. That three-way
  comparison distinguishes artifact drift from a falsified lock digest. Provider, spec and artifact
  membership are checked in both directions; unsafe lock paths and populated hashes without
  identifiable local bytes refuse rather than being trusted.
- Failing-first was recorded before implementation: all six initial cases failed at the verifier
  stub. The completed suite covers clean count output/no writes, comment-only provider edits,
  re-vendored specs, hand-edited artifacts, falsified artifact hashes, and provider/spec/artifact
  membership in both directions. The network seam and real binary namespace tests cover `check`
  alongside `build`. The committed catalogue reports `55 providers, 58 artifacts verified`; the
  ignored site projection is regenerated separately.

## Notes

- Predecessors: flux-connectors C-7 (the hash domain and reproducible hashing), C-189 (the writer,
  the path-keying lesson, the multi-document `specs` rows), C-14 (this story's intent, plus a
  `fetch`/`--upstream` half that is deliberately **not** in scope here).
- `upstream_spec_sha256` remains absent from every row. If a future row populates it without an
  identifiable local input path, `check` refuses it as unverifiable; S-016 may later wire source
  provenance into a checkable local record rather than treating an upstream claim as verified.
- A scoped or partial build must never truncate the lockfile; it is a whole-catalog artifact. The
  predecessor's `a_scoped_build_leaves_the_lockfile_byte_identical` is the shape of the test to carry.

## Superseded by

`story:the-lockfile-gets-a-verifier` in the AEP planning store, at
`.engineering/planning/story/the-lockfile-gets-a-verifier.md`, now carries this story. Migrated 2026-09-04 by the
`aep-planning:story-migration` skill; this file is kept as the source it was migrated from and is
not the place the rung is recorded any more. The artifact quotes this file's `## Acceptance`
verbatim and names it in its own `## Provenance`.
