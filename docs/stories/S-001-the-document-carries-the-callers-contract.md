---
id: S-001
title: "The document carries the caller's contract, so nothing at runtime parses source"
pillar: Catalog
status: done
design: ../design/04-the-callers-contract.md
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

- [x] `catalog/connector-document.schema.json` gains all three, **additively**: a caller-facing
      symbol per parameter, the projection description, and the contract `input_schema` in its
      **lowered, caller-typed** form — computed at build time by `catalog-build` (which has the
      lowering, `src/contract.rs`) and stored as data. Every committed canonical document was
      regenerated whole-catalogue. → `schema_version` stays 1, not a bump: additive per the
      schema's own rule and C-537; the resolution is design 04 §2.
- [x] A consumer constructs the **complete** caller-facing contract — name, description,
      `input_schema`, `expose` — from the document's data alone.
      → `catalog::Operation::{contract_description, input_schema, expose}`;
      `consumer_api.rs::the_caller_contract_is_document_data_alone` names the consumer path and
      proves the absence (no source-form parser in the crate's dependency closure).
- [x] The symbol is written by the lowering, and the second reproduction of the allocator retires:
      `crates/connector-resolve` reads the document field (its copy survives only as the
      pre-S-001-document fallback, exercised by no build here). Disagreement is refused at build:
      the fixed-point invariant catches any committed symbol the allocator would not produce, and
      `the_contract_and_the_params_state_the_same_symbols` names the operation whose two symbol
      statements drift.
- [x] The const-pinned trap is closed: the allocator reserves a symbol for every body parameter,
      `const`-pinned ones included, and the document states the shifted result.
      → `contract.rs::a_const_pinned_body_field_reserves_the_symbol_a_later_field_must_shift_past`
      (build side) and `connector-resolve/document.rs`'s reader-side twin.
- [x] The `format = "origin"` blind spot is closed.
      → `catalog_invariants.rs::every_format_origin_field_lowers_to_the_origin_slot`, with the
      cannot-go-blind counter (gitlab is the shipped case).
- [x] **`CredentialRequirement` stops being derived.** The document publishes
      `credential_requirement` (C-206's tokens), computed at build where `Operation::auth`'s
      `Option` still distinguishes declared-empty from never-declared;
      `table.rs::credential_requirement` reads it and the default-resolving derivation is gone.
      → failing-first pair: `table.rs::tests::the_document_tells_apart_the_pair_the_derivation_could_not`.
- [x] **`Acquisition::Minted` is reachable** — the carry arm, not the deferral: the document
      gains the optional `produces_credential { credential, secret }` join (zero byte cost on
      every shipped document; none declares one) and `table.rs` constructs the variant from it,
      refusing conflicting provenance by name.
      → `table.rs::tests::a_minting_join_in_the_document_reaches_acquisition_minted`.
- [x] Determinism is unbroken: the fixed-point and two-plans invariants are green over the
      regenerated tree, and the one-time differential against the predecessor ran against its
      C-552 regenerated documents (the shared fields, which its pack predates): **835/835
      operations, 1518/1518 symbols** — `symbol`, `contract.description`, `contract.input_schema`
      all byte-equal to the engine-derived values. Effects excluded (S-002).

## Progress
- 2026-08-13 — done. Ported from the predecessor's reviewed `impl/C-552` (commit `6dae5439`,
  whose parent `3650a136` is exactly the M1 import pin, so the surviving hunks applied verbatim:
  `connector-resolve` byte-identical, the document builder/schema with two call-site swaps). The
  allocator moved whole into `connector_spec::names`; the one engine-bound function
  (`OpSpec::lower`'s input-schema projection) is restated engine-free in
  `catalog-build/src/contract.rs` and held to the predecessor's engine output by the one-time
  differential: 55 providers, 835/835 operations, 1518/1518 symbols, 100% equal. The two post-M1
  items landed as data: `credential_requirement` (read, no longer derived) and the
  `produces_credential` minting join (Minted reachable). Full gate green: workspace build/test
  (all suites), clippy `-D warnings`, fmt, `catalog build` fixed point, `catalog diff` clean,
  four new whole-catalogue invariants (9–12). Decisions recorded in
  [design 04](../design/04-the-callers-contract.md).

## Notes

- Predecessor: [`C-552 — the document carries the caller's contract`](https://github.com/codewandler/flux-connectors/blob/main/docs/stories/C-552-the-document-carries-the-callers-contract.md)
  — read it before starting; its Notes carry the write set and the design edge. Ported, not
  re-derived. The private migration record is provenance; this story and design 04 contain the
  normative B10x contract.
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

## Superseded by

`story:the-document-carries-the-callers-contract` in the AEP planning store, at
`.engineering/planning/story/the-document-carries-the-callers-contract.md`, now carries this story. Migrated 2026-09-04 by the
`aep-planning:story-migration` skill; this file is kept as the source it was migrated from and is
not the place the rung is recorded any more. The artifact quotes this file's `## Acceptance`
verbatim and names it in its own `## Provenance`.
