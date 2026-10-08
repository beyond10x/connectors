---
format: aep.planning-md/3
id: story:parity-loki-query
kind: story
status: implemented
title: Loki LogQL range and metric queries and label discovery
relations:
- decomposes: epic:fluxplane-plugin-parity
- serves: vision:independent-contract-adapters
scope:
- confidence: cited
  path: CHANGELOG.md
- confidence: cited
  path: adapters/README.md
- confidence: cited
  path: crates/connectors-build/src/gate.rs
- confidence: cited
  path: crates/connectors-build/src/ignored.rs
- confidence: cited
  path: website/docs/adapters/loki.mdx
revision: 9
transitions:
- {from: "draft", to: "proposed", at: "2026-10-08T18:06:44Z", actor: "human:timo", revision: 3}
- {from: "proposed", to: "active", at: "2026-10-08T18:06:45Z", actor: "human:timo", revision: 4}
- {from: "active", to: "implemented", at: "2026-10-08T21:54:04Z", actor: "human:timo", revision: 8, decided_on: {"recorded":{"test_result":1,"review_outcome":1}}}
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

## Wave 20261008d result

Left the wave blocked on decision-blocker:loki-connection-auth. Work kept on branch `unit/loki-query-20261008d` (28d72d124): ESS reads.yaml additions, contract section 11 (logql-metric, loki-labels), adapter.json and generated descriptor, crate `connectors-loki` with 21 tests on hand-written fixtures in Loki v3.7.0 API shapes, gate enrollment in `crates/connectors-build/src/gate.rs` (applied from the implementor patch, not yet compiled). Scope confirmed: Cargo.toml (members only), Cargo.lock, adapters/loki, docs/fluxplane-plugin-parity.md; added: crates/connectors-build/src/gate.rs, crates/connectors-host/src/local/runtime.rs (for an anonymous profile).

## Wave 20261008e result

Implemented on wave/20261008e (unit commits 034ae4aa7, a6733cfdf, fa5cd3ce6). A loki.bearer connection is proved by GET /loki/api/v1/labels (200; identity source configuration); logs.query_range, logs.query_metric, logs.labels and the connection test answer through operations invoke against hand-written fixtures in Loki v3.7.0 API shapes (not captured live). Package gate on c6b3f9fc3: connectors-loki 33 passed, 1 ignored (the CLI journey, run separately: 1 passed).

Scope corrections: crates/connectors-host/src/local/runtime.rs was wrong (no host change); added crates/connectors-build/src/ignored.rs, CHANGELOG.md, adapters/README.md, website/docs/adapters/loki.mdx.

Adversary pass 1: two contract-drift findings between the connection model and the executable, both fixed (review-result:adversary-loki-query-pass-1).
