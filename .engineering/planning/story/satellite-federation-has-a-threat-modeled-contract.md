---
format: aep.planning-md/1
id: story:satellite-federation-has-a-threat-modeled-contract
kind: story
status: draft
title: Satellite federation has a threat-modeled contract
refs:
- provider: legacy
  reference: S-025
relations:
- derived_from: epic:beyond-http
scope:
- confidence: cited
  path: crates/domain
- confidence: inferred
  path: crates/protocol
- confidence: inferred
  path: crates/server
- confidence: inferred
  path: crates/service
- confidence: cited
  path: docs/design/03-beyond-http.md
revision: 4
---
## Acceptance

Verbatim from `docs/stories/S-025-satellite-federation-has-a-threat-modeled-contract.md:20`. **read**

- [x] Architecture RFCs 0001 and 0004 are accepted by ADRs 0015 and 0018 and cited by the protocol.
- [ ] Outward-only bootstrap, deployment identity, tenant binding, monotonic signed catalog/policy
      generations, downgrade refusal, and revocation are conformance-tested.
- [ ] Partition and split-brain behavior is typed; mutation with stale authority refuses by default.
- [ ] Event ordering/gap recovery composes with S-029 and byte routing does not turn control
      federation into a generic reverse tunnel.
- [ ] Compromise is bounded to enrolled destinations, credentials, grants, and one deployment.

## Context

Make satellite a contained connectors deployment role with explicit bootstrap, catalog generation,
tenant, revocation, partition, replay, and recovery behavior.

Source frontmatter: pillar Platform · areas [domain, protocol, service, server] · design `docs/design/03-beyond-http.md`. **read**

Source `note:` field, quoted: “architecture closed by ADRs 0015 and 0018; implementation/conformance remains”

## Status

`backlog` in the source. Quoted from `docs/stories/S-025-satellite-federation-has-a-threat-modeled-contract.md:5`: `status: backlog`. **read**

## Provenance

Migrated from `docs/stories/S-025-satellite-federation-has-a-threat-modeled-contract.md`, which is not deleted and now names this artifact.

- First written 2026-08-13 · last touched 2026-08-13 · 2 revision(s)
- Legacy id `S-025`, recorded as the reference `legacy:S-025`
- Migrated 2026-09-04 by the `aep-planning:story-migration` skill

## Scope

Derived 2026-09-07 by `story-scoper` at released base 4d0cd308 — cited.

- **Contract documentation:** `docs/design/03-beyond-http.md:79` — cited; outward federation, signed monotonic generations, bounded authority and separation from byte routing.
- **Authority/placement model:** `crates/domain` — cited; existing FederatedSatellite placement and generation-bound grant facts provide the current model surface.
- **Federation wire contract and conformance:** `crates/protocol` — inferred; bootstrap, tenant/deployment binding, generations, downgrade and partition outcomes need explicit contracts.
- **Federation lifecycle enforcement:** `crates/service` — inferred; leases, revocation, stale-authority refusal and recovery must compose with existing authority services.
- **Control transport:** `crates/server` — inferred; authenticated outward-established federation requires an implementation owner distinct from continuous-byte transport.
- **Disposition:** architecture acceptance is recorded historically; implementation and conformance remain work, not a documentation-only closure — cited.
- **Confidence:** medium; contract boundaries are explicit, but no concrete federation transport/module was located — inferred.
- **Would collide with:** placement/authority types, protocol contracts, authority lifecycle or server control transport — inferred.
