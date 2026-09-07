---
format: aep.planning-md/1
id: story:catalog-write-discovery
kind: story
status: draft
title: Catalog writes remain discoverable across connection order and declare approval
relations:
- derived_from: epic:post-m1
scope:
- confidence: cited
  path: crates/connectors-runtime/tests/local_catalog_writes.rs
- confidence: cited
  path: crates/integration-catalog/src/lib.rs
- confidence: cited
  path: crates/integration-catalog/src/tests.rs
revision: 6
---
## Objective

O1: governed reach. A read-only first connection must not hide a write admitted by a later connection. Mutating and destructive operations must declare required approval consistently in search and describe.

## Acceptance

Mixed read-only and writable bindings expose each operation once with all admitting identities. Read-only-only deployments still refuse writes; invocation continues to validate the explicitly chosen connection. Regression tests cover ordering and every admitted operation's effect/approval pair. No approval evaluation moves into the catalog adapter.

## Implementation

The catalog adapter now checks any admitting connection at description time and projects approval posture from declared effects. Named credential test fixtures use neutral bot labels. The focused integration-catalog suite passes all 35 tests; the full repository gate is required before publication.

## Validation

All twelve Cargo workspaces pass their locked tests, including the runtime's no-default-features lane. The catalog lock, links, story index and ESS projection checks pass. The final ESS check used the current source-built CLI because the older installed binary predates the specify command. Focused clippy passes with warnings denied. Live discovery correctly returns a mutating operation with required approval and only its writable connection.

## Scope

Derived 2026-09-07 by `story-scoper` from released base 4d0cd308 — cited.

- **Primary surface:** integration-catalog discovery and selected-Connection admission — cited.
- **Files:** `crates/integration-catalog/src/lib.rs:379` — cited; search groups operations with all admitting bindings; describe at 416 checks any admitting binding; admission at 482 checks the selected Connection.
- **Files:** `crates/integration-catalog/src/tests.rs:406` — cited; read-only-first ordering, effect/approval pairs at 429, and complete identities under search limits at 447.
- **Files:** `crates/connectors-runtime/tests/local_catalog_writes.rs:240` — cited; both binding orders, selected-writer dispatch, read-only selection refusal and zero additional egress.
- **Symbols:** `Inner::search`, `Inner::describe`, `Inner::admit_invocation`, `Binding::admits`, `effect_class`, `approval_posture` — cited.
- **Approval boundary:** `approval_posture` at lib.rs:808 projects metadata; approval evaluation is not added to the adapter — cited.
- **Documents:** no product-document changes identified for this acceptance — inferred.
- **Remaining work:** reconcile the draft lifecycle with existing implementation/evidence before scheduling new implementation — inferred.
- **Confidence:** high; current production predicates and named regressions directly cover the acceptance — cited.
- **Would collide with:** edits to catalog discovery/admission or the listed unit/runtime regression files — cited.
