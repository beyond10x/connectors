---
id: S-023
title: "Beyond-HTTP facts enter the connector document orthogonally"
pillar: Catalog
status: backlog
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

- [ ] The schema and IR type each axis independently and reject unknown driver/capability values.
- [ ] Ordinary HTTP documents lower byte-identically unless a reviewed migration says otherwise.
- [ ] Exposure to models remains distinct from operation direction and direct-byte interaction.
- [ ] Catalog build, reader, resolver, lock, site projection, and fixtures change in the single
      coordinated schema wave.
- [ ] The substrate first-party translation has an explicit mapping/overlay fixture; no field is
      called mechanically identical when its vocabulary differs.

## Progress

- (not started)
