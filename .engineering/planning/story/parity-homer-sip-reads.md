---
format: aep.planning-md/3
id: story:parity-homer-sip-reads
kind: story
status: draft
title: Homer SIP capture call reads
relations:
- decomposes: epic:fluxplane-plugin-parity
- serves: vision:independent-contract-adapters
revision: 1
---
## Outcome

Homer SIP capture call reads — parity unit U17 of `docs/fluxplane-plugin-parity.md`.

## Operations

`homer.call.list`, `homer.call.show`, `homer.test`, `homer.call.qos`, `homer.search`

33 calls since 2026-09-09 (declared and mapped undeclared names), provider `homer`, planned wave W4.

## Surface

new provider: catalog if the Homer API's Swagger document can be pinned (not checked), otherwise a native adapter.

## Acceptance

- Spec first: what the unit adds is modelled in the ESS specification it belongs to (adapter model, or pinned OpenAPI source and selection) and validated with the newest `ess` before implementation.
- Each operation above answers through `connectors operations invoke` on a saved connection, against a recorded provider fixture, with the same capability the fluxplane operation gives (the parity page names the gap per operation).
- The parity page row of each operation moves to covered, with the Connectors operation named.
