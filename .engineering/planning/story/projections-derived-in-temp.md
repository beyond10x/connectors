---
format: aep.planning-md/3
id: story:projections-derived-in-temp
kind: story
status: draft
title: OpenAPI projections are derived when a bundle is rebuilt, not tracked
relations:
- serves: vision:independent-contract-adapters
- informed_by: story:remove-unshipped-material-20261019
revision: 1
---
## Outcome

The Discovery→OpenAPI and Swagger→OpenAPI projections under
`adapters/{google,opsgenie,slack}/generated/` (12 files, 2.95 MB) are produced into the temporary
directory when a bundle is rebuilt and are no longer tracked; the bundle keeps recording its
derivation. Architecture review proposal #3 of 2026-10-19.

## Why not on 2026-10-19

`bundle_drift.rs`, the four Google tests, `slack.rs`, `opsgenie.rs` and
`crates/connectors-catalog/tests/swagger.rs:935-971` read or count the committed projections.

## Acceptance

- The eight tests project in memory and pass; `catalog --derived-from` refuses a projection that
  differs from the recomputed one, as today.
- The six guides' commands write to `.local/tmp` and are re-run in the tree.
