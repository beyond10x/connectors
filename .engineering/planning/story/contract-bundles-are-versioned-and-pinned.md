---
format: aep.planning-md/1
id: story:contract-bundles-are-versioned-and-pinned
kind: story
status: draft
title: Connector contract bundles are versioned, signed, and pinned
refs:
- provider: legacy
  reference: S-031
relations:
- derived_from: epic:contract-release
revision: 1
---
## Acceptance

Verbatim from `docs/stories/S-031-contract-bundles-are-versioned-and-pinned.md:20`. **read**

- [x] Architecture RFC 0005 is accepted by ADR 0019 and connectors owns explicit catalog and platform bundles.
- [ ] Each manifest records protocol version, source commit, generator version, hashes, and signing
      identity; consumers pin version and digest.
- [ ] Request, response, event, and channel-frame unknown-field/evolution rules are distinct and
      conformance-tested.
- [ ] A clean-room consumer passes the same vectors without repository source access.
- [ ] Release CI uses immutable action pins and emits signed evidence.

## Context

Publish schemas and conformance vectors as reproducible owner-issued releases so substrate, agent,
cloud, Flux, and autodev can pin a contract without copying from `main` or a sibling checkout.

Source frontmatter: pillar Platform · areas [catalog, protocol, ci, docs] · design `docs/design/02-architecture.md`. **read**

Source `note:` field, quoted: “architecture closed by ADR 0019; bundle/release implementation remains”

## Status

`backlog` in the source. Quoted from `docs/stories/S-031-contract-bundles-are-versioned-and-pinned.md:5`: `status: backlog`. **read**

## Provenance

Migrated from `docs/stories/S-031-contract-bundles-are-versioned-and-pinned.md`, which is not deleted and now names this artifact.

- First written 2026-08-13 · last touched 2026-08-13 · 2 revision(s)
- Legacy id `S-031`, recorded as the reference `legacy:S-031`
- Migrated 2026-09-04 by the `aep-planning:story-migration` skill
