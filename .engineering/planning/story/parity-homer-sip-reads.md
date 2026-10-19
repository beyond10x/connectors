---
format: aep.planning-md/3
id: story:parity-homer-sip-reads
kind: story
status: draft
title: Homer SIP capture call reads
relations:
- decomposes: epic:fluxplane-plugin-parity
- serves: vision:independent-contract-adapters
- depends_on: story:auth-profile-login-exchange
revision: 3
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

## Plan

Scheduled in phase P3 of the epic's plan of 2026-10-19, after story:auth-profile-login-exchange
(depends_on). Order inside the story:

1. Decide the route by reading the Homer API's own document: if its Swagger document can be
   vendored under `upstream/homer/` with a license, the connector is a catalog provider with a
   definition file; otherwise the native adapter the design names. Record the finding and the
   document's URL, retrieval date and SHA-256 in this body before any code.
2. Resolve the 30 `UNMAPPED` markers in `adapters/homer/spec/ess` through the `ess:specifying`
   interview against that document; a marker that the document does not settle stays `UNMAPPED`
   and is named in the report.
3. `call.list`, `call.show`, `call.qos`, `search` and the identity read behind `homer.test`
   (`connections revalidate`), against recorded fixtures.

Calls: 33 in the baseline (13 list, 10 show, 4 test, 3 qos, 2 search, 1 mapped). The call grouping
by Call-ID and the MOS computation of `call.list` and `call.qos` are composition in the connector,
not in the catalog engine; if the catalog route is chosen they are a native post-processing step
and this story says so.
