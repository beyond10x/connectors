---
format: aep.planning-md/3
id: story:parity-homer-sip-reads
kind: story
status: draft
title: Homer SIP capture call reads
relations:
- decomposes: epic:fluxplane-plugin-parity
- serves: vision:independent-contract-adapters
revision: 2
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

## Domain draft

Drafted 2026-10-07: `adapters/homer/spec/ess` (`connectors_homer`: a `Call` entity keyed by Call-ID, message and stream-quality structs, four selection inputs; 30 UNMAPPED markers) and `adapters/homer/design.md`. Route: a native adapter `connectors.homer`. Homer exchanges a username and password for a JWT, which neither the catalog engine nor any reviewed auth profile supports yet, and `call.list` and `call.qos` group by Call-ID and compute MOS. Open before scheduling: the markers and a reviewed login-exchange auth profile.
