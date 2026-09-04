---
format: aep.planning-md/1
id: story:retire-the-quirks-umbrella
kind: story
status: implemented
title: Retire the `quirks` umbrella — pagination, rate limits and error envelopes are ordinary facts
refs:
- provider: legacy
  reference: S-015
relations:
- derived_from: epic:catalog-day-one
scope:
- confidence: cited
  path: crates/catalog
- confidence: cited
  path: crates/catalog-build
- confidence: cited
  path: crates/connector-spec
revision: 6
---
## Acceptance

Verbatim from `docs/stories/S-015-retire-the-quirks-umbrella.md:54`. **read**

- [x] `pagination`, `rate_limit` and `error_envelope` are **first-class named fields** at their
      correct scope in `catalog/connector-document.schema.json` and in the lowering — per operation
      where they vary by endpoint, on the service where they are a property of the whole surface.
      The scope choice per field is decided once and recorded, not left to each provider.
- [x] **No `quirks` key remains anywhere**: not in the schema, not in any canonical document, not in
      the pack, not in any projection (explorer, catalogue view, effective catalogue), not in a
      provider declaration. Every committed document is regenerated — a whole-catalog change, so the
      regeneration is coordinator-owned.
- [x] `grep -ri quirk` across `catalog/` and `crates/` returns **only historical references in
      documentation** (design series, story files, CHANGELOG). A hit in a type name, a field name, a
      serde attribute or a test name fails the story.
- [x] The genuinely deviant declarations are handled deliberately, in one of two ways, and the reason
      is recorded: either a rare, precisely scoped **`workarounds`** category is introduced in which
      **each entry names the specific vendor defect it compensates for** and carries the predecessor's
      required attribution (`grant`/`behaviour`/`attribution`/`measured` — an unattributed claim is
      indistinguishable from a guess that aged), or nothing is introduced at all and the babelforce
      token-endpoint measurements find a home on the auth declaration itself. **Nothing generic is
      introduced.** If it is not in the specification, it does not become a general thing.
- [x] The migration is a rename with proof, not a redesign: the promoted fields' contents are
      unchanged, and a test shows that for every provider, the set of declared pagination /
      rate-limit / error-envelope facts before and after is identical. Any *behavioural* change to
      those fields belongs to [S-005](../../../docs/stories/S-005-header-name-rate-limit-retry.md), not here.
- [x] Determinism and the fixed point hold after regeneration: two independent builds are
      byte-identical, `catalog check` ([S-003](../../../docs/stories/S-003-the-lockfile-gets-a-verifier.md)) is
      green on the new lock, and the build reports nothing left to write.
- [x] Failing-first test named — today the schema refuses a top-level `pagination` on an operation
      and requires the umbrella.

## Context

Delete the `quirks` key and promote what it holds to first-class named fields on the operation and
the service, so the schema stops calling universal API traits *deviations*. Pagination, rate limits
and error envelopes are how HTTP APIs work; filing them under a word that means "this vendor is
broken" mis-describes ~30 shipped providers and makes the one genuinely deviant case invisible
inside the same bag.

Source frontmatter: pillar Catalog · areas [catalog, catalog-build, connector-spec]. **read**

Source `note:` field, quoted: “owner-raised: `quirks` began as a workaround bag for the broken babelforce API and drifted into a junk drawer holding universal API traits. Every other catalog in the field declares these as plain fields (Nango paginate/retry, Airbyte paginator/error_handler). Schema evolution — must run AFTER the M1 byte-identity differential passes, same wave as S-001/S-002”

## Status

`done` in the source. Quoted from `docs/stories/S-015-retire-the-quirks-umbrella.md:5`: `status: done`. **read**

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

Migrated from `docs/stories/S-015-retire-the-quirks-umbrella.md`, which is not deleted and now names this artifact.

- First written 2026-08-13 · last touched 2026-08-13 · 4 revision(s)
- Legacy id `S-015`, recorded as the reference `legacy:S-015`
- Migrated 2026-09-04 by the `aep-planning:story-migration` skill
