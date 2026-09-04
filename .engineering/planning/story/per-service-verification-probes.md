---
format: aep.planning-md/1
id: story:per-service-verification-probes
kind: story
status: draft
title: A service declares how a credential is verified, and what a failure means
refs:
- provider: legacy
  reference: S-006
relations:
- derived_from: epic:catalog-adoptions
revision: 1
---
## Acceptance

Verbatim from `docs/stories/S-006-per-service-verification-probes.md:22`. **read**

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

## Context

Let a provider declare a credential-verification probe **per service**, with a failure
classification, so the connection lifecycle's `authorized → callable` transition and its degraded
detection run from declared data rather than per-provider code. Third of the three adoptions the
precedents analysis ordered by cost/benefit.

Source frontmatter: pillar Catalog · areas [catalog, connector-spec, service]. **read**

Source `note:` field, quoted: “research/catalog-precedents.md gap table: Nango declares a verification probe on 281 of 957 providers. Ours is a top-level per-connector `verify` — service-partitioned in the IR already (flux-connectors C-194) but never published or classified. The connection lifecycle needs authorized→callable and callable→degraded to come from declared data, not per-provider code”

## Status

`backlog` in the source. Quoted from `docs/stories/S-006-per-service-verification-probes.md:5`: `status: backlog`. **read**

## Provenance

Migrated from `docs/stories/S-006-per-service-verification-probes.md`, which is not deleted and now names this artifact.

- First written 2026-08-13 · last touched 2026-08-13 · 1 revision(s)
- Legacy id `S-006`, recorded as the reference `legacy:S-006`
- Migrated 2026-09-04 by the `aep-planning:story-migration` skill
