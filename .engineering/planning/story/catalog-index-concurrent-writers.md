---
format: aep.planning-md/1
id: story:catalog-index-concurrent-writers
kind: story
status: draft
title: Two writers into one bundle directory keep both index entries
relations:
- derived_from: review-result:adversary-catalog-pipeline-pass-1
- decomposes: initiative:complete-local-connectors
- serves: vision:independent-contract-adapters
scope:
- confidence: cited
  path: crates/connectors-catalog/src/bundle.rs
revision: 2
---
## Outcome

Two writers into one bundle directory do not lose each other's index entries. Today
they do, and each writer is told it succeeded.

## Evidence

Measured on 2026-09-11 by the adversary pass on `story:catalog-bundle-pipeline`:
eight threads on a barrier, eight distinct providers, one fresh directory. All
eight runs returned a record, all eight bundle files were written, and the index
named two. Six records therefore carry an entry `bundle::read_index` does not
report. The losing count varied — 2, 1, 2, 2 across four observed rounds — and the
case has never gone green in five executions.

The window is the read-modify-write in `bundle::write`: it reads the index,
appends its row, sorts, and writes the whole file back, with nothing between the
read and the write to stop another writer doing the same.

This predates the pipeline unit: `bundle::write` is byte-identical to the commit
that introduced it and is already public, so the window is reachable without the
pipeline.

## Scope

`crates/connectors-catalog/src/bundle.rs`. A lock file, a write-and-rename with a
compare against the bytes read, or a directory of per-provider rows with the index
derived from it — the choice is open and belongs with whoever takes this.

Not in scope: making the bundle write itself atomic, which
`decision-blocker:catalog-pipeline-write-atomicity` covers separately.

## Acceptance

The adversary's own case is the acceptance: eight concurrent runs into one
directory, every run that returns a record has its entry reported by
`read_index`, over several rounds. It lives at
`crates/connectors-catalog/tests/pipeline_adversarial.rs` and is currently red.

## Verification

`cargo test --locked -p connectors-catalog`, with the exit code in the report.
