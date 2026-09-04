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
revision: 1
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
