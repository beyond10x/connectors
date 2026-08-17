---
id: S-025
title: "Satellite federation has a threat-modeled contract"
pillar: Platform
status: backlog
priority:
design: docs/design/03-beyond-http.md
epic: beyond-http
areas: [domain, protocol, service, server]
note: "architecture closed by ADRs 0015 and 0018; implementation/conformance remains"
---

# Satellite federation has a threat-modeled contract

## Goal

Make satellite a contained connectors deployment role with explicit bootstrap, catalog generation,
tenant, revocation, partition, replay, and recovery behavior.

## Acceptance

- [x] Architecture RFCs 0001 and 0004 are accepted by ADRs 0015 and 0018 and cited by the protocol.
- [ ] Outward-only bootstrap, deployment identity, tenant binding, monotonic signed catalog/policy
      generations, downgrade refusal, and revocation are conformance-tested.
- [ ] Partition and split-brain behavior is typed; mutation with stale authority refuses by default.
- [ ] Event ordering/gap recovery composes with S-029 and byte routing does not turn control
      federation into a generic reverse tunnel.
- [ ] Compromise is bounded to enrolled destinations, credentials, grants, and one deployment.

## Progress

- Architecture is accepted: outward enrollment, five-minute control lease, observation-only grace,
  signed generation chain, bounded durable queue, and no control-channel byte tunnel are fixed.
- Remaining work is connectors implementation and the listed conformance fixtures.
