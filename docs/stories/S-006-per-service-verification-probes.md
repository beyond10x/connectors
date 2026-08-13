---
id: S-006
title: "A service declares how a credential is verified, and what a failure means"
pillar: Catalog
status: backlog
priority:
design:
epic: catalog-adoptions
areas: [catalog, connector-spec, service]
note: "research/catalog-precedents.md gap table: Nango declares a verification probe on 281 of 957 providers. Ours is a top-level per-connector `verify` — service-partitioned in the IR already (flux-connectors C-194) but never published or classified. The connection lifecycle needs authorized→callable and callable→degraded to come from declared data, not per-provider code"
---

# A service declares how a credential is verified, and what a failure means

## Goal

Let a provider declare a credential-verification probe **per service**, with a failure
classification, so the connection lifecycle's `authorized → callable` transition and its degraded
detection run from declared data rather than per-provider code. Third of the three adoptions the
precedents analysis ordered by cost/benefit.

## Acceptance

- [ ] A service may declare a verification probe — endpoint, expected outcome, failure
      classification — alongside the connector-level `verify`. The resolution rule between the two is
      stated and tested (a service probe wins; the connector probe is the default for services that
      declare none), because a connector whose services live on different hosts has no single true
      probe.
- [ ] The failure classification distinguishes at least **credential invalid** (→ the connection
      becomes `degraded` and a repair flow is offered) from **service unreachable** (→ an
      *operational* event, never a credential verdict). These are the two answers the lifecycle needs
      and neither is recoverable from a bare HTTP status.
- [ ] A probe is **never an operation**: not catalogued, not addressable, not projected to models —
      the predecessor's standing rule that an authentication or verification endpoint is not a
      connector operation, and the accounting that proves a provider's withheld endpoints are
      withheld deliberately.
- [ ] At least two shipped providers declare per-service probes, and fixtures cover both
      classifications for each. Failing-first test named.
- [ ] The probe carries no credential value: placement comes from the catalog's auth section exactly
      as for any request, and a probe's request plan is subject to the same subjects-before-placement
      rule as an invocation.
- [ ] The probe is reachable from the platform's connection lifecycle (a `verify` action on a
      connection) and from `connectors doctor`, through one code path — not two implementations of
      "is this credential still good".

## Progress
- (not started)

## Notes

- Research grounding: [catalog-precedents.md](../research/catalog-precedents.md) — *"We have top-level
  `verify` per connector — confirm it covers per-service probes and failure classification; Nango's
  shape is simpler and battle-tested."*
- Predecessor reading before starting: flux-connectors C-194 (`verify` is one of the three
  service-partitioned surfaces the IR narrows), C-434 (publishing placement and verification posture
  as a published fact), C-60 (the inbound verification conformance matrix — a different `verify`;
  do not conflate credential verification with webhook signature verification).
- Domain model, Connection: the lifecycle is `created → authorized → callable`, with
  `degraded → reauthorize` repairing in place. This story is what makes the second arrow observable
  without a per-provider script.
