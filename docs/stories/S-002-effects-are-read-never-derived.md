---
id: S-002
title: "Per-operation effects are read from the document, never derived"
pillar: Catalog
status: ready
priority: 2
design:
epic: catalog-day-one
areas: [catalog, catalog-build, domain]
note: "the mistake being closed is recorded in the predecessor's own source: flux-exchange crates/exchange-host/src/grant.rs:105-124 — OperationFacts::of inserts Effect::Network when hosts is non-empty and emits nothing else, so a selector written with_effects_within([Network]) is exact today and silently over-admits the day an operation with another effect ships"
---

# Per-operation effects are read from the document, never derived

## Goal

Make `effects` a declared fact of every operation — carried by the schema, stated by every provider
declaration, and **read** by grant admission — so the domain model's invariant holds by construction:
*the facts the authorization gate decides on are the facts the catalogue publishes.* This is
architecture §2 day-one change 1, second half, and it closes a mistake the predecessor documented
against itself rather than fixed.

## What was measured, and where it is written down

`flux-exchange/crates/exchange-host/src/grant.rs:105-124` is a doc comment titled *"`effects` is
**derived**, and that is a measured gap rather than a detail"*. The catalogue published `risk`,
`idempotency`, `credentials`, `hosts` and the operation's source — and no effects field — so
`OperationFacts::of` did the most the readable data supported:

```rust
let mut effects = BTreeSet::new();
if !operation.hosts.is_empty() {
    effects.insert(Effect::Network);
}
```

`WorkspaceWrite` and `Process` were never emitted, by anything, ever. The consequence, stated in that
same comment: a selector written `with_effects_within([Effect::Network])` was exact for every shipped
connector **and would silently admit an operation with an unreported effect the day one shipped**.
Two assertions held that line — `the_whole_catalogue_declares_http` and
`effects_are_derived_from_hosts_and_never_claim_more_than_that`. The comment ends: *"When upstream
declares effects, this function reads them and the paragraph goes."* This story is that day.

## Acceptance

- [ ] `catalog/connector-document.schema.json` carries `effects` **per operation** as a closed
      vocabulary (domain model: risk, idempotency, effects, direction). An unknown effect value is a
      build error refused by name — never a downgrade, never a default, never a catch-all arm that
      answers a level it has not heard of with a plausible wrong one.
- [ ] Every operation in every provider declaration states its effects, proven by a whole-catalog
      declared-count gate: an operation with no effects declaration fails the build. Nothing infers
      effects from an HTTP verb, from `hosts`, or from `risk`.
- [ ] Grant admission in `crates/domain` reads the effects the document carries. A fence or test
      proves there is **no derivation path**: a seeded document declaring a non-network effect must
      change the admission answer, which is exactly what the derived function could not do.
- [ ] Failing-first test: an operation declaring an effect beyond `network` is admitted only by a
      selector whose effect subset contains that effect, and is refused by
      `with_effects_within([network])`. Under the predecessor's rule this test passes vacuously —
      name it so, so the regression is visible if derivation ever returns.
- [ ] The predecessor's two compensating assertions are deliberately **not** carried, and the story
      records why: they existed to bound a derivation that no longer exists. A grep proving neither
      spelling survives the migration is cheap and worth having.
- [ ] The relationship to the semantic tier is decided and recorded: host-resource effects (what a
      call touches) and semantic effects (what it *means* — money, delete, send-external, from
      flux-connectors C-155) are distinct axes, and collapsing them is what lost `money` in the
      predecessor. Either the schema carries both fields or the second is explicitly deferred with a
      named reason.

## Progress
- (not started)

## Notes

- Read `~/projects/flux-exchange/crates/exchange-host/src/grant.rs:90-147` first — the mistake, its
  blast radius and its exit condition are all documented in place by the people who made it.
- Predecessor stories worth reading: flux-connectors C-155 (the semantic-effect tier and why it is
  separate), C-552 (bundled this field with the caller-contract work; here it is split out because it
  has a second consumer and its own failure mode).
- **Collides with [S-001](S-001-the-document-carries-the-callers-contract.md)** — same schema, same
  lowering, same whole-catalog regeneration. Sequence them.
- Vision principle 2 is the thing at stake: *grants admit from risk/effects/idempotency the catalog
  declares — never from op-id lists a human maintains.* A derived effect set is an op-id list wearing
  a predicate's clothes.
