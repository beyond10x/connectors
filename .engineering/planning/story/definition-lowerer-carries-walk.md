---
format: aep.planning-md/3
id: story:definition-lowerer-carries-walk
kind: story
status: draft
title: A connectors-definition/1 operation declares a walk and the lowerer carries it
tags:
- d240
relations:
- decomposes: epic:fluxplane-plugin-parity
- serves: vision:independent-contract-adapters
- depends_on: story:catalog-operation-declares-its-walk
revision: 1
---
## Problem

`story:catalog-operation-declares-its-walk` adds `walk` to the `connectors-catalog-operations/1`
selection. An operation lowered from a `connectors-definition/1` file cannot declare one, because
that format's schema is owned by the definitions repository and its `Operation` denies unknown
fields (`crates/connectors-catalog/src/definition.rs`). Such an operation refuses `--walk` as
`unsupported`.

## Outcome

`connectors-definition/1` gains an optional `walk` on an operation (or derives it from the
existing `paging` style's `next` and `last`), and the lowerer in `crates/connectors-catalog`
carries it into the selection it writes.

## Acceptance

A test in `crates/connectors-catalog/tests/` lowers a definition with a walk and the lowered
selection carries it; `--describe` of the lowered operation shows it.
