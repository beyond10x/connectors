---
id: S-025
title: "Satellite federation has a threat-modeled contract"
pillar: Platform
status: blocked
priority:
design: docs/design/03-beyond-http.md
epic: beyond-http
areas: [domain, protocol, service, server]
note: "blocked on acceptance of b10x/architecture RFC 0004 and the trust envelope in RFC 0001"
---

# Satellite federation has a threat-modeled contract

## Goal

Make satellite a contained connectors deployment role with explicit bootstrap, catalog generation,
tenant, revocation, partition, replay, and recovery behavior.

## Acceptance

- [ ] Architecture RFCs 0001 and 0004 are accepted and cited by the protocol.
- [ ] Outward-only bootstrap, deployment identity, tenant binding, monotonic signed catalog/policy
      generations, downgrade refusal, and revocation are conformance-tested.
- [ ] Partition and split-brain behavior is typed; mutation with stale authority refuses by default.
- [ ] Event ordering/gap recovery composes with S-029 and byte routing does not turn control
      federation into a generic reverse tunnel.
- [ ] Compromise is bounded to enrolled destinations, credentials, grants, and one deployment.

## Progress

- (blocked on architecture RFCs 0001 and 0004)
