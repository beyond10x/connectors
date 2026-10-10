---
format: aep.planning-md/3
id: story:parity-confluence-cql-search
kind: story
status: active
title: Confluence CQL search
relations:
- decomposes: epic:fluxplane-plugin-parity
- serves: vision:independent-contract-adapters
revision: 3
transitions:
- {from: "draft", to: "proposed", at: "2026-10-10T09:55:32Z", actor: "human:timo", revision: 2}
- {from: "proposed", to: "active", at: "2026-10-10T09:55:32Z", actor: "human:timo", revision: 3}
---
## Outcome

Confluence CQL search — parity unit U20 of `docs/fluxplane-plugin-parity.md`.

## Operations

`confluence.page.search`

8 calls since 2026-09-09 (declared and mapped undeclared names), provider `confluence`, planned wave W5.

## Surface

a new pinned OpenAPI source (Confluence REST v1) for the existing catalog provider.

## Acceptance

- Spec first: what the unit adds is modelled in the ESS specification it belongs to (adapter model, or pinned OpenAPI source and selection) and validated with the newest `ess` before implementation.
- Each operation above answers through `connectors operations invoke` on a saved connection, against a recorded provider fixture, with the same capability the fluxplane operation gives (the parity page names the gap per operation).
- The parity page row of each operation moves to covered, with the Connectors operation named.
