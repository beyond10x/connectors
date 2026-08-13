---
id: S-015
title: "Retire the `quirks` umbrella — pagination, rate limits and error envelopes are ordinary facts"
pillar: Catalog
status: ready
priority: 4
design:
epic: catalog-day-one
areas: [catalog, catalog-build, connector-spec]
note: "owner-raised: `quirks` began as a workaround bag for the broken babelforce API and drifted into a junk drawer holding universal API traits. Every other catalog in the field declares these as plain fields (Nango paginate/retry, Airbyte paginator/error_handler). Schema evolution — must run AFTER the M1 byte-identity differential passes, same wave as S-001/S-002"
---

# Retire the `quirks` umbrella — pagination, rate limits and error envelopes are ordinary facts

## Goal

Delete the `quirks` key and promote what it holds to first-class named fields on the operation and
the service, so the schema stops calling universal API traits *deviations*. Pagination, rate limits
and error envelopes are how HTTP APIs work; filing them under a word that means "this vendor is
broken" mis-describes ~30 shipped providers and makes the one genuinely deviant case invisible
inside the same bag.

## What the word actually holds today

In the predecessor, `Quirks` hangs off an operation and carries exactly three things
(`crates/connector-spec/src/ir.rs:569-586`):

```rust
pub struct Quirks {
    pub pagination: Option<Pagination>,
    pub rate_limit: Option<RateLimit>,
    pub error_envelope: Option<ErrorEnvelope>,
}
```

None of the three is a deviation from anything. The predecessor's own source concedes it, in the
doc comment that introduced the *second* quirks type (`auth.rs:389-394`): *"`quirks.pagination` and
`quirks.rate_limit` are declarations rather than behaviour."*

The genuinely deviant case is the other one: `AuthQuirks::token_endpoint` — babelforce's token
endpoint reads a request `expires_in` its document never declares, defaulting it to *never expires*
on one grant, clamping it to sixty seconds on another, ignoring it on a third. That declaration is
deliberately narrow, and every entry is required to name the grant, what was measured, who measured
it, and when — because *"a quirk is asserted against a vendor's implementation and contradicted by
that vendor's own document"* (`auth.rs:420-427`). That discipline is worth keeping. Sharing a word
with pagination is what dilutes it.

**Industry precedent** ([research/catalog-precedents.md](../research/catalog-precedents.md)): Nango
declares `proxy.paginate` (33 providers) and `proxy.retry` (160 providers) as plain fields with no
umbrella noun; Airbyte's declarative manifest puts `paginator` and `error_handler` directly on the
stream's requester. Nobody else in the corpus files these under an exceptions bag, and our own gap
tables discuss them as ordinary catalog features throughout.

## Acceptance

- [ ] `pagination`, `rate_limit` and `error_envelope` are **first-class named fields** at their
      correct scope in `catalog/connector-document.schema.json` and in the lowering — per operation
      where they vary by endpoint, on the service where they are a property of the whole surface.
      The scope choice per field is decided once and recorded, not left to each provider.
- [ ] **No `quirks` key remains anywhere**: not in the schema, not in any canonical document, not in
      the pack, not in any projection (explorer, catalogue view, effective catalogue), not in a
      provider declaration. Every committed document is regenerated — a whole-catalog change, so the
      regeneration is coordinator-owned.
- [ ] `grep -ri quirk` across `catalog/` and `crates/` returns **only historical references in
      documentation** (design series, story files, CHANGELOG). A hit in a type name, a field name, a
      serde attribute or a test name fails the story.
- [ ] The genuinely deviant declarations are handled deliberately, in one of two ways, and the reason
      is recorded: either a rare, precisely scoped **`workarounds`** category is introduced in which
      **each entry names the specific vendor defect it compensates for** and carries the predecessor's
      required attribution (`grant`/`behaviour`/`attribution`/`measured` — an unattributed claim is
      indistinguishable from a guess that aged), or nothing is introduced at all and the babelforce
      token-endpoint measurements find a home on the auth declaration itself. **Nothing generic is
      introduced.** If it is not in the specification, it does not become a general thing.
- [ ] The migration is a rename with proof, not a redesign: the promoted fields' contents are
      unchanged, and a test shows that for every provider, the set of declared pagination /
      rate-limit / error-envelope facts before and after is identical. Any *behavioural* change to
      those fields belongs to [S-005](S-005-header-name-rate-limit-retry.md), not here.
- [ ] Determinism and the fixed point hold after regeneration: two independent builds are
      byte-identical, `connectors catalog check` ([S-003](S-003-the-lockfile-gets-a-verifier.md)) is
      green on the new lock, and the build reports nothing left to write.
- [ ] Failing-first test named — today the schema refuses a top-level `pagination` on an operation
      and requires the umbrella.

## Progress
- (not started)

## Notes

- **Sequencing constraint (owner-stated):** this must run **after** the M1 byte-identity differential
  against the predecessor's pack has passed (architecture §7.6). The differential compares field
  names; renaming them first would retire the one gate that proves the migration was faithful. It
  then belongs to the **same schema-evolution wave** as
  [S-001](S-001-the-document-carries-the-callers-contract.md) and
  [S-002](S-002-effects-are-read-never-derived.md) — all three change the document schema, the
  lowering, and every committed document, so they are one implementor's work or a strict sequence,
  never parallel authors.
- Read before starting: `crates/connector-spec/src/ir.rs` (`Quirks`, `Operation::quirks`),
  `crates/connector-spec/src/auth.rs` (`AuthQuirks`, `TokenEndpointQuirk` and the owner decision of
  2026-08-02 recorded in its doc comment), and flux-connectors C-12 (*quirks as control flow*) — the
  epic that set the original position, so this story is a deliberate reversal of it and should say so.
- Vision principle 10 (*nouns are forever*) is the reason to do this **now**: the catalog schema is
  versioned and the vocabulary is not, so a bad noun is cheap today and permanent after v1.
