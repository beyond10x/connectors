---
id: S-001
title: "The document carries the caller's contract, so nothing at runtime parses source"
pillar: Catalog
status: ready
priority: 1
design:
epic: catalog-day-one
areas: [catalog, catalog-build, connector-resolve]
note: "ported from flux-connectors C-552 (measured by C-538). Architecture §2 day-one change 1, first half: the caller-facing symbol, the error-envelope-extended description and the contract input_schema. Per-operation effects — C-552's fourth field — are S-002, because they have a second consumer (grant admission) and a mistake of their own to close"
---

# The document carries the caller's contract, so nothing at runtime parses source

## Goal

Close the gap C-538 measured in the predecessor and C-552 was filed to fix: three things a caller or
a model receives are recovered by **parsing emitted source**, because the canonical document does not
carry them. Widen `catalog/connector-document.schema.json` and the lowering so a consumer builds the
complete caller-facing contract from document data alone — the precondition for architecture §5's
absolute rule, *no runtime parsing of any source form, ever; the plan is derived from document data
only.*

The three fields, as C-552 measured them:

1. **the caller-facing symbol** — the document publishes a parameter's IR `name` (`time.start`,
   `$top`) and its `wire` name, but the contract a caller sees advertises the allocated symbol
   (`time_start`, `_top`, `response_2`). The predecessor reproduced the allocator in a second crate,
   held honest only by a differential gate comparing against the emitted declaration;
2. **the error-envelope-extended description** — the projected description appends the
   error-envelope paragraph that the document's one-line summary lacks (measured on
   `airtable-record-get`);
3. **the contract `input_schema`** — lowered from the engine's types, where the document carries the
   vendor's raw JSON Schema (`int64 integer` vs `number`).

## Acceptance

- [ ] `catalog/connector-document.schema.json` gains all three, **additively**, under a minor schema
      bump: a caller-facing symbol per parameter, the projection description, and the contract
      `input_schema` in its **lowered, caller-typed** form — computed at build time by
      `catalog-build` (which has the lowering) and stored as data, not the raw vendor schema, so a
      consumer maps it directly. Every committed canonical document is regenerated; this is a
      whole-catalog artifact change, so the regeneration is coordinator-owned, not per-provider.
- [ ] A consumer constructs the **complete** caller-facing contract — name, description,
      `input_schema`, `expose` — from the document's data alone, with no parse of any connector
      source form anywhere in the consumer. Failing-first test names the consumer path and the
      absence it proves (a guard asserting no source parser is reachable from it).
- [ ] The symbol is written by the lowering, and the second reproduction of the allocator retires:
      `crates/connector-resolve` either reads the document field or is demoted to a *validated* read
      — a document whose symbol disagrees with what the allocator would produce is refused at build,
      by name.
- [ ] C-552's const-pinned trap is closed or proven unreachable: a `const`-pinned body field whose
      name normalizes onto a later parameter's symbol must not shift symbols between the lowering and
      the document (the emitter allocated for every body parameter; the document omits const-pinned
      ones). A fixture pins the case.
- [ ] The `format = "origin"` blind spot is closed (C-538 open question 3): a loader or gate assertion
      requires every `format = "origin"` field's bound variable to lower to `["origin"]` in the
      document, so a provider declaring it for a variable inside a larger authority
      (`https://{v}.x/`) cannot silently drop Origin→Host with nothing red.
- [ ] **`CredentialRequirement` stops being derived.** The document publishes only the *effective*
      auth list, so "this operation declares `auth = []`" and "nothing is declared anywhere" are the
      same empty list, and `crates/catalog/src/table.rs::credential_requirement` resolves the
      difference from the connector default instead of reading it. That derivation reproduces the
      predecessor's classification for all 835 shipped operations and is **ambiguous in principle**
      (recorded at `table.rs:31-36`). The document carries the distinction; the failing-first test is
      the pair of documents today's derivation cannot tell apart.
- [ ] **`Acquisition::Minted` becomes reachable, or is deferred with the reason recorded.** The
      minting join lives in the provider TOML's `[[operations]]` block (`produces_credential`) and
      reaches **no document field**, so `table.rs` can never construct the variant
      (`table.rs:28-30`, `crates/catalog/src/lib.rs:339-345`). C-136's property — *a caller can use a
      credential it can never read* — is therefore unreachable through the canonical document. Either
      the document carries the join (which call mints it, and where in that call's answer the value
      arrives) or the variant's unreachability is stated where the type is defined, with the story
      that would close it named.
- [ ] Determinism is unbroken: two independent builds produce byte-identical documents and pack
      (architecture §7.2), and the one-time migration differential against the predecessor's pack
      (§7.6) still passes for every field the two share.

## Progress
- (not started)

## Notes

- Predecessor: [`C-552 — the document carries the caller's contract`](https://github.com/codewandler/flux-connectors/blob/main/docs/stories/C-552-the-document-carries-the-callers-contract.md)
  — read it before starting; its Notes carry the write set and the design edge. Ported, not
  re-derived, per decision 0026's named migration set.
- The last two acceptance items were **added after M1**, from the two schema gaps the import found in
  code rather than in a story: both are C-552-shaped document additions (a fact the caller's contract
  needs that the document does not carry), both are documented in place by the M1 implementor, and
  both ride this story's schema bump and whole-catalogue regeneration rather than earning a second one.
- **Collides with [S-002](S-002-effects-are-read-never-derived.md)**: both change the document schema
  and the lowering, and both regenerate every canonical document. One implementor, or strictly
  sequenced — never a shared wave with two authors.
- The design edge C-552 recorded and did not settle: carrying the *extended* description in the
  document means the document states host-envelope behaviour. Decide deliberately whether that text
  belongs to the artifact or to the projection layer, and record the reason in the design series
  (a numbered `docs/design/NN-*.md`, per AGENTS.md).
- The redaction row of any differential gate is a **restatement**, not a two-derivation comparison
  (plan and expectation share `placed_form`); if a gate table is written here, it must not read as
  though those two were independently derived.
