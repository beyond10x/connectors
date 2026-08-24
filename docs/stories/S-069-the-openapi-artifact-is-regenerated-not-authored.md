---
id: S-069
title: "The OpenAPI artifact is regenerated, not authored"
pillar: Platform
status: ready
epic: public-surface
areas: [server, docs]
depends_on: [S-067]
---

# The OpenAPI artifact is regenerated, not authored

## Goal

S-067's `openapi.json` was mechanically derived from the frozen
`contracts/connector-*/v0alpha1` wire schemas, but the derivation script was a one-time
scratchpad tool — the artifact is committed without a reproducible generator, below the
repo's own bar (`catalog build`/`catalog check` regenerate and verify catalog.pack). The
drift tests catch contract-side divergence but cannot regenerate, and a future route or
schema change invites hand-editing.

## Acceptance

- A committed generator (`scripts/gen-openapi.py` or equivalent, no new Rust dependency)
  rebuilds `crates/server/src/hosted/docs/openapi.json` from the frozen contract schemas
  plus one explicit overlay file for what postdates the freeze (today: the S-049
  `session_signal` variant) — the overlay is data, not code edits.
- Running the generator on a clean tree produces a byte-identical artifact (stable key
  order, stable formatting); the gate verifies generate == committed, catalog-check
  style, so a hand edit or a contract change without regeneration fails loudly.
- The S-067 drift tests keep passing unchanged — they remain the proof against the live
  Rust types; the generator is the proof against the contract schemas.
- The MCP section, health probes and security scheme — which have no frozen contract
  file — live in the generator's static template, stated as such.
