---
format: aep.planning-md/1
id: story:the-lockfile-gets-a-verifier
kind: story
status: implemented
title: '`catalog check` verifies every addressable hash and refuses unverifiable claims'
refs:
- provider: legacy
  reference: S-003
relations:
- derived_from: epic:catalog-day-one
scope:
- confidence: cited
  path: crates/catalog-build
- confidence: cited
  path: crates/connector-spec
revision: 6
---
## Acceptance

Verbatim from `docs/stories/S-003-the-lockfile-gets-a-verifier.md:31`. **read**

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

## Context

Give `connectors.lock` the verifier it never had, so vision principle 1 — *review equals execution;
every derived form is byte-identical to the committed canonical documents* — is enforced by a command
CI runs rather than asserted in a CHANGELOG. Drift is not preventable; it is **detectable**, and that
is the whole design.

Source frontmatter: pillar Catalog · areas [catalog-build, connector-spec]. **read**

Source `note:` field, quoted: “the predecessor's lockfile has a writer (C-189) and no verifier (C-14, never built), so provenance is computed, committed, and never checked. Architecture §2 day-one change 2. Review-equals-execution is a claim until something recomputes it in CI”

## Status

`done` in the source. Quoted from `docs/stories/S-003-the-lockfile-gets-a-verifier.md:5`: `status: done`. **read**

This artifact reached `implemented` with `aep artifact move --evidence test_result=1`. The journal
records that move as resting on an **assertion**, not on a run this migration observed. The flag is
what the CLI provides for evidence that lives outside the store.

What was asserted, and where it came from:

- The source records `status: done` at the line quoted above. **read**
- `bash scripts/gate.sh` was green at commit `a48030b` on 2026-09-04 — exit 0, 136 `test result: ok`
  lines across 11 workspaces. **read**, from `~/.cache/connectors-gate/gate2.log`

No per-story run was attributed to this story. The gate is a repository-wide fact, and reading it as
proof of one story's acceptance would be an inference this record does not make.

## Provenance

Migrated from `docs/stories/S-003-the-lockfile-gets-a-verifier.md`, which is not deleted and now names this artifact.

- First written 2026-08-13 · last touched 2026-08-13 · 4 revision(s)
- Legacy id `S-003`, recorded as the reference `legacy:S-003`
- Migrated 2026-09-04 by the `aep-planning:story-migration` skill
