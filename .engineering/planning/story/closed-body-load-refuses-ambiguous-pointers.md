---
format: aep.planning-md/3
id: story:closed-body-load-refuses-ambiguous-pointers
kind: story
status: draft
title: A closed body naming an array index or an enum beside a prefix is refused when its selection loads
summary: Two closed-body shapes load today and send a value the selection does not declare
relations:
- serves: vision:independent-contract-adapters
scope:
- confidence: cited
  path: adapters/catalog/src/closed.rs
- confidence: cited
  path: adapters/catalog/tests/nested_closed_body.rs
- confidence: cited
  path: crates/connectors-catalog/src/definition.rs
revision: 3
---
## Outcome

A selection whose closed body names an array index as a pointer segment, or puts a `body_enums`
set and a `body_prefixes` prefix on the same key, is refused when it loads, with a message naming
the key; or the engine contract states what each combination means and a test pins it.

## Why later

The review of release 0.54.0 (2026-10-19, lane r1) proved both shapes load today
(`adapters/catalog/tests/verify_r1_3.rs:44`, `verify_r1_5.rs:46` in the scratch tree, logs
`r1-verify-3.log`, `r1-verify-5.log`):

- `body_keys: ["/tags/0"]` with `body_fixed: {"/tags/0": "x"}` loads, and `complete`
  (`adapters/catalog/src/closed.rs:43-55`, `:413-418`) would send `{"tags": {"0": "x"}}`, an object
  where the provider expects an array. A segment `0` can also be a legitimate object key, so the
  rule needs a decision, not a patch.
- An enum checks the caller's text (`closed.rs:380-399`) before the prefix is added
  (`closed.rs:419-423`), so the value sent can be outside the declared set.

No shipped selection or definition uses either shape (`adapters/catalog/providers/*/operations.json`,
`crates/connectors-catalog/tests/fixtures/definitions/connectors/*.yaml`).

## Acceptance

- `verify_r1_3` and `verify_r1_5`, moved into `adapters/catalog/tests/nested_closed_body.rs`, pass:
  each shape is refused at load, or the contract in `docs/local-catalog-provider.md` names the
  combination and the test pins what is sent.
- The definition lowerer (`crates/connectors-catalog/src/definition.rs`) refuses the same shapes, so
  a definition never lowers to a selection that fails to load.
