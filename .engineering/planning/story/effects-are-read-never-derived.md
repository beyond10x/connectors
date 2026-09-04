---
format: aep.planning-md/1
id: story:effects-are-read-never-derived
kind: story
status: active
title: Per-operation effects are read from the document, never derived
refs:
- provider: legacy
  reference: S-002
relations:
- derived_from: epic:catalog-day-one
scope:
- confidence: cited
  path: crates/catalog
- confidence: cited
  path: crates/catalog-build
- confidence: cited
  path: crates/domain
revision: 4
---
## Acceptance

Verbatim from `docs/stories/S-002-effects-are-read-never-derived.md:43`. **read**

- [x] `catalog/connector-document.schema.json` carries `effects` **per operation** as a closed
      vocabulary (domain model: risk, idempotency, effects, direction). An unknown effect value is a
      build error refused by name — never a downgrade, never a default, never a catch-all arm that
      answers a level it has not heard of with a plausible wrong one.
- [x] Every operation in every provider declaration states its effects, proven by a whole-catalog
      declared-count gate: an operation with no effects declaration fails the build. Nothing infers
      effects from an HTTP verb, from `hosts`, or from `risk`.
- [ ] Grant admission in `crates/domain` reads the effects the document carries. A fence or test
      proves there is **no derivation path**: a seeded document declaring a non-network effect must
      change the admission answer, which is exactly what the derived function could not do.
- [ ] Failing-first test: an operation declaring an effect beyond `network` is admitted only by a
      selector whose effect subset contains that effect, and is refused by
      `with_effects_within([network])`. Under the predecessor's rule this test passes vacuously —
      name it so, so the regression is visible if derivation ever returns.
- [x] The predecessor's two compensating assertions are deliberately **not** carried, and the story
      records why: they existed to bound a derivation that no longer exists. A grep proving neither
      spelling survives the migration is cheap and worth having.
- [x] The relationship to the semantic tier is decided and recorded: host-resource effects (what a
      call touches) and semantic effects (what it *means* — money, delete, send-external, from
      flux-connectors C-155) are distinct axes, and collapsing them is what lost `money` in the
      predecessor. Either the schema carries both fields or the second is explicitly deferred with a
      named reason.

## Context

Make `effects` a declared fact of every operation — carried by the schema, stated by every provider
declaration, and **read** by grant admission — so the domain model's invariant holds by construction:
*the facts the authorization gate decides on are the facts the catalogue publishes.* This is
architecture §2 day-one change 1, second half, and it closes a mistake the predecessor documented
against itself rather than fixed.

Source frontmatter: pillar Catalog · areas [catalog, catalog-build, domain]. **read**

Source `note:` field, quoted: “the mistake being closed is recorded in the predecessor's own source: flux-exchange crates/exchange-host/src/grant.rs:105-124 — OperationFacts::of inserts Effect::Network when hosts is non-empty and emits nothing else, so a selector written with_effects_within([Network]) is exact today and silently over-admits the day an operation with another effect ships”

## Status

`blocked` in the source. Quoted from `docs/stories/S-002-effects-are-read-never-derived.md:5`: `status: blocked`. **read**

## Provenance

Migrated from `docs/stories/S-002-effects-are-read-never-derived.md`, which is not deleted and now names this artifact.

- First written 2026-08-13 · last touched 2026-08-13 · 4 revision(s)
- Legacy id `S-002`, recorded as the reference `legacy:S-002`
- Migrated 2026-09-04 by the `aep-planning:story-migration` skill
