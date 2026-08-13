---
id: S-003
title: "`connectors catalog check` recomputes every hash and exits non-zero on drift"
pillar: Catalog
status: ready
priority: 3
design:
epic: catalog-day-one
areas: [catalog-build, connector-spec]
note: "the predecessor's lockfile has a writer (C-189) and no verifier (C-14, never built), so provenance is computed, committed, and never checked. Architecture §2 day-one change 2. Review-equals-execution is a claim until something recomputes it in CI"
---

# `connectors catalog check` recomputes every hash and exits non-zero on drift

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

- [ ] `connectors catalog check` recomputes **every** hash in `connectors.lock` — each provider's
      declaration bytes, each vendored spec document, each emitted artifact by repository-relative
      path — and **exits non-zero** on any mismatch, naming the provider and which input moved.
- [ ] `check` performs **no network IO**, so CI runs it offline and hermetically. Upstream drift
      (has the vendor's document moved?) is a different question with a different answer and is not
      folded in here.
- [ ] Four drift classes are distinct, named failures, each proven by a seeded case: a mutated
      artifact; an edited provider declaration (**including a comment-only edit** — the hash domain
      is file bytes, and a comment can change what an operator believes the connector does); a
      re-vendored spec whose bytes changed; and a lock row that no longer matches its artifact.
- [ ] Coverage is checked in both directions: a provider with no lock row fails, and a lock row with
      no provider fails. A verifier that only checks the rows it finds cannot see a deletion.
- [ ] Failing-first test: a mutated committed artifact makes `check` exit non-zero and name it; a
      clean tree exits zero and prints the counts it verified (providers, artifacts) rather than a
      bare "ok".
- [ ] `check` is in the repository's gate, and the gate's definition says so — a verifier nothing
      runs is the same as no verifier.

## Progress
- (not started)

## Notes

- Predecessors: flux-connectors C-7 (the hash domain and reproducible hashing), C-189 (the writer,
  the path-keying lesson, the multi-document `specs` rows), C-14 (this story's intent, plus a
  `fetch`/`--upstream` half that is deliberately **not** in scope here).
- `upstream_spec_sha256` was filled by nobody in the predecessor (it needed the spec provenance
  sidecars wired in). Decide here whether the migrated lock carries it or whether the field is
  dropped until something fills it — a field nothing fills detects nothing and reads as if it did.
- A scoped or partial build must never truncate the lockfile; it is a whole-catalog artifact. The
  predecessor's `a_scoped_build_leaves_the_lockfile_byte_identical` is the shape of the test to carry.
