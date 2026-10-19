---
format: aep.planning-md/3
id: story:bundle-index-refuses-ambiguous-rows
kind: story
status: draft
title: The bundle index refuses a repeated provider and a file name outside the bundle directory
summary: 'review run 3 C4 F2: load joins an unchecked file_name and takes the first row'
relations:
- serves: vision:independent-contract-adapters
revision: 1
---
## Outcome

`connectors_catalog::bundle::read_index` refuses an index that names one provider twice or a
`file_name` other than `<provider>.bundle.json`, so `load` never reads a file outside the bundle
directory.

## Why later

Post-0.54.0 review run 3 (lane C4 F2, 2026-10-19): `crates/connectors-catalog/src/bundle.rs:55-57`
returns the first matching row, `read_index` (:60-82) only deserialises, and `load` joins
`entry.file_name` onto the directory (:304), so an absolute name replaces it and `../` escapes it.
Scratch test `crates/connectors-catalog/tests/verify_c4_2.rs` failed all three cases (two rows for
one provider, absolute `file_name`, `../` `file_name`; EXIT=101). Low: the shipped `connectors`
binary reads the compiled-in index and matches `file_name` against a compiled list
(`adapters/catalog/src/local.rs:288-300`); `index.json` is written only by `bundle::write`, which
always writes `<provider>.bundle.json`. A hand-edited build tree is the only reach.

## Acceptance

- `read_index` refuses both shapes with `InvalidInput`; the three scratch cases are tests in
  `crates/connectors-catalog/tests/bundle.rs`.
