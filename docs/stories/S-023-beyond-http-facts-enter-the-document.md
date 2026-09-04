---
id: S-023
title: "Beyond-HTTP facts enter the connector document orthogonally"
pillar: Catalog
status: done
priority:
design: docs/design/03-beyond-http.md
epic: beyond-http
areas: [catalog, catalog-build, connector-spec, connector-resolve]
note: "ADR 0010 delivery item 1; this is schema-wave work and never runs in parallel with another document-schema author"
---

# Beyond-HTTP facts enter the connector document orthogonally

## Goal

Represent interaction shape, closed protocol driver, placement requirements, implementation form,
and capability predicates as independent document facts without restoring a generic runtime enum.

## Acceptance

- [x] The schema and IR type each axis independently and reject unknown driver/capability values.
- [x] Ordinary HTTP documents lower byte-identically unless a reviewed migration says otherwise.
- [x] Exposure to models remains distinct from operation direction and direct-byte interaction.
- [x] Catalog build, reader, resolver, lock, site projection, and fixtures change in the single
      coordinated schema wave.
- [x] The substrate first-party translation has an explicit mapping/overlay fixture; no field is
      called mechanically identical when its vocabulary differs.

## Progress

- 2026-08-13 — done in the coordinated schema wave. Every operation now carries independently
  closed `interaction_shape`, `protocol_driver`, `placement_requirement`, `implementation_form`
  and `required_capabilities` facts through declaration/patch/selector resolution, IR, schema,
  document, pack, reader, resolver, typed catalogue API and site projection. There is no generic
  runtime enum. Placement states a requirement; it is not a runtime-selected destination.
- The existing HTTP surface was reviewed as unary, built-in `http_v1` execution in the connectors
  deployment with public-network authority. Absence, unknown values, and unresolved selector facts
  fail; no consumer supplies defaults.
- `fixtures/substrate-wire-0.1.0-axis-projection.json` pins the owner-issued `substrate-wire` 0.1.0
  bundle manifest digest and this schema digest, snapshots all twelve source operation fact sets,
  maps the source risk/idempotency/exposure vocabularies, and keeps substrate-local effects as
  explicitly mapped capability requirements rather than semantic effects. Its invariant is
  owner-local and never reads a sibling checkout. The complete provider translation and
  byte-equality proof remain S-031 work, named by the fixture rather than implied complete here.
- The substrate adapter's placement is `connectors_deployment`: placement locates the connector
  driver, not the external service whose API it calls. Here the built-in HTTP adapter runs inside
  connectors and reaches a separately deployed substrate through `private_network` authority.
  `substrate_workload` would instead mean running the connector driver itself as an isolated
  substrate workload, which is the later external-artifact case and would be false here.

## Superseded by

`story:beyond-http-facts-enter-the-document` in the AEP planning store, at
`.engineering/planning/story/beyond-http-facts-enter-the-document.md`, now carries this story. Migrated 2026-09-04 by the
`aep-planning:story-migration` skill; this file is kept as the source it was migrated from and is
not the place the rung is recorded any more. The artifact quotes this file's `## Acceptance`
verbatim and names it in its own `## Provenance`.
