---
format: aep.planning-md/3
id: story:parity-loki-query
kind: story
status: active
title: Loki LogQL range and metric queries and label discovery
relations:
- decomposes: epic:fluxplane-plugin-parity
- serves: vision:independent-contract-adapters
scope:
- confidence: inferred
  path: Cargo.lock
- confidence: inferred
  path: Cargo.toml
- confidence: inferred
  path: adapters/loki
- confidence: inferred
  path: docs/fluxplane-plugin-parity.md
revision: 4
transitions:
- {from: "draft", to: "proposed", at: "2026-10-08T18:06:44Z", actor: "human:timo", revision: 3}
- {from: "proposed", to: "active", at: "2026-10-08T18:06:45Z", actor: "human:timo", revision: 4}
---
## Outcome

Loki LogQL range and metric queries and label discovery — parity unit U04 of `docs/fluxplane-plugin-parity.md`.

## Operations

`loki.query`, `loki.metric`, `loki.labels`, `loki.test`

371 calls since 2026-09-09 (declared and mapped undeclared names), provider `loki`, planned wave W1.

## Surface

new native adapter; `adapters/loki` holds a design, an ESS model and the `logql-range` contract; metric queries and labels need contract additions.

## Acceptance

- Spec first: what the unit adds is modelled in the ESS specification it belongs to (adapter model, or pinned OpenAPI source and selection) and validated with the newest `ess` before implementation.
- Each operation above answers through `connectors operations invoke` on a saved connection, against a recorded provider fixture, with the same capability the fluxplane operation gives (the parity page names the gap per operation).
- The parity page row of each operation moves to covered, with the Connectors operation named.
