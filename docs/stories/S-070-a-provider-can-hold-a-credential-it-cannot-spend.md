---
id: S-070
title: "A provider can hold a credential it cannot spend"
pillar: Catalog
status: ready
priority: 2
design: ../design/16-subscription-credential-custody.md
epic: subscription-custody
areas: [connector-spec, catalog-build]
note: "design 16: the custody_only declaration kind. Today the loader refuses a provider with neither [spec] nor [[operations]], so custody-without-use cannot be expressed. The two workarounds available are both dishonest; this is the schema consequence."
---

# A provider can hold a credential it cannot spend

## Goal

Let a declaration say "this provider owns a credential and no request surface", so a credential
whose *use* belongs to another component can still have an owner, an address and a lifecycle here —
without any path by which connectors could spend it.

## Acceptance

- [ ] `custody_only = true` is a provider-level declaration field, schema'd in
      `crates/connector-spec/schema/provider-toml.schema.json` and represented in the IR.
- [ ] The loader **refuses**, with a named reason each, a `custody_only` provider that declares
      `[spec]`, `[[operations]]`, `[[service]]`, `base_url`, or `verify`. Each is a refusal, not a
      silent allowance, so the kind cannot smuggle a half-declared ordinary provider past review.
- [ ] The loader **requires** at least one `[[auth]]` on a `custody_only` provider — a provider that
      holds nothing has no reason to exist.
- [ ] The existing refusal at `crates/connector-spec/src/provider/validation.rs:30-36` is unchanged
      for every other provider, and its golden `tests/golden/nothing-to-generate.error` still
      matches byte for byte.
- [ ] The invariant is parameterised over every provider setting the flag, in the one consolidated
      file `crates/catalog-build/tests/main/catalog_invariants.rs`. **No per-provider test file** —
      AGENTS.md states the rule once and parameterises it.
- [ ] `catalog build` emits a custody-only entry; `catalog diff` is clean twice; `catalog check`
      passes. The site projection renders the provider without pretending it has operations.

## Progress
- (not started)

## Notes

- Why not the two available workarounds, both rejected in design 16 § The schema consequence:
  pointing `[spec]` at a document and selecting nothing is legal
  (`crates/catalog-build/src/seam.rs:741-743`) but requires a document that does not exist, and
  authoring one is the fabrication AGENTS.md step 2 forbids; declaring one honest operation makes
  the credential spendable, which is the single thing this kind exists to prevent.
- Open question carried from design 16: provider-level or credential-level flag. Provider-level
  until a second case argues otherwise.
