---
format: aep.planning-md/1
id: story:the-document-carries-the-callers-contract
kind: story
status: implemented
title: The document carries the caller's contract, so nothing at runtime parses source
refs:
- provider: legacy
  reference: S-001
relations:
- derived_from: epic:catalog-day-one
scope:
- confidence: cited
  path: crates/catalog
- confidence: cited
  path: crates/catalog-build
- confidence: cited
  path: crates/connector-resolve
revision: 6
---
## Acceptance

Verbatim from `docs/stories/S-001-the-document-carries-the-callers-contract.md:35`. **read**

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

## Context

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

Source frontmatter: pillar Catalog · areas [catalog, catalog-build, connector-resolve] · design `../design/04-the-callers-contract.md`. **read**

Source `note:` field, quoted: “ported from flux-connectors C-552 (measured by C-538). Architecture §2 day-one change 1, first half: the caller-facing symbol, the error-envelope-extended description and the contract input_schema. Per-operation effects — C-552's fourth field — are S-002, because they have a second consumer (grant admission) and a mistake of their own to close”

## Status

`done` in the source. Quoted from `docs/stories/S-001-the-document-carries-the-callers-contract.md:5`: `status: done`. **read**

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

Migrated from `docs/stories/S-001-the-document-carries-the-callers-contract.md`, which is not deleted and now names this artifact.

- First written 2026-08-13 · last touched 2026-08-13 · 6 revision(s)
- Legacy id `S-001`, recorded as the reference `legacy:S-001`
- Migrated 2026-09-04 by the `aep-planning:story-migration` skill
